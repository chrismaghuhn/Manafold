use mtgml_card_ir::{
    content_validation_only, decode_content_manifest_v1, encode_content_manifest_v1,
    validate_content_manifest_v1, BasicLandProfileV1, BasicLandSubtypeV1, CardProfileBodyV1,
    CardSemanticBindingV1, CardSemanticProfileId, DefinitionProvenanceRecordV1,
    NoExecutableProfileAdmitted, ProvenanceCatalogV1, RequiredCapabilityLifecycleV1,
    SourceProvenanceV1,
};
use mtgml_model::CardDefinitionId;
use mtgml_persistence::content_contract_digest::calculate_content_contract_id_v1;

const BASIC_LAND_KAT: &[u8] =
    include_bytes!("../../../persistence/golden/content-contract-basic-land-v1-kat.v1.json");

fn fixture_json() -> serde_json::Value {
    serde_json::from_slice(BASIC_LAND_KAT).expect("frozen Phase-2 KAT is valid JSON")
}

fn hex_bytes(value: &str) -> Vec<u8> {
    assert_eq!(value.len() % 2, 0);
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let digit = |value: u8| match value {
                b'0'..=b'9' => value - b'0',
                b'a'..=b'f' => value - b'a' + 10,
                _ => panic!("KAT hex must be lowercase hexadecimal"),
            };
            digit(pair[0]) * 16 + digit(pair[1])
        })
        .collect()
}

fn kat_manifest_bytes() -> Vec<u8> {
    hex_bytes(
        fixture_json()["canonical_payload_hex"]
            .as_str()
            .expect("KAT carries canonical payload bytes"),
    )
}

fn replace_profile_id(
    binding: &mtgml_persistence::cbor::Value,
    id: &str,
) -> mtgml_persistence::cbor::Value {
    use mtgml_persistence::cbor::Value;

    let mut binding = binding.clone();
    let Value::Array(outer) = &mut binding else {
        unreachable!()
    };
    let Value::Array(profile) = &mut outer[1] else {
        unreachable!()
    };
    profile[0] = Value::Text(id.to_owned());
    binding
}

#[test]
fn profiled_mountain_and_plains_match_frozen_content_kat_and_v1_identity() {
    let bytes = kat_manifest_bytes();
    let manifest = decode_content_manifest_v1(&bytes).expect("profile KAT decodes");
    assert_eq!(manifest.definitions.len(), 2);
    assert_eq!(
        manifest.definitions[0].semantic_binding,
        CardSemanticBindingV1::ProfiledV1 {
            profile_id: CardSemanticProfileId::parse("basic-land@1.0.0").unwrap(),
            body: CardProfileBodyV1::BasicLand(BasicLandProfileV1 {
                subtype: BasicLandSubtypeV1::Mountain,
            }),
        }
    );
    assert_eq!(
        manifest.definitions[1].semantic_binding,
        CardSemanticBindingV1::ProfiledV1 {
            profile_id: CardSemanticProfileId::parse("basic-land@1.0.0").unwrap(),
            body: CardProfileBodyV1::BasicLand(BasicLandProfileV1 {
                subtype: BasicLandSubtypeV1::Plains,
            }),
        }
    );
    assert_eq!(encode_content_manifest_v1(&manifest).unwrap(), bytes);

    let id = calculate_content_contract_id_v1(&bytes).unwrap();
    assert_eq!(
        id.as_str(),
        fixture_json()["content_contract_id"]
            .as_str()
            .expect("frozen KAT carries content identity")
    );
}

#[test]
fn profile_validation_is_structural_and_does_not_dispatch_by_card_name() {
    let bytes = kat_manifest_bytes();
    let mut manifest = decode_content_manifest_v1(&bytes).unwrap();
    manifest.definitions[0].faces[0].base_characteristics.name = "Renamed test card".to_owned();
    assert!(validate_content_manifest_v1(&manifest).is_ok());
    let renamed_bytes = encode_content_manifest_v1(&manifest).unwrap();
    assert_ne!(
        calculate_content_contract_id_v1(&renamed_bytes).unwrap(),
        calculate_content_contract_id_v1(&bytes).unwrap()
    );

    manifest.definitions[0].faces[0]
        .base_characteristics
        .type_line
        .subtypes = vec!["Plains".to_owned()];
    assert!(validate_content_manifest_v1(&manifest).is_err());
}

