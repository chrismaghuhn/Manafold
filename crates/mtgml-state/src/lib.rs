//! The complete checkpointable authoritative state and exact patch contract.
//!
//! `EngineState` is the only semantic source of truth. Kernels may hold caches,
//! but caches must be derivable and must never affect a transition.

mod construction;
mod core;
mod delta;
mod digest;
mod digest_v3;
mod digest_v4;
mod digest_v5;
mod engine;
mod execution;
mod format;
mod identity;
mod knowledge;
mod lifecycle;
mod m2_shape;
mod validation;
mod zones;

pub use construction::{
    construct_synthetic_engine_state, SyntheticResetInputs, SyntheticStateConstructionError,
    SyntheticV4Setup,
};
pub use core::{
    BaseCharacteristics, BeginningStep, CombatState, CombatStep, ControlHistory, CoreRulesState,
    EndingStep, FoundationCreatureSource, FoundationSourceKind, PlayerState, PriorityState,
    TurnPosition,
};
pub use delta::{DeltaApplicationError, SemanticDeltaOperation, StateDelta};
pub use digest::StateDigestError;
pub use digest_v4::{
    calculate_full_state_digest_v4_historical, canonical_state_bytes_v4_historical,
};
pub use digest_v5::{FULL_STATE_DIGEST_DOMAIN_V5, FULL_STATE_DIGEST_INPUT_SCHEMA_V5};
pub use engine::{EngineState, EngineStateParts, FULL_STATE_DIGEST_INPUT_SCHEMA};
pub use execution::{EffectRecord, ExecutionState, TriggerRecord};
pub use format::{CommanderState, FormatState};
pub use identity::{IdentityAllocationError, IdentityAllocatorState};
pub use knowledge::{
    KnowledgeAcquisitionCause, KnowledgeAcquisitionReason, KnowledgeHistoryChannel,
    KnowledgeInvalidationReason,
};
pub use lifecycle::{
    advance_identity_record, apply_lifecycle_to_player, apply_perspective_lifecycle,
    IdentityMutationV1, KnowledgeLocationUpdateV1, KnowledgeMutationV1, LifecycleApplicationError,
    PerspectiveLifecycleAuditV1, PerspectiveLifecycleMutationV1,
};
pub use m2_shape::{
    AssemblyStageV2, ContinuationPayloadV2, ContinuationRecordV2, KnowledgeInvalidationV2,
    KnowledgeRecordV2, KnowledgeStateV2, KnownLocationFactV2, PendingDecisionRecordV2,
    PerspectiveIdentityRecordV2, PerspectiveIdentityStateV2, PlayerKnowledgeStateV2,
    RetiredKnowledgeRecordV2, SbaGraveyardOwnerOrderV1, SbaObjectCauseV1, SbaSelectedActionV1,
    SYNTHETIC_COUNT_MAX, SYNTHETIC_COUNT_MIN,
};
pub use validation::{validate_engine_state, EngineStateViolation};
pub use zones::{
    GameObject, ObjectSnapshot, StackRecord, VisibilityPartition, ZoneKey, ZoneLocation,
    ZonePosition, ZoneState, ZoneTransition,
};

#[cfg(test)]
mod tests;
