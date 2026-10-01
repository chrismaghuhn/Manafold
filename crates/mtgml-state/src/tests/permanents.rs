// Permanents: since which turn each battlefield object's controller has
// controlled it.

/// The synthetic state with Magic card-rules authority (every live object
/// has a face), whose battlefield object 1 is controlled since turn 1.
fn state_with_content_authority() -> EngineState {
    let mut state = synthetic_state();
    state.card_rules.faces.faces =
        BTreeMap::from([(GameObjectId(1), 0), (GameObjectId(2), 0)]);
    state
        .card_rules
        .permanents
        .enter(GameObjectId(1), 1)
        .unwrap();
    state.validate().unwrap();
    state
}

/// `before` one revision later with object 3 on the battlefield. The entry
/// of object 3 is `entry`, if any.
fn with_object_on_the_battlefield(before: &EngineState, entry: Option<u64>) -> EngineState {
    let mut after = before.clone();
    after.revision = StateRevision(before.revision.0 + 1);
    let id = GameObjectId(3);
    after.zones.objects.insert(
        id,
        GameObject {
            id,
            physical_card: Some(PhysicalCardId(3)),
            card_definition: CardDefinitionId(1),
            owner: PlayerId(1),
            controller: PlayerId(1),
            tapped: false,
            face_down: false,
        },
    );
    after.zones.locations.insert(id, public_location());
    after.allocators.next_object_id = GameObjectId(4);
    after.card_rules.faces.faces.insert(id, 0);
    if let Some(turn) = entry {
        after.card_rules.permanents.permanents.insert(
            id,
            PermanentState {
                controlled_since_turn: turn,
                marked_damage: 0,
            },
        );
    }
    after
}

fn object_entered(to_zone: ZoneKind) -> SemanticDeltaOperation {
    SemanticDeltaOperation::ObjectEntered {
        old_object: None,
        new_object: GameObjectId(3),
        from_zone: ZoneKind::Hand,
        to_zone,
        tapped: false,
        face: 0,
    }
}

#[test]
fn entering_records_the_turn_and_fails_on_a_duplicate() {
    let mut permanents = PermanentsState::default();
    permanents.enter(GameObjectId(7), 3).unwrap();
    assert_eq!(
        permanents.permanents,
        BTreeMap::from([(
            GameObjectId(7),
            PermanentState {
                controlled_since_turn: 3,
                marked_damage: 0
            }
        )])
    );
    assert_eq!(
        permanents.enter(GameObjectId(7), 4),
        Err(StateFamilyMutationError::Duplicate)
    );
    assert_eq!(
        permanents.permanents[&GameObjectId(7)].controlled_since_turn,
        3,
        "a failed entry leaves the record unchanged"
    );
}

#[test]
fn pruning_keeps_only_entries_of_battlefield_objects() {
    let mut permanents = PermanentsState::default();
    for object in [1, 2, 3] {
        permanents.enter(GameObjectId(object), object).unwrap();
    }
    permanents.prune_departed_objects(&BTreeSet::from([GameObjectId(1), GameObjectId(3)]));
    assert_eq!(
        permanents.permanents.keys().copied().collect::<Vec<_>>(),
        [GameObjectId(1), GameObjectId(3)]
    );
}

#[test]
fn a_battlefield_object_without_an_entry_is_rejected() {
    let mut state = state_with_content_authority();
    state.card_rules.permanents = PermanentsState::default();
    assert_eq!(state.validate(), Err(EngineStateError::StateInvariant));
}

#[test]
fn an_entry_for_a_library_object_is_rejected() {
    let mut state = state_with_content_authority();
    state
        .card_rules
        .permanents
        .enter(GameObjectId(2), 1)
        .unwrap();
    assert_eq!(state.validate(), Err(EngineStateError::StateInvariant));
}

#[test]
fn an_entry_from_a_later_turn_is_rejected() {
    let mut state = state_with_content_authority();
    assert_eq!(state.core.turn_number, 1);
    state
        .card_rules
        .permanents
        .permanents
        .get_mut(&GameObjectId(1))
        .unwrap()
        .controlled_since_turn = 2;
    assert_eq!(state.validate(), Err(EngineStateError::StateInvariant));
}

#[test]
fn the_synthetic_shape_needs_no_entries_but_rejects_dangling_ones() {
    // The synthetic-compatibility shape has no faces and no abilities: its
    // battlefield object (1) needs no entry. An entry it does have must still
    // name a battlefield object and a turn that has begun.
    let state = synthetic_state();
    assert_eq!(state.core.turn_number, 1);
    state.validate().unwrap();

    for turn in [0, 1] {
        let mut state = synthetic_state();
        state
            .card_rules
            .permanents
            .enter(GameObjectId(1), turn)
            .unwrap();
        state.validate().unwrap();
    }

    let dangling: [(&str, GameObjectId, u64); 3] = [
        ("a library object", GameObjectId(2), 1),
        ("an object that does not exist", GameObjectId(99), 1),
        ("a later turn", GameObjectId(1), 2),
    ];
    for (what, object, turn) in dangling {
        let mut state = synthetic_state();
        state.card_rules.permanents.enter(object, turn).unwrap();
        assert_eq!(
            state.validate(),
            Err(EngineStateError::StateInvariant),
            "an entry for {what} must be rejected without card-rules authority"
        );
    }
}

