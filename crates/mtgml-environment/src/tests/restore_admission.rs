use crate::checkpoint::{
    CheckpointValidationError, EnvironmentCheckpointV5, CHECKPOINT_CODEC_ID_V5,
    CHECKPOINT_CODEC_SEMANTIC_VERSION_V5,
};
use crate::semantic_catalog::{
    admit_restore, CatalogEntry, RestoreAdmissionError, RuntimeSemanticCatalog,
};
use crate::semantic_catalog_generated::{
    synthetic_legacy_default_rules_manifest,
    synthetic_legacy_default_semantic_manifest,
};
use mtgml_decision::{AuthoritativeDecisionRequestV2, DecisionDomainV2, DecisionVisibility};
use mtgml_model::{
    CapabilityRequirementV1, CheckpointCodecIdentity, EnvironmentLimitCounters,
    ExecutionIdentityV1, ExecutionProgramV1, EpisodeStatus, FullStateDigestV4, PlayerId,
    RulesAuthorityV1, RulesContractManifestV1, SemanticContractIdV1,
    SemanticContractManifestV1, StateRevision,
};
use mtgml_persistence::semantic_contract_digest::{
    calculate_rules_contract_id_v1, calculate_semantic_contract_id_v1,
};
use mtgml_state::PendingDecisionRecordV2;
use mtgml_state::{
    construct_synthetic_engine_state, EngineState, SyntheticResetInputs, SyntheticV4Setup,
};

// === Helpers ===

fn synthetic_identity() -> ExecutionIdentityV1 {
    ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::SyntheticRulesCompat,
        semantic_contract_id: synthetic_legacy_default_semantic_contract_id(),
    }
}

fn v5_codec() -> CheckpointCodecIdentity {
    CheckpointCodecIdentity {
        codec_id: CHECKPOINT_CODEC_ID_V5.to_string(),
        semantic_version: CHECKPOINT_CODEC_SEMANTIC_VERSION_V5.to_string(),
    }
}

fn valid_v5_checkpoint(identity: ExecutionIdentityV1) -> EnvironmentCheckpointV5 {
    let bk = backend();
    let v4 = bk.checkpoint().unwrap();
    EnvironmentCheckpointV5::new(
        v4.state,
        v4.status,
        v4.limit_counters,
        v5_codec(),
        identity,
    )
    .unwrap()
}

// A state that passes generic validate_engine_state but fails the synthetic
// program's runtime validation: a standalone ChooseNumber decision (not a
// valid synthetic entry).
fn synthetic_incompatible_state() -> EngineState {
    let mut state =
        construct_synthetic_engine_state(SyntheticResetInputs {
            players: [PlayerId(1), PlayerId(2)],
            root_seed: seed(),
            setup: SyntheticV4Setup::m2_compatibility(),
        })
        .unwrap();
    state.execution.pending_decision = Some(PendingDecisionRecordV2 {
        request: AuthoritativeDecisionRequestV2 {
            decision_id: mtgml_model::DecisionId(1),
            player_decision_id: mtgml_model::PlayerDecisionIdV1(1),
            state_revision: StateRevision(0),
            actor: PlayerId(1),
            visibility: DecisionVisibility::Public,
            decision: DecisionDomainV2::ChooseNumber {
                minimum: 0,
                maximum: 3,
            },
            candidates: Vec::new(),
            continuation_id: None,
        },
    });
    // The generic state validator passes; the synthetic kernel rejects it.
    mtgml_state::validate_engine_state(&state).unwrap();
    state
}

// === Phase 1: checkpoint structural validation ===

#[test]
fn invalid_checkpoint_rejects_at_structural_validation() {
    let catalog = RuntimeSemanticCatalog::production();
    let mut checkpoint = valid_v5_checkpoint(synthetic_identity());
    // Corrupt the state digest so validate() rejects it.
    checkpoint.state_digest = FullStateDigestV4::from_digest_bytes([0xff; 32]);
    assert_eq!(
        checkpoint.state_digest,
        FullStateDigestV4::from_digest_bytes([0xff; 32])
    );
    let result = admit_restore(&catalog, &checkpoint);
    assert_eq!(
        result.unwrap_err(),
        RestoreAdmissionError::CheckpointValidation(CheckpointValidationError::StateDigest)
    );
}

