#[test]
fn fnd_028_current_player_zero_surfaces_are_characterized_without_a_policy() {
    let players = [PlayerId(0), PlayerId(1)];
    let state = mtgml_state::construct_synthetic_engine_state(
        mtgml_state::SyntheticResetInputs {
            players,
            root_seed: seed(),
        },
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

    let normal = TrustedEnvironmentController::new(backend());
    normal
        .execute_trusted_response(PlayerId(1), response(0, 0))
        .unwrap();
    let mut replay = normal.export_replay().unwrap();
    replay.steps[0].actor = PlayerId(0);
    assert!(matches!(
        replay.validate(),
        Err(mtgml_replay::ReplayValidationError::RevisionDiscontinuity)
    ));
}
