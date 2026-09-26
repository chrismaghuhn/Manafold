//! The complete checkpointable authoritative state and exact patch contract.
//!
//! `EngineState` is the only semantic source of truth. Kernels may hold caches,
//! but caches must be derivable and must never affect a transition.

mod construction;
mod core;
mod damage;
mod delta;
mod delta_v2;
mod digest;
mod digest_v3;
mod digest_v4;
mod digest_v5;
mod digest_v6;
mod engine;
mod engine_state_parts_v2;
mod engine_state_shape;
mod execution;
mod format;
mod identity;
mod knowledge;
mod lifecycle;
mod persisted_v6;
mod validation;
mod zones;

pub use construction::{
    construct_synthetic_engine_state, SyntheticResetInputs, SyntheticStateConstructionError,
    SyntheticV4Setup,
};
pub use core::{
    BaseCharacteristics, BeginningStep, CombatBlockerAssignmentV1, CombatState, CombatStep,
    ControlHistory, CoreRulesState, EndingStep, FoundationCreatureSource, FoundationSourceKind,
    PlayerState, PriorityState, TurnPosition,
};
pub use damage::{DamageAssignmentV1, DamageRecipientV1};
pub use delta::{DeltaApplicationError, SemanticDeltaOperation, StateDelta};
pub use delta_v2::{DeltaApplicationV2Error, SemanticDeltaOperationV2, StateDeltaV2};
pub use digest::StateDigestError;
pub use digest_v4::{
    calculate_full_state_digest_v4_historical, canonical_state_bytes_v4_historical,
};
pub use digest_v5::{FULL_STATE_DIGEST_DOMAIN_V5, FULL_STATE_DIGEST_INPUT_SCHEMA_V5};
pub use digest_v6::{
    calculate_full_state_digest_v6, canonical_state_bytes_v6, verify_full_state_digest_v6,
    FULL_STATE_DIGEST_DOMAIN_V6, FULL_STATE_DIGEST_INPUT_SCHEMA_V6,
};
pub use engine::{EngineState, EngineStateParts, FULL_STATE_DIGEST_INPUT_SCHEMA};
pub use engine_state_parts_v2::{EngineStatePartsV2, EngineStatePartsV2Error};
pub use engine_state_shape::{
    AssemblyStageV2, ContinuationPayloadV2, ContinuationRecordV2, KnowledgeInvalidationV2,
    KnowledgeRecordV2, KnowledgeStateV2, KnownLocationFactV2, PendingDecisionRecordV2,
    PerspectiveIdentityRecordV2, PerspectiveIdentityStateV2, PlayerKnowledgeStateV2,
    RetiredKnowledgeRecordV2, SbaGraveyardOwnerOrderV1, SbaObjectCauseV1, SbaSelectedActionV1,
    SYNTHETIC_COUNT_MAX, SYNTHETIC_COUNT_MIN,
};
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
pub use persisted_v6::{
    AbilityAuthorityStateV1, AbilityAuthorityV1, AttachmentStateV1, AttachmentTimestampV1,
    AttachmentV1, CardRulesAuthoritativeStateV1, CounterKindV1, CounterStateV1, FaceStateV1,
    FullStateDigestInputV6, ManaColorV1, ManaPoolV1, ManaRestrictionV1, ManaStateV1,
    PersistedExecutionV3, PersistedV6Error, PlayerTurnHistoryV1, TurnHistoryStateV1,
};
pub use validation::{validate_engine_state, EngineStateViolation};
pub use zones::{
    GameObject, ObjectSnapshot, StackRecord, VisibilityPartition, ZoneKey, ZoneLocation,
    ZonePosition, ZoneState, ZoneTransition,
};

#[cfg(test)]
mod tests;
