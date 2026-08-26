// Ownership fragment: endpoint binding and surface evidence. Included lexically by tests.rs so
// every identity remains tests::<name>.

#[test]
fn synthetic_endpoint_returns_v2_surface() {
    let controller = TrustedEnvironmentController::new(backend());
    let p1 = controller.bind_player(PlayerId(1)).unwrap();
    let p2 = controller.bind_player(PlayerId(2)).unwrap();

    let visible = p1.visible_decision().unwrap().unwrap();
    visible.validate().unwrap();
    assert_eq!(visible.schema_version, "player-decision-request.v2");
    assert!(p2.visible_decision().unwrap().is_none());

    let information = p1.information_state().unwrap();
    information.validate().unwrap();
    let (_, digest) =
        mtgml_wire::compute_information_state_digest_v2(&information.digest_input()).unwrap();
    assert_eq!(information.digest, digest);
}

#[test]
fn multi_player_endpoints_remain_bound_through_visibility_and_submission() {
    let controller = TrustedEnvironmentController::new(backend());
    let p1 = controller.bind_player(PlayerId(1)).unwrap();
    let p2 = controller.bind_player(PlayerId(2)).unwrap();
    let before = controller.checkpoint().unwrap();

    // Player 2 does not own the visible decision: non-disclosing
    // unavailable_decision without any oracle difference.
    let foreign = p2.submit(response(0, 0)).unwrap();
    assert_eq!(
        foreign.submission,
        mtgml_observation::PlayerStepSubmissionV1::Rejected {
            code: mtgml_observation::PlayerSubmissionCodeV1::UnavailableDecision,
        }
    );
    assert!(p2.visible_decision().unwrap().is_none());
    assert_eq!(controller.checkpoint().unwrap(), before);

    // Both endpoints remain alive and bound to their own perspectives.
    assert!(p1.visible_decision().unwrap().is_some());
    assert_eq!(p1.information_state().unwrap().perspective, PlayerId(1));
    assert_eq!(p2.information_state().unwrap().perspective, PlayerId(2));

    // After player 1 commits, both endpoints project the advanced state;
    // the created continuation keeps a visible decision alive for p1.
    let step = p1.submit(response(0, 0)).unwrap();
    assert_eq!(step.information_state.perspective, PlayerId(1));
    assert_eq!(step.information_state.state_revision, StateRevision(1));
    assert!(p1.visible_decision().unwrap().is_some());
    assert_eq!(
        p2.information_state().unwrap().state_revision,
        StateRevision(1)
    );
}

#[test]
fn non_default_player_ids_remain_bound_through_submission() {
    let players = [PlayerId(7), PlayerId(9)];
    let controller = TrustedEnvironmentController::new(
        SyntheticM1EnvironmentBackend::new(players, seed(), config(players)).unwrap(),
    );
    let p7 = controller.bind_player(PlayerId(7)).unwrap();
    let step = p7.submit(response(0, 0)).unwrap();
    assert_eq!(step.information_state.perspective, PlayerId(7));
    assert_eq!(
        controller.checkpoint().unwrap().state.revision,
        StateRevision(1)
    );
}

#[test]
fn unknown_player_binding_is_rejected_without_backend_details() {
    let controller = TrustedEnvironmentController::new(backend());
    assert!(matches!(
        controller.bind_player(PlayerId(9)),
        Err(ControllerError::UnknownPlayer)
    ));
}
