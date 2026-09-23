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
    assert_eq!(refs["zones.objects"], true, "{CASE_ID}");
    assert_eq!(refs["zones.locations"], true, "{CASE_ID}");
    assert_eq!(refs["ordered_zones"], false, "{CASE_ID}");
    assert_eq!(refs["foundation_sources"], true, "{CASE_ID}");
    assert_eq!(refs["combat.attackers"], false, "{CASE_ID}");
    assert_eq!(refs["combat.blocker_keys_or_values"], false, "{CASE_ID}");
    assert_eq!(refs["stack_records"], false, "{CASE_ID}");
    assert_eq!(refs["pending_trusted_bindings"], false, "{CASE_ID}");
    assert_eq!(refs["perspective_live_mappings"], true, "{CASE_ID}");
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
    let combat_before = combat.clone();
    assert!(matches!(
        execute_selected_zone_transition_for_conformance(
            &combat,
            OLD_BATTLEFIELD,
            battlefield_from(),
            owner_graveyard_top(P1),
            ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
        ),
        Err(KernelExecutionError::ZoneIncarnation(
            ZoneIncarnationError::CombatReference
        ))
    ));
    assert_eq!(combat, combat_before);

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
    let stack_before = stack.clone();
    assert!(matches!(
        execute_selected_zone_transition_for_conformance(
            &stack,
            OLD_BATTLEFIELD,
            battlefield_from(),
            owner_graveyard_top(P1),
            ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
        ),
        Err(KernelExecutionError::ZoneIncarnation(
            ZoneIncarnationError::StackSourceReference
        ))
    ));
    assert_eq!(stack, stack_before);

    let pending = construct_synthetic_engine_state(SyntheticResetInputs {
        players: [P1, P2],
        root_seed: mtgml_random::RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap(),
        setup: SyntheticV4Setup::m2_compatibility(),
    })
    .unwrap();
    validate_engine_state(&pending).unwrap();
    let pending_before = pending.clone();
    assert!(matches!(
        execute_selected_zone_transition_for_conformance(
            &pending,
            OLD_BATTLEFIELD,
            battlefield_from(),
            owner_graveyard_top(P1),
            ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
        ),
        Err(KernelExecutionError::ZoneIncarnation(
            ZoneIncarnationError::PendingDecisionReference
        ))
    ));
    assert_eq!(pending, pending_before);
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
    let before_digest = before.digest().unwrap();

    assert!(matches!(
        execute_selected_zone_transition_for_conformance(
            &before,
            OLD_LIBRARY_TOP,
            owner_library_top(P2),
            owner_hand(P2),
            ConformanceZoneTransitionKind::LibraryTopToOwnerHand,
        ),
        Err(KernelExecutionError::ZoneIncarnation(
            ZoneIncarnationError::NonOwnerTracksHiddenSource
        ))
    ));
    assert_eq!(before.digest().unwrap(), before_digest);
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
    let before_digest = before.digest().unwrap();
    let before_rng = before.random.clone();
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

    assert!(matches!(
        execute_selected_zone_transition_for_conformance(
            &before,
            OLD_BATTLEFIELD,
            mismatched_source,
            owner_graveyard_top(P1),
            ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
        ),
        Err(KernelExecutionError::ZoneIncarnation(
            ZoneIncarnationError::ClaimedSourceLocationMismatch
        ))
    ));
    assert!(matches!(
        execute_selected_zone_transition_for_conformance(
            &before,
            OLD_BATTLEFIELD,
            battlefield_from(),
            wrong_owner_graveyard,
            ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
        ),
        Err(KernelExecutionError::ZoneIncarnation(
            ZoneIncarnationError::DestinationMismatch
        ))
    ));
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
    assert!(matches!(
        execute_selected_zone_transition_for_conformance(
            &unadmitted_before,
            OLD_BATTLEFIELD,
            unadmitted_source,
            battlefield_from(),
            ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
        ),
        Err(KernelExecutionError::ZoneIncarnation(
            ZoneIncarnationError::UnadmittedSourceFamily
        ))
    ));
    let library_before = library_case_state();
    assert!(matches!(
        execute_selected_zone_transition_for_conformance(
            &library_before,
            OLD_LIBRARY_TOP,
            owner_library_top(P2),
            wrong_owner_hand,
            ConformanceZoneTransitionKind::LibraryTopToOwnerHand,
        ),
        Err(KernelExecutionError::ZoneIncarnation(
            ZoneIncarnationError::DestinationMismatch
        ))
    ));
    assert_eq!(before.digest().unwrap(), before_digest);
    assert_eq!(before.random, before_rng);
}

