//! Independent wire-decode nonmutation evidence for M2.G G.4.
//!
//! Malformed and noncanonical submission bytes must fail CLOSED at the
//! player wire boundary with exactly the closed layer-A code, must not
//! invoke the semantic endpoint even once (counting decorator over the
//! real handle), and must leave the COMPLETE fingerprint untouched. The
//! canonical-but-stale contrast case proves the decorator itself observes
//! real semantic submissions. Every M2.G gate remains `NOT_RUN`; nothing
//! here is a gate verdict.

pub const WIRE_MALFORMED_CLASSES: &[&str] = &[
    "leading_whitespace_noncanonical",
    "wrong_key_order_noncanonical",
    "unknown_field_added",
    "wrong_schema_version",
    "truncated_json",
    "candidate_id_overflow",
];

#[cfg(test)]
mod tests {
    use super::WIRE_MALFORMED_CLASSES;
    use crate::isolation::fingerprint::{
        assert_fingerprint_policies, capture_complete, capture_transition_product,
        FingerprintComparison,
    };
    use crate::isolation::paired::{
        base_pair_state, spawn_environment, synthetic_environment_config,
    };
    use crate::isolation::HarnessError;
    use mtgml_decision::{
        DecisionAnswerV2, DecisionResponseV2, PlayerDecisionRequestV2, DECISION_RESPONSE_V2_SCHEMA,
    };
    use mtgml_environment::{
        submit_response_bytes, PlayerBoundaryError, PlayerEndpoint, PlayerEndpointError,
        PlayerEndpointHandle,
    };
    use mtgml_model::{CandidateIdV1, PlayerDecisionIdV1, PlayerId, StateRevision};
    use mtgml_observation::{
        ObservationEnvelope, PlayerInformationStateV2, PlayerStepSubmissionV1, PlayerStepV2,
    };
    use mtgml_wire::PlayerWireErrorCodeV1;
    use std::sync::atomic::{AtomicUsize, Ordering};

    const P1: PlayerId = PlayerId(1);
    const P2: PlayerId = PlayerId(2);

    const SEED_HEX: &str = "6666666666666666666666666666666666666666666666666666666666666666";

