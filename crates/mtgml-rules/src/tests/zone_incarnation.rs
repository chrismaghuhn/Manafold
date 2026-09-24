use crate::zone_incarnation::{
    apply_selected_zone_transition_in_workspace, execute_selected_zone_transition,
    SelectedZoneTransitionKind, SelectedZoneTransitionRequest,
};
use mtgml_model::{CardDefinitionId, PhysicalCardId, RuleEventId, ZoneKind};
use mtgml_state::{
    construct_synthetic_engine_state, EngineState, GameObject, VisibilityPartition, ZoneKey,
    ZoneLocation, ZonePosition,
};

fn s2_base() -> EngineState {
    let mut state = construct_synthetic_engine_state(SyntheticResetInputs {
        players: [PlayerId(1), PlayerId(2)],
        root_seed: RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap(),
        setup: SyntheticV4Setup::synthetic_compatibility(),
    })
    .unwrap();
    state.execution.pending_decision = None;
    state.execution.continuations.clear();
    state
}

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

fn battlefield_graveyard_request(object: GameObjectId) -> SelectedZoneTransitionRequest {
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
            Some(PlayerId(1)),
            ZonePosition::Top { offset: 0 },
            VisibilityPartition::Public,
        ),
    }
}

fn battlefield_state_with_existing_graveyard() -> EngineState {
    let mut state = s2_base();
    let key = ZoneKey {
        zone: ZoneKind::Graveyard,
        player: Some(PlayerId(1)),
        visibility: VisibilityPartition::Public,
        partition: None,
    };
    for (id, offset) in [(GameObjectId(3), 0), (GameObjectId(4), 1)] {
        state.zones.objects.insert(
            id,
            GameObject {
                id,
                physical_card: Some(PhysicalCardId(id.0)),
                card_definition: CardDefinitionId(id.0),
                owner: PlayerId(1),
                controller: PlayerId(1),
                tapped: false,
                face_down: false,
            },
        );
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
    state.zones.ordered_zones.insert(key, vec![GameObjectId(3), GameObjectId(4)]);
    state.allocators.next_object_id = GameObjectId(5);
    mtgml_state::validate_engine_state(&state).unwrap();
    state
}

#[test]
fn standalone_s2_battlefield_move_pins_revision_events_delta_digest_and_visibility() {
    let before = battlefield_state_with_existing_graveyard();
    let result = execute_selected_zone_transition(
        &before,
        &battlefield_graveyard_request(GameObjectId(1)),
    )
    .unwrap();
    let new_object = GameObjectId(5);
    assert_eq!(result.next_state.revision, StateRevision(before.revision.0 + 1));
    assert_eq!(result.next_state.allocators.next_object_id, GameObjectId(6));
    assert_eq!(
        result.next_state.zones.ordered_zones[&ZoneKey {
            zone: ZoneKind::Graveyard,
            player: Some(PlayerId(1)),
            visibility: VisibilityPartition::Public,
            partition: None,
        }],
        vec![new_object, GameObjectId(3), GameObjectId(4)]
    );
    assert_eq!(
        result.next_state.zones.locations[&GameObjectId(3)].position,
        ZonePosition::Top { offset: 1 }
    );
    assert_eq!(
        result.next_state.zones.locations[&GameObjectId(4)].position,
        ZonePosition::Top { offset: 2 }
    );
    assert_eq!(result.events.len(), 3);
    assert!(matches!(
        result.events[0].event,
        AuthoritativeRuleEventKind::ZoneTransition { .. }
    ));
    assert!(matches!(
        result.events[1].event,
        AuthoritativeRuleEventKind::PerspectiveOccurrence { ref lifecycle, .. }
            if lifecycle.perspective == PlayerId(1)
    ));
    assert!(matches!(
        result.events[2].event,
        AuthoritativeRuleEventKind::PerspectiveOccurrence { ref lifecycle, .. }
            if lifecycle.perspective == PlayerId(2)
    ));
    for (offset, event) in result.events.iter().enumerate() {
        assert_eq!(event.event_id, RuleEventId(before.allocators.next_rule_event_id.0 + offset as u64));
        assert_eq!(event.state_revision, result.next_state.revision);
    }
    for player in [PlayerId(1), PlayerId(2)] {
        assert_eq!(
            result.next_state.knowledge.players[&player].next_visible_sequence.0,
            before.knowledge.players[&player].next_visible_sequence.0 + 1
        );
    }
    assert_eq!(
        result.delta.apply(&before).unwrap(),
        result.next_state,
        "the standalone S2 delta remains the complete exact replacement"
    );
    assert_eq!(result.next_state.digest().unwrap(), result.delta.after_digest);
    assert_eq!(result.next_state.zones.objects[&new_object].physical_card, Some(PhysicalCardId(1)));
    assert_eq!(before.zones.objects[&GameObjectId(1)].physical_card, Some(PhysicalCardId(1)));
}

fn library_top_state_with_second_card() -> EngineState {
    let mut state = s2_base();
    let old = state.zones.objects.get_mut(&GameObjectId(2)).unwrap();
    old.face_down = false;
    let next = GameObjectId(3);
    state.zones.objects.insert(
        next,
        GameObject {
            id: next,
            physical_card: Some(PhysicalCardId(3)),
            card_definition: CardDefinitionId(3),
            owner: PlayerId(2),
            controller: PlayerId(2),
            tapped: false,
            face_down: false,
        },
    );
    state.zones.locations.insert(
        next,
        zone_location(
            ZoneKind::Library,
            Some(PlayerId(2)),
            ZonePosition::Top { offset: 1 },
            VisibilityPartition::FaceDown,
        ),
    );
    let key = ZoneKey {
        zone: ZoneKind::Library,
        player: Some(PlayerId(2)),
        visibility: VisibilityPartition::FaceDown,
        partition: None,
    };
    state.zones.ordered_zones.get_mut(&key).unwrap().push(next);
    state.allocators.next_object_id = GameObjectId(4);
    mtgml_state::validate_engine_state(&state).unwrap();
    state
}

#[test]
fn standalone_s2_library_move_pins_top_consumption_and_private_occurrence() {
    let before = library_top_state_with_second_card();
    let request = SelectedZoneTransitionRequest {
        object: GameObjectId(2),
        kind: SelectedZoneTransitionKind::LibraryTopToOwnerHand,
        claimed_from: zone_location(
            ZoneKind::Library,
            Some(PlayerId(2)),
            ZonePosition::Top { offset: 0 },
            VisibilityPartition::FaceDown,
        ),
        claimed_to: zone_location(
            ZoneKind::Hand,
            Some(PlayerId(2)),
            ZonePosition::Unordered,
            VisibilityPartition::OwnerOnly,
        ),
    };
    let result = execute_selected_zone_transition(&before, &request).unwrap();
    assert_eq!(result.next_state.revision, StateRevision(1));
    assert_eq!(result.next_state.zones.ordered_zones[&request.claimed_from.key()], vec![GameObjectId(3)]);
    assert_eq!(result.next_state.zones.locations[&GameObjectId(3)].position, ZonePosition::Top { offset: 0 });
    assert_eq!(result.events.len(), 2);
    assert!(matches!(result.events[0].event, AuthoritativeRuleEventKind::ZoneTransition { .. }));
    assert!(matches!(
        result.events[1].event,
        AuthoritativeRuleEventKind::PerspectiveOccurrence { ref lifecycle, .. }
            if lifecycle.perspective == PlayerId(2)
    ));
    assert_eq!(result.events[0].event_id, before.allocators.next_rule_event_id);
    assert_eq!(result.events[1].event_id.0, before.allocators.next_rule_event_id.0 + 1);
    assert_eq!(result.next_state.knowledge.players[&PlayerId(2)].next_visible_sequence.0,
        before.knowledge.players[&PlayerId(2)].next_visible_sequence.0 + 1);
    assert_eq!(result.delta.apply(&before).unwrap(), result.next_state);
    assert_eq!(result.next_state.digest().unwrap(), result.delta.after_digest);
    assert_eq!(result.next_state.zones.objects[&GameObjectId(4)].physical_card, Some(PhysicalCardId(2)));
}

fn remove_object_tracking(state: &mut EngineState, object: GameObjectId) {
    for (player, identity) in &mut state.perspective_identities.players {
        if let Some(opaque) = identity.object_to_opaque.remove(&object) {
            identity.opaque_to_object.remove(&opaque);
            state
                .knowledge
                .players
                .get_mut(player)
                .unwrap()
                .active
                .remove(&opaque);
        }
    }
}

#[test]
fn s2_workspace_composes_two_moves_at_one_revision_and_reverse_inserts_order() {
    let mut state = battlefield_state_with_existing_graveyard();
    remove_object_tracking(&mut state, GameObjectId(1));
    let second_old = GameObjectId(5);
    state.zones.objects.insert(
        second_old,
        GameObject {
            id: second_old,
            physical_card: Some(PhysicalCardId(5)),
            card_definition: CardDefinitionId(5),
            owner: PlayerId(1),
            controller: PlayerId(1),
            tapped: false,
            face_down: false,
        },
    );
    state.zones.locations.insert(
        second_old,
        zone_location(
            ZoneKind::Battlefield,
            None,
            ZonePosition::Unordered,
            VisibilityPartition::Public,
        ),
    );
    state.allocators.next_object_id = GameObjectId(6);
    mtgml_state::validate_engine_state(&state).unwrap();

    let authoritative_before = state.clone();
    let mut scratch = state.clone();
    scratch.revision = StateRevision(state.revision.0 + 1);
    let outer_event_origin = state.allocators.next_rule_event_id;
    let mut events = Vec::new();

    // Desired final top-to-bottom is NEW(1), NEW(5), OLD(3), OLD(4).
    // S2 inserts at top, so invoke OLD(5) first and OLD(1) second.
    let second_transition = apply_selected_zone_transition_in_workspace(
        &mut scratch,
        &battlefield_graveyard_request(second_old),
        outer_event_origin,
        &mut events,
    )
    .unwrap();
    let first_transition = apply_selected_zone_transition_in_workspace(
        &mut scratch,
        &battlefield_graveyard_request(GameObjectId(1)),
        outer_event_origin,
        &mut events,
    )
    .unwrap();

    assert_eq!(second_transition.new_object, GameObjectId(6));
    assert_eq!(first_transition.new_object, GameObjectId(7));
    assert_eq!(scratch.revision, StateRevision(1));
    assert_eq!(scratch.allocators.next_object_id, GameObjectId(8));
    assert_eq!(
        scratch.zones.ordered_zones[&ZoneKey {
            zone: ZoneKind::Graveyard,
            player: Some(PlayerId(1)),
            visibility: VisibilityPartition::Public,
            partition: None,
        }],
        vec![GameObjectId(7), GameObjectId(6), GameObjectId(3), GameObjectId(4)]
    );
    assert_eq!(events.len(), 2);
    for (offset, event) in events.iter().enumerate() {
        assert_eq!(event.state_revision, StateRevision(1));
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
        &battlefield_graveyard_request(GameObjectId(999)),
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

#[test]
fn multiple_s2_moves_without_an_sba_batch_are_rejected_by_the_outer_contract() {
    let mut state = battlefield_state_with_existing_graveyard();
    remove_object_tracking(&mut state, GameObjectId(1));
    let second_old = GameObjectId(5);
    state.zones.objects.insert(
        second_old,
        GameObject {
            id: second_old,
            physical_card: Some(PhysicalCardId(5)),
            card_definition: CardDefinitionId(5),
            owner: PlayerId(1),
            controller: PlayerId(1),
            tapped: false,
            face_down: false,
        },
    );
    state.zones.locations.insert(
        second_old,
        zone_location(
            ZoneKind::Battlefield,
            None,
            ZonePosition::Unordered,
            VisibilityPartition::Public,
        ),
    );
    state.allocators.next_object_id = GameObjectId(6);
    mtgml_state::validate_engine_state(&state).unwrap();

    let mut scratch = state.clone();
    scratch.revision = StateRevision(1);
    let mut events = Vec::new();
    let event_origin = state.allocators.next_rule_event_id;
    apply_selected_zone_transition_in_workspace(
        &mut scratch,
        &battlefield_graveyard_request(second_old),
        event_origin,
        &mut events,
    )
    .unwrap();
    apply_selected_zone_transition_in_workspace(
        &mut scratch,
        &battlefield_graveyard_request(GameObjectId(1)),
        event_origin,
        &mut events,
    )
    .unwrap();

    assert!(matches!(
        crate::product::build_accepted_product(&state, scratch, events, |_| Ok(())),
        Err(KernelExecutionError::TransitionContract(
            crate::TransitionViolation::ZoneTransition
        ))
    ));
}
