//! Detached G0 authoritative event identity and sequential cursor.
//!
//! These are transition products, not a durable log and not a replay control
//! stream. G0j alone connects a successor producer to the current runtime.

use mtgml_model::{
    ContinuationId, DecisionId, GameObjectId, PlayerId, RuleEventId, StackObjectId, StateRevision,
    TriggerInstanceId, ZoneKind,
};
use mtgml_random::RandomStreamKeyV1;
use mtgml_state::{
    ActionCostFacts, CostCommitActionV1, CostFacts, DamageKind, DamageRecipient, EngineState,
    ManaPoolChangeCauseV1, ManaPoolV1, ManaSourceActivation, PendingTriggerRecord,
    PerspectiveLifecycleAuditV1, SemanticDeltaOperation, SourceContext, StackItemEndKindV1,
    StackItemPayload, StateDelta, TargetBinding, TemporaryEffectRecord, TurnPosition,
    ZoneTransition,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoritativeRuleEvent {
    pub event_id: RuleEventId,
    pub state_revision: StateRevision,
    pub event: AuthoritativeRuleEventKind,
}

impl AuthoritativeRuleEvent {
    fn semantic_operations(&self) -> Vec<SemanticDeltaOperation> {
        if let AuthoritativeRuleEventKind::CounterChanged {
            object,
            kind,
            before,
            after,
        } = &self.event
        {
            return vec![SemanticDeltaOperation::CounterChanged {
                object: *object,
                kind: *kind,
                from: *before,
                to: *after,
                cause: self.event_id,
            }];
        }
        self.event.semantic_operations()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthoritativeRuleEventKind {
    ZoneTransition {
        transition: Box<ZoneTransition>,
    },
    ObjectCeasedToExist {
        object: GameObjectId,
    },
    LifeChanged {
        player: PlayerId,
        from: i64,
        to: i64,
    },
    CombatDamageDealt {
        assignments: Vec<mtgml_state::DamageAssignmentV1>,
    },
    CombatDamageStepCompleted,
    MarkedDamageChanged {
        creature: GameObjectId,
        from: u64,
        to: u64,
    },
    ObjectTapped {
        object: GameObjectId,
        from: bool,
        to: bool,
    },
    DecisionCreated {
        decision: DecisionId,
    },
    DecisionCleared {
        decision: DecisionId,
    },
    SbaGraveyardOrderChosen {
        continuation: ContinuationId,
        owner: PlayerId,
        top_to_bottom: Vec<GameObjectId>,
    },
    StateBasedActionsApplied {
        actions: Vec<mtgml_state::SbaSelectedActionV1>,
    },
    PriorityChanged {
        from: mtgml_state::PriorityState,
        to: mtgml_state::PriorityState,
    },
    RandomValueSampled {
        stream: RandomStreamKeyV1,
        bound: u64,
        value: u64,
        raw_words_consumed: u64,
        cursor_before: u64,
        cursor_after: u64,
    },
    PublicOutcome {
        code: String,
    },
    TurnPositionChanged {
        from: TurnPosition,
        to: TurnPosition,
    },
    AttackersDeclared {
        defending_player: PlayerId,
        attackers: Vec<GameObjectId>,
    },
    BlockersDeclared {
        assignments: Vec<mtgml_state::CombatBlockerAssignmentV1>,
    },
    CombatEnded,
    EmptyCombatStepsSkipped,
    UntapCompleted {
        affected_objects: Vec<GameObjectId>,
    },
    ActivePlayerChanged {
        from: PlayerId,
        to: PlayerId,
    },
    TurnNumberChanged {
        from: u64,
        to: u64,
    },
    StackItemAdded {
        stack_object: StackObjectId,
        payload: StackItemPayload,
    },
    StackItemRemoved {
        stack_object: StackObjectId,
        payload: StackItemPayload,
        result: StackItemEndKindV1,
    },
    SpellCast {
        stack_object: StackObjectId,
        spell_object: GameObjectId,
        card_definition: mtgml_model::CardDefinitionId,
        face_key: mtgml_card_ir::FaceKey,
        semantic_profile_id: mtgml_card_ir::CardSemanticProfileId,
        is_creature_spell: bool,
        cost_facts: CostFacts,
    },
    AbilityActivated {
        stack_object: StackObjectId,
        source: mtgml_state::AbilitySourceContext,
        targets: Vec<TargetBinding>,
        cost_facts: CostFacts,
        once_per_turn_use_committed: bool,
    },
    TargetDeclared {
        source_stack_item: StackObjectId,
        targets: Vec<TargetBinding>,
    },
    CounterChanged {
        object: GameObjectId,
        kind: mtgml_state::CounterKindV1,
        before: u32,
        after: u32,
    },
    TriggerDetected {
        trigger: PendingTriggerRecord,
    },
    TriggerPlaced {
        trigger: TriggerInstanceId,
        stack_object: StackObjectId,
        payload: StackItemPayload,
    },
    ManaPoolChanged {
        player: PlayerId,
        before: ManaPoolV1,
        after: ManaPoolV1,
        cause: ManaPoolChangeCauseV1,
    },
    CostCommitted {
        actor: PlayerId,
        action: CostCommitActionV1,
        facts: ActionCostFacts,
        source_activations: Vec<ManaSourceActivation>,
        spent_buckets: [u32; 12],
    },
    TemporaryEffectCreated {
        effect: TemporaryEffectRecord,
    },
    TemporaryEffectExpired {
        effect: TemporaryEffectRecord,
    },
    /// A perspective-visible occurrence associated with another authoritative
    /// event in this same transition. The source ID is trusted-only; the
    /// environment projects the referenced rule event through opaque IDs.
    PerspectiveObservationOccurrence {
        lifecycle: Box<PerspectiveLifecycleAuditV1>,
        source_event_id: RuleEventId,
    },
    DamageApplied {
        source: Option<SourceContext>,
        recipient: DamageRecipient,
        post_replacement_amount: u32,
        damage_kind: DamageKind,
    },
    /// CR 103.1: the chooser picked who takes the first turn. Public.
    StartingPlayerChosen {
        chooser: PlayerId,
        starting_player: PlayerId,
    },
    /// CR 103.5: a player kept their hand (`mulligan: false`) or took a
    /// mulligan. Public.
    MulliganDeclared {
        player: PlayerId,
        mulligan: bool,
    },
    /// CR 103.3, 701.24: one library was shuffled. A trusted audit of the
    /// draw (RNG_CONTRACT.md): the stream, its cursors and the resulting
    /// order. Never projected.
    LibraryShuffled {
        player: PlayerId,
        stream: RandomStreamKeyV1,
        cursor_before: u64,
        cursor_after: u64,
        raw_words_consumed: u64,
        top_to_bottom: Vec<GameObjectId>,
    },
}

impl AuthoritativeRuleEventKind {
    /// Returns the typed audit operation(s) corresponding to this event.
    /// Bookkeeping-only changes remain in StateDelta replacement state and do
    /// not receive fabricated rule events.
    pub fn semantic_operations(&self) -> Vec<SemanticDeltaOperation> {
        match self {
            Self::ZoneTransition { transition } => vec![SemanticDeltaOperation::ZoneTransition {
                transition: transition.clone(),
            }],
            Self::ObjectCeasedToExist { object } => {
                vec![SemanticDeltaOperation::ObjectCeasedToExist { object: *object }]
            }
            Self::LifeChanged { player, from, to } => vec![SemanticDeltaOperation::LifeChanged {
                player: *player,
                from: *from,
                to: *to,
            }],
            Self::CombatDamageDealt { assignments } => {
                vec![SemanticDeltaOperation::CombatDamageDealt {
                    assignments: assignments.clone(),
                }]
            }
            Self::CombatDamageStepCompleted => {
                vec![SemanticDeltaOperation::CombatDamageStepCompleted]
            }
            Self::MarkedDamageChanged { creature, from, to } => {
                vec![SemanticDeltaOperation::MarkedDamageChanged {
                    creature: *creature,
                    from: *from,
                    to: *to,
                }]
            }
            Self::ObjectTapped { object, from, to } => {
                vec![SemanticDeltaOperation::ObjectTapped {
                    object: *object,
                    from: *from,
                    to: *to,
                }]
            }
            Self::DecisionCreated { decision } => vec![SemanticDeltaOperation::DecisionCreated {
                decision: *decision,
            }],
            Self::DecisionCleared { decision } => vec![SemanticDeltaOperation::DecisionCleared {
                decision: *decision,
            }],
            Self::SbaGraveyardOrderChosen {
                continuation,
                owner,
                top_to_bottom,
            } => vec![SemanticDeltaOperation::SbaGraveyardOrderChosen {
                continuation: *continuation,
                owner: *owner,
                top_to_bottom: top_to_bottom.clone(),
            }],
            Self::StateBasedActionsApplied { actions } => {
                vec![SemanticDeltaOperation::StateBasedActionsApplied {
                    actions: actions.clone(),
                }]
            }
            Self::PriorityChanged { from, to } => vec![SemanticDeltaOperation::PriorityChanged {
                from: *from,
                to: *to,
            }],
            Self::RandomValueSampled {
                stream,
                bound,
                value,
                raw_words_consumed,
                cursor_before,
                cursor_after,
            } => vec![SemanticDeltaOperation::RandomValueSampled {
                stream: *stream,
                bound: *bound,
                value: *value,
                raw_words_consumed: *raw_words_consumed,
                cursor_before: *cursor_before,
                cursor_after: *cursor_after,
            }],
            Self::PublicOutcome { code } => {
                vec![SemanticDeltaOperation::PublicOutcome { code: code.clone() }]
            }
            Self::TurnPositionChanged { from, to } => {
                vec![SemanticDeltaOperation::TurnPositionChanged {
                    from: *from,
                    to: *to,
                }]
            }
            Self::AttackersDeclared {
                defending_player,
                attackers,
            } => vec![SemanticDeltaOperation::AttackersDeclared {
                defending_player: *defending_player,
                attackers: attackers.clone(),
            }],
            Self::BlockersDeclared { assignments } => {
                vec![SemanticDeltaOperation::BlockersDeclared {
                    assignments: assignments.clone(),
                }]
            }
            Self::CombatEnded => vec![SemanticDeltaOperation::CombatEnded],
            Self::EmptyCombatStepsSkipped => {
                vec![SemanticDeltaOperation::EmptyCombatStepsSkipped]
            }
            Self::UntapCompleted { affected_objects } => {
                vec![SemanticDeltaOperation::UntapCompleted {
                    affected_objects: affected_objects.clone(),
                }]
            }
            Self::ActivePlayerChanged { from, to } => {
                vec![SemanticDeltaOperation::ActivePlayerChanged {
                    from: *from,
                    to: *to,
                }]
            }
            Self::TurnNumberChanged { from, to } => {
                vec![SemanticDeltaOperation::TurnNumberChanged {
                    from: *from,
                    to: *to,
                }]
            }
            Self::StackItemAdded {
                stack_object,
                payload,
            } => vec![SemanticDeltaOperation::StackItemCreated {
                stack_object: *stack_object,
                payload: Box::new(payload.clone()),
            }],
            Self::StackItemRemoved {
                stack_object,
                payload,
                result,
            } => vec![SemanticDeltaOperation::StackItemEnded {
                stack_object: *stack_object,
                payload: Box::new(payload.clone()),
                result: *result,
            }],
            Self::SpellCast {
                stack_object,
                spell_object,
                card_definition,
                face_key,
                semantic_profile_id,
                is_creature_spell,
                cost_facts,
            } => vec![SemanticDeltaOperation::SpellCast {
                stack_object: *stack_object,
                spell_object: *spell_object,
                card_definition: *card_definition,
                face_key: *face_key,
                semantic_profile_id: semantic_profile_id.clone(),
                is_creature_spell: *is_creature_spell,
                cost_facts: cost_facts.clone(),
            }],
            Self::AbilityActivated {
                stack_object,
                source,
                targets,
                cost_facts,
                once_per_turn_use_committed,
            } => vec![SemanticDeltaOperation::AbilityActivated {
                stack_object: *stack_object,
                source: Box::new(source.clone()),
                targets: targets.clone(),
                cost_facts: cost_facts.clone(),
                once_per_turn_use_committed: *once_per_turn_use_committed,
            }],
            Self::TargetDeclared {
                source_stack_item,
                targets,
            } => vec![SemanticDeltaOperation::TargetDeclared {
                source_stack_item: *source_stack_item,
                targets: targets.clone(),
            }],
            Self::CounterChanged { .. } => Vec::new(),
            Self::TriggerDetected { trigger } => {
                vec![SemanticDeltaOperation::TriggerCreated {
                    trigger: Box::new(trigger.clone()),
                }]
            }
            Self::TriggerPlaced {
                trigger,
                stack_object,
                payload,
            } => vec![SemanticDeltaOperation::TriggerPlaced {
                trigger: *trigger,
                stack_object: *stack_object,
                payload: Box::new(payload.clone()),
            }],
            Self::ManaPoolChanged {
                player,
                before,
                after,
                cause,
            } => vec![SemanticDeltaOperation::ManaPoolChanged {
                player: *player,
                from: *before,
                to: *after,
                cause: *cause,
            }],
            Self::CostCommitted {
                actor,
                action,
                facts,
                source_activations,
                spent_buckets,
            } => vec![SemanticDeltaOperation::AtomicCostCommitted {
                actor: *actor,
                action: *action,
                mana_cost: facts.mana_cost,
                source_activations: source_activations.clone(),
                spent_buckets: *spent_buckets,
                reserved_nonmana_costs: facts.reserved_nonmana_costs.clone(),
                selected_cost_operands: facts.selected_cost_operands.clone(),
            }],
            Self::TemporaryEffectCreated { effect } => {
                vec![SemanticDeltaOperation::TemporaryEffectChanged {
                    effect: effect.id,
                    from: None,
                    to: Some(Box::new(effect.clone())),
                }]
            }
            Self::TemporaryEffectExpired { effect } => {
                vec![SemanticDeltaOperation::TemporaryEffectChanged {
                    effect: effect.id,
                    from: Some(Box::new(effect.clone())),
                    to: None,
                }]
            }
            Self::PerspectiveObservationOccurrence { lifecycle, .. } => {
                vec![SemanticDeltaOperation::PerspectiveLifecycle {
                    lifecycle: lifecycle.as_ref().clone(),
                }]
            }
            Self::DamageApplied {
                source,
                recipient,
                post_replacement_amount,
                damage_kind,
            } => vec![SemanticDeltaOperation::DamageApplied {
                source: source.clone().map(Box::new),
                recipient: *recipient,
                post_replacement_amount: *post_replacement_amount,
                damage_kind: *damage_kind,
            }],
            Self::StartingPlayerChosen {
                chooser,
                starting_player,
            } => vec![SemanticDeltaOperation::StartingPlayerChosen {
                chooser: *chooser,
                starting_player: *starting_player,
            }],
            Self::MulliganDeclared { player, mulligan } => {
                vec![SemanticDeltaOperation::MulliganDeclared {
                    player: *player,
                    mulligan: *mulligan,
                }]
            }
            Self::LibraryShuffled {
                player,
                stream,
                cursor_before,
                cursor_after,
                raw_words_consumed,
                top_to_bottom,
            } => vec![SemanticDeltaOperation::LibraryShuffled {
                player: *player,
                stream: *stream,
                cursor_before: *cursor_before,
                cursor_after: *cursor_after,
                raw_words_consumed: *raw_words_consumed,
                top_to_bottom: top_to_bottom.clone(),
            }],
        }
    }
}

/// Requires every auditable event to have its exact typed Delta operation in
/// rules order. Only private continuation/request advances and the associated
/// stack-order vector operation may appear without a standalone event.
pub fn validate_event_delta_parity(
    events: &[AuthoritativeRuleEvent],
    delta: &StateDelta,
) -> Result<(), EventDeltaError> {
    let (grouped_entry_operations, grouped_entry_events) =
        validate_basic_land_entry_group(events, delta)?;
    let expected = events
        .iter()
        .filter(|event| !grouped_entry_events.contains(&event.event_id))
        .flat_map(AuthoritativeRuleEvent::semantic_operations)
        .collect::<Vec<_>>();
    let mut expected_index = 0;
    for (operation_index, operation) in delta.operations.iter().enumerate() {
        if matches!(
            operation,
            SemanticDeltaOperation::ContinuationChanged { .. }
                | SemanticDeltaOperation::PendingRequestChanged { .. }
                | SemanticDeltaOperation::StackOrderChanged { .. }
        ) || grouped_entry_operations.contains(&operation_index)
        {
            continue;
        }
        if expected.get(expected_index) != Some(operation) {
            return Err(EventDeltaError::Mismatch);
        }
        expected_index += 1;
    }
    if expected_index != expected.len() {
        return Err(EventDeltaError::Mismatch);
    }
    let has_stack_event = events.iter().any(|event| {
        matches!(
            &event.event,
            AuthoritativeRuleEventKind::StackItemAdded { .. }
                | AuthoritativeRuleEventKind::StackItemRemoved { .. }
                | AuthoritativeRuleEventKind::TriggerPlaced { .. }
        )
    });
    if !has_stack_event
        && delta
            .operations
            .iter()
            .any(|operation| matches!(operation, SemanticDeltaOperation::StackOrderChanged { .. }))
    {
        return Err(EventDeltaError::Mismatch);
    }
    Ok(())
}

/// Basic-land entry is one rules event with a small set of state-index
/// operations: the new incarnation, its land-play entitlement receipt, and
/// its derived intrinsic ability authority/opaque aliases. These operations
/// remain individually typed in StateDelta; the ZoneTransition is their
/// causal event owner. This bounded bridge preserves the already accepted
/// M4.2 event grouping without weakening parity for other transitions.
fn validate_basic_land_entry_group(
    events: &[AuthoritativeRuleEvent],
    delta: &StateDelta,
) -> Result<
    (
        std::collections::BTreeSet<usize>,
        std::collections::BTreeSet<RuleEventId>,
    ),
    EventDeltaError,
> {
    use SemanticDeltaOperation as V3;

    let has_land_play_operation = delta
        .operations
        .iter()
        .any(|operation| matches!(operation, V3::LandPlayCountChanged { .. }));
    if !has_land_play_operation {
        return Ok((
            std::collections::BTreeSet::new(),
            std::collections::BTreeSet::new(),
        ));
    }
    let transitions = events
        .iter()
        .filter_map(|record| match &record.event {
            AuthoritativeRuleEventKind::ZoneTransition { transition } => {
                Some((record.event_id, transition.as_ref()))
            }
            _ => None,
        })
        .filter(|(_, transition)| {
            transition.from.zone == mtgml_model::ZoneKind::Hand
                && transition.to.zone == mtgml_model::ZoneKind::Battlefield
        })
        .collect::<Vec<_>>();
    if transitions.is_empty() {
        return Ok((
            std::collections::BTreeSet::new(),
            std::collections::BTreeSet::new(),
        ));
    }
    if transitions.len() != 1 {
        return Err(EventDeltaError::Mismatch);
    }
    let (entry_event_id, transition) = transitions[0];
    let mut grouped = std::collections::BTreeSet::new();
    let entry_indices = delta
        .operations
        .iter()
        .enumerate()
        .filter_map(|(index, operation)| match operation {
            V3::ObjectEntered {
                old_object: Some(old),
                new_object,
                from_zone,
                to_zone,
                tapped: false,
                face: 0,
            } if *old == transition.old_object
                && *new_object == transition.new_object
                && *from_zone == mtgml_model::ZoneKind::Hand
                && *to_zone == mtgml_model::ZoneKind::Battlefield =>
            {
                Some(index)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    if entry_indices.len() != 1
        || transition.new_snapshot.object != transition.new_object
        || transition.last_known.object != transition.old_object
        || transition.new_snapshot.tapped
    {
        return Err(EventDeltaError::Mismatch);
    }
    grouped.insert(entry_indices[0]);

    let land_count_indices = delta
        .operations
        .iter()
        .enumerate()
        .filter_map(|(index, operation)| match operation {
            V3::LandPlayCountChanged {
                player,
                from: 0,
                to: 1,
            } if *player == transition.new_snapshot.controller => Some(index),
            _ => None,
        })
        .collect::<Vec<_>>();
    if land_count_indices.len() != 1 {
        return Err(EventDeltaError::Mismatch);
    }
    grouped.insert(land_count_indices[0]);

    let added_authorities = delta
        .operations
        .iter()
        .enumerate()
        .filter_map(|(index, operation)| match operation {
            V3::AbilityAuthorityAdded {
                instance, source, ..
            } if *source == transition.new_object => Some((index, instance)),
            _ => None,
        })
        .collect::<Vec<_>>();
    if added_authorities.len() != 1 {
        return Err(EventDeltaError::Mismatch);
    }
    let (authority_index, instance) = added_authorities[0];
    grouped.insert(authority_index);

    let alias_entries = delta
        .operations
        .iter()
        .enumerate()
        .filter_map(|(index, operation)| match operation {
            V3::AbilityIdentityChanged {
                perspective,
                instance: alias_instance,
                from: None,
                to: Some(_),
            } if alias_instance == instance => Some((index, *perspective)),
            _ => None,
        })
        .collect::<Vec<_>>();
    let unique_perspectives = alias_entries
        .iter()
        .map(|(_, perspective)| *perspective)
        .collect::<std::collections::BTreeSet<_>>();
    if alias_entries.is_empty() || unique_perspectives.len() != alias_entries.len() {
        return Err(EventDeltaError::Mismatch);
    }
    for (index, _) in alias_entries {
        grouped.insert(index);
    }

    Ok((grouped, [entry_event_id].into_iter().collect()))
}

/// Applies the successor delta and checks event projections against the same
/// before/after state. Legacy event kinds keep their existing validator.
pub fn validate_event_delta_state(
    before: &EngineState,
    after: &EngineState,
    events: &[AuthoritativeRuleEvent],
    delta: &StateDelta,
) -> Result<(), EventDeltaError> {
    validate_event_delta_state_inner(before, after, events, delta, DeltaCheck::Apply)
}

/// Validates the events of a transition this crate just produced, whose
/// delta `StateDelta::between_structural_only(before, after, ..)` has just
/// built. That constructor validated both states and computed their digests,
/// so the delta is checked against the same states without re-applying it
/// (which would compute both digests again).
pub(crate) fn validate_events_for_built_delta_v3(
    before: &EngineState,
    after: &EngineState,
    events: &[AuthoritativeRuleEvent],
    delta: &StateDelta,
) -> Result<(), EventDeltaError> {
    validate_event_delta_state_inner(before, after, events, delta, DeltaCheck::BuiltFrom)
}

/// How the delta is checked against the before and after states.
enum DeltaCheck {
    /// Apply the delta with full state admission.
    Apply,
    /// The delta was just built from these states; compare, do not apply.
    BuiltFrom,
}

fn validate_event_delta_state_inner(
    before: &EngineState,
    after: &EngineState,
    events: &[AuthoritativeRuleEvent],
    delta: &StateDelta,
    check: DeltaCheck,
) -> Result<(), EventDeltaError> {
    match check {
        DeltaCheck::Apply => {
            let applied = delta.apply(before).map_err(|_| EventDeltaError::Mismatch)?;
            if &applied != after {
                return Err(EventDeltaError::Mismatch);
            }
        }
        DeltaCheck::BuiltFrom => {
            if delta.before_revision != before.revision
                || delta.after_revision != after.revision
                || delta.replacement != *after
            {
                return Err(EventDeltaError::Mismatch);
            }
        }
    }
    validate_rule_event_cursor(
        before.allocators.next_rule_event_id,
        after.allocators.next_rule_event_id,
        after.revision,
        events,
    )
    .map_err(|_| EventDeltaError::Mismatch)?;
    validate_event_delta_parity(events, delta)?;
    validate_observation_occurrence_lifecycle(before, after, events, delta)?;
    let before_stack_order = &before.zones.stack_order;
    let after_stack_order = &after.zones.stack_order;
    let stack_order_operations = delta
        .operations
        .iter()
        .filter_map(|operation| match operation {
            SemanticDeltaOperation::StackOrderChanged { from, to } => Some((from, to)),
            _ => None,
        })
        .collect::<Vec<_>>();
    if before_stack_order != after_stack_order {
        if stack_order_operations.len() != 1
            || stack_order_operations[0].0 != before_stack_order
            || stack_order_operations[0].1 != after_stack_order
            || !events.iter().any(|event| {
                matches!(
                    &event.event,
                    AuthoritativeRuleEventKind::StackItemAdded { .. }
                        | AuthoritativeRuleEventKind::StackItemRemoved { .. }
                        | AuthoritativeRuleEventKind::TriggerPlaced { .. }
                )
            })
        {
            return Err(EventDeltaError::Mismatch);
        }
    } else if !stack_order_operations.is_empty() {
        return Err(EventDeltaError::Mismatch);
    }
    for operation in &delta.operations {
        validate_delta_operation_projection_v3(before, after, operation)?;
    }
    for event in events {
        validate_event_projection_v3(before, after, &event.event)?;
    }
    validate_zone_transition_chain(before, after, events)?;
    validate_damage_state_projection_v3(before, after, events)?;
    Ok(())
}

fn validate_observation_occurrence_lifecycle(
    before: &EngineState,
    after: &EngineState,
    events: &[AuthoritativeRuleEvent],
    delta: &StateDelta,
) -> Result<(), EventDeltaError> {
    let mut projected = after.clone();
    projected.knowledge = before.knowledge.clone();
    projected.perspective_identities = before.perspective_identities.clone();
    for (index, event) in events.iter().enumerate() {
        let (lifecycle, source_event_id) = match &event.event {
            AuthoritativeRuleEventKind::PerspectiveObservationOccurrence {
                lifecycle,
                source_event_id,
            } => (Some(lifecycle.as_ref()), Some(*source_event_id)),
            _ => (None, None),
        };
        let Some(lifecycle) = lifecycle else {
            continue;
        };
        if let Some(source_event_id) = source_event_id {
            let Some(source_index) = events
                .iter()
                .position(|candidate| candidate.event_id == source_event_id)
            else {
                return Err(EventDeltaError::Mismatch);
            };
            if source_index >= index
                || !is_projectable_public_source_event(&events[source_index].event)
            {
                return Err(EventDeltaError::Mismatch);
            }
        }
        if projected
            .knowledge
            .players
            .get(&lifecycle.perspective)
            .is_none_or(|knowledge| knowledge.next_visible_sequence != lifecycle.sequence)
            || mtgml_state::apply_perspective_lifecycle(&mut projected, lifecycle).is_err()
        {
            return Err(EventDeltaError::Mismatch);
        }
    }
    apply_delta_identity_changes_v3(&mut projected, delta)?;
    if projected.knowledge != after.knowledge
        || projected.perspective_identities != after.perspective_identities
    {
        return Err(EventDeltaError::Mismatch);
    }
    Ok(())
}

fn apply_delta_identity_changes_v3(
    projected: &mut mtgml_state::EngineState,
    delta: &StateDelta,
) -> Result<(), EventDeltaError> {
    use SemanticDeltaOperation as V3;

    for operation in &delta.operations {
        match operation {
            V3::AbilityIdentityChanged {
                perspective,
                instance,
                from,
                to,
            } => {
                let identity = projected
                    .perspective_identities
                    .players
                    .get_mut(perspective)
                    .ok_or(EventDeltaError::Mismatch)?;
                if identity.ability_to_opaque.get(instance).copied() != *from {
                    return Err(EventDeltaError::Mismatch);
                }
                if let Some(opaque) = from {
                    identity.ability_to_opaque.remove(instance);
                    identity.opaque_to_ability.remove(opaque);
                    identity.retired_ability_ids.insert(*opaque);
                }
                if let Some(opaque) = to {
                    if *opaque != identity.next_opaque_ability_id
                        || identity
                            .opaque_to_ability
                            .insert(*opaque, *instance)
                            .is_some()
                        || identity
                            .ability_to_opaque
                            .insert(*instance, *opaque)
                            .is_some()
                    {
                        return Err(EventDeltaError::Mismatch);
                    }
                    identity.next_opaque_ability_id = mtgml_model::OpaqueAbilityId(
                        opaque.0.checked_add(1).ok_or(EventDeltaError::Mismatch)?,
                    );
                }
            }
            V3::PendingRequestChanged {
                to: Some(request), ..
            } => {
                let identity = projected
                    .perspective_identities
                    .players
                    .get_mut(&request.actor)
                    .ok_or(EventDeltaError::Mismatch)?;
                if identity.next_player_decision_id != request.player_decision_id {
                    return Err(EventDeltaError::Mismatch);
                }
                identity.next_player_decision_id = mtgml_model::PlayerDecisionIdV1(
                    request
                        .player_decision_id
                        .0
                        .checked_add(1)
                        .ok_or(EventDeltaError::Mismatch)?,
                );
            }
            _ => {}
        }
    }
    Ok(())
}

fn is_projectable_public_source_event(event: &AuthoritativeRuleEventKind) -> bool {
    matches!(
        event,
        AuthoritativeRuleEventKind::StackItemAdded { .. }
            | AuthoritativeRuleEventKind::StackItemRemoved { .. }
            | AuthoritativeRuleEventKind::TriggerPlaced { .. }
            | AuthoritativeRuleEventKind::CounterChanged { .. }
            | AuthoritativeRuleEventKind::ManaPoolChanged { .. }
            | AuthoritativeRuleEventKind::TemporaryEffectCreated { .. }
            | AuthoritativeRuleEventKind::TemporaryEffectExpired { .. }
            | AuthoritativeRuleEventKind::ZoneTransition { .. }
            | AuthoritativeRuleEventKind::ObjectTapped { .. }
            | AuthoritativeRuleEventKind::StartingPlayerChosen { .. }
            | AuthoritativeRuleEventKind::MulliganDeclared { .. }
    )
}

fn validate_damage_state_projection_v3(
    before: &EngineState,
    after: &EngineState,
    events: &[AuthoritativeRuleEvent],
) -> Result<(), EventDeltaError> {
    let mut player_damage = std::collections::BTreeMap::<PlayerId, u64>::new();
    let mut object_damage = std::collections::BTreeMap::<GameObjectId, u64>::new();
    for event in events {
        let AuthoritativeRuleEventKind::DamageApplied {
            recipient,
            post_replacement_amount,
            ..
        } = &event.event
        else {
            continue;
        };
        let total = match recipient {
            DamageRecipient::Player(player) => player_damage.entry(*player).or_default(),
            DamageRecipient::Object(object) => object_damage.entry(*object).or_default(),
        };
        let Some(next) = total.checked_add(u64::from(*post_replacement_amount)) else {
            return Err(EventDeltaError::Mismatch);
        };
        *total = next;
    }
    for (player, amount) in player_damage {
        let Some(before_player) = before.core.players.get(&player) else {
            return Err(EventDeltaError::Mismatch);
        };
        let Some(after_player) = after.core.players.get(&player) else {
            return Err(EventDeltaError::Mismatch);
        };
        let actual_loss = i128::from(before_player.life) - i128::from(after_player.life);
        if actual_loss != i128::from(amount) {
            return Err(EventDeltaError::Mismatch);
        }
    }
    // No state component records marked damage yet: damage to an object
    // cannot be projected and fails closed.
    if !object_damage.is_empty() {
        return Err(EventDeltaError::Mismatch);
    }
    Ok(())
}

fn validate_delta_operation_projection_v3(
    before: &EngineState,
    after: &EngineState,
    operation: &SemanticDeltaOperation,
) -> Result<(), EventDeltaError> {
    let valid = match operation {
        // Operations whose state change the coverage check owns.
        SemanticDeltaOperation::ZoneTransition { .. }
        | SemanticDeltaOperation::ObjectCeasedToExist { .. }
        | SemanticDeltaOperation::LifeChanged { .. }
        | SemanticDeltaOperation::CombatDamageDealt { .. }
        | SemanticDeltaOperation::CombatDamageStepCompleted
        | SemanticDeltaOperation::MarkedDamageChanged { .. }
        | SemanticDeltaOperation::ObjectTapped { .. }
        | SemanticDeltaOperation::DecisionCreated { .. }
        | SemanticDeltaOperation::DecisionCleared { .. }
        | SemanticDeltaOperation::SbaGraveyardOrderChosen { .. }
        | SemanticDeltaOperation::StateBasedActionsApplied { .. }
        | SemanticDeltaOperation::PriorityChanged { .. }
        | SemanticDeltaOperation::RandomValueSampled { .. }
        | SemanticDeltaOperation::PublicOutcome { .. }
        | SemanticDeltaOperation::TurnPositionChanged { .. }
        | SemanticDeltaOperation::AttackersDeclared { .. }
        | SemanticDeltaOperation::BlockersDeclared { .. }
        | SemanticDeltaOperation::CombatEnded
        | SemanticDeltaOperation::EmptyCombatStepsSkipped
        | SemanticDeltaOperation::UntapCompleted { .. }
        | SemanticDeltaOperation::ActivePlayerChanged { .. }
        | SemanticDeltaOperation::TurnNumberChanged { .. }
        | SemanticDeltaOperation::PerspectiveLifecycle { .. }
        | SemanticDeltaOperation::LandPlayCountChanged { .. }
        | SemanticDeltaOperation::AbilityIdentityChanged { .. }
        | SemanticDeltaOperation::AttachmentChanged { .. }
        | SemanticDeltaOperation::ObjectFaceChanged { .. }
        | SemanticDeltaOperation::ObjectEntered { .. }
        | SemanticDeltaOperation::AbilityAuthorityAdded { .. }
        | SemanticDeltaOperation::AbilityAuthorityRemoved { .. }
        | SemanticDeltaOperation::DamageApplied { .. }
        | SemanticDeltaOperation::StartingPlayerChosen { .. }
        | SemanticDeltaOperation::MulliganDeclared { .. }
        | SemanticDeltaOperation::LibraryShuffled { .. } => true,
        SemanticDeltaOperation::StackOrderChanged { from, to } => {
            &before.zones.stack_order == from && &after.zones.stack_order == to
        }
        SemanticDeltaOperation::StackItemCreated {
            stack_object,
            payload,
        } => {
            !before.zones.stack_records.contains_key(stack_object)
                && after
                    .zones
                    .stack_records
                    .get(stack_object)
                    .and_then(|record| record.payload.as_ref())
                    == Some(payload.as_ref())
        }
        SemanticDeltaOperation::StackItemEnded {
            stack_object,
            payload,
            ..
        } => {
            before
                .zones
                .stack_records
                .get(stack_object)
                .and_then(|record| record.payload.as_ref())
                == Some(payload.as_ref())
                && !after.zones.stack_records.contains_key(stack_object)
        }
        SemanticDeltaOperation::SpellCast {
            stack_object,
            spell_object,
            card_definition,
            face_key,
            semantic_profile_id,
            cost_facts,
            ..
        } => !before.zones.stack_records.contains_key(stack_object)
            && after
                .zones
                .stack_records
                .get(stack_object)
                .is_some_and(|record| {
                    matches!(record.payload.as_ref(), Some(mtgml_state::StackItemPayload::Spell {
                    stack_card_object,
                    card_definition_id,
                    face_key: actual_face,
                    semantic_profile_id: actual_profile,
                    cost_facts: actual_cost,
                    ..
                }) if stack_card_object == spell_object
                    && card_definition_id == card_definition
                    && actual_face == face_key
                    && actual_profile == semantic_profile_id
                    && actual_cost == cost_facts)
                }),
        SemanticDeltaOperation::AbilityActivated {
            stack_object,
            source,
            targets,
            cost_facts,
            once_per_turn_use_committed,
        } => {
            let pair = (source.source.snapshot.object, source.ability_key.0);
            let history_before = before
                .card_rules
                .turn_history
                .once_ability_used
                .contains(&pair);
            let history_after = after
                .card_rules
                .turn_history
                .once_ability_used
                .contains(&pair);
            let receipt_matches = if *once_per_turn_use_committed {
                !history_before && history_after
            } else {
                history_before == history_after
            };
            !before
                .zones
                .stack_records
                .contains_key(stack_object)
                && receipt_matches
                && after
                    .zones
                    .stack_records
                    .get(stack_object)
                    .is_some_and(|record| {
                matches!(record.payload.as_ref(), Some(mtgml_state::StackItemPayload::ActivatedAbility {
                    source_context,
                    targets: actual_targets,
                    cost_facts: actual_cost,
                    ..
                }) if source_context == source.as_ref()
                    && actual_targets == targets
                    && actual_cost == cost_facts)
                    })
        }
        SemanticDeltaOperation::TargetDeclared {
            source_stack_item,
            targets,
        } => after
            .zones
            .stack_records
            .get(source_stack_item)
            .is_some_and(|record| match record.payload.as_ref() {
                Some(mtgml_state::StackItemPayload::Spell {
                    targets: actual_targets,
                    ..
                })
                | Some(mtgml_state::StackItemPayload::ActivatedAbility {
                    targets: actual_targets,
                    ..
                })
                | Some(mtgml_state::StackItemPayload::TriggeredAbility {
                    targets: actual_targets,
                    ..
                }) => actual_targets == targets,
                None => false,
            }),
        SemanticDeltaOperation::CounterChanged {
            object,
            kind,
            from,
            to,
            ..
        } => {
            let before_count = before
                .card_rules
                .counters
                .counters
                .get(object)
                .and_then(|counters| counters.get(kind))
                .copied()
                .unwrap_or(0);
            let after_count = after
                .card_rules
                .counters
                .counters
                .get(object)
                .and_then(|counters| counters.get(kind))
                .copied()
                .unwrap_or(0);
            before_count == *from && after_count == *to && from != to
        }
        SemanticDeltaOperation::TriggerCreated { trigger } => {
            !before.execution.waiting_triggers.contains_key(&trigger.id)
                && after.execution.waiting_triggers.get(&trigger.id) == Some(trigger.as_ref())
        }
        SemanticDeltaOperation::TriggerPlaced {
            trigger,
            stack_object,
            payload,
        } => {
            before.execution.waiting_triggers.contains_key(trigger)
                && !after.execution.waiting_triggers.contains_key(trigger)
                && after
                    .zones
                    .stack_records
                    .get(stack_object)
                    .and_then(|record| record.payload.as_ref())
                    == Some(payload.as_ref())
        }
        SemanticDeltaOperation::ManaPoolChanged {
            player, from, to, ..
        } => {
            before.card_rules.mana.pools.get(player) == Some(from)
                && after.card_rules.mana.pools.get(player) == Some(to)
        }
        SemanticDeltaOperation::AtomicCostCommitted {
            actor,
            action,
            mana_cost,
            source_activations,
            spent_buckets,
            reserved_nonmana_costs,
            selected_cost_operands,
        } => validate_cost_commit_projection(
            before,
            after,
            *actor,
            *action,
            &ActionCostFacts {
                mana_cost: *mana_cost,
                reserved_nonmana_costs: reserved_nonmana_costs.clone(),
                selected_cost_operands: selected_cost_operands.clone(),
            },
            source_activations,
            spent_buckets,
        ),
        SemanticDeltaOperation::ContinuationChanged {
            continuation,
            from,
            to,
        } => {
            before
                .execution
                .continuations
                .get(continuation)
                .map(|record| &record.payload)
                == from.as_deref()
                && after
                    .execution
                    .continuations
                    .get(continuation)
                    .map(|record| &record.payload)
                    == to.as_deref()
        }
        SemanticDeltaOperation::PendingRequestChanged { from, to } => {
            before.execution.pending_decision.as_ref() == from.as_deref()
                && after.execution.pending_decision.as_ref() == to.as_deref()
        }
        SemanticDeltaOperation::TemporaryEffectChanged { effect, from, to } => {
            before.execution.effects.get(effect) == from.as_deref()
                && after.execution.effects.get(effect) == to.as_deref()
        }
    };
    if valid {
        Ok(())
    } else {
        Err(EventDeltaError::Mismatch)
    }
}

fn validate_event_projection_v3(
    before: &EngineState,
    after: &EngineState,
    event: &AuthoritativeRuleEventKind,
) -> Result<(), EventDeltaError> {
    let valid = match event {
        // Checked as one chain by `validate_zone_transition_chain`.
        AuthoritativeRuleEventKind::ZoneTransition { .. }
        | AuthoritativeRuleEventKind::LibraryShuffled { .. } => true,
        AuthoritativeRuleEventKind::StartingPlayerChosen {
            chooser,
            starting_player,
        } => {
            game_start_of(before)
                .is_some_and(|start| start.chooser == *chooser && start.starting_player.is_none())
                && game_start_of(after)
                    .is_some_and(|start| start.starting_player == Some(*starting_player))
        }
        AuthoritativeRuleEventKind::MulliganDeclared { player, .. } => game_start_of(before)
            .is_some_and(|start| {
                start.stage == mtgml_state::GameStartStage::Declaring { player: *player }
            }),
        AuthoritativeRuleEventKind::ObjectCeasedToExist { object } => {
            before.zones.objects.contains_key(object) && !after.zones.objects.contains_key(object)
        }
        AuthoritativeRuleEventKind::LifeChanged { player, from, to } => {
            from != to
                && before
                    .core
                    .players
                    .get(player)
                    .is_some_and(|state| state.life == *from)
                && after
                    .core
                    .players
                    .get(player)
                    .is_some_and(|state| state.life == *to)
        }
        // No state component records marked damage yet: the event cannot be
        // projected and fails closed.
        AuthoritativeRuleEventKind::MarkedDamageChanged { .. } => false,
        AuthoritativeRuleEventKind::ObjectTapped { object, from, to } => {
            from != to
                && before
                    .zones
                    .objects
                    .get(object)
                    .is_some_and(|state| state.tapped == *from)
                && after
                    .zones
                    .objects
                    .get(object)
                    .is_some_and(|state| state.tapped == *to)
        }
        AuthoritativeRuleEventKind::DecisionCreated { decision } => {
            before
                .execution
                .pending_decision
                .as_ref()
                .is_none_or(|request| request.decision_id != *decision)
                && after
                    .execution
                    .pending_decision
                    .as_ref()
                    .is_some_and(|request| request.decision_id == *decision)
        }
        AuthoritativeRuleEventKind::DecisionCleared { decision } => {
            before
                .execution
                .pending_decision
                .as_ref()
                .is_some_and(|request| request.decision_id == *decision)
                && after
                    .execution
                    .pending_decision
                    .as_ref()
                    .is_none_or(|request| request.decision_id != *decision)
        }
        AuthoritativeRuleEventKind::PriorityChanged { from, to } => {
            from != to && before.core.priority == *from && after.core.priority == *to
        }
        AuthoritativeRuleEventKind::TurnPositionChanged { from, to } => {
            from != to && before.core.position == *from && after.core.position == *to
        }
        AuthoritativeRuleEventKind::RandomValueSampled {
            stream,
            bound,
            value,
            raw_words_consumed,
            cursor_before,
            cursor_after,
        } => {
            *bound > 0
                && *value < *bound
                && cursor_before.checked_add(*raw_words_consumed) == Some(*cursor_after)
                && before
                    .random
                    .streams
                    .get(stream)
                    .is_some_and(|cursor| cursor.next_raw_u64 == *cursor_before)
                && after
                    .random
                    .streams
                    .get(stream)
                    .is_some_and(|cursor| cursor.next_raw_u64 == *cursor_after)
        }
        AuthoritativeRuleEventKind::PublicOutcome { .. } => true,
        AuthoritativeRuleEventKind::SbaGraveyardOrderChosen {
            continuation,
            owner,
            top_to_bottom,
        } => after
            .execution
            .continuations
            .get(continuation)
            .is_some_and(|record| match &record.payload {
                mtgml_state::ContinuationPayload::MagicSbaGraveyardOrderV1 {
                    completed_owner_orders,
                    ..
                } => completed_owner_orders
                    .iter()
                    .any(|order| order.owner == *owner && order.top_to_bottom == *top_to_bottom),
                _ => false,
            }),
        AuthoritativeRuleEventKind::StateBasedActionsApplied { actions } => {
            actions.iter().all(|action| match action {
                mtgml_state::SbaSelectedActionV1::PlayerLoses { player } => after
                    .core
                    .players
                    .get(player)
                    .is_some_and(|state| state.has_lost),
                mtgml_state::SbaSelectedActionV1::ObjectToOwnerGraveyard { object, .. } => {
                    !after.zones.objects.contains_key(object)
                }
            })
        }
        AuthoritativeRuleEventKind::CombatDamageStepCompleted => after
            .combat
            .as_ref()
            .is_some_and(|combat| combat.damage_step_completed),
        AuthoritativeRuleEventKind::AttackersDeclared {
            defending_player,
            attackers,
        } => after.combat.as_ref().is_some_and(|combat| {
            combat.defending_player == *defending_player && combat.attackers == *attackers
        }),
        AuthoritativeRuleEventKind::CombatEnded => {
            before.combat.is_some() && after.combat.is_none()
        }
        AuthoritativeRuleEventKind::EmptyCombatStepsSkipped => {
            before.core.position != after.core.position
        }
        AuthoritativeRuleEventKind::UntapCompleted { affected_objects } => {
            affected_objects.iter().all(|object| {
                after
                    .zones
                    .objects
                    .get(object)
                    .is_some_and(|state| !state.tapped)
            })
        }
        AuthoritativeRuleEventKind::ActivePlayerChanged { from, to } => {
            from != to && before.core.active_player == *from && after.core.active_player == *to
        }
        AuthoritativeRuleEventKind::TurnNumberChanged { from, to } => {
            from != to && before.core.turn_number == *from && after.core.turn_number == *to
        }
        // Combat assignment/blocked-state state projection remains closed
        // until its exact legal relation is characterized and accepted.
        AuthoritativeRuleEventKind::CombatDamageDealt { .. }
        | AuthoritativeRuleEventKind::BlockersDeclared { .. } => false,
        AuthoritativeRuleEventKind::PerspectiveObservationOccurrence { .. } => true,
        AuthoritativeRuleEventKind::DamageApplied { .. } => true,
        AuthoritativeRuleEventKind::StackItemAdded {
            stack_object,
            payload,
        } => {
            !before.zones.stack_records.contains_key(stack_object)
                && after
                    .zones
                    .stack_records
                    .get(stack_object)
                    .is_some_and(|record| record.payload.as_ref() == Some(payload))
                && after.zones.stack_order.contains(stack_object)
        }
        AuthoritativeRuleEventKind::StackItemRemoved {
            stack_object,
            payload,
            ..
        } => {
            before
                .zones
                .stack_records
                .get(stack_object)
                .is_some_and(|record| record.payload.as_ref() == Some(payload))
                && !after.zones.stack_records.contains_key(stack_object)
                && !after.zones.stack_order.contains(stack_object)
        }
        AuthoritativeRuleEventKind::SpellCast {
            stack_object,
            spell_object,
            card_definition,
            face_key,
            semantic_profile_id,
            cost_facts,
            ..
        } => {
            !before.zones.stack_records.contains_key(stack_object)
                && after
                    .zones
                    .stack_records
                    .get(stack_object)
                    .is_some_and(|record| {
                        matches!(
                            record.payload.as_ref(),
                            Some(StackItemPayload::Spell {
                                stack_card_object,
                                card_definition_id,
                                face_key: actual_face,
                                semantic_profile_id: actual_profile,
                                cost_facts: actual_costs,
                                ..
                            }) if stack_card_object == spell_object
                                && card_definition_id == card_definition
                                && actual_face == face_key
                                && actual_profile == semantic_profile_id
                                && actual_costs == cost_facts
                        )
                    })
        }
        AuthoritativeRuleEventKind::AbilityActivated {
            stack_object,
            source,
            targets,
            cost_facts,
            once_per_turn_use_committed,
        } => {
            let pair = (source.source.snapshot.object, source.ability_key.0);
            let history_before = before
                .card_rules
                .turn_history
                .once_ability_used
                .contains(&pair);
            let history_after = after
                .card_rules
                .turn_history
                .once_ability_used
                .contains(&pair);
            let receipt_matches = if *once_per_turn_use_committed {
                !history_before && history_after
            } else {
                history_before == history_after
            };
            !before.zones.stack_records.contains_key(stack_object)
                && receipt_matches
                && after
                    .zones
                    .stack_records
                    .get(stack_object)
                    .is_some_and(|record| {
                        matches!(
                            record.payload.as_ref(),
                            Some(StackItemPayload::ActivatedAbility {
                                source_context,
                                targets: actual_targets,
                                cost_facts: actual_costs,
                                ..
                            }) if source_context == source
                                && actual_targets == targets
                                && actual_costs == cost_facts
                        )
                    })
        }
        AuthoritativeRuleEventKind::TargetDeclared {
            source_stack_item,
            targets,
        } => after
            .zones
            .stack_records
            .get(source_stack_item)
            .is_some_and(|record| match record.payload.as_ref() {
                Some(StackItemPayload::Spell {
                    targets: actual_targets,
                    ..
                })
                | Some(StackItemPayload::ActivatedAbility {
                    targets: actual_targets,
                    ..
                })
                | Some(StackItemPayload::TriggeredAbility {
                    targets: actual_targets,
                    ..
                }) => actual_targets == targets,
                None => false,
            }),
        AuthoritativeRuleEventKind::CounterChanged {
            object,
            kind,
            before: before_count,
            after: after_count,
        } => {
            let old = before
                .card_rules
                .counters
                .counters
                .get(object)
                .and_then(|counters| counters.get(kind))
                .copied()
                .unwrap_or(0);
            let new = after
                .card_rules
                .counters
                .counters
                .get(object)
                .and_then(|counters| counters.get(kind))
                .copied()
                .unwrap_or(0);
            old == *before_count && new == *after_count && before_count != after_count
        }
        AuthoritativeRuleEventKind::TriggerDetected { trigger } => {
            let pending = after
                .execution
                .waiting_triggers
                .get(&trigger.id)
                .is_some_and(|record| record == trigger);
            let placed = after.zones.stack_records.values().any(|record| {
                matches!(record.payload.as_ref(), Some(StackItemPayload::TriggeredAbility {
                        originating_trigger,
                        source_context,
                        captured_trigger_context,
                        ..
                    }) if *originating_trigger == trigger.id
                        && source_context == &trigger.source_context
                        && captured_trigger_context.as_ref() == &trigger.trigger_context)
            });
            !before.execution.waiting_triggers.contains_key(&trigger.id) && (pending || placed)
        }
        AuthoritativeRuleEventKind::TriggerPlaced {
            trigger,
            stack_object,
            payload,
        } => {
            before.execution.waiting_triggers.contains_key(trigger)
                && !after.execution.waiting_triggers.contains_key(trigger)
                && after
                    .zones
                    .stack_records
                    .get(stack_object)
                    .is_some_and(|record| record.payload.as_ref() == Some(payload))
        }
        AuthoritativeRuleEventKind::ManaPoolChanged {
            player,
            before: old_pool,
            after: new_pool,
            ..
        } => {
            before.card_rules.mana.pools.get(player) == Some(old_pool)
                && after.card_rules.mana.pools.get(player) == Some(new_pool)
        }
        AuthoritativeRuleEventKind::CostCommitted {
            actor,
            action,
            facts,
            source_activations,
            spent_buckets,
        } => validate_cost_commit_projection(
            before,
            after,
            *actor,
            *action,
            facts,
            source_activations,
            spent_buckets,
        ),
        AuthoritativeRuleEventKind::TemporaryEffectCreated { effect } => {
            !before.execution.effects.contains_key(&effect.id)
                && after.execution.effects.get(&effect.id) == Some(effect)
        }
        AuthoritativeRuleEventKind::TemporaryEffectExpired { effect } => {
            before.execution.effects.get(&effect.id) == Some(effect)
                && !after.execution.effects.contains_key(&effect.id)
        }
    };
    if valid {
        Ok(())
    } else {
        Err(EventDeltaError::Mismatch)
    }
}

fn game_start_of(state: &EngineState) -> Option<&mtgml_state::GameStartContinuation> {
    state
        .execution
        .continuations
        .values()
        .find_map(|record| match &record.payload {
            mtgml_state::ContinuationPayload::GameStart(start) => Some(start),
            _ => None,
        })
}

/// Zone transitions compose (CR 400.7: every move makes a new object). They
/// are replayed in event order over a copy of the before zones: each move must
/// start from where its card lies at that moment, and the replay must end in
/// the after zones for every object it created and every ordered zone it
/// changed.
fn validate_zone_transition_chain(
    before: &EngineState,
    after: &EngineState,
    events: &[AuthoritativeRuleEvent],
) -> Result<(), EventDeltaError> {
    let mismatch = || EventDeltaError::Mismatch;
    let mut zones = before.zones.clone();
    let mut cursors = before.random.streams.clone();
    let mut shuffled_streams = std::collections::BTreeSet::new();
    let mut left = std::collections::BTreeSet::new();
    let mut created = std::collections::BTreeSet::new();
    let mut ordered = std::collections::BTreeSet::new();
    for event in events {
        if let AuthoritativeRuleEventKind::LibraryShuffled {
            player,
            stream,
            cursor_before,
            cursor_after,
            raw_words_consumed,
            top_to_bottom,
        } = &event.event
        {
            if *stream
                != RandomStreamKeyV1::player_scoped(
                    mtgml_random::RandomStreamKindV1::LibraryShuffle,
                    player.0,
                )
                || cursors.get(stream).map(|cursor| cursor.next_raw_u64) != Some(*cursor_before)
            {
                return Err(mismatch());
            }
            let key = library_key(*player);
            let mut order = zones
                .ordered_zones
                .get(&key)
                .cloned()
                .ok_or_else(mismatch)?;
            let (consumed, cursor) = mtgml_random::sampling::shuffle(
                &mut order,
                &before.random.root_seed,
                stream,
                &mtgml_random::RandomStreamCursorV1 {
                    next_raw_u64: *cursor_before,
                },
            )
            .map_err(|_| mismatch())?;
            if &order != top_to_bottom
                || consumed != *raw_words_consumed
                || cursor.next_raw_u64 != *cursor_after
            {
                return Err(mismatch());
            }
            zones.ordered_zones.insert(key.clone(), order);
            rewitness_ordered_zone(&mut zones, &key)?;
            ordered.insert(key);
            cursors.insert(*stream, cursor);
            shuffled_streams.insert(*stream);
            continue;
        }
        let AuthoritativeRuleEventKind::ZoneTransition { transition } = &event.event else {
            continue;
        };
        if transition.from != transition.last_known.location
            || transition.to != transition.new_snapshot.location
            || transition.last_known.object != transition.old_object
            || transition.new_snapshot.object != transition.new_object
            || !object_snapshot_matches(&zones, &transition.last_known)
            || zones.objects.contains_key(&transition.new_object)
        {
            return Err(mismatch());
        }
        zones.objects.remove(&transition.old_object);
        let from = zones
            .locations
            .remove(&transition.old_object)
            .ok_or_else(mismatch)?;
        if let Some(members) = zones.ordered_zones.get_mut(&from.key()) {
            members.retain(|member| *member != transition.old_object);
            ordered.insert(from.key());
        }
        let snapshot = &transition.new_snapshot;
        zones.objects.insert(
            transition.new_object,
            mtgml_state::GameObject {
                id: transition.new_object,
                physical_card: snapshot.physical_card,
                card_definition: snapshot.card_definition,
                owner: snapshot.owner,
                controller: snapshot.controller,
                tapped: snapshot.tapped,
                face_down: snapshot.face_down,
            },
        );
        if let mtgml_state::ZonePosition::Top { offset } = transition.to.position {
            let members = zones.ordered_zones.entry(transition.to.key()).or_default();
            let index = usize::try_from(offset).map_err(|_| mismatch())?;
            if index > members.len() {
                return Err(mismatch());
            }
            members.insert(index, transition.new_object);
            ordered.insert(transition.to.key());
        }
        zones
            .locations
            .insert(transition.new_object, transition.to.clone());
        for key in [from.key(), transition.to.key()] {
            rewitness_ordered_zone(&mut zones, &key)?;
        }
        if !created.remove(&transition.old_object) {
            left.insert(transition.old_object);
        }
        created.insert(transition.new_object);
    }
    let objects_match = |id: &GameObjectId| {
        after.zones.objects.get(id) == zones.objects.get(id)
            && after.zones.locations.get(id) == zones.locations.get(id)
    };
    if shuffled_streams
        .iter()
        .any(|stream| after.random.streams.get(stream) != cursors.get(stream))
        || left.iter().any(|id| after.zones.objects.contains_key(id))
        || !created.iter().all(objects_match)
        || ordered.iter().any(|key| {
            let members = zones.ordered_zones.get(key);
            after.zones.ordered_zones.get(key) != members
                || !members.into_iter().flatten().all(objects_match)
        })
    {
        return Err(mismatch());
    }
    Ok(())
}

/// The ordered zone of `player`'s face-down library.
fn library_key(player: PlayerId) -> mtgml_state::ZoneKey {
    mtgml_state::ZoneLocation {
        zone: mtgml_model::ZoneKind::Library,
        player: Some(player),
        position: mtgml_state::ZonePosition::Top { offset: 0 },
        visibility: mtgml_state::VisibilityPartition::FaceDown,
        partition: None,
    }
    .key()
}

/// Every member of an ordered zone sits at `Top { offset }` equal to its
/// index; an emptied zone has no entry.
fn rewitness_ordered_zone(
    zones: &mut mtgml_state::ZoneState,
    key: &mtgml_state::ZoneKey,
) -> Result<(), EventDeltaError> {
    let Some(members) = zones.ordered_zones.get(key).cloned() else {
        return Ok(());
    };
    if members.is_empty() {
        zones.ordered_zones.remove(key);
        return Ok(());
    }
    for (index, member) in members.iter().enumerate() {
        let location = zones
            .locations
            .get_mut(member)
            .ok_or(EventDeltaError::Mismatch)?;
        location.position = mtgml_state::ZonePosition::Top {
            offset: u32::try_from(index).map_err(|_| EventDeltaError::Mismatch)?,
        };
    }
    Ok(())
}

fn object_snapshot_matches(
    zones: &mtgml_state::ZoneState,
    snapshot: &mtgml_state::ObjectSnapshot,
) -> bool {
    zones.objects.get(&snapshot.object).is_some_and(|object| {
        object.physical_card == snapshot.physical_card
            && object.card_definition == snapshot.card_definition
            && object.owner == snapshot.owner
            && object.controller == snapshot.controller
            && object.tapped == snapshot.tapped
            && object.face_down == snapshot.face_down
    }) && zones.locations.get(&snapshot.object) == Some(&snapshot.location)
}

fn validate_cost_commit_projection(
    before: &EngineState,
    after: &EngineState,
    actor: PlayerId,
    action: CostCommitActionV1,
    facts: &ActionCostFacts,
    source_activations: &[ManaSourceActivation],
    spent_buckets: &[u32; 12],
) -> bool {
    let old_pool = match before.card_rules.mana.pools.get(&actor) {
        Some(pool) => *pool,
        None => return false,
    };
    let action_projects = match action {
        CostCommitActionV1::Cast {
            stack_object,
            spell_object,
        } => after
            .zones
            .stack_records
            .get(&stack_object)
            .is_some_and(|record| {
                matches!(record.payload.as_ref(), Some(StackItemPayload::Spell {
                    stack_card_object,
                    ..
                }) if *stack_card_object == spell_object)
            }),
        CostCommitActionV1::NonManaActivation {
            source_object,
            source_ability,
        } => after.zones.stack_records.values().any(|record| {
            matches!(record.payload.as_ref(), Some(StackItemPayload::ActivatedAbility {
                    source_context,
                    ..
                }) if source_context.source.snapshot.object == source_object
                    && source_context.ability_instance_id == source_ability)
        }),
        CostCommitActionV1::StackResolution { stack_object } => {
            before.zones.stack_records.contains_key(&stack_object)
                && !after.zones.stack_records.contains_key(&stack_object)
                && !after.zones.stack_order.contains(&stack_object)
        }
    };
    if !action_projects {
        return false;
    }
    let mut pool = old_pool;

    let staged = match action {
        CostCommitActionV1::Cast { spell_object, .. } => before
            .execution
            .continuations
            .values()
            .find_map(|record| match &record.payload {
                mtgml_state::ContinuationPayload::Cast(continuation)
                    if continuation.spell_object == spell_object =>
                {
                    Some((
                        &continuation.action_cost_facts,
                        continuation.mana_payment_staging.as_ref(),
                    ))
                }
                _ => None,
            }),
        CostCommitActionV1::NonManaActivation {
            source_object,
            source_ability,
        } => before
            .execution
            .continuations
            .values()
            .find_map(|record| match &record.payload {
                mtgml_state::ContinuationPayload::NonManaActivation(continuation)
                    if continuation.source_object == source_object
                        && continuation.source_ability_instance == source_ability =>
                {
                    Some((
                        &continuation.action_cost_facts,
                        continuation.mana_payment_staging.as_ref(),
                    ))
                }
                _ => None,
            }),
        CostCommitActionV1::StackResolution { stack_object } => before
            .execution
            .continuations
            .values()
            .find_map(|record| match &record.payload {
                mtgml_state::ContinuationPayload::StackResolution(continuation)
                    if continuation.resolving_stack_object == stack_object =>
                {
                    continuation
                        .action_cost_facts
                        .as_ref()
                        .map(|facts| (facts, continuation.mana_payment_staging.as_ref()))
                }
                _ => None,
            }),
    };
    if let Some((staged_facts, staging)) = staged {
        if staged_facts != facts {
            return false;
        }
        match staging {
            Some(staging) if staging.mana_source_activations == source_activations => {}
            Some(_) => return false,
            None if source_activations.is_empty() => {}
            None => return false,
        }
    }

    let mut seen_sources = std::collections::BTreeSet::new();
    for source in source_activations {
        if !seen_sources.insert(source.source_object) {
            return false;
        }
        let Some(source_object) = before.zones.objects.get(&source.source_object) else {
            return false;
        };
        let Some(source_location) = before.zones.locations.get(&source.source_object) else {
            return false;
        };
        if source_object.tapped || source_location.zone != mtgml_model::ZoneKind::Battlefield {
            return false;
        }
        let after_source = after.zones.objects.get(&source.source_object);
        if !after_source.is_some_and(|object| object.tapped) {
            return false;
        }
        for (index, amount) in source.produced_buckets.iter().enumerate() {
            let total = if index < 6 {
                &mut pool.unrestricted[index]
            } else {
                &mut pool.creature_spell_only[index - 6]
            };
            let Some(sum) = total.checked_add(*amount) else {
                return false;
            };
            *total = sum;
        }
    }

    for (index, spent) in spent_buckets.iter().enumerate() {
        let total = if index < 6 {
            &mut pool.unrestricted[index]
        } else {
            &mut pool.creature_spell_only[index - 6]
        };
        let Some(remaining) = total.checked_sub(*spent) else {
            return false;
        };
        *total = remaining;
    }
    if after.card_rules.mana.pools.get(&actor) != Some(&pool) {
        return false;
    }

    let action_source = match action {
        CostCommitActionV1::NonManaActivation { source_object, .. } => Some(source_object),
        CostCommitActionV1::Cast { .. } | CostCommitActionV1::StackResolution { .. } => None,
    };
    let taps_source = facts
        .reserved_nonmana_costs
        .contains(&mtgml_state::ReservedNonManaCost::TapSource);
    let sacrifices_source = facts
        .reserved_nonmana_costs
        .contains(&mtgml_state::ReservedNonManaCost::SacrificeSource);
    if (taps_source || sacrifices_source) && action_source.is_none() {
        return false;
    }
    if let Some(source) = action_source.filter(|_| taps_source || sacrifices_source) {
        let Some(before_object) = before.zones.objects.get(&source) else {
            return false;
        };
        if before_object.tapped {
            return false;
        }
        if sacrifices_source {
            if after.zones.objects.contains_key(&source) {
                return false;
            }
        } else if taps_source
            && !after
                .zones
                .objects
                .get(&source)
                .is_some_and(|object| object.tapped)
        {
            return false;
        }
    }
    true
}

/// Assigns sequential RuleEventIds without mutating the source cursor. A
/// caller commits the returned next cursor atomically with the transition.
pub fn allocate_rule_events(
    first_id: RuleEventId,
    state_revision: StateRevision,
    events: impl IntoIterator<Item = AuthoritativeRuleEventKind>,
) -> Result<(Vec<AuthoritativeRuleEvent>, RuleEventId), RuleEventCursorError> {
    if first_id.0 == 0 {
        return Err(RuleEventCursorError::InvalidStart);
    }
    let mut next = first_id.0;
    let mut result = Vec::new();
    for event in events {
        let following = next.checked_add(1).ok_or(RuleEventCursorError::Exhausted)?;
        result.push(AuthoritativeRuleEvent {
            event_id: RuleEventId(next),
            state_revision,
            event,
        });
        next = following;
    }
    Ok((result, RuleEventId(next)))
}

pub fn validate_rule_event_cursor(
    first_id: RuleEventId,
    next_id: RuleEventId,
    state_revision: StateRevision,
    events: &[AuthoritativeRuleEvent],
) -> Result<(), RuleEventCursorError> {
    if first_id.0 == 0 {
        return Err(RuleEventCursorError::InvalidStart);
    }
    let mut expected = first_id.0;
    for event in events {
        if event.event_id.0 != expected || event.state_revision != state_revision {
            return Err(RuleEventCursorError::SequenceMismatch);
        }
        expected = expected
            .checked_add(1)
            .ok_or(RuleEventCursorError::Exhausted)?;
    }
    if expected != next_id.0 {
        return Err(RuleEventCursorError::SequenceMismatch);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RuleEventCursorError {
    #[error("RuleEventId V3 cursor must start at a nonzero ID")]
    InvalidStart,
    #[error("RuleEventId V3 cursor exhausted")]
    Exhausted,
    #[error("authoritative RuleEventId V3 sequence or revision does not match")]
    SequenceMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EventDeltaError {
    #[error("authoritative event vector does not match StateDelta semantic operations")]
    Mismatch,
}

#[cfg(test)]
mod tests {
    use super::*;
    use mtgml_random::RootSeed256;
    use mtgml_state::{
        construct_synthetic_engine_state, AbilityAuthorityV1, ActionCostFacts, CounterKindV1,
        EngineState, ManaCost, ManaPoolV1, ManaSourceActivation, ManaSourceActivationCost,
        SemanticDeltaOperation, StackRecord, StateDelta, SyntheticResetInputs, SyntheticV4Setup,
        VisibilityPartition, ZoneLocation, ZonePosition,
    };

    fn state() -> EngineState {
        construct_synthetic_engine_state(SyntheticResetInputs {
            players: [PlayerId(1), PlayerId(2)],
            root_seed: RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap(),
            setup: SyntheticV4Setup::synthetic_compatibility(),
        })
        .unwrap()
    }

    #[test]
    fn cursor_assigns_and_validates_one_sequential_semantic_vector() {
        let (events, next) = allocate_rule_events(
            RuleEventId(8),
            StateRevision(4),
            [
                AuthoritativeRuleEventKind::CombatEnded,
                AuthoritativeRuleEventKind::EmptyCombatStepsSkipped,
            ],
        )
        .unwrap();
        assert_eq!(
            events
                .iter()
                .map(|event| event.event_id)
                .collect::<Vec<_>>(),
            [RuleEventId(8), RuleEventId(9),]
        );
        assert_eq!(next, RuleEventId(10));
        validate_rule_event_cursor(RuleEventId(8), next, StateRevision(4), &events).unwrap();
        assert_eq!(events[0].event.semantic_operations().len(), 1);
    }

    #[test]
    fn cursor_rejects_exhaustion_and_wrong_after_cursor_without_mutation() {
        assert_eq!(
            allocate_rule_events(
                RuleEventId(u64::MAX),
                StateRevision(1),
                [AuthoritativeRuleEventKind::CombatEnded],
            ),
            Err(RuleEventCursorError::Exhausted)
        );
        assert_eq!(
            validate_rule_event_cursor(RuleEventId(8), RuleEventId(12), StateRevision(4), &[]),
            Err(RuleEventCursorError::SequenceMismatch)
        );
    }

    #[test]
    fn event_semantics_require_the_exact_ordered_delta_operation() {
        let before = state();
        let mut after = before.clone();
        after.revision = StateRevision(before.revision.0 + 1);
        let event = AuthoritativeRuleEvent {
            event_id: RuleEventId(1),
            state_revision: after.revision,
            event: AuthoritativeRuleEventKind::CombatEnded,
        };
        let operation = event
            .event
            .semantic_operations()
            .into_iter()
            .next()
            .unwrap();
        let delta = mtgml_state::StateDelta::between(&before, &after, vec![operation]).unwrap();
        validate_event_delta_parity(std::slice::from_ref(&event), &delta).unwrap();

        let missing = mtgml_state::StateDelta {
            before_revision: before.revision,
            after_revision: after.revision,
            before_digest: mtgml_state::calculate_full_state_digest(&before).unwrap(),
            after_digest: mtgml_state::calculate_full_state_digest(&after).unwrap(),
            replacement: after.clone(),
            operations: vec![],
        };
        assert_eq!(
            validate_event_delta_parity(&[event], &missing),
            Err(EventDeltaError::Mismatch)
        );
    }

    #[test]
    fn basic_land_entry_event_owns_its_typed_state_index_operations() {
        use mtgml_state::{ObjectSnapshot, ZoneLocation, ZonePosition, ZoneTransition};

        let before = state();
        let actor = PlayerId(1);
        let old_object = GameObjectId(10);
        let new_object = GameObjectId(11);
        let from = ZoneLocation {
            zone: mtgml_model::ZoneKind::Hand,
            player: Some(actor),
            position: ZonePosition::Index { index: 0 },
            visibility: VisibilityPartition::OwnerOnly,
            partition: None,
        };
        let to = ZoneLocation {
            zone: mtgml_model::ZoneKind::Battlefield,
            player: None,
            position: ZonePosition::Unordered,
            visibility: VisibilityPartition::Public,
            partition: None,
        };
        let snapshot = |object, location: ZoneLocation| ObjectSnapshot {
            object,
            physical_card: Some(mtgml_model::PhysicalCardId(20)),
            card_definition: mtgml_model::CardDefinitionId(30),
            owner: actor,
            controller: actor,
            tapped: false,
            face_down: false,
            location,
        };
        let transition = ZoneTransition {
            old_object,
            new_object,
            physical_card: Some(mtgml_model::PhysicalCardId(20)),
            from: from.clone(),
            to: to.clone(),
            last_known: snapshot(old_object, from),
            new_snapshot: snapshot(new_object, to),
        };
        let event = AuthoritativeRuleEvent {
            event_id: RuleEventId(1),
            state_revision: StateRevision(1),
            event: AuthoritativeRuleEventKind::ZoneTransition {
                transition: Box::new(transition),
            },
        };
        let ability = mtgml_model::AbilityInstanceId(4);
        let operations = vec![
            SemanticDeltaOperation::ObjectEntered {
                old_object: Some(old_object),
                new_object,
                from_zone: mtgml_model::ZoneKind::Hand,
                to_zone: mtgml_model::ZoneKind::Battlefield,
                tapped: false,
                face: 0,
            },
            SemanticDeltaOperation::LandPlayCountChanged {
                player: actor,
                from: 0,
                to: 1,
            },
            SemanticDeltaOperation::AbilityAuthorityAdded {
                instance: ability,
                source: new_object,
                ability_key: 0,
            },
            SemanticDeltaOperation::AbilityIdentityChanged {
                perspective: actor,
                instance: ability,
                from: None,
                to: Some(mtgml_model::OpaqueAbilityId(7)),
            },
        ];
        let delta = StateDelta {
            before_revision: StateRevision(0),
            after_revision: StateRevision(1),
            before_digest: mtgml_model::FullStateDigest::from_digest_bytes([0; 32]),
            after_digest: mtgml_model::FullStateDigest::from_digest_bytes([1; 32]),
            replacement: before,
            operations,
        };

        assert_eq!(
            validate_event_delta_parity(std::slice::from_ref(&event), &delta),
            Ok(())
        );

        let mut fabricated = delta.clone();
        fabricated.operations[1] = SemanticDeltaOperation::LandPlayCountChanged {
            player: PlayerId(2),
            from: 0,
            to: 1,
        };
        assert_eq!(
            validate_event_delta_parity(std::slice::from_ref(&event), &fabricated),
            Err(EventDeltaError::Mismatch)
        );
    }

    #[test]
    fn event_delta_and_after_state_projection_must_agree() {
        let before = state();
        let player = PlayerId(1);
        let old_pool = before.card_rules.mana.pools[&player];
        let mut new_pool = old_pool;
        new_pool.unrestricted[4] = 1;
        let mut after = before.clone();
        after.card_rules.mana.pools.insert(player, new_pool);
        after.revision = StateRevision(before.revision.0 + 1);
        after.allocators.next_rule_event_id = RuleEventId(2);
        let operation = SemanticDeltaOperation::ManaPoolChanged {
            player,
            from: old_pool,
            to: new_pool,
            cause: ManaPoolChangeCauseV1::Produced,
        };
        let delta = StateDelta::between(&before, &after, vec![operation]).unwrap();
        let event = AuthoritativeRuleEvent {
            event_id: RuleEventId(1),
            state_revision: after.revision,
            event: AuthoritativeRuleEventKind::ManaPoolChanged {
                player,
                before: old_pool,
                after: new_pool,
                cause: ManaPoolChangeCauseV1::Produced,
            },
        };
        validate_event_delta_state(&before, &after, &[event], &delta).unwrap();

        let mut mismatched = after.clone();
        mismatched
            .card_rules
            .mana
            .pools
            .insert(player, ManaPoolV1::default());
        assert_eq!(
            validate_event_delta_state(&before, &mismatched, &[], &delta),
            Err(EventDeltaError::Mismatch)
        );

        let wrong_operation = SemanticDeltaOperation::ManaPoolChanged {
            player,
            from: old_pool,
            to: ManaPoolV1::default(),
            cause: ManaPoolChangeCauseV1::Produced,
        };
        assert!(StateDelta::between(&before, &after, vec![wrong_operation]).is_err());
    }

    #[test]
    fn atomic_cost_commit_projects_the_staged_source_output_payment_and_stack_item() {
        let mut before = state();
        let ability_source_object = GameObjectId(3);
        before.zones.objects.insert(
            ability_source_object,
            mtgml_state::GameObject {
                id: ability_source_object,
                physical_card: Some(mtgml_model::PhysicalCardId(3)),
                card_definition: mtgml_model::CardDefinitionId(3),
                owner: PlayerId(1),
                controller: PlayerId(1),
                tapped: false,
                face_down: false,
            },
        );
        before.zones.locations.insert(
            ability_source_object,
            ZoneLocation {
                zone: mtgml_model::ZoneKind::Battlefield,
                player: None,
                position: ZonePosition::Unordered,
                visibility: VisibilityPartition::Public,
                partition: None,
            },
        );
        before.allocators.next_object_id = GameObjectId(4);
        before.allocators.next_ability_id = mtgml_model::AbilityInstanceId(3);
        before.card_rules.abilities.by_instance.insert(
            mtgml_model::AbilityInstanceId(1),
            AbilityAuthorityV1 {
                source: ability_source_object,
                ability_key: 4,
            },
        );
        before.card_rules.abilities.by_instance.insert(
            mtgml_model::AbilityInstanceId(2),
            AbilityAuthorityV1 {
                source: GameObjectId(1),
                ability_key: 9,
            },
        );
        for object in before.zones.objects.keys().copied() {
            before.card_rules.faces.faces.insert(object, 0);
        }
        before.validate().unwrap();

        let source_context = mtgml_state::AbilitySourceContext {
            source: SourceContext {
                snapshot: mtgml_state::ObjectSnapshot {
                    object: ability_source_object,
                    physical_card: Some(mtgml_model::PhysicalCardId(3)),
                    card_definition: mtgml_model::CardDefinitionId(3),
                    owner: PlayerId(1),
                    controller: PlayerId(1),
                    tapped: false,
                    face_down: false,
                    location: before.zones.locations[&ability_source_object].clone(),
                },
                face_key: mtgml_card_ir::FaceKey(0),
                semantic_profile_id: mtgml_card_ir::CardSemanticProfileId::parse(
                    "test/ability@1.0.0",
                )
                .unwrap(),
            },
            ability_instance_id: mtgml_model::AbilityInstanceId(1),
            ability_key: mtgml_card_ir::AbilityKey(4),
        };
        let mut after = before.clone();
        after.revision = StateRevision(before.revision.0 + 1);
        after.allocators.next_stack_object_id = StackObjectId(2);
        after
            .zones
            .objects
            .get_mut(&GameObjectId(1))
            .unwrap()
            .tapped = true;
        after.zones.stack_records.insert(
            StackObjectId(1),
            StackRecord {
                id: StackObjectId(1),
                controller: PlayerId(1),
                payload: Some(StackItemPayload::ActivatedAbility {
                    source_context: source_context.clone(),
                    modes: vec![],
                    targets: vec![],
                    cost_facts: CostFacts::default(),
                }),
            },
        );
        after.zones.stack_order.push(StackObjectId(1));
        let facts = ActionCostFacts {
            mana_cost: Some(ManaCost {
                colored_wubrg_counts: [0, 0, 0, 1, 0],
                colorless_count: 0,
                generic_count: 0,
            }),
            reserved_nonmana_costs: vec![],
            selected_cost_operands: vec![],
        };
        let source_activations = vec![ManaSourceActivation {
            source_object: GameObjectId(1),
            source_ability_instance: mtgml_model::AbilityInstanceId(2),
            ability_key: mtgml_card_ir::AbilityKey(9),
            semantic_profile_id: mtgml_card_ir::CardSemanticProfileId::parse(
                "test/mana-source@1.0.0",
            )
            .unwrap(),
            activation_cost_receipt: ManaSourceActivationCost::TapSource,
            produced_buckets: [0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0],
        }];
        let cost_event = AuthoritativeRuleEventKind::CostCommitted {
            actor: PlayerId(1),
            action: CostCommitActionV1::NonManaActivation {
                source_object: ability_source_object,
                source_ability: mtgml_model::AbilityInstanceId(1),
            },
            facts,
            source_activations,
            spent_buckets: [0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0],
        };
        let tap_event = AuthoritativeRuleEventKind::ObjectTapped {
            object: GameObjectId(1),
            from: false,
            to: true,
        };
        let ability_event = AuthoritativeRuleEventKind::AbilityActivated {
            stack_object: StackObjectId(1),
            source: source_context,
            targets: vec![],
            cost_facts: CostFacts::default(),
            once_per_turn_use_committed: false,
        };
        let stack_event = AuthoritativeRuleEventKind::StackItemAdded {
            stack_object: StackObjectId(1),
            payload: after.zones.stack_records[&StackObjectId(1)]
                .payload
                .clone()
                .unwrap(),
        };
        let (events, next_event_id) = allocate_rule_events(
            before.allocators.next_rule_event_id,
            after.revision,
            [cost_event, tap_event, ability_event, stack_event],
        )
        .unwrap();
        after.allocators.next_rule_event_id = next_event_id;
        after.validate().unwrap();
        let mut operations = events
            .iter()
            .flat_map(AuthoritativeRuleEvent::semantic_operations)
            .collect::<Vec<_>>();
        operations.push(SemanticDeltaOperation::StackOrderChanged {
            from: vec![],
            to: vec![StackObjectId(1)],
        });
        let delta = StateDelta::between(&before, &after, operations).unwrap();
        validate_event_delta_state(&before, &after, &events, &delta).unwrap();

        let once_key = (ability_source_object, 4);
        let mut once_after = after.clone();
        once_after
            .card_rules
            .turn_history
            .once_ability_used
            .insert(once_key);
        once_after.validate().unwrap();
        let mut once_events = events.clone();
        for event in &mut once_events {
            if let AuthoritativeRuleEventKind::AbilityActivated {
                once_per_turn_use_committed,
                ..
            } = &mut event.event
            {
                *once_per_turn_use_committed = true;
            }
        }
        let once_operations = once_events
            .iter()
            .flat_map(AuthoritativeRuleEvent::semantic_operations)
            .chain([SemanticDeltaOperation::StackOrderChanged {
                from: vec![],
                to: vec![StackObjectId(1)],
            }])
            .collect();
        let once_delta = StateDelta::between(&before, &once_after, once_operations).unwrap();
        validate_event_delta_state(&before, &once_after, &once_events, &once_delta).unwrap();

        let mut mismatched_once_after = once_after.clone();
        mismatched_once_after
            .card_rules
            .turn_history
            .once_ability_used
            .remove(&once_key);
        mismatched_once_after
            .card_rules
            .turn_history
            .once_ability_used
            .insert((GameObjectId(1), 9));
        mismatched_once_after.validate().unwrap();
        let mismatched_once_operations = once_events
            .iter()
            .flat_map(AuthoritativeRuleEvent::semantic_operations)
            .chain([SemanticDeltaOperation::StackOrderChanged {
                from: vec![],
                to: vec![StackObjectId(1)],
            }])
            .collect();
        assert!(
            StateDelta::between(&before, &mismatched_once_after, mismatched_once_operations)
                .is_err()
        );

        let mut duplicate_receipt_operations = once_events
            .iter()
            .flat_map(AuthoritativeRuleEvent::semantic_operations)
            .collect::<Vec<_>>();
        let once_operation = duplicate_receipt_operations
            .iter()
            .find(|operation| {
                matches!(
                    operation,
                    SemanticDeltaOperation::AbilityActivated {
                        once_per_turn_use_committed: true,
                        ..
                    }
                )
            })
            .unwrap()
            .clone();
        duplicate_receipt_operations.push(once_operation);
        duplicate_receipt_operations.push(SemanticDeltaOperation::StackOrderChanged {
            from: vec![],
            to: vec![StackObjectId(1)],
        });
        assert!(StateDelta::between(&before, &once_after, duplicate_receipt_operations).is_err());

        let false_receipt_operations = events
            .iter()
            .flat_map(AuthoritativeRuleEvent::semantic_operations)
            .chain([SemanticDeltaOperation::StackOrderChanged {
                from: vec![],
                to: vec![StackObjectId(1)],
            }])
            .collect();
        assert!(StateDelta::between(&before, &once_after, false_receipt_operations).is_err());

        let mut missing_history_receipt = events.clone();
        for event in &mut missing_history_receipt {
            if let AuthoritativeRuleEventKind::AbilityActivated {
                once_per_turn_use_committed,
                ..
            } = &mut event.event
            {
                *once_per_turn_use_committed = true;
            }
        }
        let missing_history_operations = missing_history_receipt
            .iter()
            .flat_map(AuthoritativeRuleEvent::semantic_operations)
            .chain([SemanticDeltaOperation::StackOrderChanged {
                from: vec![],
                to: vec![StackObjectId(1)],
            }])
            .collect();
        assert!(StateDelta::between(&before, &after, missing_history_operations).is_err());

        let mut wrong_events = events.clone();
        if let AuthoritativeRuleEventKind::CostCommitted { spent_buckets, .. } =
            &mut wrong_events[0].event
        {
            spent_buckets[3] = 2;
        }
        let wrong_operations = wrong_events
            .iter()
            .flat_map(AuthoritativeRuleEvent::semantic_operations)
            .chain([SemanticDeltaOperation::StackOrderChanged {
                from: vec![],
                to: vec![StackObjectId(1)],
            }])
            .collect();
        let wrong_delta = StateDelta::between(&before, &after, wrong_operations).unwrap();
        assert_eq!(
            validate_event_delta_state(&before, &after, &wrong_events, &wrong_delta),
            Err(EventDeltaError::Mismatch)
        );
    }

    #[test]
    fn post_replacement_damage_event_matches_the_actual_player_life_delta() {
        let before = state();
        let player = PlayerId(1);
        let life_before = before.core.players[&player].life;
        let mut after = before.clone();
        after.core.players.get_mut(&player).unwrap().life -= 3;
        after.revision = StateRevision(before.revision.0 + 1);

        let kinds = [
            AuthoritativeRuleEventKind::DamageApplied {
                source: None,
                recipient: DamageRecipient::Player(player),
                post_replacement_amount: 3,
                damage_kind: DamageKind::Noncombat,
            },
            AuthoritativeRuleEventKind::LifeChanged {
                player,
                from: life_before,
                to: life_before - 3,
            },
        ];
        let (events, next) =
            allocate_rule_events(before.allocators.next_rule_event_id, after.revision, kinds)
                .unwrap();
        after.allocators.next_rule_event_id = next;
        let delta = StateDelta::between(
            &before,
            &after,
            events
                .iter()
                .flat_map(AuthoritativeRuleEvent::semantic_operations)
                .collect(),
        )
        .unwrap();
        validate_event_delta_state(&before, &after, &events, &delta).unwrap();

        let mut wrong_events = events.clone();
        if let AuthoritativeRuleEventKind::DamageApplied {
            post_replacement_amount,
            ..
        } = &mut wrong_events[0].event
        {
            *post_replacement_amount = 4;
        }
        let wrong_delta = StateDelta::between(
            &before,
            &after,
            wrong_events
                .iter()
                .flat_map(AuthoritativeRuleEvent::semantic_operations)
                .collect(),
        )
        .unwrap();
        assert_eq!(
            validate_event_delta_state(&before, &after, &wrong_events, &wrong_delta),
            Err(EventDeltaError::Mismatch)
        );
    }

    #[test]
    fn counter_delta_operation_is_bound_to_its_sequential_event_identity() {
        let before = state();
        let object = GameObjectId(1);
        let mut after = before.clone();
        after.revision = StateRevision(before.revision.0 + 1);
        after.allocators.next_rule_event_id = RuleEventId(2);
        after
            .card_rules
            .counters
            .counters
            .entry(object)
            .or_default()
            .insert(CounterKindV1::PlusOnePlusOne, 1);
        let event = AuthoritativeRuleEvent {
            event_id: RuleEventId(1),
            state_revision: after.revision,
            event: AuthoritativeRuleEventKind::CounterChanged {
                object,
                kind: CounterKindV1::PlusOnePlusOne,
                before: 0,
                after: 1,
            },
        };
        let operations = event.semantic_operations();
        assert!(matches!(
            &operations[0],
            SemanticDeltaOperation::CounterChanged { cause, .. }
                if *cause == event.event_id
        ));
        let delta = StateDelta::between(&before, &after, operations).unwrap();
        validate_event_delta_state(&before, &after, &[event], &delta).unwrap();
    }

    #[test]
    fn public_observation_occurrence_binds_lifecycle_to_exact_prior_rule_event() {
        let before = state();
        let mut after = before.clone();
        after.revision = StateRevision(before.revision.0 + 1);
        after.allocators.next_effect_id = mtgml_model::EffectInstanceId(2);
        after.allocators.next_rule_event_id = RuleEventId(3);
        let effect = TemporaryEffectRecord {
            id: mtgml_model::EffectInstanceId(1),
            affected_objects: vec![GameObjectId(1)],
            operation: mtgml_state::TemporaryOperation::PowerToughnessDelta {
                power: 1,
                toughness: 0,
            },
            expiry: mtgml_state::EffectExpiry::UntilEndOfTurn {
                turn_number: after.core.turn_number,
            },
            timestamp: None,
        };
        after.execution.effects.insert(effect.id, effect.clone());
        let sequence = before.knowledge.players[&PlayerId(1)].next_visible_sequence;
        let lifecycle = PerspectiveLifecycleAuditV1 {
            perspective: PlayerId(1),
            sequence,
            mutation: mtgml_state::PerspectiveLifecycleMutationV1::default(),
        };
        let mut engine = after.clone();
        mtgml_state::apply_perspective_lifecycle(&mut engine, &lifecycle).unwrap();
        after = engine;

        let events = vec![
            AuthoritativeRuleEvent {
                event_id: RuleEventId(1),
                state_revision: after.revision,
                event: AuthoritativeRuleEventKind::TemporaryEffectCreated {
                    effect: effect.clone(),
                },
            },
            AuthoritativeRuleEvent {
                event_id: RuleEventId(2),
                state_revision: after.revision,
                event: AuthoritativeRuleEventKind::PerspectiveObservationOccurrence {
                    lifecycle: Box::new(lifecycle),
                    source_event_id: RuleEventId(1),
                },
            },
        ];
        let operations = events
            .iter()
            .flat_map(AuthoritativeRuleEvent::semantic_operations)
            .collect();
        let delta = StateDelta::between(&before, &after, operations).unwrap();
        validate_event_delta_state(&before, &after, &events, &delta).unwrap();

        let mut occurrence = events[1].clone();
        let mut source = events[0].clone();
        let AuthoritativeRuleEventKind::PerspectiveObservationOccurrence {
            source_event_id, ..
        } = &mut occurrence.event
        else {
            unreachable!()
        };
        *source_event_id = RuleEventId(2);
        occurrence.event_id = RuleEventId(1);
        source.event_id = RuleEventId(2);
        let forward_reference = vec![occurrence, source];
        let forward_delta = StateDelta::between(
            &before,
            &after,
            forward_reference
                .iter()
                .flat_map(AuthoritativeRuleEvent::semantic_operations)
                .collect(),
        )
        .unwrap();
        assert_eq!(
            validate_event_delta_state(&before, &after, &forward_reference, &forward_delta),
            Err(EventDeltaError::Mismatch)
        );

        let mut forged = events.clone();
        let AuthoritativeRuleEventKind::PerspectiveObservationOccurrence {
            source_event_id, ..
        } = &mut forged[1].event
        else {
            unreachable!()
        };
        *source_event_id = RuleEventId(99);
        let forged_delta = StateDelta::between(
            &before,
            &after,
            forged
                .iter()
                .flat_map(AuthoritativeRuleEvent::semantic_operations)
                .collect(),
        )
        .unwrap();
        assert_eq!(
            validate_event_delta_state(&before, &after, &forged, &forged_delta),
            Err(EventDeltaError::Mismatch)
        );
    }

    #[test]
    fn observation_occurrence_replays_state_owned_cursor_and_identity_mutation() {
        let before = state();
        let object = GameObjectId(10);
        let mut after = before.clone();
        after.revision = StateRevision(before.revision.0 + 1);
        after.allocators.next_object_id = GameObjectId(11);
        after.allocators.next_effect_id = mtgml_model::EffectInstanceId(2);
        after.allocators.next_rule_event_id = RuleEventId(3);
        after.zones.objects.insert(
            object,
            mtgml_state::GameObject {
                id: object,
                physical_card: Some(mtgml_model::PhysicalCardId(10)),
                card_definition: mtgml_model::CardDefinitionId(10),
                owner: PlayerId(1),
                controller: PlayerId(1),
                tapped: false,
                face_down: false,
            },
        );
        after.zones.locations.insert(
            object,
            mtgml_state::ZoneLocation {
                zone: mtgml_model::ZoneKind::Battlefield,
                player: None,
                position: mtgml_state::ZonePosition::Unordered,
                visibility: mtgml_state::VisibilityPartition::Public,
                partition: None,
            },
        );
        after.card_rules.faces.faces.insert(object, 0);
        let effect = TemporaryEffectRecord {
            id: mtgml_model::EffectInstanceId(1),
            affected_objects: vec![object],
            operation: mtgml_state::TemporaryOperation::PowerToughnessDelta {
                power: 1,
                toughness: 0,
            },
            expiry: mtgml_state::EffectExpiry::UntilEndOfTurn {
                turn_number: after.core.turn_number,
            },
            timestamp: None,
        };
        after.execution.effects.insert(effect.id, effect.clone());
        let opaque = after.perspective_identities.players[&PlayerId(1)].next_opaque_object_id;
        let lifecycle = PerspectiveLifecycleAuditV1 {
            perspective: PlayerId(1),
            sequence: before.knowledge.players[&PlayerId(1)].next_visible_sequence,
            mutation: mtgml_state::PerspectiveLifecycleMutationV1 {
                identity: mtgml_state::IdentityMutationV1::Allocate { opaque, object },
                knowledge: Some(mtgml_state::KnowledgeMutationV1::Acquire {
                    opaque,
                    definition: Some(after.zones.objects[&object].card_definition),
                    location: Some(after.zones.locations[&object].clone()),
                    acquisition: mtgml_state::KnowledgeAcquisitionReason::Observed {
                        channel: mtgml_state::KnowledgeHistoryChannel::Public,
                        sequence: before.knowledge.players[&PlayerId(1)].next_visible_sequence,
                        cause: mtgml_state::KnowledgeAcquisitionCause::PublicEvent,
                    },
                }),
            },
        };
        let mut projected = after.clone();
        mtgml_state::apply_perspective_lifecycle(&mut projected, &lifecycle).unwrap();
        after = projected;
        let events = vec![
            AuthoritativeRuleEvent {
                event_id: RuleEventId(1),
                state_revision: after.revision,
                event: AuthoritativeRuleEventKind::TemporaryEffectCreated { effect },
            },
            AuthoritativeRuleEvent {
                event_id: RuleEventId(2),
                state_revision: after.revision,
                event: AuthoritativeRuleEventKind::PerspectiveObservationOccurrence {
                    lifecycle: Box::new(lifecycle),
                    source_event_id: RuleEventId(1),
                },
            },
        ];
        let delta = StateDelta {
            before_revision: before.revision,
            after_revision: after.revision,
            before_digest: mtgml_model::FullStateDigest::from_digest_bytes([0; 32]),
            after_digest: mtgml_model::FullStateDigest::from_digest_bytes([1; 32]),
            replacement: after.clone(),
            operations: Vec::new(),
        };
        assert!(
            validate_observation_occurrence_lifecycle(&before, &after, &events, &delta).is_ok()
        );

        let mut bad_lifecycle = events.clone();
        let AuthoritativeRuleEventKind::PerspectiveObservationOccurrence { lifecycle, .. } =
            &mut bad_lifecycle[1].event
        else {
            unreachable!()
        };
        lifecycle.sequence.0 += 1;
        assert_eq!(
            validate_observation_occurrence_lifecycle(&before, &after, &bad_lifecycle, &delta),
            Err(EventDeltaError::Mismatch)
        );
    }
}

/// Rules-owned trusted perception/authorization policy of one perspective
/// occurrence. Trusted references are authoritative `GameObjectId`s; the
/// public opaque substitution happens exclusively in observation projection.
/// This type never enters the authoritative audit because it does not mutate
/// authoritative state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PerspectiveObservationPolicyV1 {
    /// A tracked movement perceived by the perspective. Field flags decide
    /// which incarnations are authorized for opaque substitution. Revealing
    /// only the old incarnation models a tracked disappearance; revealing
    /// only the new one models an appearance of an already-tracked identity.
    MovedInSight {
        from_zone: ZoneKind,
        to_zone: ZoneKind,
        old_object: GameObjectId,
        new_object: GameObjectId,
        reveals_old: bool,
        reveals_new: bool,
    },
    /// A previously unknown incarnation becomes visible to the perspective.
    Appeared {
        from_zone: ZoneKind,
        to_zone: ZoneKind,
        new_object: GameObjectId,
    },
    /// Knowledge-only occurrence: no observed envelope is projected.
    NoEnvelope,
    /// A public object's tapped state is authorized to change for this
    /// perspective. Trusted `GameObjectId` stays here; opaque
    /// substitution happens exclusively in observation projection.
    ObjectTapped {
        object: GameObjectId,
        tapped: bool,
    },
    SawRandomOutcome {
        label: String,
        exclusive_upper_bound: u64,
        value: u64,
    },
    AnnouncedOutcome {
        code: String,
    },
}
