// Ownership fragment: checkpoint/restore/fork/replay parity evidence. Included lexically by tests.rs so
// every identity remains tests::<name>.

#[test]
fn checkpoint_v4_validation_and_restore_nonmutation_matrix() {
    let checkpoint = backend().checkpoint().unwrap();
    checkpoint.validate().unwrap();
    assert_eq!(checkpoint.schema_version, ENVIRONMENT_CHECKPOINT_SCHEMA_V6);
    assert_eq!(checkpoint.state_digest, checkpoint.state.digest().unwrap());
    assert_eq!(checkpoint.state_digest.raw_bytes().len(), 32);
    assert!(!checkpoint.codec.codec_id.is_empty());
    assert!(!checkpoint.codec.semantic_version.is_empty());

    // Corrupting any authoritative checkpoint field must be rejected.
    let corrupt_state_digest = |mutate: fn(&mut EnvironmentCheckpointV6)| {
        let mut corrupted = backend().checkpoint().unwrap();
        mutate(&mut corrupted);
        corrupted.validate().is_err()
    };
    assert!(corrupt_state_digest(|c| {
        c.state_digest = FullStateDigestV5::from_digest_bytes([0xff; 32]);
    }));
    assert!(corrupt_state_digest(|c| {
        c.checkpoint_digest = CheckpointDigestV6::from_digest_bytes([0xee; 32]);
    }));
    assert!(corrupt_state_digest(|c| {
        c.status = EpisodeStatus::Terminal {
            reason: TerminalReason::Concession,
            players: vec![],
        };
    }));
    assert!(corrupt_state_digest(|c| {
        c.limit_counters.accepted_transitions += 1;
    }));
    assert!(corrupt_state_digest(|c| {
        c.limit_counters.decisions_submitted += 1;
    }));
    assert!(corrupt_state_digest(|c| {
        c.codec.codec_id = "unsupported-codec".into();
    }));
    assert!(corrupt_state_digest(|c| {
        c.codec.semantic_version.clear();
    }));
    assert!(corrupt_state_digest(|c| {
        c.schema_version = "environment-checkpoint.v2".into();
    }));

    // Restoring a corrupted checkpoint must leave the backend untouched.
    let backend_controller = TrustedEnvironmentController::new(backend());
    let before_checkpoint = backend_controller.checkpoint().unwrap();
    let before_replay = backend_controller.export_replay().unwrap();
    let mut corrupted = before_checkpoint.clone();
    corrupted.limit_counters.decisions_submitted += 1;
    assert!(backend_controller.restore(corrupted).is_err());
    let mut corrupted = before_checkpoint.clone();
    corrupted.status = EpisodeStatus::Truncated {
        reason: TruncationReason::ExternalStop,
        players: vec![],
    };
    // The digest no longer matches, so restore must fail closed.
    assert!(backend_controller.restore(corrupted).is_err());
    let mut corrupted = before_checkpoint.clone();
    corrupted.codec = CheckpointCodecIdentity {
        codec_id: "other".into(),
        semantic_version: "9".into(),
    };
    assert!(backend_controller.restore(corrupted).is_err());
    assert_eq!(
        backend_controller.checkpoint().unwrap(),
        before_checkpoint,
        "failed restores must not mutate the backend"
    );
    assert_eq!(backend_controller.export_replay().unwrap(), before_replay);

    // A valid restore reproduces exact identity and rebases replay.
    let source = TrustedEnvironmentController::new(backend());
    source
        .execute_trusted_response(PlayerId(1), response(0, 0))
        .unwrap();
    let advanced = source.checkpoint().unwrap();
    let target = TrustedEnvironmentController::new(backend());
    target.restore(advanced.clone()).unwrap();
    assert_eq!(target.checkpoint().unwrap(), advanced);
    let rebased = target.export_replay().unwrap();
    assert!(rebased.steps.is_empty());
    assert_eq!(
        rebased.manifest.initial_identity.checkpoint_digest,
        advanced.checkpoint_digest
    );
}

