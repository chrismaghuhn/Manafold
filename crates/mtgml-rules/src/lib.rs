//! Magic rules: the native turn progression, basic-land actions, zone
//! incarnation, characteristic queries, and authoritative V3 events.

mod basic_land;
mod characteristic_query;
mod errors;
mod events;
mod game_start;
pub use game_start::{start_game, GameStartError, STARTING_LIFE};
mod snapshots;
mod turn_progression;
mod turn_structure;
mod validation;
mod zone_incarnation;

#[cfg(test)]
mod tests;

pub use basic_land::{
    derive_basic_land_candidates, execute_basic_land_response, install_basic_land_request,
    selected_basic_land_action, validate_basic_land_pending_request, BasicLandCandidateError,
    BasicLandTransitionError, BasicLandTransitionProduct, MagicActionRequestV1,
    SelectedSuccessorDecisionV1,
};
pub(crate) use characteristic_query::{S1QueryAuthority, S1QueryError};
pub(crate) use errors::KernelExecutionError;
pub use events::{
    allocate_rule_events, validate_event_delta_parity, validate_event_delta_state,
    validate_rule_event_cursor, AuthoritativeRuleEvent, AuthoritativeRuleEventKind,
    EventDeltaError, PerspectiveObservationPolicyV1, RuleEventCursorError,
};
pub use turn_progression::{execute_magic_response, validate_magic_pending_request};
pub use turn_structure::temporal_successor;
pub(crate) use validation::TransitionViolation;
