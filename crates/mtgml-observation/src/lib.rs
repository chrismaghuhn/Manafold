//! Perspective-safe observations, information states, and observed events.
//!
//! Ownership façade: each DTO family lives in its own responsibility module;
//! every public path remains at this crate root exactly as before the split.

mod error;
mod information;
mod knowledge;
mod magic_observation;
mod observation;
mod observed_event;
mod observed_event_v3;
mod player_step;
mod player_step_v3;
mod synthetic_observation;

pub use error::ObservationValidationError;
pub use information::{
    InformationStateDigestInputV2, InformationStateEnvelope, PlayerInformationStateV2,
};
pub use knowledge::{
    PlayerKnowledgeCauseV1, PlayerKnowledgeChannelV1, PlayerKnowledgeInvalidationReasonV1,
    PlayerKnowledgeInvalidationV1, PlayerKnowledgeProvenanceV1, PlayerKnownLocationFactV1,
    PlayerKnownLocationV1, PlayerKnownObjectV1,
};
pub use magic_observation::{
    AttachmentObservationV1, CounterObservationV1, FaceObservationV1, MagicBasicLandObservationV1,
    MagicBlockedStatusV4, MagicCombatBlockerAssignmentV2, MagicCombatBlockerAssignmentV3,
    MagicCombatBlockerAssignmentV4, MagicCombatParticipationV2, MagicCombatParticipationV3,
    MagicCombatParticipationV4, MagicCompletedOrder, MagicMarkedDamageV4, MagicObservation,
    MagicObservationV2, MagicObservationV3, MagicObservationV4, MagicPendingSbaOrdering,
    MagicPlayerLifeV4, ManaPoolObservationV1, PublicCounterKindV1, PublicFaceV1,
    MAGIC_OBSERVATION_SCHEMA_V1, MAGIC_OBSERVATION_SCHEMA_V2, MAGIC_OBSERVATION_SCHEMA_V3,
    MAGIC_OBSERVATION_SCHEMA_V4,
};
pub use observation::ObservationEnvelope;
pub use observed_event::{
    ObservedEventEnvelope, ObservedEventEnvelopeV2, ObservedEventKind, ObservedEventKindV2,
};
pub use observed_event_v3::{
    ManaPoolAfterV1, ManaPoolChangeCauseV1, ObservedCounterKindV3, ObservedEventEnvelopeV3,
    ObservedEventKindV3, ObservedFaceV1,
};
pub use player_step::{
    PlayerServiceErrorCodeV1, PlayerStep, PlayerStepSubmissionV1, PlayerStepV2,
    PlayerSubmissionCodeV1,
};
pub use player_step_v3::PlayerStepV3;
pub use synthetic_observation::{
    SyntheticBeginningStep, SyntheticCombatStep, SyntheticEndingStep, SyntheticObservation,
    SyntheticPriority, SyntheticTurnPosition, SYNTHETIC_OBSERVATION_SCHEMA_V1,
};

pub const OBSERVATION_SCHEMA: &str = "observation-envelope.v1";
pub const INFORMATION_STATE_SCHEMA: &str = "information-state-envelope.v1";
pub const OBSERVED_EVENT_SCHEMA: &str = "observed-event-envelope.v1";
pub const PLAYER_STEP_SCHEMA: &str = "player-step.v1";
pub const INFORMATION_STATE_SCHEMA_V2: &str = "information-state-envelope.v2";
pub const OBSERVED_EVENT_SCHEMA_V2: &str = "observed-event-envelope.v2";
pub const PLAYER_STEP_SCHEMA_V2: &str = "player-step.v2";
pub const OBSERVED_EVENT_SCHEMA_V3: &str = "observed-event-envelope.v3";
pub const PLAYER_STEP_SCHEMA_V3: &str = "player-step.v3";
pub const MAGIC_BASIC_LAND_OBSERVATION_SCHEMA_V1: &str = "magic-basic-land-observation.v1";

#[cfg(test)]
mod provenance_tests;
#[cfg(test)]
mod tests;
