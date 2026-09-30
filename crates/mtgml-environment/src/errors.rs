use mtgml_observation::PlayerServiceErrorCodeV1;
use mtgml_replay::ReplayValidationError;
use mtgml_rules::{KernelExecutionError, TransitionViolation};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum PlayerEndpointError {
    #[error("service unavailable")]
    ServiceUnavailable,
}

impl From<PlayerServiceErrorCodeV1> for PlayerEndpointError {
    fn from(_: PlayerServiceErrorCodeV1) -> Self {
        Self::ServiceUnavailable
    }
}

use crate::checkpoint::CheckpointValidationError;
use crate::checkpoint_v7::CheckpointV7Error;
use crate::checkpoint_v8::CheckpointV8Error;
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
    #[error("replay step {step_index} has the wrong before digest")]
    BeforeDigestMismatch { step_index: u64 },
    #[error("replay step {step_index} has the wrong after digest")]
    AfterDigestMismatch { step_index: u64 },
    #[error("replay step {step_index} transition product differs")]
    TransitionMismatch { step_index: u64 },
    #[error("replay final identity differs")]
    FinalIdentityMismatch,
}

#[derive(Debug, Error)]
pub enum ControllerError {
    #[error("unknown player")]
    UnknownPlayer,
    #[error("controller lock is poisoned")]
    Poisoned,
    #[error("checkpoint validation failed: {0}")]
    CheckpointValidation(#[from] CheckpointValidationError),
    #[error("successor checkpoint validation failed: {0}")]
    CheckpointV7(#[from] CheckpointV7Error),
    #[error("G0 checkpoint validation failed: {0}")]
    CheckpointV8(#[from] CheckpointV8Error),
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
    #[error("semantic contract is not supported by this runtime")]
    SemanticContractUnsupported,
    #[error("backend failure: {0}")]
    Backend(String),
}
