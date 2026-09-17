#[test]
fn fnd_028_declared_zero_state_endpoint_and_manifest_remain_valid() {
    let players = [PlayerId(0), PlayerId(1)];
    let state = mtgml_state::construct_synthetic_engine_state(
        mtgml_state::SyntheticResetInputs {
            players,
            root_seed: seed(),
            setup: mtgml_state::SyntheticV4Setup::m2_compatibility(),
        }
    )
    .unwrap();
    assert!(mtgml_state::validate_engine_state(&state).is_ok());

    let controller = TrustedEnvironmentController::new(
        SyntheticM1EnvironmentBackend::new(players, seed(), config(players)).unwrap(),
    );
    let zero_endpoint = controller.bind_player(PlayerId(0)).unwrap();
    assert_eq!(
        zero_endpoint.information_state().unwrap().perspective,
        PlayerId(0)
    );
    assert!(controller.export_replay().unwrap().manifest.validate().is_ok());

}

#[test]
fn fnd_028_declared_zero_player_is_produced_checkpointed_forked_and_replayed() {
    let zero = PlayerId(0);
    let players = [zero, PlayerId(1)];
    let controller = TrustedEnvironmentController::new(
        SyntheticM1EnvironmentBackend::new(players, seed(), config(players)).unwrap(),
    );
    let before = controller.checkpoint().unwrap();
    let fork = controller.fork().unwrap();
    let zero_endpoint = controller.bind_player(zero).unwrap();
    let fork_zero_endpoint = fork.bind_player(zero).unwrap();

    let live_step = zero_endpoint.submit(response(0, 0)).unwrap();
    let fork_step = fork_zero_endpoint.submit(response(0, 0)).unwrap();
    assert_eq!(
        live_step.submission,
        mtgml_observation::PlayerStepSubmissionV1::Accepted
    );
    live_step.validate().unwrap();
    fork_step.validate().unwrap();
    assert_eq!(live_step, fork_step);

    let after = controller.checkpoint().unwrap();
    assert_eq!(after.state.core.players[&zero].life, 38);
    assert_eq!(after.state.revision, StateRevision(1));
    assert_eq!(fork.checkpoint().unwrap(), after);

    let replay = controller.export_replay().unwrap();
    assert_eq!(replay.steps.len(), 1);
    assert_eq!(replay.steps[0].actor, zero);
    replay.validate().unwrap();
    let bytes = mtgml_wire::encode_canonical(&replay).unwrap();
    assert!(
        String::from_utf8(bytes.clone())
            .unwrap()
            .contains("\"actor\":\"0\"")
    );
    let decoded: AuthoritativeReplayV4 = mtgml_wire::decode_canonical(&bytes).unwrap();
    assert_eq!(decoded, replay);

    let report = controller
        .execute_replay_from_checkpoint(before.clone(), replay.clone())
        .unwrap();
    assert_eq!(report.traces.len(), 1);
    assert!(report.traces[0].transition.accepted);
    assert_eq!(report.traces[0].after, after);
    assert_eq!(report.final_checkpoint, after);
    assert_eq!(controller.checkpoint().unwrap(), after);
    assert_eq!(controller.export_replay().unwrap(), replay);

    let restored = TrustedEnvironmentController::new(
        SyntheticM1EnvironmentBackend::new(players, seed(), config(players)).unwrap(),
    );
    restored
        .execute_trusted_response(zero, response(0, 0))
        .unwrap();
    restored.restore(before).unwrap();
    let restored_step = restored
        .bind_player(zero)
        .unwrap()
        .submit(response(0, 0))
        .unwrap();
    assert_eq!(restored_step, live_step);
    assert_eq!(restored.checkpoint().unwrap(), after);
    assert_eq!(restored.export_replay().unwrap(), replay);
}

fn declared_zero_run() -> (
    TrustedEnvironmentController,
    EnvironmentCheckpointV4,
    EnvironmentCheckpointV4,
    AuthoritativeReplayV4,
) {
    let players = [PlayerId(0), PlayerId(1)];
    let controller = TrustedEnvironmentController::new(
        SyntheticM1EnvironmentBackend::new(players, seed(), config(players)).unwrap(),
    );
    let before = controller.checkpoint().unwrap();
    controller
        .execute_trusted_response(PlayerId(0), response(0, 0))
        .unwrap();
    let live_after = controller.checkpoint().unwrap();
    let live_replay = controller.export_replay().unwrap();
    (controller, before, live_after, live_replay)
}

#[test]
fn fnd_028_undeclared_zero_actor() {
    let controller = TrustedEnvironmentController::new(backend());
    let before = controller.checkpoint().unwrap();
    controller
        .execute_trusted_response(PlayerId(1), response(0, 0))
        .unwrap();
    let live_after = controller.checkpoint().unwrap();
    let live_replay = controller.export_replay().unwrap();

    let mut tampered = live_replay.clone();
    tampered.steps[0].actor = PlayerId(0);
    tampered.validate().unwrap();

    assert!(matches!(
        controller.execute_replay_from_checkpoint(before, tampered),
        Err(ControllerError::ReplayExecution(
            ReplayExecutionError::ActorUnavailable { step_index: 0 }
        ))
    ));
    assert_eq!(controller.checkpoint().unwrap(), live_after);
    assert_eq!(controller.export_replay().unwrap(), live_replay);
}

#[test]
fn fnd_028_declared_zero_actor_must_match_pending_actor() {
    let players = [PlayerId(1), PlayerId(0)];
    let controller = TrustedEnvironmentController::new(
        SyntheticM1EnvironmentBackend::new(players, seed(), config(players)).unwrap(),
    );
    let before = controller.checkpoint().unwrap();
    controller
        .execute_trusted_response(PlayerId(1), response(0, 0))
        .unwrap();
    let live_after = controller.checkpoint().unwrap();
    let live_replay = controller.export_replay().unwrap();

    let mut tampered = live_replay.clone();
    tampered.steps[0].actor = PlayerId(0);
    tampered.validate().unwrap();

    assert!(matches!(
        controller.execute_replay_from_checkpoint(before, tampered),
        Err(ControllerError::ReplayExecution(
            ReplayExecutionError::ActorUnavailable { step_index: 0 }
        ))
    ));
    assert_eq!(controller.checkpoint().unwrap(), live_after);
    assert_eq!(controller.export_replay().unwrap(), live_replay);
}

#[test]
fn fnd_028_zero_actor_wrong_player_decision_id() {
    let (controller, before, live_after, live_replay) = declared_zero_run();
    let mut tampered = live_replay.clone();
    tampered.steps[0].response.player_decision_id = PlayerDecisionIdV1(999);
    tampered.validate().unwrap();

    assert!(matches!(
        controller.execute_replay_from_checkpoint(before, tampered),
        Err(ControllerError::ReplayExecution(
            ReplayExecutionError::PlayerDecisionIdentityMismatch { step_index: 0 }
        ))
    ));
    assert_eq!(controller.checkpoint().unwrap(), live_after);
    assert_eq!(controller.export_replay().unwrap(), live_replay);
}

#[test]
fn fnd_028_zero_actor_wrong_revision() {
    let (controller, _before, live_after, live_replay) = declared_zero_run();
    let mut tampered = live_replay.clone();
    tampered.steps[0].response.state_revision = StateRevision(1);

    assert_eq!(
        tampered.validate(),
        Err(mtgml_replay::ReplayValidationError::RevisionDiscontinuity)
    );
    assert_eq!(controller.checkpoint().unwrap(), live_after);
    assert_eq!(controller.export_replay().unwrap(), live_replay);
}
