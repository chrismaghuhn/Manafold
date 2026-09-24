use base64::engine::general_purpose::STANDARD;
use mtgml_decision::{
    AuthoritativeCandidateV2, AuthoritativeDecisionRequestV2, CandidateIntent, DecisionDomainV2,
    DecisionVisibility, EngineCandidateBinding,
};
use mtgml_model::{GameObjectId, OpaqueObjectId};
use mtgml_observation::{MagicObservation, MAGIC_OBSERVATION_SCHEMA_V1};
use mtgml_state::{
    ContinuationPayloadV2, ContinuationRecordV2, PendingDecisionRecordV2,
    SbaGraveyardOwnerOrderV1,
};

fn state_with_completed_sba_order() -> mtgml_state::EngineState {
    let mut state = mtgml_state::construct_synthetic_engine_state(mtgml_state::SyntheticResetInputs {
        players: [PlayerId(1), PlayerId(2)],
        root_seed: seed(),
        setup: mtgml_state::SyntheticV4Setup::synthetic_compatibility(),
    })
    .unwrap();
    state.revision = StateRevision(2);
    state.core.position = mtgml_state::TurnPosition::Beginning {
        step: mtgml_state::BeginningStep::Upkeep,
    };
    let public_location = state.zones.locations[&GameObjectId(1)].clone();
    let public_prototype = state.zones.objects[&GameObjectId(1)].clone();
    for (id, owner, physical, definition) in [
        (GameObjectId(3), PlayerId(1), 3, 3),
        (GameObjectId(4), PlayerId(2), 4, 4),
        (GameObjectId(5), PlayerId(2), 5, 5),
    ] {
        let mut object = public_prototype.clone();
        object.id = id;
        object.owner = owner;
        object.controller = owner;
        object.physical_card = Some(mtgml_model::PhysicalCardId(physical));
        object.card_definition = mtgml_model::CardDefinitionId(definition);
        state.zones.objects.insert(id, object);
        state.zones.locations.insert(id, public_location.clone());
    }
    for object in [GameObjectId(1), GameObjectId(3), GameObjectId(4), GameObjectId(5)] {
        state.foundation_sources.insert(
            object,
            mtgml_state::FoundationCreatureSource {
                source_kind: mtgml_state::FoundationSourceKind::Creature,
                base_characteristics: mtgml_state::BaseCharacteristics::Simple {
                    power: 1,
                    toughness: 0,
                },
                marked_damage: 0,
                control_history: mtgml_state::ControlHistory::DuringTurn {
                    turn_number: 1,
                    boundary: state.core.position,
                },
            },
        );
    }
    state.allocators.next_object_id = GameObjectId(6);
    for (player, pairs, next) in [
        (
            PlayerId(1),
            vec![
                (GameObjectId(3), OpaqueObjectId(2)),
                (GameObjectId(4), OpaqueObjectId(3)),
                (GameObjectId(5), OpaqueObjectId(4)),
            ],
            OpaqueObjectId(5),
        ),
        (
            PlayerId(2),
            vec![
                (GameObjectId(3), OpaqueObjectId(3)),
                (GameObjectId(4), OpaqueObjectId(4)),
                (GameObjectId(5), OpaqueObjectId(5)),
            ],
            OpaqueObjectId(6),
        ),
    ] {
        let identity = state.perspective_identities.players.get_mut(&player).unwrap();
        for (object, opaque) in pairs {
            identity.object_to_opaque.insert(object, opaque);
            identity.opaque_to_object.insert(opaque, object);
        }
        identity.next_opaque_object_id = next;
    }
    let selected_sba_actions = [GameObjectId(1), GameObjectId(3), GameObjectId(4), GameObjectId(5)]
        .into_iter()
        .map(|object| mtgml_state::SbaSelectedActionV1::ObjectToOwnerGraveyard {
            object,
            causes: vec![mtgml_state::SbaObjectCauseV1::ZeroToughness],
        })
        .collect();
    state.execution.continuations.insert(
        ContinuationId(12),
        ContinuationRecordV2 {
            id: ContinuationId(12),
            actor: PlayerId(2),
            created_at_revision: StateRevision(1),
            stage_index: 1,
            payload: ContinuationPayloadV2::MagicSbaGraveyardOrderV1 {
                round_start_revision: StateRevision(0),
                selected_sba_actions,
                apnap_owners: vec![PlayerId(1), PlayerId(2)],
                next_owner_index: 1,
                completed_owner_orders: vec![SbaGraveyardOwnerOrderV1 {
                    owner: PlayerId(1),
                    top_to_bottom: vec![GameObjectId(3), GameObjectId(1)],
                }],
            },
        },
    );
    state.execution.pending_decision = Some(PendingDecisionRecordV2 {
        request: AuthoritativeDecisionRequestV2 {
            decision_id: mtgml_model::DecisionId(14),
            player_decision_id: mtgml_model::PlayerDecisionIdV1(2),
            state_revision: StateRevision(2),
            actor: PlayerId(2),
            visibility: DecisionVisibility::ActingPlayerOnly,
            decision: DecisionDomainV2::Order {
                minimum: 2,
                maximum: 2,
            },
            candidates: [GameObjectId(4), GameObjectId(5)]
                .into_iter()
                .enumerate()
                .map(|(index, object)| AuthoritativeCandidateV2 {
                    candidate_id: mtgml_model::CandidateIdV1(index as u32),
                    visible_intent: CandidateIntent::SelectObject {
                        object: state.perspective_identities.players[&PlayerId(2)].object_to_opaque[&object],
                    },
                    trusted_binding: EngineCandidateBinding::SelectObject { object },
                })
                .collect(),
            continuation_id: Some(ContinuationId(12)),
        },
    });
    state.allocators.next_decision_id = mtgml_model::DecisionId(15);
    state.allocators.next_continuation_id = ContinuationId(13);
    state.perspective_identities.players.get_mut(&PlayerId(2)).unwrap().next_player_decision_id =
        mtgml_model::PlayerDecisionIdV1(3);
    mtgml_state::validate_engine_state(&state)
        .expect("Task-7 stage-one projection fixture must be structurally coherent");
    state
}

