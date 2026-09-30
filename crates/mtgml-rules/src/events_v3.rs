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
    PerspectiveLifecycleAuditV1, SemanticDeltaOperationV3, SourceContext, StackItemEndKindV1,
    StackItemPayload, StateDeltaV3, TargetBinding, TemporaryEffectRecord, TurnPosition,
    ZoneTransition,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoritativeRuleEventV3 {
    pub event_id: RuleEventId,
    pub state_revision: StateRevision,
    pub event: AuthoritativeRuleEventKindV3,
}

impl AuthoritativeRuleEventV3 {
    fn semantic_operations(&self) -> Vec<SemanticDeltaOperationV3> {
        if let AuthoritativeRuleEventKindV3::CounterChanged {
            object,
            kind,
            before,
            after,
        } = &self.event
        {
            return vec![SemanticDeltaOperationV3::CounterChanged {
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
pub enum AuthoritativeRuleEventKindV3 {
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
}

impl AuthoritativeRuleEventKindV3 {
    /// Returns the typed audit operation(s) corresponding to this event.
    /// Bookkeeping-only changes remain in StateDelta replacement state and do
    /// not receive fabricated rule events.
    pub fn semantic_operations(&self) -> Vec<SemanticDeltaOperationV3> {
        match self {
            Self::ZoneTransition { transition } => vec![SemanticDeltaOperationV3::ZoneTransition {
                transition: transition.clone(),
            }],
            Self::ObjectCeasedToExist { object } => {
                vec![SemanticDeltaOperationV3::ObjectCeasedToExist { object: *object }]
            }
            Self::LifeChanged { player, from, to } => vec![SemanticDeltaOperationV3::LifeChanged {
                player: *player,
                from: *from,
                to: *to,
            }],
            Self::CombatDamageDealt { assignments } => {
                vec![SemanticDeltaOperationV3::CombatDamageDealt {
                    assignments: assignments.clone(),
                }]
            }
            Self::CombatDamageStepCompleted => {
                vec![SemanticDeltaOperationV3::CombatDamageStepCompleted]
            }
            Self::MarkedDamageChanged { creature, from, to } => {
                vec![SemanticDeltaOperationV3::MarkedDamageChanged {
                    creature: *creature,
                    from: *from,
                    to: *to,
                }]
            }
            Self::ObjectTapped { object, from, to } => {
                vec![SemanticDeltaOperationV3::ObjectTapped {
                    object: *object,
                    from: *from,
                    to: *to,
                }]
            }
            Self::DecisionCreated { decision } => vec![SemanticDeltaOperationV3::DecisionCreated {
                decision: *decision,
            }],
            Self::DecisionCleared { decision } => vec![SemanticDeltaOperationV3::DecisionCleared {
                decision: *decision,
            }],
            Self::SbaGraveyardOrderChosen {
                continuation,
                owner,
                top_to_bottom,
            } => vec![SemanticDeltaOperationV3::SbaGraveyardOrderChosen {
                continuation: *continuation,
                owner: *owner,
                top_to_bottom: top_to_bottom.clone(),
            }],
            Self::StateBasedActionsApplied { actions } => {
                vec![SemanticDeltaOperationV3::StateBasedActionsApplied {
                    actions: actions.clone(),
                }]
            }
            Self::PriorityChanged { from, to } => vec![SemanticDeltaOperationV3::PriorityChanged {
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
            } => vec![SemanticDeltaOperationV3::RandomValueSampled {
                stream: *stream,
                bound: *bound,
                value: *value,
                raw_words_consumed: *raw_words_consumed,
                cursor_before: *cursor_before,
                cursor_after: *cursor_after,
            }],
            Self::PublicOutcome { code } => {
                vec![SemanticDeltaOperationV3::PublicOutcome { code: code.clone() }]
            }
            Self::TurnPositionChanged { from, to } => {
                vec![SemanticDeltaOperationV3::TurnPositionChanged {
                    from: *from,
                    to: *to,
                }]
            }
            Self::AttackersDeclared {
                defending_player,
                attackers,
            } => vec![SemanticDeltaOperationV3::AttackersDeclared {
                defending_player: *defending_player,
                attackers: attackers.clone(),
            }],
            Self::BlockersDeclared { assignments } => {
                vec![SemanticDeltaOperationV3::BlockersDeclared {
                    assignments: assignments.clone(),
                }]
            }
            Self::CombatEnded => vec![SemanticDeltaOperationV3::CombatEnded],
            Self::EmptyCombatStepsSkipped => {
                vec![SemanticDeltaOperationV3::EmptyCombatStepsSkipped]
            }
            Self::UntapCompleted { affected_objects } => {
                vec![SemanticDeltaOperationV3::UntapCompleted {
                    affected_objects: affected_objects.clone(),
                }]
            }
            Self::ActivePlayerChanged { from, to } => {
                vec![SemanticDeltaOperationV3::ActivePlayerChanged {
                    from: *from,
                    to: *to,
                }]
            }
            Self::TurnNumberChanged { from, to } => {
                vec![SemanticDeltaOperationV3::TurnNumberChanged {
                    from: *from,
                    to: *to,
                }]
            }
            Self::StackItemAdded {
                stack_object,
                payload,
            } => vec![SemanticDeltaOperationV3::StackItemCreated {
                stack_object: *stack_object,
                payload: Box::new(payload.clone()),
            }],
            Self::StackItemRemoved {
                stack_object,
                payload,
                result,
            } => vec![SemanticDeltaOperationV3::StackItemEnded {
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
            } => vec![SemanticDeltaOperationV3::SpellCast {
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
            } => vec![SemanticDeltaOperationV3::AbilityActivated {
                stack_object: *stack_object,
                source: Box::new(source.clone()),
                targets: targets.clone(),
                cost_facts: cost_facts.clone(),
                once_per_turn_use_committed: *once_per_turn_use_committed,
            }],
            Self::TargetDeclared {
                source_stack_item,
                targets,
            } => vec![SemanticDeltaOperationV3::TargetDeclared {
                source_stack_item: *source_stack_item,
                targets: targets.clone(),
            }],
            Self::CounterChanged { .. } => Vec::new(),
            Self::TriggerDetected { trigger } => {
                vec![SemanticDeltaOperationV3::TriggerCreated {
                    trigger: Box::new(trigger.clone()),
                }]
            }
            Self::TriggerPlaced {
                trigger,
                stack_object,
                payload,
            } => vec![SemanticDeltaOperationV3::TriggerPlaced {
                trigger: *trigger,
                stack_object: *stack_object,
                payload: Box::new(payload.clone()),
            }],
            Self::ManaPoolChanged {
                player,
                before,
                after,
                cause,
            } => vec![SemanticDeltaOperationV3::ManaPoolChanged {
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
            } => vec![SemanticDeltaOperationV3::AtomicCostCommitted {
                actor: *actor,
                action: *action,
                mana_cost: facts.mana_cost,
                source_activations: source_activations.clone(),
                spent_buckets: *spent_buckets,
                reserved_nonmana_costs: facts.reserved_nonmana_costs.clone(),
                selected_cost_operands: facts.selected_cost_operands.clone(),
            }],
            Self::TemporaryEffectCreated { effect } => {
                vec![SemanticDeltaOperationV3::TemporaryEffectChanged {
                    effect: effect.id,
                    from: None,
                    to: Some(Box::new(effect.clone())),
                }]
            }
            Self::TemporaryEffectExpired { effect } => {
                vec![SemanticDeltaOperationV3::TemporaryEffectChanged {
                    effect: effect.id,
                    from: Some(Box::new(effect.clone())),
                    to: None,
                }]
            }
            Self::PerspectiveObservationOccurrence { lifecycle, .. } => {
                vec![SemanticDeltaOperationV3::PerspectiveLifecycle {
                    lifecycle: lifecycle.as_ref().clone(),
                }]
            }
            Self::DamageApplied {
                source,
                recipient,
                post_replacement_amount,
                damage_kind,
            } => vec![SemanticDeltaOperationV3::DamageApplied {
                source: source.clone().map(Box::new),
                recipient: *recipient,
                post_replacement_amount: *post_replacement_amount,
                damage_kind: *damage_kind,
            }],
        }
    }
}

/// Requires every auditable event to have its exact typed Delta operation in
/// rules order. Only private continuation/request advances and the associated
/// stack-order vector operation may appear without a standalone event.
pub fn validate_event_delta_parity_v3(
    events: &[AuthoritativeRuleEventV3],
    delta: &StateDeltaV3,
) -> Result<(), EventDeltaV3Error> {
    let (grouped_entry_operations, grouped_entry_events) =
        validate_basic_land_entry_group(events, delta)?;
    let expected = events
        .iter()
        .filter(|event| !grouped_entry_events.contains(&event.event_id))
        .flat_map(AuthoritativeRuleEventV3::semantic_operations)
        .collect::<Vec<_>>();
    let mut expected_index = 0;
    for (operation_index, operation) in delta.operations.iter().enumerate() {
        if matches!(
            operation,
            SemanticDeltaOperationV3::ContinuationChanged { .. }
                | SemanticDeltaOperationV3::PendingRequestChanged { .. }
                | SemanticDeltaOperationV3::StackOrderChanged { .. }
        ) || grouped_entry_operations.contains(&operation_index)
        {
            continue;
        }
        if expected.get(expected_index) != Some(operation) {
            return Err(EventDeltaV3Error::Mismatch);
        }
        expected_index += 1;
    }
    if expected_index != expected.len() {
        return Err(EventDeltaV3Error::Mismatch);
    }
    let has_stack_event = events.iter().any(|event| {
        matches!(
            &event.event,
            AuthoritativeRuleEventKindV3::StackItemAdded { .. }
                | AuthoritativeRuleEventKindV3::StackItemRemoved { .. }
                | AuthoritativeRuleEventKindV3::TriggerPlaced { .. }
        )
    });
    if !has_stack_event
        && delta.operations.iter().any(|operation| {
            matches!(
                operation,
                SemanticDeltaOperationV3::StackOrderChanged { .. }
            )
        })
    {
        return Err(EventDeltaV3Error::Mismatch);
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
    events: &[AuthoritativeRuleEventV3],
    delta: &StateDeltaV3,
) -> Result<
    (
        std::collections::BTreeSet<usize>,
        std::collections::BTreeSet<RuleEventId>,
    ),
    EventDeltaV3Error,
> {
    use SemanticDeltaOperationV3 as V3;

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
            AuthoritativeRuleEventKindV3::ZoneTransition { transition } => {
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
        return Err(EventDeltaV3Error::Mismatch);
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
        return Err(EventDeltaV3Error::Mismatch);
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
        return Err(EventDeltaV3Error::Mismatch);
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
        return Err(EventDeltaV3Error::Mismatch);
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
        return Err(EventDeltaV3Error::Mismatch);
    }
    for (index, _) in alias_entries {
        grouped.insert(index);
    }

    Ok((grouped, [entry_event_id].into_iter().collect()))
}

/// Applies the successor delta and checks event projections against the same
/// before/after state. Legacy event kinds keep their existing validator.
pub fn validate_event_delta_state_v3(
    before: &EngineState,
    after: &EngineState,
    events: &[AuthoritativeRuleEventV3],
    delta: &StateDeltaV3,
) -> Result<(), EventDeltaV3Error> {
    validate_event_delta_state_v3_inner(before, after, events, delta, DeltaCheck::Apply)
}

/// Validates the events of a transition this crate just produced, whose
/// delta `StateDeltaV3::between_structural_only(before, after, ..)` has just
/// built. That constructor validated both states and computed their digests,
/// so the delta is checked against the same states without re-applying it
/// (which would compute both digests again).
pub(crate) fn validate_events_for_built_delta_v3(
    before: &EngineState,
    after: &EngineState,
    events: &[AuthoritativeRuleEventV3],
    delta: &StateDeltaV3,
) -> Result<(), EventDeltaV3Error> {
    validate_event_delta_state_v3_inner(before, after, events, delta, DeltaCheck::BuiltFrom)
}

/// How the delta is checked against the before and after states.
enum DeltaCheck {
    /// Apply the delta with full state admission.
    Apply,
    /// The delta was just built from these states; compare, do not apply.
    BuiltFrom,
}

fn validate_event_delta_state_v3_inner(
    before: &EngineState,
    after: &EngineState,
    events: &[AuthoritativeRuleEventV3],
    delta: &StateDeltaV3,
    check: DeltaCheck,
) -> Result<(), EventDeltaV3Error> {
    match check {
        DeltaCheck::Apply => {
            let applied = delta
                .apply(before)
                .map_err(|_| EventDeltaV3Error::Mismatch)?;
            if &applied != after {
                return Err(EventDeltaV3Error::Mismatch);
            }
        }
        DeltaCheck::BuiltFrom => {
            if delta.before_revision != before.revision
                || delta.after_revision != after.revision
                || delta.replacement != *after
            {
                return Err(EventDeltaV3Error::Mismatch);
            }
        }
    }
    validate_rule_event_cursor_v3(
        before.allocators.next_rule_event_id,
        after.allocators.next_rule_event_id,
        after.revision,
        events,
    )
    .map_err(|_| EventDeltaV3Error::Mismatch)?;
    validate_event_delta_parity_v3(events, delta)?;
    validate_observation_occurrence_lifecycle(before, after, events, delta)?;
    let before_stack_order = &before.zones.stack_order;
    let after_stack_order = &after.zones.stack_order;
    let stack_order_operations = delta
        .operations
        .iter()
        .filter_map(|operation| match operation {
            SemanticDeltaOperationV3::StackOrderChanged { from, to } => Some((from, to)),
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
                    AuthoritativeRuleEventKindV3::StackItemAdded { .. }
                        | AuthoritativeRuleEventKindV3::StackItemRemoved { .. }
                        | AuthoritativeRuleEventKindV3::TriggerPlaced { .. }
                )
            })
        {
            return Err(EventDeltaV3Error::Mismatch);
        }
    } else if !stack_order_operations.is_empty() {
        return Err(EventDeltaV3Error::Mismatch);
    }
    for operation in &delta.operations {
        validate_delta_operation_projection_v3(before, after, operation)?;
    }
    for event in events {
        validate_event_projection_v3(before, after, &event.event)?;
    }
    validate_damage_state_projection_v3(before, after, events)?;
    Ok(())
}

fn validate_observation_occurrence_lifecycle(
    before: &EngineState,
    after: &EngineState,
    events: &[AuthoritativeRuleEventV3],
    delta: &StateDeltaV3,
) -> Result<(), EventDeltaV3Error> {
    let mut projected = after.clone();
    projected.knowledge = before.knowledge.clone();
    projected.perspective_identities = before.perspective_identities.clone();
    for (index, event) in events.iter().enumerate() {
        let (lifecycle, source_event_id) = match &event.event {
            AuthoritativeRuleEventKindV3::PerspectiveObservationOccurrence {
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
                return Err(EventDeltaV3Error::Mismatch);
            };
            if source_index >= index
                || !is_projectable_public_source_event(&events[source_index].event)
            {
                return Err(EventDeltaV3Error::Mismatch);
            }
        }
        if projected
            .knowledge
            .players
            .get(&lifecycle.perspective)
            .is_none_or(|knowledge| knowledge.next_visible_sequence != lifecycle.sequence)
            || mtgml_state::apply_perspective_lifecycle(&mut projected, lifecycle).is_err()
        {
            return Err(EventDeltaV3Error::Mismatch);
        }
    }
    apply_delta_identity_changes_v3(&mut projected, delta)?;
    if projected.knowledge != after.knowledge
        || projected.perspective_identities != after.perspective_identities
    {
        return Err(EventDeltaV3Error::Mismatch);
    }
    Ok(())
}

fn apply_delta_identity_changes_v3(
    projected: &mut mtgml_state::EngineState,
    delta: &StateDeltaV3,
) -> Result<(), EventDeltaV3Error> {
    use SemanticDeltaOperationV3 as V3;

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
                    .ok_or(EventDeltaV3Error::Mismatch)?;
                if identity.ability_to_opaque.get(instance).copied() != *from {
                    return Err(EventDeltaV3Error::Mismatch);
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
                        return Err(EventDeltaV3Error::Mismatch);
                    }
                    identity.next_opaque_ability_id = mtgml_model::OpaqueAbilityId(
                        opaque.0.checked_add(1).ok_or(EventDeltaV3Error::Mismatch)?,
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
                    .ok_or(EventDeltaV3Error::Mismatch)?;
                if identity.next_player_decision_id != request.player_decision_id {
                    return Err(EventDeltaV3Error::Mismatch);
                }
                identity.next_player_decision_id = mtgml_model::PlayerDecisionIdV1(
                    request
                        .player_decision_id
                        .0
                        .checked_add(1)
                        .ok_or(EventDeltaV3Error::Mismatch)?,
                );
            }
            _ => {}
        }
    }
    Ok(())
}

fn is_projectable_public_source_event(event: &AuthoritativeRuleEventKindV3) -> bool {
    matches!(
        event,
        AuthoritativeRuleEventKindV3::StackItemAdded { .. }
            | AuthoritativeRuleEventKindV3::StackItemRemoved { .. }
            | AuthoritativeRuleEventKindV3::TriggerPlaced { .. }
            | AuthoritativeRuleEventKindV3::CounterChanged { .. }
            | AuthoritativeRuleEventKindV3::ManaPoolChanged { .. }
            | AuthoritativeRuleEventKindV3::TemporaryEffectCreated { .. }
            | AuthoritativeRuleEventKindV3::TemporaryEffectExpired { .. }
            | AuthoritativeRuleEventKindV3::ZoneTransition { .. }
            | AuthoritativeRuleEventKindV3::ObjectTapped { .. }
    )
}

fn validate_damage_state_projection_v3(
    before: &EngineState,
    after: &EngineState,
    events: &[AuthoritativeRuleEventV3],
) -> Result<(), EventDeltaV3Error> {
    let mut player_damage = std::collections::BTreeMap::<PlayerId, u64>::new();
    let mut object_damage = std::collections::BTreeMap::<GameObjectId, u64>::new();
    for event in events {
        let AuthoritativeRuleEventKindV3::DamageApplied {
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
            return Err(EventDeltaV3Error::Mismatch);
        };
        *total = next;
    }
    for (player, amount) in player_damage {
        let Some(before_player) = before.core.players.get(&player) else {
            return Err(EventDeltaV3Error::Mismatch);
        };
        let Some(after_player) = after.core.players.get(&player) else {
            return Err(EventDeltaV3Error::Mismatch);
        };
        let actual_loss = i128::from(before_player.life) - i128::from(after_player.life);
        if actual_loss != i128::from(amount) {
            return Err(EventDeltaV3Error::Mismatch);
        }
    }
    // No state component records marked damage yet: damage to an object
    // cannot be projected and fails closed.
    if !object_damage.is_empty() {
        return Err(EventDeltaV3Error::Mismatch);
    }
    Ok(())
}

fn validate_delta_operation_projection_v3(
    before: &EngineState,
    after: &EngineState,
    operation: &SemanticDeltaOperationV3,
) -> Result<(), EventDeltaV3Error> {
    let valid = match operation {
        // Operations whose state change the coverage check owns.
        SemanticDeltaOperationV3::ZoneTransition { .. }
        | SemanticDeltaOperationV3::ObjectCeasedToExist { .. }
        | SemanticDeltaOperationV3::LifeChanged { .. }
        | SemanticDeltaOperationV3::CombatDamageDealt { .. }
        | SemanticDeltaOperationV3::CombatDamageStepCompleted
        | SemanticDeltaOperationV3::MarkedDamageChanged { .. }
        | SemanticDeltaOperationV3::ObjectTapped { .. }
        | SemanticDeltaOperationV3::DecisionCreated { .. }
        | SemanticDeltaOperationV3::DecisionCleared { .. }
        | SemanticDeltaOperationV3::SbaGraveyardOrderChosen { .. }
        | SemanticDeltaOperationV3::StateBasedActionsApplied { .. }
        | SemanticDeltaOperationV3::PriorityChanged { .. }
        | SemanticDeltaOperationV3::RandomValueSampled { .. }
        | SemanticDeltaOperationV3::PublicOutcome { .. }
        | SemanticDeltaOperationV3::TurnPositionChanged { .. }
        | SemanticDeltaOperationV3::AttackersDeclared { .. }
        | SemanticDeltaOperationV3::BlockersDeclared { .. }
        | SemanticDeltaOperationV3::CombatEnded
        | SemanticDeltaOperationV3::EmptyCombatStepsSkipped
        | SemanticDeltaOperationV3::UntapCompleted { .. }
        | SemanticDeltaOperationV3::ActivePlayerChanged { .. }
        | SemanticDeltaOperationV3::TurnNumberChanged { .. }
        | SemanticDeltaOperationV3::PerspectiveLifecycle { .. }
        | SemanticDeltaOperationV3::LandPlayCountChanged { .. }
        | SemanticDeltaOperationV3::AbilityIdentityChanged { .. }
        | SemanticDeltaOperationV3::AttachmentChanged { .. }
        | SemanticDeltaOperationV3::ObjectFaceChanged { .. }
        | SemanticDeltaOperationV3::ObjectEntered { .. }
        | SemanticDeltaOperationV3::AbilityAuthorityAdded { .. }
        | SemanticDeltaOperationV3::AbilityAuthorityRemoved { .. }
        | SemanticDeltaOperationV3::DamageApplied { .. } => true,
        SemanticDeltaOperationV3::StackOrderChanged { from, to } => {
            &before.zones.stack_order == from && &after.zones.stack_order == to
        }
        SemanticDeltaOperationV3::StackItemCreated {
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
        SemanticDeltaOperationV3::StackItemEnded {
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
        SemanticDeltaOperationV3::SpellCast {
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
        SemanticDeltaOperationV3::AbilityActivated {
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
        SemanticDeltaOperationV3::TargetDeclared {
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
        SemanticDeltaOperationV3::CounterChanged {
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
        SemanticDeltaOperationV3::TriggerCreated { trigger } => {
            !before.execution.waiting_triggers.contains_key(&trigger.id)
                && after.execution.waiting_triggers.get(&trigger.id) == Some(trigger.as_ref())
        }
        SemanticDeltaOperationV3::TriggerPlaced {
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
        SemanticDeltaOperationV3::ManaPoolChanged {
            player, from, to, ..
        } => {
            before.card_rules.mana.pools.get(player) == Some(from)
                && after.card_rules.mana.pools.get(player) == Some(to)
        }
        SemanticDeltaOperationV3::AtomicCostCommitted {
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
        SemanticDeltaOperationV3::ContinuationChanged {
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
        SemanticDeltaOperationV3::PendingRequestChanged { from, to } => {
            before.execution.pending_decision.as_ref() == from.as_deref()
                && after.execution.pending_decision.as_ref() == to.as_deref()
        }
        SemanticDeltaOperationV3::TemporaryEffectChanged { effect, from, to } => {
            before.execution.effects.get(effect) == from.as_deref()
                && after.execution.effects.get(effect) == to.as_deref()
        }
    };
    if valid {
        Ok(())
    } else {
        Err(EventDeltaV3Error::Mismatch)
    }
}

fn validate_event_projection_v3(
    before: &EngineState,
    after: &EngineState,
    event: &AuthoritativeRuleEventKindV3,
) -> Result<(), EventDeltaV3Error> {
    let valid = match event {
        AuthoritativeRuleEventKindV3::ZoneTransition { transition } => {
            object_snapshot_matches(before, &transition.last_known)
                && !after.zones.objects.contains_key(&transition.old_object)
                && object_snapshot_matches(after, &transition.new_snapshot)
                && after.zones.locations.get(&transition.new_object) == Some(&transition.to)
        }
        AuthoritativeRuleEventKindV3::ObjectCeasedToExist { object } => {
            before.zones.objects.contains_key(object) && !after.zones.objects.contains_key(object)
        }
        AuthoritativeRuleEventKindV3::LifeChanged { player, from, to } => {
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
        AuthoritativeRuleEventKindV3::MarkedDamageChanged { .. } => false,
        AuthoritativeRuleEventKindV3::ObjectTapped { object, from, to } => {
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
        AuthoritativeRuleEventKindV3::DecisionCreated { decision } => {
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
        AuthoritativeRuleEventKindV3::DecisionCleared { decision } => {
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
        AuthoritativeRuleEventKindV3::PriorityChanged { from, to } => {
            from != to && before.core.priority == *from && after.core.priority == *to
        }
        AuthoritativeRuleEventKindV3::TurnPositionChanged { from, to } => {
            from != to && before.core.position == *from && after.core.position == *to
        }
        AuthoritativeRuleEventKindV3::RandomValueSampled {
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
        AuthoritativeRuleEventKindV3::PublicOutcome { .. } => true,
        AuthoritativeRuleEventKindV3::SbaGraveyardOrderChosen {
            continuation,
            owner,
            top_to_bottom,
        } => after
            .execution
            .continuations
            .get(continuation)
            .is_some_and(|record| match &record.payload {
                mtgml_state::ContinuationPayloadV3::MagicSbaGraveyardOrderV1 {
                    completed_owner_orders,
                    ..
                } => completed_owner_orders
                    .iter()
                    .any(|order| order.owner == *owner && order.top_to_bottom == *top_to_bottom),
                _ => false,
            }),
        AuthoritativeRuleEventKindV3::StateBasedActionsApplied { actions } => {
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
        AuthoritativeRuleEventKindV3::CombatDamageStepCompleted => after
            .combat
            .as_ref()
            .is_some_and(|combat| combat.damage_step_completed),
        AuthoritativeRuleEventKindV3::AttackersDeclared {
            defending_player,
            attackers,
        } => after.combat.as_ref().is_some_and(|combat| {
            combat.defending_player == *defending_player && combat.attackers == *attackers
        }),
        AuthoritativeRuleEventKindV3::CombatEnded => {
            before.combat.is_some() && after.combat.is_none()
        }
        AuthoritativeRuleEventKindV3::EmptyCombatStepsSkipped => {
            before.core.position != after.core.position
        }
        AuthoritativeRuleEventKindV3::UntapCompleted { affected_objects } => {
            affected_objects.iter().all(|object| {
                after
                    .zones
                    .objects
                    .get(object)
                    .is_some_and(|state| !state.tapped)
            })
        }
        AuthoritativeRuleEventKindV3::ActivePlayerChanged { from, to } => {
            from != to && before.core.active_player == *from && after.core.active_player == *to
        }
        AuthoritativeRuleEventKindV3::TurnNumberChanged { from, to } => {
            from != to && before.core.turn_number == *from && after.core.turn_number == *to
        }
        // Combat assignment/blocked-state state projection remains closed
        // until its exact legal relation is characterized and accepted.
        AuthoritativeRuleEventKindV3::CombatDamageDealt { .. }
        | AuthoritativeRuleEventKindV3::BlockersDeclared { .. } => false,
        AuthoritativeRuleEventKindV3::PerspectiveObservationOccurrence { .. } => true,
        AuthoritativeRuleEventKindV3::DamageApplied { .. } => true,
        AuthoritativeRuleEventKindV3::StackItemAdded {
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
        AuthoritativeRuleEventKindV3::StackItemRemoved {
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
        AuthoritativeRuleEventKindV3::SpellCast {
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
        AuthoritativeRuleEventKindV3::AbilityActivated {
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
        AuthoritativeRuleEventKindV3::TargetDeclared {
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
        AuthoritativeRuleEventKindV3::CounterChanged {
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
        AuthoritativeRuleEventKindV3::TriggerDetected { trigger } => {
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
        AuthoritativeRuleEventKindV3::TriggerPlaced {
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
        AuthoritativeRuleEventKindV3::ManaPoolChanged {
            player,
            before: old_pool,
            after: new_pool,
            ..
        } => {
            before.card_rules.mana.pools.get(player) == Some(old_pool)
                && after.card_rules.mana.pools.get(player) == Some(new_pool)
        }
        AuthoritativeRuleEventKindV3::CostCommitted {
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
        AuthoritativeRuleEventKindV3::TemporaryEffectCreated { effect } => {
            !before.execution.effects.contains_key(&effect.id)
                && after.execution.effects.get(&effect.id) == Some(effect)
        }
        AuthoritativeRuleEventKindV3::TemporaryEffectExpired { effect } => {
            before.execution.effects.get(&effect.id) == Some(effect)
                && !after.execution.effects.contains_key(&effect.id)
        }
    };
    if valid {
        Ok(())
    } else {
        Err(EventDeltaV3Error::Mismatch)
    }
}

fn object_snapshot_matches(state: &EngineState, snapshot: &mtgml_state::ObjectSnapshot) -> bool {
    state
        .zones
        .objects
        .get(&snapshot.object)
        .is_some_and(|object| {
            object.physical_card == snapshot.physical_card
                && object.card_definition == snapshot.card_definition
                && object.owner == snapshot.owner
                && object.controller == snapshot.controller
                && object.tapped == snapshot.tapped
                && object.face_down == snapshot.face_down
        })
        && state.zones.locations.get(&snapshot.object) == Some(&snapshot.location)
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
                mtgml_state::ContinuationPayloadV3::Cast(continuation)
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
                mtgml_state::ContinuationPayloadV3::NonManaActivation(continuation)
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
                mtgml_state::ContinuationPayloadV3::StackResolution(continuation)
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
pub fn allocate_rule_events_v3(
    first_id: RuleEventId,
    state_revision: StateRevision,
    events: impl IntoIterator<Item = AuthoritativeRuleEventKindV3>,
) -> Result<(Vec<AuthoritativeRuleEventV3>, RuleEventId), RuleEventCursorV3Error> {
    if first_id.0 == 0 {
        return Err(RuleEventCursorV3Error::InvalidStart);
    }
    let mut next = first_id.0;
    let mut result = Vec::new();
    for event in events {
        let following = next
            .checked_add(1)
            .ok_or(RuleEventCursorV3Error::Exhausted)?;
        result.push(AuthoritativeRuleEventV3 {
            event_id: RuleEventId(next),
            state_revision,
            event,
        });
        next = following;
    }
    Ok((result, RuleEventId(next)))
}

pub fn validate_rule_event_cursor_v3(
    first_id: RuleEventId,
    next_id: RuleEventId,
    state_revision: StateRevision,
    events: &[AuthoritativeRuleEventV3],
) -> Result<(), RuleEventCursorV3Error> {
    if first_id.0 == 0 {
        return Err(RuleEventCursorV3Error::InvalidStart);
    }
    let mut expected = first_id.0;
    for event in events {
        if event.event_id.0 != expected || event.state_revision != state_revision {
            return Err(RuleEventCursorV3Error::SequenceMismatch);
        }
        expected = expected
            .checked_add(1)
            .ok_or(RuleEventCursorV3Error::Exhausted)?;
    }
    if expected != next_id.0 {
        return Err(RuleEventCursorV3Error::SequenceMismatch);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RuleEventCursorV3Error {
    #[error("RuleEventId V3 cursor must start at a nonzero ID")]
    InvalidStart,
    #[error("RuleEventId V3 cursor exhausted")]
    Exhausted,
    #[error("authoritative RuleEventId V3 sequence or revision does not match")]
    SequenceMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EventDeltaV3Error {
    #[error("authoritative event vector does not match StateDeltaV3 semantic operations")]
    Mismatch,
}

#[cfg(test)]
mod tests {
    use super::*;
    use mtgml_random::RootSeed256;
    use mtgml_state::{
        construct_synthetic_engine_state, AbilityAuthorityV1, ActionCostFacts, CounterKindV1,
        EngineState, ManaCost, ManaPoolV1, ManaSourceActivation, ManaSourceActivationCost,
        SemanticDeltaOperationV3, StackRecord, StateDeltaV3, SyntheticResetInputs,
        SyntheticV4Setup, VisibilityPartition, ZoneLocation, ZonePosition,
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
        let (events, next) = allocate_rule_events_v3(
            RuleEventId(8),
            StateRevision(4),
            [
                AuthoritativeRuleEventKindV3::CombatEnded,
                AuthoritativeRuleEventKindV3::EmptyCombatStepsSkipped,
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
        validate_rule_event_cursor_v3(RuleEventId(8), next, StateRevision(4), &events).unwrap();
        assert_eq!(events[0].event.semantic_operations().len(), 1);
    }

    #[test]
    fn cursor_rejects_exhaustion_and_wrong_after_cursor_without_mutation() {
        assert_eq!(
            allocate_rule_events_v3(
                RuleEventId(u64::MAX),
                StateRevision(1),
                [AuthoritativeRuleEventKindV3::CombatEnded],
            ),
            Err(RuleEventCursorV3Error::Exhausted)
        );
        assert_eq!(
            validate_rule_event_cursor_v3(RuleEventId(8), RuleEventId(12), StateRevision(4), &[]),
            Err(RuleEventCursorV3Error::SequenceMismatch)
        );
    }

    #[test]
    fn event_semantics_require_the_exact_ordered_delta_operation() {
        let before = state();
        let mut after = before.clone();
        after.revision = StateRevision(before.revision.0 + 1);
        let event = AuthoritativeRuleEventV3 {
            event_id: RuleEventId(1),
            state_revision: after.revision,
            event: AuthoritativeRuleEventKindV3::CombatEnded,
        };
        let operation = event
            .event
            .semantic_operations()
            .into_iter()
            .next()
            .unwrap();
        let delta = mtgml_state::StateDeltaV3::between(&before, &after, vec![operation]).unwrap();
        validate_event_delta_parity_v3(std::slice::from_ref(&event), &delta).unwrap();

        let missing = mtgml_state::StateDeltaV3 {
            before_revision: before.revision,
            after_revision: after.revision,
            before_digest: mtgml_state::calculate_full_state_digest_v7(&before).unwrap(),
            after_digest: mtgml_state::calculate_full_state_digest_v7(&after).unwrap(),
            replacement: after.clone(),
            operations: vec![],
        };
        assert_eq!(
            validate_event_delta_parity_v3(&[event], &missing),
            Err(EventDeltaV3Error::Mismatch)
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
        let event = AuthoritativeRuleEventV3 {
            event_id: RuleEventId(1),
            state_revision: StateRevision(1),
            event: AuthoritativeRuleEventKindV3::ZoneTransition {
                transition: Box::new(transition),
            },
        };
        let ability = mtgml_model::AbilityInstanceId(4);
        let operations = vec![
            SemanticDeltaOperationV3::ObjectEntered {
                old_object: Some(old_object),
                new_object,
                from_zone: mtgml_model::ZoneKind::Hand,
                to_zone: mtgml_model::ZoneKind::Battlefield,
                tapped: false,
                face: 0,
            },
            SemanticDeltaOperationV3::LandPlayCountChanged {
                player: actor,
                from: 0,
                to: 1,
            },
            SemanticDeltaOperationV3::AbilityAuthorityAdded {
                instance: ability,
                source: new_object,
                ability_key: 0,
            },
            SemanticDeltaOperationV3::AbilityIdentityChanged {
                perspective: actor,
                instance: ability,
                from: None,
                to: Some(mtgml_model::OpaqueAbilityId(7)),
            },
        ];
        let delta = StateDeltaV3 {
            before_revision: StateRevision(0),
            after_revision: StateRevision(1),
            before_digest: mtgml_model::FullStateDigestV7::from_digest_bytes([0; 32]),
            after_digest: mtgml_model::FullStateDigestV7::from_digest_bytes([1; 32]),
            replacement: before,
            operations,
        };

        assert_eq!(
            validate_event_delta_parity_v3(std::slice::from_ref(&event), &delta),
            Ok(())
        );

        let mut fabricated = delta.clone();
        fabricated.operations[1] = SemanticDeltaOperationV3::LandPlayCountChanged {
            player: PlayerId(2),
            from: 0,
            to: 1,
        };
        assert_eq!(
            validate_event_delta_parity_v3(std::slice::from_ref(&event), &fabricated),
            Err(EventDeltaV3Error::Mismatch)
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
        let operation = SemanticDeltaOperationV3::ManaPoolChanged {
            player,
            from: old_pool,
            to: new_pool,
            cause: ManaPoolChangeCauseV1::Produced,
        };
        let delta = StateDeltaV3::between(&before, &after, vec![operation]).unwrap();
        let event = AuthoritativeRuleEventV3 {
            event_id: RuleEventId(1),
            state_revision: after.revision,
            event: AuthoritativeRuleEventKindV3::ManaPoolChanged {
                player,
                before: old_pool,
                after: new_pool,
                cause: ManaPoolChangeCauseV1::Produced,
            },
        };
        validate_event_delta_state_v3(&before, &after, &[event], &delta).unwrap();

        let mut mismatched = after.clone();
        mismatched
            .card_rules
            .mana
            .pools
            .insert(player, ManaPoolV1::default());
        assert_eq!(
            validate_event_delta_state_v3(&before, &mismatched, &[], &delta),
            Err(EventDeltaV3Error::Mismatch)
        );

        let wrong_operation = SemanticDeltaOperationV3::ManaPoolChanged {
            player,
            from: old_pool,
            to: ManaPoolV1::default(),
            cause: ManaPoolChangeCauseV1::Produced,
        };
        assert!(StateDeltaV3::between(&before, &after, vec![wrong_operation]).is_err());
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
        let cost_event = AuthoritativeRuleEventKindV3::CostCommitted {
            actor: PlayerId(1),
            action: CostCommitActionV1::NonManaActivation {
                source_object: ability_source_object,
                source_ability: mtgml_model::AbilityInstanceId(1),
            },
            facts,
            source_activations,
            spent_buckets: [0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0],
        };
        let tap_event = AuthoritativeRuleEventKindV3::ObjectTapped {
            object: GameObjectId(1),
            from: false,
            to: true,
        };
        let ability_event = AuthoritativeRuleEventKindV3::AbilityActivated {
            stack_object: StackObjectId(1),
            source: source_context,
            targets: vec![],
            cost_facts: CostFacts::default(),
            once_per_turn_use_committed: false,
        };
        let stack_event = AuthoritativeRuleEventKindV3::StackItemAdded {
            stack_object: StackObjectId(1),
            payload: after.zones.stack_records[&StackObjectId(1)]
                .payload
                .clone()
                .unwrap(),
        };
        let (events, next_event_id) = allocate_rule_events_v3(
            before.allocators.next_rule_event_id,
            after.revision,
            [cost_event, tap_event, ability_event, stack_event],
        )
        .unwrap();
        after.allocators.next_rule_event_id = next_event_id;
        after.validate().unwrap();
        let mut operations = events
            .iter()
            .flat_map(AuthoritativeRuleEventV3::semantic_operations)
            .collect::<Vec<_>>();
        operations.push(SemanticDeltaOperationV3::StackOrderChanged {
            from: vec![],
            to: vec![StackObjectId(1)],
        });
        let delta = StateDeltaV3::between(&before, &after, operations).unwrap();
        validate_event_delta_state_v3(&before, &after, &events, &delta).unwrap();

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
            if let AuthoritativeRuleEventKindV3::AbilityActivated {
                once_per_turn_use_committed,
                ..
            } = &mut event.event
            {
                *once_per_turn_use_committed = true;
            }
        }
        let once_operations = once_events
            .iter()
            .flat_map(AuthoritativeRuleEventV3::semantic_operations)
            .chain([SemanticDeltaOperationV3::StackOrderChanged {
                from: vec![],
                to: vec![StackObjectId(1)],
            }])
            .collect();
        let once_delta = StateDeltaV3::between(&before, &once_after, once_operations).unwrap();
        validate_event_delta_state_v3(&before, &once_after, &once_events, &once_delta).unwrap();

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
            .flat_map(AuthoritativeRuleEventV3::semantic_operations)
            .chain([SemanticDeltaOperationV3::StackOrderChanged {
                from: vec![],
                to: vec![StackObjectId(1)],
            }])
            .collect();
        assert!(
            StateDeltaV3::between(&before, &mismatched_once_after, mismatched_once_operations)
                .is_err()
        );

        let mut duplicate_receipt_operations = once_events
            .iter()
            .flat_map(AuthoritativeRuleEventV3::semantic_operations)
            .collect::<Vec<_>>();
        let once_operation = duplicate_receipt_operations
            .iter()
            .find(|operation| {
                matches!(
                    operation,
                    SemanticDeltaOperationV3::AbilityActivated {
                        once_per_turn_use_committed: true,
                        ..
                    }
                )
            })
            .unwrap()
            .clone();
        duplicate_receipt_operations.push(once_operation);
        duplicate_receipt_operations.push(SemanticDeltaOperationV3::StackOrderChanged {
            from: vec![],
            to: vec![StackObjectId(1)],
        });
        assert!(StateDeltaV3::between(&before, &once_after, duplicate_receipt_operations).is_err());

        let false_receipt_operations = events
            .iter()
            .flat_map(AuthoritativeRuleEventV3::semantic_operations)
            .chain([SemanticDeltaOperationV3::StackOrderChanged {
                from: vec![],
                to: vec![StackObjectId(1)],
            }])
            .collect();
        assert!(StateDeltaV3::between(&before, &once_after, false_receipt_operations).is_err());

        let mut missing_history_receipt = events.clone();
        for event in &mut missing_history_receipt {
            if let AuthoritativeRuleEventKindV3::AbilityActivated {
                once_per_turn_use_committed,
                ..
            } = &mut event.event
            {
                *once_per_turn_use_committed = true;
            }
        }
        let missing_history_operations = missing_history_receipt
            .iter()
            .flat_map(AuthoritativeRuleEventV3::semantic_operations)
            .chain([SemanticDeltaOperationV3::StackOrderChanged {
                from: vec![],
                to: vec![StackObjectId(1)],
            }])
            .collect();
        assert!(StateDeltaV3::between(&before, &after, missing_history_operations).is_err());

        let mut wrong_events = events.clone();
        if let AuthoritativeRuleEventKindV3::CostCommitted { spent_buckets, .. } =
            &mut wrong_events[0].event
        {
            spent_buckets[3] = 2;
        }
        let wrong_operations = wrong_events
            .iter()
            .flat_map(AuthoritativeRuleEventV3::semantic_operations)
            .chain([SemanticDeltaOperationV3::StackOrderChanged {
                from: vec![],
                to: vec![StackObjectId(1)],
            }])
            .collect();
        let wrong_delta = StateDeltaV3::between(&before, &after, wrong_operations).unwrap();
        assert_eq!(
            validate_event_delta_state_v3(&before, &after, &wrong_events, &wrong_delta),
            Err(EventDeltaV3Error::Mismatch)
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
            AuthoritativeRuleEventKindV3::DamageApplied {
                source: None,
                recipient: DamageRecipient::Player(player),
                post_replacement_amount: 3,
                damage_kind: DamageKind::Noncombat,
            },
            AuthoritativeRuleEventKindV3::LifeChanged {
                player,
                from: life_before,
                to: life_before - 3,
            },
        ];
        let (events, next) =
            allocate_rule_events_v3(before.allocators.next_rule_event_id, after.revision, kinds)
                .unwrap();
        after.allocators.next_rule_event_id = next;
        let delta = StateDeltaV3::between(
            &before,
            &after,
            events
                .iter()
                .flat_map(AuthoritativeRuleEventV3::semantic_operations)
                .collect(),
        )
        .unwrap();
        validate_event_delta_state_v3(&before, &after, &events, &delta).unwrap();

        let mut wrong_events = events.clone();
        if let AuthoritativeRuleEventKindV3::DamageApplied {
            post_replacement_amount,
            ..
        } = &mut wrong_events[0].event
        {
            *post_replacement_amount = 4;
        }
        let wrong_delta = StateDeltaV3::between(
            &before,
            &after,
            wrong_events
                .iter()
                .flat_map(AuthoritativeRuleEventV3::semantic_operations)
                .collect(),
        )
        .unwrap();
        assert_eq!(
            validate_event_delta_state_v3(&before, &after, &wrong_events, &wrong_delta),
            Err(EventDeltaV3Error::Mismatch)
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
        let event = AuthoritativeRuleEventV3 {
            event_id: RuleEventId(1),
            state_revision: after.revision,
            event: AuthoritativeRuleEventKindV3::CounterChanged {
                object,
                kind: CounterKindV1::PlusOnePlusOne,
                before: 0,
                after: 1,
            },
        };
        let operations = event.semantic_operations();
        assert!(matches!(
            &operations[0],
            SemanticDeltaOperationV3::CounterChanged { cause, .. }
                if *cause == event.event_id
        ));
        let delta = StateDeltaV3::between(&before, &after, operations).unwrap();
        validate_event_delta_state_v3(&before, &after, &[event], &delta).unwrap();
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
            AuthoritativeRuleEventV3 {
                event_id: RuleEventId(1),
                state_revision: after.revision,
                event: AuthoritativeRuleEventKindV3::TemporaryEffectCreated {
                    effect: effect.clone(),
                },
            },
            AuthoritativeRuleEventV3 {
                event_id: RuleEventId(2),
                state_revision: after.revision,
                event: AuthoritativeRuleEventKindV3::PerspectiveObservationOccurrence {
                    lifecycle: Box::new(lifecycle),
                    source_event_id: RuleEventId(1),
                },
            },
        ];
        let operations = events
            .iter()
            .flat_map(AuthoritativeRuleEventV3::semantic_operations)
            .collect();
        let delta = StateDeltaV3::between(&before, &after, operations).unwrap();
        validate_event_delta_state_v3(&before, &after, &events, &delta).unwrap();

        let mut occurrence = events[1].clone();
        let mut source = events[0].clone();
        let AuthoritativeRuleEventKindV3::PerspectiveObservationOccurrence {
            source_event_id, ..
        } = &mut occurrence.event
        else {
            unreachable!()
        };
        *source_event_id = RuleEventId(2);
        occurrence.event_id = RuleEventId(1);
        source.event_id = RuleEventId(2);
        let forward_reference = vec![occurrence, source];
        let forward_delta = StateDeltaV3::between(
            &before,
            &after,
            forward_reference
                .iter()
                .flat_map(AuthoritativeRuleEventV3::semantic_operations)
                .collect(),
        )
        .unwrap();
        assert_eq!(
            validate_event_delta_state_v3(&before, &after, &forward_reference, &forward_delta),
            Err(EventDeltaV3Error::Mismatch)
        );

        let mut forged = events.clone();
        let AuthoritativeRuleEventKindV3::PerspectiveObservationOccurrence {
            source_event_id, ..
        } = &mut forged[1].event
        else {
            unreachable!()
        };
        *source_event_id = RuleEventId(99);
        let forged_delta = StateDeltaV3::between(
            &before,
            &after,
            forged
                .iter()
                .flat_map(AuthoritativeRuleEventV3::semantic_operations)
                .collect(),
        )
        .unwrap();
        assert_eq!(
            validate_event_delta_state_v3(&before, &after, &forged, &forged_delta),
            Err(EventDeltaV3Error::Mismatch)
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
            AuthoritativeRuleEventV3 {
                event_id: RuleEventId(1),
                state_revision: after.revision,
                event: AuthoritativeRuleEventKindV3::TemporaryEffectCreated { effect },
            },
            AuthoritativeRuleEventV3 {
                event_id: RuleEventId(2),
                state_revision: after.revision,
                event: AuthoritativeRuleEventKindV3::PerspectiveObservationOccurrence {
                    lifecycle: Box::new(lifecycle),
                    source_event_id: RuleEventId(1),
                },
            },
        ];
        let delta = StateDeltaV3 {
            before_revision: before.revision,
            after_revision: after.revision,
            before_digest: mtgml_model::FullStateDigestV7::from_digest_bytes([0; 32]),
            after_digest: mtgml_model::FullStateDigestV7::from_digest_bytes([1; 32]),
            replacement: after.clone(),
            operations: Vec::new(),
        };
        assert!(
            validate_observation_occurrence_lifecycle(&before, &after, &events, &delta).is_ok()
        );

        let mut bad_lifecycle = events.clone();
        let AuthoritativeRuleEventKindV3::PerspectiveObservationOccurrence { lifecycle, .. } =
            &mut bad_lifecycle[1].event
        else {
            unreachable!()
        };
        lifecycle.sequence.0 += 1;
        assert_eq!(
            validate_observation_occurrence_lifecycle(&before, &after, &bad_lifecycle, &delta),
            Err(EventDeltaV3Error::Mismatch)
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
