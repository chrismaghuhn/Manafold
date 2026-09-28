//! Detached G0 authoritative event identity and sequential cursor.
//!
//! These are transition products, not a durable log and not a replay control
//! stream. G0j alone connects a successor producer to the current runtime.

use crate::events::AuthoritativeRuleEventKind;
use mtgml_model::{
    GameObjectId, PlayerId, RuleEventId, StackObjectId, StateRevision, TriggerInstanceId,
};
use mtgml_state::{
    ActionCostFacts, CostCommitActionV1, CostFacts, DamageKind, DamageRecipient,
    EngineStatePartsV3, ManaPoolChangeCauseV1, ManaPoolV1, ManaSourceActivation,
    PendingTriggerRecord, SemanticDeltaOperationV2, SemanticDeltaOperationV3, SourceContext,
    StackItemEndKindV1, StackItemPayload, StateDeltaV3, TargetBinding, TemporaryEffectRecord,
};

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
    Existing {
        event: Box<AuthoritativeRuleEventKind>,
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
            Self::Existing { event } => vec![SemanticDeltaOperationV3::Existing {
                operation: Box::new(SemanticDeltaOperationV2::Existing {
                    operation: Box::new(event.semantic_delta()),
                }),
            }],
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
            } => vec![SemanticDeltaOperationV3::AbilityActivated {
                stack_object: *stack_object,
                source: Box::new(source.clone()),
                targets: targets.clone(),
                cost_facts: cost_facts.clone(),
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
    let expected = events
        .iter()
        .flat_map(AuthoritativeRuleEventV3::semantic_operations)
        .collect::<Vec<_>>();
    let mut expected_index = 0;
    for operation in &delta.operations {
        if matches!(
            operation,
            SemanticDeltaOperationV3::ContinuationChanged { .. }
                | SemanticDeltaOperationV3::PendingRequestChanged { .. }
                | SemanticDeltaOperationV3::StackOrderChanged { .. }
        ) {
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

/// Applies the successor delta and checks event projections against the same
/// before/after state. Legacy event kinds keep their existing validator.
pub fn validate_event_delta_state_v3(
    before: &EngineStatePartsV3,
    after: &EngineStatePartsV3,
    events: &[AuthoritativeRuleEventV3],
    delta: &StateDeltaV3,
) -> Result<(), EventDeltaV3Error> {
    let applied = delta
        .apply(before)
        .map_err(|_| EventDeltaV3Error::Mismatch)?;
    if &applied != after {
        return Err(EventDeltaV3Error::Mismatch);
    }
    validate_rule_event_cursor_v3(
        before.predecessor_v5.allocators.next_rule_event_id,
        after.predecessor_v5.allocators.next_rule_event_id,
        after.predecessor_v5.revision,
        events,
    )
    .map_err(|_| EventDeltaV3Error::Mismatch)?;
    validate_event_delta_parity_v3(events, delta)?;
    let before_stack_order = &before.predecessor_v5.zones.stack_order;
    let after_stack_order = &after.predecessor_v5.zones.stack_order;
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

fn validate_damage_state_projection_v3(
    before: &EngineStatePartsV3,
    after: &EngineStatePartsV3,
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
        let Some(before_player) = before.predecessor_v5.core.players.get(&player) else {
            return Err(EventDeltaV3Error::Mismatch);
        };
        let Some(after_player) = after.predecessor_v5.core.players.get(&player) else {
            return Err(EventDeltaV3Error::Mismatch);
        };
        let actual_loss = i128::from(before_player.life) - i128::from(after_player.life);
        if actual_loss != i128::from(amount) {
            return Err(EventDeltaV3Error::Mismatch);
        }
    }
    for (object, amount) in object_damage {
        let Some(before_source) = before.predecessor_v5.foundation_sources.get(&object) else {
            return Err(EventDeltaV3Error::Mismatch);
        };
        if let Some(after_source) = after.predecessor_v5.foundation_sources.get(&object) {
            let actual = after_source
                .marked_damage
                .checked_sub(before_source.marked_damage)
                .ok_or(EventDeltaV3Error::Mismatch)?;
            if actual != amount {
                return Err(EventDeltaV3Error::Mismatch);
            }
        } else if !events.iter().any(|event| match &event.event {
            AuthoritativeRuleEventKindV3::Existing { event } => match event.as_ref() {
                AuthoritativeRuleEventKind::ObjectCeasedToExist { object: ceased } => {
                    *ceased == object
                }
                AuthoritativeRuleEventKind::ZoneTransition { transition } => {
                    transition.old_object == object
                }
                _ => false,
            },
            _ => false,
        }) {
            return Err(EventDeltaV3Error::Mismatch);
        }
    }
    Ok(())
}

fn validate_delta_operation_projection_v3(
    before: &EngineStatePartsV3,
    after: &EngineStatePartsV3,
    operation: &SemanticDeltaOperationV3,
) -> Result<(), EventDeltaV3Error> {
    let valid = match operation {
        SemanticDeltaOperationV3::Existing { .. }
        | SemanticDeltaOperationV3::DamageApplied { .. } => true,
        SemanticDeltaOperationV3::StackOrderChanged { from, to } => {
            &before.predecessor_v5.zones.stack_order == from
                && &after.predecessor_v5.zones.stack_order == to
        }
        SemanticDeltaOperationV3::StackItemCreated {
            stack_object,
            payload,
        } => {
            !before
                .predecessor_v5
                .zones
                .stack_records
                .contains_key(stack_object)
                && after
                    .predecessor_v5
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
                .predecessor_v5
                .zones
                .stack_records
                .get(stack_object)
                .and_then(|record| record.payload.as_ref())
                == Some(payload.as_ref())
                && !after
                    .predecessor_v5
                    .zones
                    .stack_records
                    .contains_key(stack_object)
        }
        SemanticDeltaOperationV3::SpellCast {
            stack_object,
            spell_object,
            card_definition,
            face_key,
            semantic_profile_id,
            cost_facts,
            ..
        } => after
            .predecessor_v5
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
        } => after
            .predecessor_v5
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
            }),
        SemanticDeltaOperationV3::TargetDeclared {
            source_stack_item,
            targets,
        } => after
            .predecessor_v5
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
                .card_rules_state
                .counters
                .counters
                .get(object)
                .and_then(|counters| counters.get(kind))
                .copied()
                .unwrap_or(0);
            let after_count = after
                .card_rules_state
                .counters
                .counters
                .get(object)
                .and_then(|counters| counters.get(kind))
                .copied()
                .unwrap_or(0);
            before_count == *from && after_count == *to && from != to
        }
        SemanticDeltaOperationV3::TriggerCreated { trigger } => {
            !before
                .execution_v4
                .waiting_triggers
                .contains_key(&trigger.id)
                && after.execution_v4.waiting_triggers.get(&trigger.id) == Some(trigger.as_ref())
        }
        SemanticDeltaOperationV3::TriggerPlaced {
            trigger,
            stack_object,
            payload,
        } => {
            before.execution_v4.waiting_triggers.contains_key(trigger)
                && !after.execution_v4.waiting_triggers.contains_key(trigger)
                && after
                    .predecessor_v5
                    .zones
                    .stack_records
                    .get(stack_object)
                    .and_then(|record| record.payload.as_ref())
                    == Some(payload.as_ref())
        }
        SemanticDeltaOperationV3::ManaPoolChanged { player, from, to, .. } => {
            before.card_rules_state.mana.pools.get(player) == Some(from)
                && after.card_rules_state.mana.pools.get(player) == Some(to)
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
                .execution_v4
                .continuations
                .get(continuation)
                .map(|record| &record.payload)
                == from.as_deref()
                && after
                    .execution_v4
                    .continuations
                    .get(continuation)
                    .map(|record| &record.payload)
                    == to.as_deref()
        }
        SemanticDeltaOperationV3::PendingRequestChanged { from, to } => {
            before.execution_v4.pending_decision.as_ref() == from.as_deref()
                && after.execution_v4.pending_decision.as_ref() == to.as_deref()
        }
        SemanticDeltaOperationV3::TemporaryEffectChanged { effect, from, to } => {
            before.execution_v4.effects.get(effect) == from.as_deref()
                && after.execution_v4.effects.get(effect) == to.as_deref()
        }
    };
    if valid {
        Ok(())
    } else {
        Err(EventDeltaV3Error::Mismatch)
    }
}

