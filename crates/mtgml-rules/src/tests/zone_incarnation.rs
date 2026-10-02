use crate::zone_incarnation::{
    apply_selected_zone_transition_in_workspace, SelectedZoneTransitionKind,
    SelectedZoneTransitionRequest,
};
use mtgml_model::{CardDefinitionId, PhysicalCardId, ZoneKind};
use mtgml_state::{
    construct_synthetic_engine_state, EngineState, GameObject, VisibilityPartition, ZoneKey,
    ZoneLocation, ZonePosition,
};

fn zone_location(
    zone: ZoneKind,
    player: Option<PlayerId>,
    position: ZonePosition,
    visibility: VisibilityPartition,
) -> ZoneLocation {
    ZoneLocation {
        zone,
        player,
        position,
        visibility,
        partition: None,
    }
}

fn owned_card(id: GameObjectId) -> GameObject {
    GameObject {
        id,
        physical_card: Some(PhysicalCardId(id.0)),
        card_definition: CardDefinitionId(id.0),
        owner: PlayerId(1),
        controller: PlayerId(1),
        tapped: false,
        face_down: false,
    }
}

fn graveyard_key() -> ZoneKey {
    ZoneKey {
        zone: ZoneKind::Graveyard,
        player: Some(PlayerId(1)),
        visibility: VisibilityPartition::Public,
        partition: None,
    }
}

/// P1 holds cards 5 and 6 in hand and has cards 3 and 4 in its graveyard.
/// No perspective tracks these four cards.
fn hand_state_with_existing_graveyard() -> EngineState {
    let mut state = construct_synthetic_engine_state(SyntheticResetInputs {
        players: [PlayerId(1), PlayerId(2)],
        root_seed: RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap(),
        setup: SyntheticV4Setup::synthetic_compatibility(),
    })
    .unwrap();
    state.execution.pending_decision = None;
    state.execution.continuations.clear();
    for (id, offset) in [(GameObjectId(3), 0), (GameObjectId(4), 1)] {
        state.zones.objects.insert(id, owned_card(id));
        state.zones.locations.insert(
            id,
            zone_location(
                ZoneKind::Graveyard,
                Some(PlayerId(1)),
                ZonePosition::Top { offset },
                VisibilityPartition::Public,
            ),
        );
    }
    state
        .zones
        .ordered_zones
        .insert(graveyard_key(), vec![GameObjectId(3), GameObjectId(4)]);
    for id in [GameObjectId(5), GameObjectId(6)] {
        state.zones.objects.insert(id, owned_card(id));
        state.zones.locations.insert(
            id,
            zone_location(
                ZoneKind::Hand,
                Some(PlayerId(1)),
                ZonePosition::Unordered,
                VisibilityPartition::OwnerOnly,
            ),
        );
    }
    state.allocators.next_object_id = GameObjectId(7);
    mtgml_state::validate_engine_state(&state).unwrap();
    state
}

fn hand_to_graveyard(object: GameObjectId) -> SelectedZoneTransitionRequest {
    SelectedZoneTransitionRequest {
        object,
        kind: SelectedZoneTransitionKind::HandToOwnerGraveyard,
        claimed_from: zone_location(
            ZoneKind::Hand,
            Some(PlayerId(1)),
            ZonePosition::Unordered,
            VisibilityPartition::OwnerOnly,
        ),
        claimed_to: zone_location(
            ZoneKind::Graveyard,
            Some(PlayerId(1)),
            ZonePosition::Top { offset: 0 },
            VisibilityPartition::Public,
        ),
    }
}