#[test]
fn tampered_checkpoint_digest_rejects_at_structural_validation() {
    let catalog = RuntimeSemanticCatalog::production();
    let mut checkpoint = valid_v5_checkpoint(synthetic_identity());
    checkpoint.checkpoint_digest = mtgml_model::CheckpointDigestV5::from_digest_bytes([0xee; 32]);
    let result = admit_restore(&catalog, &checkpoint);
    assert_eq!(
        result.unwrap_err(),
        RestoreAdmissionError::CheckpointValidation(CheckpointValidationError::CheckpointDigest)
    );
}

// === Phase 3: unknown semantic contract ===

#[test]
fn unknown_semantic_contract_id_rejected() {
    let catalog = RuntimeSemanticCatalog::production();
    let identity = ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::SyntheticRulesCompat,
        semantic_contract_id: SemanticContractIdV1::from_digest_bytes([0u8; 32]),
    };
    let checkpoint = valid_v5_checkpoint(identity);
    let result = admit_restore(&catalog, &checkpoint);
    assert_eq!(
        result.unwrap_err(),
        RestoreAdmissionError::SemanticContractUnknown
    );
}

// === Phase 4: semantic contract digest mismatch ===

#[test]
fn semantic_contract_digest_mismatch_rejected() {
    // Test-only catalog: entry has a wrong semantic_contract_id that does not
    // recompute from its manifest. The catalog is a frozen data table; the
    // admission function performs the recompute-and-compare.
    let wrong_id = SemanticContractIdV1::from_digest_bytes([0xab; 32]);
    let entry = CatalogEntry {
        semantic_contract_id: wrong_id.clone(),
        manifest: synthetic_legacy_default_semantic_manifest(),
        rules_manifest: synthetic_legacy_default_rules_manifest(),
    };
    let catalog = RuntimeSemanticCatalog::from_entries(vec![entry]);

    let identity = ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::SyntheticRulesCompat,
        semantic_contract_id: wrong_id.clone(),
    };
    let checkpoint = valid_v5_checkpoint(identity);
    let result = admit_restore(&catalog, &checkpoint);
    assert_eq!(
        result.unwrap_err(),
        RestoreAdmissionError::SemanticContractDigestMismatch
    );
}

// === Phase 5: rules contract digest mismatch ===

#[test]
fn rules_contract_digest_mismatch_rejected() {
    // Build a valid ComprehensiveRules manifest + its derived IDs, then create
    // a catalog entry whose rules_manifest is the SyntheticLegacy manifest (which
    // recomputes to a DIFFERENT rules ID than the manifest's rules_contract_id).
    let cr_rules_manifest = RulesContractManifestV1 {
        rules_authority: RulesAuthorityV1::ComprehensiveRules {
            snapshot_id: "test-cr-2026-01-01".to_string(),
        },
        capability_closure: Some(vec![CapabilityRequirementV1 {
            key: "rules/test".to_string(),
            version: "1.0.0".to_string(),
        }]),
    };
    let cr_rules_id = calculate_rules_contract_id_v1(&cr_rules_manifest).unwrap();
    let cr_manifest = SemanticContractManifestV1 {
        rules_contract_id: cr_rules_id,
        format_contract_id: None,
        content_contract_id: None,
    };
    let cr_semantic_id = calculate_semantic_contract_id_v1(&cr_manifest).unwrap();

    // Entry: semantic ID + manifest are consistent (phase 4 passes), but the
    // rules_manifest is the SyntheticLegacy one (phase 5 fails).
    let entry = CatalogEntry {
        semantic_contract_id: cr_semantic_id.clone(),
        manifest: cr_manifest,
        rules_manifest: synthetic_legacy_default_rules_manifest(),
    };
    let catalog = RuntimeSemanticCatalog::from_entries(vec![entry]);

    let identity = ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::MagicRules,
        semantic_contract_id: cr_semantic_id,
    };
    let checkpoint = valid_v5_checkpoint(identity);
    let result = admit_restore(&catalog, &checkpoint);
    assert_eq!(
        result.unwrap_err(),
        RestoreAdmissionError::RulesContractDigestMismatch
    );
}

// === Controller-level rejection nonmutation matrix ===
// (spec §12 observable invariant: rejected controller.restore() must not mutate
//  pre/post checkpoint or replay identity for ANY admission-rejection phase)

