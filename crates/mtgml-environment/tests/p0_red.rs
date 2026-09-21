use mtgml_environment::{
    EnvironmentBackend, EnvironmentCheckpointV5, ENVIRONMENT_CHECKPOINT_SCHEMA_V5,
};
use mtgml_replay::AuthoritativeReplayV5;

#[allow(dead_code)]
fn current_environment_products_are_v5<B: EnvironmentBackend>(backend: &B) {
    let _: EnvironmentCheckpointV5 = backend.checkpoint().expect("checkpoint");
    let _: AuthoritativeReplayV5 = backend.export_replay().expect("replay");
}

#[test]
fn p0_current_checkpoint_schema_is_v5() {
    assert_eq!(
        ENVIRONMENT_CHECKPOINT_SCHEMA_V5,
        "environment-checkpoint.v5"
    );
}
