//! Exact per-step conformance assertions for the current V2/V3 runtime.
//!
//! Event counts are diagnostic only.  The authoritative assertions are the
//! transition product, V3 state identity, delta, next trusted request, and
//! player projections supplied by the caller.

use mtgml_decision::{AuthoritativeDecisionRequestV2, DecisionResponseV2};
use mtgml_model::{EpisodeStatus, FullStateDigestV4, PlayerId};
use mtgml_observation::PlayerStepV2;
use mtgml_rules::{validate_transition_contract, AuthoritativeRuleEvent, TransitionResult};
use mtgml_state::{EngineState, SemanticDeltaOperation};
use std::collections::BTreeMap;
use thiserror::Error;

mod diagnostics;

/// T0 conformance facade contract (M3.T0-01, RED): case-level surface
/// requirements for the thin private conformance facade.  Intentionally
/// unimplemented; the tests in this module fail until the reviewed facade
/// slice exists.
#[cfg(test)]
mod facade;

pub use diagnostics::{
    ConformanceDifference, ConformanceFailureClass, ConformanceMismatchKind, SequenceDifference,
};

pub mod isolation;
pub mod legal_space;
pub mod lifecycle;

#[cfg(test)]
mod turn_structure;

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
    pub expected_state_digest: FullStateDigestV4,
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
    if let Some(difference) = diagnostics::compare_value(
        ConformanceFailureClass::Acceptance,
        "transition.acceptance",
        &accepted,
        &result.accepted,
    ) {
        return Err(ConformanceFailure::Detailed {
            classification: ConformanceFailureClass::Acceptance,
            difference,
        });
    }
    if let Some(difference) = diagnostics::compare_sequence(
        ConformanceFailureClass::Events,
        "transition.events",
        &expected.expected_authoritative_events,
        &result.events,
    ) {
        return Err(ConformanceFailure::Detailed {
            classification: ConformanceFailureClass::Events,
            difference,
        });
    }
    if let Some(difference) = diagnostics::compare_sequence(
        ConformanceFailureClass::Delta,
        "transition.delta.audit",
        &expected.expected_semantic_delta,
        &result.delta.audit,
    ) {
        return Err(ConformanceFailure::Detailed {
            classification: ConformanceFailureClass::Delta,
            difference,
        });
    }
    let actual_state_digest = result
        .next_state
        .digest()
        .map_err(|_| ConformanceFailure::StateDigest)?;
    if let Some(difference) = diagnostics::compare_value(
        ConformanceFailureClass::StateDigest,
        "transition.state_digest",
        &expected.expected_state_digest,
        &actual_state_digest,
    ) {
        return Err(ConformanceFailure::Detailed {
            classification: ConformanceFailureClass::StateDigest,
            difference,
        });
    }
    if let Some(difference) = diagnostics::compare_value(
        ConformanceFailureClass::NextDecision,
        "transition.next_decision",
        &expected.expected_next_decision,
        &result.next_decision,
    ) {
        return Err(ConformanceFailure::Detailed {
            classification: ConformanceFailureClass::NextDecision,
            difference,
        });
    }
    if let Some(difference) = diagnostics::compare_value(
        ConformanceFailureClass::Status,
        "transition.status",
        &expected.expected_status,
        &result.status,
    ) {
        return Err(ConformanceFailure::Detailed {
            classification: ConformanceFailureClass::Status,
            difference,
        });
    }
    if let Some(difference) =
        diagnostics::compare_player_map(&expected.expected_player_steps, actual_player_steps)
    {
        return Err(ConformanceFailure::Detailed {
            classification: ConformanceFailureClass::PlayerProjection,
            difference,
        });
    }
    if matches!(
        expected.expected_response_result,
        ExpectedResponseResult::RejectedWithoutMutation
    ) {
        if let Some(difference) =
            diagnostics::rejected_mutation_difference(&result.next_state != before)
        {
            return Err(ConformanceFailure::Detailed {
                classification: ConformanceFailureClass::RejectedMutation,
                difference,
            });
        }
    }
    Ok(())
}

