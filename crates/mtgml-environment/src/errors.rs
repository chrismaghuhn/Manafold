use thiserror::Error;

#[derive(Debug, Error)]
pub enum ControllerError {
    #[error("unknown player")]
    UnknownPlayer,
    #[error("controller lock is poisoned")]
    Poisoned,
    #[error("checkpoint is invalid: {0}")]
    InvalidCheckpoint(String),
    #[error("backend failure: {0}")]
    Backend(String),
}
