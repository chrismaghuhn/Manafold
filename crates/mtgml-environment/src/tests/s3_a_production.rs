use crate::{EnvironmentBackend, PlayerEndpoint};
use crate::checkpoint::{
    CHECKPOINT_CODEC_ID_V6, CHECKPOINT_CODEC_SEMANTIC_VERSION_V6,
};
use mtgml_decision::{DecisionAnswerV2, DecisionResponseV2};
use mtgml_model::{
    CardDefinitionId, CheckpointCodecIdentity, EpisodeStatus, GameObjectId, OpaqueObjectId,
    PhysicalCardId, PlayerId,
};
use mtgml_observation::{MagicM3Observation, PlayerStepSubmissionV1};
use mtgml_replay::{KernelIdentityV1, ReplaySchemaVersionsV6};
use mtgml_state::{
    BaseCharacteristics, ContinuationPayloadV2, ControlHistory, FoundationCreatureSource, FoundationSourceKind,
    GameObject, KnowledgeAcquisitionReason, KnowledgeRecordV2, KnownLocationFactV2,
    SbaObjectCauseV1, SbaSelectedActionV1, TurnPosition, VisibilityPartition, ZoneLocation,
    ZonePosition,
};

const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);

fn s3_replay_config() -> ReferenceEnvironmentReplayConfig {
    ReferenceEnvironmentReplayConfig {
        scenario_id: "rules/state-based-actions-combat@0.1.0:ordered-sba".into(),
        engine_build: "m3-block-1-test".into(),
        kernel: KernelIdentityV1 {
            implementation_id: "magic-reference".into(),
            semantic_version: "0.2.2".into(),
            build_profile: "test".into(),
        },
        rules_snapshot: "wotc-cr-2026-08-07-txt-20260819-sha256-4381ad1b39ab2c05f7d03633a20f711ed37277074d3266dcba5f38cbb527423f".into(),
        format_policy_snapshot: "format:none".into(),
        oracle_snapshot: "oracle:none".into(),
        schemas: ReplaySchemaVersionsV6 {
            observation: mtgml_observation::OBSERVATION_SCHEMA.into(),
            observation_payload_codec: mtgml_observation::MAGIC_M3_OBSERVATION_SCHEMA.into(),
            information_state: mtgml_observation::INFORMATION_STATE_SCHEMA_V2.into(),
            decision: mtgml_decision::PLAYER_DECISION_REQUEST_V2_SCHEMA.into(),
            decision_response: mtgml_decision::DECISION_RESPONSE_V2_SCHEMA.into(),
            observed_event: mtgml_observation::OBSERVED_EVENT_SCHEMA_V2.into(),
            player_step: mtgml_observation::PLAYER_STEP_SCHEMA_V2.into(),
            replay_step: mtgml_replay::REPLAY_STEP_SCHEMA_V6.into(),
        },
    }
}

fn s3_backend(state: mtgml_state::EngineState) -> ReferenceEnvironmentBackend {
    ReferenceEnvironmentBackend::new(ReferenceEnvironmentConfig {
        state,
        status: EpisodeStatus::Running,
        limit_counters: EnvironmentLimitCounters::default(),
        codec: CheckpointCodecIdentity {
            codec_id: CHECKPOINT_CODEC_ID_V6.into(),
            semantic_version: CHECKPOINT_CODEC_SEMANTIC_VERSION_V6.into(),
        },
        execution_identity: ReferenceEnvironmentBackend::magic_s3_a_execution_identity(),
        replay: s3_replay_config(),
    })
    .expect("the production S3.A semantic identity admits its bounded state")
}

fn s3_b_backend(state: mtgml_state::EngineState) -> ReferenceEnvironmentBackend {
    ReferenceEnvironmentBackend::new(ReferenceEnvironmentConfig {
        state,
        status: EpisodeStatus::Running,
        limit_counters: EnvironmentLimitCounters::default(),
        codec: CheckpointCodecIdentity {
            codec_id: CHECKPOINT_CODEC_ID_V6.into(),
            semantic_version: CHECKPOINT_CODEC_SEMANTIC_VERSION_V6.into(),
        },
        execution_identity: ReferenceEnvironmentBackend::magic_s3_b_execution_identity(),
        replay: s3_replay_config(),
    })
    .expect("the production S3.B semantic identity admits its bounded state")
}

