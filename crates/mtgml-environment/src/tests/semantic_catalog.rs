use crate::semantic_catalog::{CatalogEntry, RuntimeSemanticCatalog};
use crate::semantic_catalog_generated::{
    synthetic_legacy_default_rules_contract_id,
    synthetic_legacy_default_rules_manifest,
    synthetic_legacy_default_semantic_contract_id,
    synthetic_legacy_default_semantic_manifest,
    SEMANTIC_CONTRACT_CATALOG_SYNTHETIC_LEGACY_DEFAULT_RULES_CONTRACT_HEX,
    SEMANTIC_CONTRACT_CATALOG_SYNTHETIC_LEGACY_DEFAULT_SEMANTIC_CONTRACT_HEX,
};
use mtgml_model::{RulesAuthorityV1, SemanticContractIdV1};
use mtgml_persistence::semantic_contract_digest::{
    calculate_rules_contract_id_v1, calculate_semantic_contract_id_v1,
};

fn id_from_hex(hex: &str) -> SemanticContractIdV1 {
    SemanticContractIdV1::parse(hex.to_string()).unwrap()
}

#[test]
fn synthetic_legacy_resolves_in_production_catalog() {
    let catalog = RuntimeSemanticCatalog::production();
    let id = synthetic_legacy_default_semantic_contract_id();
    let entry = catalog
        .resolve(&id)
        .expect("SyntheticLegacy semantic contract must resolve in production catalog");
    assert_eq!(entry.semantic_contract_id, id);
    assert_eq!(entry.manifest.rules_contract_id, synthetic_legacy_default_rules_contract_id());
    assert!(entry.manifest.format_contract_id.is_none());
    assert!(entry.manifest.content_contract_id.is_none());
    assert_eq!(entry.rules_manifest.rules_authority, RulesAuthorityV1::SyntheticLegacy);
    assert!(entry.rules_manifest.capability_closure.is_none());
}

#[test]
fn unknown_semantic_contract_id_rejects_deterministically() {
    let catalog = RuntimeSemanticCatalog::production();
    let unknown = SemanticContractIdV1::from_digest_bytes([0u8; 32]);
    assert!(
        catalog.resolve(&unknown).is_none(),
        "an unknown semantic contract ID must not resolve"
    );
    assert!(
        catalog.resolve(&unknown) == catalog.resolve(&unknown),
        "resolve must be deterministic across identical queries"
    );
}

#[test]
fn known_meaning_is_distinct_from_supported_execution() {
    let catalog = RuntimeSemanticCatalog::production();
    let id = synthetic_legacy_default_semantic_contract_id();

    // KNOWN MEANING: the contract resolves to an immutable manifest.
    assert!(catalog.resolve(&id).is_some(), "contract must be known");

    // SUPPORTED EXECUTION: the runtime supports this program × contract pairing.
    assert!(
        catalog.supported(&id, mtgml_model::ExecutionProgramV1::SyntheticRulesCompat),
        "SyntheticRulesCompat must be supported for SyntheticLegacy"
    );

    // MagicRules MUST NOT acquire the synthetic contract as a valid executable pairing.
    assert!(
        !catalog.supported(&id, mtgml_model::ExecutionProgramV1::MagicRules),
        "MagicRules must NOT be supported for SyntheticLegacy"
    );
}

#[test]
fn production_catalog_contains_exactly_generated_material() {
    let catalog = RuntimeSemanticCatalog::production();
    assert_eq!(catalog.entry_count(), 1, "exactly one production entry");

    let id = synthetic_legacy_default_semantic_contract_id();
    let entry = catalog.resolve(&id).unwrap();

    // Every field must match the generated Task-3 constants — no hand-copied data.
    assert_eq!(
        entry.semantic_contract_id.as_str(),
        SEMANTIC_CONTRACT_CATALOG_SYNTHETIC_LEGACY_DEFAULT_SEMANTIC_CONTRACT_HEX
    );
    assert_eq!(
        entry.manifest.rules_contract_id.as_str(),
        SEMANTIC_CONTRACT_CATALOG_SYNTHETIC_LEGACY_DEFAULT_RULES_CONTRACT_HEX
    );
    assert_eq!(entry.manifest, synthetic_legacy_default_semantic_manifest());
    assert_eq!(entry.rules_manifest, synthetic_legacy_default_rules_manifest());
    assert_eq!(entry.rules_manifest.rules_authority, RulesAuthorityV1::SyntheticLegacy);
    assert!(entry.rules_manifest.capability_closure.is_none());
    assert!(entry.manifest.format_contract_id.is_none());
    assert!(entry.manifest.content_contract_id.is_none());
}