#[test]
fn accepted_endpoint_submission_commits_v3_state_delta_and_replay() {
    let controller = TrustedEnvironmentController::new(backend());
    let before = controller.checkpoint().unwrap();
    let p1 = controller.bind_player(PlayerId(1)).unwrap();

    let step = p1.submit(response(0, 0)).unwrap();
    step.validate().unwrap();
    assert_eq!(step.schema_version, PLAYER_STEP_SCHEMA_V2);
    assert_eq!(step.information_state.state_revision, StateRevision(1));
    // The entry acceptance creates the continuation and exposes stage 0.
    assert!(step.next_decision.is_some());
    assert_eq!(
        step.next_decision.as_ref().unwrap().decision,
        mtgml_decision::DecisionDomainV2::ChooseNumber {
            minimum: 0,
            maximum: 3
        }
    );

    let after = controller.checkpoint().unwrap();
    assert_eq!(after.state.revision, StateRevision(1));
    assert_eq!(after.state.core.players[&PlayerId(1)].life, 38);
    assert_eq!(after.limit_counters.decisions_submitted, 1);
    assert_eq!(after.limit_counters.accepted_transitions, 1);
    assert_eq!(after.state_digest, after.state.digest().unwrap());

    let replay = controller.export_replay().unwrap();
    replay.validate().unwrap();
    assert_eq!(replay.steps.len(), 1);
    assert_eq!(
        replay.steps[0].checkpoint_digest_before,
        before.checkpoint_digest
    );
    assert_eq!(replay.steps[0].full_state_digest_after, after.state_digest);
    assert_eq!(replay.final_identity.full_state_digest, after.state_digest);
    let bytes = mtgml_wire::encode_canonical(&replay).unwrap();
    let decoded: AuthoritativeReplayV6 = mtgml_wire::decode_canonical(&bytes).unwrap();
    assert_eq!(decoded, replay);

    // Drive the remaining stages through the bound endpoint.
    let stage0 = p1.visible_decision().unwrap().unwrap();
    let count_step = p1
        .submit(mtgml_decision::DecisionResponseV2 {
            schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
            player_decision_id: stage0.player_decision_id,
            state_revision: stage0.state_revision,
            answer: mtgml_decision::DecisionAnswerV2::ChooseNumber { value: 2 },
        })
        .unwrap();
    count_step.validate().unwrap();
    let stage1 = p1.visible_decision().unwrap().unwrap();
    assert_eq!(
        stage1.decision,
        mtgml_decision::DecisionDomainV2::ChooseMany {
            minimum: 2,
            maximum: 2
        }
    );
    let members_step = p1
        .submit(mtgml_decision::DecisionResponseV2 {
            schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
            player_decision_id: stage1.player_decision_id,
            state_revision: stage1.state_revision,
            answer: mtgml_decision::DecisionAnswerV2::SelectMany {
                candidate_ids: vec![CandidateIdV1(0), CandidateIdV1(1)],
            },
        })
        .unwrap();
    members_step.validate().unwrap();
    let stage2 = p1.visible_decision().unwrap().unwrap();
    let order_step = p1
        .submit(mtgml_decision::DecisionResponseV2 {
            schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
            player_decision_id: stage2.player_decision_id,
            state_revision: stage2.state_revision,
            answer: mtgml_decision::DecisionAnswerV2::Order {
                candidate_ids: vec![CandidateIdV1(1), CandidateIdV1(0)],
            },
        })
        .unwrap();
    order_step.validate().unwrap();
    // Completion removes the continuation and clears pending decisions.
    assert!(p1.visible_decision().unwrap().is_none());
    let completed = controller.checkpoint().unwrap();
    assert!(completed.state.execution.continuations.is_empty());
    assert_eq!(controller.export_replay().unwrap().steps.len(), 4);
}

