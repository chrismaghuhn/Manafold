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
    AssignedDamageObservationV1, BlockObservationV1, DeclaredBlockObservationV1,
    MagicCompletedOrder, MagicPendingSbaOrdering, MagicSharedExecutionObservationV1,
    ObservedBlockV1, ObservedDamageRecipientV1, ObservedDamageV1, ObservedEventEnvelopeV4,
    ObservedEventKindV4, PlayerStepSubmissionV1, PlayerStepV4, SyntheticPriority,
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
    /// With `record_views`, what each answer led to.
    views: std::cell::RefCell<Option<Vec<Recorded>>>,
}

/// An answer's effect: who answered, the step the answer returned and each
/// player's information state afterwards.
type Recorded = (PlayerId, Vec<u8>, [Vec<u8>; 2]);

impl Game {
    /// A game at P1's first precombat main with the given hands (P1's first)
    /// and ten Mountains in each library.
    fn with_hands(hands: [Vec<CardDefinitionId>; 2]) -> Self {
        let (mountain, _) = land_definitions();
        Self::with_hands_and_libraries(hands, [vec![mountain; 10], vec![mountain; 10]])
    }

    /// As `with_hands`, with the given libraries (the first card is the top).
    fn with_hands_and_libraries(
        hands: [Vec<CardDefinitionId>; 2],
        libraries: [Vec<CardDefinitionId>; 2],
    ) -> Self {
        let controller = creature_game(&libraries, &hands, 5);
        let players = [
            controller.bind_player(P1).unwrap(),
            controller.bind_player(P2).unwrap(),
        ];
        Self {
            controller,
            players,
            views: Default::default(),
        }
    }

    /// From now on, records what each answer leads to.
    fn record_views(&self) {
        *self.views.borrow_mut() = Some(Vec::new());
    }

    /// What `player` is shown after each answer since `record_views`: the step
    /// when it answered, and its information state.
    fn views_of(&self, player: PlayerId) -> Vec<(Option<Vec<u8>>, Vec<u8>)> {
        let index = usize::from(player != P1);
        self.views
            .borrow()
            .as_ref()
            .expect("views are recorded")
            .iter()
            .map(|(actor, step, information)| {
                (
                    (*actor == player).then(|| step.clone()),
                    information[index].clone(),
                )
            })
            .collect()
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
        if let Some(views) = self.views.borrow_mut().as_mut() {
            views.push((
                player,
                mtgml_wire::encode_canonical(&step).unwrap(),
                [self.information_bytes(P1), self.information_bytes(P2)],
            ));
        }
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

    /// The observation `player` receives, as it is on the wire.
    fn observation_json(&self, player: PlayerId) -> serde_json::Value {
        let information = self.endpoint(player).information_state().unwrap();
        let payload = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            &information.current_observation.payload_base64,
        )
        .unwrap();
        serde_json::from_slice(&payload).unwrap()
    }

    /// The damage marked on `object`, as `player` observes it.
    fn marked_damage(&self, player: PlayerId, object: OpaqueObjectId) -> u64 {
        let observation = self.seen(player).observation;
        observation
            .permanents
            .iter()
            .find(|permanent| permanent.object == object)
            .unwrap_or_else(|| panic!("{player:?} sees no permanent {object:?}"))
            .marked_damage
    }
}

/// The block declarations and the combat damage among `events`.
fn combat_events(events: &[ObservedEventEnvelopeV4]) -> Vec<&ObservedEventKindV4> {
    events
        .iter()
        .map(|envelope| &envelope.event)
        .filter(|event| {
            matches!(
                event,
                ObservedEventKindV4::BlockersDeclared { .. }
                    | ObservedEventKindV4::CombatDamageDealt { .. }
            )
        })
        .collect()
}

/// The blocks `player` is told of, one for each pair, ascending by blocker.
fn observed_blocks(
    state: &EngineState,
    player: PlayerId,
    pairs: &[(GameObjectId, GameObjectId)],
) -> Vec<ObservedBlockV1> {
    let mut blocks: Vec<_> = pairs
        .iter()
        .map(|(blocker, attacker)| ObservedBlockV1 {
            blocker: opaque_of(state, player, *blocker),
            attacker: opaque_of(state, player, *attacker),
        })
        .collect();
    blocks.sort_by_key(|block| block.blocker);
    blocks
}

/// The damage `player` is told of: (source, creature recipient, amount),
/// sorted in `player`'s opaque ids.
fn observed_damage(
    state: &EngineState,
    player: PlayerId,
    dealt: &[(GameObjectId, GameObjectId, u64)],
) -> Vec<ObservedDamageV1> {
    let mut damage: Vec<_> = dealt
        .iter()
        .map(|(source, recipient, amount)| ObservedDamageV1 {
            source: opaque_of(state, player, *source),
            recipient: ObservedDamageRecipientV1::Object {
                object: opaque_of(state, player, *recipient),
            },
            amount: *amount,
        })
        .collect();
    damage.sort_by_key(|damage| (damage.source, damage.recipient));
    damage
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
    let (last, observed) = product_and_observations(&half, block_in(&half, None));
    // Every answer was "no block": one declaration, with no assignments, and
    // both players are told of it: "no blocks" is one event-log shape.
    assert_eq!(declarations(&last), vec![&Vec::new()]);
    for player in [P1, P2] {
        assert_eq!(
            combat_events(&observed[&player]),
            [&ObservedEventKindV4::BlockersDeclared {
                defending_player: P2,
                blocks: Vec::new()
            }],
            "{player:?}"
        );
    }
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
    // P2 has no creature: nothing is asked, and the declaration is empty
    // (CR 509.1: the turn-based action happens whenever there are attackers).
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
    assert_eq!(declarations(&product), vec![&Vec::new()]);
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

    // The half-declared block shows in the defender's own request and
    // observation (see `the_defender_sees_its_own_partial_blocks`), and in no
    // other player's.
    assert_eq!(game.pending().0, P2);
}

/// P1 attacks on turn 7 with its Savannah Lions, and P2 has three untapped
/// Savannah Lions to ask about, one cast on each of its turns. Returns the game,
/// the attacker, and P2's creatures in the order they are asked about.
fn three_lions_may_block() -> (Game, GameObjectId, [GameObjectId; 3]) {
    let (mountain, plains) = land_definitions();
    let game = Game::with_hands_and_libraries(
        [vec![plains, lions()], vec![plains, plains, lions()]],
        [
            vec![mountain; 10],
            [vec![lions(), lions()], vec![mountain; 8]].concat(),
        ],
    );
    cast_lions(&game);
    for turn in [2, 4, 6] {
        game.run_until(start_of_main_phase(turn));
        cast_lions(&game);
    }
    game.run_until(at_attackers(7));
    let state = game.state();
    let [attacker]: [GameObjectId; 1] = lions_of(&state, P1).try_into().unwrap();
    let blockers: [GameObjectId; 3] = in_opaque_order(&state, P2, lions_of(&state, P2))
        .try_into()
        .unwrap();
    game.declare_attackers(&[opaque_of(&state, P1, attacker)]);
    game.answer(pass, pass);
    game.answer(pass, pass);
    assert_eq!(game.pending().0, P2);
    (game, attacker, blockers)
}

/// The block answers `player` is shown, one for each pair, in the order given.
fn declared_blocks(
    state: &EngineState,
    player: PlayerId,
    answers: &[(GameObjectId, Option<GameObjectId>)],
) -> Vec<DeclaredBlockObservationV1> {
    answers
        .iter()
        .map(|(blocker, attacker)| DeclaredBlockObservationV1 {
            blocker: opaque_of(state, player, *blocker),
            attacker: attacker.map(|attacker| opaque_of(state, player, attacker)),
        })
        .collect()
}

