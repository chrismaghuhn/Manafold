use mtgml_model::{
    CapabilityRequirementV1, CheckpointCodecIdentity, EnvironmentLimitCounters, EpisodeStatus,
    ExecutionIdentityV1, ExecutionProgramV1, FullStateDigestV5, PlayerId, RulesAuthorityV1,
    RulesContractManifestV1, SemanticContractManifestV1, StateRevision,
};
use mtgml_persistence::semantic_contract_digest::{
    calculate_rules_contract_id_v1, calculate_semantic_contract_id_v1,
};
use mtgml_replay::{
    AuthoritativeReplayV6, DeckIdentityV1, InitialEnvironmentIdentityV6, KernelIdentityV1,
    RandomnessIdentityV2, ReplayManifestV6, ReplaySchemaVersionsV6, SemanticContractMaterialV5,
    REPLAY_FILE_SCHEMA_V6, REPLAY_MANIFEST_SCHEMA_V6, REPLAY_STEP_SCHEMA_V6,
};
use serde_json::{Map, Value};
const CHECKPOINT_CODEC_ID_V6: &str = "in-memory-reference";
const CHECKPOINT_CODEC_VERSION_V6: &str = "6";
const ZERO_DIGEST: &str = "0000000000000000000000000000000000000000000000000000000000000000";

fn canonicalize(value: Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.into_iter().map(canonicalize).collect()),
        Value::Object(object) => {
            let mut pairs: Vec<_> = object.into_iter().collect();
            pairs.sort_by(|left, right| left.0.cmp(&right.0));
            let mut sorted = Map::new();
            for (key, value) in pairs {
                sorted.insert(key, canonicalize(value));
            }
            Value::Object(sorted)
        }
        scalar => scalar,
    }
}

fn to_canonical_json(value: &Value) -> String {
    serde_json::to_string(&canonicalize(value.clone())).unwrap()
}

fn v6_identity(
    revision: u64,
    digest_byte: u8,
    counters: EnvironmentLimitCounters,
    execution_identity: ExecutionIdentityV1,
) -> InitialEnvironmentIdentityV6 {
    let full_state_digest = FullStateDigestV5::from_digest_bytes([digest_byte; 32]);
    let checkpoint_digest = mtgml_persistence::checkpoint_digest::calculate_checkpoint_digest_v6(
        &full_state_digest.as_digest_reference(),
        &EpisodeStatus::Running,
        &counters,
        &CheckpointCodecIdentity {
            codec_id: CHECKPOINT_CODEC_ID_V6.to_string(),
            semantic_version: CHECKPOINT_CODEC_VERSION_V6.to_string(),
        },
        &execution_identity,
    )
    .unwrap();
    InitialEnvironmentIdentityV6 {
        state_revision: StateRevision(revision),
        full_state_digest,
        episode_status: EpisodeStatus::Running,
        environment_limit_counters: counters,
        checkpoint_codec_identity: CheckpointCodecIdentity {
            codec_id: CHECKPOINT_CODEC_ID_V6.to_string(),
            semantic_version: CHECKPOINT_CODEC_VERSION_V6.to_string(),
        },
        checkpoint_digest,
        execution_identity,
    }
}

fn synthetic_semantic_material() -> SemanticContractMaterialV5 {
    semantic_material(RulesContractManifestV1 {
        rules_authority: RulesAuthorityV1::SyntheticLegacy,
        capability_closure: None,
    })
}

fn semantic_material(rules_manifest: RulesContractManifestV1) -> SemanticContractMaterialV5 {
    let rules_contract_id = calculate_rules_contract_id_v1(&rules_manifest).unwrap();
    let semantic_manifest = SemanticContractManifestV1 {
        rules_contract_id,
        format_contract_id: None,
        content_contract_id: None,
    };
    let semantic_contract_id = calculate_semantic_contract_id_v1(&semantic_manifest).unwrap();
    SemanticContractMaterialV5 {
        semantic_contract_id,
        manifest: semantic_manifest,
        rules_manifest,
    }
}

