// Private rules tests for the MagicRulesKernel shell.
//
// These tests verify:
// - The kernel constructs from an admitted profile
// - No player response is accepted (S1 has no decision surface)
// - All temporal positions classify at their downstream boundary
// - No state mutation occurs on any rejection path
// - ProgramKernelV1 remains opaque: for_program(MagicRules) fails,
//   for_admitted_execution requires post-V5-admission identity

use crate::magic::MagicRulesKernel;

fn s1_state_at(position: mtgml_state::TurnPosition) -> EngineState {
    let mut state = construct_synthetic_engine_state(SyntheticResetInputs {
        players: [PlayerId(7), PlayerId(42)],
        root_seed: RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap(),
        setup: SyntheticV4Setup {
            position,
            priority: mtgml_state::PriorityState::None,
            combat: None,
            foundation_sources: std::collections::BTreeMap::new(),
        },
    })
    .unwrap();
    state.execution.pending_decision = None;
    state
}

fn three_player_s1_state() -> EngineState {
    three_player_s1_state_at(mtgml_state::TurnPosition::Beginning {
        step: BeginningStep::Untap,
    })
}

fn three_player_s1_state_at(position: TurnPosition) -> EngineState {
    let mut state = s1_state_at(position);
    state.core.players.insert(
        PlayerId(99),
        mtgml_state::PlayerState {
            life: 40,
            has_lost: false,
        },
    );
    state.knowledge.players.insert(
        PlayerId(99),
        mtgml_state::PlayerKnowledgeStateV2 {
            next_visible_sequence: mtgml_model::VisibleSequence(1),
            ..Default::default()
        },
    );
    state.perspective_identities.players.insert(
        PlayerId(99),
        mtgml_state::PerspectiveIdentityRecordV2 {
            next_opaque_object_id: mtgml_model::OpaqueObjectId(1),
            next_opaque_ability_id: mtgml_model::OpaqueAbilityId(1),
            next_player_decision_id: PlayerDecisionIdV1(1),
            ..Default::default()
        },
    );
    state
}

// --- Response rejection ---

#[test]
fn magic_turn_structure_kernel_shell_rejects_player_response() {
    let state = s1_state_at(mtgml_state::TurnPosition::Beginning {
        step: mtgml_state::BeginningStep::Untap,
    });
    let before = state.clone();
    let mut kernel = MagicRulesKernel::new();
    let result = kernel.apply(&state, PlayerId(7), &response(0, 0));
    assert!(
        matches!(
            result,
            Err(crate::KernelExecutionError::UnsupportedPlayerResponse)
        ),
        "Magic apply() must never accept a player response"
    );
    assert_eq!(state, before, "apply() must not mutate input state");
}

#[test]
fn magic_turn_structure_kernel_shell_preserves_state_on_response_rejection() {
    let state = s1_state_at(mtgml_state::TurnPosition::Beginning {
        step: mtgml_state::BeginningStep::Upkeep,
    });
    let before = state.clone();
    let mut kernel = MagicRulesKernel::new();
    let _ = kernel.apply(&state, PlayerId(7), &response(0, 0));
    assert_eq!(
        state, before,
        "input state must remain unchanged after response rejection"
    );
}

// --- Boundary classification tests (section 15) ---

#[test]
fn magic_turn_structure_kernel_shell_reports_upkeep_priority_boundary() {
    let state = s1_state_at(mtgml_state::TurnPosition::Beginning {
        step: mtgml_state::BeginningStep::Upkeep,
    });
    let before = state.clone();
    let mut kernel = MagicRulesKernel::new();
    let result = kernel.advance_forced_progress(&state);
    assert!(matches!(
        result,
        Err(crate::KernelExecutionError::UnsupportedRulesBoundary(
            crate::UnsupportedRulesBoundary::BasicPriority
        ))
    ));
    assert_eq!(state, before, "failed progress must not mutate input");
}

#[test]
fn magic_turn_structure_kernel_shell_reports_draw_boundary() {
    let state = s1_state_at(mtgml_state::TurnPosition::Beginning {
        step: mtgml_state::BeginningStep::Draw,
    });
    let before = state.clone();
    let mut kernel = MagicRulesKernel::new();
    let result = kernel.advance_forced_progress(&state);
    assert!(matches!(
        result,
        Err(crate::KernelExecutionError::UnsupportedRulesBoundary(
            crate::UnsupportedRulesBoundary::DrawCard
        ))
    ));
    assert_eq!(state, before, "failed progress must not mutate input");
}

#[test]
fn magic_turn_structure_kernel_shell_reports_precombat_priority_boundary() {
    let state = s1_state_at(mtgml_state::TurnPosition::PrecombatMain);
    let before = state.clone();
    let mut kernel = MagicRulesKernel::new();
    let result = kernel.advance_forced_progress(&state);
    assert!(matches!(
        result,
        Err(crate::KernelExecutionError::UnsupportedRulesBoundary(
            crate::UnsupportedRulesBoundary::BasicPriority
        ))
    ));
    assert_eq!(state, before, "failed progress must not mutate input");
}

#[test]
fn magic_turn_structure_kernel_shell_reports_combat_boundary() {
    let state = s1_state_at(mtgml_state::TurnPosition::Combat {
        step: mtgml_state::CombatStep::BeginningOfCombat,
    });
    let before = state.clone();
    let mut kernel = MagicRulesKernel::new();
    let result = kernel.advance_forced_progress(&state);
    assert!(matches!(
        result,
        Err(crate::KernelExecutionError::UnsupportedRulesBoundary(
            crate::UnsupportedRulesBoundary::Combat
        ))
    ));
    assert_eq!(state, before, "failed progress must not mutate input");
}

#[test]
fn magic_turn_structure_kernel_shell_reports_postcombat_priority_boundary() {
    let state = s1_state_at(mtgml_state::TurnPosition::PostcombatMain);
    let before = state.clone();
    let mut kernel = MagicRulesKernel::new();
    let result = kernel.advance_forced_progress(&state);
    assert!(matches!(
        result,
        Err(crate::KernelExecutionError::UnsupportedRulesBoundary(
            crate::UnsupportedRulesBoundary::BasicPriority
        ))
    ));
    assert_eq!(state, before, "failed progress must not mutate input");
}

