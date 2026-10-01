//! A game started from two deck lists (CR 103) through the production
//! controller and the player endpoints.

mod common;

use common::{deck_game, land_definitions, P1, P2};
use mtgml_decision::{
    CandidateIntent, DecisionAnswerV2, DecisionPurposeV4, DecisionResponseV3,
    PlayerDecisionRequestV4, DECISION_RESPONSE_V3_SCHEMA,
};
use mtgml_environment::{PlayerEndpoint, PlayerEndpointHandle, TrustedEnvironmentController};
use mtgml_model::{CardDefinitionId, PlayerId};
use mtgml_observation::{ObservedEventKindV4, PlayerStepSubmissionV1, PlayerStepV4};
use mtgml_state::{BeginningStep, TurnPosition};

struct Game {
    controller: TrustedEnvironmentController,
    players: [PlayerEndpointHandle; 2],
}

impl Game {
    fn new(decks: [Vec<CardDefinitionId>; 2], seed: u64) -> Self {
        let controller = deck_game(decks, seed);
        let players = [
            controller.bind_player(P1).unwrap(),
            controller.bind_player(P2).unwrap(),
        ];
        Self {
            controller,
            players,
        }
    }

    fn endpoint(&self, player: PlayerId) -> &PlayerEndpointHandle {
        &self.players[usize::from(player != P1)]
    }

    /// The player who has a decision, and that decision.
    fn pending(&self) -> (PlayerId, PlayerDecisionRequestV4) {
        let mut pending = self.players.iter().filter_map(|player| {
            player
                .visible_decision()
                .unwrap()
                .map(|request| (player.perspective(), request))
        });
        let first = pending.next().expect("no player has a decision");
        assert!(pending.next().is_none(), "two players have a decision");
        first
    }

    /// The deciding player picks the candidate `wanted` accepts.
    fn answer(&self, wanted: impl Fn(&CandidateIntent) -> bool) -> (PlayerId, PlayerStepV4) {
        let (player, request) = self.pending();
        let candidate = request
            .candidates
            .iter()
            .find(|candidate| wanted(&candidate.intent))
            .expect("no such candidate");
        let step = self
            .endpoint(player)
            .submit(DecisionResponseV3 {
                schema_version: DECISION_RESPONSE_V3_SCHEMA.to_owned(),
                player_decision_id: request.player_decision_id,
                view_sequence: request.view_sequence,
                answer: DecisionAnswerV2::SelectOne {
                    candidate_id: candidate.candidate_id,
                },
            })
            .unwrap();
        assert_eq!(step.submission, PlayerStepSubmissionV1::Accepted);
        (player, step)
    }

    fn core(&self) -> mtgml_state::CoreRulesState {
        self.controller.checkpoint().unwrap().state.core
    }
}

fn starting_player(player: PlayerId) -> impl Fn(&CandidateIntent) -> bool {
    move |intent| matches!(intent, CandidateIntent::SelectPlayer { player: p } if *p == player)
}

fn keep(intent: &CandidateIntent) -> bool {
    matches!(intent, CandidateIntent::ChooseBoolean { value: false })
}

fn pass(intent: &CandidateIntent) -> bool {
    matches!(intent, CandidateIntent::PassPriority)
}

/// `cards` lands, Mountains then Plains.
fn deck(mountains: usize, plains: usize) -> Vec<CardDefinitionId> {
    let (mountain, plain) = land_definitions();
    std::iter::repeat_n(mountain, mountains)
        .chain(std::iter::repeat_n(plain, plains))
        .collect()
}

#[test]
fn the_chooser_picks_and_both_players_observe_the_choice() {
    let game = Game::new([deck(6, 6), deck(6, 6)], 11);
    let (chooser, request) = game.pending();
    assert_eq!(request.purpose, DecisionPurposeV4::StartingPlayer);
    let other = if chooser == P1 { P2 } else { P1 };
    let (_, step) = game.answer(starting_player(other));
    let chosen = ObservedEventKindV4::StartingPlayerChosen {
        chooser,
        starting_player: other,
    };
    assert!(step
        .observed_events
        .iter()
        .any(|envelope| envelope.event == chosen));
    let (declarer, request) = game.pending();
    assert_eq!(declarer, other);
    assert_eq!(request.purpose, DecisionPurposeV4::MulliganDeclaration);
}

#[test]
fn both_players_keep_and_the_starting_player_takes_the_first_turn() {
    let game = Game::new([deck(6, 6), deck(6, 6)], 11);
    game.answer(starting_player(P2));
    let (first, step) = game.answer(keep);
    assert_eq!(first, P2);
    assert!(step.observed_events.iter().any(|envelope| envelope.event
        == ObservedEventKindV4::MulliganDeclared {
            player: P2,
            mulligan: false
        }));
    let (second, _) = game.answer(keep);
    assert_eq!(second, P1);
    let core = game.core();
    assert_eq!(core.turn_number, 1);
    assert_eq!(core.active_player, P2);
    assert_eq!(
        core.position,
        TurnPosition::Beginning {
            step: BeginningStep::Upkeep
        }
    );
    game.answer(pass);
    game.answer(pass);
    assert_eq!(game.core().position, TurnPosition::PrecombatMain);
    let hand = game.controller.checkpoint().unwrap().state;
    let cards_in_hand = hand
        .zones
        .locations
        .values()
        .filter(|location| {
            location.zone == mtgml_model::ZoneKind::Hand && location.player == Some(P2)
        })
        .count();
    assert_eq!(
        cards_in_hand, 7,
        "the starting player skips the turn-1 draw"
    );
}

