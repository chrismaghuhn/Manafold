//! Conformance-only isolation-harness primitives for the M2.G gates.
//!
//! Fingerprints, authorization relations, and the runtime-acceptance pair
//! builder consumed by the paired-state evidence slices. Nothing here is a
//! gate verdict: every M2.G gate remains `NOT_RUN` until its slice executes.
//! Production crates must never depend on this module; the oracle-boundary
//! guard enforces that direction.

mod fingerprint;
mod paired;
mod witnesses;

pub use fingerprint::{
    assert_fingerprint_policies, capture_complete, capture_snapshot, capture_transition_product,
    CompleteM2Fingerprint, EnvironmentFingerprint, FingerprintComparison,
    PlayerProtocolIdentitySurface, PlayerVisibleFingerprint, PlayerVisibleSnapshot,
    ReplayRecorderFingerprint, SemanticStateFingerprint, TransitionVisibleProduct,
    TrustedEnvironmentIdentitySurface,
};
pub use paired::{
    base_pair_state, build_case, spawn_environment, synthetic_environment_config, AxisKind,
    PairedCase, TransformFn, TransformReport,
};
pub use witnesses::{
    assert_witness, check_bijection, relate_decision, relate_knowledge, BijectionOutcome,
    NonVacuityPredicate, PairWitness, RelationOutcome, TrustedRenamingBijection, WitnessViolation,
};

use thiserror::Error;

/// Closed failure vocabulary of the isolation harness.
///
/// Variants are string-free; the underlying typed failures remain reachable
/// where diagnostics demand them.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum HarnessError {
    #[error("player endpoint reported service unavailability")]
    EndpointService,
    #[error("trusted controller operation failed")]
    ControllerService,
    #[error("canonical wire encoding failed")]
    WireEncoding,
    #[error("recomputed information-state digest differs from the persisted digest")]
    InformationDigestMismatch,
    #[error("root seed hex is invalid")]
    SeedFormat,
    #[error("synthetic engine state construction failed")]
    SyntheticConstruction,
    #[error("engine state failed validation: {0:?}")]
    StateValidation(mtgml_state::EngineStateViolation),
    #[error("checkpoint creation or validation failed")]
    CheckpointInvalid,
    #[error("synthetic backend rejected the checkpoint")]
    SyntheticBackendRejected,
    #[error("player binding failed")]
    BindFailed,
    #[error("semantic-state fingerprint group mismatched")]
    SemanticGroupMismatch,
    #[error("environment fingerprint group mismatched")]
    EnvironmentGroupMismatch,
    #[error("player-visible fingerprint group mismatched")]
    PlayerGroupMismatch,
    #[error("replay-recorder fingerprint group mismatched")]
    ReplayRecorderGroupMismatch,
}
