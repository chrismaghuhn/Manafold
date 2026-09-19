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

    // No replay step is fabricated for responseless progress, but the
    // recorder baseline is rebased onto the post-progress checkpoint so the
    // next real response appends against a continuous identity.
    let replay_after = controller.export_replay().unwrap();
    assert!(replay_after.steps.is_empty());
    assert!(replay_before.steps.is_empty());
    assert_eq!(replay_after.final_identity.state_revision, StateRevision(1));
    assert_eq!(
        replay_after.final_identity.full_state_digest,
        after.state_digest
    );
    assert_eq!(
        replay_after.final_identity.checkpoint_digest,
        after.checkpoint_digest
    );
    assert_eq!(
        replay_after.final_identity.environment_limit_counters,
        after.limit_counters
    );
    assert_ne!(
        replay_after.final_identity, replay_before.final_identity,
        "baseline must advance past the pre-progress identity"
    );
}

#[test]
fn forced_progress_then_response_keeps_replay_continuous() {
    // Blocker-1 regression: after standalone forced progress, a valid
    // response on the resulting real Decision must be accepted, must append
    // against the rebased baseline, and the exported replay must validate.
    let controller = TrustedEnvironmentController::new(backend_without_pending());
    controller.execute_forced_progress().unwrap();
    let baseline = controller.checkpoint().unwrap();
    assert_eq!(baseline.state.revision, StateRevision(1));

    let p1 = controller.bind_player(PlayerId(1)).unwrap();
    let step = submit_answer(&p1, order_entry_answer());
    assert!(matches!(
        step.submission,
        mtgml_observation::PlayerStepSubmissionV1::Accepted
    ));
    assert_eq!(step.information_state.state_revision, StateRevision(2));

    let replay = controller.export_replay().unwrap();
    assert_eq!(replay.steps.len(), 1);
    let recorded = &replay.steps[0];
    assert_eq!(recorded.step_index, 0);
    assert_eq!(recorded.state_revision_before, StateRevision(1));
    assert_eq!(
        recorded.checkpoint_digest_before,
        baseline.checkpoint_digest
    );
    assert_eq!(recorded.state_revision_after, StateRevision(2));
    assert_eq!(replay.final_identity.state_revision, StateRevision(2));

    let after = controller.checkpoint().unwrap();
    assert_eq!(after.limit_counters.decisions_submitted, 1);
    assert_eq!(after.limit_counters.accepted_transitions, 1);
}

#[test]
fn forced_progress_checkpoint_restores_exactly() {
    let controller = TrustedEnvironmentController::new(backend_without_pending());
    controller.execute_forced_progress().unwrap();
    let progressed = controller.checkpoint().unwrap();
    let replay_at_progress = controller.export_replay().unwrap();

    // Mutate past the forced-progress product, then restore it.
    let p1 = controller.bind_player(PlayerId(1)).unwrap();
    let _ = submit_answer(&p1, order_entry_answer());
    assert_eq!(
        controller.checkpoint().unwrap().state.revision,
        StateRevision(2)
    );

    controller.restore(progressed.clone()).unwrap();
    let restored = controller.checkpoint().unwrap();
    assert_eq!(restored, progressed);
    assert_eq!(
        controller.export_replay().unwrap(),
        replay_at_progress,
        "restore rebuilds the rebased baseline exactly"
    );
    // Projections serve from the restored checkpoint.
    let visible = p1.visible_decision().unwrap().expect("entry decision");
    assert_eq!(visible.state_revision, StateRevision(1));
    assert_eq!(visible.player_decision_id, PlayerDecisionIdV1(2));
}

#[test]
fn forced_progress_candidate_projects_successfully_pre_commit() {
    // Alignment pin: the exact projection calls the forced-progress commit
    // makes pre-commit must accept the candidate product. A committable-yet-
    // unprojectable mutant is not constructible in the current substrate —
    // verified by inspection, not assumed:
    // - opaque id 0 is structurally forbidden (m2_shape perspective_identity
    //   rejects zero ids before any projection runs);
    // - non-increasing observed history is structurally forbidden (m2_shape
    //   knowledge enforces strict sequence order);
    // - channel/cause acceptance and sequence bounds are identically strict
    //   in structural validation (state knowledge.rs) and projection
    //   validation (observation knowledge.rs);
    // - the kernel derives only projectable pending requests, and the
    //   observation envelope is derived-coherent by construction.
    // Structural validation, runtime validation, contract cursor shadows,
    // and projection validation are deliberately aligned to the same
    // invariants, so the pre-commit block is defensive hardening for future
    // substrate evolution (e.g. turn-structure knowledge shapes). This test
    // pins the alignment: if a future change breaks projectability, it
    // fails here with the exact perspective instead of a mysterious commit
    // refusal.
    use mtgml_state::EngineState;
    let players = [PlayerId(1), PlayerId(2)];
    let mut setup =
        mtgml_state::construct_synthetic_engine_state(mtgml_state::SyntheticResetInputs {
            players,
            root_seed: seed(),
            setup: mtgml_state::SyntheticV4Setup::m2_compatibility(),
        })
        .unwrap();
    setup.execution.pending_decision = None;
    let mut kernel = mtgml_rules::ProgramKernelV1::for_program(
        mtgml_model::ExecutionProgramV1::SyntheticRulesCompat,
    )
    .expect("the synthetic program is supported by the current kernel boundary");
    let product = kernel.advance_forced_progress(&setup).unwrap();
    let candidate: &EngineState = &product.next_state;
    for perspective in players {
        SyntheticM1EnvironmentBackend::synthetic_observation(candidate, perspective)
            .expect("observation must project");
        SyntheticM1EnvironmentBackend::player_information_state_from_state(candidate, perspective)
            .expect("information state must project");
        SyntheticM1EnvironmentBackend::visible_decision_from_state(candidate, perspective)
            .expect("visible decision must project");
    }
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
