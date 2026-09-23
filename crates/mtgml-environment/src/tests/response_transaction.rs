// S3.0 Task 3: characterize the existing Synthetic response boundary and
// keep an explicit compile-contract witness for the absent shared owner.

#[test]
fn synthetic_accepted_response_pins_the_current_transaction_products() {
    use mtgml_observation::PlayerStepSubmissionV1;

    let controller = TrustedEnvironmentController::new(backend());
    let before = controller.checkpoint().unwrap();
    let before_replay = controller.export_replay().unwrap();
    let p1 = controller.bind_player(PlayerId(1)).unwrap();
    let p2 = controller.bind_player(PlayerId(2)).unwrap();
    let before_observation = p1.observation().unwrap();
    let before_information = p1.information_state().unwrap();
    let before_decision = p1.visible_decision().unwrap().unwrap();

    let step = p1.submit(response(0, 0)).unwrap();
    step.validate().unwrap();
    assert_eq!(step.submission, PlayerStepSubmissionV1::Accepted);
    assert_eq!(step.schema_version, PLAYER_STEP_SCHEMA_V2);
    assert_eq!(step.information_state.state_revision, StateRevision(1));
    assert_eq!(step.next_decision, p1.visible_decision().unwrap());
    assert_eq!(step.status, EpisodeStatus::Running);
    assert_eq!(step.observed_events.len(), 0);

    let after = controller.checkpoint().unwrap();
    assert_eq!(after.state.revision, StateRevision(1));
    assert_eq!(after.status, EpisodeStatus::Running);
    assert_eq!(after.limit_counters.decisions_submitted, 1);
    assert_eq!(after.limit_counters.accepted_transitions, 1);
    assert_eq!(after.limit_counters.rule_events_emitted, 5);
    assert_eq!(
        after.limit_counters.resource_units_consumed,
        before.limit_counters.resource_units_consumed
    );
    assert_eq!(
        after.limit_counters.wall_clock_elapsed_millis,
        before.limit_counters.wall_clock_elapsed_millis
    );
    for player in [PlayerId(1), PlayerId(2)] {
        let before_identity = &before.state.perspective_identities.players[&player];
        let after_identity = &after.state.perspective_identities.players[&player];
        assert_eq!(after_identity.opaque_to_object, before_identity.opaque_to_object);
        assert_eq!(after_identity.object_to_opaque, before_identity.object_to_opaque);
        assert_eq!(after_identity.retired_object_ids, before_identity.retired_object_ids);
    }
    assert_eq!(
        after.state.perspective_identities.players[&PlayerId(1)].next_player_decision_id.0,
        before.state.perspective_identities.players[&PlayerId(1)].next_player_decision_id.0 + 1
    );
    assert_eq!(
        after.state.perspective_identities.players[&PlayerId(2)].next_player_decision_id,
        before.state.perspective_identities.players[&PlayerId(2)].next_player_decision_id
    );
    assert_eq!(after.state.knowledge, before.state.knowledge);
    assert_eq!(after.state_digest, after.state.digest().unwrap());
    assert_ne!(after, before);

    let replay = controller.export_replay().unwrap();
    replay.validate().unwrap();
    assert_eq!(replay.steps.len(), 1);
    assert_eq!(
        replay.steps[0],
        mtgml_replay::ReplayStepV6 {
            step_index: 0,
            actor: PlayerId(1),
            checkpoint_digest_before: before.checkpoint_digest.clone(),
            state_revision_before: before.state.revision,
            response: response(0, 0),
            accepted: true,
            state_revision_after: after.state.revision,
            full_state_digest_after: after.state_digest.clone(),
            episode_status_after: after.status.clone(),
            environment_limit_counters_after: after.limit_counters.clone(),
            checkpoint_digest_after: after.checkpoint_digest.clone(),
        }
    );
    assert_eq!(replay.final_identity.checkpoint_digest, after.checkpoint_digest);
    assert_eq!(before_replay.steps.len(), 0);

    // The accepted entry creates its existing continuation decision. The
    // projections are deterministic products of the committed state.
    assert_ne!(p1.observation().unwrap(), before_observation);
    assert_ne!(p1.information_state().unwrap(), before_information);
    assert_ne!(p1.visible_decision().unwrap().unwrap(), before_decision);
    assert_eq!(p2.information_state().unwrap().state_revision, StateRevision(1));
}

