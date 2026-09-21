// Private rules tests for the unreachable MagicRulesKernel shell.
//
// These tests verify that the Magic kernel shell is fail-closed: no player
// response is accepted, all temporal positions classify at their downstream
// boundary, and no state mutation occurs on any rejection path.

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
    let mut state = s1_state_at(mtgml_state::TurnPosition::Beginning {
        step: mtgml_state::BeginningStep::Untap,
    });
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

// --- Owned-but-not-yet-implemented boundary tests (section 16) ---

#[test]
fn magic_turn_structure_kernel_shell_rejects_unimplemented_cleanup() {
    let state = s1_state_at(mtgml_state::TurnPosition::Ending {
        step: mtgml_state::EndingStep::Cleanup,
    });
    let before = state.clone();
    let mut kernel = MagicRulesKernel::new();
    let result = kernel.advance_forced_progress(&state);
    assert!(
        result.is_err(),
        "quiescent cleanup is not yet implemented and must not be accepted"
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
