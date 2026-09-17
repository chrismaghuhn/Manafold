use mtgml_environment::{
    EnvironmentBackend, EnvironmentCheckpointV4, ENVIRONMENT_CHECKPOINT_SCHEMA,
};
use mtgml_replay::AuthoritativeReplayV4;

#[allow(dead_code)]
fn current_environment_products_are_v4<B: EnvironmentBackend>(backend: &B) {
    let _: EnvironmentCheckpointV4 = backend.checkpoint().expect("checkpoint");
    let _: AuthoritativeReplayV4 = backend.export_replay().expect("replay");
}

#[test]
fn p0_current_checkpoint_schema_is_v4() {
    assert_eq!(ENVIRONMENT_CHECKPOINT_SCHEMA, "environment-checkpoint.v4");
}
