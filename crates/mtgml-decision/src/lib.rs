//! Perspective-safe public decisions and exact authoritative bindings.

mod authoritative;
mod common;
mod error;
mod ordering;
mod v1;
mod v2;
mod v3;

pub use authoritative::{
    validate_candidate_binding, AuthoritativeCandidateV2, AuthoritativeDecisionRequestV2,
    EngineCandidateBinding, PerspectiveIdentityResolver,
};
pub use common::{CandidateIntent, DecisionVisibility};
pub use error::{CandidateBindingError, DecisionValidationError};
pub use ordering::CandidateOrderingV1;
pub use ordering::CandidateOrderingV2;
pub use v1::{
    ActionCandidate, CandidateAssignment, DecisionKind, DecisionResponse, PlayerDecisionRequest,
    DECISION_RESPONSE_SCHEMA, PLAYER_DECISION_REQUEST_SCHEMA,
};
pub use v2::{
    DecisionAnswerV2, DecisionDomainV2, DecisionResponseV2, PlayerDecisionRequestV2,
    VisibleCandidateV2, DECISION_RESPONSE_V2_SCHEMA, PLAYER_DECISION_REQUEST_V2_SCHEMA,
};
pub use v3::{
    validate_candidate_binding_v3, AuthoritativeCandidateV3, AuthoritativeDecisionRequestV3,
    CandidateIntentV3, EngineCandidateBindingV3, PlayerDecisionRequestV3, VisibleCandidateV3,
    PLAYER_DECISION_REQUEST_V3_SCHEMA,
};

#[cfg(test)]
mod tests;

#[cfg(test)]
#[path = "tests/batch_f.rs"]
mod batch_f;
