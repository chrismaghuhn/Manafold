use mtgml_card_ir::{
    admit_executable_profile_v1, construct_gameplay_from_content, content_validation_only,
    decode_content_manifest_v1, decode_provenance_catalog_v1, encode_content_manifest_v1,
    encode_provenance_catalog_v1, ContentContractManifestV1, ContentPreflightErrorV1,
    ExecutableProfileAdmissionV1, NoExecutableProfileAdmitted, PrintedManaSymbolV1,
    ProvenanceCatalogV1, RequiredCapabilityLifecycleV1,
};
use mtgml_model::{
    CapabilityRequirementV1, CardDefinitionId, ContentContractIdV1, ExecutionIdentityV1,
    ExecutionProgramV1, RulesAuthorityV1, RulesContractManifestV1, SemanticContractManifestV1,
};
use mtgml_persistence::{
    content_contract_digest::calculate_content_contract_id_v1,
    semantic_contract_digest::{calculate_rules_contract_id_v1, calculate_semantic_contract_id_v1},
};
const CONTENT_KAT: &[u8] =
    include_bytes!("../../../persistence/golden/content-contract-basic-land-v1-kat.v1.json");
const CONTENT_MANIFEST: &[u8] =
    include_bytes!("../../../cards/definitions/basic-land-v1/content-contract.v1.cbor");
const PINNED_PROVENANCE: &[u8] =
    include_bytes!("../../../cards/definitions/basic-land-v1/provenance.v1.cbor");
const COMBINED_MANIFEST: &[u8] = include_bytes!(
    "../../../cards/definitions/basic-land-and-vanilla-creature-v1/content-contract.v1.cbor"
);
const COMBINED_PROVENANCE: &[u8] = include_bytes!(
    "../../../cards/definitions/basic-land-and-vanilla-creature-v1/provenance.v1.cbor"
);
const PINNED_RULES_SNAPSHOT: &str = "wotc-cr-2026-09-25-txt-20260925-sha256-8d860e451f20f38865b725b42d82feb714c725373dd8f3b32b8652b3eeb070ca";

fn manifest_bytes() -> Vec<u8> {
    let kat: serde_json::Value = serde_json::from_slice(CONTENT_KAT).unwrap();
    let payload_hex = kat["canonical_payload_hex"].as_str().unwrap();
    let expected: Vec<u8> = payload_hex
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let digit = |byte: u8| match byte {
                b'0'..=b'9' => byte - b'0',
                b'a'..=b'f' => byte - b'a' + 10,
                _ => panic!("expected lowercase hex"),
            };
            (digit(pair[0]) << 4) | digit(pair[1])
        })
        .collect();
    assert_eq!(expected, CONTENT_MANIFEST);
    CONTENT_MANIFEST.to_vec()
}

fn provenance_bytes(content_contract_id: &ContentContractIdV1) -> Vec<u8> {
    let provenance = decode_provenance_catalog_v1(PINNED_PROVENANCE).unwrap();
    assert!(provenance
        .records
        .iter()
        .all(|record| &record.content_contract_id == content_contract_id));
    PINNED_PROVENANCE.to_vec()
}

fn requirement(key: &str) -> CapabilityRequirementV1 {
    CapabilityRequirementV1 {
        key: key.to_owned(),
        version: "0.1.0".to_owned(),
    }
}

fn complete_closure() -> Vec<CapabilityRequirementV1> {
    [
        "rules/basic-land-mana",
        "rules/basic-priority",
        "rules/cleanup-reset",
        "rules/combat-phase",
        "rules/declare-attackers",
        "rules/draw-card",
        "rules/game-start",
        "rules/land-play",
        "rules/mana-pool",
        "rules/state-based-actions-combat",
        "rules/state-based-actions-empty-library",
        "rules/turn-structure",
        "rules/zone-incarnation",
    ]
    .into_iter()
    .map(requirement)
    .collect()
}

