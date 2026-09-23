//! S3.P0 Task 1 contract tests for the typed Replay V6 identity family.

use mtgml_decision::{DecisionAnswerV2, DecisionResponseV2};
use mtgml_model::{
    CapabilityRequirementV1, CheckpointCodecIdentity, ContentDigest, EnvironmentLimitCounters,
    EpisodeStatus, ExecutionIdentityV1, ExecutionProgramV1, FullStateDigestV5, PlayerId,
    RulesAuthorityV1, RulesContractManifestV1, SemanticContractManifestV1, StateRevision,
};
use mtgml_persistence::semantic_contract_digest::{
    calculate_rules_contract_id_v1, calculate_semantic_contract_id_v1,
};
use mtgml_replay::{
    AuthoritativeReplayV6, DeckIdentityV1, InitialEnvironmentIdentityV6, KernelIdentityV1,
    RandomnessIdentityV2, ReplayManifestV6, ReplayRecorderV6, ReplaySchemaVersionsV6, ReplayStepV6,
    ReplayValidationError, SemanticContractMaterialV5, REPLAY_FILE_SCHEMA_V6,
    REPLAY_MANIFEST_SCHEMA_V6, REPLAY_STEP_SCHEMA_V6,
};
use serde_json::{Map, Value};

fn rules_manifest() -> RulesContractManifestV1 {
    RulesContractManifestV1 {
        rules_authority: RulesAuthorityV1::SyntheticLegacy,
        capability_closure: None,
    }
}

fn semantic_material(rules: RulesContractManifestV1) -> SemanticContractMaterialV5 {
    let rules_contract_id = calculate_rules_contract_id_v1(&rules).unwrap();
    let manifest = SemanticContractManifestV1 {
        rules_contract_id,
        format_contract_id: None,
        content_contract_id: None,
    };
    let semantic_contract_id = calculate_semantic_contract_id_v1(&manifest).unwrap();
    SemanticContractMaterialV5 {
        semantic_contract_id,
        manifest,
        rules_manifest: rules,
    }
}

fn codec() -> CheckpointCodecIdentity {
    CheckpointCodecIdentity {
        codec_id: "in-memory-reference".to_owned(),
        semantic_version: "6".to_owned(),
    }
}

fn identity(
    revision: u64,
    byte: u8,
    counters: EnvironmentLimitCounters,
    execution: ExecutionIdentityV1,
) -> InitialEnvironmentIdentityV6 {
    let full_state_digest = FullStateDigestV5::from_digest_bytes([byte; 32]);
    let checkpoint_digest = mtgml_persistence::checkpoint_digest::calculate_checkpoint_digest_v6(
        &full_state_digest.as_digest_reference(),
        &EpisodeStatus::Running,
        &counters,
        &codec(),
        &execution,
    )
    .unwrap();
    InitialEnvironmentIdentityV6 {
        state_revision: StateRevision(revision),
        full_state_digest,
        episode_status: EpisodeStatus::Running,
        environment_limit_counters: counters,
        checkpoint_codec_identity: codec(),
        checkpoint_digest,
        execution_identity: execution,
    }
}

fn manifest() -> ReplayManifestV6 {
    let semantic_contract = semantic_material(rules_manifest());
    let execution_identity = ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::SyntheticRulesCompat,
        semantic_contract_id: semantic_contract.semantic_contract_id.clone(),
    };
    ReplayManifestV6 {
        schema_version: REPLAY_MANIFEST_SCHEMA_V6.to_owned(),
        engine_build: "synthetic-build".to_owned(),
        kernel: KernelIdentityV1 {
            implementation_id: "synthetic-m3".to_owned(),
            semantic_version: "0.2.2".to_owned(),
            build_profile: "test".to_owned(),
        },
        rules_snapshot: "synthetic-rules".to_owned(),
        format_policy_snapshot: "synthetic-format".to_owned(),
        oracle_snapshot: "synthetic-oracle".to_owned(),
        card_bundle: "synthetic-bundle".to_owned(),
        schemas: ReplaySchemaVersionsV6 {
            observation: "observation-envelope.v1".to_owned(),
            observation_payload_codec: "synthetic-m3-observation.v1".to_owned(),
            information_state: "information-state-envelope.v2".to_owned(),
            decision: "player-decision-request.v2".to_owned(),
            decision_response: "decision-response.v2".to_owned(),
            observed_event: "observed-event-envelope.v2".to_owned(),
            player_step: "player-step.v2".to_owned(),
            replay_step: REPLAY_STEP_SCHEMA_V6.to_owned(),
        },
        randomness: RandomnessIdentityV2 {
            contract_id: "mtgml.rng.v1".to_owned(),
            root_seed_hex: "00".repeat(32),
        },
        decks: vec![DeckIdentityV1 {
            player: PlayerId(1),
            deck_id: "synthetic-deck".to_owned(),
            digest: ContentDigest::parse("11".repeat(32)).unwrap(),
        }],
        initial_identity: identity(
            0,
            1,
            EnvironmentLimitCounters::default(),
            execution_identity.clone(),
        ),
        execution_identity,
        semantic_contract,
    }
}

