//! Perspective-safe observations, information states, and observed events.
//!
//! Ownership façade: each DTO family lives in its own responsibility module;
//! every public path remains at this crate root exactly as before the split.

mod error;
mod information;
mod knowledge;
mod observation;
mod observed_event;
mod player_step;

pub use error::ObservationValidationError;
pub use information::{
    InformationStateDigestInputV2, InformationStateEnvelope, PlayerInformationStateV2,
};
pub use knowledge::{
    PlayerKnowledgeCauseV1, PlayerKnowledgeChannelV1, PlayerKnowledgeInvalidationReasonV1,
    PlayerKnowledgeInvalidationV1, PlayerKnowledgeProvenanceV1, PlayerKnownLocationFactV1,
    PlayerKnownLocationV1, PlayerKnownObjectV1,
};
pub use observation::ObservationEnvelope;
pub use observed_event::{
    ObservedEventEnvelope, ObservedEventEnvelopeV2, ObservedEventKind, ObservedEventKindV2,
};
pub use player_step::{
    PlayerServiceErrorCodeV1, PlayerStep, PlayerStepSubmissionV1, PlayerStepV2,
    PlayerSubmissionCodeV1,
};

pub const OBSERVATION_SCHEMA: &str = "observation-envelope.v1";
pub const INFORMATION_STATE_SCHEMA: &str = "information-state-envelope.v1";
pub const OBSERVED_EVENT_SCHEMA: &str = "observed-event-envelope.v1";
pub const PLAYER_STEP_SCHEMA: &str = "player-step.v1";
pub const INFORMATION_STATE_SCHEMA_V2: &str = "information-state-envelope.v2";
pub const OBSERVED_EVENT_SCHEMA_V2: &str = "observed-event-envelope.v2";
pub const PLAYER_STEP_SCHEMA_V2: &str = "player-step.v2";

#[cfg(test)]
mod provenance_tests;
#[cfg(test)]
mod tests;