#[test]
fn controller_restore_rejects_corrupt_checkpoint_without_mutation() {
    let controller = TrustedEnvironmentController::new(backend());
    let before_checkpoint = controller.checkpoint().unwrap();
    let before_replay = controller.export_replay().unwrap();

    let mut checkpoint = valid_v5_checkpoint(synthetic_identity());
    checkpoint.state_digest = FullStateDigestV4::from_digest_bytes([0xff; 32]);
    let result = controller.restore(checkpoint);
    assert!(matches!(
        result,
        Err(ControllerError::CheckpointValidation(
            CheckpointValidationError::StateDigest
        ))
    ));
    assert_eq!(controller.checkpoint().unwrap(), before_checkpoint);
    assert_eq!(controller.export_replay().unwrap(), before_replay);
}

#[test]
fn controller_restore_rejects_unknown_contract_without_mutation() {
    let controller = TrustedEnvironmentController::new(backend());
    let before_checkpoint = controller.checkpoint().unwrap();
    let before_replay = controller.export_replay().unwrap();

    let identity = ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::SyntheticRulesCompat,
        semantic_contract_id: SemanticContractIdV1::from_digest_bytes([0u8; 32]),
    };
    let checkpoint = valid_v5_checkpoint(identity);
    let result = controller.restore(checkpoint);
    assert!(matches!(
        result,
        Err(ControllerError::CheckpointValidation(
            CheckpointValidationError::SemanticContractUnknown
        ))
    ));
    assert_eq!(controller.checkpoint().unwrap(), before_checkpoint);
    assert_eq!(controller.export_replay().unwrap(), before_replay);
}

#[test]
fn controller_restore_rejects_program_authority_mismatch_without_mutation() {
    let controller = TrustedEnvironmentController::new(backend());
    let before_checkpoint = controller.checkpoint().unwrap();
    let before_replay = controller.export_replay().unwrap();

    let identity = ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::MagicRules,
        semantic_contract_id: synthetic_legacy_default_semantic_contract_id(),
    };
    let checkpoint = valid_v5_checkpoint(identity);
    let result = controller.restore(checkpoint);
    assert!(matches!(result, Err(ControllerError::ProgramAuthorityMismatch)));
    assert_eq!(controller.checkpoint().unwrap(), before_checkpoint);
    assert_eq!(controller.export_replay().unwrap(), before_replay);
}

#[test]
fn controller_restore_rejects_incompatible_state_without_mutation() {
    let controller = TrustedEnvironmentController::new(backend());
    let before_checkpoint = controller.checkpoint().unwrap();
    let before_replay = controller.export_replay().unwrap();

    let state = synthetic_incompatible_state();
    let checkpoint = EnvironmentCheckpointV5::new(
        state,
        EpisodeStatus::Running,
        EnvironmentLimitCounters::default(),
        v5_codec(),
        synthetic_identity(),
    )
    .unwrap();
    let result = controller.restore(checkpoint);
    assert!(matches!(
        result,
        Err(ControllerError::ProgramStateIncompatible)
    ));
    assert_eq!(controller.checkpoint().unwrap(), before_checkpoint);
    assert_eq!(controller.export_replay().unwrap(), before_replay);
}

#[test]
fn controller_restore_rejects_semantic_contract_digest_mismatch_without_mutation() {
    let wrong_id = SemanticContractIdV1::from_digest_bytes([0xab; 32]);
    let entry = CatalogEntry {
        semantic_contract_id: wrong_id.clone(),
        manifest: synthetic_legacy_default_semantic_manifest(),
        rules_manifest: synthetic_legacy_default_rules_manifest(),
    };
    let catalog = RuntimeSemanticCatalog::from_entries(vec![entry]);

    let controller = TrustedEnvironmentController::new(backend());
    let before_checkpoint = controller.checkpoint().unwrap();
    let before_replay = controller.export_replay().unwrap();

    let identity = ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::SyntheticRulesCompat,
        semantic_contract_id: wrong_id,
    };
    let checkpoint = valid_v5_checkpoint(identity);
    let result = controller.restore_with_catalog(checkpoint, catalog);
    assert!(matches!(
        result,
        Err(ControllerError::CheckpointValidation(
            CheckpointValidationError::SemanticContractDigestMismatch
        ))
    ));
    assert_eq!(controller.checkpoint().unwrap(), before_checkpoint);
    assert_eq!(controller.export_replay().unwrap(), before_replay);
}