fn validate_event_projection_v3(
    before: &EngineStatePartsV3,
    after: &EngineStatePartsV3,
    event: &AuthoritativeRuleEventKindV3,
) -> Result<(), EventDeltaV3Error> {
    let valid = match event {
        AuthoritativeRuleEventKindV3::Existing { event } => {
            validate_legacy_event_projection_v3(before, after, event)
        }
        AuthoritativeRuleEventKindV3::DamageApplied { .. } => true,
        AuthoritativeRuleEventKindV3::StackItemAdded {
            stack_object,
            payload,
        } => {
            !before
                .predecessor_v5
                .zones
                .stack_records
                .contains_key(stack_object)
                && after
                    .predecessor_v5
                    .zones
                    .stack_records
                    .get(stack_object)
                    .is_some_and(|record| record.payload.as_ref() == Some(payload))
                && after
                    .predecessor_v5
                    .zones
                    .stack_order
                    .contains(stack_object)
        }
        AuthoritativeRuleEventKindV3::StackItemRemoved {
            stack_object,
            payload,
            ..
        } => {
            before
                .predecessor_v5
                .zones
                .stack_records
                .get(stack_object)
                .is_some_and(|record| record.payload.as_ref() == Some(payload))
                && !after
                    .predecessor_v5
                    .zones
                    .stack_records
                    .contains_key(stack_object)
                && !after
                    .predecessor_v5
                    .zones
                    .stack_order
                    .contains(stack_object)
        }
        AuthoritativeRuleEventKindV3::SpellCast {
            stack_object,
            spell_object,
            card_definition,
            face_key,
            semantic_profile_id,
            cost_facts,
            ..
        } => after
            .predecessor_v5
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
            }),
        AuthoritativeRuleEventKindV3::AbilityActivated {
            stack_object,
            source,
            targets,
            cost_facts,
        } => after
            .predecessor_v5
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
            }),
        AuthoritativeRuleEventKindV3::TargetDeclared {
            source_stack_item,
            targets,
        } => after
            .predecessor_v5
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
                .card_rules_state
                .counters
                .counters
                .get(object)
                .and_then(|counters| counters.get(kind))
                .copied()
                .unwrap_or(0);
            let new = after
                .card_rules_state
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
                .execution_v4
                .waiting_triggers
                .get(&trigger.id)
                .is_some_and(|record| record == trigger);
            let placed = after
                .predecessor_v5
                .zones
                .stack_records
                .values()
                .any(|record| {
                    matches!(record.payload.as_ref(), Some(StackItemPayload::TriggeredAbility {
                        originating_trigger,
                        source_context,
                        captured_trigger_context,
                        ..
                    }) if *originating_trigger == trigger.id
                        && source_context == &trigger.source_context
                        && captured_trigger_context.as_ref() == &trigger.trigger_context)
                });
            !before
                .execution_v4
                .waiting_triggers
                .contains_key(&trigger.id)
                && (pending || placed)
        }
        AuthoritativeRuleEventKindV3::TriggerPlaced {
            trigger,
            stack_object,
            payload,
        } => {
            before.execution_v4.waiting_triggers.contains_key(trigger)
                && !after.execution_v4.waiting_triggers.contains_key(trigger)
                && after
                    .predecessor_v5
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
            before.card_rules_state.mana.pools.get(player) == Some(old_pool)
                && after.card_rules_state.mana.pools.get(player) == Some(new_pool)
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
            !before.execution_v4.effects.contains_key(&effect.id)
                && after.execution_v4.effects.get(&effect.id) == Some(effect)
        }
        AuthoritativeRuleEventKindV3::TemporaryEffectExpired { effect } => {
            before.execution_v4.effects.get(&effect.id) == Some(effect)
                && !after.execution_v4.effects.contains_key(&effect.id)
        }
    };
    if valid {
        Ok(())
    } else {
        Err(EventDeltaV3Error::Mismatch)
    }
}

