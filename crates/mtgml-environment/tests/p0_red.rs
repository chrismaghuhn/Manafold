use mtgml_environment::EnvironmentBackend;
#[cfg(feature = "historical-conformance-runtime")]
use mtgml_environment::{EnvironmentCheckpointV6, ENVIRONMENT_CHECKPOINT_SCHEMA_V6};
#[cfg(not(feature = "historical-conformance-runtime"))]
use mtgml_environment::{EnvironmentCheckpointV7, ENVIRONMENT_CHECKPOINT_SCHEMA_V7};
#[cfg(feature = "historical-conformance-runtime")]
use mtgml_replay::AuthoritativeReplayV6;
#[cfg(not(feature = "historical-conformance-runtime"))]
use mtgml_replay::AuthoritativeReplayV7;

#[allow(dead_code)]
fn current_environment_products_match_backend_mode<B: EnvironmentBackend>(backend: &B) {
    #[cfg(feature = "historical-conformance-runtime")]
    {
        let _: EnvironmentCheckpointV6 = backend.checkpoint().expect("checkpoint");
        let _: AuthoritativeReplayV6 = backend.export_replay().expect("replay");
    }
    #[cfg(not(feature = "historical-conformance-runtime"))]
    {
        let _: EnvironmentCheckpointV7 = backend.checkpoint().expect("checkpoint");
        let _: AuthoritativeReplayV7 = backend.export_replay().expect("replay");
    }
}

#[test]
fn checkpoint_schema_matches_backend_mode() {
    #[cfg(feature = "historical-conformance-runtime")]
    assert_eq!(
        ENVIRONMENT_CHECKPOINT_SCHEMA_V6,
        "environment-checkpoint.v6"
    );
    #[cfg(not(feature = "historical-conformance-runtime"))]
    assert_eq!(
        ENVIRONMENT_CHECKPOINT_SCHEMA_V7,
        "environment-checkpoint.v7"
    );
}
