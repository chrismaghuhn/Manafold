//! M2.G G.6 Node A: historical reprojection evidence over the production
//! projection paths.
//!
//! Every rebuilt player product goes through the SAME functions the live
//! submit pipeline uses (observation, information state, step assembly,
//! occurrence projection); no second projector exists here. Trusted controls
//! pin the host-independent counters EXACTLY unchanged across live commits
//! and replayed traces alike, with explicit replay application of recorded
//! external counters. The M2.G runner records the gate verdicts separately.

use super::{
    synthetic_identity, SyntheticM1EnvironmentBackend, SyntheticM1EnvironmentConfig,
    SyntheticM1ReplayConfig,
};
use crate::checkpoint::{CheckpointCodecIdentity, EnvironmentCheckpointV5};
use crate::controller::TrustedEnvironmentController;
use crate::endpoint::{PlayerEndpoint, PlayerEndpointHandle};
use crate::errors::ControllerError;
use mtgml_decision::{DecisionAnswerV2, DecisionResponseV2, DECISION_RESPONSE_V2_SCHEMA};
use mtgml_model::{CandidateIdV1, PlayerDecisionIdV1, PlayerId};
use mtgml_observation::{
    PlayerStepSubmissionV1, PlayerStepV2, INFORMATION_STATE_SCHEMA_V2, OBSERVATION_SCHEMA,
    OBSERVED_EVENT_SCHEMA_V2, PLAYER_STEP_SCHEMA_V2,
};
use mtgml_random::RootSeed256;
use mtgml_replay::{
    AuthoritativeReplayV5, DeckIdentityV1, InitialEnvironmentIdentityV5, KernelIdentityV1,
    ReplaySchemaVersionsV5, ReplayStepV5, ReplayValidationError, REPLAY_FILE_SCHEMA_V5,
};

fn config(players: [PlayerId; 2]) -> SyntheticM1EnvironmentConfig {
    SyntheticM1EnvironmentConfig {
        codec: CheckpointCodecIdentity {
            codec_id: "in-memory-reference".into(),
            semantic_version: "5".into(),
        },
        setup: mtgml_state::SyntheticV4Setup::m2_compatibility(),
        replay: SyntheticM1ReplayConfig {
            engine_build: "synthetic-build".into(),
            kernel: KernelIdentityV1 {
                implementation_id: "synthetic-m2".into(),
                semantic_version: "0.2.2".into(),
                build_profile: "test".into(),
            },
            rules_snapshot: "synthetic-rules".into(),
            format_policy_snapshot: "synthetic-format".into(),
            oracle_snapshot: "synthetic-oracle".into(),
            card_bundle: "synthetic-bundle".into(),
            randomness_contract_id: "mtgml.rng.v1".into(),
            schemas: ReplaySchemaVersionsV5 {
                observation: OBSERVATION_SCHEMA.into(),
                observation_payload_codec: "synthetic-m3-observation.v1".into(),
                information_state: INFORMATION_STATE_SCHEMA_V2.into(),
                decision: "player-decision-request.v2".into(),
                decision_response: DECISION_RESPONSE_V2_SCHEMA.into(),
                observed_event: OBSERVED_EVENT_SCHEMA_V2.into(),
                player_step: PLAYER_STEP_SCHEMA_V2.into(),
                replay_step: "replay-step.v5".into(),
            },
            decks: players
                .into_iter()
                .enumerate()
                .map(|(index, player)| DeckIdentityV1 {
                    player,
                    deck_id: format!("synthetic-deck-{}", index + 1),
                    digest: mtgml_model::ContentDigest::from_canonical_bytes(
                        format!("synthetic-deck-{}", index + 1).as_bytes(),
                    ),
                })
                .collect(),
        },
    }
}

fn seed() -> RootSeed256 {
    RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap()
}

fn backend() -> SyntheticM1EnvironmentBackend {
    let players = [PlayerId(1), PlayerId(2)];
    SyntheticM1EnvironmentBackend::new(players, seed(), config(players)).unwrap()
}

fn eventful_backend() -> SyntheticM1EnvironmentBackend {
    let players = [PlayerId(1), PlayerId(2)];
    super::eventful::backend(players, seed(), config(players)).unwrap()
}

