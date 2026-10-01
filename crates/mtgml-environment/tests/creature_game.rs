//! A deck with vanilla creatures starts a game, and its creatures are cast
//! and resolve, through the production controller and the player endpoints.

mod common;

use common::{
    creature_deck_game, creature_definitions, creature_game, creature_game_admission,
    game_admission, land_definitions, P1, P2,
};
use mtgml_decision::{
    CandidateIntent, DecisionAnswerV2, DecisionPurposeV4, DecisionResponseV3, DecisionVisibility,
    PlayerDecisionRequestV4, DECISION_RESPONSE_V3_SCHEMA,
};
use mtgml_environment::{PlayerEndpoint, PlayerEndpointHandle, TrustedEnvironmentController};
use mtgml_model::{
    CardDefinitionId, EpisodeStatus, GameObjectId, OpaqueObjectId, PlayerId, PlayerOutcome,
    PlayerResult, TerminalReason, TruncationReason, ZoneKind,
};
use mtgml_observation::{
    MagicSharedExecutionObservationV1, ObservedEventKindV4, PlayerStepSubmissionV1, PlayerStepV4,
    PublicStackItemV1, StackItemRemovalCauseV1,
};
use mtgml_state::{EngineState, PriorityState, TurnPosition};

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

    // The player is not active: during P1's turn 3 upkeep, P2 holds {W} and
    // Savannah Lions. P2 played its Plains on turn 2.
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

#[test]
fn a_payment_with_two_outcomes_is_not_offered_until_the_player_can_choose() {
    // Gray Ogre costs {2}{R}. With {R}{R}{R}{W} in the pool, paying {R}{R}{R}
    // and paying {R}{R}{W} leave different pools, so the cast needs a payment
    // decision that does not exist yet: it is not offered, and nothing is
    // paid on the player's behalf.
    let (mountain, plains) = land_definitions();
    let [_, ogre, _] = creature_definitions();
    let hands = || {
        [
            vec![mountain, mountain, mountain, plains, ogre],
            vec![mountain],
        ]
    };
    let game = Game::with_hands(hands());
    game.run_until(start_of_main_phase(7));
    game.answer(play_land, pass);
    for _ in 0..4 {
        game.answer(tap_for_mana, pass);
    }
    let pool = game.state().card_rules.mana.pools[&P1];
    assert_eq!((pool.unrestricted[3], pool.unrestricted[0]), (3, 1));
    assert!(game.casts().is_empty());
    assert_eq!(game.zones_of(P1, &[ogre]), vec![ZoneKind::Hand]);

    // With {R}{R}{R} alone, the one way to pay is offered.
    let game = Game::with_hands(hands());
    game.run_until(start_of_main_phase(5));
    game.answer(play_land, pass);
    for _ in 0..3 {
        game.answer(tap_for_mana, pass);
    }
    let pool = game.state().card_rules.mana.pools[&P1];
    assert_eq!((pool.unrestricted[3], pool.unrestricted[0]), (3, 0));
    assert_eq!(game.casts().len(), 1);
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
