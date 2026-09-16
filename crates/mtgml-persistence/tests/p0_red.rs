use mtgml_model::{
    CheckpointCodecIdentity, CheckpointDigestV4, EnvironmentLimitCounters, EpisodeStatus,
    FullStateDigestV4,
};
use mtgml_persistence::checkpoint_digest::calculate_checkpoint_digest_v4;

#[test]
fn p0_checkpoint_digest_v4_binds_v4_state_and_codec_identity() {
    let state_digest = FullStateDigestV4::from_digest_bytes([7; 32]);
    let result: CheckpointDigestV4 = calculate_checkpoint_digest_v4(
        &state_digest.as_digest_reference(),
        &EpisodeStatus::Running,
        &EnvironmentLimitCounters::default(),
        &CheckpointCodecIdentity {
            codec_id: "in-memory-reference".into(),
            semantic_version: "4".into(),
        },
    )
    .unwrap();

    assert_eq!(result.as_str().len(), 64);
}