#[test]
fn workspace_composes_two_moves_at_one_revision_and_reverse_inserts_order() {
    let state = hand_state_with_existing_graveyard();
    let authoritative_before = state.clone();
    let mut scratch = state.clone();
    scratch.revision = StateRevision(state.revision.0 + 1);
    let mut events = Vec::new();

    // Each move inserts at the top: moving 6 and then 5 leaves the new
    // incarnation of 5 on top.
    let first_transition = apply_selected_zone_transition_in_workspace(
        &mut scratch,
        &hand_to_graveyard(GameObjectId(6)),
        &mut events,
    )
    .unwrap();
    let second_transition = apply_selected_zone_transition_in_workspace(
        &mut scratch,
        &hand_to_graveyard(GameObjectId(5)),
        &mut events,
    )
    .unwrap();

    assert_eq!(first_transition.new_object, GameObjectId(7));
    assert_eq!(second_transition.new_object, GameObjectId(8));
    assert_eq!(scratch.revision, StateRevision(state.revision.0 + 1));
    assert_eq!(scratch.allocators.next_object_id, GameObjectId(9));
    assert_eq!(
        scratch.zones.ordered_zones[&graveyard_key()],
        vec![
            GameObjectId(8),
            GameObjectId(7),
            GameObjectId(3),
            GameObjectId(4)
        ]
    );
    // Each move emits its transition, then one occurrence per perspective.
    assert!(matches!(
        events.first(),
        Some(crate::zone_incarnation::ZoneMoveEvent::Transition(transition))
            if transition.new_object == GameObjectId(7)
    ));
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, crate::zone_incarnation::ZoneMoveEvent::Transition(_)))
            .count(),
        2
    );
    mtgml_state::validate_engine_state(&scratch).unwrap();
    assert_eq!(state, authoritative_before);

    let scratch_before_failure = scratch.clone();
    let events_before_failure = events.clone();
    let failure = apply_selected_zone_transition_in_workspace(
        &mut scratch,
        &hand_to_graveyard(GameObjectId(999)),
        &mut events,
    )
    .unwrap_err();
    assert!(matches!(
        failure,
        KernelExecutionError::ZoneIncarnation(ZoneIncarnationError::ObjectNotLive)
    ));
    assert_eq!(scratch, scratch_before_failure);
    assert_eq!(events, events_before_failure);
    assert_eq!(state, authoritative_before);
}

/// As `hand_state_with_existing_graveyard`, with P1's tapped card 7 on the
/// battlefield, where a game puts permanents: the one unordered public zone.
fn state_with_a_permanent() -> EngineState {
    let mut state = hand_state_with_existing_graveyard();
    state.zones.objects.insert(
        GameObjectId(7),
        GameObject {
            tapped: true,
            ..owned_card(GameObjectId(7))
        },
    );
    state.zones.locations.insert(
        GameObjectId(7),
        zone_location(
            ZoneKind::Battlefield,
            None,
            ZonePosition::Unordered,
            VisibilityPartition::Public,
        ),
    );
    state.allocators.next_object_id = GameObjectId(8);
    mtgml_state::validate_engine_state(&state).unwrap();
    state
}

fn battlefield_to_graveyard(
    object: GameObjectId,
    graveyard_of: PlayerId,
) -> SelectedZoneTransitionRequest {
    SelectedZoneTransitionRequest {
        object,
        kind: SelectedZoneTransitionKind::BattlefieldToOwnerGraveyard,
        claimed_from: zone_location(
            ZoneKind::Battlefield,
            None,
            ZonePosition::Unordered,
            VisibilityPartition::Public,
        ),
        claimed_to: zone_location(
            ZoneKind::Graveyard,
            Some(graveyard_of),
            ZonePosition::Top { offset: 0 },
            VisibilityPartition::Public,
        ),
    }
}

#[test]
fn a_permanent_goes_to_the_top_of_its_owners_graveyard_as_a_new_untapped_object() {
    // CR 400.7: the permanent that leaves the battlefield becomes a new
    // object, with no memory of its previous existence: it is not tapped.
    let state = state_with_a_permanent();
    let mut scratch = state.clone();
    scratch.revision = StateRevision(state.revision.0 + 1);
    let mut events = Vec::new();

    let transition = apply_selected_zone_transition_in_workspace(
        &mut scratch,
        &battlefield_to_graveyard(GameObjectId(7), PlayerId(1)),
        &mut events,
    )
    .unwrap();

    assert_eq!(transition.old_object, GameObjectId(7));
    assert_eq!(transition.new_object, GameObjectId(8));
    assert_eq!(transition.from.zone, ZoneKind::Battlefield);
    assert_eq!(transition.last_known.object, GameObjectId(7));
    assert!(transition.last_known.tapped && !transition.new_snapshot.tapped);
    assert_eq!(transition.physical_card, Some(PhysicalCardId(7)));
    assert!(!scratch.zones.objects.contains_key(&GameObjectId(7)));
    assert!(!scratch.zones.objects[&GameObjectId(8)].tapped);
    assert_eq!(
        scratch.zones.objects[&GameObjectId(8)].controller,
        PlayerId(1)
    );
    assert_eq!(
        scratch.zones.ordered_zones[&graveyard_key()],
        vec![GameObjectId(8), GameObjectId(3), GameObjectId(4)]
    );
    // The transition, then each perspective's occurrence of it.
    assert_eq!(events.len(), 3);
    assert!(matches!(
        events.first(),
        Some(crate::zone_incarnation::ZoneMoveEvent::Transition(_))
    ));
    mtgml_state::validate_engine_state(&scratch).unwrap();
}