#[test]
fn magic_turn_structure_kernel_shell_reports_end_step_priority_boundary() {
    let state = s1_state_at(mtgml_state::TurnPosition::Ending {
        step: mtgml_state::EndingStep::EndStep,
    });
    let before = state.clone();
    let mut kernel = MagicRulesKernel::new();
    let result = kernel.advance_forced_progress(&state);
    assert!(matches!(
        result,
        Err(crate::KernelExecutionError::UnsupportedRulesBoundary(
            crate::UnsupportedRulesBoundary::BasicPriority
        ))
    ));
    assert_eq!(state, before, "failed progress must not mutate input");
}

// --- Positive Cleanup transition tests ---

#[test]
fn magic_turn_structure_cleanup_quiescent_transition_succeeds() {
    let state = s1_state_at(mtgml_state::TurnPosition::Ending {
        step: mtgml_state::EndingStep::Cleanup,
    });
    let before = state.clone();
    let mut kernel = MagicRulesKernel::new();
    let result = kernel
        .advance_forced_progress(&state)
        .expect("quiescent cleanup must be accepted at Ending(Cleanup)");
    assert_eq!(state, before, "input state must not be mutated");

    assert!(result.accepted);

    // turn_number increments exactly once
    assert_eq!(
        result.next_state.core.turn_number,
        before.core.turn_number + 1
    );

    // active player switches to unique other declared player
    assert_eq!(result.next_state.core.active_player, PlayerId(42));

    // position becomes Beginning(Untap)
    assert_eq!(
        result.next_state.core.position,
        TurnPosition::Beginning {
            step: BeginningStep::Untap,
        }
    );

    // priority remains None
    assert_eq!(
        result.next_state.core.priority,
        mtgml_state::PriorityState::None
    );

    // event order is exactly: TurnNumberChanged, ActivePlayerChanged, TurnPositionChanged
    assert_eq!(result.events.len(), 3);
    assert!(matches!(
        &result.events[0].event,
        AuthoritativeRuleEventKind::TurnNumberChanged { from, to }
            if *from == before.core.turn_number
                && *to == before.core.turn_number + 1
    ));
    assert!(matches!(
        &result.events[1].event,
        AuthoritativeRuleEventKind::ActivePlayerChanged { from, to }
            if *from == before.core.active_player
                && *to == result.next_state.core.active_player
    ));
    assert!(matches!(
        &result.events[2].event,
        AuthoritativeRuleEventKind::TurnPositionChanged { from, to }
            if *from == TurnPosition::Ending {
                step: EndingStep::Cleanup,
            }
            && *to == TurnPosition::Beginning {
                step: BeginningStep::Untap,
            }
    ));

    // revision increments exactly once
    assert_eq!(
        result.next_state.revision,
        StateRevision(before.revision.0 + 1)
    );

    // rule-event allocator advances exactly three
    assert_eq!(
        result.next_state.allocators.next_rule_event_id,
        RuleEventId(before.allocators.next_rule_event_id.0 + 3)
    );

    // no Decision is created
    assert!(result.next_decision.is_none());

    // no RNG changes
    assert_eq!(
        result.next_state.random.root_seed,
        before.random.root_seed
    );
    assert_eq!(
        result.next_state.random.streams,
        before.random.streams
    );

    // no object tapped state changes
    assert_eq!(
        result.next_state.zones.objects,
        before.zones.objects
    );

    // delta audit matches event order
    assert_eq!(
        result.delta.audit,
        vec![
            SemanticDeltaOperation::TurnNumberChanged {
                from: before.core.turn_number,
                to: before.core.turn_number + 1,
            },
            SemanticDeltaOperation::ActivePlayerChanged {
                from: before.core.active_player,
                to: result.next_state.core.active_player,
            },
            SemanticDeltaOperation::TurnPositionChanged {
                from: TurnPosition::Ending {
                    step: EndingStep::Cleanup,
                },
                to: TurnPosition::Beginning {
                    step: BeginningStep::Untap,
                },
            },
        ]
    );

    // delta reapplies exactly to next_state
    assert_eq!(
        result.delta.apply(&before).unwrap(),
        result.next_state,
    );
}

#[test]
fn magic_turn_structure_cleanup_no_untap_performed_in_product() {
    let state = s1_state_at(mtgml_state::TurnPosition::Ending {
        step: mtgml_state::EndingStep::Cleanup,
    });
    let before = state.clone();
    let mut kernel = MagicRulesKernel::new();
    let result = kernel
        .advance_forced_progress(&state)
        .expect("quiescent cleanup must be accepted");

    // Position is Beginning(Untap) but the next player's ordinary untap
    // is NOT performed: no tapped objects changed and no UntapCompleted event.
    assert!(result.events.iter().all(|e| {
        !matches!(e.event, AuthoritativeRuleEventKind::UntapCompleted { .. })
    }));
    assert_eq!(result.next_state.zones.objects, before.zones.objects);
}

// --- Task 7: quiescent Cleanup contract-level negative tests ---

fn cleanup_state_at_turn(turn: u64) -> EngineState {
    let mut state = s1_state_at(mtgml_state::TurnPosition::Ending {
        step: mtgml_state::EndingStep::Cleanup,
    });
    state.core.turn_number = turn;
    state
}

fn cleanup_before_after() -> (EngineState, EngineState) {
    let before = cleanup_state_at_turn(1);
    let mut after = before.clone();
    after.revision = StateRevision(before.revision.0 + 1);
    after.core.turn_number = 2;
    after.core.active_player = PlayerId(42);
    after.core.position = TurnPosition::Beginning {
        step: BeginningStep::Untap,
    };
    after.allocators.next_rule_event_id = RuleEventId(before.allocators.next_rule_event_id.0 + 3);
    (before, after)
}