fn identities(
    content_contract_id: ContentContractIdV1,
    closure: Vec<CapabilityRequirementV1>,
) -> (
    RulesContractManifestV1,
    SemanticContractManifestV1,
    ExecutionIdentityV1,
) {
    let rules_manifest = RulesContractManifestV1 {
        rules_authority: RulesAuthorityV1::ComprehensiveRules {
            snapshot_id: PINNED_RULES_SNAPSHOT.to_owned(),
        },
        capability_closure: Some(closure),
    };
    let rules_id = calculate_rules_contract_id_v1(&rules_manifest).unwrap();
    let semantic_manifest = SemanticContractManifestV1 {
        rules_contract_id: rules_id,
        format_contract_id: None,
        content_contract_id: Some(content_contract_id),
    };
    let semantic_id = calculate_semantic_contract_id_v1(&semantic_manifest).unwrap();
    let execution_identity = ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::MagicRules,
        semantic_contract_id: semantic_id,
    };
    (rules_manifest, semantic_manifest, execution_identity)
}

#[test]
fn mountain_plains_derive_the_closed_roots_and_recursive_registry_closure() {
    let bytes = manifest_bytes();
    let id = calculate_content_contract_id_v1(&bytes).unwrap();
    let decoded = decode_content_manifest_v1(&bytes).unwrap();
    assert!(decoded
        .definitions
        .iter()
        .all(|definition| definition.explicit_additional_requirements.is_empty()));
    let provenance = provenance_bytes(&id);
    let (rules, semantic, execution) = identities(id.clone(), complete_closure());

    let admission =
        admit_executable_profile_v1(&bytes, &id, &provenance, &rules, &semantic, &execution)
            .expect("pinned profile and complete identity closure admit for later integration");

    assert_eq!(admission.content_contract_id(), &id);
    assert_eq!(
        admission.verified_catalog().content_contract_id(),
        &id,
        "runtime admission must retain the exact verified catalog it admitted"
    );
    for definition in &decoded.definitions {
        assert_eq!(
            admission
                .verified_catalog()
                .get(&id, definition.card_definition_id)
                .unwrap(),
            definition
        );
    }
    assert_eq!(
        admission.direct_requirement_roots(),
        vec![
            requirement("rules/basic-land-mana"),
            requirement("rules/basic-priority"),
            requirement("rules/cleanup-reset"),
            requirement("rules/combat-phase"),
            requirement("rules/declare-attackers"),
            requirement("rules/draw-card"),
            requirement("rules/game-start"),
            requirement("rules/land-play"),
            requirement("rules/mana-pool"),
            requirement("rules/state-based-actions-empty-library"),
            requirement("rules/turn-structure"),
        ]
    );
    assert_eq!(admission.resolved_capabilities(), complete_closure());
    assert_eq!(admission.semantic_contract_manifest(), &semantic);
    assert_eq!(admission.execution_identity(), &execution);

    let profile_definitions = decoded
        .definitions
        .iter()
        .map(|definition| definition.card_definition_id)
        .collect::<Vec<_>>();
    let validation_report = content_validation_only(
        &bytes,
        &id,
        &provenance,
        &profile_definitions,
        RequiredCapabilityLifecycleV1::Specified,
    )
    .unwrap();
    assert_eq!(
        construct_gameplay_from_content(&validation_report),
        Err(NoExecutableProfileAdmitted)
    );
}

#[test]
fn pinned_source_provenance_is_required_and_must_match_exact_oracle_records() {
    let bytes = manifest_bytes();
    let id = calculate_content_contract_id_v1(&bytes).unwrap();
    let (rules, semantic, execution) = identities(id.clone(), complete_closure());
    let mut provenance = decode_provenance_catalog_v1(&provenance_bytes(&id)).unwrap();

    provenance.records[0].source_provenance.source_record_id = "wrong-oracle-record".to_owned();
    let wrong_id = encode_provenance_catalog_v1(&provenance).unwrap();
    assert!(
        admit_executable_profile_v1(&bytes, &id, &wrong_id, &rules, &semantic, &execution).is_err()
    );

    provenance = decode_provenance_catalog_v1(&provenance_bytes(&id)).unwrap();
    provenance.records.pop();
    let missing = encode_provenance_catalog_v1(&provenance).unwrap();
    assert!(
        admit_executable_profile_v1(&bytes, &id, &missing, &rules, &semantic, &execution).is_err()
    );
}

