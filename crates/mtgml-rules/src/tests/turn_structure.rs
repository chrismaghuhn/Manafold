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
    after.core.position = TurnPosition::Beginning {
        step: BeginningStep::Upkeep,
    };
    after.allocators.next_rule_event_id = RuleEventId(5);
    for knowledge in after.knowledge.players.values_mut() {
        knowledge.next_visible_sequence = mtgml_model::VisibleSequence(
            knowledge.next_visible_sequence.0 + 1,
        );
    }
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
            event: AuthoritativeRuleEventKind::PerspectiveOccurrence {
                lifecycle: PerspectiveLifecycleAuditV1 {
                    perspective: PlayerId(1),
                    sequence: mtgml_model::VisibleSequence(1),
                    mutation: PerspectiveLifecycleMutationV1::default(),
                },
                observation: PerspectiveObservationPolicyV1::ObjectTapped {
                    object: GameObjectId(1),
                    tapped: false,
                },
            },
        },
        AuthoritativeRuleEvent {
            event_id: RuleEventId(3),
            state_revision: StateRevision(1),
            event: AuthoritativeRuleEventKind::PerspectiveOccurrence {
                lifecycle: PerspectiveLifecycleAuditV1 {
                    perspective: PlayerId(2),
                    sequence: mtgml_model::VisibleSequence(1),
                    mutation: PerspectiveLifecycleMutationV1::default(),
                },
                observation: PerspectiveObservationPolicyV1::ObjectTapped {
                    object: GameObjectId(1),
                    tapped: false,
                },
            },
        },
        AuthoritativeRuleEvent {
            event_id: RuleEventId(4),
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
    assert!(validate_transition_contract(&before, &result).is_ok());
}

