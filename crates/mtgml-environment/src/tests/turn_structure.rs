// Task 9: observation, public untap consequences, and product closure.
//
// These tests verify that the S1 public untap consequence is projected
// through the existing observation infrastructure using
// ObservedEventKindV2::ObjectTapped and perspective-local opaque identity.

use base64::Engine as _;
use mtgml_observation::{
    ObservedEventKindV2,
    SyntheticM3Observation, SyntheticM3BeginningStep, SyntheticM3Priority, SyntheticM3TurnPosition,
};
use mtgml_rules::{
    AuthoritativeRuleEvent, AuthoritativeRuleEventKind,
    PerspectiveObservationPolicyV1,
};
use mtgml_state::{
    PerspectiveLifecycleAuditV1, PerspectiveLifecycleMutationV1,
    SyntheticResetInputs, SyntheticV4Setup,
};
use mtgml_wire::decode_canonical;

use crate::lifecycle_projection::project_occurrence_envelopes;
use crate::SyntheticM1EnvironmentBackend;

fn untap_completed_event(
    event_id: mtgml_model::RuleEventId,
    objects: Vec<mtgml_model::GameObjectId>,
) -> AuthoritativeRuleEvent {
    AuthoritativeRuleEvent {
        event_id,
        state_revision: StateRevision(1),
        event: AuthoritativeRuleEventKind::UntapCompleted { affected_objects: objects },
    }
}

fn turn_position_changed_event(
    event_id: mtgml_model::RuleEventId,
) -> AuthoritativeRuleEvent {
    AuthoritativeRuleEvent {
        event_id,
        state_revision: StateRevision(1),
        event: AuthoritativeRuleEventKind::TurnPositionChanged {
            from: mtgml_state::TurnPosition::Beginning {
                step: mtgml_state::BeginningStep::Untap,
            },
            to: mtgml_state::TurnPosition::Beginning {
                step: mtgml_state::BeginningStep::Upkeep,
            },
        },
    }
}

fn occurrence_event(
    event_id: mtgml_model::RuleEventId,
    perspective: PlayerId,
    sequence: u64,
    object: mtgml_model::GameObjectId,
    tapped: bool,
) -> AuthoritativeRuleEvent {
    AuthoritativeRuleEvent {
        event_id,
        state_revision: StateRevision(1),
        event: AuthoritativeRuleEventKind::PerspectiveOccurrence {
            lifecycle: PerspectiveLifecycleAuditV1 {
                perspective,
                sequence: VisibleSequence(sequence),
                mutation: PerspectiveLifecycleMutationV1::default(),
            },
            observation: PerspectiveObservationPolicyV1::ObjectTapped { object, tapped },
        },
    }
}

fn base_state_with_objects(
    active_player: PlayerId,
    objects: &[(mtgml_model::GameObjectId, PlayerId, bool)],
) -> mtgml_state::EngineState {
    let mut state = mtgml_state::construct_synthetic_engine_state(
        SyntheticResetInputs {
            players: [PlayerId(1), PlayerId(2)],
            root_seed: seed(),
            setup: SyntheticV4Setup {
                position: mtgml_state::TurnPosition::Beginning {
                    step: mtgml_state::BeginningStep::Untap,
                },
                priority: mtgml_state::PriorityState::None,
                combat: None,
                foundation_sources: std::collections::BTreeMap::new(),
            },
        },
    )
    .unwrap();
    state.core.active_player = active_player;
    for (object_id, controller, tapped) in objects {
        let location = mtgml_state::ZoneLocation {
            zone: mtgml_model::ZoneKind::Battlefield,
            player: None,
            position: mtgml_state::ZonePosition::Unordered,
            visibility: mtgml_state::VisibilityPartition::Public,
            partition: None,
        };
        state.zones.objects.insert(
            *object_id,
            mtgml_state::GameObject {
                id: *object_id,
                physical_card: Some(mtgml_model::PhysicalCardId(object_id.0)),
                card_definition: mtgml_model::CardDefinitionId(object_id.0),
                owner: *controller,
                controller: *controller,
                tapped: *tapped,
                face_down: false,
            },
        );
        state.zones.locations.insert(*object_id, location);
    }
    state
}