#[test]
fn cleanup_turn_number_overflow_rejects_without_mutation() {
    let state = cleanup_state_at_turn(u64::MAX);
    let before = state.clone();
    let mut kernel = MagicRulesKernel::new();
    let result = kernel.advance_forced_progress(&state);
    assert!(
        matches!(
            result,
            Err(crate::KernelExecutionError::TurnStructure(
                crate::TurnStructureError::TurnNumberOverflow
            ))
        ),
        "u64::MAX cleanup must reject with typed overflow error"
    );
    assert_eq!(state, before, "overflow must not mutate input");
}

#[test]
fn cleanup_contract_rejects_missing_turn_number_changed() {
    let (before, mut after) = cleanup_before_after();
    after.allocators.next_rule_event_id = RuleEventId(before.allocators.next_rule_event_id.0 + 2);
    let events = vec![
        AuthoritativeRuleEvent {
            event_id: RuleEventId(before.allocators.next_rule_event_id.0 + 1),
            state_revision: StateRevision(before.revision.0 + 1),
            event: AuthoritativeRuleEventKind::ActivePlayerChanged {
                from: PlayerId(7),
                to: PlayerId(42),
            },
        },
        AuthoritativeRuleEvent {
            event_id: RuleEventId(before.allocators.next_rule_event_id.0 + 2),
            state_revision: StateRevision(before.revision.0 + 1),
            event: AuthoritativeRuleEventKind::TurnPositionChanged {
                from: TurnPosition::Ending {
                    step: EndingStep::Cleanup,
                },
                to: TurnPosition::Beginning {
                    step: BeginningStep::Untap,
                },
            },
        },
    ];
    let result = accepted_product_for_contract(&before, after, events);
    assert!(
        matches!(validate_transition_contract(&before, &result), Err(TransitionViolation::TurnStructure)),
        "missing TurnNumberChanged must reject via TurnStructure"
    );
    assert_contract_rejects_without_mutation(&before, &result);
}

#[test]
fn cleanup_contract_rejects_missing_active_player_changed() {
    let (before, mut after) = cleanup_before_after();
    after.allocators.next_rule_event_id = RuleEventId(before.allocators.next_rule_event_id.0 + 2);
    let events = vec![
        AuthoritativeRuleEvent {
            event_id: RuleEventId(before.allocators.next_rule_event_id.0),
            state_revision: StateRevision(before.revision.0 + 1),
            event: AuthoritativeRuleEventKind::TurnNumberChanged {
                from: 1,
                to: 2,
            },
        },
        AuthoritativeRuleEvent {
            event_id: RuleEventId(before.allocators.next_rule_event_id.0 + 1),
            state_revision: StateRevision(before.revision.0 + 1),
            event: AuthoritativeRuleEventKind::TurnPositionChanged {
                from: TurnPosition::Ending {
                    step: EndingStep::Cleanup,
                },
                to: TurnPosition::Beginning {
                    step: BeginningStep::Untap,
                },
            },
        },
    ];
    let result = accepted_product_for_contract(&before, after, events);
    assert!(
        matches!(validate_transition_contract(&before, &result), Err(TransitionViolation::TurnStructure)),
        "missing ActivePlayerChanged must reject via TurnStructure"
    );
    assert_contract_rejects_without_mutation(&before, &result);
}

#[test]
fn cleanup_contract_rejects_missing_position_changed() {
    let (before, mut after) = cleanup_before_after();
    // Keep after at Cleanup boundary target Beginning(Untap)
    // but omit TurnPositionChanged from events.
    after.allocators.next_rule_event_id = RuleEventId(before.allocators.next_rule_event_id.0 + 2);
    let events = vec![
        AuthoritativeRuleEvent {
            event_id: RuleEventId(before.allocators.next_rule_event_id.0),
            state_revision: StateRevision(before.revision.0 + 1),
            event: AuthoritativeRuleEventKind::TurnNumberChanged {
                from: 1,
                to: 2,
            },
        },
        AuthoritativeRuleEvent {
            event_id: RuleEventId(before.allocators.next_rule_event_id.0 + 1),
            state_revision: StateRevision(before.revision.0 + 1),
            event: AuthoritativeRuleEventKind::ActivePlayerChanged {
                from: PlayerId(7),
                to: PlayerId(42),
            },
        },
    ];
    let result = accepted_product_for_contract(&before, after, events);
    assert!(
        matches!(validate_transition_contract(&before, &result), Err(TransitionViolation::TurnStructure)),
        "missing TurnPositionChanged must reject via TurnStructure"
    );
    assert_contract_rejects_without_mutation(&before, &result);
}

#[test]
fn cleanup_contract_rejects_wrong_event_order() {
    let (before, after) = cleanup_before_after();
    let events = vec![
        AuthoritativeRuleEvent {
            event_id: RuleEventId(before.allocators.next_rule_event_id.0),
            state_revision: StateRevision(before.revision.0 + 1),
            event: AuthoritativeRuleEventKind::ActivePlayerChanged {
                from: PlayerId(7),
                to: PlayerId(42),
            },
        },
        AuthoritativeRuleEvent {
            event_id: RuleEventId(before.allocators.next_rule_event_id.0 + 1),
            state_revision: StateRevision(before.revision.0 + 1),
            event: AuthoritativeRuleEventKind::TurnNumberChanged {
                from: 1,
                to: 2,
            },
        },
        AuthoritativeRuleEvent {
            event_id: RuleEventId(before.allocators.next_rule_event_id.0 + 2),
            state_revision: StateRevision(before.revision.0 + 1),
            event: AuthoritativeRuleEventKind::TurnPositionChanged {
                from: TurnPosition::Ending {
                    step: EndingStep::Cleanup,
                },
                to: TurnPosition::Beginning {
                    step: BeginningStep::Untap,
                },
            },
        },
    ];
    let result = accepted_product_for_contract(&before, after, events);
    assert!(
        matches!(validate_transition_contract(&before, &result), Err(TransitionViolation::TurnStructure)),
        "ActivePlayerChanged before TurnNumberChanged must reject via TurnStructure"
    );
    assert_contract_rejects_without_mutation(&before, &result);
}