fn response(revision: u64) -> DecisionResponseV2 {
    DecisionResponseV2 {
        schema_version: "decision-response.v2".to_owned(),
        player_decision_id: mtgml_model::PlayerDecisionIdV1(1),
        state_revision: StateRevision(revision),
        answer: DecisionAnswerV2::ChooseNumber { value: 0 },
    }
}

#[test]
fn replay_v6_binds_new_state_checkpoint_and_real_response_identities() {
    assert_eq!(REPLAY_MANIFEST_SCHEMA_V6, "replay-manifest.v6");
    assert_eq!(REPLAY_STEP_SCHEMA_V6, "replay-step.v6");
    assert_eq!(REPLAY_FILE_SCHEMA_V6, "authoritative-replay.v6");
    let manifest = manifest();
    assert_eq!(
        manifest
            .initial_identity
            .full_state_digest
            .as_digest_reference()
            .input_schema_id,
        "full-state-digest-input.v5"
    );
    let recorder = ReplayRecorderV6::new(manifest.clone()).unwrap();
    let replay = recorder.export().unwrap();
    assert_eq!(replay.schema_version, REPLAY_FILE_SCHEMA_V6);
    assert_eq!(replay.manifest, manifest);
    replay.validate().unwrap();
}

#[test]
fn replay_v6_appends_one_response_and_has_no_forced_work_input() {
    let manifest = manifest();
    let initial = manifest.initial_identity.clone();
    let after = identity(
        1,
        2,
        EnvironmentLimitCounters {
            decisions_submitted: 1,
            accepted_transitions: 1,
            ..EnvironmentLimitCounters::default()
        },
        initial.execution_identity.clone(),
    );
    let step = ReplayStepV6 {
        step_index: 0,
        actor: PlayerId(1),
        checkpoint_digest_before: initial.checkpoint_digest.clone(),
        state_revision_before: initial.state_revision,
        response: response(0),
        accepted: true,
        state_revision_after: after.state_revision,
        full_state_digest_after: after.full_state_digest.clone(),
        episode_status_after: after.episode_status.clone(),
        environment_limit_counters_after: after.environment_limit_counters.clone(),
        checkpoint_digest_after: after.checkpoint_digest.clone(),
    };
    let mut recorder = ReplayRecorderV6::new(manifest).unwrap();
    recorder.append(step).unwrap();
    let replay = recorder.export().unwrap();
    assert_eq!(replay.steps.len(), 1);
    assert_eq!(replay.final_identity, after);
    replay.validate().unwrap();
    let wire = serde_json::to_value(&replay).unwrap();
    let step = wire["steps"][0].as_object().unwrap();
    let expected = [
        "accepted",
        "actor",
        "checkpoint_digest_after",
        "checkpoint_digest_before",
        "environment_limit_counters_after",
        "episode_status_after",
        "full_state_digest_after",
        "response",
        "state_revision_after",
        "state_revision_before",
        "step_index",
    ];
    assert_eq!(
        step.keys().map(String::as_str).collect::<Vec<_>>(),
        expected
    );
}

#[test]
fn replay_v6_rejects_v5_schema_and_unknown_fields() {
    let replay = ReplayRecorderV6::new(manifest()).unwrap().export().unwrap();
    let mut v5 = serde_json::to_value(&replay).unwrap();
    v5["schema_version"] = Value::String("authoritative-replay.v5".to_owned());
    let decoded: AuthoritativeReplayV6 = serde_json::from_value(v5).unwrap();
    assert_eq!(
        decoded.validate(),
        Err(ReplayValidationError::SchemaVersion)
    );
    let mut unknown = serde_json::to_value(&replay).unwrap();
    unknown["unknown"] = Value::Bool(true);
    unknown["steps"] = Value::Array(vec![Value::Object(Map::from_iter([(
        "forced_progress_input".to_owned(),
        Value::Bool(true),
    )]))]);
    assert!(serde_json::from_value::<AuthoritativeReplayV6>(unknown).is_err());
}

#[test]
fn magic_observation_codec_is_bound_to_sba_semantics() {
    let mut manifest = manifest();
    manifest.schemas.observation_payload_codec = "magic-m3-observation.v1".to_owned();
    assert!(manifest.validate().is_err());

    let rules = RulesContractManifestV1 {
        rules_authority: RulesAuthorityV1::ComprehensiveRules {
            snapshot_id: "cr:test".to_owned(),
        },
        capability_closure: Some(vec![CapabilityRequirementV1 {
            key: "rules/state-based-actions-combat".to_owned(),
            version: "0.1.0".to_owned(),
        }]),
    };
    let semantic = semantic_material(rules);
    manifest.rules_snapshot = "cr:test".to_owned();
    manifest.execution_identity = ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::MagicRules,
        semantic_contract_id: semantic.semantic_contract_id.clone(),
    };
    manifest.initial_identity.execution_identity = manifest.execution_identity.clone();
    manifest.initial_identity.checkpoint_digest =
        mtgml_persistence::checkpoint_digest::calculate_checkpoint_digest_v6(
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
    manifest.semantic_contract = semantic;
    assert!(manifest.validate().is_ok());
}
