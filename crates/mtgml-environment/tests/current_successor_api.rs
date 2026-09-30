mod common;

use common::CONTENT;

use std::collections::BTreeMap;

use mtgml_card_ir::{decode_content_manifest_v1, CardSemanticBindingV1};
use mtgml_decision::{DecisionAnswerV2, DecisionResponseV3};
use mtgml_environment::{
    submit_response_bytes, BasicLandReplayV8ExecutionReport, BasicLandRuntimeOutputV8,
    ControllerError, CurrentPlayerStep, EnvironmentBackend, EnvironmentCheckpointV8,
    PlayerEndpoint, PlayerEndpointError, TrustedEnvironmentController,
};
use mtgml_model::{CardDefinitionId, PlayerId};
use mtgml_observation::{ObservationEnvelopeV2, PlayerInformationStateV3, PlayerStepV4};
use mtgml_replay::AuthoritativeReplayV8;
use mtgml_state::{
    AbilityAuthorityStateV1, AbilityAuthorityV1, CardRulesAuthoritativeStateV1, EngineState,
    FaceStateV1, ManaStateV1, PlayerTurnHistoryV1, TurnHistoryStateV1, TurnPosition,
    VisibilityPartition, ZoneKey, ZoneLocation, ZonePosition,
};

struct ProductionAliasProbe {
    step: PlayerStepV4,
}

impl EnvironmentBackend for ProductionAliasProbe {
    fn players(&self) -> Vec<PlayerId> {
        vec![self.step.information_state.perspective]
    }

    fn checkpoint(&self) -> Result<EnvironmentCheckpointV8, ControllerError> {
        Err(ControllerError::Backend("probe backend".to_owned()))
    }

    fn restore(&mut self, _: EnvironmentCheckpointV8) -> Result<(), ControllerError> {
        Err(ControllerError::Backend("probe backend".to_owned()))
    }

    fn fork_boxed(&self) -> Result<Box<dyn EnvironmentBackend>, ControllerError> {
        Err(ControllerError::Backend("probe backend".to_owned()))
    }

    fn export_replay(&self) -> Result<AuthoritativeReplayV8, ControllerError> {
        Err(ControllerError::Backend("probe backend".to_owned()))
    }

    fn execute_replay(
        &self,
        _: AuthoritativeReplayV8,
    ) -> Result<BasicLandReplayV8ExecutionReport, ControllerError> {
        Err(ControllerError::Backend("probe backend".to_owned()))
    }

    fn player_observation(
        &self,
        perspective: PlayerId,
    ) -> Result<ObservationEnvelopeV2, PlayerEndpointError> {
        if perspective != self.step.information_state.perspective {
            return Err(PlayerEndpointError::ServiceUnavailable);
        }
        Ok(self.step.information_state.current_observation.clone())
    }

    fn player_information_state(
        &self,
        perspective: PlayerId,
    ) -> Result<PlayerInformationStateV3, PlayerEndpointError> {
        if perspective != self.step.information_state.perspective {
            return Err(PlayerEndpointError::ServiceUnavailable);
        }
        Ok(self.step.information_state.clone())
    }

    fn player_visible_decision(
        &self,
        perspective: PlayerId,
    ) -> Result<Option<mtgml_decision::PlayerDecisionRequestV4>, PlayerEndpointError> {
        if perspective != self.step.information_state.perspective {
            return Err(PlayerEndpointError::ServiceUnavailable);
        }
        Ok(self.step.next_decision.clone())
    }

    fn submit_player_response(
        &mut self,
        perspective: PlayerId,
        _: DecisionResponseV3,
    ) -> Result<PlayerStepV4, PlayerEndpointError> {
        if perspective != self.step.information_state.perspective {
            return Err(PlayerEndpointError::ServiceUnavailable);
        }
        Ok(self.step.clone())
    }

    fn execute_transition(
        &mut self,
        _: PlayerId,
        _: DecisionResponseV3,
    ) -> Result<BasicLandRuntimeOutputV8, PlayerEndpointError> {
        Err(PlayerEndpointError::ServiceUnavailable)
    }
}

#[test]
fn public_current_endpoint_and_wire_boundary_return_player_step_v4() {
    let step: PlayerStepV4 = serde_json::from_str(include_str!(
        "../../../schemas/examples/player-step-v4.json"
    ))
    .unwrap();
    let expected = step.clone();
    let perspective = step.information_state.perspective;
    let controller = TrustedEnvironmentController::new(ProductionAliasProbe { step });
    let endpoint = controller.bind_player(perspective).unwrap();
    let _: &dyn PlayerEndpoint = &endpoint;

    let response: DecisionResponseV3 = serde_json::from_str(include_str!(
        "../../../schemas/examples/decision-response-v3.json"
    ))
    .unwrap();
    let bytes = mtgml_wire::encode_canonical(&response).unwrap();
    let current_step: CurrentPlayerStep = submit_response_bytes(&endpoint, &bytes).unwrap();
    let successor_step: PlayerStepV4 = current_step;
    assert_eq!(successor_step, expected);
}