fn add_creature(state: &mut mtgml_state::EngineState, object_id: u64, owner: PlayerId, opaque: [u64; 2]) {
    let object = GameObjectId(object_id);
    let location = ZoneLocation {
        zone: mtgml_model::ZoneKind::Battlefield,
        player: None,
        position: ZonePosition::Unordered,
        visibility: VisibilityPartition::Public,
        partition: None,
    };
    state.zones.objects.insert(
        object,
        GameObject {
            id: object,
            physical_card: Some(PhysicalCardId(object_id)),
            card_definition: CardDefinitionId(object_id),
            owner,
            controller: owner,
            tapped: false,
            face_down: false,
        },
    );
    state.zones.locations.insert(object, location.clone());
    state.foundation_sources.insert(
        object,
        FoundationCreatureSource {
            source_kind: FoundationSourceKind::Creature,
            base_characteristics: BaseCharacteristics::Simple {
                power: 2,
                toughness: 0,
            },
            marked_damage: 0,
            control_history: ControlHistory::BeforeTurnStart { turn_number: 1 },
        },
    );
    for (player, opaque_id) in [(P1, opaque[0]), (P2, opaque[1])] {
        let opaque = OpaqueObjectId(opaque_id);
        let identity = state.perspective_identities.players.get_mut(&player).unwrap();
        identity.opaque_to_object.insert(opaque, object);
        identity.object_to_opaque.insert(object, opaque);
        identity.next_opaque_object_id.0 = identity.next_opaque_object_id.0.max(opaque_id + 1);
        state.knowledge.players.get_mut(&player).unwrap().active.insert(
            opaque,
            KnowledgeRecordV2 {
                opaque_object: opaque,
                physical_card: Some(PhysicalCardId(object_id)),
                card_definition: Some(CardDefinitionId(object_id)),
                known_location: Some(KnownLocationFactV2 {
                    location: location.clone(),
                    provenance: KnowledgeAcquisitionReason::InitialConfiguration,
                }),
                acquisition: KnowledgeAcquisitionReason::InitialConfiguration,
                historical_locations: Vec::new(),
            },
        );
    }
}

fn two_owner_stage_zero() -> mtgml_state::EngineState {
    let mut state = super::restore_admission::magic_sba_continuation_state();
    // Add two selected deaths for P2, preserving the authoritative P1 Order
    // request as APNAP stage zero.
    add_creature(&mut state, 4, P2, [3, 4]);
    add_creature(&mut state, 5, P2, [4, 5]);
    state.allocators.next_object_id = GameObjectId(6);
    state
        .perspective_identities
        .players
        .get_mut(&P1)
        .unwrap()
        .next_opaque_object_id = OpaqueObjectId(5);
    state
        .perspective_identities
        .players
        .get_mut(&P2)
        .unwrap()
        .next_opaque_object_id = OpaqueObjectId(6);
    let record = state.execution.continuations.get_mut(&mtgml_model::ContinuationId(1)).unwrap();
    let ContinuationPayloadV2::MagicSbaGraveyardOrderV1 {
        selected_sba_actions,
        apnap_owners,
        ..
    } = &mut record.payload
    else {
        unreachable!()
    };
    selected_sba_actions.extend([
        SbaSelectedActionV1::ObjectToOwnerGraveyard {
            object: GameObjectId(4),
            causes: vec![SbaObjectCauseV1::ZeroToughness],
        },
        SbaSelectedActionV1::ObjectToOwnerGraveyard {
            object: GameObjectId(5),
            causes: vec![SbaObjectCauseV1::ZeroToughness],
        },
    ]);
    *apnap_owners = vec![P1, P2];
    mtgml_state::validate_engine_state(&state).unwrap();
    state
}

