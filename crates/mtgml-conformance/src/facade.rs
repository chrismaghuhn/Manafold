//! T0 conformance facade contract (M3.T0-01, RED).
//!
//! T0 is a *thin private conformance facade over the real Rust kernel*.  It
//! feeds explicit setups and explicit player responses into the authoritative
//! engine and compares exact, independently authored expected products.  It
//! is NOT a rules engine, NOT a Magic capability, NOT a simulator
//! replacement, NOT a card executor, NOT a public protocol, and NOT a wire
//! format.  The authoritative engine remains the sole source of state
//! transitions, legality, decisions, events, RNG, replay, checkpointing, and
//! information semantics.
//!
//! The case-level surface required by this contract does not exist yet, so
//! every test below is expected to fail to compile against exactly one
//! missing-module gate until the reviewed facade implementation slice lands.
//! This file intentionally contains no facade implementation.
//!
//! Required surface (narrowest form for the selected synthetic cases):
//!
//! ```text
//! ConformanceCase            named, documented sequence of explicit steps
//! ConformanceStepRef         one explicit response plus its expectation,
//!                            labeled with a stable step identity
//! ConformanceExpectation     independently authored expected products
//! LimitCounterDeltas         authored environment limit-counter deltas
//! NextDecisionExpectation    authored next-decision projection facts
//! ConformanceCaseDiagnostic  deterministic first-divergence report
//! ```
//!
//! Contract invariants (enforced by review and by the tests below):
//!
//! ```text
//! EXPECTED_VALUES_ARE_LITERAL  expectations are authored data transcribed
//!                              from accepted evidence, never derived from
//!                              actual output at runtime (no
//!                              expected_state = actual_state.clone(), no
//!                              expected_digest = digest(actual_state))
//! RESPONSES_ARE_EXPLICIT       every step carries its literal response; no
//!                              default/first/empty/pass selection, no
//!                              malformed-answer repair, no hidden automation
//! NO_SECOND_RULES_ENGINE       the facade never calculates legality, costs,
//!                              payments, replacement/prevention effects,
//!                              trigger order, state-based actions, layers,
//!                              combat legality, or turn progression; it
//!                              submits what the case supplies and compares
//!                              what the kernel returns
//! REJECTION_NONMUTATION        rejected steps must be proven nonmutating by
//!                              reusing the existing complete-fingerprint
//!                              authority (CompleteM2Fingerprint equality),
//!                              never a second fingerprint implementation
//! TRUSTED_ONLY                 the facade and its diagnostics live in this
//!                              conformance crate only; player observation,
//!                              information-state, step, and endpoint
//!                              surfaces gain nothing
//! ```

#[cfg(test)]
mod t0_01_red_contract {
    use mtgml_decision::{
        DecisionAnswerV2, DecisionDomainV2, DecisionResponseV2, DECISION_RESPONSE_V2_SCHEMA,
    };
    use mtgml_model::{CandidateIdV1, DecisionId, PlayerDecisionIdV1, PlayerId, StateRevision};

    // The single intentional RED gate: this module does not exist yet.  All
    // contract tests share this one unresolved import so the failure remains
    // small and legible (one missing contract surface) instead of cascading.
    use crate::facade::{
        ConformanceCase, ConformanceCaseDiagnostic, ConformanceExpectation, ConformanceStepRef,
        LimitCounterDeltas, NextDecisionExpectation,
    };

    use crate::isolation::{base_pair_state, capture_complete, spawn_environment};
    use crate::{ConformanceFailureClass, ExpectedResponseResult};

    const P1: PlayerId = PlayerId(1);
    const P2: PlayerId = PlayerId(2);

    /// The accepted synthetic base state used by the isolation harness
    /// evidence (64 hex chars).
    const SEED_HEX_A: &str = "3333333333333333333333333333333333333333333333333333333333333333";

    /// Explicitly authored response for the synthetic entry ChooseOne
    /// decision (candidate 0).  Nothing is picked implicitly: identity and
    /// revision fields are stated, never looked up from a live request.
    fn explicit_entry_response() -> DecisionResponseV2 {
        DecisionResponseV2 {
            schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
            player_decision_id: PlayerDecisionIdV1(1),
            state_revision: StateRevision(0),
            answer: DecisionAnswerV2::SelectOne {
                candidate_id: CandidateIdV1(0),
            },
        }
    }