#[test]
fn verified_basic_land_runs_through_real_v8_controller_and_player_endpoints() {
    let admission = common::game_admission();
    let mut state = basic_land_state();
    let status = mtgml_model::EpisodeStatus::Running;
    mtgml_rules::install_basic_land_request_v4(&admission, &mut state, PlayerId(1), &status)
        .unwrap();
    let checkpoint = EnvironmentCheckpointV8::new_for_basic_land_profile(
        &admission,
        state.clone(),
        status.clone(),
        Default::default(),
        admission.execution_identity().clone(),
    )
    .unwrap();
    let runtime = mtgml_environment::BasicLandEnvironmentRuntimeV8::new(
        admission.clone(),
        state,
        status,
        Default::default(),
        common::replay_manifest(&admission, &checkpoint),
    )
    .unwrap();
    let controller = TrustedEnvironmentController::new(runtime);
    let player_one = controller.bind_player(PlayerId(1)).unwrap();
    let player_two = controller.bind_player(PlayerId(2)).unwrap();
    let initial = controller.checkpoint().unwrap();
    let initial_replay = controller.export_replay().unwrap();

    let request = player_one.visible_decision().unwrap().unwrap();
    assert!(player_two.visible_decision().unwrap().is_none());
    assert_eq!(player_one.observation().unwrap().perspective, PlayerId(1));
    assert_eq!(player_two.observation().unwrap().perspective, PlayerId(2));
    assert_eq!(
        player_one.information_state().unwrap().perspective,
        PlayerId(1)
    );
    assert_eq!(
        player_two.information_state().unwrap().perspective,
        PlayerId(2)
    );

    let wrong_actor = DecisionResponseV3 {
        schema_version: mtgml_decision::DECISION_RESPONSE_V3_SCHEMA.to_owned(),
        player_decision_id: request.player_decision_id,
        view_sequence: request.view_sequence,
        answer: DecisionAnswerV2::SelectOne {
            candidate_id: mtgml_model::CandidateIdV1(1),
        },
    };
    assert_eq!(
        player_two.submit(wrong_actor).unwrap().submission,
        mtgml_observation::PlayerStepSubmissionV1::Rejected {
            code: mtgml_observation::PlayerSubmissionCodeV1::UnavailableDecision,
        }
    );
    assert_eq!(controller.checkpoint().unwrap(), initial);
    assert_eq!(controller.export_replay().unwrap(), initial_replay);

    let fabricated = DecisionResponseV3 {
        schema_version: mtgml_decision::DECISION_RESPONSE_V3_SCHEMA.to_owned(),
        player_decision_id: request.player_decision_id,
        view_sequence: request.view_sequence,
        answer: DecisionAnswerV2::SelectOne {
            candidate_id: mtgml_model::CandidateIdV1(u32::MAX),
        },
    };
    assert_eq!(
        player_one.submit(fabricated).unwrap().submission,
        mtgml_observation::PlayerStepSubmissionV1::Rejected {
            code: mtgml_observation::PlayerSubmissionCodeV1::InvalidCandidate,
        }
    );
    assert_eq!(controller.checkpoint().unwrap(), initial);
    assert_eq!(controller.export_replay().unwrap(), initial_replay);

    let accepted = DecisionResponseV3 {
        schema_version: mtgml_decision::DECISION_RESPONSE_V3_SCHEMA.to_owned(),
        player_decision_id: request.player_decision_id,
        view_sequence: request.view_sequence,
        answer: DecisionAnswerV2::SelectOne {
            candidate_id: mtgml_model::CandidateIdV1(1),
        },
    };
    let bytes = mtgml_wire::encode_canonical(&accepted).unwrap();
    let current_step: CurrentPlayerStep = submit_response_bytes(&player_one, &bytes).unwrap();
    let step: PlayerStepV4 = current_step;
    step.validate().unwrap();
    assert_eq!(
        step.submission,
        mtgml_observation::PlayerStepSubmissionV1::Accepted
    );
    assert!(!step.observed_events.is_empty());

    let replay = controller.export_replay().unwrap();
    assert_eq!(replay.steps.len(), 1);
    let report = controller.execute_replay(replay).unwrap();
    assert_eq!(report.transitions.len(), 1);
    assert_eq!(report.final_checkpoint, controller.checkpoint().unwrap());
}