fn no_order_one_death_state() -> mtgml_state::EngineState {
    let mut state = super::restore_admission::magic_sba_continuation_state();
    state.execution.pending_decision = None;
    state.execution.continuations.clear();
    if let Some(source) = state.foundation_sources.get_mut(&GameObjectId(3)) {
        source.base_characteristics = BaseCharacteristics::Simple {
            power: 2,
            toughness: 2,
        };
    }
    mtgml_state::validate_engine_state(&state).unwrap();
    state
}

fn stable_state_at(position: mtgml_state::TurnPosition) -> mtgml_state::EngineState {
    let mut state = no_order_one_death_state();
    state.core.position = position;
    for source in state.foundation_sources.values_mut() {
        source.base_characteristics = BaseCharacteristics::Simple {
            power: 2,
            toughness: 2,
        };
    }
    mtgml_state::validate_engine_state(&state).unwrap();
    state
}

fn current_order_response(state: &mtgml_state::EngineState) -> DecisionResponseV2 {
    let request = &state.execution.pending_decision.as_ref().unwrap().request;
    let mut candidate_ids: Vec<_> = request.candidates.iter().map(|c| c.candidate_id).collect();
    candidate_ids.reverse();
    DecisionResponseV2 {
        schema_version: mtgml_decision::DECISION_RESPONSE_V2_SCHEMA.into(),
        player_decision_id: request.player_decision_id,
        state_revision: request.state_revision,
        answer: DecisionAnswerV2::Order { candidate_ids },
    }
}

fn current_select_one_response(
    request: &mtgml_decision::PlayerDecisionRequestV2,
) -> DecisionResponseV2 {
    DecisionResponseV2 {
        schema_version: mtgml_decision::DECISION_RESPONSE_V2_SCHEMA.into(),
        player_decision_id: request.player_decision_id,
        state_revision: request.state_revision,
        answer: DecisionAnswerV2::SelectOne {
            candidate_id: request.candidates[0].candidate_id,
        },
    }
}

