//! Random-vs-random smoke games through the production player endpoints.
//!
//! Each game: two players with seed-chosen 27-card decks start the game as
//! CR 103 prescribes (starting player, shuffles, seven-card hands,
//! mulligans) and play until turn 31 begins or the game ends. A uniform
//! random policy answers every decision using only the request its player
//! sees.
//!
//! Two kinds of games run: two basic-land decks against each other, and two
//! decks of 17 lands and 10 vanilla creatures each. The second kind casts
//! creatures, attacks, blocks (several creatures may block one attacker),
//! divides combat damage among blockers, loses creatures to damage, and can
//! end the game at 0 life.
//!
//! Thirty-turn games need an optimized build: `scripts/run_checks.py`
//! runs `cargo test --release -p mtgml-environment --test random_smoke`.
//! Set `MANAFOLD_SMOKE_GAMES` for more land games than the default one.

mod common;

use common::{
    creature_deck_game, creature_definitions, deck_game, random_lands, SplitMix64, P1, P2,
};
use mtgml_decision::{
    CandidateIntent, DecisionAnswerV2, DecisionDomainV2, DecisionPurposeV4, DecisionResponseV3,
    PlayerDecisionRequestV4, DECISION_RESPONSE_V3_SCHEMA,
};
use mtgml_environment::{PlayerEndpoint, PlayerEndpointHandle, TrustedEnvironmentController};
use mtgml_model::{
    CardDefinitionId, EpisodeStatus, PlayerId, PlayerResult, TerminalReason, ZoneKind,
};
use mtgml_observation::{ObservedEventEnvelopeV4, ObservedEventKindV4, PlayerStepSubmissionV1};

const FIRST_SEED: u64 = 0x4D41_4E41;
const LAST_TURN: u64 = 30;
const SHORT_LAST_TURN: u64 = 3;
const SHORT_FINGERPRINT: &str = "35a8ba9d4940054203b5c8328d4515a62288a1a15234b6fa7ee7664a62e04f39";
const LONG_FINGERPRINT: &str = "e6a676b0fc47b6f184dc60006e22b666abcb4d24c112c6bce12e86ce3458efa3";
/// The first creature-deck seed whose game declares a block before turn 9
/// begins, so that the short pin covers blocking and not only casting and
/// attacking (`FIRST_SEED` through `FIRST_SEED + 31` declare no block that
/// early). Random players develop slowly: none of the first 60 seeds declares
/// a block before turn 8 begins, which is why the pin runs to turn 9.
const CREATURE_SHORT_SEED: u64 = FIRST_SEED + 32;
/// Until turn 9 begins.
const CREATURE_SHORT_LAST_TURN: u64 = 8;
const CREATURE_SHORT_FINGERPRINT: &str =
    "e9761bdb156707b9b79d61c723a09808dd0411f37a4eccbaa503ab9872e0ecda";
/// How many creature games run, on the seeds `FIRST_SEED` onwards. The test
/// asserts that these games, taken together, cover every kind of play (casts,
/// multi-blocks, divisions, deaths, graveyard orders, ...). When a coverage
/// assertion reports a count of 0, the games did not reach that play: raise
/// this constant in steps of 5 until they do. The trajectories of the first
/// games do not change.
const CREATURE_GAMES: u64 = 15;
const CREATURE_LANDS: usize = 17;
const CREATURE_CREATURES: usize = 10;
const MAX_DECISIONS: usize = 5_000;

fn game_count() -> u64 {
    std::env::var("MANAFOLD_SMOKE_GAMES")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(1)
}

/// One response: who answered, what they sent, what they saw back, what
/// both players know afterwards, and the resulting checkpoint digest.
#[derive(Debug, PartialEq, Eq)]
struct Entry {
    actor: PlayerId,
    response: Vec<u8>,
    step: Vec<u8>,
    knowledge: [Vec<u8>; 2],
    checkpoint_digest: String,
}

