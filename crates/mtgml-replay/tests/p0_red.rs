use std::mem::size_of;

use mtgml_replay::{
    AuthoritativeReplayV4, InitialEnvironmentIdentityV4, ReplayManifestV4, ReplayRecorderV4,
    ReplaySchemaVersionsV4, ReplayStepV4, REPLAY_FILE_SCHEMA_V4, REPLAY_MANIFEST_SCHEMA_V4,
    REPLAY_STEP_SCHEMA_V4,
};

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