fn state_with_opaque_mapping(
    active_player: PlayerId,
    objects: &[(mtgml_model::GameObjectId, PlayerId, bool)],
    perspective: PlayerId,
) -> mtgml_state::EngineState {
    let mut state = base_state_with_objects(active_player, objects);
    let identity = state.perspective_identities.players.get_mut(&perspective).unwrap();
    for (object_id, _, _) in objects {
        let opaque = OpaqueObjectId(object_id.0 + 1000);
        identity.object_to_opaque.insert(*object_id, opaque);
        identity.opaque_to_object.insert(opaque, *object_id);
    }
    state
}

fn with_next_visible_sequence(
    mut state: mtgml_state::EngineState,
    player: PlayerId,
    seq: u64,
) -> mtgml_state::EngineState {
    if let Some(knowledge) = state.knowledge.players.get_mut(&player) {
        knowledge.next_visible_sequence = VisibleSequence(seq);
    }
    state
}

// --- A. One public untap ---

#[test]
fn observation_one_public_untap_produces_object_tapped() {
    let object = mtgml_model::GameObjectId(1);
    let active = PlayerId(1);
    let before = state_with_opaque_mapping(active, &[(object, active, true)], active);

    let events = vec![
        untap_completed_event(mtgml_model::RuleEventId(1), vec![object]),
        occurrence_event(mtgml_model::RuleEventId(2), active, 1, object, false),
        turn_position_changed_event(mtgml_model::RuleEventId(3)),
    ];

    let mut after = state_with_opaque_mapping(active, &[(object, active, false)], active);
    after = with_next_visible_sequence(after, active, 2);
    after.core.position = mtgml_state::TurnPosition::Beginning {
        step: mtgml_state::BeginningStep::Upkeep,
    };

    let envelopes = project_occurrence_envelopes(&before, &after, &events).unwrap();
    let p1 = &envelopes[&active];
    assert_eq!(p1.len(), 1, "one perspective should have one envelope");
    match &p1[0].event {
        ObservedEventKindV2::ObjectTapped { object: opaque, tapped } => {
            assert_eq!(*opaque, OpaqueObjectId(1001), "opaque ID from perspective mapping");
            assert!(!*tapped, "tapped is false after untap");
        }
        other => panic!("expected ObjectTapped, got {other:?}"),
    }
    assert_eq!(p1[0].sequence, VisibleSequence(1));
    assert_eq!(p1[0].state_revision, StateRevision(1));
    assert_eq!(p1[0].schema_version, OBSERVED_EVENT_SCHEMA_V2.to_string());
    p1[0].validate().unwrap();
}

// --- B. Multiple public untaps ---

#[test]
fn observation_multiple_public_untaps_canonical_order() {
    let obj1 = mtgml_model::GameObjectId(1);
    let obj2 = mtgml_model::GameObjectId(2);
    let active = PlayerId(1);
    let before = state_with_opaque_mapping(
        active,
        &[(obj1, active, true), (obj2, active, true)],
        active,
    );

    let events = vec![
        untap_completed_event(mtgml_model::RuleEventId(1), vec![obj1, obj2]),
        occurrence_event(mtgml_model::RuleEventId(2), active, 1, obj1, false),
        occurrence_event(mtgml_model::RuleEventId(3), active, 2, obj2, false),
        turn_position_changed_event(mtgml_model::RuleEventId(4)),
    ];

    let mut after = state_with_opaque_mapping(
        active,
        &[(obj1, active, false), (obj2, active, false)],
        active,
    );
    after = with_next_visible_sequence(after, active, 3);
    after.core.position = mtgml_state::TurnPosition::Beginning {
        step: mtgml_state::BeginningStep::Upkeep,
    };

    let envelopes = project_occurrence_envelopes(&before, &after, &events).unwrap();
    let p1 = &envelopes[&active];
    assert_eq!(p1.len(), 2);
    match &p1[0].event {
        ObservedEventKindV2::ObjectTapped { object: opaque, tapped } => {
            assert_eq!(*opaque, OpaqueObjectId(1001));
            assert!(!*tapped);
        }
        other => panic!("expected ObjectTapped at index 0, got {other:?}"),
    }
    match &p1[1].event {
        ObservedEventKindV2::ObjectTapped { object: opaque, tapped } => {
            assert_eq!(*opaque, OpaqueObjectId(1002));
            assert!(!*tapped);
        }
        other => panic!("expected ObjectTapped at index 1, got {other:?}"),
    }
    assert_eq!(p1[0].sequence, VisibleSequence(1));
    assert_eq!(p1[1].sequence, VisibleSequence(2));
}

// --- C. Empty affected set ---