#[test]
fn profile_roots_cannot_be_suppressed_and_transitive_requirements_are_mandatory() {
    let bytes = manifest_bytes();
    let id = calculate_content_contract_id_v1(&bytes).unwrap();
    let provenance = provenance_bytes(&id);

    for key in [
        "rules/basic-land-mana",
        "rules/land-play",
        "rules/mana-pool",
        "rules/basic-priority",
        "rules/state-based-actions-combat",
        "rules/turn-structure",
        "rules/zone-incarnation",
        "rules/draw-card",
        "rules/combat-phase",
        "rules/declare-attackers",
        "rules/cleanup-reset",
        "rules/state-based-actions-empty-library",
        "rules/game-start",
    ] {
        let incomplete = complete_closure()
            .into_iter()
            .filter(|entry| entry.key != key)
            .collect();
        let (rules, semantic, execution) = identities(id.clone(), incomplete);
        assert!(
            admit_executable_profile_v1(&bytes, &id, &provenance, &rules, &semantic, &execution,)
                .is_err(),
            "missing {key} must fail closed"
        );
    }
}

#[test]
fn wrong_content_and_semantic_identity_bindings_reject() {
    let bytes = manifest_bytes();
    let id = calculate_content_contract_id_v1(&bytes).unwrap();
    let provenance = provenance_bytes(&id);
    let (rules, semantic, execution) = identities(id.clone(), complete_closure());
    let wrong_content = ContentContractIdV1::parse(
        "0000000000000000000000000000000000000000000000000000000000000000",
    )
    .unwrap();

    assert!(admit_executable_profile_v1(
        &bytes,
        &wrong_content,
        &provenance,
        &rules,
        &semantic,
        &execution
    )
    .is_err());

    let mut wrong_execution = execution.clone();
    wrong_execution.semantic_contract_id = mtgml_model::SemanticContractIdV1::parse(
        "0000000000000000000000000000000000000000000000000000000000000000",
    )
    .unwrap();
    assert!(admit_executable_profile_v1(
        &bytes,
        &id,
        &provenance,
        &rules,
        &semantic,
        &wrong_execution,
    )
    .is_err());
}

#[test]
fn noncanonical_content_bytes_do_not_enter_profile_admission() {
    let bytes = manifest_bytes();
    let id = calculate_content_contract_id_v1(&bytes).unwrap();
    let provenance = provenance_bytes(&id);
    let (rules, semantic, execution) = identities(id.clone(), complete_closure());
    let mut trailing = bytes.clone();
    trailing.push(0xf6);
    assert!(decode_content_manifest_v1(&trailing).is_err());
    assert!(admit_executable_profile_v1(
        &trailing,
        &id,
        &provenance,
        &rules,
        &semantic,
        &execution,
    )
    .is_err());
}

#[test]
fn executable_admission_adds_game_rule_roots_to_card_roots() {
    let bytes = manifest_bytes();
    let id = calculate_content_contract_id_v1(&bytes).unwrap();
    let provenance = provenance_bytes(&id);
    let (rules, semantic, execution) = identities(id.clone(), complete_closure());

    let admission =
        admit_executable_profile_v1(&bytes, &id, &provenance, &rules, &semantic, &execution)
            .expect("a game admits the turn rules every Magic game needs");

    let roots: std::collections::BTreeSet<&str> = admission
        .direct_requirement_roots()
        .iter()
        .map(|root| root.key.as_str())
        .collect();
    assert_eq!(
        roots,
        std::collections::BTreeSet::from([
            "rules/basic-land-mana",
            "rules/land-play",
            "rules/mana-pool",
            "rules/turn-structure",
            "rules/basic-priority",
            "rules/draw-card",
            "rules/combat-phase",
            "rules/declare-attackers",
            "rules/cleanup-reset",
            "rules/state-based-actions-empty-library",
            "rules/game-start",
        ])
    );
    assert_eq!(admission.resolved_capabilities(), complete_closure());
    let resolved: Vec<&str> = admission
        .resolved_capabilities()
        .iter()
        .map(|capability| capability.key.as_str())
        .collect();
    assert!(!resolved.contains(&"rules/declare-blockers"));
    assert!(!resolved.contains(&"rules/combat-damage"));
}

