use std::mem::size_of;

use mtgml_model::{
    CheckpointCodecIdentity, ContentDigest, EnvironmentLimitCounters, EpisodeStatus,
    FullStateDigestV4, PlayerId, StateRevision,
};
use mtgml_replay::v4::{
    AuthoritativeReplayV4, InitialEnvironmentIdentityV4, ReplayManifestV4, ReplayRecorderV4,
    ReplaySchemaVersionsV4, ReplayStepV4, REPLAY_FILE_SCHEMA_V4, REPLAY_MANIFEST_SCHEMA_V4,
    REPLAY_STEP_SCHEMA_V4,
};
use mtgml_replay::{DeckIdentityV1, KernelIdentityV1, RandomnessIdentityV2, ReplayValidationError};

#[test]
fn p0_replay_v4_identity_family_is_present_and_closed() {
    assert_eq!(REPLAY_MANIFEST_SCHEMA_V4, "replay-manifest.v4");
    assert_eq!(REPLAY_STEP_SCHEMA_V4, "replay-step.v4");
    assert_eq!(REPLAY_FILE_SCHEMA_V4, "authoritative-replay.v4");
    assert!(size_of::<InitialEnvironmentIdentityV4>() > 0);
    assert!(size_of::<ReplayManifestV4>() > 0);
    assert!(size_of::<ReplaySchemaVersionsV4>() > 0);
    assert!(size_of::<ReplayStepV4>() > 0);
    assert!(size_of::<AuthoritativeReplayV4>() > 0);
    assert!(size_of::<ReplayRecorderV4>() > 0);
}

#[test]
fn p0_replay_v4_schema_identity_explicitly_binds_m3_observation_payload() {
    let schemas = ReplaySchemaVersionsV4 {
        observation: "observation-envelope.v1".into(),
        observation_payload_codec: "synthetic-m3-observation.v1".into(),
        information_state: "information-state-envelope.v2".into(),
        decision: "player-decision-request.v2".into(),
        decision_response: "decision-response.v2".into(),
        observed_event: "observed-event-envelope.v2".into(),
        player_step: "player-step.v2".into(),
        replay_step: "replay-step.v4".into(),
    };

    assert_eq!(
        schemas.observation_payload_codec,
        "synthetic-m3-observation.v1"
    );
}

fn valid_manifest_v4() -> ReplayManifestV4 {
    let full_state_digest = FullStateDigestV4::from_digest_bytes([0; 32]);
    let codec = CheckpointCodecIdentity {
        codec_id: "in-memory-reference".into(),
        semantic_version: "4".into(),
    };
    let counters = EnvironmentLimitCounters::default();
    let checkpoint_digest = mtgml_persistence::checkpoint_digest::calculate_checkpoint_digest_v4(
        &full_state_digest.as_digest_reference(),
        &EpisodeStatus::Running,
        &counters,
        &codec,
    )
    .unwrap();

    ReplayManifestV4 {
        schema_version: REPLAY_MANIFEST_SCHEMA_V4.into(),
        engine_build: "synthetic-build".into(),
        kernel: KernelIdentityV1 {
            implementation_id: "synthetic-m3".into(),
            semantic_version: "0.2.2".into(),
            build_profile: "test".into(),
        },
        rules_snapshot: "synthetic-rules".into(),
        format_policy_snapshot: "synthetic-format".into(),
        oracle_snapshot: "synthetic-oracle".into(),
        card_bundle: "synthetic-bundle".into(),
        schemas: ReplaySchemaVersionsV4 {
            observation: "observation-envelope.v1".into(),
            observation_payload_codec: "synthetic-m3-observation.v1".into(),
            information_state: "information-state-envelope.v2".into(),
            decision: "player-decision-request.v2".into(),
            decision_response: "decision-response.v2".into(),
            observed_event: "observed-event-envelope.v2".into(),
            player_step: "player-step.v2".into(),
            replay_step: REPLAY_STEP_SCHEMA_V4.into(),
        },
        randomness: RandomnessIdentityV2 {
            contract_id: "mtgml.rng.v1".into(),
            root_seed_hex: "00".repeat(32),
        },
        decks: vec![DeckIdentityV1 {
            player: PlayerId(1),
            deck_id: "synthetic-deck".into(),
            digest: ContentDigest::parse("11".repeat(32)).unwrap(),
        }],
        initial_identity: InitialEnvironmentIdentityV4 {
            state_revision: StateRevision(0),
            full_state_digest,
            episode_status: EpisodeStatus::Running,
            environment_limit_counters: counters,
            checkpoint_codec_identity: codec,
            checkpoint_digest,
        },
    }
}

#[test]
fn p0_replay_v4_accepts_the_m3_observation_payload_codec() {
    valid_manifest_v4().validate().unwrap();
}

#[test]
fn p0_replay_v4_rejects_the_historical_m2_observation_payload_codec() {
    let mut manifest = valid_manifest_v4();
    manifest.schemas.observation_payload_codec = "synthetic-m2-observation.v1".into();

    assert_eq!(
        manifest.validate(),
        Err(ReplayValidationError::ReplayStepIdentity)
    );
}
