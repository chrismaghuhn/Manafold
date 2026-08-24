//! Exact per-step conformance assertions for the current V2/V3 runtime.
//!
//! Event counts are diagnostic only.  The authoritative assertions are the
//! transition product, V3 state identity, delta, next trusted request, and
//! player projections supplied by the caller.

use mtgml_decision::{AuthoritativeDecisionRequestV2, DecisionResponseV2};
use mtgml_model::{EpisodeStatus, FullStateDigestV3, PlayerId};
use mtgml_observation::PlayerStepV2;
use mtgml_rules::{validate_transition_contract, AuthoritativeRuleEvent, TransitionResult};
use mtgml_state::{EngineState, SemanticDeltaOperation};
use std::collections::BTreeMap;
use thiserror::Error;

pub mod isolation;
pub mod legal_space;
pub mod lifecycle;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpectedResponseResult {
    Accepted,
    RejectedWithoutMutation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConformanceStep {
    pub expected_current_decision: Option<AuthoritativeDecisionRequestV2>,
    pub response: DecisionResponseV2,
    pub expected_response_result: ExpectedResponseResult,
    pub expected_authoritative_events: Vec<AuthoritativeRuleEvent>,
    pub expected_semantic_delta: Vec<SemanticDeltaOperation>,
    pub expected_state_digest: FullStateDigestV3,
    pub expected_next_decision: Option<AuthoritativeDecisionRequestV2>,
    pub expected_player_steps: BTreeMap<PlayerId, PlayerStepV2>,
    pub expected_status: EpisodeStatus,
}

pub fn assert_exact_transition(
    before: &EngineState,
    actual_current_decision: Option<&AuthoritativeDecisionRequestV2>,
    actual_response: &DecisionResponseV2,
    result: &TransitionResult,
    actual_player_steps: &BTreeMap<PlayerId, PlayerStepV2>,
    expected: &ConformanceStep,
) -> Result<(), ConformanceFailure> {
    assert_conformance_inputs(
        actual_current_decision,
        actual_response,
        expected.expected_current_decision.as_ref(),
        &expected.response,
    )?;
    validate_transition_contract(before, result)
        .map_err(|error| ConformanceFailure::Contract(error.to_string()))?;
    let accepted = matches!(
        expected.expected_response_result,
        ExpectedResponseResult::Accepted
    );
    if result.accepted != accepted {
        return Err(ConformanceFailure::Acceptance);
    }
    if result.events != expected.expected_authoritative_events {
        return Err(ConformanceFailure::Events);
    }
    if result.delta.audit != expected.expected_semantic_delta {
        return Err(ConformanceFailure::Delta);
    }
    if result
        .next_state
        .digest()
        .map_err(|_| ConformanceFailure::StateDigest)?
        != expected.expected_state_digest
    {
        return Err(ConformanceFailure::StateDigest);
    }
    if result.next_decision != expected.expected_next_decision {
        return Err(ConformanceFailure::NextDecision);
    }
    if result.status != expected.expected_status {
        return Err(ConformanceFailure::Status);
    }
    if actual_player_steps != &expected.expected_player_steps {
        return Err(ConformanceFailure::PlayerProjection);
    }
    if matches!(
        expected.expected_response_result,
        ExpectedResponseResult::RejectedWithoutMutation
    ) && &result.next_state != before
    {
        return Err(ConformanceFailure::RejectedMutation);
    }
    Ok(())
}

fn assert_conformance_inputs(
    actual_current_decision: Option<&AuthoritativeDecisionRequestV2>,
    actual_response: &DecisionResponseV2,
    expected_current_decision: Option<&AuthoritativeDecisionRequestV2>,
    expected_response: &DecisionResponseV2,
) -> Result<(), ConformanceFailure> {
    if actual_current_decision != expected_current_decision {
        return Err(ConformanceFailure::CurrentDecision);
    }
    if actual_response != expected_response {
        return Err(ConformanceFailure::Response);
    }
    Ok(())
}

/// Diagnostic helper only; never a correctness gate.
pub fn minimum_event_count_diagnostic(events: &[AuthoritativeRuleEvent], minimum: usize) -> bool {
    events.len() >= minimum
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ConformanceFailure {
    #[error("transition contract failed: {0}")]
    Contract(String),
    #[error("current trusted decision differed")]
    CurrentDecision,
    #[error("submitted response differed")]
    Response,
    #[error("response acceptance differed")]
    Acceptance,
    #[error("authoritative events differed")]
    Events,
    #[error("semantic delta differed")]
    Delta,
    #[error("state digest differed")]
    StateDigest,
    #[error("next trusted decision differed")]
    NextDecision,
    #[error("episode status differed")]
    Status,
    #[error("per-player projection differed")]
    PlayerProjection,
    #[error("rejected response mutated state")]
    RejectedMutation,
}

#[cfg(test)]
mod tests {
    use super::*;
    use mtgml_decision::{
        DecisionAnswerV2, DecisionDomainV2, DecisionResponseV2, DecisionVisibility,
    };
    use mtgml_model::{DecisionId, PlayerDecisionIdV1, StateRevision};

    fn decision(id: u64) -> AuthoritativeDecisionRequestV2 {
        AuthoritativeDecisionRequestV2 {
            decision_id: DecisionId(id),
            player_decision_id: PlayerDecisionIdV1(id),
            state_revision: StateRevision(0),
            actor: PlayerId(1),
            visibility: DecisionVisibility::Public,
            decision: DecisionDomainV2::ChooseNumber {
                minimum: 0,
                maximum: 0,
            },
            candidates: vec![],
            continuation_id: None,
        }
    }

    fn response(id: u64) -> DecisionResponseV2 {
        DecisionResponseV2 {
            schema_version: "decision-response.v2".into(),
            player_decision_id: PlayerDecisionIdV1(id),
            state_revision: StateRevision(0),
            answer: DecisionAnswerV2::ChooseNumber { value: 0 },
        }
    }

    #[test]
    fn current_decision_is_an_asserted_conformance_input() {
        let expected_decision = decision(1);
        let actual_decision = decision(2);
        let submitted = response(1);
        assert_eq!(
            assert_conformance_inputs(
                Some(&actual_decision),
                &submitted,
                Some(&expected_decision),
                &submitted,
            ),
            Err(ConformanceFailure::CurrentDecision)
        );
    }

    #[test]
    fn submitted_response_is_an_asserted_conformance_input() {
        let visible = decision(1);
        let expected_response = response(1);
        let actual_response = response(2);
        assert_eq!(
            assert_conformance_inputs(
                Some(&visible),
                &actual_response,
                Some(&visible),
                &expected_response,
            ),
            Err(ConformanceFailure::Response)
        );
    }
}
