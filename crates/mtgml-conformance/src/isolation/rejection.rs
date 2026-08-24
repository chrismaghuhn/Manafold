//! Complete typed-rejection fingerprint matrix for M2.G G.4.
//!
//! Every row drives one rejected typed response through the REAL player
//! endpoint of a live runtime-accepted synthetic environment and proves,
//! per row, that the closed rejected outcome rides only the returned step
//! while the COMPLETE four-group fingerprint (semantic, environment,
//! player, replay recorder) is identical before and after.
//!
//! Frozen-program boundary documented by row
//! `choosemany_cardinality_above_max`: program coherence pins every
//! engine-offered ChooseMany request to `minimum == maximum ==
//! candidate_count`, and membership/uniqueness/canonicality strictly
//! precede cardinality in the decision authority's classification order,
//! so no well-formed submission can reach the `actual > maximum` arm
//! through any live request. That row drives the maximum-exceeding shape
//! and pins the actual closed classification as executable precedence
//! evidence instead of asserting an outcome this program cannot produce.
//! Every M2.G gate remains `NOT_RUN`; nothing here is a gate verdict.

/// Coverage tags of the executed semantic-rejection matrix below.
///
/// This list MUST mirror `SEMANTIC_CASES` in the test module row for row:
/// the coverage test fails closed when either side drifts apart, so a new
/// executed case without a pinned tag (or vice versa) can never silently
/// widen or shrink the declared evidence surface.
pub const SEMANTIC_REJECTION_ROWS: &[&str] = &[
    "stale_player_decision_id",
    "stale_state_revision",
    "unknown_candidate_id",
    "duplicate_selectmany_member",
    "choosemany_noncanonical_ordering",
    "choosemany_cardinality_below_min",
    "choosemany_cardinality_above_max",
    "choosenumber_below_min",
    "choosenumber_above_max",
    "order_duplicate_member",
    "order_invalid_member",
    "wrong_answer_union_variant",
    "episode_closed_terminal",
    "unavailable_foreign_actor",
    "unavailable_requestless_instant",
];

#[cfg(test)]
mod tests {
    use super::SEMANTIC_REJECTION_ROWS;
    use crate::isolation::fingerprint::{
        assert_fingerprint_policies, capture_complete, capture_transition_product,
        FingerprintComparison,
    };
    use crate::isolation::paired::test_support::accepted_entry_submission;
    use crate::isolation::paired::{
        base_pair_state, spawn_environment, synthetic_environment_config,
    };
    use crate::isolation::HarnessError;
    use mtgml_decision::{
        DecisionAnswerV2, DecisionResponseV2, PlayerDecisionRequestV2, DECISION_RESPONSE_V2_SCHEMA,
    };
    use mtgml_environment::{
        EnvironmentCheckpointV3, PlayerEndpoint, PlayerEndpointHandle, TrustedEnvironmentController,
    };
    use mtgml_model::{
        CandidateIdV1, EpisodeStatus, PlayerDecisionIdV1, PlayerId, StateRevision, TerminalReason,
    };
    use mtgml_observation::PlayerStepSubmissionV1;
    use mtgml_state::validate_engine_state;

    const P1: PlayerId = PlayerId(1);
    const P2: PlayerId = PlayerId(2);

    const SEED_HEX: &str = "5555555555555555555555555555555555555555555555555555555555555555";

    /// Assembly cardinality chosen while advancing the continuation chain;
    /// the resulting ChooseMany/Order surfaces hold exactly two dense
    /// candidates with inclusive bounds {2, 2}.
    const MEMBER_COUNT: i64 = 2;

    /// The one row whose literal expectation is unreachable in this frozen
    /// program; see the module documentation and
    /// [`above_maximum_answers_classify_before_the_cardinality_arm`].
    const ABOVE_MAXIMUM_ROW: &str = "choosemany_cardinality_above_max";

    #[derive(Debug, Clone, Copy)]
    enum Stage {
        Entry,
        Count,
        Members,
        Order,
        CompletedTerminal,
        Requestless,
    }

    struct SemanticCase {
        name: &'static str,
        stage: Stage,
        actor_index: usize,
        expected_code: &'static str,
        response: fn(Option<&PlayerDecisionRequestV2>) -> Result<DecisionResponseV2, HarnessError>,
    }