fn random_answer(request: &PlayerDecisionRequestV4, rng: &mut SplitMix64) -> DecisionAnswerV2 {
    let ids: Vec<_> = request
        .candidates
        .iter()
        .map(|candidate| candidate.candidate_id)
        .collect();
    match request.decision_domain_v2 {
        DecisionDomainV2::ChooseMany { minimum, maximum } => {
            let size = minimum as usize + rng.below((maximum - minimum) as usize + 1);
            let mut pool = ids;
            let mut chosen = Vec::with_capacity(size);
            for _ in 0..size {
                chosen.push(pool.remove(rng.below(pool.len())));
            }
            chosen.sort();
            DecisionAnswerV2::SelectMany {
                candidate_ids: chosen,
            }
        }
        DecisionDomainV2::ChooseOne => DecisionAnswerV2::SelectOne {
            candidate_id: ids[rng.below(ids.len())],
        },
        // A uniformly random subset of an allowed size, in a uniformly
        // random order.
        DecisionDomainV2::Order { minimum, maximum } => {
            let size = minimum as usize + rng.below((maximum - minimum) as usize + 1);
            let mut pool = ids;
            let mut chosen = Vec::with_capacity(size);
            for _ in 0..size {
                chosen.push(pool.remove(rng.below(pool.len())));
            }
            DecisionAnswerV2::Order {
                candidate_ids: chosen,
            }
        }
        ref other => panic!("unexpected decision domain {other:?}"),
    }
}

/// What the players did, counted from the requests they saw and the
/// answers the engine accepted.
#[derive(Debug, Default, PartialEq, Eq)]
struct Tally {
    /// Accepted answers that cast a spell.
    casts: usize,
    /// Mana-payment decisions answered.
    payments: usize,
    /// Attacker declarations that attack with at least one creature.
    attacks: usize,
    /// Blocker declarations that block an attacker.
    blocks: usize,
    /// Observed block declarations that block one attacker with two or more
    /// creatures.
    multi_blocks: usize,
    /// Combat damage divisions answered.
    divisions: usize,
    /// Observed zone moves from the battlefield to a graveyard (only
    /// creatures die in these games).
    deaths: usize,
    /// Graveyard orders answered.
    graveyard_orders: usize,
}

impl Tally {
    fn record(&mut self, request: &PlayerDecisionRequestV4, answer: &DecisionAnswerV2) {
        match (&request.purpose, answer) {
            (DecisionPurposeV4::ManaPayment, _) => self.payments += 1,
            (
                DecisionPurposeV4::AttackerDeclaration,
                DecisionAnswerV2::SelectMany { candidate_ids },
            ) if !candidate_ids.is_empty() => self.attacks += 1,
            (DecisionPurposeV4::CombatDamageAssignment, _) => self.divisions += 1,
            (DecisionPurposeV4::SbaGraveyardOrder, _) => self.graveyard_orders += 1,
            (
                DecisionPurposeV4::BlockerDeclaration,
                DecisionAnswerV2::SelectOne { candidate_id },
            ) => {
                let chosen = request
                    .candidates
                    .iter()
                    .find(|candidate| candidate.candidate_id == *candidate_id)
                    .expect("the answer names one of the candidates");
                if matches!(
                    chosen.intent,
                    CandidateIntent::DeclareBlock {
                        attacker: Some(_),
                        ..
                    }
                ) {
                    self.blocks += 1;
                }
            }
            (DecisionPurposeV4::PriorityAction, DecisionAnswerV2::SelectOne { candidate_id }) => {
                let chosen = request
                    .candidates
                    .iter()
                    .find(|candidate| candidate.candidate_id == *candidate_id)
                    .expect("the answer names one of the candidates");
                if matches!(chosen.intent, CandidateIntent::CastSpell { .. }) {
                    self.casts += 1;
                }
            }
            _ => {}
        }
    }

