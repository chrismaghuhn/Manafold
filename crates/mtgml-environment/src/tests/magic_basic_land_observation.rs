use super::*;
use mtgml_card_ir::{
    BaseCharacteristicsV1, CardDefinitionEnvelopeV1, CardSemanticBindingV1,
    ContentContractManifestV1, DefinitionProvenanceRecordV1, FaceDefinitionV1, FaceKey,
    ProvenanceCatalogV1, SourceProvenanceV1, TypeLineV1, VerifiedContentCatalogV1,
};
use mtgml_decision::{
    CandidateIntentV3, DecisionDomainV2, DecisionVisibility, PlayerDecisionRequestV3,
    VisibleCandidateV3, PLAYER_DECISION_REQUEST_V3_SCHEMA,
};
use mtgml_model::{
    CandidateIdV1, CapabilityRequirementV1, CardDefinitionId, ExecutionIdentityV1,
    ExecutionProgramV1, InformationStateDigestV2, PlayerDecisionIdV1, RulesAuthorityV1,
    RulesContractManifestV1, SemanticContractManifestV1, VisibleSequence,
};
use mtgml_observation::{
    ObservationEnvelope, PlayerInformationStateV2, PlayerStepV3, INFORMATION_STATE_SCHEMA_V2,
};
use mtgml_state::{
    AbilityAuthorityStateV1, AttachmentStateV1, CardRulesAuthoritativeStateV1, CounterKindV1,
    CounterStateV1, EngineStatePartsV2, FaceStateV1, ManaPoolV1, ManaStateV1, PlayerTurnHistoryV1,
    TurnHistoryStateV1,
};
use std::collections::BTreeMap;

fn catalog_for(definition_ids: &[u64]) -> VerifiedContentCatalogV1 {
    let definitions = definition_ids
        .iter()
        .map(|id| CardDefinitionEnvelopeV1 {
            envelope_version: "card-definition-envelope.v1".to_owned(),
            card_definition_id: CardDefinitionId(*id),
            faces: vec![FaceDefinitionV1 {
                face_key: FaceKey(0),
                base_characteristics: BaseCharacteristicsV1 {
                    name: format!("Fixture {id}"),
                    mana_cost: None,
                    color_indicator: vec![],
                    type_line: TypeLineV1 {
                        supertypes: vec!["Basic".into()],
                        card_types: vec!["Land".into()],
                        subtypes: vec!["Plains".into()],
                    },
                    power_toughness: None,
                    loyalty: None,
                    defense: None,
                },
            }],
            ability_identities: vec![],
            semantic_binding: CardSemanticBindingV1::UnprofiledV1,
            definition_references: vec![],
            explicit_additional_requirements: vec![],
        })
        .collect();
    let manifest = ContentContractManifestV1 {
        schema_version: "content-contract-manifest.v1".into(),
        definitions,
    };
    let bytes = mtgml_card_ir::encode_content_manifest_v1(&manifest).unwrap();
    let id = mtgml_persistence::content_contract_digest::calculate_content_contract_id_v1(&bytes)
        .unwrap();
    let provenance = ProvenanceCatalogV1 {
        schema_version: "definition-provenance-catalog.v1".into(),
        records: definition_ids
            .iter()
            .map(|definition| DefinitionProvenanceRecordV1 {
                content_contract_id: id.clone(),
                card_definition_id: CardDefinitionId(*definition),
                source_provenance: SourceProvenanceV1 {
                    source_snapshot_id: "snapshot/test-1".into(),
                    source_record_id: format!("record/{definition}"),
                    source_record_codec_id: "test-fixture.v1".into(),
                    source_record_digest: [0x5a; 32],
                },
            })
            .collect(),
    };
    VerifiedContentCatalogV1::build(&bytes, &id, provenance).unwrap()
}

fn execution_authority(
    catalog: &VerifiedContentCatalogV1,
) -> (
    ExecutionIdentityV1,
    SemanticContractManifestV1,
    RulesContractManifestV1,
) {
    let rules = RulesContractManifestV1 {
        rules_authority: RulesAuthorityV1::ComprehensiveRules {
            snapshot_id: "rules-fixture-snapshot".into(),
        },
        capability_closure: Some(vec![CapabilityRequirementV1 {
            key: "rules/land-play".into(),
            version: "0.1.0".into(),
        }]),
    };
    let rules_id =
        mtgml_persistence::semantic_contract_digest::calculate_rules_contract_id_v1(&rules)
            .unwrap();
    let semantic = SemanticContractManifestV1 {
        rules_contract_id: rules_id,
        format_contract_id: None,
        content_contract_id: Some(catalog.content_contract_id().clone()),
    };
    let semantic_id =
        mtgml_persistence::semantic_contract_digest::calculate_semantic_contract_id_v1(&semantic)
            .unwrap();
    (
        ExecutionIdentityV1 {
            program_kind: ExecutionProgramV1::MagicRules,
            semantic_contract_id: semantic_id,
        },
        semantic,
        rules,
    )
}

