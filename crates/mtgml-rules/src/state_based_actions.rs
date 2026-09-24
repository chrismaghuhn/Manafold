//! RED seam for the Rules-owned typed SBA continuation validator.
//!
//! Task 6 first pins a read-only entry point. The semantic derivation is added
//! in the following implementation commit.

use mtgml_state::EngineState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum SbaContinuationValidationError {
    NotS3AConformanceCandidate,
    NoActiveSbaContinuation,
    SemanticPlanValidationNotImplemented,
    UnsupportedSbaProfile,
    SelectedActionSetMismatch,
    ApnapOwnersMismatch,
}

pub(crate) fn validate_sba_order_continuation(
    _state: &EngineState,
) -> Result<(), SbaContinuationValidationError> {
    Err(SbaContinuationValidationError::SemanticPlanValidationNotImplemented)
}
