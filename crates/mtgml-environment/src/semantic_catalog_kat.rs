//! Crate-internal recompute KAT for the generated semantic catalog
//! (spec §10 chain, §19.4).
//!
//! The manifests below are built EXCLUSIVELY from the GENERATED manifest
//! constructors — there is no hand-copied manifest in this KAT. Every ID
//! value must recompute byte-equal from those generated facts via the §9
//! persistence functions; drift fails the build.

use crate::semantic_catalog_generated::{
    magic_s3_a_ordered_sba_0_1_0_rules_contract_id, magic_s3_a_ordered_sba_0_1_0_rules_manifest,
    magic_s3_a_ordered_sba_0_1_0_semantic_contract_id,
    magic_s3_a_ordered_sba_0_1_0_semantic_manifest,
    magic_s3_c_draw_interaction_0_1_0_rules_contract_id,
    magic_s3_c_draw_interaction_0_1_0_rules_manifest,
    magic_s3_c_draw_interaction_0_1_0_semantic_contract_id,
    magic_s3_c_draw_interaction_0_1_0_semantic_manifest,
    magic_turn_structure_0_1_0_rules_contract_id, magic_turn_structure_0_1_0_rules_manifest,
    magic_turn_structure_0_1_0_semantic_contract_id, magic_turn_structure_0_1_0_semantic_manifest,
    synthetic_legacy_default_rules_contract_id, synthetic_legacy_default_rules_manifest,
    synthetic_legacy_default_semantic_contract_id, synthetic_legacy_default_semantic_manifest,
    SEMANTIC_CONTRACT_CATALOG_MAGIC_S3_A_ORDERED_SBA_0_1_0_RULES_CONTRACT_HEX,
    SEMANTIC_CONTRACT_CATALOG_MAGIC_S3_A_ORDERED_SBA_0_1_0_SEMANTIC_CONTRACT_HEX,
    SEMANTIC_CONTRACT_CATALOG_MAGIC_S3_C_DRAW_INTERACTION_0_1_0_RULES_CONTRACT_HEX,
    SEMANTIC_CONTRACT_CATALOG_MAGIC_S3_C_DRAW_INTERACTION_0_1_0_SEMANTIC_CONTRACT_HEX,
    SEMANTIC_CONTRACT_CATALOG_MAGIC_TURN_STRUCTURE_0_1_0_RULES_CONTRACT_HEX,
    SEMANTIC_CONTRACT_CATALOG_MAGIC_TURN_STRUCTURE_0_1_0_SEMANTIC_CONTRACT_HEX,
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
fn rules_contract_id_recomputes_for_turn_structure() {
    let computed = mtgml_persistence::semantic_contract_digest::calculate_rules_contract_id_v1(
        &magic_turn_structure_0_1_0_rules_manifest(),
    )
    .expect("generated turn-structure rules manifest is valid");
    let generated_id = magic_turn_structure_0_1_0_rules_contract_id();
    assert_eq!(
        generated_id, computed,
        "generated turn-structure rules contract ID constant drifted from its canonical meaning"
    );
    assert_eq!(
        generated_id.as_str(),
        SEMANTIC_CONTRACT_CATALOG_MAGIC_TURN_STRUCTURE_0_1_0_RULES_CONTRACT_HEX
    );
}

#[test]
fn rules_contract_id_recomputes_for_s3_a_ordered_sba() {
    let computed = mtgml_persistence::semantic_contract_digest::calculate_rules_contract_id_v1(
        &magic_s3_a_ordered_sba_0_1_0_rules_manifest(),
    )
    .expect("generated S3.A rules manifest is valid");
    let generated = magic_s3_a_ordered_sba_0_1_0_rules_contract_id();
    assert_eq!(generated, computed);
    assert_eq!(
        generated.as_str(),
        SEMANTIC_CONTRACT_CATALOG_MAGIC_S3_A_ORDERED_SBA_0_1_0_RULES_CONTRACT_HEX
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
fn semantic_contract_id_recomputes_for_turn_structure() {
    let computed = mtgml_persistence::semantic_contract_digest::calculate_semantic_contract_id_v1(
        &magic_turn_structure_0_1_0_semantic_manifest(),
    )
    .expect("generated turn-structure semantic manifest is valid");
    let generated_id = magic_turn_structure_0_1_0_semantic_contract_id();
    assert_eq!(
        generated_id, computed,
        "generated turn-structure semantic contract ID constant drifted from its canonical meaning"
    );
    assert_eq!(
        generated_id.as_str(),
        SEMANTIC_CONTRACT_CATALOG_MAGIC_TURN_STRUCTURE_0_1_0_SEMANTIC_CONTRACT_HEX
    );
}

#[test]
fn semantic_contract_id_recomputes_for_s3_a_ordered_sba() {
    let computed = mtgml_persistence::semantic_contract_digest::calculate_semantic_contract_id_v1(
        &magic_s3_a_ordered_sba_0_1_0_semantic_manifest(),
    )
    .expect("generated S3.A semantic manifest is valid");
    let generated = magic_s3_a_ordered_sba_0_1_0_semantic_contract_id();
    assert_eq!(generated, computed);
    assert_eq!(
        generated.as_str(),
        SEMANTIC_CONTRACT_CATALOG_MAGIC_S3_A_ORDERED_SBA_0_1_0_SEMANTIC_CONTRACT_HEX
    );
}

#[test]
fn s3_c_draw_contract_ids_recompute_from_generated_manifest_facts() {
    let rules = mtgml_persistence::semantic_contract_digest::calculate_rules_contract_id_v1(
        &magic_s3_c_draw_interaction_0_1_0_rules_manifest(),
    )
    .expect("generated Draw rules manifest is valid");
    let semantic = mtgml_persistence::semantic_contract_digest::calculate_semantic_contract_id_v1(
        &magic_s3_c_draw_interaction_0_1_0_semantic_manifest(),
    )
    .expect("generated Draw semantic manifest is valid");
    assert_eq!(rules, magic_s3_c_draw_interaction_0_1_0_rules_contract_id());
    assert_eq!(
        semantic,
        magic_s3_c_draw_interaction_0_1_0_semantic_contract_id()
    );
    assert_eq!(
        rules.as_str(),
        SEMANTIC_CONTRACT_CATALOG_MAGIC_S3_C_DRAW_INTERACTION_0_1_0_RULES_CONTRACT_HEX
    );
    assert_eq!(
        semantic.as_str(),
        SEMANTIC_CONTRACT_CATALOG_MAGIC_S3_C_DRAW_INTERACTION_0_1_0_SEMANTIC_CONTRACT_HEX
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
    let values = [
        SEMANTIC_CONTRACT_CATALOG_SYNTHETIC_LEGACY_DEFAULT_RULES_CONTRACT_HEX,
        SEMANTIC_CONTRACT_CATALOG_SYNTHETIC_LEGACY_DEFAULT_SEMANTIC_CONTRACT_HEX,
        SEMANTIC_CONTRACT_CATALOG_MAGIC_TURN_STRUCTURE_0_1_0_RULES_CONTRACT_HEX,
        SEMANTIC_CONTRACT_CATALOG_MAGIC_TURN_STRUCTURE_0_1_0_SEMANTIC_CONTRACT_HEX,
        SEMANTIC_CONTRACT_CATALOG_MAGIC_S3_A_ORDERED_SBA_0_1_0_RULES_CONTRACT_HEX,
        SEMANTIC_CONTRACT_CATALOG_MAGIC_S3_A_ORDERED_SBA_0_1_0_SEMANTIC_CONTRACT_HEX,
        SEMANTIC_CONTRACT_CATALOG_MAGIC_S3_C_DRAW_INTERACTION_0_1_0_RULES_CONTRACT_HEX,
        SEMANTIC_CONTRACT_CATALOG_MAGIC_S3_C_DRAW_INTERACTION_0_1_0_SEMANTIC_CONTRACT_HEX,
    ];
    for value in values {
        assert_eq!(value.len(), 64, "canonical digest hex is 64 characters");
        assert!(
            value
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()),
            "canonical digest hex is lowercase"
        );
    }
}
