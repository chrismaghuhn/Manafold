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

mod diagnostics;

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

    #[test]
    fn current_decision_failure_exposes_a_semantic_path() {
        let expected = decision(1);
        let actual = decision(2);
        let difference = crate::diagnostics::compare_value(
            crate::diagnostics::ConformanceFailureClass::CurrentDecision,
            "current_decision",
            &expected,
            &actual,
        )
        .expect("different current decisions must fail");

        assert_eq!(difference.semantic_path, "current_decision");
        assert_eq!(
            difference.mismatch_kind,
            crate::diagnostics::ConformanceMismatchKind::ValueChanged
        );
    }

    #[test]
    fn event_difference_reports_the_first_nonzero_index() {
        let difference = crate::diagnostics::compare_sequence(
            crate::diagnostics::ConformanceFailureClass::Events,
            "transition.events",
            &[10_u8, 20, 30, 40],
            &[10_u8, 20, 31, 40],
        )
        .expect("difference");

        assert_eq!(difference.semantic_path, "transition.events[2]");
        assert_eq!(
            difference.mismatch_kind,
            crate::diagnostics::ConformanceMismatchKind::ValueChanged
        );
        assert_eq!(difference.sequence.unwrap().first_differing_index, 2);
    }

    #[test]
    fn event_missing_at_end_is_distinct_from_an_extra_event() {
        let missing = crate::diagnostics::compare_sequence(
            crate::diagnostics::ConformanceFailureClass::Events,
            "transition.events",
            &[1_u8, 2, 3],
            &[1_u8, 2],
        )
        .expect("missing difference");
        let extra = crate::diagnostics::compare_sequence(
            crate::diagnostics::ConformanceFailureClass::Events,
            "transition.events",
            &[1_u8, 2],
            &[1_u8, 2, 3],
        )
        .expect("extra difference");

        assert_eq!(
            missing.mismatch_kind,
            crate::diagnostics::ConformanceMismatchKind::ExpectedEntryMissing
        );
        assert_eq!(
            extra.mismatch_kind,
            crate::diagnostics::ConformanceMismatchKind::UnexpectedExtraEntry
        );
        assert_eq!(missing.sequence.unwrap().first_differing_index, 2);
        assert_eq!(extra.sequence.unwrap().first_differing_index, 2);
    }

    #[test]
    fn delta_difference_uses_the_delta_audit_path() {
        let difference = crate::diagnostics::compare_sequence(
            crate::diagnostics::ConformanceFailureClass::Delta,
            "transition.delta.audit",
            &[1_u8, 2, 3],
            &[1_u8, 9, 3],
        )
        .expect("difference");

        assert_eq!(difference.semantic_path, "transition.delta.audit[1]");
    }

    #[test]
    fn scalar_and_player_differences_are_typed() {
        let next = crate::diagnostics::compare_value(
            crate::diagnostics::ConformanceFailureClass::NextDecision,
            "transition.next_decision",
            &Some(1_u8),
            &None,
        )
        .expect("next decision difference");
        assert_eq!(
            next.mismatch_kind,
            crate::diagnostics::ConformanceMismatchKind::ValueChanged
        );

        let status = crate::diagnostics::compare_value(
            crate::diagnostics::ConformanceFailureClass::Status,
            "transition.status",
            &1_u8,
            &2_u8,
        )
        .expect("status difference");
        assert_eq!(status.semantic_path, "transition.status");

        let expected = BTreeMap::from([(PlayerId(1), 10_u8), (PlayerId(2), 20_u8)]);
        let actual = BTreeMap::from([(PlayerId(1), 11_u8), (PlayerId(2), 20_u8)]);
        let player =
            crate::diagnostics::compare_player_map(&expected, &actual).expect("player difference");
        assert_eq!(player.semantic_path, "player_steps[player:1]");
        assert_eq!(
            player.mismatch_kind,
            crate::diagnostics::ConformanceMismatchKind::PlayerStepDiffered
        );
    }

    #[test]
    fn player_map_reports_missing_and_unexpected_players() {
        let missing = crate::diagnostics::compare_player_map(
            &BTreeMap::from([(PlayerId(1), 10_u8)]),
            &BTreeMap::new(),
        )
        .expect("missing player");
        let unexpected = crate::diagnostics::compare_player_map(
            &BTreeMap::new(),
            &BTreeMap::from([(PlayerId(2), 20_u8)]),
        )
        .expect("unexpected player");

        assert_eq!(
            missing.mismatch_kind,
            crate::diagnostics::ConformanceMismatchKind::PlayerMissing
        );
        assert_eq!(
            unexpected.mismatch_kind,
            crate::diagnostics::ConformanceMismatchKind::UnexpectedPlayer
        );
        assert_eq!(missing.semantic_path, "player_steps[player:1]");
        assert_eq!(unexpected.semantic_path, "player_steps[player:2]");
    }

    #[test]
    fn rejected_mutation_has_a_distinct_diagnostic_classification() {
        let difference =
            crate::diagnostics::rejected_mutation_difference(true).expect("changed rejected state");

        assert_eq!(
            difference.surface,
            crate::diagnostics::ConformanceFailureClass::RejectedMutation
        );
        assert_eq!(
            difference.mismatch_kind,
            crate::diagnostics::ConformanceMismatchKind::RejectedMutation
        );
        assert_eq!(difference.semantic_path, "rejected_mutation.next_state");
        assert!(crate::diagnostics::rejected_mutation_difference(false).is_none());
    }

    #[test]
    fn first_difference_preserves_declared_precedence() {
        let event = crate::diagnostics::compare_value(
            crate::diagnostics::ConformanceFailureClass::Events,
            "transition.events[0]",
            &1_u8,
            &2_u8,
        );
        let status = crate::diagnostics::compare_value(
            crate::diagnostics::ConformanceFailureClass::Status,
            "transition.status",
            &1_u8,
            &2_u8,
        );

        let first =
            crate::diagnostics::first_difference([event, status]).expect("first difference");
        assert_eq!(
            first.surface,
            crate::diagnostics::ConformanceFailureClass::Events
        );
    }

    #[test]
    fn signature_marker_contains_only_structured_identity_fields() {
        let mut first = crate::diagnostics::compare_value(
            crate::diagnostics::ConformanceFailureClass::Events,
            "transition.events[2]",
            &"expected summary",
            &"actual summary",
        )
        .expect("difference");
        let marker = first.signature_marker();
        first.expected_summary = "ROOT_SEED_SECRET_SENTINEL".into();
        first.actual_summary = "PRIVATE_HAND_SENTINEL".into();

        assert_eq!(marker, first.signature_marker());
        assert_eq!(
            marker,
            "MANAFOLD_FAILURE_SIGNATURE v1 surface=events path=transition.events[2] mismatch_kind=value_changed"
        );
    }
}
