use mtgml_observation::PlayerServiceErrorCodeV1;
use mtgml_replay::ReplayValidationError;
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
    #[error("replay validation failed: {0}")]
    ReplayValidation(#[from] ReplayValidationError),
    #[error("replay execution failed: {0}")]
    ReplayExecution(#[from] ReplayExecutionError),
    #[error("semantic contract is not supported by this runtime")]
    SemanticContractUnsupported,
    #[error("backend failure: {0}")]
    Backend(String),
}
