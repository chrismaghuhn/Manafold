//! Each player's observation shows the public player state — life, hand and
//! library sizes — and the tapped permanents, through the production
//! controller and the player endpoints.

mod common;

use common::{deck_game, land_definitions, P1, P2};
use mtgml_decision::{
    CandidateIntent, DecisionAnswerV2, DecisionPurposeV4, DecisionResponseV3,
    DECISION_RESPONSE_V3_SCHEMA,
};
use mtgml_environment::{PlayerEndpoint, PlayerEndpointHandle, TrustedEnvironmentController};
use mtgml_model::{CardDefinitionId, PlayerId};
use mtgml_observation::{
    MagicSharedExecutionObservationV1, PlayerObservationV1, PlayerStepSubmissionV1,
};
use mtgml_state::{BeginningStep, TurnPosition};

struct Game {
    controller: TrustedEnvironmentController,
    players: [PlayerEndpointHandle; 2],
}

impl Game {
    fn new() -> Self {
        let (mountain, plains) = land_definitions();
        let deck = || {
            std::iter::repeat_n(mountain, 6)
                .chain(std::iter::repeat_n(plains, 6))
                .collect::<Vec<CardDefinitionId>>()
        };
        let controller = deck_game([deck(), deck()], 11);
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

    /// The deciding player answers with the first candidate `wanted`
    /// accepts, or with the first `fallback` candidate.
    fn answer(
        &self,
        wanted: impl Fn(&CandidateIntent) -> bool,
        fallback: impl Fn(&CandidateIntent) -> bool,
    ) {
        let (player, request) = self
            .players
            .iter()
            .find_map(|player| {
                player
                    .visible_decision()
                    .unwrap()
                    .map(|request| (player.perspective(), request))
            })
            .expect("no player has a decision");
        let answer = if request.purpose == DecisionPurposeV4::MulliganBottom {
            DecisionAnswerV2::Order {
                candidate_ids: vec![request.candidates[0].candidate_id],
            }
        } else if request.purpose == DecisionPurposeV4::AttackerDeclaration {
            // Lands do not attack.
            DecisionAnswerV2::SelectMany {
                candidate_ids: Vec::new(),
            }
        } else {
            let candidate = request
                .candidates
                .iter()
                .find(|candidate| wanted(&candidate.intent))
                .or_else(|| {
                    request
                        .candidates
                        .iter()
                        .find(|candidate| fallback(&candidate.intent))
                })
                .unwrap_or_else(|| panic!("no such candidate for {:?}", request.purpose));
            DecisionAnswerV2::SelectOne {
                candidate_id: candidate.candidate_id,
            }
        };
        let step = self
            .endpoint(player)
            .submit(DecisionResponseV3 {
                schema_version: DECISION_RESPONSE_V3_SCHEMA.to_owned(),
                player_decision_id: request.player_decision_id,
                view_sequence: request.view_sequence,
                answer,
            })
            .unwrap();
        assert_eq!(step.submission, PlayerStepSubmissionV1::Accepted);
    }

    fn observation(&self, player: PlayerId) -> MagicSharedExecutionObservationV1 {
        let information = self.endpoint(player).information_state().unwrap();
        let payload = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            &information.current_observation.payload_base64,
        )
        .unwrap();
        mtgml_wire::decode_canonical(&payload).unwrap()
    }

    /// Both players see the same player rows.
    fn players_seen(&self) -> Vec<PlayerObservationV1> {
        let seen = self.observation(P1).players;
        assert_eq!(self.observation(P2).players, seen);
        seen
    }

    fn position(&self) -> (u64, TurnPosition) {
        let core = self.controller.checkpoint().unwrap().state.core;
        (core.turn_number, core.position)
    }
}

fn row(player: PlayerId, hand_count: u32, library_count: u32) -> PlayerObservationV1 {
    PlayerObservationV1 {
        player,
        life: 20,
        hand_count,
        library_count,
    }
}

fn starting_player(player: PlayerId) -> impl Fn(&CandidateIntent) -> bool {
    move |intent| matches!(intent, CandidateIntent::SelectPlayer { player: p } if *p == player)
}

fn keep(intent: &CandidateIntent) -> bool {
    matches!(intent, CandidateIntent::ChooseBoolean { value: false })
}

fn mulligan(intent: &CandidateIntent) -> bool {
    matches!(intent, CandidateIntent::ChooseBoolean { value: true })
}

fn pass(intent: &CandidateIntent) -> bool {
    matches!(intent, CandidateIntent::PassPriority)
}

fn play_land(intent: &CandidateIntent) -> bool {
    matches!(intent, CandidateIntent::PlayLand { .. })
}

fn tap_for_mana(intent: &CandidateIntent) -> bool {
    matches!(intent, CandidateIntent::ActivateAbility { .. })
}

#[test]
fn both_players_see_life_hand_and_library_sizes_from_the_start() {
    let game = Game::new();
    assert_eq!(game.players_seen(), vec![row(P1, 0, 12), row(P2, 0, 12)]);
    game.answer(starting_player(P2), pass);
    assert_eq!(game.players_seen(), vec![row(P1, 7, 5), row(P2, 7, 5)]);
    game.answer(keep, pass);
    game.answer(keep, pass);
    assert_eq!(game.players_seen(), vec![row(P1, 7, 5), row(P2, 7, 5)]);
}

#[test]
fn the_opponent_sees_a_mulligan_change_the_hand_size() {
    let game = Game::new();
    game.answer(starting_player(P1), pass);
    game.answer(mulligan, pass);
    game.answer(keep, pass);
    // P1 drew a new seven and has not yet put one card on the bottom.
    assert_eq!(game.players_seen(), vec![row(P1, 7, 5), row(P2, 7, 5)]);
    game.answer(pass, pass);
    assert_eq!(game.players_seen(), vec![row(P1, 6, 6), row(P2, 7, 5)]);
}

#[test]
fn a_land_tapped_for_mana_is_tapped_for_both_players_until_it_untaps() {
    let game = Game::new();
    game.answer(starting_player(P2), pass);
    game.answer(keep, pass);
    game.answer(keep, pass);
    while game.position() != (1, TurnPosition::PrecombatMain) {
        game.answer(pass, pass);
    }
    game.answer(play_land, pass);
    assert!(game.observation(P1).tapped.is_empty());
    game.answer(tap_for_mana, pass);
    // Each player sees the land under their own opaque id.
    let state = game.controller.checkpoint().unwrap().state;
    let mut on_battlefield = state
        .zones
        .locations
        .iter()
        .filter(|(_, location)| location.zone == mtgml_model::ZoneKind::Battlefield)
        .map(|(object, _)| *object);
    let land = on_battlefield.next().unwrap();
    assert!(on_battlefield.next().is_none());
    for player in [P1, P2] {
        let own = state.perspective_identities.players[&player].object_to_opaque[&land];
        assert_eq!(game.observation(player).tapped, vec![own], "{player:?}");
    }
    assert_eq!(game.players_seen(), vec![row(P1, 7, 5), row(P2, 6, 5)]);

    // P1 plays a land on turn 2 and keeps seven cards; P2 untaps on turn 3.
    let upkeep = TurnPosition::Beginning {
        step: BeginningStep::Upkeep,
    };
    while game.position() != (3, upkeep) {
        game.answer(play_land, pass);
    }
    assert!(game.observation(P1).tapped.is_empty());
    assert!(game.observation(P2).tapped.is_empty());
    assert_eq!(game.players_seen(), vec![row(P1, 7, 4), row(P2, 6, 5)]);
}