#[test]
fn the_defender_sees_its_own_partial_blocks() {
    // CR 509.1a: P2 answers for one untapped creature at a time. The answers
    // so far are P2's own and shown to P2 alone, in P2's opaque ids. P1 is
    // shown nothing, and P1's bytes do not change.
    let (game, attacker, blockers) = three_lions_may_block();
    let state = game.state();
    let pending = |player: PlayerId| game.seen(player).observation.pending_blocks;
    let wire = |player: PlayerId| game.observation_json(player)["pending_blocks"].clone();
    let id = |object: GameObjectId| opaque_of(&state, P2, object).0.to_string();

    // The declaration has begun and no creature has been answered: P2's list is
    // empty, and not null, which is how P2 tells a declaration begun from none.
    assert_eq!(pending(P2), Some(Vec::new()));
    assert_eq!(wire(P2), serde_json::json!([]));
    assert_eq!(pending(P1), None);
    assert!(wire(P1).is_null());
    let information = game.information_bytes(P1);

    // The first creature blocks the attacker.
    let (actor, step) = game.declare_block(Some(opaque_of(&state, P2, attacker)));
    assert_eq!(actor, P2);
    assert!(step.observed_events.is_empty());
    assert_eq!(
        pending(P2),
        Some(declared_blocks(
            &state,
            P2,
            &[(blockers[0], Some(attacker))]
        ))
    );
    assert_eq!(
        wire(P2),
        serde_json::json!([{"blocker": id(blockers[0]), "attacker": id(attacker)}])
    );
    assert_eq!(pending(P1), None);
    assert!(wire(P1).is_null());
    assert_eq!(game.information_bytes(P1), information);

    // The second does not block. Its answer is a row with a null attacker.
    game.declare_block(None);
    assert_eq!(
        pending(P2),
        Some(declared_blocks(
            &state,
            P2,
            &[(blockers[0], Some(attacker)), (blockers[1], None)]
        ))
    );
    assert_eq!(
        wire(P2),
        serde_json::json!([
            {"blocker": id(blockers[0]), "attacker": id(attacker)},
            {"blocker": id(blockers[1]), "attacker": null},
        ])
    );
    assert_eq!(pending(P1), None);
    assert_eq!(game.information_bytes(P1), information);
    assert_eq!(game.endpoint(P1).visible_decision().unwrap(), None);

    // The last answer completes the declaration: nothing is pending for
    // anybody, and the blocks are the public ones.
    game.declare_block(Some(opaque_of(&state, P2, attacker)));
    assert_eq!(game.pending().0, P1, "the attacker has priority");
    for player in [P1, P2] {
        assert_eq!(pending(player), None, "{player:?}");
        assert!(wire(player).is_null(), "{player:?}");
        assert_eq!(
            game.seen(player).observation.blocking.len(),
            2,
            "{player:?}"
        );
    }
}

#[test]
fn a_restored_partial_block_gives_the_same_observation() {
    // The partial answers are rederived from the state, so a checkpoint taken
    // with the declaration pending shows each player what the game showed at
    // that moment.
    let (game, attacker, _) = three_lions_may_block();
    let state = game.state();
    let own = Some(opaque_of(&state, P2, attacker));
    let shown = |game: &Game| [game.information_bytes(P1), game.information_bytes(P2)];
    let at_first = (game.checkpoint(), shown(&game));
    game.declare_block(own);
    let after_one = (game.checkpoint(), shown(&game));
    game.declare_block(None);
    let after_two = (game.checkpoint(), shown(&game));

    assert_ne!(at_first.1[1], after_one.1[1]);
    assert_ne!(after_one.1[1], after_two.1[1]);
    for (checkpoint, expected) in [&after_one, &at_first, &after_two, &after_one] {
        game.controller.restore(checkpoint.clone()).unwrap();
        assert_eq!(&game.checkpoint(), checkpoint);
        assert_eq!(&shown(&game), expected);
    }

    // Playing on from the restored half leads to what the game showed.
    game.declare_block(None);
    assert_eq!(game.checkpoint(), after_two.0);
    assert_eq!(shown(&game), after_two.1);
}

/// Exchanges the opaque ids `player` has for `a` and `b`, in the identity and
/// in the knowledge it is kept under, so that the order of the ids is not the
/// order of the objects.
fn swap_opaque_ids(state: &mut EngineState, player: PlayerId, a: GameObjectId, b: GameObjectId) {
    let identity = state
        .perspective_identities
        .players
        .get_mut(&player)
        .unwrap();
    let (opaque_a, opaque_b) = (identity.object_to_opaque[&a], identity.object_to_opaque[&b]);
    identity.object_to_opaque.insert(a, opaque_b);
    identity.object_to_opaque.insert(b, opaque_a);
    identity.opaque_to_object.insert(opaque_a, b);
    identity.opaque_to_object.insert(opaque_b, a);
    let active = &mut state.knowledge.players.get_mut(&player).unwrap().active;
    let record_a = active.remove(&opaque_a).unwrap();
    let record_b = active.remove(&opaque_b).unwrap();
    active.insert(
        opaque_b,
        mtgml_state::KnowledgeRecordV2 {
            opaque_object: opaque_b,
            ..record_a
        },
    );
    active.insert(
        opaque_a,
        mtgml_state::KnowledgeRecordV2 {
            opaque_object: opaque_a,
            ..record_b
        },
    );
}

#[test]
fn the_partial_blocks_are_listed_in_the_order_of_the_defenders_opaque_ids() {
    // The order of the objects means nothing to a player. P2 has answered for
    // two creatures, and the ids P2 knows them by are in the opposite order to
    // the objects: the answers are listed in the order of the ids.
    let (game, attacker, blockers) = three_lions_may_block();
    let state = game.state();
    game.declare_block(Some(opaque_of(&state, P2, attacker)));
    game.declare_block(None);
    let reached = game.checkpoint();
    let mut forged = reached.state.clone();
    swap_opaque_ids(&mut forged, P2, blockers[0], blockers[1]);
    assert!(blockers[0] < blockers[1]);
    assert!(opaque_of(&forged, P2, blockers[1]) < opaque_of(&forged, P2, blockers[0]));
    restore_state(&game, &reached, forged.clone()).unwrap();

    assert_eq!(
        game.seen(P2).observation.pending_blocks,
        Some(declared_blocks(
            &forged,
            P2,
            &[(blockers[1], None), (blockers[0], Some(attacker))]
        ))
    );
    let wire = game.observation_json(P2)["pending_blocks"].clone();
    let listed: Vec<u64> = wire
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["blocker"].as_str().unwrap().parse().unwrap())
        .collect();
    assert_eq!(listed.len(), 2);
    assert!(listed[0] < listed[1], "{listed:?}");
    assert_eq!(game.seen(P1).observation.pending_blocks, None);

    // The declaration goes on from the restored state.
    game.declare_block(None);
    for player in [P1, P2] {
        assert_eq!(game.seen(player).observation.pending_blocks, None);
    }
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
fn both_players_see_the_blocks_once_declared() {
    // CR 509.1g, 509.1h: a block is told to nobody while the declaration is
    // made: not by the first answer. The answer that completes it is public:
    // both players observe one `BlockersDeclared`, and the observation lists
    // the blocked attackers and each blocking creature with its attacker, in
    // the player's own opaque ids. Priority passing to the attacker is public
    // turn structure.
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
        // Before the declaration there is no block to see.
        assert!(after.observation.blocked.is_empty() && after.observation.blocking.is_empty());
    }
    game.answer(pass, pass);
    game.answer(pass, pass);
    assert_eq!(game.pending().0, P2);
    let blockers: [GameObjectId; 2] = in_opaque_order(&state, P2, lions_of(&state, P2))
        .try_into()
        .unwrap();

    // The first answer makes no block public: it has no event, and the
    // attacker sees no change. The defender's own answer shows in its
    // `pending_blocks`, and in nothing else.
    let [attacker_before, mut defender_before] = game.seen_by_both();
    let at_first = game.state();
    let (_, observed_first) =
        product_and_observations(&at_first, block_in(&at_first, Some(attackers[0])));
    assert!(observed_first.values().all(|events| events.is_empty()));
    game.declare_block(own(attackers[0]));
    let [attacker_after, defender_after] = game.seen_by_both();
    assert_eq!(attacker_before, attacker_after);
    assert_ne!(
        defender_before.observation.pending_blocks,
        defender_after.observation.pending_blocks
    );
    defender_before.observation.pending_blocks = defender_after.observation.pending_blocks.clone();
    assert_eq!(defender_before, defender_after);
    assert!(game.state().combat.as_ref().unwrap().blockers.is_empty());

    // The last answer records the blocks, and both players see them.
    let at_last = game.state();
    let (product, observed) =
        product_and_observations(&at_last, block_in(&at_last, Some(attackers[1])));
    let (actor, step) = game.declare_block(own(attackers[1]));
    assert_eq!(actor, P2);
    assert_eq!(step.observed_events, observed[&P2]);
    let combat = game.state().combat.clone().unwrap();
    assert_eq!(combat.blockers.len(), 2, "the blocks are recorded");
    assert_eq!(combat.blocked_attackers.len(), 2);
    assert_eq!(game.pending().0, P1, "the attacker has priority");
    assert_eq!(game.state(), product.next_state);
    let pairs = [(blockers[0], attackers[0]), (blockers[1], attackers[1])];

    for player in [P1, P2] {
        // One declaration, listed by the player's own ids.
        assert_eq!(
            combat_events(&observed[&player]),
            [&ObservedEventKindV4::BlockersDeclared {
                defending_player: P2,
                blocks: observed_blocks(&state, player, &pairs),
            }],
            "{player:?}"
        );
        let observation = game.seen(player).observation;
        let mut blocked: Vec<_> = attackers
            .iter()
            .map(|attacker| opaque_of(&state, player, *attacker))
            .collect();
        blocked.sort();
        assert_eq!(observation.blocked, blocked, "{player:?}");
        assert_eq!(observation.attacking, blocked, "{player:?}");
        let blocking: Vec<_> = observed_blocks(&state, player, &pairs)
            .into_iter()
            .map(|block| BlockObservationV1 {
                blocker: block.blocker,
                attacker: Some(block.attacker),
            })
            .collect();
        assert_eq!(observation.blocking, blocking, "{player:?}");
        assert!(blocking
            .windows(2)
            .all(|pair| pair[0].blocker < pair[1].blocker));
        // The declaration is complete: no answer is pending any more.
        assert_eq!(observation.pending_blocks, None);
        assert_eq!(observation.pending_damage_assignment, None);
        // On the wire a block names both creatures.
        let wire = game.observation_json(player);
        assert_eq!(
            wire["blocking"][0]["attacker"],
            serde_json::json!(blocking[0].attacker.unwrap().0.to_string())
        );
        assert!(wire["pending_blocks"].is_null());
    }
    // The attacker holds priority.
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

    // Both players see the damage dealt and then the Lions go from the
    // battlefield to the graveyard, and follow it: it keeps the opaque id they
    // knew it by. They see nothing else of the step.
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
        assert!(
            matches!(
                observed[&player][..],
                [
                    ObservedEventEnvelopeV4 {
                        event: ObservedEventKindV4::CombatDamageDealt { .. },
                        ..
                    },
                    ObservedEventEnvelopeV4 {
                        event: ObservedEventKindV4::ObjectMoved { .. },
                        ..
                    }
                ]
            ),
            "{player:?}: {:?}",
            observed[&player]
        );
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
/// and of the same transition from `state` with no damage marked: the same
/// events. Damage is marked by the combat damage and removed by the cleanup
/// step; the observation shows what is marked, so the removal is seen as the
/// mark going to 0, not as an event.
fn no_event_shows_the_damage_removal(state: &EngineState, answer: &DecisionAnswerV2) {
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
    // begins with the Giant undamaged. Both players see the damage marked in
    // the observation, and see it go to 0 with no event for the removal.
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
    no_event_shows_the_damage_removal(&before, &answer);
    for player in [P1, P2] {
        assert_eq!(
            game.marked_damage(player, opaque_of(&before, player, giant_object)),
            2,
            "{player:?}"
        );
    }

    let (actor, step) = game.answer(pass, pass);
    assert_eq!(actor, P2);
    assert_eq!(step.observed_events, observed[&P2]);
    let after = game.state();
    assert_eq!(after, product.next_state);
    for player in [P1, P2] {
        assert_eq!(
            game.marked_damage(player, opaque_of(&after, player, giant_object)),
            0,
            "{player:?}"
        );
        assert!(
            combat_events(&observed[&player]).is_empty()
                && observed[&player].iter().all(|envelope| !matches!(
                    envelope.event,
                    ObservedEventKindV4::ObjectMoved { .. }
                )),
            "{player:?}: no combat event and no move is observed"
        );
    }
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
    // the damage from P2's creature, and P2's turn begins. No event shows the
    // removal: both players see the mark go from 2 to 0 in the observation.
    let answer = pass_in(&before);
    let (product, observed) = product_and_observations(&before, answer.clone());
    assert_eq!(damage_changes(&product), [(giant_object, 2, 0)]);
    no_event_shows_the_damage_removal(&before, &answer);
    for player in [P1, P2] {
        assert_eq!(
            game.marked_damage(player, opaque_of(&before, player, giant_object)),
            2,
            "{player:?}"
        );
    }
    let (actor, step) = game.answer(pass, pass);
    assert_eq!(actor, P2);
    assert_eq!(step.observed_events, observed[&P2]);
    let after = game.state();
    assert_eq!(after, product.next_state);
    for player in [P1, P2] {
        assert_eq!(
            game.marked_damage(player, opaque_of(&after, player, giant_object)),
            0,
            "{player:?}"
        );
    }
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
    no_event_shows_the_damage_removal(state, &answer);
    // Both players see the damage still marked while the discard is asked.
    for player in [P1, P2] {
        assert_eq!(
            game.marked_damage(player, opaque_of(state, player, giant_object)),
            2,
            "{player:?}"
        );
    }

    let (actor, step) = game.submit(answer);
    assert_eq!(actor, P1);
    assert_eq!(step.observed_events, observed[&P1]);
    let after = game.state();
    assert_eq!(after, product.next_state);
    assert_eq!((after.core.turn_number, after.core.active_player), (12, P2));
    assert_eq!(marked(&after, giant_object), 0);
    for player in [P1, P2] {
        assert_eq!(
            game.marked_damage(player, opaque_of(&after, player, giant_object)),
            0,
            "{player:?}"
        );
    }

    let checkpoint = game.checkpoint();
    let report = game
        .controller
        .execute_replay(game.controller.export_replay().unwrap())
        .unwrap();
    assert_eq!(report.final_checkpoint, checkpoint);
}

