// Ownership fragment: typed rejection, nondisclosure, nonmutation evidence. Included lexically by tests.rs so
// every identity remains tests::<name>.

#[test]
fn rejected_submission_preserves_outer_environment_identity() {
    let controller = TrustedEnvironmentController::new(backend());
    let before_checkpoint = controller.checkpoint().unwrap();
    let before_replay = controller.export_replay().unwrap();
    let rejected = controller
        .execute_trusted_response(PlayerId(1), response(1, 0))
        .unwrap();

    assert!(!rejected.accepted);
    assert_eq!(controller.checkpoint().unwrap(), before_checkpoint);
    assert_eq!(controller.export_replay().unwrap(), before_replay);
}

#[test]
fn stale_endpoint_response_is_rejected_without_mutation() {
    let controller = TrustedEnvironmentController::new(backend());
    let p1 = controller.bind_player(PlayerId(1)).unwrap();
    let before = controller.checkpoint().unwrap();
    let rejected = p1.submit(response(0, 1)).unwrap();
    assert_eq!(
        rejected.submission,
        mtgml_observation::PlayerStepSubmissionV1::Rejected {
            code: mtgml_observation::PlayerSubmissionCodeV1::StaleDecision,
        }
    );
    assert_eq!(controller.checkpoint().unwrap(), before);
}

#[test]
fn player_api_errors_do_not_render_trusted_values() {
    let controller = TrustedEnvironmentController::new(backend());
    let p1 = controller.bind_player(PlayerId(1)).unwrap();

    // Typed rejections are Ok steps carrying only the closed code.
    let stale = p1.submit(response(0, 9)).unwrap();
    assert_eq!(
        stale.submission,
        mtgml_observation::PlayerStepSubmissionV1::Rejected {
            code: mtgml_observation::PlayerSubmissionCodeV1::StaleDecision,
        }
    );

    // Trusted controller errors stay inside orchestration; assert no
    // trusted detail leaks through their rendering either.
    let bind_failure = match controller.bind_player(PlayerId(9)) {
        Ok(_) => "bound".to_string(),
        Err(error) => format!("{error}"),
    };
    for vocabulary in ["seed", "digest", "checkpoint", "gameobject", "decisionid"] {
        assert!(
            !bind_failure.to_lowercase().contains(vocabulary),
            "leaked {vocabulary}"
        );
    }

    // Serialized typed rejections must not carry trusted detail either.
    let bytes = mtgml_wire::encode_canonical(&stale).unwrap();
    let rendered = String::from_utf8(bytes).unwrap().to_lowercase();
    for vocabulary in ["continuation", "binding", "gameobject", "decisionid"] {
        assert!(
            !rendered.contains(vocabulary),
            "leaked {vocabulary} in public step"
        );
    }
}

