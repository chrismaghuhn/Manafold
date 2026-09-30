#![cfg(not(feature = "historical-conformance-runtime"))]
//! Complete turns through the production V8 controller and player endpoints.

mod common;

use common::{land_definitions, land_game, two_player_land_game, P1, P2};
use mtgml_decision::{
    CandidateIntentV4, DecisionAnswerV2, DecisionPurposeV4, DecisionResponseV3,
    PlayerDecisionRequestV4, DECISION_RESPONSE_V3_SCHEMA,
};
use mtgml_environment::{
    PlayerEndpoint, PlayerEndpointError, PlayerEndpointHandle, TrustedEnvironmentController,
};
use mtgml_model::{EpisodeStatus, PlayerId, PlayerOutcome, PlayerResult, TerminalReason, ZoneKind};
use mtgml_observation::{PlayerStepSubmissionV1, PlayerSubmissionCodeV1};
use mtgml_state::{BeginningStep, TurnPosition};

const UPKEEP: TurnPosition = TurnPosition::Beginning {
    step: BeginningStep::Upkeep,
};

struct Game {
    controller: TrustedEnvironmentController,
    players: [PlayerEndpointHandle; 2],
}

impl Game {
    fn new(controller: TrustedEnvironmentController) -> Self {
        let players = [
            controller.bind_player(P1).unwrap(),
            controller.bind_player(P2).unwrap(),
        ];
        Self {
            controller,
            players,
        }
    }

    fn core(&self) -> mtgml_state::CoreRulesState {
        self.controller
            .checkpoint()
            .unwrap()
            .state
            .predecessor_v5
            .core
    }

    fn at(&self, position: TurnPosition, turn: u64) -> bool {
        let core = self.core();
        core.position == position && core.turn_number == turn
    }

    /// The deciding player answers with the scripted policy: play the first
    /// offered land if `play_lands`, declare no attackers, discard the first
    /// card, otherwise pass. Uses only what that player sees.
    fn respond(
        &self,
        play_lands: bool,
    ) -> Result<(PlayerId, PlayerDecisionRequestV4), PlayerEndpointError> {
        for player in &self.players {
            let Some(request) = player.visible_decision()? else {
                continue;
            };
            let answer = scripted_answer(&request, play_lands);
            let step = player.submit(DecisionResponseV3 {
                schema_version: DECISION_RESPONSE_V3_SCHEMA.to_owned(),
                player_decision_id: request.player_decision_id,
                view_sequence: request.view_sequence,
                answer,
            })?;
            assert_eq!(step.submission, PlayerStepSubmissionV1::Accepted);
            return Ok((player.perspective(), request));
        }
        panic!("no player has a decision");
    }

    fn respond_until(&self, play_lands: bool, until: impl Fn(&Self) -> bool) {
        for _ in 0..2_000 {
            if until(self) {
                return;
            }
            self.respond(play_lands).unwrap();
        }
        panic!("condition not reached");
    }

    /// Cards in `player`'s `zone`; battlefield locations carry no player, so
    /// permanents count by controller.
    fn zone_count(&self, player: PlayerId, zone: ZoneKind) -> usize {
        let state = self.controller.checkpoint().unwrap().state.predecessor_v5;
        state
            .zones
            .objects
            .values()
            .filter(|object| {
                let location = &state.zones.locations[&object.id];
                location.zone == zone
                    && if zone == ZoneKind::Battlefield {
                        object.controller == player
                    } else {
                        location.player == Some(player)
                    }
            })
            .count()
    }
}

fn scripted_answer(request: &PlayerDecisionRequestV4, play_lands: bool) -> DecisionAnswerV2 {
    match request.purpose {
        DecisionPurposeV4::AttackerDeclaration => DecisionAnswerV2::SelectMany {
            candidate_ids: Vec::new(),
        },
        DecisionPurposeV4::HandSizeDiscard => DecisionAnswerV2::SelectMany {
            candidate_ids: vec![request.candidates[0].candidate_id],
        },
        _ => {
            let pick = |wanted: fn(&CandidateIntentV4) -> bool| {
                request
                    .candidates
                    .iter()
                    .find(|candidate| wanted(&candidate.intent))
                    .map(|candidate| candidate.candidate_id)
            };
            let land = if play_lands {
                pick(|intent| matches!(intent, CandidateIntentV4::PlayLand { .. }))
            } else {
                None
            };
            DecisionAnswerV2::SelectOne {
                candidate_id: land
                    .or_else(|| pick(|intent| matches!(intent, CandidateIntentV4::PassPriority)))
                    .unwrap(),
            }
        }
    }
}

#[test]
fn two_players_complete_a_turn_and_reach_the_next_upkeep() {
    let game = Game::new(two_player_land_game(10, 3, 1));
    game.respond_until(true, |game| game.at(UPKEEP, 2));

    let core = game.core();
    assert_eq!(core.active_player, P2);
    assert!(game.players[1].visible_decision().unwrap().is_some());
    assert!(game.players[0].visible_decision().unwrap().is_none());
}

#[test]
fn ten_turns_of_land_drops_are_deterministic() {
    let game = Game::new(two_player_land_game(10, 3, 1));
    game.respond_until(true, |game| game.at(UPKEEP, 4));
    let fork = Game::new(game.controller.fork().unwrap());
    let turn_four = game.controller.checkpoint().unwrap();

    game.respond_until(true, |game| game.at(UPKEEP, 11));
    let last = game.controller.checkpoint().unwrap();
    assert_eq!(game.zone_count(P1, ZoneKind::Battlefield), 5);
    assert_eq!(game.zone_count(P2, ZoneKind::Battlefield), 5);

    let report = game
        .controller
        .execute_replay(game.controller.export_replay().unwrap())
        .unwrap();
    assert_eq!(report.final_checkpoint, last);

    fork.respond_until(true, |game| game.at(UPKEEP, 11));
    assert_eq!(fork.controller.checkpoint().unwrap(), last);

    let restored = Game::new(two_player_land_game(10, 3, 1));
    restored.controller.restore(turn_four).unwrap();
    restored.respond_until(true, |game| game.at(UPKEEP, 11));
    let restored_last = restored.controller.checkpoint().unwrap();
    assert_eq!(restored_last.state_digest, last.state_digest);
    assert_eq!(restored_last.checkpoint_digest, last.checkpoint_digest);
}