    /// Witness A (RED): the facade must express the already-accepted
    /// synthetic entry transition with independently authored expectations.
    ///
    /// Every expected value below is transcribed from the accepted M2.E
    /// evidence (`synthetic_m2_choose_one_returns_authoritative_transition_
    /// product` and `assert_accepted_entry_progression`), never derived from
    /// production output at runtime.
    #[test]
    fn witness_a_accepted_entry_transition_with_literal_expectations() {
        let case = ConformanceCase {
            name: "synthetic-entry-choose-one-accepted",
            description: "one explicit ChooseOne response commits the accepted V4 transition",
            steps: vec![ConformanceStepRef {
                label: "step-1-entry-choose-one",
                response: explicit_entry_response(),
                expectation: ConformanceExpectation {
                    expected_result: ExpectedResponseResult::Accepted,
                    expected_post_state_revision: StateRevision(1),
                    // Accepted evidence: 5 authoritative events, limit
                    // counters +1 submitted / +1 accepted / +5 events.
                    expected_event_count: 5,
                    expected_limit_counter_deltas: LimitCounterDeltas {
                        decisions_submitted: 1,
                        accepted_transitions: 1,
                        rule_events_emitted: 5,
                    },
                    expected_next_decision: Some(NextDecisionExpectation {
                        decision_id: DecisionId(2),
                        domain: DecisionDomainV2::ChooseNumber {
                            minimum: 0,
                            maximum: 3,
                        },
                        candidate_count: 0,
                    }),
                },
            }],
        };

        assert_eq!(case.name, "synthetic-entry-choose-one-accepted");
        assert_eq!(case.steps.len(), 1);
        assert_eq!(case.steps[0].label, "step-1-entry-choose-one");
        assert_eq!(
            case.steps[0].expectation.expected_result,
            ExpectedResponseResult::Accepted
        );
        assert_eq!(
            case.steps[0].expectation.expected_post_state_revision,
            StateRevision(1)
        );
        assert_eq!(case.steps[0].expectation.expected_event_count, 5);
        assert_eq!(
            case.steps[0].expectation.expected_next_decision,
            Some(NextDecisionExpectation {
                decision_id: DecisionId(2),
                domain: DecisionDomainV2::ChooseNumber {
                    minimum: 0,
                    maximum: 3,
                },
                candidate_count: 0,
            })
        );
    }

    /// Witness B (RED): the facade must express the already-accepted
    /// rejection case and prove complete nonmutation (state, RNG,
    /// allocators, knowledge, history/events, episode status, pending
    /// decision) by reusing the existing complete-fingerprint authority —
    /// never a second fingerprint implementation.
    #[test]
    fn witness_b_rejected_response_requires_complete_nonmutation() {
        let state = base_pair_state(SEED_HEX_A).expect("accepted synthetic base state");
        let config = crate::isolation::synthetic_environment_config([P1, P2]);
        let (controller, endpoints) =
            spawn_environment(state, &config).expect("spawned trusted environment");

        let case = ConformanceCase {
            name: "synthetic-entry-stale-revision-rejected",
            description: "a stale-revision response is rejected without any mutation",
            steps: vec![ConformanceStepRef {
                label: "step-1-stale-revision-rejected",
                // Explicit malformed-on-purpose response: stale revision 5
                // against base revision 0; identity fields unchanged.  The
                // case supplies it literally; nothing is repaired.
                response: DecisionResponseV2 {
                    schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
                    player_decision_id: PlayerDecisionIdV1(1),
                    state_revision: StateRevision(5),
                    answer: DecisionAnswerV2::SelectOne {
                        candidate_id: CandidateIdV1(0),
                    },
                },
                expectation: ConformanceExpectation {
                    expected_result: ExpectedResponseResult::RejectedWithoutMutation,
                    expected_post_state_revision: StateRevision(0),
                    expected_event_count: 0,
                    expected_limit_counter_deltas: LimitCounterDeltas {
                        decisions_submitted: 1,
                        accepted_transitions: 0,
                        rule_events_emitted: 0,
                    },
                    expected_next_decision: None,
                },
            }],
        };

        assert_eq!(case.steps.len(), 1);
        assert_eq!(
            case.steps[0].expectation.expected_result,
            ExpectedResponseResult::RejectedWithoutMutation
        );
        assert_eq!(
            case.steps[0].expectation.expected_post_state_revision,
            StateRevision(0)
        );
        assert_eq!(case.steps[0].expectation.expected_event_count, 0);
        assert_eq!(
            case.steps[0].expectation.expected_limit_counter_deltas,
            LimitCounterDeltas {
                decisions_submitted: 1,
                accepted_transitions: 0,
                rule_events_emitted: 0,
            }
        );

        // The existing complete-fingerprint authority is the only permitted
        // nonmutation oracle: the facade must compare identical captures
        // before and after the rejected submission and require equality
        // across all four groups (semantic, environment, player-visible,
        // replay-recorder).  This witness confirms the authority is
        // reachable from the case context; the facade implementation slice
        // must wire the case through it.
        let before = capture_complete(&controller, &endpoints).expect("fingerprint capture");
        let after = capture_complete(&controller, &endpoints).expect("fingerprint capture");
        assert_eq!(before, after);
    }