#[test]
fn untap_contract_rejects_missing_untap_completed_with_tap_substitute() {
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
    after.core.position = TurnPosition::Beginning {
        step: BeginningStep::Upkeep,
    };
    after.allocators.next_rule_event_id = RuleEventId(3);
    let events = vec![
        AuthoritativeRuleEvent {
            event_id: RuleEventId(1),
            state_revision: StateRevision(1),
            event: AuthoritativeRuleEventKind::ObjectTapped {
                object: GameObjectId(1),
                from: true,
                to: false,
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
    let before_snapshot = before.clone();
    let result_snapshot = result.clone();
    assert!(matches!(
        validate_transition_contract(&before, &result),
        Err(TransitionViolation::TurnStructure)
    ));
    assert_eq!(before, before_snapshot);
    assert_eq!(result, result_snapshot);
}

#[test]
fn untap_contract_rejects_missing_empty_untap_completed() {
    let before = state_without_pending_decision();
    let mut after = before.clone();
    after.revision = StateRevision(1);
    after.core.position = TurnPosition::Beginning {
        step: BeginningStep::Upkeep,
    };
    after.allocators.next_rule_event_id = RuleEventId(2);
    let events = vec![AuthoritativeRuleEvent {
        event_id: RuleEventId(1),
        state_revision: StateRevision(1),
        event: AuthoritativeRuleEventKind::TurnPositionChanged {
            from: TurnPosition::Beginning {
                step: BeginningStep::Untap,
            },
            to: TurnPosition::Beginning {
                step: BeginningStep::Upkeep,
            },
        },
    }];
    let result = accepted_product_for_contract(&before, after, events);
    let before_snapshot = before.clone();
    let result_snapshot = result.clone();
    assert!(matches!(
        validate_transition_contract(&before, &result),
        Err(TransitionViolation::TurnStructure)
    ));
    assert_eq!(before, before_snapshot);
    assert_eq!(result, result_snapshot);
}

// --- Task 6 event-shape enforcement negatives ---

#[test]
fn untap_contract_rejects_preceding_tap_event_completeness_bypass() {
    // Exploit regression: a preceding ObjectTapped event should not be
    // allowed to make an UntapCompleted appear complete by emptying the
    // cursor's eligibility set before UntapCompleted is validated. The
    // event-shape check must reject any product whose UntapCompleted is not
    // the first of exactly two events.
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
    after.core.position = TurnPosition::Beginning {
        step: BeginningStep::Upkeep,
    };
    after.allocators.next_rule_event_id = RuleEventId(4);
    let events = vec![
        AuthoritativeRuleEvent {
            event_id: RuleEventId(1),
            state_revision: StateRevision(1),
            event: AuthoritativeRuleEventKind::ObjectTapped {
                object: GameObjectId(1),
                from: true,
                to: false,
            },
        },
        AuthoritativeRuleEvent {
            event_id: RuleEventId(2),
            state_revision: StateRevision(1),
            event: AuthoritativeRuleEventKind::UntapCompleted {
                affected_objects: vec![],
            },
        },
        AuthoritativeRuleEvent {
            event_id: RuleEventId(3),
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
    let error = validate_transition_contract(&before, &result).unwrap_err();
    assert!(matches!(error, TransitionViolation::TurnStructure));
}

#[test]
fn untap_contract_rejects_untap_completed_not_first() {
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
    after.core.position = TurnPosition::Beginning {
        step: BeginningStep::Upkeep,
    };
    after.allocators.next_rule_event_id = RuleEventId(3);
    let events = vec![
        AuthoritativeRuleEvent {
            event_id: RuleEventId(1),
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
        AuthoritativeRuleEvent {
            event_id: RuleEventId(2),
            state_revision: StateRevision(1),
            event: AuthoritativeRuleEventKind::UntapCompleted {
                affected_objects: vec![GameObjectId(1)],
            },
        },
    ];
    let result = accepted_product_for_contract(&before, after, events);
    assert_contract_rejects_without_mutation(&before, &result);
}

#[test]
fn untap_contract_rejects_extra_event_after_position_change() {
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
    after.core.position = TurnPosition::Beginning {
        step: BeginningStep::Upkeep,
    };
    after.allocators.next_rule_event_id = RuleEventId(4);
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
        AuthoritativeRuleEvent {
            event_id: RuleEventId(3),
            state_revision: StateRevision(1),
            event: AuthoritativeRuleEventKind::UntapCompleted {
                affected_objects: vec![],
            },
        },
    ];
    let result = accepted_product_for_contract(&before, after, events);
    assert_contract_rejects_without_mutation(&before, &result);
}

#[test]
fn untap_completed_rejects_non_untap_cursor_position() {
    // Defense-in-depth: UntapCompleted is only valid when the cursor is at
    // Beginning(Untap). A cursor at any other position rejects it.
    let mut before = state_without_pending_decision();
    before.core.position = TurnPosition::Beginning {
        step: BeginningStep::Upkeep,
    };
    let mut cursor = crate::semantic_cursor::SemanticValidationCursor::from_state(&before).unwrap();
    assert!(matches!(
        cursor.apply(&event_untap_completed(vec![]).event),
        Err(TransitionViolation::TurnStructure)
    ));
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

// --- Helper: add a single card to a player's Hand zone with proper
// knowledge/identity bookkeeping (owner-only visibility). ---

fn add_hand_card(state: &mut EngineState, object_id: GameObjectId, owner: PlayerId) {
    use mtgml_model::{CardDefinitionId, OpaqueObjectId, PhysicalCardId};
    use mtgml_state::{
        GameObject, KnownLocationFactV2, KnowledgeAcquisitionReason, KnowledgeRecordV2,
        VisibilityPartition, ZoneLocation, ZonePosition,
    };

    let physical_card = PhysicalCardId(object_id.0);
    let card_definition = CardDefinitionId(object_id.0);
    let opaque = OpaqueObjectId(object_id.0);

    state.zones.objects.insert(
        object_id,
        GameObject {
            id: object_id,
            physical_card: Some(physical_card),
            card_definition,
            owner,
            controller: owner,
            tapped: false,
            face_down: false,
        },
    );

    let location = ZoneLocation {
        zone: ZoneKind::Hand,
        player: Some(owner),
        position: ZonePosition::Unordered,
        visibility: VisibilityPartition::OwnerOnly,
        partition: None,
    };
    state.zones.locations.insert(object_id, location.clone());

    let next_id = GameObjectId(object_id.0 + 1);
    if state.allocators.next_object_id.0 < next_id.0 {
        state.allocators.next_object_id = next_id;
    }

    let knowledge_record = KnowledgeRecordV2 {
        opaque_object: opaque,
        physical_card: Some(physical_card),
        card_definition: Some(card_definition),
        known_location: Some(KnownLocationFactV2 {
            location,
            provenance: KnowledgeAcquisitionReason::InitialConfiguration,
        }),
        acquisition: KnowledgeAcquisitionReason::InitialConfiguration,
        historical_locations: Vec::new(),
    };

    state
        .knowledge
        .players
        .get_mut(&owner)
        .unwrap()
        .active
        .insert(opaque, knowledge_record);

    let identity = state
        .perspective_identities
        .players
        .get_mut(&owner)
        .unwrap();
    identity.opaque_to_object.insert(opaque, object_id);
    identity.object_to_opaque.insert(object_id, opaque);
    if identity.next_opaque_object_id.0 <= opaque.0 {
        identity.next_opaque_object_id = OpaqueObjectId(opaque.0 + 1);
    }
}

/// Adds a Hand-zone card with `player: None` (ambiguous ownership) to `state`,
/// including the knowledge/perspective-identity bookkeeping required for
/// `validate_engine_state` to accept the fixture.
fn add_ambiguous_hand_card(state: &mut EngineState, object_id: GameObjectId, owner: PlayerId) {
    use mtgml_model::{CardDefinitionId, OpaqueObjectId, PhysicalCardId};
    use mtgml_state::{
        GameObject, KnownLocationFactV2, KnowledgeAcquisitionReason, KnowledgeRecordV2,
        VisibilityPartition, ZoneLocation, ZonePosition,
    };

    let physical_card = PhysicalCardId(object_id.0);
    let card_definition = CardDefinitionId(object_id.0);
    let opaque = OpaqueObjectId(object_id.0);

    state.zones.objects.insert(
        object_id,
        GameObject {
            id: object_id,
            physical_card: Some(physical_card),
            card_definition,
            owner,
            controller: owner,
            tapped: false,
            face_down: false,
        },
    );

    let location = ZoneLocation {
        zone: ZoneKind::Hand,
        player: None,
        position: ZonePosition::Unordered,
        visibility: VisibilityPartition::OwnerOnly,
        partition: None,
    };
    state.zones.locations.insert(object_id, location.clone());

    let next_id = GameObjectId(object_id.0 + 1);
    if state.allocators.next_object_id.0 < next_id.0 {
        state.allocators.next_object_id = next_id;
    }

    let knowledge_record = KnowledgeRecordV2 {
        opaque_object: opaque,
        physical_card: Some(physical_card),
        card_definition: Some(card_definition),
        known_location: Some(KnownLocationFactV2 {
            location,
            provenance: KnowledgeAcquisitionReason::InitialConfiguration,
        }),
        acquisition: KnowledgeAcquisitionReason::InitialConfiguration,
        historical_locations: Vec::new(),
    };

    state
        .knowledge
        .players
        .get_mut(&owner)
        .unwrap()
        .active
        .insert(opaque, knowledge_record);

    let identity = state
        .perspective_identities
        .players
        .get_mut(&owner)
        .unwrap();
    identity.opaque_to_object.insert(opaque, object_id);
    identity.object_to_opaque.insert(object_id, opaque);
    if identity.next_opaque_object_id.0 <= opaque.0 {
        identity.next_opaque_object_id = OpaqueObjectId(opaque.0 + 1);
    }
}

fn cleanup_state_at_turn_with_hand(turn: u64, hand_count: usize) -> EngineState {
    let mut state = state_without_pending_decision();
    state.core.turn_number = turn;
    state.core.position = TurnPosition::Ending { step: EndingStep::Cleanup };
    for i in 3..(3 + hand_count as u64) {
        add_hand_card(&mut state, GameObjectId(i), PlayerId(1));
    }
    state
}

fn cleanup_state_with_marked_damage() -> EngineState {
    let mut state = state_without_pending_decision();
    state.core.position = TurnPosition::Ending { step: EndingStep::Cleanup };
    state.foundation_sources.insert(
        GameObjectId(1),
        mtgml_state::FoundationCreatureSource {
            source_kind: mtgml_state::FoundationSourceKind::Creature,
            base_characteristics: mtgml_state::BaseCharacteristics::Simple {
                power: 3,
                toughness: 3,
            },
            marked_damage: 1,
            control_history: mtgml_state::ControlHistory::BeforeTurnStart { turn_number: 1 },
        },
    );
    state
}

fn cleanup_product_for_contract(before: &EngineState) -> TransitionResult {
    let mut after = before.clone();
    after.revision = StateRevision(before.revision.0 + 1);
    after.core.turn_number = before.core.turn_number + 1;
    after.core.active_player = PlayerId(2);
    after.core.position = TurnPosition::Beginning { step: BeginningStep::Untap };
    after.allocators.next_rule_event_id = RuleEventId(before.allocators.next_rule_event_id.0 + 3);
    let events = vec![
        AuthoritativeRuleEvent {
            event_id: RuleEventId(before.allocators.next_rule_event_id.0),
            state_revision: StateRevision(before.revision.0 + 1),
            event: AuthoritativeRuleEventKind::TurnNumberChanged {
                from: before.core.turn_number,
                to: before.core.turn_number + 1,
            },
        },
        AuthoritativeRuleEvent {
            event_id: RuleEventId(before.allocators.next_rule_event_id.0 + 1),
            state_revision: StateRevision(before.revision.0 + 1),
            event: AuthoritativeRuleEventKind::ActivePlayerChanged {
                from: before.core.active_player,
                to: PlayerId(2),
            },
        },
        AuthoritativeRuleEvent {
            event_id: RuleEventId(before.allocators.next_rule_event_id.0 + 2),
            state_revision: StateRevision(before.revision.0 + 1),
            event: AuthoritativeRuleEventKind::TurnPositionChanged {
                from: TurnPosition::Ending { step: EndingStep::Cleanup },
                to: TurnPosition::Beginning { step: BeginningStep::Untap },
            },
        },
    ];
    accepted_product_for_contract(before, after, events)
}

// --- Task 7 FIX_03 RED: contract-level quiescent Cleanup detection ---

#[test]
fn cleanup_contract_rejects_nonquiescent_hand_product() {
    let before = cleanup_state_at_turn_with_hand(1, 8);
    let result = cleanup_product_for_contract(&before);
    assert!(
        matches!(validate_transition_contract(&before, &result), Err(TransitionViolation::TurnStructure)),
        "non-quiescent Cleanup with active hand > 7 must reject via TurnStructure"
    );
    assert_contract_rejects_without_mutation(&before, &result);
}

#[test]
fn cleanup_contract_rejects_nonquiescent_damage_product() {
    let before = cleanup_state_with_marked_damage();
    let result = cleanup_product_for_contract(&before);
    assert!(
        matches!(validate_transition_contract(&before, &result), Err(TransitionViolation::TurnStructure)),
        "non-quiescent Cleanup with marked damage must reject via TurnStructure"
    );
    assert_contract_rejects_without_mutation(&before, &result);
}

#[test]
fn cleanup_contract_rejects_ambiguous_hand_product() {
    use mtgml_state::validate_engine_state;

    let mut before = cleanup_state_at_turn_with_hand(1, 0);
    add_ambiguous_hand_card(&mut before, GameObjectId(3), PlayerId(1));
    assert!(
        validate_engine_state(&before).is_ok(),
        "fixture with ambiguous Hand must still pass generic engine-state validation"
    );
    let result = cleanup_product_for_contract(&before);
    assert!(
        matches!(validate_transition_contract(&before, &result), Err(TransitionViolation::TurnStructure)),
        "ambiguous Hand ownership (player=None) must reject via TurnStructure"
    );
    assert_contract_rejects_without_mutation(&before, &result);
}

#[test]
fn untap_contract_rejects_unrelated_zone_location_mutation() {
    use mtgml_state::{KnownLocationFactV2, KnowledgeAcquisitionReason};

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


// --- RED: malformed event vectors must not panic ---

#[test]
fn untap_empty_event_product_rejects_no_panic() {
    let mut before = state_without_pending_decision();
    before
        .zones
        .objects
        .get_mut(&GameObjectId(1))
        .unwrap()
        .tapped = true;
    let after = before.clone();
    let events = vec![];
    let result = accepted_product_for_contract(&before, after, events);
    assert!(validate_transition_contract(&before, &result).is_err());
}

#[test]
fn untap_one_event_product_rejects_no_panic() {
    let mut before = state_without_pending_decision();
    before
        .zones
        .objects
        .get_mut(&GameObjectId(1))
        .unwrap()
        .tapped = true;
    let mut after = before.clone();
    after.revision = StateRevision(1);
    after.core.position = TurnPosition::Beginning {
        step: BeginningStep::Upkeep,
    };
    let events = vec![AuthoritativeRuleEvent {
        event_id: RuleEventId(1),
        state_revision: StateRevision(1),
        event: AuthoritativeRuleEventKind::UntapCompleted {
            affected_objects: vec![GameObjectId(1)],
        },
    }];
    let result = accepted_product_for_contract(&before, after, events);
    assert!(validate_transition_contract(&before, &result).is_err());
}


#[test]
fn untap_contract_rejects_missing_occurrence() {
    let mut before = state_without_pending_decision();
    before.zones.objects.get_mut(&GameObjectId(1)).unwrap().tapped = true;
    let mut after = before.clone();
    after.revision = StateRevision(1);
    after.zones.objects.get_mut(&GameObjectId(1)).unwrap().tapped = false;
    after.core.position = TurnPosition::Beginning { step: BeginningStep::Upkeep };
    after.allocators.next_rule_event_id = RuleEventId(3);
    for knowledge in after.knowledge.players.values_mut() {
        knowledge.next_visible_sequence = mtgml_model::VisibleSequence(knowledge.next_visible_sequence.0 + 1);
    }
    let events = vec![
        AuthoritativeRuleEvent { event_id: RuleEventId(1), state_revision: StateRevision(1), event: AuthoritativeRuleEventKind::UntapCompleted { affected_objects: vec![GameObjectId(1)] } },
        AuthoritativeRuleEvent { event_id: RuleEventId(3), state_revision: StateRevision(1), event: AuthoritativeRuleEventKind::TurnPositionChanged { from: TurnPosition::Beginning { step: BeginningStep::Untap }, to: TurnPosition::Beginning { step: BeginningStep::Upkeep } } },
    ];
    let result = accepted_product_for_contract(&before, after, events);
    assert_contract_rejects_without_mutation(&before, &result);
}

#[test]
fn untap_contract_rejects_duplicate_occurrence() {
    let mut before = state_without_pending_decision();
    before.zones.objects.get_mut(&GameObjectId(1)).unwrap().tapped = true;
    let mut after = before.clone();
    after.revision = StateRevision(1);
    after.zones.objects.get_mut(&GameObjectId(1)).unwrap().tapped = false;
    after.core.position = TurnPosition::Beginning { step: BeginningStep::Upkeep };
    after.allocators.next_rule_event_id = RuleEventId(4);
    for knowledge in after.knowledge.players.values_mut() {
        knowledge.next_visible_sequence = mtgml_model::VisibleSequence(knowledge.next_visible_sequence.0 + 1);
    }
    let events = vec![
        AuthoritativeRuleEvent { event_id: RuleEventId(1), state_revision: StateRevision(1), event: AuthoritativeRuleEventKind::UntapCompleted { affected_objects: vec![GameObjectId(1)] } },
        AuthoritativeRuleEvent { event_id: RuleEventId(2), state_revision: StateRevision(1), event: AuthoritativeRuleEventKind::PerspectiveOccurrence { lifecycle: PerspectiveLifecycleAuditV1 { perspective: PlayerId(1), sequence: mtgml_model::VisibleSequence(1), mutation: PerspectiveLifecycleMutationV1::default() }, observation: PerspectiveObservationPolicyV1::ObjectTapped { object: GameObjectId(1), tapped: false } } },
        AuthoritativeRuleEvent { event_id: RuleEventId(3), state_revision: StateRevision(1), event: AuthoritativeRuleEventKind::PerspectiveOccurrence { lifecycle: PerspectiveLifecycleAuditV1 { perspective: PlayerId(1), sequence: mtgml_model::VisibleSequence(2), mutation: PerspectiveLifecycleMutationV1::default() }, observation: PerspectiveObservationPolicyV1::ObjectTapped { object: GameObjectId(1), tapped: false } } },
        AuthoritativeRuleEvent { event_id: RuleEventId(4), state_revision: StateRevision(1), event: AuthoritativeRuleEventKind::TurnPositionChanged { from: TurnPosition::Beginning { step: BeginningStep::Untap }, to: TurnPosition::Beginning { step: BeginningStep::Upkeep } } },
    ];
    let result = accepted_product_for_contract(&before, after, events);
    assert_contract_rejects_without_mutation(&before, &result);
}

#[test]
fn untap_contract_rejects_wrong_object() {
    let mut before = state_without_pending_decision();
    before.zones.objects.get_mut(&GameObjectId(1)).unwrap().tapped = true;
    let mut after = before.clone();
    after.revision = StateRevision(1);
    after.zones.objects.get_mut(&GameObjectId(1)).unwrap().tapped = false;
    after.core.position = TurnPosition::Beginning { step: BeginningStep::Upkeep };
    after.allocators.next_rule_event_id = RuleEventId(3);
    for knowledge in after.knowledge.players.values_mut() {
        knowledge.next_visible_sequence = mtgml_model::VisibleSequence(knowledge.next_visible_sequence.0 + 1);
    }
    let events = vec![
        AuthoritativeRuleEvent { event_id: RuleEventId(1), state_revision: StateRevision(1), event: AuthoritativeRuleEventKind::UntapCompleted { affected_objects: vec![GameObjectId(1)] } },
        AuthoritativeRuleEvent { event_id: RuleEventId(2), state_revision: StateRevision(1), event: AuthoritativeRuleEventKind::PerspectiveOccurrence { lifecycle: PerspectiveLifecycleAuditV1 { perspective: PlayerId(1), sequence: mtgml_model::VisibleSequence(1), mutation: PerspectiveLifecycleMutationV1::default() }, observation: PerspectiveObservationPolicyV1::ObjectTapped { object: GameObjectId(2), tapped: false } } },
        AuthoritativeRuleEvent { event_id: RuleEventId(3), state_revision: StateRevision(1), event: AuthoritativeRuleEventKind::TurnPositionChanged { from: TurnPosition::Beginning { step: BeginningStep::Untap }, to: TurnPosition::Beginning { step: BeginningStep::Upkeep } } },
    ];
    let result = accepted_product_for_contract(&before, after, events);
    assert_contract_rejects_without_mutation(&before, &result);
}

#[test]
fn untap_contract_rejects_wrong_tapped() {
    let mut before = state_without_pending_decision();
    before.zones.objects.get_mut(&GameObjectId(1)).unwrap().tapped = true;
    let mut after = before.clone();
    after.revision = StateRevision(1);
    after.zones.objects.get_mut(&GameObjectId(1)).unwrap().tapped = false;
    after.core.position = TurnPosition::Beginning { step: BeginningStep::Upkeep };
    after.allocators.next_rule_event_id = RuleEventId(3);
    for knowledge in after.knowledge.players.values_mut() {
        knowledge.next_visible_sequence = mtgml_model::VisibleSequence(knowledge.next_visible_sequence.0 + 1);
    }
    let events = vec![
        AuthoritativeRuleEvent { event_id: RuleEventId(1), state_revision: StateRevision(1), event: AuthoritativeRuleEventKind::UntapCompleted { affected_objects: vec![GameObjectId(1)] } },
        AuthoritativeRuleEvent { event_id: RuleEventId(2), state_revision: StateRevision(1), event: AuthoritativeRuleEventKind::PerspectiveOccurrence { lifecycle: PerspectiveLifecycleAuditV1 { perspective: PlayerId(1), sequence: mtgml_model::VisibleSequence(1), mutation: PerspectiveLifecycleMutationV1::default() }, observation: PerspectiveObservationPolicyV1::ObjectTapped { object: GameObjectId(1), tapped: true } } },
        AuthoritativeRuleEvent { event_id: RuleEventId(3), state_revision: StateRevision(1), event: AuthoritativeRuleEventKind::TurnPositionChanged { from: TurnPosition::Beginning { step: BeginningStep::Untap }, to: TurnPosition::Beginning { step: BeginningStep::Upkeep } } },
    ];
    let result = accepted_product_for_contract(&before, after, events);
    assert_contract_rejects_without_mutation(&before, &result);
}

#[test]
fn untap_contract_rejects_wrong_order() {
    let mut before = state_without_pending_decision();
    before.zones.objects.get_mut(&GameObjectId(1)).unwrap().tapped = true;
    let mut after = before.clone();
    after.revision = StateRevision(1);
    after.zones.objects.get_mut(&GameObjectId(1)).unwrap().tapped = false;
    after.core.position = TurnPosition::Beginning { step: BeginningStep::Upkeep };
    after.allocators.next_rule_event_id = RuleEventId(3);
    for knowledge in after.knowledge.players.values_mut() {
        knowledge.next_visible_sequence = mtgml_model::VisibleSequence(knowledge.next_visible_sequence.0 + 1);
    }
    let events = vec![
        AuthoritativeRuleEvent { event_id: RuleEventId(2), state_revision: StateRevision(1), event: AuthoritativeRuleEventKind::PerspectiveOccurrence { lifecycle: PerspectiveLifecycleAuditV1 { perspective: PlayerId(1), sequence: mtgml_model::VisibleSequence(1), mutation: PerspectiveLifecycleMutationV1::default() }, observation: PerspectiveObservationPolicyV1::ObjectTapped { object: GameObjectId(1), tapped: false } } },
        AuthoritativeRuleEvent { event_id: RuleEventId(1), state_revision: StateRevision(1), event: AuthoritativeRuleEventKind::UntapCompleted { affected_objects: vec![GameObjectId(1)] } },
        AuthoritativeRuleEvent { event_id: RuleEventId(3), state_revision: StateRevision(1), event: AuthoritativeRuleEventKind::TurnPositionChanged { from: TurnPosition::Beginning { step: BeginningStep::Untap }, to: TurnPosition::Beginning { step: BeginningStep::Upkeep } } },
    ];
    let result = accepted_product_for_contract(&before, after, events);
    assert_contract_rejects_without_mutation(&before, &result);
}

#[test]
fn untap_contract_rejects_extra_middle_event() {
    let mut before = state_without_pending_decision();
    before.zones.objects.get_mut(&GameObjectId(1)).unwrap().tapped = true;
    let mut after = before.clone();
    after.revision = StateRevision(1);
    after.zones.objects.get_mut(&GameObjectId(1)).unwrap().tapped = false;
    after.core.position = TurnPosition::Beginning { step: BeginningStep::Upkeep };
    after.allocators.next_rule_event_id = RuleEventId(4);
    for knowledge in after.knowledge.players.values_mut() {
        knowledge.next_visible_sequence = mtgml_model::VisibleSequence(knowledge.next_visible_sequence.0 + 1);
    }
    let events = vec![
        AuthoritativeRuleEvent { event_id: RuleEventId(1), state_revision: StateRevision(1), event: AuthoritativeRuleEventKind::UntapCompleted { affected_objects: vec![GameObjectId(1)] } },
        AuthoritativeRuleEvent { event_id: RuleEventId(2), state_revision: StateRevision(1), event: AuthoritativeRuleEventKind::PerspectiveOccurrence { lifecycle: PerspectiveLifecycleAuditV1 { perspective: PlayerId(1), sequence: mtgml_model::VisibleSequence(1), mutation: PerspectiveLifecycleMutationV1::default() }, observation: PerspectiveObservationPolicyV1::ObjectTapped { object: GameObjectId(1), tapped: false } } },
        AuthoritativeRuleEvent { event_id: RuleEventId(3), state_revision: StateRevision(1), event: AuthoritativeRuleEventKind::TurnPositionChanged { from: TurnPosition::Beginning { step: BeginningStep::Untap }, to: TurnPosition::Beginning { step: BeginningStep::Upkeep } } },
        AuthoritativeRuleEvent { event_id: RuleEventId(4), state_revision: StateRevision(1), event: AuthoritativeRuleEventKind::TurnPositionChanged { from: TurnPosition::Beginning { step: BeginningStep::Untap }, to: TurnPosition::Beginning { step: BeginningStep::Upkeep } } },
    ];
    let result = accepted_product_for_contract(&before, after, events);
    assert_contract_rejects_without_mutation(&before, &result);
}
