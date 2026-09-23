#[test]
fn fnd_017b_closed_status_with_pending_decision_is_rejected_at_checkpoint_owner() {
    let controller = environment_at_members_stage();
    let checkpoint = controller.checkpoint().unwrap();
    assert!(checkpoint.state.execution.pending_decision.is_some());
    let status = EpisodeStatus::Terminal {
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
    };
    assert!(matches!(
        EnvironmentCheckpointV6::new(
            checkpoint.state.clone(),
            status,
            checkpoint.limit_counters.clone(),
            checkpoint.codec.clone(), synthetic_identity(),
        ),
        Err(CheckpointValidationError::CompletedWithDecision)
    ));
}

#[test]
fn fnd_018_environment_checkpoint_is_not_a_raw_serde_surface() {
    let source = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/checkpoint.rs"));
    assert!(!source.contains("Serialize"));
    assert!(!source.contains("Deserialize"));
}