fn submit_answer(
    endpoint: &PlayerEndpointHandle,
    answer: mtgml_decision::DecisionAnswerV2,
) -> PlayerStepV2 {
    let request = endpoint
        .visible_decision()
        .unwrap()
        .expect("a stage decision is visible");
    let step = endpoint
        .submit(mtgml_decision::DecisionResponseV2 {
            schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
            player_decision_id: request.player_decision_id,
            state_revision: request.state_revision,
            answer,
        })
        .unwrap();
    step.validate().unwrap();
    step
}

fn order_entry_answer() -> mtgml_decision::DecisionAnswerV2 {
    mtgml_decision::DecisionAnswerV2::SelectOne {
        candidate_id: CandidateIdV1(0),
    }
}

fn number_answer(value: i64) -> mtgml_decision::DecisionAnswerV2 {
    mtgml_decision::DecisionAnswerV2::ChooseNumber { value }
}

fn members_answer(ids: &[u32]) -> mtgml_decision::DecisionAnswerV2 {
    mtgml_decision::DecisionAnswerV2::SelectMany {
        candidate_ids: ids.iter().copied().map(CandidateIdV1).collect(),
    }
}

struct PerspectiveBytes {
    information_bytes: Vec<u8>,
    observation_bytes: Vec<u8>,
    visible_decision_bytes: Option<Vec<u8>>,
}

fn visible_decision_bytes(endpoint: &PlayerEndpointHandle) -> Option<Vec<u8>> {
    endpoint
        .visible_decision()
        .unwrap()
        .map(|request| mtgml_wire::encode_canonical(&request).unwrap())
}

#[test]
fn eventful_replay_reprojects_both_perspectives_byte_exactly() {
    let controller = TrustedEnvironmentController::new(eventful_backend());
    let p1 = controller.bind_player(PlayerId(1)).unwrap();
    let _p2 = controller.bind_player(PlayerId(2)).unwrap();
    let cp0 = controller.checkpoint().unwrap();
    let live_step = submit_answer(&p1, order_entry_answer());
    assert_eq!(live_step.submission, PlayerStepSubmissionV1::Accepted);
    assert!(!live_step.observed_events.is_empty());
    let live_after = controller.checkpoint().unwrap();
    let live_replay = controller.export_replay().unwrap();
    assert_eq!(live_replay.steps.len(), 1);
    assert_eq!(
        live_replay.final_identity,
        InitialEnvironmentIdentityV5 {
            state_revision: live_after.state.revision,
            full_state_digest: live_after.state_digest.clone(),
            episode_status: live_after.status.clone(),
            environment_limit_counters: live_after.limit_counters.clone(),
            checkpoint_codec_identity: live_after.codec.clone(),
            checkpoint_digest: live_after.checkpoint_digest.clone(),
            execution_identity: live_after.execution_identity.clone(),
        }
    );

    let report = controller
        .execute_replay_from_checkpoint(cp0.clone(), live_replay.clone())
        .unwrap();
    assert_eq!(report.traces.len(), 1);
    let trace = &report.traces[0];
    let replay_events = crate::lifecycle_projection::project_occurrence_envelopes(
        &trace.before.state,
        &trace.after.state,
        &trace.transition.events,
    )
    .unwrap();
    assert!(!trace.transition.events.is_empty());
    assert!(!replay_events[&PlayerId(1)].is_empty());
    assert!(!replay_events[&PlayerId(2)].is_empty());
    assert_eq!(replay_events[&PlayerId(1)], live_step.observed_events);
    assert!(matches!(
        &replay_events[&PlayerId(1)][0].event,
        mtgml_observation::ObservedEventKindV2::ObjectMoved {
            old_object: None,
            new_object: Some(_),
            ..
        }
    ));
    assert!(matches!(
        &replay_events[&PlayerId(2)][0].event,
        mtgml_observation::ObservedEventKindV2::PublicOutcome { code }
            if code == "p2-public"
    ));

    let mut replay_step = SyntheticM1EnvironmentBackend::player_step_from_state(
        &trace.after.state,
        PlayerId(1),
        trace.after.status.clone(),
        PlayerStepSubmissionV1::Accepted,
    )
    .unwrap();
    replay_step.observed_events = replay_events[&PlayerId(1)].clone();
    replay_step.validate().unwrap();
    assert_eq!(
        mtgml_wire::encode_canonical(&replay_step).unwrap(),
        mtgml_wire::encode_canonical(&live_step).unwrap(),
    );

    let p1_bytes = serde_json::to_vec(&replay_events[&PlayerId(1)]).unwrap();
    let p2_bytes = serde_json::to_vec(&replay_events[&PlayerId(2)]).unwrap();
    assert_ne!(
        p1_bytes, p2_bytes,
        "perspective products must remain separated"
    );
    for bytes in [&p1_bytes, &p2_bytes] {
        let text = String::from_utf8_lossy(bytes);
        for forbidden in [
            "GameObjectId",
            "PhysicalCardId",
            "DecisionId",
            "RuleEventId",
            "root_seed",
            "raw_words",
            "checkpoint_digest",
        ] {
            assert!(!text.contains(forbidden), "forbidden field {forbidden}");
        }
    }

    assert_eq!(controller.checkpoint().unwrap(), live_after);
    assert_eq!(controller.export_replay().unwrap(), live_replay);
}

