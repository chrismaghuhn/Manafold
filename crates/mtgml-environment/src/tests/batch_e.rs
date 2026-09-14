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