fn add_object(
    state: &mut mtgml_state::EngineState,
    definition: CardDefinitionId,
    owner: PlayerId,
    zone: mtgml_model::ZoneKind,
    opaque_id: u64,
) -> mtgml_model::GameObjectId {
    let id = state.allocators.allocate_object_id().unwrap();
    state.zones.objects.insert(
        id,
        mtgml_state::GameObject {
            id,
            physical_card: None,
            card_definition: definition,
            owner,
            controller: owner,
            tapped: false,
            face_down: false,
        },
    );
    let key = ZoneKey {
        zone,
        player: Some(owner),
        visibility: if zone == mtgml_model::ZoneKind::Battlefield {
            VisibilityPartition::Public
        } else {
            VisibilityPartition::OwnerOnly
        },
        partition: None,
    };
    let offset = state.zones.ordered_zones.get(&key).map_or(0, Vec::len) as u32;
    state.zones.locations.insert(
        id,
        ZoneLocation {
            zone,
            player: Some(owner),
            position: ZonePosition::Top { offset },
            visibility: key.visibility,
            partition: None,
        },
    );
    state.zones.ordered_zones.entry(key).or_default().push(id);
    for (player, identity) in &mut state.perspective_identities.players {
        let opaque = mtgml_model::OpaqueObjectId(opaque_id + player.0 * 100);
        identity.object_to_opaque.insert(id, opaque);
        identity.opaque_to_object.insert(opaque, id);
        identity.next_opaque_object_id.0 = identity.next_opaque_object_id.0.max(opaque.0 + 1);
    }
    id
}

fn basic_land_state() -> EngineState {
    let definitions = decode_content_manifest_v1(CONTENT).unwrap();
    let find_definition = |subtype| {
        definitions
            .definitions
            .iter()
            .find(|definition| {
                matches!(definition.semantic_binding,
                    CardSemanticBindingV1::ProfiledV1 { body, .. } if body.subtype == subtype)
            })
            .unwrap()
            .card_definition_id
    };
    let mountain = find_definition(mtgml_card_ir::BasicLandSubtypeV1::Mountain);
    let plains = find_definition(mtgml_card_ir::BasicLandSubtypeV1::Plains);
    let actor = PlayerId(1);
    let mut setup = mtgml_state::SyntheticV4Setup::synthetic_compatibility();
    setup.position = TurnPosition::PrecombatMain;
    setup.priority = mtgml_state::PriorityState::HeldBy {
        player: actor,
        consecutive_passes: 0,
    };
    let mut engine =
        mtgml_state::construct_synthetic_engine_state(mtgml_state::SyntheticResetInputs {
            players: [PlayerId(1), PlayerId(2)],
            root_seed: mtgml_random::RootSeed256([0x38; 32]),
            setup,
        })
        .unwrap();
    add_object(
        &mut engine,
        mountain,
        actor,
        mtgml_model::ZoneKind::Hand,
        10,
    );
    add_object(&mut engine, plains, actor, mtgml_model::ZoneKind::Hand, 11);
    let mountain_battlefield = add_object(
        &mut engine,
        mountain,
        actor,
        mtgml_model::ZoneKind::Battlefield,
        12,
    );
    mtgml_state::validate_engine_state(&engine).unwrap();
    let mana = ManaStateV1 {
        pools: engine
            .core
            .players
            .keys()
            .map(|player| (*player, Default::default()))
            .collect(),
    };
    let turn_history = TurnHistoryStateV1 {
        turn_number: engine.core.turn_number,
        players: engine
            .core
            .players
            .keys()
            .map(|player| (*player, PlayerTurnHistoryV1::default()))
            .collect(),
        ..Default::default()
    };
    let faces = FaceStateV1 {
        faces: engine
            .zones
            .objects
            .keys()
            .map(|object| (*object, 0))
            .collect(),
    };
    let abilities = AbilityAuthorityStateV1 {
        by_instance: BTreeMap::from([(
            mtgml_model::AbilityInstanceId(1),
            AbilityAuthorityV1 {
                source: mountain_battlefield,
                ability_key: 0,
            },
        )]),
    };
    let card_rules = CardRulesAuthoritativeStateV1 {
        mana,
        turn_history,
        faces,
        abilities,
        ..Default::default()
    };
    let mut state = engine;
    state.card_rules = card_rules;
    state.allocators.next_ability_id = mtgml_model::AbilityInstanceId(2);
    for (player, identity) in &mut state.perspective_identities.players {
        let opaque = mtgml_model::OpaqueAbilityId(player.0);
        identity
            .ability_to_opaque
            .insert(mtgml_model::AbilityInstanceId(1), opaque);
        identity
            .opaque_to_ability
            .insert(opaque, mtgml_model::AbilityInstanceId(1));
        identity.next_opaque_ability_id.0 = identity.next_opaque_ability_id.0.max(player.0 + 1);
    }
    state
}
