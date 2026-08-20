//! Authoritative events and exact, compositional transition validation.

mod contract;
mod errors;
mod events;
mod semantic_cursor;
mod snapshots;
mod synthetic;
mod transition;
mod validation;

#[cfg(test)]
mod tests;

pub use contract::validate_transition_contract;
pub use errors::KernelExecutionError;
pub use events::{AuthoritativeRuleEvent, AuthoritativeRuleEventKind};
pub use synthetic::SyntheticM1RulesKernel;
pub use transition::{RulesKernel, TransitionResult};
pub use validation::TransitionViolation;
