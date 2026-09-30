use mtgml_environment::EnvironmentBackend;
use mtgml_environment::{EnvironmentCheckpointV8, ENVIRONMENT_CHECKPOINT_SCHEMA_V8};
use mtgml_replay::AuthoritativeReplayV8;

#[allow(dead_code)]
fn current_environment_products_match_backend_mode<B: EnvironmentBackend>(backend: &B) {
    let _: EnvironmentCheckpointV8 = backend.checkpoint().expect("checkpoint");
    let _: AuthoritativeReplayV8 = backend.export_replay().expect("replay");
}

#[test]
fn checkpoint_schema_matches_backend_mode() {
    assert_eq!(
        ENVIRONMENT_CHECKPOINT_SCHEMA_V8,
        "environment-checkpoint.v8"
    );
}
