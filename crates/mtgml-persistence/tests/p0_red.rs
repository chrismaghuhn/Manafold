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

#[test]
fn p0_checkpoint_digest_v4_known_answers() {
    let codec = CheckpointCodecIdentity {
        codec_id: "in-memory-reference".into(),
        semantic_version: "4".into(),
    };
    let counters = EnvironmentLimitCounters::default();
    let seven = FullStateDigestV4::from_digest_bytes([7; 32]);
    let seven_digest = calculate_checkpoint_digest_v4(
        &seven.as_digest_reference(),
        &EpisodeStatus::Running,
        &counters,
        &codec,
    )
    .unwrap();
    assert_eq!(
        seven_digest.as_str(),
        "c880c64a0ce039c87f4b79d9f80280d203e7c394b56f08bdd808e410b7cb023c"
    );
    let zero = FullStateDigestV4::from_digest_bytes([0; 32]);
    let zero_digest = calculate_checkpoint_digest_v4(
        &zero.as_digest_reference(),
        &EpisodeStatus::Running,
        &counters,
        &codec,
    )
    .unwrap();
    assert_eq!(
        zero_digest.as_str(),
        "8e24ae4933f04d8b93f4afa4d76711ce9109112c21a05f8c90164564cf5b5f2c"
    );
}