fn magic_payload(state: &mtgml_state::EngineState, perspective: PlayerId) -> MagicObservation {
    let envelope = crate::player_projection::project_observation_with_profile(
        state,
        perspective,
        crate::player_projection::ObservationProjectionProfile::Magic,
    )
    .unwrap();
    let bytes = STANDARD.decode(envelope.payload_base64).unwrap();
    mtgml_wire::decode_canonical(&bytes).unwrap()
}

#[test]
fn accepted_apnap_order_is_projected_with_each_perspectives_opaque_ids_without_mutation() {
    let state = state_with_completed_sba_order();
    let before = state.clone();
    let p1 = magic_payload(&state, PlayerId(1));
    let p2 = magic_payload(&state, PlayerId(2));
    assert_eq!(p1.schema_version, MAGIC_OBSERVATION_SCHEMA_V1);
    let p1_order = p1.pending_sba_ordering.as_ref().unwrap();
    let p2_order = p2.pending_sba_ordering.as_ref().unwrap();
    assert_eq!(p1_order.next_order_owner, PlayerId(2));
    assert_eq!(p2_order.next_order_owner, PlayerId(2));
    assert_eq!(p1_order.completed_orders[0].owner, PlayerId(1));
    assert_eq!(p2_order.completed_orders[0].owner, PlayerId(1));
    assert_eq!(p1_order.completed_orders[0].ordered_objects, vec![OpaqueObjectId(2), OpaqueObjectId(1)]);
    assert_eq!(p2_order.completed_orders[0].ordered_objects, vec![OpaqueObjectId(3), OpaqueObjectId(1)]);
    assert_ne!(p1_order.completed_orders[0].ordered_objects, p2_order.completed_orders[0].ordered_objects);
    assert_eq!(state, before);
    assert_eq!(magic_payload(&state, PlayerId(1)), p1);
    assert_eq!(magic_payload(&state, PlayerId(2)), p2);
    let p1_bytes = mtgml_wire::encode_canonical(&p1).unwrap();
    let public_text = String::from_utf8(p1_bytes).unwrap();
    for forbidden in [
        "continuation",
        "game_object_id",
        "physical_card_id",
        "card_definition_id",
        "decision_id",
        "allocator",
    ] {
        assert!(!public_text.contains(forbidden));
    }
    let p1_step = crate::player_projection::project_player_step_with_profile(
        &state,
        PlayerId(1),
        EpisodeStatus::Running,
        mtgml_observation::PlayerStepSubmissionV1::Accepted,
        crate::player_projection::ObservationProjectionProfile::Magic,
    )
    .unwrap();
    let p2_step = crate::player_projection::project_player_step_with_profile(
        &state,
        PlayerId(2),
        EpisodeStatus::Running,
        mtgml_observation::PlayerStepSubmissionV1::Accepted,
        crate::player_projection::ObservationProjectionProfile::Magic,
    )
    .unwrap();
    assert!(p1_step.next_decision.is_none());
    let p2_request = p2_step.next_decision.unwrap();
    assert_eq!(p2_request.actor, PlayerId(2));
    assert_eq!(p2_request.candidates.len(), 2);
}

