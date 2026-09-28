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
    StackItemPayload, TemporaryEffectRecord,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DeltaApplicationV3Error {
    #[error("FullStateDigestV7 calculation failed")]
    DigestCalculation,
    #[error("StateDeltaV3 before revision or digest does not match")]
    BeforeMismatch,
    #[error("StateDeltaV3 replacement does not match its after identity")]
    AfterMismatch,
    #[error("StateDeltaV3 contains invalid replacement state: {0}")]
    InvalidReplacement(#[from] EngineStatePartsV3Error),
}