/// P1's Hill Giant (3/3) blocked by every creature of P2: a Gray Ogre (2/2)
/// and some Savannah Lions (2/1).
struct GiantFight {
    game: Game,
    giant: GameObjectId,
    ogre: GameObjectId,
    lions: Vec<GameObjectId>,
    /// P2's creatures in the order P1 divides the Giant's damage among them:
    /// the order of P1's opaque ids, not of the object ids.
    blockers: Vec<GameObjectId>,
}

/// P2 casts a Savannah Lions on turn 2 (and a second on turn 4 when
/// `lions_count` is 2) and a Gray Ogre on turn 6; P1 casts a Hill Giant on turn
/// 7. On turn 9 the Giant attacks and every creature of P2 blocks it. P2 holds
/// priority in the declare blockers step: its pass opens the combat damage
/// step.
fn giant_blocked_by(lions_count: usize) -> GiantFight {
    let [_, ogre_card, giant_card] = creature_definitions();
    let (mountain, plains) = land_definitions();
    let mut hand = vec![plains, lions(), plains];
    if lions_count == 2 {
        hand.push(lions());
    }
    hand.push(ogre_card);
    let game = Game::with_hands([
        vec![mountain, mountain, mountain, mountain, giant_card],
        hand,
    ]);
    game.run_until(start_of_main_phase(2));
    cast_lions(&game);
    if lions_count == 2 {
        game.run_until(start_of_main_phase(4));
        cast_lions(&game);
    }
    game.run_until(start_of_main_phase(6));
    cast_creature(&game, 3);
    game.run_until(start_of_main_phase(7));
    cast_creature(&game, 4);
    game.run_until(at_attackers(9));
    every_creature_blocks_the_giant(game, lions_count)
}

/// At P1's attacker declaration with a Hill Giant, against P2's Gray Ogre and
/// `lions_count` Savannah Lions: the Giant attacks and every creature of P2
/// blocks it. P2 holds priority in the declare blockers step: its pass opens
/// the combat damage step.
fn every_creature_blocks_the_giant(game: Game, lions_count: usize) -> GiantFight {
    let [_, ogre_card, giant_card] = creature_definitions();
    let state = game.state();
    let [giant]: [GameObjectId; 1] = creatures_of(&state, P1, giant_card).try_into().unwrap();
    let [ogre]: [GameObjectId; 1] = creatures_of(&state, P2, ogre_card).try_into().unwrap();
    let lions = lions_of(&state, P2);
    assert_eq!(lions.len(), lions_count);
    let blockers = in_opaque_order(&state, P1, [lions.clone(), vec![ogre]].concat());
    game.declare_attackers(&[opaque_of(&state, P1, giant)]);
    game.answer(pass, pass);
    game.answer(pass, pass);
    for _ in &blockers {
        game.declare_block(Some(opaque_of(&state, P2, giant)));
    }
    game.answer(pass, pass);
    assert_eq!(game.pending().0, P2);
    assert_eq!(
        game.state().core.position,
        TurnPosition::Combat {
            step: CombatStep::DeclareBlockers
        }
    );
    GiantFight {
        game,
        giant,
        ogre,
        lions,
        blockers,
    }
}

/// The answer to a division request that gives its recipient `amount`.
fn damage_answer(request: &PlayerDecisionRequestV4, amount: u64) -> DecisionAnswerV2 {
    let candidate = request
        .candidates
        .iter()
        .find(|candidate| {
            matches!(candidate.intent,
                CandidateIntent::AssignCombatDamage { amount: offered, .. } if offered == amount)
        })
        .unwrap_or_else(|| panic!("the request does not offer {amount}"));
    DecisionAnswerV2::SelectOne {
        candidate_id: candidate.candidate_id,
    }
}

impl GiantFight {
    /// P2 passes, which opens the combat damage step: P1 is asked how the
    /// Giant divides its damage. Returns that request.
    fn open_the_damage_step(&self) -> PlayerDecisionRequestV4 {
        assert_eq!(self.game.answer(pass, pass).0, P2);
        let (actor, request) = self.game.pending();
        assert_eq!(actor, P1);
        assert_eq!(request.purpose, DecisionPurposeV4::CombatDamageAssignment);
        request
    }

    /// P1 gives `amount` of the Giant's damage to the creature it is asked
    /// about.
    fn assign(&self, amount: u64) -> (PlayerId, PlayerStepV4) {
        let (actor, request) = self.game.pending();
        assert_eq!(
            (actor, &request.purpose),
            (P1, &DecisionPurposeV4::CombatDamageAssignment)
        );
        self.game.submit(damage_answer(&request, amount))
    }

