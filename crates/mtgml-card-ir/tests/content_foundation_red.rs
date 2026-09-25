use mtgml_card_ir::{
    validate_content_manifest_v1, BaseCharacteristicsV1, CardDefinitionEnvelopeV1,
    CardSemanticBindingV1, ContentContractManifestV1, FaceDefinitionV1, FaceKey, TypeLineV1,
};
use mtgml_model::CardDefinitionId;

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
    manifest.definitions[0].faces.push(manifest.definitions[0].faces[0].clone());
    assert!(validate_content_manifest_v1(&manifest).is_err());
}

#[test]
fn content_manifest_encoding_matches_the_normative_known_answer() {
    let bytes = mtgml_card_ir::encode_content_manifest_v1(&minimal_manifest()).unwrap();
    let expected = include_bytes!("../../../persistence/golden/content-contract-minimal-v1.cbor");
    assert_eq!(bytes, expected);
}
