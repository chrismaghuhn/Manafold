//! Random-vs-random smoke games through the production player endpoints.
//!
//! Each game: two players with seed-chosen basic lands (twenty in each
//! library, seven in hand) play until turn 31 begins. A uniform random
//! policy answers every decision using only the request its player sees.
//!
//! Thirty-turn games need an optimized build: `scripts/run_checks.py`
//! runs `cargo test --release -p mtgml-environment --test random_smoke`.
//! Set `MANAFOLD_SMOKE_GAMES` for more games than the default one.

mod common;

use common::{two_player_land_game, SplitMix64, P1, P2};
use mtgml_decision::{
    DecisionAnswerV2, DecisionDomainV2, DecisionResponseV3, PlayerDecisionRequestV4,
    DECISION_RESPONSE_V3_SCHEMA,
};
use mtgml_environment::{PlayerEndpoint, PlayerEndpointHandle, TrustedEnvironmentController};
use mtgml_model::{EpisodeStatus, PlayerId};
use mtgml_observation::PlayerStepSubmissionV1;

const FIRST_SEED: u64 = 0x4D41_4E41;
const LAST_TURN: u64 = 30;
const SHORT_LAST_TURN: u64 = 3;
const SHORT_FINGERPRINT: &str = "350cf070471b7bcc386d2063d6316b4caa522437ecc2a64311febcf6f9051473";
const LONG_FINGERPRINT: &str = "44b09d6547f86a37396c265ea5d91c8051b8491df8290bfa52dafc18df9ce387";
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
        ref other => panic!("unexpected decision domain {other:?}"),
    }
}

fn play(seed: u64, last_turn: u64) -> (Vec<Entry>, TrustedEnvironmentController) {
    let controller = two_player_land_game(20, 7, seed);
    let players: [PlayerEndpointHandle; 2] = [
        controller.bind_player(P1).unwrap(),
        controller.bind_player(P2).unwrap(),
    ];
    let mut rng = SplitMix64(seed ^ 0x5EED_0FA1_1CE5);
    let mut trajectory = Vec::new();
    loop {
        let checkpoint = controller.checkpoint().unwrap();
        if checkpoint.state.core.turn_number > last_turn
            || !matches!(checkpoint.status, EpisodeStatus::Running)
        {
            return (trajectory, controller);
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
        let response = DecisionResponseV3 {
            schema_version: DECISION_RESPONSE_V3_SCHEMA.to_owned(),
            player_decision_id: request.player_decision_id,
            view_sequence: request.view_sequence,
            answer: random_answer(&request, &mut rng),
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
