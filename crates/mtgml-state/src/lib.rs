//! The complete checkpointable authoritative state and exact patch contract.
//!
//! `EngineState` is the only semantic source of truth. Kernels may hold caches,
//! but caches must be derivable and must never affect a transition.

pub mod core;
pub mod delta;
pub mod digest;
pub mod engine;
pub mod execution;
pub mod format;
pub mod identity;
pub mod knowledge;
pub mod validation;
pub mod zones;

pub use core::{CoreRulesState, PlayerState};
pub use delta::{DeltaApplicationError, SemanticDeltaOperation, StateDelta};
pub use digest::StateDigestError;
pub use engine::{EngineState, EngineStateParts};
pub use execution::{ContinuationRecord, ExecutionState, PendingDecisionRecord};
pub use format::{CommanderState, FormatState};
pub use identity::{IdentityAllocatorState, PerspectiveIdentityMap, PerspectiveIdentityState};
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