#[test]
fn checkpoint_restore_and_fork_are_exact_and_rebase_replay() {
    let source = TrustedEnvironmentController::new(backend());
    let c0 = source.checkpoint().unwrap();
    let fork_a = source.fork().unwrap();
    let fork_b = source.fork().unwrap();
    let transition_a = fork_a
        .execute_trusted_response(PlayerId(1), response(0, 0))
        .unwrap();
    let transition_b = fork_b
        .execute_trusted_response(PlayerId(1), response(0, 0))
        .unwrap();

    assert_eq!(transition_a, transition_b);
    assert_eq!(fork_a.checkpoint().unwrap(), fork_b.checkpoint().unwrap());
    assert_eq!(
        fork_a.export_replay().unwrap(),
        fork_b.export_replay().unwrap()
    );
    assert_eq!(source.checkpoint().unwrap(), c0);

    source.restore(fork_a.checkpoint().unwrap()).unwrap();
    let rebased = source.export_replay().unwrap();
    assert!(rebased.steps.is_empty());
    assert_eq!(
        rebased.manifest.initial_identity.state_revision,
        StateRevision(1)
    );
    assert_eq!(rebased.final_identity.state_revision, StateRevision(1));
}

#[test]
fn semantic_replay_reproduces_the_authoritative_transition() {
    let controller = TrustedEnvironmentController::new(backend());
    let c0 = controller.checkpoint().unwrap();
    let live = controller
        .execute_trusted_response(PlayerId(1), response(0, 0))
        .unwrap();
    let after = controller.checkpoint().unwrap();
    let replay = controller.export_replay().unwrap();

    let report = controller
        .execute_replay_from_checkpoint(c0, replay)
        .unwrap();
    assert_eq!(report.traces.len(), 1);
    assert_eq!(report.traces[0].transition, live);
    assert_eq!(report.traces[0].after, after);
    assert_eq!(report.final_checkpoint, after);
}

#[test]
fn replay_rejects_wrong_player_decision_id_before_trusted_execution() {
    let controller = TrustedEnvironmentController::new(backend());
    let checkpoint = controller.checkpoint().unwrap();
    controller
        .execute_trusted_response(PlayerId(1), response(0, 0))
        .unwrap();
    let live_replay = controller.export_replay().unwrap();
    let live_checkpoint = controller.checkpoint().unwrap();
    let initial = live_replay.manifest.initial_identity.clone();

    // A detached replay can validate the response shape without reconstructing
    // the authoritative request. The execution boundary must still bind the
    // player-decision identity before trusted execution.
    let mut tampered = live_replay.clone();
    let step = &mut tampered.steps[0];
    step.response.player_decision_id = PlayerDecisionIdV1(999);
    step.accepted = false;
    step.state_revision_after = initial.state_revision;
    step.full_state_digest_after = initial.full_state_digest.clone();
    step.episode_status_after = initial.episode_status.clone();
    step.environment_limit_counters_after = initial.environment_limit_counters.clone();
    step.checkpoint_digest_after = initial.checkpoint_digest.clone();
    tampered.final_identity = initial;
    tampered.validate().unwrap();

    let result = controller.execute_replay_from_checkpoint(checkpoint, tampered);
    assert!(matches!(
        result,
        Err(ControllerError::ReplayExecution(
            ReplayExecutionError::PlayerDecisionIdentityMismatch { step_index: 0 }
        ))
    ));
    assert_eq!(controller.checkpoint().unwrap(), live_checkpoint);
    assert_eq!(controller.export_replay().unwrap(), live_replay);
}

#[test]
fn closed_status_trusted_execution_is_rejected_without_mutation() {
    let controller = environment_at_members_stage();
    let p1 = controller.bind_player(PlayerId(1)).unwrap();
    let _ = submit_answer(&p1, members_answer(&[0, 1]));
    let _ = submit_answer(&p1, order_answer(&[1, 0]));
    let state = controller.checkpoint().unwrap().state;
    let codec = CheckpointCodecIdentity {
        codec_id: "in-memory-reference".into(),
        semantic_version: "6".into(),
    };

    for status in [
        EpisodeStatus::Terminal {
            reason: TerminalReason::Concession,
            players: vec![
                PlayerOutcome {
                    player: PlayerId(1),
                    result: PlayerResult::Loss,
                },
                PlayerOutcome {
                    player: PlayerId(2),
                    result: PlayerResult::Win,
                },
            ],
        },
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
        },
    ] {
        let checkpoint = EnvironmentCheckpointV6::new(
            state.clone(),
            status,
            EnvironmentLimitCounters::default(),
            codec.clone(),
        synthetic_identity()
        )
        .unwrap();
        let closed = TrustedEnvironmentController::new(
            SyntheticM1EnvironmentBackend::from_checkpoint(
                checkpoint,
                config([PlayerId(1), PlayerId(2)]),
            )
            .unwrap(),
        );
        let before = closed.checkpoint().unwrap();
        let replay = closed.export_replay().unwrap();
        let transition = closed
            .execute_trusted_response(PlayerId(1), response(0, state.revision.0))
            .unwrap();
        assert!(!transition.accepted);
        assert_eq!(closed.checkpoint().unwrap(), before);
        assert_eq!(closed.export_replay().unwrap(), replay);
    }
}

