use crate::zone_incarnation::{
    apply_selected_zone_transition_in_workspace, SelectedZoneTransitionKind,
    SelectedZoneTransitionRequest,
};
use mtgml_model::{CardDefinitionId, PhysicalCardId, RuleEventId, ZoneKind};
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
    let outer_event_origin = state.allocators.next_rule_event_id;
    let mut events = Vec::new();

    // Each move inserts at the top: moving 6 and then 5 leaves the new
    // incarnation of 5 on top.
    let first_transition = apply_selected_zone_transition_in_workspace(
        &mut scratch,
        &hand_to_graveyard(GameObjectId(6)),
        outer_event_origin,
        &mut events,
    )
    .unwrap();
    let second_transition = apply_selected_zone_transition_in_workspace(
        &mut scratch,
        &hand_to_graveyard(GameObjectId(5)),
        outer_event_origin,
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
    assert!(!events.is_empty());
    for (offset, event) in events.iter().enumerate() {
        assert_eq!(event.state_revision, scratch.revision);
        assert_eq!(
            event.event_id,
            RuleEventId(outer_event_origin.0 + offset as u64)
        );
    }
    mtgml_state::validate_engine_state(&scratch).unwrap();
    assert_eq!(state, authoritative_before);

    let scratch_before_failure = scratch.clone();
    let events_before_failure = events.clone();
    let failure = apply_selected_zone_transition_in_workspace(
        &mut scratch,
        &hand_to_graveyard(GameObjectId(999)),
        outer_event_origin,
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
