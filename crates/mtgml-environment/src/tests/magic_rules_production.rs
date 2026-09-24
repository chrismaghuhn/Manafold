use crate::{EnvironmentBackend, PlayerEndpoint};
use crate::checkpoint::{
    CHECKPOINT_CODEC_ID_V6, CHECKPOINT_CODEC_SEMANTIC_VERSION_V6,
};
use mtgml_decision::{DecisionAnswerV2, DecisionResponseV2};
use mtgml_model::{
    CardDefinitionId, CheckpointCodecIdentity, EpisodeStatus, GameObjectId, OpaqueObjectId,
    PhysicalCardId, PlayerId,
    StateRevision,
};
use mtgml_observation::{MagicObservation, PlayerStepSubmissionV1};
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
            observation_payload_codec: mtgml_observation::MAGIC_OBSERVATION_SCHEMA_V1.into(),
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
        execution_identity: ReferenceEnvironmentBackend::magic_state_based_actions_execution_identity(),
        replay: s3_replay_config(),
    })
    .expect("the production S3.A semantic identity admits its bounded state")
}

fn basic_priority_backend(state: mtgml_state::EngineState) -> ReferenceEnvironmentBackend {
    ReferenceEnvironmentBackend::new(ReferenceEnvironmentConfig {
        state,
        status: EpisodeStatus::Running,
        limit_counters: EnvironmentLimitCounters::default(),
        codec: CheckpointCodecIdentity {
            codec_id: CHECKPOINT_CODEC_ID_V6.into(),
            semantic_version: CHECKPOINT_CODEC_SEMANTIC_VERSION_V6.into(),
        },
        execution_identity: ReferenceEnvironmentBackend::magic_basic_priority_execution_identity(),
        replay: s3_replay_config(),
    })
    .expect("the production S3.B semantic identity admits its bounded state")
}

