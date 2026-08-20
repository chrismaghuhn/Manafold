use mtgml_state::{EngineStateViolation, StateDigestError};
use thiserror::Error;

use crate::TransitionViolation;

#[derive(Debug, Error)]
pub enum KernelExecutionError {
    #[error("before state is invalid: {0}")]
    BeforeState(EngineStateViolation),
    #[error("revision would overflow")]
    RevisionOverflow,
    #[error("rule event identity would overflow")]
    RuleEventIdOverflow,
    #[error("state delta construction failed: {0}")]
    Delta(StateDigestError),
    #[error("after state is invalid: {0}")]
    AfterState(EngineStateViolation),
    #[error("transition contract failed: {0}")]
    TransitionContract(TransitionViolation),
}