fn snapshot_bytes(endpoint: &PlayerEndpointHandle) -> PerspectiveBytes {
    PerspectiveBytes {
        information_bytes: mtgml_wire::encode_canonical(&endpoint.information_state().unwrap())
            .unwrap(),
        observation_bytes: mtgml_wire::encode_canonical(&endpoint.observation().unwrap()).unwrap(),
        visible_decision_bytes: visible_decision_bytes(endpoint),
    }
}

#[test]
fn historical_reprojection_byte_exact() {
    let controller = TrustedEnvironmentController::new(backend());
    let p1 = controller.bind_player(PlayerId(1)).unwrap();
    let p2 = controller.bind_player(PlayerId(2)).unwrap();

    // The initial checkpoint BEFORE any submission anchors the replay.
    let cp0 = controller.checkpoint().unwrap();

    // DECISION-RICH live run: accepted submissions through the REAL
    // endpoints (entry -> count -> members), capturing per-step products.
    let mut captures: Vec<(
        PlayerStepV2,
        crate::checkpoint::EnvironmentCheckpointV5,
        _,
        _,
    )> = Vec::new();
    let mut previous_counters = cp0.limit_counters.clone();
    for answer in [
        order_entry_answer(),
        number_answer(2),
        members_answer(&[0, 1]),
    ] {
        let step = submit_answer(&p1, answer);
        assert_eq!(
            step.submission,
            PlayerStepSubmissionV1::Accepted,
            "fixture submissions must be accepted"
        );
        let checkpoint = controller.checkpoint().unwrap();
        // Live trusted controls: exactly +1 decision and +1 accepted
        // transition; the host-independent counters stay EXACTLY unchanged.
        assert_eq!(
            checkpoint.limit_counters.decisions_submitted,
            previous_counters.decisions_submitted + 1
        );
        assert_eq!(
            checkpoint.limit_counters.accepted_transitions,
            previous_counters.accepted_transitions + 1
        );
        assert!(
            checkpoint.limit_counters.rule_events_emitted >= previous_counters.rule_events_emitted
        );
        assert_eq!(
            checkpoint.limit_counters.resource_units_consumed,
            previous_counters.resource_units_consumed
        );
        assert_eq!(
            checkpoint.limit_counters.wall_clock_elapsed_millis,
            previous_counters.wall_clock_elapsed_millis
        );
        previous_counters = checkpoint.limit_counters.clone();
        captures.push((step, checkpoint, snapshot_bytes(&p1), snapshot_bytes(&p2)));
    }

    let live_final = controller.checkpoint().unwrap();
    let replay = controller.export_replay().unwrap();
    assert_eq!(replay.steps.len(), 3);
    assert!(replay.steps.iter().all(|step| step.accepted));

    let report = controller
        .execute_replay_from_checkpoint(cp0.clone(), replay.clone())
        .unwrap();
    assert_eq!(report.traces.len(), 3);

    let mut previous_after_digest = cp0.checkpoint_digest.clone();
    for (index, trace) in report.traces.iter().enumerate() {
        assert_eq!(trace.step_index, index as u64);
        // Identity-chain continuity across the executed segment.
        assert_eq!(trace.before.checkpoint_digest, previous_after_digest);
        assert_eq!(trace.after.state, trace.transition.next_state);
        assert_eq!(trace.after.status, trace.transition.status);
        previous_after_digest = trace.after.checkpoint_digest.clone();

        // Replayed trusted controls: exact +1/+1/+events progression and
        // EXACTLY unchanged host-independent counters.
        assert_eq!(
            trace.after.limit_counters.decisions_submitted,
            trace.before.limit_counters.decisions_submitted + 1
        );
        assert_eq!(
            trace.after.limit_counters.accepted_transitions,
            trace.before.limit_counters.accepted_transitions + 1
        );
        let events = u64::try_from(trace.transition.events.len()).unwrap();
        assert_eq!(
            trace.after.limit_counters.rule_events_emitted,
            trace.before.limit_counters.rule_events_emitted + events
        );
        assert_eq!(
            trace.after.limit_counters.resource_units_consumed,
            trace.before.limit_counters.resource_units_consumed
        );
        assert_eq!(
            trace.after.limit_counters.wall_clock_elapsed_millis,
            trace.before.limit_counters.wall_clock_elapsed_millis
        );

        // Both chain ends equal the live-run captures step by step.
        let expected_before_counters = if index == 0 {
            &cp0.limit_counters
        } else {
            &captures[index - 1].1.limit_counters
        };
        assert_eq!(&trace.before.limit_counters, expected_before_counters);
        let (live_step, live_checkpoint, live_p1, live_p2) = &captures[index];
        assert_eq!(&trace.after.limit_counters, &live_checkpoint.limit_counters);
        assert_eq!(trace.after.state.revision, live_checkpoint.state.revision);

        // HISTORICAL REPROJECTION through the production functions only.
        let actor = replay.steps[index].actor;
        let envelopes = crate::lifecycle_projection::project_occurrence_envelopes(
            &trace.before.state,
            &trace.after.state,
            &trace.transition.events,
        )
        .unwrap();
        let mut rebuilt = SyntheticM1EnvironmentBackend::player_step_from_state(
            &trace.after.state,
            actor,
            trace.after.status.clone(),
            PlayerStepSubmissionV1::Accepted,
        )
        .unwrap();
        if let Some(observed) = envelopes.get(&actor) {
            rebuilt.observed_events = observed.clone();
        }
        rebuilt.validate().unwrap();
        assert_eq!(
            mtgml_wire::encode_canonical(&rebuilt).unwrap(),
            mtgml_wire::encode_canonical(live_step).unwrap(),
            "rebuilt step {index} must be byte-exact with the live product"
        );

        // Rebuilt-endpoint surface for the visible-decision parity check:
        // both perspectives read through REAL bound endpoints over the
        // replayed after-state.
        let step_config = config([PlayerId(1), PlayerId(2)]);
        let step_checkpoint = EnvironmentCheckpointV5::new(
            trace.after.state.clone(),
            trace.after.status.clone(),
            trace.after.limit_counters.clone(),
            step_config.codec.clone(),
            synthetic_identity(),
        )
        .unwrap();
        let step_controller = TrustedEnvironmentController::new(
            SyntheticM1EnvironmentBackend::from_checkpoint(step_checkpoint, step_config).unwrap(),
        );
        let step_endpoints = [
            step_controller.bind_player(PlayerId(1)).unwrap(),
            step_controller.bind_player(PlayerId(2)).unwrap(),
        ];

        for (step_endpoint, (perspective, live_bytes)) in step_endpoints
            .iter()
            .zip([(PlayerId(1), live_p1), (PlayerId(2), live_p2)])
        {
            let information = SyntheticM1EnvironmentBackend::player_information_state_from_state(
                &trace.after.state,
                perspective,
            )
            .unwrap();
            assert_eq!(
                mtgml_wire::encode_canonical(&information).unwrap(),
                live_bytes.information_bytes
            );
            let observation = SyntheticM1EnvironmentBackend::synthetic_observation(
                &trace.after.state,
                perspective,
            )
            .unwrap();
            assert_eq!(
                mtgml_wire::encode_canonical(&observation).unwrap(),
                live_bytes.observation_bytes
            );
            assert_eq!(
                visible_decision_bytes(step_endpoint),
                live_bytes.visible_decision_bytes,
                "rebuilt visible decision must match the live capture at step {index}"
            );
        }
    }
    assert_eq!(
        previous_after_digest,
        replay.final_identity.checkpoint_digest
    );

    // Current-state parity: endpoints rebuilt from the FINAL checkpoint
    // reproduce the live final projections byte-for-byte.
    let rebuilt_controller = TrustedEnvironmentController::new(
        SyntheticM1EnvironmentBackend::from_checkpoint(
            report.final_checkpoint.clone(),
            config([PlayerId(1), PlayerId(2)]),
        )
        .unwrap(),
    );
    let rebuilt_endpoints = [
        rebuilt_controller.bind_player(PlayerId(1)).unwrap(),
        rebuilt_controller.bind_player(PlayerId(2)).unwrap(),
    ];
    let (_, _, final_p1, final_p2) = captures.last().unwrap();
    for (endpoint, live_bytes) in rebuilt_endpoints.iter().zip([final_p1, final_p2]) {
        assert_eq!(
            mtgml_wire::encode_canonical(&endpoint.information_state().unwrap()).unwrap(),
            live_bytes.information_bytes
        );
        assert_eq!(
            mtgml_wire::encode_canonical(&endpoint.observation().unwrap()).unwrap(),
            live_bytes.observation_bytes
        );
        assert_eq!(
            visible_decision_bytes(endpoint),
            live_bytes.visible_decision_bytes,
            "rebuilt visible decision must match the live final capture"
        );
    }

    // Execution ran on an internal fork: the live controller is untouched,
    // including its recorder segment.
    assert_eq!(controller.checkpoint().unwrap(), live_final);
    assert_eq!(controller.export_replay().unwrap(), replay);
}

