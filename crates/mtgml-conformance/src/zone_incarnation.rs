//! M3.S2 direct-request RED cases and substrate characterization.
//!
//! Positive cases deliberately stop at the private production-owned seam.
//! Their independently authored expected products remain the conformance
//! oracle; production output never supplies expected semantic values.

use mtgml_model::{CardDefinitionId, GameObjectId, PhysicalCardId, PlayerId, ZoneKind};
use mtgml_rules::{
    execute_selected_zone_transition_for_conformance, ConformanceZoneTransitionKind,
    KernelExecutionError, ZoneIncarnationError,
};
use mtgml_state::{
    construct_synthetic_engine_state, validate_engine_state, BaseCharacteristics, ControlHistory,
    EngineState, FoundationCreatureSource, FoundationSourceKind, GameObject, ObjectSnapshot,
    SyntheticResetInputs, SyntheticV4Setup, VisibilityPartition, ZoneKey, ZoneLocation,
    ZonePosition,
};
use std::collections::BTreeMap;

const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);
const OLD_BATTLEFIELD: GameObjectId = GameObjectId(1);
const OLD_LIBRARY_TOP: GameObjectId = GameObjectId(2);
const CARD_BATTLEFIELD: PhysicalCardId = PhysicalCardId(1);
const CARD_LIBRARY: PhysicalCardId = PhysicalCardId(2);

fn base_state() -> EngineState {
    let mut state = construct_synthetic_engine_state(SyntheticResetInputs {
        players: [P1, P2],
        root_seed: mtgml_random::RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap(),
        setup: SyntheticV4Setup::m2_compatibility(),
    })
    .unwrap();
    // These direct primitive cases do not carry or fabricate a player response.
    state.execution.pending_decision = None;
    state.execution.continuations.clear();
    state
}