#[test]
fn observation_empty_affected_set_no_object_tapped() {
    let active = PlayerId(1);
    let before = state_with_opaque_mapping(active, &[], active);

    let events = vec![
        untap_completed_event(mtgml_model::RuleEventId(1), vec![]),
        turn_position_changed_event(mtgml_model::RuleEventId(2)),
    ];

    let mut after = state_with_opaque_mapping(active, &[], active);
    after = with_next_visible_sequence(after, active, 1);
    after.core.position = mtgml_state::TurnPosition::Beginning {
        step: mtgml_state::BeginningStep::Upkeep,
    };

    let envelopes = project_occurrence_envelopes(&before, &after, &events).unwrap();
    let p1 = &envelopes[&active];
    assert!(
        p1.is_empty(),
        "empty affected set must produce no ObjectTapped envelopes"
    );
}

// --- D. Non-active player's tapped permanent ---

#[test]
fn observation_nonactive_tapped_object_not_projected() {
    let active = PlayerId(1);
    let nonactive = PlayerId(2);
    let obj = mtgml_model::GameObjectId(1);
    let before = state_with_opaque_mapping(active, &[(obj, nonactive, true)], active);
    let before = {
        let mut s = before;
        let identity = s.perspective_identities.players.get_mut(&nonactive).unwrap();
        let opaque = OpaqueObjectId(1);
        identity.object_to_opaque.insert(obj, opaque);
        identity.opaque_to_object.insert(opaque, obj);
        s
    };

    let events = vec![
        untap_completed_event(mtgml_model::RuleEventId(1), vec![]),
        turn_position_changed_event(mtgml_model::RuleEventId(2)),
    ];

    let mut after = state_with_opaque_mapping(active, &[(obj, nonactive, true)], active);
    after = with_next_visible_sequence(after, active, 1);
    after.core.position = mtgml_state::TurnPosition::Beginning {
        step: mtgml_state::BeginningStep::Upkeep,
    };

    let envelopes = project_occurrence_envelopes(&before, &after, &events).unwrap();
    let p1 = &envelopes[&active];
    assert!(
        p1.is_empty(),
        "non-active player's tapped object must not receive an untap observation"
    );
    assert!(
        after.zones.objects.get(&obj).unwrap().tapped,
        "non-active object must remain tapped"
    );
}

// --- E. Exact temporal observation ---

#[test]
fn observation_exact_temporal_fields_after_untap() {
    let object = mtgml_model::GameObjectId(1);
    let active = PlayerId(1);
    let before = state_with_opaque_mapping(active, &[(object, active, true)], active);

    let events = vec![
        untap_completed_event(mtgml_model::RuleEventId(1), vec![object]),
        occurrence_event(mtgml_model::RuleEventId(2), active, 1, object, false),
        turn_position_changed_event(mtgml_model::RuleEventId(3)),
    ];

    let mut after = state_with_opaque_mapping(active, &[(object, active, false)], active);
    after = with_next_visible_sequence(after, active, 2);
    after.core.position = mtgml_state::TurnPosition::Beginning {
        step: mtgml_state::BeginningStep::Upkeep,
    };

    let envelope = SyntheticM1EnvironmentBackend::synthetic_observation(&after, active)
        .unwrap();
    let payload_bytes = base64::engine::general_purpose::STANDARD
        .decode(&envelope.payload_base64)
        .unwrap();
    let payload: SyntheticM3Observation =
        decode_canonical(&payload_bytes).unwrap();
    assert_eq!(payload.active_player, active);
    assert_eq!(payload.turn_number, "1");
    assert!(matches!(
        payload.turn_position,
        SyntheticM3TurnPosition::Beginning {
            step: SyntheticM3BeginningStep::Upkeep,
        }
    ));
    assert!(matches!(payload.priority, SyntheticM3Priority::None));

    let envelopes = project_occurrence_envelopes(&before, &after, &events).unwrap();
    let p1 = &envelopes[&active];
    assert_eq!(p1.len(), 1);
    assert_eq!(p1[0].state_revision, StateRevision(1));
    p1[0].validate().unwrap();
}

// --- Determinism: rerun produces identical output ---

