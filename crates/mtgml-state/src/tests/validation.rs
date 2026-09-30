// Ownership fragment: cross-component EngineState violation evidence. Included lexically by tests.rs so
// every identity remains tests::<name>.

#[test]
fn synthetic_state_is_the_current_engine_state_shape() {
    let state = synthetic_state();
    validate_engine_state(&state).unwrap();
    assert_eq!(state.revision, StateRevision(0));
    assert_eq!(state.execution, crate::ExecutionState::default());
    assert_eq!(state.knowledge.players.len(), 2);
    assert_eq!(state.perspective_identities.players.len(), 2);
}

#[test]
fn valid_empty_shell_passes_cross_component_validation() {
    let state = empty_shell();
    validate_engine_state(&state).unwrap();
}


#[test]
fn unordered_object_must_not_appear_in_ordered_zones() {
    let mut state = synthetic_state();
    let key = state.zones.locations.get(&GameObjectId(2)).unwrap().key();
    state
        .zones
        .ordered_zones
        .get_mut(&key)
        .unwrap()
        .push(GameObjectId(1));
    assert!(matches!(
        validate_engine_state(&state),
        Err(EngineStateViolation::OrderedZoneMismatch)
    ));
}

#[test]
fn ordered_object_must_appear_exactly_once() {
    let mut state = synthetic_state();
    let key = state.zones.locations.get(&GameObjectId(2)).unwrap().key();
    let ordered = state.zones.ordered_zones.get_mut(&key).unwrap();
    ordered.push(GameObjectId(2));
    assert!(matches!(
        validate_engine_state(&state),
        Err(EngineStateViolation::OrderedZoneMismatch)
    ));
}

#[test]
fn ordered_object_must_not_be_missing() {
    let mut state = synthetic_state();
    state
        .zones
        .locations
        .get_mut(&GameObjectId(2))
        .unwrap()
        .position = ZonePosition::Unordered;
    assert!(matches!(
        validate_engine_state(&state),
        Err(EngineStateViolation::OrderedZoneMismatch)
    ));
}

#[test]
fn duplicate_live_physical_card_incarnation_rejected() {
    let mut state = synthetic_state();
    state
        .zones
        .objects
        .get_mut(&GameObjectId(1))
        .unwrap()
        .physical_card = Some(PhysicalCardId(2));
    assert!(matches!(
        validate_engine_state(&state),
        Err(EngineStateViolation::DuplicatePhysicalCard)
    ));
}



#[test]
fn opaque_allocator_must_reference_declared_player() {
    let mut state = empty_shell();
    state.random.streams.insert(
        RandomStreamKeyV1::player_scoped(RandomStreamKindV1::SyntheticM1, 9),
        RandomStreamCursorV1::default(),
    );
    assert!(matches!(
        validate_engine_state(&state),
        Err(EngineStateViolation::RandomState)
    ));
}

#[test]
fn commander_designation_must_reference_owned_live_physical_card() {
    let mut state = synthetic_state();
    state.format = FormatState::Commander {
        state: CommanderState {
            designations: BTreeMap::from([(PlayerId(2), vec![PhysicalCardId(1)])]),
            cast_counts: BTreeMap::new(),
            damage: BTreeMap::new(),
        },
    };
    assert!(matches!(
        validate_engine_state(&state),
        Err(EngineStateViolation::FormatMismatch)
    ));
}

#[test]
fn valid_commander_structural_references_are_accepted() {
    let mut state = synthetic_state();
    state.format = FormatState::Commander {
        state: CommanderState {
            designations: BTreeMap::from([(PlayerId(1), vec![PhysicalCardId(1)])]),
            cast_counts: BTreeMap::from([(PhysicalCardId(1), 2)]),
            damage: BTreeMap::from([(PhysicalCardId(1), BTreeMap::from([(PlayerId(2), 5)]))]),
        },
    };
    validate_engine_state(&state).unwrap();
}

