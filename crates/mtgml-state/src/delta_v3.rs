//! Detached full-replacement StateDeltaV3 bound to FullStateDigestV7.

use mtgml_decision::AuthoritativeDecisionRequestV4;
use mtgml_model::{
    ContinuationId, EffectInstanceId, FullStateDigestV7, GameObjectId, PlayerId, StackObjectId,
    StateRevision, TriggerInstanceId,
};

use crate::{
    calculate_full_state_digest_v7, ContinuationPayloadV3, DamageKind, DamageRecipient,
    EngineStatePartsV3, EngineStatePartsV3Error, ManaCost, ManaPoolV1, PendingTriggerRecord,
    ReservedNonManaCost, SelectedCostOperand, SemanticDeltaOperationV2, SourceContext,
    StackItemPayload, TargetRef, TemporaryEffectRecord,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemanticDeltaOperationV3 {
    Existing {
        operation: Box<SemanticDeltaOperationV2>,
    },
    StackOrderChanged {
        from: Vec<StackObjectId>,
        to: Vec<StackObjectId>,
    },
    StackItemCreated {
        stack_object: StackObjectId,
        payload: Box<StackItemPayload>,
    },
    StackItemEnded {
        stack_object: StackObjectId,
        payload: Box<StackItemPayload>,
        result: StackItemEndKindV1,
    },
    SpellCast {
        stack_object: StackObjectId,
        spell_object: GameObjectId,
        card_definition: mtgml_model::CardDefinitionId,
        face_key: mtgml_card_ir::FaceKey,
        semantic_profile_id: mtgml_card_ir::CardSemanticProfileId,
        is_creature_spell: bool,
        cost_facts: crate::CostFacts,
    },
    AbilityActivated {
        stack_object: StackObjectId,
        source: Box<crate::AbilitySourceContext>,
        targets: Vec<crate::TargetBinding>,
        cost_facts: crate::CostFacts,
    },
    TargetDeclared {
        source_stack_item: StackObjectId,
        targets: Vec<crate::TargetBinding>,
    },
    CounterChanged {
        object: GameObjectId,
        kind: crate::CounterKindV1,
        from: u32,
        to: u32,
        cause: mtgml_model::RuleEventId,
    },
    TriggerCreated {
        trigger: Box<PendingTriggerRecord>,
    },
    TriggerPlaced {
        trigger: TriggerInstanceId,
        stack_object: StackObjectId,
        payload: Box<StackItemPayload>,
    },
    ManaPoolChanged {
        player: PlayerId,
        from: ManaPoolV1,
        to: ManaPoolV1,
        cause: ManaPoolChangeCauseV1,
    },
    AtomicCostCommitted {
        actor: PlayerId,
        action: CostCommitActionV1,
        mana_cost: Option<ManaCost>,
        source_activations: Vec<crate::ManaSourceActivation>,
        spent_buckets: [u32; 12],
        reserved_nonmana_costs: Vec<ReservedNonManaCost>,
        selected_cost_operands: Vec<SelectedCostOperand>,
    },
    DamageApplied {
        source: Option<Box<SourceContext>>,
        recipient: DamageRecipient,
        post_replacement_amount: u32,
        damage_kind: DamageKind,
    },
    ContinuationChanged {
        continuation: ContinuationId,
        from: Option<Box<ContinuationPayloadV3>>,
        to: Option<Box<ContinuationPayloadV3>>,
    },
    PendingRequestChanged {
        from: Option<Box<AuthoritativeDecisionRequestV4>>,
        to: Option<Box<AuthoritativeDecisionRequestV4>>,
    },
    TemporaryEffectChanged {
        effect: EffectInstanceId,
        from: Option<Box<TemporaryEffectRecord>>,
        to: Option<Box<TemporaryEffectRecord>>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CostCommitActionV1 {
    Cast {
        stack_object: StackObjectId,
        spell_object: GameObjectId,
    },
    NonManaActivation {
        source_object: GameObjectId,
        source_ability: mtgml_model::AbilityInstanceId,
    },
    StackResolution {
        stack_object: StackObjectId,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackItemEndKindV1 {
    Resolved,
    Countered,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManaPoolChangeCauseV1 {
    Produced,
    Emptied,
    Spent,
}

/// Exact full-state replacement plus an ordered semantic audit trace.
/// Applying operations never reconstructs the state; `replacement` remains
/// the one authoritative after-state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateDeltaV3 {
    pub before_revision: StateRevision,
    pub after_revision: StateRevision,
    pub before_digest: FullStateDigestV7,
    pub after_digest: FullStateDigestV7,
    pub replacement: EngineStatePartsV3,
    pub operations: Vec<SemanticDeltaOperationV3>,
}

impl StateDeltaV3 {
    pub fn between(
        before: &EngineStatePartsV3,
        after: &EngineStatePartsV3,
        operations: Vec<SemanticDeltaOperationV3>,
    ) -> Result<Self, DeltaApplicationV3Error> {
        before.validate()?;
        after.validate()?;
        validate_revision_step(
            before.predecessor_v5.revision,
            after.predecessor_v5.revision,
        )?;
        validate_delta_operation_coverage(before, after, &operations)?;
        Ok(Self {
            before_revision: before.predecessor_v5.revision,
            after_revision: after.predecessor_v5.revision,
            before_digest: digest(before)?,
            after_digest: digest(after)?,
            replacement: after.clone(),
            operations,
        })
    }

    pub fn apply(
        &self,
        before: &EngineStatePartsV3,
    ) -> Result<EngineStatePartsV3, DeltaApplicationV3Error> {
        before.validate()?;
        if before.predecessor_v5.revision != self.before_revision
            || digest(before)? != self.before_digest
        {
            return Err(DeltaApplicationV3Error::BeforeMismatch);
        }
        self.replacement.validate()?;
        validate_revision_step(self.before_revision, self.after_revision)?;
        validate_delta_operation_coverage(before, &self.replacement, &self.operations)?;
        if self.replacement.predecessor_v5.revision != self.after_revision
            || digest(&self.replacement)? != self.after_digest
        {
            return Err(DeltaApplicationV3Error::AfterMismatch);
        }
        Ok(self.replacement.clone())
    }
}

fn digest(state: &EngineStatePartsV3) -> Result<FullStateDigestV7, DeltaApplicationV3Error> {
    calculate_full_state_digest_v7(state).map_err(|_| DeltaApplicationV3Error::DigestCalculation)
}

fn validate_revision_step(
    before: StateRevision,
    after: StateRevision,
) -> Result<(), DeltaApplicationV3Error> {
    let expected = before
        .0
        .checked_add(1)
        .ok_or(DeltaApplicationV3Error::RevisionProgression)?;
    if after.0 != expected {
        return Err(DeltaApplicationV3Error::RevisionProgression);
    }
    Ok(())
}

fn validate_delta_operation_coverage(
    before: &EngineStatePartsV3,
    after: &EngineStatePartsV3,
    operations: &[SemanticDeltaOperationV3],
) -> Result<(), DeltaApplicationV3Error> {
    use SemanticDeltaOperationV3 as V3;
    let has_v3 = |predicate: &dyn Fn(&V3) -> bool| operations.iter().any(predicate);
    let has_v2 = |predicate: &dyn Fn(&SemanticDeltaOperationV2) -> bool| {
        operations.iter().any(|operation| match operation {
            V3::Existing { operation } => predicate(operation),
            _ => false,
        })
    };
    let has_legacy = |predicate: &dyn Fn(&crate::SemanticDeltaOperation) -> bool| {
        operations.iter().any(|operation| match operation {
            V3::Existing { operation: v2 } => match v2.as_ref() {
                SemanticDeltaOperationV2::Existing { operation } => predicate(operation),
                _ => false,
            },
            _ => false,
        })
    };
    let uncovered = || Err(DeltaApplicationV3Error::UncoveredMutation);

    let old_before = &before.predecessor_v5;
    let old_after = &after.predecessor_v5;
    // Core rules state.
    if old_before.core.active_player != old_after.core.active_player
        && !has_legacy(&|operation| {
            matches!(operation,
            crate::SemanticDeltaOperation::ActivePlayerChanged { from, to }
                if *from == old_before.core.active_player && *to == old_after.core.active_player)
        })
    {
        return uncovered();
    }
    if old_before.core.turn_number != old_after.core.turn_number
        && !has_legacy(&|operation| {
            matches!(operation,
            crate::SemanticDeltaOperation::TurnNumberChanged { from, to }
                if *from == old_before.core.turn_number && *to == old_after.core.turn_number)
        })
    {
        return uncovered();
    }
    if old_before.core.position != old_after.core.position
        && !has_legacy(&|operation| {
            matches!(operation,
            crate::SemanticDeltaOperation::TurnPositionChanged { from, to }
                if *from == old_before.core.position && *to == old_after.core.position)
        })
    {
        return uncovered();
    }
    if old_before.core.priority != old_after.core.priority
        && !has_legacy(&|operation| {
            matches!(operation,
            crate::SemanticDeltaOperation::PriorityChanged { from, to }
                if *from == old_before.core.priority && *to == old_after.core.priority)
        })
    {
        return uncovered();
    }
    for (player, old) in &old_before.core.players {
        let Some(new) = old_after.core.players.get(player) else {
            return uncovered();
        };
        if old.life != new.life
            && !has_legacy(&|operation| {
                matches!(operation,
                crate::SemanticDeltaOperation::LifeChanged { player: changed, from, to }
                    if changed == player && *from == old.life && *to == new.life)
            })
            && !has_v3(&|operation| {
                matches!(operation,
                V3::DamageApplied { recipient: DamageRecipient::Player(changed), .. }
                    if changed == player)
            })
        {
            return uncovered();
        }
        if old.has_lost != new.has_lost
            && !has_legacy(&|operation| {
                matches!(operation,
                crate::SemanticDeltaOperation::StateBasedActionsApplied { actions }
                    if actions.iter().any(|action| matches!(action,
                        crate::SbaSelectedActionV1::PlayerLoses { player: changed }
                            if changed == player)))
            })
        {
            return uncovered();
        }
    }
    if old_before
        .core
        .players
        .keys()
        .ne(old_after.core.players.keys())
    {
        return uncovered();
    }
    if old_before.combat != old_after.combat
        && !has_legacy(&|operation| {
            matches!(
                operation,
                crate::SemanticDeltaOperation::AttackersDeclared { .. }
                    | crate::SemanticDeltaOperation::BlockersDeclared { .. }
                    | crate::SemanticDeltaOperation::CombatDamageDealt { .. }
                    | crate::SemanticDeltaOperation::CombatDamageStepCompleted
                    | crate::SemanticDeltaOperation::CombatEnded
            )
        })
    {
        return uncovered();
    }

    // Zone objects and ordered memberships. Every change must be named by a
    // typed incarnation, stack, tap, or face operation.
    if old_before.zones.stack_order != old_after.zones.stack_order
        && !has_v3(&|operation| {
            matches!(operation,
            V3::StackOrderChanged { from, to }
                if from == &old_before.zones.stack_order && to == &old_after.zones.stack_order)
        })
    {
        return uncovered();
    }
    for (id, old_record) in &old_before.zones.stack_records {
        if let Some(new_record) = old_after.zones.stack_records.get(id) {
            if old_record != new_record {
                return uncovered();
            }
        } else if !has_v3(&|operation| {
            matches!(operation,
            V3::StackItemEnded { stack_object, payload, .. }
                if stack_object == id && old_record.payload.as_ref() == Some(payload.as_ref()))
        }) {
            return uncovered();
        }
    }
    for (id, new_record) in &old_after.zones.stack_records {
        if !old_before.zones.stack_records.contains_key(id)
            && !has_v3(&|operation| {
                matches!(operation,
                V3::StackItemCreated { stack_object, payload }
                    if stack_object == id && new_record.payload.as_ref() == Some(payload.as_ref()))
            })
        {
            return uncovered();
        }
    }
    for (id, old_object) in &old_before.zones.objects {
        match old_after.zones.objects.get(id) {
            Some(new_object) if old_object == new_object => {}
            Some(new_object)
                if old_object.tapped != new_object.tapped
                    && old_object.physical_card == new_object.physical_card
                    && old_object.card_definition == new_object.card_definition
                    && old_object.owner == new_object.owner
                    && old_object.controller == new_object.controller
                    && old_object.face_down == new_object.face_down =>
            {
                if !has_v2(&|operation| {
                    matches!(operation,
                    SemanticDeltaOperationV2::ObjectTapped { object, from, to }
                        if *object == *id && *from == old_object.tapped && *to == new_object.tapped)
                }) && !has_legacy(&|operation| {
                    matches!(operation,
                        crate::SemanticDeltaOperation::ObjectTapped { object, from, to }
                            if *object == *id && *from == old_object.tapped && *to == new_object.tapped)
                }) {
                    return uncovered();
                }
            }
            Some(_) => return uncovered(),
            None => {
                if !has_v2(&|operation| {
                    matches!(operation,
                    SemanticDeltaOperationV2::ObjectEntered { old_object: Some(old), .. }
                        if *old == *id)
                }) && !has_legacy(&|operation| {
                    matches!(operation,
                        crate::SemanticDeltaOperation::ZoneTransition { transition }
                            if transition.old_object == *id)
                }) && !has_legacy(&|operation| {
                    matches!(operation,
                        crate::SemanticDeltaOperation::ObjectCeasedToExist { object }
                            if *object == *id)
                }) {
                    return uncovered();
                }
            }
        }
    }
    for (id, new_object) in &old_after.zones.objects {
        if !old_before.zones.objects.contains_key(id)
            && !has_v2(&|operation| {
                matches!(operation,
                SemanticDeltaOperationV2::ObjectEntered { new_object: entered, .. }
                    if *entered == *id)
            })
            && !has_legacy(&|operation| {
                matches!(operation,
                crate::SemanticDeltaOperation::ZoneTransition { transition }
                    if transition.new_object == *id)
            })
        {
            // A newly constructed Stack card object is named by its spell
            // payload and the canonical zones_v2 child.
            let on_stack = old_after
                .zones
                .locations
                .get(id)
                .is_some_and(|location| location.zone == mtgml_model::ZoneKind::Stack)
                && old_after.zones.stack_records.values().any(|record| {
                    matches!(record.payload.as_ref(), Some(StackItemPayload::Spell {
                        stack_card_object,
                        ..
                    }) if stack_card_object == id)
                });
            if !on_stack || new_object.id != *id {
                return uncovered();
            }
        }
    }
    if old_before.zones.locations != old_after.zones.locations
        && !has_v2(&|operation| matches!(operation, SemanticDeltaOperationV2::ObjectEntered { .. }))
        && !has_legacy(&|operation| {
            matches!(
                operation,
                crate::SemanticDeltaOperation::ZoneTransition { .. }
            )
        })
    {
        return uncovered();
    }
    if old_before.zones.ordered_zones != old_after.zones.ordered_zones
        && !has_v2(&|operation| matches!(operation, SemanticDeltaOperationV2::ObjectEntered { .. }))
        && !has_legacy(&|operation| {
            matches!(
                operation,
                crate::SemanticDeltaOperation::ZoneTransition { .. }
            )
        })
    {
        return uncovered();
    }

    // Execution V4 authority requires an exact owning operation for every
    // staged continuation, pending request, trigger, and effect change.
    if old_before.execution != old_after.execution {
        return uncovered();
    }
    if before.execution_v4.pending_decision != after.execution_v4.pending_decision
        && !has_v3(&|operation| {
            matches!(operation,
            V3::PendingRequestChanged { from, to }
                if from.as_deref() == before.execution_v4.pending_decision.as_ref()
                    && to.as_deref() == after.execution_v4.pending_decision.as_ref())
        })
    {
        return uncovered();
    }
    let continuation_ids = before
        .execution_v4
        .continuations
        .keys()
        .chain(after.execution_v4.continuations.keys())
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    for id in continuation_ids {
        let from = before
            .execution_v4
            .continuations
            .get(&id)
            .map(|record| &record.payload);
        let to = after
            .execution_v4
            .continuations
            .get(&id)
            .map(|record| &record.payload);
        if from != to
            && !has_v3(&|operation| {
                matches!(operation,
                V3::ContinuationChanged { continuation, from: op_from, to: op_to }
                    if *continuation == id && op_from.as_deref() == from && op_to.as_deref() == to)
            })
        {
            return uncovered();
        }
    }
    let trigger_ids = before
        .execution_v4
        .waiting_triggers
        .keys()
        .chain(after.execution_v4.waiting_triggers.keys())
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    for id in trigger_ids {
        let from = before.execution_v4.waiting_triggers.get(&id);
        let to = after.execution_v4.waiting_triggers.get(&id);
        if from != to
            && !has_v3(&|operation| {
                matches!(operation,
                V3::TriggerCreated { trigger } if to == Some(trigger.as_ref()))
                    || matches!(operation,
                    V3::TriggerPlaced { trigger, .. }
                        if *trigger == id && from.is_some() && to.is_none())
            })
        {
            return uncovered();
        }
    }
    let effect_ids = before
        .execution_v4
        .effects
        .keys()
        .chain(after.execution_v4.effects.keys())
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    for id in effect_ids {
        let from = before.execution_v4.effects.get(&id);
        let to = after.execution_v4.effects.get(&id);
        if from != to
            && !has_v3(&|operation| {
                matches!(operation,
                V3::TemporaryEffectChanged { effect, from: op_from, to: op_to }
                    if *effect == id && op_from.as_deref() == from && op_to.as_deref() == to)
            })
        {
            return uncovered();
        }
    }

    // Core card-rules families that are not in the explicit G0 owner list
    // cannot change under this detached contract cut.
    let old_rules = &before.card_rules_state;
    let new_rules = &after.card_rules_state;
    if old_rules.mana != new_rules.mana {
        for player in old_rules
            .mana
            .pools
            .keys()
            .chain(new_rules.mana.pools.keys())
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
        {
            let from = old_rules
                .mana
                .pools
                .get(&player)
                .copied()
                .unwrap_or_default();
            let to = new_rules
                .mana
                .pools
                .get(&player)
                .copied()
                .unwrap_or_default();
            if from != to
                && !has_v3(&|operation| {
                    matches!(operation,
                    V3::ManaPoolChanged { player: op_player, from: op_from, to: op_to, .. }
                        if *op_player == player && *op_from == from && *op_to == to)
                })
                && !has_v3(&|operation| {
                    matches!(operation,
                    V3::AtomicCostCommitted { actor, .. } if *actor == player)
                })
                && !has_v2(&|operation| {
                    matches!(operation,
                    SemanticDeltaOperationV2::ManaAdded { player: op_player, .. }
                        | SemanticDeltaOperationV2::ManaPoolEmptied { player: op_player, .. }
                        if *op_player == player)
                })
            {
                return uncovered();
            }
        }
    }
    if old_rules.counters != new_rules.counters {
        for (object, kinds) in &old_rules.counters.counters {
            for (kind, from) in kinds {
                let to = new_rules
                    .counters
                    .counters
                    .get(object)
                    .and_then(|values| values.get(kind))
                    .copied()
                    .unwrap_or(0);
                if *from != to
                    && !has_v3(&|operation| {
                        matches!(operation,
                        V3::CounterChanged { object: op_object, kind: op_kind, from: op_from, to: op_to, .. }
                            if op_object == object && op_kind == kind && *op_from == *from && *op_to == to)
                    })
                    && !has_v2(&|operation| {
                        matches!(operation,
                        SemanticDeltaOperationV2::CounterChanged { object: op_object, kind: op_kind, from: op_from, to: op_to, .. }
                            if op_object == object && op_kind == kind && *op_from == *from && *op_to == to)
                    })
                {
                    return uncovered();
                }
            }
        }
        for (object, kinds) in &new_rules.counters.counters {
            for (kind, to) in kinds {
                let from = old_rules
                    .counters
                    .counters
                    .get(object)
                    .and_then(|values| values.get(kind))
                    .copied()
                    .unwrap_or(0);
                if from != *to
                    && !has_v3(&|operation| {
                        matches!(operation,
                        V3::CounterChanged { object: op_object, kind: op_kind, from: op_from, to: op_to, .. }
                            if op_object == object && op_kind == kind && *op_from == from && *op_to == *to)
                    })
                    && !has_v2(&|operation| {
                        matches!(operation,
                        SemanticDeltaOperationV2::CounterChanged { object: op_object, kind: op_kind, from: op_from, to: op_to, .. }
                            if op_object == object && op_kind == kind && *op_from == from && *op_to == *to)
                    })
                {
                    return uncovered();
                }
            }
        }
    }
    if old_rules.attachments != new_rules.attachments
        && !has_v2(&|operation| {
            matches!(
                operation,
                SemanticDeltaOperationV2::AttachmentChanged { .. }
            )
        })
        && !has_legacy(&|operation| {
            matches!(
                operation,
                crate::SemanticDeltaOperation::ZoneTransition { .. }
            )
        })
    {
        return uncovered();
    }
    if old_rules.faces != new_rules.faces
        && !has_v2(&|operation| {
            matches!(
                operation,
                SemanticDeltaOperationV2::ObjectFaceChanged { .. }
            )
        })
        && !has_v2(&|operation| matches!(operation, SemanticDeltaOperationV2::ObjectEntered { .. }))
    {
        return uncovered();
    }
    if old_rules.abilities != new_rules.abilities
        && !has_v2(&|operation| {
            matches!(
                operation,
                SemanticDeltaOperationV2::AbilityAuthorityAdded { .. }
                    | SemanticDeltaOperationV2::AbilityAuthorityRemoved { .. }
            )
        })
        && !has_v2(&|operation| {
            matches!(
                operation,
                SemanticDeltaOperationV2::AbilityIdentityChanged { .. }
            )
        })
    {
        return uncovered();
    }
    if !validate_turn_history_delta(before, after, operations) {
        return uncovered();
    }

    if before.predecessor_v5.format != after.predecessor_v5.format {
        return uncovered();
    }
    if old_before.foundation_sources != old_after.foundation_sources
        || old_before.random != old_after.random
            && !has_legacy(&|operation| {
                matches!(
                    operation,
                    crate::SemanticDeltaOperation::RandomValueSampled { .. }
                )
            })
        || (old_before.knowledge != old_after.knowledge
            || old_before.perspective_identities != old_after.perspective_identities)
            && !has_legacy(&|operation| {
                matches!(
                    operation,
                    crate::SemanticDeltaOperation::PerspectiveLifecycle { .. }
                )
            })
    {
        return uncovered();
    }

    let old_allocators = &old_before.allocators;
    let new_allocators = &old_after.allocators;
    if new_allocators.next_object_id < old_allocators.next_object_id
        || new_allocators.next_ability_id < old_allocators.next_ability_id
        || new_allocators.next_stack_object_id < old_allocators.next_stack_object_id
        || new_allocators.next_effect_id < old_allocators.next_effect_id
        || new_allocators.next_trigger_id < old_allocators.next_trigger_id
        || new_allocators.next_decision_id < old_allocators.next_decision_id
        || new_allocators.next_continuation_id < old_allocators.next_continuation_id
        || new_allocators.next_rule_event_id < old_allocators.next_rule_event_id
    {
        return uncovered();
    }
    if new_allocators.next_object_id != old_allocators.next_object_id
        && !has_v2(&|operation| matches!(operation, SemanticDeltaOperationV2::ObjectEntered { .. }))
        && !has_legacy(&|operation| {
            matches!(
                operation,
                crate::SemanticDeltaOperation::ZoneTransition { .. }
            )
        })
        && !has_v3(&|operation| matches!(operation, V3::StackItemCreated { .. }))
    {
        return uncovered();
    }
    if new_allocators.next_ability_id != old_allocators.next_ability_id
        && !has_v2(&|operation| {
            matches!(
                operation,
                SemanticDeltaOperationV2::AbilityAuthorityAdded { .. }
                    | SemanticDeltaOperationV2::AbilityAuthorityRemoved { .. }
            )
        })
    {
        return uncovered();
    }
    if new_allocators.next_stack_object_id != old_allocators.next_stack_object_id
        && !has_v3(&|operation| matches!(operation, V3::StackItemCreated { .. }))
    {
        return uncovered();
    }
    if new_allocators.next_effect_id != old_allocators.next_effect_id
        && !has_v3(&|operation| matches!(operation, V3::TemporaryEffectChanged { to: Some(_), .. }))
    {
        return uncovered();
    }
    if new_allocators.next_trigger_id != old_allocators.next_trigger_id
        && !has_v3(&|operation| matches!(operation, V3::TriggerCreated { .. }))
    {
        return uncovered();
    }
    if new_allocators.next_decision_id != old_allocators.next_decision_id
        && !has_v3(&|operation| matches!(operation, V3::PendingRequestChanged { to: Some(_), .. }))
    {
        return uncovered();
    }
    if new_allocators.next_continuation_id != old_allocators.next_continuation_id
        && !has_v3(&|operation| matches!(operation, V3::ContinuationChanged { to: Some(_), .. }))
    {
        return uncovered();
    }
    if new_allocators.next_rule_event_id != old_allocators.next_rule_event_id
        && operations.is_empty()
    {
        return uncovered();
    }
    Ok(())
}

fn validate_turn_history_delta(
    before: &EngineStatePartsV3,
    after: &EngineStatePartsV3,
    operations: &[SemanticDeltaOperationV3],
) -> bool {
    use crate::SemanticDeltaOperation as Legacy;
    use SemanticDeltaOperationV3 as V3;
    let old = &before.card_rules_state.turn_history;
    let new = &after.card_rules_state.turn_history;
    let has_land_count = |player, from, to| {
        operations.iter().any(|operation| match operation {
            V3::Existing { operation } => matches!(
                operation.as_ref(),
                SemanticDeltaOperationV2::LandPlayCountChanged {
                    player: op_player,
                    from: op_from,
                    to: op_to
                } if *op_player == player && *op_from == from && *op_to == to
            ),
            _ => false,
        })
    };
    let has_life_loss = |player| {
        let Some(old_player) = before.predecessor_v5.core.players.get(&player) else {
            return false;
        };
        let Some(new_player) = after.predecessor_v5.core.players.get(&player) else {
            return false;
        };
        operations.iter().any(|operation| match operation {
            V3::Existing { operation } => match operation.as_ref() {
                SemanticDeltaOperationV2::Existing { operation } => matches!(operation.as_ref(),
                    Legacy::LifeChanged { player: op_player, from, to }
                        if *op_player == player && *from == old_player.life && *to == new_player.life && from > to),
                _ => false,
            },
            V3::DamageApplied {
                recipient: DamageRecipient::Player(op_player),
                post_replacement_amount,
                ..
            } => *op_player == player
                && old_player.life.checked_sub(i64::from(*post_replacement_amount))
                    == Some(new_player.life),
            _ => false,
        })
    };

    if old.turn_number != before.predecessor_v5.core.turn_number
        || new.turn_number != after.predecessor_v5.core.turn_number
    {
        return false;
    }
    if old.turn_number != new.turn_number {
        let exact_turn_change = operations.iter().any(|operation| match operation {
            V3::Existing { operation } => match operation.as_ref() {
                SemanticDeltaOperationV2::Existing { operation } => matches!(
                    operation.as_ref(),
                    Legacy::TurnNumberChanged { from, to }
                        if *from == old.turn_number && *to == new.turn_number
                ),
                _ => false,
            },
            _ => false,
        });
        if !exact_turn_change
            || new
                .players
                .values()
                .any(|history| *history != Default::default())
            || !new.target_occurrences.is_empty()
            || !new.once_ability_used.is_empty()
        {
            return false;
        }
        return true;
    }
    if old.players.keys().ne(new.players.keys()) {
        return false;
    }
    for (player, old_history) in &old.players {
        let Some(new_history) = new.players.get(player) else {
            return false;
        };
        let casts = operations
            .iter()
            .filter(|operation| match operation {
                V3::SpellCast { stack_object, .. } => after
                    .predecessor_v5
                    .zones
                    .stack_records
                    .get(stack_object)
                    .is_some_and(|record| record.controller == *player),
                _ => false,
            })
            .count();
        let noncreature_casts = operations
            .iter()
            .filter(|operation| match operation {
                V3::SpellCast {
                    stack_object,
                    is_creature_spell: false,
                    ..
                } => after
                    .predecessor_v5
                    .zones
                    .stack_records
                    .get(stack_object)
                    .is_some_and(|record| record.controller == *player),
                _ => false,
            })
            .count();
        if u32::try_from(casts)
            .ok()
            .and_then(|count| old_history.spells_cast_total.checked_add(count))
            != Some(new_history.spells_cast_total)
            || u32::try_from(noncreature_casts)
                .ok()
                .and_then(|count| old_history.noncreature_spells_cast.checked_add(count))
                != Some(new_history.noncreature_spells_cast)
        {
            return false;
        }
        if old_history.land_plays_used != new_history.land_plays_used
            && !has_land_count(
                *player,
                old_history.land_plays_used,
                new_history.land_plays_used,
            )
        {
            return false;
        }
        if old_history.lost_life_this_turn != new_history.lost_life_this_turn
            && !(!old_history.lost_life_this_turn
                && new_history.lost_life_this_turn
                && has_life_loss(*player))
        {
            return false;
        }
        // G0e operation vocabulary has no typed red-source or permanent-card
        // graveyard fact; fail closed until a sufficient operation exists.
        if old_history.red_noncombat_damage_dealt != new_history.red_noncombat_damage_dealt
            || old_history.permanent_card_to_graveyard != new_history.permanent_card_to_graveyard
        {
            return false;
        }
    }

    let mut expected_target_additions = std::collections::BTreeSet::new();
    let mut expected_once_additions = std::collections::BTreeSet::new();
    for operation in operations {
        match operation {
            V3::TargetDeclared {
                source_stack_item,
                targets,
            } => {
                let Some(actor) = after
                    .predecessor_v5
                    .zones
                    .stack_records
                    .get(source_stack_item)
                    .map(|record| record.controller)
                else {
                    return false;
                };
                expected_target_additions.extend(targets.iter().filter_map(|binding| {
                    match binding.target {
                        TargetRef::Object(object) => Some((object, actor)),
                        _ => None,
                    }
                }));
            }
            V3::AbilityActivated {
                source, targets, ..
            } => {
                expected_once_additions
                    .insert((source.source.snapshot.object, source.ability_key.0));
                expected_target_additions.extend(targets.iter().filter_map(|binding| {
                    match binding.target {
                        TargetRef::Object(object) => {
                            Some((object, source.source.snapshot.controller))
                        }
                        _ => None,
                    }
                }));
            }
            _ => {}
        }
    }
    let target_additions = new
        .target_occurrences
        .difference(&old.target_occurrences)
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    let expected_target_additions = expected_target_additions
        .difference(&old.target_occurrences)
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    if !old.target_occurrences.is_subset(&new.target_occurrences)
        || target_additions != expected_target_additions
    {
        return false;
    }
    let once_additions = new
        .once_ability_used
        .difference(&old.once_ability_used)
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    if !old.once_ability_used.is_subset(&new.once_ability_used)
        || !once_additions.is_subset(&expected_once_additions)
    {
        return false;
    }
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DeltaApplicationV3Error {
    #[error("FullStateDigestV7 calculation failed")]
    DigestCalculation,
    #[error("StateDeltaV3 before revision or digest does not match")]
    BeforeMismatch,
    #[error("StateDeltaV3 replacement does not match its after identity")]
    AfterMismatch,
    #[error("StateDeltaV3 revision must advance exactly once without overflow")]
    RevisionProgression,
    #[error("StateDeltaV3 operations do not cover every authoritative mutation")]
    UncoveredMutation,
    #[error("StateDeltaV3 contains invalid replacement state: {0}")]
    InvalidReplacement(#[from] EngineStatePartsV3Error),
}