    const SEMANTIC_CASES: &[SemanticCase] = &[
        SemanticCase {
            name: "stale_player_decision_id",
            stage: Stage::Entry,
            actor_index: 0,
            expected_code: "stale_decision",
            response: stale_player_decision_id,
        },
        SemanticCase {
            name: "stale_state_revision",
            stage: Stage::Entry,
            actor_index: 0,
            expected_code: "stale_decision",
            response: stale_state_revision,
        },
        SemanticCase {
            name: "unknown_candidate_id",
            stage: Stage::Entry,
            actor_index: 0,
            expected_code: "invalid_candidate",
            response: unknown_candidate_id,
        },
        SemanticCase {
            name: "duplicate_selectmany_member",
            stage: Stage::Members,
            actor_index: 0,
            expected_code: "duplicate_assignment",
            response: duplicate_selectmany_member,
        },
        SemanticCase {
            name: "choosemany_noncanonical_ordering",
            stage: Stage::Members,
            actor_index: 0,
            expected_code: "invalid_order",
            response: choosemany_noncanonical_ordering,
        },
        SemanticCase {
            name: "choosemany_cardinality_below_min",
            stage: Stage::Members,
            actor_index: 0,
            expected_code: "invalid_cardinality",
            response: choosemany_cardinality_below_min,
        },
        SemanticCase {
            name: ABOVE_MAXIMUM_ROW,
            stage: Stage::Members,
            actor_index: 0,
            // Membership precedes cardinality, so the maximum-exceeding
            // shape classifies as invalid_candidate; fabricating an
            // invalid_cardinality outcome is impossible on any live request.
            expected_code: "invalid_candidate",
            response: choosemany_cardinality_above_max,
        },
        SemanticCase {
            name: "choosenumber_below_min",
            stage: Stage::Count,
            actor_index: 0,
            expected_code: "invalid_number",
            response: choosenumber_below_min,
        },
        SemanticCase {
            name: "choosenumber_above_max",
            stage: Stage::Count,
            actor_index: 0,
            expected_code: "invalid_number",
            response: choosenumber_above_max,
        },
        SemanticCase {
            name: "order_duplicate_member",
            stage: Stage::Order,
            actor_index: 0,
            expected_code: "duplicate_assignment",
            response: order_duplicate_member,
        },
        SemanticCase {
            name: "order_invalid_member",
            stage: Stage::Order,
            actor_index: 0,
            expected_code: "invalid_candidate",
            response: order_invalid_member,
        },
        SemanticCase {
            name: "wrong_answer_union_variant",
            stage: Stage::Entry,
            actor_index: 0,
            expected_code: "invalid_answer",
            response: wrong_answer_union_variant,
        },
        SemanticCase {
            name: "episode_closed_terminal",
            stage: Stage::CompletedTerminal,
            actor_index: 0,
            expected_code: "episode_closed",
            response: plausible,
        },
        SemanticCase {
            name: "unavailable_foreign_actor",
            stage: Stage::Entry,
            actor_index: 1,
            expected_code: "unavailable_decision",
            response: plausible,
        },
        SemanticCase {
            name: "unavailable_requestless_instant",
            stage: Stage::Requestless,
            actor_index: 0,
            expected_code: "unavailable_decision",
            response: plausible,
        },
    ];

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

    fn owned(
        request: Option<&PlayerDecisionRequestV2>,
    ) -> Result<&PlayerDecisionRequestV2, HarnessError> {
        request.ok_or(HarnessError::TransformPreconditionViolated)
    }

    fn answered(request: &PlayerDecisionRequestV2, answer: DecisionAnswerV2) -> DecisionResponseV2 {
        DecisionResponseV2 {
            schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
            player_decision_id: request.player_decision_id,
            state_revision: request.state_revision,
            answer,
        }
    }

    fn member(
        request: &PlayerDecisionRequestV2,
        index: usize,
    ) -> Result<CandidateIdV1, HarnessError> {
        request
            .candidates
            .get(index)
            .map(|candidate| candidate.candidate_id)
            .ok_or(HarnessError::TransformFixtureAbsent)
    }

    /// A candidate id beyond the dense request surface.
    fn beyond_surface(request: &PlayerDecisionRequestV2) -> Result<CandidateIdV1, HarnessError> {
        let beyond = u32::try_from(request.candidates.len())
            .map_err(|_| HarnessError::TransformPreconditionViolated)?
            .checked_add(9)
            .ok_or(HarnessError::TransformPreconditionViolated)?;
        Ok(CandidateIdV1(beyond))
    }