fn content_only_closure() -> Vec<CapabilityRequirementV1> {
    [
        "rules/basic-land-mana",
        "rules/basic-priority",
        "rules/land-play",
        "rules/mana-pool",
        "rules/state-based-actions-combat",
        "rules/turn-structure",
        "rules/zone-incarnation",
    ]
    .into_iter()
    .map(requirement)
    .collect()
}

#[test]
fn closures_other_than_the_full_game_reject() {
    let bytes = manifest_bytes();
    let id = calculate_content_contract_id_v1(&bytes).unwrap();
    let provenance = provenance_bytes(&id);
    let mut partial = content_only_closure();
    partial.push(requirement("rules/draw-card"));
    partial.sort_by(|left, right| left.key.cmp(&right.key));
    let (rules, semantic, execution) = identities(id.clone(), partial);
    assert!(
        admit_executable_profile_v1(&bytes, &id, &provenance, &rules, &semantic, &execution)
            .is_err()
    );
}

#[test]
fn only_the_full_game_closure_is_admitted() {
    let bytes = manifest_bytes();
    let id = calculate_content_contract_id_v1(&bytes).unwrap();
    let provenance = provenance_bytes(&id);
    let (rules, semantic, execution) = identities(id.clone(), content_only_closure());
    assert!(
        admit_executable_profile_v1(&bytes, &id, &provenance, &rules, &semantic, &execution)
            .is_err()
    );
    let (rules, semantic, execution) = identities(id.clone(), complete_closure());
    assert!(
        admit_executable_profile_v1(&bytes, &id, &provenance, &rules, &semantic, &execution)
            .is_ok()
    );
}

const CREATURE_ROOTS: [&str; 6] = [
    "rules/cast-creature-spell",
    "rules/stack-resolution",
    "rules/summoning-sickness",
    "rules/combat-damage",
    "rules/damage-and-life",
    "rules/state-based-actions-combat",
];

/// The land closure, the six creature roots, and what they pull in
/// (`rules/declare-blockers`, through `rules/combat-damage`).
fn combined_closure() -> Vec<CapabilityRequirementV1> {
    [
        "rules/basic-land-mana",
        "rules/basic-priority",
        "rules/cast-creature-spell",
        "rules/cleanup-reset",
        "rules/combat-damage",
        "rules/combat-phase",
        "rules/damage-and-life",
        "rules/declare-attackers",
        "rules/declare-blockers",
        "rules/draw-card",
        "rules/game-start",
        "rules/land-play",
        "rules/mana-pool",
        "rules/stack-resolution",
        "rules/state-based-actions-combat",
        "rules/state-based-actions-empty-library",
        "rules/summoning-sickness",
        "rules/turn-structure",
        "rules/zone-incarnation",
    ]
    .into_iter()
    .map(requirement)
    .collect()
}

/// Admit the committed combined catalog after editing its manifest and
/// provenance. The edited manifest gets its own content id and the provenance
/// follows it, so only the pinned-record check can object.
fn admit_combined_after(
    edit_manifest: impl FnOnce(&mut ContentContractManifestV1),
    edit_provenance: impl FnOnce(&mut ProvenanceCatalogV1),
) -> Result<ExecutableProfileAdmissionV1, ContentPreflightErrorV1> {
    let mut manifest = decode_content_manifest_v1(COMBINED_MANIFEST).unwrap();
    edit_manifest(&mut manifest);
    let bytes = encode_content_manifest_v1(&manifest).unwrap();
    let id = calculate_content_contract_id_v1(&bytes).unwrap();
    let mut provenance = decode_provenance_catalog_v1(COMBINED_PROVENANCE).unwrap();
    for record in &mut provenance.records {
        record.content_contract_id = id.clone();
    }
    edit_provenance(&mut provenance);
    let provenance = encode_provenance_catalog_v1(&provenance).unwrap();
    let (rules, semantic, execution) = identities(id.clone(), combined_closure());
    admit_executable_profile_v1(&bytes, &id, &provenance, &rules, &semantic, &execution)
}