#[test]
fn accepted_trusted_counters_are_recomputed_exactly() {
    let controller = TrustedEnvironmentController::new(backend());
    let before = controller.checkpoint().unwrap();
    let transition = controller
        .execute_trusted_response(PlayerId(1), response(0, 0))
        .unwrap();
    assert!(transition.accepted);
    let after = controller.checkpoint().unwrap();
    assert_eq!(
        after.limit_counters.decisions_submitted,
        before.limit_counters.decisions_submitted + 1
    );
    assert_eq!(
        after.limit_counters.accepted_transitions,
        before.limit_counters.accepted_transitions + 1
    );
    assert_eq!(
        after.limit_counters.rule_events_emitted,
        before.limit_counters.rule_events_emitted
            + u64::try_from(transition.events.len()).unwrap()
    );
    assert_eq!(
        after.limit_counters.resource_units_consumed,
        before.limit_counters.resource_units_consumed
    );
    assert_eq!(
        after.limit_counters.wall_clock_elapsed_millis,
        before.limit_counters.wall_clock_elapsed_millis
    );
}

#[test]
fn closed_status_player_outcomes_require_authoritative_player_universe() {
    let (state, _) = two_perspective_outcome_product();
    let codec = CheckpointCodecIdentity {
        codec_id: "in-memory-reference".into(),
        semantic_version: "6".into(),
    };
    let statuses = [
        EpisodeStatus::Terminal {
            reason: TerminalReason::Concession,
            players: vec![],
        },
        EpisodeStatus::Terminal {
            reason: TerminalReason::Concession,
            players: vec![PlayerOutcome {
                player: PlayerId(999),
                result: PlayerResult::Win,
            }],
        },
        EpisodeStatus::Terminal {
            reason: TerminalReason::Concession,
            players: vec![
                PlayerOutcome {
                    player: PlayerId(1),
                    result: PlayerResult::Win,
                },
                PlayerOutcome {
                    player: PlayerId(1),
                    result: PlayerResult::Loss,
                },
            ],
        },
        EpisodeStatus::Terminal {
            reason: TerminalReason::Concession,
            players: vec![
                PlayerOutcome {
                    player: PlayerId(1),
                    result: PlayerResult::Win,
                },
                PlayerOutcome {
                    player: PlayerId(2),
                    result: PlayerResult::Loss,
                },
            ],
        },
    ];
    assert!(
        EnvironmentCheckpointV6::new(
            state.clone(),
            statuses[0].clone(),
            EnvironmentLimitCounters::default(),
            codec.clone(),
        synthetic_identity()
        )
        .is_err()
    );
    assert!(
        EnvironmentCheckpointV6::new(
            state.clone(),
            statuses[1].clone(),
            EnvironmentLimitCounters::default(),
            codec.clone(),
        synthetic_identity()
        )
        .is_err()
    );
    assert!(
        EnvironmentCheckpointV6::new(
            state.clone(),
            statuses[2].clone(),
            EnvironmentLimitCounters::default(),
            codec.clone(),
        synthetic_identity()
        )
        .is_err()
    );
    assert!(
        EnvironmentCheckpointV6::new(
            state,
            statuses[3].clone(),
            EnvironmentLimitCounters::default(),
            codec,
        synthetic_identity()
        )
        .is_ok()
    );
}