#[test]
fn malformed_wire_bytes_never_reach_the_semantic_endpoint() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use crate::endpoint::{PlayerEndpoint, PlayerEndpointError};

    /// Seam probe: counts every semantic `submit` crossing the A/B split.
    struct CountingEndpoint<'a> {
        inner: &'a dyn PlayerEndpoint,
        submit_calls: AtomicUsize,
    }
    impl PlayerEndpoint for CountingEndpoint<'_> {
        fn perspective(&self) -> PlayerId {
            self.inner.perspective()
        }
        fn observation(
            &self,
        ) -> Result<mtgml_observation::ObservationEnvelope, PlayerEndpointError> {
            self.inner.observation()
        }
        fn information_state(
            &self,
        ) -> Result<mtgml_observation::PlayerInformationStateV2, PlayerEndpointError> {
            self.inner.information_state()
        }
        fn visible_decision(
            &self,
        ) -> Result<Option<mtgml_decision::PlayerDecisionRequestV2>, PlayerEndpointError> {
            self.inner.visible_decision()
        }
        fn submit(
            &self,
            response: DecisionResponseV2,
        ) -> Result<PlayerStepV2, PlayerEndpointError> {
            self.submit_calls.fetch_add(1, Ordering::SeqCst);
            self.inner.submit(response)
        }
    }

    let controller = TrustedEnvironmentController::new(backend());
    let handle = controller.bind_player(PlayerId(1)).unwrap();
    let endpoint = CountingEndpoint {
        inner: &handle,
        submit_calls: AtomicUsize::new(0),
    };
    let before = public_fingerprint(&controller);
    let submit_count = |endpoint: &CountingEndpoint| endpoint.submit_calls.load(Ordering::SeqCst);

    let malformed = b"{not json";
    let boundary_error = crate::submit_response_bytes(&endpoint, malformed).unwrap_err();
    assert_eq!(boundary_error.code(), "malformed_response");
    assert_eq!(submit_count(&endpoint), 0, "malformed bytes reached submit");

    // Noncanonical: valid JSON, wrong key order.
    let canonical = mtgml_wire::encode_canonical(&response(0, 0)).unwrap();
    let mut noncanonical = Vec::with_capacity(canonical.len() + 1);
    noncanonical.push(b' ');
    noncanonical.extend_from_slice(&canonical);
    let boundary_error = crate::submit_response_bytes(&endpoint, &noncanonical).unwrap_err();
    assert_eq!(boundary_error.code(), "malformed_response");
    assert_eq!(
        submit_count(&endpoint),
        0,
        "noncanonical bytes reached submit"
    );

    // Wrong schema version.
    let wrong_schema = String::from_utf8(canonical.clone())
        .unwrap()
        .replace("decision-response.v2", "decision-response.v1")
        .into_bytes();
    let boundary_error = crate::submit_response_bytes(&endpoint, &wrong_schema).unwrap_err();
    assert_eq!(boundary_error.code(), "malformed_response");
    assert_eq!(
        submit_count(&endpoint),
        0,
        "wrong-schema bytes reached submit"
    );

    // Positive seam control: canonical bytes carrying a semantically
    // invalid answer do reach Layer B exactly once and return a typed
    // rejected PlayerStep there.
    let mut stale = response(0, 0);
    stale.state_revision = StateRevision(9);
    let stale_bytes = mtgml_wire::encode_canonical(&stale).unwrap();
    let step = crate::submit_response_bytes(&endpoint, &stale_bytes).unwrap();
    assert_eq!(
        step.submission,
        mtgml_observation::PlayerStepSubmissionV1::Rejected {
            code: mtgml_observation::PlayerSubmissionCodeV1::StaleDecision,
        }
    );
    assert_eq!(submit_count(&endpoint), 1);

    // Zero mutation across both layers.
    assert_eq!(public_fingerprint(&controller), before);
}

#[test]
fn request_existence_is_not_an_error_oracle() {
    let controller = TrustedEnvironmentController::new(backend());
    let p1 = controller.bind_player(PlayerId(1)).unwrap();
    let p2 = controller.bind_player(PlayerId(2)).unwrap();

    // Case A: no request exists at all. The continuation chain was fully
    // completed, so pending_decision is None while the episode stays
    // Running; P2 submits into a genuinely requestless state.
    let _ = submit_answer(&p1, order_entry_answer());
    let _ = submit_answer(&p1, number_answer(2));
    let _ = submit_answer(&p1, members_answer(&[0, 1]));
    let _ = submit_answer(&p1, order_answer(&[1, 0]));
    let no_request = p2.submit(response(0, 0)).unwrap();
    assert_eq!(
        no_request.submission,
        mtgml_observation::PlayerStepSubmissionV1::Rejected {
            code: mtgml_observation::PlayerSubmissionCodeV1::UnavailableDecision,
        }
    );

    // Case B: a decision exists but belongs to P1 (paired foreign-request state).
    let other = TrustedEnvironmentController::new(backend());
    let other_p2 = other.bind_player(PlayerId(2)).unwrap();
    let foreign = other_p2.submit(response(0, 0)).unwrap();
    assert_eq!(
        foreign.submission,
        mtgml_observation::PlayerStepSubmissionV1::Rejected {
            code: mtgml_observation::PlayerSubmissionCodeV1::UnavailableDecision,
        }
    );

    // The public rejection surface must not distinguish "no request
    // exists" from "the request belongs to another perspective" by
    // anything other than the legitimately changed environment product.
    let foreign_information = mtgml_wire::encode_canonical(&foreign.information_state).unwrap();
    let no_request_information =
        mtgml_wire::encode_canonical(&no_request.information_state).unwrap();
    assert_ne!(
        foreign_information, no_request_information,
        "states differ legitimately; the CODE must not"
    );
    let code_of = |submission: &mtgml_observation::PlayerStepSubmissionV1| match submission {
        mtgml_observation::PlayerStepSubmissionV1::Rejected { code } => *code,
        other => panic!("expected rejected submission, got {other:?}"),
    };
    assert_eq!(
        code_of(&foreign.submission),
        code_of(&no_request.submission),
        "request existence leaked as a distinct code"
    );
}