#[test]
fn content_validation_accepts_profile_but_gameplay_construction_stays_closed() {
    let bytes = kat_manifest_bytes();
    let content_id = calculate_content_contract_id_v1(&bytes).unwrap();
    let provenance = ProvenanceCatalogV1 {
        schema_version: "definition-provenance-catalog.v1".to_owned(),
        records: [CardDefinitionId(1), CardDefinitionId(2)]
            .into_iter()
            .map(|card_definition_id| DefinitionProvenanceRecordV1 {
                content_contract_id: content_id.clone(),
                card_definition_id,
                source_provenance: SourceProvenanceV1 {
                    source_snapshot_id: "fixture-snapshot".to_owned(),
                    source_record_id: format!("fixture-{card_definition_id:?}"),
                    source_record_codec_id: "oracle-record.v1".to_owned(),
                    source_record_digest: [0x5a; 32],
                },
            })
            .collect(),
    };
    let provenance_bytes = mtgml_card_ir::encode_provenance_catalog_v1(&provenance).unwrap();
    let report = content_validation_only(
        &bytes,
        &content_id,
        &provenance_bytes,
        &[CardDefinitionId(1), CardDefinitionId(2)],
        RequiredCapabilityLifecycleV1::Proposed,
    )
    .expect("structurally valid profiled content is recognized");

    assert_eq!(
        mtgml_card_ir::construct_gameplay_from_content(&report),
        Err(NoExecutableProfileAdmitted)
    );
}

#[test]
fn profile_codec_rejects_unknown_profile_body_and_bad_shape() {
    use mtgml_persistence::cbor::{self, Value};

    let original = kat_manifest_bytes();
    let mut value = cbor::decode_canonical(&original).unwrap();
    let Value::Array(root) = &mut value else {
        unreachable!()
    };
    let Value::Array(definitions) = &mut root[2] else {
        unreachable!()
    };
    let Value::Array(definition) = &mut definitions[0] else {
        unreachable!()
    };

    let original_binding = definition[4].clone();
    definition[4] = replace_profile_id(&original_binding, "future-profile@1.0.0");
    let unknown_profile_value = value.clone();
    let unknown_profile = cbor::encode_canonical(&unknown_profile_value).unwrap();
    assert!(decode_content_manifest_v1(&unknown_profile).is_err());

    let Value::Array(root) = &mut value else {
        unreachable!()
    };
    let Value::Array(definitions) = &mut root[2] else {
        unreachable!()
    };
    let Value::Array(definition) = &mut definitions[0] else {
        unreachable!()
    };
    definition[4] = original_binding.clone();
    let Value::Array(binding) = &mut definition[4] else {
        unreachable!()
    };
    let Value::Array(profile) = &mut binding[1] else {
        unreachable!()
    };
    profile[1] = Value::Array(vec![Value::Text("unknown-body.v1".to_owned())]);
    let unknown_body = cbor::encode_canonical(&value).unwrap();
    assert!(decode_content_manifest_v1(&unknown_body).is_err());

    assert!(decode_content_manifest_v1(&original[..original.len() - 1]).is_err());
    let mut trailing = original;
    trailing.push(0);
    assert!(decode_content_manifest_v1(&trailing).is_err());

    let canonical = kat_manifest_bytes();
    let mut noncanonical = canonical.clone();
    assert_eq!(&noncanonical[1..3], &[0x78, 0x1c]);
    noncanonical.splice(1..3, [0x79, 0x00, 0x1c]);
    assert!(decode_content_manifest_v1(&noncanonical).is_err());
}

#[test]
fn profile_requires_exact_single_face_and_local_ability_shape() {
    let bytes = kat_manifest_bytes();
    let mut manifest = decode_content_manifest_v1(&bytes).unwrap();

    let duplicate_face = manifest.definitions[0].faces[0].clone();
    manifest.definitions[0].faces.push(duplicate_face);
    assert!(validate_content_manifest_v1(&manifest).is_err());

    let mut manifest = decode_content_manifest_v1(&bytes).unwrap();
    manifest.definitions[0].ability_identities[0].ability_key = mtgml_card_ir::AbilityKey(1);
    assert!(validate_content_manifest_v1(&manifest).is_err());
}