#[test]
fn checkpoint_identity_tampering_is_rejected() {
    let mut checkpoint = backend().checkpoint().unwrap();
    checkpoint.state_digest = FullStateDigestV5::from_digest_bytes([0xff; 32]);
    assert_eq!(
        checkpoint.validate().unwrap_err(),
        CheckpointValidationError::StateDigest
    );
}

#[test]
fn checkpoint_restore_repeats_exact_transition_and_replay_segment() {
    let source = TrustedEnvironmentController::new(backend());
    let initial = source.checkpoint().unwrap();
    let live = source
        .execute_trusted_response(PlayerId(1), response(0, 0))
        .unwrap();
    let live_after = source.checkpoint().unwrap();
    let live_replay = source.export_replay().unwrap();

    let restored = TrustedEnvironmentController::new(backend());
    restored.restore(initial).unwrap();
    let repeated = restored
        .execute_trusted_response(PlayerId(1), response(0, 0))
        .unwrap();
    assert_eq!(repeated, live);
    assert_eq!(restored.checkpoint().unwrap(), live_after);
    assert_eq!(restored.export_replay().unwrap(), live_replay);
}

#[test]
fn forks_diverge_only_on_explicit_input() {
    let source = TrustedEnvironmentController::new(backend());
    let before_fork = source.checkpoint().unwrap();

    let accepted_fork = source.fork().unwrap();
    let rejected_fork = source.fork().unwrap();
    assert_eq!(accepted_fork.checkpoint().unwrap(), before_fork);

    // A rejected diagnostic input leaves the fork at the shared identity.
    rejected_fork
        .execute_trusted_response(PlayerId(1), response(1, 0))
        .unwrap();
    assert_eq!(rejected_fork.checkpoint().unwrap(), before_fork);

    // Only an explicitly accepted input diverges the fork.
    let transition = accepted_fork
        .execute_trusted_response(PlayerId(1), response(0, 0))
        .unwrap();
    assert!(transition.accepted);
    let diverged = accepted_fork.checkpoint().unwrap();
    assert_ne!(diverged, before_fork);
    assert_eq!(
        source.checkpoint().unwrap(),
        before_fork,
        "the source must not observe fork inputs"
    );
}

