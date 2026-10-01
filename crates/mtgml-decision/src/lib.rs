//! Perspective-safe public decisions and exact authoritative bindings.

mod answer;
mod common;
mod error;
mod ordering;
mod response;
mod v4;

pub use answer::{DecisionAnswerV2, DecisionDomainV2};
pub use common::DecisionVisibility;
pub use error::DecisionValidationError;
pub use ordering::CandidateOrdering;
pub use response::{DecisionResponseV3, DECISION_RESPONSE_V3_SCHEMA};
pub use v4::{
    AuthoritativeCandidate, AuthoritativeDecisionRequest, CandidateIntent, CostFactsV1,
    CostOperandOperationV1, CostRouteClassV1, CostRouteDescriptorV1, CostRouteV1, CounterKindV1,
    DamageKindV1, DecisionPurposeV4, EngineCandidateBinding, LifeChangeCauseV1,
    ManaSourceActivationCostV1, PlayerDecisionRequestV4, PrintedManaSymbolsV1, SafeAttackerFactV1,
    SafeDamageRecipientV1, SafeTargetDescriptorV1, SafeTriggerDescriptorV1, SafeTriggerSubjectV1,
    SafeZoneKindV1, SyntheticAssemblyStageV1, TriggerEventKindV1, VisibleCandidate,
    PLAYER_DECISION_REQUEST_V4_SCHEMA,
};

#[cfg(test)]
mod tests;

#[cfg(test)]
#[path = "tests/batch_f.rs"]
mod batch_f;
