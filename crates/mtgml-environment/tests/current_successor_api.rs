#![cfg(not(feature = "historical-conformance-runtime"))]

use std::collections::BTreeMap;

use mtgml_card_ir::{
    admit_executable_profile_v1, decode_content_manifest_v1, CardSemanticBindingV1,
    ExecutableProfileAdmissionV1,
};
use mtgml_decision::{DecisionAnswerV2, DecisionResponseV3};
use mtgml_environment::{
    submit_response_bytes, BasicLandReplayV8ExecutionReport, BasicLandRuntimeOutputV8,
    ControllerError, CurrentPlayerStep, EnvironmentBackend, EnvironmentCheckpointV8,
    PlayerEndpoint, PlayerEndpointError, TrustedEnvironmentController,
};
use mtgml_model::{
    CapabilityRequirementV1, CardDefinitionId, ExecutionIdentityV1, ExecutionProgramV1, PlayerId,
    RulesAuthorityV1, RulesContractManifestV1, SemanticContractManifestV1,
};
use mtgml_observation::{ObservationEnvelopeV2, PlayerInformationStateV3, PlayerStepV4};
use mtgml_replay::{
    AuthoritativeReplayV8, ContentContractMaterialV1, InitialEnvironmentIdentityV8,
    ReplayManifestV8, SemanticContractMaterialV7,
};
use mtgml_state::{
    AbilityAuthorityStateV1, AbilityAuthorityV1, CardRulesAuthoritativeStateV1, EngineStatePartsV2,
    EngineStatePartsV3, FaceStateV1, ManaStateV1, PlayerTurnHistoryV1, TurnHistoryStateV1,
    TurnPosition, VisibilityPartition, ZoneKey, ZoneLocation, ZonePosition,
};

const CONTENT: &[u8] =
    include_bytes!("../../../cards/definitions/basic-land-v1/content-contract.v1.cbor");
const PROVENANCE: &[u8] =
    include_bytes!("../../../cards/definitions/basic-land-v1/provenance.v1.cbor");
const RULES_SNAPSHOT: &str = "wotc-cr-2026-09-25-txt-20260925-sha256-8d860e451f20f38865b725b42d82feb714c725373dd8f3b32b8652b3eeb070ca";

struct ProductionAliasProbe {
    step: PlayerStepV4,
}

impl EnvironmentBackend for ProductionAliasProbe {
    fn players(&self) -> Vec<PlayerId> {
        vec![self.step.information_state.perspective]
    }

    fn checkpoint(&self) -> Result<EnvironmentCheckpointV8, ControllerError> {
        Err(ControllerError::SemanticContractUnsupported)
    }

    fn restore(&mut self, _: EnvironmentCheckpointV8) -> Result<(), ControllerError> {
        Err(ControllerError::SemanticContractUnsupported)
    }

    fn fork_boxed(&self) -> Result<Box<dyn EnvironmentBackend>, ControllerError> {
        Err(ControllerError::SemanticContractUnsupported)
    }

    fn export_replay(&self) -> Result<AuthoritativeReplayV8, ControllerError> {
        Err(ControllerError::SemanticContractUnsupported)
    }