    /// Counts the events the answering player observed of the step. Both
    /// players observe block declarations and deaths, so one perspective
    /// counts each of them once.
    fn observe(&mut self, events: &[ObservedEventEnvelopeV4]) {
        for envelope in events {
            match &envelope.event {
                ObservedEventKindV4::BlockersDeclared { blocks, .. } => {
                    if blocks.iter().enumerate().any(|(index, block)| {
                        blocks[..index]
                            .iter()
                            .any(|other| other.attacker == block.attacker)
                    }) {
                        self.multi_blocks += 1;
                    }
                }
                ObservedEventKindV4::ObjectMoved {
                    from: ZoneKind::Battlefield,
                    to: ZoneKind::Graveyard,
                    ..
                } => self.deaths += 1,
                _ => {}
            }
        }
    }

    fn add(&mut self, other: &Tally) {
        self.casts += other.casts;
        self.payments += other.payments;
        self.attacks += other.attacks;
        self.blocks += other.blocks;
        self.multi_blocks += other.multi_blocks;
        self.divisions += other.divisions;
        self.deaths += other.deaths;
        self.graveyard_orders += other.graveyard_orders;
    }
}

/// Two basic-land decks.
fn play(seed: u64, last_turn: u64) -> (Vec<Entry>, TrustedEnvironmentController) {
    let mut decks = SplitMix64(seed);
    let controller = deck_game(
        [random_lands(27, &mut decks), random_lands(27, &mut decks)],
        seed,
    );
    let (trajectory, _, controller) = play_out(controller, seed, last_turn);
    (trajectory, controller)
}

/// 17 random lands and 10 random creatures (of the three vanilla creatures).
fn creature_deck(rng: &mut SplitMix64) -> Vec<CardDefinitionId> {
    let witnesses = creature_definitions();
    let mut deck = random_lands(CREATURE_LANDS, rng);
    deck.extend((0..CREATURE_CREATURES).map(|_| witnesses[rng.below(witnesses.len())]));
    deck
}

/// Two creature decks.
fn play_creatures(seed: u64, last_turn: u64) -> (Vec<Entry>, Tally, TrustedEnvironmentController) {
    let mut decks = SplitMix64(seed);
    let decks = [creature_deck(&mut decks), creature_deck(&mut decks)];
    play_out(creature_deck_game(decks, seed), seed, last_turn)
}

/// Answers every decision at random until turn `last_turn + 1` begins or
/// the game ends.
fn play_out(
    controller: TrustedEnvironmentController,
    seed: u64,
    last_turn: u64,
) -> (Vec<Entry>, Tally, TrustedEnvironmentController) {
    let players: [PlayerEndpointHandle; 2] = [
        controller.bind_player(P1).unwrap(),
        controller.bind_player(P2).unwrap(),
    ];
    let mut rng = SplitMix64(seed ^ 0x5EED_0FA1_1CE5);
    let mut trajectory = Vec::new();
    let mut tally = Tally::default();
    loop {
        let checkpoint = controller.checkpoint().unwrap();
        if checkpoint.state.core.turn_number > last_turn
            || !matches!(checkpoint.status, EpisodeStatus::Running)
        {
            return (trajectory, tally, controller);
        }
        assert!(
            trajectory.len() < MAX_DECISIONS,
            "seed {seed:#x}: too many decisions"
        );
        let (player, request) = players
            .iter()
            .find_map(|player| {
                player
                    .visible_decision()
                    .unwrap()
                    .map(|request| (player, request))
            })
            .unwrap_or_else(|| panic!("seed {seed:#x}: no player has a decision"));
        let answer = random_answer(&request, &mut rng);
        let response = DecisionResponseV3 {
            schema_version: DECISION_RESPONSE_V3_SCHEMA.to_owned(),
            player_decision_id: request.player_decision_id,
            view_sequence: request.view_sequence,
            answer: answer.clone(),
        };
        let response_bytes = mtgml_wire::encode_canonical(&response).unwrap();
        let step = player
            .submit(response)
            .unwrap_or_else(|error| panic!("seed {seed:#x}: {error:?}"));
        assert_eq!(
            step.submission,
            PlayerStepSubmissionV1::Accepted,
            "seed {seed:#x}"
        );
        tally.record(&request, &answer);
        tally.observe(&step.observed_events);
        trajectory.push(Entry {
            actor: player.perspective(),
            response: response_bytes,
            step: mtgml_wire::encode_canonical(&step).unwrap(),
            knowledge: [
                mtgml_wire::encode_canonical(&players[0].information_state().unwrap()).unwrap(),
                mtgml_wire::encode_canonical(&players[1].information_state().unwrap()).unwrap(),
            ],
            checkpoint_digest: controller
                .checkpoint()
                .unwrap()
                .checkpoint_digest
                .as_str()
                .to_owned(),
        });
    }
}

