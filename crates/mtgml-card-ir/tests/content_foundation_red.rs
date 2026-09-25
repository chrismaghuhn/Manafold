use mtgml_card_ir::{
    decode_content_manifest_v1, decode_provenance_catalog_v1, encode_content_manifest_v1,
    encode_provenance_catalog_v1, validate_content_manifest_v1, BaseCharacteristicsV1,
    CardDefinitionEnvelopeV1, CardSemanticBindingV1, ContentContractManifestV1,
    DefinitionProvenanceRecordV1, DefinitionReferenceV1, FaceDefinitionV1, FaceKey, ManaColorV1,
    PrintedManaSymbolV1, ProvenanceCatalogV1, SourceProvenanceV1, TypeLineV1,
    VerifiedContentCatalogV1,
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
fn duplicate_and_conflicting_definition_ids_have_distinct_failures() {
    let mut duplicate = minimal_manifest();
    duplicate.definitions.push(duplicate.definitions[0].clone());
    assert_eq!(
        validate_content_manifest_v1(&duplicate),
        Err(mtgml_card_ir::ContentValidationErrorV1::DuplicateDefinitionId)
    );

    let mut conflict = minimal_manifest();
    let mut conflicting_definition = conflict.definitions[0].clone();
    conflicting_definition.faces[0].base_characteristics.name =
        "Different rule-relevant name".to_owned();
    conflict.definitions.push(conflicting_definition);
    assert_eq!(
        validate_content_manifest_v1(&conflict),
        Err(mtgml_card_ir::ContentValidationErrorV1::IdentityConflict)
    );
}

#[test]
fn unknown_definition_reference_relation_has_its_own_error_class() {
    let mut manifest = minimal_manifest();
    manifest.definitions[0]
        .definition_references
        .push(DefinitionReferenceV1 {
            relation: "unreviewed_relation".to_owned(),
            target: CardDefinitionId(2),
            target_face_key: None,
        });
    assert_eq!(
        validate_content_manifest_v1(&manifest),
        Err(mtgml_card_ir::ContentValidationErrorV1::UnknownReferenceRelation)
    );
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

fn provenance_bytes(id: &ContentContractIdV1, definitions: &[CardDefinitionId]) -> Vec<u8> {
    encode_provenance_catalog_v1(&provenance_catalog(id, definitions)).unwrap()
}

#[test]
fn provenance_catalog_matches_its_canonical_audit_fixture() {
    let id = ContentContractIdV1::parse(
        "105a083f417293333532c3ffed1d96c13f74664c3bbaf64e8fad995918fb5a4a",
    )
    .unwrap();
    let catalog = provenance_catalog(&id, &[CardDefinitionId(1)]);
    let encoded = encode_provenance_catalog_v1(&catalog).unwrap();
    assert_eq!(
        encoded,
        include_bytes!("../../../persistence/golden/content-provenance-minimal-v1.cbor")
    );
    assert_eq!(decode_provenance_catalog_v1(&encoded).unwrap(), catalog);
}

#[test]
fn verified_catalog_is_content_scoped_and_provenance_excluded_from_digest() {
    let manifest = minimal_manifest();
    let bytes = encode_content_manifest_v1(&manifest).unwrap();
    let id = calculate_content_contract_id_v1(&bytes).unwrap();
    let catalog = VerifiedContentCatalogV1::build_from_bytes(
        &bytes,
        &id,
        provenance_bytes(&id, &[CardDefinitionId(1)]).as_slice(),
    )
    .unwrap();
    assert!(catalog.get(&id, CardDefinitionId(1)).is_ok());
    let other = ContentContractIdV1::parse("00".repeat(32)).unwrap();
    assert!(catalog.get(&other, CardDefinitionId(1)).is_err());
    assert_eq!(calculate_content_contract_id_v1(&bytes).unwrap(), id);

    let mut changed_audit = provenance_catalog(&id, &[CardDefinitionId(1)]);
    changed_audit.records[0]
        .source_provenance
        .source_snapshot_id = "snapshot/other-authoring-run".to_owned();
    let provenance = encode_provenance_catalog_v1(&changed_audit).unwrap();
    assert_eq!(
        decode_provenance_catalog_v1(&provenance).unwrap(),
        changed_audit
    );
    assert_eq!(calculate_content_contract_id_v1(&bytes).unwrap(), id);
    assert!(VerifiedContentCatalogV1::build_from_bytes(&bytes, &id, &provenance).is_ok());
}

#[test]
fn provenance_membership_and_identity_must_match_exactly() {
    let manifest = minimal_manifest();
    let bytes = encode_content_manifest_v1(&manifest).unwrap();
    let id = calculate_content_contract_id_v1(&bytes).unwrap();
    let other_id = ContentContractIdV1::parse("11".repeat(32)).unwrap();
    let mut missing = provenance_catalog(&id, &[CardDefinitionId(1)]);
    missing.records.clear();
    let mut duplicate = provenance_catalog(&id, &[CardDefinitionId(1)]);
    duplicate.records.push(duplicate.records[0].clone());
    let extra = provenance_catalog(&id, &[CardDefinitionId(1), CardDefinitionId(2)]);
    let wrong_content = provenance_catalog(&other_id, &[CardDefinitionId(1)]);
    let wrong_definition = provenance_catalog(&id, &[CardDefinitionId(2)]);

    for provenance in [missing, duplicate, extra, wrong_content, wrong_definition] {
        assert_eq!(
            VerifiedContentCatalogV1::build(&bytes, &id, provenance),
            Err(mtgml_card_ir::CatalogBuildErrorV1::ProvenanceCatalogMismatch)
        );
    }
}

#[test]
fn catalog_preflight_checks_manifest_and_identity_before_provenance() {
    let malformed_payload = [0xff];
    let malformed_provenance = [0xff];
    let claimed_id = ContentContractIdV1::parse("00".repeat(32)).unwrap();
    assert!(matches!(
        VerifiedContentCatalogV1::build_from_bytes(
            &malformed_payload,
            &claimed_id,
            &malformed_provenance,
        ),
        Err(mtgml_card_ir::CatalogBuildErrorV1::InvalidManifest(_))
    ));

    let payload = encode_content_manifest_v1(&minimal_manifest()).unwrap();
    assert_eq!(
        VerifiedContentCatalogV1::build_from_bytes(&payload, &claimed_id, &malformed_provenance,),
        Err(mtgml_card_ir::CatalogBuildErrorV1::ContentIdentityMismatch)
    );
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

#[test]
fn content_validation_success_never_authorizes_gameplay() {
    let manifest = minimal_manifest();
    let bytes = encode_content_manifest_v1(&manifest).unwrap();
    let id = calculate_content_contract_id_v1(&bytes).unwrap();
    let report = mtgml_card_ir::content_validation_only(
        &bytes,
        &id,
        provenance_bytes(&id, &[CardDefinitionId(1)]).as_slice(),
        &[CardDefinitionId(1)],
        mtgml_card_ir::RequiredCapabilityLifecycleV1::Specified,
    )
    .unwrap();
    assert_eq!(
        report.authorization,
        mtgml_card_ir::ContentAuthorizationV1::ValidationOnly
    );
    assert_eq!(
        mtgml_card_ir::construct_gameplay_from_content(&report),
        Err(mtgml_card_ir::NoExecutableProfileAdmitted)
    );
}

#[test]
fn capability_registry_closure_uses_registered_versions_and_lifecycle() {
    let mut manifest = minimal_manifest();
    manifest.definitions[0]
        .explicit_additional_requirements
        .push(mtgml_model::CapabilityRequirementV1 {
            key: "rules/basic-priority".to_owned(),
            version: "0.1.0".to_owned(),
        });
    let bytes = encode_content_manifest_v1(&manifest).unwrap();
    let id = calculate_content_contract_id_v1(&bytes).unwrap();
    let report = mtgml_card_ir::content_validation_only(
        &bytes,
        &id,
        provenance_bytes(&id, &[CardDefinitionId(1)]).as_slice(),
        &[CardDefinitionId(1)],
        mtgml_card_ir::RequiredCapabilityLifecycleV1::Covered,
    )
    .unwrap();
    assert_eq!(report.resolved_capabilities.len(), 4);
    assert!(report
        .resolved_capabilities
        .iter()
        .any(|value| value.key == "rules/turn-structure"));

    manifest.definitions[0].explicit_additional_requirements[0].version = "0.2.0".to_owned();
    let bytes = encode_content_manifest_v1(&manifest).unwrap();
    let id = calculate_content_contract_id_v1(&bytes).unwrap();
    let error = mtgml_card_ir::content_validation_only(
        &bytes,
        &id,
        provenance_bytes(&id, &[CardDefinitionId(1)]).as_slice(),
        &[CardDefinitionId(1)],
        mtgml_card_ir::RequiredCapabilityLifecycleV1::Specified,
    )
    .unwrap_err();
    assert!(matches!(
        error,
        mtgml_card_ir::ContentPreflightErrorV1::UnknownCapabilityVersion { .. }
    ));
}

#[test]
fn capability_lifecycle_below_explicit_threshold_rejects() {
    let mut manifest = minimal_manifest();
    manifest.definitions[0]
        .explicit_additional_requirements
        .push(mtgml_model::CapabilityRequirementV1 {
            key: "rules/basic-priority".to_owned(),
            version: "0.1.0".to_owned(),
        });
    let bytes = encode_content_manifest_v1(&manifest).unwrap();
    let id = calculate_content_contract_id_v1(&bytes).unwrap();
    let error = mtgml_card_ir::content_validation_only(
        &bytes,
        &id,
        provenance_bytes(&id, &[CardDefinitionId(1)]).as_slice(),
        &[CardDefinitionId(1)],
        mtgml_card_ir::RequiredCapabilityLifecycleV1::Certified,
    )
    .unwrap_err();
    assert!(matches!(
        error,
        mtgml_card_ir::ContentPreflightErrorV1::CapabilityLifecycleBelowRequirement { .. }
    ));
}

#[test]
fn strict_cbor_decoder_rejects_profile_variants_arity_order_and_trailing_data() {
    use mtgml_persistence::cbor::{self, Value};

    let mut value = cbor::decode_canonical(include_bytes!(
        "../../../persistence/golden/content-contract-minimal-v1.cbor"
    ))
    .unwrap();
    let Value::Array(root) = &mut value else {
        unreachable!()
    };
    let Value::Array(definitions) = &mut root[2] else {
        unreachable!()
    };
    let Value::Array(definition) = &mut definitions[0] else {
        unreachable!()
    };
    definition[4] = Value::Array(vec![
        Value::Text("profiled".to_owned()),
        Value::Array(vec![
            Value::Text("test/profile@1.0.0".to_owned()),
            Value::Array(vec![]),
        ]),
    ]);
    let profiled = cbor::encode_canonical(&value).unwrap();
    assert_eq!(
        decode_content_manifest_v1(&profiled),
        Err(mtgml_card_ir::ContentValidationErrorV1::ProfiledBindingNotAdmitted)
    );

    let valid = encode_content_manifest_v1(&minimal_manifest()).unwrap();
    let mut trailing = valid.clone();
    trailing.push(0);
    assert!(decode_content_manifest_v1(&trailing).is_err());
    let mut wrong_arity = cbor::decode_canonical(&valid).unwrap();
    let Value::Array(root) = &mut wrong_arity else {
        unreachable!()
    };
    root.pop();
    let wrong_arity = cbor::encode_canonical(&wrong_arity).unwrap();
    assert!(decode_content_manifest_v1(&wrong_arity).is_err());
}

#[test]
fn unknown_closed_mana_variant_has_a_typed_error() {
    use mtgml_persistence::cbor::{self, Value};

    let mut value = cbor::decode_canonical(include_bytes!(
        "../../../persistence/golden/content-contract-minimal-v1.cbor"
    ))
    .unwrap();
    let Value::Array(root) = &mut value else {
        unreachable!()
    };
    let Value::Array(definitions) = &mut root[2] else {
        unreachable!()
    };
    let Value::Array(definition) = &mut definitions[0] else {
        unreachable!()
    };
    let Value::Array(faces) = &mut definition[2] else {
        unreachable!()
    };
    let Value::Array(face) = &mut faces[0] else {
        unreachable!()
    };
    let Value::Array(characteristics) = &mut face[1] else {
        unreachable!()
    };
    characteristics[1] = Value::Array(vec![Value::Array(vec![
        Value::Text("snow".to_owned()),
        Value::Null,
    ])]);
    let bytes = cbor::encode_canonical(&value).unwrap();
    assert_eq!(
        decode_content_manifest_v1(&bytes),
        Err(mtgml_card_ir::ContentValidationErrorV1::UnknownFieldOrVariant)
    );
}

#[test]
fn test_only_profile_bodies_change_manifest_bytes_without_minting_identity() {
    use mtgml_persistence::cbor::{self, Value};

    let body_bytes = |value: u64| {
        let mut manifest = cbor::decode_canonical(include_bytes!(
            "../../../persistence/golden/content-contract-minimal-v1.cbor"
        ))
        .unwrap();
        let Value::Array(root) = &mut manifest else {
            unreachable!()
        };
        let Value::Array(definitions) = &mut root[2] else {
            unreachable!()
        };
        let Value::Array(definition) = &mut definitions[0] else {
            unreachable!()
        };
        definition[4] = Value::Array(vec![
            Value::Text("profiled".to_owned()),
            Value::Array(vec![
                Value::Text("test/typed-profile@1.0.0".to_owned()),
                Value::Array(vec![Value::Unsigned(value)]),
            ]),
        ]);
        cbor::encode_canonical(&manifest).unwrap()
    };
    let body_a = body_bytes(1);
    let body_b = body_bytes(2);
    assert_ne!(body_a, body_b);
    // These are codec-seam bytes only: neither is passed to production
    // ContentContractIdV1 calculation or catalog construction.
}

#[test]
fn duplicate_ability_key_and_invalid_face_binding_reject() {
    let mut manifest = minimal_manifest();
    manifest.definitions[0].ability_identities = vec![
        mtgml_card_ir::AbilityIdentityV1 {
            ability_key: mtgml_card_ir::AbilityKey(1),
            face_key: FaceKey(0),
        },
        mtgml_card_ir::AbilityIdentityV1 {
            ability_key: mtgml_card_ir::AbilityKey(1),
            face_key: FaceKey(0),
        },
    ];
    assert!(validate_content_manifest_v1(&manifest).is_err());
    manifest.definitions[0].ability_identities[1].ability_key = mtgml_card_ir::AbilityKey(2);
    manifest.definitions[0].ability_identities[1].face_key = FaceKey(1);
    assert!(validate_content_manifest_v1(&manifest).is_err());
}

#[test]
fn content_id_changes_with_rule_relevant_definition_data_and_mismatch_cannot_build_catalog() {
    let manifest = minimal_manifest();
    let original = encode_content_manifest_v1(&manifest).unwrap();
    let original_id = calculate_content_contract_id_v1(&original).unwrap();

    let mut variants = Vec::new();
    let mut mutate = |change: fn(&mut ContentContractManifestV1)| {
        let mut changed = manifest.clone();
        change(&mut changed);
        variants.push(changed);
    };
    mutate(|value| value.definitions[0].card_definition_id = CardDefinitionId(2));
    mutate(|value| {
        let characteristics = value.definitions[0].faces[0].base_characteristics.clone();
        value.definitions[0].faces.push(FaceDefinitionV1 {
            face_key: FaceKey(1),
            base_characteristics: characteristics,
        })
    });
    mutate(|value| value.definitions[0].faces[0].base_characteristics.name = "Changed".to_owned());
    mutate(|value| {
        value.definitions[0].faces[0].base_characteristics.mana_cost =
            Some(vec![PrintedManaSymbolV1::White])
    });
    mutate(|value| {
        value.definitions[0].faces[0]
            .base_characteristics
            .color_indicator = vec![ManaColorV1::Red]
    });
    mutate(|value| {
        value.definitions[0].faces[0]
            .base_characteristics
            .type_line
            .supertypes = vec!["Legendary".to_owned()]
    });
    mutate(|value| {
        value.definitions[0].faces[0]
            .base_characteristics
            .type_line
            .card_types = vec!["Artifact".to_owned()]
    });
    mutate(|value| {
        value.definitions[0].faces[0]
            .base_characteristics
            .type_line
            .subtypes = vec!["FixtureType".to_owned()]
    });
    mutate(|value| {
        value.definitions[0].faces[0]
            .base_characteristics
            .power_toughness = Some((2, 3))
    });
    mutate(|value| value.definitions[0].faces[0].base_characteristics.loyalty = Some(3));
    mutate(|value| value.definitions[0].faces[0].base_characteristics.defense = Some(4));
    mutate(|value| {
        value.definitions[0].ability_identities = vec![mtgml_card_ir::AbilityIdentityV1 {
            ability_key: mtgml_card_ir::AbilityKey(1),
            face_key: FaceKey(0),
        }]
    });
    mutate(|value| {
        let characteristics = value.definitions[0].faces[0].base_characteristics.clone();
        value.definitions[0].faces.push(FaceDefinitionV1 {
            face_key: FaceKey(1),
            base_characteristics: characteristics,
        });
        value.definitions[0].ability_identities = vec![mtgml_card_ir::AbilityIdentityV1 {
            ability_key: mtgml_card_ir::AbilityKey(1),
            face_key: FaceKey(1),
        }];
    });
    mutate(|value| {
        value.definitions[0].ability_identities = vec![mtgml_card_ir::AbilityIdentityV1 {
            ability_key: mtgml_card_ir::AbilityKey(2),
            face_key: FaceKey(0),
        }]
    });
    mutate(|value| {
        value.definitions[0].definition_references = vec![DefinitionReferenceV1 {
            relation: "required_definition".to_owned(),
            target: CardDefinitionId(2),
            target_face_key: None,
        }]
    });
    mutate(|value| {
        value.definitions[0].definition_references = vec![DefinitionReferenceV1 {
            relation: "required_definition".to_owned(),
            target: CardDefinitionId(2),
            target_face_key: Some(FaceKey(0)),
        }]
    });
    mutate(|value| {
        value.definitions[0].explicit_additional_requirements =
            vec![mtgml_model::CapabilityRequirementV1 {
                key: "rules/basic-priority".to_owned(),
                version: "0.1.0".to_owned(),
            }]
    });

    for changed in variants {
        let changed_bytes = encode_content_manifest_v1(&changed).unwrap();
        let changed_id = calculate_content_contract_id_v1(&changed_bytes).unwrap();
        assert_ne!(original, changed_bytes);
        assert_ne!(original_id, changed_id);
    }

    let mut changed = manifest.clone();
    changed.definitions[0].faces[0].base_characteristics.name = "Changed".to_owned();
    let changed_bytes = encode_content_manifest_v1(&changed).unwrap();
    let changed_id = calculate_content_contract_id_v1(&changed_bytes).unwrap();
    assert!(VerifiedContentCatalogV1::build(
        &original,
        &changed_id,
        provenance_catalog(&changed_id, &[CardDefinitionId(1)]),
    )
    .is_err());
}

#[test]
fn missing_reference_and_canonical_cycle_path_reject() {
    let mut missing = minimal_manifest();
    missing.definitions[0]
        .definition_references
        .push(DefinitionReferenceV1 {
            relation: "required_definition".to_owned(),
            target: CardDefinitionId(404),
            target_face_key: None,
        });
    let missing_bytes = encode_content_manifest_v1(&missing).unwrap();
    let missing_id = calculate_content_contract_id_v1(&missing_bytes).unwrap();
    let catalog = VerifiedContentCatalogV1::build(
        &missing_bytes,
        &missing_id,
        provenance_catalog(&missing_id, &[CardDefinitionId(1)]),
    )
    .unwrap();
    assert!(matches!(
        catalog.close_definition_roots(&missing_id, &[CardDefinitionId(1)]),
        Err(mtgml_card_ir::DefinitionClosureErrorV1::MissingDefinition {
            id: CardDefinitionId(404)
        })
    ));

    let mut cycle = minimal_manifest();
    let mut second = cycle.definitions[0].clone();
    second.card_definition_id = CardDefinitionId(2);
    cycle.definitions[0]
        .definition_references
        .push(DefinitionReferenceV1 {
            relation: "required_definition".to_owned(),
            target: CardDefinitionId(2),
            target_face_key: None,
        });
    second.definition_references.push(DefinitionReferenceV1 {
        relation: "required_definition".to_owned(),
        target: CardDefinitionId(1),
        target_face_key: None,
    });
    cycle.definitions.push(second);
    let bytes = encode_content_manifest_v1(&cycle).unwrap();
    let id = calculate_content_contract_id_v1(&bytes).unwrap();
    let catalog = VerifiedContentCatalogV1::build(
        &bytes,
        &id,
        provenance_catalog(&id, &[CardDefinitionId(1), CardDefinitionId(2)]),
    )
    .unwrap();
    assert_eq!(
        catalog.close_definition_roots(&id, &[CardDefinitionId(1)]),
        Err(mtgml_card_ir::DefinitionClosureErrorV1::ReferenceCycle {
            path: vec![
                CardDefinitionId(1),
                CardDefinitionId(2),
                CardDefinitionId(1)
            ]
        })
    );
}

#[test]
fn unknown_capability_and_conflicting_reachable_versions_reject() {
    let mut unknown = minimal_manifest();
    unknown.definitions[0]
        .explicit_additional_requirements
        .push(mtgml_model::CapabilityRequirementV1 {
            key: "rules/not-registered".to_owned(),
            version: "1.0.0".to_owned(),
        });
    let bytes = encode_content_manifest_v1(&unknown).unwrap();
    let id = calculate_content_contract_id_v1(&bytes).unwrap();
    let error = mtgml_card_ir::content_validation_only(
        &bytes,
        &id,
        provenance_bytes(&id, &[CardDefinitionId(1)]).as_slice(),
        &[CardDefinitionId(1)],
        mtgml_card_ir::RequiredCapabilityLifecycleV1::Specified,
    )
    .unwrap_err();
    assert!(matches!(
        error,
        mtgml_card_ir::ContentPreflightErrorV1::UnknownCapability { .. }
    ));

    let mut conflict = minimal_manifest();
    let mut second = conflict.definitions[0].clone();
    second.card_definition_id = CardDefinitionId(2);
    second
        .explicit_additional_requirements
        .push(mtgml_model::CapabilityRequirementV1 {
            key: "rules/basic-priority".to_owned(),
            version: "0.2.0".to_owned(),
        });
    conflict.definitions[0]
        .explicit_additional_requirements
        .push(mtgml_model::CapabilityRequirementV1 {
            key: "rules/basic-priority".to_owned(),
            version: "0.1.0".to_owned(),
        });
    second.definition_references.clear();
    conflict.definitions[0]
        .definition_references
        .push(DefinitionReferenceV1 {
            relation: "required_definition".to_owned(),
            target: CardDefinitionId(2),
            target_face_key: None,
        });
    conflict.definitions.push(second);
    let bytes = encode_content_manifest_v1(&conflict).unwrap();
    let id = calculate_content_contract_id_v1(&bytes).unwrap();
    let error = mtgml_card_ir::content_validation_only(
        &bytes,
        &id,
        provenance_bytes(&id, &[CardDefinitionId(1), CardDefinitionId(2)]).as_slice(),
        &[CardDefinitionId(1)],
        mtgml_card_ir::RequiredCapabilityLifecycleV1::Specified,
    )
    .unwrap_err();
    assert!(matches!(
        error,
        mtgml_card_ir::ContentPreflightErrorV1::CapabilityVersionConflict { .. }
    ));
}
