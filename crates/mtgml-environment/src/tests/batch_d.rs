// Ownership fragment: Batch-D checkpoint closure evidence. Included
// lexically by tests.rs.

#[test]
fn batch_d_invalid_ordered_state_cannot_construct_checkpoint() {
    let mut state =
        mtgml_state::construct_synthetic_engine_state(mtgml_state::SyntheticResetInputs {
            players: [PlayerId(1), PlayerId(2)],
            root_seed: seed(),
            setup: mtgml_state::SyntheticV4Setup::m2_compatibility(),
        })
        .unwrap();
    let codec = CheckpointCodecIdentity {
        codec_id: "in-memory-reference".into(),
        semantic_version: "5".into(),
    };
    assert!(
        EnvironmentCheckpointV5::new(
            state.clone(),
            EpisodeStatus::Running,
            EnvironmentLimitCounters::default(),
            codec.clone(),
        synthetic_identity()
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
        EnvironmentCheckpointV5::new(
            state,
            EpisodeStatus::Running,
            EnvironmentLimitCounters::default(),
            codec,
        synthetic_identity()
        ),
        Err(CheckpointValidationError::StateDigest)
    );
}
