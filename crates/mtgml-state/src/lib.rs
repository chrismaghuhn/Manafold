//! The complete checkpointable authoritative state and exact patch contract.
//!
//! `EngineState` is the only semantic source of truth. Kernels may hold caches,
//! but caches must be derivable and must never affect a transition.

mod construction;
mod core;
mod delta;
mod digest;
mod digest_v3;
mod engine;
mod execution;
mod format;
mod identity;
mod knowledge;
mod m2_shape;
mod validation;
mod zones;

pub use construction::{
    construct_synthetic_engine_state, SyntheticResetInputs, SyntheticStateConstructionError,
};
pub use core::{CoreRulesState, PlayerState};
pub use delta::{DeltaApplicationError, SemanticDeltaOperation, StateDelta};
pub use digest::StateDigestError;
pub use engine::{EngineState, EngineStateParts, FULL_STATE_DIGEST_INPUT_SCHEMA};
pub use execution::{EffectRecord, ExecutionState, TriggerRecord};
pub use format::{CommanderState, FormatState};
pub use identity::{IdentityAllocationError, IdentityAllocatorState};
pub use knowledge::{
    KnowledgeAcquisitionCause, KnowledgeAcquisitionReason, KnowledgeHistoryChannel,
    KnowledgeInvalidationReason, KnowledgePoint,
};
pub use m2_shape::{
    AssemblyStageV2, ContinuationPayloadV2, ContinuationRecordV2, KnowledgeHistoryRecordV2,
    KnowledgeRecordV2, KnowledgeStateV2, PendingDecisionRecordV2, PerspectiveIdentityRecordV2,
    PerspectiveIdentityStateV2, PlayerKnowledgeStateV2, RetiredKnowledgeRecordV2,
};
pub use validation::{validate_engine_state, EngineStateViolation};
pub use zones::{
    GameObject, ObjectSnapshot, StackRecord, VisibilityPartition, ZoneKey, ZoneLocation,
    ZonePosition, ZoneState, ZoneTransition,
};

#[cfg(test)]
mod tests;