#[test]
fn semantic_replay_rejects_tampered_identity_without_live_mutation() {
    use mtgml_persistence::checkpoint_digest::calculate_checkpoint_digest_v6;
    use mtgml_replay::InitialEnvironmentIdentityV6 as Identity;

    fn recompute(identity: &Identity) -> Identity {
        let mut fixed = identity.clone();
        fixed.checkpoint_digest = calculate_checkpoint_digest_v6(
            &fixed.full_state_digest.as_digest_reference(),
            &fixed.episode_status,
            &fixed.environment_limit_counters,
            &fixed.checkpoint_codec_identity,
            &fixed.execution_identity,
        )
        .unwrap();
        fixed
    }

    let controller = TrustedEnvironmentController::new(backend());
    let c0 = controller.checkpoint().unwrap();
    controller
        .execute_trusted_response(PlayerId(1), response(0, 0))
        .unwrap();
    let after = controller.checkpoint().unwrap();
    let live_replay = controller.export_replay().unwrap();
    let live_counters = after.limit_counters.clone();

    let run = |replay: AuthoritativeReplayV6| {
        let fresh = TrustedEnvironmentController::new(backend());
        fresh.execute_replay_from_checkpoint(c0.clone(), replay)
    };

    // Wrong actor is rejected before execution.
    let mut tampered = live_replay.clone();
    tampered.steps[0].actor = PlayerId(2);
    assert!(matches!(
        run(tampered),
        Err(ControllerError::ReplayExecution(
            ReplayExecutionError::ActorUnavailable { step_index: 0 }
        ))
    ));

    // A recorded counter divergence is rejected against the recomputed product.
    let mut tampered = live_replay.clone();
    tampered.steps[0]
        .environment_limit_counters_after
        .rule_events_emitted += 2;
    tampered.steps[0].checkpoint_digest_after = {
        let identity = Identity {
            state_revision: tampered.steps[0].state_revision_after,
            full_state_digest: tampered.steps[0].full_state_digest_after.clone(),
            episode_status: tampered.steps[0].episode_status_after.clone(),
            environment_limit_counters: tampered.steps[0].environment_limit_counters_after.clone(),
            checkpoint_codec_identity: tampered
                .manifest
                .initial_identity
                .checkpoint_codec_identity
                .clone(),
            checkpoint_digest: mtgml_model::CheckpointDigestV6::from_digest_bytes([0; 32]),
            execution_identity: tampered.manifest.initial_identity.execution_identity.clone(),
        };
        let identity = recompute(&identity);
        tampered.final_identity = identity.clone();
        identity.checkpoint_digest
    };
    // Deterministic rule-event counters cannot be supplied as external trace
    // data, so the replay fails before applying the candidate checkpoint.
    assert!(matches!(
        run(tampered),
        Err(ControllerError::ReplayExecution(ReplayExecutionError::CounterMismatch {
            step_index: 0
        }))
    ));

    // A wrong final full-state digest is rejected after execution.
    let mut tampered = live_replay.clone();
    tampered.steps[0].full_state_digest_after = FullStateDigestV5::from_digest_bytes([7; 32]);
    let identity = Identity {
        state_revision: tampered.steps[0].state_revision_after,
        full_state_digest: tampered.steps[0].full_state_digest_after.clone(),
        episode_status: tampered.steps[0].episode_status_after.clone(),
        environment_limit_counters: tampered.steps[0].environment_limit_counters_after.clone(),
        checkpoint_codec_identity: tampered
            .manifest
            .initial_identity
            .checkpoint_codec_identity
            .clone(),
        checkpoint_digest: mtgml_model::CheckpointDigestV6::from_digest_bytes([0; 32]),
        execution_identity: tampered.manifest.initial_identity.execution_identity.clone(),
    };
    let identity = recompute(&identity);
    tampered.steps[0].checkpoint_digest_after = identity.checkpoint_digest.clone();
    tampered.final_identity = identity;
    assert!(matches!(
        run(tampered),
        Err(ControllerError::ReplayExecution(
            ReplayExecutionError::AfterDigestMismatch { step_index: 0 }
        ))
    ));

    // A different root seed cannot masquerade as the same environment.
    let mut tampered = live_replay.clone();
    tampered.manifest.randomness.root_seed_hex = "22".repeat(32);
    assert!(matches!(
        run(tampered),
        Err(ControllerError::ReplayExecution(
            ReplayExecutionError::ManifestMismatch
        ))
    ));

    // The live backend remains untouched by every failed replay attempt.
    assert_eq!(controller.checkpoint().unwrap(), after);
    assert_eq!(controller.export_replay().unwrap(), live_replay);
    assert_eq!(
        controller.checkpoint().unwrap().limit_counters,
        live_counters
    );
}