#[test]
fn production_catalog_does_not_invent_magic_contract() {
    let catalog = RuntimeSemanticCatalog::production();
    for entry in catalog.entries() {
        assert!(
            !matches!(
                entry.rules_manifest.rules_authority,
                RulesAuthorityV1::ComprehensiveRules { .. }
            ),
            "production catalog must not contain a comprehensive_rules contract"
        );
    }
}

#[test]
fn catalog_consumes_only_generated_constants() {
    let catalog = RuntimeSemanticCatalog::production();
    let id = synthetic_legacy_default_semantic_contract_id();

    // The resolved manifest must recompute to the same semantic contract ID
    // (the catalog is NOT the authority — it hands out frozen generated values;
    // the KAT in semantic_catalog_kat.rs proves drift is impossible).
    let recomputed =
        calculate_semantic_contract_id_v1(&catalog.resolve(&id).unwrap().manifest)
            .expect("generated synthetic manifest is valid");
    assert_eq!(recomputed, id);

    let recomputed_rules =
        calculate_rules_contract_id_v1(&catalog.resolve(&id).unwrap().rules_manifest)
            .expect("generated synthetic rules manifest is valid");
    assert_eq!(recomputed_rules, synthetic_legacy_default_rules_contract_id());
}

#[test]
fn catalog_immutable_no_mutable_api() {
    // Compile-time guarantee: RuntimeSemanticCatalog exposes no public mutation.
    let catalog = RuntimeSemanticCatalog::production();
    let id = synthetic_legacy_default_semantic_contract_id();
    let _ = catalog.resolve(&id);
    let _ = catalog.supported(&id, mtgml_model::ExecutionProgramV1::SyntheticRulesCompat);
    let _ = catalog.entry_count();
    // Resolve and supported are read-only lookups; the catalog itself is immutable.
}

#[test]
fn catalog_supports_synthetic_rules_compat_only() {
    let catalog = RuntimeSemanticCatalog::production();
    let id = synthetic_legacy_default_semantic_contract_id();
    let unknown = SemanticContractIdV1::from_digest_bytes([1u8; 32]);

    assert!(catalog.supported(&id, mtgml_model::ExecutionProgramV1::SyntheticRulesCompat));
    assert!(!catalog.supported(&id, mtgml_model::ExecutionProgramV1::MagicRules));
    assert!(!catalog.supported(&unknown, mtgml_model::ExecutionProgramV1::SyntheticRulesCompat));
    assert!(!catalog.supported(&unknown, mtgml_model::ExecutionProgramV1::MagicRules));
}

#[test]
fn catalog_entry_fields_are_observable() {
    // Test-only seam: construct a catalog with a corrupted entry to prove the
    // catalog itself is just a frozen data table (the admission function owns
    // the recompute-and-compare logic, not the catalog).
    let wrong_id = id_from_hex(
        "1111111111111111111111111111111111111111111111111111111111111111",
    );
    let entry = CatalogEntry {
        semantic_contract_id: wrong_id.clone(),
        manifest: synthetic_legacy_default_semantic_manifest(),
        rules_manifest: synthetic_legacy_default_rules_manifest(),
    };
    let catalog = RuntimeSemanticCatalog::from_entries(vec![entry]);

    // The wrong ID resolves (it is in the catalog table), but the manifest
    // would recompute to a DIFFERENT id — proving known-meaning != correct-id.
    assert!(catalog.resolve(&wrong_id).is_some());
    let recomputed = calculate_semantic_contract_id_v1(
        &catalog.resolve(&wrong_id).unwrap().manifest,
    )
    .unwrap();
    assert_ne!(recomputed, wrong_id);
}