    /// What P1 is offered for `blocker`: each amount from 0 to `most`, in the
    /// opaque ids of P1.
    fn offered_for(
        &self,
        state: &EngineState,
        blocker: GameObjectId,
        most: u64,
    ) -> Vec<CandidateIntent> {
        (0..=most)
            .map(|amount| CandidateIntent::AssignCombatDamage {
                attacker: opaque_of(state, P1, self.giant),
                recipient: opaque_of(state, P1, blocker),
                amount,
            })
            .collect()
    }
}

/// The combat damage assignment in progress: who divides, the attackers and
/// the blockers still to ask, and the amounts answered.
fn damage_division(
    state: &EngineState,
) -> (
    PlayerId,
    Vec<GameObjectId>,
    Vec<GameObjectId>,
    BTreeMap<GameObjectId, u64>,
) {
    let [record] = state
        .execution
        .continuations
        .values()
        .collect::<Vec<_>>()
        .try_into()
        .expect("one continuation");
    let ContinuationPayload::CombatDamageAssignment {
        player,
        pending_attackers,
        pending_blockers,
        assigned,
    } = &record.payload
    else {
        panic!("not a combat damage assignment: {record:?}");
    };
    (
        *player,
        pending_attackers.clone(),
        pending_blockers.clone(),
        assigned.clone(),
    )
}

/// Whether `state` waits for the damage of an attacker to be divided.
fn dividing(state: &EngineState) -> bool {
    state.execution.continuations.values().any(|record| {
        matches!(
            record.payload,
            ContinuationPayload::CombatDamageAssignment { .. }
        )
    })
}

/// The damage the product's `CombatDamageDealt` assigns, as (source, creature
/// recipient, amount), sorted. There is exactly one such event.
fn damage_dealt_to_creatures(
    product: &BasicLandTransitionProduct,
) -> Vec<(GameObjectId, GameObjectId, u64)> {
    let dealt: Vec<_> = product
        .events
        .iter()
        .filter_map(|event| match &event.event {
            AuthoritativeRuleEventKind::CombatDamageDealt { assignments } => Some(assignments),
            _ => None,
        })
        .collect();
    let [assignments] = dealt[..] else {
        panic!("one CombatDamageDealt: {dealt:?}")
    };
    let mut dealt: Vec<_> = assignments
        .iter()
        .map(|assignment| match assignment.recipient {
            mtgml_state::DamageRecipientV1::Creature { object } => {
                (assignment.source, object, assignment.amount)
            }
            other => panic!("no player is dealt damage: {other:?}"),
        })
        .collect();
    dealt.sort();
    dealt
}

#[test]
fn a_hill_giant_blocked_by_lions_and_ogre_can_assign_every_division() {
    // CR 510.1c: a creature blocked by two or more creatures assigns its combat
    // damage to them divided as its controller chooses. P1's Hill Giant (3/3)
    // is blocked by P2's Savannah Lions and Gray Ogre, so P1 is asked, once,
    // for the first of them in the order of P1's opaque ids, how much of the 3
    // it gets, which is any of 0, 1, 2 and 3; the other gets what is left.
    let fight = giant_blocked_by(1);
    let request = fight.open_the_damage_step();
    let at_request = fight.game.checkpoint();
    let state = &at_request.state;

    // Nobody has priority while the damage is divided, and none is dealt yet.
    assert_eq!(
        state.core.position,
        TurnPosition::Combat {
            step: CombatStep::CombatDamage
        }
    );
    assert_eq!(state.core.priority, PriorityState::None);
    let combat = state.combat.as_ref().unwrap();
    assert!(!combat.damage_step_completed);
    assert_eq!(combat.attackers, vec![fight.giant]);
    assert_eq!(combat.blockers.len(), 2);
    assert!(state
        .card_rules
        .permanents
        .permanents
        .values()
        .all(|permanent| permanent.marked_damage == 0));

    // The request is P1's alone, one answer, with an amount for the first of
    // the two blockers in P1's opaque order.
    assert_eq!(request.visibility, DecisionVisibility::ActingPlayerOnly);
    assert_eq!(request.decision_domain_v2, DecisionDomainV2::ChooseOne);
    assert_eq!(fight.blockers.len(), 2);
    assert_eq!(
        fight.game.offered(),
        fight.offered_for(state, fight.blockers[0], 3)
    );
    assert_eq!(fight.game.endpoint(P2).visible_decision().unwrap(), None);

    // The division waits in a continuation that names P1, the attacker and both
    // blockers, in P1's opaque order, with nothing answered.
    let [(_, record)] = state.execution.continuations.iter().collect::<Vec<_>>()[..] else {
        panic!("one continuation")
    };
    assert_eq!(record.created_at_revision, state.revision);
    assert_eq!(
        record.payload,
        ContinuationPayload::CombatDamageAssignment {
            player: P1,
            pending_attackers: vec![fight.giant],
            pending_blockers: fight.blockers.clone(),
            assigned: BTreeMap::new(),
        }
    );
    assert_eq!(
        state
            .execution
            .pending_decision
            .as_ref()
            .unwrap()
            .continuation_id,
        Some(record.id)
    );

    // Every one of the four amounts is an answer the game takes, and asks
    // nothing more: the last blocker gets the rest.
    for amount in 0..=3 {
        fight.game.controller.restore(at_request.clone()).unwrap();
        fight.assign(amount);
        let after = fight.game.state();
        assert!(!dividing(&after), "{amount}");
        assert!(after.combat.as_ref().unwrap().damage_step_completed);
        assert_ne!(
            fight.game.pending().1.purpose,
            DecisionPurposeV4::CombatDamageAssignment,
            "{amount}"
        );
    }
}

#[test]
fn each_division_gives_the_expected_deaths() {
    // CR 510.1c, 510.2, 704.5g: Hill Giant (3/3) is blocked by Savannah Lions
    // (2/1) and Gray Ogre (2/2) and divides its 3 damage between them as
    // (to the Lions, to the Ogre). The two deal the Giant 4, which is lethal in
    // every division. What the Giant gives each of them is lethal to the Lions
    // from 1 and to the Ogre from 2.
    //
    // For each division: the damage marked on the Lions and on the Ogre when it
    // survives, and none when it dies.
    type Survivor = Option<u64>;
    let cases: [((u64, u64), Survivor, Survivor); 4] = [
        ((0, 3), Some(0), None),
        ((1, 2), None, None),
        ((2, 1), None, Some(1)),
        ((3, 0), None, Some(0)),
    ];
    for ((to_lions, to_ogre), lions_survives, ogre_survives) in cases {
        let fight = giant_blocked_by(1);
        let [lions] = fight.lions[..] else {
            unreachable!()
        };
        let (giant, ogre) = (fight.giant, fight.ogre);
        let request = fight.open_the_damage_step();
        let before = fight.game.state();
        let first = fight.blockers[0];
        let to_first = if first == lions { to_lions } else { to_ogre };
        let answer = damage_answer(&request, to_first);
        let (product, observed) = product_and_observations(&before, answer.clone());
        let lives = before.core.players.clone();

        let (actor, step) = fight.game.submit(answer);
        assert_eq!(actor, P1, "{to_lions}/{to_ogre}");
        assert_eq!(step.observed_events, observed[&P1]);
        let answered = fight.game.state();
        assert_eq!(answered, product.next_state);
        assert!(!dividing(&answered));
        assert!(answered.combat.as_ref().unwrap().damage_step_completed);
        // Only creatures are dealt damage: nobody loses life.
        assert_eq!(answered.core.players, lives);

        // One event holds the whole assignment: the Giant's division, whose
        // zero amounts are not assignments, and the damage of the blockers.
        let mut expected = vec![
            (giant, lions, to_lions),
            (giant, ogre, to_ogre),
            (lions, giant, 2),
            (ogre, giant, 2),
        ];
        expected.retain(|(_, _, amount)| *amount > 0);
        expected.sort();
        assert_eq!(damage_dealt_to_creatures(&product), expected);

        // In the division (1, 2) both of P2's creatures die together, so P2
        // orders its two cards (CR 404.3) and P1 is not asked: it loses only
        // the Giant. The creatures die with the answer to that request, with
        // the damage they were dealt on them until then.
        if (to_lions, to_ogre) == (1, 2) {
            let (asked, next) = fight.game.pending();
            assert_eq!(
                (asked, &next.purpose),
                (P2, &DecisionPurposeV4::SbaGraveyardOrder)
            );
            assert_eq!(fight.game.endpoint(P1).visible_decision().unwrap(), None);
            assert_eq!(next.candidates.len(), 2);
            assert_eq!(
                [giant, lions, ogre].map(|creature| marked(&answered, creature)),
                [4, 1, 2]
            );
            let wanted: Vec<_> = next
                .candidates
                .iter()
                .map(|candidate| match candidate.intent {
                    CandidateIntent::SelectObject { object } => object,
                    _ => unreachable!(),
                })
                .collect();
            fight.game.submit(order_answer(&next, &wanted));
        }
        let after = fight.game.state();
        assert!(after.execution.continuations.is_empty());
        let (asked, next) = fight.game.pending();
        assert_eq!(
            (asked, next.purpose),
            (P1, DecisionPurposeV4::PriorityAction),
            "{to_lions}/{to_ogre}"
        );

        // The Giant is dealt 4 in every division, and dies.
        assert!(!after.zones.objects.contains_key(&giant));
        for (creature, survives) in [(lions, lions_survives), (ogre, ogre_survives)] {
            match survives {
                Some(damage) => {
                    assert_eq!(
                        after.zones.locations[&creature].zone,
                        ZoneKind::Battlefield,
                        "{to_lions}/{to_ogre}"
                    );
                    assert_eq!(marked(&after, creature), damage, "{to_lions}/{to_ogre}");
                }
                None => assert!(
                    !after.zones.objects.contains_key(&creature),
                    "{to_lions}/{to_ogre}"
                ),
            }
        }
        assert_eq!(graveyard_of(&after, P1).len(), 1);
        let p2_dead = usize::from(lions_survives.is_none()) + usize::from(ogre_survives.is_none());
        assert_eq!(graveyard_of(&after, P2).len(), p2_dead);
    }
}

