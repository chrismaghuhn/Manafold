// Turn-structure event and delta vocabulary tests.

use mtgml_state::{BeginningStep, EndingStep, TurnPosition};
use mtgml_model::ZoneKind;

fn turn_position_beginning() -> TurnPosition {
    TurnPosition::Beginning {
        step: BeginningStep::Untap,
    }
}

fn event_turn_number_changed(from: u64, to: u64) -> AuthoritativeRuleEvent {
    AuthoritativeRuleEvent {
        event_id: RuleEventId(1),
        state_revision: StateRevision(1),
        event: AuthoritativeRuleEventKind::TurnNumberChanged { from, to },
    }
}

fn event_turn_position_changed(from: TurnPosition, to: TurnPosition) -> AuthoritativeRuleEvent {
    AuthoritativeRuleEvent {
        event_id: RuleEventId(1),
        state_revision: StateRevision(1),
        event: AuthoritativeRuleEventKind::TurnPositionChanged { from, to },
    }
}

fn event_active_player_changed(from: PlayerId, to: PlayerId) -> AuthoritativeRuleEvent {
    AuthoritativeRuleEvent {
        event_id: RuleEventId(1),
        state_revision: StateRevision(1),
        event: AuthoritativeRuleEventKind::ActivePlayerChanged { from, to },
    }
}

fn event_untap_completed(objects: Vec<GameObjectId>) -> AuthoritativeRuleEvent {
    AuthoritativeRuleEvent {
        event_id: RuleEventId(1),
        state_revision: StateRevision(1),
        event: AuthoritativeRuleEventKind::UntapCompleted { affected_objects: objects },
    }
}

// --- Event→delta mapping ---

#[test]
fn turn_number_changed_maps_to_turn_number_changed_delta() {
    let event = event_turn_number_changed(1, 2);
    assert!(matches!(
        event.event.semantic_delta(),
        SemanticDeltaOperation::TurnNumberChanged { from: 1, to: 2 }
    ));
}

#[test]
fn turn_position_changed_maps_to_turn_position_changed_delta() {
    let from = turn_position_beginning();
    let to = TurnPosition::PrecombatMain;
    let event = event_turn_position_changed(from, to);
    assert!(matches!(
        event.event.semantic_delta(),
        SemanticDeltaOperation::TurnPositionChanged { from: f, to: t }
        if f == from && t == to
    ));
}

#[test]
fn active_player_changed_maps_to_active_player_changed_delta() {
    let event = event_active_player_changed(PlayerId(1), PlayerId(2));
    assert!(matches!(
        event.event.semantic_delta(),
        SemanticDeltaOperation::ActivePlayerChanged { from: p1, to: p2 }
        if p1 == PlayerId(1) && p2 == PlayerId(2)
    ));
}

#[test]
fn untap_completed_maps_to_untap_completed_delta() {
    let objects = vec![GameObjectId(1), GameObjectId(2)];
    let event = event_untap_completed(objects.clone());
    assert!(matches!(
        event.event.semantic_delta(),
        SemanticDeltaOperation::UntapCompleted { affected_objects: a }
        if a == objects
    ));
}

// --- TurnNumberChanged rejection cases (cursor-level) ---

#[test]
fn turn_number_changed_rejects_wrong_from() {
    let before = state_without_pending_decision();
    let mut cursor = crate::semantic_cursor::SemanticValidationCursor::from_state(&before).unwrap();
    assert!(matches!(
        cursor.apply(&event_turn_number_changed(5, 6).event),
        Err(TransitionViolation::TurnStructure)
    ));
}

#[test]
fn turn_number_changed_rejects_decrement() {
    let before = state_without_pending_decision();
    let mut cursor = crate::semantic_cursor::SemanticValidationCursor::from_state(&before).unwrap();
    assert!(matches!(
        cursor.apply(&event_turn_number_changed(1, 0).event),
        Err(TransitionViolation::TurnStructure)
    ));
}

#[test]
fn turn_number_changed_rejects_skip() {
    let before = state_without_pending_decision();
    let mut cursor = crate::semantic_cursor::SemanticValidationCursor::from_state(&before).unwrap();
    assert!(matches!(
        cursor.apply(&event_turn_number_changed(1, 3).event),
        Err(TransitionViolation::TurnStructure)
    ));
}

