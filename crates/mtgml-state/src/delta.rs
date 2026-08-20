use mtgml_model::{DecisionId, FullStateDigestV2, GameObjectId, PlayerId, StateRevision};
use mtgml_random::RandomStreamKeyV1;
use serde::{Deserialize, Serialize};

use crate::digest::StateDigestError;
use crate::engine::{EngineState, EngineStateParts};
use crate::zones::ZoneTransition;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SemanticDeltaOperation {
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
}

/// Exact state patch. The replacement contains every authoritative component;
/// `audit` is the semantic trace and is intentionally not used to reconstruct state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StateDelta {
    pub before_revision: StateRevision,
    pub after_revision: StateRevision,
    pub before_digest: FullStateDigestV2,
    pub after_digest: FullStateDigestV2,
    pub replacement: EngineStateParts,
    pub audit: Vec<SemanticDeltaOperation>,
}

impl StateDelta {
    pub fn between(
        before: &EngineState,
        after: &EngineState,
        audit: Vec<SemanticDeltaOperation>,
    ) -> Result<Self, StateDigestError> {
        Ok(Self {
            before_revision: before.revision,
            after_revision: after.revision,
            before_digest: before.digest()?,
            after_digest: after.digest()?,
            replacement: after.parts(),
            audit,
        })
    }

    pub fn apply(&self, before: &EngineState) -> Result<EngineState, DeltaApplicationError> {
        let before_digest = before
            .digest()
            .map_err(|_| DeltaApplicationError::DigestCalculation)?;
        if before.revision != self.before_revision || before_digest != self.before_digest {
            return Err(DeltaApplicationError::BeforeMismatch);
        }
        let after = EngineState::from(self.replacement.clone());
        let after_digest = after
            .digest()
            .map_err(|_| DeltaApplicationError::DigestCalculation)?;
        if after.revision != self.after_revision || after_digest != self.after_digest {
            return Err(DeltaApplicationError::AfterMismatch);
        }
        Ok(after)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DeltaApplicationError {
    #[error("state digest calculation failed while applying a delta")]
    DigestCalculation,
    #[error("delta does not apply to this before-state")]
    BeforeMismatch,
    #[error("delta replacement does not match its declared after identity")]
    AfterMismatch,
}
