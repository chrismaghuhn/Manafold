//! Ownership: shared validation-error vocabulary for every observation DTO
//! generation. Crate-root public path is unchanged via `lib.rs` re-export.

use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ObservationValidationError {
    #[error("unsupported schema version or empty codec")]
    SchemaOrCodec,
    #[error("payload is not canonical base64")]
    Base64,
    #[error("information state and current observation disagree")]
    InformationStateMismatch,
    #[error("observed event label/code must be non-empty")]
    EmptyEventText,
    #[error("random outcome is outside its declared range")]
    RandomOutcome,
    #[error("observed event belongs to a future revision")]
    FutureEvent,
    #[error("next decision is invalid for this endpoint")]
    Decision,
    #[error("episode status is invalid")]
    EpisodeStatus,
    #[error("information-state retained knowledge is not canonical")]
    RetainedKnowledge,
    #[error("information-state visible sequence is not monotonic")]
    VisibleSequence,
    #[error("information-state perspective or revision is inconsistent")]
    PerspectiveRevision,
    #[error("submission outcome contradicts the step product")]
    Submission,
}
