//! Detached full-replacement StateDeltaV2 bound to FullStateDigestV6.

use mtgml_model::{FullStateDigestV6, StateRevision};

use crate::{
    calculate_full_state_digest_v6, CounterKindV1, EngineStatePartsV2, EngineStatePartsV2Error,
    ManaColorV1, ManaPoolV1, ManaRestrictionV1, SemanticDeltaOperation,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemanticDeltaOperationV2 {
    Existing {
        operation: Box<SemanticDeltaOperation>,
    },
    LandPlayCountChanged {
        player: mtgml_model::PlayerId,
        from: u8,
        to: u8,
    },
    ManaAdded {
        player: mtgml_model::PlayerId,
        color: ManaColorV1,
        restriction: ManaRestrictionV1,
        amount: u32,
        source: mtgml_model::GameObjectId,
        ability_key: u32,
    },
    ManaPoolEmptied {
        player: mtgml_model::PlayerId,
        previous_pool: ManaPoolV1,
    },
    CounterChanged {
        object: mtgml_model::GameObjectId,
        kind: CounterKindV1,
        from: u32,
        to: u32,
        cause: mtgml_model::RuleEventId,
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateDeltaV2 {
    pub before_revision: StateRevision,
    pub after_revision: StateRevision,
    pub before_digest: FullStateDigestV6,
    pub after_digest: FullStateDigestV6,
    pub replacement: EngineStatePartsV2,
    pub operations: Vec<SemanticDeltaOperationV2>,
}

impl StateDeltaV2 {
    pub fn between(
        before: &EngineStatePartsV2,
        after: &EngineStatePartsV2,
        operations: Vec<SemanticDeltaOperationV2>,
    ) -> Result<Self, DeltaApplicationV2Error> {
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
        before: &EngineStatePartsV2,
    ) -> Result<EngineStatePartsV2, DeltaApplicationV2Error> {
        before.validate()?;
        if before.predecessor_v5.revision != self.before_revision
            || digest(before)? != self.before_digest
        {
            return Err(DeltaApplicationV2Error::BeforeMismatch);
        }
        self.replacement.validate()?;
        if self.replacement.predecessor_v5.revision != self.after_revision
            || digest(&self.replacement)? != self.after_digest
        {
            return Err(DeltaApplicationV2Error::AfterMismatch);
        }
        Ok(self.replacement.clone())
    }
}

fn digest(parts: &EngineStatePartsV2) -> Result<FullStateDigestV6, DeltaApplicationV2Error> {
    calculate_full_state_digest_v6(&parts.materialize(), parts.card_rules_state.clone())
        .map_err(|_| DeltaApplicationV2Error::DigestCalculation)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DeltaApplicationV2Error {
    #[error("FullStateDigestV6 calculation failed")]
    DigestCalculation,
    #[error("StateDeltaV2 before revision or digest does not match")]
    BeforeMismatch,
    #[error("StateDeltaV2 replacement does not match its after identity")]
    AfterMismatch,
    #[error("StateDeltaV2 contains invalid replacement state: {0}")]
    InvalidReplacement(#[from] EngineStatePartsV2Error),
}
