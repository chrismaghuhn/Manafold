//! The defending player declares blocks, one untapped creature at a time,
//! through the production controller and the player endpoints (CR 509.1).

mod common;

use common::{
    creature_definitions, creature_game, creature_game_admission, land_definitions, P1, P2,
};
use mtgml_decision::{
    CandidateIntent, DecisionAnswerV2, DecisionDomainV2, DecisionPurposeV4, DecisionResponseV3,
    DecisionVisibility, PlayerDecisionRequestV4, DECISION_RESPONSE_V3_SCHEMA,
};
use mtgml_environment::{
    EnvironmentCheckpointV8, PlayerEndpoint, PlayerEndpointHandle, TrustedEnvironmentController,
};
use mtgml_model::{
    CandidateIdV1, CardDefinitionId, EpisodeStatus, GameObjectId, OpaqueObjectId,
    PlayerDecisionIdV1, PlayerId, StateRevision, TruncationReason, VisibleSequence, ZoneKind,
};
use mtgml_observation::{
    MagicCompletedOrder, MagicPendingSbaOrdering, MagicSharedExecutionObservationV1,
    ObservedEventEnvelopeV4, ObservedEventKindV4, PlayerStepSubmissionV1, PlayerStepV4,
    SyntheticPriority,
};
use mtgml_rules::{AuthoritativeRuleEventKind, BasicLandTransitionProduct};
use mtgml_state::{
    CombatBlockerAssignmentV1, CombatStep, ContinuationPayload, EndingStep, EngineState,
    PriorityState, SbaObjectCauseV1, SbaSelectedActionV1, SemanticDeltaOperation, TurnPosition,
    VisibilityPartition, ZoneLocation, ZonePosition,
};
use std::collections::{BTreeMap, BTreeSet};

struct Game {
    controller: TrustedEnvironmentController,
    players: [PlayerEndpointHandle; 2],
}