#[test]
fn a_move_from_the_battlefield_is_refused_unless_it_is_one() {
    let state = state_with_a_permanent();
    let mut scratch = state.clone();
    scratch.revision = StateRevision(state.revision.0 + 1);
    let mut events = Vec::new();
    let mut refused = |scratch: &mut EngineState, request: SelectedZoneTransitionRequest| {
        let before = (scratch.clone(), events.clone());
        let failure = apply_selected_zone_transition_in_workspace(scratch, &request, &mut events)
            .unwrap_err();
        // A refused move changes nothing.
        assert_eq!((scratch.clone(), events.clone()), before);
        failure
    };

    // The graveyard is the owner's.
    assert!(matches!(
        refused(
            &mut scratch,
            battlefield_to_graveyard(GameObjectId(7), PlayerId(2))
        ),
        KernelExecutionError::ZoneIncarnation(ZoneIncarnationError::DestinationMismatch)
    ));
    // The source is the battlefield: not a hand card (the claim is right)...
    assert!(matches!(
        refused(
            &mut scratch,
            SelectedZoneTransitionRequest {
                kind: SelectedZoneTransitionKind::BattlefieldToOwnerGraveyard,
                ..hand_to_graveyard(GameObjectId(5))
            }
        ),
        KernelExecutionError::ZoneIncarnation(ZoneIncarnationError::UnadmittedSourceFamily)
    ));
    // ...and a hand move does not take a permanent.
    assert!(matches!(
        refused(
            &mut scratch,
            SelectedZoneTransitionRequest {
                object: GameObjectId(7),
                claimed_from: zone_location(
                    ZoneKind::Battlefield,
                    None,
                    ZonePosition::Unordered,
                    VisibilityPartition::Public,
                ),
                ..hand_to_graveyard(GameObjectId(7))
            }
        ),
        KernelExecutionError::ZoneIncarnation(ZoneIncarnationError::UnadmittedSourceFamily)
    ));
    // A battlefield that belongs to a player is not the one a game has.
    let mut elsewhere = scratch.clone();
    let location = elsewhere.zones.locations.get_mut(&GameObjectId(7)).unwrap();
    location.player = Some(PlayerId(1));
    let request = SelectedZoneTransitionRequest {
        claimed_from: location.clone(),
        ..battlefield_to_graveyard(GameObjectId(7), PlayerId(1))
    };
    assert!(matches!(
        refused(&mut elsewhere, request),
        KernelExecutionError::ZoneIncarnation(ZoneIncarnationError::UnadmittedSourceFamily)
    ));
    // A permanent in combat is removed from it first (CR 506.4).
    let mut attacking = scratch.clone();
    attacking.combat = Some(mtgml_state::CombatState {
        defending_player: PlayerId(2),
        attackers: vec![GameObjectId(7)],
        damage_step_completed: false,
        blocked_attackers: Default::default(),
        blockers: Default::default(),
    });
    assert!(matches!(
        refused(
            &mut attacking,
            battlefield_to_graveyard(GameObjectId(7), PlayerId(1))
        ),
        KernelExecutionError::ZoneIncarnation(ZoneIncarnationError::CombatReference)
    ));
    // A graveyard with a face-down card is not one this move can order.
    let mut face_down = scratch.clone();
    face_down
        .zones
        .objects
        .get_mut(&GameObjectId(3))
        .unwrap()
        .face_down = true;
    assert!(matches!(
        refused(
            &mut face_down,
            battlefield_to_graveyard(GameObjectId(7), PlayerId(1))
        ),
        KernelExecutionError::ZoneIncarnation(ZoneIncarnationError::UnsupportedSourceProfile)
    ));
}
