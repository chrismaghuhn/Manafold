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

#[test]
fn nine_attackers_are_a_valid_combat() {
    // CR 508.1a: the active player chooses any number of their creatures that
    // can attack. The state has no cap.
    let mut state = synthetic_state();
    let attackers: Vec<GameObjectId> = (3..12).map(GameObjectId).collect();
    for id in &attackers {
        state.zones.objects.insert(
            *id,
            GameObject {
                id: *id,
                physical_card: Some(PhysicalCardId(id.0)),
                card_definition: CardDefinitionId(1),
                owner: PlayerId(1),
                controller: PlayerId(1),
                tapped: true,
                face_down: false,
            },
        );
        state.zones.locations.insert(*id, public_location());
    }
    state.allocators.next_object_id = GameObjectId(12);
    state.core.position = TurnPosition::Combat {
        step: crate::CombatStep::DeclareAttackers,
    };
    state.combat = Some(CombatState {
        defending_player: PlayerId(2),
        attackers: attackers.clone(),
        damage_step_completed: false,
        blocked_attackers: BTreeSet::new(),
        blockers: BTreeMap::new(),
    });
    assert_eq!(attackers.len(), 9);
    validate_engine_state(&state).unwrap();
}

/// A permanent on the battlefield under `controller`'s control.
fn put_on_battlefield(state: &mut EngineState, id: u64, controller: PlayerId) {
    let id = GameObjectId(id);
    state.zones.objects.insert(
        id,
        GameObject {
            id,
            physical_card: Some(PhysicalCardId(id.0)),
            card_definition: CardDefinitionId(1),
            owner: controller,
            controller,
            tapped: false,
            face_down: false,
        },
    );
    state.zones.locations.insert(id, public_location());
    state.allocators.next_object_id = GameObjectId(state.allocators.next_object_id.0.max(id.0 + 1));
}

/// Player 1 attacks player 2 with objects 3 and 4 and player 2 controls
/// objects 5 and 6, in the declare blockers step before any block.
fn combat_before_blocks() -> EngineState {
    let mut state = synthetic_state();
    for (id, controller) in [(3, 1), (4, 1), (5, 2), (6, 2)] {
        put_on_battlefield(&mut state, id, PlayerId(controller));
    }
    state.core.position = TurnPosition::Combat {
        step: crate::CombatStep::DeclareBlockers,
    };
    state.combat = Some(CombatState {
        defending_player: PlayerId(2),
        attackers: vec![GameObjectId(3), GameObjectId(4)],
        damage_step_completed: false,
        blocked_attackers: BTreeSet::new(),
        blockers: BTreeMap::new(),
    });
    state
}

/// `blocker` blocks `attacker`, which becomes blocked (CR 509.1g, 509.1h).
fn block(state: &mut EngineState, blocker: u64, attacker: u64) {
    let combat = state.combat.as_mut().unwrap();
    combat
        .blockers
        .insert(GameObjectId(blocker), GameObjectId(attacker));
    combat.blocked_attackers.insert(GameObjectId(attacker));
}

#[test]
fn two_blockers_on_one_attacker_validate() {
    // CR 509.1a, 509.1g: any number of creatures may block one attacker, and
    // the state has no cap.
    let mut state = combat_before_blocks();
    block(&mut state, 5, 3);
    block(&mut state, 6, 3);
    validate_engine_state(&state).unwrap();

    // One blocker on each attacker validates as well.
    let mut state = combat_before_blocks();
    block(&mut state, 5, 3);
    block(&mut state, 6, 4);
    validate_engine_state(&state).unwrap();
}

#[test]
fn a_blocked_attacker_stays_blocked_without_a_blocker() {
    // CR 509.1h: the attacker remains blocked when every blocker has left.
    let mut state = combat_before_blocks();
    state
        .combat
        .as_mut()
        .unwrap()
        .blocked_attackers
        .insert(GameObjectId(3));
    validate_engine_state(&state).unwrap();
}

#[test]
fn a_blocker_of_the_attacking_player_is_rejected() {
    // CR 509.1a: the defending player chooses the blockers.
    let mut state = combat_before_blocks();
    put_on_battlefield(&mut state, 7, PlayerId(1));
    block(&mut state, 7, 3);
    assert_eq!(
        validate_engine_state(&state),
        Err(EngineStateViolation::CombatState)
    );
}

#[test]
fn a_blocker_for_a_non_attacker_is_rejected() {
    // Object 7 is a creature that did not attack.
    let mut state = combat_before_blocks();
    put_on_battlefield(&mut state, 7, PlayerId(1));
    let mut blocked = state.clone();
    block(&mut blocked, 5, 7);
    assert_eq!(
        validate_engine_state(&blocked),
        Err(EngineStateViolation::CombatState)
    );
    // Not even without recording the attacker as blocked.
    state
        .combat
        .as_mut()
        .unwrap()
        .blockers
        .insert(GameObjectId(5), GameObjectId(7));
    assert_eq!(
        validate_engine_state(&state),
        Err(EngineStateViolation::CombatState)
    );
}

#[test]
fn a_blocker_is_on_the_battlefield() {
    // Object 2 is a card in player 2's library; object 99 does not exist.
    for blocker in [2, 99] {
        let mut state = combat_before_blocks();
        block(&mut state, blocker, 3);
        assert!(validate_engine_state(&state).is_err(), "blocker {blocker}");
    }
}

#[test]
fn a_block_makes_its_attacker_blocked() {
    // CR 509.1h: an attacker with a blocker is a blocked attacker.
    let mut state = combat_before_blocks();
    block(&mut state, 5, 3);
    state.combat.as_mut().unwrap().blocked_attackers.clear();
    assert_eq!(
        validate_engine_state(&state),
        Err(EngineStateViolation::CombatState)
    );
}