    /// Conformance-owned counting decorator around a REAL bound handle: it
    /// forwards every read verbatim and records exactly how many times the
    /// semantic `submit` was reached.
    struct IsolationCountingEndpoint<'a> {
        inner: &'a PlayerEndpointHandle,
        submissions: AtomicUsize,
    }

    impl IsolationCountingEndpoint<'_> {
        fn submissions(&self) -> usize {
            self.submissions.load(Ordering::SeqCst)
        }
    }

    impl PlayerEndpoint for IsolationCountingEndpoint<'_> {
        fn perspective(&self) -> PlayerId {
            self.inner.perspective()
        }

        fn observation(&self) -> Result<ObservationEnvelope, PlayerEndpointError> {
            self.inner.observation()
        }

        fn information_state(&self) -> Result<PlayerInformationStateV2, PlayerEndpointError> {
            self.inner.information_state()
        }

        fn visible_decision(&self) -> Result<Option<PlayerDecisionRequestV2>, PlayerEndpointError> {
            self.inner.visible_decision()
        }

        fn submit(
            &self,
            response: DecisionResponseV2,
        ) -> Result<PlayerStepV2, PlayerEndpointError> {
            self.submissions.fetch_add(1, Ordering::SeqCst);
            self.inner.submit(response)
        }
    }

    fn config() -> mtgml_environment::SyntheticM1EnvironmentConfig {
        synthetic_environment_config([P1, P2])
    }

    fn plausible_response() -> DecisionResponseV2 {
        DecisionResponseV2 {
            schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
            player_decision_id: PlayerDecisionIdV1(1),
            state_revision: StateRevision(0),
            answer: DecisionAnswerV2::SelectOne {
                candidate_id: CandidateIdV1(0),
            },
        }
    }

    type CorruptBytes = fn(canonical: &[u8]) -> Vec<u8>;

    /// Shape sanity guard: every corruptor below assumes canonical
    /// JSON-object bytes. If canonical encoding ever drifts away from that
    /// shape, the mutation must fail loudly here instead of silently
    /// producing a meaningless malformed class.
    fn assert_canonical_object_shape(canonical: &[u8]) {
        assert!(
            canonical.first() == Some(&b'{') && canonical.last() == Some(&b'}'),
            "canonical submission bytes are no longer a JSON object; corruptor shapes drifted"
        );
    }

    const MALFORMED_CLASSES: &[(&str, CorruptBytes)] = &[
        ("leading_whitespace_noncanonical", leading_whitespace),
        ("wrong_key_order_noncanonical", wrong_key_order),
        ("unknown_field_added", unknown_field_added),
        ("wrong_schema_version", wrong_schema_version),
        ("truncated_json", truncated_json),
        ("candidate_id_overflow", candidate_id_overflow),
    ];

    fn leading_whitespace(canonical: &[u8]) -> Vec<u8> {
        assert_canonical_object_shape(canonical);
        let mut bytes = Vec::with_capacity(canonical.len() + 1);
        bytes.push(b' ');
        bytes.extend_from_slice(canonical);
        bytes
    }

    /// Canonical bytes sort object keys; re-emitting the same fields in a
    /// fixed non-sorted top-level order breaks the canonical comparison.
    fn wrong_key_order(canonical: &[u8]) -> Vec<u8> {
        assert_canonical_object_shape(canonical);
        let value: serde_json::Value = serde_json::from_slice(canonical).unwrap();
        let fields = value.as_object().unwrap();
        let order = [
            "schema_version",
            "player_decision_id",
            "state_revision",
            "answer",
        ];
        let pieces: Vec<String> = order
            .iter()
            .map(|key| format!("\"{key}\":{}", fields[*key]))
            .collect();
        format!("{{{}}}", pieces.join(",")).into_bytes()
    }

    fn unknown_field_added(canonical: &[u8]) -> Vec<u8> {
        assert_canonical_object_shape(canonical);
        let mut bytes = canonical[..canonical.len() - 1].to_vec();
        bytes.extend_from_slice(b",\"conformance_unknown_field\":1}");
        bytes
    }

    fn wrong_schema_version(canonical: &[u8]) -> Vec<u8> {
        assert_canonical_object_shape(canonical);
        std::str::from_utf8(canonical)
            .unwrap()
            .replace(DECISION_RESPONSE_V2_SCHEMA, "decision-response.v9")
            .into_bytes()
    }

    fn truncated_json(canonical: &[u8]) -> Vec<u8> {
        assert_canonical_object_shape(canonical);
        canonical[..canonical.len() - 8].to_vec()
    }

    fn candidate_id_overflow(canonical: &[u8]) -> Vec<u8> {
        assert_canonical_object_shape(canonical);
        std::str::from_utf8(canonical)
            .unwrap()
            .replace("\"candidate_id\":0", "\"candidate_id\":4294967296")
            .into_bytes()
    }

    #[test]
    fn malformed_classes_zero_submit_and_zero_mutation() -> Result<(), HarnessError> {
        assert_eq!(
            MALFORMED_CLASSES.len(),
            WIRE_MALFORMED_CLASSES.len(),
            "every pinned class must be executed"
        );
        for (name, corrupt) in MALFORMED_CLASSES {
            let state = base_pair_state(SEED_HEX)?;
            let (controller, endpoints) = spawn_environment(state, &config())?;
            assert!(
                endpoints[0]
                    .visible_decision()
                    .map_err(|_| HarnessError::EndpointService)?
                    .is_some(),
                "class {name} expects the entry request to be visible"
            );
            let counting = IsolationCountingEndpoint {
                inner: &endpoints[0],
                submissions: AtomicUsize::new(0),
            };
            let canonical = mtgml_wire::encode_canonical(&plausible_response())
                .map_err(|_| HarnessError::WireEncoding)?;
            let before = capture_complete(&controller, &endpoints)?;

            let outcome = submit_response_bytes(&counting, &corrupt(&canonical));

            let Err(failure) = outcome else {
                panic!("class {name} was decoded and submitted instead of failing closed");
            };
            assert_eq!(
                failure.code(),
                "malformed_response",
                "class {name} closed code"
            );
            assert!(
                matches!(
                    failure,
                    PlayerBoundaryError::Wire(PlayerWireErrorCodeV1::MalformedResponse)
                ),
                "class {name} must fail at layer A"
            );
            assert_eq!(
                counting.submissions(),
                0,
                "class {name} must never reach the semantic submit"
            );

            let after = capture_complete(&controller, &endpoints)?;
            assert_fingerprint_policies(&before, &after, FingerprintComparison::All)
                .unwrap_or_else(|error| panic!("class {name} mutated the environment: {error:?}"));
        }
        Ok(())
    }

    /// Contrast case proving the counting decorator observes real semantic
    /// traffic: canonical bytes carrying only a stale identity decode fine,
    /// reach `submit` exactly once, and still mutate nothing.
    #[test]
    fn canonical_stale_bytes_reach_semantic_submit_exactly_once() -> Result<(), HarnessError> {
        let state = base_pair_state(SEED_HEX)?;
        let (controller, endpoints) = spawn_environment(state, &config())?;
        let request = endpoints[0]
            .visible_decision()
            .map_err(|_| HarnessError::EndpointService)?
            .ok_or(HarnessError::TransformPreconditionViolated)?;
        let stale = DecisionResponseV2 {
            schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
            player_decision_id: PlayerDecisionIdV1(
                request
                    .player_decision_id
                    .0
                    .checked_add(1)
                    .ok_or(HarnessError::TransformPreconditionViolated)?,
            ),
            state_revision: request.state_revision,
            answer: DecisionAnswerV2::SelectOne {
                candidate_id: request
                    .candidates
                    .first()
                    .map(|candidate| candidate.candidate_id)
                    .ok_or(HarnessError::TransformFixtureAbsent)?,
            },
        };
        let canonical =
            mtgml_wire::encode_canonical(&stale).map_err(|_| HarnessError::WireEncoding)?;
        let counting = IsolationCountingEndpoint {
            inner: &endpoints[0],
            submissions: AtomicUsize::new(0),
        };
        let before = capture_complete(&controller, &endpoints)?;

        let step = match submit_response_bytes(&counting, &canonical) {
            Ok(step) => step,
            Err(failure) => panic!("canonical stale bytes must decode, got {failure:?}"),
        };

        assert_eq!(
            counting.submissions(),
            1,
            "the semantic submit must run once"
        );
        assert_eq!(
            step.submission,
            PlayerStepSubmissionV1::Rejected {
                code: mtgml_observation::PlayerSubmissionCodeV1::StaleDecision,
            }
        );
        let product = capture_transition_product(Ok(step))?;
        assert_eq!(
            product.semantic_submission_code.as_deref(),
            Some("stale_decision")
        );

        let after = capture_complete(&controller, &endpoints)?;
        assert_fingerprint_policies(&before, &after, FingerprintComparison::All)
            .unwrap_or_else(|error| panic!("stale rejection mutated the environment: {error:?}"));
        Ok(())
    }

    #[test]
    fn coverage_tags_match_executed_classes() {
        assert_eq!(WIRE_MALFORMED_CLASSES.len(), 6);
        assert_eq!(
            MALFORMED_CLASSES.len(),
            WIRE_MALFORMED_CLASSES.len(),
            "every pinned class must be executed"
        );
        for (name, _) in MALFORMED_CLASSES {
            assert!(
                WIRE_MALFORMED_CLASSES.contains(name),
                "class {name} is not pinned by the coverage tag"
            );
        }
        let mut unique = WIRE_MALFORMED_CLASSES.to_vec();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(
            unique.len(),
            WIRE_MALFORMED_CLASSES.len(),
            "coverage tags must be unique"
        );
    }
}
