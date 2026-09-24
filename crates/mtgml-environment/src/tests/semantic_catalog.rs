use crate::semantic_catalog::{CatalogEntry, RuntimeSemanticCatalog};
use crate::semantic_catalog_generated::{
    magic_s3_a_ordered_sba_0_1_0_rules_contract_id,
    magic_s3_a_ordered_sba_0_1_0_rules_manifest,
    magic_s3_a_ordered_sba_0_1_0_semantic_contract_id,
    magic_s3_a_ordered_sba_0_1_0_semantic_manifest,
    magic_s3_b_basic_priority_0_1_0_rules_contract_id,
    magic_s3_b_basic_priority_0_1_0_rules_manifest,
    magic_s3_b_basic_priority_0_1_0_semantic_contract_id,
    magic_s3_b_basic_priority_0_1_0_semantic_manifest,
    magic_turn_structure_0_1_0_rules_contract_id,
    magic_turn_structure_0_1_0_rules_manifest,
    magic_turn_structure_0_1_0_semantic_contract_id,
    magic_turn_structure_0_1_0_semantic_manifest,
    synthetic_legacy_default_rules_contract_id,
    synthetic_legacy_default_rules_manifest,
    synthetic_legacy_default_semantic_manifest,
    SEMANTIC_CONTRACT_CATALOG_MAGIC_TURN_STRUCTURE_0_1_0_RULES_CONTRACT_HEX,
    SEMANTIC_CONTRACT_CATALOG_MAGIC_TURN_STRUCTURE_0_1_0_SEMANTIC_CONTRACT_HEX,
    SEMANTIC_CONTRACT_CATALOG_SYNTHETIC_LEGACY_DEFAULT_RULES_CONTRACT_HEX,
    SEMANTIC_CONTRACT_CATALOG_SYNTHETIC_LEGACY_DEFAULT_SEMANTIC_CONTRACT_HEX,
};
use mtgml_model::{
    CapabilityRequirementV1, ExecutionProgramV1, RulesAuthorityV1, RulesContractManifestV1,
    SemanticContractIdV1, SemanticContractManifestV1,
};
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
    assert_eq!(catalog.entry_count(), 4, "Synthetic, frozen S1, S3.A, and S3.B identities");

    let syn_id = synthetic_legacy_default_semantic_contract_id();
    let syn_entry = catalog.resolve(&syn_id).unwrap();
    assert_eq!(syn_entry.semantic_contract_id, syn_id);
    assert_eq!(syn_entry.manifest.rules_contract_id, synthetic_legacy_default_rules_contract_id());
    assert_eq!(syn_entry.manifest, synthetic_legacy_default_semantic_manifest());
    assert_eq!(syn_entry.rules_manifest, synthetic_legacy_default_rules_manifest());
    assert_eq!(syn_entry.rules_manifest.rules_authority, RulesAuthorityV1::SyntheticLegacy);
    assert!(syn_entry.rules_manifest.capability_closure.is_none());
    assert!(syn_entry.manifest.format_contract_id.is_none());
    assert!(syn_entry.manifest.content_contract_id.is_none());

    let ts_id = magic_turn_structure_0_1_0_semantic_contract_id();
    let ts_entry = catalog.resolve(&ts_id).unwrap();
    assert_eq!(ts_entry.semantic_contract_id, ts_id);
    assert_eq!(ts_entry.manifest.rules_contract_id, magic_turn_structure_0_1_0_rules_contract_id());
    assert_eq!(ts_entry.manifest, magic_turn_structure_0_1_0_semantic_manifest());
    assert_eq!(ts_entry.rules_manifest, magic_turn_structure_0_1_0_rules_manifest());
    assert!(matches!(ts_entry.rules_manifest.rules_authority, RulesAuthorityV1::ComprehensiveRules { .. }));
    assert!(ts_entry.rules_manifest.capability_closure.is_some());
    assert!(ts_entry.manifest.format_contract_id.is_none());
    assert!(ts_entry.manifest.content_contract_id.is_none());

    let s3a_id = magic_s3_a_ordered_sba_0_1_0_semantic_contract_id();
    let s3a_entry = catalog.resolve(&s3a_id).unwrap();
    assert_eq!(s3a_entry.semantic_contract_id, s3a_id);
    assert_eq!(
        s3a_entry.manifest.rules_contract_id,
        magic_s3_a_ordered_sba_0_1_0_rules_contract_id()
    );
    assert_eq!(
        s3a_entry.manifest,
        magic_s3_a_ordered_sba_0_1_0_semantic_manifest()
    );
    assert_eq!(
        s3a_entry.rules_manifest,
        magic_s3_a_ordered_sba_0_1_0_rules_manifest()
    );
    assert!(s3a_entry.rules_manifest.capability_closure.is_some());
    let s3a_closure = s3a_entry
        .rules_manifest
        .capability_closure
        .as_ref()
        .unwrap();
    assert_eq!(s3a_closure.len(), 3);
    assert_eq!(s3a_closure[0].key, "rules/state-based-actions-combat");
    assert_eq!(s3a_closure[1].key, "rules/turn-structure");
    assert_eq!(s3a_closure[2].key, "rules/zone-incarnation");
    assert!(catalog.supported(&s3a_id, ExecutionProgramV1::MagicRules));
    assert_ne!(s3a_id, ts_id, "S3.A gets its own generated semantic identity");
    assert!(s3a_entry.manifest.format_contract_id.is_none());
    assert!(s3a_entry.manifest.content_contract_id.is_none());

    let s3b_id = magic_s3_b_basic_priority_0_1_0_semantic_contract_id();
    let s3b_entry = catalog.resolve(&s3b_id).unwrap();
    assert_eq!(s3b_entry.semantic_contract_id, s3b_id);
    assert_eq!(
        s3b_entry.manifest.rules_contract_id,
        magic_s3_b_basic_priority_0_1_0_rules_contract_id()
    );
    assert_eq!(
        s3b_entry.manifest,
        magic_s3_b_basic_priority_0_1_0_semantic_manifest()
    );
    assert_eq!(
        s3b_entry.rules_manifest,
        magic_s3_b_basic_priority_0_1_0_rules_manifest()
    );
    let s3b_closure = s3b_entry.rules_manifest.capability_closure.as_ref().unwrap();
    assert_eq!(s3b_closure.len(), 4);
    assert_eq!(s3b_closure[0].key, "rules/basic-priority");
    assert_eq!(s3b_closure[1].key, "rules/state-based-actions-combat");
    assert_eq!(s3b_closure[2].key, "rules/turn-structure");
    assert_eq!(s3b_closure[3].key, "rules/zone-incarnation");
    assert!(catalog.supported(&s3b_id, ExecutionProgramV1::MagicRules));
    assert_ne!(s3b_id, s3a_id, "S3.B must not reinterpret the S3.A identity");
    assert_ne!(s3b_id, ts_id, "S3.B must not reinterpret the S1 identity");
}

