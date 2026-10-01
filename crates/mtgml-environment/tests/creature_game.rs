//! A deck with vanilla creatures starts a game, and its creatures are cast
//! and resolve, through the production controller and the player endpoints.

mod common;

use common::{
    creature_deck_game, creature_definitions, creature_game, creature_game_admission,
    game_admission, land_definitions, P1, P2,
};
use mtgml_decision::{
    CandidateIntent, DecisionAnswerV2, DecisionDomainV2, DecisionPurposeV4, DecisionResponseV3,
    DecisionVisibility, PlayerDecisionRequestV4, DECISION_RESPONSE_V3_SCHEMA,
};
use mtgml_environment::{PlayerEndpoint, PlayerEndpointHandle, TrustedEnvironmentController};
use mtgml_model::{
    CandidateIdV1, CardDefinitionId, EpisodeStatus, GameObjectId, OpaqueObjectId,
    PlayerDecisionIdV1, PlayerId, PlayerOutcome, PlayerResult, TerminalReason, TruncationReason,
    ZoneKind,
};
use mtgml_observation::{
    MagicSharedExecutionObservationV1, ObservedEventKindV4, PlayerStepSubmissionV1, PlayerStepV4,
    PublicStackItemV1, StackItemRemovalCauseV1,
};
use mtgml_rules::{AuthoritativeRuleEventKind, BasicLandTransitionProduct};
use mtgml_state::{
    CastContinuationStage, ContinuationPayload, DeltaApplicationError, EngineState,
    ManaPaymentStage, ManaPaymentStaging, ManaPoolChangeCauseV1, ManaPoolV1, PriorityState,
    SemanticDeltaOperation, StateDelta, TurnPosition,
};

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
            // No creature has been cast on a turn that reaches combat.
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

    fn observation(&self, player: PlayerId) -> MagicSharedExecutionObservationV1 {
        let information = self.endpoint(player).information_state().unwrap();
        let payload = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            &information.current_observation.payload_base64,
        )
        .unwrap();
        mtgml_wire::decode_canonical(&payload).unwrap()
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
    assert!(cast_at < committed_at && committed_at < spent_at);
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
fn the_opponent_learns_the_card_only_when_it_is_cast() {
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
        let p2_knows_the_lions = || {
            game.endpoint(P2)
                .information_state()
                .unwrap()
                .retained_knowledge
                .iter()
                .any(|known| {
                    matches!(
                        known,
                        mtgml_observation::PlayerKnownObjectV1::Active {
                            known_definition: Some(definition),
                            ..
                        } if *definition == lions
                    )
                })
        };
        record(None);
        record(Some(game.answer(play_land, pass)));
        record(Some(game.answer(tap_for_mana, pass)));
        assert!(!p2_knows_the_lions());
        record(Some(game.answer(cast_spell, pass)));
        assert!(p2_knows_the_lions(), "the cast shows the card");
        record(Some(game.answer(pass, pass)));
        record(Some(game.answer(pass, pass)));
    }
    assert_eq!(p2_views[0], p2_views[1]);
    // The games do differ: P1 holds different cards.
    assert_ne!(p1_views[0], p1_views[1]);
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