    fn stale_player_decision_id(
        request: Option<&PlayerDecisionRequestV2>,
    ) -> Result<DecisionResponseV2, HarnessError> {
        let request = owned(request)?;
        let mut response = answered(
            request,
            DecisionAnswerV2::SelectOne {
                candidate_id: member(request, 0)?,
            },
        );
        response.player_decision_id = PlayerDecisionIdV1(
            response
                .player_decision_id
                .0
                .checked_add(1)
                .ok_or(HarnessError::TransformPreconditionViolated)?,
        );
        Ok(response)
    }

    fn stale_state_revision(
        request: Option<&PlayerDecisionRequestV2>,
    ) -> Result<DecisionResponseV2, HarnessError> {
        let request = owned(request)?;
        let mut response = answered(
            request,
            DecisionAnswerV2::SelectOne {
                candidate_id: member(request, 0)?,
            },
        );
        response.state_revision = StateRevision(
            response
                .state_revision
                .0
                .checked_add(1)
                .ok_or(HarnessError::TransformPreconditionViolated)?,
        );
        Ok(response)
    }

    fn unknown_candidate_id(
        request: Option<&PlayerDecisionRequestV2>,
    ) -> Result<DecisionResponseV2, HarnessError> {
        let request = owned(request)?;
        Ok(answered(
            request,
            DecisionAnswerV2::SelectOne {
                candidate_id: beyond_surface(request)?,
            },
        ))
    }

    fn duplicate_selectmany_member(
        request: Option<&PlayerDecisionRequestV2>,
    ) -> Result<DecisionResponseV2, HarnessError> {
        let request = owned(request)?;
        Ok(answered(
            request,
            DecisionAnswerV2::SelectMany {
                candidate_ids: vec![member(request, 0)?, member(request, 0)?],
            },
        ))
    }

    fn choosemany_noncanonical_ordering(
        request: Option<&PlayerDecisionRequestV2>,
    ) -> Result<DecisionResponseV2, HarnessError> {
        let request = owned(request)?;
        Ok(answered(
            request,
            DecisionAnswerV2::SelectMany {
                candidate_ids: vec![member(request, 1)?, member(request, 0)?],
            },
        ))
    }

    fn choosemany_cardinality_below_min(
        request: Option<&PlayerDecisionRequestV2>,
    ) -> Result<DecisionResponseV2, HarnessError> {
        let request = owned(request)?;
        Ok(answered(
            request,
            DecisionAnswerV2::SelectMany {
                candidate_ids: vec![member(request, 0)?],
            },
        ))
    }

    fn choosemany_cardinality_above_max(
        request: Option<&PlayerDecisionRequestV2>,
    ) -> Result<DecisionResponseV2, HarnessError> {
        let request = owned(request)?;
        Ok(answered(
            request,
            DecisionAnswerV2::SelectMany {
                candidate_ids: vec![
                    member(request, 0)?,
                    member(request, 1)?,
                    beyond_surface(request)?,
                ],
            },
        ))
    }

    fn choosenumber_below_min(
        request: Option<&PlayerDecisionRequestV2>,
    ) -> Result<DecisionResponseV2, HarnessError> {
        let request = owned(request)?;
        let minimum = match request.decision {
            mtgml_decision::DecisionDomainV2::ChooseNumber { minimum, .. } => minimum,
            _ => return Err(HarnessError::TransformFixtureAbsent),
        };
        let value = minimum
            .checked_sub(1)
            .ok_or(HarnessError::TransformPreconditionViolated)?;
        Ok(answered(request, DecisionAnswerV2::ChooseNumber { value }))
    }

    fn choosenumber_above_max(
        request: Option<&PlayerDecisionRequestV2>,
    ) -> Result<DecisionResponseV2, HarnessError> {
        let request = owned(request)?;
        let maximum = match request.decision {
            mtgml_decision::DecisionDomainV2::ChooseNumber { maximum, .. } => maximum,
            _ => return Err(HarnessError::TransformFixtureAbsent),
        };
        let value = maximum
            .checked_add(1)
            .ok_or(HarnessError::TransformPreconditionViolated)?;
        Ok(answered(request, DecisionAnswerV2::ChooseNumber { value }))
    }

    fn order_duplicate_member(
        request: Option<&PlayerDecisionRequestV2>,
    ) -> Result<DecisionResponseV2, HarnessError> {
        let request = owned(request)?;
        Ok(answered(
            request,
            DecisionAnswerV2::Order {
                candidate_ids: vec![member(request, 0)?, member(request, 0)?],
            },
        ))
    }