#[test]
fn cleanup_contract_rejects_wrong_turn_increment() {
    let (before, mut after) = cleanup_before_after();
    after.core.turn_number = 3;
    let events = vec![
        AuthoritativeRuleEvent {
            event_id: RuleEventId(before.allocators.next_rule_event_id.0),
            state_revision: StateRevision(before.revision.0 + 1),
            event: AuthoritativeRuleEventKind::TurnNumberChanged {
                from: 1,
                to: 3,
            },
        },
        AuthoritativeRuleEvent {
            event_id: RuleEventId(before.allocators.next_rule_event_id.0 + 1),
            state_revision: StateRevision(before.revision.0 + 1),
            event: AuthoritativeRuleEventKind::ActivePlayerChanged {
                from: PlayerId(7),
                to: PlayerId(42),
            },
        },
        AuthoritativeRuleEvent {
            event_id: RuleEventId(before.allocators.next_rule_event_id.0 + 2),
            state_revision: StateRevision(before.revision.0 + 1),
            event: AuthoritativeRuleEventKind::TurnPositionChanged {
                from: TurnPosition::Ending {
                    step: EndingStep::Cleanup,
                },
                to: TurnPosition::Beginning {
                    step: BeginningStep::Untap,
                },
            },
        },
    ];
    let result = accepted_product_for_contract(&before, after, events);
    assert!(
        matches!(validate_transition_contract(&before, &result), Err(TransitionViolation::TurnStructure)),
        "wrong turn increment (1->3) must reject via TurnStructure"
    );
    assert_contract_rejects_without_mutation(&before, &result);
}

#[test]
fn cleanup_contract_rejects_wrong_active_player() {
    let (before, mut after) = cleanup_before_after();
    after.core.active_player = PlayerId(7);
    let events = vec![
        AuthoritativeRuleEvent {
            event_id: RuleEventId(before.allocators.next_rule_event_id.0),
            state_revision: StateRevision(before.revision.0 + 1),
            event: AuthoritativeRuleEventKind::TurnNumberChanged {
                from: 1,
                to: 2,
            },
        },
        AuthoritativeRuleEvent {
            event_id: RuleEventId(before.allocators.next_rule_event_id.0 + 1),
            state_revision: StateRevision(before.revision.0 + 1),
            event: AuthoritativeRuleEventKind::ActivePlayerChanged {
                from: PlayerId(7),
                to: PlayerId(7),
            },
        },
        AuthoritativeRuleEvent {
            event_id: RuleEventId(before.allocators.next_rule_event_id.0 + 2),
            state_revision: StateRevision(before.revision.0 + 1),
            event: AuthoritativeRuleEventKind::TurnPositionChanged {
                from: TurnPosition::Ending {
                    step: EndingStep::Cleanup,
                },
                to: TurnPosition::Beginning {
                    step: BeginningStep::Untap,
                },
            },
        },
    ];
    let result = accepted_product_for_contract(&before, after, events);
    assert!(
        matches!(validate_transition_contract(&before, &result), Err(TransitionViolation::TurnStructure)),
        "same active player must reject via TurnStructure"
    );
    assert_contract_rejects_without_mutation(&before, &result);
}

#[test]
fn cleanup_contract_rejects_extra_event() {
    let (before, after) = cleanup_before_after();
    let events = vec![
        AuthoritativeRuleEvent {
            event_id: RuleEventId(before.allocators.next_rule_event_id.0),
            state_revision: StateRevision(before.revision.0 + 1),
            event: AuthoritativeRuleEventKind::TurnNumberChanged {
                from: 1,
                to: 2,
            },
        },
        AuthoritativeRuleEvent {
            event_id: RuleEventId(before.allocators.next_rule_event_id.0 + 1),
            state_revision: StateRevision(before.revision.0 + 1),
            event: AuthoritativeRuleEventKind::ActivePlayerChanged {
                from: PlayerId(7),
                to: PlayerId(42),
            },
        },
        AuthoritativeRuleEvent {
            event_id: RuleEventId(before.allocators.next_rule_event_id.0 + 2),
            state_revision: StateRevision(before.revision.0 + 1),
            event: AuthoritativeRuleEventKind::TurnPositionChanged {
                from: TurnPosition::Ending {
                    step: EndingStep::Cleanup,
                },
                to: TurnPosition::Beginning {
                    step: BeginningStep::Untap,
                },
            },
        },
        AuthoritativeRuleEvent {
            event_id: RuleEventId(before.allocators.next_rule_event_id.0 + 3),
            state_revision: StateRevision(before.revision.0 + 1),
            event: AuthoritativeRuleEventKind::TurnNumberChanged {
                from: 2,
                to: 3,
            },
        },
    ];
    let result = accepted_product_for_contract(&before, after, events);
    assert!(
        matches!(validate_transition_contract(&before, &result), Err(TransitionViolation::TurnStructure)),
        "extra event must reject via TurnStructure"
    );
    assert_contract_rejects_without_mutation(&before, &result);
}

// --- FIX B: unique-other player in transition contract ---

