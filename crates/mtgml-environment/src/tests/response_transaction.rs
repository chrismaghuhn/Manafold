// S3.0 Task 3: characterize the existing Synthetic response boundary and
// keep an explicit compile-contract witness for the absent shared owner.

use crate::controller::EnvironmentBackend;

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
    use crate::response_transaction::{
        ResponseTransaction, ResponseTransactionFailurePoint, TestApplyOverride,
    };
    use mtgml_observation::ObservedEventEnvelopeV2;
    use std::collections::BTreeMap;

    type Hook = fn(
        &EnvironmentCheckpointV6,
        &mtgml_rules::TransitionResult,
        &BTreeMap<PlayerId, Vec<ObservedEventEnvelopeV2>>,
    ) -> Result<(), ControllerError>;
    type SharedEntry = for<'a> fn(
        ResponseTransaction<'a>,
        PlayerId,
        DecisionResponseV2,
        Hook,
        Option<ResponseTransactionFailurePoint>,
        Option<TestApplyOverride>,
    ) -> Result<mtgml_rules::TransitionResult, ControllerError>;
    let _shared_entry_point: SharedEntry = execute_response_transaction::<Hook>;
}

#[derive(Debug, PartialEq, Eq)]
struct ResponseWorldSnapshot {
    checkpoint: EnvironmentCheckpointV6,
    replay: AuthoritativeReplayV6,
    player_products: Vec<(PlayerId, Vec<Vec<u8>>)>,
}

fn response_world_snapshot(backend: &SyntheticM1EnvironmentBackend) -> ResponseWorldSnapshot {
    let mut player_products = Vec::new();
    for player in backend.players() {
        let observation = backend.player_observation(player).unwrap();
        let information = backend.player_information_state(player).unwrap();
        let decision = backend.player_visible_decision(player).unwrap();
        player_products.push((
            player,
            vec![
                mtgml_wire::encode_canonical(&observation).unwrap(),
                mtgml_wire::encode_canonical(&information).unwrap(),
                match decision {
                    Some(decision) => mtgml_wire::encode_canonical(&decision).unwrap(),
                    None => b"null".to_vec(),
                },
            ],
        ));
    }
    ResponseWorldSnapshot {
        checkpoint: backend.checkpoint().unwrap(),
        replay: backend.export_replay().unwrap(),
        player_products,
    }
}

fn response_for_backend(
    backend: &SyntheticM1EnvironmentBackend,
    actor: PlayerId,
    answer: DecisionAnswerV2,
) -> DecisionResponseV2 {
    let request = backend
        .player_visible_decision(actor)
        .unwrap()
        .expect("test response actor owns a visible Decision");
    DecisionResponseV2 {
        schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
        player_decision_id: request.player_decision_id,
        state_revision: request.state_revision,
        answer,
    }
}

#[test]
fn shared_transaction_injected_candidate_replay_and_projection_failures_are_atomic() {
    use crate::response_transaction::ResponseTransactionFailurePoint as Failure;

    for failure in [
        Failure::CandidateCheckpoint,
        Failure::ReplayAppend,
        Failure::ReplayExport,
        Failure::OccurrenceProjection,
        Failure::PlayerProjectionValidation,
    ] {
        let mut backend = backend();
        let before = response_world_snapshot(&backend);
        let response = response_for_backend(&backend, PlayerId(1), order_entry_answer());
        let result = backend.execute_response_with_failure_point(
            PlayerId(1),
            response,
            Some(failure),
            |_candidate, _transition, _occurrences| Ok(()),
        );
        assert!(result.is_err(), "{failure:?} must be injected");
        assert_eq!(response_world_snapshot(&backend), before, "{failure:?}");
    }
}

#[test]
fn shared_transaction_before_commit_hook_failure_is_atomic() {
    let mut backend = backend();
    let before = response_world_snapshot(&backend);
    let response = response_for_backend(&backend, PlayerId(1), order_entry_answer());
    let result = backend.execute_response_with_failure_point(
        PlayerId(1),
        response,
        None,
        |candidate, transition, _occurrences| {
            assert_eq!(candidate.state.revision, StateRevision(1));
            assert!(transition.accepted);
            Err(ControllerError::Backend("injected before-commit failure".into()))
        },
    );
    assert!(result.is_err());
    assert_eq!(response_world_snapshot(&backend), before);
}