#[test]
fn the_combined_catalog_is_admitted_with_creature_roots() {
    let id = calculate_content_contract_id_v1(COMBINED_MANIFEST).unwrap();
    let (rules, semantic, execution) = identities(id.clone(), combined_closure());

    let admission = admit_executable_profile_v1(
        COMBINED_MANIFEST,
        &id,
        COMBINED_PROVENANCE,
        &rules,
        &semantic,
        &execution,
    )
    .expect("two lands and three vanilla creatures admit with their pinned records");

    assert_eq!(admission.resolved_capabilities(), combined_closure());
    let direct: Vec<&str> = admission
        .direct_requirement_roots()
        .iter()
        .map(|root| root.key.as_str())
        .collect();
    for root in CREATURE_ROOTS {
        assert!(direct.contains(&root), "{root} must be a direct root");
    }
    let decoded = decode_content_manifest_v1(COMBINED_MANIFEST).unwrap();
    assert_eq!(decoded.definitions.len(), 5);
    for definition in &decoded.definitions {
        assert_eq!(
            admission
                .verified_catalog()
                .get(&id, definition.card_definition_id)
                .unwrap(),
            definition
        );
    }
}

#[test]
fn a_closure_without_the_creature_roots_is_refused() {
    let id = calculate_content_contract_id_v1(COMBINED_MANIFEST).unwrap();
    let (rules, semantic, execution) = identities(id.clone(), complete_closure());
    assert_eq!(
        admit_executable_profile_v1(
            COMBINED_MANIFEST,
            &id,
            COMBINED_PROVENANCE,
            &rules,
            &semantic,
            &execution,
        )
        .unwrap_err(),
        ContentPreflightErrorV1::CapabilityClosureMismatch,
        "creature content derives its roots; the land closure is not enough"
    );
}

#[test]
fn a_creature_record_with_other_characteristics_is_refused() {
    type Edit = fn(&mut ContentContractManifestV1);
    let cases: [(&str, Edit); 5] = [
        ("a 9/9 body", |manifest| {
            manifest.definitions[2].faces[0]
                .base_characteristics
                .power_toughness = Some((9, 9))
        }),
        ("another name", |manifest| {
            manifest.definitions[2].faces[0].base_characteristics.name = "Savannah Cats".to_owned()
        }),
        ("another mana cost", |manifest| {
            manifest.definitions[2].faces[0]
                .base_characteristics
                .mana_cost = Some(vec![PrintedManaSymbolV1::Red])
        }),
        ("another subtype", |manifest| {
            manifest.definitions[2].faces[0]
                .base_characteristics
                .type_line
                .subtypes = vec!["Ogre".to_owned()]
        }),
        ("an extra subtype", |manifest| {
            manifest.definitions[2].faces[0]
                .base_characteristics
                .type_line
                .subtypes
                .push("Warrior".to_owned())
        }),
    ];
    for (name, edit) in cases {
        assert_eq!(
            admit_combined_after(edit, |_| {}).unwrap_err(),
            ContentPreflightErrorV1::PinnedSourceProvenanceMismatch,
            "Savannah Lions with {name}"
        );
    }
}

#[test]
fn an_unpinned_oracle_record_is_refused() {
    assert_eq!(
        admit_combined_after(
            |_| {},
            |provenance| {
                provenance.records[2].source_provenance.source_record_id =
                    "00000000-0000-0000-0000-000000000000".to_owned()
            },
        )
        .unwrap_err(),
        ContentPreflightErrorV1::PinnedSourceProvenanceMismatch
    );
    assert_eq!(
        admit_combined_after(
            |_| {},
            |provenance| provenance.records[2].source_provenance.source_record_digest = [7; 32],
        )
        .unwrap_err(),
        ContentPreflightErrorV1::PinnedSourceProvenanceMismatch,
        "the digest of the pinned record, not just its id, must match"
    );
}