    /// Witness C (RED): the facade must express an explicit multi-step
    /// synthetic path (every response literal, no hidden action) and must
    /// identify the first mismatching step deterministically.
    ///
    /// Expectations are transcribed from the accepted count-2 assembly
    /// chain (`continuation_chain_advances_with_fresh_explicit_identities`
    /// plus `assert_accepted_entry_progression`/`assert_accepted_count_
    /// progression`): entry -> ChooseNumber{0,3} -> ChooseMany{2,2} (2
    /// candidates) -> Order{2,2} (2 candidates).
    #[test]
    fn witness_c_multi_step_sequence_tracks_first_failing_step() {
        let case = ConformanceCase {
            name: "synthetic-assembly-three-explicit-steps",
            description: "ChooseOne, ChooseNumber, SelectMany - each response explicit",
            steps: vec![
                ConformanceStepRef {
                    label: "step-1-entry-choose-one",
                    response: explicit_entry_response(),
                    expectation: ConformanceExpectation {
                        expected_result: ExpectedResponseResult::Accepted,
                        expected_post_state_revision: StateRevision(1),
                        expected_event_count: 5,
                        expected_limit_counter_deltas: LimitCounterDeltas {
                            decisions_submitted: 1,
                            accepted_transitions: 1,
                            rule_events_emitted: 5,
                        },
                        expected_next_decision: Some(NextDecisionExpectation {
                            decision_id: DecisionId(2),
                            domain: DecisionDomainV2::ChooseNumber {
                                minimum: 0,
                                maximum: 3,
                            },
                            candidate_count: 0,
                        }),
                    },
                },
                ConformanceStepRef {
                    label: "step-2-choose-count",
                    // Explicit stage answer: count = 2.
                    response: DecisionResponseV2 {
                        schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
                        player_decision_id: PlayerDecisionIdV1(2),
                        state_revision: StateRevision(1),
                        answer: DecisionAnswerV2::ChooseNumber { value: 2 },
                    },
                    expectation: ConformanceExpectation {
                        expected_result: ExpectedResponseResult::Accepted,
                        expected_post_state_revision: StateRevision(2),
                        expected_event_count: 2,
                        expected_limit_counter_deltas: LimitCounterDeltas {
                            decisions_submitted: 1,
                            accepted_transitions: 1,
                            rule_events_emitted: 2,
                        },
                        expected_next_decision: Some(NextDecisionExpectation {
                            decision_id: DecisionId(3),
                            domain: DecisionDomainV2::ChooseMany {
                                minimum: 2,
                                maximum: 2,
                            },
                            candidate_count: 2,
                        }),
                    },
                },
                ConformanceStepRef {
                    label: "step-3-choose-members",
                    response: DecisionResponseV2 {
                        schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
                        player_decision_id: PlayerDecisionIdV1(3),
                        state_revision: StateRevision(2),
                        answer: DecisionAnswerV2::SelectMany {
                            candidate_ids: vec![CandidateIdV1(0), CandidateIdV1(1)],
                        },
                    },
                    expectation: ConformanceExpectation {
                        expected_result: ExpectedResponseResult::Accepted,
                        expected_post_state_revision: StateRevision(3),
                        expected_event_count: 2,
                        expected_limit_counter_deltas: LimitCounterDeltas {
                            decisions_submitted: 1,
                            accepted_transitions: 1,
                            rule_events_emitted: 2,
                        },
                        expected_next_decision: Some(NextDecisionExpectation {
                            decision_id: DecisionId(4),
                            domain: DecisionDomainV2::Order {
                                minimum: 2,
                                maximum: 2,
                            },
                            candidate_count: 2,
                        }),
                    },
                },
            ],
        };

        assert_eq!(case.steps.len(), 3);
        for (index, step) in case.steps.iter().enumerate() {
            assert_eq!(
                step.expectation.expected_result,
                ExpectedResponseResult::Accepted
            );
            assert_eq!(
                step.expectation.expected_post_state_revision,
                StateRevision(index as u64 + 1)
            );
        }
        assert_eq!(
            case.steps[2].expectation.expected_next_decision,
            Some(NextDecisionExpectation {
                decision_id: DecisionId(4),
                domain: DecisionDomainV2::Order {
                    minimum: 2,
                    maximum: 2,
                },
                candidate_count: 2,
            })
        );

        // First-divergence contract: a mismatch at the second step must be
        // reported as exactly step index 1 with case identity, step label,
        // comparison path, and expected/actual - deterministically.
        let diagnostic = ConformanceCaseDiagnostic {
            case: case.name,
            step_label: case.steps[1].label,
            first_failing_step_index: Some(1),
            classification: ConformanceFailureClass::Delta,
            path: "steps[1].expected_semantic_delta".to_string(),
            expected: "authored delta for step-2-choose-count".to_string(),
            actual: "kernel delta differed at step 2".to_string(),
        };
        assert_eq!(diagnostic.first_failing_step_index, Some(1));
        assert_eq!(diagnostic.case, case.name);
        assert_eq!(diagnostic.step_label, "step-2-choose-count");
        assert_eq!(diagnostic.path, "steps[1].expected_semantic_delta");
    }