#[test]
fn diagnostic_rejected_step_executes_with_intact_identity_chain() {
    let controller = TrustedEnvironmentController::new(backend());
    let cp0 = controller.checkpoint().unwrap();
    let segment = controller.export_replay().unwrap();
    assert!(segment.steps.is_empty());

    // One hand-built accepted:false diagnostic step preserving EVERY
    // after-field of the starting identity (structural contract).
    let initial = &segment.manifest.initial_identity;
    let diagnostic_step = ReplayStepV5 {
        step_index: 0,
        actor: PlayerId(1),
        checkpoint_digest_before: initial.checkpoint_digest.clone(),
        state_revision_before: initial.state_revision,
        response: DecisionResponseV2 {
            schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
            player_decision_id: PlayerDecisionIdV1(1),
            state_revision: initial.state_revision,
            answer: DecisionAnswerV2::SelectOne {
                candidate_id: CandidateIdV1(1),
            },
        },
        accepted: false,
        state_revision_after: initial.state_revision,
        full_state_digest_after: initial.full_state_digest.clone(),
        episode_status_after: initial.episode_status.clone(),
        environment_limit_counters_after: initial.environment_limit_counters.clone(),
        checkpoint_digest_after: initial.checkpoint_digest.clone(),
    };
    let diagnostic = AuthoritativeReplayV5 {
        schema_version: REPLAY_FILE_SCHEMA_V5.into(),
        manifest: segment.manifest.clone(),
        steps: vec![diagnostic_step],
        final_identity: initial.clone(),
    };
    diagnostic.validate().unwrap();

    let report = controller
        .execute_replay_from_checkpoint(cp0.clone(), diagnostic)
        .unwrap();
    assert_eq!(report.traces.len(), 1);
    let trace = &report.traces[0];
    assert!(!trace.transition.accepted);
    assert_eq!(trace.before.checkpoint_digest, cp0.checkpoint_digest);
    assert_eq!(trace.after.checkpoint_digest, cp0.checkpoint_digest);
    assert_eq!(trace.after, trace.before);
    assert_eq!(report.final_checkpoint, cp0);

    // Zero mutation of the live controller, including its recorder.
    assert_eq!(controller.checkpoint().unwrap(), cp0);
    assert!(controller.export_replay().unwrap().steps.is_empty());
}

