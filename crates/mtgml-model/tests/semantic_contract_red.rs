//! Task 1 contract tests: V5 semantic contract vocabulary (spec §7, §7b, §7c, §8).
//!
//! These are the permanent Task-1 contract tests. The RED phase expects a
//! compile failure caused ONLY by the missing Task-1 production types/API.
//! Structural validation (`validate`) is asserted separately from wire/decode
//! rejection.

use mtgml_model::{
    CapabilityRequirementV1, ContentContractIdV1, FormatContractIdV1, RulesAuthorityV1,
    RulesContractIdV1, RulesContractManifestV1, RulesContractManifestValidationError,
    SemanticContractIdV1, SemanticContractManifestV1,
};

fn entry(key: &str, version: &str) -> CapabilityRequirementV1 {
    CapabilityRequirementV1 {
        key: key.to_owned(),
        version: version.to_owned(),
    }
}

fn synthetic_manifest_json() -> serde_json::Value {
    serde_json::json!({
        "rules_authority": { "variant": "synthetic_legacy" },
        "capability_closure": null
    })
}

fn comprehensive_manifest_json(closure: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "rules_authority": {
            "variant": "comprehensive_rules",
            "snapshot_id": "CR-2026-09-19"
        },
        "capability_closure": closure
    })
}

#[test]
fn synthetic_legacy_with_null_closure_is_valid() {
    let manifest: RulesContractManifestV1 =
        serde_json::from_value(synthetic_manifest_json()).unwrap();
    assert_eq!(manifest.rules_authority, RulesAuthorityV1::SyntheticLegacy);
    assert!(manifest.capability_closure.is_none());
    manifest.validate().unwrap();
}

#[test]
fn synthetic_legacy_with_capability_closure_rejects() {
    for closure in [
        serde_json::json!([]),
        serde_json::json!([{ "key": "rules/synthetic-transition", "version": "1.0.0" }]),
    ] {
        let value = serde_json::json!({
            "rules_authority": { "variant": "synthetic_legacy" },
            "capability_closure": closure
        });
        let manifest: RulesContractManifestV1 = serde_json::from_value(value).unwrap();
        assert!(manifest.validate().is_err());
    }
}

#[test]
fn comprehensive_rules_minimal_valid_manifest() {
    let manifest: RulesContractManifestV1 = serde_json::from_value(comprehensive_manifest_json(
        serde_json::json!([{ "key": "rules/synthetic-transition", "version": "1.0.0" }]),
    ))
    .unwrap();
    manifest.validate().unwrap();
}

#[test]
fn comprehensive_rules_empty_snapshot_rejects() {
    let value = serde_json::json!({
        "rules_authority": { "variant": "comprehensive_rules", "snapshot_id": "" },
        "capability_closure": [{ "key": "rules/synthetic-transition", "version": "1.0.0" }]
    });
    let manifest: RulesContractManifestV1 = serde_json::from_value(value).unwrap();
    assert!(manifest.validate().is_err());
}

#[test]
fn comprehensive_rules_missing_snapshot_rejects_at_decode() {
    let value = serde_json::json!({
        "rules_authority": { "variant": "comprehensive_rules" },
        "capability_closure": null
    });
    assert!(serde_json::from_value::<RulesContractManifestV1>(value).is_err());
}

#[test]
fn synthetic_legacy_snapshot_payload_rejects_at_decode() {
    let value = serde_json::json!({
        "rules_authority": { "variant": "synthetic_legacy", "snapshot_id": "x" },
        "capability_closure": null
    });
    assert!(serde_json::from_value::<RulesContractManifestV1>(value).is_err());
}

#[test]
fn comprehensive_rules_null_closure_rejects() {
    let manifest: RulesContractManifestV1 =
        serde_json::from_value(comprehensive_manifest_json(serde_json::json!(null))).unwrap();
    assert!(manifest.validate().is_err());
}