#[test]
fn cleanup_contract_rejects_three_player_non_unique_other() {
    let before = three_player_s1_state_at(mtgml_state::TurnPosition::Ending {
        step: mtgml_state::EndingStep::Cleanup,
    });
    let mut after = before.clone();
    after.revision = StateRevision(before.revision.0 + 1);
    after.core.turn_number = 2;
    after.core.active_player = PlayerId(99);
    after.core.position = TurnPosition::Beginning {
        step: BeginningStep::Untap,
    };
    after.allocators.next_rule_event_id = RuleEventId(before.allocators.next_rule_event_id.0 + 3);
    let events = vec![
        AuthoritativeRuleEvent {
            event_id: RuleEventId(before.allocators.next_rule_event_id.0),
            state_revision: StateRevision(before.revision.0 + 1),
            event: AuthoritativeRuleEventKind::TurnNumberChanged {
                from: 1,
                to: 2,
            },
        },
        AuthoritativeRuleEvent {
            event_id: RuleEventId(before.allocators.next_rule_event_id.0 + 1),
            state_revision: StateRevision(before.revision.0 + 1),
            event: AuthoritativeRuleEventKind::ActivePlayerChanged {
                from: PlayerId(7),
                to: PlayerId(99),
            },
        },
        AuthoritativeRuleEvent {
            event_id: RuleEventId(before.allocators.next_rule_event_id.0 + 2),
            state_revision: StateRevision(before.revision.0 + 1),
            event: AuthoritativeRuleEventKind::TurnPositionChanged {
                from: TurnPosition::Ending {
                    step: EndingStep::Cleanup,
                },
                to: TurnPosition::Beginning {
                    step: BeginningStep::Untap,
                },
            },
        },
    ];
    let result = accepted_product_for_contract(&before, after, events);
    assert!(
        matches!(validate_transition_contract(&before, &result), Err(TransitionViolation::TurnStructure)),
        "non-unique-other in 3-player state must reject via TurnStructure"
    );
    assert_contract_rejects_without_mutation(&before, &result);
}

// --- FIX C: unique-other in semantic cursor ---

#[test]
fn active_player_changed_cursor_rejects_three_player_target() {
    let before = three_player_s1_state_at(mtgml_state::TurnPosition::Beginning {
        step: BeginningStep::Untap,
    });
    use crate::semantic_cursor::SemanticValidationCursor;
    use crate::events::AuthoritativeRuleEventKind;

    let mut cursor = SemanticValidationCursor::from_state(&before)
        .expect("cursor from 3-player state should be constructible");
    let event = AuthoritativeRuleEventKind::ActivePlayerChanged {
        from: PlayerId(7),
        to: PlayerId(99),
    };
    assert!(
        matches!(cursor.apply(&event), Err(TransitionViolation::TurnStructure)),
        "ActivePlayerChanged to non-unique-other in 3-player cursor must reject via TurnStructure"
    );
}

// --- FIX D: unchecked turn arithmetic removed from validator ---

#[test]
fn cleanup_contract_max_turn_rejects_without_panic() {
    let mut before = s1_state_at(mtgml_state::TurnPosition::Ending {
        step: mtgml_state::EndingStep::Cleanup,
    });
    before.core.turn_number = u64::MAX;
    let mut after = before.clone();
    after.revision = StateRevision(before.revision.0 + 1);
    after.core.turn_number = 0;
    after.core.active_player = PlayerId(42);
    after.core.position = TurnPosition::Beginning {
        step: BeginningStep::Untap,
    };
    after.allocators.next_rule_event_id = RuleEventId(before.allocators.next_rule_event_id.0 + 3);
    let events = vec![
        AuthoritativeRuleEvent {
            event_id: RuleEventId(before.allocators.next_rule_event_id.0),
            state_revision: StateRevision(before.revision.0 + 1),
            event: AuthoritativeRuleEventKind::TurnNumberChanged {
                from: u64::MAX,
                to: 0,
            },
        },
        AuthoritativeRuleEvent {
            event_id: RuleEventId(before.allocators.next_rule_event_id.0 + 1),
            state_revision: StateRevision(before.revision.0 + 1),
            event: AuthoritativeRuleEventKind::ActivePlayerChanged {
                from: PlayerId(7),
                to: PlayerId(42),
            },
        },
        AuthoritativeRuleEvent {
            event_id: RuleEventId(before.allocators.next_rule_event_id.0 + 2),
            state_revision: StateRevision(before.revision.0 + 1),
            event: AuthoritativeRuleEventKind::TurnPositionChanged {
                from: TurnPosition::Ending {
                    step: EndingStep::Cleanup,
                },
                to: TurnPosition::Beginning {
                    step: BeginningStep::Untap,
                },
            },
        },
    ];
    let result = accepted_product_for_contract(&before, after, events);
    assert!(
        matches!(validate_transition_contract(&before, &result), Err(TransitionViolation::TurnStructure)),
        "u64::MAX Cleanup turn must reject via TurnStructure without panic"
    );
    assert_contract_rejects_without_mutation(&before, &result);
}

// --- FIX E: Task-7 events forbidden outside Cleanup boundary ---

#[test]
fn turn_switch_events_reject_outside_cleanup_boundary() {
    let before = s1_state_at(mtgml_state::TurnPosition::Beginning {
        step: BeginningStep::Upkeep,
    });
    let mut after = before.clone();
    after.revision = StateRevision(before.revision.0 + 1);
    after.allocators.next_rule_event_id = RuleEventId(before.allocators.next_rule_event_id.0 + 1);
    let events = vec![AuthoritativeRuleEvent {
        event_id: RuleEventId(before.allocators.next_rule_event_id.0),
        state_revision: StateRevision(before.revision.0 + 1),
        event: AuthoritativeRuleEventKind::TurnNumberChanged {
            from: 1,
            to: 2,
        },
    }];
    let result = accepted_product_for_contract(&before, after, events);
    assert!(
        matches!(validate_transition_contract(&before, &result), Err(TransitionViolation::TurnStructure)),
        "TurnNumberChanged outside Cleanup must reject via TurnStructure"
    );
    assert_contract_rejects_without_mutation(&before, &result);
}

#[test]
fn active_player_transient_switch_outside_cleanup_rejects() {
    let before = s1_state_at(mtgml_state::TurnPosition::Beginning {
        step: BeginningStep::Upkeep,
    });
    let mut after = before.clone();
    after.revision = StateRevision(before.revision.0 + 1);
    after.allocators.next_rule_event_id = RuleEventId(before.allocators.next_rule_event_id.0 + 2);
    let events = vec![
        AuthoritativeRuleEvent {
            event_id: RuleEventId(before.allocators.next_rule_event_id.0),
            state_revision: StateRevision(before.revision.0 + 1),
            event: AuthoritativeRuleEventKind::ActivePlayerChanged {
                from: PlayerId(7),
                to: PlayerId(42),
            },
        },
        AuthoritativeRuleEvent {
            event_id: RuleEventId(before.allocators.next_rule_event_id.0 + 1),
            state_revision: StateRevision(before.revision.0 + 1),
            event: AuthoritativeRuleEventKind::ActivePlayerChanged {
                from: PlayerId(42),
                to: PlayerId(7),
            },
        },
    ];
    let result = accepted_product_for_contract(&before, after, events);
    assert!(
        matches!(validate_transition_contract(&before, &result), Err(TransitionViolation::TurnStructure)),
        "transient ActivePlayerChanged outside Cleanup must reject via TurnStructure"
    );
    assert_contract_rejects_without_mutation(&before, &result);
}