#[test]
fn the_digest_record_ends_with_the_sorted_permanents() {
    let mut state = state_with_content_authority();
    state
        .card_rules
        .permanents
        .permanents
        .insert(
            GameObjectId(3),
            PermanentState {
                controlled_since_turn: 0,
                marked_damage: 2,
            },
        );
    let Value::Array(record) = state.card_rules.canonical_value().unwrap() else {
        panic!("the card-rules record is an array");
    };
    assert_eq!(record.len(), 8);
    assert_eq!(
        record[7],
        Value::Array(vec![
            Value::Array(vec![
                Value::Unsigned(1),
                Value::Unsigned(1),
                Value::Unsigned(0)
            ]),
            Value::Array(vec![
                Value::Unsigned(3),
                Value::Unsigned(0),
                Value::Unsigned(2)
            ]),
        ])
    );
}

#[test]
fn the_digest_binds_the_marked_damage_of_each_permanent() {
    let baseline = state_with_content_authority();
    let digest = |state: &EngineState| calculate_full_state_digest(state).unwrap();
    let mut seen = BTreeSet::from([digest(&baseline)]);
    for damage in [1, 2, u64::MAX] {
        let mut marked = baseline.clone();
        marked
            .card_rules
            .permanents
            .mark_damage(GameObjectId(1), damage)
            .unwrap();
        assert!(seen.insert(digest(&marked)), "{damage} marked damage");
    }
}

#[test]
fn delta_accepts_an_entry_made_when_the_object_enters_the_battlefield() {
    let before = state_with_content_authority();
    let after = with_object_on_the_battlefield(&before, Some(before.core.turn_number));
    let delta = StateDelta::between_structural_only(
        &before,
        &after,
        vec![object_entered(ZoneKind::Battlefield)],
    )
    .expect("an entering permanent is recorded with the current turn");
    assert_eq!(delta.apply_structural_only(&before).unwrap(), after);
}

/// The zone transition of a spell leaving the stack: object 9 becomes
/// object 3 in `to_zone`.
fn spell_zone_transition(to_zone: ZoneKind) -> SemanticDeltaOperation {
    let snapshot = |object: u64, zone: ZoneKind| ObjectSnapshot {
        object: GameObjectId(object),
        physical_card: Some(PhysicalCardId(3)),
        card_definition: CardDefinitionId(1),
        owner: PlayerId(1),
        controller: PlayerId(1),
        tapped: false,
        face_down: false,
        location: ZoneLocation {
            zone,
            ..public_location()
        },
    };
    SemanticDeltaOperation::ZoneTransition {
        transition: Box::new(ZoneTransition {
            old_object: GameObjectId(9),
            new_object: GameObjectId(3),
            physical_card: Some(PhysicalCardId(3)),
            from: snapshot(9, ZoneKind::Stack).location,
            to: snapshot(3, to_zone).location,
            last_known: snapshot(9, ZoneKind::Stack),
            new_snapshot: snapshot(3, to_zone),
        }),
    }
}

#[test]
fn delta_accepts_an_entry_made_by_a_zone_transition_into_the_battlefield() {
    let before = state_with_content_authority();
    let after = with_object_on_the_battlefield(&before, Some(before.core.turn_number));
    // The new object also needs its face covered, which is not under test.
    let face = SemanticDeltaOperation::ObjectFaceChanged {
        object: GameObjectId(3),
        from_face: 0,
        to_face: 0,
    };
    StateDelta::between_structural_only(
        &before,
        &after,
        vec![spell_zone_transition(ZoneKind::Battlefield), face.clone()],
    )
    .expect("a resolving permanent spell enters the battlefield by a zone transition");
    // A transition into any other zone does not make an entry.
    assert_eq!(
        StateDelta::between_structural_only(
            &before,
            &after,
            vec![spell_zone_transition(ZoneKind::Graveyard), face]
        )
        .unwrap_err(),
        DeltaApplicationError::UncoveredMutation
    );
}

#[test]
fn delta_rejects_an_entry_without_the_object_entering_the_battlefield() {
    let before = state_with_content_authority();
    let after = with_object_on_the_battlefield(&before, Some(before.core.turn_number));
    assert_eq!(
        StateDelta::between_structural_only(&before, &after, vec![object_entered(ZoneKind::Hand)])
            .unwrap_err(),
        DeltaApplicationError::UncoveredMutation
    );
}

#[test]
fn delta_rejects_an_entry_for_another_turn() {
    let before = state_with_content_authority();
    // Turn 0 is not later than the current turn, so the state is valid, but
    // the object entered during the current turn.
    let after = with_object_on_the_battlefield(&before, Some(before.core.turn_number - 1));
    assert_eq!(
        StateDelta::between_structural_only(
            &before,
            &after,
            vec![object_entered(ZoneKind::Battlefield)]
        )
        .unwrap_err(),
        DeltaApplicationError::UncoveredMutation
    );
}

