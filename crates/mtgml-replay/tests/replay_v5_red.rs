use mtgml_decision::DecisionResponseV2;
use mtgml_model::{
    CapabilityRequirementV1, CheckpointCodecIdentity, ContentDigest, EnvironmentLimitCounters,
    EpisodeStatus, ExecutionIdentityV1, ExecutionProgramV1, FullStateDigestV4, PlayerId,
    RulesAuthorityV1, RulesContractIdV1, RulesContractManifestV1, SemanticContractIdV1,
    SemanticContractManifestV1, StateRevision,
};
use mtgml_persistence::semantic_contract_digest::{
    calculate_rules_contract_id_v1, calculate_semantic_contract_id_v1,
};
use mtgml_replay::{
    AuthoritativeReplayV5, DeckIdentityV1, InitialEnvironmentIdentityV5, KernelIdentityV1,
    RandomnessIdentityV2, ReplayManifestV5, ReplayRecorderV5, ReplaySchemaVersionsV5, ReplayStepV5,
    ReplayValidationError, REPLAY_FILE_SCHEMA_V5, REPLAY_MANIFEST_SCHEMA_V5, REPLAY_STEP_SCHEMA_V5,
};

const CHECKPOINT_CODEC_ID_V5: &str = "in-memory-reference";
const CHECKPOINT_CODEC_VERSION_V5: &str = "5";

fn synthetic_rules_manifest() -> RulesContractManifestV1 {
    RulesContractManifestV1 {
        rules_authority: RulesAuthorityV1::SyntheticLegacy,
        capability_closure: None,
    }
}

fn semantic_contract_material(
    rules_manifest: RulesContractManifestV1,
) -> mtgml_replay::SemanticContractMaterialV5 {
    let rules_contract_id = calculate_rules_contract_id_v1(&rules_manifest).unwrap();
    let semantic_manifest = SemanticContractManifestV1 {
        rules_contract_id,
        format_contract_id: None,
        content_contract_id: None,
    };
    let semantic_contract_id = calculate_semantic_contract_id_v1(&semantic_manifest).unwrap();
    mtgml_replay::SemanticContractMaterialV5 {
        semantic_contract_id,
        manifest: semantic_manifest,
        rules_manifest,
    }
}

fn comprehensive_manifest(snapshot_id: &str) -> mtgml_replay::SemanticContractMaterialV5 {
    let rules_manifest = RulesContractManifestV1 {
        rules_authority: RulesAuthorityV1::ComprehensiveRules {
            snapshot_id: snapshot_id.to_string(),
        },
        capability_closure: Some(vec![CapabilityRequirementV1 {
            key: "rules/synthetic-m1".to_string(),
            version: "1.0.0".to_string(),
        }]),
    };
    semantic_contract_material(rules_manifest)
}

fn synthetic_execution_identity() -> ExecutionIdentityV1 {
    let material = semantic_contract_material(synthetic_rules_manifest());
    ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::SyntheticRulesCompat,
        semantic_contract_id: material.semantic_contract_id,
    }
}

fn full_state_digest(byte: u8) -> FullStateDigestV4 {
    FullStateDigestV4::from_digest_bytes([byte; 32])
}

fn checkpoint_codec_v5() -> CheckpointCodecIdentity {
    CheckpointCodecIdentity {
        codec_id: CHECKPOINT_CODEC_ID_V5.to_string(),
        semantic_version: CHECKPOINT_CODEC_VERSION_V5.to_string(),
    }
}

fn v5_identity(
    revision: u64,
    digest_byte: u8,
    counters: EnvironmentLimitCounters,
    execution_identity: ExecutionIdentityV1,
) -> InitialEnvironmentIdentityV5 {
    let full_state_digest = full_state_digest(digest_byte);
    let checkpoint_digest = mtgml_persistence::checkpoint_digest::calculate_checkpoint_digest_v5(
        &full_state_digest.as_digest_reference(),
        &EpisodeStatus::Running,
        &counters,
        &checkpoint_codec_v5(),
        &execution_identity,
    )
    .unwrap();
    InitialEnvironmentIdentityV5 {
        state_revision: StateRevision(revision),
        full_state_digest,
        episode_status: EpisodeStatus::Running,
        environment_limit_counters: counters,
        checkpoint_codec_identity: checkpoint_codec_v5(),
        checkpoint_digest,
        execution_identity,
    }
}

