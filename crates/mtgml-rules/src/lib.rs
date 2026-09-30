//! Authoritative events and exact, compositional transition validation.

mod basic_land;
mod basic_land_v4;
mod basic_priority;
mod characteristic_query;
mod combat_damage;
mod contract;
mod decision_stage;
mod errors;
mod events;
mod events_v3;
#[cfg(feature = "synthetic-conformance-fixtures")]
pub mod fixture_support;
mod magic;
mod product;
mod program_kernel;
mod semantic_cursor;
mod semantic_execution_generated;
mod snapshots;
mod state_based_actions;
mod successor_contract;
mod synthetic;
mod transition;
mod turn_progression;
mod turn_structure;
mod validation;
mod zone_incarnation;

#[cfg(test)]
mod tests;

pub use basic_land::{
    derive_basic_land_candidates, validate_basic_land_pending_request,
    AuthoritativeRuleEventKindV2, AuthoritativeRuleEventV2, BasicLandCandidateError,
    BasicLandFaceV1, BasicLandTransitionError, BasicLandTransitionProductV1, MagicActionRequestV1,
    SelectedSuccessorDecisionV1, SuccessorObservationPolicyV1,
};
pub use basic_land_v4::{
    derive_basic_land_candidates_v4, execute_basic_land_response_v4, install_basic_land_request_v4,
    selected_basic_land_action_v4, validate_basic_land_pending_request_v4,
    BasicLandTransitionProductV4,
};
pub(crate) use characteristic_query::{S1QueryAuthority, S1QueryError};
pub use contract::validate_transition_contract;
pub use errors::{KernelExecutionError, ZoneIncarnationError};
pub use events::{
    AuthoritativeRuleEvent, AuthoritativeRuleEventKind, OccurrencePairingError,
    PerspectiveObservationPolicyV1,
};
pub use events_v3::{
    allocate_rule_events_v3, validate_event_delta_parity_v3, validate_event_delta_state_v3,
    validate_rule_event_cursor_v3, AuthoritativeRuleEventKindV3, AuthoritativeRuleEventV3,
    EventDeltaV3Error, RuleEventCursorV3Error,
};
pub use program_kernel::{validate_runtime_state, validate_runtime_state_for_contract};
#[cfg(any(test, feature = "historical-runtime-testkit"))]
pub use program_kernel::{ProgramKernelConstructionErrorV1, ProgramKernelV1};
pub use semantic_execution_generated::execution_contract_supported;
#[cfg(feature = "magic-conformance-testkit")]
pub use state_based_actions::SbaContinuationValidationError;
pub use successor_contract::validate_successor_transition_contract;
pub use synthetic::validate_synthetic_runtime_state;
pub use transition::{PredecessorTransitionResult, RulesKernel, TransitionResult};
pub use turn_progression::{execute_magic_response_v4, validate_magic_pending_request_v4};
pub use turn_structure::{
    temporal_successor, unsupported_rules_boundary, validate_turn_structure_support,
    TurnStructureError, TurnStructureSupportProfile, UnsupportedRulesBoundary,
};
pub use validation::TransitionViolation;

#[cfg(feature = "magic-conformance-testkit")]
pub use zone_incarnation::{
    execute_selected_zone_transition_for_conformance, ConformanceZoneTransitionKind,
};
