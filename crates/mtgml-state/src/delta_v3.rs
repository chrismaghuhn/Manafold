//! Detached full-replacement StateDeltaV3 bound to FullStateDigestV7.

use mtgml_decision::AuthoritativeDecisionRequestV4;
use mtgml_model::{
    ContinuationId, EffectInstanceId, FullStateDigestV7, GameObjectId, PlayerId, StackObjectId,
    StateRevision, TriggerInstanceId,
};

use crate::lifecycle::PerspectiveLifecycleAuditV1;
use crate::zones::ZoneTransition;
use crate::TurnPosition;
use mtgml_model::DecisionId;
use mtgml_random::RandomStreamKeyV1;

use crate::{
    calculate_full_state_digest_v7, ContinuationPayloadV3, DamageKind, DamageRecipient,
    EngineState, EngineStatePartsV3Error, ManaCost, ManaPoolV1, PendingTriggerRecord,
    ReservedNonManaCost, SelectedCostOperand, SourceContext, StackItemPayload, TargetRef,
    TemporaryEffectRecord,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemanticDeltaOperationV3 {
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
        assignments: Vec<crate::DamageAssignmentV1>,
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
        actions: Vec<crate::SbaSelectedActionV1>,
    },
    PriorityChanged {
        from: crate::PriorityState,
        to: crate::PriorityState,
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
        assignments: Vec<crate::CombatBlockerAssignmentV1>,
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
    /// Complete state-changing meaning of one perspective-visible occurrence
    /// (M2.E). Carries perspective, consumed visible sequence, and the typed
    /// identity/knowledge mutation as the single state-owned audit payload.
    PerspectiveLifecycle {
        lifecycle: PerspectiveLifecycleAuditV1,
    },
    LandPlayCountChanged {
        player: mtgml_model::PlayerId,
        from: u8,
        to: u8,
    },
    AbilityIdentityChanged {
        perspective: mtgml_model::PlayerId,
        instance: mtgml_model::AbilityInstanceId,
        from: Option<mtgml_model::OpaqueAbilityId>,
        to: Option<mtgml_model::OpaqueAbilityId>,
    },
    AttachmentChanged {
        source: mtgml_model::GameObjectId,
        from_target: Option<mtgml_model::GameObjectId>,
        to_target: Option<mtgml_model::GameObjectId>,
        timestamp_revision: StateRevision,
        operation_ordinal: u32,
    },
    ObjectFaceChanged {
        object: mtgml_model::GameObjectId,
        from_face: u32,
        to_face: u32,
    },
    ObjectEntered {
        old_object: Option<mtgml_model::GameObjectId>,
        new_object: mtgml_model::GameObjectId,
        from_zone: mtgml_model::ZoneKind,
        to_zone: mtgml_model::ZoneKind,
        tapped: bool,
        face: u32,
    },
    AbilityAuthorityAdded {
        instance: mtgml_model::AbilityInstanceId,
        source: mtgml_model::GameObjectId,
        ability_key: u32,
    },
    AbilityAuthorityRemoved {
        instance: mtgml_model::AbilityInstanceId,
        source: mtgml_model::GameObjectId,
        ability_key: u32,
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
        once_per_turn_use_committed: bool,
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
    pub replacement: EngineState,
    pub operations: Vec<SemanticDeltaOperationV3>,
}

impl StateDeltaV3 {
    pub fn between(
        before: &EngineState,
        after: &EngineState,
        operations: Vec<SemanticDeltaOperationV3>,
    ) -> Result<Self, DeltaApplicationV3Error> {
        before.validate()?;
        after.validate()?;
        validate_revision_step(before.revision, after.revision)?;
        validate_delta_operation_coverage(before, after, &operations)?;
        Ok(Self {
            before_revision: before.revision,
            after_revision: after.revision,
            before_digest: digest(before)?,
            after_digest: digest(after)?,
            replacement: after.clone(),
            operations,
        })
    }

    /// Constructs a structural Delta from states whose exact profile-dependent
    /// Decision domain is validated by the containing RulesKernel transaction.
    /// This function validates structure and coverage only; it does not admit
    /// either state. Generic callers must use `between`.
    #[doc(hidden)]
    pub fn between_structural_only(
        before: &EngineState,
        after: &EngineState,
        operations: Vec<SemanticDeltaOperationV3>,
    ) -> Result<Self, DeltaApplicationV3Error> {
        before.validate_structure()?;
        after.validate_structure()?;
        validate_revision_step(before.revision, after.revision)?;
        validate_delta_operation_coverage(before, after, &operations)?;
        Ok(Self {
            before_revision: before.revision,
            after_revision: after.revision,
            before_digest: digest_structural_only(before)?,
            after_digest: digest_structural_only(after)?,
            replacement: after.clone(),
            operations,
        })
    }

    pub fn apply(&self, before: &EngineState) -> Result<EngineState, DeltaApplicationV3Error> {
        before.validate()?;
        if before.revision != self.before_revision || digest(before)? != self.before_digest {
            return Err(DeltaApplicationV3Error::BeforeMismatch);
        }
        self.replacement.validate()?;
        validate_revision_step(self.before_revision, self.after_revision)?;
        validate_delta_operation_coverage(before, &self.replacement, &self.operations)?;
        if self.replacement.revision != self.after_revision
            || digest(&self.replacement)? != self.after_digest
        {
            return Err(DeltaApplicationV3Error::AfterMismatch);
        }
        Ok(self.replacement.clone())
    }

    /// Applies a structural Delta after RulesKernel domain validation at the
    /// containing environment transaction boundary. This operation alone is
    /// not state admission; generic callers must use `apply`.
    #[doc(hidden)]
    pub fn apply_structural_only(
        &self,
        before: &EngineState,
    ) -> Result<EngineState, DeltaApplicationV3Error> {
        before.validate_structure()?;
        if before.revision != self.before_revision
            || digest_structural_only(before)? != self.before_digest
        {
            return Err(DeltaApplicationV3Error::BeforeMismatch);
        }
        self.replacement.validate_structure()?;
        validate_revision_step(self.before_revision, self.after_revision)?;
        validate_delta_operation_coverage(before, &self.replacement, &self.operations)?;
        if self.replacement.revision != self.after_revision
            || digest_structural_only(&self.replacement)? != self.after_digest
        {
            return Err(DeltaApplicationV3Error::AfterMismatch);
        }
        Ok(self.replacement.clone())
    }
}

fn digest(state: &EngineState) -> Result<FullStateDigestV7, DeltaApplicationV3Error> {
    calculate_full_state_digest_v7(state).map_err(|_| DeltaApplicationV3Error::DigestCalculation)
}

fn digest_structural_only(
    state: &EngineState,
) -> Result<FullStateDigestV7, DeltaApplicationV3Error> {
    crate::calculate_full_state_digest_v7_structural_only(state)
        .map_err(|_| DeltaApplicationV3Error::DigestCalculation)
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
    before: &EngineState,
    after: &EngineState,
    operations: &[SemanticDeltaOperationV3],
) -> Result<(), DeltaApplicationV3Error> {
    use SemanticDeltaOperationV3 as V3;
    let has = |predicate: &dyn Fn(&V3) -> bool| operations.iter().any(predicate);
    let uncovered = || Err(DeltaApplicationV3Error::UncoveredMutation);

    let old_before = before;
    let old_after = after;
    // An untap step claims exactly the permanents it untapped: each listed
    // object was tapped before and is untapped after.
    if has(&|operation| {
        matches!(operation,
        V3::UntapCompleted { affected_objects }
            if affected_objects.iter().any(|object| {
                !(old_before.zones.objects.get(object).is_some_and(|old| old.tapped)
                    && old_after.zones.objects.get(object).is_some_and(|new| !new.tapped))
            }))
    }) {
        return uncovered();
    }
    // Core rules state.
    if old_before.core.active_player != old_after.core.active_player
        && !has(&|operation| {
            matches!(operation,
            V3::ActivePlayerChanged { from, to }
                if *from == old_before.core.active_player && *to == old_after.core.active_player)
        })
    {
        return uncovered();
    }
    if old_before.core.turn_number != old_after.core.turn_number
        && !has(&|operation| {
            matches!(operation,
            V3::TurnNumberChanged { from, to }
                if *from == old_before.core.turn_number && *to == old_after.core.turn_number)
        })
    {
        return uncovered();
    }
    if old_before.core.position != old_after.core.position
        && !has(&|operation| {
            matches!(operation,
            V3::TurnPositionChanged { from, to }
                if *from == old_before.core.position && *to == old_after.core.position)
        })
    {
        return uncovered();
    }
    if old_before.core.priority != old_after.core.priority
        && !has(&|operation| {
            matches!(operation,
            V3::PriorityChanged { from, to }
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
            && !has(&|operation| {
                matches!(operation,
                V3::LifeChanged { player: changed, from, to }
                    if changed == player && *from == old.life && *to == new.life)
            })
            && !has(&|operation| {
                matches!(operation,
                V3::DamageApplied { recipient: DamageRecipient::Player(changed), .. }
                    if changed == player)
            })
        {
            return uncovered();
        }
        if old.has_lost != new.has_lost
            && !has(&|operation| {
                matches!(operation,
                V3::StateBasedActionsApplied { actions }
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
        && !has(&|operation| {
            matches!(
                operation,
                V3::AttackersDeclared { .. }
                    | V3::BlockersDeclared { .. }
                    | V3::CombatDamageDealt { .. }
                    | V3::CombatDamageStepCompleted
                    | V3::CombatEnded
            )
        })
    {
        return uncovered();
    }

    // Zone objects and ordered memberships. Every change must be named by a
    // typed incarnation, stack, tap, or face operation.
    if old_before.zones.stack_order != old_after.zones.stack_order
        && !has(&|operation| {
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
        } else if !has(&|operation| {
            matches!(operation,
            V3::StackItemEnded { stack_object, payload, .. }
                if stack_object == id && old_record.payload.as_ref() == Some(payload.as_ref()))
        }) {
            return uncovered();
        }
    }
    for (id, new_record) in &old_after.zones.stack_records {
        if !old_before.zones.stack_records.contains_key(id) {
            let exact_creation_count = operations
                .iter()
                .filter(|operation| {
                    matches!(operation,
                        V3::StackItemCreated { stack_object, payload }
                            if stack_object == id && new_record.payload.as_ref() == Some(payload.as_ref()))
                })
                .count();
            if exact_creation_count != 1 {
                return uncovered();
            }
            let matching_action_count = operations
                .iter()
                .filter(
                    |operation| match (new_record.payload.as_ref(), *operation) {
                        (
                            Some(StackItemPayload::Spell { .. }),
                            V3::SpellCast { stack_object, .. },
                        ) => *stack_object == *id,
                        (
                            Some(StackItemPayload::ActivatedAbility { .. }),
                            V3::AbilityActivated { stack_object, .. },
                        ) => *stack_object == *id,
                        (
                            Some(StackItemPayload::TriggeredAbility {
                                originating_trigger,
                                ..
                            }),
                            V3::TriggerPlaced {
                                trigger,
                                stack_object,
                                payload,
                            },
                        ) => {
                            *stack_object == *id
                                && trigger == originating_trigger
                                && new_record.payload.as_ref() == Some(payload.as_ref())
                        }
                        _ => false,
                    },
                )
                .count();
            if matching_action_count != 1 {
                return uncovered();
            }
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
                let untapped_by_untap_step = old_object.tapped
                    && !new_object.tapped
                    && has(&|operation| {
                        matches!(operation,
                            V3::UntapCompleted { affected_objects }
                                if affected_objects.contains(id))
                    });
                let covered = untapped_by_untap_step
                    || has(&|operation| {
                        matches!(operation,
                            V3::ObjectTapped { object, from, to }
                                if *object == *id && *from == old_object.tapped && *to == new_object.tapped)
                    });
                if !covered {
                    return uncovered();
                }
            }
            Some(_) => return uncovered(),
            None => {
                if !has(&|operation| {
                    matches!(operation,
                    V3::ObjectEntered { old_object: Some(old), .. }
                        if *old == *id)
                }) && !has(&|operation| {
                    matches!(operation,
                        V3::ZoneTransition { transition }
                            if transition.old_object == *id)
                }) && !has(&|operation| {
                    matches!(operation,
                        V3::ObjectCeasedToExist { object }
                            if *object == *id)
                }) {
                    return uncovered();
                }
            }
        }
    }
    for (id, new_object) in &old_after.zones.objects {
        if !old_before.zones.objects.contains_key(id)
            && !has(&|operation| {
                matches!(operation,
                V3::ObjectEntered { new_object: entered, .. }
                    if *entered == *id)
            })
            && !has(&|operation| {
                matches!(operation,
                V3::ZoneTransition { transition }
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
        && !has(&|operation| matches!(operation, V3::ObjectEntered { .. }))
        && !has(&|operation| matches!(operation, V3::ZoneTransition { .. }))
    {
        return uncovered();
    }
    if old_before.zones.ordered_zones != old_after.zones.ordered_zones
        && !has(&|operation| matches!(operation, V3::ObjectEntered { .. }))
        && !has(&|operation| matches!(operation, V3::ZoneTransition { .. }))
    {
        return uncovered();
    }

    // Execution V4 authority requires an exact owning operation for every
    // staged continuation, pending request, trigger, and effect change.
    if before.execution.pending_decision != after.execution.pending_decision
        && !has(&|operation| {
            matches!(operation,
            V3::PendingRequestChanged { from, to }
                if from.as_deref() == before.execution.pending_decision.as_ref()
                    && to.as_deref() == after.execution.pending_decision.as_ref())
        })
    {
        return uncovered();
    }
    let continuation_ids = before
        .execution
        .continuations
        .keys()
        .chain(after.execution.continuations.keys())
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    for id in continuation_ids {
        let from = before
            .execution
            .continuations
            .get(&id)
            .map(|record| &record.payload);
        let to = after
            .execution
            .continuations
            .get(&id)
            .map(|record| &record.payload);
        if from != to
            && !has(&|operation| {
                matches!(operation,
                V3::ContinuationChanged { continuation, from: op_from, to: op_to }
                    if *continuation == id && op_from.as_deref() == from && op_to.as_deref() == to)
            })
        {
            return uncovered();
        }
    }
    let trigger_ids = before
        .execution
        .waiting_triggers
        .keys()
        .chain(after.execution.waiting_triggers.keys())
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    for id in trigger_ids {
        let from = before.execution.waiting_triggers.get(&id);
        let to = after.execution.waiting_triggers.get(&id);
        if from != to
            && !has(&|operation| {
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
        .execution
        .effects
        .keys()
        .chain(after.execution.effects.keys())
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    for id in effect_ids {
        let from = before.execution.effects.get(&id);
        let to = after.execution.effects.get(&id);
        if from != to
            && !has(&|operation| {
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
    let old_rules = &before.card_rules;
    let new_rules = &after.card_rules;
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
                && !has(&|operation| {
                    matches!(operation,
                    V3::ManaPoolChanged { player: op_player, from: op_from, to: op_to, .. }
                        if *op_player == player && *op_from == from && *op_to == to)
                })
                && !has(&|operation| {
                    matches!(operation,
                    V3::AtomicCostCommitted { actor, .. } if *actor == player)
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
                    && !has(&|operation| {
                        matches!(operation,
                        V3::CounterChanged { object: op_object, kind: op_kind, from: op_from, to: op_to, .. }
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
                    && !has(&|operation| {
                        matches!(operation,
                        V3::CounterChanged { object: op_object, kind: op_kind, from: op_from, to: op_to, .. }
                            if op_object == object && op_kind == kind && *op_from == from && *op_to == *to)
                    })
                {
                    return uncovered();
                }
            }
        }
    }
    if old_rules.attachments != new_rules.attachments
        && !has(&|operation| matches!(operation, V3::AttachmentChanged { .. }))
        && !has(&|operation| matches!(operation, V3::ZoneTransition { .. }))
    {
        return uncovered();
    }
    // A zone transition carries the card's face from the old incarnation to
    // the new one unchanged.
    let faces_follow_zone_transitions = || {
        let mut expected = old_rules.faces.faces.clone();
        for operation in operations {
            if let V3::ZoneTransition { transition } = operation {
                if let Some(face) = expected.remove(&transition.old_object) {
                    expected.insert(transition.new_object, face);
                }
            }
        }
        expected == new_rules.faces.faces
    };
    if old_rules.faces != new_rules.faces
        && !has(&|operation| matches!(operation, V3::ObjectFaceChanged { .. }))
        && !has(&|operation| matches!(operation, V3::ObjectEntered { .. }))
        && !faces_follow_zone_transitions()
    {
        return uncovered();
    }
    if old_rules.abilities != new_rules.abilities
        && !has(&|operation| {
            matches!(
                operation,
                V3::AbilityAuthorityAdded { .. } | V3::AbilityAuthorityRemoved { .. }
            )
        })
        && !has(&|operation| matches!(operation, V3::AbilityIdentityChanged { .. }))
    {
        return uncovered();
    }
    if !validate_turn_history_delta(before, after, operations) {
        return uncovered();
    }

    if before.format != after.format {
        return uncovered();
    }
    for operation in operations {
        let action_stack = match operation {
            V3::SpellCast { stack_object, .. } | V3::AbilityActivated { stack_object, .. } => {
                Some(*stack_object)
            }
            _ => None,
        };
        if let Some(stack_object) = action_stack {
            let action_occurrences = operations
                .iter()
                .filter(|candidate| match candidate {
                    V3::SpellCast {
                        stack_object: candidate_stack,
                        ..
                    }
                    | V3::AbilityActivated {
                        stack_object: candidate_stack,
                        ..
                    } => *candidate_stack == stack_object,
                    _ => false,
                })
                .count();
            if action_occurrences != 1 {
                return uncovered();
            }
        }
        match operation {
            V3::SpellCast {
                stack_object,
                spell_object,
                card_definition,
                face_key,
                semantic_profile_id,
                cost_facts,
                ..
            } => {
                if before.zones.stack_records.contains_key(stack_object) {
                    return uncovered();
                }
                let Some(record) = after.zones.stack_records.get(stack_object) else {
                    return uncovered();
                };
                let payload = record.payload.as_ref();
                let expected = matches!(payload,
                    Some(StackItemPayload::Spell {
                        stack_card_object, card_definition_id, face_key: actual_face,
                        semantic_profile_id: actual_profile, cost_facts: actual_costs, ..
                    }) if stack_card_object == spell_object && card_definition_id == card_definition
                        && actual_face == face_key && actual_profile == semantic_profile_id
                        && actual_costs == cost_facts);
                let creation_count = operations
                    .iter()
                    .filter(|candidate| {
                        matches!(candidate,
                    V3::StackItemCreated { stack_object: created, payload: created_payload }
                        if created == stack_object && Some(created_payload.as_ref()) == payload)
                    })
                    .count();
                if !expected || creation_count != 1 {
                    return uncovered();
                }
            }
            V3::AbilityActivated {
                stack_object,
                source,
                targets,
                cost_facts,
                ..
            } => {
                if before.zones.stack_records.contains_key(stack_object) {
                    return uncovered();
                }
                let Some(record) = after.zones.stack_records.get(stack_object) else {
                    return uncovered();
                };
                let payload = record.payload.as_ref();
                let expected = matches!(payload,
                    Some(StackItemPayload::ActivatedAbility {
                        source_context, targets: actual_targets, cost_facts: actual_costs, ..
                    }) if source_context == source.as_ref() && actual_targets == targets
                        && actual_costs == cost_facts);
                let creation_count = operations
                    .iter()
                    .filter(|candidate| {
                        matches!(candidate,
                    V3::StackItemCreated { stack_object: created, payload: created_payload }
                        if created == stack_object && Some(created_payload.as_ref()) == payload)
                    })
                    .count();
                if !expected || creation_count != 1 {
                    return uncovered();
                }
            }
            _ => {}
        }
    }
    if old_before.random != old_after.random
        && !has(&|operation| matches!(operation, V3::RandomValueSampled { .. }))
        || old_before.knowledge != old_after.knowledge
            && !has(&|operation| matches!(operation, V3::PerspectiveLifecycle { .. }))
    {
        return uncovered();
    }
    for (perspective, old) in &old_before.perspective_identities.players {
        let Some(new) = old_after.perspective_identities.players.get(perspective) else {
            return uncovered();
        };
        let object_identity_changed = old.object_to_opaque != new.object_to_opaque
            || old.opaque_to_object != new.opaque_to_object
            || old.retired_object_ids != new.retired_object_ids
            || old.next_opaque_object_id != new.next_opaque_object_id;
        if object_identity_changed
            && !has(&|operation| matches!(operation, V3::PerspectiveLifecycle { .. }))
        {
            return uncovered();
        }
        let ability_identity_changed = old.ability_to_opaque != new.ability_to_opaque
            || old.opaque_to_ability != new.opaque_to_ability
            || old.retired_ability_ids != new.retired_ability_ids
            || old.next_opaque_ability_id != new.next_opaque_ability_id;
        if ability_identity_changed
            && !has(&|operation| {
                matches!(operation,
                    V3::AbilityIdentityChanged {
                        perspective: changed, ..
                    } if changed == perspective)
            })
        {
            return uncovered();
        }
        if old.next_player_decision_id != new.next_player_decision_id
            && !has(&|operation| {
                matches!(operation,
                    V3::PendingRequestChanged { to: Some(request), .. }
                        if request.actor == *perspective
                            && request.player_decision_id == old.next_player_decision_id
                            && request.player_decision_id.0.checked_add(1)
                                == Some(new.next_player_decision_id.0))
            })
        {
            return uncovered();
        }
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
        && !has(&|operation| matches!(operation, V3::ObjectEntered { .. }))
        && !has(&|operation| matches!(operation, V3::ZoneTransition { .. }))
        && !has(&|operation| matches!(operation, V3::StackItemCreated { .. }))
    {
        return uncovered();
    }
    if new_allocators.next_ability_id != old_allocators.next_ability_id
        && !has(&|operation| {
            matches!(
                operation,
                V3::AbilityAuthorityAdded { .. } | V3::AbilityAuthorityRemoved { .. }
            )
        })
    {
        return uncovered();
    }
    if new_allocators.next_stack_object_id != old_allocators.next_stack_object_id
        && !has(&|operation| matches!(operation, V3::StackItemCreated { .. }))
    {
        return uncovered();
    }
    if new_allocators.next_effect_id != old_allocators.next_effect_id
        && !has(&|operation| matches!(operation, V3::TemporaryEffectChanged { to: Some(_), .. }))
    {
        return uncovered();
    }
    if new_allocators.next_trigger_id != old_allocators.next_trigger_id
        && !has(&|operation| matches!(operation, V3::TriggerCreated { .. }))
    {
        return uncovered();
    }
    if new_allocators.next_decision_id != old_allocators.next_decision_id
        && !has(&|operation| matches!(operation, V3::PendingRequestChanged { to: Some(_), .. }))
    {
        return uncovered();
    }
    if new_allocators.next_continuation_id != old_allocators.next_continuation_id
        && !has(&|operation| matches!(operation, V3::ContinuationChanged { to: Some(_), .. }))
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
    before: &EngineState,
    after: &EngineState,
    operations: &[SemanticDeltaOperationV3],
) -> bool {
    use SemanticDeltaOperationV3 as V3;
    let old = &before.card_rules.turn_history;
    let new = &after.card_rules.turn_history;
    let has_land_count = |player, from, to| {
        operations.iter().any(|operation| {
            matches!(operation,
                V3::LandPlayCountChanged {
                    player: op_player,
                    from: op_from,
                    to: op_to
                } if *op_player == player && *op_from == from && *op_to == to)
        })
    };
    let has_life_loss = |player| {
        let Some(old_player) = before.core.players.get(&player) else {
            return false;
        };
        let Some(new_player) = after.core.players.get(&player) else {
            return false;
        };
        operations.iter().any(|operation| match operation {
            V3::LifeChanged {
                player: op_player,
                from,
                to,
            } => {
                *op_player == player
                    && *from == old_player.life
                    && *to == new_player.life
                    && from > to
            }
            V3::DamageApplied {
                recipient: DamageRecipient::Player(op_player),
                post_replacement_amount,
                ..
            } => {
                *op_player == player
                    && old_player
                        .life
                        .checked_sub(i64::from(*post_replacement_amount))
                        == Some(new_player.life)
            }
            _ => false,
        })
    };

    if old.turn_number != before.core.turn_number || new.turn_number != after.core.turn_number {
        return false;
    }
    if old.turn_number != new.turn_number {
        let exact_turn_change = operations.iter().any(|operation| {
            matches!(operation,
                V3::TurnNumberChanged { from, to }
                    if *from == old.turn_number && *to == new.turn_number)
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
    let mut once_receipt_count = 0usize;
    for operation in operations {
        match operation {
            V3::TargetDeclared {
                source_stack_item,
                targets,
            } => {
                let Some(actor) = after
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
                source,
                targets,
                once_per_turn_use_committed,
                ..
            } => {
                if *once_per_turn_use_committed {
                    once_receipt_count += 1;
                    expected_once_additions
                        .insert((source.source.snapshot.object, source.ability_key.0));
                }
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
        || once_receipt_count != expected_once_additions.len()
        || once_additions != expected_once_additions
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