#[test]
fn comprehensive_rules_empty_closure_rejects() {
    let manifest: RulesContractManifestV1 =
        serde_json::from_value(comprehensive_manifest_json(serde_json::json!([]))).unwrap();
    assert!(manifest.validate().is_err());
}

#[test]
fn invalid_capability_keys_reject() {
    for key in [
        "Rules/synthetic-transition",
        "rulesx/synthetic",
        "rules/",
        "rules/-x",
        "rules/x/",
        "format/Commander",
        "format/",
        "other/x",
        "",
        "rules/x//y",
    ] {
        let manifest = RulesContractManifestV1 {
            rules_authority: RulesAuthorityV1::ComprehensiveRules {
                snapshot_id: "CR-2026-09-19".to_owned(),
            },
            capability_closure: Some(vec![entry(key, "1.0.0")]),
        };
        assert!(
            manifest.validate().is_err(),
            "capability key must reject: {key}"
        );
    }
}

#[test]
fn valid_capability_keys_accept() {
    for key in [
        "rules/synthetic-transition",
        "mechanic/lifelink",
        "decision/priority-pass",
        "visibility/hand-cards",
        "tooling/replay-verify",
        "format/commander",
        "format/commander/100-point",
    ] {
        let manifest = RulesContractManifestV1 {
            rules_authority: RulesAuthorityV1::ComprehensiveRules {
                snapshot_id: "CR-2026-09-19".to_owned(),
            },
            capability_closure: Some(vec![entry(key, "1.0.0")]),
        };
        assert!(
            manifest.validate().is_ok(),
            "capability key must accept: {key}"
        );
    }
}

#[test]
fn invalid_capability_versions_reject() {
    for version in [
        "", "1", "1.0", "v1.0.0", "1.0.0.0", "1.0.x", "1.0.-", "1..0",
    ] {
        let manifest = RulesContractManifestV1 {
            rules_authority: RulesAuthorityV1::ComprehensiveRules {
                snapshot_id: "CR-2026-09-19".to_owned(),
            },
            capability_closure: Some(vec![entry("rules/synthetic-transition", version)]),
        };
        assert!(
            manifest.validate().is_err(),
            "capability version must reject: {version}"
        );
    }
}

#[test]
fn duplicate_capability_key_rejects() {
    let manifest = RulesContractManifestV1 {
        rules_authority: RulesAuthorityV1::ComprehensiveRules {
            snapshot_id: "CR-2026-09-19".to_owned(),
        },
        capability_closure: Some(vec![
            entry("rules/synthetic-transition", "1.0.0"),
            entry("rules/synthetic-transition", "2.0.0"),
        ]),
    };
    assert_eq!(
        manifest.validate(),
        Err(RulesContractManifestValidationError::CapabilityClosureDuplicateKey)
    );
}

#[test]
fn unsorted_capability_closure_rejects() {
    let manifest = RulesContractManifestV1 {
        rules_authority: RulesAuthorityV1::ComprehensiveRules {
            snapshot_id: "CR-2026-09-19".to_owned(),
        },
        capability_closure: Some(vec![
            entry("rules/synthetic-transition", "1.0.0"),
            entry("mechanic/lifelink", "1.0.0"),
        ]),
    };
    assert_eq!(
        manifest.validate(),
        Err(RulesContractManifestValidationError::CapabilityClosureNotSorted)
    );
}

#[test]
fn unknown_rules_authority_variant_rejects_at_decode() {
    for variant in ["synthetic-rules", "m3_legacy", "comprehensive", "unknown"] {
        let value = serde_json::json!({
            "rules_authority": { "variant": variant },
            "capability_closure": null
        });
        assert!(
            serde_json::from_value::<RulesContractManifestV1>(value).is_err(),
            "rules authority variant must reject: {variant}"
        );
    }
}