fn valid_manifest_v5() -> ReplayManifestV5 {
    let execution_identity = synthetic_execution_identity();
    let semantic_contract = semantic_contract_material(synthetic_rules_manifest());
    let initial_identity = v5_identity(
        0,
        0,
        EnvironmentLimitCounters::default(),
        execution_identity.clone(),
    );
    ReplayManifestV5 {
        schema_version: REPLAY_MANIFEST_SCHEMA_V5.to_string(),
        engine_build: "synthetic-build".to_string(),
        kernel: KernelIdentityV1 {
            implementation_id: "synthetic-m3".to_string(),
            semantic_version: "0.2.2".to_string(),
            build_profile: "test".to_string(),
        },
        rules_snapshot: "synthetic-rules".to_string(),
        format_policy_snapshot: "synthetic-format".to_string(),
        oracle_snapshot: "synthetic-oracle".to_string(),
        card_bundle: "synthetic-bundle".to_string(),
        schemas: ReplaySchemaVersionsV5 {
            observation: "observation-envelope.v1".to_string(),
            observation_payload_codec: "synthetic-m3-observation.v1".to_string(),
            information_state: "information-state-envelope.v2".to_string(),
            decision: "player-decision-request.v2".to_string(),
            decision_response: "decision-response.v2".to_string(),
            observed_event: "observed-event-envelope.v2".to_string(),
            player_step: "player-step.v2".to_string(),
            replay_step: REPLAY_STEP_SCHEMA_V5.to_string(),
        },
        randomness: RandomnessIdentityV2 {
            contract_id: "mtgml.rng.v1".to_string(),
            root_seed_hex: "00".repeat(32),
        },
        decks: vec![DeckIdentityV1 {
            player: PlayerId(1),
            deck_id: "synthetic-deck".to_string(),
            digest: ContentDigest::parse("11".repeat(32)).unwrap(),
        }],
        initial_identity,
        execution_identity,
        semantic_contract,
    }
}

fn response_v2(revision: u64) -> DecisionResponseV2 {
    DecisionResponseV2 {
        schema_version: "decision-response.v2".to_string(),
        player_decision_id: mtgml_model::PlayerDecisionIdV1(1),
        state_revision: StateRevision(revision),
        answer: mtgml_decision::DecisionAnswerV2::ChooseNumber { value: 0 },
    }
}

#[test]
fn replay_v5_roundtrip_empty_record_validates() {
    let manifest = valid_manifest_v5();
    manifest.validate().unwrap();

    let recorder = ReplayRecorderV5::new(manifest.clone()).unwrap();
    let replay = recorder.export().unwrap();
    assert_eq!(replay.schema_version, REPLAY_FILE_SCHEMA_V5);
    assert_eq!(replay.manifest, manifest);
    assert!(replay.steps.is_empty());
    replay.validate().unwrap();
}

#[test]
fn replay_v5_rejects_manifest_semantic_contract_id_mismatch() {
    let mut manifest = valid_manifest_v5();
    manifest.semantic_contract.semantic_contract_id =
        SemanticContractIdV1::from_digest_bytes([0xab; 32]);
    assert_eq!(
        manifest.validate(),
        Err(ReplayValidationError::SemanticContractMismatch)
    );
}

#[test]
fn replay_v5_rejects_manifest_semantic_manifest_hash_mismatch() {
    let mut manifest = valid_manifest_v5();
    manifest.semantic_contract.manifest.rules_contract_id =
        RulesContractIdV1::from_digest_bytes([9; 32]);
    assert_eq!(
        manifest.validate(),
        Err(ReplayValidationError::SemanticContractMismatch)
    );
}

#[test]
fn replay_v5_rejects_manifest_rules_manifest_hash_mismatch() {
    let mut manifest = valid_manifest_v5();
    // Swap in a ComprehensiveRules manifest (different content) but keep the
    // semantic manifest's rules_contract_id pointing at the original
    // SyntheticLegacy → hash mismatch ⇒ detached rejection.
    manifest.semantic_contract.rules_manifest = RulesContractManifestV1 {
        rules_authority: RulesAuthorityV1::ComprehensiveRules {
            snapshot_id: "cr-snapshot-2026-09".to_string(),
        },
        capability_closure: Some(vec![CapabilityRequirementV1 {
            key: "rules/synthetic-m1".to_string(),
            version: "1.0.0".to_string(),
        }]),
    };
    // The rules_manifest has changed but the rules_contract_id in the semantic
    // manifest still points at the old one.
    assert_eq!(
        manifest.validate(),
        Err(ReplayValidationError::SemanticContractMismatch)
    );
}