// --- Task 6 regressions (must remain green) ---

#[test]
fn magic_turn_structure_kernel_shell_rejects_unsupported_cleanup_old() {
    let state = s1_state_at(mtgml_state::TurnPosition::Ending {
        step: mtgml_state::EndingStep::EndStep,
    });
    let before = state.clone();
    let mut kernel = MagicRulesKernel::new();
    let result = kernel.advance_forced_progress(&state);
    assert!(
        result.is_err(),
        "EndStep still routes through unsupported boundary"
    );
    assert_eq!(state, before, "rejected work must not mutate input");
}

// --- Invalid profile rejection tests (section 27) ---

#[test]
fn magic_turn_structure_kernel_shell_rejects_held_priority() {
    let mut state = s1_state_at(mtgml_state::TurnPosition::Beginning {
        step: mtgml_state::BeginningStep::Untap,
    });
    state.core.priority = mtgml_state::PriorityState::HeldBy {
        player: PlayerId(7),
        consecutive_passes: 0,
    };
    let before = state.clone();
    let mut kernel = MagicRulesKernel::new();
    let result = kernel.advance_forced_progress(&state);
    assert!(
        matches!(
            result,
            Err(crate::KernelExecutionError::TurnStructure(
                crate::TurnStructureError::PriorityHeld
            ))
        ),
        "held priority must fail through the S1 support validator"
    );
    assert_eq!(state, before, "rejected work must not mutate input");
}

#[test]
fn magic_turn_structure_kernel_shell_rejects_three_players() {
    let state = three_player_s1_state();
    let before = state.clone();
    let mut kernel = MagicRulesKernel::new();
    let result = kernel.advance_forced_progress(&state);
    assert!(
        matches!(
            result,
            Err(crate::KernelExecutionError::TurnStructure(
                crate::TurnStructureError::UnsupportedPlayerCount
            ))
        ),
        "three-player states must fail through the S1 support validator"
    );
    assert_eq!(state, before, "rejected work must not mutate input");
}

// --- Program dispatch negative (section 28) ---

#[test]
fn magic_turn_structure_kernel_shell_program_dispatch_still_unsupported() {
    let result = crate::ProgramKernelV1::for_program(
        mtgml_model::ExecutionProgramV1::MagicRules,
    );
    assert!(
        matches!(
            result,
            Err(crate::ProgramKernelConstructionErrorV1::UnsupportedProgram)
        ),
        "MagicRules must remain unsupported at the program kernel boundary"
    );
}

// --- Positive ordinary untap tests (section 24) ---

/// Adds a Battlefield object with the given controller and tapped state to the
/// state, including the mandatory knowledge/perspective-identity bookkeeping
/// required for `validate_engine_state` to accept the fixture.
fn add_battlefield_object(state: &mut EngineState, object_id: GameObjectId, controller: PlayerId, tapped: bool) {
    use mtgml_model::{CardDefinitionId, OpaqueObjectId, PhysicalCardId, ZoneKind};
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
            owner: controller,
            controller,
            tapped,
            face_down: false,
        },
    );

    let location = ZoneLocation {
        zone: ZoneKind::Battlefield,
        player: None,
        position: ZonePosition::Unordered,
        visibility: VisibilityPartition::Public,
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
            location: location.clone(),
            provenance: KnowledgeAcquisitionReason::InitialConfiguration,
        }),
        acquisition: KnowledgeAcquisitionReason::InitialConfiguration,
        historical_locations: Vec::new(),
    };

    for player_id in state.core.players.keys().copied().collect::<Vec<_>>() {
        state
            .knowledge
            .players
            .get_mut(&player_id)
            .unwrap()
            .active
            .insert(opaque, knowledge_record.clone());
        let identity = state
            .perspective_identities
            .players
            .get_mut(&player_id)
            .unwrap();
        identity.opaque_to_object.insert(opaque, object_id);
        identity.object_to_opaque.insert(object_id, opaque);
        if identity.next_opaque_object_id.0 <= opaque.0 {
            identity.next_opaque_object_id = OpaqueObjectId(opaque.0 + 1);
        }
    }
}

fn untap_state_with_one_active_tapped() -> EngineState {
    let mut state = s1_state_at(mtgml_state::TurnPosition::Beginning {
        step: mtgml_state::BeginningStep::Untap,
    });
    state
        .zones
        .objects
        .get_mut(&GameObjectId(1))
        .unwrap()
        .tapped = true;
    state
}

#[test]
fn magic_turn_structure_untap_one_active_tapped() {
    let state = untap_state_with_one_active_tapped();
    let before = state.clone();
    let mut kernel = MagicRulesKernel::new();
    let result = kernel
        .advance_forced_progress(&state)
        .expect("ordinary untap must be accepted at Beginning(Untap)");
    assert_eq!(state, before, "input state must not be mutated");

    assert!(result.accepted);
    assert_eq!(
        result.next_state.revision,
        StateRevision(before.revision.0 + 1)
    );
    assert_eq!(
        result.next_state.core.position,
        TurnPosition::Beginning {
            step: BeginningStep::Upkeep,
        }
    );
    assert!(
        !result.next_state.zones.objects[&GameObjectId(1)].tapped,
        "untapped object must have tapped=false"
    );

    assert_eq!(result.events.len(), 2);
    assert!(matches!(
        &result.events[0].event,
        AuthoritativeRuleEventKind::UntapCompleted { affected_objects }
        if *affected_objects == vec![GameObjectId(1)]
    ));
    assert_eq!(result.events[0].event_id, before.allocators.next_rule_event_id);
    assert_eq!(
        result.events[0].state_revision,
        result.next_state.revision,
    );
    assert!(matches!(
        &result.events[1].event,
        AuthoritativeRuleEventKind::TurnPositionChanged { from, to }
        if *from == TurnPosition::Beginning { step: BeginningStep::Untap }
        && *to == TurnPosition::Beginning { step: BeginningStep::Upkeep }
    ));
    assert_eq!(
        result.events[1].event_id,
        RuleEventId(before.allocators.next_rule_event_id.0 + 1),
    );
    assert_eq!(
        result.events[1].state_revision,
        result.next_state.revision,
    );

    assert!(result.next_decision.is_none());
    assert_eq!(result.status, mtgml_model::EpisodeStatus::Running);
}

