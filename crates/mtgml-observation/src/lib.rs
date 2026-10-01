//! Perspective-safe observations, information states, and observed events.
//!
//! Ownership façade: each DTO family lives in its own responsibility module;
//! every public path remains at this crate root exactly as before the split.

mod error;
mod information;
mod knowledge;
mod magic_observation;
mod magic_shared_execution_observation_v1;
mod observation;
mod observed_event_v4;
mod player_step;
mod player_step_v4;
mod synthetic_observation;

pub use error::ObservationValidationError;
pub use information::{
    InformationStateDigestInput, PlayerInformationState, INFORMATION_STATE_DIGEST_INPUT_SCHEMA_V3,
    INFORMATION_STATE_SCHEMA_V3,
};
pub use knowledge::{
    PlayerKnowledgeCauseV1, PlayerKnowledgeChannelV1, PlayerKnowledgeInvalidationReasonV1,
    PlayerKnowledgeInvalidationV1, PlayerKnowledgeProvenanceV1, PlayerKnownLocationFactV1,
    PlayerKnownLocationV1, PlayerKnownObjectV1,
};
pub use magic_observation::{
    AttachmentObservationV1, CounterObservationV1, FaceObservationV1, MagicBasicLandObservationV1,
    MagicCompletedOrder, MagicPendingSbaOrdering, ManaPoolObservationV1, PermanentObservationV1,
    PlayerObservationV1, PublicCounterKindV1, PublicFaceV1,
};
pub use magic_shared_execution_observation_v1::{
    MagicSharedExecutionObservationV1, PublicEffectExpiryV1, PublicEffectKeywordV1, PublicModeV1,
    PublicStackItemV1, PublicTemporaryEffectV1, PublicTemporaryOperationV1,
    MAGIC_SHARED_EXECUTION_OBSERVATION_SCHEMA_V1,
};
pub use observation::{ObservationEnvelope, OBSERVATION_SCHEMA_V2};
pub use observed_event_v4::{
    ManaPoolAfterV1, ManaPoolChangeCauseV2, ObservedCounterKindV3, ObservedEventEnvelopeV4,
    ObservedEventKindV4, ObservedFaceV1, StackItemRemovalCauseV1,
};
pub use player_step::{PlayerServiceErrorCodeV1, PlayerStepSubmissionV1, PlayerSubmissionCodeV1};
pub use player_step_v4::PlayerStepV4;
pub use synthetic_observation::{
    SyntheticBeginningStep, SyntheticCombatStep, SyntheticEndingStep, SyntheticPriority,
    SyntheticTurnPosition,
};

pub const OBSERVED_EVENT_SCHEMA_V4: &str = "observed-event-envelope.v4";
pub const PLAYER_STEP_SCHEMA_V4: &str = "player-step.v4";
pub const MAGIC_BASIC_LAND_OBSERVATION_SCHEMA_V1: &str = "magic-basic-land-observation.v1";

#[cfg(test)]
mod provenance_tests;
#[cfg(test)]
mod tests;