    fn order_invalid_member(
        request: Option<&PlayerDecisionRequestV2>,
    ) -> Result<DecisionResponseV2, HarnessError> {
        let request = owned(request)?;
        Ok(answered(
            request,
            DecisionAnswerV2::Order {
                candidate_ids: vec![member(request, 0)?, beyond_surface(request)?],
            },
        ))
    }

    fn wrong_answer_union_variant(
        request: Option<&PlayerDecisionRequestV2>,
    ) -> Result<DecisionResponseV2, HarnessError> {
        let request = owned(request)?;
        Ok(answered(
            request,
            DecisionAnswerV2::ChooseNumber { value: 0 },
        ))
    }

    fn plausible(
        _request: Option<&PlayerDecisionRequestV2>,
    ) -> Result<DecisionResponseV2, HarnessError> {
        Ok(plausible_response())
    }

    fn visible_request(
        endpoint: &PlayerEndpointHandle,
    ) -> Result<PlayerDecisionRequestV2, HarnessError> {
        endpoint
            .visible_decision()
            .map_err(|_| HarnessError::EndpointService)?
            .ok_or(HarnessError::TransformPreconditionViolated)
    }

    fn submit_expected_accept(
        endpoint: &PlayerEndpointHandle,
        response: DecisionResponseV2,
    ) -> Result<(), HarnessError> {
        let step = endpoint
            .submit(response)
            .map_err(|_| HarnessError::EndpointService)?;
        assert_eq!(
            step.submission,
            PlayerStepSubmissionV1::Accepted,
            "stage advancement must be accepted"
        );
        Ok(())
    }

    fn accept_count(endpoints: &[PlayerEndpointHandle; 2], value: i64) -> Result<(), HarnessError> {
        let request = visible_request(&endpoints[0])?;
        submit_expected_accept(
            &endpoints[0],
            answered(&request, DecisionAnswerV2::ChooseNumber { value }),
        )
    }

    fn accept_members(endpoints: &[PlayerEndpointHandle; 2]) -> Result<(), HarnessError> {
        let request = visible_request(&endpoints[0])?;
        submit_expected_accept(
            &endpoints[0],
            answered(
                &request,
                DecisionAnswerV2::SelectMany {
                    candidate_ids: vec![member(&request, 0)?, member(&request, 1)?],
                },
            ),
        )
    }

    fn accept_order(endpoints: &[PlayerEndpointHandle; 2]) -> Result<(), HarnessError> {
        let request = visible_request(&endpoints[0])?;
        submit_expected_accept(
            &endpoints[0],
            answered(
                &request,
                DecisionAnswerV2::Order {
                    candidate_ids: vec![member(&request, 0)?, member(&request, 1)?],
                },
            ),
        )
    }

    /// Rebuilds the completed episode as a Terminal checkpoint and restores
    /// it in place, mirroring the established checkpoint-construction
    /// pattern; the bound handles stay valid across the restore.
    fn restore_terminal(controller: &TrustedEnvironmentController) -> Result<(), HarnessError> {
        let completed = controller
            .checkpoint()
            .map_err(|_| HarnessError::ControllerService)?;
        let terminal = EnvironmentCheckpointV3::new(
            completed.state.clone(),
            EpisodeStatus::Terminal {
                reason: TerminalReason::Concession,
                players: Vec::new(),
            },
            completed.limit_counters.clone(),
            completed.codec.clone(),
        )
        .map_err(|_| HarnessError::CheckpointInvalid)?;
        controller
            .restore(terminal)
            .map_err(|_| HarnessError::ControllerService)
    }

    fn spawned_pair(
    ) -> Result<(TrustedEnvironmentController, [PlayerEndpointHandle; 2]), HarnessError> {
        spawn_environment(base_pair_state(SEED_HEX)?, &config())
    }

    fn spawn_at(
        stage: Stage,
    ) -> Result<(TrustedEnvironmentController, [PlayerEndpointHandle; 2]), HarnessError> {
        match stage {
            Stage::Requestless => {
                let mut state = base_pair_state(SEED_HEX)?;
                state.execution.pending_decision = None;
                validate_engine_state(&state).map_err(HarnessError::StateValidation)?;
                spawn_environment(state, &config())
            }
            Stage::Entry => spawned_pair(),
            Stage::Count => {
                let pair = spawned_pair()?;
                accepted_entry_submission(&pair.1[0])?;
                Ok(pair)
            }
            Stage::Members => {
                let pair = spawned_pair()?;
                accepted_entry_submission(&pair.1[0])?;
                accept_count(&pair.1, MEMBER_COUNT)?;
                Ok(pair)
            }
            Stage::Order => {
                let pair = spawned_pair()?;
                accepted_entry_submission(&pair.1[0])?;
                accept_count(&pair.1, MEMBER_COUNT)?;
                accept_members(&pair.1)?;
                Ok(pair)
            }
            Stage::CompletedTerminal => {
                let pair = spawned_pair()?;
                accepted_entry_submission(&pair.1[0])?;
                accept_count(&pair.1, MEMBER_COUNT)?;
                accept_members(&pair.1)?;
                accept_order(&pair.1)?;
                restore_terminal(&pair.0)?;
                Ok(pair)
            }
        }
    }

