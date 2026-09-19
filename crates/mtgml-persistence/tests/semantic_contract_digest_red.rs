//! Task 2 contract tests: canonical rules/semantic contract digest machinery
//! (spec §7–§9, §19.1/§19.2).
//!
//! The RED phase expects failure caused ONLY by the missing Task-2 digest
//! APIs. KAT hex literals are frozen during GREEN and asserted byte-identically
//! by the Rust and Python suites (shared vectors, independent implementations).

use mtgml_model::{
    CapabilityRequirementV1, ContentContractIdV1, FormatContractIdV1, RulesAuthorityV1,
    RulesContractIdV1, RulesContractManifestV1, SemanticContractIdV1, SemanticContractManifestV1,
};
use mtgml_persistence::semantic_contract_digest::{
    calculate_rules_contract_id_v1, calculate_semantic_contract_id_v1,
};

fn synthetic_rules_manifest() -> RulesContractManifestV1 {
    RulesContractManifestV1 {
        rules_authority: RulesAuthorityV1::SyntheticLegacy,
        capability_closure: None,
    }
}

fn minimal_comprehensive_manifest() -> RulesContractManifestV1 {
    RulesContractManifestV1 {
        rules_authority: RulesAuthorityV1::ComprehensiveRules {
            snapshot_id: "CR-2026-09-19".to_owned(),
        },
        capability_closure: Some(vec![CapabilityRequirementV1 {
            key: "rules/synthetic-transition".to_owned(),
            version: "1.0.0".to_owned(),
        }]),
    }
}

fn semantic_manifest(
    rules: RulesContractIdV1,
    format: Option<FormatContractIdV1>,
    content: Option<ContentContractIdV1>,
) -> SemanticContractManifestV1 {
    SemanticContractManifestV1 {
        rules_contract_id: rules,
        format_contract_id: format,
        content_contract_id: content,
    }
}

#[test]
fn kat_synthetic_legacy_rules_contract_id_is_frozen() {
    let id = calculate_rules_contract_id_v1(&synthetic_rules_manifest()).unwrap();
    assert_eq!(
        id.as_str(),
        "19bac684b74115b4fab823dfae2d3e75c23ee24f78effe51e9f72a33d4a58521"
    );
}

#[test]
fn kat_minimal_comprehensive_rules_contract_id_is_frozen() {
    let id = calculate_rules_contract_id_v1(&minimal_comprehensive_manifest()).unwrap();
    assert_eq!(
        id.as_str(),
        "6759a1bfa9ab56da74bdddde7c2e56ade99fc3169bffe9e16ce27f3947574722"
    );
}

#[test]
fn kat_semantic_contract_id_synthetic_null_null_is_frozen() {
    let rules = calculate_rules_contract_id_v1(&synthetic_rules_manifest()).unwrap();
    let id = calculate_semantic_contract_id_v1(&semantic_manifest(rules, None, None)).unwrap();
    assert_eq!(
        id.as_str(),
        "66ccac959475370e641e853473cbdd7f88489399587794b43f66cfa0342b1be4"
    );
}

#[test]
fn kat_semantic_contract_id_hypothetical_magic_null_null_is_frozen() {
    // KAT-only value (spec §19.2): a hypothetical Magic rules ID exercises
    // the non-synthetic path; it is NOT a catalog entry and no production
    // Magic contract exists (spec §25).
    let rules = RulesContractIdV1::from_digest_bytes([0x5a; 32]);
    let id = calculate_semantic_contract_id_v1(&semantic_manifest(rules, None, None)).unwrap();
    assert_eq!(
        id.as_str(),
        "f82a44a672f1095d7db11a505e0d6004e1e0670604f71cea80b3787255f883e9"
    );
}

