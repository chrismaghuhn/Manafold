use mtgml_replay::ReplayValidationError;
use mtgml_rules::{KernelExecutionError, TransitionViolation};
use thiserror::Error;

use crate::checkpoint::CheckpointValidationError;
use mtgml_state::SyntheticStateConstructionError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum EnvironmentCommitError {
    #[error("a rejected transition changed the environment product")]
    RejectedMutation,
    #[error("candidate environment product did not match the transition")]
    CandidateMismatch,
    #[error("candidate player projection could not be validated")]
    PlayerProjectionInvalid,
}

#[derive(Debug, Error)]
pub enum ReplayExecutionError {
    #[error("replay manifest does not match the starting checkpoint")]
    ManifestMismatch,
    #[error("replay step {step_index} has no authorized pending actor")]
    ActorUnavailable { step_index: u64 },
    #[error("replay step {step_index} has the wrong before revision")]
    BeforeRevisionMismatch { step_index: u64 },
    #[error("replay step {step_index} has the wrong before digest")]
    BeforeDigestMismatch { step_index: u64 },
    #[error("replay step {step_index} has the wrong accepted outcome")]
    OutcomeMismatch { step_index: u64 },
    #[error("replay step {step_index} has the wrong after revision")]
    AfterRevisionMismatch { step_index: u64 },
    #[error("replay step {step_index} has the wrong after digest")]
    AfterDigestMismatch { step_index: u64 },
    #[error("replay step {step_index} transition product differs")]
    TransitionMismatch { step_index: u64 },
    #[error("replay step {step_index} environment counters differ")]
    CounterMismatch { step_index: u64 },
    #[error("replay final identity differs")]
    FinalIdentityMismatch,
}

#[derive(Debug, Error)]
pub enum ControllerError {
    #[error("unknown player")]
    UnknownPlayer,
    #[error("controller lock is poisoned")]
    Poisoned,
    #[error("checkpoint is invalid: {0}")]
    InvalidCheckpoint(String),
    #[error("checkpoint validation failed: {0}")]
    CheckpointValidation(#[from] CheckpointValidationError),
    #[error("kernel execution failed: {0}")]
    KernelExecution(#[from] KernelExecutionError),
    #[error("synthetic state construction failed: {0}")]
    StateConstruction(#[from] SyntheticStateConstructionError),
    #[error("transition contract failed: {0}")]
    TransitionContract(#[from] TransitionViolation),
    #[error("environment commit failed: {0}")]
    EnvironmentCommit(#[from] EnvironmentCommitError),
    #[error("replay validation failed: {0}")]
    ReplayValidation(#[from] ReplayValidationError),
    #[error("replay execution failed: {0}")]
    ReplayExecution(#[from] ReplayExecutionError),
    #[error("checkpoint codec is not supported by this backend")]
    UnsupportedCheckpointCodec,
    #[error("checkpoint state is not executable by the synthetic program")]
    UnsupportedSyntheticState,
    #[error("environment counter {counter} would overflow")]
    CounterOverflow { counter: &'static str },
    #[error("replay identity does not match this backend")]
    ReplayIdentityMismatch,
    #[error("backend failure: {0}")]
    Backend(String),
}