impl Game {
    /// A game at P1's first precombat main with the given hands (P1's first)
    /// and ten Mountains in each library.
    fn with_hands(hands: [Vec<CardDefinitionId>; 2]) -> Self {
        let (mountain, _) = land_definitions();
        let libraries = [vec![mountain; 10], vec![mountain; 10]];
        let controller = creature_game(&libraries, &hands, 5);
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

    fn state(&self) -> EngineState {
        self.checkpoint().state
    }

    fn checkpoint(&self) -> EnvironmentCheckpointV8 {
        self.controller.checkpoint().unwrap()
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

    /// The deciding player submits `answer` to the request it has.
    fn submit(&self, answer: DecisionAnswerV2) -> (PlayerId, PlayerStepV4) {
        let (player, request) = self.pending();
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

    /// The deciding player answers with the first candidate `wanted` accepts,
    /// or with the first `fallback` candidate. Declares no attackers, and
    /// declares no block.
    fn answer(
        &self,
        wanted: impl Fn(&CandidateIntent) -> bool,
        fallback: impl Fn(&CandidateIntent) -> bool,
    ) -> (PlayerId, PlayerStepV4) {
        let (_, request) = self.pending();
        let answer = match request.purpose {
            DecisionPurposeV4::AttackerDeclaration => DecisionAnswerV2::SelectMany {
                candidate_ids: Vec::new(),
            },
            DecisionPurposeV4::BlockerDeclaration => block_answer(&request, None),
            _ => {
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
            }
        };
        self.submit(answer)
    }

    /// The deciding player declares exactly `attackers`, which the request
    /// must offer.
    fn declare_attackers(&self, attackers: &[OpaqueObjectId]) -> PlayerStepV4 {
        let (_, request) = self.pending();
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
        self.submit(DecisionAnswerV2::SelectMany { candidate_ids })
            .1
    }

    /// The defender answers the block request it has: the creature it asks
    /// about blocks `attacker`, or nothing.
    fn declare_block(&self, attacker: Option<OpaqueObjectId>) -> (PlayerId, PlayerStepV4) {
        let (_, request) = self.pending();
        assert_eq!(request.purpose, DecisionPurposeV4::BlockerDeclaration);
        self.submit(block_answer(&request, attacker))
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

    /// Passes until `reached` holds: plays no land and casts nothing, and
    /// declares no attackers and no block.
    fn pass_until(&self, reached: impl Fn(&EngineState) -> bool) {
        let mut answers = 0;
        while !reached(&self.state()) {
            self.answer(|_| false, pass);
            answers += 1;
            assert!(answers < 500, "the game never reached the wanted state");
        }
    }

    /// The bytes of what `player` knows and sees.
    fn information_bytes(&self, player: PlayerId) -> Vec<u8> {
        mtgml_wire::encode_canonical(&self.endpoint(player).information_state().unwrap()).unwrap()
    }

    /// What `player` has seen and knows, without who holds priority, which
    /// every step of the turn changes and which is public: how many visible
    /// occurrences there have been, what they retain, and what the
    /// observation shows.
    fn seen(&self, player: PlayerId) -> Seen {
        let information = self.endpoint(player).information_state().unwrap();
        let payload = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            &information.current_observation.payload_base64,
        )
        .unwrap();
        let mut observation: MagicSharedExecutionObservationV1 =
            mtgml_wire::decode_canonical(&payload).unwrap();
        observation.priority = SyntheticPriority::None;
        Seen {
            sequence: information.next_visible_sequence,
            knowledge: serde_json::to_string(&information.retained_knowledge).unwrap(),
            observation,
        }
    }

    fn seen_by_both(&self) -> [Seen; 2] {
        [self.seen(P1), self.seen(P2)]
    }
}

/// See `Game::seen`.
#[derive(Debug, PartialEq)]
struct Seen {
    sequence: VisibleSequence,
    knowledge: String,
    observation: MagicSharedExecutionObservationV1,
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

/// The answer to a block request in which its creature blocks `attacker`, or
/// nothing.
fn block_answer(
    request: &PlayerDecisionRequestV4,
    attacker: Option<OpaqueObjectId>,
) -> DecisionAnswerV2 {
    let candidate = request
        .candidates
        .iter()
        .find(|candidate| {
            matches!(candidate.intent,
                CandidateIntent::DeclareBlock { attacker: offered, .. } if offered == attacker)
        })
        .unwrap_or_else(|| panic!("the request does not offer {attacker:?}"));
    DecisionAnswerV2::SelectOne {
        candidate_id: candidate.candidate_id,
    }
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

/// The active player plays a land, taps `mana` of its mana sources and casts
/// the creature in its hand, which costs that much; the spell resolves.
fn cast_creature(game: &Game, mana: usize) {
    game.answer(play_land, pass);
    for _ in 0..mana {
        game.answer(tap_for_mana, pass);
    }
    game.answer(cast_spell, pass);
    game.answer(pass, pass);
    game.answer(pass, pass);
}

/// As `cast_creature`, for a Savannah Lions (which costs {W}).
fn cast_lions(game: &Game) {
    cast_creature(game, 1);
}

fn lions() -> CardDefinitionId {
    creature_definitions()[0]
}

/// The creatures made from `definition` that `owner` controls on the
/// battlefield, in object order.
fn creatures_of(
    state: &EngineState,
    owner: PlayerId,
    definition: CardDefinitionId,
) -> Vec<GameObjectId> {
    state
        .zones
        .objects
        .values()
        .filter(|object| {
            object.card_definition == definition
                && object.controller == owner
                && state.zones.locations[&object.id].zone == ZoneKind::Battlefield
        })
        .map(|object| object.id)
        .collect()
}

/// The Savannah Lions `owner` controls on the battlefield, in object order.
fn lions_of(state: &EngineState, owner: PlayerId) -> Vec<GameObjectId> {
    creatures_of(state, owner, lions())
}

/// The opaque id `player` has for `object`.
fn opaque_of(state: &EngineState, player: PlayerId, object: GameObjectId) -> OpaqueObjectId {
    state.perspective_identities.players[&player].object_to_opaque[&object]
}

/// `objects` in the order of `player`'s opaque ids.
fn in_opaque_order(
    state: &EngineState,
    player: PlayerId,
    mut objects: Vec<GameObjectId>,
) -> Vec<GameObjectId> {
    objects.sort_by_key(|object| opaque_of(state, player, *object));
    objects
}

/// P1 and P2 each cast a Savannah Lions on each of their first two turns (P1
/// on turns 1 and 3, P2 on turns 2 and 4), and P1 is at its attacker
/// declaration of turn 5, where both its Lions can attack and both of P2's
/// are untapped.
fn two_lions_each() -> Game {
    let (_, plains) = land_definitions();
    let hand = || vec![plains, lions(), plains, lions()];
    let game = Game::with_hands([hand(), hand()]);
    cast_lions(&game);
    for turn in 2..=4 {
        game.run_until(start_of_main_phase(turn));
        cast_lions(&game);
    }
    game.run_until(at_attackers(5));
    let state = game.state();
    assert_eq!(lions_of(&state, P1).len(), 2);
    assert_eq!(lions_of(&state, P2).len(), 2);
    game
}

/// P1 attacks with both of its Lions and both players pass, which opens the
/// declare blockers step: P2 is asked about its first creature. Returns the
/// attackers, and P2's creatures in the order they are asked about.
fn attack_with_both_lions(game: &Game) -> ([GameObjectId; 2], [GameObjectId; 2]) {
    let state = game.state();
    let attackers: [GameObjectId; 2] = lions_of(&state, P1).try_into().unwrap();
    let blockers: [GameObjectId; 2] = in_opaque_order(&state, P2, lions_of(&state, P2))
        .try_into()
        .unwrap();
    game.declare_attackers(&attackers.map(|attacker| opaque_of(&state, P1, attacker)));
    game.answer(pass, pass);
    game.answer(pass, pass);
    (attackers, blockers)
}

/// P2 casts a Savannah Lions on its first turn. P1 casts `attacker`, a creature
/// that costs `cost` mana and which it pays for with that many Mountains, on
/// its `cost`th turn, and is at its attacker declaration of the turn after,
/// where the creature can attack and the Lions is untapped. Returns the game,
/// the creature and the Lions.
fn attacker_against_a_lions(
    attacker: CardDefinitionId,
    cost: usize,
) -> (Game, GameObjectId, GameObjectId) {
    let (mountain, plains) = land_definitions();
    let hand = [vec![mountain; cost], vec![attacker]].concat();
    let game = Game::with_hands([hand, vec![plains, lions()]]);
    game.run_until(start_of_main_phase(2));
    cast_lions(&game);
    let turn = 2 * cost as u64 - 1;
    game.run_until(start_of_main_phase(turn));
    cast_creature(&game, cost);
    game.run_until(at_attackers(turn + 2));
    let state = game.state();
    let [creature]: [GameObjectId; 1] = creatures_of(&state, P1, attacker).try_into().unwrap();
    let [blocker]: [GameObjectId; 1] = lions_of(&state, P2).try_into().unwrap();
    (game, creature, blocker)
}

/// P1 attacks with `attacker`, P2 blocks it with its Lions, and both players
/// pass until P2, who holds priority in the declare blockers step, would open
/// the combat damage step by passing.
fn attack_and_block(game: &Game, attacker: GameObjectId) {
    let state = game.state();
    game.declare_attackers(&[opaque_of(&state, P1, attacker)]);
    game.answer(pass, pass);
    game.answer(pass, pass);
    game.declare_block(Some(opaque_of(&state, P2, attacker)));
    game.answer(pass, pass);
    let state = game.state();
    assert_eq!(
        state.core.position,
        TurnPosition::Combat {
            step: CombatStep::DeclareBlockers
        }
    );
    assert_eq!(game.pending().0, P2);
}

/// The block candidates P2 is offered for `blocker`: "no block" first, then
/// each attacker in the order of P2's opaque ids.
fn expected_candidates(
    state: &EngineState,
    blocker: GameObjectId,
    attackers: &[GameObjectId],
) -> Vec<CandidateIntent> {
    let blocker = opaque_of(state, P2, blocker);
    let mut attackers: Vec<_> = attackers
        .iter()
        .map(|attacker| opaque_of(state, P2, *attacker))
        .collect();
    attackers.sort();
    std::iter::once(None)
        .chain(attackers.into_iter().map(Some))
        .map(|attacker| CandidateIntent::DeclareBlock { blocker, attacker })
        .collect()
}

/// What `execute_magic_response` makes of `answer` by the deciding player.
fn product_of(state: &EngineState, answer: DecisionAnswerV2) -> BasicLandTransitionProduct {
    let request = state.execution.pending_decision.as_ref().unwrap();
    mtgml_rules::execute_magic_response(
        &creature_game_admission(),
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
    .unwrap()
}

/// The answer in `state`'s pending block request in which its creature blocks
/// `attacker`, or nothing.
fn block_in(state: &EngineState, attacker: Option<GameObjectId>) -> DecisionAnswerV2 {
    let request = state.execution.pending_decision.as_ref().unwrap();
    let candidate = request
        .candidates
        .iter()
        .find(|candidate| {
            matches!(candidate.trusted_binding,
                mtgml_decision::EngineCandidateBinding::DeclareBlock { attacker: bound, .. }
                    if bound == attacker)
        })
        .unwrap();
    DecisionAnswerV2::SelectOne {
        candidate_id: candidate.candidate_id,
    }
}

/// The answer that passes priority in `state`'s pending request.
fn pass_in(state: &EngineState) -> DecisionAnswerV2 {
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
    let product = product_of(state, answer);
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

/// The zone moves of the events `player` observes: the opaque ids of the
/// object before and after, and the zones.
fn observed_moves(
    events: &[ObservedEventEnvelopeV4],
) -> Vec<(
    Option<OpaqueObjectId>,
    Option<OpaqueObjectId>,
    ZoneKind,
    ZoneKind,
)> {
    events
        .iter()
        .filter_map(|envelope| match &envelope.event {
            ObservedEventKindV4::ObjectMoved {
                old_object,
                new_object,
                from,
                to,
                ..
            } => Some((*old_object, *new_object, *from, *to)),
            _ => None,
        })
        .collect()
}

/// The block declarations the product's events make, with their assignments.
fn declarations(product: &BasicLandTransitionProduct) -> Vec<&Vec<CombatBlockerAssignmentV1>> {
    product
        .events
        .iter()
        .filter_map(|event| match &event.event {
            AuthoritativeRuleEventKind::BlockersDeclared { assignments } => Some(assignments),
            _ => None,
        })
        .collect()
}

fn assignment(blocker: GameObjectId, attacker: GameObjectId) -> CombatBlockerAssignmentV1 {
    CombatBlockerAssignmentV1 { blocker, attacker }
}

/// The block declaration in progress.
fn block_declaration(
    state: &EngineState,
) -> (
    PlayerId,
    Vec<GameObjectId>,
    BTreeMap<GameObjectId, Option<GameObjectId>>,
) {
    let [record] = state
        .execution
        .continuations
        .values()
        .collect::<Vec<_>>()
        .try_into()
        .expect("one continuation");
    let ContinuationPayload::BlockDeclaration {
        defender,
        pending_blockers,
        declared,
    } = &record.payload
    else {
        panic!("not a block declaration: {record:?}");
    };
    (*defender, pending_blockers.clone(), declared.clone())
}

#[test]
fn the_defender_declares_blocks_one_creature_at_a_time() {
    let game = two_lions_each();
    let (attackers, blockers) = attack_with_both_lions(&game);
    let state = game.state();

    // CR 509.1: the declare blockers step starts with the declaration, which
    // nobody has priority for (CR 509.2).
    assert_eq!(
        state.core.position,
        TurnPosition::Combat {
            step: CombatStep::DeclareBlockers
        }
    );
    assert_eq!(state.core.priority, PriorityState::None);

    // CR 509.1a: P2 is asked about its first untapped creature, in the order of
    // its own opaque ids: that creature blocks one of the two attackers, or
    // nothing.
    let (actor, request) = game.pending();
    assert_eq!(actor, P2);
    assert_eq!(request.purpose, DecisionPurposeV4::BlockerDeclaration);
    assert_eq!(request.decision_domain_v2, DecisionDomainV2::ChooseOne);
    assert_eq!(request.visibility, DecisionVisibility::ActingPlayerOnly);
    assert_eq!(
        game.offered(),
        expected_candidates(&state, blockers[0], &attackers)
    );
    assert_eq!(game.offered().len(), 3);
    assert_eq!(
        block_declaration(&state),
        (P2, blockers.to_vec(), BTreeMap::new())
    );
    let continuation = *state.execution.continuations.keys().next().unwrap();
    assert_eq!(
        state
            .execution
            .pending_decision
            .as_ref()
            .unwrap()
            .continuation_id,
        Some(continuation)
    );
    game.controller.restore(game.checkpoint()).unwrap();

    // The first answer: the first creature blocks the first attacker. Nothing
    // is declared yet.
    let product = product_of(&state, block_in(&state, Some(attackers[0])));
    assert_eq!(declarations(&product), Vec::<&Vec<_>>::new());
    assert!(product.delta.operations.iter().any(|operation| matches!(
        operation,
        SemanticDeltaOperation::ContinuationChanged { .. }
    )));
    let (answered_by, step) = game.declare_block(Some(opaque_of(&state, P2, attackers[0])));
    assert_eq!(answered_by, P2);
    assert!(step.observed_events.is_empty());
    let half = game.state();
    assert_eq!(half, product.next_state);
    let combat = half.combat.as_ref().unwrap();
    assert!(combat.blockers.is_empty());
    assert!(combat.blocked_attackers.is_empty());
    assert_eq!(
        block_declaration(&half),
        (
            P2,
            vec![blockers[1]],
            BTreeMap::from([(blockers[0], Some(attackers[0]))])
        )
    );
    // The same continuation carries on, and the request names the next creature.
    assert_eq!(half.execution.continuations.len(), 1);
    assert_eq!(
        half.execution
            .pending_decision
            .as_ref()
            .unwrap()
            .continuation_id,
        Some(continuation)
    );
    assert_eq!(game.pending().0, P2);
    assert_eq!(
        game.offered(),
        expected_candidates(&half, blockers[1], &attackers)
    );
    game.controller.restore(game.checkpoint()).unwrap();

    // The second answer completes the declaration: the second creature blocks
    // the second attacker. One BlockersDeclared follows, sorted by blocker.
    let product = product_of(&half, block_in(&half, Some(attackers[1])));
    let mut expected = vec![
        assignment(blockers[0], attackers[0]),
        assignment(blockers[1], attackers[1]),
    ];
    expected.sort_by_key(|assignment| assignment.blocker);
    assert_eq!(declarations(&product), vec![&expected]);
    game.declare_block(Some(opaque_of(&half, P2, attackers[1])));
    let declared = game.state();
    assert_eq!(declared, product.next_state);

    // CR 509.1g, 509.1h: both attackers are blocked, each by one creature.
    let combat = declared.combat.as_ref().unwrap();
    assert_eq!(
        combat.blockers,
        BTreeMap::from([
            (blockers[0], Some(attackers[0])),
            (blockers[1], Some(attackers[1]))
        ])
    );
    assert_eq!(
        combat.blocked_attackers,
        BTreeSet::from([attackers[0], attackers[1]])
    );
    // The declaration is over: no continuation, still the declare blockers
    // step, and the active player has priority (CR 509.2).
    assert!(declared.execution.continuations.is_empty());
    assert_eq!(
        declared.core.position,
        TurnPosition::Combat {
            step: CombatStep::DeclareBlockers
        }
    );
    assert_eq!(
        declared.core.priority,
        PriorityState::HeldBy {
            player: P1,
            consecutive_passes: 0
        }
    );
    let (actor, request) = game.pending();
    assert_eq!(actor, P1);
    assert_eq!(request.purpose, DecisionPurposeV4::PriorityAction);
    game.controller.restore(game.checkpoint()).unwrap();
}

#[test]
fn declaring_no_block_declares_an_empty_block_and_the_attack_stays_unblocked() {
    let game = two_lions_each();
    let (attackers, blockers) = attack_with_both_lions(&game);
    let state = game.state();
    assert_eq!(game.pending().0, P2);

    let first = product_of(&state, block_in(&state, None));
    assert_eq!(declarations(&first), Vec::<&Vec<_>>::new());
    game.declare_block(None);
    let half = game.state();
    let last = product_of(&half, block_in(&half, None));
    // Every answer was "no block": one declaration, with no assignments.
    assert_eq!(declarations(&last), vec![&Vec::new()]);
    game.declare_block(None);

    let declared = game.state();
    let combat = declared.combat.as_ref().unwrap();
    assert!(combat.blockers.is_empty());
    assert!(combat.blocked_attackers.is_empty());
    assert_eq!(combat.attackers, attackers.to_vec());
    assert!(declared.execution.continuations.is_empty());
    assert_eq!(game.pending().0, P1);
    assert_eq!(blockers.len(), 2);

    // Unblocked, both Lions deal their 2 damage to P2 (CR 510.1a).
    let life = declared.core.players[&P2].life;
    game.answer(pass, pass);
    game.answer(pass, pass);
    assert_eq!(game.state().core.players[&P2].life, life - 4);
}

#[test]
fn a_defender_without_an_untapped_creature_is_not_asked() {
    // P2 has no creature: nothing is asked and nothing is declared.
    let (_, plains) = land_definitions();
    let (mountain, _) = land_definitions();
    let game = Game::with_hands([vec![plains, lions()], vec![mountain]]);
    cast_lions(&game);
    game.run_until(at_attackers(3));
    let state = game.state();
    let attacker = lions_of(&state, P1)[0];
    game.declare_attackers(&[opaque_of(&state, P1, attacker)]);
    game.answer(pass, pass);
    let before = game.state();
    let product = product_of(
        &before,
        DecisionAnswerV2::SelectOne {
            candidate_id: before
                .execution
                .pending_decision
                .as_ref()
                .unwrap()
                .candidates[0]
                .candidate_id,
        },
    );
    assert_eq!(declarations(&product), Vec::<&Vec<_>>::new());
    assert!(product.next_state.execution.continuations.is_empty());
    assert_eq!(
        product
            .next_decision
            .as_ref()
            .map(|request| (request.actor, request.purpose.clone())),
        Some((P1, DecisionPurposeV4::PriorityAction))
    );
}

#[test]
fn a_tapped_creature_is_not_asked_to_block() {
    // P2 attacks on turn 4 with the Lions it cast on turn 2, which stays tapped
    // through P1's turn 5; the Lions it cast on turn 4 is untapped.
    let (_, plains) = land_definitions();
    let hand = || vec![plains, lions(), plains, lions()];
    let game = Game::with_hands([hand(), hand()]);
    cast_lions(&game);
    for turn in 2..=4 {
        game.run_until(start_of_main_phase(turn));
        cast_lions(&game);
    }
    game.run_until(at_attackers(4));
    let state = game.state();
    let old_lions = lions_of(&state, P2)
        .into_iter()
        .min_by_key(|object| state.card_rules.permanents.permanents[object].controlled_since_turn)
        .unwrap();
    game.declare_attackers(&[opaque_of(&state, P2, old_lions)]);
    // P1 has two untapped creatures and declines to block with either.
    game.answer(pass, pass);
    game.answer(pass, pass);
    assert_eq!(
        game.pending().1.purpose,
        DecisionPurposeV4::BlockerDeclaration
    );
    game.declare_block(None);
    game.declare_block(None);

    game.run_until(at_attackers(5));
    let state = game.state();
    assert!(state.zones.objects[&old_lions].tapped);
    let new_lions = lions_of(&state, P2)
        .into_iter()
        .find(|object| *object != old_lions)
        .unwrap();
    let attackers = lions_of(&state, P1);
    game.declare_attackers(
        &attackers
            .iter()
            .map(|a| opaque_of(&state, P1, *a))
            .collect::<Vec<_>>(),
    );
    game.answer(pass, pass);
    game.answer(pass, pass);

    // Only the untapped creature is asked about, and then the declaration ends.
    let state = game.state();
    assert_eq!(
        block_declaration(&state),
        (P2, vec![new_lions], BTreeMap::new())
    );
    assert_eq!(
        game.offered(),
        expected_candidates(&state, new_lions, &attackers)
    );
    game.declare_block(Some(opaque_of(&state, P2, attackers[0])));
    let declared = game.state();
    assert_eq!(
        declared.combat.as_ref().unwrap().blockers,
        BTreeMap::from([(new_lions, Some(attackers[0]))])
    );
    assert_eq!(game.pending().0, P1);
}

#[test]
fn a_half_declared_block_is_invisible_to_the_attacker() {
    let game = two_lions_each();
    let state = game.state();
    // The harness sees a change: the attack itself is public.
    let before_attack = game.information_bytes(P1);
    let (attackers, _) = attack_with_both_lions(&game);
    assert_ne!(game.information_bytes(P1), before_attack);

    // The attacker knows nothing of the declaration in progress.
    let information = game.information_bytes(P1);
    let observation = game
        .endpoint(P1)
        .information_state()
        .unwrap()
        .current_observation;
    assert_eq!(game.endpoint(P1).visible_decision().unwrap(), None);
    game.declare_block(Some(opaque_of(&state, P2, attackers[0])));
    assert_eq!(game.information_bytes(P1), information);
    assert_eq!(
        game.endpoint(P1)
            .information_state()
            .unwrap()
            .current_observation,
        observation
    );
    assert_eq!(game.endpoint(P1).visible_decision().unwrap(), None);

    // The half-declared block shows in no player's observation, and the
    // defender's own request is the only place it is.
    assert_eq!(game.pending().0, P2);
}

#[test]
fn a_restored_half_declared_block_continues_identically() {
    let game = two_lions_each();
    let (attackers, _) = attack_with_both_lions(&game);
    let state = game.state();
    let own = |attacker: GameObjectId| Some(opaque_of(&state, P2, attacker));
    let at_first = game.checkpoint();
    game.declare_block(own(attackers[1]));
    let half = game.checkpoint();
    game.declare_block(own(attackers[0]));
    let declared = game.checkpoint();

    // The checkpoint with the whole declaration pending restores the request
    // and its continuation, and the same answers lead to the same checkpoints.
    game.controller.restore(at_first.clone()).unwrap();
    assert_eq!(game.checkpoint(), at_first);
    assert_eq!(
        game.pending().1.purpose,
        DecisionPurposeV4::BlockerDeclaration
    );
    game.declare_block(own(attackers[1]));
    assert_eq!(game.checkpoint(), half);

    // So does the half-declared block.
    game.controller.restore(half.clone()).unwrap();
    assert_eq!(game.checkpoint(), half);
    assert_eq!(game.pending().0, P2);
    game.declare_block(own(attackers[0]));
    assert_eq!(game.checkpoint(), declared);

    // The whole game, blocks included, replays to the same checkpoint.
    let report = game
        .controller
        .execute_replay(game.controller.export_replay().unwrap())
        .unwrap();
    assert_eq!(report.final_checkpoint, declared);
}

#[test]
fn a_rejected_block_answer_changes_nothing() {
    let game = two_lions_each();
    attack_with_both_lions(&game);
    let state = game.state();
    let at_request = game.checkpoint();
    let (_, request) = game.pending();
    let rejected = |player: PlayerId, answer: DecisionAnswerV2, stale: bool| {
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
        assert_eq!(game.checkpoint(), at_request);
        assert_eq!(game.pending().1, request);
    };
    // A candidate the request does not offer.
    rejected(
        P2,
        DecisionAnswerV2::SelectOne {
            candidate_id: CandidateIdV1(3),
        },
        false,
    );
    // Two blocks for one creature at once.
    rejected(
        P2,
        DecisionAnswerV2::SelectMany {
            candidate_ids: vec![CandidateIdV1(1), CandidateIdV1(2)],
        },
        false,
    );
    // An answer to another decision.
    rejected(
        P2,
        DecisionAnswerV2::SelectOne {
            candidate_id: CandidateIdV1(0),
        },
        true,
    );
    // The attacker has no decision to answer.
    rejected(
        P1,
        DecisionAnswerV2::SelectOne {
            candidate_id: CandidateIdV1(0),
        },
        false,
    );
    // The rules refuse the attacker's answer as well.
    let pending = state.execution.pending_decision.as_ref().unwrap();
    assert_eq!(
        mtgml_rules::execute_magic_response(
            &creature_game_admission(),
            &state,
            P1,
            &DecisionResponseV3 {
                schema_version: DECISION_RESPONSE_V3_SCHEMA.to_owned(),
                player_decision_id: pending.player_decision_id,
                view_sequence: pending.view_sequence,
                answer: DecisionAnswerV2::SelectOne {
                    candidate_id: CandidateIdV1(0)
                },
            },
            &EpisodeStatus::Running,
        ),
        Err(mtgml_rules::BasicLandTransitionError::InvalidSelection)
    );
    // The block is still asked, and still works.
    game.answer(pass, pass);
    assert_eq!(game.pending().0, P2);
}

/// `state` with `edit` applied to its block declaration.
fn forged(
    state: &EngineState,
    edit: impl FnOnce(
        &mut PlayerId,
        &mut Vec<GameObjectId>,
        &mut BTreeMap<GameObjectId, Option<GameObjectId>>,
    ),
) -> EngineState {
    let mut forged = state.clone();
    let record = forged.execution.continuations.values_mut().next().unwrap();
    let ContinuationPayload::BlockDeclaration {
        defender,
        pending_blockers,
        declared,
    } = &mut record.payload
    else {
        unreachable!()
    };
    edit(defender, pending_blockers, declared);
    forged
}

/// Restores into `game` a checkpoint of `state` with the status, limits and
/// execution identity of `reached`, a checkpoint of the same game: the
/// checkpoint is made from the state and then restored, as a stored one is. An
/// error is a refusal of either step; a refused restore leaves the game as it
/// was.
fn restore_state(
    game: &Game,
    reached: &EnvironmentCheckpointV8,
    state: EngineState,
) -> Result<(), String> {
    let checkpoint = EnvironmentCheckpointV8::new_for_basic_land_profile(
        &creature_game_admission(),
        state,
        reached.status.clone(),
        reached.limit_counters.clone(),
        reached.execution_identity.clone(),
    )
    .map_err(|error| format!("{error:?}"))?;
    game.controller
        .restore(checkpoint)
        .map_err(|error| format!("{error:?}"))
}

#[test]
fn a_restored_block_declaration_made_at_another_revision_is_refused() {
    // The declaration is made by the transition that opens the declare blockers
    // step and gains one answer with each revision after it, so the revision it
    // was created at is the state's, less the creatures answered. Another value
    // is a record no game makes: it would restore, and go on with a different
    // digest than the game that was played.
    let game = two_lions_each();
    let (attackers, _) = attack_with_both_lions(&game);
    let at_first = game.checkpoint();
    game.declare_block(Some(opaque_of(&at_first.state, P2, attackers[0])));
    let half = game.checkpoint();

    for (checkpoint, answered) in [(&at_first, 0), (&half, 1)] {
        let state = &checkpoint.state;
        let created = |state: &EngineState| {
            state
                .execution
                .continuations
                .values()
                .next()
                .unwrap()
                .created_at_revision
        };
        assert_eq!(created(state).0, state.revision.0 - answered);
        // The state as played restores.
        restore_state(&game, checkpoint, state.clone()).unwrap();
        assert_eq!(&game.checkpoint(), checkpoint);
        // Each other revision up to the state's own is refused: the check that
        // a record is not newer than the state does not see these.
        for revision in (0..=state.revision.0).filter(|revision| *revision != created(state).0) {
            let mut forged = state.clone();
            forged
                .execution
                .continuations
                .values_mut()
                .next()
                .unwrap()
                .created_at_revision = StateRevision(revision);
            assert!(
                restore_state(&game, checkpoint, forged).is_err(),
                "{answered} answered, created at revision {revision}"
            );
            assert_eq!(&game.checkpoint(), checkpoint);
        }
    }
}

#[test]
fn a_restored_block_declaration_the_game_could_not_have_reached_is_refused() {
    let admission = creature_game_admission();
    let running = EpisodeStatus::Running;
    let game = two_lions_each();
    let (attackers, blockers) = attack_with_both_lions(&game);
    let at_first = game.state();
    game.declare_block(Some(opaque_of(&at_first, P2, attackers[0])));
    let half = game.state();
    // Both the whole declaration and the half-declared one are states the game
    // reaches, and restore.
    for state in [&at_first, &half] {
        mtgml_rules::validate_magic_pending_request(&admission, state, &running).unwrap();
    }
    let refused = |what: &str, tampered: &EngineState| {
        assert!(
            mtgml_rules::validate_magic_pending_request(&admission, tampered, &running).is_err(),
            "{what}"
        );
    };
    let p1_creature = attackers[0];
    // An untapped land of P2: the state alone holds nothing against it.
    let p2_land = at_first
        .zones
        .objects
        .values()
        .find(|object| {
            object.controller == P2
                && !object.tapped
                && at_first.zones.locations[&object.id].zone == ZoneKind::Battlefield
                && !blockers.contains(&object.id)
        })
        .unwrap()
        .id;

    // The declaration names the defender.
    refused(
        "the attacking player as the declaring player",
        &forged(&at_first, |defender, _, _| *defender = P1),
    );
    // CR 509.1a: it names the creatures the defender controls, untapped.
    refused(
        "a creature of the attacking player",
        &forged(&at_first, |_, pending, _| pending[1] = p1_creature),
    );
    refused(
        "a land, which is no creature",
        &forged(&at_first, |_, pending, _| pending[1] = p2_land),
    );
    let mut tapped = at_first.clone();
    tapped.zones.objects.get_mut(&blockers[1]).unwrap().tapped = true;
    refused("a tapped creature that is still to be asked", &tapped);
    // Every untapped creature is asked, in the order of the defender's opaque
    // ids, and each exactly once.
    refused(
        "an untapped creature that is never asked",
        &forged(&at_first, |_, pending, _| pending.truncate(1)),
    );
    refused(
        "the creatures asked in another order",
        &forged(&at_first, |_, pending, _| pending.reverse()),
    );
    refused(
        "a creature asked twice",
        &forged(&at_first, |_, pending, _| pending[1] = pending[0]),
    );
    refused(
        "a creature both asked and answered",
        &forged(&half, |_, pending, declared| {
            declared.clear();
            pending.insert(0, blockers[0]);
            pending.truncate(2);
            declared.insert(blockers[1], None);
        }),
    );
    refused(
        "nobody left to ask",
        &forged(&half, |_, pending, declared| {
            pending.clear();
            declared.insert(blockers[1], None);
        }),
    );
    refused(
        "the first creature asked about again",
        &forged(&half, |_, pending, declared| {
            pending.insert(0, blockers[0]);
            declared.clear();
        }),
    );
    // CR 509.1a: an answered creature blocks a real attacker, or nothing.
    refused(
        "a block of a creature that does not attack",
        &forged(&half, |_, _, declared| {
            declared.insert(blockers[0], Some(blockers[1]));
        }),
    );
    refused(
        "a block of the blocker's own creature",
        &forged(&half, |_, _, declared| {
            declared.insert(blockers[0], Some(blockers[0]));
        }),
    );
    // The declaration is one declaration: nothing is blocked while it is open.
    let mut blocked = half.clone();
    let combat = blocked.combat.as_mut().unwrap();
    combat.blockers.insert(blockers[0], Some(attackers[0]));
    combat.blocked_attackers.insert(attackers[0]);
    refused("a block recorded before the declaration ends", &blocked);
    // Nobody has priority while the declaration is open, and it belongs to the
    // declare blockers step.
    let mut priority = half.clone();
    priority.core.priority = PriorityState::HeldBy {
        player: P1,
        consecutive_passes: 0,
    };
    refused("priority during the declaration", &priority);
    let mut step = half.clone();
    step.core.position = TurnPosition::Combat {
        step: CombatStep::DeclareAttackers,
    };
    refused("the declaration in the declare attackers step", &step);

    // The request is the one the continuation calls for.
    let tamper_request = |edit: &dyn Fn(&mut mtgml_decision::AuthoritativeDecisionRequest)| {
        let mut tampered = half.clone();
        edit(tampered.execution.pending_decision.as_mut().unwrap());
        tampered
    };
    refused(
        "a request for the creature that was answered",
        &tamper_request(&|request| {
            let at_first_request = at_first.execution.pending_decision.as_ref().unwrap();
            request.candidates = at_first_request.candidates.clone();
        }),
    );
    refused(
        "a request without its \"no block\" candidate",
        &tamper_request(&|request| {
            request.candidates.remove(0);
            for (index, candidate) in request.candidates.iter_mut().enumerate() {
                candidate.candidate_id = CandidateIdV1(index as u32);
            }
        }),
    );
    refused(
        "a request without one of the attackers",
        &tamper_request(&|request| {
            request.candidates.truncate(2);
        }),
    );
    refused(
        "a public request",
        &tamper_request(&|request| request.visibility = DecisionVisibility::Public),
    );
    refused(
        "a request without its continuation",
        &tamper_request(&|request| request.continuation_id = None),
    );
    refused(
        "a request of another actor",
        &tamper_request(&|request| request.actor = P1),
    );
    refused(
        "a request for several answers",
        &tamper_request(&|request| {
            request.decision_domain_v2 = DecisionDomainV2::ChooseMany {
                minimum: 0,
                maximum: 3,
            }
        }),
    );
    refused(
        "a priority request in the middle of the declaration",
        &tamper_request(&|request| request.purpose = DecisionPurposeV4::PriorityAction),
    );

    // A declaration is never part of a closed episode, and needs the rule.
    let closed = EpisodeStatus::Truncated {
        reason: TruncationReason::ExternalStop,
        players: Vec::new(),
    };
    assert!(mtgml_rules::validate_magic_pending_request(&admission, &half, &closed).is_err());
    assert!(mtgml_rules::validate_magic_pending_request(
        &common::game_admission(),
        &half,
        &running
    )
    .is_err());
}

#[test]
fn the_completing_block_answer_shows_nothing_about_blocks_to_either_player() {
    // Blocks are shown to no player until the observation step (INFORMATION_MODEL):
    // not the first answer, and not the one that completes the declaration and
    // records them. Priority passing to the attacker is public turn structure;
    // nothing else about either player's observation, visible events or
    // knowledge changes.
    let game = two_lions_each();
    let state = game.state();
    let attackers: [GameObjectId; 2] = lions_of(&state, P1).try_into().unwrap();
    let own = |attacker: GameObjectId| Some(opaque_of(&state, P2, attacker));

    // Positive control: the same comparison does see the attack, which is
    // public.
    let before_attack = game.seen_by_both();
    let step = game.declare_attackers(&attackers.map(|attacker| opaque_of(&state, P1, attacker)));
    let after_attack = game.seen_by_both();
    assert!(!step.observed_events.is_empty());
    for (before, after) in before_attack.iter().zip(&after_attack) {
        assert_ne!(before.sequence, after.sequence);
        assert_ne!(before.observation.attacking, after.observation.attacking);
    }
    game.answer(pass, pass);
    game.answer(pass, pass);
    assert_eq!(game.pending().0, P2);

    // The first answer shows nothing, and neither does the last.
    let before_first = game.seen_by_both();
    game.declare_block(own(attackers[0]));
    let before = game.seen_by_both();
    assert_eq!(before_first, before);
    assert!(game.state().combat.as_ref().unwrap().blockers.is_empty());
    let (_, step) = game.declare_block(own(attackers[1]));
    let after = game.seen_by_both();
    let combat = game.state().combat.clone().unwrap();
    assert_eq!(combat.blockers.len(), 2, "the blocks are recorded");
    assert_eq!(combat.blocked_attackers.len(), 2);
    assert_eq!(game.pending().0, P1, "the attacker has priority");

    assert!(step.observed_events.is_empty());
    assert_eq!(before, after);
    // The attacker's own step shows nothing new either.
    assert_eq!(
        game.endpoint(P1)
            .visible_decision()
            .unwrap()
            .unwrap()
            .purpose,
        DecisionPurposeV4::PriorityAction
    );
}

#[test]
fn two_creatures_may_block_one_attacker() {
    // CR 509.1a, 509.1h: any number of creatures may block one attacker, which
    // is blocked; the other attacker stays unblocked.
    let game = two_lions_each();
    let (attackers, blockers) = attack_with_both_lions(&game);
    let state = game.state();
    let own = |attacker: GameObjectId| Some(opaque_of(&state, P2, attacker));
    game.declare_block(own(attackers[0]));
    game.declare_block(own(attackers[0]));

    let declared = game.state();
    let combat = declared.combat.as_ref().unwrap();
    assert_eq!(
        combat.blockers,
        BTreeMap::from([
            (blockers[0], Some(attackers[0])),
            (blockers[1], Some(attackers[0]))
        ])
    );
    assert_eq!(combat.blocked_attackers, BTreeSet::from([attackers[0]]));
    assert!(declared.execution.continuations.is_empty());
    assert_eq!(game.pending().0, P1);
    game.controller.restore(game.checkpoint()).unwrap();
}

#[test]
fn a_3_3_blocked_by_a_2_1_kills_it_and_survives() {
    // CR 510.1c, 510.1d, 510.2, 704.5g: P1's Hill Giant (3/3) attacks and P2's
    // Savannah Lions (2/1) blocks it. The Giant deals 3 damage to the Lions,
    // which is lethal; the Lions deals 2 to the Giant, which is not. The Lions
    // is destroyed and goes to P2's graveyard as a new object (CR 400.7), and
    // both players see it move. The Giant survives with 2 damage marked, and
    // stays an attacking, blocked creature until combat ends (CR 509.1h).
    let [_, _, giant] = creature_definitions();
    let (game, giant_object, lions_object) = attacker_against_a_lions(giant, 4);
    attack_and_block(&game, giant_object);
    let before = game.state();
    let blocked = before.combat.clone().unwrap();
    assert_eq!(
        blocked.blockers,
        BTreeMap::from([(lions_object, Some(giant_object))])
    );
    let card = before.zones.objects[&lions_object].physical_card;
    let lives = before.core.players.clone();

    let (product, observed) = product_and_observations(&before, pass_in(&before));
    let (actor, step) = game.answer(pass, pass);
    assert_eq!(actor, P2);
    assert_eq!(step.observed_events, observed[&P2]);
    let after = game.state();
    assert_eq!(after, product.next_state);

    // The Lions is gone, and its card is in P2's graveyard as another object.
    assert!(!after.zones.objects.contains_key(&lions_object));
    let graveyard: Vec<_> = after
        .zones
        .locations
        .iter()
        .filter(|(_, location)| location.zone == ZoneKind::Graveyard)
        .map(|(object, location)| (*object, location.player))
        .collect();
    let [(dead, Some(owner))] = graveyard[..] else {
        panic!("one card in a graveyard: {graveyard:?}")
    };
    assert_eq!(owner, P2);
    assert_ne!(dead, lions_object);
    assert_eq!(after.zones.objects[&dead].card_definition, lions());
    assert_eq!(after.zones.objects[&dead].physical_card, card);
    assert!(after.card_rules.turn_history.players[&P2].permanent_card_to_graveyard);
    assert!(!after.card_rules.turn_history.players[&P1].permanent_card_to_graveyard);
    // The new object is not a permanent, and has no damage marked.
    let permanents = &after.card_rules.permanents.permanents;
    assert!(!permanents.contains_key(&lions_object) && !permanents.contains_key(&dead));

    // The Giant survives with the Lions' 2 damage marked on it, and neither
    // player lost life.
    assert_eq!(
        after.zones.locations[&giant_object].zone,
        ZoneKind::Battlefield
    );
    assert_eq!(permanents[&giant_object].marked_damage, 2);
    assert_eq!(after.core.players, lives);
    // It is still attacking, and still blocked, with its blocker gone.
    let combat = after.combat.clone().unwrap();
    assert!(combat.damage_step_completed);
    assert_eq!(combat.attackers, vec![giant_object]);
    assert_eq!(combat.blocked_attackers, BTreeSet::from([giant_object]));
    assert!(combat.blockers.is_empty());

    // Both players see the Lions go from the battlefield to the graveyard, and
    // follow it: it keeps the opaque id they knew it by. They see nothing
    // else of the step: the damage and the blocks are not shown yet.
    for player in [P1, P2] {
        let opaque = opaque_of(&before, player, lions_object);
        assert_eq!(opaque_of(&after, player, dead), opaque, "{player:?}");
        assert_eq!(
            observed_moves(&observed[&player]),
            [(
                Some(opaque),
                Some(opaque),
                ZoneKind::Battlefield,
                ZoneKind::Graveyard
            )],
            "{player:?}"
        );
        assert_eq!(observed[&player].len(), 1, "{player:?}");
    }

    // CR 510.3: the active player has priority in the combat damage step.
    assert_eq!(
        after.core.position,
        TurnPosition::Combat {
            step: CombatStep::CombatDamage
        }
    );
    assert_eq!(game.pending().0, P1);

    // Combat ends with the end of combat step (CR 511.3): the Giant is still
    // attacking and still has its damage until then, and the damage until the
    // cleanup step (CR 120.6, 514.2), which the tests of the cleanup step go
    // through.
    let still_in_combat = |state: &EngineState| state.combat == after.combat;
    game.answer(pass, pass);
    game.answer(pass, pass);
    let end_of_combat = game.state();
    assert_eq!(
        end_of_combat.core.position,
        TurnPosition::Combat {
            step: CombatStep::EndOfCombat
        }
    );
    assert!(still_in_combat(&end_of_combat));
    game.answer(pass, pass);
    game.answer(pass, pass);
    let postcombat = game.state();
    assert_eq!(postcombat.core.position, TurnPosition::PostcombatMain);
    assert!(postcombat.combat.is_none());
    for state in [&end_of_combat, &postcombat] {
        assert_eq!(
            state.card_rules.permanents.permanents[&giant_object].marked_damage,
            2
        );
    }

    // The whole game, the death included, replays to the same checkpoint.
    let checkpoint = game.checkpoint();
    let report = game
        .controller
        .execute_replay(game.controller.export_replay().unwrap())
        .unwrap();
    assert_eq!(report.final_checkpoint, checkpoint);
}

#[test]
fn a_2_2_and_a_2_1_that_block_each_other_both_die_one_to_each_graveyard() {
    // CR 510.2, 704.3, 704.5g: P1's Gray Ogre (2/2) attacks and P2's Savannah
    // Lions (2/1) blocks it. Each deals 2 damage to the other, which is lethal
    // to both (the Ogre has exactly 2 damage on a toughness of 2). Both are
    // destroyed as one state-based action, each to its owner's graveyard.
    let [_, ogre, _] = creature_definitions();
    let (game, ogre_object, lions_object) = attacker_against_a_lions(ogre, 3);
    attack_and_block(&game, ogre_object);
    let before = game.state();
    let (product, observed) = product_and_observations(&before, pass_in(&before));
    game.answer(pass, pass);
    let after = game.state();
    assert_eq!(after, product.next_state);

    for dead in [ogre_object, lions_object] {
        assert!(!after.zones.objects.contains_key(&dead));
    }
    let in_graveyard = |owner: PlayerId| -> Vec<GameObjectId> {
        after
            .zones
            .locations
            .iter()
            .filter(|(_, location)| {
                location.zone == ZoneKind::Graveyard && location.player == Some(owner)
            })
            .map(|(object, _)| *object)
            .collect()
    };
    let [ogre_card] = in_graveyard(P1)[..] else {
        panic!("P1's graveyard holds the Ogre")
    };
    let [lions_card] = in_graveyard(P2)[..] else {
        panic!("P2's graveyard holds the Lions")
    };
    assert_eq!(after.zones.objects[&ogre_card].card_definition, ogre);
    assert_eq!(after.zones.objects[&lions_card].card_definition, lions());
    for owner in [P1, P2] {
        assert!(after.card_rules.turn_history.players[&owner].permanent_card_to_graveyard);
    }
    // Neither the objects that died nor the new ones are permanents.
    for object in [ogre_object, lions_object, ogre_card, lions_card] {
        assert!(!after.card_rules.permanents.permanents.contains_key(&object));
    }
    // Nothing of them is left in combat.
    let combat = after.combat.clone().unwrap();
    assert!(combat.attackers.is_empty() && combat.blockers.is_empty());
    assert!(combat.blocked_attackers.is_empty() && combat.damage_step_completed);

    // One state-based action: both, in object order, each for lethal damage.
    let mut in_object_order = [ogre_object, lions_object];
    in_object_order.sort();
    let destroyed = |object| mtgml_state::SbaSelectedActionV1::ObjectToOwnerGraveyard {
        object,
        causes: vec![mtgml_state::SbaObjectCauseV1::LethalDamage],
    };
    let performed: Vec<_> = product
        .events
        .iter()
        .filter_map(|event| match &event.event {
            AuthoritativeRuleEventKind::StateBasedActionsApplied { actions } => Some(actions),
            _ => None,
        })
        .collect();
    assert_eq!(performed, [&in_object_order.map(destroyed).to_vec()]);
    // Both players see both deaths, in that order, and follow each card.
    for player in [P1, P2] {
        let opaque = |object| Some(opaque_of(&before, player, object));
        assert_eq!(
            observed_moves(&observed[&player]),
            in_object_order.map(|object| (
                opaque(object),
                opaque(object),
                ZoneKind::Battlefield,
                ZoneKind::Graveyard
            )),
            "{player:?}"
        );
    }
    assert_eq!(game.pending().0, P1);
    // A combat with nobody left in it restores (CR 506.4).
    game.controller.restore(game.checkpoint()).unwrap();
}

#[test]
fn a_restored_combat_after_a_blocker_died_continues_identically() {
    // CR 509.1h: the Hill Giant stays blocked after the Savannah Lions that
    // blocked it was destroyed. A checkpoint taken from the damage step to the
    // end step restores, and the same answers lead to the same checkpoints.
    let [_, _, giant] = creature_definitions();
    let (game, giant_object, lions_object) = attacker_against_a_lions(giant, 4);
    attack_and_block(&game, giant_object);
    let mut checkpoints = Vec::new();
    // P2 passes, and the damage step begins; then each step ends by two passes.
    game.answer(pass, pass);
    for _ in 0..3 {
        checkpoints.push(game.checkpoint());
        game.answer(pass, pass);
        game.answer(pass, pass);
    }
    checkpoints.push(game.checkpoint());
    let positions: Vec<_> = checkpoints
        .iter()
        .map(|checkpoint| checkpoint.state.core.position)
        .collect();
    assert_eq!(
        positions,
        [
            TurnPosition::Combat {
                step: CombatStep::CombatDamage
            },
            TurnPosition::Combat {
                step: CombatStep::EndOfCombat
            },
            TurnPosition::PostcombatMain,
            TurnPosition::Ending {
                step: mtgml_state::EndingStep::EndStep
            },
        ]
    );
    for checkpoint in &checkpoints {
        let state = &checkpoint.state;
        assert!(!state.zones.objects.contains_key(&lions_object));
        assert_eq!(
            state.card_rules.permanents.permanents[&giant_object].marked_damage,
            2
        );
    }
    let last = game.checkpoint();

    // Each restores, and the answers that followed it lead to the next one.
    for window in checkpoints.windows(2) {
        game.controller.restore(window[0].clone()).unwrap();
        assert_eq!(game.checkpoint(), window[0]);
        game.answer(pass, pass);
        game.answer(pass, pass);
        assert_eq!(game.checkpoint(), window[1]);
    }
    assert_eq!(game.checkpoint(), last);
}

/// P1 casts a Savannah Lions on its first turn, and P2 a Hill Giant on its
/// fourth, with the four lands it plays. On turn 9 P1 attacks with the Lions
/// and P2 blocks it with the Giant. P2 holds priority in the declare blockers
/// step: its pass opens the combat damage step. Returns the game, the Lions
/// and the Giant.
fn a_lions_blocked_by_a_giant() -> (Game, GameObjectId, GameObjectId) {
    let (mountain, plains) = land_definitions();
    let [_, _, giant] = creature_definitions();
    let hand = [vec![mountain; 4], vec![giant]].concat();
    let game = Game::with_hands([vec![plains, lions()], hand]);
    cast_lions(&game);
    game.run_until(start_of_main_phase(8));
    cast_creature(&game, 4);
    game.run_until(at_attackers(9));
    let state = game.state();
    let [lions_object]: [GameObjectId; 1] = lions_of(&state, P1).try_into().unwrap();
    let [giant_object]: [GameObjectId; 1] = creatures_of(&state, P2, giant).try_into().unwrap();
    attack_and_block(&game, lions_object);
    (game, lions_object, giant_object)
}

#[test]
fn a_blocker_whose_attacker_died_is_still_blocking_until_combat_ends() {
    // CR 509.1g, 506.4: P1's Savannah Lions (2/1) attacks, and P2's Hill Giant
    // (3/3) blocks it. The Lions is destroyed in the combat damage step and so
    // removed from combat. The Giant survives with 2 damage marked, and it
    // remains a blocking creature, which blocks nothing, until the combat
    // ends: the combat records it with no attacker.
    let (game, lions_object, giant_object) = a_lions_blocked_by_a_giant();
    let combat = game.state().combat.unwrap();
    assert_eq!(
        combat.blockers,
        BTreeMap::from([(giant_object, Some(lions_object))])
    );

    game.answer(pass, pass);
    let at_damage = game.checkpoint();
    let state = &at_damage.state;
    assert_eq!(
        state.core.position,
        TurnPosition::Combat {
            step: CombatStep::CombatDamage
        }
    );
    assert!(!state.zones.objects.contains_key(&lions_object));
    let [dead] = graveyard_of(state, P1)[..] else {
        panic!("P1's graveyard holds the Lions")
    };
    assert_eq!(state.zones.objects[&dead].card_definition, lions());
    assert_eq!(marked(state, giant_object), 2);
    let combat = state.combat.clone().unwrap();
    assert!(combat.damage_step_completed);
    assert!(combat.attackers.is_empty() && combat.blocked_attackers.is_empty());
    assert_eq!(combat.blockers, BTreeMap::from([(giant_object, None)]));
    game.controller.restore(at_damage.clone()).unwrap();
    assert_eq!(game.checkpoint(), at_damage);

    // The end of combat step: the Giant still blocks nothing, and still has its
    // damage. The checkpoint restores, and the answers that follow it lead to
    // the same state as before.
    game.pass_until(|state| {
        state.core.position
            == TurnPosition::Combat {
                step: CombatStep::EndOfCombat,
            }
    });
    let at_end_of_combat = game.checkpoint();
    let state = &at_end_of_combat.state;
    assert_eq!(marked(state, giant_object), 2);
    assert_eq!(
        state.combat.as_ref().unwrap().blockers,
        BTreeMap::from([(giant_object, None)])
    );
    game.answer(pass, pass);
    game.answer(pass, pass);
    let after = game.checkpoint();
    game.controller.restore(at_end_of_combat.clone()).unwrap();
    assert_eq!(game.checkpoint(), at_end_of_combat);
    game.answer(pass, pass);
    game.answer(pass, pass);
    assert_eq!(game.checkpoint(), after);

    // The combat phase has ended, and with it the combat: nothing blocks.
    assert_eq!(after.state.core.position, TurnPosition::PostcombatMain);
    assert_eq!(after.state.combat, None);
    assert_eq!(marked(&after.state, giant_object), 2);
}

#[test]
fn a_restored_blocked_attacker_has_its_blocker_until_the_damage_is_dealt() {
    // CR 509.1h: an attacker is blocked by the creature that blocks it. The
    // only way a blocker leaves combat is to die in the state-based actions
    // after the combat damage step (CR 704.5g, 506.4), and no instant can
    // remove one before: with the damage still to be dealt, a blocked attacker
    // whose blocker is gone is a state no game rests in, and restore refuses
    // it. After the damage it is the state of the Hill Giant that killed its
    // blocker.
    let [_, _, giant] = creature_definitions();
    let (game, giant_object, lions_object) = attacker_against_a_lions(giant, 4);
    attack_and_block(&game, giant_object);
    let blocked = game.checkpoint();
    let combat = blocked.state.combat.as_ref().unwrap();
    assert_eq!(
        combat.blockers.get(&lions_object),
        Some(&Some(giant_object))
    );
    assert!(combat.blocked_attackers.contains(&giant_object) && !combat.damage_step_completed);
    restore_state(&game, &blocked, blocked.state.clone()).unwrap();
    assert_eq!(game.checkpoint(), blocked);

    // The blocker is gone before the damage: refused, and the game is as it was.
    let mut lost = blocked.state.clone();
    lost.combat.as_mut().unwrap().blockers.clear();
    assert!(restore_state(&game, &blocked, lost).is_err());
    assert_eq!(game.checkpoint(), blocked);

    // The damage is dealt, and the Lions is destroyed: the Giant stays blocked
    // with no blocker left, and that restores.
    game.answer(pass, pass);
    let damaged = game.checkpoint();
    let combat = damaged.state.combat.as_ref().unwrap();
    assert!(combat.blockers.is_empty() && combat.damage_step_completed);
    assert!(combat.blocked_attackers.contains(&giant_object));
    restore_state(&game, &damaged, damaged.state.clone()).unwrap();
    assert_eq!(game.checkpoint(), damaged);
}

#[test]
fn a_restored_end_of_combat_has_dealt_the_damage_of_its_attackers() {
    // CR 508.8, 510.1, 510.2: with attackers declared the combat damage step
    // always runs, so a combat that reaches the end of combat step with
    // attackers still in it has dealt its damage. Without attackers the step
    // is skipped, and a combat whose attackers all died has dealt its damage
    // too: both are states the game reaches, and restore them.
    let at_end_of_combat = |state: &EngineState| {
        state.core.position
            == TurnPosition::Combat {
                step: CombatStep::EndOfCombat,
            }
    };
    let admission = creature_game_admission();
    let restore = |game: &Game, state: EngineState| {
        let reached = game.checkpoint();
        let checkpoint = EnvironmentCheckpointV8::new_for_basic_land_profile(
            &admission,
            state,
            reached.status.clone(),
            reached.limit_counters.clone(),
            reached.execution_identity.clone(),
        )
        .map_err(|error| format!("{error:?}"))?;
        game.controller
            .restore(checkpoint)
            .map_err(|error| format!("{error:?}"))
    };

    // P1's Hill Giant attacks, and P2 does not block with its Savannah Lions.
    // The Giant is still attacking at the end of combat and nothing is marked
    // on any creature, so only the flag says that its damage was dealt.
    let [_, _, giant] = creature_definitions();
    let (game, giant_object, _) = attacker_against_a_lions(giant, 4);
    let state = game.state();
    game.declare_attackers(&[opaque_of(&state, P1, giant_object)]);
    game.pass_until(at_end_of_combat);
    let reached = game.checkpoint();
    let combat = reached.state.combat.as_ref().unwrap();
    assert_eq!(combat.attackers, [giant_object]);
    assert!(combat.damage_step_completed);
    assert!(combat.blockers.is_empty() && combat.blocked_attackers.is_empty());
    assert!(reached
        .state
        .card_rules
        .permanents
        .permanents
        .values()
        .all(|permanent| permanent.marked_damage == 0));
    restore(&game, reached.state.clone()).unwrap();
    assert_eq!(game.checkpoint(), reached);

    // The same combat with its damage undealt is not one the game makes.
    let mut undealt = reached.state.clone();
    undealt.combat.as_mut().unwrap().damage_step_completed = false;
    assert!(restore(&game, undealt).is_err());
    assert_eq!(game.checkpoint(), reached);

    // Both creatures of a fight die: the combat has no attacker left, and its
    // damage was dealt.
    let [_, ogre, _] = creature_definitions();
    let (game, ogre_object, _) = attacker_against_a_lions(ogre, 3);
    attack_and_block(&game, ogre_object);
    game.answer(pass, pass);
    game.pass_until(at_end_of_combat);
    let reached = game.checkpoint();
    let combat = reached.state.combat.as_ref().unwrap();
    assert!(combat.attackers.is_empty() && combat.damage_step_completed);
    restore(&game, reached.state.clone()).unwrap();
    assert_eq!(game.checkpoint(), reached);

    // No attacker is declared: the combat has no damage step, and its flag
    // stays false.
    let (game, _, _) = attacker_against_a_lions(giant, 4);
    game.pass_until(at_end_of_combat);
    let reached = game.checkpoint();
    let combat = reached.state.combat.as_ref().unwrap();
    assert!(combat.attackers.is_empty() && !combat.damage_step_completed);
    restore(&game, reached.state.clone()).unwrap();
    assert_eq!(game.checkpoint(), reached);
}

/// The cards in `owner`'s graveyard, top first.
fn graveyard_of(state: &EngineState, owner: PlayerId) -> Vec<GameObjectId> {
    let key = ZoneLocation {
        zone: ZoneKind::Graveyard,
        player: Some(owner),
        position: ZonePosition::Top { offset: 0 },
        visibility: VisibilityPartition::Public,
        partition: None,
    }
    .key();
    state
        .zones
        .ordered_zones
        .get(&key)
        .cloned()
        .unwrap_or_default()
}

/// The answer to a graveyard order request that puts the cards `top_to_bottom`
/// (the ids their owner knows them by) in that order.
fn order_answer(
    request: &PlayerDecisionRequestV4,
    top_to_bottom: &[OpaqueObjectId],
) -> DecisionAnswerV2 {
    DecisionAnswerV2::Order {
        candidate_ids: top_to_bottom
            .iter()
            .map(|wanted| {
                request
                    .candidates
                    .iter()
                    .find(|candidate| {
                        matches!(candidate.intent,
                            CandidateIntent::SelectObject { object } if object == *wanted)
                    })
                    .unwrap_or_else(|| panic!("the request does not offer {wanted:?}"))
                    .candidate_id
            })
            .collect(),
    }
}

/// What `player` is told, in the observation, of the order the owners are
/// arranging their graveyards in.
fn pending_ordering(game: &Game, player: PlayerId) -> Option<MagicPendingSbaOrdering> {
    game.seen(player).observation.pending_sba_ordering
}

/// The state-based action that destroys `object`, which was dealt lethal damage.
fn destroyed(object: GameObjectId) -> SbaSelectedActionV1 {
    SbaSelectedActionV1::ObjectToOwnerGraveyard {
        object,
        causes: vec![SbaObjectCauseV1::LethalDamage],
    }
}

/// The Gray Ogre and the Hill Giant of P1 against two Savannah Lions of P2.
struct Fight {
    game: Game,
    ogre: GameObjectId,
    giant: GameObjectId,
    /// P2's Lions in the order P2 is asked about them: the one that blocks the
    /// Ogre, then the one that blocks the Giant.
    lions: [GameObjectId; 2],
}

/// P2 casts a Savannah Lions on each of its first two turns (2 and 4); P1 casts
/// a Gray Ogre on turn 5 and a Hill Giant on turn 7. On turn 9 P1 attacks with
/// both and P2 blocks the Ogre with the Lions it is asked about first and the
/// Giant with the other. P2 holds priority in the declare blockers step: its
/// pass opens the combat damage step.
fn ogre_and_giant_blocked_by_two_lions() -> Fight {
    let [_, ogre, giant] = creature_definitions();
    let (mountain, plains) = land_definitions();
    let game = Game::with_hands([
        vec![mountain, mountain, mountain, mountain, ogre, giant],
        vec![plains, lions(), plains, lions()],
    ]);
    game.run_until(start_of_main_phase(2));
    cast_lions(&game);
    game.run_until(start_of_main_phase(4));
    cast_lions(&game);
    game.run_until(start_of_main_phase(5));
    cast_creature(&game, 3);
    game.run_until(start_of_main_phase(7));
    cast_creature(&game, 4);
    game.run_until(at_attackers(9));

    let state = game.state();
    let [ogre_object]: [GameObjectId; 1] = creatures_of(&state, P1, ogre).try_into().unwrap();
    let [giant_object]: [GameObjectId; 1] = creatures_of(&state, P1, giant).try_into().unwrap();
    let lions: [GameObjectId; 2] = in_opaque_order(&state, P2, lions_of(&state, P2))
        .try_into()
        .unwrap();
    game.declare_attackers(&[
        opaque_of(&state, P1, ogre_object),
        opaque_of(&state, P1, giant_object),
    ]);
    game.answer(pass, pass);
    game.answer(pass, pass);
    game.declare_block(Some(opaque_of(&state, P2, ogre_object)));
    game.declare_block(Some(opaque_of(&state, P2, giant_object)));
    game.answer(pass, pass);
    assert_eq!(game.pending().0, P2);
    assert_eq!(
        game.state().core.position,
        TurnPosition::Combat {
            step: CombatStep::DeclareBlockers
        }
    );
    Fight {
        game,
        ogre: ogre_object,
        giant: giant_object,
        lions,
    }
}

#[test]
fn two_creatures_dying_together_ask_their_owner_for_the_order() {
    // CR 404.3, 704.3, 704.5g: P1 attacks with its Gray Ogre (2/2) and Hill
    // Giant (3/3); P2 blocks the Ogre with a Savannah Lions (2/1) and the Giant
    // with another. Both Lions die (2 and 3 damage on a toughness of 1); the
    // Ogre dies (2 damage on a toughness of 2); the Giant survives with 2
    // damage marked. P1 has one dying card and is not asked how to arrange it.
    // P2 owns two and orders them: that decision is P2's alone, and the
    // creatures die when it is answered, not before.
    let Fight {
        game,
        ogre,
        giant,
        lions: [lions_a, lions_b],
    } = ogre_and_giant_blocked_by_two_lions();
    game.answer(pass, pass);
    let at_order = game.checkpoint();
    let before = &at_order.state;

    // The request: P2's own, over its two Lions, in the order of its opaque ids.
    let (actor, request) = game.pending();
    assert_eq!(actor, P2);
    assert_eq!(game.endpoint(P1).visible_decision().unwrap(), None);
    assert_eq!(request.purpose, DecisionPurposeV4::SbaGraveyardOrder);
    assert_eq!(request.visibility, DecisionVisibility::ActingPlayerOnly);
    assert_eq!(
        request.decision_domain_v2,
        DecisionDomainV2::Order {
            minimum: 2,
            maximum: 2
        }
    );
    assert_eq!(
        game.offered(),
        [lions_a, lions_b].map(|lions| CandidateIntent::SelectObject {
            object: opaque_of(before, P2, lions)
        })
    );

    // Nothing has died yet, and every creature carries the damage it was dealt.
    for creature in [ogre, giant, lions_a, lions_b] {
        assert_eq!(
            before.zones.locations[&creature].zone,
            ZoneKind::Battlefield
        );
    }
    let marked =
        |creature: GameObjectId| before.card_rules.permanents.permanents[&creature].marked_damage;
    assert_eq!([ogre, giant, lions_a, lions_b].map(marked), [2, 2, 2, 3]);
    // The batch waits in a continuation with P2 as the only owner asked.
    let mut dying = [ogre, lions_a, lions_b];
    dying.sort();
    let [(_, record)] = before.execution.continuations.iter().collect::<Vec<_>>()[..] else {
        panic!("one continuation")
    };
    assert_eq!(
        record.payload,
        ContinuationPayload::MagicSbaGraveyardOrderV1 {
            round_start_revision: StateRevision(before.revision.0 - 1),
            selected_sba_actions: dying.map(destroyed).to_vec(),
            apnap_owners: vec![P2],
            next_owner_index: 0,
            completed_owner_orders: Vec::new(),
        }
    );
    // Both players see that P2 is ordering, and nothing of how.
    for player in [P1, P2] {
        assert_eq!(
            pending_ordering(&game, player),
            Some(MagicPendingSbaOrdering {
                completed_orders: Vec::new(),
                next_order_owner: P2
            }),
            "{player:?}"
        );
    }

    // Whichever way P2 answers, its graveyard lies that way, top to bottom.
    // The batch moves in object order, so only one of the two answers is the
    // engine's own order; the other shows the answer decides.
    let mut by_object = [lions_a, lions_b];
    by_object.sort();
    let mut reversed = by_object;
    reversed.reverse();
    for top_to_bottom in [by_object, reversed] {
        game.controller.restore(at_order.clone()).unwrap();
        let wanted = top_to_bottom.map(|lions| opaque_of(before, P2, lions));
        let answer = order_answer(&request, &wanted);
        let (product, observed) = product_and_observations(before, answer.clone());
        let (actor, step) = game.submit(answer);
        assert_eq!(actor, P2);
        assert_eq!(step.observed_events, observed[&P2]);
        let after = game.state();
        assert_eq!(after, product.next_state);

        // P2's graveyard is in its order, and its cards are the Lions.
        let pile = graveyard_of(&after, P2);
        assert_eq!(
            pile.iter()
                .map(|card| opaque_of(&after, P2, *card))
                .collect::<Vec<_>>(),
            wanted
        );
        assert!(pile
            .iter()
            .all(|card| after.zones.objects[card].card_definition == lions()));
        // The Ogre is in P1's graveyard, alone.
        let [ogre_card] = graveyard_of(&after, P1)[..] else {
            panic!("P1's graveyard holds the Ogre")
        };
        assert_eq!(
            opaque_of(&after, P1, ogre_card),
            opaque_of(before, P1, ogre)
        );
        for dead in [ogre, lions_a, lions_b] {
            assert!(!after.zones.objects.contains_key(&dead));
        }
        // The Giant survives with 2 damage marked, attacking and still blocked.
        assert_eq!(
            after.card_rules.permanents.permanents[&giant].marked_damage,
            2
        );
        let combat = after.combat.clone().unwrap();
        assert_eq!(combat.attackers, vec![giant]);
        assert_eq!(combat.blocked_attackers, BTreeSet::from([giant]));
        assert!(combat.blockers.is_empty());
        // The order is in the events, then the whole batch in one.
        let chosen: Vec<_> = product
            .events
            .iter()
            .filter_map(|event| match &event.event {
                AuthoritativeRuleEventKind::SbaGraveyardOrderChosen {
                    owner,
                    top_to_bottom,
                    ..
                } => Some((*owner, top_to_bottom.clone())),
                _ => None,
            })
            .collect();
        assert_eq!(chosen, [(P2, top_to_bottom.to_vec())]);
        let applied: Vec<_> = product
            .events
            .iter()
            .filter_map(|event| match &event.event {
                AuthoritativeRuleEventKind::StateBasedActionsApplied { actions } => Some(actions),
                _ => None,
            })
            .collect();
        assert_eq!(applied, [&dying.map(destroyed).to_vec()]);
        // Both players see the three deaths, and follow each card.
        for player in [P1, P2] {
            let mut moves = observed_moves(&observed[&player]);
            moves.sort();
            let mut expected: Vec<_> = dying
                .iter()
                .map(|dead| {
                    let opaque = Some(opaque_of(before, player, *dead));
                    (opaque, opaque, ZoneKind::Battlefield, ZoneKind::Graveyard)
                })
                .collect();
            expected.sort();
            assert_eq!(moves, expected, "{player:?}");
        }
        // The decision and the continuation are over, and the active player
        // has priority in the combat damage step (CR 510.3).
        assert!(after.execution.continuations.is_empty());
        assert_eq!(game.pending().0, P1);
        assert_eq!(
            after.core.position,
            TurnPosition::Combat {
                step: CombatStep::CombatDamage
            }
        );
        for player in [P1, P2] {
            assert_eq!(pending_ordering(&game, player), None, "{player:?}");
        }
        game.controller.restore(game.checkpoint()).unwrap();
    }
}

/// P1 attacks with both of its Lions and P2 blocks each with one of its own:
/// all four die. Returns the game at the combat damage step, with P1, the
/// active player, asked to order its two.
fn four_lions_dying_together() -> Game {
    let game = two_lions_each();
    let (attackers, _) = attack_with_both_lions(&game);
    let state = game.state();
    game.declare_block(Some(opaque_of(&state, P2, attackers[0])));
    game.declare_block(Some(opaque_of(&state, P2, attackers[1])));
    game.answer(pass, pass);
    game.answer(pass, pass);
    game
}

/// The Lions of `owner`, whose order the pending request asks for, from the
/// higher object id to the lower, and the answer that puts them in that order.
fn higher_object_first(game: &Game, owner: PlayerId) -> (Vec<GameObjectId>, DecisionAnswerV2) {
    let state = game.state();
    let (actor, request) = game.pending();
    assert_eq!(actor, owner);
    let mut lions = lions_of(&state, owner);
    lions.sort();
    lions.reverse();
    let wanted: Vec<_> = lions
        .iter()
        .map(|lions| opaque_of(&state, owner, *lions))
        .collect();
    (lions, order_answer(&request, &wanted))
}

#[test]
fn owners_order_in_turn_order_and_the_second_sees_the_first() {
    // CR 101.4, 404.3: two cards of each owner die together. The active player
    // orders its two first, then the other player its two (APNAP), who knows
    // the first order when it answers (CR 101.4b). The creatures die with the
    // last answer, and each graveyard is in its owner's order.
    let game = four_lions_dying_together();
    let at_first = game.checkpoint();
    assert_eq!(game.pending().0, P1);
    let p1_lions = lions_of(&at_first.state, P1);
    let p2_lions = lions_of(&at_first.state, P2);
    assert_eq!((p1_lions.len(), p2_lions.len()), (2, 2));
    assert_eq!(game.endpoint(P2).visible_decision().unwrap(), None);
    let [(_, record)] = at_first
        .state
        .execution
        .continuations
        .iter()
        .collect::<Vec<_>>()[..]
    else {
        panic!("one continuation")
    };
    let ContinuationPayload::MagicSbaGraveyardOrderV1 {
        apnap_owners,
        next_owner_index,
        selected_sba_actions,
        ..
    } = &record.payload
    else {
        panic!("a graveyard order")
    };
    assert_eq!((apnap_owners.clone(), *next_owner_index), (vec![P1, P2], 0));
    // One batch holding all four.
    let mut dying = [p1_lions.clone(), p2_lions.clone()].concat();
    dying.sort();
    assert_eq!(
        selected_sba_actions,
        &dying.iter().copied().map(destroyed).collect::<Vec<_>>()
    );

    // P1 answers; nothing dies, and nothing is seen of it yet.
    let (p1_order, answer) = higher_object_first(&game, P1);
    let (_, observed) = product_and_observations(&at_first.state, answer.clone());
    assert!(observed.values().all(|events| events.is_empty()));
    let (actor, step) = game.submit(answer);
    assert_eq!(actor, P1);
    assert!(step.observed_events.is_empty());
    let at_second = game.checkpoint();
    assert_eq!(at_second.state.revision.0, at_first.state.revision.0 + 1);
    assert_eq!(game.pending().0, P2);
    assert_eq!(game.endpoint(P1).visible_decision().unwrap(), None);
    for creature in &dying {
        assert!(at_second.state.zones.objects.contains_key(creature));
    }
    // Each player is told P1's order in the ids it knows the cards by, and that
    // P2 is next.
    for player in [P1, P2] {
        assert_eq!(
            pending_ordering(&game, player),
            Some(MagicPendingSbaOrdering {
                completed_orders: vec![MagicCompletedOrder {
                    owner: P1,
                    ordered_objects: p1_order
                        .iter()
                        .map(|lions| opaque_of(&at_second.state, player, *lions))
                        .collect(),
                }],
                next_order_owner: P2
            }),
            "{player:?}"
        );
    }

    // P2 answers, and the batch applies.
    let (p2_order, answer) = higher_object_first(&game, P2);
    let (actor, _) = game.submit(answer);
    assert_eq!(actor, P2);
    let after = game.state();
    assert_eq!(after.revision.0, at_second.state.revision.0 + 1);
    assert!(after.execution.continuations.is_empty());
    for (owner, order) in [(P1, &p1_order), (P2, &p2_order)] {
        assert_eq!(
            graveyard_of(&after, owner)
                .iter()
                .map(|card| opaque_of(&after, owner, *card))
                .collect::<Vec<_>>(),
            order
                .iter()
                .map(|lions| opaque_of(&at_second.state, owner, *lions))
                .collect::<Vec<_>>(),
            "{owner:?}"
        );
    }
    for creature in &dying {
        assert!(!after.zones.objects.contains_key(creature));
    }
    assert_eq!(game.pending().0, P1);
    for player in [P1, P2] {
        assert_eq!(pending_ordering(&game, player), None, "{player:?}");
    }
}

#[test]
fn a_restored_graveyard_order_checkpoint_continues_identically() {
    // Each order checkpoint of the two owners restores, and the same answer
    // leads to the same next checkpoint; the whole game replays to the end.
    let game = four_lions_dying_together();
    let at_first = game.checkpoint();
    let (_, first_answer) = higher_object_first(&game, P1);
    game.submit(first_answer.clone());
    let at_second = game.checkpoint();
    let (_, second_answer) = higher_object_first(&game, P2);
    game.submit(second_answer.clone());
    let at_end = game.checkpoint();
    assert_ne!(at_first, at_second);
    assert_ne!(at_second, at_end);

    game.controller.restore(at_first.clone()).unwrap();
    assert_eq!(game.checkpoint(), at_first);
    assert_eq!(game.pending().0, P1);
    game.submit(first_answer);
    assert_eq!(game.checkpoint(), at_second);
    game.controller.restore(at_second.clone()).unwrap();
    assert_eq!(game.checkpoint(), at_second);
    assert_eq!(game.pending().0, P2);
    game.submit(second_answer);
    assert_eq!(game.checkpoint(), at_end);

    let report = game
        .controller
        .execute_replay(game.controller.export_replay().unwrap())
        .unwrap();
    assert_eq!(report.final_checkpoint, at_end);
}

/// The damage marked on `object`.
fn marked(state: &EngineState, object: GameObjectId) -> u64 {
    state.card_rules.permanents.permanents[&object].marked_damage
}

/// `state` with no damage marked on any permanent.
fn without_damage(state: &EngineState) -> EngineState {
    let mut undamaged = state.clone();
    for permanent in undamaged.card_rules.permanents.permanents.values_mut() {
        permanent.marked_damage = 0;
    }
    undamaged
}

/// The damage changes the product's events make: the creature, and the damage
/// marked on it before and after.
fn damage_changes(product: &BasicLandTransitionProduct) -> Vec<(GameObjectId, u64, u64)> {
    product
        .events
        .iter()
        .filter_map(|event| match &event.event {
            AuthoritativeRuleEventKind::MarkedDamageChanged { creature, from, to } => {
                Some((*creature, *from, *to))
            }
            _ => None,
        })
        .collect()
}

/// The end step with the player who is not the active one holding priority:
/// their pass ends the step, and the cleanup step begins.
fn at_the_end_of_the_end_step(state: &EngineState) -> bool {
    state.core.position
        == (TurnPosition::Ending {
            step: EndingStep::EndStep,
        })
        && matches!(state.core.priority,
            PriorityState::HeldBy { player, .. } if player != state.core.active_player)
}

/// What each player observes of the transition `answer` makes from `state`,
/// and of the same transition from `state` with no damage marked: the same,
/// because no player observes damage marked.
fn observed_the_same_without_the_damage(state: &EngineState, answer: &DecisionAnswerV2) {
    let undamaged = without_damage(state);
    assert_ne!(&undamaged, state);
    let (_, with_damage) = product_and_observations(state, answer.clone());
    let (_, without) = product_and_observations(&undamaged, answer.clone());
    assert_eq!(with_damage, without);
}

#[test]
fn a_creature_that_survived_a_block_has_no_damage_marked_in_the_next_turn() {
    // CR 120.6, 514.2: P1's Hill Giant attacks, P2's Savannah Lions blocks it
    // and dies, and the Giant survives with 2 damage marked on it. The damage
    // stays until the cleanup step, which P1's hand does not ask to discard
    // in: the transition that ends the turn removes it, and the next turn
    // begins with the Giant undamaged. No player observes any of that.
    let [_, _, giant] = creature_definitions();
    let (game, giant_object, lions_object) = attacker_against_a_lions(giant, 4);
    attack_and_block(&game, giant_object);
    game.answer(pass, pass);
    game.pass_until(at_the_end_of_the_end_step);
    let before = game.state();
    assert_eq!((before.core.turn_number, game.pending().0), (9, P2));
    assert!(!before.zones.objects.contains_key(&lions_object));
    assert_eq!(marked(&before, giant_object), 2);

    let answer = pass_in(&before);
    let (product, observed) = product_and_observations(&before, answer.clone());
    assert_eq!(damage_changes(&product), [(giant_object, 2, 0)]);
    observed_the_same_without_the_damage(&before, &answer);

    let (actor, step) = game.answer(pass, pass);
    assert_eq!(actor, P2);
    assert_eq!(step.observed_events, observed[&P2]);
    let after = game.state();
    assert_eq!(after, product.next_state);
    // P2's turn has begun, and the Giant is undamaged.
    assert_eq!(
        (
            after.core.turn_number,
            after.core.active_player,
            after.core.position
        ),
        (
            10,
            P2,
            TurnPosition::Beginning {
                step: mtgml_state::BeginningStep::Upkeep
            }
        )
    );
    assert_eq!(
        after.zones.locations[&giant_object].zone,
        ZoneKind::Battlefield
    );
    assert_eq!(marked(&after, giant_object), 0);
    assert!(after
        .card_rules
        .permanents
        .permanents
        .values()
        .all(|permanent| permanent.marked_damage == 0));

    // The game goes on: the Giant, undamaged, is offered as an attacker again
    // on P1's next turn, and the whole game replays to the same checkpoint.
    game.run_until(at_attackers(11));
    let next_attack = game.state();
    assert_eq!(marked(&next_attack, giant_object), 0);
    assert!(game.offered().contains(&CandidateIntent::SelectObject {
        object: opaque_of(&next_attack, P1, giant_object)
    }));
    let checkpoint = game.checkpoint();
    game.controller.restore(checkpoint.clone()).unwrap();
    assert_eq!(game.checkpoint(), checkpoint);
    let report = game
        .controller
        .execute_replay(game.controller.export_replay().unwrap())
        .unwrap();
    assert_eq!(report.final_checkpoint, checkpoint);
}

#[test]
fn a_blocker_that_survives_keeps_its_damage_until_the_cleanup_of_the_attackers_turn() {
    // CR 120.6, 514.2: P1's Savannah Lions (2/1) attacks, and P2's Hill Giant
    // (3/3) blocks it. The Lions dies, and the Giant, which P2 controls and
    // which is not the active player's creature, survives with 2 damage
    // marked. The damage stays through P1's end of combat, postcombat main
    // phase and end step, and the cleanup step of P1's turn removes it: P2's
    // turn begins with the Giant undamaged.
    let (mountain, plains) = land_definitions();
    let [_, _, giant] = creature_definitions();
    let hand = [vec![mountain; 4], vec![giant]].concat();
    let game = Game::with_hands([vec![plains, lions()], hand]);
    cast_lions(&game);
    game.run_until(start_of_main_phase(8));
    cast_creature(&game, 4);
    game.run_until(at_attackers(9));
    let state = game.state();
    let [lions_object]: [GameObjectId; 1] = lions_of(&state, P1).try_into().unwrap();
    let [giant_object]: [GameObjectId; 1] = creatures_of(&state, P2, giant).try_into().unwrap();
    assert_eq!(marked(&state, giant_object), 0);

    attack_and_block(&game, lions_object);
    game.answer(pass, pass);
    // The combat damage step: the Lions is destroyed, and the Giant, which
    // P2 controls, has the damage the Lions dealt marked on it.
    let at_damage = game.state();
    assert_eq!(
        at_damage.core.position,
        TurnPosition::Combat {
            step: CombatStep::CombatDamage
        }
    );
    assert!(!at_damage.zones.objects.contains_key(&lions_object));
    assert_eq!(
        at_damage.zones.locations[&giant_object].zone,
        ZoneKind::Battlefield
    );
    assert_eq!(marked(&at_damage, giant_object), 2);

    // It stays through each step of P1's turn that follows, with P1 active.
    for position in [
        TurnPosition::Combat {
            step: CombatStep::EndOfCombat,
        },
        TurnPosition::PostcombatMain,
        TurnPosition::Ending {
            step: EndingStep::EndStep,
        },
    ] {
        game.pass_until(move |state| state.core.position == position);
        let state = game.state();
        assert_eq!((state.core.turn_number, state.core.active_player), (9, P1));
        assert_eq!(marked(&state, giant_object), 2, "{position:?}");
    }
    game.pass_until(at_the_end_of_the_end_step);
    let before = game.state();
    assert_eq!((before.core.turn_number, game.pending().0), (9, P2));
    assert_eq!(marked(&before, giant_object), 2);

    // The pass that ends the end step begins the cleanup step, which removes
    // the damage from P2's creature, and P2's turn begins. Nothing of this is
    // observed.
    let answer = pass_in(&before);
    let (product, observed) = product_and_observations(&before, answer.clone());
    assert_eq!(damage_changes(&product), [(giant_object, 2, 0)]);
    observed_the_same_without_the_damage(&before, &answer);
    let (actor, step) = game.answer(pass, pass);
    assert_eq!(actor, P2);
    assert_eq!(step.observed_events, observed[&P2]);
    let after = game.state();
    assert_eq!(after, product.next_state);
    assert_eq!((after.core.turn_number, after.core.active_player), (10, P2));
    assert_eq!(
        after.zones.locations[&giant_object].zone,
        ZoneKind::Battlefield
    );
    assert_eq!(marked(&after, giant_object), 0);
    assert!(after
        .card_rules
        .permanents
        .permanents
        .values()
        .all(|permanent| permanent.marked_damage == 0));
}

/// P2 casts a Savannah Lions on its first turn, and P1 a Hill Giant on its
/// fourth, with the four Mountains it plays. P1 plays no land after that: it
/// keeps the card it draws every turn, which makes eight in its hand on turn
/// 11. The Savannah Lions it holds besides cannot be cast without white mana;
/// they only keep its hand full. P1 is at its attacker declaration of turn 11.
/// Returns the game, the Giant and P2's Lions.
fn giant_attacks_with_a_full_hand() -> (Game, GameObjectId, GameObjectId) {
    let (mountain, plains) = land_definitions();
    let [_, _, giant] = creature_definitions();
    let hand = [vec![mountain; 4], vec![giant], vec![lions(); 3]].concat();
    let game = Game::with_hands([hand, vec![plains, lions()]]);
    game.run_until(start_of_main_phase(2));
    cast_lions(&game);
    game.run_until(start_of_main_phase(7));
    cast_creature(&game, 4);
    game.pass_until(at_attackers(11));
    let state = game.state();
    let [giant_object]: [GameObjectId; 1] = creatures_of(&state, P1, giant).try_into().unwrap();
    let [lions_object]: [GameObjectId; 1] = lions_of(&state, P2).try_into().unwrap();
    let in_hand = |player| {
        state
            .zones
            .locations
            .values()
            .filter(|location| location.zone == ZoneKind::Hand && location.player == Some(player))
            .count()
    };
    assert_eq!(in_hand(P1), 8);
    (game, giant_object, lions_object)
}

#[test]
fn a_discard_in_the_cleanup_step_comes_before_the_damage_is_removed() {
    // CR 514.1, 514.2: as above, but P1 holds eight cards at the cleanup step,
    // so it first discards one. The damage is still marked while it is asked,
    // and a checkpoint there restores with it (and still refuses damage that
    // is lethal). The answer discards, removes the damage, and ends the turn.
    let (game, giant_object, lions_object) = giant_attacks_with_a_full_hand();
    attack_and_block(&game, giant_object);
    game.answer(pass, pass);
    game.pass_until(at_the_end_of_the_end_step);
    game.answer(pass, pass);

    let at_discard = game.checkpoint();
    let state = &at_discard.state;
    let (asked, request) = game.pending();
    assert_eq!(
        (asked, request.purpose),
        (P1, DecisionPurposeV4::HandSizeDiscard)
    );
    assert_eq!(
        (state.core.turn_number, state.core.position),
        (
            11,
            TurnPosition::Ending {
                step: EndingStep::Cleanup
            }
        )
    );
    assert!(!state.zones.objects.contains_key(&lions_object));
    assert_eq!(marked(state, giant_object), 2);

    // The checkpoint restores, damage and all.
    game.controller.restore(at_discard.clone()).unwrap();
    assert_eq!(game.checkpoint(), at_discard);
    // The restore path (the same validation) takes this state and refuses
    // the same one with lethal damage on the Giant.
    let admission = creature_game_admission();
    let restored = |state: EngineState| {
        EnvironmentCheckpointV8::new_for_basic_land_profile(
            &admission,
            state,
            at_discard.status.clone(),
            at_discard.limit_counters.clone(),
            at_discard.execution_identity.clone(),
        )
    };
    assert!(restored(state.clone()).is_ok());
    let mut lethal = state.clone();
    lethal
        .card_rules
        .permanents
        .permanents
        .get_mut(&giant_object)
        .unwrap()
        .marked_damage = 3;
    assert!(restored(lethal).is_err());

    // The discard ends the turn: the damage goes with it, after the card.
    let answer = DecisionAnswerV2::SelectMany {
        candidate_ids: vec![request.candidates[0].candidate_id],
    };
    let (product, observed) = product_and_observations(state, answer.clone());
    assert_eq!(damage_changes(&product), [(giant_object, 2, 0)]);
    let index_of = |pick: fn(&AuthoritativeRuleEventKind) -> bool| {
        product.events.iter().position(|event| pick(&event.event))
    };
    let discard =
        index_of(|event| matches!(event, AuthoritativeRuleEventKind::ZoneTransition { .. }));
    let removal = index_of(|event| {
        matches!(
            event,
            AuthoritativeRuleEventKind::MarkedDamageChanged { .. }
        )
    });
    assert!(discard.unwrap() < removal.unwrap());
    observed_the_same_without_the_damage(state, &answer);

    let (actor, step) = game.submit(answer);
    assert_eq!(actor, P1);
    assert_eq!(step.observed_events, observed[&P1]);
    let after = game.state();
    assert_eq!(after, product.next_state);
    assert_eq!((after.core.turn_number, after.core.active_player), (12, P2));
    assert_eq!(marked(&after, giant_object), 0);

    let checkpoint = game.checkpoint();
    let report = game
        .controller
        .execute_replay(game.controller.export_replay().unwrap())
        .unwrap();
    assert_eq!(report.final_checkpoint, checkpoint);
}
