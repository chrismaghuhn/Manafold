//! S3.P0 Task 1 RED contract for the typed Replay V6 identity family.

use mtgml_decision::DecisionResponseV2;
use mtgml_model::{CheckpointDigestV6, FullStateDigestV5};
use mtgml_replay::{
    AuthoritativeReplayV6, InitialEnvironmentIdentityV6, ReplayManifestV6, ReplayRecorderV6,
    ReplaySchemaVersionsV6, ReplayStepV6, REPLAY_FILE_SCHEMA_V6, REPLAY_MANIFEST_SCHEMA_V6,
    REPLAY_STEP_SCHEMA_V6,
};

#[test]
fn replay_v6_binds_new_state_checkpoint_and_real_response_identities() {
    assert_eq!(REPLAY_MANIFEST_SCHEMA_V6, "replay-manifest.v6");
    assert_eq!(REPLAY_STEP_SCHEMA_V6, "replay-step.v6");
    assert_eq!(REPLAY_FILE_SCHEMA_V6, "authoritative-replay.v6");

    let initial_identity = |identity: &InitialEnvironmentIdentityV6| {
        let _: &FullStateDigestV5 = &identity.full_state_digest;
        let _: &CheckpointDigestV6 = &identity.checkpoint_digest;
    };
    let replay_step = |step: &ReplayStepV6| {
        let _: &CheckpointDigestV6 = &step.checkpoint_digest_before;
        let _: &CheckpointDigestV6 = &step.checkpoint_digest_after;
        let _: &FullStateDigestV5 = &step.full_state_digest_after;
        let _: &DecisionResponseV2 = &step.response;
    };
    let replay_family = |manifest: ReplayManifestV6,
                         schemas: ReplaySchemaVersionsV6,
                         recorder: ReplayRecorderV6,
                         replay: AuthoritativeReplayV6| {
        drop((manifest, schemas, recorder, replay));
    };

    let _ = (initial_identity, replay_step, replay_family);
}