#[test]
fn synthetic_authoritative_response_pins_event_delta_and_occurrence_products() {
    use mtgml_rules::AuthoritativeRuleEventKind as Event;

    let controller = TrustedEnvironmentController::new(backend());
    let before = controller.checkpoint().unwrap();
    let transition = controller
        .execute_trusted_response(PlayerId(1), response(0, 0))
        .unwrap();
    assert!(transition.accepted);
    assert_eq!(transition.status, EpisodeStatus::Running);
    assert_eq!(transition.next_decision.as_ref().unwrap().state_revision, StateRevision(1));
    assert_eq!(transition.events.len(), 5);
    assert_eq!(
        transition
            .events
            .iter()
            .map(|event| event.event_id.0)
            .collect::<Vec<_>>(),
        vec![1, 2, 3, 4, 5]
    );
    assert!(transition.events.iter().all(|event| event.state_revision == StateRevision(1)));
    assert!(matches!(transition.events[0].event, Event::LifeChanged { from: 40, to: 39, .. }));
    assert!(matches!(transition.events[1].event, Event::LifeChanged { from: 39, to: 38, .. }));
    assert!(matches!(transition.events[2].event, Event::RandomValueSampled { .. }));
    assert!(matches!(transition.events[3].event, Event::DecisionCleared { .. }));
    assert!(matches!(transition.events[4].event, Event::DecisionCreated { .. }));
    assert_eq!(transition.delta.apply(&before.state).unwrap(), transition.next_state);

    let occurrences = crate::lifecycle_projection::project_occurrence_envelopes(
        &before.state,
        &transition.next_state,
        &transition.events,
    )
    .unwrap();
    assert_eq!(occurrences.len(), 2);
    assert!(occurrences.values().all(Vec::is_empty));

    let after = controller.checkpoint().unwrap();
    assert_eq!(after.state, transition.next_state);
    assert_eq!(after.status, transition.status);
    assert_eq!(after.limit_counters.rule_events_emitted, 5);
}

#[test]
fn synthetic_rejected_response_preserves_complete_checkpoint_and_player_products() {
    use mtgml_observation::{PlayerStepSubmissionV1, PlayerSubmissionCodeV1};

    let controller = TrustedEnvironmentController::new(backend());
    let before = controller.checkpoint().unwrap();
    let replay_before = controller.export_replay().unwrap();
    let p1 = controller.bind_player(PlayerId(1)).unwrap();
    let p2 = controller.bind_player(PlayerId(2)).unwrap();
    let observation_before = p1.observation().unwrap();
    let information_before = p1.information_state().unwrap();
    let decision_before = p1.visible_decision().unwrap();
    let opponent_observation_before = p2.observation().unwrap();
    let opponent_information_before = p2.information_state().unwrap();
    let opponent_decision_before = p2.visible_decision().unwrap();

    let rejected = p1.submit(response(1, 0)).unwrap();
    rejected.validate().unwrap();
    assert_eq!(
        rejected.submission,
        PlayerStepSubmissionV1::Rejected {
            code: PlayerSubmissionCodeV1::InvalidCandidate,
        }
    );

    assert_eq!(controller.checkpoint().unwrap(), before);
    assert_eq!(controller.export_replay().unwrap(), replay_before);
    assert_eq!(p1.observation().unwrap(), observation_before);
    assert_eq!(p1.information_state().unwrap(), information_before);
    assert_eq!(p1.visible_decision().unwrap(), decision_before);
    assert_eq!(p2.observation().unwrap(), opponent_observation_before);
    assert_eq!(p2.information_state().unwrap(), opponent_information_before);
    assert_eq!(p2.visible_decision().unwrap(), opponent_decision_before);
    assert_eq!(rejected.information_state, information_before);
    assert_eq!(rejected.observed_events.len(), 0);
    assert_eq!(rejected.status, before.status);
    assert_eq!(rejected.next_decision, decision_before);
}

#[test]
fn reference_magic_response_remains_unavailable_while_running() {
    use mtgml_observation::{PlayerStepSubmissionV1, PlayerSubmissionCodeV1};

    let controller = reference_controller(reference_state(mtgml_state::TurnPosition::Beginning {
        step: mtgml_state::BeginningStep::Upkeep,
    }));
    let before = controller.checkpoint().unwrap();
    let replay_before = controller.export_replay().unwrap();
    let player = controller.bind_player(PlayerId(1)).unwrap();

    let step = player.submit(response(0, before.state.revision.0)).unwrap();
    assert_eq!(
        step.submission,
        PlayerStepSubmissionV1::Rejected {
            code: PlayerSubmissionCodeV1::UnavailableDecision,
        }
    );
    assert_eq!(controller.checkpoint().unwrap(), before);
    assert_eq!(controller.export_replay().unwrap(), replay_before);
}

#[test]
fn shared_response_transaction_production_entry_point_exists() {
    // S3.0 Task 4 is expected to introduce this environment-owned seam. Keep
    // this compile-contract RED until the shared primitive owns commit order.
    use crate::response_transaction::execute_response_transaction;

    let _shared_entry_point = execute_response_transaction;
}