#[test]
fn turn_number_changed_rejects_overflow() {
    let mut before = state_without_pending_decision();
    before.core.turn_number = u64::MAX;
    let mut cursor = crate::semantic_cursor::SemanticValidationCursor::from_state(&before).unwrap();
    assert!(matches!(
        cursor.apply(&event_turn_number_changed(u64::MAX, 0).event),
        Err(TransitionViolation::TurnStructure)
    ));
}

// --- TurnPositionChanged mismatched from ---

#[test]
fn turn_position_changed_rejects_mismatched_from() {
    let before = state_without_pending_decision();
    let mut cursor = crate::semantic_cursor::SemanticValidationCursor::from_state(&before).unwrap();
    assert!(matches!(
        cursor.apply(&event_turn_position_changed(
            TurnPosition::PrecombatMain,
            TurnPosition::Ending {
                step: EndingStep::EndStep,
            },
        ).event),
        Err(TransitionViolation::TurnStructure)
    ));
}

// --- ActivePlayerChanged rejection cases ---

#[test]
fn active_player_changed_rejects_mismatched_from() {
    let before = state_without_pending_decision();
    let mut cursor = crate::semantic_cursor::SemanticValidationCursor::from_state(&before).unwrap();
    assert!(matches!(
        cursor.apply(&event_active_player_changed(PlayerId(2), PlayerId(1)).event),
        Err(TransitionViolation::TurnStructure)
    ));
}

#[test]
fn active_player_changed_rejects_same_player() {
    let before = state_without_pending_decision();
    let mut cursor = crate::semantic_cursor::SemanticValidationCursor::from_state(&before).unwrap();
    assert!(matches!(
        cursor.apply(&event_active_player_changed(PlayerId(1), PlayerId(1)).event),
        Err(TransitionViolation::TurnStructure)
    ));
}

#[test]
fn active_player_changed_rejects_undeclared_target() {
    let before = state_without_pending_decision();
    let mut cursor = crate::semantic_cursor::SemanticValidationCursor::from_state(&before).unwrap();
    assert!(matches!(
        cursor.apply(&event_active_player_changed(PlayerId(1), PlayerId(3)).event),
        Err(TransitionViolation::TurnStructure)
    ));
}

// --- UntapCompleted rejection cases ---

#[test]
fn untap_completed_rejects_duplicate_objects() {
    let before = state_without_pending_decision();
    let mut cursor = crate::semantic_cursor::SemanticValidationCursor::from_state(&before).unwrap();
    assert!(matches!(
        cursor.apply(&event_untap_completed(vec![GameObjectId(1), GameObjectId(1)]).event),
        Err(TransitionViolation::TurnStructure)
    ));
}

#[test]
fn untap_completed_rejects_noncanonical_order() {
    let mut before = state_without_pending_decision();
    before.zones.objects.get_mut(&GameObjectId(1)).unwrap().tapped = true;
    before.zones.objects.get_mut(&GameObjectId(2)).unwrap().tapped = true;
    let mut cursor = crate::semantic_cursor::SemanticValidationCursor::from_state(&before).unwrap();
    assert!(matches!(
        cursor.apply(&event_untap_completed(vec![GameObjectId(2), GameObjectId(1)]).event),
        Err(TransitionViolation::TurnStructure)
    ));
}

#[test]
fn untap_completed_rejects_missing_object() {
    let before = state_without_pending_decision();
    let mut cursor = crate::semantic_cursor::SemanticValidationCursor::from_state(&before).unwrap();
    assert!(matches!(
        cursor.apply(&event_untap_completed(vec![GameObjectId(99)]).event),
        Err(TransitionViolation::TurnStructure)
    ));
}

#[test]
fn untap_completed_rejects_already_untapped() {
    let before = state_without_pending_decision();
    let mut cursor = crate::semantic_cursor::SemanticValidationCursor::from_state(&before).unwrap();
    assert!(matches!(
        cursor.apply(&event_untap_completed(vec![GameObjectId(1)]).event),
        Err(TransitionViolation::TurnStructure)
    ));
}

#[test]
fn untap_completed_rejects_outside_battlefield() {
    let mut before = state_without_pending_decision();
    before.zones.objects.get_mut(&GameObjectId(2)).unwrap().tapped = true;
    let mut cursor = crate::semantic_cursor::SemanticValidationCursor::from_state(&before).unwrap();
    assert!(matches!(
        cursor.apply(&event_untap_completed(vec![GameObjectId(2)]).event),
        Err(TransitionViolation::TurnStructure)
    ));
}