#[test]
fn replay_v5_rejects_non_null_format_contract_id() {
    let mut manifest = valid_manifest_v5();
    let format_id = mtgml_model::FormatContractIdV1::parse("ab".repeat(32)).unwrap();
    manifest.semantic_contract.manifest.format_contract_id = Some(format_id);
    // Recompute the semantic_contract_id from the modified manifest so the
    // digest-content binding passes — the test must isolate the NULL-only
    // guard, not fail at the identity recompute.
    let new_id = mtgml_persistence::semantic_contract_digest::calculate_semantic_contract_id_v1(
        &manifest.semantic_contract.manifest,
    )
    .unwrap();
    manifest.semantic_contract.semantic_contract_id = new_id.clone();
    manifest.execution_identity.semantic_contract_id = new_id.clone();
    manifest
        .initial_identity
        .execution_identity
        .semantic_contract_id = new_id;
    // Recompute the initial_identity checkpoint digest for the updated identity.
    manifest.initial_identity.checkpoint_digest =
        mtgml_persistence::checkpoint_digest::calculate_checkpoint_digest_v5(
            &manifest
                .initial_identity
                .full_state_digest
                .as_digest_reference(),
            &manifest.initial_identity.episode_status,
            &manifest.initial_identity.environment_limit_counters,
            &manifest.initial_identity.checkpoint_codec_identity,
            &manifest.initial_identity.execution_identity,
        )
        .unwrap();
    // All digest/content bindings valid BUT format_contract_id is non-null.
    assert_eq!(
        manifest.validate(),
        Err(ReplayValidationError::SemanticContractMismatch)
    );
}

#[test]
fn replay_v5_rejects_non_null_content_contract_id() {
    let mut manifest = valid_manifest_v5();
    let content_id = mtgml_model::ContentContractIdV1::parse("cd".repeat(32)).unwrap();
    manifest.semantic_contract.manifest.content_contract_id = Some(content_id);
    let new_id = mtgml_persistence::semantic_contract_digest::calculate_semantic_contract_id_v1(
        &manifest.semantic_contract.manifest,
    )
    .unwrap();
    manifest.semantic_contract.semantic_contract_id = new_id.clone();
    manifest.execution_identity.semantic_contract_id = new_id.clone();
    manifest
        .initial_identity
        .execution_identity
        .semantic_contract_id = new_id;
    manifest.initial_identity.checkpoint_digest =
        mtgml_persistence::checkpoint_digest::calculate_checkpoint_digest_v5(
            &manifest
                .initial_identity
                .full_state_digest
                .as_digest_reference(),
            &manifest.initial_identity.episode_status,
            &manifest.initial_identity.environment_limit_counters,
            &manifest.initial_identity.checkpoint_codec_identity,
            &manifest.initial_identity.execution_identity,
        )
        .unwrap();
    // All digest/content bindings valid BUT content_contract_id is non-null.
    assert_eq!(
        manifest.validate(),
        Err(ReplayValidationError::SemanticContractMismatch)
    );
}

#[test]
fn replay_v5_rejects_execution_identity_mismatch_manifest_vs_initial() {
    let mut manifest = valid_manifest_v5();
    let final_identity = manifest.initial_identity.clone();
    manifest.initial_identity.execution_identity = ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::MagicRules,
        semantic_contract_id: manifest.semantic_contract.semantic_contract_id.clone(),
    };
    // Recompute the initial_identity checkpoint digest with the new identity.
    let new_digest = mtgml_persistence::checkpoint_digest::calculate_checkpoint_digest_v5(
        &manifest
            .initial_identity
            .full_state_digest
            .as_digest_reference(),
        &manifest.initial_identity.episode_status,
        &manifest.initial_identity.environment_limit_counters,
        &manifest.initial_identity.checkpoint_codec_identity,
        &manifest.initial_identity.execution_identity,
    )
    .unwrap();
    manifest.initial_identity.checkpoint_digest = new_digest;

    let replay = AuthoritativeReplayV5 {
        schema_version: REPLAY_FILE_SCHEMA_V5.to_string(),
        manifest,
        steps: vec![],
        final_identity,
    };
    assert_eq!(
        replay.validate(),
        Err(ReplayValidationError::SemanticContractMismatch)
    );
}