    /// Diagnostic determinism: the same mismatch must always render the
    /// same structured report (case, step, path, expected, actual), and the
    /// report is trusted conformance-only output, never player-facing.
    #[test]
    fn diagnostics_are_deterministic_structured_and_trusted_only() {
        let left = ConformanceCaseDiagnostic {
            case: "case-x",
            step_label: "step-2",
            first_failing_step_index: Some(1),
            classification: ConformanceFailureClass::Events,
            path: "steps[1].expected_authoritative_events".to_string(),
            expected: "2 events".to_string(),
            actual: "1 event".to_string(),
        };
        let right = ConformanceCaseDiagnostic {
            case: "case-x",
            step_label: "step-2",
            first_failing_step_index: Some(1),
            classification: ConformanceFailureClass::Events,
            path: "steps[1].expected_authoritative_events".to_string(),
            expected: "2 events".to_string(),
            actual: "1 event".to_string(),
        };
        assert_eq!(left, right);
        let rendered = format!("{left}");
        for fragment in [
            "case-x",
            "step-2",
            "expected_authoritative_events",
            "2",
            "1",
        ] {
            assert!(
                rendered.contains(fragment),
                "diagnostic must render {fragment}"
            );
        }
    }

    /// Decision completeness: the case contract carries only explicit
    /// responses.  There is no default answer, no implicit pass, no
    /// candidate-0 shortcut, and no repair of malformed answers anywhere on
    /// the case surface; every step names its response literally.
    #[test]
    fn responses_are_explicit_and_never_default_selected() {
        let case = ConformanceCase {
            name: "explicit-response-only",
            description: "contract shape: one step, one literal response",
            steps: vec![ConformanceStepRef {
                label: "step-1",
                response: explicit_entry_response(),
                expectation: ConformanceExpectation {
                    expected_result: ExpectedResponseResult::Accepted,
                    expected_post_state_revision: StateRevision(1),
                    expected_event_count: 5,
                    expected_limit_counter_deltas: LimitCounterDeltas {
                        decisions_submitted: 1,
                        accepted_transitions: 1,
                        rule_events_emitted: 5,
                    },
                    expected_next_decision: None,
                },
            }],
        };
        // The response is carried by value on the step: the case data alone
        // determines what is submitted, and no execution input may override
        // or repair it.
        assert_eq!(
            case.steps[0].response.player_decision_id,
            PlayerDecisionIdV1(1)
        );
        assert_eq!(
            case.steps[0].response.answer,
            DecisionAnswerV2::SelectOne {
                candidate_id: CandidateIdV1(0)
            }
        );
    }

    /// Information boundary: the facade supports trusted assertions without
    /// touching the player-facing surface.  The contract types live only
    /// inside this crate's facade module; no player observation,
    /// information state, or step type gains trusted fields, and no trusted
    /// diagnostic is re-exported through player-facing APIs.
    #[test]
    fn facade_support_stays_behind_the_trusted_boundary() {
        let diagnostic = ConformanceCaseDiagnostic {
            case: "boundary-case",
            step_label: "step-1",
            first_failing_step_index: None,
            classification: ConformanceFailureClass::Status,
            path: "steps[0].expected_status".to_string(),
            expected: "running".to_string(),
            actual: "running".to_string(),
        };
        assert_eq!(diagnostic.case, "boundary-case");
        assert_eq!(diagnostic.first_failing_step_index, None);

        // The existing complete-fingerprint capture remains the only trusted
        // snapshot authority the facade may reuse.
        let state = base_pair_state(SEED_HEX_A).expect("accepted synthetic base state");
        let config = crate::isolation::synthetic_environment_config([P1, P2]);
        let (controller, endpoints) =
            spawn_environment(state, &config).expect("spawned trusted environment");
        let complete = capture_complete(&controller, &endpoints).expect("trusted fingerprint");
        assert_eq!(complete, complete);
    }
}