#[test]
fn commander_designation_membership_must_be_canonical() {
    let mut sorted = lifecycle_fixture();
    sorted.format = FormatState::Commander {
        state: CommanderState {
            designations: BTreeMap::from([(
                PlayerId(1),
                vec![PhysicalCardId(3), PhysicalCardId(4)],
            )]),
            cast_counts: BTreeMap::new(),
            damage: BTreeMap::new(),
        },
    };
    assert_eq!(validate_engine_state(&sorted), Ok(()));

    let mut permuted = sorted.clone();
    if let FormatState::Commander { state: commander } = &mut permuted.format {
        commander.designations.insert(
            PlayerId(1),
            vec![PhysicalCardId(4), PhysicalCardId(3)],
        );
    }
    let before = permuted.clone();
    assert_eq!(
        validate_engine_state(&permuted),
        Err(EngineStateViolation::FormatMismatch)
    );
    assert_eq!(permuted, before);
}

#[test]
fn commander_ledger_must_reference_a_designated_physical_card() {
    let mut state = synthetic_state();
    state.format = FormatState::Commander {
        state: CommanderState {
            designations: BTreeMap::from([(PlayerId(1), vec![PhysicalCardId(1)])]),
            cast_counts: BTreeMap::from([(PhysicalCardId(2), 1)]),
            damage: BTreeMap::new(),
        },
    };
    assert!(matches!(
        validate_engine_state(&state),
        Err(EngineStateViolation::FormatMismatch)
    ));
}

#[test]
fn commander_damage_ledger_must_reference_a_declared_player() {
    let mut state = synthetic_state();
    state.format = FormatState::Commander {
        state: CommanderState {
            designations: BTreeMap::from([(PlayerId(1), vec![PhysicalCardId(1)])]),
            cast_counts: BTreeMap::new(),
            damage: BTreeMap::from([(PhysicalCardId(1), BTreeMap::from([(PlayerId(9), 3)]))]),
        },
    };
    assert!(matches!(
        validate_engine_state(&state),
        Err(EngineStateViolation::FormatMismatch)
    ));
}

#[test]
fn empty_ordered_zone_keys_must_reference_declared_players() {
    let mut state = synthetic_state();
    state.zones.ordered_zones.insert(
        ZoneKey {
            zone: ZoneKind::Library,
            player: Some(PlayerId(999)),
            visibility: VisibilityPartition::FaceDown,
            partition: None,
        },
        Vec::new(),
    );
    let before = state.clone();
    assert_eq!(
        validate_engine_state(&state),
        Err(EngineStateViolation::ObjectPlayerMismatch)
    );
    assert_eq!(state, before);
}




#[test]
fn simultaneous_violations_preserve_the_existing_error_precedence() {
    // Zones-segment violations win over a later random-state violation.
    let mut state = synthetic_state();
    let key = state.zones.locations.get(&GameObjectId(2)).unwrap().key();
    state
        .zones
        .ordered_zones
        .get_mut(&key)
        .unwrap()
        .push(GameObjectId(2));
    state.random.streams.insert(
        RandomStreamKeyV1::player_scoped(RandomStreamKindV1::SyntheticM1, 9),
        RandomStreamCursorV1::default(),
    );
    assert!(matches!(
        validate_engine_state(&state),
        Err(EngineStateViolation::OrderedZoneMismatch)
    ));

    // Shape violations win over format-segment violations.
    let mut state = synthetic_state();
    state.knowledge.players.remove(&PlayerId(2));
    state.format = FormatState::Commander {
        state: CommanderState {
            designations: BTreeMap::from([(PlayerId(2), vec![PhysicalCardId(1)])]),
            cast_counts: BTreeMap::new(),
            damage: BTreeMap::new(),
        },
    };
    assert!(matches!(
        validate_engine_state(&state),
        Err(EngineStateViolation::EngineStateShape(
            EngineStateShapeViolation::PlayerCoverage
        ))
    ));
}