#[test]
fn replay_v5_rejects_execution_identity_mismatch_initial_vs_final() {
    let manifest = valid_manifest_v5();
    let initial = manifest.initial_identity.clone();
    let mut final_identity = initial.clone();
    final_identity.execution_identity = ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::MagicRules,
        semantic_contract_id: manifest.semantic_contract.semantic_contract_id.clone(),
    };
    let new_digest = mtgml_persistence::checkpoint_digest::calculate_checkpoint_digest_v5(
        &final_identity.full_state_digest.as_digest_reference(),
        &final_identity.episode_status,
        &final_identity.environment_limit_counters,
        &final_identity.checkpoint_codec_identity,
        &final_identity.execution_identity,
    )
    .unwrap();
    final_identity.checkpoint_digest = new_digest;

    let replay = AuthoritativeReplayV5 {
        schema_version: REPLAY_FILE_SCHEMA_V5.to_string(),
        manifest,
        steps: vec![],
        final_identity,
    };
    assert_eq!(
        replay.validate(),
        Err(ReplayValidationError::SemanticContractMismatch)
    );
}

#[test]
fn replay_v5_rejects_semantic_contract_id_not_matching_identity() {
    let mut manifest = valid_manifest_v5();
    // Change the semantic_contract_id in the execution identity without
    // changing the material. This breaks the two-way consistency: the
    // identity's semantic_contract_id != material.semantic_contract_id.
    manifest.execution_identity.semantic_contract_id =
        SemanticContractIdV1::from_digest_bytes([0xcd; 32]);
    assert_eq!(
        manifest.validate(),
        Err(ReplayValidationError::SemanticContractMismatch)
    );
}

#[test]
fn replay_v5_synthetic_legacy_treats_rules_snapshot_as_informational() {
    let mut manifest = valid_manifest_v5();
    // SyntheticLegacy: rules_snapshot is informational — a mismatch must NOT
    // be rejected as a semantic contract mismatch.
    manifest.rules_snapshot = "anything-whatsoever".to_string();
    manifest.validate().unwrap();
}

#[test]
fn replay_v5_comprehensive_rules_rejects_snapshot_mismatch() {
    let material = comprehensive_manifest("cr-snapshot-2026-09");
    let execution_identity = ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::MagicRules,
        semantic_contract_id: material.semantic_contract_id.clone(),
    };
    let mut manifest = valid_manifest_v5();
    manifest.semantic_contract = material;
    manifest.execution_identity = execution_identity.clone();
    manifest.initial_identity.execution_identity = execution_identity;
    // The rules_snapshot is "synthetic-rules" (from valid_manifest_v5) but
    // the contract's ComprehensiveRules authority has snapshot_id
    // "cr-snapshot-2026-09" → mismatch ⇒ detached rejection.
    let digest = mtgml_persistence::checkpoint_digest::calculate_checkpoint_digest_v5(
        &manifest
            .initial_identity
            .full_state_digest
            .as_digest_reference(),
        &manifest.initial_identity.episode_status,
        &manifest.initial_identity.environment_limit_counters,
        &manifest.initial_identity.checkpoint_codec_identity,
        &manifest.initial_identity.execution_identity,
    )
    .unwrap();
    manifest.initial_identity.checkpoint_digest = digest;

    assert_eq!(
        manifest.validate(),
        Err(ReplayValidationError::RulesSnapshotMismatch)
    );
}

#[test]
fn replay_v5_comprehensive_rules_accepts_matching_snapshot() {
    let material = comprehensive_manifest("synthetic-rules");
    let execution_identity = ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::MagicRules,
        semantic_contract_id: material.semantic_contract_id.clone(),
    };
    let mut manifest = valid_manifest_v5();
    manifest.semantic_contract = material;
    manifest.execution_identity = execution_identity.clone();
    manifest.initial_identity.execution_identity = execution_identity;
    let digest = mtgml_persistence::checkpoint_digest::calculate_checkpoint_digest_v5(
        &manifest
            .initial_identity
            .full_state_digest
            .as_digest_reference(),
        &manifest.initial_identity.episode_status,
        &manifest.initial_identity.environment_limit_counters,
        &manifest.initial_identity.checkpoint_codec_identity,
        &manifest.initial_identity.execution_identity,
    )
    .unwrap();
    manifest.initial_identity.checkpoint_digest = digest;
    manifest.validate().unwrap();
}