/// SHA-256 over every entry, each field length-prefixed so entry
/// boundaries are unambiguous. Pinned below: any change to what the
/// players see, send or know, or to the checkpoint identity, changes it.
fn fingerprint(trajectory: &[Entry]) -> String {
    let mut buffer = Vec::new();
    for entry in trajectory {
        buffer.extend_from_slice(&entry.actor.0.to_le_bytes());
        for field in [
            entry.response.as_slice(),
            entry.step.as_slice(),
            entry.knowledge[0].as_slice(),
            entry.knowledge[1].as_slice(),
            entry.checkpoint_digest.as_bytes(),
        ] {
            buffer.extend_from_slice(&(field.len() as u64).to_le_bytes());
            buffer.extend_from_slice(field);
        }
    }
    mtgml_model::Digest::from_bytes(&buffer).as_str().to_owned()
}

#[test]
fn short_game_matches_its_pinned_fingerprint() {
    assert_eq!(
        fingerprint(&play(FIRST_SEED, SHORT_LAST_TURN).0),
        SHORT_FINGERPRINT
    );
}

#[test]
#[cfg_attr(debug_assertions, ignore = "needs a release build; see module docs")]
fn random_games_run_thirty_turns_deterministically_and_replay() {
    for game in 0..game_count() {
        let seed = FIRST_SEED + game;
        let (trajectory, controller) = play(seed, LAST_TURN);
        let last = controller.checkpoint().unwrap();
        assert_eq!(last.state.core.turn_number, LAST_TURN + 1, "seed {seed:#x}");

        let (again, _) = play(seed, LAST_TURN);
        assert_eq!(
            trajectory, again,
            "seed {seed:#x}: same seed, different trajectory"
        );
        if game == 0 {
            assert_eq!(fingerprint(&trajectory), LONG_FINGERPRINT, "seed {seed:#x}");
        }

        let report = controller
            .execute_replay(controller.export_replay().unwrap())
            .unwrap();
        assert_eq!(
            report.final_checkpoint, last,
            "seed {seed:#x}: replay diverges"
        );
    }
}

#[test]
#[cfg_attr(debug_assertions, ignore = "needs a release build; see module docs")]
fn different_seeds_give_different_trajectories() {
    let (first, _) = play(FIRST_SEED, LAST_TURN);
    let (second, _) = play(FIRST_SEED + 1, LAST_TURN);
    assert_ne!(first, second);
}

#[test]
fn creature_short_game_matches_its_pinned_fingerprint() {
    let (trajectory, tally, _) = play_creatures(CREATURE_SHORT_SEED, CREATURE_SHORT_LAST_TURN);
    // If a change moves the block out of this game, fail here rather than keep
    // a pin that covers casting and attacking only; then choose the first seed
    // that declares a block before the turn and re-pin.
    assert!(
        tally.blocks > 0,
        "the pinned game no longer declares a block: {tally:?}"
    );
    assert_eq!(
        fingerprint(&trajectory),
        CREATURE_SHORT_FINGERPRINT,
        "{tally:?}"
    );
}

