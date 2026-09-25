use mtgml_card_ir::{
    decode_content_manifest_v1, encode_content_manifest_v1, validate_content_manifest_v1,
    BaseCharacteristicsV1, CardDefinitionEnvelopeV1, CardSemanticBindingV1,
    ContentContractManifestV1, DefinitionProvenanceRecordV1, DefinitionReferenceV1,
    FaceDefinitionV1, FaceKey, ManaColorV1, PrintedManaSymbolV1, ProvenanceCatalogV1,
    SourceProvenanceV1, TypeLineV1, VerifiedContentCatalogV1,
};
use mtgml_model::{CardDefinitionId, ContentContractIdV1};
use mtgml_persistence::content_contract_digest::calculate_content_contract_id_v1;

fn minimal_manifest() -> ContentContractManifestV1 {
    ContentContractManifestV1 {
        schema_version: "content-contract-manifest.v1".to_owned(),
        definitions: vec![CardDefinitionEnvelopeV1 {
            envelope_version: "card-definition-envelope.v1".to_owned(),
            card_definition_id: CardDefinitionId(1),
            faces: vec![FaceDefinitionV1 {
                face_key: FaceKey(0),
                base_characteristics: BaseCharacteristicsV1 {
                    name: "Fixture".to_owned(),
                    mana_cost: None,
                    color_indicator: vec![],
                    type_line: TypeLineV1 {
                        supertypes: vec![],
                        card_types: vec!["Creature".to_owned()],
                        subtypes: vec![],
                    },
                    power_toughness: Some((1, 1)),
                    loyalty: None,
                    defense: None,
                },
            }],
            ability_identities: vec![],
            semantic_binding: CardSemanticBindingV1::UnprofiledV1,
            definition_references: vec![],
            explicit_additional_requirements: vec![],
        }],
    }
}

#[test]
fn minimal_unprofiled_definition_is_structurally_valid() {
    assert!(validate_content_manifest_v1(&minimal_manifest()).is_ok());
}

#[test]
fn duplicate_face_identity_is_rejected() {
    let mut manifest = minimal_manifest();
    let duplicate = manifest.definitions[0].faces[0].clone();
    manifest.definitions[0].faces.push(duplicate);
    assert!(validate_content_manifest_v1(&manifest).is_err());
}

#[test]
fn content_manifest_encoding_matches_the_normative_known_answer() {
    let bytes = mtgml_card_ir::encode_content_manifest_v1(&minimal_manifest()).unwrap();
    let expected = include_bytes!("../../../persistence/golden/content-contract-minimal-v1.cbor");
    assert_eq!(bytes, expected);
    assert_eq!(
        decode_content_manifest_v1(&bytes).unwrap(),
        minimal_manifest()
    );
}

#[test]
fn ordered_hybrid_symbols_round_trip_as_printed_characteristics() {
    let mut manifest = minimal_manifest();
    manifest.definitions[0].faces[0]
        .base_characteristics
        .mana_cost = Some(vec![
        PrintedManaSymbolV1::Hybrid(ManaColorV1::Green, ManaColorV1::White),
        PrintedManaSymbolV1::Hybrid(ManaColorV1::White, ManaColorV1::Green),
        PrintedManaSymbolV1::Hybrid(ManaColorV1::White, ManaColorV1::Blue),
        PrintedManaSymbolV1::Hybrid(ManaColorV1::Blue, ManaColorV1::White),
        PrintedManaSymbolV1::Hybrid(ManaColorV1::White, ManaColorV1::Blue),
    ]);
    let bytes = encode_content_manifest_v1(&manifest).unwrap();
    assert_eq!(decode_content_manifest_v1(&bytes).unwrap(), manifest);
}

#[test]
fn invalid_hybrid_and_zero_generic_symbols_fail_closed() {
    let mut manifest = minimal_manifest();
    manifest.definitions[0].faces[0]
        .base_characteristics
        .mana_cost = Some(vec![PrintedManaSymbolV1::Hybrid(
        ManaColorV1::Green,
        ManaColorV1::Green,
    )]);
    assert!(validate_content_manifest_v1(&manifest).is_err());
    manifest.definitions[0].faces[0]
        .base_characteristics
        .mana_cost = Some(vec![PrintedManaSymbolV1::Generic(0)]);
    assert!(validate_content_manifest_v1(&manifest).is_err());
}

fn provenance(
    id: &ContentContractIdV1,
    definition: CardDefinitionId,
) -> DefinitionProvenanceRecordV1 {
    DefinitionProvenanceRecordV1 {
        content_contract_id: id.clone(),
        card_definition_id: definition,
        source_provenance: SourceProvenanceV1 {
            source_snapshot_id: "snapshot/test-1".to_owned(),
            source_record_id: format!("record/{definition}"),
            source_record_codec_id: "test-record.v1".to_owned(),
            source_record_digest: [0x5a; 32],
        },
    }
}

fn provenance_catalog(
    id: &ContentContractIdV1,
    definitions: &[CardDefinitionId],
) -> ProvenanceCatalogV1 {
    ProvenanceCatalogV1 {
        schema_version: "definition-provenance-catalog.v1".to_owned(),
        records: definitions
            .iter()
            .map(|definition| provenance(id, *definition))
            .collect(),
    }
}

#[test]
fn verified_catalog_is_content_scoped_and_provenance_excluded_from_digest() {
    let manifest = minimal_manifest();
    let bytes = encode_content_manifest_v1(&manifest).unwrap();
    let id = calculate_content_contract_id_v1(&bytes).unwrap();
    let catalog = VerifiedContentCatalogV1::build(
        &bytes,
        &id,
        provenance_catalog(&id, &[CardDefinitionId(1)]),
    )
    .unwrap();
    assert!(catalog.get(&id, CardDefinitionId(1)).is_ok());
    let other = ContentContractIdV1::parse("00".repeat(32)).unwrap();
    assert!(catalog.get(&other, CardDefinitionId(1)).is_err());
    assert_eq!(calculate_content_contract_id_v1(&bytes).unwrap(), id);
}

#[test]
fn deterministic_definition_closure_follows_only_same_catalog_edges() {
    let mut manifest = minimal_manifest();
    let mut second = manifest.definitions[0].clone();
    second.card_definition_id = CardDefinitionId(2);
    second.definition_references.clear();
    manifest.definitions[0]
        .definition_references
        .push(DefinitionReferenceV1 {
            relation: "required_definition".to_owned(),
            target: CardDefinitionId(2),
            target_face_key: Some(FaceKey(0)),
        });
    manifest.definitions.push(second);
    let bytes = encode_content_manifest_v1(&manifest).unwrap();
    let id = calculate_content_contract_id_v1(&bytes).unwrap();
    let catalog = VerifiedContentCatalogV1::build(
        &bytes,
        &id,
        provenance_catalog(&id, &[CardDefinitionId(1), CardDefinitionId(2)]),
    )
    .unwrap();
    assert_eq!(
        catalog
            .close_definition_roots(&id, &[CardDefinitionId(1)])
            .unwrap(),
        vec![CardDefinitionId(1), CardDefinitionId(2)]
    );
}
