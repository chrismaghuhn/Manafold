//! A deck with vanilla creatures starts a game, and its creatures are cast
//! and resolve, through the production controller and the player endpoints.

mod common;

use common::{
    creature_deck_game, creature_definitions, creature_game, creature_game_admission,
    creature_game_with_life, game_admission, land_definitions, P1, P2,
};
use mtgml_decision::{
    CandidateIntent, DecisionAnswerV2, DecisionDomainV2, DecisionPurposeV4, DecisionResponseV3,
    DecisionVisibility, PlayerDecisionRequestV4, DECISION_RESPONSE_V3_SCHEMA,
};
use mtgml_environment::{
    CheckpointV8Error, EnvironmentCheckpointV8, PlayerEndpoint, PlayerEndpointError,
    PlayerEndpointHandle, TrustedEnvironmentController,
};
use mtgml_model::{
    CandidateIdV1, CardDefinitionId, EpisodeStatus, GameObjectId, OpaqueObjectId,
    PlayerDecisionIdV1, PlayerId, PlayerOutcome, PlayerResult, TerminalReason, TruncationReason,
    ZoneKind,
};
use mtgml_observation::{
    MagicSharedExecutionObservationV1, ObservedEventEnvelopeV4, ObservedEventKindV4,
    PermanentObservationV1, PlayerStepSubmissionV1, PlayerStepV4, PublicStackItemV1,
    StackItemRemovalCauseV1,
};
use mtgml_rules::{AuthoritativeRuleEventKind, BasicLandTransitionProduct};
use mtgml_state::{
    CastContinuationStage, ContinuationPayload, DeltaApplicationError, EngineState, GameObject,
    ManaPaymentStage, ManaPaymentStaging, ManaPoolChangeCauseV1, ManaPoolV1, PriorityState,
    SemanticDeltaOperation, StackItemPayload, StackRecord, StateDelta, TurnPosition,
};
use std::collections::BTreeMap;

struct Game {
    controller: TrustedEnvironmentController,
    players: [PlayerEndpointHandle; 2],
}

impl Game {
    fn new(decks: [Vec<CardDefinitionId>; 2], seed: u64) -> Self {
        Self::bound(creature_deck_game(decks, seed))
    }

    /// A game at P1's first precombat main with the given hands (P1's first)
    /// and ten Mountains in each library.
    fn with_hands(hands: [Vec<CardDefinitionId>; 2]) -> Self {
        let (mountain, _) = land_definitions();
        let libraries = [vec![mountain; 10], vec![mountain; 10]];
        Self::bound(creature_game(&libraries, &hands, 5))
    }

    /// As `with_hands`, with the players at the given life totals.
    fn with_hands_and_life(hands: [Vec<CardDefinitionId>; 2], life: [i64; 2]) -> Self {
        let (mountain, _) = land_definitions();
        let libraries = [vec![mountain; 10], vec![mountain; 10]];
        Self::bound(creature_game_with_life(&libraries, &hands, 5, life))
    }