#[test]
fn from_checkpoint_rejects_states_the_kernel_cannot_execute() {
    use mtgml_decision::{DecisionDomainV2, DecisionVisibility};
    use mtgml_state::PendingDecisionRecordV2;

    let codec = CheckpointCodecIdentity {
        codec_id: "in-memory-reference".into(),
        semantic_version: "6".into(),
    };
    let base = mtgml_state::construct_synthetic_engine_state(mtgml_state::SyntheticResetInputs {
        players: [PlayerId(1), PlayerId(2)],
        root_seed: seed(),
        setup: mtgml_state::SyntheticV4Setup::m2_compatibility(),
    })
    .unwrap();

    // A structurally valid generic EngineState with a standalone
    // ChooseNumber pending request and no continuation.
    let mut standalone_number = base.clone();
    standalone_number.execution.pending_decision = Some(PendingDecisionRecordV2 {
        request: mtgml_decision::AuthoritativeDecisionRequestV2 {
            decision_id: mtgml_model::DecisionId(1),
            player_decision_id: PlayerDecisionIdV1(1),
            state_revision: StateRevision(0),
            actor: PlayerId(1),
            visibility: DecisionVisibility::Public,
            decision: DecisionDomainV2::ChooseNumber {
                minimum: 0,
                maximum: 3,
            },
            candidates: Vec::new(),
            continuation_id: None,
        },
    });
    mtgml_state::validate_engine_state(&standalone_number).unwrap();
    let checkpoint = EnvironmentCheckpointV6::new(
        standalone_number.clone(),
        EpisodeStatus::Running,
        EnvironmentLimitCounters::default(),
        codec.clone(),
    synthetic_identity()
    )
    .expect("generic validation passes");
    assert!(matches!(
        SyntheticM1EnvironmentBackend::from_checkpoint(
            checkpoint.clone(),
            config([PlayerId(1), PlayerId(2)])
        ),
        Err(ControllerError::ProgramStateIncompatible)
    ));
    let controller = TrustedEnvironmentController::new(backend());
    assert!(matches!(
        controller.restore(checkpoint),
        Err(ControllerError::ProgramStateIncompatible)
    ));

    // A root ChooseOne whose kernel preconditions are violated (life not at
    // the entry value) is equally unsupported.
    let mut mismatched_entry = base;
    mismatched_entry
        .core
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .life = 39;
    let checkpoint = EnvironmentCheckpointV6::new(
        mismatched_entry,
        EpisodeStatus::Running,
        EnvironmentLimitCounters::default(),
        codec.clone(),
    synthetic_identity()
    )
    .unwrap();
    assert!(matches!(
        SyntheticM1EnvironmentBackend::from_checkpoint(
            checkpoint.clone(),
            config([PlayerId(1), PlayerId(2)])
        ),
        Err(ControllerError::ProgramStateIncompatible)
    ));

    // The genuine program remains restorable.
    let genuine_checkpoint = environment_at_members_stage().checkpoint().unwrap();
    assert!(SyntheticM1EnvironmentBackend::from_checkpoint(
        genuine_checkpoint,
        config([PlayerId(1), PlayerId(2)])
    )
    .is_ok());
}

#[test]
fn unsupported_standalone_decisions_are_internal_kernel_failures() {
    use mtgml_decision::{DecisionDomainV2, DecisionVisibility};
    use mtgml_state::PendingDecisionRecordV2;

    let mut state =
        mtgml_state::construct_synthetic_engine_state(mtgml_state::SyntheticResetInputs {
            players: [PlayerId(1), PlayerId(2)],
            root_seed: seed(),
            setup: mtgml_state::SyntheticV4Setup::m2_compatibility(),
        })
        .unwrap();
    state.execution.pending_decision = Some(PendingDecisionRecordV2 {
        request: mtgml_decision::AuthoritativeDecisionRequestV2 {
            decision_id: mtgml_model::DecisionId(1),
            player_decision_id: PlayerDecisionIdV1(1),
            state_revision: StateRevision(0),
            actor: PlayerId(1),
            visibility: DecisionVisibility::Public,
            decision: DecisionDomainV2::ChooseNumber {
                minimum: 0,
                maximum: 3,
            },
            candidates: Vec::new(),
            continuation_id: None,
        },
    });
    // Soundness boundary: the engine never turns its own unsupported offer
    // into a player rejection; it is an internal failure before execution.
    let mut kernel = mtgml_rules::ProgramKernelV1::for_program(
        mtgml_model::ExecutionProgramV1::SyntheticRulesCompat,
    )
    .expect("the synthetic program is supported by the current kernel boundary");
    assert!(matches!(
        kernel.apply(
            &state,
            PlayerId(1),
            &mtgml_decision::DecisionResponseV2 {
                schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
                player_decision_id: PlayerDecisionIdV1(1),
                state_revision: StateRevision(0),
                answer: number_answer(1),
            }
        ),
        Err(mtgml_rules::KernelExecutionError::UnsupportedStagePath)
    ));
}