#[test]
fn rules_manifest_rejects_unknown_fields() {
    let value = serde_json::json!({
        "rules_authority": { "variant": "synthetic_legacy" },
        "capability_closure": null,
        "provenance": "extra"
    });
    assert!(serde_json::from_value::<RulesContractManifestV1>(value).is_err());

    let value = serde_json::json!({
        "rules_authority": { "variant": "comprehensive_rules", "snapshot_id": "CR-2026-09-19" },
        "capability_closure": [{ "key": "rules/synthetic-transition", "version": "1.0.0", "lifecycle": "certified" }]
    });
    assert!(serde_json::from_value::<RulesContractManifestV1>(value).is_err());
}

#[test]
fn rules_manifest_rejects_missing_fields() {
    let missing_authority = serde_json::json!({ "capability_closure": null });
    assert!(serde_json::from_value::<RulesContractManifestV1>(missing_authority).is_err());

    let missing_closure = serde_json::json!({
        "rules_authority": { "variant": "synthetic_legacy" }
    });
    assert!(serde_json::from_value::<RulesContractManifestV1>(missing_closure).is_err());
}

#[test]
fn semantic_contract_manifest_json_shape_is_exact() {
    let manifest = SemanticContractManifestV1 {
        rules_contract_id: RulesContractIdV1::from_digest_bytes([0x12; 32]),
        format_contract_id: None,
        content_contract_id: None,
    };
    let rendered = serde_json::to_value(&manifest).unwrap();
    assert_eq!(
        rendered,
        serde_json::json!({
            "rules_contract_id": "1212121212121212121212121212121212121212121212121212121212121212",
            "format_contract_id": null,
            "content_contract_id": null
        })
    );
    let decoded: SemanticContractManifestV1 = serde_json::from_value(rendered).unwrap();
    assert_eq!(decoded, manifest);

    let unknown_field = serde_json::json!({
        "rules_contract_id": "1212121212121212121212121212121212121212121212121212121212121212",
        "format_contract_id": null,
        "content_contract_id": null,
        "extra": true
    });
    assert!(serde_json::from_value::<SemanticContractManifestV1>(unknown_field).is_err());

    let missing_field = serde_json::json!({
        "rules_contract_id": "1212121212121212121212121212121212121212121212121212121212121212",
        "format_contract_id": null
    });
    assert!(serde_json::from_value::<SemanticContractManifestV1>(missing_field).is_err());
}

#[test]
fn semantic_contract_manifest_reserved_ids_round_trip() {
    // The typed seams must survive the future arrival of real format/content
    // contract IDs unchanged (spec §14): non-null values decode/encode as
    // 64-lowercase-hex strings. Production emission stays None (spec §13).
    let manifest = SemanticContractManifestV1 {
        rules_contract_id: RulesContractIdV1::from_digest_bytes([0x12; 32]),
        format_contract_id: Some(FormatContractIdV1::from_digest_bytes([0x34; 32])),
        content_contract_id: Some(ContentContractIdV1::from_digest_bytes([0x56; 32])),
    };
    let rendered = serde_json::to_value(&manifest).unwrap();
    assert_eq!(
        rendered,
        serde_json::json!({
            "rules_contract_id": "1212121212121212121212121212121212121212121212121212121212121212",
            "format_contract_id": "3434343434343434343434343434343434343434343434343434343434343434",
            "content_contract_id": "5656565656565656565656565656565656565656565656565656565656565656"
        })
    );
    let decoded: SemanticContractManifestV1 = serde_json::from_value(rendered).unwrap();
    assert_eq!(decoded, manifest);
}

#[test]
fn contract_id_domains_are_distinct() {
    assert_eq!(RulesContractIdV1::DOMAIN, "mtgml.rules-contract.v1");
    assert_eq!(SemanticContractIdV1::DOMAIN, "mtgml.semantic-contract.v1");
    assert_eq!(FormatContractIdV1::DOMAIN, "mtgml.format-contract.v1");
    assert_eq!(ContentContractIdV1::DOMAIN, "mtgml.content-contract.v1");

    let id = SemanticContractIdV1::from_digest_bytes([9; 32]);
    assert_eq!(id.as_str().len(), 64);
    assert!(SemanticContractIdV1::parse(id.as_str()).is_ok());
}