#[test]
fn untap_completed_rejects_non_active_player() {
    let mut before = state_without_pending_decision();
    before.zones.objects.get_mut(&GameObjectId(1)).unwrap().tapped = true;
    before.zones.objects.get_mut(&GameObjectId(1)).unwrap().controller = PlayerId(2);
    let mut cursor = crate::semantic_cursor::SemanticValidationCursor::from_state(&before).unwrap();
    assert!(matches!(
        cursor.apply(&event_untap_completed(vec![GameObjectId(1)]).event),
        Err(TransitionViolation::TurnStructure)
    ));
}

// --- Valid untap cursor mutation (through transition contract) ---

#[test]
fn valid_untap_completed_passes_transition_contract() {
    let mut before = state_without_pending_decision();
    before.zones.objects.get_mut(&GameObjectId(1)).unwrap().tapped = true;
    let mut after = before.clone();
    after.revision = StateRevision(1);
    after.zones.objects.get_mut(&GameObjectId(1)).unwrap().tapped = false;
    after.allocators.next_rule_event_id = mtgml_model::RuleEventId(2);
    let events = vec![event_untap_completed(vec![GameObjectId(1)])];
    let result = accepted_product_for_contract(&before, after, events);
    assert!(validate_transition_contract(&before, &result).is_ok());
}

// --- Final parity rejection cases (cursor-level) ---

#[test]
fn cursor_final_parity_rejects_turn_number_mismatch() {
    let before = state_without_pending_decision();
    let mut cursor = crate::semantic_cursor::SemanticValidationCursor::from_state(&before).unwrap();
    cursor.apply(&event_turn_number_changed(1, 2).event).unwrap();
    let mut after = before.clone();
    after.core.turn_number = 99;
    assert!(matches!(
        cursor.validate_final_state(&after),
        Err(TransitionViolation::UnexplainedMutation)
    ));
}

#[test]
fn cursor_final_parity_rejects_active_player_mismatch() {
    let before = state_without_pending_decision();
    let mut cursor = crate::semantic_cursor::SemanticValidationCursor::from_state(&before).unwrap();
    cursor.apply(&event_active_player_changed(PlayerId(1), PlayerId(2)).event).unwrap();
    let mut after = before.clone();
    after.core.active_player = PlayerId(1);
    assert!(matches!(
        cursor.validate_final_state(&after),
        Err(TransitionViolation::UnexplainedMutation)
    ));
}

#[test]
fn cursor_final_parity_rejects_position_mismatch() {
    let before = state_without_pending_decision();
    let mut cursor = crate::semantic_cursor::SemanticValidationCursor::from_state(&before).unwrap();
    cursor
        .apply(
            &event_turn_position_changed(
                turn_position_beginning(),
                TurnPosition::Beginning {
                    step: BeginningStep::Upkeep,
                },
            )
            .event,
        )
        .unwrap();
    let mut after = before.clone();
    after.core.position = TurnPosition::Ending {
        step: EndingStep::EndStep,
    };
    assert!(matches!(
        cursor.validate_final_state(&after),
        Err(TransitionViolation::UnexplainedMutation)
    ));
}

#[test]
fn cursor_final_parity_rejects_tapped_state_mismatch() {
    let mut before = state_without_pending_decision();
    before.zones.objects.get_mut(&GameObjectId(1)).unwrap().tapped = true;
    let mut cursor = crate::semantic_cursor::SemanticValidationCursor::from_state(&before).unwrap();
    cursor
        .apply(&event_untap_completed(vec![GameObjectId(1)]).event)
        .unwrap();
    let mut after = before.clone();
    after.zones.objects.get_mut(&GameObjectId(1)).unwrap().tapped = true;
    assert!(matches!(
        cursor.validate_final_state(&after),
        Err(TransitionViolation::ObjectTraceIncomplete)
    ));
}

// --- Task 6: ordinary untap contract enforcement negatives ---

#[test]
fn untap_completed_omits_eligible_object_is_rejected() {
    let mut before = state_without_pending_decision();
    before
        .zones
        .objects
        .get_mut(&GameObjectId(1))
        .unwrap()
        .tapped = true;
    let mut cursor = crate::semantic_cursor::SemanticValidationCursor::from_state(&before).unwrap();
    // GameObjectId(1) is eligible (Battlefield, active-player, tapped) but
    // the event omits it from the affected set.
    assert!(matches!(
        cursor.apply(&event_untap_completed(vec![]).event),
        Err(TransitionViolation::TurnStructure)
    ));
}

