use super::*;
use mtgml_card_ir::{
    BaseCharacteristicsV1, CardDefinitionEnvelopeV1, CardSemanticBindingV1,
    ContentContractManifestV1, DefinitionProvenanceRecordV1, FaceDefinitionV1, FaceKey,
    ProvenanceCatalogV1, SourceProvenanceV1, TypeLineV1, VerifiedContentCatalogV1,
};
use mtgml_model::{
    CapabilityRequirementV1, CardDefinitionId, ExecutionIdentityV1, ExecutionProgramV1,
    InformationStateDigestV2, RulesAuthorityV1, RulesContractManifestV1,
    SemanticContractManifestV1, VisibleSequence,
};
use mtgml_observation::{
    ObservationEnvelope, PlayerInformationStateV2, INFORMATION_STATE_SCHEMA_V2,
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
}
