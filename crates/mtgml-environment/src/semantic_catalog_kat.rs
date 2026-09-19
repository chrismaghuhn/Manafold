//! Crate-internal recompute KAT for the generated semantic catalog
//! (spec §10 chain, §19.4).
//!
//! RED expectation was: compile failure caused ONLY by the missing generated
//! module/constants. GREEN: every ID value recomputes byte-equal from the
//! emitted manifest constants via the §9 persistence functions.

use crate::semantic_catalog_generated::{
    synthetic_legacy_default_rules_contract_id, synthetic_legacy_default_semantic_contract_id,
    SEMANTIC_CONTRACT_CATALOG_SYNTHETIC_LEGACY_DEFAULT_RULES_CONTRACT_HEX,
    SEMANTIC_CONTRACT_CATALOG_SYNTHETIC_LEGACY_DEFAULT_SEMANTIC_CONTRACT_HEX,
};

#[test]
fn rules_contract_id_value_recomputes_byte_equal() {
    let manifest = mtgml_model::RulesContractManifestV1 {
        rules_authority: mtgml_model::RulesAuthorityV1::SyntheticLegacy,
        capability_closure: None,
    };
    let computed =
        mtgml_persistence::semantic_contract_digest::calculate_rules_contract_id_v1(&manifest)
            .expect("synthetic legacy rules manifest is valid");
    let materialized = synthetic_legacy_default_rules_contract_id();
    assert_eq!(
        materialized, computed,
        "generated rules contract ID constant drifted from its canonical meaning"
    );
    assert_eq!(
        materialized.as_str(),
        SEMANTIC_CONTRACT_CATALOG_SYNTHETIC_LEGACY_DEFAULT_RULES_CONTRACT_HEX
    );
}

#[test]
fn semantic_contract_id_value_recomputes_byte_equal() {
    let manifest = mtgml_model::SemanticContractManifestV1 {
        rules_contract_id: synthetic_legacy_default_rules_contract_id(),
        format_contract_id: None,
        content_contract_id: None,
    };
    let computed =
        mtgml_persistence::semantic_contract_digest::calculate_semantic_contract_id_v1(&manifest)
            .expect("synthetic legacy semantic manifest is valid");
    let materialized = synthetic_legacy_default_semantic_contract_id();
    assert_eq!(
        materialized, computed,
        "generated semantic contract ID constant drifted from its canonical meaning"
    );
    assert_eq!(
        materialized.as_str(),
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