fn basic_land_parts(root_seed: mtgml_random::RootSeed256) -> EngineStatePartsV2 {
    let mut state =
        mtgml_state::construct_synthetic_engine_state(mtgml_state::SyntheticResetInputs {
            players: [PlayerId(1), PlayerId(2)],
            root_seed,
            setup: mtgml_state::SyntheticV4Setup::synthetic_compatibility(),
        })
        .unwrap();
    for player in [PlayerId(1), PlayerId(2)] {
        let identity = state
            .perspective_identities
            .players
            .get_mut(&player)
            .unwrap();
        for (object, location) in &state.zones.locations {
            if location.zone == mtgml_model::ZoneKind::Battlefield
                && !identity.object_to_opaque.contains_key(object)
            {
                let opaque = identity.next_opaque_object_id;
                identity.next_opaque_object_id.0 += 1;
                identity.object_to_opaque.insert(*object, opaque);
                identity.opaque_to_object.insert(opaque, *object);
            }
        }
    }
    let mut mana = ManaStateV1::default();
    let mut history = BTreeMap::new();
    for player in [PlayerId(1), PlayerId(2)] {
        let mut pool = ManaPoolV1::default();
        pool.unrestricted[3] = u32::from(player == PlayerId(1));
        mana.pools.insert(player, pool);
        history.insert(player, PlayerTurnHistoryV1::default());
    }
    let mut faces = FaceStateV1::default();
    for object in state.zones.objects.keys() {
        faces.faces.insert(*object, 0);
    }
    let mut counters = CounterStateV1::default();
    if let Some((object, _)) = state
        .zones
        .locations
        .iter()
        .find(|(_, location)| location.zone == mtgml_model::ZoneKind::Battlefield)
    {
        counters.counters.insert(
            *object,
            BTreeMap::from([(CounterKindV1::PlusOnePlusOne, 2)]),
        );
    }
    EngineStatePartsV2::from_state(
        &state,
        CardRulesAuthoritativeStateV1 {
            mana,
            turn_history: TurnHistoryStateV1 {
                turn_number: state.core.turn_number,
                players: history,
                ..TurnHistoryStateV1::default()
            },
            counters,
            attachments: AttachmentStateV1::default(),
            faces,
            abilities: AbilityAuthorityStateV1::default(),
        },
    )
}

#[test]
fn magic_basic_land_projection_uses_opaque_public_ids_and_verified_content_faces() {
    let parts = basic_land_parts(seed());
    parts.validate().unwrap();
    let catalog = catalog_for(&[1, 2]);
    let (identity, semantic, rules) = execution_authority(&catalog);
    let projected = crate::player_projection::project_magic_basic_land_observation_v1(
        &parts,
        PlayerId(1),
        &identity,
        &semantic,
        &rules,
        &catalog,
    )
    .unwrap();
    assert_eq!(
        projected.payload_codec,
        mtgml_observation::MAGIC_BASIC_LAND_OBSERVATION_SCHEMA_V1
    );
    let payload = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        projected.payload_base64,
    )
    .unwrap();
    let value: serde_json::Value = serde_json::from_slice(&payload).unwrap();
    assert_eq!(value["mana_pools"][0]["unrestricted"][3], 1);
    assert_eq!(value["counters"][0]["object"], "1");
    assert_eq!(value["faces"][0]["face"], "front");
    let encoded = String::from_utf8(payload).unwrap();
    for forbidden in [
        "game_object_id",
        "physical_card_id",
        "face_key",
        "card_definition_id",
    ] {
        assert!(!encoded.contains(forbidden), "leaked {forbidden}");
    }
}

