//! Authoritative events and exact, compositional transition validation.

mod basic_priority;
mod contract;
mod decision_stage;
mod errors;
mod events;
#[cfg(feature = "synthetic-conformance-fixtures")]
pub mod fixture_support;
mod magic;
mod product;
mod program_kernel;
mod semantic_cursor;
mod semantic_execution_generated;
mod snapshots;
mod state_based_actions;
mod synthetic;
mod transition;
mod turn_structure;
mod validation;
mod zone_incarnation;

#[cfg(test)]
mod tests;

pub use contract::validate_transition_contract;
pub use errors::{KernelExecutionError, ZoneIncarnationError};
pub use events::{
    AuthoritativeRuleEvent, AuthoritativeRuleEventKind, OccurrencePairingError,
    PerspectiveObservationPolicyV1,
};
pub use program_kernel::{
    validate_runtime_state, validate_runtime_state_for_contract, ProgramKernelConstructionErrorV1,
    ProgramKernelV1,
};
pub use semantic_execution_generated::execution_contract_supported;
#[cfg(feature = "magic-conformance-testkit")]
pub use state_based_actions::SbaContinuationValidationError;
pub use synthetic::validate_synthetic_runtime_state;
pub use transition::{RulesKernel, TransitionResult};
pub use turn_structure::{
    temporal_successor, unsupported_rules_boundary, validate_turn_structure_support,
    TurnStructureError, TurnStructureSupportProfile, UnsupportedRulesBoundary,
};
pub use validation::TransitionViolation;

#[cfg(feature = "magic-conformance-testkit")]
pub use zone_incarnation::{
    execute_selected_zone_transition_for_conformance, ConformanceZoneTransitionKind,
};
