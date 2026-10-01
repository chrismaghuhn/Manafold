//! A permanent records the turn since which its controller controls it. Lands
//! enter by being played, through the production controller and the player
//! endpoints.

mod common;

use common::{two_player_land_game, P1, P2};
use mtgml_decision::{
    CandidateIntent, DecisionAnswerV2, DecisionPurposeV4, DecisionResponseV3,
    DECISION_RESPONSE_V3_SCHEMA,
};
use mtgml_environment::{PlayerEndpoint, PlayerEndpointHandle, TrustedEnvironmentController};
use mtgml_model::{GameObjectId, PlayerId, ZoneKind};
use mtgml_observation::PlayerStepSubmissionV1;
use mtgml_state::{EngineState, PermanentState};
use std::collections::BTreeMap;

struct Game {
    controller: TrustedEnvironmentController,
    players: [PlayerEndpointHandle; 2],
}

impl Game {
    fn new() -> Self {
        let controller = two_player_land_game(10, 3, 1);
        let players = [
            controller.bind_player(P1).unwrap(),
            controller.bind_player(P2).unwrap(),
        ];
        Self {
            controller,
            players,
        }
    }

    fn state(&self) -> EngineState {
        self.controller.checkpoint().unwrap().state
    }

    /// The deciding player plays a land if one is offered, declares no
    /// attackers and discards the first card if asked, and otherwise passes.
    fn respond(&self) {
        for player in &self.players {
            let Some(request) = player.visible_decision().unwrap() else {
                continue;
            };
            let pick = |wanted: fn(&CandidateIntent) -> bool| {
                request
                    .candidates
                    .iter()
                    .find(|candidate| wanted(&candidate.intent))
                    .map(|candidate| candidate.candidate_id)
            };
            let answer = match request.purpose {
                DecisionPurposeV4::AttackerDeclaration => DecisionAnswerV2::SelectMany {
                    candidate_ids: Vec::new(),
                },
                DecisionPurposeV4::HandSizeDiscard => DecisionAnswerV2::SelectMany {
                    candidate_ids: vec![request.candidates[0].candidate_id],
                },
                _ => DecisionAnswerV2::SelectOne {
                    candidate_id: pick(|intent| matches!(intent, CandidateIntent::PlayLand { .. }))
                        .or_else(|| pick(|intent| matches!(intent, CandidateIntent::PassPriority)))
                        .unwrap(),
                },
            };
            let step = player
                .submit(DecisionResponseV3 {
                    schema_version: DECISION_RESPONSE_V3_SCHEMA.to_owned(),
                    player_decision_id: request.player_decision_id,
                    view_sequence: request.view_sequence,
                    answer,
                })
                .unwrap();
            assert_eq!(step.submission, PlayerStepSubmissionV1::Accepted);
            return;
        }
        panic!("no player has a decision");
    }
}

/// Each battlefield object, with its controller.
fn battlefield(state: &EngineState) -> BTreeMap<GameObjectId, PlayerId> {
    state
        .zones
        .objects
        .values()
        .filter(|object| state.zones.locations[&object.id].zone == ZoneKind::Battlefield)
        .map(|object| (object.id, object.controller))
        .collect()
}

#[test]
fn a_land_played_on_turn_one_is_controlled_since_turn_one() {
    let game = Game::new();
    assert_eq!(game.state().core.turn_number, 1);
    assert!(game.state().card_rules.permanents.permanents.is_empty());

    // P1's first decision of the game is its first main phase: play the land.
    game.respond();

    let state = game.state();
    let lands = battlefield(&state);
    assert_eq!(lands.len(), 1);
    let (&land, &controller) = lands.iter().next().unwrap();
    assert_eq!(controller, P1);
    assert_eq!(
        state.card_rules.permanents.permanents,
        BTreeMap::from([(
            land,
            PermanentState {
                controlled_since_turn: 1
            }
        )])
    );
}

#[test]
fn each_land_records_the_turn_it_was_played_on() {
    let game = Game::new();
    // Turns 1 to 4 are over when turn 5 begins.
    while game.state().core.turn_number < 5 {
        game.respond();
    }

    let state = game.state();
    let mut turns_by_controller: BTreeMap<PlayerId, Vec<u64>> = BTreeMap::new();
    for (land, controller) in battlefield(&state) {
        turns_by_controller
            .entry(controller)
            .or_default()
            .push(state.card_rules.permanents.permanents[&land].controlled_since_turn);
    }
    for turns in turns_by_controller.values_mut() {
        turns.sort();
    }
    assert_eq!(
        turns_by_controller,
        BTreeMap::from([(P1, vec![1, 3]), (P2, vec![2, 4])])
    );
    // Exactly the battlefield objects have entries: lands that were drawn,
    // discarded or still in hand have none.
    assert_eq!(
        state
            .card_rules
            .permanents
            .permanents
            .keys()
            .copied()
            .collect::<Vec<_>>(),
        battlefield(&state).into_keys().collect::<Vec<_>>()
    );
}