fn validate_legacy_event_projection_v3(
    before: &EngineStatePartsV3,
    after: &EngineStatePartsV3,
    event: &AuthoritativeRuleEventKind,
) -> bool {
    match event {
        AuthoritativeRuleEventKind::ZoneTransition { transition } => {
            object_snapshot_matches(&before.predecessor_v5, &transition.last_known)
                && !after
                    .predecessor_v5
                    .zones
                    .objects
                    .contains_key(&transition.old_object)
                && object_snapshot_matches(&after.predecessor_v5, &transition.new_snapshot)
                && after
                    .predecessor_v5
                    .zones
                    .locations
                    .get(&transition.new_object)
                    == Some(&transition.to)
        }
        AuthoritativeRuleEventKind::ObjectCeasedToExist { object } => {
            before.predecessor_v5.zones.objects.contains_key(object)
                && !after.predecessor_v5.zones.objects.contains_key(object)
        }
        AuthoritativeRuleEventKind::LifeChanged { player, from, to } => {
            from != to
                && before
                    .predecessor_v5
                    .core
                    .players
                    .get(player)
                    .is_some_and(|state| state.life == *from)
                && after
                    .predecessor_v5
                    .core
                    .players
                    .get(player)
                    .is_some_and(|state| state.life == *to)
        }
        AuthoritativeRuleEventKind::MarkedDamageChanged { creature, from, to } => {
            from != to
                && before
                    .predecessor_v5
                    .foundation_sources
                    .get(creature)
                    .is_some_and(|source| source.marked_damage == *from)
                && after
                    .predecessor_v5
                    .foundation_sources
                    .get(creature)
                    .is_some_and(|source| source.marked_damage == *to)
        }
        AuthoritativeRuleEventKind::ObjectTapped { object, from, to } => {
            from != to
                && before
                    .predecessor_v5
                    .zones
                    .objects
                    .get(object)
                    .is_some_and(|state| state.tapped == *from)
                && after
                    .predecessor_v5
                    .zones
                    .objects
                    .get(object)
                    .is_some_and(|state| state.tapped == *to)
        }
        AuthoritativeRuleEventKind::DecisionCreated { decision } => {
            before
                .execution_v4
                .pending_decision
                .as_ref()
                .is_none_or(|request| request.decision_id != *decision)
                && after
                    .execution_v4
                    .pending_decision
                    .as_ref()
                    .is_some_and(|request| request.decision_id == *decision)
        }
        AuthoritativeRuleEventKind::DecisionCleared { decision } => {
            before
                .execution_v4
                .pending_decision
                .as_ref()
                .is_some_and(|request| request.decision_id == *decision)
                && after
                    .execution_v4
                    .pending_decision
                    .as_ref()
                    .is_none_or(|request| request.decision_id != *decision)
        }
        AuthoritativeRuleEventKind::PriorityChanged { from, to } => {
            from != to
                && before.predecessor_v5.core.priority == *from
                && after.predecessor_v5.core.priority == *to
        }
        AuthoritativeRuleEventKind::TurnPositionChanged { from, to } => {
            from != to
                && before.predecessor_v5.core.position == *from
                && after.predecessor_v5.core.position == *to
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
                    .predecessor_v5
                    .random
                    .streams
                    .get(stream)
                    .is_some_and(|cursor| cursor.next_raw_u64 == *cursor_before)
                && after
                    .predecessor_v5
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
            .execution_v4
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
        AuthoritativeRuleEventKind::StateBasedActionsApplied { actions } => {
            actions.iter().all(|action| match action {
                mtgml_state::SbaSelectedActionV1::PlayerLoses { player } => after
                    .predecessor_v5
                    .core
                    .players
                    .get(player)
                    .is_some_and(|state| state.has_lost),
                mtgml_state::SbaSelectedActionV1::ObjectToOwnerGraveyard { object, .. } => {
                    !after.predecessor_v5.zones.objects.contains_key(object)
                }
            })
        }
        AuthoritativeRuleEventKind::CombatDamageStepCompleted => after
            .predecessor_v5
            .combat
            .as_ref()
            .is_some_and(|combat| combat.damage_step_completed),
        AuthoritativeRuleEventKind::AttackersDeclared {
            defending_player,
            attackers,
        } => after.predecessor_v5.combat.as_ref().is_some_and(|combat| {
            combat.defending_player == *defending_player && combat.attackers == *attackers
        }),
        AuthoritativeRuleEventKind::CombatEnded => {
            before.predecessor_v5.combat.is_some() && after.predecessor_v5.combat.is_none()
        }
        AuthoritativeRuleEventKind::EmptyCombatStepsSkipped => {
            before.predecessor_v5.core.position != after.predecessor_v5.core.position
        }
        AuthoritativeRuleEventKind::UntapCompleted { affected_objects } => {
            affected_objects.iter().all(|object| {
                after
                    .predecessor_v5
                    .zones
                    .objects
                    .get(object)
                    .is_some_and(|state| !state.tapped)
            })
        }
        AuthoritativeRuleEventKind::ActivePlayerChanged { from, to } => {
            from != to
                && before.predecessor_v5.core.active_player == *from
                && after.predecessor_v5.core.active_player == *to
        }
        AuthoritativeRuleEventKind::TurnNumberChanged { from, to } => {
            from != to
                && before.predecessor_v5.core.turn_number == *from
                && after.predecessor_v5.core.turn_number == *to
        }
        AuthoritativeRuleEventKind::PerspectiveOccurrence { lifecycle, .. } => {
            let mut projected: mtgml_state::EngineState = before.predecessor_v5.clone().into();
            mtgml_state::apply_perspective_lifecycle(&mut projected, lifecycle).is_ok()
                && projected.knowledge == after.predecessor_v5.knowledge
                && projected.perspective_identities == after.predecessor_v5.perspective_identities
        }
        // Combat assignment/blocked-state state projection remains closed
        // until its exact legal relation is characterized and accepted.
        AuthoritativeRuleEventKind::CombatDamageDealt { .. }
        | AuthoritativeRuleEventKind::BlockersDeclared { .. } => false,
    }
}

fn object_snapshot_matches(
    state: &mtgml_state::EngineStateParts,
    snapshot: &mtgml_state::ObjectSnapshot,
) -> bool {
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
    before: &EngineStatePartsV3,
    after: &EngineStatePartsV3,
    actor: PlayerId,
    action: CostCommitActionV1,
    facts: &ActionCostFacts,
    source_activations: &[ManaSourceActivation],
    spent_buckets: &[u32; 12],
) -> bool {
    let old_pool = match before.card_rules_state.mana.pools.get(&actor) {
        Some(pool) => *pool,
        None => return false,
    };
    let action_projects = match action {
        CostCommitActionV1::Cast {
            stack_object,
            spell_object,
        } => after
            .predecessor_v5
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
        } => after
            .predecessor_v5
            .zones
            .stack_records
            .values()
            .any(|record| {
                matches!(record.payload.as_ref(), Some(StackItemPayload::ActivatedAbility {
                    source_context,
                    ..
                }) if source_context.source.snapshot.object == source_object
                    && source_context.ability_instance_id == source_ability)
            }),
        CostCommitActionV1::StackResolution { stack_object } => {
            before
                .predecessor_v5
                .zones
                .stack_records
                .contains_key(&stack_object)
                && !after
                    .predecessor_v5
                    .zones
                    .stack_records
                    .contains_key(&stack_object)
                && !after
                    .predecessor_v5
                    .zones
                    .stack_order
                    .contains(&stack_object)
        }
    };
    if !action_projects {
        return false;
    }
    let mut pool = old_pool;

    let staged = match action {
        CostCommitActionV1::Cast { spell_object, .. } => before
            .execution_v4
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
            .execution_v4
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
            .execution_v4
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
        let Some(source_object) = before
            .predecessor_v5
            .zones
            .objects
            .get(&source.source_object)
        else {
            return false;
        };
        let Some(source_location) = before
            .predecessor_v5
            .zones
            .locations
            .get(&source.source_object)
        else {
            return false;
        };
        if source_object.tapped || source_location.zone != mtgml_model::ZoneKind::Battlefield {
            return false;
        }
        let after_source = after
            .predecessor_v5
            .zones
            .objects
            .get(&source.source_object);
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
    if after.card_rules_state.mana.pools.get(&actor) != Some(&pool) {
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
        let Some(before_object) = before.predecessor_v5.zones.objects.get(&source) else {
            return false;
        };
        if before_object.tapped {
            return false;
        }
        if sacrifices_source {
            if after.predecessor_v5.zones.objects.contains_key(&source) {
                return false;
            }
        } else if taps_source
            && !after
                .predecessor_v5
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
        construct_synthetic_engine_state, AbilityAuthorityV1, ActionCostFacts,
        CardRulesAuthoritativeStateV1, CounterKindV1, EngineStatePartsV3, ExecutionStateV4,
        ManaCost, ManaPoolV1, ManaSourceActivation, ManaSourceActivationCost,
        SemanticDeltaOperationV3, StackRecord, StateDeltaV3, SyntheticResetInputs,
        SyntheticV4Setup, VisibilityPartition, ZoneLocation, ZonePosition,
    };

    fn state() -> EngineStatePartsV3 {
        let mut predecessor = construct_synthetic_engine_state(SyntheticResetInputs {
            players: [PlayerId(1), PlayerId(2)],
            root_seed: RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap(),
            setup: SyntheticV4Setup::synthetic_compatibility(),
        })
        .unwrap();
        predecessor.execution = Default::default();
        let mut card_rules = CardRulesAuthoritativeStateV1::default();
        for player in predecessor.core.players.keys().copied() {
            card_rules.mana.pools.insert(player, Default::default());
            card_rules
                .turn_history
                .players
                .insert(player, Default::default());
        }
        card_rules.turn_history.turn_number = predecessor.core.turn_number;
        EngineStatePartsV3::new(predecessor.parts(), ExecutionStateV4::default(), card_rules)
            .unwrap()
    }

    #[test]
    fn cursor_assigns_and_validates_one_sequential_semantic_vector() {
        let (events, next) = allocate_rule_events_v3(
            RuleEventId(8),
            StateRevision(4),
            [
                AuthoritativeRuleEventKindV3::Existing {
                    event: Box::new(AuthoritativeRuleEventKind::CombatEnded),
                },
                AuthoritativeRuleEventKindV3::Existing {
                    event: Box::new(AuthoritativeRuleEventKind::EmptyCombatStepsSkipped),
                },
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
                [AuthoritativeRuleEventKindV3::Existing {
                    event: Box::new(AuthoritativeRuleEventKind::CombatEnded),
                }],
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
        let state = state();
        let event = AuthoritativeRuleEventV3 {
            event_id: RuleEventId(1),
            state_revision: state.predecessor_v5.revision,
            event: AuthoritativeRuleEventKindV3::Existing {
                event: Box::new(AuthoritativeRuleEventKind::CombatEnded),
            },
        };
        let operation = event
            .event
            .semantic_operations()
            .into_iter()
            .next()
            .unwrap();
        let delta = mtgml_state::StateDeltaV3::between(&state, &state, vec![operation]).unwrap();
        validate_event_delta_parity_v3(std::slice::from_ref(&event), &delta).unwrap();

        let missing = mtgml_state::StateDeltaV3::between(&state, &state, vec![]).unwrap();
        assert_eq!(
            validate_event_delta_parity_v3(&[event], &missing),
            Err(EventDeltaV3Error::Mismatch)
        );
    }

    #[test]
    fn event_delta_and_after_state_projection_must_agree() {
        let before = state();
        let player = PlayerId(1);
        let old_pool = before.card_rules_state.mana.pools[&player];
        let mut new_pool = old_pool;
        new_pool.unrestricted[4] = 1;
        let mut after = before.clone();
        after.card_rules_state.mana.pools.insert(player, new_pool);
        after.predecessor_v5.revision = StateRevision(before.predecessor_v5.revision.0 + 1);
        after.predecessor_v5.allocators.next_rule_event_id = RuleEventId(2);
        let operation = SemanticDeltaOperationV3::ManaPoolChanged {
            player,
            from: old_pool,
            to: new_pool,
            cause: ManaPoolChangeCauseV1::Produced,
        };
        let delta = StateDeltaV3::between(&before, &after, vec![operation]).unwrap();
        let event = AuthoritativeRuleEventV3 {
            event_id: RuleEventId(1),
            state_revision: after.predecessor_v5.revision,
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
            .card_rules_state
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
        let wrong_delta = StateDeltaV3::between(&before, &after, vec![wrong_operation]).unwrap();
        let wrong_event = AuthoritativeRuleEventV3 {
            event_id: RuleEventId(1),
            state_revision: after.predecessor_v5.revision,
            event: AuthoritativeRuleEventKindV3::ManaPoolChanged {
                player,
                before: old_pool,
                after: ManaPoolV1::default(),
                cause: ManaPoolChangeCauseV1::Produced,
            },
        };
        assert_eq!(
            validate_event_delta_state_v3(&before, &after, &[wrong_event], &wrong_delta),
            Err(EventDeltaV3Error::Mismatch)
        );
    }

    #[test]
    fn atomic_cost_commit_projects_the_staged_source_output_payment_and_stack_item() {
        let mut before = state();
        let ability_source_object = GameObjectId(3);
        before.predecessor_v5.zones.objects.insert(
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
        before.predecessor_v5.zones.locations.insert(
            ability_source_object,
            ZoneLocation {
                zone: mtgml_model::ZoneKind::Battlefield,
                player: None,
                position: ZonePosition::Unordered,
                visibility: VisibilityPartition::Public,
                partition: None,
            },
        );
        before.predecessor_v5.allocators.next_object_id = GameObjectId(4);
        before.predecessor_v5.allocators.next_ability_id = mtgml_model::AbilityInstanceId(3);
        before.card_rules_state.abilities.by_instance.insert(
            mtgml_model::AbilityInstanceId(1),
            AbilityAuthorityV1 {
                source: ability_source_object,
                ability_key: 4,
            },
        );
        before.card_rules_state.abilities.by_instance.insert(
            mtgml_model::AbilityInstanceId(2),
            AbilityAuthorityV1 {
                source: GameObjectId(1),
                ability_key: 9,
            },
        );
        for object in before.predecessor_v5.zones.objects.keys().copied() {
            before.card_rules_state.faces.faces.insert(object, 0);
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
                    location: before.predecessor_v5.zones.locations[&ability_source_object].clone(),
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
        after.predecessor_v5.revision = StateRevision(before.predecessor_v5.revision.0 + 1);
        after.predecessor_v5.allocators.next_stack_object_id = StackObjectId(2);
        after
            .predecessor_v5
            .zones
            .objects
            .get_mut(&GameObjectId(1))
            .unwrap()
            .tapped = true;
        after.predecessor_v5.zones.stack_records.insert(
            StackObjectId(1),
            StackRecord {
                id: StackObjectId(1),
                controller: PlayerId(1),
                source_object: None,
                source_ability: None,
                payload: Some(StackItemPayload::ActivatedAbility {
                    source_context: source_context.clone(),
                    modes: vec![],
                    targets: vec![],
                    cost_facts: CostFacts::default(),
                }),
            },
        );
        after
            .predecessor_v5
            .zones
            .stack_order
            .push(StackObjectId(1));
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
        let tap_event = AuthoritativeRuleEventKindV3::Existing {
            event: Box::new(AuthoritativeRuleEventKind::ObjectTapped {
                object: GameObjectId(1),
                from: false,
                to: true,
            }),
        };
        let ability_event = AuthoritativeRuleEventKindV3::AbilityActivated {
            stack_object: StackObjectId(1),
            source: source_context,
            targets: vec![],
            cost_facts: CostFacts::default(),
        };
        let stack_event = AuthoritativeRuleEventKindV3::StackItemAdded {
            stack_object: StackObjectId(1),
            payload: after.predecessor_v5.zones.stack_records[&StackObjectId(1)]
                .payload
                .clone()
                .unwrap(),
        };
        let (events, next_event_id) = allocate_rule_events_v3(
            before.predecessor_v5.allocators.next_rule_event_id,
            after.predecessor_v5.revision,
            [cost_event, tap_event, ability_event, stack_event],
        )
        .unwrap();
        after.predecessor_v5.allocators.next_rule_event_id = next_event_id;
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
        let life_before = before.predecessor_v5.core.players[&player].life;
        let mut after = before.clone();
        after
            .predecessor_v5
            .core
            .players
            .get_mut(&player)
            .unwrap()
            .life -= 3;
        after.predecessor_v5.revision = StateRevision(before.predecessor_v5.revision.0 + 1);

        let kinds = [
            AuthoritativeRuleEventKindV3::DamageApplied {
                source: None,
                recipient: DamageRecipient::Player(player),
                post_replacement_amount: 3,
                damage_kind: DamageKind::Noncombat,
            },
            AuthoritativeRuleEventKindV3::Existing {
                event: Box::new(AuthoritativeRuleEventKind::LifeChanged {
                    player,
                    from: life_before,
                    to: life_before - 3,
                }),
            },
        ];
        let (events, next) = allocate_rule_events_v3(
            before.predecessor_v5.allocators.next_rule_event_id,
            after.predecessor_v5.revision,
            kinds,
        )
        .unwrap();
        after.predecessor_v5.allocators.next_rule_event_id = next;
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
        after.predecessor_v5.revision = StateRevision(before.predecessor_v5.revision.0 + 1);
        after.predecessor_v5.allocators.next_rule_event_id = RuleEventId(2);
        after
            .card_rules_state
            .counters
            .counters
            .entry(object)
            .or_default()
            .insert(CounterKindV1::PlusOnePlusOne, 1);
        let event = AuthoritativeRuleEventV3 {
            event_id: RuleEventId(1),
            state_revision: after.predecessor_v5.revision,
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
}