#[test]
fn a_division_that_runs_out_asks_for_no_more() {
    // CR 510.1c: the Giant's 3 damage is blocked by two Savannah Lions and a
    // Gray Ogre. Giving all 3 to the first blocker leaves nothing for the
    // others, so they are not asked: each is assigned 0.
    let fight = giant_blocked_by(2);
    let request = fight.open_the_damage_step();
    let before = fight.game.state();
    assert_eq!(fight.blockers.len(), 3);
    let first = fight.blockers[0];
    assert_eq!(fight.game.offered(), fight.offered_for(&before, first, 3));
    let answer = damage_answer(&request, 3);
    let (product, _) = product_and_observations(&before, answer.clone());

    fight.game.submit(answer);
    let after = fight.game.state();
    assert_eq!(after, product.next_state);
    // Nothing more is asked: the division is over and P1 has priority.
    assert!(after.execution.continuations.is_empty());
    let (asked, next) = fight.game.pending();
    assert_eq!(
        (asked, next.purpose),
        (P1, DecisionPurposeV4::PriorityAction)
    );

    // The first blocker is assigned all 3, the others nothing; each of the
    // three deals the Giant its own 2.
    let mut expected = vec![(fight.giant, first, 3)];
    expected.extend(
        fight
            .blockers
            .iter()
            .map(|blocker| (*blocker, fight.giant, 2)),
    );
    expected.sort();
    assert_eq!(damage_dealt_to_creatures(&product), expected);
    assert!(!after.zones.objects.contains_key(&first));
    for other in &fight.blockers[1..] {
        assert_eq!(after.zones.locations[other].zone, ZoneKind::Battlefield);
        assert_eq!(marked(&after, *other), 0);
    }
    assert!(!after.zones.objects.contains_key(&fight.giant));
}

#[test]
fn a_restored_partial_division_continues_identically() {
    // The Giant's 3 damage is blocked by two Savannah Lions and a Gray Ogre.
    // P1 gives 1 to the first, which leaves 2 for the other two, and is asked
    // for the second.
    let fight = giant_blocked_by(2);
    let game = &fight.game;
    fight.open_the_damage_step();
    let at_first = game.checkpoint();
    fight.assign(1);
    let half = game.checkpoint();

    // The half-divided damage names who has been answered and who is asked
    // next, and the revision it was created at is the state's less the answers.
    let state = &half.state;
    assert_eq!(
        damage_division(state),
        (
            P1,
            vec![fight.giant],
            fight.blockers[1..].to_vec(),
            BTreeMap::from([(fight.blockers[0], 1)])
        )
    );
    assert_eq!(
        state
            .execution
            .continuations
            .values()
            .next()
            .unwrap()
            .created_at_revision
            .0,
        state.revision.0 - 1
    );
    assert_eq!(
        game.offered(),
        fight.offered_for(state, fight.blockers[1], 2)
    );

    fight.assign(1);
    let divided = game.checkpoint();
    // 1, 1 and the 1 that is left: both Lions die, and P2 orders its two.
    assert_eq!(
        game.pending().1.purpose,
        DecisionPurposeV4::SbaGraveyardOrder
    );

    // The checkpoint with the whole division pending restores the request and
    // its continuation, and the same answers lead to the same checkpoints.
    game.controller.restore(at_first.clone()).unwrap();
    assert_eq!(game.checkpoint(), at_first);
    assert_eq!(
        game.pending().1.purpose,
        DecisionPurposeV4::CombatDamageAssignment
    );
    fight.assign(1);
    assert_eq!(game.checkpoint(), half);

    // So does the half-divided one.
    game.controller.restore(half.clone()).unwrap();
    assert_eq!(game.checkpoint(), half);
    assert_eq!(game.pending().0, P1);
    fight.assign(1);
    assert_eq!(game.checkpoint(), divided);

    // The whole game, the division included, replays to the same checkpoint.
    let report = game
        .controller
        .execute_replay(game.controller.export_replay().unwrap())
        .unwrap();
    assert_eq!(report.final_checkpoint, divided);
}

/// `state` with `edit` applied to its combat damage assignment.
fn forged_division(
    state: &EngineState,
    edit: impl FnOnce(
        &mut PlayerId,
        &mut Vec<GameObjectId>,
        &mut Vec<GameObjectId>,
        &mut BTreeMap<GameObjectId, u64>,
    ),
) -> EngineState {
    let mut forged = state.clone();
    let record = forged.execution.continuations.values_mut().next().unwrap();
    let ContinuationPayload::CombatDamageAssignment {
        player,
        pending_attackers,
        pending_blockers,
        assigned,
    } = &mut record.payload
    else {
        unreachable!()
    };
    edit(player, pending_attackers, pending_blockers, assigned);
    forged
}

