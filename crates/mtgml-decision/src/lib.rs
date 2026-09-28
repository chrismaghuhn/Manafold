//! Perspective-safe public decisions and exact authoritative bindings.

mod authoritative;
mod common;
mod error;
mod ordering;
mod v1;
mod v2;
mod v3;
mod v4;

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
    CandidateIntentV3, DecisionResponseV3, EngineCandidateBindingV3, PlayerDecisionRequestV3,
    VisibleCandidateV3, DECISION_RESPONSE_V3_SCHEMA, PLAYER_DECISION_REQUEST_V3_SCHEMA,
};
pub use v4::{
    AuthoritativeCandidateV4, AuthoritativeDecisionRequestV4, CandidateIntentV4,
    CandidateOrderingV3, CostFactsV1, CostOperandOperationV1, CostRouteV1, CounterKindV1,
    DamageKindV1, DecisionPurposeV4, EngineCandidateBindingV4, LifeChangeCauseV1,
    ManaSourceActivationCostV1, PlayerDecisionRequestV4, SafeAttackerFactV1, SafeDamageRecipientV1,
    SafeTargetDescriptorV1, SafeTriggerDescriptorV1, SafeTriggerSubjectV1, SafeZoneKindV1,
    SyntheticAssemblyStageV1, TriggerEventKindV1, VisibleCandidateV4,
    PLAYER_DECISION_REQUEST_V4_SCHEMA,
};

#[cfg(test)]
mod tests;

#[cfg(test)]
#[path = "tests/batch_f.rs"]
mod batch_f;