#[test]
fn a_player_learns_nothing_about_the_opponents_deck_order() {
    // Same seed, same answers; only P2's deck order differs.
    let games = [
        Game::new([deck(6, 6), deck(6, 6)], 23),
        Game::new(
            [
                deck(6, 6),
                deck(0, 6).into_iter().chain(deck(6, 0)).collect(),
            ],
            23,
        ),
    ];
    let mut p1_views: [Vec<Vec<u8>>; 2] = [Vec::new(), Vec::new()];
    for (game, views) in games.iter().zip(&mut p1_views) {
        let (chooser, _) = game.pending();
        let mut record = |player: PlayerId, step: &PlayerStepV4| {
            if player == P1 {
                views.push(mtgml_wire::encode_canonical(step).unwrap());
            }
            views.push(
                mtgml_wire::encode_canonical(&game.endpoint(P1).information_state().unwrap())
                    .unwrap(),
            );
        };
        let (player, step) = game.answer(starting_player(chooser));
        record(player, &step);
        let (player, step) = game.answer(keep);
        record(player, &step);
        let (player, step) = game.answer(keep);
        record(player, &step);
    }
    assert_eq!(p1_views[0], p1_views[1]);
    // The games do differ: P2 holds different cards.
    let p2_states: Vec<_> = games
        .iter()
        .map(|game| {
            mtgml_wire::encode_canonical(&game.endpoint(P2).information_state().unwrap()).unwrap()
        })
        .collect();
    assert_ne!(p2_states[0], p2_states[1]);
}

#[test]
fn a_restored_pregame_checkpoint_continues_identically() {
    let game = Game::new([deck(6, 6), deck(6, 6)], 31);
    let start = game.controller.checkpoint().unwrap();
    game.answer(starting_player(P1));
    game.answer(keep);
    let ahead = game.controller.checkpoint().unwrap();
    game.controller.restore(start.clone()).unwrap();
    assert_eq!(game.controller.checkpoint().unwrap(), start);
    game.answer(starting_player(P1));
    game.answer(keep);
    assert_eq!(game.controller.checkpoint().unwrap(), ahead);
}

fn mulligan(intent: &CandidateIntent) -> bool {
    matches!(intent, CandidateIntent::ChooseBoolean { value: true })
}

impl Game {
    /// The bottoming player puts the candidates at `picks` on the bottom.
    fn bottom(&self, picks: &[usize]) -> (PlayerId, PlayerStepV4) {
        let (player, request) = self.pending();
        assert_eq!(request.purpose, DecisionPurposeV4::MulliganBottom);
        let step = self
            .endpoint(player)
            .submit(DecisionResponseV3 {
                schema_version: DECISION_RESPONSE_V3_SCHEMA.to_owned(),
                player_decision_id: request.player_decision_id,
                view_sequence: request.view_sequence,
                answer: DecisionAnswerV2::Order {
                    candidate_ids: picks
                        .iter()
                        .map(|index| request.candidates[*index].candidate_id)
                        .collect(),
                },
            })
            .unwrap();
        assert_eq!(step.submission, PlayerStepSubmissionV1::Accepted);
        (player, step)
    }
}

#[test]
fn a_mulligan_hides_the_hand_and_the_bottom_choice_from_the_opponent() {
    // Two games that differ only in which card P1 puts on the bottom.
    let mut p2_views: [Vec<Vec<u8>>; 2] = [Vec::new(), Vec::new()];
    let mut digests = Vec::new();
    for (pick, views) in [0_usize, 6].into_iter().zip(&mut p2_views) {
        let game = Game::new([deck(6, 6), deck(6, 6)], 41);
        let mut record = |game: &Game, player: PlayerId, step: &PlayerStepV4| {
            if player == P2 {
                views.push(mtgml_wire::encode_canonical(step).unwrap());
            }
            views.push(
                mtgml_wire::encode_canonical(&game.endpoint(P2).information_state().unwrap())
                    .unwrap(),
            );
        };
        let (player, step) = game.answer(starting_player(P1));
        record(&game, player, &step);
        let (player, step) = game.answer(mulligan);
        record(&game, player, &step);
        let (player, step) = game.answer(keep);
        record(&game, player, &step);
        // P1 saw its old hand leave for the library.
        let (_, bottom_request) = game.pending();
        assert_eq!(bottom_request.actor, P1);
        let (player, step) = game.bottom(&[pick]);
        record(&game, player, &step);
        let (player, step) = game.answer(keep);
        record(&game, player, &step);
        assert_eq!(game.core().turn_number, 1);
        digests.push(game.controller.checkpoint().unwrap().checkpoint_digest);
    }
    assert_eq!(p2_views[0], p2_views[1]);
    // The games do differ: a different card lies at the bottom.
    assert_ne!(digests[0], digests[1]);
}

#[test]
fn the_mulligan_owner_observes_its_cards_leave() {
    let game = Game::new([deck(6, 6), deck(6, 6)], 43);
    game.answer(starting_player(P2));
    game.answer(keep);
    let (_, step) = game.answer(mulligan);
    let left: Vec<_> = step
        .observed_events
        .iter()
        .filter(|envelope| {
            matches!(
                envelope.event,
                ObservedEventKindV4::ObjectMoved {
                    old_object: Some(_),
                    new_object: None,
                    from: mtgml_model::ZoneKind::Hand,
                    to: mtgml_model::ZoneKind::Library,
                    ..
                }
            )
        })
        .collect();
    assert_eq!(left.len(), 7, "the old hand went into the library");
}
