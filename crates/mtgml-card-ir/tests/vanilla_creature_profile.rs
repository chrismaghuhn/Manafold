use mtgml_card_ir::{
    decode_content_manifest_v1, encode_content_manifest_v1, validate_content_manifest_v1,
    AbilityIdentityV1, AbilityKey, BaseCharacteristicsV1, BasicLandProfileV1, BasicLandSubtypeV1,
    CardDefinitionEnvelopeV1, CardProfileBodyV1, CardSemanticBindingV1, CardSemanticProfileId,
    ContentContractManifestV1, ContentValidationErrorV1, FaceDefinitionV1, FaceKey, ManaColorV1,
    PrintedManaSymbolV1, TypeLineV1, BASIC_LAND_PROFILE_ID_V1, CARD_DEFINITION_ENVELOPE_V1,
    CONTENT_CONTRACT_MANIFEST_V1, VANILLA_CREATURE_PROFILE_BODY_V1, VANILLA_CREATURE_PROFILE_ID_V1,
};
use mtgml_model::CardDefinitionId;
use mtgml_persistence::cbor::{self, Value};
use mtgml_persistence::content_contract_digest::calculate_content_contract_id_v1;

const BASIC_LAND_CONTENT: &[u8] =
    include_bytes!("../../../cards/definitions/basic-land-v1/content-contract.v1.cbor");
const BASIC_LAND_PROVENANCE: &[u8] =
    include_bytes!("../../../cards/definitions/basic-land-v1/provenance.v1.cbor");
const BASIC_LAND_KAT: &[u8] =
    include_bytes!("../../../persistence/golden/content-contract-basic-land-v1-kat.v1.json");

fn savannah_lions() -> CardDefinitionEnvelopeV1 {
    CardDefinitionEnvelopeV1 {
        envelope_version: CARD_DEFINITION_ENVELOPE_V1.to_owned(),
        card_definition_id: CardDefinitionId(1),
        faces: vec![FaceDefinitionV1 {
            face_key: FaceKey(0),
            base_characteristics: BaseCharacteristicsV1 {
                name: "Savannah Lions".to_owned(),
                mana_cost: Some(vec![PrintedManaSymbolV1::White]),
                color_indicator: vec![],
                type_line: TypeLineV1 {
                    supertypes: vec![],
                    card_types: vec!["Creature".to_owned()],
                    subtypes: vec!["Cat".to_owned()],
                },
                power_toughness: Some((2, 1)),
                loyalty: None,
                defense: None,
            },
        }],
        ability_identities: vec![],
        semantic_binding: CardSemanticBindingV1::ProfiledV1 {
            profile_id: CardSemanticProfileId::parse(VANILLA_CREATURE_PROFILE_ID_V1).unwrap(),
            body: CardProfileBodyV1::VanillaCreature,
        },
        definition_references: vec![],
        explicit_additional_requirements: vec![],
    }
}

fn manifest_of(definition: CardDefinitionEnvelopeV1) -> ContentContractManifestV1 {
    ContentContractManifestV1 {
        schema_version: CONTENT_CONTRACT_MANIFEST_V1.to_owned(),
        definitions: vec![definition],
    }
}

fn validation_class(manifest: &ContentContractManifestV1) -> Result<(), ContentValidationErrorV1> {
    validate_content_manifest_v1(manifest).map_err(|diagnostic| diagnostic.class)
}

/// Replace the profiled payload `[profile_id, body]` of the only definition in
/// an encoded manifest and re-encode canonically.
fn with_profile_payload(bytes: &[u8], edit: impl FnOnce(&mut Vec<Value>)) -> Vec<u8> {
    let mut value = cbor::decode_canonical(bytes).unwrap();
    let Value::Array(root) = &mut value else {
        unreachable!()
    };
    let Value::Array(definitions) = &mut root[2] else {
        unreachable!()
    };
    let Value::Array(definition) = &mut definitions[0] else {
        unreachable!()
    };
    let Value::Array(binding) = &mut definition[4] else {
        unreachable!()
    };
    let Value::Array(profile) = &mut binding[1] else {
        unreachable!()
    };
    edit(profile);
    cbor::encode_canonical(&value).unwrap()
}