/// One game of two creature decks played twice (same trajectory) and replayed
/// (same final checkpoint). Returns what the players did and whether the game
/// ended with a winner.
fn check_creature_game(seed: u64) -> (Tally, bool) {
    let (trajectory, tally, controller) = play_creatures(seed, LAST_TURN);
    let last = controller.checkpoint().unwrap();

    let (again, again_tally, _) = play_creatures(seed, LAST_TURN);
    assert_eq!(
        trajectory, again,
        "seed {seed:#x}: same seed, different trajectory"
    );
    assert_eq!(tally, again_tally, "seed {seed:#x}");

    let report = controller
        .execute_replay(controller.export_replay().unwrap())
        .unwrap();
    assert_eq!(
        report.final_checkpoint, last,
        "seed {seed:#x}: replay diverges"
    );

    let has_winner = match &last.status {
        EpisodeStatus::Running => {
            assert_eq!(last.state.core.turn_number, LAST_TURN + 1, "seed {seed:#x}");
            // CR 704.5a: a player at 0 or less life loses the game, so no
            // player of a game that is still running is there.
            for (player, state) in &last.state.core.players {
                assert!(
                    state.life > 0,
                    "seed {seed:#x}: {player:?} is at {} life in a running game",
                    state.life
                );
            }
            false
        }
        EpisodeStatus::Terminal {
            reason: TerminalReason::RulesLoss,
            players,
        } => {
            let results: Vec<_> = players
                .iter()
                .map(|outcome| {
                    (
                        outcome.result,
                        last.state.core.players[&outcome.player].life,
                    )
                })
                .collect();
            assert_eq!(results.len(), 2, "seed {seed:#x}: {players:?}");
            let losers = results
                .iter()
                .filter(|(result, _)| *result == PlayerResult::Loss)
                .count();
            let winners = results
                .iter()
                .filter(|(result, _)| *result == PlayerResult::Win)
                .count();
            assert_eq!((winners, losers), (1, 1), "seed {seed:#x}: {players:?}");
            for (result, life) in results {
                match result {
                    PlayerResult::Loss => {
                        assert!(life <= 0, "seed {seed:#x}: the loser is at {life} life");
                    }
                    PlayerResult::Win => {
                        assert!(life > 0, "seed {seed:#x}: the winner is at {life} life");
                    }
                    other => panic!("seed {seed:#x}: unexpected result {other:?}"),
                }
            }
            true
        }
        other => panic!("seed {seed:#x}: unexpected status {other:?}"),
    };
    (tally, has_winner)
}

/// The message of a failed coverage assertion: what the games did not reach,
/// the remedy, and what they did reach.
fn uncovered(what: &str, total: &Tally) -> String {
    format!("{what} in {CREATURE_GAMES} games; raise CREATURE_GAMES in steps of 5: {total:?}")
}

#[test]
#[cfg_attr(debug_assertions, ignore = "needs a release build; see module docs")]
fn creature_games_fight_to_a_winner() {
    // Each game builds its own controller, so the games run side by side.
    let games: Vec<(Tally, bool)> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..CREATURE_GAMES)
            .map(|game| scope.spawn(move || check_creature_game(FIRST_SEED + game)))
            .collect();
        handles
            .into_iter()
            .map(|handle| {
                handle
                    .join()
                    .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
            })
            .collect()
    });
    let mut total = Tally::default();
    let mut winners = 0;
    for (tally, has_winner) in &games {
        total.add(tally);
        winners += usize::from(*has_winner);
    }
    eprintln!("creature games: {total:?}, {winners} of {CREATURE_GAMES} ended with a winner");
    assert!(
        total.casts > 0,
        "{}",
        uncovered("no spell was cast", &total)
    );
    assert!(
        total.payments > 0,
        "{}",
        uncovered("no mana payment was decided", &total)
    );
    assert!(
        total.attacks > 0,
        "{}",
        uncovered("no creature attacked", &total)
    );
    assert!(
        total.blocks > 0,
        "{}",
        uncovered("no creature blocked", &total)
    );
    assert!(
        total.multi_blocks > 0,
        "{}",
        uncovered("no attacker was blocked twice", &total)
    );
    assert!(
        total.divisions > 0,
        "{}",
        uncovered("no combat damage was divided", &total)
    );
    assert!(
        total.deaths > 0,
        "{}",
        uncovered("no creature died", &total)
    );
    assert!(
        total.graveyard_orders > 0,
        "{}",
        uncovered("no graveyard order was decided", &total)
    );
    assert!(
        winners > 0,
        "{}",
        uncovered("no game ended with a winner", &total)
    );
}