#[test]
fn production_catalog_turn_structure_known_and_executable() {
    let catalog = RuntimeSemanticCatalog::production();
    let ts_id = magic_turn_structure_0_1_0_semantic_contract_id();
    let entry = catalog.resolve(&ts_id).expect(
        "turn-structure contract must resolve in production catalog",
    );

    // Known: the contract resolves to an immutable manifest.
    assert!(catalog.resolve(&ts_id).is_some());

    // Executable: MagicRules IS supported for this exact contract (Task 8).
    assert!(
        catalog.supported(&ts_id, ExecutionProgramV1::MagicRules),
        "MagicRules MUST be supported for turn-structure contract after Task 8",
    );

    // The authority is comprehensive_rules with the exact CR snapshot.
    assert!(matches!(&entry.rules_manifest.rules_authority, RulesAuthorityV1::ComprehensiveRules { snapshot_id } if snapshot_id == "wotc-cr-2026-08-07-txt-20260819-sha256-4381ad1b39ab2c05f7d03633a20f711ed37277074d3266dcba5f38cbb527423f"));

    // Exact one-element capability closure.
    let closure = entry.rules_manifest.capability_closure.clone().expect("closure must be present");
    assert_eq!(closure.len(), 1);
    assert_eq!(closure[0].key, "rules/turn-structure");
    assert_eq!(closure[0].version, "0.1.0");

    // Null dimensions.
    assert!(entry.manifest.format_contract_id.is_none());
    assert!(entry.manifest.content_contract_id.is_none());

    // IDs match generated constants.
    assert_eq!(
        entry.semantic_contract_id.as_str(),
        SEMANTIC_CONTRACT_CATALOG_MAGIC_TURN_STRUCTURE_0_1_0_SEMANTIC_CONTRACT_HEX
    );
    assert_eq!(
        entry.manifest.rules_contract_id.as_str(),
        SEMANTIC_CONTRACT_CATALOG_MAGIC_TURN_STRUCTURE_0_1_0_RULES_CONTRACT_HEX
    );
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

#[test]
fn program_kernel_construction_error_maps_to_controller_error() {
    // Fix-05: ProgramKernelConstructionErrorV1::UnsupportedProgram maps
    // deterministically onto ControllerError::SemanticContractUnsupported.
    let kernel_err = mtgml_rules::ProgramKernelConstructionErrorV1::UnsupportedProgram;
    let controller_err: ControllerError = kernel_err.into();
    assert!(
        matches!(controller_err, ControllerError::SemanticContractUnsupported),
        "UnsupportedProgram must map to SemanticContractUnsupported, not Backend(String)"
    );
}

#[test]
fn exact_turn_structure_contract_resolves_in_production_catalog() {
    // Step 1: Independently author the exact RulesContractManifestV1
    // for rules/turn-structure@0.1.0 under comprehensive_rules authority.
    let rules_manifest = RulesContractManifestV1 {
        rules_authority: RulesAuthorityV1::ComprehensiveRules {
            snapshot_id: "wotc-cr-2026-08-07-txt-20260819-sha256-4381ad1b39ab2c05f7d03633a20f711ed37277074d3266dcba5f38cbb527423f".to_string(),
        },
        capability_closure: Some(vec![CapabilityRequirementV1 {
            key: "rules/turn-structure".to_string(),
            version: "0.1.0".to_string(),
        }]),
    };

    // Step 2: Mechanically compute RulesContractIdV1 using production digest.
    let rules_contract_id = calculate_rules_contract_id_v1(&rules_manifest)
        .expect("independently authored rules manifest must be valid");

    // Step 3: Independently author the exact SemanticContractManifestV1.
    let semantic_manifest = SemanticContractManifestV1 {
        rules_contract_id,
        format_contract_id: None,
        content_contract_id: None,
    };

    // Step 4: Mechanically compute SemanticContractIdV1.
    let semantic_contract_id = calculate_semantic_contract_id_v1(&semantic_manifest)
        .expect("independently authored semantic manifest must be valid");

    // Step 5: Construct production catalog.
    let catalog = RuntimeSemanticCatalog::production();

    // Step 6: The exact turn-structure contract MUST resolve.
    let entry = catalog.resolve(&semantic_contract_id).expect(
        "exact turn-structure contract must resolve in production catalog",
    );
    assert_eq!(entry.semantic_contract_id, semantic_contract_id);
}

#[test]
fn wrong_snapshot_rejected() {
    // Independently author an otherwise-identical contract with a
    // DIFFERENT CR snapshot. Mechanically derive its semantic ID.
    let rules_manifest = RulesContractManifestV1 {
        rules_authority: RulesAuthorityV1::ComprehensiveRules {
            snapshot_id: "wotc-cr-2026-08-07-txt-20260819-sha256-different-snapshot-value-0000000000000000000000000000000000000000000000000000".to_string(),
        },
        capability_closure: Some(vec![CapabilityRequirementV1 {
            key: "rules/turn-structure".to_string(),
            version: "0.1.0".to_string(),
        }]),
    };
    let rules_contract_id = calculate_rules_contract_id_v1(&rules_manifest)
        .expect("wrong-snapshot manifest must be structurally valid");
    let semantic_manifest = SemanticContractManifestV1 {
        rules_contract_id,
        format_contract_id: None,
        content_contract_id: None,
    };
    let wrong_id = calculate_semantic_contract_id_v1(&semantic_manifest)
        .expect("wrong-snapshot semantic manifest must be valid");

    let catalog = RuntimeSemanticCatalog::production();
    assert!(
        catalog.resolve(&wrong_id).is_none(),
        "wrong snapshot must not resolve",
    );
}

#[test]
fn wrong_closure_rejected() {
    // Independently author an otherwise-identical contract with a
    // DIFFERENT capability closure. Mechanically derive its semantic ID.
    let rules_manifest = RulesContractManifestV1 {
        rules_authority: RulesAuthorityV1::ComprehensiveRules {
            snapshot_id: "wotc-cr-2026-08-07-txt-20260819-sha256-4381ad1b39ab2c05f7d03633a20f711ed37277074d3266dcba5f38cbb527423f".to_string(),
        },
        capability_closure: Some(vec![CapabilityRequirementV1 {
            key: "rules/turn-structure".to_string(),
            version: "0.2.0".to_string(),
        }]),
    };
    let rules_contract_id = calculate_rules_contract_id_v1(&rules_manifest)
        .expect("wrong-closure manifest must be structurally valid");
    let semantic_manifest = SemanticContractManifestV1 {
        rules_contract_id,
        format_contract_id: None,
        content_contract_id: None,
    };
    let wrong_id = calculate_semantic_contract_id_v1(&semantic_manifest)
        .expect("wrong-closure semantic manifest must be valid");

    let catalog = RuntimeSemanticCatalog::production();
    assert!(
        catalog.resolve(&wrong_id).is_none(),
        "wrong closure must not resolve",
    );
}

#[test]
fn magic_rules_kernel_unsupported() {
    // Task 3 must NOT activate MagicRules. The kernel construction
    // path for MagicRules still returns UnsupportedProgram.
    let result = mtgml_rules::ProgramKernelV1::for_program(ExecutionProgramV1::MagicRules);
    assert!(
        matches!(result, Err(mtgml_rules::ProgramKernelConstructionErrorV1::UnsupportedProgram)),
        "MagicRules kernel construction must still return UnsupportedProgram",
    );
}

#[test]
fn synthetic_not_supported_with_magic_rules() {
    // Synthetic contract + MagicRules = not supported (authority family mismatch).
    let catalog = RuntimeSemanticCatalog::production();
    let syn_id = synthetic_legacy_default_semantic_contract_id();
    assert!(
        !catalog.supported(&syn_id, ExecutionProgramV1::MagicRules),
        "MagicRules must NOT be supported for SyntheticLegacy",
    );
}

#[test]
fn support_matrix_exact_program_contract_pairing() {
    // ADR 0055 §2.9: supported() is the frozen exact
    // (program_kind, SemanticContractIdV1) runtime-support predicate.
    // Known contract does NOT automatically imply support.
    let catalog = RuntimeSemanticCatalog::production();
    let syn_id = synthetic_legacy_default_semantic_contract_id();
    let ts_id = magic_turn_structure_0_1_0_semantic_contract_id();
    let unknown = SemanticContractIdV1::from_digest_bytes([0u8; 32]);

    // synthetic + SyntheticRulesCompat = true
    assert!(
        catalog.supported(&syn_id, ExecutionProgramV1::SyntheticRulesCompat),
        "SyntheticRulesCompat must be supported for synthetic_legacy_default",
    );

    // synthetic + MagicRules = false
    assert!(
        !catalog.supported(&syn_id, ExecutionProgramV1::MagicRules),
        "MagicRules must NOT be supported for synthetic_legacy_default",
    );

    // turn-structure + SyntheticRulesCompat = false
    assert!(
        !catalog.supported(&ts_id, ExecutionProgramV1::SyntheticRulesCompat),
        "SyntheticRulesCompat must NOT be supported for turn-structure contract",
    );

    // turn-structure + MagicRules = true (Task 8: exact admitted MagicRules)
    assert!(
        catalog.supported(&ts_id, ExecutionProgramV1::MagicRules),
        "MagicRules must be supported for exact turn-structure contract",
    );

    // unknown + either program = false
    assert!(
        !catalog.supported(&unknown, ExecutionProgramV1::SyntheticRulesCompat),
        "unknown ID must not be supported with SyntheticRulesCompat",
    );
    assert!(
        !catalog.supported(&unknown, ExecutionProgramV1::MagicRules),
        "unknown ID must not be supported with MagicRules",
    );
}

#[test]
fn exact_turn_structure_support() {
    let catalog = RuntimeSemanticCatalog::production();
    let ts_id = magic_turn_structure_0_1_0_semantic_contract_id();

    // POSITIVE: exact S1 contract + MagicRules = supported
    assert!(
        catalog.supported(&ts_id, ExecutionProgramV1::MagicRules),
        "exact turn-structure contract MUST be supported under MagicRules",
    );

    // NEGATIVE MATRIX: known does not imply supported
    let syn_id = synthetic_legacy_default_semantic_contract_id();
    let unknown = SemanticContractIdV1::from_digest_bytes([0u8; 32]);

    // MagicRules + synthetic semantic ID = false
    assert!(
        !catalog.supported(&syn_id, ExecutionProgramV1::MagicRules),
        "MagicRules must NOT be supported for synthetic legacy",
    );

    // SyntheticRulesCompat + exact S1 semantic ID = false
    assert!(
        !catalog.supported(&ts_id, ExecutionProgramV1::SyntheticRulesCompat),
        "SyntheticRulesCompat must NOT be supported for turn-structure",
    );

    // MagicRules + arbitrary known ComprehensiveRules contract = false
    let arbitrary_cr_rules_manifest = RulesContractManifestV1 {
        rules_authority: RulesAuthorityV1::ComprehensiveRules {
            snapshot_id: "wotc-cr-2026-08-07-txt-20260819-sha256-different-snapshot-0000000000000000000000000000000000000000000000000000".to_string(),
        },
        capability_closure: Some(vec![CapabilityRequirementV1 {
            key: "rules/turn-structure".to_string(),
            version: "0.1.0".to_string(),
        }]),
    };
    let arbitrary_cr_rules_id =
        calculate_rules_contract_id_v1(&arbitrary_cr_rules_manifest)
            .expect("arbitrary CR manifest is valid");
    let arbitrary_cr_semantic_manifest = SemanticContractManifestV1 {
        rules_contract_id: arbitrary_cr_rules_id,
        format_contract_id: None,
        content_contract_id: None,
    };
    let arbitrary_cr_semantic_id =
        calculate_semantic_contract_id_v1(&arbitrary_cr_semantic_manifest)
            .expect("arbitrary CR semantic manifest is valid");
    assert!(
        !catalog.supported(&arbitrary_cr_semantic_id, ExecutionProgramV1::MagicRules),
        "MagicRules must NOT be supported for arbitrary ComprehensiveRules contract",
    );

    // unknown semantic ID = false
    assert!(
        !catalog.supported(&unknown, ExecutionProgramV1::MagicRules),
        "unknown ID must not be supported with MagicRules",
    );
}

#[test]
fn exact_turn_structure_manifest_evidence() {
    let catalog = RuntimeSemanticCatalog::production();
    let ts_id = magic_turn_structure_0_1_0_semantic_contract_id();
    let entry = catalog.resolve(&ts_id).expect("S1 contract must resolve");

    // Authority = ComprehensiveRules pinned snapshot.
    assert!(matches!(
        &entry.rules_manifest.rules_authority,
        RulesAuthorityV1::ComprehensiveRules { snapshot_id }
        if snapshot_id == "wotc-cr-2026-08-07-txt-20260819-sha256-4381ad1b39ab2c05f7d03633a20f711ed37277074d3266dcba5f38cbb527423f"
    ));

    // Capability closure = exactly [rules/turn-structure@0.1.0].
    let closure = entry.rules_manifest.capability_closure.clone().expect("closure must be present");
    assert_eq!(closure.len(), 1);
    assert_eq!(closure[0].key, "rules/turn-structure");
    assert_eq!(closure[0].version, "0.1.0");

    // Format = None, Content = None.
    assert!(entry.manifest.format_contract_id.is_none());
    assert!(entry.manifest.content_contract_id.is_none());

    // IDs match generated constants.
    assert_eq!(
        entry.semantic_contract_id.as_str(),
        SEMANTIC_CONTRACT_CATALOG_MAGIC_TURN_STRUCTURE_0_1_0_SEMANTIC_CONTRACT_HEX
    );
    assert_eq!(
        entry.manifest.rules_contract_id.as_str(),
        SEMANTIC_CONTRACT_CATALOG_MAGIC_TURN_STRUCTURE_0_1_0_RULES_CONTRACT_HEX
    );
}

#[test]
fn exact_turn_structure_negative_contract_evidence() {
    // Synthetic ID + MagicRules: not supported.
    let catalog = RuntimeSemanticCatalog::production();
    let syn_id = synthetic_legacy_default_semantic_contract_id();
    assert!(
        !catalog.supported(&syn_id, ExecutionProgramV1::MagicRules),
        "synthetic+MagicRules must not be supported"
    );

    // S1 ID + SyntheticRulesCompat: not supported.
    let ts_id = magic_turn_structure_0_1_0_semantic_contract_id();
    assert!(
        !catalog.supported(&ts_id, ExecutionProgramV1::SyntheticRulesCompat),
        "S1+SyntheticRulesCompat must not be supported"
    );
}