#[test]
fn a_forged_partial_division_is_refused() {
    let fight = giant_blocked_by(2);
    let game = &fight.game;
    fight.open_the_damage_step();
    let at_first = game.checkpoint();
    fight.assign(1);
    let half = game.checkpoint();
    let [b0, b1, b2] = fight.blockers[..] else {
        unreachable!()
    };
    let refused = |what: &str, reached: &EnvironmentCheckpointV8, forged: EngineState| {
        let before = game.checkpoint();
        assert!(
            restore_state(game, reached, forged).is_err(),
            "restored: {what}"
        );
        assert_eq!(game.checkpoint(), before, "{what}");
    };

    // The states as played restore.
    restore_state(game, &at_first, at_first.state.clone()).unwrap();
    restore_state(game, &half, half.state.clone()).unwrap();

    // CR 510.1c: all of the attacker's damage is divided, none more. An amount
    // above what is left is refused, however it is placed.
    refused(
        "an amount above the damage the attacker has",
        &half,
        forged_division(&half.state, |_, _, _, assigned| {
            assigned.insert(b0, 4);
        }),
    );
    refused(
        "two amounts that add up to more than the attacker has",
        &half,
        forged_division(&half.state, |_, _, pending, assigned| {
            pending.remove(0);
            assigned.insert(b1, 3);
        }),
    );
    refused(
        "an amount that leaves nothing, with blockers still to ask",
        &half,
        forged_division(&half.state, |_, _, _, assigned| {
            assigned.insert(b0, 3);
        }),
    );
    refused(
        "an answer for a blocker that is not asked before the first",
        &half,
        forged_division(&half.state, |_, _, pending, assigned| {
            pending.clear();
            pending.push(b0);
            assigned.clear();
            assigned.insert(b1, 1);
        }),
    );
    refused(
        "an answer for the blocker that gets the rest",
        &half,
        forged_division(&half.state, |_, _, pending, assigned| {
            pending.truncate(1);
            assigned.insert(b2, 1);
        }),
    );
    refused(
        "an answer for a creature that does not block",
        &half,
        forged_division(&half.state, |_, _, _, assigned| {
            assigned.insert(fight.giant, 1);
        }),
    );

    // The record is made by the transition that opens the damage step and
    // gains one answer with each revision after it, so the revision it was
    // created at is the state's, less the answers. The values next to it and
    // the first revision are refused; the sweep over every other value, which
    // the same check refuses, is not worth its time.
    for (reached, answered) in [(&at_first, 0), (&half, 1)] {
        let created = |state: &EngineState| {
            state
                .execution
                .continuations
                .values()
                .next()
                .unwrap()
                .created_at_revision
        };
        assert_eq!(
            created(&reached.state).0,
            reached.state.revision.0 - answered
        );
        let real = created(&reached.state).0;
        let others: BTreeSet<u64> = [0, real - 1, real + 1]
            .into_iter()
            .filter(|revision| *revision != real)
            .collect();
        assert!(others.len() >= 2, "{others:?}");
        for revision in others {
            let mut forged = reached.state.clone();
            forged
                .execution
                .continuations
                .values_mut()
                .next()
                .unwrap()
                .created_at_revision = StateRevision(revision);
            refused(
                &format!("{answered} answered, created at revision {revision}"),
                reached,
                forged,
            );
        }
    }

    // The blockers asked are the first attacker's, in P1's opaque order.
    refused(
        "the blockers asked in another order",
        &at_first,
        forged_division(&at_first.state, |_, _, pending, _| pending.swap(0, 1)),
    );
    refused(
        "the blockers asked in reverse",
        &at_first,
        forged_division(&at_first.state, |_, _, pending, _| pending.reverse()),
    );
    refused(
        "the remaining blockers swapped",
        &half,
        forged_division(&half.state, |_, _, pending, _| pending.swap(0, 1)),
    );
    refused(
        "a blocker that is asked and answered",
        &half,
        forged_division(&half.state, |_, _, pending, _| pending.insert(0, b0)),
    );
    refused(
        "a blocker that is never asked",
        &at_first,
        forged_division(&at_first.state, |_, _, pending, _| pending.truncate(2)),
    );
    refused(
        "nobody left to ask",
        &half,
        forged_division(&half.state, |_, _, pending, _| pending.clear()),
    );
    refused(
        "no attacker to divide",
        &at_first,
        forged_division(&at_first.state, |_, attackers, _, _| attackers.clear()),
    );
    refused(
        "an attacker twice",
        &at_first,
        forged_division(&at_first.state, |_, attackers, _, _| {
            attackers.push(attackers[0]);
        }),
    );
    refused(
        "the defending player as the one who divides",
        &at_first,
        forged_division(&at_first.state, |player, _, _, _| *player = P2),
    );

    // The damage has not been dealt while it is divided, and nobody has
    // priority.
    for reached in [&at_first, &half] {
        let mut dealt = reached.state.clone();
        dealt.combat.as_mut().unwrap().damage_step_completed = true;
        refused("a division with the damage already dealt", reached, dealt);
        let mut priority = reached.state.clone();
        priority.core.priority = PriorityState::HeldBy {
            player: P1,
            consecutive_passes: 0,
        };
        refused("priority during the division", reached, priority);
        let mut step = reached.state.clone();
        step.core.position = TurnPosition::Combat {
            step: CombatStep::EndOfCombat,
        };
        refused("the division in another step", reached, step);
    }
    // The request is the one the continuation calls for.
    let tampered =
        |reached: &EnvironmentCheckpointV8,
         edit: &dyn Fn(&mut mtgml_decision::AuthoritativeDecisionRequest)| {
            let mut forged = reached.state.clone();
            edit(forged.execution.pending_decision.as_mut().unwrap());
            forged
        };
    refused(
        "the request of the first blocker after it was answered",
        &half,
        tampered(&half, &|request| {
            request.candidates = at_first
                .state
                .execution
                .pending_decision
                .as_ref()
                .unwrap()
                .candidates
                .clone();
        }),
    );
    refused(
        "a request without its largest amount",
        &at_first,
        tampered(&at_first, &|request| {
            request.candidates.pop();
        }),
    );
    refused(
        "a request with an amount too many",
        &at_first,
        tampered(&at_first, &|request| {
            let mut last = request.candidates.last().unwrap().clone();
            last.candidate_id = CandidateIdV1(request.candidates.len() as u32);
            if let (
                CandidateIntent::AssignCombatDamage { amount, .. },
                mtgml_decision::EngineCandidateBinding::AssignCombatDamage {
                    amount: bound, ..
                },
            ) = (&mut last.visible_intent, &mut last.trusted_binding)
            {
                *amount += 1;
                *bound += 1;
            }
            request.candidates.push(last);
        }),
    );
    refused(
        "a request for the blocker after the first",
        &at_first,
        tampered(&at_first, &|request| {
            for candidate in &mut request.candidates {
                if let (
                    CandidateIntent::AssignCombatDamage { recipient, .. },
                    mtgml_decision::EngineCandidateBinding::AssignCombatDamage {
                        recipient: bound,
                        ..
                    },
                ) = (
                    &mut candidate.visible_intent,
                    &mut candidate.trusted_binding,
                ) {
                    *recipient = opaque_of(&at_first.state, P1, b1);
                    *bound = b1;
                }
            }
        }),
    );
    refused(
        "a public request",
        &half,
        tampered(&half, &|request| {
            request.visibility = DecisionVisibility::Public
        }),
    );
    refused(
        "a request of the defending player",
        &half,
        tampered(&half, &|request| request.actor = P2),
    );
    refused(
        "a request without its continuation",
        &half,
        tampered(&half, &|request| request.continuation_id = None),
    );
    refused(
        "a request for several answers",
        &half,
        tampered(&half, &|request| {
            request.decision_domain_v2 = DecisionDomainV2::ChooseMany {
                minimum: 0,
                maximum: 2,
            }
        }),
    );
    refused(
        "a priority request in the middle of the division",
        &half,
        tampered(&half, &|request| {
            request.purpose = DecisionPurposeV4::PriorityAction
        }),
    );

    // A division needs the rule, and is never part of a closed episode.
    let admission = creature_game_admission();
    let running = EpisodeStatus::Running;
    mtgml_rules::validate_magic_pending_request(&admission, &half.state, &running).unwrap();
    assert!(mtgml_rules::validate_magic_pending_request(
        &common::game_admission(),
        &half.state,
        &running
    )
    .is_err());
    let closed = EpisodeStatus::Truncated {
        reason: TruncationReason::ExternalStop,
        players: Vec::new(),
    };
    assert!(mtgml_rules::validate_magic_pending_request(&admission, &half.state, &closed).is_err());
}

#[test]
fn a_half_divided_damage_is_invisible_to_the_defender() {
    // CR 510.1c: P1 divides the Giant's damage one blocker at a time. P2 sees
    // nothing of it, not the request, not an answer, and not what is left: only
    // P1 sees its own answers (see
    // `the_attacking_player_sees_its_own_partial_division`).
    let fight = giant_blocked_by(2);
    let game = &fight.game;
    fight.open_the_damage_step();
    let information = game.information_bytes(P2);
    let observation = game
        .endpoint(P2)
        .information_state()
        .unwrap()
        .current_observation;
    let seen = game.seen(P2);
    assert_eq!(game.endpoint(P2).visible_decision().unwrap(), None);

    let (_, step) = fight.assign(1);
    assert!(step.observed_events.is_empty());
    assert_eq!(game.pending().0, P1, "P1 divides on");
    assert_eq!(game.information_bytes(P2), information);
    assert_eq!(
        game.endpoint(P2)
            .information_state()
            .unwrap()
            .current_observation,
        observation
    );
    assert_eq!(game.seen(P2), seen);
    assert_eq!(game.endpoint(P2).visible_decision().unwrap(), None);

    // Positive control: the same comparison does see what is public. The last
    // answer makes both Lions die together, so P2 is asked to arrange its two
    // cards, which both players see; and when it has, they see the deaths.
    fight.assign(1);
    assert_eq!(game.pending().0, P2);
    assert_ne!(game.information_bytes(P2), information);
    assert_ne!(game.seen(P2), seen);
    let (_, request) = game.pending();
    let wanted: Vec<_> = request
        .candidates
        .iter()
        .map(|candidate| match candidate.intent {
            CandidateIntent::SelectObject { object } => object,
            _ => unreachable!(),
        })
        .collect();
    let (_, step) = game.submit(order_answer(&request, &wanted));
    assert!(!step.observed_events.is_empty());
}

/// P1's Hill Giant (3/3) is blocked by four creatures of P2: three Savannah
/// Lions and a Gray Ogre, so that P1 answers twice with the division still
/// going on (a creature is asked about while two or more have no amount). P2
/// casts a Lions on turns 2, 4 and 8 and the Ogre on turn 6.
fn giant_blocked_by_four() -> GiantFight {
    let [_, ogre_card, giant_card] = creature_definitions();
    let (mountain, plains) = land_definitions();
    let game = Game::with_hands_and_libraries(
        [
            vec![mountain, mountain, mountain, mountain, giant_card],
            vec![plains, plains, lions(), ogre_card],
        ],
        [
            vec![mountain; 10],
            [
                vec![mountain, lions(), mountain, lions()],
                vec![mountain; 6],
            ]
            .concat(),
        ],
    );
    game.run_until(start_of_main_phase(2));
    cast_lions(&game);
    game.run_until(start_of_main_phase(4));
    cast_lions(&game);
    game.run_until(start_of_main_phase(6));
    cast_creature(&game, 3);
    game.run_until(start_of_main_phase(7));
    cast_creature(&game, 4);
    game.run_until(start_of_main_phase(8));
    cast_lions(&game);
    game.run_until(at_attackers(9));
    let fight = every_creature_blocks_the_giant(game, 3);
    assert_eq!(fight.blockers.len(), 4);
    fight
}

/// The division answers P1 is shown for the Giant, one for each pair of a
/// blocker and its amount, in the order given.
fn assigned_damage(
    fight: &GiantFight,
    answers: &[(GameObjectId, u64)],
) -> Vec<AssignedDamageObservationV1> {
    let state = fight.game.state();
    answers
        .iter()
        .map(|(blocker, amount)| AssignedDamageObservationV1 {
            attacker: opaque_of(&state, P1, fight.giant),
            blocker: opaque_of(&state, P1, *blocker),
            amount: *amount,
        })
        .collect()
}

