// Ownership fragment: Batch-E checkpoint/replay boundary RED cases. Included
// lexically by tests.rs so existing environment fixtures remain shared.

#[test]
fn fnd_020_current_producer_rejects_a_false_observation_schema_identity() {
    let mut invalid = config([PlayerId(1), PlayerId(2)]);
    invalid.replay.schemas.observation = "observation-envelope.v999".into();
    assert!(matches!(
        SyntheticM1EnvironmentBackend::new(
            [PlayerId(1), PlayerId(2)],
            seed(),
            invalid,
        ),
        Err(ControllerError::ReplayIdentityMismatch)
    ));
}

#[test]
fn fnd_022b_checkpoint_requires_the_exact_authoritative_player_universe() {
    let (state, _) = two_perspective_outcome_product();
    let codec = CheckpointCodecIdentity {
        codec_id: "synthetic-m2-memory".into(),
        semantic_version: "3".into(),
    };
    for players in [
        Vec::new(),
        vec![PlayerOutcome {
            player: PlayerId(999),
            result: PlayerResult::Win,
        }],
        vec![
            PlayerOutcome {
                player: PlayerId(1),
                result: PlayerResult::Win,
            },
            PlayerOutcome {
                player: PlayerId(1),
                result: PlayerResult::Loss,
            },
        ],
    ] {
        let status = EpisodeStatus::Terminal {
            reason: TerminalReason::Concession,
            players,
        };
        assert!(
            EnvironmentCheckpointV3::new(
                state.clone(),
                status,
                EnvironmentLimitCounters::default(),
                codec.clone(),
            )
            .is_err()
        );
    }
    assert!(EnvironmentCheckpointV3::new(
        state,
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
        EnvironmentLimitCounters::default(),
        codec,
    )
    .is_ok());
}

#[test]
fn fnd_025_checkpoint_rejects_noncanonical_status_order() {
    let (state, _) = two_perspective_outcome_product();
    let status = EpisodeStatus::Terminal {
        reason: TerminalReason::Concession,
        players: vec![
            PlayerOutcome {
                player: PlayerId(2),
                result: PlayerResult::Loss,
            },
            PlayerOutcome {
                player: PlayerId(1),
                result: PlayerResult::Win,
            },
        ],
    };
    assert!(
        EnvironmentCheckpointV3::new(
            state,
            status,
            EnvironmentLimitCounters::default(),
            CheckpointCodecIdentity {
                codec_id: "synthetic-m2-memory".into(),
                semantic_version: "3".into(),
            },
        )
        .is_err()
    );
}

#[test]
fn fnd_025_checkpoint_digest_helper_retains_defensive_outcome_sort() {
    let state = backend().checkpoint().unwrap().state;
    let digest = state.digest().unwrap();
    let counters = EnvironmentLimitCounters::default();
    let codec = CheckpointCodecIdentity {
        codec_id: "synthetic-m2-memory".into(),
        semantic_version: "3".into(),
    };
    let sorted = EpisodeStatus::Terminal {
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
    };
    let permuted = EpisodeStatus::Terminal {
        reason: TerminalReason::Concession,
        players: vec![
            PlayerOutcome {
                player: PlayerId(2),
                result: PlayerResult::Loss,
            },
            PlayerOutcome {
                player: PlayerId(1),
                result: PlayerResult::Win,
            },
        ],
    };
    let sorted_digest = mtgml_persistence::checkpoint_digest::calculate_checkpoint_digest_v3(
        &digest.as_digest_reference(),
        &sorted,
        &counters,
        &codec,
    )
    .unwrap();
    let permuted_digest =
        mtgml_persistence::checkpoint_digest::calculate_checkpoint_digest_v3(
            &digest.as_digest_reference(),
            &permuted,
            &counters,
            &codec,
        )
        .unwrap();
    assert_eq!(sorted_digest, permuted_digest);
}