    /// Every declared row drives its rejected submission through the real
    /// endpoint and leaves the COMPLETE fingerprint byte-identical.
    #[test]
    fn semantic_matrix_fingerprint_stable() -> Result<(), HarnessError> {
        for case in SEMANTIC_CASES {
            let (controller, endpoints) = spawn_at(case.stage)?;
            let actor = &endpoints[case.actor_index];
            let request = actor
                .visible_decision()
                .map_err(|_| HarnessError::EndpointService)?;
            let before = capture_complete(&controller, &endpoints)?;

            let response = (case.response)(request.as_ref())?;
            let step = actor
                .submit(response)
                .map_err(|_| HarnessError::EndpointService)?;
            let product = capture_transition_product(Ok(step))?;

            assert_eq!(
                product.semantic_submission_code.as_deref(),
                Some(case.expected_code),
                "row {} closed submission code",
                case.name
            );
            assert!(
                product.endpoint_error_code.is_none(),
                "row {} must reject semantically, not fail service-level",
                case.name
            );

            let after = capture_complete(&controller, &endpoints)?;
            assert_fingerprint_policies(&before, &after, FingerprintComparison::All)
                .unwrap_or_else(|error| {
                    panic!("row {} mutated the environment: {error:?}", case.name)
                });
        }
        Ok(())
    }

    /// Executable precedence evidence for the documented boundary row: both
    /// shapes whose cardinality exceeds the inclusive maximum classify in an
    /// earlier answer-set class and never reach the cardinality arm.
    #[test]
    fn above_maximum_answers_classify_before_the_cardinality_arm() -> Result<(), HarnessError> {
        let (controller, endpoints) = spawn_at(Stage::Members)?;
        let before = capture_complete(&controller, &endpoints)?;

        let attempts: [(bool, &str); 2] = [
            (false, "ascending with one beyond-surface id"),
            (true, "duplicate-heavy over-long set"),
        ];
        for (duplicated, description) in attempts {
            let request = visible_request(&endpoints[0])?;
            let candidate_ids = if duplicated {
                vec![
                    member(&request, 0)?,
                    member(&request, 0)?,
                    member(&request, 1)?,
                ]
            } else {
                vec![
                    member(&request, 0)?,
                    member(&request, 1)?,
                    beyond_surface(&request)?,
                ]
            };
            let expected = if duplicated {
                "duplicate_assignment"
            } else {
                "invalid_candidate"
            };

            let step = endpoints[0]
                .submit(answered(
                    &request,
                    DecisionAnswerV2::SelectMany { candidate_ids },
                ))
                .map_err(|_| HarnessError::EndpointService)?;
            let product = capture_transition_product(Ok(step))?;
            assert_eq!(
                product.semantic_submission_code.as_deref(),
                Some(expected),
                "{description} must classify before the cardinality arm"
            );

            let after = capture_complete(&controller, &endpoints)?;
            assert_fingerprint_policies(&before, &after, FingerprintComparison::All)
                .unwrap_or_else(|error| panic!("{description} mutated the environment: {error:?}"));
        }
        Ok(())
    }

    #[test]
    fn coverage_tags_match_executed_matrix() {
        assert_eq!(SEMANTIC_REJECTION_ROWS.len(), 15);
        assert_eq!(
            SEMANTIC_CASES.len(),
            SEMANTIC_REJECTION_ROWS.len(),
            "every pinned row must be executed by the matrix"
        );
        for case in SEMANTIC_CASES {
            assert!(
                SEMANTIC_REJECTION_ROWS.contains(&case.name),
                "row {} is not pinned by the coverage tag",
                case.name
            );
        }
        let mut unique = SEMANTIC_REJECTION_ROWS.to_vec();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(
            unique.len(),
            SEMANTIC_REJECTION_ROWS.len(),
            "coverage tags must be unique"
        );
        assert!(SEMANTIC_REJECTION_ROWS.contains(&ABOVE_MAXIMUM_ROW));
    }
}
