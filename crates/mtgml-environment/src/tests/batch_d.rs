// Ownership fragment: Batch-D checkpoint closure evidence. Included
// lexically by tests.rs.

#[test]
fn batch_d_invalid_ordered_state_cannot_construct_checkpoint() {
    let mut state =
        mtgml_state::construct_synthetic_engine_state(mtgml_state::SyntheticResetInputs {
            players: [PlayerId(1), PlayerId(2)],
            root_seed: seed(),
        })
        .unwrap();
    state
        .zones
        .locations
        .get_mut(&mtgml_model::GameObjectId(2))
        .unwrap()
        .position = mtgml_state::ZonePosition::Bottom { offset: 0 };

    assert_eq!(
        EnvironmentCheckpointV3::new(
            state,
            EpisodeStatus::Running,
            EnvironmentLimitCounters::default(),
            CheckpointCodecIdentity {
                codec_id: "synthetic-m2-memory".into(),
                semantic_version: "3".into(),
            },
        ),
        Err(CheckpointValidationError::StateDigest)
    );
}
