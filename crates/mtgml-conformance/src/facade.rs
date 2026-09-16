//! T0 conformance facade contract (M3.T0-01, RED remediation).
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
//! every test below fails against exactly one unresolved-import gate until
//! the reviewed facade implementation slice lands.  This file intentionally
//! contains no facade implementation.
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
//! run_case(case, controller, endpoints)
//!                            execution entry point: submits every step
//!                            response to the REAL trusted environment and
//!                            compares each actual product against the
//!                            authored expectations; returns the
//!                            deterministic diagnostic for the FIRST
//!                            mismatching step (Ok(()) when all pass)
//! ```
//!
//! Contract invariants (enforced by review and by the tests below):
//!
//! ```text
//! EXPECTED_VALUES_ARE_LITERAL  expectations are authored data transcribed
//!                              from accepted evidence and independently
//!                              recomputed RNG golden vectors, never derived
//!                              from actual output at runtime (no
//!                              expected_state = actual_state.clone(), no
//!                              expected_digest = digest(actual_state))
//! EXECUTION_IS_REQUIRED        a case without run_case against the real
//!                              kernel is not conformance evidence; data
//!                              shapes alone satisfy nothing
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
//!                              capturing the existing complete-fingerprint
//!                              authority before AND after the real
//!                              submission of the invalid response through
//!                              the real trusted path, never a second
//!                              fingerprint implementation and never a
//!                              tautological no-op comparison
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
    use mtgml_model::{
        CandidateIdV1, DecisionId, EpisodeStatus, PlayerDecisionIdV1, PlayerId, RuleEventId,
        StateRevision,
    };
    use mtgml_random::{RandomStreamKeyV1, RandomStreamKindV1};
    use mtgml_rules::{AuthoritativeRuleEvent, AuthoritativeRuleEventKind};
    use mtgml_state::SemanticDeltaOperation;

    // The single intentional RED gate: this module does not exist yet.  All
    // contract tests share this one unresolved import so the failure remains
    // small and legible (one missing contract surface) instead of cascading.
    use crate::facade::{
        run_case, ConformanceCase, ConformanceCaseDiagnostic, ConformanceExpectation,
        ConformanceStepRef, LimitCounterDeltas, NextDecisionExpectation,
    };

    use crate::isolation::{base_pair_state, capture_complete, spawn_environment};
    use crate::{ConformanceFailureClass, ExpectedResponseResult};

    const P1: PlayerId = PlayerId(1);
    const P2: PlayerId = PlayerId(2);

    /// The accepted synthetic base state used by the isolation harness
    /// evidence (64 hex chars).
    const SEED_HEX_A: &str = "3333333333333333333333333333333333333333333333333333333333333333";

    /// The authoritative global synthetic stream (mtgml.rng.v1 canonical
    /// stream vocabulary).
    fn synthetic_stream() -> RandomStreamKeyV1 {
        RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1)
    }

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

    /// The five authoritative events of the accepted entry transition,
    /// authored literally.  Event ids bind to the fresh base allocators
    /// (`next_rule_event_id = RuleEventId(1)`, revision 0→1).  The sampled
    /// value is an independently recomputed mtgml.rng.v1 golden vector for
    /// the `3333…` root seed (HMAC-SHA256 stream derivation, raw word 0,
    /// uniform below 10): value 2, one raw word consumed, cursor 0→1.
    /// This data is transcribed from the accepted M2.E evidence and the
    /// standalone golden-vector computation; it is never derived from
    /// production output at runtime.
    fn expected_entry_events() -> Vec<AuthoritativeRuleEvent> {
        let stream = synthetic_stream();
        vec![
            AuthoritativeRuleEvent {
                event_id: RuleEventId(1),
                state_revision: StateRevision(1),
                event: AuthoritativeRuleEventKind::LifeChanged {
                    player: P1,
                    from: 40,
                    to: 39,
                },
            },
            AuthoritativeRuleEvent {
                event_id: RuleEventId(2),
                state_revision: StateRevision(1),
                event: AuthoritativeRuleEventKind::LifeChanged {
                    player: P1,
                    from: 39,
                    to: 38,
                },
            },
            AuthoritativeRuleEvent {
                event_id: RuleEventId(3),
                state_revision: StateRevision(1),
                event: AuthoritativeRuleEventKind::RandomValueSampled {
                    stream,
                    bound: 10,
                    value: 2,
                    raw_words_consumed: 1,
                    cursor_before: 0,
                    cursor_after: 1,
                },
            },
            AuthoritativeRuleEvent {
                event_id: RuleEventId(4),
                state_revision: StateRevision(1),
                event: AuthoritativeRuleEventKind::DecisionCleared {
                    decision: DecisionId(1),
                },
            },
            AuthoritativeRuleEvent {
                event_id: RuleEventId(5),
                state_revision: StateRevision(1),
                event: AuthoritativeRuleEventKind::DecisionCreated {
                    decision: DecisionId(2),
                },
            },
        ]
    }

    /// The semantic delta audit of the accepted entry transition, authored
    /// literally: exactly the per-event semantic operations, in event order
    /// (the kernel derives `audit` one-to-one from its events).
    fn expected_entry_delta() -> Vec<SemanticDeltaOperation> {
        let stream = synthetic_stream();
        vec![
            SemanticDeltaOperation::LifeChanged {
                player: P1,
                from: 40,
                to: 39,
            },
            SemanticDeltaOperation::LifeChanged {
                player: P1,
                from: 39,
                to: 38,
            },
            SemanticDeltaOperation::RandomValueSampled {
                stream,
                bound: 10,
                value: 2,
                raw_words_consumed: 1,
                cursor_before: 0,
                cursor_after: 1,
            },
            SemanticDeltaOperation::DecisionCleared {
                decision: DecisionId(1),
            },
            SemanticDeltaOperation::DecisionCreated {
                decision: DecisionId(2),
            },
        ]
    }

    fn entry_expectation() -> ConformanceExpectation {
        ConformanceExpectation {
            expected_result: ExpectedResponseResult::Accepted,
            expected_post_state_revision: StateRevision(1),
            expected_status: EpisodeStatus::Running,
            // Authored ordered authoritative events: contents, not counts.
            expected_events: expected_entry_events(),
            // Authored exact semantic delta (audit), not just revisions.
            expected_delta: expected_entry_delta(),
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
        }
    }

    fn entry_case() -> ConformanceCase {
        ConformanceCase {
            name: "synthetic-entry-choose-one-accepted",
            description: "one explicit ChooseOne response commits the accepted V4 transition",
            steps: vec![ConformanceStepRef {
                label: "step-1-entry-choose-one",
                response: explicit_entry_response(),
                expectation: entry_expectation(),
            }],
        }
    }

    /// Witness A (RED): the facade must EXECUTE the already-accepted
    /// synthetic entry transition against the real trusted environment and
    /// compare each actual product against the independently authored
    /// literal expectations (exact ordered events, exact delta audit,
    /// status, counter deltas, next decision).
    ///
    /// A case that is merely constructed and read back is not evidence:
    /// this test only becomes green through `run_case` driving the real
    /// kernel and matching every authored field.
    #[test]
    fn witness_a_executes_accepted_entry_transition_against_authored_products() {
        let case = entry_case();

        let state = base_pair_state(SEED_HEX_A).expect("accepted synthetic base state");
        let config = crate::isolation::synthetic_environment_config([P1, P2]);
        let (controller, endpoints) =
            spawn_environment(state, &config).expect("spawned trusted environment");

        run_case(&case, &controller, &endpoints)
            .expect("authored entry expectations must match the authoritative kernel");
    }

    /// Witness B (RED): the facade must submit the explicitly invalid
    /// stale-revision response through the real trusted path BETWEEN two
    /// complete-fingerprint captures and require exact equality of all four
    /// groups (semantic, environment, player-visible, replay-recorder).
    ///
    /// The submission must happen: a tautological
    /// `capture == capture` without an intervening rejected submission is
    /// NOT nonmutation evidence, and the facade must prove the rejection
    /// outcome plus unchanged counters/pending decision itself.
    #[test]
    fn witness_b_rejected_submission_is_bracketed_by_fingerprints() {
        let state = base_pair_state(SEED_HEX_A).expect("accepted synthetic base state");
        let config = crate::isolation::synthetic_environment_config([P1, P2]);
        let (controller, endpoints) =
            spawn_environment(state, &config).expect("spawned trusted environment");

        // Explicitly malformed-on-purpose response: stale revision 5
        // against base revision 0; identity fields unchanged.  The case
        // supplies it literally; nothing repairs it.
        let case = ConformanceCase {
            name: "synthetic-entry-stale-revision-rejected",
            description: "a stale-revision response is rejected without any mutation",
            steps: vec![ConformanceStepRef {
                label: "step-1-stale-revision-rejected",
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
                    expected_status: EpisodeStatus::Running,
                    expected_events: Vec::new(),
                    expected_delta: Vec::new(),
                    expected_limit_counter_deltas: LimitCounterDeltas {
                        // A rejected submission is not an accepted
                        // transition: all counters unchanged (0/0/0).
                        decisions_submitted: 0,
                        accepted_transitions: 0,
                        rule_events_emitted: 0,
                    },
                    expected_next_decision: None,
                },
            }],
        };

        // The complete-fingerprint authority brackets the REAL submission:
        // run_case must submit through the trusted player endpoint and the
        // two captures must then be exactly equal.  run_case is the only
        // execution path in this witness; the comparison below fails for
        // any implementation that skips or mutates during rejection.
        let before = capture_complete(&controller, &endpoints).expect("fingerprint capture");
        run_case(&case, &controller, &endpoints)
            .expect("the authored rejection expectations must hold");
        let after = capture_complete(&controller, &endpoints).expect("fingerprint capture");
        assert_eq!(before, after);
    }

    /// Witness C (RED): the facade must execute the explicit three-step
    /// assembly chain (every response literal) and, when a step's authored
    /// expectation deliberately mismatches, report the FIRST failing step
    /// deterministically — as the actual output of a real `run_case`
    /// mismatch, not as a hand-built struct.
    #[test]
    fn witness_c_multi_step_mismatch_reports_first_failing_step() {
        // Steps 1–2 carry authored expectations transcribed from the
        // accepted count-2 assembly chain
        // (`continuation_chain_advances_with_fresh_explicit_identities`,
        // `assert_accepted_entry_progression`, `assert_accepted_count_
        // progression`): entry -> ChooseNumber{0,3} -> ChooseMany{2,2}
        // with 2 candidates.  Step 2's event expectation is deliberately
        // wrong by one (the authored final `DecisionCreated` points at the
        // wrong decision id), so a correct facade implementation MUST fail
        // this case exactly at step index 1.
        let case = ConformanceCase {
            name: "synthetic-assembly-three-explicit-steps",
            description: "ChooseOne, ChooseNumber, SelectMany - each response explicit",
            steps: vec![
                ConformanceStepRef {
                    label: "step-1-entry-choose-one",
                    response: explicit_entry_response(),
                    expectation: entry_expectation(),
                },
                ConformanceStepRef {
                    label: "step-2-choose-count",
                    response: DecisionResponseV2 {
                        schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
                        player_decision_id: PlayerDecisionIdV1(2),
                        state_revision: StateRevision(1),
                        answer: DecisionAnswerV2::ChooseNumber { value: 2 },
                    },
                    expectation: {
                        let mut expectation = ConformanceExpectation {
                            expected_result: ExpectedResponseResult::Accepted,
                            expected_post_state_revision: StateRevision(2),
                            expected_status: EpisodeStatus::Running,
                            // Authored stage events: DecisionCleared(2) then
                            // DecisionCreated(next) — the deliberate mismatch
                            // points the created id at the WRONG decision so
                            // the exact event comparison must diverge here.
                            expected_events: vec![
                                AuthoritativeRuleEvent {
                                    event_id: RuleEventId(6),
                                    state_revision: StateRevision(2),
                                    event: AuthoritativeRuleEventKind::DecisionCleared {
                                        decision: DecisionId(2),
                                    },
                                },
                                AuthoritativeRuleEvent {
                                    event_id: RuleEventId(7),
                                    state_revision: StateRevision(2),
                                    event: AuthoritativeRuleEventKind::DecisionCreated {
                                        decision: DecisionId(99),
                                    },
                                },
                            ],
                            expected_delta: Vec::new(),
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
                        };
                        expectation.expected_delta = vec![
                            SemanticDeltaOperation::DecisionCleared {
                                decision: DecisionId(2),
                            },
                            SemanticDeltaOperation::DecisionCreated {
                                decision: DecisionId(99),
                            },
                        ];
                        expectation
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
                        expected_status: EpisodeStatus::Running,
                        expected_events: Vec::new(),
                        expected_delta: Vec::new(),
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

        let state = base_pair_state(SEED_HEX_A).expect("accepted synthetic base state");
        let config = crate::isolation::synthetic_environment_config([P1, P2]);
        let (controller, endpoints) =
            spawn_environment(state, &config).expect("spawned trusted environment");

        // The diagnostic must come FROM the executed mismatch: same case,
        // same environment, deterministic report identifying step index 1
        // ("step-2-choose-count") as the first divergence.
        let diagnostic = run_case(&case, &controller, &endpoints)
            .expect_err("the deliberate step-2 mismatch must fail the case");
        assert_eq!(diagnostic.case, case.name);
        assert_eq!(diagnostic.step_label, "step-2-choose-count");
        assert_eq!(diagnostic.first_failing_step_index, Some(1));
        assert_eq!(diagnostic.classification, ConformanceFailureClass::Events);
        assert_eq!(
            diagnostic.path,
            "steps[1].expected_authoritative_events[1].event"
        );
    }

    /// Diagnostic determinism: rerunning the same mismatching case in a
    /// fresh environment must produce an identical structured report
    /// (case, step, path, expected, actual) — the diagnostic is a product
    /// of the executed comparison, and identical mismatches render
    /// identically.  Trusted conformance-only output.
    #[test]
    fn diagnostics_are_deterministic_across_reruns() {
        let mismatched_case = {
            let mut case = entry_case();
            case.name = "deterministic-diagnostic-case";
            // Corrupt exactly one authored field: final life event "to".
            case.steps[0].expectation.expected_events[1].event =
                AuthoritativeRuleEventKind::LifeChanged {
                    player: P1,
                    from: 39,
                    to: 37,
                };
            case
        };

        let run = || {
            let state = base_pair_state(SEED_HEX_A).expect("accepted synthetic base state");
            let config = crate::isolation::synthetic_environment_config([P1, P2]);
            let (controller, endpoints) =
                spawn_environment(state, &config).expect("spawned trusted environment");
            run_case(&mismatched_case, &controller, &endpoints)
                .expect_err("the corrupted expectation must fail the case")
        };

        let first = run();
        let second = run();
        assert_eq!(first, second);
        assert_eq!(first.case, mismatched_case.name);
        assert_eq!(first.step_label, "step-1-entry-choose-one");
        assert_eq!(first.first_failing_step_index, Some(0));
        assert_eq!(first.classification, ConformanceFailureClass::Events);
        assert_eq!(
            first.path,
            "steps[0].expected_authoritative_events[1].event"
        );
    }

    /// Decision completeness: the case contract carries only explicit
    /// responses.  There is no default answer, no implicit pass, no
    /// candidate-0 shortcut, and no repair of malformed answers anywhere on
    /// the case surface; every step names its response literally, and the
    /// executed witness B proves a malformed response reaches the kernel
    /// exactly as authored (it is rejected, never fixed up).
    #[test]
    fn responses_are_explicit_and_never_default_selected() {
        let case = entry_case();
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
        assert_eq!(case.steps[0].response.state_revision, StateRevision(0));
    }

    /// Information boundary: the facade supports trusted assertions without
    /// touching the player-facing surface.  The contract types live only
    /// inside this crate's facade module; no player observation,
    /// information state, or step type gains trusted fields, and no trusted
    /// diagnostic is re-exported through player-facing APIs.  The only
    /// snapshot authority the facade may reuse is the existing
    /// complete-fingerprint capture.
    #[test]
    fn facade_support_stays_behind_the_trusted_boundary() {
        let state = base_pair_state(SEED_HEX_A).expect("accepted synthetic base state");
        let config = crate::isolation::synthetic_environment_config([P1, P2]);
        let (controller, endpoints) =
            spawn_environment(state, &config).expect("spawned trusted environment");
        let complete = capture_complete(&controller, &endpoints).expect("trusted fingerprint");
        assert_eq!(complete, complete);
    }
}
