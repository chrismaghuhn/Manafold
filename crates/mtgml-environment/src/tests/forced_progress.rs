// Ownership fragment: forced-progress environment commit evidence.
// Included lexically by tests.rs so every identity remains tests::<name>.
//
// RED: SyntheticM1EnvironmentBackend::execute_forced_progress and the
// controller passthrough do not exist yet.

use mtgml_model::DecisionId;

fn backend_without_pending() -> SyntheticM1EnvironmentBackend {
    let players = [PlayerId(1), PlayerId(2)];
    let mut state =
        mtgml_state::construct_synthetic_engine_state(mtgml_state::SyntheticResetInputs {
            players,
            root_seed: seed(),
            setup: mtgml_state::SyntheticV4Setup::m2_compatibility(),
        })
        .unwrap();
    state.execution.pending_decision = None;
    let checkpoint = EnvironmentCheckpointV4::new(
        state,
        EpisodeStatus::Running,
        EnvironmentLimitCounters::default(),
        CheckpointCodecIdentity {
            codec_id: "in-memory-reference".into(),
            semantic_version: "4".into(),
        },
    )
    .unwrap();
    SyntheticM1EnvironmentBackend::from_checkpoint(checkpoint, config(players)).unwrap()
}

#[test]
fn forced_progress_commits_without_response_counters_or_replay_step() {
    let controller = TrustedEnvironmentController::new(backend_without_pending());
    let before = controller.checkpoint().unwrap();
    assert!(before.state.execution.pending_decision.is_none());
    let replay_before = controller.export_replay().unwrap();
    assert!(replay_before.steps.is_empty());

    let product = controller.execute_forced_progress().unwrap();

    assert!(product.accepted);
    assert_eq!(product.next_state.revision, StateRevision(1));
    assert_eq!(
        product
            .next_state
            .execution
            .pending_decision
            .as_ref()
            .map(|record| record.request.decision_id),
        Some(DecisionId(2))
    );
    assert_eq!(product.events.len(), 1);
    assert!(matches!(product.status, EpisodeStatus::Running));

    // Forced progress is not a submitted player decision: all three
    // limit-counter deltas that track submissions must be exactly zero
    // except the single emitted authoritative event.
    let after = controller.checkpoint().unwrap();
    assert_eq!(
        after.limit_counters.decisions_submitted,
        before.limit_counters.decisions_submitted
    );
    assert_eq!(
        after.limit_counters.accepted_transitions,
        before.limit_counters.accepted_transitions
    );
    assert_eq!(
        after
            .limit_counters
            .rule_events_emitted
            .checked_sub(before.limit_counters.rule_events_emitted),
        Some(product.events.len() as u64)
    );
    assert_eq!(after.state, product.next_state);

    // No replay step is fabricated for responseless progress: the exported
    // replay is byte-identical before and after.
    let replay_after = controller.export_replay().unwrap();
    assert_eq!(replay_after, replay_before);
}

#[test]
fn forced_progress_fork_parity_matches_main_controller() {
    let controller = TrustedEnvironmentController::new(backend_without_pending());
    let fork = controller.fork().unwrap();
    let main_product = controller.execute_forced_progress().unwrap();
    let fork_product = fork.execute_forced_progress().unwrap();
    assert_eq!(main_product.next_state, fork_product.next_state);
    assert_eq!(main_product.events, fork_product.events);
    assert_eq!(main_product.delta, fork_product.delta);
    assert_eq!(main_product.next_decision, fork_product.next_decision);
    assert_eq!(
        controller.checkpoint().unwrap(),
        fork.checkpoint().unwrap()
    );
    assert_eq!(
        controller.export_replay().unwrap(),
        fork.export_replay().unwrap()
    );
}

#[test]
fn forced_progress_failure_leaves_checkpoint_counters_and_replay_unchanged() {
    let mut setup =
        mtgml_state::construct_synthetic_engine_state(mtgml_state::SyntheticResetInputs {
            players: [PlayerId(1), PlayerId(2)],
            root_seed: seed(),
            setup: mtgml_state::SyntheticV4Setup::m2_compatibility(),
        })
        .unwrap();
    setup.execution.pending_decision = None;
    // Break the entry fixture total: life 39 is structurally valid (so the
    // backend still spawns) but the entry program requires exactly 40, so
    // stabilization must fail closed after spawn.
    setup
        .core
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .life = 39;
    let players = [PlayerId(1), PlayerId(2)];
    let checkpoint = EnvironmentCheckpointV4::new(
        setup,
        EpisodeStatus::Running,
        EnvironmentLimitCounters::default(),
        CheckpointCodecIdentity {
            codec_id: "in-memory-reference".into(),
            semantic_version: "4".into(),
        },
    )
    .unwrap();
    let controller = TrustedEnvironmentController::new(
        SyntheticM1EnvironmentBackend::from_checkpoint(checkpoint, config(players)).unwrap(),
    );
    let before = controller.checkpoint().unwrap();
    let replay_before = controller.export_replay().unwrap();

    assert!(controller.execute_forced_progress().is_err());

    let after = controller.checkpoint().unwrap();
    assert_eq!(after, before, "failed progress must not commit");
    assert_eq!(
        controller.export_replay().unwrap(),
        replay_before,
        "failed progress must not touch replay"
    );
}

#[test]
fn forced_progress_rejects_state_with_pending_decision() {
    let controller = TrustedEnvironmentController::new(backend());
    assert!(controller
        .checkpoint()
        .unwrap()
        .state
        .execution
        .pending_decision
        .is_some());
    let before = controller.checkpoint().unwrap();
    assert!(controller.execute_forced_progress().is_err());
    assert_eq!(controller.checkpoint().unwrap(), before);
}
