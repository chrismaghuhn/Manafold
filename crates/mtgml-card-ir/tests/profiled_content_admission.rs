use mtgml_card_ir::{
    admit_executable_profile_v1, construct_gameplay_from_content, content_validation_only,
    decode_content_manifest_v1, decode_provenance_catalog_v1, encode_provenance_catalog_v1,
    NoExecutableProfileAdmitted, RequiredCapabilityLifecycleV1,
};
use mtgml_model::{
    CapabilityRequirementV1, ContentContractIdV1, ExecutionIdentityV1, ExecutionProgramV1,
    RulesAuthorityV1, RulesContractManifestV1, SemanticContractManifestV1,
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
        admission.direct_requirement_roots(),
        vec![
            requirement("rules/basic-land-mana"),
            requirement("rules/land-play"),
            requirement("rules/mana-pool"),
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

    let mut wrong_semantic = execution.clone();
    wrong_semantic.semantic_contract_id = mtgml_model::SemanticContractIdV1::parse(
        "0000000000000000000000000000000000000000000000000000000000000000",
    )
    .unwrap();
    assert!(admit_executable_profile_v1(
        &bytes,
        &id,
        &provenance,
        &rules,
        &semantic,
        &wrong_semantic,
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
