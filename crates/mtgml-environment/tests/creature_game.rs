//! A deck with vanilla creatures starts a game through the production
//! controller and the player endpoints. Creatures sit in hand and library:
//! nothing casts them yet.

mod common;

use common::{creature_deck_game, creature_definitions, game_admission, land_definitions, P1, P2};
use mtgml_decision::{
    CandidateIntent, DecisionAnswerV2, DecisionPurposeV4, DecisionResponseV3,
    DECISION_RESPONSE_V3_SCHEMA,
};
use mtgml_environment::{PlayerEndpoint, PlayerEndpointHandle, TrustedEnvironmentController};
use mtgml_model::{CardDefinitionId, PlayerId, ZoneKind};
use mtgml_observation::PlayerStepSubmissionV1;

struct Game {
    controller: TrustedEnvironmentController,
    players: [PlayerEndpointHandle; 2],
}

impl Game {
    fn new(decks: [Vec<CardDefinitionId>; 2], seed: u64) -> Self {
        let controller = creature_deck_game(decks, seed);
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
    /// accepts, or with the first `fallback` candidate. Every candidate the
    /// player is offered is checked on the way: no creature can be cast yet.
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
        assert!(
            !request
                .candidates
                .iter()
                .any(|candidate| matches!(candidate.intent, CandidateIntent::CastSpell { .. })),
            "a creature spell is offered before casting exists"
        );
        let answer = if request.purpose == DecisionPurposeV4::MulliganBottom {
            DecisionAnswerV2::Order {
                candidate_ids: vec![request.candidates[0].candidate_id],
            }
        } else if request.purpose == DecisionPurposeV4::AttackerDeclaration {
            // Nothing can attack: no creature is on the battlefield.
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

    fn state(&self) -> mtgml_state::EngineState {
        self.controller.checkpoint().unwrap().state
    }

    /// The zone of each of `player`'s cards of `definitions`.
    fn zones_of(&self, player: PlayerId, definitions: &[CardDefinitionId]) -> Vec<ZoneKind> {
        let state = self.state();
        state
            .zones
            .objects
            .values()
            .filter(|object| {
                object.owner == player && definitions.contains(&object.card_definition)
            })
            .map(|object| state.zones.locations[&object.id].zone)
            .collect()
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

fn play_land(intent: &CandidateIntent) -> bool {
    matches!(intent, CandidateIntent::PlayLand { .. })
}

/// 20 Mountains and Plains.
fn lands() -> Vec<CardDefinitionId> {
    let (mountain, plains) = land_definitions();
    std::iter::repeat_n(mountain, 10)
        .chain(std::iter::repeat_n(plains, 10))
        .collect()
}

/// 20 lands and 7 creatures: Savannah Lions, Gray Ogre and Hill Giant twice,
/// and one more Savannah Lions.
fn creature_deck() -> Vec<CardDefinitionId> {
    let creatures = creature_definitions();
    let mut deck = lands();
    deck.extend(creatures);
    deck.extend(&creatures[..3]);
    deck.push(creatures[0]);
    deck
}

#[test]
fn creature_definitions_are_the_three_vanilla_creatures() {
    assert_eq!(
        creature_definitions(),
        [
            CardDefinitionId(3),
            CardDefinitionId(4),
            CardDefinitionId(5)
        ]
    );
}

#[test]
fn a_deck_with_creatures_starts_a_game() {
    let creatures = creature_definitions();
    let mut opponent = lands();
    opponent.extend(lands().into_iter().take(7));
    let game = Game::new([creature_deck(), opponent], 6);
    assert_eq!(creature_deck().len(), 27);

    game.answer(starting_player(P1), pass);
    game.answer(keep, pass);
    game.answer(keep, pass);

    // All seven creatures are in P1's hand or library, and the game began
    // with a hand that holds at least one of them.
    let in_zones = game.zones_of(P1, &creatures);
    assert_eq!(in_zones.len(), 7);
    assert!(in_zones
        .iter()
        .all(|zone| matches!(zone, ZoneKind::Hand | ZoneKind::Library)));
    assert!(
        in_zones.contains(&ZoneKind::Hand),
        "pick a seed whose opening hand holds a creature"
    );
    assert_eq!(game.state().core.turn_number, 1);

    // Turn 1 runs to its end with land plays only: creature cards are never
    // offered as plays, and they stay where they were.
    while game.state().core.turn_number == 1 {
        game.answer(play_land, pass);
        assert!(game
            .zones_of(P1, &creatures)
            .iter()
            .all(|zone| matches!(zone, ZoneKind::Hand | ZoneKind::Library)));
    }
    let core = game.state().core;
    assert_eq!(core.turn_number, 2);
    assert_eq!(core.active_player, P2);
    let on_battlefield = game
        .state()
        .zones
        .locations
        .values()
        .filter(|location| location.zone == ZoneKind::Battlefield)
        .count();
    assert_eq!(on_battlefield, 1, "P1 played one land and nothing else");
}

#[test]
fn a_creature_under_the_land_only_admission_is_refused() {
    let deck_with_creature = {
        let mut deck = lands();
        deck.push(creature_definitions()[0]);
        deck
    };
    let outcome = mtgml_rules::start_game(
        &game_admission(),
        [(P1, deck_with_creature), (P2, lands())],
        mtgml_random::RootSeed256([0; 32]),
    );
    assert_eq!(outcome, Err(mtgml_rules::GameStartError::UnknownDefinition));
}