#[test]
fn turn_position_changed_rejects_illegal_temporal_successor() {
    let before = state_without_pending_decision();
    let mut cursor = crate::semantic_cursor::SemanticValidationCursor::from_state(&before).unwrap();
    // Beginning(Untap) -> PrecombatMain is not the temporal successor of Untap.
    assert!(matches!(
        cursor.apply(
            &event_turn_position_changed(
                TurnPosition::Beginning {
                    step: BeginningStep::Untap,
                },
                TurnPosition::PrecombatMain,
            )
            .event
        ),
        Err(TransitionViolation::TurnStructure)
    ));
}

#[test]
fn untap_contract_rejects_unrelated_field_mutation() {
    let mut before = state_without_pending_decision();
    before
        .zones
        .objects
        .get_mut(&GameObjectId(1))
        .unwrap()
        .tapped = true;
    let mut after = before.clone();
    after.revision = StateRevision(1);
    after
        .zones
        .objects
        .get_mut(&GameObjectId(1))
        .unwrap()
        .tapped = false;
    after
        .zones
        .objects
        .get_mut(&GameObjectId(1))
        .unwrap()
        .face_down = true;
    after.allocators.next_rule_event_id = RuleEventId(3);
    let events = vec![
        AuthoritativeRuleEvent {
            event_id: RuleEventId(1),
            state_revision: StateRevision(1),
            event: AuthoritativeRuleEventKind::UntapCompleted {
                affected_objects: vec![GameObjectId(1)],
            },
        },
        AuthoritativeRuleEvent {
            event_id: RuleEventId(2),
            state_revision: StateRevision(1),
            event: AuthoritativeRuleEventKind::TurnPositionChanged {
                from: TurnPosition::Beginning {
                    step: BeginningStep::Untap,
                },
                to: TurnPosition::Beginning {
                    step: BeginningStep::Upkeep,
                },
            },
        },
    ];
    let result = accepted_product_for_contract(&before, after, events);
    assert_contract_rejects_without_mutation(&before, &result);
}

#[test]
fn untap_contract_rejects_unrelated_zone_location_mutation() {
    use mtgml_state::{KnownLocationFactV2, KnowledgeAcquisitionReason, KnowledgeRecordV2};

    let mut before = state_without_pending_decision();
    before
        .zones
        .objects
        .get_mut(&GameObjectId(1))
        .unwrap()
        .tapped = true;

    let new_location = mtgml_state::ZoneLocation {
        zone: ZoneKind::Graveyard,
        player: Some(PlayerId(1)),
        position: mtgml_state::ZonePosition::Unordered,
        visibility: mtgml_state::VisibilityPartition::Public,
        partition: None,
    };

    let mut after = before.clone();
    after.revision = StateRevision(1);
    after
        .zones
        .objects
        .get_mut(&GameObjectId(1))
        .unwrap()
        .tapped = false;
    after
        .zones
        .locations
        .insert(GameObjectId(1), new_location.clone());

    // Align knowledge so the after-state passes structural validation; the
    // cursor will still catch the location divergence.
    for player_id in after.core.players.keys() {
        let identity = after
            .perspective_identities
            .players
            .get_mut(player_id)
            .unwrap();
        let opaque = identity.object_to_opaque.get(&GameObjectId(1)).copied();
        if let Some(opaque) = opaque {
            if let Some(record) =
                after.knowledge.players.get_mut(player_id).unwrap().active.get_mut(&opaque)
            {
                record.known_location = Some(KnownLocationFactV2 {
                    location: new_location.clone(),
                    provenance: KnowledgeAcquisitionReason::InitialConfiguration,
                });
            }
        }
    }

    after.allocators.next_rule_event_id = RuleEventId(3);
    let events = vec![
        AuthoritativeRuleEvent {
            event_id: RuleEventId(1),
            state_revision: StateRevision(1),
            event: AuthoritativeRuleEventKind::UntapCompleted {
                affected_objects: vec![GameObjectId(1)],
            },
        },
        AuthoritativeRuleEvent {
            event_id: RuleEventId(2),
            state_revision: StateRevision(1),
            event: AuthoritativeRuleEventKind::TurnPositionChanged {
                from: TurnPosition::Beginning {
                    step: BeginningStep::Untap,
                },
                to: TurnPosition::Beginning {
                    step: BeginningStep::Upkeep,
                },
            },
        },
    ];
    let result = accepted_product_for_contract(&before, after, events);
    assert_contract_rejects_without_mutation(&before, &result);
}
