//! The complete checkpointable authoritative state and exact patch contract.
//!
//! `EngineState` is the only semantic source of truth. Kernels may hold caches,
//! but caches must be derivable and must never affect a transition.

mod card_rules;
mod construction;
mod core;
mod damage;
mod delta;
mod digest;
mod engine;
mod engine_state_shape;
mod execution;
mod format;
mod identity;
mod knowledge;
mod lifecycle;
mod semantic_mutations;
mod shared_execution;
mod validation;
mod zones;

/// Detached G0 successor digest identities. G0c freezes their names; G0e
/// owns canonical input validation and digest production.
pub const FULL_STATE_DIGEST_DOMAIN_V7: &str = "mtgml.full-state-digest.v7";
pub const FULL_STATE_DIGEST_INPUT_SCHEMA_V7: &str = "full-state-digest-input.v7";

pub use card_rules::{
    AbilityAuthorityStateV1, AbilityAuthorityV1, AttachmentStateV1, AttachmentTimestampV1,
    AttachmentV1, CardRulesAuthoritativeStateV1, CardRulesStateError, CounterKindV1,
    CounterStateV1, FaceStateV1, ManaColorV1, ManaPoolV1, ManaRestrictionV1, ManaStateV1,
    PlayerTurnHistoryV1, TurnHistoryStateV1,
};
pub use construction::{
    construct_synthetic_engine_state, SyntheticResetInputs, SyntheticStateConstructionError,
    SyntheticV4Setup,
};
pub use core::{
    BeginningStep, CombatBlockerAssignmentV1, CombatState, CombatStep, CoreRulesState, EndingStep,
    PlayerState, PriorityState, TurnPosition,
};
pub use damage::{DamageAssignmentV1, DamageRecipientV1};
pub use delta::{
    CostCommitActionV1, DeltaApplicationError, ManaPoolChangeCauseV1, SemanticDeltaOperation,
    StackItemEndKindV1, StateDelta,
};
pub use digest::{
    calculate_full_state_digest, calculate_full_state_digest_structural_only,
    canonical_state_bytes, full_state_digest_from_payload, StateDigestError,
};
pub use engine::{EngineState, EngineStateError, STARTING_HAND_SIZE};
pub use engine_state_shape::{
    AssemblyStageV2, KnowledgeInvalidationV2, KnowledgeRecordV2, KnowledgeStateV2,
    KnownLocationFactV2, PerspectiveIdentityRecordV2, PerspectiveIdentityStateV2,
    PlayerKnowledgeStateV2, RetiredKnowledgeRecordV2, SbaGraveyardOwnerOrderV1, SbaObjectCauseV1,
    SbaSelectedActionV1, SYNTHETIC_COUNT_MAX,
};
pub use execution::{EffectRecord, ExecutionState};
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
pub use semantic_mutations::{
    AttachmentChangeV1, CounterAnnihilationChangeV1, RoleAttachmentRetirementV1,
    StateFamilyMutationError,
};
pub use shared_execution::{
    AbilitySourceContext, ActionCostFacts, AttackerFact, CastContinuation, CastContinuationStage,
    CompletedTriggerOrder, ContinuationPayload, ContinuationRecord, CostFacts, CostRoute,
    DamageKind, DamageRecipient, EffectExpiry, EffectTimestamp, GameStartContinuation,
    GameStartStage, LifeChangeCause, ManaCost, ManaPaymentStage, ManaPaymentStaging,
    ManaSourceActivation, ManaSourceActivationCost, ModeBinding, NonManaActivationContinuation,
    NonManaActivationStage, PendingTriggerRecord, ReservedNonManaCost, SelectedCostOperand,
    SelectedTriggerTarget, SourceContext, StackItemPayload, StackResolutionContinuation,
    StackResolutionStage, TargetBinding, TargetRef, TemporaryEffectRecord, TemporaryKeyword,
    TemporaryOperation, TriggerActorRequestRoot, TriggerEventSnapshot,
    TriggerPlacementContinuation, TriggerTargetTiming,
};
pub use validation::{validate_engine_state, EngineStateViolation};
pub use zones::{
    GameObject, ObjectSnapshot, StackRecord, VisibilityPartition, ZoneKey, ZoneLocation,
    ZonePosition, ZoneState, ZoneTransition,
};

#[cfg(test)]
mod tests;
