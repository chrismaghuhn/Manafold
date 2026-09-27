use super::*;
use crate::semantic_catalog_generated::synthetic_legacy_default_semantic_contract_id;

use mtgml_decision::{DecisionAnswerV2, DecisionResponseV2, DECISION_RESPONSE_V2_SCHEMA};

use mtgml_model::{
    CandidateIdV1, CheckpointDigestV6, ContentDigest, ContinuationId, EpisodeStatus,
    ExecutionIdentityV1, ExecutionProgramV1, FullStateDigestV5, OpaqueObjectId, PlayerDecisionIdV1,
    PlayerId, PlayerOutcome, PlayerResult, StateRevision, TerminalReason, TruncationReason,
};

use mtgml_observation::{
    PlayerStepV2, INFORMATION_STATE_SCHEMA_V2, OBSERVATION_SCHEMA, OBSERVED_EVENT_SCHEMA_V2,
    PLAYER_STEP_SCHEMA_V2,
};

use mtgml_random::RootSeed256;
use std::collections::BTreeMap;

use mtgml_replay::{
    AuthoritativeReplayV6, DeckIdentityV1, KernelIdentityV1, ReplaySchemaVersionsV6,
};

mod magic_basic_land_observation;

#[test]
fn successor_event_projection_uses_only_policy_authorized_opaque_identity() {
    use mtgml_rules::{
        AuthoritativeRuleEventKindV2, AuthoritativeRuleEventV2, SuccessorObservationPolicyV1,
    };
    use mtgml_state::{
        EngineStatePartsV2, PerspectiveLifecycleAuditV1, PerspectiveLifecycleMutationV1,
    };

    let before = magic_basic_land_observation::basic_land_parts(seed());
    let before_engine = before.materialize();
    let object = before_engine
        .zones
        .objects
        .keys()
        .copied()
        .find(|object| {
            before_engine.zones.locations[object].zone == mtgml_model::ZoneKind::Battlefield
        })
        .unwrap();
    let mut after_engine = before_engine.clone();
    let next_revision = StateRevision(before_engine.revision.0 + 1);
    after_engine.revision = next_revision;
    after_engine.zones.objects.get_mut(&object).unwrap().tapped = true;
    let perspective = PlayerId(1);
    let sequence = after_engine
        .knowledge
        .players
        .get(&perspective)
        .unwrap()
        .next_visible_sequence;
    let audit = PerspectiveLifecycleAuditV1 {
        perspective,
        sequence,
        mutation: PerspectiveLifecycleMutationV1::default(),
    };
    mtgml_state::apply_perspective_lifecycle(&mut after_engine, &audit).unwrap();
    let after = EngineStatePartsV2::from_state(&after_engine, before.card_rules_state.clone());
    let opaque =
        before_engine.perspective_identities.players[&perspective].object_to_opaque[&object];
    let event = AuthoritativeRuleEventV2 {
        event_id: mtgml_model::RuleEventId(1),
        state_revision: next_revision,
        event: AuthoritativeRuleEventKindV2::PerspectiveOccurrence {
            lifecycle: Box::new(audit),
            observation: SuccessorObservationPolicyV1::ObjectTapped {
                object,
                tapped: true,
            },
        },
    };

    let projected =
        crate::successor_projection::project_successor_events_v3(&before, &after, &[event])
            .unwrap();
    assert!(matches!(
        projected[&perspective][0].event,
        mtgml_observation::ObservedEventKindV3::ObjectTapped { object, tapped: true }
            if object == opaque
    ));
    assert!(projected[&PlayerId(2)].is_empty());
    assert_eq!(
        after.predecessor_v5.knowledge.players[&perspective]
            .next_visible_sequence
            .0,
        sequence.0 + 1
    );
}