#[test]
fn opponent_never_learns_drawn_cards_end_to_end() {
    let (mountain, plains) = land_definitions();
    let hands = [vec![mountain; 3], vec![plains; 3]];
    let games = [
        Game::new(land_game(&[vec![mountain; 6], vec![plains; 6]], &hands, 7)),
        Game::new(land_game(&[vec![plains; 6], vec![plains; 6]], &hands, 7)),
    ];
    let mut responses = 0;
    while games[0].core().turn_number < 8 {
        for game in &games {
            game.respond(false).unwrap();
        }
        responses += 1;
        assert_eq!(
            games[0].players[1].information_state().unwrap(),
            games[1].players[1].information_state().unwrap(),
            "P2's knowledge differs after response {responses}"
        );
    }
    assert_ne!(
        games[0].players[0].information_state().unwrap(),
        games[1].players[0].information_state().unwrap(),
        "P1 does see its own draws"
    );
}

#[test]
fn drawing_from_an_empty_library_ends_the_game() {
    // Both libraries are empty: P1 skips the turn-1 draw, P2 would draw on
    // turn 2 and loses (CR 121.4, 704.5b); P1 wins (CR 104.2a).
    let game = Game::new(two_player_land_game(0, 3, 1));
    let mut last = None;
    while game.controller.checkpoint().unwrap().status == EpisodeStatus::Running {
        last = Some(game.respond(false).unwrap());
    }
    let checkpoint = game.controller.checkpoint().unwrap();
    assert_eq!(
        checkpoint.status,
        EpisodeStatus::Terminal {
            reason: TerminalReason::RulesLoss,
            players: vec![
                PlayerOutcome {
                    player: P1,
                    result: PlayerResult::Win,
                },
                PlayerOutcome {
                    player: P2,
                    result: PlayerResult::Loss,
                },
            ],
        }
    );
    assert_eq!(game.core().turn_number, 2);
    for player in &game.players {
        assert_eq!(player.visible_decision().unwrap(), None);
        player.information_state().unwrap();
    }

    let (actor, request) = last.unwrap();
    let late = DecisionResponseV3 {
        schema_version: DECISION_RESPONSE_V3_SCHEMA.to_owned(),
        player_decision_id: request.player_decision_id,
        view_sequence: request.view_sequence,
        answer: DecisionAnswerV2::SelectOne {
            candidate_id: request.candidates[0].candidate_id,
        },
    };
    let player = game
        .players
        .iter()
        .find(|p| p.perspective() == actor)
        .unwrap();
    assert_eq!(
        player.submit(late).unwrap().submission,
        PlayerStepSubmissionV1::Rejected {
            code: PlayerSubmissionCodeV1::EpisodeClosed,
        }
    );
    assert_eq!(game.controller.checkpoint().unwrap(), checkpoint);

    let report = game
        .controller
        .execute_replay(game.controller.export_replay().unwrap())
        .unwrap();
    assert_eq!(report.final_checkpoint, checkpoint);
}

#[test]
fn checkpoints_stay_bound_to_their_state_digest() {
    let game = Game::new(two_player_land_game(10, 3, 1));
    let admission = common::game_admission();
    while !game.at(UPKEEP, 3) {
        game.respond(true).unwrap();
        let checkpoint = game.controller.checkpoint().unwrap();
        assert_eq!(
            checkpoint.state_digest,
            mtgml_state::calculate_full_state_digest_v7_structural_only(&checkpoint.state).unwrap()
        );
        assert!(checkpoint
            .validate_for_basic_land_profile(&admission)
            .is_ok());
    }
}

#[test]
fn a_game_cannot_start_with_hands_the_slice_cannot_reach() {
    // P1 is active in the first main phase and may hold eight; P2 may hold
    // seven. One card more would need several discards at some cleanup.
    let (mountain, _) = land_definitions();
    let hand = |cards: usize| vec![mountain; cards];
    let libraries = [hand(3), hand(3)];
    assert!(common::try_land_game(&libraries, &[hand(8), hand(7)], 1).is_ok());
    assert!(common::try_land_game(&libraries, &[hand(9), hand(7)], 1).is_err());
    assert!(common::try_land_game(&libraries, &[hand(8), hand(8)], 1).is_err());
}

#[test]
fn hand_size_discard_through_the_player_endpoint() {
    // Both players hold seven and never play a land: P2 draws to eight on
    // turn 2 and P1 on turn 3; each discards at their own cleanup.
    let game = Game::new(two_player_land_game(10, 7, 3));
    let mut discards = Vec::new();
    while game.core().turn_number < 4 {
        let turn = game.core().turn_number;
        let (player, request) = game.respond(false).unwrap();
        if request.purpose == DecisionPurposeV4::HandSizeDiscard {
            discards.push((turn, player));
        }
    }
    assert_eq!(discards, vec![(2, P2), (3, P1)]);
    for player in [P1, P2] {
        assert_eq!(game.zone_count(player, ZoneKind::Hand), 7);
        assert_eq!(game.zone_count(player, ZoneKind::Graveyard), 1);
    }
}