fn draw_backend(state: mtgml_state::EngineState) -> ReferenceEnvironmentBackend {
    let mut replay = s3_replay_config();
    replay.scenario_id = "rules/draw-card@0.1.0:upkeep-to-draw".into();
    ReferenceEnvironmentBackend::new(ReferenceEnvironmentConfig {
        state,
        status: EpisodeStatus::Running,
        limit_counters: EnvironmentLimitCounters::default(),
        codec: CheckpointCodecIdentity {
            codec_id: CHECKPOINT_CODEC_ID_V6.into(),
            semantic_version: CHECKPOINT_CODEC_SEMANTIC_VERSION_V6.into(),
        },
        execution_identity: ReferenceEnvironmentBackend::magic_draw_execution_identity(),
        replay,
    })
    .expect("the production S3.C semantic identity admits its bounded state")
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

fn stable_draw_state_at_upkeep() -> mtgml_state::EngineState {
    let mut state = stable_state_at(TurnPosition::Beginning {
        step: mtgml_state::BeginningStep::Upkeep,
    });
    // Object 2 is the selected S2 Library-top card owned by P2 in the shared
    // authoritative fixture. Make its ordinary Draw Step the active player.
    state.core.active_player = P2;
    state.core.turn_number = 2;
    state.zones.objects.get_mut(&GameObjectId(2)).unwrap().face_down = false;
    mtgml_state::validate_engine_state(&state).unwrap();
    state
}

fn replace_library_top_identity(
    state: &mut mtgml_state::EngineState,
    physical: u64,
    definition: u64,
) {
    let object = GameObjectId(2);
    state.zones.objects.get_mut(&object).unwrap().physical_card = Some(PhysicalCardId(physical));
    state.zones.objects.get_mut(&object).unwrap().card_definition = CardDefinitionId(definition);
    if let Some(opaque) = state.perspective_identities.players[&P2]
        .object_to_opaque
        .get(&object)
        .copied()
    {
        let record = state.knowledge.players.get_mut(&P2).unwrap().active.get_mut(&opaque).unwrap();
        record.physical_card = Some(PhysicalCardId(physical));
        record.card_definition = Some(CardDefinitionId(definition));
    }
    mtgml_state::validate_engine_state(state).unwrap();
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

#[test]
// Rules authority: accepted `wotc-cr-2026-08-07-txt-20260819-sha256-
// 4381ad1b39ab2c05f7d03633a20f711ed37277074d3266dcba5f38cbb527423f`;
// CR 121.1 defines the top-Library-to-Hand draw, and CR 504.1 / 504.2 define
// the ordinary Draw Step action followed by active-player priority.
fn ordinary_draw_uses_s2_and_opens_active_priority() {
    let controller = TrustedEnvironmentController::new(draw_backend(stable_draw_state_at_upkeep()));
    controller.execute_forced_progress().unwrap();
    let initial = controller.checkpoint().unwrap();
    let old_top = GameObjectId(2);
    assert_eq!(initial.state.zones.ordered_zones[&mtgml_state::ZoneKey {
        zone: mtgml_model::ZoneKind::Library,
        player: Some(P2),
        visibility: mtgml_state::VisibilityPartition::FaceDown,
        partition: None,
    }][0], old_top);
    assert!(!initial.state.foundation_sources.contains_key(&old_top));
    let physical = initial.state.zones.objects[&old_top].physical_card;
    let old_library = initial.state.zones.locations[&old_top].clone();
    let active = controller.bind_player(P2).unwrap();
    let nonactive = controller.bind_player(P1).unwrap();
    assert_eq!(active.visible_decision().unwrap().unwrap().actor, P2);
    let mut restored_before_draw = draw_backend(initial.state.clone());
    restored_before_draw.restore(initial.clone()).unwrap();
    assert_eq!(restored_before_draw.checkpoint().unwrap(), initial);
    assert_eq!(controller.fork().unwrap().checkpoint().unwrap(), initial);
    let active_request = active.visible_decision().unwrap().unwrap();
    let before_stale = controller.checkpoint().unwrap();
    let replay_before_stale = controller.export_replay().unwrap();
    let p1_before_stale = player_fingerprint(&controller, P1);
    let p2_before_stale = player_fingerprint(&controller, P2);
    let mut stale = current_select_one_response(&active_request);
    stale.state_revision = StateRevision(stale.state_revision.0 - 1);
    let rejected = active.submit(stale).unwrap();
    assert!(matches!(
        rejected.submission,
        PlayerStepSubmissionV1::Rejected {
            code: mtgml_observation::PlayerSubmissionCodeV1::StaleDecision
        }
    ));
    assert_eq!(controller.checkpoint().unwrap(), before_stale);
    assert_eq!(controller.export_replay().unwrap(), replay_before_stale);
    assert_eq!(player_fingerprint(&controller, P1), p1_before_stale);
    assert_eq!(player_fingerprint(&controller, P2), p2_before_stale);
    active.submit(current_select_one_response(&active_request)).unwrap();
    let after_active_pass = controller.checkpoint().unwrap();
    let nonactive_request = nonactive.visible_decision().unwrap().unwrap();

    let mut direct_backend = draw_backend(after_active_pass.state.clone());
    direct_backend.restore(after_active_pass.clone()).unwrap();
    let transition = direct_backend
        .execute_trusted_response(P1, current_select_one_response(&nonactive_request))
        .expect("one real second pass composes Draw, S2, SBA, and Priority");
    assert_eq!(
        transition.next_state.revision,
        StateRevision(after_active_pass.state.revision.0 + 2)
    );
    assert_eq!(transition.delta.apply(&after_active_pass.state).unwrap(), transition.next_state);
    assert_eq!(
        transition.events.iter().filter(|event| matches!(
            &event.event,
            mtgml_rules::AuthoritativeRuleEventKind::ZoneTransition { transition }
                if transition.from.zone == mtgml_model::ZoneKind::Library
                    && transition.to.zone == mtgml_model::ZoneKind::Hand
        )).count(),
        1,
        "the forced product contains exactly one S2 Library-to-Hand transition"
    );
    assert!(matches!(
        &transition.events[3].event,
        mtgml_rules::AuthoritativeRuleEventKind::ZoneTransition { transition }
            if transition.old_object == old_top
                && transition.new_object == after_active_pass.state.allocators.next_object_id
    ));
    assert!(transition.events[..3].iter().all(|event| {
        event.state_revision == StateRevision(after_active_pass.state.revision.0 + 1)
    }));
    assert!(transition.events[3..].iter().all(|event| {
        event.state_revision == StateRevision(after_active_pass.state.revision.0 + 2)
    }));
    assert_eq!(transition.next_state.random, after_active_pass.state.random);
    let forced_replay = direct_backend.export_replay().unwrap();
    assert_eq!(forced_replay.steps.len(), 1);
    assert_eq!(forced_replay.steps[0].response, current_select_one_response(&nonactive_request));

    nonactive.submit(current_select_one_response(&nonactive_request)).unwrap();

    let after = controller.checkpoint().unwrap();
    assert_eq!(after, direct_backend.checkpoint().unwrap());
    assert_eq!(after.state.core.position, TurnPosition::Beginning {
        step: mtgml_state::BeginningStep::Draw,
    });
    assert_eq!(after.state.core.priority, mtgml_state::PriorityState::HeldBy {
        player: P2,
        consecutive_passes: 0,
    });
    assert!(!after.state.zones.objects.contains_key(&old_top));
    let new_top = *after.state.zones.locations.iter().find_map(|(id, location)| {
        (location.zone == mtgml_model::ZoneKind::Hand && location.player == Some(P2))
            .then_some(id)
    }).expect("Draw must create a fresh S2 incarnation in P2's Hand");
    assert_ne!(new_top, old_top);
    assert_eq!(after.state.zones.objects[&new_top].physical_card, physical);
    assert_eq!(new_top, after_active_pass.state.allocators.next_object_id);
    assert_eq!(old_library.zone, mtgml_model::ZoneKind::Library);
    assert_eq!(after.state.execution.pending_decision.as_ref().unwrap().request.actor, P2);
    for identity in after.state.perspective_identities.players.values() {
        assert!(!identity.object_to_opaque.contains_key(&old_top));
    }
    assert!(after.state.perspective_identities.players[&P2]
        .object_to_opaque
        .contains_key(&new_top));
    assert!(!after.state.perspective_identities.players[&P1]
        .object_to_opaque
        .contains_key(&new_top));

    let mut restored_after = draw_backend(after.state.clone());
    restored_after.restore(after.clone()).unwrap();
    assert_eq!(restored_after.checkpoint().unwrap(), after);
    let fork_after = controller.fork().unwrap();
    assert_eq!(fork_after.checkpoint().unwrap(), after);

    let replay = controller.export_replay().unwrap();
    assert_eq!(replay.steps.len(), 2);
    assert_eq!(replay.steps[1].response, current_select_one_response(&nonactive_request));
    let report = controller.execute_replay_from_checkpoint(initial, replay).unwrap();
    assert_eq!(report.final_checkpoint, after);

    let draw_priority_response =
        current_select_one_response(&active.visible_decision().unwrap().unwrap());
    let frozen_s3_b_before = after.state.clone();
    let mut s3_b_kernel = mtgml_rules::ProgramKernelV1::for_admitted_execution(
        mtgml_model::ExecutionProgramV1::MagicRules,
        crate::semantic_catalog_generated::magic_s3_b_basic_priority_0_1_0_semantic_contract_id(),
    )
    .unwrap();
    assert!(matches!(
        s3_b_kernel.apply(&frozen_s3_b_before, P2, &draw_priority_response),
        Err(mtgml_rules::KernelExecutionError::UnsupportedStagePath)
    ));
    assert_eq!(frozen_s3_b_before, after.state);

    let mut s3_c_kernel = mtgml_rules::ProgramKernelV1::for_admitted_execution(
        mtgml_model::ExecutionProgramV1::MagicRules,
        crate::semantic_catalog_generated::magic_s3_c_draw_interaction_0_1_0_semantic_contract_id(),
    )
    .unwrap();
    let s3_c_pass = s3_c_kernel
        .apply(&after.state, P2, &draw_priority_response)
        .unwrap();
    assert_eq!(
        s3_c_pass.next_state.core.priority,
        mtgml_state::PriorityState::HeldBy {
            player: P1,
            consecutive_passes: 1,
        }
    );
}

#[test]
fn ordinary_draw_preserves_opponent_noninterference_across_hidden_worlds() {
    let base = stable_draw_state_at_upkeep();
    let mut world_a = base.clone();
    let mut world_b = base;
    replace_library_top_identity(&mut world_a, 20, 120);
    replace_library_top_identity(&mut world_b, 30, 130);

    let first = TrustedEnvironmentController::new(draw_backend(world_a));
    let second = TrustedEnvironmentController::new(draw_backend(world_b));
    first.execute_forced_progress().unwrap();
    second.execute_forced_progress().unwrap();
    assert_eq!(player_fingerprint(&first, P1), player_fingerprint(&second, P1));

    let first_active = first.bind_player(P2).unwrap();
    let second_active = second.bind_player(P2).unwrap();
    first_active
        .submit(current_select_one_response(&first_active.visible_decision().unwrap().unwrap()))
        .unwrap();
    second_active
        .submit(current_select_one_response(&second_active.visible_decision().unwrap().unwrap()))
        .unwrap();
    assert_eq!(player_fingerprint(&first, P1), player_fingerprint(&second, P1));

    let first_nonactive = first.bind_player(P1).unwrap();
    let second_nonactive = second.bind_player(P1).unwrap();
    let first_request = first_nonactive.visible_decision().unwrap().unwrap();
    let second_request = second_nonactive.visible_decision().unwrap().unwrap();
    assert_eq!(first_request, second_request);
    let first_step = first_nonactive
        .submit(current_select_one_response(&first_request))
        .unwrap();
    let second_step = second_nonactive
        .submit(current_select_one_response(&second_request))
        .unwrap();

    assert_eq!(
        mtgml_wire::encode_canonical(&first_step).unwrap(),
        mtgml_wire::encode_canonical(&second_step).unwrap(),
        "the nonactive PlayerStep must not expose the hidden card identity"
    );
    assert_eq!(player_fingerprint(&first, P1), player_fingerprint(&second, P1));
    assert_ne!(
        player_fingerprint(&first, P2),
        player_fingerprint(&second, P2),
        "the owner receives the private identity authorized by S2"
    );
}

#[test]
fn ordinary_draw_rejections_are_atomic_and_fail_closed() {
    let mut wrong_turn = stable_draw_state_at_upkeep();
    wrong_turn.core.position = TurnPosition::Beginning {
        step: mtgml_state::BeginningStep::Draw,
    };
    wrong_turn.core.turn_number = 1;
    mtgml_state::validate_engine_state(&wrong_turn).unwrap();

    let mut wrong_owner = stable_draw_state_at_upkeep();
    wrong_owner.core.position = TurnPosition::Beginning {
        step: mtgml_state::BeginningStep::Draw,
    };
    wrong_owner.core.active_player = P1;
    mtgml_state::validate_engine_state(&wrong_owner).unwrap();

    let mut empty_library = stable_draw_state_at_upkeep();
    empty_library.core.position = TurnPosition::Beginning {
        step: mtgml_state::BeginningStep::Draw,
    };
    let old_top = GameObjectId(2);
    let key = mtgml_state::ZoneKey {
        zone: mtgml_model::ZoneKind::Library,
        player: Some(P2),
        visibility: mtgml_state::VisibilityPartition::FaceDown,
        partition: None,
    };
    empty_library.zones.ordered_zones.remove(&key);
    empty_library.zones.objects.remove(&old_top);
    empty_library.zones.locations.remove(&old_top);
    for (perspective, identity) in &mut empty_library.perspective_identities.players {
        if let Some(opaque) = identity.object_to_opaque.remove(&old_top) {
            identity.opaque_to_object.remove(&opaque);
            empty_library
                .knowledge
                .players
                .get_mut(perspective)
                .unwrap()
                .active
                .remove(&opaque);
        }
    }
    mtgml_state::validate_engine_state(&empty_library).unwrap();

    for (label, state) in [
        ("turn one", wrong_turn),
        ("wrong Library owner", wrong_owner),
        ("empty Library", empty_library),
    ] {
        let before = state.clone();
        let mut kernel = mtgml_rules::ProgramKernelV1::for_admitted_execution(
            mtgml_model::ExecutionProgramV1::MagicRules,
            crate::semantic_catalog_generated::magic_s3_c_draw_interaction_0_1_0_semantic_contract_id(),
        )
        .unwrap();
        let rejected = kernel.advance_forced_progress(&state);
        assert!(rejected.is_err(), "{label} must fail closed");
        assert_eq!(state, before, "{label} rejection must not mutate EngineState");
    }

    let mut partial_draw = stable_draw_state_at_upkeep();
    partial_draw.core.position = TurnPosition::Beginning {
        step: mtgml_state::BeginningStep::Draw,
    };
    partial_draw.core.turn_number = 2;
    let mut invalid_config = ReferenceEnvironmentConfig {
        state: partial_draw,
        status: EpisodeStatus::Running,
        limit_counters: EnvironmentLimitCounters::default(),
        codec: CheckpointCodecIdentity {
            codec_id: CHECKPOINT_CODEC_ID_V6.into(),
            semantic_version: CHECKPOINT_CODEC_SEMANTIC_VERSION_V6.into(),
        },
        execution_identity: ReferenceEnvironmentBackend::magic_draw_execution_identity(),
        replay: s3_replay_config(),
    };
    invalid_config.replay.scenario_id = "rules/draw-card@0.1.0:partial-draw-rejected".into();
    assert!(
        ReferenceEnvironmentBackend::new(invalid_config).is_err(),
        "Draw + Priority=None is kernel-local and cannot be admitted as a checkpoint"
    );
}

#[test]
fn postdraw_sba_order_continuation_restores_forks_and_resumes() {
    let mut state = stable_draw_state_at_upkeep();
    state.core.position = TurnPosition::Beginning {
        step: mtgml_state::BeginningStep::Draw,
    };
    add_creature(&mut state, 4, P2, [3, 4]);
    add_creature(&mut state, 5, P2, [4, 5]);
    state.allocators.next_object_id = GameObjectId(6);
    mtgml_state::validate_engine_state(&state).unwrap();
    let before = state.clone();

    let mut kernel = mtgml_rules::ProgramKernelV1::for_admitted_execution(
        mtgml_model::ExecutionProgramV1::MagicRules,
        crate::semantic_catalog_generated::magic_s3_c_draw_interaction_0_1_0_semantic_contract_id(),
    )
    .unwrap();
    let draw_and_sba = kernel.advance_forced_progress(&state).unwrap();
    assert_eq!(
        draw_and_sba.next_state.revision,
        StateRevision(before.revision.0 + 2),
        "Draw and the persisted Order Decision retain separate Rules revisions"
    );
    let pending = draw_and_sba.next_state.execution.pending_decision.as_ref().unwrap();
    assert_eq!(pending.request.actor, P2);
    assert!(matches!(pending.request.decision, mtgml_decision::DecisionDomainV2::Order { .. }));
    assert_eq!(draw_and_sba.next_state.core.priority, mtgml_state::PriorityState::None);
    assert!(!draw_and_sba.events.iter().any(|event| matches!(
        event.event,
        mtgml_rules::AuthoritativeRuleEventKind::StateBasedActionsApplied { .. }
    )));
    assert!(draw_and_sba.next_state.zones.objects.contains_key(&GameObjectId(4)));
    assert!(draw_and_sba.next_state.zones.objects.contains_key(&GameObjectId(5)));
    assert_eq!(
        draw_and_sba.events.iter().filter(|event| matches!(
            &event.event,
            mtgml_rules::AuthoritativeRuleEventKind::ZoneTransition { transition }
                if transition.from.zone == mtgml_model::ZoneKind::Library
                    && transition.to.zone == mtgml_model::ZoneKind::Hand
        )).count(),
        1
    );

    let backend = draw_backend(draw_and_sba.next_state.clone());
    let controller = TrustedEnvironmentController::new(backend);
    let order_checkpoint = controller.checkpoint().unwrap();
    let mut restored = draw_backend(order_checkpoint.state.clone());
    restored.restore(order_checkpoint.clone()).unwrap();
    assert_eq!(restored.checkpoint().unwrap(), order_checkpoint);
    assert_eq!(controller.fork().unwrap().checkpoint().unwrap(), order_checkpoint);

    let order_response = current_order_response(&order_checkpoint.state);
    let mut direct_backend = draw_backend(order_checkpoint.state.clone());
    direct_backend.restore(order_checkpoint.clone()).unwrap();
    direct_backend.execute_trusted_response(P2, order_response.clone()).unwrap();
    controller.bind_player(P2).unwrap().submit(order_response).unwrap();
    let after_order = controller.checkpoint().unwrap();
    assert_eq!(
        after_order.state.core.priority,
        mtgml_state::PriorityState::HeldBy {
            player: P2,
            consecutive_passes: 0,
        }
    );
    assert_eq!(after_order, direct_backend.checkpoint().unwrap());
    assert!(after_order.state.execution.continuations.is_empty());
    assert!(!after_order.state.zones.objects.contains_key(&GameObjectId(4)));
    assert!(!after_order.state.zones.objects.contains_key(&GameObjectId(5)));
    assert_eq!(controller.export_replay().unwrap().steps.len(), 1);
}

fn decode_magic_observation(
    envelope: mtgml_observation::ObservationEnvelope,
) -> MagicObservation {
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
        ReferenceEnvironmentBackend::magic_state_based_actions_execution_identity()
    );
    let mut restored_stage_zero = s3_backend(stage_zero.state.clone());
    restored_stage_zero.restore(stage_zero.clone()).unwrap();
    assert_eq!(restored_stage_zero.checkpoint().unwrap(), stage_zero);
    assert_eq!(
        backend.player_observation(P1).unwrap().payload_codec,
        mtgml_observation::MAGIC_OBSERVATION_SCHEMA_V1
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
        mtgml_observation::MAGIC_OBSERVATION_SCHEMA_V1
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
fn production_basic_priority_final_order_then_priority_is_one_r_plus_two_replay_commit() {
    let initial_state = super::restore_admission::magic_sba_continuation_state();
    let initial = basic_priority_backend(initial_state).checkpoint().unwrap();
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
    let mut direct_backend = basic_priority_backend(initial.state.clone());
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
        crate::player_projection::ObservationProjectionProfile::Magic,
    )
    .expect("S3.B candidate PlayerStep projection");
    projected_step.validate().expect("S3.B PlayerStep validation");
    let backend = basic_priority_backend(initial.state.clone());
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
fn production_basic_priority_no_order_sba_finishes_before_opening_priority_in_one_forced_product() {
    let state = no_order_one_death_state();
    let expected_revision = state.revision.0 + 1;
    let mut backend = basic_priority_backend(state);
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
    let mut restored = basic_priority_backend(checkpoint.state.clone());
    restored.restore(checkpoint.clone()).unwrap();
    assert_eq!(restored.checkpoint().unwrap(), checkpoint);
}

#[test]
fn production_basic_priority_restore_admission_is_distinct_from_s1_and_s3_a() {
    let mut backend = basic_priority_backend(no_order_one_death_state());
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
        ReferenceEnvironmentBackend::magic_state_based_actions_execution_identity(),
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
fn production_basic_priority_active_priority_and_after_first_pass_restore_fork_and_replay() {
    let mut backend = basic_priority_backend(stable_state_at(TurnPosition::Beginning {
        step: mtgml_state::BeginningStep::Upkeep,
    }));
    backend.execute_forced_progress().unwrap();
    let stage_zero = backend.checkpoint().unwrap();
    let p1_request = backend.player_visible_decision(P1).unwrap().unwrap();
    assert!(backend.player_visible_decision(P2).unwrap().is_none());
    assert_eq!(p1_request.actor, P1);

    let mut restored_stage_zero = basic_priority_backend(stage_zero.state.clone());
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
    let mut restored_stage_one = basic_priority_backend(stage_one.state.clone());
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
fn production_basic_priority_end_step_two_pass_reference_replay_closes_through_cleanup() {
    let mut backend = basic_priority_backend(stable_state_at(TurnPosition::Ending {
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
fn production_basic_priority_pass_window_noninterference_hides_opponent_library_definition() {
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

    let mut first_backend = basic_priority_backend(first);
    let mut second_backend = basic_priority_backend(second);
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