fn assert_conformance_inputs(
    actual_current_decision: Option<&AuthoritativeDecisionRequestV2>,
    actual_response: &DecisionResponseV2,
    expected_current_decision: Option<&AuthoritativeDecisionRequestV2>,
    expected_response: &DecisionResponseV2,
) -> Result<(), ConformanceFailure> {
    if let Some(difference) = diagnostics::first_difference([
        diagnostics::compare_value(
            ConformanceFailureClass::CurrentDecision,
            "current_decision",
            &expected_current_decision,
            &actual_current_decision,
        ),
        diagnostics::compare_value(
            ConformanceFailureClass::Response,
            "response",
            expected_response,
            actual_response,
        ),
    ]) {
        return Err(ConformanceFailure::Detailed {
            classification: difference.surface,
            difference,
        });
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
    #[error("conformance difference ({classification:?}): {difference}")]
    Detailed {
        classification: ConformanceFailureClass,
        difference: ConformanceDifference,
    },
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

impl ConformanceFailure {
    pub fn classification(&self) -> Option<ConformanceFailureClass> {
        match self {
            Self::Contract(_) => None,
            Self::Detailed { classification, .. } => Some(*classification),
            Self::CurrentDecision => Some(ConformanceFailureClass::CurrentDecision),
            Self::Response => Some(ConformanceFailureClass::Response),
            Self::Acceptance => Some(ConformanceFailureClass::Acceptance),
            Self::Events => Some(ConformanceFailureClass::Events),
            Self::Delta => Some(ConformanceFailureClass::Delta),
            Self::StateDigest => Some(ConformanceFailureClass::StateDigest),
            Self::NextDecision => Some(ConformanceFailureClass::NextDecision),
            Self::Status => Some(ConformanceFailureClass::Status),
            Self::PlayerProjection => Some(ConformanceFailureClass::PlayerProjection),
            Self::RejectedMutation => Some(ConformanceFailureClass::RejectedMutation),
        }
    }

    pub fn difference(&self) -> Option<&ConformanceDifference> {
        match self {
            Self::Detailed { difference, .. } => Some(difference),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mtgml_decision::{
        DecisionAnswerV2, DecisionDomainV2, DecisionResponseV2, DecisionVisibility,
    };
    use mtgml_model::{
        DecisionId, FullStateDigestV4, PlayerDecisionIdV1, PlayerOutcome, PlayerResult,
        StateRevision, TruncationReason,
    };

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

    fn lifecycle_conformance_case() -> (EngineState, TransitionResult, ConformanceStep) {
        let before = lifecycle::lifecycle_fixture();
        let result = lifecycle::scenario_public_fanout_two_reveals(&before).unwrap();
        let submitted = response(1);
        let expected = ConformanceStep {
            expected_current_decision: None,
            response: submitted,
            expected_response_result: ExpectedResponseResult::Accepted,
            expected_authoritative_events: result.events.clone(),
            expected_semantic_delta: result.delta.audit.clone(),
            expected_state_digest: result.next_state.digest().unwrap(),
            expected_next_decision: result.next_decision.clone(),
            expected_player_steps: BTreeMap::new(),
            expected_status: result.status.clone(),
        };
        (before, result, expected)
    }

    fn truncated_status() -> EpisodeStatus {
        EpisodeStatus::Truncated {
            reason: TruncationReason::ExternalStop,
            players: vec![
                PlayerOutcome {
                    player: PlayerId(1),
                    result: PlayerResult::Unresolved,
                },
                PlayerOutcome {
                    player: PlayerId(2),
                    result: PlayerResult::Unresolved,
                },
            ],
        }
    }

    fn player_step_fixture() -> PlayerStepV2 {
        serde_json::from_str(include_str!("../../../wire/golden/player-step.v2.json"))
            .expect("valid synthetic player-step fixture")
    }

    fn assert_detailed_failure(
        outcome: Result<(), ConformanceFailure>,
        classification: ConformanceFailureClass,
        path: &str,
        mismatch_kind: ConformanceMismatchKind,
    ) -> ConformanceDifference {
        let failure = outcome.expect_err("conformance mismatch must fail");
        assert_eq!(failure.classification(), Some(classification));
        let difference = failure.difference().expect("detailed conformance mismatch");
        assert_eq!(difference.semantic_path, path);
        assert_eq!(difference.mismatch_kind, mismatch_kind);
        difference.clone()
    }

    fn lifecycle_product_case() -> (
        EngineState,
        TransitionResult,
        Vec<AuthoritativeRuleEvent>,
        FullStateDigestV4,
    ) {
        let before = lifecycle::lifecycle_fixture();
        let result = lifecycle::scenario_explicit_forget(&before).unwrap();
        let expected_events = result.events.clone();
        let expected_digest = result.next_state.digest().unwrap();
        (before, result, expected_events, expected_digest)
    }

    #[derive(PartialEq)]
    struct SecretDiagnosticValue(&'static str);

    impl std::fmt::Debug for SecretDiagnosticValue {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(formatter, "SECRET_DIAGNOSTIC_SENTINEL({})", self.0)
        }
    }

    #[test]
    fn fnd_031_default_difference_does_not_render_debug_values() {
        let difference = crate::diagnostics::compare_value(
            crate::diagnostics::ConformanceFailureClass::StateDigest,
            "transition.secret",
            &SecretDiagnosticValue("root-seed"),
            &SecretDiagnosticValue("private-card"),
        )
        .expect("different values must produce a difference");
        let rendered = difference.to_string();
        assert!(rendered.contains("path=transition.secret"));
        assert!(rendered.contains("mismatch_kind=value_changed"));
        assert!(!rendered.contains("SECRET_DIAGNOSTIC_SENTINEL"));
        assert!(!rendered.contains("root-seed"));
        assert!(!rendered.contains("private-card"));
    }

    #[test]
    fn fnd_031_sequence_summaries_preserve_presence_shape_without_values() {
        let missing = crate::diagnostics::compare_sequence(
            crate::diagnostics::ConformanceFailureClass::Events,
            "transition.events",
            &[1_u8, 2, 3],
            &[1_u8, 2],
        )
        .expect("missing entry must produce a difference");
        assert_eq!(missing.expected_summary, "<present>");
        assert_eq!(missing.actual_summary, "<missing>");

        let extra = crate::diagnostics::compare_sequence(
            crate::diagnostics::ConformanceFailureClass::Events,
            "transition.events",
            &[1_u8, 2],
            &[1_u8, 2, 3],
        )
        .expect("extra entry must produce a difference");
        assert_eq!(extra.expected_summary, "<missing>");
        assert_eq!(extra.actual_summary, "<present>");
    }

    #[test]
    fn current_decision_is_an_asserted_conformance_input() {
        let expected_decision = decision(1);
        let actual_decision = decision(2);
        let submitted = response(1);
        let failure = assert_conformance_inputs(
            Some(&actual_decision),
            &submitted,
            Some(&expected_decision),
            &submitted,
        )
        .expect_err("different current decisions must fail");
        assert_eq!(
            failure.classification(),
            Some(ConformanceFailureClass::CurrentDecision)
        );
        assert_eq!(
            failure
                .difference()
                .expect("structured difference")
                .semantic_path,
            "current_decision"
        );
    }

    #[test]
    fn submitted_response_is_an_asserted_conformance_input() {
        let visible = decision(1);
        let expected_response = response(1);
        let actual_response = response(2);
        let failure = assert_conformance_inputs(
            Some(&visible),
            &actual_response,
            Some(&visible),
            &expected_response,
        )
        .expect_err("different responses must fail");
        assert_eq!(
            failure.classification(),
            Some(ConformanceFailureClass::Response)
        );
        assert_eq!(
            failure
                .difference()
                .expect("structured difference")
                .semantic_path,
            "response"
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

    #[test]
    fn exact_transition_event_failure_exposes_the_first_event_path() {
        let (before, result, mut expected) = lifecycle_conformance_case();
        assert!(
            result.events.len() > 3,
            "fixture needs a nonzero event index"
        );
        expected.expected_authoritative_events[3] =
            expected.expected_authoritative_events[0].clone();

        let failure = assert_exact_transition(
            &before,
            None,
            &expected.response,
            &result,
            &BTreeMap::new(),
            &expected,
        )
        .expect_err("different expected event must fail");

        assert_eq!(
            failure.classification(),
            Some(ConformanceFailureClass::Events)
        );
        assert_eq!(
            failure
                .difference()
                .expect("detailed event mismatch")
                .semantic_path,
            "transition.events[3]"
        );
    }

    #[test]
    fn exact_transition_acceptance_failure_is_detailed() {
        let (before, result, mut expected) = lifecycle_conformance_case();
        expected.expected_response_result = ExpectedResponseResult::RejectedWithoutMutation;

        assert_detailed_failure(
            assert_exact_transition(
                &before,
                None,
                &expected.response,
                &result,
                &BTreeMap::new(),
                &expected,
            ),
            ConformanceFailureClass::Acceptance,
            "transition.acceptance",
            ConformanceMismatchKind::ValueChanged,
        );
    }

    #[test]
    fn exact_transition_event_length_differences_are_detailed() {
        let (before, result, mut expected) = lifecycle_conformance_case();
        let last = expected
            .expected_authoritative_events
            .last()
            .cloned()
            .unwrap();
        expected.expected_authoritative_events.push(last);
        let missing = assert_detailed_failure(
            assert_exact_transition(
                &before,
                None,
                &expected.response,
                &result,
                &BTreeMap::new(),
                &expected,
            ),
            ConformanceFailureClass::Events,
            &format!("transition.events[{}]", result.events.len()),
            ConformanceMismatchKind::ExpectedEntryMissing,
        );
        assert_eq!(
            missing
                .sequence
                .expect("sequence difference")
                .expected_length,
            result.events.len() + 1
        );

        let (before, result, mut expected) = lifecycle_conformance_case();
        expected.expected_authoritative_events.pop();
        let extra = assert_detailed_failure(
            assert_exact_transition(
                &before,
                None,
                &expected.response,
                &result,
                &BTreeMap::new(),
                &expected,
            ),
            ConformanceFailureClass::Events,
            &format!(
                "transition.events[{}]",
                expected.expected_authoritative_events.len()
            ),
            ConformanceMismatchKind::UnexpectedExtraEntry,
        );
        assert_eq!(
            extra.sequence.expect("sequence difference").actual_length,
            expected.expected_authoritative_events.len() + 1
        );
    }

    #[test]
    fn exact_transition_delta_failure_reports_a_nonzero_index() {
        let (before, result, mut expected) = lifecycle_conformance_case();
        let replacement = expected.expected_semantic_delta[0].clone();
        expected.expected_semantic_delta[1] = replacement;

        assert_detailed_failure(
            assert_exact_transition(
                &before,
                None,
                &expected.response,
                &result,
                &BTreeMap::new(),
                &expected,
            ),
            ConformanceFailureClass::Delta,
            "transition.delta.audit[1]",
            ConformanceMismatchKind::ValueChanged,
        );
    }

    #[test]
    fn exact_transition_state_digest_failure_is_detailed() {
        let (before, result, mut expected) = lifecycle_conformance_case();
        expected.expected_state_digest =
            FullStateDigestV4::parse("ff".repeat(32)).expect("synthetic digest");

        assert_detailed_failure(
            assert_exact_transition(
                &before,
                None,
                &expected.response,
                &result,
                &BTreeMap::new(),
                &expected,
            ),
            ConformanceFailureClass::StateDigest,
            "transition.state_digest",
            ConformanceMismatchKind::ValueChanged,
        );
    }

    #[test]
    fn exact_transition_next_decision_failure_is_detailed() {
        let (before, result, mut expected) = lifecycle_conformance_case();
        expected.expected_next_decision = Some(decision(99));

        assert_detailed_failure(
            assert_exact_transition(
                &before,
                None,
                &expected.response,
                &result,
                &BTreeMap::new(),
                &expected,
            ),
            ConformanceFailureClass::NextDecision,
            "transition.next_decision",
            ConformanceMismatchKind::ValueChanged,
        );
    }

    #[test]
    fn exact_transition_status_failure_is_detailed() {
        let (before, result, mut expected) = lifecycle_conformance_case();
        expected.expected_status = truncated_status();

        assert_detailed_failure(
            assert_exact_transition(
                &before,
                None,
                &expected.response,
                &result,
                &BTreeMap::new(),
                &expected,
            ),
            ConformanceFailureClass::Status,
            "transition.status",
            ConformanceMismatchKind::ValueChanged,
        );
    }

    #[test]
    fn exact_transition_player_projection_reports_the_first_player() {
        let (before, result, mut expected) = lifecycle_conformance_case();
        expected
            .expected_player_steps
            .insert(PlayerId(1), player_step_fixture());

        assert_detailed_failure(
            assert_exact_transition(
                &before,
                None,
                &expected.response,
                &result,
                &BTreeMap::new(),
                &expected,
            ),
            ConformanceFailureClass::PlayerProjection,
            "player_steps[player:1]",
            ConformanceMismatchKind::PlayerMissing,
        );
    }

    #[test]
    fn exact_transition_same_inputs_produce_the_same_diagnostic() {
        let (before, result, mut expected) = lifecycle_conformance_case();
        let last = expected
            .expected_authoritative_events
            .last()
            .cloned()
            .unwrap();
        expected.expected_authoritative_events.push(last);

        let first = assert_exact_transition(
            &before,
            None,
            &expected.response,
            &result,
            &BTreeMap::new(),
            &expected,
        )
        .expect_err("event mismatch");
        let second = assert_exact_transition(
            &before,
            None,
            &expected.response,
            &result,
            &BTreeMap::new(),
            &expected,
        )
        .expect_err("event mismatch");

        assert_eq!(first, second);
    }

    #[test]
    fn exact_transition_reports_event_before_later_status_difference() {
        let (before, result, mut expected) = lifecycle_conformance_case();
        let last = expected
            .expected_authoritative_events
            .last()
            .cloned()
            .unwrap();
        expected.expected_authoritative_events.push(last);
        expected.expected_status = truncated_status();

        let failure = assert_exact_transition(
            &before,
            None,
            &expected.response,
            &result,
            &BTreeMap::new(),
            &expected,
        )
        .expect_err("multiple mismatches");
        assert_eq!(
            failure.classification(),
            Some(ConformanceFailureClass::Events)
        );
        assert_eq!(
            failure
                .difference()
                .expect("event difference")
                .mismatch_kind,
            ConformanceMismatchKind::ExpectedEntryMissing
        );
    }

    #[test]
    fn exact_transition_contract_failure_stays_separate() {
        let (before, result, expected) = lifecycle_conformance_case();
        let mut tampered = result.clone();
        tampered.events.pop();

        let failure = assert_exact_transition(
            &before,
            None,
            &expected.response,
            &tampered,
            &BTreeMap::new(),
            &expected,
        )
        .expect_err("invalid transition product");
        assert!(matches!(failure, ConformanceFailure::Contract(_)));
        assert!(failure.difference().is_none());
    }

    #[test]
    fn exact_product_event_failure_is_detailed() {
        let (before, result, mut expected_events, expected_digest) = lifecycle_product_case();
        let last = expected_events.last().cloned().unwrap();
        expected_events.push(last);

        assert_detailed_failure(
            lifecycle::assert_exact_transition_product(
                &before,
                &result,
                &expected_events,
                &expected_digest,
            ),
            ConformanceFailureClass::Events,
            &format!("transition.events[{}]", result.events.len()),
            ConformanceMismatchKind::ExpectedEntryMissing,
        );
    }

    #[test]
    fn exact_product_digest_failure_is_detailed() {
        let (before, result, expected_events, _) = lifecycle_product_case();
        let wrong_digest = FullStateDigestV4::parse("ff".repeat(32)).expect("synthetic digest");

        assert_detailed_failure(
            lifecycle::assert_exact_transition_product(
                &before,
                &result,
                &expected_events,
                &wrong_digest,
            ),
            ConformanceFailureClass::StateDigest,
            "transition.state_digest",
            ConformanceMismatchKind::ValueChanged,
        );
    }

    #[test]
    fn exact_product_status_failure_is_detailed() {
        let (before, result, expected_events, expected_digest) = lifecycle_product_case();
        let mut tampered = result.clone();
        tampered.status = truncated_status();

        assert_detailed_failure(
            lifecycle::assert_exact_transition_product(
                &before,
                &tampered,
                &expected_events,
                &expected_digest,
            ),
            ConformanceFailureClass::Status,
            "transition.status",
            ConformanceMismatchKind::ValueChanged,
        );
    }
}
