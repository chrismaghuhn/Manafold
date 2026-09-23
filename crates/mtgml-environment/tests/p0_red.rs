use mtgml_environment::{
    EnvironmentBackend, EnvironmentCheckpointV6, ENVIRONMENT_CHECKPOINT_SCHEMA_V6,
};
use mtgml_replay::AuthoritativeReplayV6;

#[allow(dead_code)]
fn current_environment_products_are_v6<B: EnvironmentBackend>(backend: &B) {
    let _: EnvironmentCheckpointV6 = backend.checkpoint().expect("checkpoint");
    let _: AuthoritativeReplayV6 = backend.export_replay().expect("replay");
}

#[test]
fn p0_current_checkpoint_schema_is_v6() {
    assert_eq!(
        ENVIRONMENT_CHECKPOINT_SCHEMA_V6,
        "environment-checkpoint.v6"
    );
}