    fn bound(controller: TrustedEnvironmentController) -> Self {
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

    /// Whether `player` knows which card `definition` an object is.
    fn knows(&self, player: PlayerId, definition: CardDefinitionId) -> bool {
        self.endpoint(player)
            .information_state()
            .unwrap()
            .retained_knowledge
            .iter()
            .any(|known| {
                matches!(
                    known,
                    mtgml_observation::PlayerKnownObjectV1::Active {
                        known_definition: Some(known_definition),
                        ..
                    } if *known_definition == definition
                )
            })
    }

    /// What the deciding player is offered.
    fn offered(&self) -> Vec<CandidateIntent> {
        self.pending()
            .1
            .candidates
            .into_iter()
            .map(|candidate| candidate.intent)
            .collect()
    }

    /// The cards the deciding player is offered to cast.
    fn casts(&self) -> Vec<OpaqueObjectId> {
        self.offered()
            .into_iter()
            .filter_map(|intent| match intent {
                CandidateIntent::CastSpell { object } => Some(object),
                _ => None,
            })
            .collect()
    }

    /// The deciding player answers with the first candidate `wanted`
    /// accepts, or with the first `fallback` candidate. The step is that
    /// player's.
    fn answer(
        &self,
        wanted: impl Fn(&CandidateIntent) -> bool,
        fallback: impl Fn(&CandidateIntent) -> bool,
    ) -> (PlayerId, PlayerStepV4) {
        let (player, request) = self.pending();
        let answer = if request.purpose == DecisionPurposeV4::MulliganBottom {
            DecisionAnswerV2::Order {
                candidate_ids: vec![request.candidates[0].candidate_id],
            }
        } else if request.purpose == DecisionPurposeV4::AttackerDeclaration {
            // Declares no attackers; `declare_attackers` attacks.
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
        (player, step)
    }

    /// The deciding player declares exactly `attackers`, which the request
    /// must offer.
    fn declare_attackers(&self, attackers: &[OpaqueObjectId]) -> (PlayerId, PlayerStepV4) {
        let (player, request) = self.pending();
        assert_eq!(request.purpose, DecisionPurposeV4::AttackerDeclaration);
        let candidate_ids: Vec<_> = request
            .candidates
            .iter()
            .filter(|candidate| {
                matches!(candidate.intent,
                    CandidateIntent::SelectObject { object } if attackers.contains(&object))
            })
            .map(|candidate| candidate.candidate_id)
            .collect();
        assert_eq!(candidate_ids.len(), attackers.len(), "{attackers:?}");
        let step = self
            .endpoint(player)
            .submit(DecisionResponseV3 {
                schema_version: DECISION_RESPONSE_V3_SCHEMA.to_owned(),
                player_decision_id: request.player_decision_id,
                view_sequence: request.view_sequence,
                answer: DecisionAnswerV2::SelectMany { candidate_ids },
            })
            .unwrap();
        assert_eq!(step.submission, PlayerStepSubmissionV1::Accepted);
        (player, step)
    }

    /// Plays lands and passes until `reached` holds.
    fn run_until(&self, reached: impl Fn(&EngineState) -> bool) {
        let mut answers = 0;
        while !reached(&self.state()) {
            self.answer(play_land, pass);
            answers += 1;
            assert!(answers < 500, "the game never reached the wanted state");
        }
    }

    fn state(&self) -> EngineState {
        self.controller.checkpoint().unwrap().state
    }

    /// As `answer`, and the checkpoint the answer leads to restores: it is a
    /// state the game may be restored in.
    fn answer_restorable(
        &self,
        wanted: impl Fn(&CandidateIntent) -> bool,
        fallback: impl Fn(&CandidateIntent) -> bool,
    ) {
        self.answer(wanted, fallback);
        self.assert_restorable();
    }

    /// The game's checkpoint is one that a restore accepts.
    fn assert_restorable(&self) {
        let reached = self.controller.checkpoint().unwrap();
        let state = &reached.state;
        let restored = restored(&reached, state.clone());
        assert!(
            restored.is_ok(),
            "a state the game reached is refused on restore: {restored:?}\nturn {} {:?} {:?}, stack {:?}, request {:?}",
            state.core.turn_number,
            state.core.position,
            state.core.priority,
            state.zones.stack_order,
            state.execution.pending_decision.as_ref().map(|request| &request.purpose),
        );
    }

    fn observation(&self, player: PlayerId) -> MagicSharedExecutionObservationV1 {
        mtgml_wire::decode_canonical(&self.observation_payload(player)).unwrap()
    }

    /// The bytes of the observation `player` receives.
    fn observation_payload(&self, player: PlayerId) -> Vec<u8> {
        let information = self.endpoint(player).information_state().unwrap();
        base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            &information.current_observation.payload_base64,
        )
        .unwrap()
    }

    /// The only object of `owner` made from `definition` that is in `zone`.
    fn only_object(
        &self,
        owner: PlayerId,
        definition: CardDefinitionId,
        zone: ZoneKind,
    ) -> GameObjectId {
        let state = self.state();
        let mut found = state.zones.objects.values().filter(|object| {
            object.owner == owner
                && object.card_definition == definition
                && state.zones.locations[&object.id].zone == zone
        });
        let object = found.next().expect("no such object").id;
        assert!(found.next().is_none(), "more than one such object");
        object
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

fn tap_for_mana(intent: &CandidateIntent) -> bool {
    matches!(intent, CandidateIntent::ActivateAbility { .. })
}

fn cast_spell(intent: &CandidateIntent) -> bool {
    matches!(intent, CandidateIntent::CastSpell { .. })
}

/// Restoring `state` in place of the game that reached `reached`: the same
/// admission, status and counters, and the checkpoint a restore builds.
fn restored(
    reached: &EnvironmentCheckpointV8,
    state: EngineState,
) -> Result<EnvironmentCheckpointV8, CheckpointV8Error> {
    EnvironmentCheckpointV8::new_for_basic_land_profile(
        &creature_game_admission(),
        state,
        reached.status.clone(),
        reached.limit_counters.clone(),
        reached.execution_identity.clone(),
    )
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

/// The first main phase of `turn`, with its active player holding priority
/// and no action taken yet.
fn start_of_main_phase(turn: u64) -> impl Fn(&EngineState) -> bool {
    move |state| {
        state.core.turn_number == turn
            && state.core.position == TurnPosition::PrecombatMain
            && state.core.priority
                == (PriorityState::HeldBy {
                    player: state.core.active_player,
                    consecutive_passes: 0,
                })
    }
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

    // Turn 1 runs to its end with land plays only: no mana is made, so no
    // creature can be paid for, and the creature cards stay where they were.
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

#[test]
fn a_creature_is_cast_and_resolves_onto_the_battlefield() {
    let (mountain, plains) = land_definitions();
    let [lions, _, giant] = creature_definitions();
    let game = Game::with_hands([vec![plains, lions, giant], vec![mountain]]);
    assert_eq!(game.state().core.turn_number, 1);

    // Turn 1: P1 plays the Plains and taps it for {W}. Savannah Lions costs
    // {W}; Hill Giant costs {3}{R} and is not offered.
    game.answer(play_land, pass);
    game.answer(tap_for_mana, pass);
    let lions_card = game.only_object(P1, lions, ZoneKind::Hand);
    let own = game.state().perspective_identities.players[&P1].object_to_opaque[&lions_card];
    assert_eq!(game.casts(), vec![own]);
    // The request names a hidden card, so only its actor receives it.
    assert_eq!(
        game.pending().1.visibility,
        DecisionVisibility::ActingPlayerOnly
    );
    let (caster, step) = game.answer(cast_spell, pass);
    assert_eq!(caster, P1);

    // The spell is on the stack, in public, for both players.
    assert!(step.observed_events.iter().any(|envelope| matches!(
        &envelope.event,
        ObservedEventKindV4::StackItemAdded {
            item: PublicStackItemV1::Spell { controller, .. },
            ..
        } if *controller == P1
    )));
    let state = game.state();
    let spell = game.only_object(P1, lions, ZoneKind::Stack);
    assert_eq!(state.zones.stack_order.len(), 1);
    for player in [P1, P2] {
        let opaque = state.perspective_identities.players[&player].object_to_opaque[&spell];
        match game.observation(player).stack.as_slice() {
            [PublicStackItemV1::Spell {
                controller,
                card_object,
                ..
            }] => {
                assert_eq!(*controller, P1);
                assert_eq!(*card_object, opaque, "{player:?}");
            }
            other => panic!("{player:?} sees {other:?}"),
        }
    }
    // The cost left the pool, the cast is on record, and the caster has
    // priority again (CR 117.3c).
    assert_eq!(
        state.card_rules.mana.pools[&P1],
        mtgml_state::ManaPoolV1::default()
    );
    assert_eq!(
        state.card_rules.turn_history.players[&P1].spells_cast_total,
        1
    );
    assert_eq!(
        state.core.priority,
        PriorityState::HeldBy {
            player: P1,
            consecutive_passes: 0
        }
    );
    assert_eq!(game.pending().0, P1);

    // Both pass: the spell resolves, and the step has not changed.
    game.answer(pass, pass);
    assert_eq!(game.pending().0, P2, "P2 receives priority after P1 passes");
    assert_eq!(game.state().zones.stack_order.len(), 1);
    let (second, step) = game.answer(pass, pass);
    assert_eq!(second, P2);
    assert!(step.observed_events.iter().any(|envelope| matches!(
        &envelope.event,
        ObservedEventKindV4::StackItemRemoved {
            cause: StackItemRemovalCauseV1::Resolved,
            ..
        }
    )));
    assert!(step.observed_events.iter().any(|envelope| matches!(
        &envelope.event,
        ObservedEventKindV4::ObjectMoved {
            from: ZoneKind::Stack,
            to: ZoneKind::Battlefield,
            ..
        }
    )));
    let state = game.state();
    assert!(state.zones.stack_order.is_empty() && state.zones.stack_records.is_empty());
    let creature = game.only_object(P1, lions, ZoneKind::Battlefield);
    assert_eq!(state.zones.objects[&creature].controller, P1);
    assert_eq!(
        state.card_rules.permanents.permanents[&creature].controlled_since_turn,
        state.core.turn_number
    );
    assert_eq!(state.core.turn_number, 1);
    assert_eq!(state.core.position, TurnPosition::PrecombatMain);
    // The active player receives priority (CR 117.3b).
    assert_eq!(
        state.core.priority,
        PriorityState::HeldBy {
            player: P1,
            consecutive_passes: 0
        }
    );
    assert_eq!(game.pending().0, P1);
    for player in [P1, P2] {
        assert!(game.observation(player).stack.is_empty());
    }
}

#[test]
fn casting_is_offered_only_at_sorcery_speed_with_an_exact_payment() {
    let (_, plains) = land_definitions();
    let [lions, _, giant] = creature_definitions();
    let game = Game::with_hands([
        vec![plains, plains, lions, lions, giant],
        vec![plains, lions],
    ]);

    // The pool cannot pay: P1 has a Plains but has made no mana.
    game.answer(play_land, pass);
    assert!(game.casts().is_empty());

    // It is not a main phase: with {W} in the pool and Savannah Lions in hand,
    // P1 may only pass or make mana in the beginning of combat.
    game.answer(pass, pass);
    game.answer(pass, pass);
    let core = game.state().core;
    assert_eq!(core.turn_number, 1);
    assert!(matches!(core.position, TurnPosition::Combat { .. }));
    game.answer(tap_for_mana, pass);
    assert_eq!(game.state().card_rules.mana.pools[&P1].unrestricted[0], 1);
    assert!(game.casts().is_empty());

    // The non-active player has no priority window to cast in outside a main
    // phase either: during P1's turn 3 upkeep, P2 holds {W} and Savannah
    // Lions (P2 played its Plains on turn 2). The phase rules the cast out
    // here, so this case does not show the active-player condition; that one
    // is shown in a main phase, in
    // `the_opponent_can_only_pass_or_make_mana_while_a_spell_is_on_the_stack`.
    game.run_until(|state| {
        state.core.turn_number == 3
            && state.core.priority
                == (PriorityState::HeldBy {
                    player: P2,
                    consecutive_passes: 1,
                })
    });
    game.answer(tap_for_mana, pass);
    assert_eq!(game.state().card_rules.mana.pools[&P2].unrestricted[0], 1);
    assert!(game.casts().is_empty());

    // The exact payment of two Savannah Lions is offered in P1's main phase.
    game.run_until(start_of_main_phase(3));
    game.answer(play_land, pass);
    game.answer(tap_for_mana, pass);
    game.answer(tap_for_mana, pass);
    assert_eq!(game.state().card_rules.mana.pools[&P1].unrestricted[0], 2);
    assert_eq!(game.casts().len(), 2, "both Savannah Lions, not Hill Giant");
    game.answer(cast_spell, pass);

    // The stack is not empty: {W} and a Savannah Lions are left, and nothing
    // may be cast.
    assert_eq!(game.state().card_rules.mana.pools[&P1].unrestricted[0], 1);
    assert_eq!(game.zones_of(P1, &[lions]).len(), 2);
    assert!(game.zones_of(P1, &[lions]).contains(&ZoneKind::Hand));
    assert_eq!(game.pending().0, P1);
    assert!(game.casts().is_empty());
}

/// The mana spent from each bucket: W, U, B, R, G, C, then the same six
/// buckets of mana that only pays for creature spells.
type Spend = [u32; 12];

/// {R}{R}{R}: leaves {W}.
const RED_RED_RED: Spend = [0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0];
/// {R}{R}{W}: leaves {R}.
const RED_RED_WHITE: Spend = [1, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0];

fn pay(spend: Spend) -> impl Fn(&CandidateIntent) -> bool {
    move |intent| matches!(intent, CandidateIntent::SelectManaPayment { spent_buckets } if *spent_buckets == spend)
}

fn any_payment(intent: &CandidateIntent) -> bool {
    matches!(intent, CandidateIntent::SelectManaPayment { .. })
}

/// P1's main phase of turn 7 with {R}{R}{R}{W} in the pool (three Mountains
/// and a Plains tapped for mana) and Gray Ogre in hand.
fn ogre_with_red_red_red_and_white() -> Game {
    let (mountain, plains) = land_definitions();
    let [_, ogre, _] = creature_definitions();
    let game = Game::with_hands([
        vec![mountain, mountain, mountain, plains, ogre],
        vec![mountain],
    ]);
    game.run_until(start_of_main_phase(7));
    game.answer(play_land, pass);
    for _ in 0..4 {
        game.answer(tap_for_mana, pass);
    }
    let pool = game.state().card_rules.mana.pools[&P1];
    assert_eq!((pool.unrestricted[3], pool.unrestricted[0]), (3, 1));
    game
}

/// As `ogre_with_red_red_red_and_white`, with the Ogre cast: P1 is asked how
/// to pay for it.
fn ogre_cast_awaiting_payment() -> Game {
    let game = ogre_with_red_red_red_and_white();
    game.answer(cast_spell, pass);
    assert_eq!(game.pending().1.purpose, DecisionPurposeV4::ManaPayment);
    game
}

/// The rule events of a step, in order.
fn rule_events(product: &BasicLandTransitionProduct) -> Vec<&AuthoritativeRuleEventKind> {
    product.events.iter().map(|record| &record.event).collect()
}

/// What `execute_magic_response` makes of the answer of the deciding player
/// that `wanted` accepts: the same entry point the controller runs.
fn respond(
    state: &EngineState,
    wanted: impl Fn(&CandidateIntent) -> bool,
) -> BasicLandTransitionProduct {
    let request = state.execution.pending_decision.as_ref().unwrap();
    let candidate = request
        .candidates
        .iter()
        .find(|candidate| wanted(&candidate.visible_intent))
        .expect("no such candidate");
    mtgml_rules::execute_magic_response(
        &creature_game_admission(),
        state,
        request.actor,
        &DecisionResponseV3 {
            schema_version: DECISION_RESPONSE_V3_SCHEMA.to_owned(),
            player_decision_id: request.player_decision_id,
            view_sequence: request.view_sequence,
            answer: DecisionAnswerV2::SelectOne {
                candidate_id: candidate.candidate_id,
            },
        },
        &EpisodeStatus::Running,
    )
    .unwrap()
}

/// Casting Gray Ogre with {R}{R}{R}{W} in the pool, and paying {R}{R}{R}: the
/// state before, and the two steps.
struct OgreCast {
    before: EngineState,
    begin: BasicLandTransitionProduct,
    complete: BasicLandTransitionProduct,
}

fn ogre_cast() -> OgreCast {
    let before = ogre_with_red_red_red_and_white().state();
    let begin = respond(&before, cast_spell);
    let complete = respond(&begin.next_state, pay(RED_RED_RED));
    OgreCast {
        before,
        begin,
        complete,
    }
}

#[test]
fn gray_ogre_with_three_mountains_and_a_plains_asks_how_to_pay() {
    // Gray Ogre costs {2}{R}. With {R}{R}{R}{W} in the pool, paying {R}{R}{R}
    // and paying {R}{R}{W} leave different pools, so the player decides.
    let [_, ogre, _] = creature_definitions();
    let game = ogre_with_red_red_red_and_white();
    assert_eq!(game.casts().len(), 1, "the Ogre has two ways to be paid");
    assert_eq!(game.zones_of(P1, &[ogre]), vec![ZoneKind::Hand]);
    let at_priority = game.controller.checkpoint().unwrap();

    // CR 601.2a: the card is on the stack, in public, while P1 pays.
    let (caster, step) = game.answer(cast_spell, pass);
    assert_eq!(caster, P1);
    assert!(step.observed_events.iter().any(|envelope| matches!(
        &envelope.event,
        ObservedEventKindV4::StackItemAdded {
            item: PublicStackItemV1::Spell { controller, .. },
            ..
        } if *controller == P1
    )));
    let (asked, request) = game.pending();
    assert_eq!(asked, P1);
    assert_eq!(request.purpose, DecisionPurposeV4::ManaPayment);
    assert_eq!(request.decision_domain_v2, DecisionDomainV2::ChooseOne);
    assert_eq!(request.visibility, DecisionVisibility::ActingPlayerOnly);
    assert_eq!(
        request
            .candidates
            .iter()
            .map(|candidate| candidate.intent.clone())
            .collect::<Vec<_>>(),
        [RED_RED_RED, RED_RED_WHITE]
            .map(|spent_buckets| CandidateIntent::SelectManaPayment { spent_buckets })
    );
    assert!(game.endpoint(P2).visible_decision().unwrap().is_none());

    // Nothing is paid yet, and the spell is not cast (CR 601.2i).
    let paying = game.state();
    assert_eq!(game.zones_of(P1, &[ogre]), vec![ZoneKind::Stack]);
    assert_eq!(paying.zones.stack_order.len(), 1);
    let pool = paying.card_rules.mana.pools[&P1];
    assert_eq!((pool.unrestricted[3], pool.unrestricted[0]), (3, 1));
    assert_eq!(
        paying.card_rules.turn_history.players[&P1].spells_cast_total,
        0
    );
    let continuations: Vec<_> = paying.execution.continuations.values().collect();
    let [record] = continuations.as_slice() else {
        panic!("{continuations:?}")
    };
    let ContinuationPayload::Cast(cast) = &record.payload else {
        panic!("{record:?}")
    };
    assert_eq!(cast.actor, P1);
    assert_eq!(cast.stage, CastContinuationStage::PayingMana);
    assert_eq!(
        cast.mana_payment_staging,
        Some(ManaPaymentStaging {
            stage: ManaPaymentStage::AwaitingFinalAllocation,
            mana_source_activations: Vec::new(),
        })
    );
    assert_eq!(
        paying.core.priority,
        PriorityState::HeldBy {
            player: P1,
            consecutive_passes: 0
        }
    );
    for player in [P1, P2] {
        assert!(
            matches!(
                game.observation(player).stack.as_slice(),
                [PublicStackItemV1::Spell { .. }]
            ),
            "{player:?} sees the spell on the stack"
        );
    }

    // Keeping {W}: the {R}{R}{R} payment leaves {W} in the pool.
    let (payer, step) = game.answer(pay(RED_RED_RED), any_payment);
    assert_eq!(payer, P1);
    assert!(step.observed_events.iter().any(|envelope| matches!(
        &envelope.event,
        ObservedEventKindV4::ManaPoolChanged { player, .. } if *player == P1
    )));
    let paid = game.state();
    let pool = paid.card_rules.mana.pools[&P1];
    assert_eq!((pool.unrestricted[3], pool.unrestricted[0]), (0, 1));
    assert_eq!(
        paid.card_rules.turn_history.players[&P1].spells_cast_total,
        1
    );
    assert!(paid.execution.continuations.is_empty());
    assert_eq!(paid.zones.stack_order.len(), 1);
    assert_eq!(
        paid.core.priority,
        PriorityState::HeldBy {
            player: P1,
            consecutive_passes: 0
        }
    );
    assert_eq!(game.pending().1.purpose, DecisionPurposeV4::PriorityAction);
    assert_eq!(game.pending().0, P1, "the caster has priority (CR 117.3c)");

    // Paying {R}{R}{W} instead leaves {R}.
    game.controller.restore(at_priority).unwrap();
    game.answer(cast_spell, pass);
    game.answer(pay(RED_RED_WHITE), any_payment);
    let pool = game.state().card_rules.mana.pools[&P1];
    assert_eq!((pool.unrestricted[3], pool.unrestricted[0]), (1, 0));

    // The spell resolves as any other creature spell does.
    game.answer(pass, pass);
    game.answer(pass, pass);
    game.only_object(P1, ogre, ZoneKind::Battlefield);
    assert!(game.state().zones.stack_order.is_empty());
}

#[test]
fn a_spell_is_cast_only_when_its_payment_completes() {
    let cast = ogre_cast();
    // Asking how to pay puts the card on the stack and nothing else: no cost
    // is committed, no mana leaves the pool, and the spell is not cast.
    let asking = rule_events(&cast.begin);
    assert!(asking
        .iter()
        .any(|event| matches!(event, AuthoritativeRuleEventKind::StackItemAdded { .. })));
    assert!(!asking.iter().any(|event| matches!(
        event,
        AuthoritativeRuleEventKind::SpellCast { .. }
            | AuthoritativeRuleEventKind::CostCommitted { .. }
            | AuthoritativeRuleEventKind::ManaPoolChanged { .. }
    )));
    // Paying casts it (CR 601.2h, 601.2i): the cost, the pool and the cast
    // are in the one step, and the card is not put on the stack again.
    let paying = rule_events(&cast.complete);
    let position = |wanted: fn(&AuthoritativeRuleEventKind) -> bool| {
        paying.iter().position(|event| wanted(event)).unwrap()
    };
    let cast_at = position(|event| {
        matches!(
            event,
            AuthoritativeRuleEventKind::SpellCast {
                is_creature_spell: true,
                ..
            }
        )
    });
    let committed_at = position(|event| {
        matches!(
            event,
            AuthoritativeRuleEventKind::CostCommitted { spent_buckets, source_activations, .. }
                if *spent_buckets == RED_RED_RED && source_activations.is_empty()
        )
    });
    let spent_at = position(|event| {
        matches!(
            event,
            AuthoritativeRuleEventKind::ManaPoolChanged {
                cause: ManaPoolChangeCauseV1::Spent,
                ..
            }
        )
    });
    assert!(
        committed_at < spent_at,
        "the pool shows the mana spent after the cost is committed"
    );
    // This is the trusted order today, not the order of the rules: CR 601.2h
    // has the cost paid before CR 601.2i makes the spell cast, so the payment
    // would come first. The order is a known deferred item (nothing projected
    // depends on it; revisit it with the first trigger on a cast spell). This
    // assertion pins today's order so that changing it is deliberate.
    assert!(
        cast_at < committed_at,
        "the trusted order today is SpellCast before the payment; CR 601.2h to 601.2i would put the payment first (deferred)"
    );
    assert!(!paying.iter().any(|event| matches!(
        event,
        AuthoritativeRuleEventKind::StackItemAdded { .. }
            | AuthoritativeRuleEventKind::ZoneTransition { .. }
    )));
}

#[test]
fn a_restored_payment_checkpoint_continues_identically() {
    let game = ogre_cast_awaiting_payment();
    let at_payment = game.controller.checkpoint().unwrap();
    assert_eq!(at_payment.state.execution.continuations.len(), 1);

    game.answer(pay(RED_RED_WHITE), any_payment);
    let paid = game.controller.checkpoint().unwrap();
    assert!(paid.state.execution.continuations.is_empty());

    // The game that was paid for replays to the same checkpoint.
    let report = game
        .controller
        .execute_replay(game.controller.export_replay().unwrap())
        .unwrap();
    assert_eq!(report.final_checkpoint, paid);

    // Restoring the checkpoint with the payment pending restores the request
    // and the same continuation, and the same answer leads to the same state.
    game.controller.restore(at_payment.clone()).unwrap();
    assert_eq!(game.controller.checkpoint().unwrap(), at_payment);
    assert_eq!(game.pending().1.purpose, DecisionPurposeV4::ManaPayment);
    game.answer(pay(RED_RED_WHITE), any_payment);
    assert_eq!(game.controller.checkpoint().unwrap(), paid);
}

#[test]
fn a_rejected_payment_answer_changes_nothing() {
    let game = ogre_cast_awaiting_payment();
    let at_payment = game.controller.checkpoint().unwrap();
    let (_, request) = game.pending();
    let answer = |player: PlayerId, answer: DecisionAnswerV2, stale: bool| {
        let step = game
            .endpoint(player)
            .submit(DecisionResponseV3 {
                schema_version: DECISION_RESPONSE_V3_SCHEMA.to_owned(),
                player_decision_id: if stale {
                    PlayerDecisionIdV1(request.player_decision_id.0 + 1)
                } else {
                    request.player_decision_id
                },
                view_sequence: request.view_sequence,
                answer,
            })
            .unwrap();
        assert!(
            matches!(step.submission, PlayerStepSubmissionV1::Rejected { .. }),
            "{:?}",
            step.submission
        );
        assert_eq!(game.controller.checkpoint().unwrap(), at_payment);
        assert_eq!(game.pending().1, request);
    };
    // A payment the request does not offer.
    answer(
        P1,
        DecisionAnswerV2::SelectOne {
            candidate_id: CandidateIdV1(2),
        },
        false,
    );
    // Both payments at once.
    answer(
        P1,
        DecisionAnswerV2::SelectMany {
            candidate_ids: vec![CandidateIdV1(0), CandidateIdV1(1)],
        },
        false,
    );
    // An answer to another decision.
    answer(
        P1,
        DecisionAnswerV2::SelectOne {
            candidate_id: CandidateIdV1(0),
        },
        true,
    );
    // The opponent has no decision to answer.
    answer(
        P2,
        DecisionAnswerV2::SelectOne {
            candidate_id: CandidateIdV1(0),
        },
        false,
    );
    // The payment is still asked, and still works.
    game.answer(pay(RED_RED_RED), any_payment);
    assert_eq!(
        game.state().card_rules.turn_history.players[&P1].spells_cast_total,
        1
    );
}

#[test]
fn a_payment_request_the_pending_cast_does_not_call_for_is_refused() {
    let admission = creature_game_admission();
    let state = ogre_cast_awaiting_payment().state();
    let running = EpisodeStatus::Running;
    mtgml_rules::validate_magic_pending_request(&admission, &state, &running).unwrap();

    let refused = |what: &str, tampered: &EngineState| {
        assert!(
            mtgml_rules::validate_magic_pending_request(&admission, tampered, &running).is_err(),
            "{what}"
        );
    };

    // The offered payments are not the ones the pool allows.
    let mut tampered = state.clone();
    tampered
        .execution
        .pending_decision
        .as_mut()
        .unwrap()
        .candidates
        .truncate(1);
    refused("a payment left out", &tampered);
    let mut tampered = state.clone();
    let candidates = &mut tampered
        .execution
        .pending_decision
        .as_mut()
        .unwrap()
        .candidates;
    candidates.reverse();
    for (index, candidate) in candidates.iter_mut().enumerate() {
        candidate.candidate_id = CandidateIdV1(index as u32);
    }
    refused("the payments out of order", &tampered);
    // The pool no longer pays two ways, so there is no choice to ask for.
    let mut tampered = state.clone();
    tampered
        .card_rules
        .mana
        .pools
        .get_mut(&P1)
        .unwrap()
        .unrestricted[0] = 0;
    refused("a payment with one way", &tampered);
    // The request is for the actor only.
    let mut tampered = state.clone();
    tampered
        .execution
        .pending_decision
        .as_mut()
        .unwrap()
        .visibility = DecisionVisibility::Public;
    refused("a public payment request", &tampered);
    // The continuation is not the cast of the spell the catalog describes.
    let mut tampered = state.clone();
    let record = tampered
        .execution
        .continuations
        .values_mut()
        .next()
        .unwrap();
    let ContinuationPayload::Cast(cast) = &mut record.payload else {
        unreachable!()
    };
    cast.action_cost_facts
        .mana_cost
        .as_mut()
        .unwrap()
        .generic_count = 1;
    refused("another printed cost", &tampered);
    // The caster does not hold priority.
    let mut tampered = state.clone();
    tampered.core.priority = PriorityState::HeldBy {
        player: P2,
        consecutive_passes: 0,
    };
    refused("the opponent holding priority", &tampered);
    // The episode is closed, or the admission has no such rule.
    let closed = EpisodeStatus::Truncated {
        reason: TruncationReason::ExternalStop,
        players: vec![
            PlayerOutcome {
                player: P1,
                result: PlayerResult::Loss,
            },
            PlayerOutcome {
                player: P2,
                result: PlayerResult::Loss,
            },
        ],
    };
    assert!(mtgml_rules::validate_magic_pending_request(&admission, &state, &closed).is_err());
    assert!(
        mtgml_rules::validate_magic_pending_request(&game_admission(), &state, &running).is_err()
    );
}

#[test]
fn a_payment_with_one_way_is_not_asked() {
    // With {R}{R}{R} alone, the one way to pay Gray Ogre is the payment: the
    // cast is one step, and nothing is asked.
    let (mountain, plains) = land_definitions();
    let [_, ogre, _] = creature_definitions();
    let game = Game::with_hands([
        vec![mountain, mountain, mountain, plains, ogre],
        vec![mountain],
    ]);
    game.run_until(start_of_main_phase(5));
    game.answer(play_land, pass);
    for _ in 0..3 {
        game.answer(tap_for_mana, pass);
    }
    let pool = game.state().card_rules.mana.pools[&P1];
    assert_eq!((pool.unrestricted[3], pool.unrestricted[0]), (3, 0));
    assert_eq!(game.casts().len(), 1);
    game.answer(cast_spell, pass);
    assert_eq!(game.pending().1.purpose, DecisionPurposeV4::PriorityAction);
    let state = game.state();
    assert!(state.execution.continuations.is_empty());
    assert_eq!(state.zones.stack_order.len(), 1);
    assert_eq!(state.card_rules.mana.pools[&P1], ManaPoolV1::default());
    assert_eq!(
        state.card_rules.turn_history.players[&P1].spells_cast_total,
        1
    );
}

#[test]
fn a_spell_record_needs_its_creation_operation() {
    let cast = ogre_cast();
    let ops = |product: &BasicLandTransitionProduct| product.delta.operations.clone();
    // The steps themselves are the delta rule's positive cases.
    StateDelta::between_structural_only(&cast.before, &cast.begin.next_state, ops(&cast.begin))
        .unwrap();
    StateDelta::between_structural_only(
        &cast.begin.next_state,
        &cast.complete.next_state,
        ops(&cast.complete),
    )
    .unwrap();

    let mut without_creation = ops(&cast.begin);
    without_creation
        .retain(|operation| !matches!(operation, SemanticDeltaOperation::StackItemCreated { .. }));
    assert_eq!(
        StateDelta::between_structural_only(&cast.before, &cast.begin.next_state, without_creation),
        Err(DeltaApplicationError::UncoveredMutation)
    );
}

#[test]
fn a_spell_is_cast_in_the_transition_that_creates_its_record_or_ends_its_payment() {
    let cast = ogre_cast();
    let spell_cast = cast
        .complete
        .delta
        .operations
        .iter()
        .find(|operation| matches!(operation, SemanticDeltaOperation::SpellCast { .. }))
        .unwrap()
        .clone();
    let with_a_cast_counted = |state: &EngineState| {
        let mut state = state.clone();
        state
            .card_rules
            .turn_history
            .players
            .get_mut(&P1)
            .unwrap()
            .spells_cast_total += 1;
        state
    };

    // The step that only asks how to pay creates the record: it is not the
    // step that casts the spell, even if it counts one.
    let mut creating = cast.begin.delta.operations.clone();
    creating.push(spell_cast.clone());
    assert_eq!(
        StateDelta::between_structural_only(
            &cast.before,
            &with_a_cast_counted(&cast.begin.next_state),
            creating
        ),
        Err(DeltaApplicationError::UncoveredMutation)
    );

    // A step that leaves the payment pending neither creates the record nor
    // ends the continuation.
    let mut waiting = cast.begin.next_state.clone();
    waiting.revision.0 += 1;
    waiting
        .execution
        .pending_decision
        .as_mut()
        .unwrap()
        .state_revision = waiting.revision;
    let waiting = with_a_cast_counted(&waiting);
    let operations = vec![
        spell_cast.clone(),
        SemanticDeltaOperation::PendingRequestChanged {
            from: cast
                .begin
                .next_state
                .execution
                .pending_decision
                .clone()
                .map(Box::new),
            to: waiting.execution.pending_decision.clone().map(Box::new),
        },
    ];
    assert_eq!(
        StateDelta::between_structural_only(&cast.begin.next_state, &waiting, operations),
        Err(DeltaApplicationError::UncoveredMutation)
    );

    // Dropping the cast operation while the state still counts a cast is
    // caught by the turn history, not by the timing rule.
    let mut without_cast = cast.complete.delta.operations.clone();
    without_cast.retain(|operation| !matches!(operation, SemanticDeltaOperation::SpellCast { .. }));
    assert_eq!(
        StateDelta::between_structural_only(
            &cast.begin.next_state,
            &cast.complete.next_state,
            without_cast
        ),
        Err(DeltaApplicationError::UncoveredMutation)
    );
}

#[test]
fn ending_a_payment_must_cast_the_spell() {
    let cast = ogre_cast();
    // The completing step with no cast: no `SpellCast` operation, and the
    // spells-cast count not raised, so the turn history is consistent with
    // it. The spell stays on the stack with its continuation gone; only the
    // rule that the step ending a payment casts the spell rejects this.
    let mut never_cast = cast.complete.next_state.clone();
    never_cast
        .card_rules
        .turn_history
        .players
        .get_mut(&P1)
        .unwrap()
        .spells_cast_total -= 1;
    assert_eq!(
        never_cast.card_rules.turn_history,
        cast.begin.next_state.card_rules.turn_history
    );
    assert_eq!(never_cast.zones, cast.begin.next_state.zones);
    assert!(never_cast.execution.continuations.is_empty());
    let mut without_cast = cast.complete.delta.operations.clone();
    without_cast.retain(|operation| !matches!(operation, SemanticDeltaOperation::SpellCast { .. }));
    let outcome =
        StateDelta::between_structural_only(&cast.begin.next_state, &never_cast, without_cast);
    assert!(
        matches!(outcome, Err(DeltaApplicationError::UncoveredMutation)),
        "a spell left on the stack that was never cast was accepted: {:?}",
        outcome.map(|delta| delta.operations.len())
    );
    // With the cast operation and the count, the same step is accepted.
    StateDelta::between_structural_only(
        &cast.begin.next_state,
        &cast.complete.next_state,
        cast.complete.delta.operations.clone(),
    )
    .unwrap();
}

#[test]
fn a_payment_cannot_end_in_the_transition_that_takes_its_spell_off_the_stack() {
    let cast = ogre_cast();
    // CR 601.2i: the real game. P1 and P2 pass with the cast spell on the
    // stack, and it resolves.
    let passed = respond(&cast.complete.next_state, pass);
    let resolution = respond(&passed.next_state, pass);
    assert!(resolution.next_state.zones.stack_order.is_empty());
    StateDelta::between_structural_only(
        &passed.next_state,
        &resolution.next_state,
        resolution.delta.operations.clone(),
    )
    .unwrap();

    // The same resolution, but from a state in which the payment of the spell
    // is still pending, so that the transition also ends it. Its operations
    // name the resolution and the end of the continuation, and none of them
    // is a `SpellCast`: the spell would resolve without ever being cast.
    // Casting is not undone in this slice (CR 601.2 would rewind an illegal
    // one), so no transition may end a payment and take the spell off the
    // stack.
    let mut paying = passed.next_state.clone();
    paying.execution.continuations = cast.begin.next_state.execution.continuations.clone();
    // The pending request of that payment, as of this revision.
    let mut asking = cast
        .begin
        .next_state
        .execution
        .pending_decision
        .clone()
        .unwrap();
    assert_eq!(asking.purpose, DecisionPurposeV4::ManaPayment);
    asking.state_revision = paying.revision;
    asking.view_sequence = paying.knowledge.players[&P1].next_visible_sequence;
    paying.execution.pending_decision = Some(asking.clone());
    let continuations: Vec<_> = paying.execution.continuations.iter().collect();
    let [(continuation, record)] = continuations.as_slice() else {
        panic!("{continuations:?}")
    };
    let mut operations = resolution.delta.operations.clone();
    operations.retain(|operation| {
        !matches!(
            operation,
            SemanticDeltaOperation::PendingRequestChanged { .. }
        )
    });
    operations.push(SemanticDeltaOperation::PendingRequestChanged {
        from: Some(Box::new(asking)),
        to: resolution
            .next_state
            .execution
            .pending_decision
            .clone()
            .map(Box::new),
    });
    operations.push(SemanticDeltaOperation::ContinuationChanged {
        continuation: **continuation,
        from: Some(Box::new(record.payload.clone())),
        to: None,
    });
    assert!(!operations
        .iter()
        .any(|operation| matches!(operation, SemanticDeltaOperation::SpellCast { .. })));
    let outcome = StateDelta::between_structural_only(&paying, &resolution.next_state, operations);
    assert!(
        matches!(outcome, Err(DeltaApplicationError::UncoveredMutation)),
        "a spell that was never cast left the stack: {:?}",
        outcome.map(|delta| delta.operations.len())
    );
}

#[test]
fn the_opponent_can_only_pass_or_make_mana_while_a_spell_is_on_the_stack() {
    let (mountain, plains) = land_definitions();
    let [lions, _, _] = creature_definitions();
    let game = Game::with_hands([
        vec![plains, plains, lions],
        vec![plains, lions, mountain, mountain],
    ]);
    game.run_until(start_of_main_phase(3));
    game.answer(play_land, pass);
    game.answer(tap_for_mana, pass);
    game.answer(tap_for_mana, pass);
    game.answer(cast_spell, pass);
    assert_eq!(game.state().zones.stack_order.len(), 1);

    // P2 holds an untapped Plains, Savannah Lions and a land in hand.
    game.answer(pass, pass);
    assert_eq!(game.pending().0, P2);
    let offered = game.offered();
    assert!(offered.contains(&CandidateIntent::PassPriority));
    assert!(offered
        .iter()
        .any(|intent| matches!(intent, CandidateIntent::ActivateAbility { .. })));
    assert!(
        offered.iter().all(|intent| matches!(
            intent,
            CandidateIntent::PassPriority | CandidateIntent::ActivateAbility { .. }
        )),
        "{offered:?}"
    );

    // Making mana is an action: the passes start over, and the spell stays.
    game.answer(tap_for_mana, pass);
    assert_eq!(game.state().card_rules.mana.pools[&P2].unrestricted[0], 1);
    assert_eq!(game.offered(), vec![CandidateIntent::PassPriority]);
    game.answer(pass, pass);
    assert_eq!(game.pending().0, P1);
    assert_eq!(game.state().zones.stack_order.len(), 1);

    // Both players have now passed in succession: the spell resolves, and
    // the mana the opponent made is still in its pool.
    game.answer(pass, pass);
    let state = game.state();
    assert!(state.zones.stack_order.is_empty());
    game.only_object(P1, lions, ZoneKind::Battlefield);
    assert_eq!(state.card_rules.mana.pools[&P2].unrestricted[0], 1);
    assert_eq!(game.pending().0, P1);

    // CR 302.1, 117.1a: only the active player casts a creature at sorcery
    // speed. P1 passes, so P2 holds priority in P1's precombat main phase.
    // Everything but the active-player condition allows the cast: it is a
    // main phase, the stack is empty, P2 has priority, {W} in its pool and
    // Savannah Lions in hand.
    game.answer(pass, pass);
    let state = game.state();
    assert_eq!(state.core.turn_number, 3);
    assert_eq!(state.core.active_player, P1);
    assert_eq!(state.core.position, TurnPosition::PrecombatMain);
    assert!(state.zones.stack_order.is_empty());
    assert_eq!(
        state.core.priority,
        PriorityState::HeldBy {
            player: P2,
            consecutive_passes: 1
        }
    );
    assert_eq!(state.card_rules.mana.pools[&P2].unrestricted[0], 1);
    assert_eq!(game.zones_of(P2, &[lions]), vec![ZoneKind::Hand]);
    assert_eq!(game.pending().0, P2);
    assert!(game.casts().is_empty(), "{:?}", game.offered());
}

#[test]
fn the_active_player_cannot_play_a_land_while_a_spell_is_on_the_stack() {
    let (mountain, plains) = land_definitions();
    let [lions, _, _] = creature_definitions();
    // P1 plays a Plains on turn 1. On turn 3 it holds a Plains and a drawn
    // Mountain, has played no land yet, and taps the first Plains for {W}.
    let game = Game::with_hands([vec![plains, plains, lions], vec![mountain]]);
    game.answer(play_land, pass);
    game.run_until(start_of_main_phase(3));
    game.answer(tap_for_mana, pass);
    let before_the_cast = game.state();
    assert_eq!(before_the_cast.core.active_player, P1);
    assert_eq!(before_the_cast.core.position, TurnPosition::PrecombatMain);
    assert!(before_the_cast.zones.stack_order.is_empty());
    assert_eq!(
        before_the_cast.card_rules.turn_history.players[&P1].land_plays_used,
        0
    );
    assert_eq!(
        game.zones_of(P1, &[plains, mountain])
            .iter()
            .filter(|zone| **zone == ZoneKind::Hand)
            .count(),
        2,
        "P1 holds a Plains and the Mountain it drew"
    );
    // With the stack empty, P1 may play either land (CR 305.1) and may cast.
    assert_eq!(game.offered().iter().filter(|i| play_land(i)).count(), 2);
    assert_eq!(game.casts().len(), 1);

    // CR 305.1, 305.2: with Savannah Lions on the stack, the same player with
    // the same lands in hand and no land played has no land to play. Only the
    // stack differs.
    game.answer(cast_spell, pass);
    let state = game.state();
    assert_eq!(state.zones.stack_order.len(), 1);
    assert_eq!(state.core.active_player, P1);
    assert_eq!(state.core.position, TurnPosition::PrecombatMain);
    assert_eq!(
        state.core.priority,
        PriorityState::HeldBy {
            player: P1,
            consecutive_passes: 0
        }
    );
    assert_eq!(
        state.card_rules.turn_history.players[&P1].land_plays_used,
        0
    );
    assert_eq!(game.pending().0, P1);
    let offered = game.offered();
    assert!(
        !offered.iter().any(play_land),
        "a land is offered with a spell on the stack: {offered:?}"
    );
    assert_eq!(offered, vec![CandidateIntent::PassPriority]);

    // Once the spell has resolved, the stack is empty again, and so is the
    // land play that P1 never used.
    game.answer(pass, pass);
    game.answer(pass, pass);
    assert!(game.state().zones.stack_order.is_empty());
    assert_eq!(game.pending().0, P1);
    assert_eq!(game.offered().iter().filter(|i| play_land(i)).count(), 2);
}

#[test]
fn the_opponent_learns_the_card_only_when_it_goes_on_the_stack() {
    let (mountain, plains) = land_definitions();
    let [lions, ogre, giant] = creature_definitions();
    // Two games that differ only in the caster's other hand card.
    let games = [
        Game::with_hands([vec![plains, lions, giant], vec![mountain]]),
        Game::with_hands([vec![plains, lions, ogre], vec![mountain]]),
    ];
    let mut p2_views: [Vec<Vec<u8>>; 2] = [Vec::new(), Vec::new()];
    let mut p1_views: [Vec<Vec<u8>>; 2] = [Vec::new(), Vec::new()];
    for (game, (p2, p1)) in games.iter().zip(p2_views.iter_mut().zip(&mut p1_views)) {
        let mut record = |actor: Option<(PlayerId, PlayerStepV4)>| {
            if let Some((P2, step)) = &actor {
                p2.push(mtgml_wire::encode_canonical(step).unwrap());
            }
            p2.push(
                mtgml_wire::encode_canonical(&game.endpoint(P2).information_state().unwrap())
                    .unwrap(),
            );
            p1.push(
                mtgml_wire::encode_canonical(&game.endpoint(P1).information_state().unwrap())
                    .unwrap(),
            );
        };
        let p2_knows_the_lions = || game.knows(P2, lions);
        record(None);
        record(Some(game.answer(play_land, pass)));
        record(Some(game.answer(tap_for_mana, pass)));
        assert!(!p2_knows_the_lions());
        record(Some(game.answer(cast_spell, pass)));
        assert!(p2_knows_the_lions(), "the stack shows the card");
        record(Some(game.answer(pass, pass)));
        record(Some(game.answer(pass, pass)));
    }
    assert_eq!(p2_views[0], p2_views[1]);
    // The games do differ: P1 holds different cards.
    assert_ne!(p1_views[0], p1_views[1]);
}

#[test]
fn the_card_is_public_while_its_payment_is_pending() {
    let [_, ogre, _] = creature_definitions();
    let game = ogre_with_red_red_red_and_white();
    assert!(!game.knows(P2, ogre));
    game.answer(cast_spell, pass);
    // CR 601.2a: the card is public as soon as it is on the stack, before
    // the payment the caster is asked for.
    assert_eq!(game.pending().1.purpose, DecisionPurposeV4::ManaPayment);
    assert!(game.knows(P2, ogre));
}

#[test]
fn a_restored_checkpoint_with_a_spell_on_the_stack_continues_identically() {
    let (mountain, plains) = land_definitions();
    let [lions, _, giant] = creature_definitions();
    let game = Game::with_hands([vec![plains, lions, giant], vec![mountain]]);
    game.answer(play_land, pass);
    game.answer(tap_for_mana, pass);
    game.answer(cast_spell, pass);
    let on_the_stack = game.controller.checkpoint().unwrap();
    assert_eq!(on_the_stack.state.zones.stack_order.len(), 1);

    game.answer(pass, pass);
    game.answer(pass, pass);
    let resolved = game.controller.checkpoint().unwrap();
    assert!(resolved.state.zones.stack_order.is_empty());

    game.controller.restore(on_the_stack.clone()).unwrap();
    assert_eq!(game.controller.checkpoint().unwrap(), on_the_stack);
    game.answer(pass, pass);
    game.answer(pass, pass);
    assert_eq!(game.controller.checkpoint().unwrap(), resolved);
}

#[test]
fn a_game_with_a_cast_replays_to_the_same_checkpoint() {
    let (mountain, plains) = land_definitions();
    let [lions, _, giant] = creature_definitions();
    let game = Game::with_hands([vec![plains, lions, giant], vec![mountain]]);
    game.answer(play_land, pass);
    game.answer(tap_for_mana, pass);
    game.answer(cast_spell, pass);
    game.answer(pass, pass);
    game.answer(pass, pass);
    game.only_object(P1, lions, ZoneKind::Battlefield);
    let last = game.controller.checkpoint().unwrap();

    let report = game
        .controller
        .execute_replay(game.controller.export_replay().unwrap())
        .unwrap();
    assert_eq!(report.final_checkpoint, last);
}

#[test]
fn a_closed_episode_may_hold_a_creature_spell_on_the_stack() {
    let (mountain, plains) = land_definitions();
    let [lions, _, giant] = creature_definitions();
    let game = Game::with_hands([vec![plains, lions, giant], vec![mountain]]);
    game.answer(play_land, pass);
    game.answer(tap_for_mana, pass);
    game.answer(cast_spell, pass);
    let mut state = game.state();
    // A closed episode has no pending request.
    state.execution.pending_decision = None;
    let players = vec![
        PlayerOutcome {
            player: P1,
            result: PlayerResult::Win,
        },
        PlayerOutcome {
            player: P2,
            result: PlayerResult::Loss,
        },
    ];
    let terminal = EpisodeStatus::Terminal {
        reason: TerminalReason::RulesLoss,
        players: players.clone(),
    };
    let truncated = EpisodeStatus::Truncated {
        reason: TruncationReason::ExternalStop,
        players,
    };
    let admission = creature_game_admission();
    for status in [&terminal, &truncated] {
        assert_eq!(
            mtgml_rules::validate_magic_pending_request(&admission, &state, status),
            Ok(())
        );
    }
    // A running episode needs its request.
    assert!(mtgml_rules::validate_magic_pending_request(
        &admission,
        &state,
        &EpisodeStatus::Running
    )
    .is_err());
    // The land-only admission does not know the creature on the stack.
    assert!(
        mtgml_rules::validate_magic_pending_request(&game_admission(), &state, &terminal).is_err()
    );
}

/// The attacker declaration of `turn`, before it is answered.
fn at_attackers(turn: u64) -> impl Fn(&EngineState) -> bool {
    move |state| {
        state.core.turn_number == turn
            && state
                .execution
                .pending_decision
                .as_ref()
                .is_some_and(|request| request.purpose == DecisionPurposeV4::AttackerDeclaration)
    }
}

/// P1's turn 1: it plays its Plains, taps it for {W} and casts `creature`
/// (which must cost {W}), and the creature resolves.
fn cast_on_turn_one(game: &Game, creature: CardDefinitionId) -> GameObjectId {
    game.answer(play_land, pass);
    game.answer(tap_for_mana, pass);
    game.answer(cast_spell, pass);
    game.answer(pass, pass);
    game.answer(pass, pass);
    game.only_object(P1, creature, ZoneKind::Battlefield)
}

/// P1 cast Savannah Lions on turn 1 and is at its attacker declaration of
/// turn 3, where the Lions can attack. P2 controls no creature and has
/// `p2_life` life.
fn lions_ready_to_attack(p2_life: i64) -> (Game, GameObjectId) {
    let (mountain, plains) = land_definitions();
    let [lions, _, _] = creature_definitions();
    let game = Game::with_hands_and_life([vec![plains, lions], vec![mountain]], [20, p2_life]);
    let creature = cast_on_turn_one(&game, lions);
    game.run_until(at_attackers(3));
    (game, creature)
}

/// The opaque id `player` has for `object`.
fn opaque_of(state: &EngineState, player: PlayerId, object: GameObjectId) -> OpaqueObjectId {
    state.perspective_identities.players[&player].object_to_opaque[&object]
}

/// What `execute_magic_response` makes of `answer` by the deciding player,
/// and what each player observes of it: the same entry points the controller
/// runs, with every perspective's events, not only the actor's.
fn product_and_observations(
    state: &EngineState,
    answer: DecisionAnswerV2,
) -> (
    BasicLandTransitionProduct,
    BTreeMap<PlayerId, Vec<ObservedEventEnvelopeV4>>,
) {
    let admission = creature_game_admission();
    let request = state.execution.pending_decision.as_ref().unwrap();
    let product = mtgml_rules::execute_magic_response(
        &admission,
        state,
        request.actor,
        &DecisionResponseV3 {
            schema_version: DECISION_RESPONSE_V3_SCHEMA.to_owned(),
            player_decision_id: request.player_decision_id,
            view_sequence: request.view_sequence,
            answer,
        },
        &EpisodeStatus::Running,
    )
    .unwrap();
    let observed =
        mtgml_environment::successor_projection::project_successor_events_v4_for_basic_land_profile(
            &admission,
            state,
            &EpisodeStatus::Running,
            &product.next_state,
            &product.status,
            &product.events,
            Some(&product.delta),
        )
        .unwrap();
    (product, observed)
}

/// The answer that passes priority.
fn pass_answer(state: &EngineState) -> DecisionAnswerV2 {
    let request = state.execution.pending_decision.as_ref().unwrap();
    DecisionAnswerV2::SelectOne {
        candidate_id: request
            .candidates
            .iter()
            .find(|candidate| candidate.visible_intent == CandidateIntent::PassPriority)
            .expect("no pass")
            .candidate_id,
    }
}

fn life_of(observation: &MagicSharedExecutionObservationV1, player: PlayerId) -> i64 {
    observation
        .players
        .iter()
        .find(|entry| entry.player == player)
        .unwrap()
        .life
}

#[test]
fn a_creature_cannot_attack_the_turn_it_arrives_but_can_on_its_controllers_next_turn() {
    let (mountain, plains) = land_definitions();
    let [lions, _, _] = creature_definitions();
    let game = Game::with_hands([vec![plains, lions], vec![mountain]]);
    let creature = cast_on_turn_one(&game, lions);
    assert_eq!(
        game.state().card_rules.permanents.permanents[&creature].controlled_since_turn,
        1
    );

    // CR 302.6: it came under P1's control this turn, so it cannot attack on
    // it. The declaration is still asked, and offers no attacker.
    game.run_until(at_attackers(1));
    let (actor, request) = game.pending();
    assert_eq!(actor, P1);
    assert_eq!(
        request.decision_domain_v2,
        DecisionDomainV2::ChooseMany {
            minimum: 0,
            maximum: 0
        }
    );
    assert!(request.candidates.is_empty());
    game.declare_attackers(&[]);

    // On P1's next turn it has been under P1's control since the turn began.
    game.run_until(at_attackers(3));
    let own = opaque_of(&game.state(), P1, creature);
    let (actor, request) = game.pending();
    assert_eq!(actor, P1);
    assert_eq!(request.visibility, DecisionVisibility::ActingPlayerOnly);
    assert_eq!(
        request.decision_domain_v2,
        DecisionDomainV2::ChooseMany {
            minimum: 0,
            maximum: 1
        }
    );
    assert_eq!(
        game.offered(),
        vec![CandidateIntent::SelectObject { object: own }]
    );
}

#[test]
fn an_unblocked_attack_lowers_the_defenders_life() {
    let (mountain, _) = land_definitions();
    let [_, _, giant] = creature_definitions();
    let game = Game::with_hands_and_life(
        [
            vec![mountain, mountain, mountain, mountain, giant],
            vec![mountain],
        ],
        [20, 20],
    );
    // Hill Giant costs {3}{R}: P1 casts it on turn 7 with four Mountains.
    game.run_until(start_of_main_phase(7));
    game.answer(play_land, pass);
    for _ in 0..4 {
        game.answer(tap_for_mana, pass);
    }
    game.answer(cast_spell, pass);
    game.answer(pass, pass);
    game.answer(pass, pass);
    let creature = game.only_object(P1, giant, ZoneKind::Battlefield);
    game.run_until(at_attackers(9));
    let own = opaque_of(&game.state(), P1, creature);
    game.declare_attackers(&[own]);
    for player in [P1, P2] {
        assert_eq!(life_of(&game.observation(player), P2), 20);
    }

    // Both players pass in the declare attackers step; P2 has no creature, so
    // the blockers step has no declaration, and both pass again.
    game.answer(pass, pass);
    game.answer(pass, pass);
    assert_eq!(
        game.state().core.position,
        TurnPosition::Combat {
            step: mtgml_state::CombatStep::DeclareBlockers
        }
    );
    game.answer(pass, pass);

    // CR 510.1a, 510.2: P2's pass opens the combat damage step, in which the
    // Giant deals 3 damage to P2 (CR 120.3a).
    let before = game.state();
    assert_eq!(before.core.active_player, P1);
    let (product, observed) = product_and_observations(&before, pass_answer(&before));
    for player in [P1, P2] {
        assert!(
            observed[&player].iter().any(|envelope| envelope.event
                == ObservedEventKindV4::LifeChanged {
                    player: P2,
                    from: 20,
                    to: 17
                }),
            "{player:?} observes the life change"
        );
    }
    let (second, step) = game.answer(pass, pass);
    assert_eq!(second, P2);
    assert_eq!(step.observed_events, observed[&P2]);
    assert_eq!(game.state(), product.next_state);

    let state = game.state();
    assert_eq!(state.core.players[&P2].life, 17);
    assert_eq!(state.core.players[&P1].life, 20);
    for player in [P1, P2] {
        let observation = game.observation(player);
        assert_eq!(life_of(&observation, P2), 17, "{player:?}");
        assert_eq!(life_of(&observation, P1), 20, "{player:?}");
    }
    assert!(state.combat.as_ref().unwrap().damage_step_completed);
    assert!(state.card_rules.turn_history.players[&P2].lost_life_this_turn);
    assert!(!state.card_rules.turn_history.players[&P1].lost_life_this_turn);
    // CR 510.3: the active player receives priority in the damage step.
    assert_eq!(
        state.core.position,
        TurnPosition::Combat {
            step: mtgml_state::CombatStep::CombatDamage
        }
    );
    assert_eq!(
        state.core.priority,
        PriorityState::HeldBy {
            player: P1,
            consecutive_passes: 0
        }
    );
    assert_eq!(game.pending().0, P1);
    // The Giant survives: nothing damages it.
    game.only_object(P1, giant, ZoneKind::Battlefield);
}

#[test]
fn attackers_tap_and_both_players_see_the_attack() {
    let (game, creature) = lions_ready_to_attack(20);
    let before = game.state();
    assert!(!before.zones.objects[&creature].tapped);
    let request = before.execution.pending_decision.as_ref().unwrap();
    let (product, observed) = product_and_observations(
        &before,
        DecisionAnswerV2::SelectMany {
            candidate_ids: vec![request.candidates[0].candidate_id],
        },
    );
    // CR 508.1f: the creature taps, and then it is declared (CR 508.1k);
    // both players see both.
    for player in [P1, P2] {
        let id = opaque_of(&before, player, creature);
        let events: Vec<_> = observed[&player]
            .iter()
            .map(|envelope| &envelope.event)
            .collect();
        let tapped = events
            .iter()
            .position(|event| {
                **event
                    == ObservedEventKindV4::ObjectTapped {
                        object: id,
                        tapped: true,
                    }
            })
            .unwrap_or_else(|| panic!("{player:?} does not see the creature tap: {events:?}"));
        let declared = events
            .iter()
            .position(|event| {
                **event
                    == ObservedEventKindV4::AttackersDeclared {
                        attacking_player: P1,
                        defending_player: P2,
                        attackers: vec![id],
                    }
            })
            .unwrap_or_else(|| panic!("{player:?} does not see the attack: {events:?}"));
        assert!(tapped < declared);
    }

    let (actor, step) = game.declare_attackers(&[opaque_of(&before, P1, creature)]);
    assert_eq!(actor, P1);
    assert_eq!(step.observed_events, observed[&P1]);
    let state = game.state();
    assert_eq!(state, product.next_state);
    assert!(state.zones.objects[&creature].tapped);
    let combat = state.combat.as_ref().unwrap();
    assert_eq!(combat.defending_player, P2);
    assert_eq!(combat.attackers, vec![creature]);
    assert!(combat.blockers.is_empty());
    assert!(combat.blocked_attackers.is_empty());
    for player in [P1, P2] {
        assert!(game
            .observation(player)
            .tapped
            .contains(&opaque_of(&state, player, creature)));
    }
    // CR 508.2: the active player receives priority in the declare attackers
    // step.
    assert_eq!(
        state.core.position,
        TurnPosition::Combat {
            step: mtgml_state::CombatStep::DeclareAttackers
        }
    );
    assert_eq!(game.pending().0, P1);
}

#[test]
fn an_empty_attack_declaration_is_public() {
    let (mountain, plains) = land_definitions();
    let game = Game::with_hands([vec![plains], vec![mountain]]);
    game.run_until(at_attackers(1));
    let before = game.state();
    let (_, observed) = product_and_observations(
        &before,
        DecisionAnswerV2::SelectMany {
            candidate_ids: Vec::new(),
        },
    );
    for player in [P1, P2] {
        assert!(
            observed[&player].iter().any(|envelope| envelope.event
                == ObservedEventKindV4::AttackersDeclared {
                    attacking_player: P1,
                    defending_player: P2,
                    attackers: Vec::new(),
                }),
            "{player:?} observes that nobody attacks"
        );
    }
}

/// P1 attacks with its Savannah Lions, and both players pass until P2's pass
/// would open the combat damage step.
fn attack_to_the_damage_step(p2_life: i64) -> Game {
    let (game, creature) = lions_ready_to_attack(p2_life);
    game.declare_attackers(&[opaque_of(&game.state(), P1, creature)]);
    for _ in 0..3 {
        game.answer(pass, pass);
    }
    assert_eq!(
        game.state().core.position,
        TurnPosition::Combat {
            step: mtgml_state::CombatStep::DeclareBlockers
        }
    );
    assert_eq!(game.pending().0, P2);
    game
}

/// P2 lost to combat damage and is at `life`: the status, the checkpoint and
/// what both players observe say so.
fn assert_p2_lost_to_combat_damage(game: &Game, life: i64) {
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
    assert_eq!(checkpoint.state.core.players[&P2].life, life);
    assert!(checkpoint.state.core.players[&P2].has_lost);
    assert!(!checkpoint.state.core.players[&P1].has_lost);
    for player in [P1, P2] {
        assert_eq!(game.endpoint(player).visible_decision().unwrap(), None);
        // A closed episode still shows its final observation, with the life
        // that ended the game.
        assert_eq!(life_of(&game.observation(player), P2), life, "{player:?}");
    }
}

#[test]
fn zero_life_ends_the_game() {
    let game = attack_to_the_damage_step(2);
    let (actor, request) = game.pending();
    assert_eq!(actor, P2);
    let (second, step) = game.answer(pass, pass);
    assert_eq!(second, P2);
    // CR 120.3a, 704.5a: Savannah Lions deals 2 damage, P2 is at 0 life and
    // loses (CR 104.2a: P1 wins).
    assert!(step.observed_events.iter().any(|envelope| envelope.event
        == ObservedEventKindV4::LifeChanged {
            player: P2,
            from: 2,
            to: 0
        }));
    assert!(matches!(step.status, EpisodeStatus::Terminal { .. }));
    assert_p2_lost_to_combat_damage(&game, 0);

    // The episode is closed: a further answer is refused, and changes nothing.
    let checkpoint = game.controller.checkpoint().unwrap();
    let late = game
        .endpoint(P2)
        .submit(DecisionResponseV3 {
            schema_version: DECISION_RESPONSE_V3_SCHEMA.to_owned(),
            player_decision_id: request.player_decision_id,
            view_sequence: request.view_sequence,
            answer: DecisionAnswerV2::SelectOne {
                candidate_id: request.candidates[0].candidate_id,
            },
        })
        .unwrap();
    assert_eq!(
        late.submission,
        PlayerStepSubmissionV1::Rejected {
            code: mtgml_observation::PlayerSubmissionCodeV1::EpisodeClosed
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
fn overkill_damage_shows_negative_life_and_ends_the_game() {
    let game = attack_to_the_damage_step(1);
    let (_, step) = game.answer(pass, pass);
    // The Lions deals 2 damage to a player at 1 life: the life total goes
    // below 0 (CR 120.3a), and the observation shows it.
    assert!(step.observed_events.iter().any(|envelope| envelope.event
        == ObservedEventKindV4::LifeChanged {
            player: P2,
            from: 1,
            to: -1
        }));
    assert_p2_lost_to_combat_damage(&game, -1);
    let checkpoint = game.controller.checkpoint().unwrap();
    let report = game
        .controller
        .execute_replay(game.controller.export_replay().unwrap())
        .unwrap();
    assert_eq!(report.final_checkpoint, checkpoint);
}

#[test]
fn a_defender_with_an_untapped_creature_fails_closed() {
    let (_, plains) = land_definitions();
    let [lions, _, _] = creature_definitions();
    let game = Game::with_hands([vec![plains, lions], vec![plains, lions]]);
    let attacker = cast_on_turn_one(&game, lions);
    // P2 casts its own Savannah Lions on turn 2, and it stays untapped.
    game.run_until(start_of_main_phase(2));
    game.answer(play_land, pass);
    game.answer(tap_for_mana, pass);
    game.answer(cast_spell, pass);
    game.answer(pass, pass);
    game.answer(pass, pass);
    let blocker = game.only_object(P2, lions, ZoneKind::Battlefield);
    assert!(!game.state().zones.objects[&blocker].tapped);

    game.run_until(at_attackers(3));
    game.declare_attackers(&[opaque_of(&game.state(), P1, attacker)]);
    game.answer(pass, pass);

    // P2's pass would open the declare blockers step, in which P2 could block.
    // Blocks are not supported yet: the endpoint reports the unsupported rule
    // instead of a step, and nothing changes.
    let before = game.controller.checkpoint().unwrap();
    let (actor, request) = game.pending();
    assert_eq!(actor, P2);
    let outcome = game.endpoint(P2).submit(DecisionResponseV3 {
        schema_version: DECISION_RESPONSE_V3_SCHEMA.to_owned(),
        player_decision_id: request.player_decision_id,
        view_sequence: request.view_sequence,
        answer: pass_answer(&before.state),
    });
    assert_eq!(outcome, Err(PlayerEndpointError::ServiceUnavailable));
    assert_eq!(game.controller.checkpoint().unwrap(), before);
    assert_eq!(game.pending().1, request);

    // The rules say why.
    let pending = before.state.execution.pending_decision.as_ref().unwrap();
    assert_eq!(
        mtgml_rules::execute_magic_response(
            &creature_game_admission(),
            &before.state,
            P2,
            &DecisionResponseV3 {
                schema_version: DECISION_RESPONSE_V3_SCHEMA.to_owned(),
                player_decision_id: pending.player_decision_id,
                view_sequence: pending.view_sequence,
                answer: pass_answer(&before.state),
            },
            &EpisodeStatus::Running,
        ),
        Err(mtgml_rules::BasicLandTransitionError::TurnProgressUnsupported)
    );
}

#[test]
fn a_restored_attack_continues_identically() {
    let (game, creature) = lions_ready_to_attack(20);
    let attacker = opaque_of(&game.state(), P1, creature);
    let at_declaration = game.controller.checkpoint().unwrap();
    game.declare_attackers(&[attacker]);
    let declared = game.controller.checkpoint().unwrap();
    for _ in 0..4 {
        game.answer(pass, pass);
    }
    let damaged = game.controller.checkpoint().unwrap();
    assert_eq!(damaged.state.core.players[&P2].life, 18);

    // The pending declaration, with its candidate, restores and continues.
    game.controller.restore(at_declaration.clone()).unwrap();
    assert_eq!(game.controller.checkpoint().unwrap(), at_declaration);
    assert_eq!(game.offered().len(), 1);
    game.declare_attackers(&[attacker]);
    assert_eq!(game.controller.checkpoint().unwrap(), declared);

    // So does an attack in progress.
    game.controller.restore(declared.clone()).unwrap();
    assert_eq!(game.controller.checkpoint().unwrap(), declared);
    for _ in 0..4 {
        game.answer(pass, pass);
    }
    assert_eq!(game.controller.checkpoint().unwrap(), damaged);

    // The whole game, attack included, replays to the same checkpoint.
    let report = game
        .controller
        .execute_replay(game.controller.export_replay().unwrap())
        .unwrap();
    assert_eq!(report.final_checkpoint, damaged);
}

#[test]
fn a_restored_combat_the_game_could_not_reach_is_refused() {
    let (_, plains) = land_definitions();
    let [lions, _, _] = creature_definitions();
    let game = Game::with_hands([vec![plains, lions], vec![plains, lions]]);
    let attacker = cast_on_turn_one(&game, lions);
    game.run_until(start_of_main_phase(2));
    game.answer(play_land, pass);
    game.answer(tap_for_mana, pass);
    game.answer(cast_spell, pass);
    game.answer(pass, pass);
    game.answer(pass, pass);
    let blocker = game.only_object(P2, lions, ZoneKind::Battlefield);
    game.run_until(at_attackers(3));
    game.declare_attackers(&[opaque_of(&game.state(), P1, attacker)]);

    // P1 attacks and holds priority in the declare attackers step, while P2's
    // Savannah Lions is untapped: the game reaches this.
    let reached = game.controller.checkpoint().unwrap();
    let admission = creature_game_admission();
    let restore = |state: EngineState| {
        EnvironmentCheckpointV8::new_for_basic_land_profile(
            &admission,
            state,
            reached.status.clone(),
            reached.limit_counters.clone(),
            reached.execution_identity.clone(),
        )
    };
    let checkpoint = restore(reached.state.clone()).unwrap();
    game.controller.restore(checkpoint).unwrap();
    assert_eq!(game.controller.checkpoint().unwrap(), reached);

    // It never reaches the declare blockers step, or any later one, with a
    // blocker possible: the pass that would open it is refused.
    for step in [
        mtgml_state::CombatStep::DeclareBlockers,
        mtgml_state::CombatStep::CombatDamage,
    ] {
        let mut forged = reached.state.clone();
        forged.core.position = TurnPosition::Combat { step };
        assert!(restore(forged).is_err(), "{step:?}");
    }
    // Were the blocker tapped, it could not block (CR 509.1a).
    let mut tapped = reached.state.clone();
    tapped.zones.objects.get_mut(&blocker).unwrap().tapped = true;
    tapped.core.position = TurnPosition::Combat {
        step: mtgml_state::CombatStep::DeclareBlockers,
    };
    restore(tapped).unwrap();

    // An attack against its own controller is not one the game makes.
    let mut forged = reached.state.clone();
    forged.combat.as_mut().unwrap().defending_player = P1;
    assert!(restore(forged).is_err());
    assert_eq!(game.controller.checkpoint().unwrap(), reached);
}

/// Savannah Lions cast on P1's turn 1 and on the stack: P1 holds priority
/// with the cast complete. And Gray Ogre cast on P1's turn 7 and on the
/// stack, with P1 still choosing how to pay. Each is a state the game reaches.
fn checkpoints_with_a_spell_on_the_stack() -> [(&'static str, EnvironmentCheckpointV8); 2] {
    let (mountain, plains) = land_definitions();
    let [lions, _, _] = creature_definitions();
    let cast = Game::with_hands([vec![plains, lions], vec![mountain]]);
    cast.answer(play_land, pass);
    cast.answer(tap_for_mana, pass);
    cast.answer(cast_spell, pass);
    let cast = cast.controller.checkpoint().unwrap();
    assert_eq!(
        cast.state
            .execution
            .pending_decision
            .as_ref()
            .unwrap()
            .purpose,
        DecisionPurposeV4::PriorityAction
    );
    let paying = ogre_cast_awaiting_payment()
        .controller
        .checkpoint()
        .unwrap();
    assert_eq!(
        paying
            .state
            .execution
            .pending_decision
            .as_ref()
            .unwrap()
            .purpose,
        DecisionPurposeV4::ManaPayment
    );
    [("a cast spell", cast), ("a spell being paid for", paying)]
}

/// `state` with `forge` applied to the one spell on its stack: its stack
/// record and its card object.
fn with_the_spell_forged(
    state: &EngineState,
    forge: impl FnOnce(&mut StackRecord, &mut GameObject),
) -> EngineState {
    let mut forged = state.clone();
    let [top] = forged.zones.stack_order.as_slice() else {
        panic!("not one spell on the stack");
    };
    let record = forged.zones.stack_records.get_mut(top).unwrap();
    let Some(StackItemPayload::Spell {
        stack_card_object, ..
    }) = record.payload.clone()
    else {
        panic!("the stack holds no spell");
    };
    let object = forged.zones.objects.get_mut(&stack_card_object).unwrap();
    forge(record, object);
    forged
}

/// Only the active player casts a creature spell (CR 302.1, 117.1a), and the
/// player who casts it controls it (CR 112.2, 601.2a) and owns its card
/// (CR 108.3: every card of this slice's decks starts in its owner's deck). So
/// P1, the active player, controls the spell's stack record and card object
/// and owns the card. A restore refuses any other state, and accepts the
/// state it was forged from.
fn assert_forged_spell_is_refused(forge: impl Fn(&mut StackRecord, &mut GameObject), what: &str) {
    let mut forgeries = Forgeries::default();
    for (state_name, reached) in checkpoints_with_a_spell_on_the_stack() {
        let state = &reached.state;
        assert_eq!(state.core.active_player, P1);
        restored(&reached, state.clone()).unwrap();
        let forged = with_the_spell_forged(state, &forge);
        forgeries.restore(format!("{what}, {state_name}"), &reached, forged);
    }
    forgeries.assert_all_refused();
}

/// The forged states a restore accepted instead of refusing.
#[derive(Default)]
struct Forgeries {
    accepted: Vec<String>,
}

impl Forgeries {
    /// Restores `forged` in place of the game that reached `reached`: it must
    /// be refused as an invalid state.
    fn restore(
        &mut self,
        what: impl std::fmt::Display,
        reached: &EnvironmentCheckpointV8,
        forged: EngineState,
    ) {
        match restored(reached, forged) {
            Err(CheckpointV8Error::State) => {}
            other => self
                .accepted
                .push(format!("{what}: {:?}", other.map(|_| ()))),
        }
    }

    fn assert_all_refused(self) {
        assert!(
            self.accepted.is_empty(),
            "a restore accepted what the game cannot reach:
{}",
            self.accepted.join(
                "
"
            )
        );
    }
}

#[test]
fn a_restored_spell_with_a_stack_record_the_active_player_does_not_control_is_refused() {
    assert_forged_spell_is_refused(
        |record, _| record.controller = P2,
        "the stack record is controlled by P2",
    );
}

#[test]
fn a_restored_spell_with_a_card_the_active_player_does_not_own_is_refused() {
    assert_forged_spell_is_refused(|_, object| object.owner = P2, "the card is owned by P2");
}

#[test]
fn a_restored_spell_with_a_card_object_the_active_player_does_not_control_is_refused() {
    assert_forged_spell_is_refused(
        |_, object| object.controller = P2,
        "the card object is controlled by P2",
    );
}

#[test]
fn a_restored_spell_of_the_player_who_is_not_active_is_refused() {
    // The record, the card and its object agree with each other, and the
    // active player is the other one.
    assert_forged_spell_is_refused(
        |record, object| {
            record.controller = P2;
            object.owner = P2;
            object.controller = P2;
        },
        "the spell is P2's, and P1 is the active player",
    );
}

#[test]
fn a_restored_spell_outside_a_main_phase_is_refused() {
    let mut forgeries = Forgeries::default();
    for (state_name, reached) in checkpoints_with_a_spell_on_the_stack() {
        for position in [
            TurnPosition::Beginning {
                step: mtgml_state::BeginningStep::Upkeep,
            },
            TurnPosition::Combat {
                step: mtgml_state::CombatStep::BeginningOfCombat,
            },
            TurnPosition::Ending {
                step: mtgml_state::EndingStep::EndStep,
            },
        ] {
            let mut forged = reached.state.clone();
            forged.core.position = position;
            forgeries.restore(format!("{state_name} in {position:?}"), &reached, forged);
        }
    }
    forgeries.assert_all_refused();
}

#[test]
fn a_restored_permanent_its_owner_does_not_control_is_refused() {
    // P1's turn 1: it plays its Plains, taps it, and casts Savannah Lions,
    // which resolves. The state has a land and a creature, both P1's; the
    // Plains is tapped, so no mana ability of it is on offer to change.
    let (mountain, plains) = land_definitions();
    let [lions, _, _] = creature_definitions();
    let game = Game::with_hands([vec![plains, lions], vec![mountain]]);
    let creature = cast_on_turn_one(&game, lions);
    let land = game.only_object(P1, plains, ZoneKind::Battlefield);
    let reached = game.controller.checkpoint().unwrap();
    assert!(reached.state.zones.objects[&land].tapped);
    restored(&reached, reached.state.clone()).unwrap();

    // A permanent is controlled by the player under whose control it entered
    // the battlefield (CR 110.2), and no card of this pool changes control: a
    // permanent that P2 controls and P1 owns is not one the game makes.
    let mut forgeries = Forgeries::default();
    for (what, object) in [("the creature", creature), ("the land", land)] {
        let mut forged = reached.state.clone();
        forged.zones.objects.get_mut(&object).unwrap().controller = P2;
        forgeries.restore(
            format!("{what} is controlled by P2 and owned by P1"),
            &reached,
            forged,
        );
    }
    forgeries.assert_all_refused();
}

#[test]
fn a_restored_attacker_that_could_not_have_attacked_is_refused() {
    let (game, creature) = lions_ready_to_attack(20);
    game.declare_attackers(&[opaque_of(&game.state(), P1, creature)]);
    let reached = game.controller.checkpoint().unwrap();
    let state = &reached.state;
    assert_eq!(state.combat.as_ref().unwrap().attackers, vec![creature]);
    assert!(state.zones.objects[&creature].tapped);
    assert!(
        state.card_rules.permanents.permanents[&creature].controlled_since_turn
            < state.core.turn_number
    );
    restored(&reached, state.clone()).unwrap();

    // CR 508.1f: declaring an attacker taps it.
    let mut forgeries = Forgeries::default();
    let mut untapped = state.clone();
    untapped.zones.objects.get_mut(&creature).unwrap().tapped = false;
    forgeries.restore("an attacker that is untapped", &reached, untapped);
    // CR 302.6, 508.1a: a creature that came under its controller's control
    // this turn cannot attack.
    let mut sick = state.clone();
    sick.card_rules
        .permanents
        .permanents
        .get_mut(&creature)
        .unwrap()
        .controlled_since_turn = state.core.turn_number;
    forgeries.restore("an attacker that arrived this turn", &reached, sick);
    forgeries.assert_all_refused();
}

#[test]
fn every_boundary_of_a_cast_a_resolution_an_attack_and_its_damage_restores() {
    let (mountain, plains) = land_definitions();
    let [lions, _, _] = creature_definitions();

    // P1 casts Savannah Lions on turn 1, attacks with it on turn 3, and casts
    // a second one in its postcombat main phase. Every answer on the way
    // leads to a checkpoint that restores.
    let game = Game::with_hands_and_life(
        [vec![plains, plains, lions, lions], vec![mountain]],
        [20, 20],
    );
    game.answer_restorable(play_land, pass);
    game.answer_restorable(tap_for_mana, pass);
    game.answer_restorable(cast_spell, pass);
    game.answer_restorable(pass, pass);
    game.answer_restorable(pass, pass);
    let first = game.only_object(P1, lions, ZoneKind::Battlefield);
    while !at_attackers(3)(&game.state()) {
        game.answer_restorable(play_land, pass);
    }
    game.declare_attackers(&[opaque_of(&game.state(), P1, first)]);
    game.assert_restorable();
    while game.state().core.position != TurnPosition::PostcombatMain {
        game.answer_restorable(pass, pass);
    }
    let state = game.state();
    assert_eq!(state.core.turn_number, 3);
    assert_eq!(state.core.players[&P2].life, 18, "the attack dealt damage");
    // The second spell is cast and resolves in the postcombat main phase.
    game.answer_restorable(tap_for_mana, pass);
    game.answer_restorable(cast_spell, pass);
    let state = game.state();
    assert_eq!(state.core.position, TurnPosition::PostcombatMain);
    assert_eq!(state.zones.stack_order.len(), 1);
    game.answer_restorable(pass, pass);
    game.answer_restorable(pass, pass);
    assert_eq!(game.zones_of(P1, &[lions]), vec![ZoneKind::Battlefield; 2]);

    // Gray Ogre is paid for in a choice of two ways, and attacks on turn 9.
    let game = ogre_with_red_red_red_and_white();
    game.assert_restorable();
    game.answer_restorable(cast_spell, pass);
    assert_eq!(game.pending().1.purpose, DecisionPurposeV4::ManaPayment);
    game.answer_restorable(pay(RED_RED_WHITE), any_payment);
    game.answer_restorable(pass, pass);
    game.answer_restorable(pass, pass);
    while !at_attackers(9)(&game.state()) {
        game.answer_restorable(play_land, pass);
    }
    let ogre = game.only_object(P1, creature_definitions()[1], ZoneKind::Battlefield);
    let life = game.state().core.players[&P2].life;
    game.declare_attackers(&[opaque_of(&game.state(), P1, ogre)]);
    game.assert_restorable();
    while game.state().core.position != TurnPosition::PostcombatMain {
        game.answer_restorable(pass, pass);
    }
    assert_eq!(
        game.state().core.players[&P2].life,
        life - 2,
        "Gray Ogre is a 2/2 and dealt its damage"
    );
}

/// The permanent `object` as `player` is shown it: under their own opaque id,
/// with `printed` power and toughness if it is a creature.
fn permanent_row(
    state: &EngineState,
    player: PlayerId,
    object: GameObjectId,
    printed: Option<(i64, i64)>,
    controlled_since_turn: u64,
) -> PermanentObservationV1 {
    PermanentObservationV1 {
        object: opaque_of(state, player, object),
        controller: state.zones.objects[&object].controller,
        controlled_since_turn,
        power: printed.map(|(power, _)| power),
        toughness: printed.map(|(_, toughness)| toughness),
    }
}

/// The rows in the order a player is shown them: ascending by the opaque ids
/// that player has for the permanents.
fn ascending(mut rows: Vec<PermanentObservationV1>) -> Vec<PermanentObservationV1> {
    rows.sort_by_key(|row| row.object);
    rows
}

/// What `player` is shown of the whole battlefield, read from the
/// authoritative state: one row per permanent, lands included, and the
/// printed power and toughness of the permanents made from `creatures`.
fn battlefield_rows(
    state: &EngineState,
    player: PlayerId,
    creatures: &[(CardDefinitionId, (i64, i64))],
) -> Vec<PermanentObservationV1> {
    ascending(
        state
            .zones
            .locations
            .iter()
            .filter(|(_, location)| location.zone == ZoneKind::Battlefield)
            .map(|(object, _)| {
                let definition = state.zones.objects[object].card_definition;
                let printed = creatures
                    .iter()
                    .find(|(creature, _)| *creature == definition)
                    .map(|(_, printed)| *printed);
                let since = state.card_rules.permanents.permanents[object].controlled_since_turn;
                permanent_row(state, player, *object, printed, since)
            })
            .collect(),
    )
}

#[test]
fn both_players_see_every_permanent_with_its_controller_and_each_creatures_power_toughness() {
    let (mountain, plains) = land_definitions();
    let [lions, _, giant] = creature_definitions();
    let game = Game::with_hands([vec![plains, lions], vec![plains, lions]]);
    for player in [P1, P2] {
        assert!(game.observation(player).permanents.is_empty(), "{player:?}");
    }

    // P1 plays its Plains and casts Savannah Lions on turn 1. Both players
    // see both permanents, each under their own opaque id, with the player
    // who controls it and the turn since which they have (CR 302.6). The land
    // has no power or toughness; the Lions shows its printed 2/1 (CR 208.1).
    let first = cast_on_turn_one(&game, lions);
    let first_land = game.only_object(P1, plains, ZoneKind::Battlefield);
    let state = game.state();
    for player in [P1, P2] {
        assert_eq!(
            game.observation(player).permanents,
            ascending(vec![
                permanent_row(&state, player, first_land, None, 1),
                permanent_row(&state, player, first, Some((2, 1)), 1),
            ]),
            "{player:?}"
        );
        // On the wire the arrival turn is a decimal string, as `turn_number`,
        // and the land's power and toughness are null.
        let wire: serde_json::Value =
            serde_json::from_slice(&game.observation_payload(player)).unwrap();
        assert_eq!(wire["turn_number"], "1");
        let rows = wire["permanents"].as_array().unwrap();
        assert_eq!(rows.len(), 2);
        for row in rows {
            assert_eq!(row["controller"], "1");
            assert_eq!(row["controlled_since_turn"], "1");
            if row["object"] == opaque_of(&state, player, first).to_string() {
                assert_eq!(
                    (row["power"].as_i64(), row["toughness"].as_i64()),
                    (Some(2), Some(1))
                );
            } else {
                assert!(
                    row["power"].is_null() && row["toughness"].is_null(),
                    "{row}"
                );
            }
        }
    }

    // P2 plays its Plains and casts its own Savannah Lions on turn 2: the
    // rows are one per permanent whoever controls it, ascending by the
    // viewer's opaque ids, and say whose each is.
    game.run_until(start_of_main_phase(2));
    game.answer(play_land, pass);
    game.answer(tap_for_mana, pass);
    game.answer(cast_spell, pass);
    game.answer(pass, pass);
    game.answer(pass, pass);
    let second = game.only_object(P2, lions, ZoneKind::Battlefield);
    let second_land = game.only_object(P2, plains, ZoneKind::Battlefield);
    let state = game.state();
    for player in [P1, P2] {
        let shown = game.observation(player).permanents;
        assert_eq!(
            shown,
            ascending(vec![
                permanent_row(&state, player, first_land, None, 1),
                permanent_row(&state, player, first, Some((2, 1)), 1),
                permanent_row(&state, player, second_land, None, 2),
                permanent_row(&state, player, second, Some((2, 1)), 2),
            ]),
            "{player:?}"
        );
        let controlled_by =
            |controller: PlayerId| shown.iter().filter(move |row| row.controller == controller);
        assert_eq!(controlled_by(P1).count(), 2, "{player:?}");
        assert_eq!(controlled_by(P2).count(), 2, "{player:?}");
    }

    // Another creature has its own printed power and toughness: Hill Giant is
    // 3/3 and arrives on turn 7, among the lands both players have played.
    let game = Game::with_hands([
        vec![mountain, mountain, mountain, mountain, giant],
        vec![mountain],
    ]);
    game.run_until(start_of_main_phase(7));
    game.answer(play_land, pass);
    for _ in 0..4 {
        game.answer(tap_for_mana, pass);
    }
    game.answer(cast_spell, pass);
    game.answer(pass, pass);
    game.answer(pass, pass);
    let creature = game.only_object(P1, giant, ZoneKind::Battlefield);
    let state = game.state();
    for player in [P1, P2] {
        let shown = game.observation(player).permanents;
        assert_eq!(
            shown,
            battlefield_rows(&state, player, &[(giant, (3, 3))]),
            "{player:?}"
        );
        // Only the Giant has power and toughness.
        let creatures: Vec<_> = shown
            .iter()
            .filter(|row| row.power.is_some() || row.toughness.is_some())
            .collect();
        assert_eq!(
            creatures,
            vec![&permanent_row(&state, player, creature, Some((3, 3)), 7)],
            "{player:?}"
        );
        // Every land is shown under its controller.
        let controlled_by =
            |controller: PlayerId| shown.iter().filter(move |row| row.controller == controller);
        assert_eq!(controlled_by(P1).count(), 5, "four lands and the Giant");
        assert!(controlled_by(P2).count() >= 1, "{player:?}");
        assert!(controlled_by(P2).all(|row| row.power.is_none()));
    }
}

#[test]
fn both_players_see_which_creatures_attack() {
    let (_, plains) = land_definitions();
    let [lions, _, _] = creature_definitions();
    let game = Game::with_hands([vec![plains, plains, lions, lions], vec![plains]]);
    let first = cast_on_turn_one(&game, lions);

    // On turn 3 P1 casts a second Savannah Lions, which cannot attack the turn
    // it arrives (CR 302.6); the first can.
    game.run_until(start_of_main_phase(3));
    game.answer(play_land, pass);
    game.answer(tap_for_mana, pass);
    game.answer(cast_spell, pass);
    game.answer(pass, pass);
    game.answer(pass, pass);
    let state = game.state();
    let second = state
        .zones
        .objects
        .values()
        .map(|object| object.id)
        .find(|id| {
            *id != first
                && state.zones.objects[id].card_definition == lions
                && state.zones.locations[id].zone == ZoneKind::Battlefield
        })
        .expect("the second Savannah Lions is on the battlefield");
    game.run_until(at_attackers(3));
    let lions_printed = [(lions, (2, 1))];
    for player in [P1, P2] {
        let observation = game.observation(player);
        assert_eq!(
            observation.permanents,
            battlefield_rows(&state, player, &lions_printed),
            "{player:?}"
        );
        let creatures = observation
            .permanents
            .iter()
            .filter(|row| row.power.is_some());
        assert_eq!(creatures.count(), 2, "{player:?}");
        assert!(observation.attacking.is_empty(), "{player:?}");
    }

    // The attack is declared: both players see it, as the ids of the
    // attackers they know, and the other creature and the lands are not in it.
    game.declare_attackers(&[opaque_of(&state, P1, first)]);
    let state = game.state();
    assert_eq!(state.combat.as_ref().unwrap().attackers, vec![first]);
    for player in [P1, P2] {
        let observation = game.observation(player);
        assert_eq!(
            observation.attacking,
            vec![opaque_of(&state, player, first)],
            "{player:?}"
        );
        assert_eq!(
            observation.permanents,
            battlefield_rows(&state, player, &lions_printed),
            "{player:?}"
        );
        assert!(observation
            .permanents
            .iter()
            .any(|row| row.object == opaque_of(&state, player, second) && row.power.is_some()));
        // The attacker is one of the creatures, and tapped for both players.
        assert!(observation.attacking.iter().all(|id| observation
            .permanents
            .iter()
            .any(|row| row.object == *id && row.power.is_some())));
        assert!(observation.tapped.contains(&observation.attacking[0]));
    }

    // The combat ends with the step: nothing attacks any more.
    game.run_until(|state| state.combat.is_none());
    let state = game.state();
    for player in [P1, P2] {
        let observation = game.observation(player);
        assert!(observation.attacking.is_empty(), "{player:?}");
        assert_eq!(
            observation.permanents,
            battlefield_rows(&state, player, &lions_printed),
            "{player:?}"
        );
    }
}