#[test]
fn s2_task2_request_preconditions_fail_closed() {
    let absent_state = task2_battlefield_case_state();
    assert!(matches!(
        execute_selected_zone_transition_for_conformance(
            &absent_state,
            GameObjectId(99),
            battlefield_from(),
            owner_graveyard_top(P1),
            ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
        ),
        Err(KernelExecutionError::ZoneIncarnation(
            ZoneIncarnationError::ObjectNotLive
        ))
    ));

    let mut no_physical = task2_battlefield_case_state();
    no_physical
        .zones
        .objects
        .get_mut(&OLD_BATTLEFIELD)
        .unwrap()
        .physical_card = None;
    validate_engine_state(&no_physical).unwrap();
    assert!(matches!(
        execute_selected_zone_transition_for_conformance(
            &no_physical,
            OLD_BATTLEFIELD,
            battlefield_from(),
            owner_graveyard_top(P1),
            ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
        ),
        Err(KernelExecutionError::ZoneIncarnation(
            ZoneIncarnationError::PhysicalCardRequired
        ))
    ));

    let library = task2_library_case_state();
    let non_top = GameObjectId(3);
    let non_top_from = location(
        ZoneKind::Library,
        Some(P2),
        ZonePosition::Top { offset: 1 },
        VisibilityPartition::FaceDown,
    );
    assert!(matches!(
        execute_selected_zone_transition_for_conformance(
            &library,
            non_top,
            non_top_from,
            owner_hand(P2),
            ConformanceZoneTransitionKind::LibraryTopToOwnerHand,
        ),
        Err(KernelExecutionError::ZoneIncarnation(
            ZoneIncarnationError::LibrarySourceNotTop
        ))
    ));

    let mut exhausted = task2_battlefield_case_state();
    exhausted.allocators.next_object_id = GameObjectId(u64::MAX);
    validate_engine_state(&exhausted).unwrap();
    let exhausted_digest = exhausted.digest().unwrap();
    assert!(matches!(
        execute_selected_zone_transition_for_conformance(
            &exhausted,
            OLD_BATTLEFIELD,
            battlefield_from(),
            owner_graveyard_top(P1),
            ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
        ),
        Err(KernelExecutionError::IdentityAllocation(
            mtgml_state::IdentityAllocationError::GameObjectIdExhausted
        ))
    ));
    assert_eq!(exhausted.allocators.next_object_id, GameObjectId(u64::MAX));
    assert_eq!(exhausted.digest().unwrap(), exhausted_digest);

    let mut invalid_before = task2_battlefield_case_state();
    invalid_before.zones.locations.remove(&OLD_BATTLEFIELD);
    assert!(matches!(
        execute_selected_zone_transition_for_conformance(
            &invalid_before,
            OLD_BATTLEFIELD,
            battlefield_from(),
            owner_graveyard_top(P1),
            ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
        ),
        Err(KernelExecutionError::BeforeState(_))
    ));
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
    // FixtureTransition supplies only a valid base ZoneTransition product;
    // this test mutates its trusted audit and calls the real validator.
    use mtgml_model::{RuleEventId, StateRevision, VisibleSequence};
    use mtgml_rules::fixture_support::FixtureTransition;
    use mtgml_rules::{AuthoritativeRuleEvent, AuthoritativeRuleEventKind};
    use mtgml_state::{
        IdentityMutationV1, PerspectiveLifecycleAuditV1, PerspectiveLifecycleMutationV1,
    };

    let mut before = base_state();
    let old = GameObjectId(3);
    before.zones.objects.insert(
        old,
        GameObject {
            id: old,
            physical_card: Some(PhysicalCardId(3)),
            card_definition: CardDefinitionId(3),
            owner: P1,
            controller: P1,
            tapped: false,
            face_down: false,
        },
    );
    before.zones.locations.insert(
        old,
        location(
            ZoneKind::Exile,
            None,
            ZonePosition::Unordered,
            VisibilityPartition::Public,
        ),
    );
    before.allocators.next_object_id = GameObjectId(4);
    validate_engine_state(&before).unwrap();
    let mut candidate = FixtureTransition::start(&before).unwrap();
    candidate
        .move_object_incarnation(
            old,
            location(
                ZoneKind::Battlefield,
                None,
                ZonePosition::Unordered,
                VisibilityPartition::Public,
            ),
        )
        .unwrap();
    let mut result = candidate.finish().unwrap();

    let stale_lifecycle = PerspectiveLifecycleAuditV1 {
        perspective: P1,
        sequence: VisibleSequence(1),
        mutation: PerspectiveLifecycleMutationV1 {
            identity: IdentityMutationV1::Allocate {
                opaque: mtgml_model::OpaqueObjectId(2),
                object: old,
            },
            knowledge: None,
        },
    };
    result.events.push(AuthoritativeRuleEvent {
        event_id: RuleEventId(2),
        state_revision: StateRevision(1),
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
        "RED s2.mutant.stale_old_lifecycle_occurrence: expected semantic cursor rejection for OLD after ZoneTransition"
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