fn reseal_replay_step(replay: &mut AuthoritativeReplayV3) {
    let step = &mut replay.steps[0];
    let identity = mtgml_replay::InitialEnvironmentIdentityV3 {
        state_revision: step.state_revision_after,
        full_state_digest: step.full_state_digest_after.clone(),
        episode_status: step.episode_status_after.clone(),
        environment_limit_counters: step.environment_limit_counters_after.clone(),
        checkpoint_codec_identity: replay
            .manifest
            .initial_identity
            .checkpoint_codec_identity
            .clone(),
        checkpoint_digest: CheckpointDigestV3::from_digest_bytes([0; 32]),
    };
    step.checkpoint_digest_after =
        mtgml_persistence::checkpoint_digest::calculate_checkpoint_digest_v3(
            &identity.full_state_digest.as_digest_reference(),
            &identity.episode_status,
            &identity.environment_limit_counters,
            &identity.checkpoint_codec_identity,
        )
        .unwrap();
    replay.final_identity = mtgml_replay::InitialEnvironmentIdentityV3 {
        checkpoint_digest: step.checkpoint_digest_after.clone(),
        ..identity
    };
}

#[test]
fn fnd_022e_replay_applies_recorded_external_counter_progression() {
    let controller = TrustedEnvironmentController::new(backend());
    let initial = controller.checkpoint().unwrap();
    controller
        .execute_trusted_response(PlayerId(1), response(0, 0))
        .unwrap();
    let live_after = controller.checkpoint().unwrap();
    let mut replay = controller.export_replay().unwrap();
    replay.steps[0]
        .environment_limit_counters_after
        .resource_units_consumed += 5;
    replay.steps[0]
        .environment_limit_counters_after
        .wall_clock_elapsed_millis += 1000;
    reseal_replay_step(&mut replay);
    replay.validate().unwrap();

    let report = controller
        .execute_replay_from_checkpoint(initial, replay.clone())
        .unwrap();
    assert_eq!(
        report.final_checkpoint.limit_counters,
        replay.steps[0].environment_limit_counters_after
    );
    assert_eq!(
        controller.checkpoint().unwrap(),
        live_after,
        "replay runs on an internal backend"
    );
}

#[test]
fn fnd_023_structural_replay_validation_does_not_verify_backend_state() {
    let controller = TrustedEnvironmentController::new(backend());
    let initial = controller.checkpoint().unwrap();
    controller
        .execute_trusted_response(PlayerId(1), response(0, 0))
        .unwrap();
    let live_after = controller.checkpoint().unwrap();
    let mut tampered = controller.export_replay().unwrap();
    tampered.steps[0].full_state_digest_after =
        mtgml_model::FullStateDigestV3::from_digest_bytes([0x7f; 32]);
    reseal_replay_step(&mut tampered);

    tampered.validate().unwrap();
    let result = controller.execute_replay_from_checkpoint(initial, tampered);
    assert!(matches!(
        result,
        Err(ControllerError::ReplayExecution(
            ReplayExecutionError::AfterDigestMismatch { step_index: 0 }
        ))
    ));
    assert_eq!(controller.checkpoint().unwrap(), live_after);
}

#[test]
fn fnd_024_trusted_rejection_is_not_recorded_in_live_replay() {
    let controller = TrustedEnvironmentController::new(backend());
    let before_checkpoint = controller.checkpoint().unwrap();
    let before_replay = controller.export_replay().unwrap();
    let transition = controller
        .execute_trusted_response(PlayerId(1), response(1, 0))
        .unwrap();

    assert!(!transition.accepted);
    assert_eq!(controller.checkpoint().unwrap(), before_checkpoint);
    assert_eq!(controller.export_replay().unwrap(), before_replay);
}

#[test]
fn fnd_024_external_counter_cannot_invent_a_truncated_status() {
    let controller = TrustedEnvironmentController::new(backend());
    let initial = controller.checkpoint().unwrap();
    controller
        .execute_trusted_response(PlayerId(1), response(0, 0))
        .unwrap();
    let mut replay = controller.export_replay().unwrap();
    replay.steps[0]
        .environment_limit_counters_after
        .wall_clock_elapsed_millis += 1000;
    replay.steps[0].episode_status_after = EpisodeStatus::Truncated {
        reason: TruncationReason::WallClockLimit,
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
    };
    reseal_replay_step(&mut replay);
    replay.validate().unwrap();

    assert!(matches!(
        controller.execute_replay_from_checkpoint(initial, replay),
        Err(ControllerError::ReplayExecution(
            ReplayExecutionError::TransitionMismatch { step_index: 0 }
        ))
    ));
}
