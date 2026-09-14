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
    let codec = CheckpointCodecIdentity {
        codec_id: "synthetic-m2-memory".into(),
        semantic_version: "3".into(),
    };
    assert!(
        EnvironmentCheckpointV3::new(
            state.clone(),
            EpisodeStatus::Running,
            EnvironmentLimitCounters::default(),
            codec.clone(),
        )
        .is_ok()
    );
    state
        .zones
        .locations
        .get_mut(&mtgml_model::GameObjectId(2))
        .unwrap()
        .position = mtgml_state::ZonePosition::Bottom { offset: 0 };
    state
        .knowledge
        .players
        .get_mut(&PlayerId(2))
        .unwrap()
        .active
        .get_mut(&mtgml_model::OpaqueObjectId(2))
        .unwrap()
        .known_location
        .as_mut()
        .unwrap()
        .location
        .position = mtgml_state::ZonePosition::Bottom { offset: 0 };

    assert_eq!(
        EnvironmentCheckpointV3::new(
            state,
            EpisodeStatus::Running,
            EnvironmentLimitCounters::default(),
            codec,
        ),
        Err(CheckpointValidationError::StateDigest)
    );
}
