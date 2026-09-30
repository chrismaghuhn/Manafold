use thiserror::Error;

#[derive(Debug, Error)]
pub enum TransitionViolation {
    #[error("object trace does not compose to the final state")]
    ObjectTraceIncomplete,
}