#[test]
fn initial_order_stage_projects_empty_progress_and_first_apnap_owner() {
    let mut state = state_with_completed_sba_order();
    state.revision = StateRevision(1);
    let continuation = state.execution.continuations.values_mut().next().unwrap();
    continuation.actor = PlayerId(1);
    continuation.stage_index = 0;
    continuation.created_at_revision = StateRevision(1);
    if let ContinuationPayloadV2::MagicSbaGraveyardOrderV1 {
        next_owner_index,
        completed_owner_orders,
        ..
    } = &mut continuation.payload
    {
        *next_owner_index = 0;
        completed_owner_orders.clear();
    }
    let request = &mut state
        .execution
        .pending_decision
        .as_mut()
        .unwrap()
        .request;
    request.decision_id = mtgml_model::DecisionId(13);
    request.player_decision_id = mtgml_model::PlayerDecisionIdV1(2);
    request.state_revision = StateRevision(1);
    request.actor = PlayerId(1);
    request.candidates = [GameObjectId(1), GameObjectId(3)]
        .into_iter()
        .enumerate()
        .map(|(index, object)| AuthoritativeCandidateV2 {
            candidate_id: mtgml_model::CandidateIdV1(index as u32),
            visible_intent: CandidateIntent::SelectObject {
                object: state.perspective_identities.players[&PlayerId(1)].object_to_opaque[&object],
            },
            trusted_binding: EngineCandidateBinding::SelectObject { object },
        })
        .collect();
    state.allocators.next_decision_id = mtgml_model::DecisionId(14);
    state.perspective_identities.players.get_mut(&PlayerId(1)).unwrap().next_player_decision_id =
        mtgml_model::PlayerDecisionIdV1(3);
    mtgml_state::validate_engine_state(&state)
        .expect("Task-7 stage-zero projection fixture must be structurally coherent");
    let observation = magic_payload(&state, PlayerId(1));
    let progress = observation.pending_sba_ordering.unwrap();
    assert!(progress.completed_orders.is_empty());
    assert_eq!(progress.next_order_owner, PlayerId(1));
}

#[test]
fn missing_perspective_opaque_mapping_fails_closed() {
    let mut state = state_with_completed_sba_order();
    state.perspective_identities.players.get_mut(&PlayerId(2)).unwrap().object_to_opaque.remove(&GameObjectId(3));
    assert!(crate::player_projection::project_observation_with_profile(
        &state,
        PlayerId(2),
        crate::player_projection::ObservationProjectionProfile::Magic,
    ).is_err());
}

#[test]
fn production_projection_remains_synthetic_even_for_magic_continuation_state() {
    let state = state_with_completed_sba_order();
    let envelope = crate::player_projection::project_observation(&state, PlayerId(1)).unwrap();
    assert_eq!(envelope.payload_codec, "synthetic-m3-observation.v1");
}

#[test]
fn admitted_s1_reference_magic_program_still_projects_synthetic_codec() {
    let state = super::reference_state(mtgml_state::TurnPosition::PrecombatMain);
    let controller = super::reference_controller(state);
    let observation = controller
        .bind_player(PlayerId(1))
        .unwrap()
        .observation()
        .unwrap();
    assert_eq!(observation.payload_codec, "synthetic-m3-observation.v1");
}