fn decode_magic_observation(
    envelope: mtgml_observation::ObservationEnvelope,
) -> MagicM3Observation {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(envelope.payload_base64)
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

fn player_fingerprint(controller: &TrustedEnvironmentController, player: PlayerId) -> Vec<u8> {
    let endpoint = controller.bind_player(player).unwrap();
    let mut bytes = mtgml_wire::encode_canonical(&endpoint.observation().unwrap()).unwrap();
    bytes.extend(
        mtgml_wire::encode_canonical(&endpoint.information_state().unwrap()).unwrap(),
    );
    match endpoint.visible_decision().unwrap() {
        Some(decision) => {
            bytes.push(1);
            bytes.extend(mtgml_wire::encode_canonical(&decision).unwrap());
        }
        None => bytes.push(0),
    }
    bytes
}

#[test]
fn production_s3_identity_restores_pending_order_stages_and_projects_magic_codec() {
    let backend = s3_backend(two_owner_stage_zero());
    let stage_zero = backend.checkpoint().unwrap();
    assert_eq!(
        stage_zero.execution_identity,
        ReferenceEnvironmentBackend::magic_s3_a_execution_identity()
    );
    let mut restored_stage_zero = s3_backend(stage_zero.state.clone());
    restored_stage_zero.restore(stage_zero.clone()).unwrap();
    assert_eq!(restored_stage_zero.checkpoint().unwrap(), stage_zero);
    assert_eq!(
        backend.player_observation(P1).unwrap().payload_codec,
        mtgml_observation::MAGIC_M3_OBSERVATION_SCHEMA
    );
    assert_eq!(
        decode_magic_observation(backend.player_observation(P1).unwrap())
            .pending_sba_ordering
            .unwrap()
            .next_order_owner,
        P1
    );
    let forked = backend.fork_boxed().unwrap();
    assert_eq!(forked.checkpoint().unwrap(), stage_zero);

    let controller = TrustedEnvironmentController::new(backend);
    let endpoint = controller.bind_player(P1).unwrap();
    let accepted = endpoint.submit(current_order_response(&stage_zero.state)).unwrap();
    assert!(matches!(accepted.submission, PlayerStepSubmissionV1::Accepted));
    let stage_one = controller.checkpoint().unwrap();
    let mut restored_stage_one = s3_backend(stage_one.state.clone());
    restored_stage_one.restore(stage_one.clone()).unwrap();
    assert_eq!(restored_stage_one.checkpoint().unwrap(), stage_one);
    let s1_p1 = decode_magic_observation(
        controller.bind_player(P1).unwrap().observation().unwrap(),
    );
    assert_eq!(s1_p1.pending_sba_ordering.as_ref().unwrap().next_order_owner, P2);
    assert_eq!(s1_p1.pending_sba_ordering.as_ref().unwrap().completed_orders.len(), 1);
    assert!(controller.bind_player(P1).unwrap().visible_decision().unwrap().is_none());
    let p2_observation = controller.bind_player(P2).unwrap().observation().unwrap();
    assert_eq!(
        p2_observation.payload_codec,
        mtgml_observation::MAGIC_M3_OBSERVATION_SCHEMA
    );
    let s1_p2 = decode_magic_observation(p2_observation);
    assert_eq!(s1_p2.pending_sba_ordering.as_ref().unwrap().next_order_owner, P2);
    assert_eq!(s1_p2.pending_sba_ordering.as_ref().unwrap().completed_orders.len(), 1);
    assert_ne!(
        s1_p1.pending_sba_ordering.as_ref().unwrap().completed_orders[0].ordered_objects,
        s1_p2.pending_sba_ordering.as_ref().unwrap().completed_orders[0].ordered_objects,
        "each perspective sees the public order only through local opaque identities"
    );
    assert!(controller.bind_player(P2).unwrap().visible_decision().unwrap().is_some());

    let replay = controller.export_replay().unwrap();
    assert_eq!(replay.steps.len(), 1);
    let report = controller
        .execute_replay_from_checkpoint(stage_zero, replay)
        .unwrap();
    assert_eq!(report.final_checkpoint, stage_one);
    let fork = controller.fork().unwrap();
    assert_eq!(fork.checkpoint().unwrap(), stage_one);
}

#[test]
fn production_s3_terminal_final_order_is_replayed_and_nonterminal_final_order_is_blocked_atomically() {
    let mut terminal_state = super::restore_admission::magic_sba_continuation_state();
    terminal_state.core.players.get_mut(&P1).unwrap().life = 0;
    if let ContinuationPayloadV2::MagicSbaGraveyardOrderV1 {
        selected_sba_actions,
        ..
    } = &mut terminal_state
        .execution
        .continuations
        .get_mut(&mtgml_model::ContinuationId(1))
        .unwrap()
        .payload
    {
        selected_sba_actions.insert(0, SbaSelectedActionV1::PlayerLoses { player: P1 });
    }
    let terminal_backend = s3_backend(terminal_state);
    let before_final = terminal_backend.checkpoint().unwrap();
    let controller = TrustedEnvironmentController::new(terminal_backend);
    let endpoint = controller.bind_player(P1).unwrap();
    let terminal_step = endpoint
        .submit(current_order_response(&before_final.state))
        .unwrap();
    assert!(matches!(terminal_step.status, EpisodeStatus::Terminal { .. }));
    let terminal_checkpoint = controller.checkpoint().unwrap();
    assert!(matches!(terminal_checkpoint.status, EpisodeStatus::Terminal { .. }));
    let terminal_fork = controller.fork().unwrap();
    assert_eq!(terminal_fork.checkpoint().unwrap(), terminal_checkpoint);
    let replay = controller.export_replay().unwrap();
    assert_eq!(replay.steps.len(), 1);
    let report = controller
        .execute_replay_from_checkpoint(before_final, replay)
        .unwrap();
    assert_eq!(report.final_checkpoint, terminal_checkpoint);
    controller.restore(terminal_checkpoint.clone()).unwrap();
    assert_eq!(controller.checkpoint().unwrap(), terminal_checkpoint);

    let nonterminal_backend = s3_backend(
        super::restore_admission::magic_sba_continuation_state(),
    );
    let before = nonterminal_backend.checkpoint().unwrap();
    let replay_before = nonterminal_backend.export_replay().unwrap();
    let controller = TrustedEnvironmentController::new(nonterminal_backend);
    let endpoint = controller.bind_player(P1).unwrap();
    let p1_before = player_fingerprint(&controller, P1);
    let p2_before = player_fingerprint(&controller, P2);
    assert!(endpoint
        .submit(current_order_response(&before.state))
        .is_err());
    assert_eq!(controller.checkpoint().unwrap(), before);
    assert_eq!(controller.export_replay().unwrap(), replay_before);
    assert_eq!(player_fingerprint(&controller, P1), p1_before);
    assert_eq!(player_fingerprint(&controller, P2), p2_before);
}

#[test]
fn production_s3_no_order_batch_restores_and_forks_at_the_stable_boundary() {
    let mut backend = s3_backend(no_order_one_death_state());
    let transition = backend
        .execute_forced_progress()
        .expect("no-order S3.A forced progress applies one complete batch");
    assert!(transition.accepted);
    assert!(transition.next_decision.is_none());
    assert_eq!(
        transition
            .events
            .iter()
            .filter(|event| matches!(
                event.event,
                mtgml_rules::AuthoritativeRuleEventKind::StateBasedActionsApplied { .. }
            ))
            .count(),
        1
    );
    let checkpoint = backend.checkpoint().unwrap();
    assert!(matches!(checkpoint.status, EpisodeStatus::Running));
    assert!(checkpoint.state.execution.pending_decision.is_none());
    assert!(checkpoint.state.execution.continuations.is_empty());
    let replay = backend.export_replay().unwrap();
    assert!(replay.steps.is_empty(), "standalone forced progress has no fake replay input");
    let fork = backend.fork_boxed().unwrap();
    assert_eq!(fork.checkpoint().unwrap(), checkpoint);

    let mut restored = s3_backend(checkpoint.state.clone());
    restored.restore(checkpoint.clone()).unwrap();
    assert_eq!(restored.checkpoint().unwrap(), checkpoint);
}

#[test]
fn production_s3_b_final_order_then_priority_is_one_r_plus_two_replay_commit() {
    let initial_state = super::restore_admission::magic_sba_continuation_state();
    let initial = s3_b_backend(initial_state).checkpoint().unwrap();
    let response = current_order_response(&initial.state);
    let mut kernel = mtgml_rules::ProgramKernelV1::for_admitted_execution(
        mtgml_model::ExecutionProgramV1::MagicRules,
        crate::semantic_catalog_generated::magic_s3_b_basic_priority_0_1_0_semantic_contract_id(),
    )
    .unwrap();
    let final_batch = kernel.apply(&initial.state, P1, &response).unwrap();
    assert_eq!(final_batch.next_state.revision.0, initial.state.revision.0 + 1);
    assert!(final_batch
        .events
        .iter()
        .all(|event| event.state_revision.0 == initial.state.revision.0 + 1));
    let opened = kernel.advance_forced_progress(&final_batch.next_state).unwrap();
    assert_eq!(opened.next_state.revision.0, final_batch.next_state.revision.0 + 1);
    assert!(opened
        .events
        .iter()
        .all(|event| event.state_revision.0 == initial.state.revision.0 + 2));
    let mut merged_events = final_batch.events.clone();
    merged_events.extend(opened.events.clone());
    let merged_audit = merged_events
        .iter()
        .map(|event| event.event.semantic_delta())
        .collect();
    let merged = mtgml_rules::TransitionResult {
        accepted: true,
        next_state: opened.next_state.clone(),
        delta: mtgml_state::StateDelta::between(&initial.state, &opened.next_state, merged_audit)
            .unwrap(),
        events: merged_events,
        next_decision: opened.next_decision.clone(),
        status: opened.status.clone(),
    };
    mtgml_rules::validate_transition_contract(&initial.state, &merged).unwrap();
    let mut direct_backend = s3_b_backend(initial.state.clone());
    crate::controller::EnvironmentBackend::execute_trusted_response(
        &mut direct_backend,
        P1,
        response.clone(),
    )
    .expect("shared response transaction must commit the two-revision composition");
    let direct_checkpoint = direct_backend.checkpoint().unwrap();
    let projected_step = crate::player_projection::project_player_step_with_profile(
        &direct_checkpoint.state,
        P1,
        direct_checkpoint.status.clone(),
        PlayerStepSubmissionV1::Accepted,
        crate::player_projection::ObservationProjectionProfile::MagicM3,
    )
    .expect("S3.B candidate PlayerStep projection");
    projected_step.validate().expect("S3.B PlayerStep validation");
    let backend = s3_b_backend(initial.state.clone());
    let controller = TrustedEnvironmentController::new(backend);
    let p1 = controller.bind_player(P1).unwrap();
    assert!(p1.visible_decision().unwrap().is_some());
    assert!(controller.bind_player(P2).unwrap().visible_decision().unwrap().is_none());

    let accepted = p1.submit(response).unwrap();
    assert!(matches!(accepted.submission, PlayerStepSubmissionV1::Accepted));
    let final_checkpoint = controller.checkpoint().unwrap();
    assert_eq!(final_checkpoint.state.revision.0, initial.state.revision.0 + 2);
    assert_eq!(
        final_checkpoint.state.core.priority,
        mtgml_state::PriorityState::HeldBy {
            player: final_checkpoint.state.core.active_player,
            consecutive_passes: 0
        }
    );
    let request = final_checkpoint.state.execution.pending_decision.as_ref().unwrap();
    assert_eq!(request.request.actor, P1);
    assert_eq!(request.request.candidates.len(), 1);
    assert!(p1.visible_decision().unwrap().is_some());
    assert!(controller.bind_player(P2).unwrap().visible_decision().unwrap().is_none());

    let replay = controller.export_replay().unwrap();
    assert_eq!(replay.steps.len(), 1);
    assert_eq!(replay.steps[0].state_revision_before, initial.state.revision);
    assert_eq!(replay.steps[0].state_revision_after, final_checkpoint.state.revision);
    let report = controller
        .execute_replay_from_checkpoint(initial, replay)
        .unwrap();
    assert_eq!(report.final_checkpoint, final_checkpoint);
}

#[test]
fn production_s3_b_no_order_sba_finishes_before_opening_priority_in_one_forced_product() {
    let state = no_order_one_death_state();
    let expected_revision = state.revision.0 + 1;
    let mut backend = s3_b_backend(state);
    let transition = backend
        .execute_forced_progress()
        .expect("one no-order SBA round must reach the pass-only window");
    assert!(transition.accepted);
    assert_eq!(transition.next_state.revision.0, expected_revision);
    assert_eq!(
        transition.next_state.core.priority,
        mtgml_state::PriorityState::HeldBy {
            player: P1,
            consecutive_passes: 0
        }
    );
    assert!(matches!(
        transition.events[0].event,
        mtgml_rules::AuthoritativeRuleEventKind::StateBasedActionsApplied { .. }
    ));
    assert!(matches!(
        transition.events[transition.events.len() - 2].event,
        mtgml_rules::AuthoritativeRuleEventKind::PriorityChanged { .. }
    ));
    assert_eq!(backend.player_visible_decision(P1).unwrap().unwrap().actor, P1);
    assert!(backend.player_visible_decision(P2).unwrap().is_none());
    assert!(backend.export_replay().unwrap().steps.is_empty());
    let checkpoint = backend.checkpoint().unwrap();
    let fork = backend.fork_boxed().unwrap();
    assert_eq!(fork.checkpoint().unwrap(), checkpoint);
    let mut restored = s3_b_backend(checkpoint.state.clone());
    restored.restore(checkpoint.clone()).unwrap();
    assert_eq!(restored.checkpoint().unwrap(), checkpoint);
}

#[test]
fn production_s3_b_restore_admission_is_distinct_from_s1_and_s3_a() {
    let mut backend = s3_b_backend(no_order_one_death_state());
    backend.execute_forced_progress().unwrap();
    let valid = backend.checkpoint().unwrap();
    let catalog = crate::semantic_catalog::RuntimeSemanticCatalog::production();
    crate::semantic_catalog::admit_restore(&catalog, &valid).unwrap();

    let mut wrong_codec = s3_replay_config();
    wrong_codec.schemas.observation_payload_codec = "synthetic-m3-observation.v1".into();
    assert!(ReferenceEnvironmentBackend::new(ReferenceEnvironmentConfig {
        state: valid.state.clone(),
        status: valid.status.clone(),
        limit_counters: valid.limit_counters.clone(),
        codec: valid.codec.clone(),
        execution_identity: valid.execution_identity.clone(),
        replay: wrong_codec,
    })
    .is_err());

    for identity in [
        ReferenceEnvironmentBackend::magic_execution_identity(),
        ReferenceEnvironmentBackend::magic_s3_a_execution_identity(),
    ] {
        let wrong = crate::EnvironmentCheckpointV6::new(
            valid.state.clone(),
            valid.status.clone(),
            valid.limit_counters.clone(),
            valid.codec.clone(),
            identity,
        )
        .unwrap();
        assert!(crate::semantic_catalog::admit_restore(&catalog, &wrong).is_err());
    }

    let mut unsupported = valid.state.clone();
    unsupported.format = mtgml_state::FormatState::Commander {
        state: mtgml_state::CommanderState {
            designations: Default::default(),
            cast_counts: Default::default(),
            damage: Default::default(),
        },
    };
    let unsupported = crate::EnvironmentCheckpointV6::new(
        unsupported,
        valid.status.clone(),
        valid.limit_counters.clone(),
        valid.codec.clone(),
        valid.execution_identity.clone(),
    )
    .unwrap();
    assert!(crate::semantic_catalog::admit_restore(&catalog, &unsupported).is_err());
}

#[test]
fn production_s3_b_active_priority_and_after_first_pass_restore_fork_and_replay() {
    let mut backend = s3_b_backend(stable_state_at(TurnPosition::Beginning {
        step: mtgml_state::BeginningStep::Upkeep,
    }));
    backend.execute_forced_progress().unwrap();
    let stage_zero = backend.checkpoint().unwrap();
    let p1_request = backend.player_visible_decision(P1).unwrap().unwrap();
    assert!(backend.player_visible_decision(P2).unwrap().is_none());
    assert_eq!(p1_request.actor, P1);

    let mut restored_stage_zero = s3_b_backend(stage_zero.state.clone());
    restored_stage_zero.restore(stage_zero.clone()).unwrap();
    assert_eq!(restored_stage_zero.checkpoint().unwrap(), stage_zero);
    assert_eq!(restored_stage_zero.player_visible_decision(P1).unwrap(), Some(p1_request));
    assert!(restored_stage_zero.player_visible_decision(P2).unwrap().is_none());
    assert_eq!(backend.fork_boxed().unwrap().checkpoint().unwrap(), stage_zero);

    let controller = TrustedEnvironmentController::new(backend);
    let visible = controller.bind_player(P1).unwrap().visible_decision().unwrap().unwrap();
    let accepted = controller
        .bind_player(P1)
        .unwrap()
        .submit(current_select_one_response(&visible))
        .unwrap();
    assert!(matches!(accepted.submission, PlayerStepSubmissionV1::Accepted));
    let stage_one = controller.checkpoint().unwrap();
    assert_eq!(
        stage_one.state.core.priority,
        mtgml_state::PriorityState::HeldBy {
            player: P2,
            consecutive_passes: 1
        }
    );
    assert!(controller.bind_player(P1).unwrap().visible_decision().unwrap().is_none());
    let p2_decision = controller.bind_player(P2).unwrap().visible_decision().unwrap().unwrap();
    assert_eq!(p2_decision.actor, P2);
    let mut restored_stage_one = s3_b_backend(stage_one.state.clone());
    restored_stage_one.restore(stage_one.clone()).unwrap();
    assert_eq!(restored_stage_one.checkpoint().unwrap(), stage_one);
    assert_eq!(restored_stage_one.player_visible_decision(P2).unwrap(), Some(p2_decision));
    assert_eq!(controller.fork().unwrap().checkpoint().unwrap(), stage_one);

    let replay = controller.export_replay().unwrap();
    assert_eq!(replay.steps.len(), 1);
    let report = controller
        .execute_replay_from_checkpoint(stage_zero, replay)
        .unwrap();
    assert_eq!(report.final_checkpoint, stage_one);
}

#[test]
fn production_s3_b_end_step_two_pass_reference_replay_closes_through_cleanup() {
    let mut backend = s3_b_backend(stable_state_at(TurnPosition::Ending {
        step: mtgml_state::EndingStep::EndStep,
    }));
    backend.execute_forced_progress().unwrap();
    let initial = backend.checkpoint().unwrap();
    let controller = TrustedEnvironmentController::new(backend);

    for actor in [P1, P2] {
        let endpoint = controller.bind_player(actor).unwrap();
        let request = endpoint.visible_decision().unwrap().unwrap();
        assert_eq!(request.actor, actor);
        let accepted = endpoint
            .submit(current_select_one_response(&request))
            .unwrap_or_else(|error| panic!("priority submit for {actor:?}: {error:?}"));
        assert!(matches!(accepted.submission, PlayerStepSubmissionV1::Accepted));
    }
    let final_checkpoint = controller.checkpoint().unwrap();
    assert_eq!(
        final_checkpoint.state.core.position,
        TurnPosition::Beginning {
            step: mtgml_state::BeginningStep::Untap
        }
    );
    assert_eq!(final_checkpoint.state.core.priority, mtgml_state::PriorityState::None);
    assert!(final_checkpoint.state.execution.pending_decision.is_none());
    let replay = controller.export_replay().unwrap();
    assert_eq!(replay.steps.len(), 2);
    let report = controller
        .execute_replay_from_checkpoint(initial, replay)
        .unwrap();
    assert_eq!(report.final_checkpoint, final_checkpoint);
}

#[test]
fn production_s3_b_pass_window_noninterference_hides_opponent_library_definition() {
    let first = stable_state_at(TurnPosition::Beginning {
        step: mtgml_state::BeginningStep::Upkeep,
    });
    let mut second = first.clone();
    second.zones.objects.get_mut(&GameObjectId(2)).unwrap().card_definition = CardDefinitionId(999);
    second
        .knowledge
        .players
        .get_mut(&P2)
        .unwrap()
        .active
        .get_mut(&OpaqueObjectId(2))
        .unwrap()
        .card_definition = Some(CardDefinitionId(999));
    mtgml_state::validate_engine_state(&second).unwrap();

    let mut first_backend = s3_b_backend(first);
    let mut second_backend = s3_b_backend(second);
    first_backend.execute_forced_progress().unwrap();
    second_backend.execute_forced_progress().unwrap();
    let first_controller = TrustedEnvironmentController::new(first_backend);
    let second_controller = TrustedEnvironmentController::new(second_backend);

    assert_eq!(
        player_fingerprint(&first_controller, P1),
        player_fingerprint(&second_controller, P1),
        "the non-owner sees identical observation, information state, and pass request bytes"
    );
    assert_ne!(
        player_fingerprint(&first_controller, P2),
        player_fingerprint(&second_controller, P2),
        "the owner retains its authorized hidden-card knowledge"
    );
}