#[test]
fn controller_restore_rejects_rules_contract_digest_mismatch_without_mutation() {
    let cr_rules_manifest = RulesContractManifestV1 {
        rules_authority: RulesAuthorityV1::ComprehensiveRules {
            snapshot_id: "test-cr-2026-01-01".to_string(),
        },
        capability_closure: Some(vec![CapabilityRequirementV1 {
            key: "rules/test".to_string(),
            version: "1.0.0".to_string(),
        }]),
    };
    let cr_rules_id = calculate_rules_contract_id_v1(&cr_rules_manifest).unwrap();
    let cr_manifest = SemanticContractManifestV1 {
        rules_contract_id: cr_rules_id,
        format_contract_id: None,
        content_contract_id: None,
    };
    let cr_semantic_id = calculate_semantic_contract_id_v1(&cr_manifest).unwrap();

    let entry = CatalogEntry {
        semantic_contract_id: cr_semantic_id.clone(),
        manifest: cr_manifest,
        rules_manifest: synthetic_legacy_default_rules_manifest(),
    };
    let catalog = RuntimeSemanticCatalog::from_entries(vec![entry]);

    let controller = TrustedEnvironmentController::new(backend());
    let before_checkpoint = controller.checkpoint().unwrap();
    let before_replay = controller.export_replay().unwrap();

    let identity = ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::MagicRules,
        semantic_contract_id: cr_semantic_id,
    };
    let checkpoint = valid_v5_checkpoint(identity);
    let result = controller.restore_with_catalog(checkpoint, catalog);
    assert!(matches!(
        result,
        Err(ControllerError::CheckpointValidation(
            CheckpointValidationError::RulesContractDigestMismatch
        ))
    ));
    assert_eq!(controller.checkpoint().unwrap(), before_checkpoint);
    assert_eq!(controller.export_replay().unwrap(), before_replay);
}

#[test]
fn controller_restore_rejects_unsupported_program_without_mutation() {
    let cr_rules_manifest = RulesContractManifestV1 {
        rules_authority: RulesAuthorityV1::ComprehensiveRules {
            snapshot_id: "test-cr-2026-01-01".to_string(),
        },
        capability_closure: Some(vec![CapabilityRequirementV1 {
            key: "rules/test".to_string(),
            version: "1.0.0".to_string(),
        }]),
    };
    let cr_rules_id = calculate_rules_contract_id_v1(&cr_rules_manifest).unwrap();
    let cr_manifest = SemanticContractManifestV1 {
        rules_contract_id: cr_rules_id,
        format_contract_id: None,
        content_contract_id: None,
    };
    let cr_semantic_id = calculate_semantic_contract_id_v1(&cr_manifest).unwrap();

    let entry = CatalogEntry {
        semantic_contract_id: cr_semantic_id.clone(),
        manifest: cr_manifest,
        rules_manifest: cr_rules_manifest,
    };
    let catalog = RuntimeSemanticCatalog::from_entries(vec![entry]);

    let controller = TrustedEnvironmentController::new(backend());
    let before_checkpoint = controller.checkpoint().unwrap();
    let before_replay = controller.export_replay().unwrap();

    let identity = ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::MagicRules,
        semantic_contract_id: cr_semantic_id,
    };
    let checkpoint = valid_v5_checkpoint(identity);
    let result = controller.restore_with_catalog(checkpoint, catalog);
    assert!(matches!(result, Err(ControllerError::SemanticContractUnsupported)));
    assert_eq!(controller.checkpoint().unwrap(), before_checkpoint);
    assert_eq!(controller.export_replay().unwrap(), before_replay);
}

// === Phase 6: program × authority mismatch ===

#[test]
fn program_authority_mismatch_rejected() {
    let catalog = RuntimeSemanticCatalog::production();
    // SyntheticLegacy has synthetic_legacy authority; MagicRules pairs only
    // with comprehensive_rules → mismatch.
    let identity = ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::MagicRules,
        semantic_contract_id: synthetic_legacy_default_semantic_contract_id(),
    };
    let checkpoint = valid_v5_checkpoint(identity);
    let result = admit_restore(&catalog, &checkpoint);
    assert_eq!(
        result.unwrap_err(),
        RestoreAdmissionError::ProgramAuthorityMismatch
    );
}

// === Phase 7: runtime unsupported ===