#[test]
fn magic_turn_structure_untap_empty_affected_set() {
    let state = s1_state_at(mtgml_state::TurnPosition::Beginning {
        step: mtgml_state::BeginningStep::Untap,
    });
    let before = state.clone();
    let mut kernel = MagicRulesKernel::new();
    let result = kernel
        .advance_forced_progress(&state)
        .expect("ordinary untap with no eligible objects must still complete the boundary");
    assert_eq!(state, before, "input state must not be mutated");

    assert!(result.accepted);
    assert_eq!(
        result.next_state.core.position,
        TurnPosition::Beginning {
            step: BeginningStep::Upkeep,
        }
    );
    assert_eq!(result.events.len(), 2);
    assert!(matches!(
        &result.events[0].event,
        AuthoritativeRuleEventKind::UntapCompleted { affected_objects }
        if affected_objects.is_empty()
    ));
    assert!(matches!(
        &result.events[1].event,
        AuthoritativeRuleEventKind::TurnPositionChanged { .. }
    ));
    assert!(result.next_decision.is_none());
}

#[test]
fn magic_turn_structure_untap_nonactive_control_remains_tapped() {
    let mut state = s1_state_at(mtgml_state::TurnPosition::Beginning {
        step: mtgml_state::BeginningStep::Untap,
    });
    state
        .zones
        .objects
        .get_mut(&GameObjectId(1))
        .unwrap()
        .tapped = true;
    add_battlefield_object(&mut state, GameObjectId(3), PlayerId(42), true);
    let before = state.clone();
    let mut kernel = MagicRulesKernel::new();
    let result = kernel
        .advance_forced_progress(&state)
        .expect("ordinary untap must be accepted");
    assert_eq!(state, before, "input state must not be mutated");

    assert!(result.accepted);
    assert!(matches!(
        &result.events[0].event,
        AuthoritativeRuleEventKind::UntapCompleted { affected_objects }
        if *affected_objects == vec![GameObjectId(1)]
    ));
    assert!(
        result.next_state.zones.objects[&GameObjectId(3)].tapped,
        "nonactive-controlled object must remain tapped"
    );
    assert!(
        !result.next_state.zones.objects[&GameObjectId(1)].tapped,
        "active-controlled object must be untapped"
    );
    assert_eq!(
        result.next_state.core.position,
        TurnPosition::Beginning {
            step: BeginningStep::Upkeep,
        }
    );
}

#[test]
fn magic_turn_structure_untap_multiple_objects_canonical() {
    let mut state = s1_state_at(mtgml_state::TurnPosition::Beginning {
        step: mtgml_state::BeginningStep::Untap,
    });
    state
        .zones
        .objects
        .get_mut(&GameObjectId(1))
        .unwrap()
        .tapped = true;
    add_battlefield_object(&mut state, GameObjectId(3), PlayerId(7), true);
    let before = state.clone();
    let mut kernel = MagicRulesKernel::new();
    let result = kernel
        .advance_forced_progress(&state)
        .expect("ordinary untap with multiple eligible objects must be accepted");
    assert_eq!(state, before, "input state must not be mutated");

    assert!(result.accepted);
    assert!(matches!(
        &result.events[0].event,
        AuthoritativeRuleEventKind::UntapCompleted { affected_objects }
        if *affected_objects == vec![GameObjectId(1), GameObjectId(3)]
    ));
    assert!(
        !result.next_state.zones.objects[&GameObjectId(1)].tapped,
        "first object must be untapped"
    );
    assert!(
        !result.next_state.zones.objects[&GameObjectId(3)].tapped,
        "second object must be untapped"
    );
    assert_eq!(result.events[0].event_id, before.allocators.next_rule_event_id);
    assert_eq!(
        result.events[1].event_id,
        RuleEventId(before.allocators.next_rule_event_id.0 + 1)
    );
}

#[test]
fn magic_turn_structure_untap_narrow_mutation() {
    let state = untap_state_with_one_active_tapped();
    let mut kernel = MagicRulesKernel::new();
    let result = kernel
        .advance_forced_progress(&state)
        .expect("ordinary untap must be accepted");

    let mut expected_after = state.clone();
    expected_after.revision = StateRevision(state.revision.0 + 1);
    expected_after.allocators.next_rule_event_id =
        RuleEventId(state.allocators.next_rule_event_id.0 + 2);
    expected_after.core.position = TurnPosition::Beginning {
        step: BeginningStep::Upkeep,
    };
    expected_after
        .zones
        .objects
        .get_mut(&GameObjectId(1))
        .unwrap()
        .tapped = false;

    assert_eq!(
        result.next_state, expected_after,
        "ordinary untap must only change revision, rule-event allocator, position, and tapped"
    );
}

#[test]
fn magic_turn_structure_untap_exact_delta_audit_and_reapply() {
    let state = untap_state_with_one_active_tapped();
    let before = state.clone();
    let mut kernel = MagicRulesKernel::new();
    let result = kernel
        .advance_forced_progress(&state)
        .expect("ordinary untap must be accepted");

    // Exact audit sequence must match event order.
    assert_eq!(
        result.delta.audit,
        vec![
            SemanticDeltaOperation::UntapCompleted {
                affected_objects: vec![GameObjectId(1)],
            },
            SemanticDeltaOperation::TurnPositionChanged {
                from: TurnPosition::Beginning {
                    step: BeginningStep::Untap,
                },
                to: TurnPosition::Beginning {
                    step: BeginningStep::Upkeep,
                },
            },
        ]
    );

    // Delta reapplication must reconstruct the exact next state.
    assert_eq!(
        result.delta.apply(&before).unwrap(),
        result.next_state,
    );
}