#[test]
fn a_vanilla_creature_definition_round_trips() {
    let manifest = manifest_of(savannah_lions());
    let bytes = encode_content_manifest_v1(&manifest).expect("Savannah Lions encodes");
    let decoded = decode_content_manifest_v1(&bytes).expect("Savannah Lions decodes");
    assert_eq!(decoded, manifest);
    assert_eq!(encode_content_manifest_v1(&decoded).unwrap(), bytes);
}

#[test]
fn the_vanilla_body_is_written_as_its_label_and_null() {
    let bytes = encode_content_manifest_v1(&manifest_of(savannah_lions())).unwrap();
    let mut payload = None;
    with_profile_payload(&bytes, |profile| payload = Some(profile.clone()));
    assert_eq!(
        payload.unwrap(),
        vec![
            Value::Text(VANILLA_CREATURE_PROFILE_ID_V1.to_owned()),
            Value::Array(vec![
                Value::Text(VANILLA_CREATURE_PROFILE_BODY_V1.to_owned()),
                Value::Null,
            ]),
        ]
    );
}

#[test]
fn a_vanilla_creature_gets_a_content_id() {
    let bytes = encode_content_manifest_v1(&manifest_of(savannah_lions())).unwrap();
    calculate_content_contract_id_v1(&bytes)
        .expect("the identity gate admits the vanilla-creature profile");
}

#[test]
fn a_vanilla_creature_with_rules_or_bad_shape_is_rejected() {
    type Edit = fn(&mut CardDefinitionEnvelopeV1);
    let cases: [(&str, Edit, ContentValidationErrorV1); 11] = [
        (
            "toughness 0",
            |d| d.faces[0].base_characteristics.power_toughness = Some((2, 0)),
            ContentValidationErrorV1::InvalidCharacteristic,
        ),
        (
            "negative power",
            |d| d.faces[0].base_characteristics.power_toughness = Some((-1, 1)),
            ContentValidationErrorV1::InvalidCharacteristic,
        ),
        (
            "no power and toughness",
            |d| d.faces[0].base_characteristics.power_toughness = None,
            ContentValidationErrorV1::InvalidCharacteristic,
        ),
        (
            "an ability identity",
            |d| {
                d.ability_identities = vec![AbilityIdentityV1 {
                    ability_key: AbilityKey(0),
                    face_key: FaceKey(0),
                }]
            },
            ContentValidationErrorV1::InvalidLocalReference,
        ),
        (
            "creature and artifact",
            |d| {
                d.faces[0].base_characteristics.type_line.card_types =
                    vec!["Creature".to_owned(), "Artifact".to_owned()]
            },
            ContentValidationErrorV1::InvalidCharacteristic,
        ),
        (
            "a supertype",
            |d| d.faces[0].base_characteristics.type_line.supertypes = vec!["Legendary".to_owned()],
            ContentValidationErrorV1::InvalidCharacteristic,
        ),
        (
            "no subtype",
            |d| d.faces[0].base_characteristics.type_line.subtypes = vec![],
            ContentValidationErrorV1::InvalidCharacteristic,
        ),
        (
            "no mana cost",
            |d| d.faces[0].base_characteristics.mana_cost = None,
            ContentValidationErrorV1::InvalidCharacteristic,
        ),
        (
            "a hybrid mana symbol",
            |d| {
                d.faces[0].base_characteristics.mana_cost = Some(vec![PrintedManaSymbolV1::Hybrid(
                    ManaColorV1::White,
                    ManaColorV1::Red,
                )])
            },
            ContentValidationErrorV1::InvalidCharacteristic,
        ),
        (
            "a color indicator",
            |d| d.faces[0].base_characteristics.color_indicator = vec![ManaColorV1::Red],
            ContentValidationErrorV1::InvalidCharacteristic,
        ),
        (
            "loyalty",
            |d| d.faces[0].base_characteristics.loyalty = Some(3),
            ContentValidationErrorV1::InvalidCharacteristic,
        ),
    ];
    for (name, edit, expected) in cases {
        let mut definition = savannah_lions();
        edit(&mut definition);
        assert_eq!(
            validation_class(&manifest_of(definition)),
            Err(expected),
            "{name}"
        );
    }

    let mut defense = savannah_lions();
    defense.faces[0].base_characteristics.defense = Some(1);
    assert_eq!(
        validation_class(&manifest_of(defense)),
        Err(ContentValidationErrorV1::InvalidCharacteristic),
        "defense"
    );

    let mut two_faces = savannah_lions();
    let mut second = two_faces.faces[0].clone();
    second.face_key = FaceKey(1);
    two_faces.faces.push(second);
    assert_eq!(
        validation_class(&manifest_of(two_faces)),
        Err(ContentValidationErrorV1::InvalidLocalIdentity),
        "two faces"
    );
}