#[test]
fn hidden_rng_change_preserves_basic_land_observation_bytes_and_digest() {
    let a = basic_land_parts(seed());
    let other_seed = mtgml_random::RootSeed256::from_lower_hex(&"99".repeat(32)).unwrap();
    let b = basic_land_parts(other_seed);
    let catalog = catalog_for(&[1, 2]);
    let (identity, semantic, rules) = execution_authority(&catalog);
    let left = crate::player_projection::project_magic_basic_land_observation_v1(
        &a,
        PlayerId(1),
        &identity,
        &semantic,
        &rules,
        &catalog,
    )
    .unwrap();
    let right = crate::player_projection::project_magic_basic_land_observation_v1(
        &b,
        PlayerId(1),
        &identity,
        &semantic,
        &rules,
        &catalog,
    )
    .unwrap();
    assert_eq!(left.payload_base64, right.payload_base64);
    assert_eq!(left.digest, right.digest);
    let make_information = |observation: ObservationEnvelope| {
        let mut information = PlayerInformationStateV2 {
            schema_version: INFORMATION_STATE_SCHEMA_V2.into(),
            perspective: PlayerId(1),
            state_revision: observation.state_revision,
            current_observation: observation,
            next_visible_sequence: VisibleSequence(1),
            retained_knowledge: Vec::new(),
            digest: InformationStateDigestV2::from_canonical_bytes(b"placeholder"),
        };
        let (_, digest) =
            mtgml_wire::compute_information_state_digest_v2(&information.digest_input()).unwrap();
        information.digest = digest;
        information
    };
    let left_information = make_information(left);
    let right_information = make_information(right);
    assert_eq!(left_information.digest, right_information.digest);
    assert_eq!(
        mtgml_wire::encode_canonical(&left_information).unwrap(),
        mtgml_wire::encode_canonical(&right_information).unwrap()
    );

    // Candidate legality is deliberately outside this projection. Two
    // independently decoded, already supplied V3 requests with equivalent
    // public semantics remain equal when composed into paired PlayerSteps.
    let mut step_a: PlayerStepV3 = serde_json::from_str(include_str!(
        "../../../../schemas/examples/player-step-v3-event-next-decision.json"
    ))
    .unwrap();
    let mut step_b = step_a.clone();
    for (step, information) in [
        (&mut step_a, left_information),
        (&mut step_b, right_information),
    ] {
        step.information_state = information;
        step.observed_events.clear();
        let decision = step.next_decision.as_mut().unwrap();
        decision.actor = PlayerId(1);
        decision.state_revision = step.information_state.state_revision;
        step.validate().unwrap();
    }
    assert_eq!(step_a.next_decision, step_b.next_decision);
    assert_eq!(
        mtgml_wire::encode_canonical(&step_a).unwrap(),
        mtgml_wire::encode_canonical(&step_b).unwrap()
    );
}