#[test]
fn unsupported_program_rejected_as_semantic_contract_unsupported() {
    // Build a valid ComprehensiveRules contract + SyntheticLegacy catalog entry,
    // then create a checkpoint with MagicRules + the CR contract ID.
    // Phase 3: CR contract resolves (known meaning)
    // Phase 4: semantic manifest recomputes to CR semantic ID (matches)
    // Phase 5: rules manifest recomputes to CR rules ID (matches)
    // Phase 6: MagicRules × ComprehensiveRules is the frozen compatible pairing (passes)
    // Phase 7: MagicRules is NOT supported by this runtime (pre-S1) → SemanticContractUnsupported
    let cr_rules_manifest = RulesContractManifestV1 {
        rules_authority: RulesAuthorityV1::ComprehensiveRules {
            snapshot_id: "test-cr-2026-01-01".to_string(),
        },
        capability_closure: Some(vec![CapabilityRequirementV1 {
            key: "rules/test".to_string(),
            version: "1.0.0".to_string(),
        }]),
    };
    let cr_rules_id = calculate_rules_contract_id_v1(&cr_rules_manifest).unwrap();
    let cr_manifest = SemanticContractManifestV1 {
        rules_contract_id: cr_rules_id,
        format_contract_id: None,
        content_contract_id: None,
    };
    let cr_semantic_id = calculate_semantic_contract_id_v1(&cr_manifest).unwrap();

    let entry = CatalogEntry {
        semantic_contract_id: cr_semantic_id.clone(),
        manifest: cr_manifest,
        rules_manifest: cr_rules_manifest,
    };
    let catalog = RuntimeSemanticCatalog::from_entries(vec![entry]);

    let identity = ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::MagicRules,
        semantic_contract_id: cr_semantic_id,
    };
    let checkpoint = valid_v5_checkpoint(identity);
    let result = admit_restore(&catalog, &checkpoint);
    assert_eq!(
        result.unwrap_err(),
        RestoreAdmissionError::SemanticContractUnsupported
    );
}

// === Phase 8: program × EngineState incompatibility ===

#[test]
fn program_state_incompatible_rejected() {
    let catalog = RuntimeSemanticCatalog::production();
    let state = synthetic_incompatible_state();
    let codec = v5_codec();
    let identity = synthetic_identity();
    let checkpoint = EnvironmentCheckpointV5::new(
        state,
        EpisodeStatus::Running,
        EnvironmentLimitCounters::default(),
        codec,
        identity,
    )
    .unwrap();
    let result = admit_restore(&catalog, &checkpoint);
    assert_eq!(
        result.unwrap_err(),
        RestoreAdmissionError::ProgramStateIncompatible
    );
}

// === Positive: valid SyntheticRulesCompat + SyntheticLegacy admits ===

#[test]
fn synthetic_legacy_admits_under_synthetic_program() {
    let catalog = RuntimeSemanticCatalog::production();
    let checkpoint = valid_v5_checkpoint(synthetic_identity());
    let result = admit_restore(&catalog, &checkpoint);
    assert!(result.is_ok(), "valid synthetic restore must be admitted");
}

// === Admission phase ordering ===

#[test]
fn earlier_phase_defects_are_not_reported_as_later_failures() {
    // An unknown semantic contract (phase 3) must NOT be reported as a
    // program-authority mismatch (phase 6) or unsupported (phase 7).
    let catalog = RuntimeSemanticCatalog::production();
    let identity = ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::MagicRules,
        semantic_contract_id: SemanticContractIdV1::from_digest_bytes([0u8; 32]),
    };
    let checkpoint = valid_v5_checkpoint(identity);
    let result = admit_restore(&catalog, &checkpoint);
    assert_eq!(
        result.unwrap_err(),
        RestoreAdmissionError::SemanticContractUnknown
    );
}

// === Phase ordering: digest mismatch precedes authority check ===

#[test]
fn digest_mismatch_precedes_authority_mismatch() {
    // A catalog entry with a mismatched semantic ID (phase 4) must fail at
    // phase 4, NOT reported as a phase-6 ProgramAuthorityMismatch.
    let wrong_id = SemanticContractIdV1::from_digest_bytes([0xab; 32]);
    let entry = CatalogEntry {
        semantic_contract_id: wrong_id.clone(),
        manifest: synthetic_legacy_default_semantic_manifest(),
        rules_manifest: synthetic_legacy_default_rules_manifest(),
    };
    let catalog = RuntimeSemanticCatalog::from_entries(vec![entry]);

    let identity = ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::MagicRules,
        semantic_contract_id: wrong_id,
    };
    let checkpoint = valid_v5_checkpoint(identity);
    let result = admit_restore(&catalog, &checkpoint);
    assert_eq!(
        result.unwrap_err(),
        RestoreAdmissionError::SemanticContractDigestMismatch
    );
}