#[test]
fn replay_v5_step_uses_checkpoint_digest_v5_types() {
    let manifest = valid_manifest_v5();
    let initial = manifest.initial_identity.clone();
    let step = ReplayStepV5 {
        step_index: 0,
        actor: PlayerId(1),
        checkpoint_digest_before: initial.checkpoint_digest.clone(),
        state_revision_before: initial.state_revision,
        response: response_v2(0),
        accepted: false,
        state_revision_after: initial.state_revision.clone(),
        full_state_digest_after: initial.full_state_digest.clone(),
        episode_status_after: initial.episode_status.clone(),
        environment_limit_counters_after: initial.environment_limit_counters.clone(),
        checkpoint_digest_after: initial.checkpoint_digest.clone(),
    };
    let replay = AuthoritativeReplayV5 {
        schema_version: REPLAY_FILE_SCHEMA_V5.to_string(),
        manifest,
        steps: vec![step],
        final_identity: initial,
    };
    replay.validate().unwrap();
}

#[test]
fn replay_v5_recorder_appends_accepted_step_and_exports() {
    let manifest = valid_manifest_v5();
    let initial = manifest.initial_identity.clone();
    let next_revision = StateRevision(1);
    let next_counters = EnvironmentLimitCounters {
        decisions_submitted: 1,
        accepted_transitions: 1,
        ..EnvironmentLimitCounters::default()
    };
    let next_digest = mtgml_persistence::checkpoint_digest::calculate_checkpoint_digest_v5(
        &full_state_digest(1).as_digest_reference(),
        &EpisodeStatus::Running,
        &next_counters,
        &checkpoint_codec_v5(),
        &initial.execution_identity,
    )
    .unwrap();
    let after = InitialEnvironmentIdentityV5 {
        state_revision: next_revision,
        full_state_digest: full_state_digest(1),
        episode_status: EpisodeStatus::Running,
        environment_limit_counters: next_counters,
        checkpoint_codec_identity: checkpoint_codec_v5(),
        checkpoint_digest: next_digest,
        execution_identity: initial.execution_identity.clone(),
    };
    let step = ReplayStepV5 {
        step_index: 0,
        actor: PlayerId(1),
        checkpoint_digest_before: initial.checkpoint_digest.clone(),
        state_revision_before: initial.state_revision,
        response: response_v2(0),
        accepted: true,
        state_revision_after: after.state_revision,
        full_state_digest_after: after.full_state_digest.clone(),
        episode_status_after: after.episode_status.clone(),
        environment_limit_counters_after: after.environment_limit_counters.clone(),
        checkpoint_digest_after: after.checkpoint_digest.clone(),
    };

    let mut recorder = ReplayRecorderV5::new(manifest).unwrap();
    recorder.append(step).unwrap();
    assert_eq!(recorder.step_count(), 1);
    let replay = recorder.export().unwrap();
    assert_eq!(replay.steps.len(), 1);
    replay.validate().unwrap();
    assert_eq!(replay.final_identity, after);
    // Three-way identity preserved through the accepted step.
    assert_eq!(
        replay.final_identity.execution_identity,
        replay.manifest.execution_identity
    );
}

#[test]
fn replay_v5_v4_types_remain_untouched_and_pass() {
    // V4 must continue to compile and validate independently.
    let full_state_digest = FullStateDigestV4::from_digest_bytes([0; 32]);
    let codec = CheckpointCodecIdentity {
        codec_id: "in-memory-reference".to_string(),
        semantic_version: "4".to_string(),
    };
    let checkpoint_digest = mtgml_persistence::checkpoint_digest::calculate_checkpoint_digest_v4(
        &full_state_digest.as_digest_reference(),
        &EpisodeStatus::Running,
        &EnvironmentLimitCounters::default(),
        &codec,
    )
    .unwrap();
    let v4_identity = mtgml_replay::InitialEnvironmentIdentityV4 {
        state_revision: StateRevision(0),
        full_state_digest,
        episode_status: EpisodeStatus::Running,
        environment_limit_counters: EnvironmentLimitCounters::default(),
        checkpoint_codec_identity: codec,
        checkpoint_digest,
    };
    v4_identity.validate().unwrap();
}