#[test]
fn observation_untap_deterministic_rerun() {
    let object = mtgml_model::GameObjectId(1);
    let active = PlayerId(1);
    let before = state_with_opaque_mapping(active, &[(object, active, true)], active);

    let events = vec![
        untap_completed_event(mtgml_model::RuleEventId(1), vec![object]),
        occurrence_event(mtgml_model::RuleEventId(2), active, 1, object, false),
        turn_position_changed_event(mtgml_model::RuleEventId(3)),
    ];

    let mut after = state_with_opaque_mapping(active, &[(object, active, false)], active);
    after = with_next_visible_sequence(after, active, 2);
    after.core.position = mtgml_state::TurnPosition::Beginning {
        step: mtgml_state::BeginningStep::Upkeep,
    };

    let first = project_occurrence_envelopes(&before, &after, &events).unwrap();
    let second = project_occurrence_envelopes(&before, &after, &events).unwrap();
    assert_eq!(
        first, second,
        "identical inputs must produce identical projection"
    );
}

// --- Negative: authorized but unresolvable opaque object ---

#[test]
fn observation_authorized_unresolvable_fails_closed() {
    let object = mtgml_model::GameObjectId(1);
    let active = PlayerId(1);
    let before = base_state_with_objects(active, &[(object, active, true)]);
    let before = {
        let mut s = before;
        let identity = s.perspective_identities.players.get_mut(&active).unwrap();
        identity.object_to_opaque.remove(&object);
        s
    };

    let events = vec![occurrence_event(
        mtgml_model::RuleEventId(1),
        active,
        1,
        object,
        false,
    )];

    let mut after = base_state_with_objects(active, &[(object, active, false)]);
    after.core.position = mtgml_state::TurnPosition::Beginning {
        step: mtgml_state::BeginningStep::Upkeep,
    };

    let result = project_occurrence_envelopes(&before, &after, &events);
    assert!(
        matches!(result, Err(crate::lifecycle_projection::LifecycleProjectionError::AuthorizedObjectUnresolvable)),
        "authorized but unresolvable object must fail with AuthorizedObjectUnresolvable"
    );
}

// --- Negative: privileged identity leak check ---

#[test]
fn observation_object_tapped_no_trusted_id_in_envelope() {
    let object = mtgml_model::GameObjectId(1);
    let active = PlayerId(1);
    let before = state_with_opaque_mapping(active, &[(object, active, true)], active);

    let events = vec![occurrence_event(
        mtgml_model::RuleEventId(1),
        active,
        1,
        object,
        false,
    )];

    let mut after = state_with_opaque_mapping(active, &[(object, active, false)], active);
    after = with_next_visible_sequence(after, active, 2);
    after.core.position = mtgml_state::TurnPosition::Beginning {
        step: mtgml_state::BeginningStep::Upkeep,
    };

    let envelopes = project_occurrence_envelopes(&before, &after, &events).unwrap();
    let bytes = serde_json::to_vec(&envelopes).unwrap();
    let rendered = String::from_utf8(bytes).unwrap();

    for forbidden in [
        "\"game_object_id\"",
        "\"physical_card_id\"",
        "\"root_seed\"",
        "\"checkpoint_digest\"",
        "\"full_state_digest\"",
        "\"stream_key\"",
        "\"next_raw_u64\"",
        "\"decision_id\"",
        "\"continuation_id\"",
        "\"rule_event_id\"",
    ] {
        assert!(
            !rendered.contains(forbidden),
            "projection leaked forbidden key {forbidden}"
        );
    }
}

// --- F. Paired-state noninterference ---

/// Build a state with a single Battlefield object where the card
/// definition, controller, zone, and tapped state are explicitly
/// specified. This allows constructing two states that share the
/// same authorized semantic facts while differing only in the
/// trusted GameObjectId and its perspective identity mapping.
fn state_with_explicit_object(
    active_player: PlayerId,
    object_id: mtgml_model::GameObjectId,
    card_definition: mtgml_model::CardDefinitionId,
    controller: PlayerId,
    tapped: bool,
) -> mtgml_state::EngineState {
    let mut state = mtgml_state::construct_synthetic_engine_state(
        SyntheticResetInputs {
            players: [PlayerId(1), PlayerId(2)],
            root_seed: seed(),
            setup: SyntheticV4Setup {
                position: mtgml_state::TurnPosition::Beginning {
                    step: mtgml_state::BeginningStep::Untap,
                },
                priority: mtgml_state::PriorityState::None,
                combat: None,
                foundation_sources: std::collections::BTreeMap::new(),
            },
        },
    )
    .unwrap();
    state.core.active_player = active_player;
    let location = mtgml_state::ZoneLocation {
        zone: mtgml_model::ZoneKind::Battlefield,
        player: None,
        position: mtgml_state::ZonePosition::Unordered,
        visibility: mtgml_state::VisibilityPartition::Public,
        partition: None,
    };
    state.zones.objects.insert(
        object_id,
        mtgml_state::GameObject {
            id: object_id,
            physical_card: Some(mtgml_model::PhysicalCardId(card_definition.0)),
            card_definition,
            owner: controller,
            controller,
            tapped,
            face_down: false,
        },
    );
    state.zones.locations.insert(object_id, location);
    state
}