#[test]
fn typed_rejection_codes_matrix() {
    let controller = TrustedEnvironmentController::new(backend());
    let p1 = controller.bind_player(PlayerId(1)).unwrap();

    let expected_code = |step: &PlayerStepV2| -> mtgml_observation::PlayerSubmissionCodeV1 {
        match &step.submission {
            mtgml_observation::PlayerStepSubmissionV1::Rejected { code } => *code,
            other => panic!("expected rejected submission, got {other:?}"),
        }
    };

    // Accept entry to reach stage 0 (ChooseCount).
    let _ = submit_answer(&p1, order_entry_answer());

    // stale_decision via revision mismatch.
    let visible = p1.visible_decision().unwrap().unwrap();
    let stale = p1
        .submit(mtgml_decision::DecisionResponseV2 {
            schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
            player_decision_id: visible.player_decision_id,
            state_revision: StateRevision(9),
            answer: number_answer(1),
        })
        .unwrap();
    assert_eq!(
        expected_code(&stale),
        mtgml_observation::PlayerSubmissionCodeV1::StaleDecision,
        "stale"
    );

    // invalid_answer via Order variant against ChooseNumber domain.
    let wrong_variant = p1
        .submit(mtgml_decision::DecisionResponseV2 {
            schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
            player_decision_id: visible.player_decision_id,
            state_revision: visible.state_revision,
            answer: mtgml_decision::DecisionAnswerV2::Order {
                candidate_ids: vec![],
            },
        })
        .unwrap();
    assert_eq!(
        expected_code(&wrong_variant),
        mtgml_observation::PlayerSubmissionCodeV1::InvalidAnswer,
        "invalid_answer"
    );

    // Advance to ChooseMembers{2,2}.
    let _ = submit_answer(&p1, number_answer(2));

    // invalid_candidate.
    let request = p1.visible_decision().unwrap().unwrap();
    let unknown = p1
        .submit(mtgml_decision::DecisionResponseV2 {
            schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
            player_decision_id: request.player_decision_id,
            state_revision: request.state_revision,
            answer: members_answer(&[7]),
        })
        .unwrap();
    assert_eq!(
        expected_code(&unknown),
        mtgml_observation::PlayerSubmissionCodeV1::InvalidCandidate,
        "invalid_candidate"
    );

    // duplicate_assignment.
    let dup = p1
        .submit(mtgml_decision::DecisionResponseV2 {
            schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
            player_decision_id: request.player_decision_id,
            state_revision: request.state_revision,
            answer: members_answer(&[0, 0]),
        })
        .unwrap();
    assert_eq!(
        expected_code(&dup),
        mtgml_observation::PlayerSubmissionCodeV1::DuplicateAssignment,
        "duplicate_assignment"
    );

    // invalid_cardinality.
    let card = p1
        .submit(mtgml_decision::DecisionResponseV2 {
            schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
            player_decision_id: request.player_decision_id,
            state_revision: request.state_revision,
            answer: members_answer(&[0]),
        })
        .unwrap();
    assert_eq!(
        expected_code(&card),
        mtgml_observation::PlayerSubmissionCodeV1::InvalidCardinality,
        "invalid_cardinality"
    );

    // invalid_order (noncanonical representation).
    let order = p1
        .submit(mtgml_decision::DecisionResponseV2 {
            schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
            player_decision_id: request.player_decision_id,
            state_revision: request.state_revision,
            answer: members_answer(&[1, 0]),
        })
        .unwrap();
    assert_eq!(
        expected_code(&order),
        mtgml_observation::PlayerSubmissionCodeV1::InvalidOrder,
        "invalid_order"
    );

    // Complete the continuation (clears pending_decision).
    let _ = submit_answer(&p1, members_answer(&[0, 1]));
    let _ = submit_answer(&p1, order_answer(&[1, 0]));

    // Build a truncated checkpoint to drive episode_closed.
    let completed_state = controller.checkpoint().unwrap().state;
    let truncated_checkpoint = EnvironmentCheckpointV3::new(
        completed_state,
        EpisodeStatus::Truncated {
            reason: TruncationReason::ExternalStop,
            players: vec![],
        },
        EnvironmentLimitCounters::default(),
        CheckpointCodecIdentity {
            codec_id: "synthetic-m2-memory".into(),
            semantic_version: "3".into(),
        },
    )
    .unwrap();
    let truncated_env = TrustedEnvironmentController::new(
        SyntheticM1EnvironmentBackend::from_checkpoint(
            truncated_checkpoint,
            config([PlayerId(1), PlayerId(2)]),
        )
        .unwrap(),
    );
    let truncated_p1 = truncated_env.bind_player(PlayerId(1)).unwrap();
    let closed = truncated_p1.submit(response(0, 99)).unwrap();
    assert_eq!(
        expected_code(&closed),
        mtgml_observation::PlayerSubmissionCodeV1::EpisodeClosed,
        "episode_closed"
    );
}

