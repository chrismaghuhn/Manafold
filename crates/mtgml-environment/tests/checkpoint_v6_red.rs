//! S3.P0 Task 1 RED contract for the complete V6 environment checkpoint.

use mtgml_environment::{
    EnvironmentCheckpointV6, CHECKPOINT_CODEC_ID_V6, CHECKPOINT_CODEC_SEMANTIC_VERSION_V6,
    ENVIRONMENT_CHECKPOINT_SCHEMA_V6,
};
use mtgml_model::{
    CheckpointCodecIdentity, CheckpointDigestV6, EnvironmentLimitCounters, EpisodeStatus,
    ExecutionIdentityV1, ExecutionProgramV1, FullStateDigestV5, PlayerId, SemanticContractIdV1,
};
use mtgml_random::RootSeed256;
use mtgml_state::{
    construct_synthetic_engine_state, EngineState, SyntheticResetInputs, SyntheticV4Setup,
};

const CONTRACT_A: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const CONTRACT_B: &str = "2222222222222222222222222222222222222222222222222222222222222222";

fn state() -> EngineState {
    construct_synthetic_engine_state(SyntheticResetInputs {
        players: [PlayerId(1), PlayerId(2)],
        root_seed: RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap(),
        setup: SyntheticV4Setup::m2_compatibility(),
    })
    .unwrap()
}

fn identity(contract: &str) -> ExecutionIdentityV1 {
    ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::SyntheticRulesCompat,
        semantic_contract_id: SemanticContractIdV1::parse(contract).unwrap(),
    }
}

fn checkpoint() -> EnvironmentCheckpointV6 {
    EnvironmentCheckpointV6::new(
        state(),
        EpisodeStatus::Running,
        EnvironmentLimitCounters::default(),
        CheckpointCodecIdentity {
            codec_id: CHECKPOINT_CODEC_ID_V6.to_owned(),
            semantic_version: CHECKPOINT_CODEC_SEMANTIC_VERSION_V6.to_owned(),
        },
        identity(CONTRACT_A),
    )
    .unwrap()
}

#[test]
fn v6_checkpoint_exposes_the_complete_new_identity_surface() {
    assert_eq!(
        ENVIRONMENT_CHECKPOINT_SCHEMA_V6,
        "environment-checkpoint.v6"
    );
    assert_eq!(CHECKPOINT_CODEC_ID_V6, "in-memory-reference");
    assert_eq!(CHECKPOINT_CODEC_SEMANTIC_VERSION_V6, "6");

    let field_contract = |checkpoint: &EnvironmentCheckpointV6| {
        let _: &EngineState = &checkpoint.state;
        let _: &FullStateDigestV5 = &checkpoint.state_digest;
        let _: &EpisodeStatus = &checkpoint.status;
        let _: &EnvironmentLimitCounters = &checkpoint.limit_counters;
        let _: &CheckpointCodecIdentity = &checkpoint.codec;
        let _: &ExecutionIdentityV1 = &checkpoint.execution_identity;
        let _: &CheckpointDigestV6 = &checkpoint.checkpoint_digest;
    };
    let _ = field_contract;
}

#[test]
fn v6_checkpoint_construction_and_recomputation_are_exact() {
    let checkpoint = checkpoint();
    assert_eq!(
        checkpoint.state_digest.as_str(),
        "c0438363164ab19eeeb50dd43130a3136642d75274938775a0f2d7555faa026b"
    );
    assert_eq!(
        checkpoint.checkpoint_digest.as_str(),
        "a6d255706349703270a82e44aec8587fb229d8935d644a00c11cbe4fdb79a644"
    );
    assert_eq!(checkpoint.schema_version, ENVIRONMENT_CHECKPOINT_SCHEMA_V6);
    assert_eq!(checkpoint.state_digest, checkpoint.state.digest().unwrap());
    checkpoint.validate().unwrap();

    let mut bad_state_digest = checkpoint.clone();
    bad_state_digest.state_digest = FullStateDigestV5::from_digest_bytes([0xee; 32]);
    assert!(bad_state_digest.validate().is_err());

    let mut bad_checkpoint_digest = checkpoint.clone();
    bad_checkpoint_digest.checkpoint_digest = CheckpointDigestV6::from_digest_bytes([0xdd; 32]);
    assert!(bad_checkpoint_digest.validate().is_err());

    let mut bad_execution_identity = checkpoint.clone();
    bad_execution_identity.execution_identity = identity(CONTRACT_B);
    assert!(bad_execution_identity.validate().is_err());

    let mut bad_codec = checkpoint;
    bad_codec.codec.semantic_version = "5".into();
    assert!(bad_codec.validate().is_err());
}