#[test]
fn observation_paired_state_noninterference() {
    let active = PlayerId(1);
    let card_def = mtgml_model::CardDefinitionId(42);

    // State A: trusted GameObjectId(1) -> opaque OpaqueObjectId(9001)
    // CardDefinitionId = 42, controller = PlayerId(1), tapped = true
    let before_a = {
        let mut state = state_with_explicit_object(
            active,
            mtgml_model::GameObjectId(1),
            card_def,
            active,
            true,
        );
        let identity = state.perspective_identities.players.get_mut(&active).unwrap();
        identity.object_to_opaque.clear();
        identity.opaque_to_object.clear();
        identity.object_to_opaque.insert(mtgml_model::GameObjectId(1), mtgml_model::OpaqueObjectId(9001));
        identity.opaque_to_object.insert(mtgml_model::OpaqueObjectId(9001), mtgml_model::GameObjectId(1));
        state = with_next_visible_sequence(state, active, 1);
        state
    };

    // State B: trusted GameObjectId(7) -> opaque OpaqueObjectId(9001)
    // CardDefinitionId = 42 (SAME), controller = PlayerId(1) (SAME), tapped = true (SAME)
    let before_b = {
        let mut state = state_with_explicit_object(
            active,
            mtgml_model::GameObjectId(7),
            card_def,
            active,
            true,
        );
        let identity = state.perspective_identities.players.get_mut(&active).unwrap();
        identity.object_to_opaque.clear();
        identity.opaque_to_object.clear();
        identity.object_to_opaque.insert(mtgml_model::GameObjectId(7), mtgml_model::OpaqueObjectId(9001));
        identity.opaque_to_object.insert(mtgml_model::OpaqueObjectId(9001), mtgml_model::GameObjectId(7));
        state = with_next_visible_sequence(state, active, 1);
        state
    };

    let mut after_a = state_with_explicit_object(active, mtgml_model::GameObjectId(1), card_def, active, false);
    after_a = with_next_visible_sequence(after_a, active, 2);
    let mut after_b = state_with_explicit_object(active, mtgml_model::GameObjectId(7), card_def, active, false);
    after_b = with_next_visible_sequence(after_b, active, 2);

    // Restore opaque mappings in after states.
    for (after, object_id) in [(&mut after_a, mtgml_model::GameObjectId(1)), (&mut after_b, mtgml_model::GameObjectId(7))] {
        let identity = after.perspective_identities.players.get_mut(&active).unwrap();
        identity.object_to_opaque.clear();
        identity.opaque_to_object.clear();
        identity.object_to_opaque.insert(object_id, mtgml_model::OpaqueObjectId(9001));
        identity.opaque_to_object.insert(mtgml_model::OpaqueObjectId(9001), object_id);
    }

    let events_a = vec![occurrence_event(mtgml_model::RuleEventId(1), active, 1, mtgml_model::GameObjectId(1), false)];
    let events_b = vec![occurrence_event(mtgml_model::RuleEventId(1), active, 1, mtgml_model::GameObjectId(7), false)];

    let envelopes_a = project_occurrence_envelopes(&before_a, &after_a, &events_a).unwrap();
    let envelopes_b = project_occurrence_envelopes(&before_b, &after_b, &events_b).unwrap();

    let bytes_a = serde_json::to_vec(&envelopes_a).unwrap();
    let bytes_b = serde_json::to_vec(&envelopes_b).unwrap();

    assert_eq!(bytes_a, bytes_b, "player-safe observation bytes must be identical for equivalent public states");

    // Also compare the player-level observation (information state)
    // for the same perspective. Both states must produce identical
    // player observation bytes because the authorized semantic
    // facts are equal and only the trusted identity mapping differs.
    let obs_a = SyntheticM1EnvironmentBackend::synthetic_observation(&before_a, active).unwrap();
    let obs_b = SyntheticM1EnvironmentBackend::synthetic_observation(&before_b, active).unwrap();
    let obs_bytes_a = serde_json::to_vec(&obs_a).unwrap();
    let obs_bytes_b = serde_json::to_vec(&obs_b).unwrap();
    assert_eq!(obs_bytes_a, obs_bytes_b, "player observation bytes must be identical for equivalent public states");
}