fn location(
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

fn battlefield_from() -> ZoneLocation {
    location(
        ZoneKind::Battlefield,
        None,
        ZonePosition::Unordered,
        VisibilityPartition::Public,
    )
}

fn owner_graveyard_top(owner: PlayerId) -> ZoneLocation {
    location(
        ZoneKind::Graveyard,
        Some(owner),
        ZonePosition::Top { offset: 0 },
        VisibilityPartition::Public,
    )
}

fn owner_library_top(owner: PlayerId) -> ZoneLocation {
    location(
        ZoneKind::Library,
        Some(owner),
        ZonePosition::Top { offset: 0 },
        VisibilityPartition::FaceDown,
    )
}

fn owner_hand(owner: PlayerId) -> ZoneLocation {
    location(
        ZoneKind::Hand,
        Some(owner),
        ZonePosition::Unordered,
        VisibilityPartition::OwnerOnly,
    )
}

fn battlefield_case_state() -> EngineState {
    let mut state = base_state();
    let graveyard = location(
        ZoneKind::Graveyard,
        Some(P1),
        ZonePosition::Top { offset: 0 },
        VisibilityPartition::Public,
    );
    let second = location(
        ZoneKind::Graveyard,
        Some(P1),
        ZonePosition::Top { offset: 1 },
        VisibilityPartition::Public,
    );
    for (id, definition, physical, zone_location) in [
        (GameObjectId(3), 3, 3, graveyard.clone()),
        (GameObjectId(4), 4, 4, second),
    ] {
        state.zones.objects.insert(
            id,
            GameObject {
                id,
                physical_card: Some(PhysicalCardId(physical)),
                card_definition: CardDefinitionId(definition),
                owner: P1,
                controller: P1,
                tapped: false,
                face_down: false,
            },
        );
        state.zones.locations.insert(id, zone_location);
    }
    state.zones.ordered_zones.insert(
        ZoneKey {
            zone: ZoneKind::Graveyard,
            player: Some(P1),
            visibility: VisibilityPartition::Public,
            partition: None,
        },
        vec![GameObjectId(3), GameObjectId(4)],
    );
    state.allocators.next_object_id = GameObjectId(5);
    state.foundation_sources.insert(
        OLD_BATTLEFIELD,
        FoundationCreatureSource {
            source_kind: FoundationSourceKind::Creature,
            base_characteristics: BaseCharacteristics::Simple {
                power: 2,
                toughness: 2,
            },
            marked_damage: 1,
            control_history: ControlHistory::BeforeTurnStart { turn_number: 0 },
        },
    );
    validate_engine_state(&state).expect("authored battlefield case state is valid");
    state
}

fn library_case_state() -> EngineState {
    let mut state = base_state();
    // The zone represents hidden status; the card itself is not face-down.
    state
        .zones
        .objects
        .get_mut(&OLD_LIBRARY_TOP)
        .unwrap()
        .face_down = false;
    let second = GameObjectId(3);
    let library_top_one = location(
        ZoneKind::Library,
        Some(P2),
        ZonePosition::Top { offset: 1 },
        VisibilityPartition::FaceDown,
    );
    state.zones.objects.insert(
        second,
        GameObject {
            id: second,
            physical_card: Some(PhysicalCardId(3)),
            card_definition: CardDefinitionId(3),
            owner: P2,
            controller: P2,
            tapped: false,
            face_down: false,
        },
    );
    state.zones.locations.insert(second, library_top_one);
    state.zones.ordered_zones.insert(
        ZoneKey {
            zone: ZoneKind::Library,
            player: Some(P2),
            visibility: VisibilityPartition::FaceDown,
            partition: None,
        },
        vec![OLD_LIBRARY_TOP, second],
    );
    state.allocators.next_object_id = GameObjectId(4);
    validate_engine_state(&state).expect("authored library case state is valid");
    state
}

fn remove_object_tracking(state: &mut EngineState, object: GameObjectId) {
    for (player, identity) in &mut state.perspective_identities.players {
        if let Some(opaque) = identity.object_to_opaque.remove(&object) {
            identity.opaque_to_object.remove(&opaque);
            state
                .knowledge
                .players
                .get_mut(player)
                .expect("knowledge covers each player")
                .active
                .remove(&opaque);
        }
    }
}

/// Task-2-only positive fixture: source objects are intentionally untracked
/// and carry no FoundationSource so that Task-3 lifecycle/closure semantics
/// are not smuggled into this slice.
fn task2_battlefield_case_state() -> EngineState {
    let mut state = battlefield_case_state();
    remove_object_tracking(&mut state, OLD_BATTLEFIELD);
    state.foundation_sources.remove(&OLD_BATTLEFIELD);
    validate_engine_state(&state).expect("Task-2 battlefield state is valid");
    state
}

/// Task-2-only Library fixture excludes the owner mapping update; Task 3 owns
/// lifecycle integration and the corresponding player products.
fn task2_library_case_state() -> EngineState {
    let mut state = library_case_state();
    remove_object_tracking(&mut state, OLD_LIBRARY_TOP);
    validate_engine_state(&state).expect("Task-2 library state is valid");
    state
}

fn first_private_library_case_state() -> EngineState {
    let mut state = library_case_state();
    remove_object_tracking(&mut state, OLD_LIBRARY_TOP);
    validate_engine_state(&state).expect("first-private library state is valid");
    state
}

fn battlefield_request() -> Result<mtgml_rules::TransitionResult, KernelExecutionError> {
    execute_selected_zone_transition_for_conformance(
        &task2_battlefield_case_state(),
        OLD_BATTLEFIELD,
        battlefield_from(),
        owner_graveyard_top(P1),
        ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
    )
}

fn library_request() -> Result<mtgml_rules::TransitionResult, KernelExecutionError> {
    execute_selected_zone_transition_for_conformance(
        &task2_library_case_state(),
        OLD_LIBRARY_TOP,
        owner_library_top(P2),
        owner_hand(P2),
        ConformanceZoneTransitionKind::LibraryTopToOwnerHand,
    )
}

fn assert_unrelated_allocators_unchanged(
    before: &mtgml_state::IdentityAllocatorState,
    after: &mtgml_state::IdentityAllocatorState,
) {
    assert_eq!(after.next_ability_id, before.next_ability_id);
    assert_eq!(after.next_stack_object_id, before.next_stack_object_id);
    assert_eq!(after.next_effect_id, before.next_effect_id);
    assert_eq!(after.next_trigger_id, before.next_trigger_id);
    assert_eq!(after.next_decision_id, before.next_decision_id);
    assert_eq!(after.next_continuation_id, before.next_continuation_id);
}

fn assert_event_delta_mirror(result: &mtgml_rules::TransitionResult) {
    assert_eq!(
        result.delta.audit,
        result
            .events
            .iter()
            .map(|event| event.event.semantic_delta())
            .collect::<Vec<_>>()
    );
    for (index, event) in result.events.iter().enumerate() {
        assert_eq!(event.event_id.0, index as u64 + 1);
        assert_eq!(event.state_revision, result.next_state.revision);
    }
}

fn assert_rejected_request_preserves_complete_state<T>(
    before: &EngineState,
    result: Result<T, KernelExecutionError>,
    expected_error: impl FnOnce(&KernelExecutionError) -> bool,
) {
    let before_state = before.clone();
    let before_digest = before.digest().ok();
    let error = match result {
        Err(error) => error,
        Ok(_) => panic!("expected typed S2 request rejection"),
    };
    assert!(
        expected_error(&error),
        "unexpected typed rejection: {error:?}"
    );
    assert_eq!(*before, before_state);
    assert_eq!(before.digest().ok(), before_digest);
    assert_eq!(before.revision, before_state.revision);
    assert_eq!(before.zones, before_state.zones);
    assert_eq!(before.allocators, before_state.allocators);
    assert_eq!(before.combat, before_state.combat);
    assert_eq!(before.foundation_sources, before_state.foundation_sources);
    assert_eq!(before.execution, before_state.execution);
    assert_eq!(before.random, before_state.random);
    assert_eq!(before.knowledge, before_state.knowledge);
    assert_eq!(
        before.perspective_identities,
        before_state.perspective_identities
    );
    assert_eq!(before.format, before_state.format);
}

fn accepted_task2_battlefield_product() -> (EngineState, mtgml_rules::TransitionResult) {
    let before = task2_battlefield_case_state();
    let result = execute_selected_zone_transition_for_conformance(
        &before,
        OLD_BATTLEFIELD,
        battlefield_from(),
        owner_graveyard_top(P1),
        ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
    )
    .unwrap();
    (before, result)
}

fn accepted_task2_library_product_with_three_cards() -> (EngineState, mtgml_rules::TransitionResult)
{
    let mut before = task2_library_case_state();
    let third = GameObjectId(4);
    before.zones.objects.insert(
        third,
        GameObject {
            id: third,
            physical_card: Some(PhysicalCardId(4)),
            card_definition: CardDefinitionId(4),
            owner: P2,
            controller: P2,
            tapped: false,
            face_down: false,
        },
    );
    before.zones.locations.insert(
        third,
        location(
            ZoneKind::Library,
            Some(P2),
            ZonePosition::Top { offset: 2 },
            VisibilityPartition::FaceDown,
        ),
    );
    before
        .zones
        .ordered_zones
        .get_mut(&owner_library_top(P2).key())
        .unwrap()
        .push(third);
    before.allocators.next_object_id = GameObjectId(5);
    validate_engine_state(&before).unwrap();
    let result = execute_selected_zone_transition_for_conformance(
        &before,
        OLD_LIBRARY_TOP,
        owner_library_top(P2),
        owner_hand(P2),
        ConformanceZoneTransitionKind::LibraryTopToOwnerHand,
    )
    .unwrap();
    (before, result)
}

fn mutated_zone_transition(
    result: &mut mtgml_rules::TransitionResult,
) -> &mut mtgml_state::ZoneTransition {
    match &mut result.events[0].event {
        mtgml_rules::AuthoritativeRuleEventKind::ZoneTransition { transition } => transition,
        other => panic!("expected ZoneTransition first, got {other:?}"),
    }
}

fn rebuild_candidate_delta(before: &EngineState, result: &mut mtgml_rules::TransitionResult) {
    let audit = result
        .events
        .iter()
        .map(|event| event.event.semantic_delta())
        .collect();
    result.delta = mtgml_state::StateDelta::between(before, &result.next_state, audit).unwrap();
}

fn assert_transition_violation(
    before: &EngineState,
    result: &mtgml_rules::TransitionResult,
    expected: impl FnOnce(&mtgml_rules::TransitionViolation) -> bool,
) {
    let violation = mtgml_rules::validate_transition_contract(before, result)
        .expect_err("mutated accepted product must be rejected");
    assert!(
        expected(&violation),
        "unexpected validator violation: {violation:?}"
    );
}

#[test]
fn s2_zone_battlefield_graveyard() {
    let before = task2_battlefield_case_state();
    assert_eq!(before.allocators.next_object_id, GameObjectId(5));
    // Independent expected vector and redundant position witnesses:
    // [NEW(5), G0(3), G1(4)] at offsets 0, 1, 2.
    let expected_order = vec![GameObjectId(5), GameObjectId(3), GameObjectId(4)];
    let expected_positions = [
        (GameObjectId(5), ZonePosition::Top { offset: 0 }),
        (GameObjectId(3), ZonePosition::Top { offset: 1 }),
        (GameObjectId(4), ZonePosition::Top { offset: 2 }),
    ];
    assert_eq!(
        expected_order,
        [GameObjectId(5), GameObjectId(3), GameObjectId(4)]
    );
    assert_eq!(expected_positions[2].1, ZonePosition::Top { offset: 2 });
    let result = execute_selected_zone_transition_for_conformance(
        &before,
        OLD_BATTLEFIELD,
        battlefield_from(),
        owner_graveyard_top(P1),
        ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
    )
    .expect("s2.zone.battlefield_graveyard accepted core product");
    assert!(result.accepted);
    assert!(!result
        .next_state
        .zones
        .objects
        .contains_key(&OLD_BATTLEFIELD));
    assert!(!result
        .next_state
        .zones
        .locations
        .contains_key(&OLD_BATTLEFIELD));
    let key = owner_graveyard_top(P1).key();
    assert_eq!(result.next_state.zones.ordered_zones[&key], expected_order);
    for (object, expected_position) in expected_positions {
        assert_eq!(
            result.next_state.zones.locations[&object].position,
            expected_position
        );
    }
    let new = &result.next_state.zones.objects[&GameObjectId(5)];
    assert_eq!(new.physical_card, Some(CARD_BATTLEFIELD));
    assert_eq!(new.card_definition, CardDefinitionId(1));
    assert_eq!(new.owner, P1);
    assert_eq!(new.controller, P1);
    assert!(!new.tapped);
    assert!(!new.face_down);
    assert_eq!(result.next_state.allocators.next_object_id, GameObjectId(6));
    assert_eq!(result.events.len(), 1);
    assert_eq!(result.next_state.allocators.next_rule_event_id.0, 2);
    assert_unrelated_allocators_unchanged(&before.allocators, &result.next_state.allocators);
}

#[test]
fn s2_zone_library_hand_top() {
    let before = task2_library_case_state();
    assert_eq!(
        before.zones.ordered_zones.values().next().unwrap(),
        &[OLD_LIBRARY_TOP, GameObjectId(3)]
    );
    // Independent expected source remainder and destination identity.
    let expected_remaining_library = vec![GameObjectId(3)];
    let expected_new_object = GameObjectId(4);
    assert_eq!(expected_remaining_library, [GameObjectId(3)]);
    assert_eq!(expected_new_object, GameObjectId(4));
    let result = library_request().expect("s2.zone.library_hand_top accepted core product");
    assert!(result.accepted);
    assert!(!result
        .next_state
        .zones
        .objects
        .contains_key(&OLD_LIBRARY_TOP));
    assert!(!result
        .next_state
        .zones
        .locations
        .contains_key(&OLD_LIBRARY_TOP));
    let key = owner_library_top(P2).key();
    assert_eq!(
        result.next_state.zones.ordered_zones[&key],
        expected_remaining_library
    );
    assert_eq!(
        result.next_state.zones.locations[&GameObjectId(3)].position,
        ZonePosition::Top { offset: 0 }
    );
    let new = &result.next_state.zones.objects[&expected_new_object];
    assert_eq!(new.physical_card, Some(CARD_LIBRARY));
    assert_eq!(new.card_definition, CardDefinitionId(2));
    assert_eq!(new.owner, P2);
    assert_eq!(new.controller, P2);
    assert!(!new.tapped);
    assert!(!new.face_down);
    assert_eq!(
        result.next_state.zones.locations[&expected_new_object],
        owner_hand(P2)
    );
    assert_eq!(result.next_state.allocators.next_object_id, GameObjectId(5));
    assert_eq!(result.events.len(), 2);
    assert_eq!(
        result.next_state.allocators.next_rule_event_id,
        mtgml_model::RuleEventId(3)
    );
    assert_eq!(result.events[0].event_id, mtgml_model::RuleEventId(1));
    assert_eq!(result.events[1].event_id, mtgml_model::RuleEventId(2));
    assert_unrelated_allocators_unchanged(&before.allocators, &result.next_state.allocators);
    let old_snapshot = ObjectSnapshot {
        object: OLD_LIBRARY_TOP,
        physical_card: Some(CARD_LIBRARY),
        card_definition: CardDefinitionId(2),
        owner: P2,
        controller: P2,
        tapped: false,
        face_down: false,
        location: owner_library_top(P2),
    };
    let transition = match &result.events[0].event {
        mtgml_rules::AuthoritativeRuleEventKind::ZoneTransition { transition } => transition,
        other => panic!("unexpected first event: {other:?}"),
    };
    assert_eq!(transition.last_known, old_snapshot);
    assert_eq!(transition.new_snapshot.object, expected_new_object);
    assert_eq!(transition.new_snapshot.location, owner_hand(P2));
    assert_eq!(result.delta.apply(&before).unwrap(), result.next_state);
    assert_eq!(result.next_state.random, before.random);
    assert_eq!(result.next_state.revision.0, before.revision.0 + 1);
}

#[test]
fn s2_zone_library_singleton_removes_empty_order_key() {
    let mut before = base_state();
    before
        .zones
        .objects
        .get_mut(&OLD_LIBRARY_TOP)
        .unwrap()
        .face_down = false;
    remove_object_tracking(&mut before, OLD_LIBRARY_TOP);
    validate_engine_state(&before).unwrap();
    let library_key = owner_library_top(P2).key();
    assert_eq!(before.zones.ordered_zones[&library_key], [OLD_LIBRARY_TOP]);

    let result = execute_selected_zone_transition_for_conformance(
        &before,
        OLD_LIBRARY_TOP,
        owner_library_top(P2),
        owner_hand(P2),
        ConformanceZoneTransitionKind::LibraryTopToOwnerHand,
    )
    .expect("singleton library transition");
    assert!(!result
        .next_state
        .zones
        .ordered_zones
        .contains_key(&library_key));
    assert_eq!(result.next_state.zones.ordered_zones.len(), 0);
    assert_eq!(
        result.next_state.zones.locations[&GameObjectId(3)],
        owner_hand(P2)
    );
}

#[test]
fn s2_zone_graveyard_order() {
    let before = task2_battlefield_case_state();
    let key = before.zones.locations.get(&GameObjectId(3)).unwrap().key();
    assert_eq!(
        before.zones.ordered_zones[&key],
        [GameObjectId(3), GameObjectId(4)]
    );
    // Literal expected post-state; do not call production ordering helpers.
    let expected = [
        (GameObjectId(5), ZonePosition::Top { offset: 0 }),
        (GameObjectId(3), ZonePosition::Top { offset: 1 }),
        (GameObjectId(4), ZonePosition::Top { offset: 2 }),
    ];
    assert_eq!(expected[0].1, ZonePosition::Top { offset: 0 });
    assert_eq!(expected[1].1, ZonePosition::Top { offset: 1 });
    assert_eq!(expected[2].1, ZonePosition::Top { offset: 2 });
    let result = execute_selected_zone_transition_for_conformance(
        &before,
        OLD_BATTLEFIELD,
        battlefield_from(),
        owner_graveyard_top(P1),
        ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
    )
    .expect("graveyard ordering product");
    assert_eq!(
        result.next_state.zones.ordered_zones[&key],
        [GameObjectId(5), GameObjectId(3), GameObjectId(4),]
    );
    for (object, position) in expected {
        assert_eq!(
            result.next_state.zones.locations[&object].position,
            position
        );
    }
}

#[test]
fn s2_zone_event_delta_cursor_expectations() {
    let before = task2_battlefield_case_state();
    let old_snapshot = ObjectSnapshot {
        object: OLD_BATTLEFIELD,
        physical_card: Some(CARD_BATTLEFIELD),
        card_definition: CardDefinitionId(1),
        owner: P1,
        controller: P1,
        tapped: false,
        face_down: false,
        location: location(
            ZoneKind::Battlefield,
            None,
            ZonePosition::Unordered,
            VisibilityPartition::Public,
        ),
    };
    let expected_transition = mtgml_state::ZoneTransition {
        old_object: OLD_BATTLEFIELD,
        new_object: GameObjectId(5),
        physical_card: Some(CARD_BATTLEFIELD),
        from: location(
            ZoneKind::Battlefield,
            None,
            ZonePosition::Unordered,
            VisibilityPartition::Public,
        ),
        to: location(
            ZoneKind::Graveyard,
            Some(P1),
            ZonePosition::Top { offset: 0 },
            VisibilityPartition::Public,
        ),
        last_known: old_snapshot,
        new_snapshot: ObjectSnapshot {
            object: GameObjectId(5),
            physical_card: Some(CARD_BATTLEFIELD),
            card_definition: CardDefinitionId(1),
            owner: P1,
            controller: P1,
            tapped: false,
            face_down: false,
            location: location(
                ZoneKind::Graveyard,
                Some(P1),
                ZonePosition::Top { offset: 0 },
                VisibilityPartition::Public,
            ),
        },
    };
    let expected_event = mtgml_rules::AuthoritativeRuleEventKind::ZoneTransition {
        transition: Box::new(expected_transition.clone()),
    };
    let expected_delta = mtgml_state::SemanticDeltaOperation::ZoneTransition {
        transition: Box::new(expected_transition),
    };
    assert_eq!(expected_event.semantic_delta(), expected_delta);
    assert_eq!(before.revision.0.checked_add(1), Some(1));
    let result = execute_selected_zone_transition_for_conformance(
        &before,
        OLD_BATTLEFIELD,
        battlefield_from(),
        owner_graveyard_top(P1),
        ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
    )
    .expect("event/delta/cursor product");
    assert_eq!(result.events.len(), 1);
    assert_eq!(result.events[0].event, expected_event);
    assert_eq!(
        result.events[0].event_id,
        before.allocators.next_rule_event_id
    );
    assert_eq!(result.events[0].state_revision, result.next_state.revision);
    assert_eq!(result.next_state.allocators.next_rule_event_id.0, 2);
    assert_eq!(result.delta.audit, vec![expected_delta]);
    assert_eq!(result.delta.apply(&before).unwrap(), result.next_state);
    assert_eq!(result.next_state.revision.0, 1);
    assert_eq!(result.next_state.random, before.random);
}

#[test]
fn s2_identity_destination_canonical_state() {
    // `controller = owner` is GameObject storage normalization for non-
    // battlefield rows, not a Magic rule assigning those cards a controller.
    let battlefield_expected = GameObject {
        id: GameObjectId(5),
        physical_card: Some(CARD_BATTLEFIELD),
        card_definition: CardDefinitionId(1),
        owner: P1,
        controller: P1,
        tapped: false,
        face_down: false,
    };
    let library_expected = GameObject {
        id: GameObjectId(4),
        physical_card: Some(CARD_LIBRARY),
        card_definition: CardDefinitionId(2),
        owner: P2,
        controller: P2,
        tapped: false,
        face_down: false,
    };
    let expected_battlefield_snapshot = ObjectSnapshot {
        object: GameObjectId(5),
        physical_card: Some(CARD_BATTLEFIELD),
        card_definition: CardDefinitionId(1),
        owner: P1,
        controller: P1,
        tapped: false,
        face_down: false,
        location: location(
            ZoneKind::Graveyard,
            Some(P1),
            ZonePosition::Top { offset: 0 },
            VisibilityPartition::Public,
        ),
    };
    let expected_library_snapshot = ObjectSnapshot {
        object: GameObjectId(4),
        physical_card: Some(CARD_LIBRARY),
        card_definition: CardDefinitionId(2),
        owner: P2,
        controller: P2,
        tapped: false,
        face_down: false,
        location: location(
            ZoneKind::Hand,
            Some(P2),
            ZonePosition::Unordered,
            VisibilityPartition::OwnerOnly,
        ),
    };
    assert_eq!(battlefield_expected.controller, battlefield_expected.owner);
    assert_eq!(library_expected.controller, library_expected.owner);
    assert_eq!(expected_battlefield_snapshot.object, GameObjectId(5));
    assert_eq!(
        expected_battlefield_snapshot.location.zone,
        ZoneKind::Graveyard
    );
    assert_eq!(expected_library_snapshot.object, GameObjectId(4));
    assert_eq!(expected_library_snapshot.location.zone, ZoneKind::Hand);
    let result = battlefield_request().expect("canonical Graveyard row");
    assert_eq!(
        result.next_state.zones.objects[&GameObjectId(5)],
        battlefield_expected
    );
    let transition = match &result.events[0].event {
        mtgml_rules::AuthoritativeRuleEventKind::ZoneTransition { transition } => transition,
        other => panic!("unexpected first event: {other:?}"),
    };
    assert_eq!(transition.new_snapshot, expected_battlefield_snapshot);
}

#[test]
fn s2_identity_destination_canonical_state_library() {
    let expected = ObjectSnapshot {
        object: GameObjectId(4),
        physical_card: Some(CARD_LIBRARY),
        card_definition: CardDefinitionId(2),
        owner: P2,
        controller: P2,
        tapped: false,
        face_down: false,
        location: location(
            ZoneKind::Hand,
            Some(P2),
            ZonePosition::Unordered,
            VisibilityPartition::OwnerOnly,
        ),
    };
    assert_eq!(expected.physical_card, Some(CARD_LIBRARY));
    assert_eq!(expected.owner, P2);
    assert_eq!(expected.controller, P2);
    assert!(!expected.tapped);
    assert!(!expected.face_down);
    let result = library_request().expect("canonical Hand row");
    assert_eq!(
        result.next_state.zones.objects[&GameObjectId(4)],
        GameObject {
            id: expected.object,
            physical_card: expected.physical_card,
            card_definition: expected.card_definition,
            owner: expected.owner,
            controller: expected.controller,
            tapped: expected.tapped,
            face_down: expected.face_down,
        }
    );
    let transition = match &result.events[0].event {
        mtgml_rules::AuthoritativeRuleEventKind::ZoneTransition { transition } => transition,
        other => panic!("unexpected first event: {other:?}"),
    };
    assert_eq!(transition.new_snapshot, expected);
}

#[test]
fn s2_identity_old_reference_closure_characterization() {
    const CASE_ID: &str = "s2.identity.old_reference_closure";
    let state = battlefield_case_state();
    let old = OLD_BATTLEFIELD;
    // Pin each current authoritative reference category in this validated
    // state. S2 admission requires all non-source sites to be absent.
    let refs = BTreeMap::from([
        ("zones.objects", state.zones.objects.contains_key(&old)),
        ("zones.locations", state.zones.locations.contains_key(&old)),
        (
            "ordered_zones",
            state
                .zones
                .ordered_zones
                .values()
                .any(|ids| ids.contains(&old)),
        ),
        (
            "foundation_sources",
            state.foundation_sources.contains_key(&old),
        ),
        (
            "combat.attackers",
            state
                .combat
                .as_ref()
                .is_some_and(|c| c.attackers.contains(&old)),
        ),
        (
            "combat.blocker_keys_or_values",
            state.combat.as_ref().is_some_and(|c| {
                c.blockers
                    .iter()
                    .any(|(attacker, blocker)| *attacker == old || *blocker == Some(old))
            }),
        ),
        (
            "stack_records",
            state
                .zones
                .stack_records
                .values()
                .any(|r| r.source_object == Some(old)),
        ),
        (
            "pending_trusted_bindings",
            state.execution.pending_decision.as_ref().is_some_and(|p| {
                p.request
                    .candidates
                    .iter()
                    .any(|c| match &c.trusted_binding {
                        mtgml_decision::EngineCandidateBinding::CastSpell { object }
                        | mtgml_decision::EngineCandidateBinding::SelectObject { object } => {
                            *object == old
                        }
                        _ => false,
                    })
            }),
        ),
        (
            "perspective_live_mappings",
            state
                .perspective_identities
                .players
                .values()
                .any(|identity| identity.object_to_opaque.contains_key(&old)),
        ),
    ]);
    assert!(refs["zones.objects"], "{CASE_ID}");
    assert!(refs["zones.locations"], "{CASE_ID}");
    assert!(!refs["ordered_zones"], "{CASE_ID}");
    assert!(refs["foundation_sources"], "{CASE_ID}");
    assert!(!refs["combat.attackers"], "{CASE_ID}");
    assert!(!refs["combat.blocker_keys_or_values"], "{CASE_ID}");
    assert!(!refs["stack_records"], "{CASE_ID}");
    assert!(!refs["pending_trusted_bindings"], "{CASE_ID}");
    assert!(refs["perspective_live_mappings"], "{CASE_ID}");
    // These current shapes contain no GameObjectId fields: effects, waiting
    // triggers, delayed effects, continuations, and FormatState.
    assert!(state.execution.effects.is_empty());
    assert!(state.execution.waiting_triggers.is_empty());
    assert!(state.execution.delayed_effects.is_empty());
    assert!(state.execution.continuations.is_empty());
    assert!(matches!(state.format, mtgml_state::FormatState::None));
}

#[test]
fn s2_identity_old_reference_forbidden_sites_reject() {
    let mut combat = task2_battlefield_case_state();
    combat.combat = Some(mtgml_state::CombatState {
        defending_player: P2,
        attackers: vec![OLD_BATTLEFIELD],
        blockers: BTreeMap::from([(OLD_BATTLEFIELD, None)]),
    });
    validate_engine_state(&combat).unwrap();
    assert_rejected_request_preserves_complete_state(
        &combat,
        execute_selected_zone_transition_for_conformance(
            &combat,
            OLD_BATTLEFIELD,
            battlefield_from(),
            owner_graveyard_top(P1),
            ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
        ),
        |error| {
            matches!(
                error,
                KernelExecutionError::ZoneIncarnation(ZoneIncarnationError::CombatReference)
            )
        },
    );

    let mut stack = task2_battlefield_case_state();
    let stack_id = mtgml_model::StackObjectId(1);
    stack.zones.stack_records.insert(
        stack_id,
        mtgml_state::StackRecord {
            id: stack_id,
            controller: P1,
            source_object: Some(OLD_BATTLEFIELD),
            source_ability: None,
        },
    );
    stack.zones.stack_order.push(stack_id);
    stack.allocators.next_stack_object_id = mtgml_model::StackObjectId(2);
    validate_engine_state(&stack).unwrap();
    assert_rejected_request_preserves_complete_state(
        &stack,
        execute_selected_zone_transition_for_conformance(
            &stack,
            OLD_BATTLEFIELD,
            battlefield_from(),
            owner_graveyard_top(P1),
            ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
        ),
        |error| {
            matches!(
                error,
                KernelExecutionError::ZoneIncarnation(ZoneIncarnationError::StackSourceReference)
            )
        },
    );

    let pending = construct_synthetic_engine_state(SyntheticResetInputs {
        players: [P1, P2],
        root_seed: mtgml_random::RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap(),
        setup: SyntheticV4Setup::m2_compatibility(),
    })
    .unwrap();
    validate_engine_state(&pending).unwrap();
    assert_rejected_request_preserves_complete_state(
        &pending,
        execute_selected_zone_transition_for_conformance(
            &pending,
            OLD_BATTLEFIELD,
            battlefield_from(),
            owner_graveyard_top(P1),
            ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
        ),
        |error| {
            matches!(
                error,
                KernelExecutionError::ZoneIncarnation(
                    ZoneIncarnationError::PendingDecisionReference
                )
            )
        },
    );
}

#[test]
fn s2_identity_public_remap() {
    let before = battlefield_case_state();
    let result = execute_selected_zone_transition_for_conformance(
        &before,
        OLD_BATTLEFIELD,
        battlefield_from(),
        owner_graveyard_top(P1),
        ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
    )
    .unwrap();
    assert_event_delta_mirror(&result);
    assert_eq!(result.events.len(), 3);
    assert!(matches!(
        result.events[0].event,
        mtgml_rules::AuthoritativeRuleEventKind::ZoneTransition { .. }
    ));
    assert!(!result
        .next_state
        .zones
        .objects
        .contains_key(&OLD_BATTLEFIELD));
    assert!(!result
        .next_state
        .zones
        .locations
        .contains_key(&OLD_BATTLEFIELD));
    assert!(!result
        .next_state
        .zones
        .ordered_zones
        .values()
        .any(|objects| objects.contains(&OLD_BATTLEFIELD)));
    assert!(!result
        .next_state
        .foundation_sources
        .contains_key(&OLD_BATTLEFIELD));
    assert!(!result
        .next_state
        .zones
        .stack_records
        .values()
        .any(|record| record.source_object == Some(OLD_BATTLEFIELD)));
    assert!(result.next_state.combat.as_ref().is_none_or(|combat| {
        !combat.attackers.contains(&OLD_BATTLEFIELD)
            && !combat.blockers.contains_key(&OLD_BATTLEFIELD)
            && !combat
                .blockers
                .values()
                .any(|blocker| *blocker == Some(OLD_BATTLEFIELD))
    }));
    assert!(result
        .next_state
        .execution
        .pending_decision
        .as_ref()
        .is_none_or(
            |pending| !pending.request.candidates.iter().any(|candidate| {
                matches!(
                    candidate.trusted_binding,
                    mtgml_decision::EngineCandidateBinding::CastSpell { object }
                        | mtgml_decision::EngineCandidateBinding::SelectObject { object }
                        if object == OLD_BATTLEFIELD
                )
            })
        ));
    assert!(result
        .next_state
        .perspective_identities
        .players
        .values()
        .all(|identity| !identity.object_to_opaque.contains_key(&OLD_BATTLEFIELD)));
    let opaque = mtgml_model::OpaqueObjectId(1);
    let occurrences: Vec<_> = result
        .events
        .iter()
        .filter_map(|event| match &event.event {
            mtgml_rules::AuthoritativeRuleEventKind::PerspectiveOccurrence {
                lifecycle,
                observation,
            } => Some((lifecycle, observation)),
            _ => None,
        })
        .collect();
    assert_eq!(
        occurrences
            .iter()
            .map(|(lifecycle, _)| lifecycle.perspective)
            .collect::<Vec<_>>(),
        vec![P1, P2]
    );
    for perspective in [P1, P2] {
        let before_identity = &before.perspective_identities.players[&perspective];
        let after_identity = &result.next_state.perspective_identities.players[&perspective];
        assert_eq!(
            after_identity.opaque_to_object.get(&opaque),
            Some(&GameObjectId(5))
        );
        assert!(!after_identity
            .object_to_opaque
            .contains_key(&OLD_BATTLEFIELD));
        assert_eq!(
            after_identity.next_opaque_object_id,
            before_identity.next_opaque_object_id
        );
        let before_knowledge = &before.knowledge.players[&perspective];
        let after_knowledge = &result.next_state.knowledge.players[&perspective];
        assert_eq!(
            after_knowledge.next_visible_sequence,
            mtgml_model::VisibleSequence(2)
        );
        let record = &after_knowledge.active[&opaque];
        assert_eq!(record.card_definition, Some(CardDefinitionId(1)));
        assert_eq!(record.historical_locations.len(), 1);
        assert_eq!(record.historical_locations[0].location, battlefield_from());
        let current = record.known_location.as_ref().unwrap();
        assert_eq!(current.location, owner_graveyard_top(P1));
        assert_eq!(
            current.provenance,
            mtgml_state::KnowledgeAcquisitionReason::Observed {
                channel: mtgml_state::KnowledgeHistoryChannel::Public,
                sequence: mtgml_model::VisibleSequence(1),
                cause: mtgml_state::KnowledgeAcquisitionCause::PublicEvent,
            }
        );
        assert_eq!(
            before_knowledge.next_visible_sequence,
            mtgml_model::VisibleSequence(1)
        );
    }
    for (_, observation) in occurrences {
        assert!(matches!(
            observation,
            mtgml_rules::PerspectiveObservationPolicyV1::MovedInSight {
                from_zone: ZoneKind::Battlefield,
                to_zone: ZoneKind::Graveyard,
                old_object: OLD_BATTLEFIELD,
                new_object: GameObjectId(5),
                reveals_old: true,
                reveals_new: true,
            }
        ));
    }
    for (event, perspective) in result.events.iter().skip(1).zip([P1, P2]) {
        let mtgml_rules::AuthoritativeRuleEventKind::PerspectiveOccurrence { lifecycle, .. } =
            &event.event
        else {
            panic!("expected perspective occurrence after ZoneTransition");
        };
        assert_eq!(lifecycle.perspective, perspective);
        assert_eq!(lifecycle.sequence, mtgml_model::VisibleSequence(1));
        assert_eq!(
            lifecycle.mutation.identity,
            mtgml_state::IdentityMutationV1::Remap {
                opaque,
                from_object: OLD_BATTLEFIELD,
                to_object: GameObjectId(5),
            }
        );
    }
    let projected = mtgml_environment::lifecycle_projection::project_occurrence_envelopes(
        &before,
        &result.next_state,
        &result.events,
    )
    .unwrap();
    assert_event_delta_mirror(&result);
    for perspective in [P1, P2] {
        assert_eq!(projected[&perspective].len(), 1);
        assert_eq!(
            projected[&perspective][0].event,
            mtgml_observation::ObservedEventKindV2::ObjectMoved {
                old_object: Some(opaque),
                new_object: Some(opaque),
                from: ZoneKind::Battlefield,
                to: ZoneKind::Graveyard,
            }
        );
    }
}

#[test]
fn s2_identity_owner_hand_private() {
    let before = first_private_library_case_state();
    let non_owner_identity = before.perspective_identities.players[&P1].clone();
    let non_owner_knowledge = before.knowledge.players[&P1].clone();
    let result = execute_selected_zone_transition_for_conformance(
        &before,
        OLD_LIBRARY_TOP,
        owner_library_top(P2),
        owner_hand(P2),
        ConformanceZoneTransitionKind::LibraryTopToOwnerHand,
    )
    .unwrap();
    assert_event_delta_mirror(&result);
    let new = GameObjectId(4);
    let opaque = mtgml_model::OpaqueObjectId(3);
    let owner_identity = &result.next_state.perspective_identities.players[&P2];
    assert_eq!(owner_identity.opaque_to_object.get(&opaque), Some(&new));
    assert_eq!(
        owner_identity.next_opaque_object_id,
        mtgml_model::OpaqueObjectId(4)
    );
    assert!(!owner_identity
        .object_to_opaque
        .contains_key(&OLD_LIBRARY_TOP));
    let owner_knowledge = &result.next_state.knowledge.players[&P2];
    assert_eq!(
        owner_knowledge.next_visible_sequence,
        mtgml_model::VisibleSequence(2)
    );
    let record = &owner_knowledge.active[&opaque];
    assert_eq!(record.physical_card, None);
    assert_eq!(record.card_definition, Some(CardDefinitionId(2)));
    assert!(record.historical_locations.is_empty());
    assert_eq!(
        record.known_location.as_ref().unwrap().location,
        owner_hand(P2)
    );
    let acquisition = mtgml_state::KnowledgeAcquisitionReason::Observed {
        channel: mtgml_state::KnowledgeHistoryChannel::Private,
        sequence: mtgml_model::VisibleSequence(1),
        cause: mtgml_state::KnowledgeAcquisitionCause::OwnPrivateIdentity,
    };
    assert_eq!(record.acquisition, acquisition);
    assert_eq!(
        record.known_location.as_ref().unwrap().provenance,
        acquisition
    );
    assert_eq!(
        result.next_state.perspective_identities.players[&P1],
        non_owner_identity
    );
    assert_eq!(
        result.next_state.knowledge.players[&P1],
        non_owner_knowledge
    );
    assert_eq!(result.events.len(), 2);
    assert!(matches!(
        &result.events[1].event,
        mtgml_rules::AuthoritativeRuleEventKind::PerspectiveOccurrence {
            lifecycle: mtgml_state::PerspectiveLifecycleAuditV1 {
                perspective: P2,
                sequence: mtgml_model::VisibleSequence(1),
                mutation: mtgml_state::PerspectiveLifecycleMutationV1 {
                    identity: mtgml_state::IdentityMutationV1::Allocate {
                        opaque: mtgml_model::OpaqueObjectId(3),
                        object: GameObjectId(4),
                    },
                    knowledge: Some(mtgml_state::KnowledgeMutationV1::Acquire { .. }),
                },
            },
            observation: mtgml_rules::PerspectiveObservationPolicyV1::NoEnvelope,
        }
    ));
    let projected = mtgml_environment::lifecycle_projection::project_occurrence_envelopes(
        &before,
        &result.next_state,
        &result.events,
    )
    .unwrap();
    assert!(projected[&P1].is_empty());
    assert!(projected[&P2].is_empty());
}

#[test]
fn s2_identity_owner_hand_private_preknown() {
    let before = library_case_state();
    let owner_allocator = before.perspective_identities.players[&P2].next_opaque_object_id;
    let non_owner_identity = before.perspective_identities.players[&P1].clone();
    let non_owner_knowledge = before.knowledge.players[&P1].clone();
    let opaque = before.perspective_identities.players[&P2].object_to_opaque[&OLD_LIBRARY_TOP];
    let result = execute_selected_zone_transition_for_conformance(
        &before,
        OLD_LIBRARY_TOP,
        owner_library_top(P2),
        owner_hand(P2),
        ConformanceZoneTransitionKind::LibraryTopToOwnerHand,
    )
    .unwrap();
    assert_event_delta_mirror(&result);
    let owner_identity = &result.next_state.perspective_identities.players[&P2];
    assert_eq!(
        owner_identity.opaque_to_object.get(&opaque),
        Some(&GameObjectId(4))
    );
    assert!(!owner_identity
        .object_to_opaque
        .contains_key(&OLD_LIBRARY_TOP));
    assert_eq!(owner_identity.next_opaque_object_id, owner_allocator);
    let owner_knowledge = &result.next_state.knowledge.players[&P2];
    assert_eq!(
        owner_knowledge.next_visible_sequence,
        mtgml_model::VisibleSequence(2)
    );
    let record = &owner_knowledge.active[&opaque];
    assert_eq!(record.card_definition, Some(CardDefinitionId(2)));
    assert_eq!(record.historical_locations.len(), 1);
    assert_eq!(
        record.historical_locations[0].location,
        owner_library_top(P2)
    );
    assert_eq!(
        record.known_location.as_ref().unwrap().location,
        owner_hand(P2)
    );
    assert_eq!(
        record.known_location.as_ref().unwrap().provenance,
        mtgml_state::KnowledgeAcquisitionReason::Observed {
            channel: mtgml_state::KnowledgeHistoryChannel::Private,
            sequence: mtgml_model::VisibleSequence(1),
            cause: mtgml_state::KnowledgeAcquisitionCause::OwnPrivateIdentity,
        }
    );
    assert_eq!(
        result.next_state.perspective_identities.players[&P1],
        non_owner_identity
    );
    assert_eq!(
        result.next_state.knowledge.players[&P1],
        non_owner_knowledge
    );
    assert_eq!(result.events.len(), 2);
    assert!(matches!(
        &result.events[1].event,
        mtgml_rules::AuthoritativeRuleEventKind::PerspectiveOccurrence {
            lifecycle: mtgml_state::PerspectiveLifecycleAuditV1 {
                perspective: P2,
                mutation: mtgml_state::PerspectiveLifecycleMutationV1 {
                    identity: mtgml_state::IdentityMutationV1::Remap {
                        opaque: seen,
                        from_object: OLD_LIBRARY_TOP,
                        to_object: GameObjectId(4),
                    },
                    knowledge: Some(mtgml_state::KnowledgeMutationV1::UpdateLocation { .. }),
                },
                ..
            },
            observation: mtgml_rules::PerspectiveObservationPolicyV1::MovedInSight {
                from_zone: ZoneKind::Library,
                to_zone: ZoneKind::Hand,
                old_object: OLD_LIBRARY_TOP,
                new_object: GameObjectId(4),
                reveals_old: true,
                reveals_new: true,
            },
        } if *seen == opaque
    ));
    let projected = mtgml_environment::lifecycle_projection::project_occurrence_envelopes(
        &before,
        &result.next_state,
        &result.events,
    )
    .unwrap();
    assert!(projected[&P1].is_empty());
    assert_eq!(projected[&P2].len(), 1);
    assert_eq!(
        projected[&P2][0].event,
        mtgml_observation::ObservedEventKindV2::ObjectMoved {
            old_object: Some(opaque),
            new_object: Some(opaque),
            from: ZoneKind::Library,
            to: ZoneKind::Hand,
        }
    );
}

#[test]
fn s2_library_non_owner_mapping_rejects() {
    let mut before = library_case_state();
    let opaque = mtgml_model::OpaqueObjectId(2);
    let library_location = owner_library_top(P2);
    let p1_identity = before.perspective_identities.players.get_mut(&P1).unwrap();
    p1_identity.opaque_to_object.insert(opaque, OLD_LIBRARY_TOP);
    p1_identity.object_to_opaque.insert(OLD_LIBRARY_TOP, opaque);
    p1_identity.next_opaque_object_id = mtgml_model::OpaqueObjectId(3);
    before
        .knowledge
        .players
        .get_mut(&P1)
        .unwrap()
        .active
        .insert(
            opaque,
            mtgml_state::KnowledgeRecordV2 {
                opaque_object: opaque,
                physical_card: Some(CARD_LIBRARY),
                card_definition: Some(CardDefinitionId(2)),
                known_location: Some(mtgml_state::KnownLocationFactV2 {
                    location: library_location,
                    provenance: mtgml_state::KnowledgeAcquisitionReason::InitialConfiguration,
                }),
                historical_locations: Vec::new(),
                acquisition: mtgml_state::KnowledgeAcquisitionReason::InitialConfiguration,
            },
        );
    validate_engine_state(&before).unwrap();
    assert_rejected_request_preserves_complete_state(
        &before,
        execute_selected_zone_transition_for_conformance(
            &before,
            OLD_LIBRARY_TOP,
            owner_library_top(P2),
            owner_hand(P2),
            ConformanceZoneTransitionKind::LibraryTopToOwnerHand,
        ),
        |error| {
            matches!(
                error,
                KernelExecutionError::ZoneIncarnation(
                    ZoneIncarnationError::NonOwnerTracksHiddenSource
                )
            )
        },
    );
}

#[test]
fn s2_observation_lifecycle_matrix() {
    let public_before = battlefield_case_state();
    let public_result = execute_selected_zone_transition_for_conformance(
        &public_before,
        OLD_BATTLEFIELD,
        battlefield_from(),
        owner_graveyard_top(P1),
        ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
    )
    .unwrap();
    let public_projection = mtgml_environment::lifecycle_projection::project_occurrence_envelopes(
        &public_before,
        &public_result.next_state,
        &public_result.events,
    )
    .unwrap();
    assert_eq!(public_projection[&P1].len(), 1);
    assert_eq!(public_projection[&P2].len(), 1);

    let first_private_before = first_private_library_case_state();
    let first_private_result = execute_selected_zone_transition_for_conformance(
        &first_private_before,
        OLD_LIBRARY_TOP,
        owner_library_top(P2),
        owner_hand(P2),
        ConformanceZoneTransitionKind::LibraryTopToOwnerHand,
    )
    .unwrap();
    let first_private_projection =
        mtgml_environment::lifecycle_projection::project_occurrence_envelopes(
            &first_private_before,
            &first_private_result.next_state,
            &first_private_result.events,
        )
        .unwrap();
    assert!(first_private_projection[&P1].is_empty());
    assert!(first_private_projection[&P2].is_empty());

    let preknown_before = library_case_state();
    let preknown_result = execute_selected_zone_transition_for_conformance(
        &preknown_before,
        OLD_LIBRARY_TOP,
        owner_library_top(P2),
        owner_hand(P2),
        ConformanceZoneTransitionKind::LibraryTopToOwnerHand,
    )
    .unwrap();
    let preknown_projection =
        mtgml_environment::lifecycle_projection::project_occurrence_envelopes(
            &preknown_before,
            &preknown_result.next_state,
            &preknown_result.events,
        )
        .unwrap();
    assert!(preknown_projection[&P1].is_empty());
    assert_eq!(preknown_projection[&P2].len(), 1);
}

#[test]
fn s2_identity_foundation_source_cessation() {
    let before = battlefield_case_state();
    assert!(before.foundation_sources.contains_key(&OLD_BATTLEFIELD));
    let expected_new = GameObjectId(5);
    assert!(!before.foundation_sources.contains_key(&expected_new));
    let result = execute_selected_zone_transition_for_conformance(
        &before,
        OLD_BATTLEFIELD,
        battlefield_from(),
        owner_graveyard_top(P1),
        ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
    )
    .expect("tracked creature moves through integrated core product");
    assert!(!result
        .next_state
        .foundation_sources
        .contains_key(&OLD_BATTLEFIELD));
    assert!(!result
        .next_state
        .foundation_sources
        .contains_key(&expected_new));
}

#[test]
fn s2_request_vocabulary_represents_typed_rejection_inputs() {
    let before = task2_battlefield_case_state();
    let mismatched_source = location(
        ZoneKind::Hand,
        Some(P1),
        ZonePosition::Unordered,
        VisibilityPartition::OwnerOnly,
    );
    let wrong_owner_graveyard = owner_graveyard_top(P2);
    let unadmitted_source = location(
        ZoneKind::Exile,
        None,
        ZonePosition::Unordered,
        VisibilityPartition::Public,
    );
    let wrong_owner_hand = owner_hand(P1);

    assert_rejected_request_preserves_complete_state(
        &before,
        execute_selected_zone_transition_for_conformance(
            &before,
            OLD_BATTLEFIELD,
            mismatched_source,
            owner_graveyard_top(P1),
            ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
        ),
        |error| {
            matches!(
                error,
                KernelExecutionError::ZoneIncarnation(
                    ZoneIncarnationError::ClaimedSourceLocationMismatch
                )
            )
        },
    );
    assert_rejected_request_preserves_complete_state(
        &before,
        execute_selected_zone_transition_for_conformance(
            &before,
            OLD_BATTLEFIELD,
            battlefield_from(),
            wrong_owner_graveyard,
            ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
        ),
        |error| {
            matches!(
                error,
                KernelExecutionError::ZoneIncarnation(ZoneIncarnationError::DestinationMismatch)
            )
        },
    );
    let mut unadmitted_before = before.clone();
    let actual_unadmitted_source = location(
        ZoneKind::Exile,
        None,
        ZonePosition::Unordered,
        VisibilityPartition::Public,
    );
    unadmitted_before
        .zones
        .locations
        .insert(OLD_BATTLEFIELD, actual_unadmitted_source.clone());
    validate_engine_state(&unadmitted_before).unwrap();
    assert_rejected_request_preserves_complete_state(
        &unadmitted_before,
        execute_selected_zone_transition_for_conformance(
            &unadmitted_before,
            OLD_BATTLEFIELD,
            unadmitted_source,
            battlefield_from(),
            ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
        ),
        |error| {
            matches!(
                error,
                KernelExecutionError::ZoneIncarnation(ZoneIncarnationError::UnadmittedSourceFamily)
            )
        },
    );
    let library_before = library_case_state();
    assert_rejected_request_preserves_complete_state(
        &library_before,
        execute_selected_zone_transition_for_conformance(
            &library_before,
            OLD_LIBRARY_TOP,
            owner_library_top(P2),
            wrong_owner_hand,
            ConformanceZoneTransitionKind::LibraryTopToOwnerHand,
        ),
        |error| {
            matches!(
                error,
                KernelExecutionError::ZoneIncarnation(ZoneIncarnationError::DestinationMismatch)
            )
        },
    );
}

#[test]
fn s2_task2_request_preconditions_fail_closed() {
    let absent_state = task2_battlefield_case_state();
    assert_rejected_request_preserves_complete_state(
        &absent_state,
        execute_selected_zone_transition_for_conformance(
            &absent_state,
            GameObjectId(99),
            battlefield_from(),
            owner_graveyard_top(P1),
            ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
        ),
        |error| {
            matches!(
                error,
                KernelExecutionError::ZoneIncarnation(ZoneIncarnationError::ObjectNotLive)
            )
        },
    );

    let mut no_physical = task2_battlefield_case_state();
    no_physical
        .zones
        .objects
        .get_mut(&OLD_BATTLEFIELD)
        .unwrap()
        .physical_card = None;
    validate_engine_state(&no_physical).unwrap();
    assert_rejected_request_preserves_complete_state(
        &no_physical,
        execute_selected_zone_transition_for_conformance(
            &no_physical,
            OLD_BATTLEFIELD,
            battlefield_from(),
            owner_graveyard_top(P1),
            ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
        ),
        |error| {
            matches!(
                error,
                KernelExecutionError::ZoneIncarnation(ZoneIncarnationError::PhysicalCardRequired)
            )
        },
    );

    let library = task2_library_case_state();
    let non_top = GameObjectId(3);
    let non_top_from = location(
        ZoneKind::Library,
        Some(P2),
        ZonePosition::Top { offset: 1 },
        VisibilityPartition::FaceDown,
    );
    assert_rejected_request_preserves_complete_state(
        &library,
        execute_selected_zone_transition_for_conformance(
            &library,
            non_top,
            non_top_from,
            owner_hand(P2),
            ConformanceZoneTransitionKind::LibraryTopToOwnerHand,
        ),
        |error| {
            matches!(
                error,
                KernelExecutionError::ZoneIncarnation(ZoneIncarnationError::LibrarySourceNotTop)
            )
        },
    );

    let mut exhausted = task2_battlefield_case_state();
    exhausted.allocators.next_object_id = GameObjectId(u64::MAX);
    validate_engine_state(&exhausted).unwrap();
    assert_rejected_request_preserves_complete_state(
        &exhausted,
        execute_selected_zone_transition_for_conformance(
            &exhausted,
            OLD_BATTLEFIELD,
            battlefield_from(),
            owner_graveyard_top(P1),
            ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
        ),
        |error| {
            matches!(
                error,
                KernelExecutionError::IdentityAllocation(
                    mtgml_state::IdentityAllocationError::GameObjectIdExhausted
                )
            )
        },
    );

    let mut invalid_before = task2_battlefield_case_state();
    invalid_before.zones.locations.remove(&OLD_BATTLEFIELD);
    assert_rejected_request_preserves_complete_state(
        &invalid_before,
        execute_selected_zone_transition_for_conformance(
            &invalid_before,
            OLD_BATTLEFIELD,
            battlefield_from(),
            owner_graveyard_top(P1),
            ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
        ),
        |error| matches!(error, KernelExecutionError::BeforeState(_)),
    );
}

#[test]
fn s2_rejection_unsupported_profile_atomic() {
    let mut face_down_battlefield = task2_battlefield_case_state();
    face_down_battlefield
        .zones
        .objects
        .get_mut(&OLD_BATTLEFIELD)
        .unwrap()
        .face_down = true;
    validate_engine_state(&face_down_battlefield).unwrap();
    assert_rejected_request_preserves_complete_state(
        &face_down_battlefield,
        execute_selected_zone_transition_for_conformance(
            &face_down_battlefield,
            OLD_BATTLEFIELD,
            battlefield_from(),
            owner_graveyard_top(P1),
            ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
        ),
        |error| {
            matches!(
                error,
                KernelExecutionError::ZoneIncarnation(
                    ZoneIncarnationError::UnsupportedSourceProfile
                )
            )
        },
    );

    let mut face_down_library = task2_library_case_state();
    face_down_library
        .zones
        .objects
        .get_mut(&OLD_LIBRARY_TOP)
        .unwrap()
        .face_down = true;
    validate_engine_state(&face_down_library).unwrap();
    assert_rejected_request_preserves_complete_state(
        &face_down_library,
        execute_selected_zone_transition_for_conformance(
            &face_down_library,
            OLD_LIBRARY_TOP,
            owner_library_top(P2),
            owner_hand(P2),
            ConformanceZoneTransitionKind::LibraryTopToOwnerHand,
        ),
        |error| {
            matches!(
                error,
                KernelExecutionError::ZoneIncarnation(
                    ZoneIncarnationError::UnsupportedSourceProfile
                )
            )
        },
    );

    let mut face_down_graveyard = task2_battlefield_case_state();
    face_down_graveyard
        .zones
        .objects
        .get_mut(&GameObjectId(3))
        .unwrap()
        .face_down = true;
    validate_engine_state(&face_down_graveyard).unwrap();
    assert_rejected_request_preserves_complete_state(
        &face_down_graveyard,
        execute_selected_zone_transition_for_conformance(
            &face_down_graveyard,
            OLD_BATTLEFIELD,
            battlefield_from(),
            owner_graveyard_top(P1),
            ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
        ),
        |error| {
            matches!(
                error,
                KernelExecutionError::ZoneIncarnation(
                    ZoneIncarnationError::UnsupportedSourceProfile
                )
            )
        },
    );

    let mut library_with_source = task2_library_case_state();
    library_with_source.foundation_sources.insert(
        OLD_LIBRARY_TOP,
        FoundationCreatureSource {
            source_kind: FoundationSourceKind::Creature,
            base_characteristics: BaseCharacteristics::Simple {
                power: 2,
                toughness: 2,
            },
            marked_damage: 0,
            control_history: ControlHistory::BeforeTurnStart { turn_number: 0 },
        },
    );
    validate_engine_state(&library_with_source).unwrap();
    assert_rejected_request_preserves_complete_state(
        &library_with_source,
        execute_selected_zone_transition_for_conformance(
            &library_with_source,
            OLD_LIBRARY_TOP,
            owner_library_top(P2),
            owner_hand(P2),
            ConformanceZoneTransitionKind::LibraryTopToOwnerHand,
        ),
        |error| {
            matches!(
                error,
                KernelExecutionError::ZoneIncarnation(
                    ZoneIncarnationError::UnsupportedSourceProfile
                )
            )
        },
    );
}

#[test]
fn s2_state_late_candidate_failures_are_atomic() {
    let mut exhausted_event = battlefield_case_state();
    exhausted_event.allocators.next_rule_event_id = mtgml_model::RuleEventId(u64::MAX);
    validate_engine_state(&exhausted_event).unwrap();
    assert_rejected_request_preserves_complete_state(
        &exhausted_event,
        execute_selected_zone_transition_for_conformance(
            &exhausted_event,
            OLD_BATTLEFIELD,
            battlefield_from(),
            owner_graveyard_top(P1),
            ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
        ),
        |error| matches!(error, KernelExecutionError::RuleEventIdOverflow),
    );

    let mut exhausted_opaque = first_private_library_case_state();
    exhausted_opaque
        .perspective_identities
        .players
        .get_mut(&P2)
        .unwrap()
        .next_opaque_object_id = mtgml_model::OpaqueObjectId(u64::MAX);
    validate_engine_state(&exhausted_opaque).unwrap();
    assert_rejected_request_preserves_complete_state(
        &exhausted_opaque,
        execute_selected_zone_transition_for_conformance(
            &exhausted_opaque,
            OLD_LIBRARY_TOP,
            owner_library_top(P2),
            owner_hand(P2),
            ConformanceZoneTransitionKind::LibraryTopToOwnerHand,
        ),
        |error| {
            matches!(
                error,
                KernelExecutionError::PerspectiveLifecycle(
                    mtgml_state::LifecycleApplicationError::AllocatorOverflow
                )
            )
        },
    );

    let mut revision_overflow = battlefield_case_state();
    revision_overflow.revision = mtgml_model::StateRevision(u64::MAX);
    validate_engine_state(&revision_overflow).unwrap();
    assert_rejected_request_preserves_complete_state(
        &revision_overflow,
        execute_selected_zone_transition_for_conformance(
            &revision_overflow,
            OLD_BATTLEFIELD,
            battlefield_from(),
            owner_graveyard_top(P1),
            ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
        ),
        |error| matches!(error, KernelExecutionError::RevisionOverflow),
    );
}

fn assert_environment_fingerprint_unchanged_for_rejection(
    case: &str,
    before_state: &EngineState,
    object: GameObjectId,
    claimed_from: ZoneLocation,
    claimed_to: ZoneLocation,
    kind: ConformanceZoneTransitionKind,
    expected_error: impl FnOnce(&KernelExecutionError) -> bool,
) {
    let config = crate::isolation::synthetic_environment_config([P1, P2]);
    let (controller, endpoints) = crate::isolation::spawn_environment(
        before_state.clone(),
        &config,
    )
    .unwrap_or_else(|error| {
        panic!("{case}: valid rejection before-state was not environment-admissible: {error:?}")
    });
    let before = crate::isolation::capture_complete(&controller, &endpoints).unwrap();
    let authoritative_before = before.semantic.engine_state_equal_probe.clone();

    assert_rejected_request_preserves_complete_state(
        &authoritative_before,
        execute_selected_zone_transition_for_conformance(
            &authoritative_before,
            object,
            claimed_from,
            claimed_to,
            kind,
        ),
        |error| {
            let matches = expected_error(error);
            assert!(matches, "{case}: unexpected direct S2 rejection: {error:?}");
            matches
        },
    );

    let after = crate::isolation::capture_complete(&controller, &endpoints).unwrap();
    crate::isolation::assert_fingerprint_policies(
        &before,
        &after,
        crate::isolation::FingerprintComparison::All,
    )
    .unwrap();
}

#[test]
fn s2_rejected_direct_requests_preserve_complete_environment_fingerprint_matrix() {
    type Expected = fn(&KernelExecutionError) -> bool;
    type Case = (
        &'static str,
        EngineState,
        GameObjectId,
        ZoneLocation,
        ZoneLocation,
        ConformanceZoneTransitionKind,
        Expected,
    );

    let absent = task2_battlefield_case_state();
    validate_engine_state(&absent).unwrap();
    let mismatched_location = task2_battlefield_case_state();
    validate_engine_state(&mismatched_location).unwrap();
    let claimed_hand = owner_hand(P1);
    let mut unadmitted = task2_battlefield_case_state();
    let exile = location(
        ZoneKind::Exile,
        None,
        ZonePosition::Unordered,
        VisibilityPartition::Public,
    );
    unadmitted
        .zones
        .locations
        .insert(OLD_BATTLEFIELD, exile.clone());
    validate_engine_state(&unadmitted).unwrap();
    let wrong_owner = task2_battlefield_case_state();
    validate_engine_state(&wrong_owner).unwrap();
    let library = task2_library_case_state();
    validate_engine_state(&library).unwrap();
    let mut missing_physical = task2_battlefield_case_state();
    missing_physical
        .zones
        .objects
        .get_mut(&OLD_BATTLEFIELD)
        .unwrap()
        .physical_card = None;
    validate_engine_state(&missing_physical).unwrap();
    let mut unsupported_profile = task2_battlefield_case_state();
    unsupported_profile
        .zones
        .objects
        .get_mut(&OLD_BATTLEFIELD)
        .unwrap()
        .face_down = true;
    validate_engine_state(&unsupported_profile).unwrap();

    let mut combat = task2_battlefield_case_state();
    combat.combat = Some(mtgml_state::CombatState {
        defending_player: P2,
        attackers: vec![OLD_BATTLEFIELD],
        blockers: BTreeMap::from([(OLD_BATTLEFIELD, None)]),
    });
    validate_engine_state(&combat).unwrap();
    let mut stack = task2_battlefield_case_state();
    let stack_id = mtgml_model::StackObjectId(1);
    stack.zones.stack_records.insert(
        stack_id,
        mtgml_state::StackRecord {
            id: stack_id,
            controller: P1,
            source_object: Some(OLD_BATTLEFIELD),
            source_ability: None,
        },
    );
    stack.zones.stack_order.push(stack_id);
    stack.allocators.next_stack_object_id = mtgml_model::StackObjectId(2);
    validate_engine_state(&stack).unwrap();
    let pending = construct_synthetic_engine_state(SyntheticResetInputs {
        players: [P1, P2],
        root_seed: mtgml_random::RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap(),
        setup: SyntheticV4Setup::m2_compatibility(),
    })
    .unwrap();
    validate_engine_state(&pending).unwrap();

    let mut non_owner_tracking = library_case_state();
    let opaque = mtgml_model::OpaqueObjectId(2);
    non_owner_tracking
        .perspective_identities
        .players
        .get_mut(&P1)
        .unwrap()
        .opaque_to_object
        .insert(opaque, OLD_LIBRARY_TOP);
    non_owner_tracking
        .perspective_identities
        .players
        .get_mut(&P1)
        .unwrap()
        .object_to_opaque
        .insert(OLD_LIBRARY_TOP, opaque);
    non_owner_tracking
        .perspective_identities
        .players
        .get_mut(&P1)
        .unwrap()
        .next_opaque_object_id = mtgml_model::OpaqueObjectId(3);
    non_owner_tracking
        .knowledge
        .players
        .get_mut(&P1)
        .unwrap()
        .active
        .insert(
            opaque,
            mtgml_state::KnowledgeRecordV2 {
                opaque_object: opaque,
                physical_card: Some(CARD_LIBRARY),
                card_definition: Some(CardDefinitionId(2)),
                known_location: Some(mtgml_state::KnownLocationFactV2 {
                    location: owner_library_top(P2),
                    provenance: mtgml_state::KnowledgeAcquisitionReason::InitialConfiguration,
                }),
                historical_locations: Vec::new(),
                acquisition: mtgml_state::KnowledgeAcquisitionReason::InitialConfiguration,
            },
        );
    validate_engine_state(&non_owner_tracking).unwrap();

    let mut library_foundation = task2_library_case_state();
    library_foundation.foundation_sources.insert(
        OLD_LIBRARY_TOP,
        FoundationCreatureSource {
            source_kind: FoundationSourceKind::Creature,
            base_characteristics: BaseCharacteristics::Simple {
                power: 2,
                toughness: 2,
            },
            marked_damage: 0,
            control_history: ControlHistory::BeforeTurnStart { turn_number: 0 },
        },
    );
    validate_engine_state(&library_foundation).unwrap();

    let mut object_exhaustion = task2_battlefield_case_state();
    object_exhaustion.allocators.next_object_id = GameObjectId(u64::MAX);
    validate_engine_state(&object_exhaustion).unwrap();
    let mut event_exhaustion = task2_battlefield_case_state();
    event_exhaustion.allocators.next_rule_event_id = mtgml_model::RuleEventId(u64::MAX);
    validate_engine_state(&event_exhaustion).unwrap();
    let mut opaque_exhaustion = first_private_library_case_state();
    opaque_exhaustion
        .perspective_identities
        .players
        .get_mut(&P2)
        .unwrap()
        .next_opaque_object_id = mtgml_model::OpaqueObjectId(u64::MAX);
    validate_engine_state(&opaque_exhaustion).unwrap();
    let mut revision_overflow = battlefield_case_state();
    revision_overflow.revision = mtgml_model::StateRevision(u64::MAX);
    validate_engine_state(&revision_overflow).unwrap();

    let first = execute_selected_zone_transition_for_conformance(
        &battlefield_case_state(),
        OLD_BATTLEFIELD,
        battlefield_from(),
        owner_graveyard_top(P1),
        ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
    )
    .expect("historical witness Step 1 is a real accepted S2 transition");
    let historical_after = first.next_state;
    validate_engine_state(&historical_after).unwrap();

    let cases: Vec<Case> = vec![
        (
            "source_absent",
            absent.clone(),
            GameObjectId(99),
            battlefield_from(),
            owner_graveyard_top(P1),
            ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
            |error| {
                matches!(
                    error,
                    KernelExecutionError::ZoneIncarnation(ZoneIncarnationError::ObjectNotLive)
                )
            },
        ),
        (
            "claimed_source_location_mismatch",
            mismatched_location.clone(),
            OLD_BATTLEFIELD,
            claimed_hand,
            owner_graveyard_top(P1),
            ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
            |error| {
                matches!(
                    error,
                    KernelExecutionError::ZoneIncarnation(
                        ZoneIncarnationError::ClaimedSourceLocationMismatch
                    )
                )
            },
        ),
        (
            "unadmitted_source_family",
            unadmitted,
            OLD_BATTLEFIELD,
            exile,
            owner_graveyard_top(P1),
            ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
            |error| {
                matches!(
                    error,
                    KernelExecutionError::ZoneIncarnation(
                        ZoneIncarnationError::UnadmittedSourceFamily
                    )
                )
            },
        ),
        (
            "wrong_owner_destination",
            wrong_owner,
            OLD_BATTLEFIELD,
            battlefield_from(),
            owner_graveyard_top(P2),
            ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
            |error| {
                matches!(
                    error,
                    KernelExecutionError::ZoneIncarnation(
                        ZoneIncarnationError::DestinationMismatch
                    )
                )
            },
        ),
        (
            "library_not_top",
            library.clone(),
            GameObjectId(3),
            location(
                ZoneKind::Library,
                Some(P2),
                ZonePosition::Top { offset: 1 },
                VisibilityPartition::FaceDown,
            ),
            owner_hand(P2),
            ConformanceZoneTransitionKind::LibraryTopToOwnerHand,
            |error| {
                matches!(
                    error,
                    KernelExecutionError::ZoneIncarnation(
                        ZoneIncarnationError::LibrarySourceNotTop
                    )
                )
            },
        ),
        (
            "physical_card_missing",
            missing_physical,
            OLD_BATTLEFIELD,
            battlefield_from(),
            owner_graveyard_top(P1),
            ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
            |error| {
                matches!(
                    error,
                    KernelExecutionError::ZoneIncarnation(
                        ZoneIncarnationError::PhysicalCardRequired
                    )
                )
            },
        ),
        (
            "unsupported_source_profile",
            unsupported_profile,
            OLD_BATTLEFIELD,
            battlefield_from(),
            owner_graveyard_top(P1),
            ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
            |error| {
                matches!(
                    error,
                    KernelExecutionError::ZoneIncarnation(
                        ZoneIncarnationError::UnsupportedSourceProfile
                    )
                )
            },
        ),
        (
            "combat_reference",
            combat,
            OLD_BATTLEFIELD,
            battlefield_from(),
            owner_graveyard_top(P1),
            ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
            |error| {
                matches!(
                    error,
                    KernelExecutionError::ZoneIncarnation(ZoneIncarnationError::CombatReference)
                )
            },
        ),
        (
            "stack_source_reference",
            stack,
            OLD_BATTLEFIELD,
            battlefield_from(),
            owner_graveyard_top(P1),
            ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
            |error| {
                matches!(
                    error,
                    KernelExecutionError::ZoneIncarnation(
                        ZoneIncarnationError::StackSourceReference
                    )
                )
            },
        ),
        (
            "pending_decision_reference",
            pending,
            OLD_BATTLEFIELD,
            battlefield_from(),
            owner_graveyard_top(P1),
            ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
            |error| {
                matches!(
                    error,
                    KernelExecutionError::ZoneIncarnation(
                        ZoneIncarnationError::PendingDecisionReference
                    )
                )
            },
        ),
        (
            "non_owner_hidden_tracking",
            non_owner_tracking,
            OLD_LIBRARY_TOP,
            owner_library_top(P2),
            owner_hand(P2),
            ConformanceZoneTransitionKind::LibraryTopToOwnerHand,
            |error| {
                matches!(
                    error,
                    KernelExecutionError::ZoneIncarnation(
                        ZoneIncarnationError::NonOwnerTracksHiddenSource
                    )
                )
            },
        ),
        (
            "library_foundation_source",
            library_foundation,
            OLD_LIBRARY_TOP,
            owner_library_top(P2),
            owner_hand(P2),
            ConformanceZoneTransitionKind::LibraryTopToOwnerHand,
            |error| {
                matches!(
                    error,
                    KernelExecutionError::ZoneIncarnation(
                        ZoneIncarnationError::UnsupportedSourceProfile
                    )
                )
            },
        ),
        (
            "object_allocator_exhaustion",
            object_exhaustion,
            OLD_BATTLEFIELD,
            battlefield_from(),
            owner_graveyard_top(P1),
            ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
            |error| {
                matches!(
                    error,
                    KernelExecutionError::IdentityAllocation(
                        mtgml_state::IdentityAllocationError::GameObjectIdExhausted
                    )
                )
            },
        ),
        (
            "event_id_exhaustion",
            event_exhaustion,
            OLD_BATTLEFIELD,
            battlefield_from(),
            owner_graveyard_top(P1),
            ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
            |error| matches!(error, KernelExecutionError::RuleEventIdOverflow),
        ),
        (
            "opaque_allocator_exhaustion",
            opaque_exhaustion,
            OLD_LIBRARY_TOP,
            owner_library_top(P2),
            owner_hand(P2),
            ConformanceZoneTransitionKind::LibraryTopToOwnerHand,
            |error| {
                matches!(
                    error,
                    KernelExecutionError::PerspectiveLifecycle(
                        mtgml_state::LifecycleApplicationError::AllocatorOverflow
                    )
                )
            },
        ),
        (
            "revision_overflow",
            revision_overflow,
            OLD_BATTLEFIELD,
            battlefield_from(),
            owner_graveyard_top(P1),
            ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
            |error| matches!(error, KernelExecutionError::RevisionOverflow),
        ),
        (
            "historical_stale_old",
            historical_after,
            OLD_BATTLEFIELD,
            battlefield_from(),
            owner_graveyard_top(P1),
            ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
            |error| {
                matches!(
                    error,
                    KernelExecutionError::ZoneIncarnation(ZoneIncarnationError::ObjectNotLive)
                )
            },
        ),
    ];

    for (case, before, object, from, to, kind, expected_error) in cases {
        assert_environment_fingerprint_unchanged_for_rejection(
            case,
            &before,
            object,
            from,
            to,
            kind,
            expected_error,
        );
    }

    // This malformed state is deliberately not admitted as an environment
    // checkpoint. Its direct full-state/digest rejection is covered by the
    // request precondition matrix above.
}

#[test]
fn s2_mutant_object_allocator_progression() {
    let (before, valid) = accepted_task2_battlefield_product();

    let mut skipped_cursor = valid.clone();
    skipped_cursor.next_state.allocators.next_object_id = GameObjectId(7);
    rebuild_candidate_delta(&before, &mut skipped_cursor);
    assert_transition_violation(&before, &skipped_cursor, |violation| {
        matches!(
            violation,
            mtgml_rules::TransitionViolation::ObjectAllocatorProgression
        )
    });

    let mut skipped_identity = valid.clone();
    let allocated = GameObjectId(5);
    let unexpected = GameObjectId(6);
    let mut row = skipped_identity
        .next_state
        .zones
        .objects
        .remove(&allocated)
        .unwrap();
    row.id = unexpected;
    skipped_identity
        .next_state
        .zones
        .objects
        .insert(unexpected, row);
    let new_location = skipped_identity
        .next_state
        .zones
        .locations
        .remove(&allocated)
        .unwrap();
    skipped_identity
        .next_state
        .zones
        .locations
        .insert(unexpected, new_location);
    for members in skipped_identity.next_state.zones.ordered_zones.values_mut() {
        for member in members {
            if *member == allocated {
                *member = unexpected;
            }
        }
    }
    let transition = mutated_zone_transition(&mut skipped_identity);
    transition.new_object = unexpected;
    transition.new_snapshot.object = unexpected;
    skipped_identity.next_state.allocators.next_object_id = GameObjectId(7);
    validate_engine_state(&skipped_identity.next_state).unwrap();
    rebuild_candidate_delta(&before, &mut skipped_identity);
    assert_transition_violation(&before, &skipped_identity, |violation| {
        matches!(
            violation,
            mtgml_rules::TransitionViolation::ObjectAllocatorProgression
        )
    });

    let mut reused_live_id = valid.clone();
    let expected_new = GameObjectId(5);
    let reused = GameObjectId(3);
    let new_row = reused_live_id
        .next_state
        .zones
        .objects
        .remove(&expected_new)
        .unwrap();
    let old_graveyard_row = reused_live_id
        .next_state
        .zones
        .objects
        .remove(&reused)
        .unwrap();
    assert_ne!(old_graveyard_row.physical_card, new_row.physical_card);
    reused_live_id.next_state.zones.objects.insert(
        reused,
        GameObject {
            id: reused,
            ..new_row
        },
    );
    reused_live_id
        .next_state
        .zones
        .locations
        .remove(&expected_new);
    reused_live_id.next_state.zones.locations.remove(&reused);
    reused_live_id
        .next_state
        .zones
        .locations
        .insert(reused, owner_graveyard_top(P1));
    let key = owner_graveyard_top(P1).key();
    reused_live_id
        .next_state
        .zones
        .ordered_zones
        .insert(key, vec![reused, GameObjectId(4)]);
    reused_live_id.next_state.zones.locations.insert(
        GameObjectId(4),
        location(
            ZoneKind::Graveyard,
            Some(P1),
            ZonePosition::Top { offset: 1 },
            VisibilityPartition::Public,
        ),
    );
    mutated_zone_transition(&mut reused_live_id).new_object = reused;
    mutated_zone_transition(&mut reused_live_id)
        .new_snapshot
        .object = reused;
    validate_engine_state(&reused_live_id.next_state).unwrap();
    rebuild_candidate_delta(&before, &mut reused_live_id);
    assert_transition_violation(&before, &reused_live_id, |violation| {
        matches!(
            violation,
            mtgml_rules::TransitionViolation::ObjectAllocatorProgression
        )
    });

    let mut unrelated_allocator = valid;
    unrelated_allocator.next_state.allocators.next_effect_id = mtgml_model::EffectInstanceId(2);
    validate_engine_state(&unrelated_allocator.next_state).unwrap();
    rebuild_candidate_delta(&before, &mut unrelated_allocator);
    assert_transition_violation(&before, &unrelated_allocator, |violation| {
        matches!(
            violation,
            mtgml_rules::TransitionViolation::UnrelatedAllocatorProgression
        )
    });
}

#[test]
fn s2_mutant_snapshot_pairing_and_physical_continuity() {
    let (before, valid) = accepted_task2_battlefield_product();

    let mut stale_lki = valid.clone();
    mutated_zone_transition(&mut stale_lki).last_known.tapped = true;
    rebuild_candidate_delta(&before, &mut stale_lki);
    assert_transition_violation(&before, &stale_lki, |violation| {
        matches!(violation, mtgml_rules::TransitionViolation::ZoneTransition)
    });

    let mut wrong_new_snapshot = valid.clone();
    mutated_zone_transition(&mut wrong_new_snapshot)
        .new_snapshot
        .face_down = true;
    rebuild_candidate_delta(&before, &mut wrong_new_snapshot);
    assert_transition_violation(&before, &wrong_new_snapshot, |violation| {
        matches!(violation, mtgml_rules::TransitionViolation::ZoneTransition)
    });

    let mut missing_physical_continuity = valid;
    mutated_zone_transition(&mut missing_physical_continuity).physical_card = None;
    rebuild_candidate_delta(&before, &mut missing_physical_continuity);
    assert_transition_violation(&before, &missing_physical_continuity, |violation| {
        matches!(violation, mtgml_rules::TransitionViolation::ZoneTransition)
    });

    let mut mismatched_physical = accepted_task2_battlefield_product().1;
    let transition = mutated_zone_transition(&mut mismatched_physical);
    transition.physical_card = Some(PhysicalCardId(99));
    transition.new_snapshot.physical_card = Some(PhysicalCardId(99));
    rebuild_candidate_delta(&before, &mut mismatched_physical);
    assert_transition_violation(&before, &mismatched_physical, |violation| {
        matches!(violation, mtgml_rules::TransitionViolation::ZoneTransition)
    });

    let mut reused_old = accepted_task2_battlefield_product().1;
    let transition = mutated_zone_transition(&mut reused_old);
    transition.new_object = OLD_BATTLEFIELD;
    transition.new_snapshot.object = OLD_BATTLEFIELD;
    rebuild_candidate_delta(&before, &mut reused_old);
    assert_transition_violation(&before, &reused_old, |violation| {
        matches!(violation, mtgml_rules::TransitionViolation::ZoneTransition)
    });

    let mut wrong_definition = accepted_task2_battlefield_product().1;
    mutated_zone_transition(&mut wrong_definition)
        .new_snapshot
        .card_definition = CardDefinitionId(99);
    rebuild_candidate_delta(&before, &mut wrong_definition);
    assert_transition_violation(&before, &wrong_definition, |violation| {
        matches!(violation, mtgml_rules::TransitionViolation::ZoneTransition)
    });

    let mut wrong_owner = accepted_task2_battlefield_product().1;
    mutated_zone_transition(&mut wrong_owner).new_snapshot.owner = P2;
    rebuild_candidate_delta(&before, &mut wrong_owner);
    assert_transition_violation(&before, &wrong_owner, |violation| {
        matches!(violation, mtgml_rules::TransitionViolation::ZoneTransition)
    });

    let mut wrong_destination = accepted_task2_battlefield_product().1;
    mutated_zone_transition(&mut wrong_destination).to = location(
        ZoneKind::Exile,
        None,
        ZonePosition::Unordered,
        VisibilityPartition::Public,
    );
    rebuild_candidate_delta(&before, &mut wrong_destination);
    assert_transition_violation(&before, &wrong_destination, |violation| {
        matches!(violation, mtgml_rules::TransitionViolation::ZoneTransition)
    });
}

#[test]
fn s2_mutant_graveyard_and_library_order() {
    let (before_battlefield, valid_battlefield) = accepted_task2_battlefield_product();
    let mut swapped_graveyard = valid_battlefield;
    let graveyard_key = owner_graveyard_top(P1).key();
    swapped_graveyard.next_state.zones.ordered_zones.insert(
        graveyard_key.clone(),
        vec![GameObjectId(5), GameObjectId(4), GameObjectId(3)],
    );
    swapped_graveyard
        .next_state
        .zones
        .locations
        .get_mut(&GameObjectId(3))
        .unwrap()
        .position = ZonePosition::Top { offset: 2 };
    swapped_graveyard
        .next_state
        .zones
        .locations
        .get_mut(&GameObjectId(4))
        .unwrap()
        .position = ZonePosition::Top { offset: 1 };
    validate_engine_state(&swapped_graveyard.next_state).unwrap();
    rebuild_candidate_delta(&before_battlefield, &mut swapped_graveyard);
    assert_transition_violation(&before_battlefield, &swapped_graveyard, |violation| {
        matches!(
            violation,
            mtgml_rules::TransitionViolation::ZoneOrderProgression
        )
    });

    let (before_library, valid_library) = accepted_task2_library_product_with_three_cards();
    let mut swapped_library = valid_library;
    let library_key = owner_library_top(P2).key();
    swapped_library
        .next_state
        .zones
        .ordered_zones
        .insert(library_key, vec![GameObjectId(4), GameObjectId(3)]);
    swapped_library
        .next_state
        .zones
        .locations
        .get_mut(&GameObjectId(3))
        .unwrap()
        .position = ZonePosition::Top { offset: 1 };
    swapped_library
        .next_state
        .zones
        .locations
        .get_mut(&GameObjectId(4))
        .unwrap()
        .position = ZonePosition::Top { offset: 0 };
    validate_engine_state(&swapped_library.next_state).unwrap();
    rebuild_candidate_delta(&before_library, &mut swapped_library);
    assert_transition_violation(&before_library, &swapped_library, |violation| {
        matches!(
            violation,
            mtgml_rules::TransitionViolation::ZoneOrderProgression
        )
    });

    let mut unrelated_before = task2_battlefield_case_state();
    remove_object_tracking(&mut unrelated_before, OLD_LIBRARY_TOP);
    validate_engine_state(&unrelated_before).unwrap();
    let mut unrelated_zone = execute_selected_zone_transition_for_conformance(
        &unrelated_before,
        OLD_BATTLEFIELD,
        battlefield_from(),
        owner_graveyard_top(P1),
        ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
    )
    .unwrap();
    let library_key = owner_library_top(P2).key();
    unrelated_zone
        .next_state
        .zones
        .ordered_zones
        .remove(&library_key);
    unrelated_zone
        .next_state
        .zones
        .locations
        .insert(OLD_LIBRARY_TOP, owner_hand(P2));
    validate_engine_state(&unrelated_zone.next_state).unwrap();
    rebuild_candidate_delta(&unrelated_before, &mut unrelated_zone);
    assert_transition_violation(&unrelated_before, &unrelated_zone, |violation| {
        matches!(
            violation,
            mtgml_rules::TransitionViolation::ZoneOrderProgression
        )
    });
}

#[test]
fn s2_mutant_empty_library_key_rejects() {
    let mut before = base_state();
    before
        .zones
        .objects
        .get_mut(&OLD_LIBRARY_TOP)
        .unwrap()
        .face_down = false;
    remove_object_tracking(&mut before, OLD_LIBRARY_TOP);
    validate_engine_state(&before).unwrap();
    let key = owner_library_top(P2).key();
    let mut result = execute_selected_zone_transition_for_conformance(
        &before,
        OLD_LIBRARY_TOP,
        owner_library_top(P2),
        owner_hand(P2),
        ConformanceZoneTransitionKind::LibraryTopToOwnerHand,
    )
    .unwrap();
    result
        .next_state
        .zones
        .ordered_zones
        .insert(key, Vec::new());
    assert_transition_violation(&before, &result, |violation| {
        matches!(
            violation,
            mtgml_rules::TransitionViolation::AfterState(
                mtgml_state::EngineStateViolation::OrderedZoneMismatch
            )
        )
    });
}

#[test]
fn s2_zone_library_reindexes_all_remaining_members() {
    let (before, result) = accepted_task2_library_product_with_three_cards();
    let key = owner_library_top(P2).key();
    assert_eq!(
        result.next_state.zones.ordered_zones[&key],
        [GameObjectId(3), GameObjectId(4)]
    );
    assert_eq!(
        result.next_state.zones.locations[&GameObjectId(3)].position,
        ZonePosition::Top { offset: 0 }
    );
    assert_eq!(
        result.next_state.zones.locations[&GameObjectId(4)].position,
        ZonePosition::Top { offset: 1 }
    );
    assert_eq!(
        result.next_state.zones.locations[&GameObjectId(5)],
        owner_hand(P2)
    );
    assert_eq!(result.next_state.random, before.random);
    assert_eq!(result.delta.apply(&before).unwrap(), result.next_state);
}

#[test]
fn s2_mutant_foundation_source_transfer() {
    let (before, mut result) = accepted_task2_battlefield_product();
    let new = GameObjectId(5);
    result.next_state.foundation_sources.insert(
        new,
        FoundationCreatureSource {
            source_kind: FoundationSourceKind::Creature,
            base_characteristics: BaseCharacteristics::Simple {
                power: 2,
                toughness: 2,
            },
            marked_damage: 1,
            control_history: ControlHistory::BeforeTurnStart { turn_number: 0 },
        },
    );
    validate_engine_state(&result.next_state).unwrap();
    rebuild_candidate_delta(&before, &mut result);
    assert_transition_violation(&before, &result, |violation| {
        matches!(
            violation,
            mtgml_rules::TransitionViolation::FoundationSourceProgression
        )
    });

    let mut before_with_unrelated_source = battlefield_case_state();
    let unrelated = GameObjectId(6);
    before_with_unrelated_source.zones.objects.insert(
        unrelated,
        GameObject {
            id: unrelated,
            physical_card: Some(PhysicalCardId(6)),
            card_definition: CardDefinitionId(6),
            owner: P2,
            controller: P2,
            tapped: false,
            face_down: false,
        },
    );
    before_with_unrelated_source
        .zones
        .locations
        .insert(unrelated, battlefield_from());
    before_with_unrelated_source.allocators.next_object_id = GameObjectId(7);
    let unrelated_source = FoundationCreatureSource {
        source_kind: FoundationSourceKind::Creature,
        base_characteristics: BaseCharacteristics::Simple {
            power: 3,
            toughness: 3,
        },
        marked_damage: 0,
        control_history: ControlHistory::BeforeTurnStart { turn_number: 0 },
    };
    before_with_unrelated_source
        .foundation_sources
        .insert(unrelated, unrelated_source);
    validate_engine_state(&before_with_unrelated_source).unwrap();
    let mut unrelated_deleted = execute_selected_zone_transition_for_conformance(
        &before_with_unrelated_source,
        OLD_BATTLEFIELD,
        battlefield_from(),
        owner_graveyard_top(P1),
        ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
    )
    .unwrap();
    assert_eq!(
        unrelated_deleted
            .next_state
            .foundation_sources
            .get(&unrelated),
        Some(&unrelated_source)
    );
    unrelated_deleted
        .next_state
        .foundation_sources
        .remove(&unrelated);
    rebuild_candidate_delta(&before_with_unrelated_source, &mut unrelated_deleted);
    assert_transition_violation(
        &before_with_unrelated_source,
        &unrelated_deleted,
        |violation| {
            matches!(
                violation,
                mtgml_rules::TransitionViolation::FoundationSourceProgression
            )
        },
    );
}

#[test]
fn s2_mutant_event_delta_pairing() {
    let (before, mut omitted_audit) = accepted_task2_battlefield_product();
    omitted_audit.delta.audit.clear();
    assert_transition_violation(&before, &omitted_audit, |violation| {
        matches!(
            violation,
            mtgml_rules::TransitionViolation::EventDeltaMismatch
        )
    });

    let (before, mut replacement_mismatch) = accepted_task2_battlefield_product();
    replacement_mismatch.delta.replacement.core.turn_number += 1;
    assert_transition_violation(&before, &replacement_mismatch, |violation| {
        matches!(
            violation,
            mtgml_rules::TransitionViolation::DeltaReapplication
        )
    });

    let (before, mut wrong_event_identity) = accepted_task2_battlefield_product();
    wrong_event_identity.events[0].event_id = mtgml_model::RuleEventId(2);
    assert_transition_violation(&before, &wrong_event_identity, |violation| {
        matches!(violation, mtgml_rules::TransitionViolation::EventIdentity)
    });

    let (before, mut wrong_event_revision) = accepted_task2_battlefield_product();
    wrong_event_revision.events[0].state_revision = mtgml_model::StateRevision(2);
    assert_transition_violation(&before, &wrong_event_revision, |violation| {
        matches!(violation, mtgml_rules::TransitionViolation::EventIdentity)
    });

    let (before, mut skipped_event_cursor) = accepted_task2_battlefield_product();
    skipped_event_cursor
        .next_state
        .allocators
        .next_rule_event_id = mtgml_model::RuleEventId(3);
    rebuild_candidate_delta(&before, &mut skipped_event_cursor);
    assert_transition_violation(&before, &skipped_event_cursor, |violation| {
        matches!(violation, mtgml_rules::TransitionViolation::EventIdentity)
    });

    let (before, mut reordered_occurrences) = {
        let before = battlefield_case_state();
        let result = execute_selected_zone_transition_for_conformance(
            &before,
            OLD_BATTLEFIELD,
            battlefield_from(),
            owner_graveyard_top(P1),
            ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
        )
        .unwrap();
        (before, result)
    };
    reordered_occurrences.events.swap(1, 2);
    reordered_occurrences.delta.audit = reordered_occurrences
        .events
        .iter()
        .map(|event| event.event.semantic_delta())
        .collect();
    assert_transition_violation(&before, &reordered_occurrences, |violation| {
        matches!(
            violation,
            mtgml_rules::TransitionViolation::OccurrencePairing
        )
    });
}

#[test]
fn s2_mutant_lifecycle_pairing_matrix() {
    let (before_public, valid_public) = {
        let before = battlefield_case_state();
        let result = execute_selected_zone_transition_for_conformance(
            &before,
            OLD_BATTLEFIELD,
            battlefield_from(),
            owner_graveyard_top(P1),
            ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
        )
        .unwrap();
        (before, result)
    };

    let mut wrong_public_remap = valid_public.clone();
    let lifecycle = match &mut wrong_public_remap.events[1].event {
        mtgml_rules::AuthoritativeRuleEventKind::PerspectiveOccurrence { lifecycle, .. } => {
            lifecycle
        }
        _ => panic!("expected public lifecycle occurrence"),
    };
    lifecycle.mutation.identity = mtgml_state::IdentityMutationV1::Remap {
        opaque: mtgml_model::OpaqueObjectId(1),
        from_object: OLD_BATTLEFIELD,
        to_object: GameObjectId(6),
    };
    rebuild_candidate_delta(&before_public, &mut wrong_public_remap);
    assert_transition_violation(&before_public, &wrong_public_remap, |violation| {
        matches!(
            violation,
            mtgml_rules::TransitionViolation::OccurrencePairing
        )
    });

    let mut wrong_public_location = valid_public;
    let lifecycle = match &mut wrong_public_location.events[1].event {
        mtgml_rules::AuthoritativeRuleEventKind::PerspectiveOccurrence { lifecycle, .. } => {
            lifecycle
        }
        _ => panic!("expected public lifecycle occurrence"),
    };
    if let Some(mtgml_state::KnowledgeMutationV1::UpdateLocation { fact, .. }) =
        &mut lifecycle.mutation.knowledge
    {
        fact.location = owner_hand(P1);
    } else {
        panic!("expected public location update");
    }
    rebuild_candidate_delta(&before_public, &mut wrong_public_location);
    assert_transition_violation(&before_public, &wrong_public_location, |violation| {
        matches!(
            violation,
            mtgml_rules::TransitionViolation::OccurrencePairing
        )
    });

    let (before_private, mut wrong_private_allocate) = {
        let before = first_private_library_case_state();
        let result = execute_selected_zone_transition_for_conformance(
            &before,
            OLD_LIBRARY_TOP,
            owner_library_top(P2),
            owner_hand(P2),
            ConformanceZoneTransitionKind::LibraryTopToOwnerHand,
        )
        .unwrap();
        (before, result)
    };
    let lifecycle = match &mut wrong_private_allocate.events[1].event {
        mtgml_rules::AuthoritativeRuleEventKind::PerspectiveOccurrence { lifecycle, .. } => {
            lifecycle
        }
        _ => panic!("expected owner private lifecycle occurrence"),
    };
    lifecycle.mutation.identity = mtgml_state::IdentityMutationV1::Allocate {
        opaque: mtgml_model::OpaqueObjectId(3),
        object: OLD_LIBRARY_TOP,
    };
    rebuild_candidate_delta(&before_private, &mut wrong_private_allocate);
    assert_transition_violation(&before_private, &wrong_private_allocate, |violation| {
        matches!(
            violation,
            mtgml_rules::TransitionViolation::OccurrencePairing
        )
    });

    let (before_first_private, mut wrong_allocate_cursor) = {
        let before = first_private_library_case_state();
        let result = execute_selected_zone_transition_for_conformance(
            &before,
            OLD_LIBRARY_TOP,
            owner_library_top(P2),
            owner_hand(P2),
            ConformanceZoneTransitionKind::LibraryTopToOwnerHand,
        )
        .unwrap();
        (before, result)
    };
    let owner_identity = wrong_allocate_cursor
        .next_state
        .perspective_identities
        .players
        .get_mut(&P2)
        .unwrap();
    assert_eq!(
        owner_identity.next_opaque_object_id,
        mtgml_model::OpaqueObjectId(4)
    );
    owner_identity.next_opaque_object_id = mtgml_model::OpaqueObjectId(5);
    validate_engine_state(&wrong_allocate_cursor.next_state)
        .expect("the mutant cursor remains structurally valid");
    rebuild_candidate_delta(&before_first_private, &mut wrong_allocate_cursor);
    assert_transition_violation(&before_first_private, &wrong_allocate_cursor, |violation| {
        matches!(
            violation,
            mtgml_rules::TransitionViolation::OccurrencePairing
        )
    });

    let (before_private, mut non_owner_occurrence) = {
        let before = first_private_library_case_state();
        let result = execute_selected_zone_transition_for_conformance(
            &before,
            OLD_LIBRARY_TOP,
            owner_library_top(P2),
            owner_hand(P2),
            ConformanceZoneTransitionKind::LibraryTopToOwnerHand,
        )
        .unwrap();
        (before, result)
    };
    if let mtgml_rules::AuthoritativeRuleEventKind::PerspectiveOccurrence { lifecycle, .. } =
        &mut non_owner_occurrence.events[1].event
    {
        lifecycle.perspective = P1;
    } else {
        panic!("expected owner private lifecycle occurrence");
    }
    rebuild_candidate_delta(&before_private, &mut non_owner_occurrence);
    assert_transition_violation(&before_private, &non_owner_occurrence, |violation| {
        matches!(
            violation,
            mtgml_rules::TransitionViolation::OccurrencePairing
        )
    });

    let (before_preknown, mut opaque_allocator_mutant) = {
        let before = library_case_state();
        let result = execute_selected_zone_transition_for_conformance(
            &before,
            OLD_LIBRARY_TOP,
            owner_library_top(P2),
            owner_hand(P2),
            ConformanceZoneTransitionKind::LibraryTopToOwnerHand,
        )
        .unwrap();
        (before, result)
    };
    opaque_allocator_mutant
        .next_state
        .perspective_identities
        .players
        .get_mut(&P2)
        .unwrap()
        .next_opaque_object_id = mtgml_model::OpaqueObjectId(4);
    validate_engine_state(&opaque_allocator_mutant.next_state).unwrap();
    rebuild_candidate_delta(&before_preknown, &mut opaque_allocator_mutant);
    assert_transition_violation(&before_preknown, &opaque_allocator_mutant, |violation| {
        matches!(
            violation,
            mtgml_rules::TransitionViolation::OccurrencePairing
        )
    });
}

#[test]
fn s2_mutant_foundation_old_retained_rejects() {
    let before = battlefield_case_state();
    let mut result = execute_selected_zone_transition_for_conformance(
        &before,
        OLD_BATTLEFIELD,
        battlefield_from(),
        owner_graveyard_top(P1),
        ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
    )
    .unwrap();
    result
        .next_state
        .foundation_sources
        .insert(OLD_BATTLEFIELD, before.foundation_sources[&OLD_BATTLEFIELD]);
    assert_transition_violation(&before, &result, |violation| {
        matches!(
            violation,
            mtgml_rules::TransitionViolation::AfterState(
                mtgml_state::EngineStateViolation::FoundationSource
            )
        )
    });
}

#[test]
fn s2_mutant_old_reference_after_state_rejects() {
    let (before, valid) = accepted_task2_battlefield_product();

    let mut combat_reference = valid.clone();
    combat_reference.next_state.combat = Some(mtgml_state::CombatState {
        defending_player: P2,
        attackers: vec![OLD_BATTLEFIELD],
        blockers: BTreeMap::from([(OLD_BATTLEFIELD, None)]),
    });
    assert_transition_violation(&before, &combat_reference, |violation| {
        matches!(
            violation,
            mtgml_rules::TransitionViolation::AfterState(
                mtgml_state::EngineStateViolation::CombatState
            )
        )
    });

    let mut stack_reference = valid.clone();
    let stack_id = mtgml_model::StackObjectId(1);
    stack_reference.next_state.zones.stack_records.insert(
        stack_id,
        mtgml_state::StackRecord {
            id: stack_id,
            controller: P1,
            source_object: Some(OLD_BATTLEFIELD),
            source_ability: None,
        },
    );
    stack_reference.next_state.zones.stack_order.push(stack_id);
    stack_reference.next_state.allocators.next_stack_object_id = mtgml_model::StackObjectId(2);
    assert_transition_violation(&before, &stack_reference, |violation| {
        matches!(
            violation,
            mtgml_rules::TransitionViolation::AfterState(
                mtgml_state::EngineStateViolation::StackMismatch
            )
        )
    });

    let mut ordered_reference = valid.clone();
    ordered_reference
        .next_state
        .zones
        .ordered_zones
        .get_mut(&owner_graveyard_top(P1).key())
        .unwrap()
        .push(OLD_BATTLEFIELD);
    assert_transition_violation(&before, &ordered_reference, |violation| {
        matches!(
            violation,
            mtgml_rules::TransitionViolation::AfterState(
                mtgml_state::EngineStateViolation::OrderedZoneMismatch
            )
        )
    });

    let mut identity_reference = valid.clone();
    let identities = identity_reference
        .next_state
        .perspective_identities
        .players
        .get_mut(&P1)
        .unwrap();
    let opaque = identities.next_opaque_object_id;
    identities.next_opaque_object_id.0 += 1;
    identities.opaque_to_object.insert(opaque, OLD_BATTLEFIELD);
    identities.object_to_opaque.insert(OLD_BATTLEFIELD, opaque);
    assert_transition_violation(&before, &identity_reference, |violation| {
        matches!(
            violation,
            mtgml_rules::TransitionViolation::AfterState(
                mtgml_state::EngineStateViolation::PerspectiveIdentityMismatch
            )
        )
    });

    let pending_fixture = construct_synthetic_engine_state(SyntheticResetInputs {
        players: [P1, P2],
        root_seed: mtgml_random::RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap(),
        setup: SyntheticV4Setup::m2_compatibility(),
    })
    .unwrap();
    let mut pending_reference = valid;
    let mut pending = pending_fixture
        .execution
        .pending_decision
        .expect("synthetic fixture has a trusted pending request");
    pending.request.state_revision = pending_reference.next_state.revision;
    pending_reference.next_state.execution.pending_decision = Some(pending);
    assert_transition_violation(&before, &pending_reference, |violation| {
        matches!(
            violation,
            mtgml_rules::TransitionViolation::AfterState(
                mtgml_state::EngineStateViolation::PendingDecisionMismatch
            )
        )
    });
}

#[test]
fn s2_rejection_stale_old_incarnation_historical_witness() {
    let before = battlefield_case_state();
    // Step 1 uses the production-owned executor and the rich tracked source.
    let first = execute_selected_zone_transition_for_conformance(
        &before,
        OLD_BATTLEFIELD,
        battlefield_from(),
        owner_graveyard_top(P1),
        ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
    )
    .expect("first accepted OLD -> NEW transition");
    let after_first = first.next_state.clone();
    let captured_after_first = after_first.clone();
    let after_first_digest = after_first.digest().unwrap();
    let retry = execute_selected_zone_transition_for_conformance(
        &after_first,
        OLD_BATTLEFIELD,
        battlefield_from(),
        owner_graveyard_top(P1),
        ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
    );
    assert!(matches!(
        retry,
        Err(KernelExecutionError::ZoneIncarnation(
            ZoneIncarnationError::ObjectNotLive
        ))
    ));
    assert_eq!(after_first, captured_after_first);
    assert_eq!(after_first.digest().unwrap(), after_first_digest);
}

#[test]
fn s2_mutant_stale_old_lifecycle_occurrence_is_validator_only() {
    // This is an internal candidate-product mutant, never a typed S2 request.
    // Step 1 is produced by the real S2 executor; only the later trusted audit
    // occurrence is mutated.
    use mtgml_rules::{AuthoritativeRuleEvent, AuthoritativeRuleEventKind};
    use mtgml_state::{
        IdentityMutationV1, PerspectiveLifecycleAuditV1, PerspectiveLifecycleMutationV1,
    };

    let before = battlefield_case_state();
    let mut result = execute_selected_zone_transition_for_conformance(
        &before,
        OLD_BATTLEFIELD,
        battlefield_from(),
        owner_graveyard_top(P1),
        ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
    )
    .unwrap();

    let stale_lifecycle = PerspectiveLifecycleAuditV1 {
        perspective: P1,
        sequence: result.next_state.knowledge.players[&P1].next_visible_sequence,
        mutation: PerspectiveLifecycleMutationV1 {
            identity: IdentityMutationV1::Allocate {
                opaque: mtgml_model::OpaqueObjectId(2),
                object: OLD_BATTLEFIELD,
            },
            knowledge: None,
        },
    };
    result.events.push(AuthoritativeRuleEvent {
        event_id: result.next_state.allocators.next_rule_event_id,
        state_revision: result.next_state.revision,
        event: AuthoritativeRuleEventKind::PerspectiveOccurrence {
            lifecycle: stale_lifecycle.clone(),
            observation: mtgml_rules::PerspectiveObservationPolicyV1::NoEnvelope,
        },
    });
    result
        .delta
        .audit
        .push(mtgml_state::SemanticDeltaOperation::PerspectiveLifecycle {
            lifecycle: stale_lifecycle,
        });
    let error = mtgml_rules::validate_transition_contract(&before, &result).unwrap_err();
    assert!(
        matches!(error, mtgml_rules::TransitionViolation::OccurrencePairing),
        "s2.mutant.stale_old_lifecycle_occurrence: expected contract rejection for OLD after ZoneTransition"
    );
}

#[test]
fn s2_valid_requests_preserve_input_and_rng() {
    let battlefield = task2_battlefield_case_state();
    let library = task2_library_case_state();
    let battlefield_digest = battlefield.digest().unwrap();
    let library_digest = library.digest().unwrap();
    let battlefield_rng = battlefield.random.clone();
    let library_rng = library.random.clone();
    let battlefield_result = execute_selected_zone_transition_for_conformance(
        &battlefield,
        OLD_BATTLEFIELD,
        battlefield_from(),
        owner_graveyard_top(P1),
        ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
    )
    .unwrap();
    let library_result = execute_selected_zone_transition_for_conformance(
        &library,
        OLD_LIBRARY_TOP,
        owner_library_top(P2),
        owner_hand(P2),
        ConformanceZoneTransitionKind::LibraryTopToOwnerHand,
    )
    .unwrap();
    assert!(battlefield_result.accepted);
    assert!(library_result.accepted);
    assert_eq!(battlefield_result.next_state.random, battlefield_rng);
    assert_eq!(library_result.next_state.random, library_rng);
    assert_eq!(battlefield.digest().unwrap(), battlefield_digest);
    assert_eq!(library.digest().unwrap(), library_digest);
    assert_eq!(battlefield.random, battlefield_rng);
    assert_eq!(library.random, library_rng);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum S2ParityScenario {
    BattlefieldTracked,
    LibraryFirstPrivate,
    LibraryPreknown,
    LibraryFirstPrivateSingleton,
}

fn s2_scenario_state(scenario: S2ParityScenario) -> EngineState {
    match scenario {
        S2ParityScenario::BattlefieldTracked => battlefield_case_state(),
        S2ParityScenario::LibraryFirstPrivate => first_private_library_case_state(),
        S2ParityScenario::LibraryPreknown => library_case_state(),
        S2ParityScenario::LibraryFirstPrivateSingleton => {
            let mut state = first_private_library_case_state();
            let ordered = state
                .zones
                .ordered_zones
                .get_mut(&owner_library_top(P2).key())
                .expect("authored Library exists");
            let removed = ordered.pop().expect("authored Library has a second member");
            remove_object_tracking(&mut state, removed);
            state.zones.objects.remove(&removed).unwrap();
            state.zones.locations.remove(&removed).unwrap();
            validate_engine_state(&state).unwrap();
            state
        }
    }
}

fn s2_scenario_request(
    scenario: S2ParityScenario,
    before: &EngineState,
) -> (
    GameObjectId,
    ZoneLocation,
    ZoneLocation,
    ConformanceZoneTransitionKind,
) {
    match scenario {
        S2ParityScenario::BattlefieldTracked => (
            OLD_BATTLEFIELD,
            battlefield_from(),
            owner_graveyard_top(P1),
            ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
        ),
        S2ParityScenario::LibraryFirstPrivate
        | S2ParityScenario::LibraryPreknown
        | S2ParityScenario::LibraryFirstPrivateSingleton => {
            let top = before
                .zones
                .ordered_zones
                .get(&owner_library_top(P2).key())
                .and_then(|objects| objects.first())
                .copied()
                .expect("authored owner Library has a top card");
            (
                top,
                owner_library_top(P2),
                owner_hand(P2),
                ConformanceZoneTransitionKind::LibraryTopToOwnerHand,
            )
        }
    }
}

fn execute_s2_scenario(
    before: &EngineState,
    scenario: S2ParityScenario,
) -> mtgml_rules::TransitionResult {
    let (object, from, to, kind) = s2_scenario_request(scenario, before);
    execute_selected_zone_transition_for_conformance(before, object, from, to, kind)
        .expect("selected S2 scenario is accepted by the production-owned executor")
}

fn s2_pair_unchanged(
    _state: &mut EngineState,
) -> Result<crate::isolation::TransformReport, crate::isolation::HarnessError> {
    Ok(crate::isolation::TransformReport {
        mutated_fields: &[],
    })
}

fn s2_pair_rename_hidden_library_top(
    state: &mut EngineState,
) -> Result<crate::isolation::TransformReport, crate::isolation::HarnessError> {
    let mut candidate = state.clone();
    let renamed = GameObjectId(9);
    let mut object = candidate
        .zones
        .objects
        .remove(&OLD_LIBRARY_TOP)
        .ok_or(crate::isolation::HarnessError::TransformFixtureAbsent)?;
    object.id = renamed;
    candidate.zones.objects.insert(renamed, object);
    let location = candidate
        .zones
        .locations
        .remove(&OLD_LIBRARY_TOP)
        .ok_or(crate::isolation::HarnessError::TransformFixtureAbsent)?;
    candidate.zones.locations.insert(renamed, location);
    for members in candidate.zones.ordered_zones.values_mut() {
        for member in members {
            if *member == OLD_LIBRARY_TOP {
                *member = renamed;
            }
        }
    }
    for identity in candidate.perspective_identities.players.values_mut() {
        for object in identity.opaque_to_object.values_mut() {
            if *object == OLD_LIBRARY_TOP {
                *object = renamed;
            }
        }
        if let Some(opaque) = identity.object_to_opaque.remove(&OLD_LIBRARY_TOP) {
            identity.object_to_opaque.insert(renamed, opaque);
        }
    }
    candidate.allocators.next_object_id = GameObjectId(
        renamed
            .0
            .checked_add(1)
            .ok_or(crate::isolation::HarnessError::TransformPreconditionViolated)?,
    );
    *state = candidate;
    Ok(crate::isolation::TransformReport {
        mutated_fields: &["zones", "allocators", "perspective_identities"],
    })
}

fn assert_s2_family_state(before: &EngineState, after: &EngineState, scenario: S2ParityScenario) {
    let new = before.allocators.next_object_id;
    assert_eq!(after.allocators.next_object_id.0, new.0 + 1);
    match scenario {
        S2ParityScenario::BattlefieldTracked => {
            let key = owner_graveyard_top(P1).key();
            let old_members = before.zones.ordered_zones.get(&key).unwrap();
            let new_members = after.zones.ordered_zones.get(&key).unwrap();
            let mut expected = vec![new];
            expected.extend(old_members.iter().copied());
            assert_eq!(new_members, &expected);
            assert!(!after.zones.objects.contains_key(&OLD_BATTLEFIELD));
            assert!(!after.zones.locations.contains_key(&OLD_BATTLEFIELD));
            assert!(!after.foundation_sources.contains_key(&OLD_BATTLEFIELD));
            assert!(!after.foundation_sources.contains_key(&new));
            for (offset, object) in new_members.iter().enumerate() {
                assert_eq!(
                    after.zones.locations[object].position,
                    ZonePosition::Top {
                        offset: u32::try_from(offset).unwrap()
                    }
                );
            }
        }
        S2ParityScenario::LibraryFirstPrivate
        | S2ParityScenario::LibraryPreknown
        | S2ParityScenario::LibraryFirstPrivateSingleton => {
            let key = owner_library_top(P2).key();
            let old_members = before.zones.ordered_zones.get(&key).unwrap();
            let expected: Vec<_> = old_members.iter().copied().skip(1).collect();
            if expected.is_empty() {
                assert!(!after.zones.ordered_zones.contains_key(&key));
            } else {
                assert_eq!(after.zones.ordered_zones.get(&key), Some(&expected));
                for (offset, object) in expected.iter().enumerate() {
                    assert_eq!(
                        after.zones.locations[object].position,
                        ZonePosition::Top {
                            offset: u32::try_from(offset).unwrap()
                        }
                    );
                }
            }
            let (old, _, _, _) = s2_scenario_request(scenario, before);
            assert!(!after.zones.objects.contains_key(&old));
            assert!(!after.zones.locations.contains_key(&old));
            assert_eq!(after.zones.locations[&new], owner_hand(P2));
            assert!(!after
                .zones
                .ordered_zones
                .values()
                .any(|objects| objects.contains(&new)));

            assert_eq!(
                after.perspective_identities.players[&P1],
                before.perspective_identities.players[&P1]
            );
            assert_eq!(after.knowledge.players[&P1], before.knowledge.players[&P1]);

            let old_owner_identity = &before.perspective_identities.players[&P2];
            let new_owner_identity = &after.perspective_identities.players[&P2];
            let old_owner_knowledge = &before.knowledge.players[&P2];
            let new_owner_knowledge = &after.knowledge.players[&P2];
            let sequence = old_owner_knowledge.next_visible_sequence;
            assert_eq!(new_owner_knowledge.next_visible_sequence.0, sequence.0 + 1);
            let (old, _, _, _) = s2_scenario_request(scenario, before);
            if let Some(opaque) = old_owner_identity.object_to_opaque.get(&old).copied() {
                assert_eq!(new_owner_identity.object_to_opaque.get(&new), Some(&opaque));
                assert!(!new_owner_identity.object_to_opaque.contains_key(&old));
                assert_eq!(
                    new_owner_identity.next_opaque_object_id,
                    old_owner_identity.next_opaque_object_id
                );
                let old_record = &old_owner_knowledge.active[&opaque];
                let new_record = &new_owner_knowledge.active[&opaque];
                assert_eq!(
                    new_record.historical_locations.last(),
                    old_record.known_location.as_ref()
                );
                let current = new_record.known_location.as_ref().unwrap();
                assert_eq!(current.location, owner_hand(P2));
                assert_eq!(
                    current.provenance,
                    mtgml_state::KnowledgeAcquisitionReason::Observed {
                        channel: mtgml_state::KnowledgeHistoryChannel::Private,
                        sequence,
                        cause: mtgml_state::KnowledgeAcquisitionCause::OwnPrivateIdentity,
                    }
                );
            } else {
                let opaque = old_owner_identity.next_opaque_object_id;
                assert_eq!(new_owner_identity.object_to_opaque.get(&new), Some(&opaque));
                assert!(!new_owner_identity.object_to_opaque.contains_key(&old));
                assert_eq!(new_owner_identity.next_opaque_object_id.0, opaque.0 + 1);
                let record = &new_owner_knowledge.active[&opaque];
                assert_eq!(
                    record.card_definition,
                    Some(after.zones.objects[&new].card_definition)
                );
                assert!(record.historical_locations.is_empty());
                let current = record.known_location.as_ref().unwrap();
                assert_eq!(current.location, owner_hand(P2));
                assert_eq!(
                    current.provenance,
                    mtgml_state::KnowledgeAcquisitionReason::Observed {
                        channel: mtgml_state::KnowledgeHistoryChannel::Private,
                        sequence,
                        cause: mtgml_state::KnowledgeAcquisitionCause::OwnPrivateIdentity,
                    }
                );
            }
        }
    }
}

fn assert_replay_segment_anchored(
    replay: &mtgml_replay::AuthoritativeReplayV6,
    checkpoint: &mtgml_environment::EnvironmentCheckpointV6,
) {
    assert!(replay.steps.is_empty());
    let anchor = &replay.manifest.initial_identity;
    assert_eq!(anchor.state_revision, checkpoint.state.revision);
    assert_eq!(anchor.full_state_digest, checkpoint.state_digest);
    assert_eq!(anchor.episode_status, checkpoint.status);
    assert_eq!(anchor.environment_limit_counters, checkpoint.limit_counters);
    assert_eq!(anchor.checkpoint_codec_identity, checkpoint.codec);
    assert_eq!(anchor.checkpoint_digest, checkpoint.checkpoint_digest);
    assert_eq!(anchor.execution_identity, checkpoint.execution_identity);
}

fn s2_checkpoint_restore_case(scenario: S2ParityScenario) {
    let before = s2_scenario_state(scenario);
    let config = crate::isolation::synthetic_environment_config([P1, P2]);
    let (input_controller, _) =
        crate::isolation::spawn_environment(before.clone(), &config).unwrap();
    let input_checkpoint = input_controller.checkpoint().unwrap();
    assert_eq!(input_checkpoint.state, before);
    let result = execute_s2_scenario(&input_checkpoint.state, scenario);
    assert_eq!(result.next_state.random, input_checkpoint.state.random);
    assert_s2_family_state(&input_checkpoint.state, &result.next_state, scenario);

    let (controller, endpoints) =
        crate::isolation::spawn_environment(result.next_state.clone(), &config).unwrap();
    let checkpoint = controller.checkpoint().unwrap();
    assert_eq!(checkpoint.state, result.next_state);
    assert_eq!(checkpoint.state_digest, result.next_state.digest().unwrap());
    assert_eq!(checkpoint, controller.checkpoint().unwrap());
    let before_restore = crate::isolation::capture_complete(&controller, &endpoints).unwrap();
    controller.restore(checkpoint.clone()).unwrap();
    let after_restore = crate::isolation::capture_complete(&controller, &endpoints).unwrap();
    crate::isolation::assert_fingerprint_policies(
        &before_restore,
        &after_restore,
        crate::isolation::FingerprintComparison::ExcludeReplayRecorder,
    )
    .unwrap();
    assert_eq!(controller.checkpoint().unwrap(), checkpoint);
    assert_s2_family_state(
        &input_checkpoint.state,
        &controller.checkpoint().unwrap().state,
        scenario,
    );
    let restored_replay = controller.export_replay().unwrap();
    assert_replay_segment_anchored(&restored_replay, &checkpoint);
}

#[test]
// Stable S2 case: `s2.replay.checkpoint_restore` (Battlefield family).
fn s2_replay_checkpoint_restore_battlefield() {
    s2_checkpoint_restore_case(S2ParityScenario::BattlefieldTracked);
}

#[test]
// Stable S2 case: `s2.replay.checkpoint_restore` (Library family).
fn s2_replay_checkpoint_restore_library() {
    s2_checkpoint_restore_case(S2ParityScenario::LibraryFirstPrivate);
    s2_checkpoint_restore_case(S2ParityScenario::LibraryFirstPrivateSingleton);
}

fn s2_fork_parity_case(scenario: S2ParityScenario) {
    let before = s2_scenario_state(scenario);
    let result = execute_s2_scenario(&before, scenario);
    assert_eq!(result.next_state.random, before.random);
    assert_s2_family_state(&before, &result.next_state, scenario);

    let config = crate::isolation::synthetic_environment_config([P1, P2]);
    let (source, source_endpoints) =
        crate::isolation::spawn_environment(result.next_state.clone(), &config).unwrap();
    let source_checkpoint = source.checkpoint().unwrap();
    assert_eq!(source_checkpoint.state, result.next_state);
    let source_before = crate::isolation::capture_complete(&source, &source_endpoints).unwrap();
    let fork = source.fork().unwrap();
    let fork_endpoints = [fork.bind_player(P1).unwrap(), fork.bind_player(P2).unwrap()];
    let fork_checkpoint = fork.checkpoint().unwrap();
    let fork_fingerprint = crate::isolation::capture_complete(&fork, &fork_endpoints).unwrap();
    crate::isolation::assert_fingerprint_policies(
        &source_before,
        &fork_fingerprint,
        crate::isolation::FingerprintComparison::ExcludeReplayRecorder,
    )
    .unwrap();
    assert_eq!(source_checkpoint, fork_checkpoint);
    let fork_replay = fork.export_replay().unwrap();
    assert_replay_segment_anchored(&fork_replay, &source_checkpoint);
    assert_eq!(
        crate::isolation::capture_complete(&source, &source_endpoints).unwrap(),
        source_before,
        "forking must not mutate the source environment"
    );
    assert_s2_family_state(&before, &fork_checkpoint.state, scenario);
}

#[test]
// Stable S2 case: `s2.replay.fork` (Battlefield family).
fn s2_replay_fork_battlefield() {
    s2_fork_parity_case(S2ParityScenario::BattlefieldTracked);
}

#[test]
// Stable S2 case: `s2.replay.fork` (Library family).
fn s2_replay_fork_library() {
    s2_fork_parity_case(S2ParityScenario::LibraryFirstPrivate);
    s2_fork_parity_case(S2ParityScenario::LibraryPreknown);
}

fn s2_deterministic_rerun_case(scenario: S2ParityScenario) {
    let before = s2_scenario_state(scenario);
    let config = crate::isolation::synthetic_environment_config([P1, P2]);
    let (input_controller, _) =
        crate::isolation::spawn_environment(before.clone(), &config).unwrap();
    let checkpoint = input_controller.checkpoint().unwrap();
    assert_eq!(checkpoint.state, before);
    assert_eq!(input_controller.checkpoint().unwrap(), checkpoint);
    let before_state = checkpoint.state.clone();
    let request = s2_scenario_request(scenario, &checkpoint.state);
    assert_eq!(request, s2_scenario_request(scenario, &before_state));
    let first = execute_selected_zone_transition_for_conformance(
        &before_state,
        request.0,
        request.1.clone(),
        request.2.clone(),
        request.3,
    )
    .expect("first identical direct S2 request is accepted");
    let second = execute_selected_zone_transition_for_conformance(
        &checkpoint.state,
        request.0,
        request.1.clone(),
        request.2.clone(),
        request.3,
    )
    .expect("second identical direct S2 request is accepted");
    assert_eq!(
        first, second,
        "identical checkpoint/request reruns must match"
    );
    assert!(first.accepted);
    assert_eq!(
        first.next_state.digest().unwrap(),
        second.next_state.digest().unwrap()
    );
    assert_eq!(first.next_state.random, before_state.random);
    assert_eq!(second.next_state.random, before_state.random);
    assert_eq!(first.next_state.allocators, second.next_state.allocators);
    assert_s2_family_state(&before_state, &first.next_state, scenario);
    let first_projection = mtgml_environment::lifecycle_projection::project_occurrence_envelopes(
        &before_state,
        &first.next_state,
        &first.events,
    )
    .unwrap();
    let second_projection = mtgml_environment::lifecycle_projection::project_occurrence_envelopes(
        &before_state,
        &second.next_state,
        &second.events,
    )
    .unwrap();
    assert_eq!(first_projection, second_projection);

    let (first_controller, first_endpoints) =
        crate::isolation::spawn_environment(first.next_state.clone(), &config).unwrap();
    let (second_controller, second_endpoints) =
        crate::isolation::spawn_environment(second.next_state.clone(), &config).unwrap();
    let first_checkpoint = first_controller.checkpoint().unwrap();
    let second_checkpoint = second_controller.checkpoint().unwrap();
    assert_eq!(first_checkpoint, second_checkpoint);
    assert_eq!(
        first_checkpoint.state_digest,
        second_checkpoint.state_digest
    );
    let first_visible =
        crate::isolation::capture_complete(&first_controller, &first_endpoints).unwrap();
    let second_visible =
        crate::isolation::capture_complete(&second_controller, &second_endpoints).unwrap();
    crate::isolation::assert_fingerprint_policies(
        &first_visible,
        &second_visible,
        crate::isolation::FingerprintComparison::All,
    )
    .unwrap();
}

#[test]
// Stable S2 case: `s2.replay.rerun`; this is deterministic rerun, not replay.
fn s2_replay_rerun() {
    s2_deterministic_rerun_case(S2ParityScenario::BattlefieldTracked);
    s2_deterministic_rerun_case(S2ParityScenario::LibraryFirstPrivate);
    s2_deterministic_rerun_case(S2ParityScenario::LibraryPreknown);
}

#[test]
// Stable S2 case: `s2.observation.library_noninterference`.
fn s2_observation_library_noninterference_pair() {
    let base = library_case_state();
    let paired = crate::isolation::build_case(
        "s2.observation.library_noninterference",
        crate::isolation::AxisKind::ObjectRenaming,
        &base,
        s2_pair_unchanged,
        s2_pair_rename_hidden_library_top,
        crate::isolation::PairWitness::new(
            P1,
            Some(crate::isolation::TrustedRenamingBijection {
                objects: BTreeMap::from([(OLD_LIBRARY_TOP, GameObjectId(9))]),
                abilities: BTreeMap::new(),
            }),
            crate::isolation::NonVacuityPredicate::ObjectRenaming,
        ),
    )
    .unwrap();
    let source_key = owner_library_top(P2).key();
    let old_a = paired.state_a.zones.ordered_zones[&source_key][0];
    let old_b = paired.state_b.zones.ordered_zones[&source_key][0];
    assert_ne!(paired.state_a, paired.state_b);
    assert_ne!(old_a, old_b, "the moved hidden top itself must differ");
    let before_bijection = crate::isolation::TrustedRenamingBijection {
        objects: BTreeMap::from([(old_a, old_b)]),
        abilities: BTreeMap::new(),
    };
    let before_witness = crate::isolation::PairWitness::new(
        P1,
        Some(before_bijection),
        crate::isolation::NonVacuityPredicate::ObjectRenaming,
    );
    crate::isolation::assert_witness(&paired.state_a, &paired.state_b, &before_witness).unwrap();

    let config = crate::isolation::synthetic_environment_config([P1, P2]);
    let (before_controller_a, endpoints_a) =
        crate::isolation::spawn_environment(paired.state_a.clone(), &config).unwrap();
    let (before_controller_b, endpoints_b) =
        crate::isolation::spawn_environment(paired.state_b.clone(), &config).unwrap();
    let before_fingerprint_a =
        crate::isolation::capture_complete(&before_controller_a, &endpoints_a).unwrap();
    let before_fingerprint_b =
        crate::isolation::capture_complete(&before_controller_b, &endpoints_b).unwrap();
    assert_eq!(
        before_fingerprint_a.player.p1_snapshot,
        before_fingerprint_b.player.p1_snapshot
    );

    let request_a = (
        old_a,
        owner_library_top(P2),
        owner_hand(P2),
        ConformanceZoneTransitionKind::LibraryTopToOwnerHand,
    );
    let request_b = (
        old_b,
        owner_library_top(P2),
        owner_hand(P2),
        ConformanceZoneTransitionKind::LibraryTopToOwnerHand,
    );
    let result_a = execute_selected_zone_transition_for_conformance(
        &paired.state_a,
        request_a.0,
        request_a.1.clone(),
        request_a.2.clone(),
        request_a.3,
    )
    .unwrap();
    let result_b = execute_selected_zone_transition_for_conformance(
        &paired.state_b,
        request_b.0,
        request_b.1.clone(),
        request_b.2.clone(),
        request_b.3,
    )
    .unwrap();
    assert_eq!(result_a.next_state.random, paired.state_a.random);
    assert_eq!(result_b.next_state.random, paired.state_b.random);
    let new_a = result_a
        .events
        .iter()
        .find_map(|event| match &event.event {
            mtgml_rules::AuthoritativeRuleEventKind::ZoneTransition { transition } => {
                Some(transition.new_object)
            }
            _ => None,
        })
        .unwrap();
    let new_b = result_b
        .events
        .iter()
        .find_map(|event| match &event.event {
            mtgml_rules::AuthoritativeRuleEventKind::ZoneTransition { transition } => {
                Some(transition.new_object)
            }
            _ => None,
        })
        .unwrap();
    assert_ne!(new_a, new_b);
    let after_bijection = crate::isolation::TrustedRenamingBijection {
        objects: BTreeMap::from([(new_a, new_b)]),
        abilities: BTreeMap::new(),
    };
    let after_witness = crate::isolation::PairWitness::new(
        P1,
        Some(after_bijection),
        crate::isolation::NonVacuityPredicate::ObjectRenaming,
    );
    crate::isolation::assert_witness(&result_a.next_state, &result_b.next_state, &after_witness)
        .unwrap();

    let projected_a = mtgml_environment::lifecycle_projection::project_occurrence_envelopes(
        &paired.state_a,
        &result_a.next_state,
        &result_a.events,
    )
    .unwrap();
    let projected_b = mtgml_environment::lifecycle_projection::project_occurrence_envelopes(
        &paired.state_b,
        &result_b.next_state,
        &result_b.events,
    )
    .unwrap();
    let events_a = &projected_a[&P1];
    let events_b = &projected_b[&P1];
    assert!(events_a.is_empty());
    assert!(events_b.is_empty());
    let event_bytes = |events: &[mtgml_observation::ObservedEventEnvelopeV2]| {
        events
            .iter()
            .map(|event| mtgml_wire::encode_canonical(event).unwrap())
            .collect::<Vec<_>>()
    };
    assert_eq!(event_bytes(events_a), event_bytes(events_b));
    assert_eq!(
        result_a.next_state.knowledge.players[&P1].next_visible_sequence,
        paired.state_a.knowledge.players[&P1].next_visible_sequence
    );
    assert_eq!(
        result_b.next_state.knowledge.players[&P1].next_visible_sequence,
        paired.state_b.knowledge.players[&P1].next_visible_sequence
    );

    let (after_controller_a, after_endpoints_a) =
        crate::isolation::spawn_environment(result_a.next_state.clone(), &config).unwrap();
    let (after_controller_b, after_endpoints_b) =
        crate::isolation::spawn_environment(result_b.next_state.clone(), &config).unwrap();
    let after_fingerprint_a =
        crate::isolation::capture_complete(&after_controller_a, &after_endpoints_a).unwrap();
    let after_fingerprint_b =
        crate::isolation::capture_complete(&after_controller_b, &after_endpoints_b).unwrap();
    assert_eq!(
        after_fingerprint_a.player.p1_snapshot,
        after_fingerprint_b.player.p1_snapshot
    );
    assert_eq!(
        after_fingerprint_a
            .player
            .p1_snapshot
            .current_visible_sequence,
        before_fingerprint_a
            .player
            .p1_snapshot
            .current_visible_sequence
    );
    assert_eq!(
        after_fingerprint_b
            .player
            .p1_snapshot
            .current_visible_sequence,
        before_fingerprint_b
            .player
            .p1_snapshot
            .current_visible_sequence
    );
    assert_eq!(
        after_fingerprint_a
            .player
            .p1_snapshot
            .current_visible_sequence
            .0
            .checked_sub(
                before_fingerprint_a
                    .player
                    .p1_snapshot
                    .current_visible_sequence
                    .0,
            ),
        Some(0)
    );
    assert_eq!(
        after_fingerprint_b
            .player
            .p1_snapshot
            .current_visible_sequence
            .0
            .checked_sub(
                before_fingerprint_b
                    .player
                    .p1_snapshot
                    .current_visible_sequence
                    .0,
            ),
        Some(0)
    );
}