#[test]
fn kat_semantic_contract_id_reserved_dimensions_are_raw_bytes() {
    // The reserved typed seams decode via canonical hex text only (Task 1);
    // their raw 32 bytes enter the canonical preimage (spec §8).
    let rules = calculate_rules_contract_id_v1(&synthetic_rules_manifest()).unwrap();
    let format = FormatContractIdV1::parse("34".repeat(32)).unwrap();
    let content = ContentContractIdV1::parse("56".repeat(32)).unwrap();
    let id =
        calculate_semantic_contract_id_v1(&semantic_manifest(rules, Some(format), Some(content)))
            .unwrap();
    assert_eq!(
        id.as_str(),
        "5465b1d7799db3ed2175211cb49a342520258ee7e6782a7f8aa45b985ca150fb"
    );
}

#[test]
fn distinct_manifests_produce_distinct_rule_ids() {
    let synthetic = calculate_rules_contract_id_v1(&synthetic_rules_manifest()).unwrap();
    let comprehensive = calculate_rules_contract_id_v1(&minimal_comprehensive_manifest()).unwrap();
    assert_ne!(synthetic.as_str(), comprehensive.as_str());

    let other_snapshot = RulesContractManifestV1 {
        rules_authority: RulesAuthorityV1::ComprehensiveRules {
            snapshot_id: "CR-OTHER".to_owned(),
        },
        capability_closure: Some(vec![CapabilityRequirementV1 {
            key: "rules/synthetic-transition".to_owned(),
            version: "1.0.0".to_owned(),
        }]),
    };
    assert_ne!(
        comprehensive.as_str(),
        calculate_rules_contract_id_v1(&other_snapshot)
            .unwrap()
            .as_str()
    );
}

#[test]
fn invalid_manifests_fail_closed_before_hashing() {
    let unsorted = RulesContractManifestV1 {
        rules_authority: RulesAuthorityV1::ComprehensiveRules {
            snapshot_id: "CR-2026-09-19".to_owned(),
        },
        capability_closure: Some(vec![
            CapabilityRequirementV1 {
                key: "rules/synthetic-transition".to_owned(),
                version: "1.0.0".to_owned(),
            },
            CapabilityRequirementV1 {
                key: "mechanic/lifelink".to_owned(),
                version: "1.0.0".to_owned(),
            },
        ]),
    };
    assert!(calculate_rules_contract_id_v1(&unsorted).is_err());

    let duplicate = RulesContractManifestV1 {
        rules_authority: RulesAuthorityV1::ComprehensiveRules {
            snapshot_id: "CR-2026-09-19".to_owned(),
        },
        capability_closure: Some(vec![
            CapabilityRequirementV1 {
                key: "rules/synthetic-transition".to_owned(),
                version: "1.0.0".to_owned(),
            },
            CapabilityRequirementV1 {
                key: "rules/synthetic-transition".to_owned(),
                version: "2.0.0".to_owned(),
            },
        ]),
    };
    assert!(calculate_rules_contract_id_v1(&duplicate).is_err());

    let synthetic_with_closure = RulesContractManifestV1 {
        rules_authority: RulesAuthorityV1::SyntheticLegacy,
        capability_closure: Some(vec![CapabilityRequirementV1 {
            key: "rules/synthetic-transition".to_owned(),
            version: "1.0.0".to_owned(),
        }]),
    };
    assert!(calculate_rules_contract_id_v1(&synthetic_with_closure).is_err());
}

#[test]
fn rule_ids_and_semantic_ids_are_distinct_typed_domains() {
    let rules = calculate_rules_contract_id_v1(&synthetic_rules_manifest()).unwrap();
    let semantic =
        calculate_semantic_contract_id_v1(&semantic_manifest(rules.clone(), None, None)).unwrap();
    assert_eq!(RulesContractIdV1::DOMAIN, "mtgml.rules-contract.v1");
    assert_eq!(SemanticContractIdV1::DOMAIN, "mtgml.semantic-contract.v1");
    assert_ne!(rules.as_str(), semantic.as_str());
}
