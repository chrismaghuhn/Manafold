//! Perspective-safe public decisions and exact authoritative bindings.

mod common;
mod error;
mod ordering;
mod v2;
mod v3;
mod v4;

pub use common::DecisionVisibility;
pub use error::DecisionValidationError;
pub use v2::{DecisionAnswerV2, DecisionDomainV2};
pub use v3::{DecisionResponseV3, DECISION_RESPONSE_V3_SCHEMA};
pub use v4::{
    AuthoritativeCandidateV4, AuthoritativeDecisionRequestV4, CandidateIntentV4,
    CandidateOrderingV3, CostFactsV1, CostOperandOperationV1, CostRouteClassV1,
    CostRouteDescriptorV1, CostRouteV1, CounterKindV1, DamageKindV1, DecisionPurposeV4,
    EngineCandidateBindingV4, LifeChangeCauseV1, ManaSourceActivationCostV1,
    PlayerDecisionRequestV4, PrintedManaSymbolsV1, SafeAttackerFactV1, SafeDamageRecipientV1,
    SafeTargetDescriptorV1, SafeTriggerDescriptorV1, SafeTriggerSubjectV1, SafeZoneKindV1,
    SyntheticAssemblyStageV1, TriggerEventKindV1, VisibleCandidateV4,
    PLAYER_DECISION_REQUEST_V4_SCHEMA,
};

#[cfg(test)]
mod tests;

#[cfg(test)]
#[path = "tests/batch_f.rs"]
mod batch_f;