fn cr_semantic_material(snapshot_id: &str) -> SemanticContractMaterialV5 {
    semantic_material(RulesContractManifestV1 {
        rules_authority: RulesAuthorityV1::ComprehensiveRules {
            snapshot_id: snapshot_id.to_string(),
        },
        capability_closure: Some(vec![CapabilityRequirementV1 {
            key: "rules/synthetic-m1".to_string(),
            version: "1.0.0".to_string(),
        }]),
    })
}

fn base_manifest(
    execution_identity: ExecutionIdentityV1,
    semantic_contract: SemanticContractMaterialV5,
) -> ReplayManifestV6 {
    let initial_identity = v6_identity(
        0,
        0,
        EnvironmentLimitCounters::default(),
        execution_identity.clone(),
    );
    ReplayManifestV6 {
        schema_version: REPLAY_MANIFEST_SCHEMA_V6.to_string(),
        engine_build: "git:0000000".to_string(),
        kernel: KernelIdentityV1 {
            implementation_id: "mtgml-reference".to_string(),
            semantic_version: "0.2.2".to_string(),
            build_profile: "test".to_string(),
        },
        rules_snapshot: "cr:synthetic-v1".to_string(),
        format_policy_snapshot: "commander:synthetic-v1".to_string(),
        oracle_snapshot: "oracle:synthetic-v1".to_string(),
        card_bundle: "bundle:synthetic-v1".to_string(),
        schemas: ReplaySchemaVersionsV6 {
            observation: "observation-envelope.v1".to_string(),
            observation_payload_codec: "synthetic-m3-observation.v1".to_string(),
            information_state: "information-state-envelope.v2".to_string(),
            decision: "player-decision-request.v2".to_string(),
            decision_response: "decision-response.v2".to_string(),
            observed_event: "observed-event-envelope.v2".to_string(),
            player_step: "player-step.v2".to_string(),
            replay_step: REPLAY_STEP_SCHEMA_V6.to_string(),
        },
        randomness: RandomnessIdentityV2 {
            contract_id: "mtgml.rng.v1".to_string(),
            root_seed_hex: "00".repeat(32),
        },
        decks: vec![DeckIdentityV1 {
            player: PlayerId(1),
            deck_id: "deck:synthetic-p1".to_string(),
            digest: mtgml_model::ContentDigest::parse(ZERO_DIGEST).unwrap(),
        }],
        initial_identity,
        execution_identity,
        semantic_contract,
    }
}

fn manifest_for_rules(
    rules_manifest: RulesContractManifestV1,
    observation_codec: &str,
) -> ReplayManifestV6 {
    let program_kind = match &rules_manifest.rules_authority {
        RulesAuthorityV1::SyntheticLegacy => ExecutionProgramV1::SyntheticRulesCompat,
        RulesAuthorityV1::ComprehensiveRules { .. } => ExecutionProgramV1::MagicRules,
    };
    let semantic = semantic_material(rules_manifest);
    let execution = ExecutionIdentityV1 {
        program_kind,
        semantic_contract_id: semantic.semantic_contract_id.clone(),
    };
    let mut manifest = base_manifest(execution.clone(), semantic.clone());
    if let RulesAuthorityV1::ComprehensiveRules { snapshot_id } =
        &semantic.rules_manifest.rules_authority
    {
        manifest.rules_snapshot = snapshot_id.clone();
    }
    manifest.schemas.observation_payload_codec = observation_codec.to_owned();
    manifest.execution_identity = execution.clone();
    manifest.initial_identity = v6_identity(
        manifest.initial_identity.state_revision.0,
        0,
        manifest.initial_identity.environment_limit_counters,
        execution,
    );
    manifest.semantic_contract = semantic;
    manifest
}

