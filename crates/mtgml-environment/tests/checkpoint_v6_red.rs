//! S3.P0 Task 1 RED contract for the complete V6 environment checkpoint.

use mtgml_environment::{
    EnvironmentCheckpointV6, CHECKPOINT_CODEC_ID_V6, CHECKPOINT_CODEC_SEMANTIC_VERSION_V6,
    ENVIRONMENT_CHECKPOINT_SCHEMA_V6,
};
use mtgml_model::{
    CheckpointCodecIdentity, CheckpointDigestV6, EnvironmentLimitCounters, EpisodeStatus,
    ExecutionIdentityV1, FullStateDigestV5,
};
use mtgml_state::EngineState;

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
