//! The complete checkpointable authoritative state and exact patch contract.
//!
//! `EngineState` is the only semantic source of truth. Kernels may hold caches,
//! but caches must be derivable and must never affect a transition.

mod construction;
mod core;
mod delta;
mod digest;
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
pub use execution::{
    ContinuationRecord, EffectRecord, ExecutionState, PendingDecisionRecord, TriggerRecord,
};
pub use format::{CommanderState, FormatState};
pub use identity::{
    IdentityAllocationError, IdentityAllocatorState, PerspectiveIdentityMap,
    PerspectiveIdentityState,
};
pub use knowledge::{
    KnowledgeAcquisitionReason, KnowledgeHistoryChannel, KnowledgeInvalidationReason,
    KnowledgeInvalidationRecord, KnowledgePoint, KnowledgeState, KnownObjectIdentity,
    PlayerKnowledgeState,
};
pub use validation::{validate_engine_state, EngineStateViolation};
pub use zones::{
    GameObject, ObjectSnapshot, StackRecord, VisibilityPartition, ZoneKey, ZoneLocation,
    ZonePosition, ZoneState, ZoneTransition,
};

#[cfg(test)]
mod tests;