#[test]
fn successor_runtime_commits_v3_steps_checkpoint_and_replay_atomically() {
    use mtgml_card_ir::{
        admit_executable_profile_v1, decode_content_manifest_v1, ExecutableProfileAdmissionV1,
    };
    use mtgml_model::{ContentContractIdV1, EnvironmentLimitCounters};
    use mtgml_observation::ObservedEventKindV3;
    use mtgml_replay::ReplayManifestV7;
    use mtgml_state::EngineStatePartsV2;

    fn admitted_profile() -> (ExecutableProfileAdmissionV1, ReplayManifestV7) {
        let content_bytes =
            include_bytes!("../../../cards/definitions/basic-land-v1/content-contract.v1.cbor");
        let provenance_bytes =
            include_bytes!("../../../cards/definitions/basic-land-v1/provenance.v1.cbor");
        let manifest: ReplayManifestV7 = serde_json::from_str(include_str!(
            "../../../schemas/examples/replay-manifest-v7-phase9-admitted-basic-land.json"
        ))
        .unwrap();
        let content = manifest
            .semantic_contract
            .content_contract
            .as_ref()
            .unwrap();
        let content_id = ContentContractIdV1::parse(content.content_contract_id.as_str()).unwrap();
        let admission = admit_executable_profile_v1(
            content_bytes,
            &content_id,
            provenance_bytes,
            &manifest.semantic_contract.rules_manifest,
            &manifest.semantic_contract.manifest,
            &manifest.execution_identity,
        )
        .unwrap();
        let _ = decode_content_manifest_v1(content_bytes).unwrap();
        (admission, manifest)
    }

    let (admission, mut manifest) = admitted_profile();
    let mut state = magic_basic_land_observation::basic_land_parts(seed());
    let catalog_manifest = decode_content_manifest_v1(include_bytes!(
        "../../../cards/definitions/basic-land-v1/content-contract.v1.cbor"
    ))
    .unwrap();
    let definitions = catalog_manifest
        .definitions
        .iter()
        .map(|definition| definition.card_definition_id)
        .collect::<Vec<_>>();
    let mut engine = state.materialize();
    for (index, object) in engine.zones.objects.values_mut().enumerate() {
        object.card_definition = definitions[index % definitions.len()];
    }
    let actor = PlayerId(2);
    let hand_object = engine
        .zones
        .locations
        .iter()
        .find_map(|(object, location)| {
            (location.zone == mtgml_model::ZoneKind::Library && location.player == Some(actor))
                .then_some(*object)
        })
        .unwrap();
    let previous_location = engine.zones.locations[&hand_object].clone();
    if matches!(
        previous_location.position,
        mtgml_state::ZonePosition::Top { .. }
    ) {
        let previous_key = previous_location.key();
        let ordered = engine.zones.ordered_zones.get_mut(&previous_key).unwrap();
        ordered.retain(|object| *object != hand_object);
        if ordered.is_empty() {
            engine.zones.ordered_zones.remove(&previous_key);
        }
    }
    let hand_location = mtgml_state::ZoneLocation {
        zone: mtgml_model::ZoneKind::Hand,
        player: Some(actor),
        position: mtgml_state::ZonePosition::Top { offset: 0 },
        visibility: mtgml_state::VisibilityPartition::OwnerOnly,
        partition: None,
    };
    let hand_key = hand_location.key();
    engine
        .zones
        .ordered_zones
        .entry(hand_key)
        .or_default()
        .push(hand_object);
    engine
        .zones
        .locations
        .insert(hand_object, hand_location.clone());
    let opaque = {
        let identity = engine
            .perspective_identities
            .players
            .get_mut(&actor)
            .unwrap();
        let opaque = if let Some(existing) = identity.object_to_opaque.get(&hand_object) {
            *existing
        } else {
            let opaque = identity.next_opaque_object_id;
            identity.next_opaque_object_id.0 += 1;
            identity.object_to_opaque.insert(hand_object, opaque);
            identity.opaque_to_object.insert(opaque, hand_object);
            opaque
        };
        opaque
    };
    engine
        .knowledge
        .players
        .get_mut(&actor)
        .unwrap()
        .active
        .insert(
            opaque,
            mtgml_state::KnowledgeRecordV2 {
                opaque_object: opaque,
                physical_card: engine.zones.objects[&hand_object].physical_card,
                card_definition: Some(engine.zones.objects[&hand_object].card_definition),
                known_location: Some(mtgml_state::KnownLocationFactV2 {
                    location: hand_location,
                    provenance: mtgml_state::KnowledgeAcquisitionReason::InitialConfiguration,
                }),
                acquisition: mtgml_state::KnowledgeAcquisitionReason::InitialConfiguration,
                historical_locations: Vec::new(),
            },
        );
    engine.core.position = mtgml_state::TurnPosition::PrecombatMain;
    engine.core.active_player = actor;
    engine.core.priority = mtgml_state::PriorityState::HeldBy {
        player: actor,
        consecutive_passes: 0,
    };
    state = EngineStatePartsV2::from_state(&engine, state.card_rules_state.clone());
    state
        .card_rules_state
        .mana
        .add(
            actor,
            mtgml_state::ManaColorV1::Red,
            mtgml_state::ManaRestrictionV1::Unrestricted,
            1,
        )
        .unwrap();
    state.execution_v3.pending_decision = None;
    state.validate().unwrap();

    let install_kernel =
        mtgml_rules::ProgramKernelV1::for_executable_profile(admission.clone()).unwrap();
    let request = install_kernel
        .install_successor_request(&mut state, actor, &EpisodeStatus::Running)
        .unwrap();
    let before_checkpoint = EnvironmentCheckpointV7::new(
        state.clone(),
        EpisodeStatus::Running,
        EnvironmentLimitCounters::default(),
        admission.execution_identity().clone(),
    )
    .unwrap();
    let mut second_deck = manifest.decks.first().unwrap().clone();
    second_deck.player = actor;
    second_deck.deck_id = "deck:synthetic-p2".to_owned();
    manifest.decks.push(second_deck);
    manifest.decks.sort_by_key(|deck| deck.player);
    manifest.initial_identity = mtgml_replay::InitialEnvironmentIdentityV7 {
        state_revision: state.predecessor_v5.revision,
        full_state_digest: before_checkpoint.state_digest.clone(),
        episode_status: EpisodeStatus::Running,
        environment_limit_counters: EnvironmentLimitCounters::default(),
        checkpoint_codec_identity: before_checkpoint.codec.clone(),
        checkpoint_digest: before_checkpoint.checkpoint_digest.clone(),
        execution_identity: admission.execution_identity().clone(),
    };

    // A structurally valid, self-consistent checkpoint must still prove that
    // its stored request contains the complete RulesKernel candidate surface.
    let mut incomplete_state = before_checkpoint.state.clone();
    let incomplete_request = incomplete_state
        .execution_v3
        .pending_decision
        .as_mut()
        .unwrap();
    let removed_candidate = incomplete_request
        .candidates
        .iter()
        .position(|candidate| {
            matches!(
                candidate.visible_intent,
                mtgml_decision::CandidateIntentV3::PlayLand { .. }
            )
        })
        .expect("fixture has a legal PlayLand candidate");
    incomplete_request.candidates.remove(removed_candidate);
    for (index, candidate) in incomplete_request.candidates.iter_mut().enumerate() {
        candidate.candidate_id = CandidateIdV1(index as u32);
    }
    let incomplete_checkpoint = EnvironmentCheckpointV7::new(
        incomplete_state.clone(),
        EpisodeStatus::Running,
        EnvironmentLimitCounters::default(),
        admission.execution_identity().clone(),
    )
    .unwrap();
    let incomplete_manifest = {
        let mut manifest = manifest.clone();
        manifest.initial_identity = mtgml_replay::InitialEnvironmentIdentityV7 {
            state_revision: incomplete_checkpoint.state.predecessor_v5.revision,
            full_state_digest: incomplete_checkpoint.state_digest.clone(),
            episode_status: incomplete_checkpoint.status.clone(),
            environment_limit_counters: incomplete_checkpoint.limit_counters.clone(),
            checkpoint_codec_identity: incomplete_checkpoint.codec.clone(),
            checkpoint_digest: incomplete_checkpoint.checkpoint_digest.clone(),
            execution_identity: admission.execution_identity().clone(),
        };
        manifest
    };
    assert!(crate::SuccessorEnvironmentRuntime::new(
        admission.clone(),
        incomplete_state,
        EpisodeStatus::Running,
        EnvironmentLimitCounters::default(),
        incomplete_manifest.clone(),
    )
    .is_err());
    let empty_incomplete_replay = mtgml_replay::ReplayRecorderV7::new(incomplete_manifest)
        .unwrap()
        .export()
        .unwrap();
    assert!(crate::replay_v7_execution::execute_authoritative_replay_v7(
        admission.clone(),
        incomplete_checkpoint.clone(),
        empty_incomplete_replay,
    )
    .is_err());

    let mut runtime = crate::SuccessorEnvironmentRuntime::new(
        admission.clone(),
        state,
        EpisodeStatus::Running,
        EnvironmentLimitCounters::default(),
        manifest,
    )
    .unwrap();
    let runtime_before_bad_restore = runtime.checkpoint().unwrap();
    let replay_before_bad_restore = runtime.export_replay().unwrap();
    assert!(runtime.restore(incomplete_checkpoint).is_err());
    assert_eq!(runtime.checkpoint().unwrap(), runtime_before_bad_restore);
    assert_eq!(runtime.export_replay().unwrap(), replay_before_bad_restore);
    runtime.restore(before_checkpoint.clone()).unwrap();
    assert_eq!(runtime.checkpoint().unwrap(), before_checkpoint);
    assert_eq!(
        runtime.visible_decision(actor).unwrap().as_ref(),
        Some(&request.project_player_request().unwrap())
    );

    let valid_candidate = request.candidates[0].candidate_id;
    let mut mismatched_revision = DecisionResponseV2 {
        schema_version: DECISION_RESPONSE_V2_SCHEMA.to_owned(),
        player_decision_id: request.player_decision_id,
        state_revision: request.state_revision,
        answer: DecisionAnswerV2::SelectOne {
            candidate_id: valid_candidate,
        },
    };
    mismatched_revision.state_revision = mtgml_model::StateRevision(
        request
            .state_revision
            .0
            .checked_add(1)
            .expect("test revision fits"),
    );
    let mut wrong_decision = DecisionResponseV2 {
        schema_version: DECISION_RESPONSE_V2_SCHEMA.to_owned(),
        player_decision_id: request.player_decision_id,
        state_revision: request.state_revision,
        answer: DecisionAnswerV2::SelectOne {
            candidate_id: valid_candidate,
        },
    };
    wrong_decision.player_decision_id = mtgml_model::PlayerDecisionIdV1(
        request
            .player_decision_id
            .0
            .checked_add(1)
            .expect("test identity fits"),
    );
    let invalid_submissions = [
        (
            actor,
            DecisionResponseV2 {
                schema_version: DECISION_RESPONSE_V2_SCHEMA.to_owned(),
                player_decision_id: request.player_decision_id,
                state_revision: request.state_revision,
                answer: DecisionAnswerV2::SelectOne {
                    candidate_id: CandidateIdV1(u32::MAX),
                },
            },
        ),
        (actor, mismatched_revision),
        (actor, wrong_decision),
        (
            request
                .candidates
                .iter()
                .find_map(|_| {
                    before_checkpoint
                        .state
                        .predecessor_v5
                        .core
                        .players
                        .keys()
                        .copied()
                        .find(|player| *player != actor)
                })
                .expect("two-player fixture has another actor"),
            DecisionResponseV2 {
                schema_version: DECISION_RESPONSE_V2_SCHEMA.to_owned(),
                player_decision_id: request.player_decision_id,
                state_revision: request.state_revision,
                answer: DecisionAnswerV2::SelectOne {
                    candidate_id: valid_candidate,
                },
            },
        ),
    ];
    for (submitted_actor, response) in invalid_submissions {
        let mut rejected_runtime = runtime.fork().unwrap();
        let before_rejection = rejected_runtime.checkpoint().unwrap();
        let replay_before_rejection = rejected_runtime.export_replay().unwrap();
        let replay_bytes_before_rejection =
            mtgml_wire::encode_canonical(&replay_before_rejection).unwrap();
        let result = rejected_runtime.submit(submitted_actor, response);
        if let Ok(rejected) = result {
            assert!(!rejected.transition.accepted);
            assert_eq!(rejected.checkpoint, before_rejection);
            assert!(rejected.transition.events.is_empty());
            assert!(rejected
                .player_steps
                .values()
                .all(|step| step.observed_events.is_empty()));
        }
        assert_eq!(rejected_runtime.checkpoint().unwrap(), before_rejection);
        let replay_after_rejection = rejected_runtime.export_replay().unwrap();
        assert_eq!(replay_after_rejection, replay_before_rejection);
        assert_eq!(
            mtgml_wire::encode_canonical(&replay_after_rejection).unwrap(),
            replay_bytes_before_rejection,
            "rejected responses must not mutate replay/history bytes"
        );
    }

    let fork = runtime.fork().unwrap();
    let candidate = request
        .candidates
        .iter()
        .find(|candidate| {
            matches!(
                candidate.visible_intent,
                mtgml_decision::CandidateIntentV3::PlayLand { .. }
            )
        })
        .unwrap();
    let response = DecisionResponseV2 {
        schema_version: DECISION_RESPONSE_V2_SCHEMA.to_owned(),
        player_decision_id: request.player_decision_id,
        state_revision: request.state_revision,
        answer: DecisionAnswerV2::SelectOne {
            candidate_id: candidate.candidate_id,
        },
    };
    let controller =
        crate::controller_successor::TrustedEnvironmentController::new(runtime.fork().unwrap());
    let endpoint = controller.bind_player(actor).unwrap();
    use crate::endpoint_successor::PlayerEndpoint as _;
    assert_eq!(
        endpoint.visible_decision().unwrap(),
        Some(request.project_player_request().unwrap())
    );
    let controller_step = endpoint.submit(response.clone()).unwrap();
    let controller_checkpoint = controller.checkpoint().unwrap();

    let output = runtime.submit(actor, response.clone()).unwrap();
    let fork_output = {
        let mut fork = fork;
        fork.submit(actor, response.clone()).unwrap()
    };
    assert_eq!(
        output, fork_output,
        "direct and forked runtime products match"
    );
    assert_eq!(controller_step, output.player_steps[&actor]);
    assert_eq!(controller_checkpoint, output.checkpoint);
    assert!(output.transition.accepted);
    assert_eq!(output.checkpoint.state, *runtime.state());
    assert_eq!(output.player_steps.len(), 2);
    assert_eq!(
        output.player_steps[&actor]
            .next_decision
            .as_ref()
            .unwrap()
            .candidates
            .len(),
        output
            .transition
            .next_decision
            .as_ref()
            .unwrap()
            .candidates
            .len()
    );
    assert!(output.player_steps[&actor]
        .observed_events
        .iter()
        .any(|event| matches!(event.event, ObservedEventKindV3::ObjectMoved { .. })));
    let replay = runtime.export_replay().unwrap();
    replay.validate().unwrap();

    let replayed = runtime.execute_replay(replay.clone()).unwrap();
    assert_eq!(replayed.final_checkpoint, output.checkpoint);
    assert_eq!(replayed.transitions.len(), 1);
    assert_eq!(replayed.transitions[0].transition, output.transition);
    assert_eq!(replayed.transitions[0].player_steps, output.player_steps);

    let mut tampered = replay;
    tampered.steps[0].response.answer = DecisionAnswerV2::SelectOne {
        candidate_id: CandidateIdV1(u32::MAX),
    };
    assert!(crate::replay_v7_execution::execute_authoritative_replay_v7(
        admission.clone(),
        before_checkpoint,
        tampered,
    )
    .is_err());

    // Two priority passes close the current window atomically: pools empty,
    // turn position advances, and the next V3 decision is already installed.
    let make_pass_response = |request: &mtgml_decision::PlayerDecisionRequestV3| {
        let pass = request
            .candidates
            .iter()
            .find(|candidate| {
                matches!(
                    candidate.intent,
                    mtgml_decision::CandidateIntentV3::PassPriority
                )
            })
            .expect("priority request exposes pass");
        DecisionResponseV2 {
            schema_version: DECISION_RESPONSE_V2_SCHEMA.to_owned(),
            player_decision_id: request.player_decision_id,
            state_revision: request.state_revision,
            answer: DecisionAnswerV2::SelectOne {
                candidate_id: pass.candidate_id,
            },
        }
    };
    let active_request = runtime.visible_decision(actor).unwrap().unwrap();
    let first_pass = runtime
        .submit(actor, make_pass_response(&active_request))
        .unwrap();
    assert!(first_pass.transition.accepted);
    let nonactive = first_pass.transition.next_decision.as_ref().unwrap().actor;
    assert_ne!(nonactive, actor);
    let nonactive_request = runtime.visible_decision(nonactive).unwrap().unwrap();
    let second_pass = runtime
        .submit(nonactive, make_pass_response(&nonactive_request))
        .unwrap();
    assert!(second_pass.transition.accepted);
    assert_eq!(
        second_pass.checkpoint.state.predecessor_v5.core.position,
        mtgml_state::TurnPosition::Combat {
            step: mtgml_state::CombatStep::BeginningOfCombat,
        }
    );
    assert_eq!(
        second_pass.checkpoint.state.predecessor_v5.core.priority,
        mtgml_state::PriorityState::HeldBy {
            player: actor,
            consecutive_passes: 0,
        }
    );
    assert_eq!(
        second_pass.checkpoint.state.card_rules_state.mana.pools[&actor],
        mtgml_state::ManaPoolV1::default()
    );
    assert_eq!(
        second_pass
            .transition
            .next_decision
            .as_ref()
            .map(|request| request.actor),
        Some(actor),
        "accepted progress may not expose Running without a decision"
    );
    let mut missing_pool_clear = second_pass.transition.clone();
    let clear_operation = missing_pool_clear
        .delta
        .operations
        .iter()
        .position(|operation| {
            matches!(
                operation,
                mtgml_state::SemanticDeltaOperationV2::ManaPoolEmptied { .. }
            )
        })
        .unwrap();
    missing_pool_clear.delta.operations.remove(clear_operation);
    missing_pool_clear.delta = mtgml_state::StateDeltaV2::between(
        &first_pass.checkpoint.state,
        &missing_pool_clear.next_state,
        missing_pool_clear.delta.operations.clone(),
    )
    .unwrap();
    assert!(mtgml_rules::validate_successor_transition_contract(
        &admission,
        &first_pass.checkpoint.state,
        &missing_pool_clear,
    )
    .is_err());

    // Beginning of Combat flows to the attacker declaration boundary. This
    // admitted profile has no attacker-declaration producer, so it must fail
    // closed rather than fabricate another ordinary priority request.
    let beginning_combat_request = runtime.visible_decision(actor).unwrap().unwrap();
    let beginning_combat_first_pass = runtime
        .submit(actor, make_pass_response(&beginning_combat_request))
        .unwrap();
    assert!(beginning_combat_first_pass.transition.accepted);
    let next_actor = beginning_combat_first_pass
        .transition
        .next_decision
        .as_ref()
        .unwrap()
        .actor;
    let before_unsupported_boundary = runtime.checkpoint().unwrap();
    let replay_before_unsupported_boundary = runtime.export_replay().unwrap();
    let replay_bytes_before_unsupported_boundary =
        serde_json::to_vec(&replay_before_unsupported_boundary).unwrap();
    let next_request = runtime.visible_decision(next_actor).unwrap().unwrap();
    assert!(runtime
        .submit(next_actor, make_pass_response(&next_request))
        .is_err());
    assert_eq!(runtime.checkpoint().unwrap(), before_unsupported_boundary);
    assert_eq!(
        runtime.export_replay().unwrap(),
        replay_before_unsupported_boundary
    );
    assert_eq!(
        serde_json::to_vec(&runtime.export_replay().unwrap()).unwrap(),
        replay_bytes_before_unsupported_boundary
    );
}