#[test]
fn shared_transaction_forced_progress_boundary_failure_is_atomic() {
    use crate::response_transaction::ResponseTransactionFailurePoint::ForcedProgress;

    let mut backend = backend();
    let actor = PlayerId(1);
    // Move to the existing final Order response. It completes the synthetic
    // chain without a Decision, which is precisely the one-advance boundary.
    let initial = response_for_backend(&backend, actor, order_entry_answer());
    assert!(backend.execute_trusted_response(actor, initial).unwrap().accepted);
    let count = response_for_backend(&backend, actor, number_answer(2));
    assert!(backend.execute_trusted_response(actor, count).unwrap().accepted);
    let members = response_for_backend(&backend, actor, members_answer(&[0, 1]));
    assert!(backend.execute_trusted_response(actor, members).unwrap().accepted);

    let before = response_world_snapshot(&backend);
    let final_order = response_for_backend(&backend, actor, order_answer(&[1, 0]));
    let result = backend.execute_response_with_failure_point(
        actor,
        final_order,
        Some(ForcedProgress),
        |_candidate, _transition, _occurrences| Ok(()),
    );
    assert!(result.is_err(), "forced-progress boundary must be reached");
    assert_eq!(response_world_snapshot(&backend), before);
}

#[test]
fn shared_transaction_final_synthetic_response_adds_only_its_real_replay_step() {
    let mut backend = backend();
    let actor = PlayerId(1);
    for answer in [order_entry_answer(), number_answer(2), members_answer(&[0, 1])] {
        let response = response_for_backend(&backend, actor, answer);
        assert!(backend.execute_trusted_response(actor, response).unwrap().accepted);
    }
    let replay_before = backend.export_replay().unwrap();
    assert_eq!(replay_before.steps.len(), 3);

    let response = response_for_backend(&backend, actor, order_answer(&[1, 0]));
    let transition = backend.execute_trusted_response(actor, response.clone()).unwrap();
    assert!(transition.accepted);
    assert!(transition.next_decision.is_none());
    assert_eq!(transition.status, EpisodeStatus::Running);

    let replay_after = backend.export_replay().unwrap();
    assert_eq!(replay_after.steps.len(), 4);
    assert_eq!(replay_after.steps[3].response, response);
    assert!(replay_after.steps[3].accepted);
    assert_eq!(replay_after.steps[3].state_revision_after, StateRevision(4));
}

#[test]
fn shared_transaction_kernel_rejection_is_atomic() {
    let mut backend = backend();
    let before = response_world_snapshot(&backend);
    let rejected = backend
        .execute_trusted_response(PlayerId(1), response(1, 0))
        .unwrap();
    assert!(!rejected.accepted);
    assert_eq!(response_world_snapshot(&backend), before);
}

#[test]
fn shared_transaction_counter_overflow_is_atomic() {
    let players = [PlayerId(1), PlayerId(2)];
    let original = backend().checkpoint().unwrap();
    let checkpoint = EnvironmentCheckpointV6::new(
        original.state,
        original.status,
        EnvironmentLimitCounters {
            decisions_submitted: u64::MAX,
            ..original.limit_counters
        },
        original.codec,
        synthetic_identity(),
    )
    .unwrap();
    let mut backend = SyntheticM1EnvironmentBackend::from_checkpoint(checkpoint, config(players))
        .unwrap();
    let before = response_world_snapshot(&backend);
    let response = response_for_backend(&backend, PlayerId(1), order_entry_answer());
    let error = backend
        .execute_trusted_response(PlayerId(1), response)
        .expect_err("counter overflow must stop before commit");
    assert!(matches!(error, ControllerError::CounterOverflow { .. }));
    assert_eq!(response_world_snapshot(&backend), before);
}
