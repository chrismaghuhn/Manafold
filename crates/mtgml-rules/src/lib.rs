//! Authoritative events and exact, compositional transition validation.

mod contract;
mod errors;
mod events;
#[cfg(feature = "m2-conformance-fixtures")]
pub mod fixture_support;
mod product;
mod program_kernel;
mod semantic_cursor;
mod snapshots;
mod synthetic;
mod transition;
mod validation;

#[cfg(test)]
mod tests;

pub use contract::validate_transition_contract;
pub use errors::KernelExecutionError;
pub use events::{
    AuthoritativeRuleEvent, AuthoritativeRuleEventKind, OccurrencePairingError,
    PerspectiveObservationPolicyV1,
};
pub use program_kernel::{
    validate_runtime_state, ProgramKernelConstructionErrorV1, ProgramKernelV1,
};
pub use synthetic::validate_synthetic_runtime_state;
pub use transition::{RulesKernel, TransitionResult};
pub use validation::TransitionViolation;