#[test]
fn recorded_external_counter_progression_is_applied_without_live_mutation() {
    use mtgml_model::CheckpointDigestV5;
    use mtgml_persistence::checkpoint_digest::calculate_checkpoint_digest_v5;
    use mtgml_replay::InitialEnvironmentIdentityV5;

    // Re-anchors the after-identity digests onto a mutated counter set so
    // ONLY the recorded counter progression diverges from what deterministic
    // execution reproduces (the structural gate alone already rejects
    // digest-inconsistent recordings).
    fn resealed(tampered: &mut AuthoritativeReplayV5) {
        let identity = InitialEnvironmentIdentityV5 {
            state_revision: tampered.steps[0].state_revision_after,
            full_state_digest: tampered.steps[0].full_state_digest_after.clone(),
            episode_status: tampered.steps[0].episode_status_after.clone(),
            environment_limit_counters: tampered.steps[0].environment_limit_counters_after.clone(),
            checkpoint_codec_identity: tampered
                .manifest
                .initial_identity
                .checkpoint_codec_identity
                .clone(),
            checkpoint_digest: CheckpointDigestV5::from_digest_bytes([0; 32]),
            execution_identity: tampered
                .manifest
                .initial_identity
                .execution_identity
                .clone(),
        };
        let identity = InitialEnvironmentIdentityV5 {
            checkpoint_digest: calculate_checkpoint_digest_v5(
                &identity.full_state_digest.as_digest_reference(),
                &identity.episode_status,
                &identity.environment_limit_counters,
                &identity.checkpoint_codec_identity,
                &identity.execution_identity,
            )
            .unwrap(),
            ..identity
        };
        tampered.steps[0].checkpoint_digest_after = identity.checkpoint_digest.clone();
        tampered.final_identity = identity;
    }

    let live = TrustedEnvironmentController::new(backend());
    let cp0 = live.checkpoint().unwrap();
    let p1 = live.bind_player(PlayerId(1)).unwrap();
    let _ = submit_answer(&p1, order_entry_answer());
    let after = live.checkpoint().unwrap();
    let pristine = live.export_replay().unwrap();
    assert_eq!(pristine.steps.len(), 1);

    let run = |replay: AuthoritativeReplayV5| {
        TrustedEnvironmentController::new(backend())
            .execute_replay_from_checkpoint(cp0.clone(), replay)
    };

    // Digest-consistent forward wall-clock progression is trusted replay
    // control data and must be applied to the replay-owned backend.
    let mut tampered = pristine.clone();
    tampered.steps[0]
        .environment_limit_counters_after
        .wall_clock_elapsed_millis += 1000;
    resealed(&mut tampered);
    tampered.validate().unwrap();
    let expected_counters = tampered.steps[0].environment_limit_counters_after.clone();
    let report = run(tampered).unwrap();
    assert_eq!(report.final_checkpoint.limit_counters, expected_counters);

    // Same for resource-unit progression.
    let mut tampered = pristine.clone();
    tampered.steps[0]
        .environment_limit_counters_after
        .resource_units_consumed += 5;
    resealed(&mut tampered);
    tampered.validate().unwrap();
    let expected_counters = tampered.steps[0].environment_limit_counters_after.clone();
    let report = run(tampered).unwrap();
    assert_eq!(report.final_checkpoint.limit_counters, expected_counters);

    // A decisions_submitted overcount is rejected at the earliest gate by
    // the structural exact +1-per-accepted-step contract itself.
    let mut tampered = pristine.clone();
    tampered.steps[0]
        .environment_limit_counters_after
        .decisions_submitted += 1;
    assert!(matches!(
        run(tampered),
        Err(ControllerError::ReplayValidation(
            ReplayValidationError::CounterProgression
        ))
    ));

    // Failed executions never touched the live controller.
    assert_eq!(live.checkpoint().unwrap(), after);
    assert_eq!(live.export_replay().unwrap(), pristine);
}