#[test]
fn a_record_from_another_snapshot_or_codec_is_refused() {
    assert_eq!(
        admit_combined_after(
            |_| {},
            |provenance| {
                provenance.records[3].source_provenance.source_snapshot_id =
                    "oracle-cards-20260101000000".to_owned()
            },
        )
        .unwrap_err(),
        ContentPreflightErrorV1::PinnedSourceProvenanceMismatch
    );
    assert_eq!(
        admit_combined_after(
            |_| {},
            |provenance| {
                provenance.records[3]
                    .source_provenance
                    .source_record_codec_id = "scryfall.oracle-card-json.v2".to_owned()
            },
        )
        .unwrap_err(),
        ContentPreflightErrorV1::PinnedSourceProvenanceMismatch
    );
}

#[test]
fn a_pinned_record_cannot_back_two_definitions() {
    // A second Savannah Lions under its own id, with the Lions record again:
    // every characteristic matches, so only the duplicate can refuse it.
    let outcome = admit_combined_after(
        |manifest| {
            let mut second_lions = manifest.definitions[2].clone();
            second_lions.card_definition_id = CardDefinitionId(6);
            manifest.definitions.push(second_lions);
        },
        |provenance| {
            let mut second = provenance.records[2].clone();
            second.card_definition_id = CardDefinitionId(6);
            provenance.records.push(second);
        },
    );
    assert_eq!(
        outcome.unwrap_err(),
        ContentPreflightErrorV1::PinnedSourceProvenanceMismatch
    );
}

#[test]
fn a_land_body_is_not_admitted_on_a_creature_record() {
    // Definition 3 carries the Mountain body but the Lions record.
    let outcome = admit_combined_after(
        |manifest| {
            let mut mountain = manifest.definitions[0].clone();
            mountain.card_definition_id = CardDefinitionId(3);
            manifest.definitions[2] = mountain;
        },
        |_| {},
    );
    assert_eq!(
        outcome.unwrap_err(),
        ContentPreflightErrorV1::ExecutableProfileNotAdmitted
    );
}

#[test]
fn the_land_only_admission_is_unchanged() {
    // Pinned from master before the creature profile existed: a land-only
    // game keeps its closure and its identities, so its games keep their bytes.
    const MASTER_RULES_CONTRACT_ID: &str =
        "681617e7c4835735cc86c887c44acb4e8cae04505e490d35abf7623b07ee1bc3";
    const MASTER_SEMANTIC_CONTRACT_ID: &str =
        "dcbadb8096975369134070715ac7e8a208c022ab71b26a187ba31ce96af8b0f3";

    let bytes = manifest_bytes();
    let id = calculate_content_contract_id_v1(&bytes).unwrap();
    assert_eq!(
        id.as_str(),
        "80d26c187739664e880948e767e44ed9791aa7c25e7ef703e6d63385311fb346"
    );
    let provenance = provenance_bytes(&id);
    let (rules, semantic, execution) = identities(id.clone(), complete_closure());
    let admission =
        admit_executable_profile_v1(&bytes, &id, &provenance, &rules, &semantic, &execution)
            .expect("the land-only catalog still admits");

    assert_eq!(admission.resolved_capabilities(), complete_closure());
    assert_eq!(
        calculate_rules_contract_id_v1(admission.rules_contract_manifest())
            .unwrap()
            .as_str(),
        MASTER_RULES_CONTRACT_ID
    );
    assert_eq!(
        admission.semantic_contract_id().as_str(),
        MASTER_SEMANTIC_CONTRACT_ID
    );
    let resolved: Vec<&str> = admission
        .resolved_capabilities()
        .iter()
        .map(|capability| capability.key.as_str())
        .collect();
    for root in CREATURE_ROOTS {
        // The land closure already holds the state-based-action capability as
        // a dependency; the other five are creature roots.
        if root != "rules/state-based-actions-combat" {
            assert!(!resolved.contains(&root), "{root} is a creature root");
        }
    }
}
