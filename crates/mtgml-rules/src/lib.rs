//! Magic rules: the native turn progression, basic-land actions, zone
//! incarnation, characteristic queries, and authoritative V3 events.

mod basic_land;
mod basic_land_v4;
mod characteristic_query;
mod errors;
mod events;
mod events_v3;
mod snapshots;
mod turn_progression;
mod turn_structure;
mod validation;
mod zone_incarnation;

#[cfg(test)]
mod tests;

pub use basic_land::{
    derive_basic_land_candidates, AuthoritativeRuleEventKindV2, AuthoritativeRuleEventV2,
    BasicLandCandidateError, BasicLandFaceV1, BasicLandTransitionError, MagicActionRequestV1,
    SelectedSuccessorDecisionV1, SuccessorObservationPolicyV1,
};
pub use basic_land_v4::{
    derive_basic_land_candidates_v4, execute_basic_land_response_v4, install_basic_land_request_v4,
    selected_basic_land_action_v4, validate_basic_land_pending_request_v4,
    BasicLandTransitionProductV4,
};
pub(crate) use characteristic_query::{S1QueryAuthority, S1QueryError};
pub use errors::{KernelExecutionError, ZoneIncarnationError};
pub use events::{
    AuthoritativeRuleEvent, AuthoritativeRuleEventKind, PerspectiveObservationPolicyV1,
};
pub use events_v3::{
    allocate_rule_events_v3, validate_event_delta_parity_v3, validate_event_delta_state_v3,
    validate_rule_event_cursor_v3, AuthoritativeRuleEventKindV3, AuthoritativeRuleEventV3,
    EventDeltaV3Error, RuleEventCursorV3Error,
};
pub use turn_progression::{execute_magic_response_v4, validate_magic_pending_request_v4};
pub use turn_structure::{temporal_successor, TurnStructureError, UnsupportedRulesBoundary};
pub use validation::TransitionViolation;
