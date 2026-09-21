//! Task 1 contract tests: V5 execution identity vocabulary (spec §6, §7b).
//!
//! These are the permanent Task-1 contract tests. The RED phase expects a
//! compile failure caused ONLY by the missing Task-1 production types/API.

use mtgml_model::{
    CheckpointDigestV5, ExecutionIdentityV1, ExecutionProgramV1, SemanticContractIdV1,
};

#[test]
fn execution_program_wire_values_are_exact() {
    assert_eq!(
        serde_json::to_value(ExecutionProgramV1::SyntheticRulesCompat).unwrap(),
        serde_json::json!("synthetic_rules_compat")
    );
    assert_eq!(
        serde_json::to_value(ExecutionProgramV1::MagicRules).unwrap(),
        serde_json::json!("magic_rules")
    );
}

#[test]
fn execution_program_decodes_both_canonical_values() {
    assert_eq!(
        serde_json::from_value::<ExecutionProgramV1>(serde_json::json!("synthetic_rules_compat"))
            .unwrap(),
        ExecutionProgramV1::SyntheticRulesCompat
    );
    assert_eq!(
        serde_json::from_value::<ExecutionProgramV1>(serde_json::json!("magic_rules")).unwrap(),
        ExecutionProgramV1::MagicRules
    );
}

#[test]
fn unknown_program_strings_reject() {
    for value in [
        "synthetic_rules",
        "synthetic-rules",
        "legacy",
        "unknown",
        "synthetic_rules_compatx",
        "magic",
        "magic_rulesx",
        "",
    ] {
        assert!(
            serde_json::from_value::<ExecutionProgramV1>(serde_json::json!(value)).is_err(),
            "program value must reject: {value}"
        );
    }
}

#[test]
fn milestone_style_program_names_reject() {
    for value in [
        "m3",
        "m3_magic",
        "m3-magic",
        "s1",
        "magic_s1",
        "magic-s1",
        "magic_rules_m3",
    ] {
        assert!(
            serde_json::from_value::<ExecutionProgramV1>(serde_json::json!(value)).is_err(),
            "milestone-style program name must reject: {value}"
        );
    }
}

#[test]
fn execution_identity_json_shape_is_exact() {
    let identity = ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::MagicRules,
        semantic_contract_id: SemanticContractIdV1::from_digest_bytes([0xab; 32]),
    };
    let rendered = serde_json::to_value(&identity).unwrap();
    assert_eq!(
        rendered,
        serde_json::json!({
            "program_kind": "magic_rules",
            "semantic_contract_id":
                "abababababababababababababababababababababababababababababababab"
        })
    );
    let decoded: ExecutionIdentityV1 = serde_json::from_value(rendered).unwrap();
    assert_eq!(decoded, identity);
}

#[test]
fn execution_identity_rejects_unknown_fields() {
    let value = serde_json::json!({
        "program_kind": "synthetic_rules_compat",
        "semantic_contract_id": SemanticContractIdV1::from_digest_bytes([0; 32]).as_str(),
        "kernel": "extra"
    });
    assert!(serde_json::from_value::<ExecutionIdentityV1>(value).is_err());
}

#[test]
fn execution_identity_rejects_missing_fields() {
    let missing_program = serde_json::json!({
        "semantic_contract_id": SemanticContractIdV1::from_digest_bytes([1; 32]).as_str()
    });
    assert!(serde_json::from_value::<ExecutionIdentityV1>(missing_program).is_err());

    let missing_contract = serde_json::json!({ "program_kind": "magic_rules" });
    assert!(serde_json::from_value::<ExecutionIdentityV1>(missing_contract).is_err());
}

#[test]
fn semantic_contract_id_json_is_64_lowercase_hex() {
    let id = SemanticContractIdV1::from_digest_bytes([0xcd; 32]);
    assert_eq!(id.as_str().len(), 64);
    assert_eq!(id.as_str(), "cd".repeat(32));
    assert_eq!(SemanticContractIdV1::parse(id.as_str()).unwrap(), id);

    for bad in [
        "CD".repeat(32),
        "cd".repeat(31),
        "cd".repeat(33),
        "gg".repeat(32),
        String::new(),
    ] {
        assert!(
            SemanticContractIdV1::parse(bad.clone()).is_err(),
            "semantic contract id must reject: {bad}"
        );
    }
    assert!(
        serde_json::from_value::<SemanticContractIdV1>(serde_json::json!("CD".repeat(32))).is_err()
    );
}

#[test]
fn checkpoint_digest_v5_is_a_distinct_v5_domain_newtype() {
    assert_eq!(CheckpointDigestV5::DOMAIN, "mtgml.checkpoint-digest.v5");
    let digest = CheckpointDigestV5::from_digest_bytes([7; 32]);
    assert_eq!(digest.as_str().len(), 64);
    assert!(CheckpointDigestV5::parse(digest.as_str()).is_ok());
}
