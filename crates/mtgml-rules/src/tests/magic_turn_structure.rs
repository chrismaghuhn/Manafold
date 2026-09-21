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
fn magic_turn_structure_kernel_shell_rejects_unimplemented_untap() {
    let state = s1_state_at(mtgml_state::TurnPosition::Beginning {
        step: mtgml_state::BeginningStep::Untap,
    });
    let before = state.clone();
    let mut kernel = MagicRulesKernel::new();
    let result = kernel.advance_forced_progress(&state);
    assert!(
        result.is_err(),
        "ordinary untap is not yet implemented and must not be accepted"
    );
    assert_eq!(state, before, "rejected work must not mutate input");
}

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