#[test]
fn delta_rejects_a_changed_entry_while_the_object_stays() {
    let before = state_with_content_authority();
    let mut after = before.clone();
    after.revision = StateRevision(before.revision.0 + 1);
    after
        .card_rules
        .permanents
        .permanents
        .get_mut(&GameObjectId(1))
        .unwrap()
        .controlled_since_turn = 0;
    assert_eq!(
        StateDelta::between_structural_only(&before, &after, Vec::new()).unwrap_err(),
        DeltaApplicationError::UncoveredMutation
    );
}

#[test]
fn delta_rejects_a_removed_entry_while_the_object_stays() {
    // Without card-rules authority the state check does not see the missing
    // entry; the delta must.
    let mut before = synthetic_state();
    before
        .card_rules
        .permanents
        .enter(GameObjectId(1), 1)
        .unwrap();
    let mut after = before.clone();
    after.revision = StateRevision(before.revision.0 + 1);
    after.card_rules.permanents = PermanentsState::default();
    assert_eq!(
        StateDelta::between_structural_only(&before, &after, Vec::new()).unwrap_err(),
        DeltaApplicationError::UncoveredMutation
    );
}

/// `before` one revision later with `damage` more damage marked on the
/// battlefield object 1.
fn with_damage_marked(before: &EngineState, damage: u64) -> EngineState {
    let mut after = before.clone();
    after.revision = StateRevision(before.revision.0 + 1);
    after
        .card_rules
        .permanents
        .mark_damage(GameObjectId(1), damage)
        .unwrap();
    after
}

fn marked_damage_changed(creature: u64, from: u64, to: u64) -> SemanticDeltaOperation {
    SemanticDeltaOperation::MarkedDamageChanged {
        creature: GameObjectId(creature),
        from,
        to,
    }
}

#[test]
fn marking_damage_adds_to_the_damage_marked_and_fails_without_a_permanent() {
    let mut permanents = PermanentsState::default();
    permanents.enter(GameObjectId(7), 3).unwrap();
    assert_eq!(permanents.mark_damage(GameObjectId(7), 2), Ok((0, 2)));
    assert_eq!(permanents.mark_damage(GameObjectId(7), 3), Ok((2, 5)));
    assert_eq!(permanents.permanents[&GameObjectId(7)].marked_damage, 5);
    assert_eq!(
        permanents.permanents[&GameObjectId(7)].controlled_since_turn,
        3
    );
    // A failed mark leaves the record unchanged.
    assert_eq!(
        permanents.mark_damage(GameObjectId(7), u64::MAX),
        Err(StateFamilyMutationError::Overflow)
    );
    assert_eq!(
        permanents.mark_damage(GameObjectId(8), 1),
        Err(StateFamilyMutationError::UnknownObject)
    );
    assert_eq!(permanents.permanents[&GameObjectId(7)].marked_damage, 5);
}

#[test]
fn delta_accepts_marked_damage_with_its_event() {
    let before = state_with_content_authority();
    let after = with_damage_marked(&before, 2);
    let delta = StateDelta::between_structural_only(
        &before,
        &after,
        vec![marked_damage_changed(1, 0, 2)],
    )
    .expect("damage is marked by a MarkedDamageChanged that names both values");
    assert_eq!(delta.apply_structural_only(&before).unwrap(), after);

    // More damage later is another change, from the damage marked before.
    let more = with_damage_marked(&after, 1);
    StateDelta::between_structural_only(&after, &more, vec![marked_damage_changed(1, 2, 3)])
        .unwrap();
}

#[test]
fn delta_rejects_marked_damage_without_the_event() {
    let before = state_with_content_authority();
    let after = with_damage_marked(&before, 2);
    assert_eq!(
        StateDelta::between_structural_only(&before, &after, Vec::new()).unwrap_err(),
        DeltaApplicationError::UncoveredMutation
    );
}

#[test]
fn delta_rejects_marked_damage_that_its_event_does_not_name() {
    let before = state_with_content_authority();
    let after = with_damage_marked(&before, 2);
    for (what, operation) in [
        ("another creature", marked_damage_changed(2, 0, 2)),
        ("another start", marked_damage_changed(1, 1, 2)),
        ("another end", marked_damage_changed(1, 0, 3)),
    ] {
        assert_eq!(
            StateDelta::between_structural_only(&before, &after, vec![operation]).unwrap_err(),
            DeltaApplicationError::UncoveredMutation,
            "an event for {what}"
        );
    }
}

#[test]
fn delta_rejects_a_permanent_that_enters_damaged() {
    // CR 400.7: an object that enters is a new object, with no damage marked.
    let before = state_with_content_authority();
    let mut after = with_object_on_the_battlefield(&before, Some(before.core.turn_number));
    after
        .card_rules
        .permanents
        .mark_damage(GameObjectId(3), 1)
        .unwrap();
    assert_eq!(
        StateDelta::between_structural_only(
            &before,
            &after,
            vec![object_entered(ZoneKind::Battlefield)]
        )
        .unwrap_err(),
        DeltaApplicationError::UncoveredMutation
    );
}