fn config(players: [PlayerId; 2]) -> SyntheticRulesEnvironmentConfig {
    SyntheticRulesEnvironmentConfig {
        codec: CheckpointCodecIdentity {
            codec_id: "in-memory-reference".into(),
            semantic_version: "6".into(),
        },
        setup: mtgml_state::SyntheticV4Setup::synthetic_compatibility(),
        replay: SyntheticRulesReplayConfig {
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
            schemas: ReplaySchemaVersionsV6 {
                observation: OBSERVATION_SCHEMA.into(),
                observation_payload_codec: "synthetic-m3-observation.v1".into(),
                information_state: INFORMATION_STATE_SCHEMA_V2.into(),
                decision: "player-decision-request.v2".into(),
                decision_response: DECISION_RESPONSE_V2_SCHEMA.into(),
                observed_event: OBSERVED_EVENT_SCHEMA_V2.into(),
                player_step: PLAYER_STEP_SCHEMA_V2.into(),
                replay_step: "replay-step.v6".into(),
            },
            decks: players
                .into_iter()
                .enumerate()
                .map(|(index, player)| DeckIdentityV1 {
                    player,
                    deck_id: format!("synthetic-deck-{}", index + 1),
                    digest: ContentDigest::from_canonical_bytes(
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

fn response(candidate_id: u32, revision: u64) -> DecisionResponseV2 {
    DecisionResponseV2 {
        schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
        player_decision_id: PlayerDecisionIdV1(1),
        state_revision: StateRevision(revision),
        answer: DecisionAnswerV2::SelectOne {
            candidate_id: CandidateIdV1(candidate_id),
        },
    }
}

fn synthetic_identity() -> ExecutionIdentityV1 {
    ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::SyntheticRulesCompat,
        semantic_contract_id: synthetic_legacy_default_semantic_contract_id(),
    }
}

fn backend() -> SyntheticRulesEnvironmentBackend {
    let players = [PlayerId(1), PlayerId(2)];
    SyntheticRulesEnvironmentBackend::new(players, seed(), config(players)).unwrap()
}

fn rich_provenance_state() -> mtgml_state::EngineState {
    use mtgml_model::VisibleSequence;
    use mtgml_state::{
        KnowledgeAcquisitionCause, KnowledgeAcquisitionReason, KnowledgeHistoryChannel,
        KnowledgeInvalidationReason, KnowledgeInvalidationV2, KnownLocationFactV2,
        RetiredKnowledgeRecordV2,
    };
    let observed = |channel, sequence: u64, cause| KnowledgeAcquisitionReason::Observed {
        channel,
        sequence: VisibleSequence(sequence),
        cause,
    };
    let mut state =
        mtgml_state::construct_synthetic_engine_state(mtgml_state::SyntheticResetInputs {
            players: [PlayerId(1), PlayerId(2)],
            root_seed: seed(),
            setup: mtgml_state::SyntheticV4Setup::synthetic_compatibility(),
        })
        .unwrap();

    let identity = state
        .perspective_identities
        .players
        .get_mut(&PlayerId(1))
        .unwrap();
    identity
        .opaque_to_object
        .insert(mtgml_model::OpaqueObjectId(3), mtgml_model::GameObjectId(2));
    identity
        .object_to_opaque
        .insert(mtgml_model::GameObjectId(2), mtgml_model::OpaqueObjectId(3));
    identity.next_opaque_object_id = mtgml_model::OpaqueObjectId(4);
    identity
        .retired_object_ids
        .insert(mtgml_model::OpaqueObjectId(2));

    let knowledge = state.knowledge.players.get_mut(&PlayerId(1)).unwrap();
    let hidden_location = mtgml_state::ZoneLocation {
        zone: mtgml_model::ZoneKind::Library,
        player: Some(PlayerId(2)),
        position: mtgml_state::ZonePosition::Top { offset: 0 },
        visibility: mtgml_state::VisibilityPartition::FaceDown,
        partition: None,
    };

    // Retired record: private_look acquisition, own_private_identity history,
    // explicit_reveal invalidation.
    let mut retired = RetiredKnowledgeRecordV2 {
        opaque_object: mtgml_model::OpaqueObjectId(2),
        physical_card: None,
        card_definition: None,
        last_known_location: Some(KnownLocationFactV2 {
            location: hidden_location.clone(),
            provenance: observed(
                KnowledgeHistoryChannel::Private,
                2,
                KnowledgeAcquisitionCause::PrivateLook,
            ),
        }),
        historical_locations: vec![KnownLocationFactV2 {
            location: hidden_location.clone(),
            provenance: observed(
                KnowledgeHistoryChannel::Private,
                1,
                KnowledgeAcquisitionCause::OwnPrivateIdentity,
            ),
        }],
        acquisition: observed(
            KnowledgeHistoryChannel::Private,
            0,
            KnowledgeAcquisitionCause::PrivateLook,
        ),
        invalidation: KnowledgeInvalidationV2 {
            provenance: observed(
                KnowledgeHistoryChannel::Public,
                3,
                KnowledgeAcquisitionCause::ExplicitReveal,
            ),
            reason: KnowledgeInvalidationReason::Shuffle,
        },
    };
    retired.last_known_location = Some(KnownLocationFactV2 {
        location: hidden_location.clone(),
        provenance: observed(
            KnowledgeHistoryChannel::Private,
            2,
            KnowledgeAcquisitionCause::PrivateLook,
        ),
    });
    knowledge
        .retired
        .insert(mtgml_model::OpaqueObjectId(2), retired);
    knowledge.active.remove(&mtgml_model::OpaqueObjectId(2));
    knowledge.next_visible_sequence = VisibleSequence(4);

    // Active record with explicit_reveal current-fact provenance.
    knowledge.active.insert(
        mtgml_model::OpaqueObjectId(3),
        mtgml_state::KnowledgeRecordV2 {
            opaque_object: mtgml_model::OpaqueObjectId(3),
            physical_card: None,
            card_definition: Some(mtgml_model::CardDefinitionId(2)),
            known_location: Some(KnownLocationFactV2 {
                location: hidden_location,
                provenance: observed(
                    KnowledgeHistoryChannel::Public,
                    0,
                    KnowledgeAcquisitionCause::ExplicitReveal,
                ),
            }),
            acquisition: observed(
                KnowledgeHistoryChannel::Public,
                0,
                KnowledgeAcquisitionCause::ExplicitReveal,
            ),
            historical_locations: Vec::new(),
        },
    );
    state
}

fn expected_retained_knowledge() -> Vec<mtgml_observation::PlayerKnownObjectV1> {
    use mtgml_model::{CardDefinitionId, OpaqueObjectId, PlayerId, VisibleSequence, ZoneKind};
    use mtgml_observation::{
        PlayerKnowledgeCauseV1, PlayerKnowledgeChannelV1, PlayerKnowledgeInvalidationReasonV1,
        PlayerKnowledgeInvalidationV1, PlayerKnowledgeProvenanceV1, PlayerKnownLocationFactV1,
        PlayerKnownLocationV1, PlayerKnownObjectV1,
    };

    let initial = || PlayerKnowledgeProvenanceV1::InitialConfiguration;
    let observed = |channel, sequence, cause| PlayerKnowledgeProvenanceV1::Observed {
        channel,
        sequence: VisibleSequence(sequence),
        cause,
    };
    let battlefield = || PlayerKnownLocationV1 {
        zone: ZoneKind::Battlefield,
        player: None,
    };
    let hidden_library = || PlayerKnownLocationV1 {
        zone: ZoneKind::Library,
        player: Some(PlayerId(2)),
    };
    let fact = |location, provenance| PlayerKnownLocationFactV1 {
        location,
        provenance,
    };

    vec![
        PlayerKnownObjectV1::Active {
            opaque_object_id: OpaqueObjectId(1),
            known_definition: Some(CardDefinitionId(1)),
            current_known_location_fact: Some(fact(battlefield(), initial())),
            historical_locations: Vec::new(),
            acquisition: initial(),
        },
        PlayerKnownObjectV1::Retired {
            opaque_object_id: OpaqueObjectId(2),
            known_definition: None,
            last_known_location_fact: Some(fact(
                hidden_library(),
                observed(
                    PlayerKnowledgeChannelV1::Private,
                    2,
                    PlayerKnowledgeCauseV1::PrivateLook,
                ),
            )),
            historical_locations: vec![fact(
                hidden_library(),
                observed(
                    PlayerKnowledgeChannelV1::Private,
                    1,
                    PlayerKnowledgeCauseV1::OwnPrivateIdentity,
                ),
            )],
            acquisition: observed(
                PlayerKnowledgeChannelV1::Private,
                0,
                PlayerKnowledgeCauseV1::PrivateLook,
            ),
            invalidation: PlayerKnowledgeInvalidationV1 {
                provenance: observed(
                    PlayerKnowledgeChannelV1::Public,
                    3,
                    PlayerKnowledgeCauseV1::ExplicitReveal,
                ),
                reason: PlayerKnowledgeInvalidationReasonV1::Shuffle,
            },
        },
        PlayerKnownObjectV1::Active {
            opaque_object_id: OpaqueObjectId(3),
            known_definition: Some(CardDefinitionId(2)),
            current_known_location_fact: Some(fact(
                hidden_library(),
                observed(
                    PlayerKnowledgeChannelV1::Public,
                    0,
                    PlayerKnowledgeCauseV1::ExplicitReveal,
                ),
            )),
            historical_locations: Vec::new(),
            acquisition: observed(
                PlayerKnowledgeChannelV1::Public,
                0,
                PlayerKnowledgeCauseV1::ExplicitReveal,
            ),
        },
    ]
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

fn number_answer(value: i64) -> mtgml_decision::DecisionAnswerV2 {
    mtgml_decision::DecisionAnswerV2::ChooseNumber { value }
}

fn members_answer(ids: &[u32]) -> mtgml_decision::DecisionAnswerV2 {
    mtgml_decision::DecisionAnswerV2::SelectMany {
        candidate_ids: ids.iter().copied().map(CandidateIdV1).collect(),
    }
}

fn order_answer(ids: &[u32]) -> mtgml_decision::DecisionAnswerV2 {
    mtgml_decision::DecisionAnswerV2::Order {
        candidate_ids: ids.iter().copied().map(CandidateIdV1).collect(),
    }
}

/// Drives entry + ChooseCount(2) so the environment sits at the nonterminal
/// ChooseMembers stage of continuation C(1).
fn environment_at_members_stage() -> TrustedEnvironmentController {
    let controller = TrustedEnvironmentController::new(backend());
    let p1 = controller.bind_player(PlayerId(1)).unwrap();
    let _ = submit_answer(&p1, order_entry_answer());
    let _ = submit_answer(&p1, number_answer(2));
    controller
}

fn order_entry_answer() -> mtgml_decision::DecisionAnswerV2 {
    mtgml_decision::DecisionAnswerV2::SelectOne {
        candidate_id: CandidateIdV1(0),
    }
}

fn public_fingerprint(controller: &TrustedEnvironmentController) -> Vec<u8> {
    let checkpoint = controller.checkpoint().unwrap();
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&checkpoint.state_digest.raw_bytes());
    bytes.extend_from_slice(&checkpoint.checkpoint_digest.raw_bytes());
    bytes.extend(serde_json::to_vec(&controller.export_replay().unwrap()).unwrap());
    bytes
}

use mtgml_model::{GameObjectId, VisibleSequence};

use mtgml_rules::PredecessorTransitionResult;

use mtgml_state::{construct_synthetic_engine_state, EngineState};

fn m2e_fixture() -> EngineState {
    use mtgml_state::{GameObject, VisibilityPartition, ZoneLocation, ZonePosition};
    let mut state = construct_synthetic_engine_state(mtgml_state::SyntheticResetInputs {
        players: [PlayerId(1), PlayerId(2)],
        root_seed: seed(),
        setup: mtgml_state::SyntheticV4Setup::synthetic_compatibility(),
    })
    .unwrap();
    let exile = ZoneLocation {
        zone: mtgml_model::ZoneKind::Exile,
        player: None,
        position: ZonePosition::Unordered,
        visibility: VisibilityPartition::Public,
        partition: None,
    };
    for index in 3..=4u64 {
        let object = GameObjectId(index);
        state.zones.objects.insert(
            object,
            GameObject {
                id: object,
                physical_card: Some(mtgml_model::PhysicalCardId(index)),
                card_definition: mtgml_model::CardDefinitionId(index),
                owner: PlayerId(1),
                controller: PlayerId(1),
                tapped: false,
                face_down: false,
            },
        );
        state.zones.locations.insert(object, exile.clone());
    }
    state.allocators.next_object_id = GameObjectId(5);
    state.execution.pending_decision = None;
    state.execution.continuations.clear();
    state
}

fn battlefield_location() -> mtgml_state::ZoneLocation {
    mtgml_state::ZoneLocation {
        zone: mtgml_model::ZoneKind::Battlefield,
        player: None,
        position: mtgml_state::ZonePosition::Unordered,
        visibility: mtgml_state::VisibilityPartition::Public,
        partition: None,
    }
}

fn hidden_hand(player: PlayerId) -> mtgml_state::ZoneLocation {
    mtgml_state::ZoneLocation {
        zone: mtgml_model::ZoneKind::Hand,
        player: Some(player),
        position: mtgml_state::ZonePosition::Unordered,
        visibility: mtgml_state::VisibilityPartition::OwnerOnly,
        partition: None,
    }
}

/// Reveal GO3 to P1 (opaque 2) and then track it through an incarnation
/// change into a hidden zone. Returns the product of the single transition.
fn tracked_incarnation_product() -> Result<(EngineState, PredecessorTransitionResult), ()> {
    use mtgml_rules::fixture_support::{FixtureTransition, PlannedOccurrence};
    let before = m2e_fixture();
    let mut transition = FixtureTransition::start(&before).map_err(|_| ())?;
    let revealed = transition
        .move_object_incarnation(GameObjectId(3), battlefield_location())
        .map_err(|_| ())?;
    transition
        .apply_occurrence(PlannedOccurrence {
            lifecycle: mtgml_state::PerspectiveLifecycleAuditV1 {
                perspective: PlayerId(1),
                sequence: VisibleSequence(1),
                mutation: mtgml_state::PerspectiveLifecycleMutationV1 {
                    identity: mtgml_state::IdentityMutationV1::Allocate {
                        opaque: OpaqueObjectId(2),
                        object: revealed,
                    },
                    knowledge: Some(mtgml_state::KnowledgeMutationV1::Acquire {
                        opaque: OpaqueObjectId(2),
                        definition: Some(mtgml_model::CardDefinitionId(3)),
                        location: Some(battlefield_location()),
                        acquisition: mtgml_state::KnowledgeAcquisitionReason::Observed {
                            channel: mtgml_state::KnowledgeHistoryChannel::Public,
                            sequence: VisibleSequence(1),
                            cause: mtgml_state::KnowledgeAcquisitionCause::ExplicitReveal,
                        },
                    }),
                },
            },
            observation: mtgml_rules::PerspectiveObservationPolicyV1::Appeared {
                from_zone: mtgml_model::ZoneKind::Exile,
                to_zone: mtgml_model::ZoneKind::Battlefield,
                new_object: revealed,
            },
        })
        .map_err(|_| ())?;
    let hidden = transition
        .move_object_incarnation(revealed, hidden_hand(PlayerId(2)))
        .map_err(|_| ())?;
    transition
        .apply_occurrence(PlannedOccurrence {
            lifecycle: mtgml_state::PerspectiveLifecycleAuditV1 {
                perspective: PlayerId(1),
                sequence: VisibleSequence(2),
                mutation: mtgml_state::PerspectiveLifecycleMutationV1 {
                    identity: mtgml_state::IdentityMutationV1::Remap {
                        opaque: OpaqueObjectId(2),
                        from_object: revealed,
                        to_object: hidden,
                    },
                    knowledge: Some(mtgml_state::KnowledgeMutationV1::CurrentToHistory {
                        opaque: OpaqueObjectId(2),
                        observed_definition: Some(mtgml_model::CardDefinitionId(3)),
                    }),
                },
            },
            observation: mtgml_rules::PerspectiveObservationPolicyV1::MovedInSight {
                from_zone: mtgml_model::ZoneKind::Battlefield,
                to_zone: mtgml_model::ZoneKind::Hand,
                old_object: revealed,
                new_object: hidden,
                reveals_old: true,
                reveals_new: false,
            },
        })
        .map_err(|_| ())?;
    let result = transition.finish().map_err(|_| ())?;
    Ok((before, result))
}

fn two_perspective_outcome_product() -> (EngineState, PredecessorTransitionResult) {
    use mtgml_rules::fixture_support::{FixtureTransition, PlannedOccurrence};

    let before = m2e_fixture();
    let mut transition = FixtureTransition::start(&before).unwrap();
    for (perspective, code) in [(PlayerId(1), "p1-outcome"), (PlayerId(2), "p2-outcome")] {
        transition
            .apply_occurrence(PlannedOccurrence {
                lifecycle: mtgml_state::PerspectiveLifecycleAuditV1 {
                    perspective,
                    sequence: VisibleSequence(1),
                    mutation: mtgml_state::PerspectiveLifecycleMutationV1::default(),
                },
                observation: mtgml_rules::PerspectiveObservationPolicyV1::AnnouncedOutcome {
                    code: code.into(),
                },
            })
            .unwrap();
    }
    (before, transition.finish().unwrap())
}

#[test]
fn observed_event_v3_projection_preserves_rules_owned_audience_and_opaque_substitution() {
    let (before, result) = tracked_incarnation_product().unwrap();
    let projected = crate::lifecycle_projection::project_occurrence_envelopes_v3(
        &before,
        &result.next_state,
        &result.events,
    )
    .unwrap();
    let p1 = &projected[&PlayerId(1)];
    assert_eq!(p1.len(), 2);
    match &p1[0].event {
        mtgml_observation::ObservedEventKindV3::ObjectMoved {
            old_object: None,
            new_object: Some(OpaqueObjectId(2)),
            entering_face: None,
            tapped: None,
            ..
        } => {}
        other => panic!("unexpected V3 appearance projection {other:?}"),
    }
    match &p1[1].event {
        mtgml_observation::ObservedEventKindV3::ObjectMoved {
            old_object: Some(OpaqueObjectId(2)),
            new_object: None,
            ..
        } => {}
        other => panic!("unexpected V3 disappearance projection {other:?}"),
    }

    let (before, result) = two_perspective_outcome_product();
    let audience = crate::lifecycle_projection::project_occurrence_envelopes_v3(
        &before,
        &result.next_state,
        &result.events,
    )
    .unwrap();
    for (player, code) in [(PlayerId(1), "p1-outcome"), (PlayerId(2), "p2-outcome")] {
        assert!(matches!(
            &audience[&player][0].event,
            mtgml_observation::ObservedEventKindV3::PublicOutcome { code: actual }
                if actual == code
        ));
    }
    let mut without_other_perspective = result.events.clone();
    without_other_perspective.retain(|event| {
        !matches!(
            &event.event,
            mtgml_rules::AuthoritativeRuleEventKind::PerspectiveOccurrence { lifecycle, .. }
                if lifecycle.perspective == PlayerId(2)
        )
    });
    let mut paired_after = result.next_state.clone();
    paired_after
        .knowledge
        .players
        .get_mut(&PlayerId(2))
        .unwrap()
        .next_visible_sequence = before.knowledge.players[&PlayerId(2)].next_visible_sequence;
    let paired = crate::lifecycle_projection::project_occurrence_envelopes_v3(
        &before,
        &paired_after,
        &without_other_perspective,
    )
    .unwrap();
    assert_eq!(
        serde_json::to_vec(&audience[&PlayerId(1)]).unwrap(),
        serde_json::to_vec(&paired[&PlayerId(1)]).unwrap(),
        "another perspective's unauthorized occurrence changed P1 event bytes"
    );
}

#[test]
fn authorized_v3_battlefield_entry_projects_face_and_tapped_in_one_move_event() {
    let (before, result) = tracked_incarnation_product().unwrap();
    let entry_facts = BTreeMap::from([(
        (PlayerId(1), VisibleSequence(1)),
        crate::lifecycle_projection::AuthorizedBattlefieldEntryFactsV3 {
            entering_face: Some(mtgml_observation::ObservedFaceV1::Back),
            tapped: Some(true),
        },
    )]);
    let projected = crate::lifecycle_projection::project_occurrence_envelopes_v3_with_entry_facts(
        &before,
        &result.next_state,
        &result.events,
        &entry_facts,
    )
    .unwrap();
    let p1 = &projected[&PlayerId(1)];
    assert_eq!(
        p1.len(),
        2,
        "entry facts must not synthesize a second event"
    );
    assert!(matches!(
        &p1[0].event,
        mtgml_observation::ObservedEventKindV3::ObjectMoved {
            old_object: None,
            new_object: Some(_),
            to: mtgml_model::ZoneKind::Battlefield,
            entering_face: Some(mtgml_observation::ObservedFaceV1::Back),
            tapped: Some(true),
            ..
        }
    ));
    assert!(!p1.iter().any(|envelope| matches!(
        envelope.event,
        mtgml_observation::ObservedEventKindV3::ObjectFaceChanged { .. }
    )));
}

#[test]
fn global_hidden_allocator_history_cannot_move_opaque_assignment() {
    use mtgml_rules::fixture_support::{FixtureTransition, PlannedOccurrence};
    let base = m2e_fixture();
    let mut variant = base.clone();
    // Hidden global allocation history differs wildly between the pair,
    // including the risky global OBJECT allocator itself.
    variant.allocators.next_effect_id = mtgml_model::EffectInstanceId(900);
    variant.allocators.next_trigger_id = mtgml_model::TriggerInstanceId(700);
    variant.allocators.next_object_id = GameObjectId(500);

    let mut previous: Option<Vec<u8>> = None;
    let mut collected_seen: Vec<u32> = Vec::new();
    for state in [base, variant] {
        let mut transition = FixtureTransition::start(&state).unwrap();
        let seen = transition
            .move_object_incarnation(GameObjectId(3), battlefield_location())
            .unwrap();
        transition
            .apply_occurrence(PlannedOccurrence {
                lifecycle: mtgml_state::PerspectiveLifecycleAuditV1 {
                    perspective: PlayerId(1),
                    sequence: VisibleSequence(1),
                    mutation: mtgml_state::PerspectiveLifecycleMutationV1 {
                        identity: mtgml_state::IdentityMutationV1::Allocate {
                            opaque: OpaqueObjectId(2),
                            object: seen,
                        },
                        knowledge: Some(mtgml_state::KnowledgeMutationV1::Acquire {
                            opaque: OpaqueObjectId(2),
                            definition: None,
                            location: Some(battlefield_location()),
                            acquisition: mtgml_state::KnowledgeAcquisitionReason::Observed {
                                channel: mtgml_state::KnowledgeHistoryChannel::Public,
                                sequence: VisibleSequence(1),
                                cause: mtgml_state::KnowledgeAcquisitionCause::ExplicitReveal,
                            },
                        }),
                    },
                },
                observation: mtgml_rules::PerspectiveObservationPolicyV1::Appeared {
                    from_zone: mtgml_model::ZoneKind::Exile,
                    to_zone: mtgml_model::ZoneKind::Battlefield,
                    new_object: seen,
                },
            })
            .unwrap();
        let result = transition.finish().unwrap();
        let identity = &result.next_state.perspective_identities.players[&PlayerId(1)];
        assert_eq!(
            identity.opaque_to_object.get(&OpaqueObjectId(2)),
            Some(&GameObjectId(seen.0))
        );
        collected_seen.push(u32::try_from(seen.0).unwrap());
        let knowledge_bytes =
            serde_json::to_vec(&result.next_state.knowledge.players[&PlayerId(1)]).unwrap();
        if let Some(previous_bytes) = previous.as_ref() {
            assert_eq!(previous_bytes, &knowledge_bytes);
        }
        previous = Some(knowledge_bytes);
    }
    // The trusted incarnations must differ (hidden object-allocator history)
    assert_ne!(collected_seen[0], collected_seen[1]);
}

// Lexical fragments: physical discoverability without changing any
// tests::<name> identity addressed by the M1/M2 gate runners.
include!("tests/forced_progress.rs");
include!("tests/checkpoint_replay.rs");
include!("tests/player_endpoint.rs");
include!("tests/continuation.rs");
include!("tests/information_projection.rs");
include!("tests/error_nonmutation.rs");
mod response_transaction {
    use super::*;
    include!("tests/response_transaction.rs");
}
include!("tests/batch_d.rs");
include!("tests/batch_e.rs");
include!("tests/batch_f.rs");
include!("tests/batch_g.rs");
include!("tests/turn_structure.rs");
mod magic_observation {
    use super::*;
    include!("tests/magic_observation.rs");
}

mod semantic_catalog {
    #![allow(unused_imports)]
    use super::*;
    use crate::semantic_catalog_generated::synthetic_legacy_default_semantic_contract_id;
    include!("tests/semantic_catalog.rs");
}

mod restore_admission {
    use super::*;
    use crate::semantic_catalog_generated::synthetic_legacy_default_semantic_contract_id;
    include!("tests/restore_admission.rs");
}

mod magic_rules_production {
    use super::*;
    include!("tests/magic_rules_production.rs");
}