#[test]
fn trusted_game_object_renaming_preserves_public_observation_and_information_bytes() {
    let original = basic_land_parts(seed());
    let mut renamed = original.clone();
    let state = renamed.materialize();
    let (old_id, location) = state
        .zones
        .locations
        .iter()
        .find(|(_, location)| location.zone == mtgml_model::ZoneKind::Battlefield)
        .map(|(id, location)| (*id, location.clone()))
        .expect("fixture has a battlefield object");
    let new_id = mtgml_model::GameObjectId(10_000);
    assert!(!state.zones.objects.contains_key(&new_id));

    // Keep every perspective's public identity stable while changing the
    // authoritative GameObjectId and all references to that incarnation.
    for identity in renamed
        .predecessor_v5
        .perspective_identities
        .players
        .values_mut()
    {
        if let Some(opaque) = identity.object_to_opaque.remove(&old_id) {
            identity.object_to_opaque.insert(new_id, opaque);
            assert_eq!(
                identity.opaque_to_object.insert(opaque, new_id),
                Some(old_id)
            );
        }
    }
    let object = renamed
        .predecessor_v5
        .zones
        .objects
        .remove(&old_id)
        .expect("fixture object exists");
    let mut object = object;
    object.id = new_id;
    renamed.predecessor_v5.zones.objects.insert(new_id, object);
    renamed
        .predecessor_v5
        .zones
        .locations
        .remove(&old_id)
        .expect("fixture location exists");
    renamed
        .predecessor_v5
        .zones
        .locations
        .insert(new_id, location);
    for object_id in renamed
        .predecessor_v5
        .zones
        .ordered_zones
        .values_mut()
        .flat_map(|objects| objects.iter_mut())
    {
        if *object_id == old_id {
            *object_id = new_id;
        }
    }
    renamed.predecessor_v5.allocators.next_object_id = mtgml_model::GameObjectId(new_id.0 + 1);
    if let Some(pending) = renamed.predecessor_v5.execution.pending_decision.as_mut() {
        for candidate in &mut pending.request.candidates {
            match &mut candidate.trusted_binding {
                mtgml_decision::EngineCandidateBinding::SelectObject { object }
                    if *object == old_id =>
                {
                    *object = new_id;
                }
                _ => {}
            }
        }
    }
    if let Some(counts) = renamed.card_rules_state.counters.counters.remove(&old_id) {
        renamed
            .card_rules_state
            .counters
            .counters
            .insert(new_id, counts);
    }
    if let Some(face) = renamed.card_rules_state.faces.faces.remove(&old_id) {
        renamed.card_rules_state.faces.faces.insert(new_id, face);
    }

    original.validate().unwrap();
    mtgml_state::validate_engine_state(&renamed.materialize()).unwrap();
    renamed.validate().unwrap();

    let catalog = catalog_for(&[1, 2]);
    let (identity, semantic, rules) = execution_authority(&catalog);
    let project = |parts: &EngineStatePartsV2| {
        crate::player_projection::project_magic_basic_land_observation_v1(
            parts,
            PlayerId(1),
            &identity,
            &semantic,
            &rules,
            &catalog,
        )
        .unwrap()
    };
    let original_observation = project(&original);
    let renamed_observation = project(&renamed);
    assert_eq!(
        mtgml_wire::encode_canonical(&original_observation).unwrap(),
        mtgml_wire::encode_canonical(&renamed_observation).unwrap()
    );
    assert_eq!(original_observation.digest, renamed_observation.digest);

    let make_information = |observation: ObservationEnvelope| {
        let mut information = PlayerInformationStateV2 {
            schema_version: INFORMATION_STATE_SCHEMA_V2.into(),
            perspective: PlayerId(1),
            state_revision: observation.state_revision,
            current_observation: observation,
            next_visible_sequence: VisibleSequence(1),
            retained_knowledge: Vec::new(),
            digest: InformationStateDigestV2::from_canonical_bytes(b"placeholder"),
        };
        let (_, digest) =
            mtgml_wire::compute_information_state_digest_v2(&information.digest_input()).unwrap();
        information.digest = digest;
        information
    };
    let original_information = make_information(original_observation);
    let renamed_information = make_information(renamed_observation);
    assert_eq!(original_information.digest, renamed_information.digest);
    assert_eq!(
        mtgml_wire::encode_canonical(&original_information).unwrap(),
        mtgml_wire::encode_canonical(&renamed_information).unwrap()
    );

    let request_for = |parts: &EngineStatePartsV2, object: mtgml_model::GameObjectId| {
        let state = parts.materialize();
        let opaque = state.perspective_identities.players[&PlayerId(1)].object_to_opaque[&object];
        let request = PlayerDecisionRequestV3 {
            schema_version: PLAYER_DECISION_REQUEST_V3_SCHEMA.into(),
            player_decision_id: PlayerDecisionIdV1(1),
            state_revision: state.revision,
            actor: PlayerId(1),
            visibility: DecisionVisibility::Public,
            decision: DecisionDomainV2::ChooseOne,
            candidates: vec![VisibleCandidateV3 {
                candidate_id: CandidateIdV1(0),
                intent: CandidateIntentV3::SelectObject { object: opaque },
            }],
        };
        request.validate().unwrap();
        request
    };
    let request_a = request_for(&original, old_id);
    let request_b = request_for(&renamed, new_id);
    assert_eq!(request_a, request_b);

    let mut step_a: PlayerStepV3 = serde_json::from_str(include_str!(
        "../../../../schemas/examples/player-step-v3-event-next-decision.json"
    ))
    .unwrap();
    let mut step_b = step_a.clone();
    step_a.information_state = original_information;
    step_b.information_state = renamed_information;
    step_a.observed_events.clear();
    step_b.observed_events.clear();
    step_a.next_decision = Some(request_a);
    step_b.next_decision = Some(request_b);
    step_a.validate().unwrap();
    step_b.validate().unwrap();
    assert_eq!(
        mtgml_wire::encode_canonical(&step_a).unwrap(),
        mtgml_wire::encode_canonical(&step_b).unwrap()
    );
}