    fn execute_replay(
        &self,
        _: AuthoritativeReplayV8,
    ) -> Result<BasicLandReplayV8ExecutionReport, ControllerError> {
        Err(ControllerError::SemanticContractUnsupported)
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
    let admission = basic_land_admission();
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
        replay_manifest(&admission, &checkpoint),
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

fn basic_land_admission() -> ExecutableProfileAdmissionV1 {
    let content_id =
        mtgml_persistence::content_contract_digest::calculate_content_contract_id_v1(CONTENT)
            .unwrap();
    let closure = [
        "rules/basic-land-mana",
        "rules/basic-priority",
        "rules/cleanup-reset",
        "rules/combat-phase",
        "rules/declare-attackers",
        "rules/draw-card",
        "rules/land-play",
        "rules/mana-pool",
        "rules/state-based-actions-combat",
        "rules/turn-structure",
        "rules/zone-incarnation",
    ]
    .into_iter()
    .map(|key| CapabilityRequirementV1 {
        key: key.to_owned(),
        version: "0.1.0".to_owned(),
    })
    .collect();
    let rules = RulesContractManifestV1 {
        rules_authority: RulesAuthorityV1::ComprehensiveRules {
            snapshot_id: RULES_SNAPSHOT.to_owned(),
        },
        capability_closure: Some(closure),
    };
    let semantic = SemanticContractManifestV1 {
        rules_contract_id:
            mtgml_persistence::semantic_contract_digest::calculate_rules_contract_id_v1(&rules)
                .unwrap(),
        format_contract_id: None,
        content_contract_id: Some(content_id.clone()),
    };
    let execution = ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::MagicRules,
        semantic_contract_id:
            mtgml_persistence::semantic_contract_digest::calculate_semantic_contract_id_v1(
                &semantic,
            )
            .unwrap(),
    };
    admit_executable_profile_v1(
        CONTENT,
        &content_id,
        PROVENANCE,
        &rules,
        &semantic,
        &execution,
    )
    .unwrap()
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

fn basic_land_state() -> EngineStatePartsV3 {
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
    let mut state = EngineStatePartsV2::from_state(&engine, card_rules);
    state.predecessor_v5.allocators.next_ability_id = mtgml_model::AbilityInstanceId(2);
    for (player, identity) in &mut state.predecessor_v5.perspective_identities.players {
        let opaque = mtgml_model::OpaqueAbilityId(player.0);
        identity
            .ability_to_opaque
            .insert(mtgml_model::AbilityInstanceId(1), opaque);
        identity
            .opaque_to_ability
            .insert(opaque, mtgml_model::AbilityInstanceId(1));
        identity.next_opaque_ability_id.0 = identity.next_opaque_ability_id.0.max(player.0 + 1);
    }
    EngineStatePartsV3::new(
        state.predecessor_v5,
        Default::default(),
        state.card_rules_state,
    )
    .unwrap()
}

fn checkpoint_identity(checkpoint: &EnvironmentCheckpointV8) -> InitialEnvironmentIdentityV8 {
    InitialEnvironmentIdentityV8 {
        state_revision: checkpoint.state.predecessor_v5.revision,
        full_state_digest: checkpoint.state_digest.clone(),
        episode_status: checkpoint.status.clone(),
        environment_limit_counters: checkpoint.limit_counters.clone(),
        checkpoint_codec_identity: checkpoint.codec.clone(),
        checkpoint_digest: checkpoint.checkpoint_digest.clone(),
        execution_identity: checkpoint.execution_identity.clone(),
    }
}

fn replay_manifest(
    admission: &ExecutableProfileAdmissionV1,
    checkpoint: &EnvironmentCheckpointV8,
) -> ReplayManifestV8 {
    let mut manifest: ReplayManifestV8 = serde_json::from_str(include_str!(
        "../../../schemas/examples/replay-manifest-v8.json"
    ))
    .unwrap();
    manifest.execution_identity = admission.execution_identity().clone();
    manifest.semantic_contract = SemanticContractMaterialV7 {
        semantic_contract_id: admission.semantic_contract_id().clone(),
        manifest: admission.semantic_contract_manifest().clone(),
        rules_manifest: admission.rules_contract_manifest().clone(),
        content_contract: Some(
            ContentContractMaterialV1::from_manifest(decode_content_manifest_v1(CONTENT).unwrap())
                .unwrap(),
        ),
    };
    let mut second_deck = manifest.decks[0].clone();
    second_deck.player = PlayerId(2);
    second_deck.deck_id = "deck:synthetic-p2".to_owned();
    manifest.decks.push(second_deck);
    manifest.rules_snapshot = RULES_SNAPSHOT.to_owned();
    manifest.card_bundle = admission.content_contract_id().to_string();
    manifest.randomness.root_seed_hex = checkpoint
        .state
        .predecessor_v5
        .random
        .root_seed
        .to_lower_hex();
    manifest.initial_identity = checkpoint_identity(checkpoint);
    manifest
}
