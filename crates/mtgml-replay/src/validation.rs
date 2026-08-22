use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ReplayValidationError {
    #[error("unsupported replay schema version")]
    SchemaVersion,
    #[error("replay identity fields must be non-empty")]
    EmptyIdentity,
    #[error("root seed is not canonical lowercase hexadecimal")]
    Seed,
    #[error("replay manifest must identify at least one deck")]
    MissingDecks,
    #[error("each player must have exactly one deck identity")]
    DuplicateDeckPlayer,
    #[error("replay revisions are not contiguous")]
    RevisionDiscontinuity,
    #[error("rejected response mutated the authoritative revision or full-state identity")]
    RejectedMutation,
    #[error("decision response is invalid")]
    Response,
    #[error("final replay identity does not match its steps")]
    FinalIdentity,
    #[error("an empty replay must end at its initial identity")]
    EmptyReplayIdentity,
    #[error("unsupported RNG contract in replay")]
    UnsupportedRngContract,
    #[error("replay-step schema identity must be replay-step.v2")]
    ReplayStepIdentity,
    #[error("replay checkpoint identity does not recompute")]
    CheckpointIdentity,
    #[error("replay step actor identity is invalid")]
    Actor,
    #[error("accepted replay step counter progression is not deterministic")]
    CounterProgression,
}