#[test]
fn print_all_fixtures() {
    let material = synthetic_semantic_material();
    let execution_identity = ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::SyntheticRulesCompat,
        semantic_contract_id: material.semantic_contract_id.clone(),
    };

    // Golden manifest
    let manifest = base_manifest(execution_identity.clone(), material.clone());
    manifest.validate().unwrap();
    println!(
        "GOLDEN_MANIFEST: {}",
        to_canonical_json(&serde_json::to_value(&manifest).unwrap())
    );

    // Golden authoritative replay (empty)
    let material2 = synthetic_semantic_material();
    let exec_id2 = ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::SyntheticRulesCompat,
        semantic_contract_id: material2.semantic_contract_id.clone(),
    };
    let manifest2 = base_manifest(exec_id2, material2);
    manifest2.validate().unwrap();
    let initial = manifest2.initial_identity.clone();
    let replay = mtgml_replay::AuthoritativeReplayV6 {
        schema_version: "authoritative-replay.v6".to_string(),
        manifest: manifest2,
        steps: vec![],
        final_identity: initial,
    };
    replay.validate().unwrap();
    println!(
        "GOLDEN_REPLAY: {}",
        to_canonical_json(&serde_json::to_value(&replay).unwrap())
    );

    // Negative: unknown program_kind
    {
        let m = base_manifest(execution_identity.clone(), material.clone());
        let mut value = serde_json::to_value(&m).unwrap();
        value["execution_identity"]["program_kind"] =
            serde_json::Value::String("unknown_program".to_string());
        println!("NEG_UNKNOWN_PROGRAM: {}", to_canonical_json(&value));
    }

    // Negative: wrong digest length (63 hex chars)
    {
        let m = base_manifest(execution_identity.clone(), material.clone());
        let mut value = serde_json::to_value(&m).unwrap();
        value["semantic_contract"]["semantic_contract_id"] = serde_json::Value::String(
            "66ccac959475370e641e853473cbdd7f88489399587794b43f66cfa0342b1be".to_string(),
        );
        println!("NEG_WRONG_DIGEST: {}", to_canonical_json(&value));
    }

    // Negative: semantic-contract mismatch
    {
        let m = base_manifest(execution_identity.clone(), material.clone());
        let mut value = serde_json::to_value(&m).unwrap();
        value["semantic_contract"]["semantic_contract_id"] =
            serde_json::Value::String(ZERO_DIGEST.to_string());
        println!("NEG_SEMANTIC_MISMATCH: {}", to_canonical_json(&value));
    }

    // Negative: rules-contract mismatch (change rules_manifest to CR)
    {
        let m = base_manifest(execution_identity.clone(), material.clone());
        let mut value = serde_json::to_value(&m).unwrap();
        value["semantic_contract"]["rules_manifest"]["rules_authority"] =
            serde_json::json!({"variant": "comprehensive_rules", "snapshot_id": "cr:synthetic-v1"});
        value["semantic_contract"]["rules_manifest"]["capability_closure"] =
            serde_json::json!([{"key": "rules/synthetic-m1", "version": "1.0.0"}]);
        println!("NEG_RULES_MISMATCH: {}", to_canonical_json(&value));
    }

    // Negative: CR rules_snapshot mismatch
    {
        let cr_material = cr_semantic_material("comprehensive-rules-2026-09-01");
        let cr_identity = ExecutionIdentityV1 {
            program_kind: ExecutionProgramV1::MagicRules,
            semantic_contract_id: cr_material.semantic_contract_id.clone(),
        };
        let mut m = base_manifest(cr_identity, cr_material);
        m.rules_snapshot = "synthetic-rules".to_string();
        let value = serde_json::to_value(&m).unwrap();
        println!("NEG_CR_SNAPSHOT: {}", to_canonical_json(&value));
    }

    // Negative: three-way identity mismatch (initial_identity.execution_identity.program_kind differs)
    {
        let m = base_manifest(execution_identity.clone(), material.clone());
        let mut value = serde_json::to_value(&m).unwrap();
        value["initial_identity"]["execution_identity"]["program_kind"] =
            serde_json::Value::String("magic_rules".to_string());
        println!("NEG_THREE_WAY: {}", to_canonical_json(&value));
    }

    // Negative: authoritative three-way identity mismatch (final diverges from initial)
    {
        let m = base_manifest(execution_identity.clone(), material.clone());
        let mut final_identity = m.initial_identity.clone();
        final_identity.execution_identity = ExecutionIdentityV1 {
            program_kind: ExecutionProgramV1::MagicRules,
            semantic_contract_id: m.semantic_contract.semantic_contract_id.clone(),
        };
        let final_digest = mtgml_persistence::checkpoint_digest::calculate_checkpoint_digest_v6(
            &final_identity.full_state_digest.as_digest_reference(),
            &final_identity.episode_status,
            &final_identity.environment_limit_counters,
            &final_identity.checkpoint_codec_identity,
            &final_identity.execution_identity,
        )
        .unwrap();
        final_identity.checkpoint_digest = final_digest;

        let replay = AuthoritativeReplayV6 {
            schema_version: REPLAY_FILE_SCHEMA_V6.to_string(),
            manifest: m,
            steps: vec![],
            final_identity,
        };
        println!(
            "NEG_AUTHORITATIVE_THREE_WAY: {}",
            to_canonical_json(&serde_json::to_value(&replay).unwrap())
        );
    }

    // Negative: unknown field
    {
        let m = base_manifest(execution_identity.clone(), material.clone());
        let mut value = serde_json::to_value(&m).unwrap();
        value["unknown_field"] = serde_json::Value::Bool(true);
        println!("NEG_UNKNOWN_FIELD: {}", to_canonical_json(&value));
    }

    // Negative: wrong schema version
    {
        let mut m = base_manifest(execution_identity.clone(), material);
        m.schema_version = "replay-manifest.v5".to_string();
        let value = serde_json::to_value(&m).unwrap();
        println!("NEG_WRONG_SCHEMA: {}", to_canonical_json(&value));
    }

    // Exact CR SBA semantics require the Magic observation payload codec.
    {
        let rules = RulesContractManifestV1 {
            rules_authority: RulesAuthorityV1::ComprehensiveRules {
                snapshot_id: "cr:sba-codec-test".to_owned(),
            },
            capability_closure: Some(vec![CapabilityRequirementV1 {
                key: "rules/state-based-actions-combat".to_owned(),
                version: "0.1.0".to_owned(),
            }]),
        };
        let value =
            serde_json::to_value(manifest_for_rules(rules, "synthetic-m3-observation.v1")).unwrap();
        println!("NEG_CODEC_SBA_SYNTHETIC: {}", to_canonical_json(&value));
    }

    // A different version does not admit the Magic observation payload.
    {
        let rules = RulesContractManifestV1 {
            rules_authority: RulesAuthorityV1::ComprehensiveRules {
                snapshot_id: "cr:sba-codec-test".to_owned(),
            },
            capability_closure: Some(vec![CapabilityRequirementV1 {
                key: "rules/state-based-actions-combat".to_owned(),
                version: "999.0.0".to_owned(),
            }]),
        };
        let value =
            serde_json::to_value(manifest_for_rules(rules, "magic-m3-observation.v1")).unwrap();
        println!(
            "NEG_CODEC_WRONG_SBA_VERSION_MAGIC: {}",
            to_canonical_json(&value)
        );
    }

    // SyntheticLegacy cannot claim Magic codec semantics by adding a fake closure.
    {
        let m = base_manifest(execution_identity.clone(), synthetic_semantic_material());
        let mut value = serde_json::to_value(&m).unwrap();
        value["schemas"]["observation_payload_codec"] =
            serde_json::Value::String("magic-m3-observation.v1".to_owned());
        value["semantic_contract"]["rules_manifest"]["capability_closure"] =
            serde_json::json!([{"key": "rules/state-based-actions-combat", "version": "0.1.0"}]);
        println!(
            "NEG_CODEC_SYNTHETIC_AUTHORITY_MAGIC: {}",
            to_canonical_json(&value)
        );
    }
}
