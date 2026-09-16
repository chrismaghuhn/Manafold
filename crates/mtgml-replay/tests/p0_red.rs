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