#[test]
fn a_vanilla_creature_with_zero_power_is_accepted() {
    let mut definition = savannah_lions();
    definition.faces[0].base_characteristics.power_toughness = Some((0, 1));
    assert_eq!(validation_class(&manifest_of(definition)), Ok(()));
}

#[test]
fn the_profile_id_and_the_body_must_agree() {
    let mut creature_under_land_id = savannah_lions();
    creature_under_land_id.semantic_binding = CardSemanticBindingV1::ProfiledV1 {
        profile_id: CardSemanticProfileId::parse(BASIC_LAND_PROFILE_ID_V1).unwrap(),
        body: CardProfileBodyV1::VanillaCreature,
    };
    assert_eq!(
        validation_class(&manifest_of(creature_under_land_id)),
        Err(ContentValidationErrorV1::UnknownSemanticProfile)
    );

    let mut land_under_creature_id = savannah_lions();
    land_under_creature_id.semantic_binding = CardSemanticBindingV1::ProfiledV1 {
        profile_id: CardSemanticProfileId::parse(VANILLA_CREATURE_PROFILE_ID_V1).unwrap(),
        body: CardProfileBodyV1::BasicLand(BasicLandProfileV1 {
            subtype: BasicLandSubtypeV1::Mountain,
        }),
    };
    assert_eq!(
        validation_class(&manifest_of(land_under_creature_id)),
        Err(ContentValidationErrorV1::UnknownSemanticProfile)
    );
}

#[test]
fn the_decoder_rejects_a_malformed_vanilla_body() {
    let bytes = encode_content_manifest_v1(&manifest_of(savannah_lions())).unwrap();
    let body_label = |label: &str, payload: Value| {
        with_profile_payload(&bytes, |profile| {
            profile[1] = Value::Array(vec![Value::Text(label.to_owned()), payload])
        })
    };

    for (name, malformed) in [
        (
            "unknown body label",
            body_label("vanilla-creature-profile.v2", Value::Null),
        ),
        (
            "the basic-land body label",
            body_label("basic-land-profile.v1", Value::Null),
        ),
        (
            "a text payload",
            body_label(
                VANILLA_CREATURE_PROFILE_BODY_V1,
                Value::Text("mountain".to_owned()),
            ),
        ),
        (
            "a one-element body",
            with_profile_payload(&bytes, |profile| {
                profile[1] = Value::Array(vec![Value::Text(
                    VANILLA_CREATURE_PROFILE_BODY_V1.to_owned(),
                )])
            }),
        ),
    ] {
        assert!(decode_content_manifest_v1(&malformed).is_err(), "{name}");
        assert!(
            calculate_content_contract_id_v1(&malformed).is_err(),
            "{name} must not get a content id"
        );
    }
}

