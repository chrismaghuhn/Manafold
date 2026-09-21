//! Crate-internal recompute KAT for the generated semantic catalog
//! (spec §10 chain, §19.4).
//!
//! The manifests below are built EXCLUSIVELY from the GENERATED manifest
//! constructors — there is no hand-copied manifest in this KAT. Every ID
//! value must recompute byte-equal from those generated facts via the §9
//! persistence functions; drift fails the build.

use crate::semantic_catalog_generated::{
    synthetic_legacy_default_rules_contract_id, synthetic_legacy_default_rules_manifest,
    synthetic_legacy_default_semantic_contract_id, synthetic_legacy_default_semantic_manifest,
    SEMANTIC_CONTRACT_CATALOG_SYNTHETIC_LEGACY_DEFAULT_RULES_CONTRACT_HEX,
    SEMANTIC_CONTRACT_CATALOG_SYNTHETIC_LEGACY_DEFAULT_SEMANTIC_CONTRACT_HEX,
};

#[test]
fn rules_contract_id_recomputes_from_generated_manifest_facts() {
    let computed = mtgml_persistence::semantic_contract_digest::calculate_rules_contract_id_v1(
        &synthetic_legacy_default_rules_manifest(),
    )
    .expect("generated synthetic legacy rules manifest is valid");
    let generated_id = synthetic_legacy_default_rules_contract_id();
    assert_eq!(
        generated_id, computed,
        "generated rules contract ID constant drifted from its canonical meaning"
    );
    assert_eq!(
        generated_id.as_str(),
        SEMANTIC_CONTRACT_CATALOG_SYNTHETIC_LEGACY_DEFAULT_RULES_CONTRACT_HEX
    );
}

#[test]
fn semantic_contract_id_recomputes_from_generated_manifest_facts() {
    let computed = mtgml_persistence::semantic_contract_digest::calculate_semantic_contract_id_v1(
        &synthetic_legacy_default_semantic_manifest(),
    )
    .expect("generated synthetic legacy semantic manifest is valid");
    let generated_id = synthetic_legacy_default_semantic_contract_id();
    assert_eq!(
        generated_id, computed,
        "generated semantic contract ID constant drifted from its canonical meaning"
    );
    assert_eq!(
        generated_id.as_str(),
        SEMANTIC_CONTRACT_CATALOG_SYNTHETIC_LEGACY_DEFAULT_SEMANTIC_CONTRACT_HEX
    );
}

#[test]
fn catalog_constants_are_distinct_typed_domains() {
    assert_eq!(
        mtgml_model::RulesContractIdV1::DOMAIN,
        "mtgml.rules-contract.v1"
    );
    assert_eq!(
        mtgml_model::SemanticContractIdV1::DOMAIN,
        "mtgml.semantic-contract.v1"
    );
    assert_ne!(
        synthetic_legacy_default_rules_contract_id().as_str(),
        synthetic_legacy_default_semantic_contract_id().as_str()
    );
}

#[test]
fn generated_constants_render_canonical_lowercase_hex() {
    let rules = SEMANTIC_CONTRACT_CATALOG_SYNTHETIC_LEGACY_DEFAULT_RULES_CONTRACT_HEX;
    let semantic = SEMANTIC_CONTRACT_CATALOG_SYNTHETIC_LEGACY_DEFAULT_SEMANTIC_CONTRACT_HEX;
    for value in [rules, semantic] {
        assert_eq!(value.len(), 64, "canonical digest hex is 64 characters");
        assert!(
            value
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()),
            "canonical digest hex is lowercase"
        );
    }
}