#[test]
fn internal_failures_surface_only_service_unavailable() {
    use mtgml_state::PendingDecisionRecordV2;

    // A structurally valid generic state with a standalone ChooseNumber
    // pending request is not executable by the synthetic kernel; restore
    // must reject it before any projection can expose it.
    let mut state =
        mtgml_state::construct_synthetic_engine_state(mtgml_state::SyntheticResetInputs {
            players: [PlayerId(1), PlayerId(2)],
            root_seed: seed(),
        })
        .unwrap();
    state.execution.pending_decision = Some(PendingDecisionRecordV2 {
        request: mtgml_decision::AuthoritativeDecisionRequestV2 {
            decision_id: mtgml_model::DecisionId(1),
            player_decision_id: PlayerDecisionIdV1(1),
            state_revision: StateRevision(0),
            actor: PlayerId(1),
            visibility: mtgml_decision::DecisionVisibility::Public,
            decision: mtgml_decision::DecisionDomainV2::ChooseNumber {
                minimum: 0,
                maximum: 3,
            },
            candidates: Vec::new(),
            continuation_id: None,
        },
    });
    let checkpoint = EnvironmentCheckpointV3::new(
        state,
        EpisodeStatus::Running,
        EnvironmentLimitCounters::default(),
        CheckpointCodecIdentity {
            codec_id: "synthetic-m2-memory".into(),
            semantic_version: "3".into(),
        },
    )
    .unwrap();

    let result = SyntheticM1EnvironmentBackend::from_checkpoint(
        checkpoint.clone(),
        config([PlayerId(1), PlayerId(2)]),
    );
    let error = match result {
        Err(error) => error,
        Ok(_) => panic!("unsupported state must be rejected"),
    };
    let rendered = format!("{error}");
    for vocabulary in ["seed", "digest", "gameobject", "decisionid", "continuation"] {
        assert!(
            !rendered.to_lowercase().contains(vocabulary),
            "leaked {vocabulary}"
        );
    }

    // The same closed surface must hold across the public player boundary:
    // an internal service defect driven through `PlayerEndpoint::submit`
    // (here: authoritative limit-counter exhaustion while committing an
    // otherwise fully accepted submission) may not disclose trusted detail
    // and must map to exactly `service_unavailable`.
    let players = [PlayerId(1), PlayerId(2)];
    let fresh = backend().checkpoint().unwrap();
    let exhausted_checkpoint = EnvironmentCheckpointV3::new(
        fresh.state,
        fresh.status.clone(),
        EnvironmentLimitCounters {
            decisions_submitted: u64::MAX,
            ..fresh.limit_counters.clone()
        },
        fresh.codec.clone(),
    )
    .unwrap();
    let exhausted_controller = TrustedEnvironmentController::new(
        SyntheticM1EnvironmentBackend::from_checkpoint(exhausted_checkpoint, config(players))
            .unwrap(),
    );
    let p1 = exhausted_controller.bind_player(PlayerId(1)).unwrap();
    let before = public_fingerprint(&exhausted_controller);
    let request = p1
        .visible_decision()
        .unwrap()
        .expect("entry decision visible");
    let bytes = mtgml_wire::encode_canonical(&DecisionResponseV2 {
        schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
        player_decision_id: request.player_decision_id,
        state_revision: request.state_revision,
        answer: order_entry_answer(),
    })
    .unwrap();
    let boundary_error = crate::submit_response_bytes(&p1, &bytes).unwrap_err();
    assert_eq!(boundary_error.code(), "service_unavailable");

    // The failed internal commit must not have mutated anything.
    assert_eq!(public_fingerprint(&exhausted_controller), before);
}

// ---------------------------------------------------------------- M2.E projection