#[test]
fn the_attacking_player_sees_its_own_partial_division() {
    // CR 510.1c: P1 divides the Giant's 3 damage one blocker at a time. The
    // amounts answered so far are P1's own and shown to P1 alone, in P1's
    // opaque ids. P2 is shown nothing, and P2's bytes do not change.
    let fight = giant_blocked_by_four();
    let game = &fight.game;
    let state = game.state();
    let pending = |player: PlayerId| game.seen(player).observation.pending_damage_assignment;
    let wire =
        |player: PlayerId| game.observation_json(player)["pending_damage_assignment"].clone();
    let id = |object: GameObjectId| opaque_of(&state, P1, object).0.to_string();

    // The step has not opened: no division is pending.
    assert_eq!((pending(P1), pending(P2)), (None, None));
    fight.open_the_damage_step();

    // The division has begun and no blocker has been answered: P1's list is
    // empty, and not null.
    assert_eq!(pending(P1), Some(Vec::new()));
    assert_eq!(wire(P1), serde_json::json!([]));
    assert_eq!(pending(P2), None);
    assert!(wire(P2).is_null());
    let information = game.information_bytes(P2);

    // The first blocker is given 1 of the 3, which leaves 2.
    let (actor, step) = fight.assign(1);
    assert_eq!(actor, P1);
    assert!(step.observed_events.is_empty());
    assert_eq!(
        pending(P1),
        Some(assigned_damage(&fight, &[(fight.blockers[0], 1)]))
    );
    assert_eq!(
        wire(P1),
        serde_json::json!([{
            "attacker": id(fight.giant),
            "blocker": id(fight.blockers[0]),
            "amount": "1",
        }])
    );
    assert_eq!(pending(P2), None);
    assert!(wire(P2).is_null());
    assert_eq!(game.information_bytes(P2), information);

    // The second is given 0, and that is an answer too.
    fight.assign(0);
    assert_eq!(
        pending(P1),
        Some(assigned_damage(
            &fight,
            &[(fight.blockers[0], 1), (fight.blockers[1], 0)]
        ))
    );
    assert_eq!(
        wire(P1),
        serde_json::json!([
            {"attacker": id(fight.giant), "blocker": id(fight.blockers[0]), "amount": "1"},
            {"attacker": id(fight.giant), "blocker": id(fight.blockers[1]), "amount": "0"},
        ])
    );
    assert_eq!(pending(P2), None);
    assert_eq!(game.information_bytes(P2), information);
    assert_eq!(game.endpoint(P2).visible_decision().unwrap(), None);
    assert!(dividing(&game.state()));

    // The third is given the 2 that are left, and the fourth gets none: the
    // division is complete and the damage is dealt. Nothing is pending for
    // anybody.
    fight.assign(2);
    assert!(!dividing(&game.state()));
    for player in [P1, P2] {
        assert_eq!(pending(player), None, "{player:?}");
        assert!(wire(player).is_null(), "{player:?}");
    }
}

#[test]
fn a_restored_partial_division_gives_the_same_observation() {
    // The partial answers are rederived from the state, so a checkpoint taken
    // with the division pending shows each player what the game showed at that
    // moment.
    let fight = giant_blocked_by_four();
    let game = &fight.game;
    fight.open_the_damage_step();
    let shown = || [game.information_bytes(P1), game.information_bytes(P2)];
    let at_first = (game.checkpoint(), shown());
    fight.assign(1);
    let after_one = (game.checkpoint(), shown());
    fight.assign(0);
    let after_two = (game.checkpoint(), shown());

    assert_ne!(at_first.1[0], after_one.1[0]);
    assert_ne!(after_one.1[0], after_two.1[0]);
    for (checkpoint, expected) in [&after_one, &at_first, &after_two, &after_one] {
        game.controller.restore(checkpoint.clone()).unwrap();
        assert_eq!(&game.checkpoint(), checkpoint);
        assert_eq!(&shown(), expected);
    }

    // Playing on from the restored half leads to what the game showed.
    fight.assign(0);
    assert_eq!(game.checkpoint(), after_two.0);
    assert_eq!(shown(), after_two.1);
}

#[test]
fn the_partial_division_is_listed_in_the_order_of_the_attackers_opaque_ids() {
    // The order of the objects means nothing to a player. P1 has answered for
    // two blockers, and the ids P1 knows them by are in the opposite order to
    // the objects: the answers are listed in the order of the ids.
    let fight = giant_blocked_by_four();
    let game = &fight.game;
    fight.open_the_damage_step();
    fight.assign(1);
    fight.assign(0);
    let reached = game.checkpoint();
    let (first, second) = (fight.blockers[0], fight.blockers[1]);
    let mut forged = reached.state.clone();
    swap_opaque_ids(&mut forged, P1, first, second);
    assert!(first < second);
    assert!(opaque_of(&forged, P1, second) < opaque_of(&forged, P1, first));
    restore_state(game, &reached, forged.clone()).unwrap();

    let giant = opaque_of(&forged, P1, fight.giant);
    let row = |blocker: GameObjectId, amount: u64| AssignedDamageObservationV1 {
        attacker: giant,
        blocker: opaque_of(&forged, P1, blocker),
        amount,
    };
    assert_eq!(
        game.seen(P1).observation.pending_damage_assignment,
        Some(vec![row(second, 0), row(first, 1)])
    );
    let wire = game.observation_json(P1)["pending_damage_assignment"].clone();
    let listed: Vec<u64> = wire
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["blocker"].as_str().unwrap().parse().unwrap())
        .collect();
    assert_eq!(listed.len(), 2);
    assert!(listed[0] < listed[1], "{listed:?}");
    assert_eq!(game.seen(P2).observation.pending_damage_assignment, None);

    // The division goes on from the restored state.
    fight.assign(2);
    assert!(!dividing(&game.state()));
    for player in [P1, P2] {
        assert_eq!(
            game.seen(player).observation.pending_damage_assignment,
            None
        );
    }
}

#[test]
fn both_players_see_combat_damage_and_marks() {
    // CR 510.1, 510.2, 120.3e: P1's Hill Giant (3/3) attacks and P2's Savannah
    // Lions (2/1) blocks it. Both players observe the damage dealt, once, with
    // both assignments in their own opaque ids, ahead of the Lions' death;
    // the Giant survives with 2 damage marked, which both see in their
    // observation, as a decimal string, and stays an attacking, blocked
    // creature without a blocker. No event shows the mark.
    let [_, _, giant] = creature_definitions();
    let (game, giant_object, lions_object) = attacker_against_a_lions(giant, 4);
    attack_and_block(&game, giant_object);
    let before = game.state();
    // Before the damage nothing is marked on any permanent.
    for player in [P1, P2] {
        let observation = game.seen(player).observation;
        assert!(observation
            .permanents
            .iter()
            .all(|permanent| permanent.marked_damage == 0));
    }
    let (product, observed) = product_and_observations(&before, pass_in(&before));
    let (actor, step) = game.answer(pass, pass);
    assert_eq!(actor, P2);
    assert_eq!(step.observed_events, observed[&P2]);
    let after = game.state();
    assert_eq!(after, product.next_state);

    for player in [P1, P2] {
        let events = &observed[&player];
        let giant_id = opaque_of(&before, player, giant_object);
        let lions_id = opaque_of(&before, player, lions_object);
        // One CombatDamageDealt holds both assignments: the Giant's 3 to the
        // Lions, the Lions' 2 to the Giant, sorted by the player's source id.
        assert_eq!(
            combat_events(events),
            [&ObservedEventKindV4::CombatDamageDealt {
                assignments: observed_damage(
                    &before,
                    player,
                    &[
                        (giant_object, lions_object, 3),
                        (lions_object, giant_object, 2)
                    ]
                )
            }],
            "{player:?}"
        );
        // It comes first, and the Lions' death is the only other event, seen
        // as before: the card keeps the opaque id the player knew it by.
        assert_eq!(events.len(), 2, "{player:?}: {events:?}");
        assert!(matches!(
            events[0].event,
            ObservedEventKindV4::CombatDamageDealt { .. }
        ));
        assert_eq!(
            observed_moves(events),
            [(
                Some(lions_id),
                Some(lions_id),
                ZoneKind::Battlefield,
                ZoneKind::Graveyard
            )],
            "{player:?}"
        );
        assert_eq!(events[1].sequence.0, events[0].sequence.0 + 1);

        // The Giant has 2 damage marked, and no permanent has any other.
        let observation = game.seen(player).observation;
        let damaged: Vec<_> = observation
            .permanents
            .iter()
            .filter(|permanent| permanent.marked_damage != 0)
            .map(|permanent| (permanent.object, permanent.marked_damage))
            .collect();
        assert_eq!(damaged, [(giant_id, 2)], "{player:?}");
        // The Giant is still attacking, and still blocked with its blocker
        // gone (CR 509.1h): no creature blocks.
        assert_eq!(observation.attacking, [giant_id], "{player:?}");
        assert_eq!(observation.blocked, [giant_id], "{player:?}");
        assert_eq!(observation.blocking, [], "{player:?}");
        // On the wire it is a decimal string, "0" for every other permanent.
        let wire = game.observation_json(player);
        let rows = wire["permanents"].as_array().unwrap();
        assert!(rows.len() > 1);
        let giant_wire = giant_id.0.to_string();
        for row in rows {
            let expected = if row["object"] == giant_wire.as_str() {
                "2"
            } else {
                "0"
            };
            assert_eq!(row["marked_damage"], expected, "{player:?}: {row}");
        }
    }
}