#[test]
fn checkpoint_restore_preserves_the_lifecycle_public_surface() {
    let (_before, result) = tracked_incarnation_product().unwrap();
    // Give the state a retired opaque id so authoritative closure covers
    // retirement sets, not only active tracking.
    let mut state = result.next_state.clone();
    state.revision = mtgml_model::StateRevision(state.revision.0 + 1);
    mtgml_state::apply_perspective_lifecycle(
        &mut state,
        &mtgml_state::PerspectiveLifecycleAuditV1 {
            perspective: PlayerId(1),
            sequence: VisibleSequence(3),
            mutation: mtgml_state::PerspectiveLifecycleMutationV1 {
                identity: mtgml_state::IdentityMutationV1::Retire {
                    opaque: OpaqueObjectId(2),
                    object: GameObjectId(6),
                },
                knowledge: Some(mtgml_state::KnowledgeMutationV1::Invalidate {
                    opaque: OpaqueObjectId(2),
                    reason: mtgml_state::KnowledgeInvalidationReason::ExplicitForget,
                    invalidation_provenance: mtgml_state::KnowledgeAcquisitionReason::Observed {
                        channel: mtgml_state::KnowledgeHistoryChannel::Public,
                        sequence: VisibleSequence(3),
                        cause: mtgml_state::KnowledgeAcquisitionCause::PublicEvent,
                    },
                }),
            },
        },
    )
    .unwrap();
    let codec = CheckpointCodecIdentity {
        codec_id: "in-memory-reference".into(),
        semantic_version: "6".into(),
    };
    let counters = EnvironmentLimitCounters::default();
    let checkpoint = EnvironmentCheckpointV6::new(
        state.clone(),
        EpisodeStatus::Running,
        counters.clone(),
        codec.clone(),
    synthetic_identity()
    )
    .unwrap();
    let original = TrustedEnvironmentController::new(
        SyntheticM1EnvironmentBackend::from_checkpoint(
            checkpoint.clone(),
            config([PlayerId(1), PlayerId(2)]),
        )
        .unwrap(),
    );
    let restored = TrustedEnvironmentController::new(
        SyntheticM1EnvironmentBackend::from_checkpoint(
            checkpoint,
            config([PlayerId(1), PlayerId(2)]),
        )
        .unwrap(),
    );
    restored.restore(original.checkpoint().unwrap()).unwrap();
    let p1_bytes_original = mtgml_wire::encode_canonical(
        &original
            .bind_player(PlayerId(1))
            .unwrap()
            .information_state()
            .unwrap(),
    )
    .unwrap();
    let p1_bytes_restored = mtgml_wire::encode_canonical(
        &restored
            .bind_player(PlayerId(1))
            .unwrap()
            .information_state()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(p1_bytes_original, p1_bytes_restored);
    // Authoritative lifecycle closure: knowledge, provenance/history,
    // mappings, retirement sets, allocators and the visible cursor are all
    // bound by checkpoint identity, not only the public projection bytes.
    assert_eq!(
        restored.checkpoint().unwrap(),
        original.checkpoint().unwrap()
    );
    assert_eq!(original.checkpoint().unwrap().state, state);
}

#[test]
fn equal_input_fork_reproduces_lifecycle_public_bytes() {
    let (_before, result) = tracked_incarnation_product().unwrap();
    let codec = CheckpointCodecIdentity {
        codec_id: "in-memory-reference".into(),
        semantic_version: "6".into(),
    };
    let checkpoint = EnvironmentCheckpointV6::new(
        result.next_state.clone(),
        EpisodeStatus::Running,
        EnvironmentLimitCounters::default(),
        codec,
    synthetic_identity()
    )
    .unwrap();
    let original = TrustedEnvironmentController::new(
        SyntheticM1EnvironmentBackend::from_checkpoint(
            checkpoint.clone(),
            config([PlayerId(1), PlayerId(2)]),
        )
        .unwrap(),
    );
    let fork = original.fork().unwrap();
    let p1_original = original.bind_player(PlayerId(1)).unwrap();
    let p1_fork = fork.bind_player(PlayerId(1)).unwrap();
    assert_eq!(
        mtgml_wire::encode_canonical(&p1_original.information_state().unwrap()).unwrap(),
        mtgml_wire::encode_canonical(&p1_fork.information_state().unwrap()).unwrap()
    );
    assert_eq!(original.checkpoint().unwrap(), checkpoint);
    // Equal-input fork preserves the complete authoritative lifecycle state.
    assert_eq!(fork.checkpoint().unwrap(), original.checkpoint().unwrap());
}