#[test]
fn the_basic_land_bytes_are_unchanged() {
    let kat: serde_json::Value = serde_json::from_slice(BASIC_LAND_KAT).unwrap();
    let expected_id = kat["content_contract_id"].as_str().unwrap();
    assert!(expected_id.starts_with("80d26c18"));

    let manifest = decode_content_manifest_v1(BASIC_LAND_CONTENT)
        .expect("the committed basic-land content decodes");
    assert_eq!(
        encode_content_manifest_v1(&manifest).unwrap(),
        BASIC_LAND_CONTENT
    );
    assert_eq!(
        calculate_content_contract_id_v1(BASIC_LAND_CONTENT)
            .unwrap()
            .as_str(),
        expected_id
    );
    assert!(manifest.definitions.iter().all(|definition| matches!(
        &definition.semantic_binding,
        CardSemanticBindingV1::ProfiledV1 {
            body: CardProfileBodyV1::BasicLand(_),
            ..
        }
    )));
}

#[test]
fn a_vanilla_creature_is_not_admitted_as_the_basic_land_profile() {
    use mtgml_card_ir::{
        admit_executable_profile_v1, decode_provenance_catalog_v1, encode_provenance_catalog_v1,
        ContentPreflightErrorV1,
    };
    use mtgml_model::{
        CapabilityRequirementV1, ExecutionIdentityV1, ExecutionProgramV1, RulesAuthorityV1,
        RulesContractManifestV1, SemanticContractManifestV1,
    };
    use mtgml_persistence::semantic_contract_digest::{
        calculate_rules_contract_id_v1, calculate_semantic_contract_id_v1,
    };

    // Two vanilla creatures under the ids and Oracle records that the pinned
    // basic-land content uses: the profile body, not the id or provenance,
    // decides, and a creature is never a Mountain or a Plains.
    let mut second = savannah_lions();
    second.card_definition_id = CardDefinitionId(2);
    let manifest = ContentContractManifestV1 {
        schema_version: CONTENT_CONTRACT_MANIFEST_V1.to_owned(),
        definitions: vec![savannah_lions(), second],
    };
    let bytes = encode_content_manifest_v1(&manifest).unwrap();
    let content_id = calculate_content_contract_id_v1(&bytes).unwrap();

    let mut provenance = decode_provenance_catalog_v1(BASIC_LAND_PROVENANCE).unwrap();
    for record in &mut provenance.records {
        record.content_contract_id = content_id.clone();
    }
    let provenance = encode_provenance_catalog_v1(&provenance).unwrap();

    let rules = RulesContractManifestV1 {
        rules_authority: RulesAuthorityV1::ComprehensiveRules {
            snapshot_id: "wotc-cr-2026-09-25-txt-20260925-sha256-8d860e451f20f38865b725b42d82feb714c725373dd8f3b32b8652b3eeb070ca".to_owned(),
        },
        capability_closure: Some(vec![CapabilityRequirementV1 {
            key: "rules/basic-priority".to_owned(),
            version: "0.1.0".to_owned(),
        }]),
    };
    let semantic = SemanticContractManifestV1 {
        rules_contract_id: calculate_rules_contract_id_v1(&rules).unwrap(),
        format_contract_id: None,
        content_contract_id: Some(content_id.clone()),
    };
    let execution = ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::MagicRules,
        semantic_contract_id: calculate_semantic_contract_id_v1(&semantic).unwrap(),
    };

    let outcome = admit_executable_profile_v1(
        &bytes,
        &content_id,
        &provenance,
        &rules,
        &semantic,
        &execution,
    );
    assert!(
        matches!(
            outcome,
            Err(ContentPreflightErrorV1::ExecutableProfileNotAdmitted)
        ),
        "a vanilla body must not admit as the basic-land profile"
    );
}