// --- Task 7 FIX_03 RED: kernel-level quiescent Cleanup detection ---

fn cleanup_state_with_active_hand(count: usize) -> EngineState {
    let mut state = s1_state_at(mtgml_state::TurnPosition::Ending {
        step: mtgml_state::EndingStep::Cleanup,
    });
    for i in 3..(3 + count as u64) {
        add_hand_card(&mut state, GameObjectId(i), PlayerId(7));
    }
    state
}

fn cleanup_state_with_opponent_hand(count: usize) -> EngineState {
    let mut state = s1_state_at(mtgml_state::TurnPosition::Ending {
        step: mtgml_state::EndingStep::Cleanup,
    });
    for i in 3..(3 + count as u64) {
        add_hand_card(&mut state, GameObjectId(i), PlayerId(42));
    }
    state
}

#[test]
fn cleanup_active_hand_at_limit_is_quiescent() {
    let state = cleanup_state_with_active_hand(7);
    let before = state.clone();
    let mut kernel = MagicRulesKernel::new();
    let result = kernel
        .advance_forced_progress(&state)
        .expect("quiescent cleanup with active hand at 7 must be accepted");
    assert_eq!(state, before, "input state must not be mutated");
    assert!(result.accepted);
    assert_eq!(result.next_state.core.active_player, PlayerId(42));
    assert_eq!(result.next_state.core.turn_number, 2);
    assert_eq!(
        result.next_state.core.position,
        TurnPosition::Beginning { step: BeginningStep::Untap }
    );
    assert!(result.next_decision.is_none());
}

#[test]
fn cleanup_active_hand_above_limit_rejects() {
    let state = cleanup_state_with_active_hand(8);
    let before = state.clone();
    let mut kernel = MagicRulesKernel::new();
    let result = kernel.advance_forced_progress(&state);
    assert!(
        matches!(
            result,
            Err(crate::KernelExecutionError::UnsupportedRulesBoundary(
                crate::UnsupportedRulesBoundary::CleanupReset
            ))
        ),
        "active hand above 7 at Cleanup must fail closed with CleanupReset"
    );
    assert_eq!(state, before, "rejected cleanup must not mutate input");
}

#[test]
fn cleanup_opponent_large_hand_does_not_block_active_cleanup() {
    let state = cleanup_state_with_opponent_hand(8);
    let before = state.clone();
    let mut kernel = MagicRulesKernel::new();
    let result = kernel
        .advance_forced_progress(&state)
        .expect("opponent hand above 7 must not block active cleanup");
    assert_eq!(state, before, "input state must not be mutated");
    assert!(result.accepted);
    assert_eq!(result.next_state.core.active_player, PlayerId(42));
}

#[test]
fn cleanup_marked_damage_rejects() {
    let state = cleanup_state_with_marked_damage();
    let before = state.clone();
    let mut kernel = MagicRulesKernel::new();
    let result = kernel.advance_forced_progress(&state);
    assert!(
        matches!(
            result,
            Err(crate::KernelExecutionError::UnsupportedRulesBoundary(
                crate::UnsupportedRulesBoundary::CleanupReset
            ))
        ),
        "marked damage at Cleanup must fail closed with CleanupReset"
    );
    assert_eq!(state, before, "rejected cleanup must not mutate input");
}

#[test]
fn cleanup_ambiguous_hand_ownership_rejects() {
    let mut state = s1_state_at(mtgml_state::TurnPosition::Ending {
        step: mtgml_state::EndingStep::Cleanup,
    });
    add_ambiguous_hand_card(&mut state, GameObjectId(3), PlayerId(7));
    assert!(
        mtgml_state::validate_engine_state(&state).is_ok(),
        "fixture with ambiguous Hand must still pass generic engine-state validation"
    );
    let before = state.clone();
    let mut kernel = MagicRulesKernel::new();
    let result = kernel.advance_forced_progress(&state);
    assert!(
        matches!(
            result,
            Err(crate::KernelExecutionError::UnsupportedRulesBoundary(
                crate::UnsupportedRulesBoundary::CleanupReset
            ))
        ),
        "Hand-zone location with player=None must fail closed with CleanupReset"
    );
    assert_eq!(state, before, "rejected cleanup must not mutate input");
}

// === Admitted construction evidence (Task 8) ===

use crate::magic::MagicExecutionProfile;
use mtgml_model::{ExecutionProgramV1, SemanticContractIdV1};

#[test]
fn kernel_from_admitted_profile_construction() {
    let profile = MagicExecutionProfile::new(SemanticContractIdV1::from_digest_bytes([0u8; 32]));
    let _kernel = MagicRulesKernel::from_admitted_profile(profile);
}

#[test]
fn kernel_new_is_test_construction_only() {
    let _kernel = MagicRulesKernel::new();
}

#[test]
fn for_program_magic_rules_still_unsupported() {
    let result = ProgramKernelV1::for_program(ExecutionProgramV1::MagicRules);
    assert!(
        matches!(
            result,
            Err(ProgramKernelConstructionErrorV1::UnsupportedProgram)
        ),
        "for_program(MagicRules) must still be UnsupportedProgram"
    );
}

#[test]
fn for_admitted_execution_rejects_synthetic_program() {
    let result = ProgramKernelV1::for_admitted_execution(
        ExecutionProgramV1::SyntheticRulesCompat,
        SemanticContractIdV1::from_digest_bytes([0u8; 32]),
    );
    assert!(
        matches!(
            result,
            Err(ProgramKernelConstructionErrorV1::UnsupportedProgram)
        ),
        "for_admitted_execution(SyntheticRulesCompat) must be UnsupportedProgram"
    );
}