#[test]
fn both_players_see_a_blocker_whose_attacker_died_blocking_nothing() {
    // CR 509.1g, 506.4: P1's Savannah Lions (2/1) attacks and P2's Hill Giant
    // (3/3) blocks it. The Lions dies, and the Giant, which survives with 2
    // damage marked, stays a blocking creature that blocks nothing until
    // combat ends. Both players see it: a `blocking` entry with no attacker,
    // and no blocked or attacking creature.
    let (game, lions_object, giant_object) = a_lions_blocked_by_a_giant();
    let before = game.state();
    for player in [P1, P2] {
        let observation = game.seen(player).observation;
        let lions_id = opaque_of(&before, player, lions_object);
        assert_eq!(observation.attacking, [lions_id], "{player:?}");
        assert_eq!(observation.blocked, [lions_id], "{player:?}");
        assert_eq!(
            observation.blocking,
            [BlockObservationV1 {
                blocker: opaque_of(&before, player, giant_object),
                attacker: Some(lions_id)
            }],
            "{player:?}"
        );
    }

    game.answer(pass, pass);
    let after = game.state();
    assert!(!after.zones.objects.contains_key(&lions_object));
    assert_eq!(
        after.combat.as_ref().unwrap().blockers,
        BTreeMap::from([(giant_object, None)])
    );
    for player in [P1, P2] {
        let observation = game.seen(player).observation;
        let giant_id = opaque_of(&after, player, giant_object);
        assert_eq!(observation.attacking, [], "{player:?}");
        assert_eq!(observation.blocked, [], "{player:?}");
        assert_eq!(
            observation.blocking,
            [BlockObservationV1 {
                blocker: giant_id,
                attacker: None
            }],
            "{player:?}"
        );
        assert_eq!(game.marked_damage(player, giant_id), 2, "{player:?}");
        // On the wire the missing attacker is a null, and the key is there.
        let wire = game.observation_json(player);
        let row = wire["blocking"][0].as_object().unwrap();
        assert_eq!(row.len(), 2);
        assert_eq!(row["attacker"], serde_json::Value::Null);
        assert_eq!(row["blocker"], giant_id.0.to_string());
    }

    // It blocks nothing until the combat phase ends, and then the combat is
    // gone: nobody blocks.
    game.pass_until(|state| {
        state.core.position
            == TurnPosition::Combat {
                step: CombatStep::EndOfCombat,
            }
    });
    for player in [P1, P2] {
        assert_eq!(game.seen(player).observation.blocking.len(), 1);
    }
    game.pass_until(|state| state.core.position == TurnPosition::PostcombatMain);
    for player in [P1, P2] {
        let observation = game.seen(player).observation;
        assert_eq!(observation.blocking, [], "{player:?}");
        assert_eq!(observation.blocked, [], "{player:?}");
    }
}

#[test]
fn an_attack_nobody_can_block_shows_an_empty_block_declaration() {
    // CR 508.8, 509.1: P2 has no creature, so nothing is asked of it, and the
    // declare blockers step's turn-based action is an empty declaration that
    // both players are told of: the same event shape as an attack that is
    // declined to be blocked.
    let (mountain, plains) = land_definitions();
    let game = Game::with_hands([vec![plains, lions()], vec![mountain]]);
    cast_lions(&game);
    game.run_until(at_attackers(3));
    let state = game.state();
    let attacker = lions_of(&state, P1)[0];
    game.declare_attackers(&[opaque_of(&state, P1, attacker)]);
    game.answer(pass, pass);
    let before = game.state();
    assert_eq!(game.pending().0, P2);
    let (product, observed) = product_and_observations(&before, pass_in(&before));
    let (actor, step) = game.answer(pass, pass);
    assert_eq!(actor, P2);
    assert_eq!(step.observed_events, observed[&P2]);
    assert_eq!(game.state(), product.next_state);

    for player in [P1, P2] {
        assert_eq!(
            combat_events(&observed[&player]),
            [&ObservedEventKindV4::BlockersDeclared {
                defending_player: P2,
                blocks: Vec::new()
            }],
            "{player:?}"
        );
        let observation = game.seen(player).observation;
        assert_eq!(
            observation.attacking,
            [opaque_of(&state, player, attacker)],
            "{player:?}"
        );
        assert_eq!(observation.blocked, [], "{player:?}");
        assert_eq!(observation.blocking, [], "{player:?}");
    }
    // The step is the declare blockers step, and the attacker has priority.
    assert_eq!(
        game.state().core.position,
        TurnPosition::Combat {
            step: CombatStep::DeclareBlockers
        }
    );
    assert_eq!(game.pending().0, P1);
    // The unblocked Lions then deals its damage to P2, and nothing is blocked.
    let life = game.state().core.players[&P2].life;
    game.answer(pass, pass);
    game.answer(pass, pass);
    assert_eq!(game.state().core.players[&P2].life, life - 2);
}

/// P1 casts a Savannah Lions on turn 1 and P2 a Hill Giant on turn 8; on turn
/// 9 the Lions attacks and the Giant blocks it. The Lions dies, and the Giant
/// survives with 2 damage marked until the cleanup step. The game goes on to the
/// start of P2's turn 10. Each player also holds a card that is never played:
/// `hidden` for `hider`, a Gray Ogre for the other. The cards of the `hider`'s
/// library below the first five, which are not drawn by then, are in the order
/// `library_tail` gives: `true` for a Plains, `false` for a Mountain.
fn a_fight_with_a_hidden_card(
    hider: PlayerId,
    hidden: CardDefinitionId,
    library_tail: [bool; 5],
) -> Game {
    let (mountain, plains) = land_definitions();
    let [_, ogre, giant] = creature_definitions();
    let mut extra = [ogre, ogre];
    extra[usize::from(hider != P1)] = hidden;
    let hands = [
        vec![plains, lions(), extra[0]],
        [vec![mountain; 4], vec![giant, extra[1]]].concat(),
    ];
    let mut libraries = [vec![mountain; 10], vec![mountain; 10]];
    libraries[usize::from(hider != P1)] = vec![mountain; 5]
        .into_iter()
        .chain(library_tail.map(|white| if white { plains } else { mountain }))
        .collect();
    let game = Game::with_hands_and_libraries(hands, libraries);
    game.record_views();
    cast_lions(&game);
    game.run_until(start_of_main_phase(8));
    cast_creature(&game, 4);
    game.run_until(at_attackers(9));
    let state = game.state();
    let [lions_object]: [GameObjectId; 1] = lions_of(&state, P1).try_into().unwrap();
    let [giant_object]: [GameObjectId; 1] = creatures_of(&state, P2, giant).try_into().unwrap();
    attack_and_block(&game, lions_object);
    // The fight is in what both players see: the block, then the damage.
    for player in [P1, P2] {
        assert_eq!(game.seen(player).observation.blocking.len(), 1);
    }
    game.answer(pass, pass);
    for player in [P1, P2] {
        let giant_id = opaque_of(&state, player, giant_object);
        assert_eq!(game.marked_damage(player, giant_id), 2, "{player:?}");
    }
    game.pass_until(start_of_main_phase(10));
    game
}

#[test]
fn a_player_learns_nothing_of_the_opponents_hidden_cards_through_a_fight() {
    // Noninterference over a whole fight, with the blocks, the damage, the
    // marks and the deaths in it: two games that differ only in what one
    // player holds unseen (a card in hand that is never played, and the order
    // of cards in their library that are not drawn) give the other player the
    // same steps, information states, observations and events, byte for byte.
    // `game_start::a_player_learns_nothing_about_the_opponents_deck_order` is
    // land-only and ends at the first upkeep, so it never reaches combat.
    let [lions, ogre, _] = creature_definitions();
    for (observer, hider) in [(P1, P2), (P2, P1)] {
        let games = [
            a_fight_with_a_hidden_card(hider, lions, [false, false, true, true, false]),
            a_fight_with_a_hidden_card(hider, ogre, [false, true, false, false, true]),
        ];
        let seen: Vec<_> = games.iter().map(|game| game.views_of(observer)).collect();
        assert!(seen[0].len() > 40, "{}", seen[0].len());
        assert_eq!(seen[0], seen[1], "{observer:?} learned something");
        // The games do differ: the hider holds other cards.
        let held: Vec<_> = games.iter().map(|game| game.views_of(hider)).collect();
        assert_ne!(held[0], held[1], "{hider:?}");
    }
}
