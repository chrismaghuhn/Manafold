//! Durable `MagicRulesKernel` owner.
//!
//! This module establishes the milestone-free, future-authoritative owner of
//! Magic execution inside `mtgml-rules`. The kernel is structurally present but
//! intentionally unreachable from `ProgramKernelV1` and from the environment in
//! this task: `ExecutionProgramV1::MagicRules` remains
//! `ProgramKernelConstructionErrorV1::UnsupportedProgram`.
//!
//! The shell is fail-closed by construction:
//! - `apply()` never accepts a player response; S1 has no player decision
//!   surface in the supported slice.
//! - `advance_forced_progress()` validates the S1 supported-state profile, then
//!   classifies the current temporal position at its downstream boundary and
//!   returns a typed `Err` for every position. No transition is accepted until
//!   the corresponding semantic operation exists.

use mtgml_decision::DecisionResponseV2;
use mtgml_model::{PlayerId, RuleEventId, StateRevision};
use mtgml_state::{validate_engine_state, BeginningStep, EndingStep, EngineState, TurnPosition};

use crate::errors::KernelExecutionError;
use crate::events::{AuthoritativeRuleEvent, AuthoritativeRuleEventKind};
use crate::product::build_accepted_product;
use crate::transition::{RulesKernel, TransitionResult};
use crate::turn_structure::{
    derive_ordinary_untap_affected_objects, temporal_successor, unsupported_rules_boundary,
    validate_quiescent_cleanup_boundary, validate_turn_structure_support, TurnStructureError,
    TurnStructureSupportProfile, UnsupportedRulesBoundary,
};

/// Durable, milestone-free owner of Magic execution.
///
/// Reachable only from crate-internal tests in this task. `Task 8` owns the
/// contract-aware constructor that binds this kernel to V5 admission.
#[allow(dead_code)]
pub(crate) struct MagicRulesKernel;

impl RulesKernel for MagicRulesKernel {
    /// Trusted response execution entry point.
    ///
    /// S1 has no player decision surface: every `DecisionResponseV2` is
    /// rejected without inspecting its contents, and without mutating the input
    /// `&EngineState`.
    fn apply(
        &mut self,
        _state: &EngineState,
        _trusted_actor: PlayerId,
        _response: &DecisionResponseV2,
    ) -> Result<TransitionResult, KernelExecutionError> {
        Err(KernelExecutionError::UnsupportedPlayerResponse)
    }
}

#[allow(dead_code)]
impl MagicRulesKernel {
    /// Rules-owned forced-progress shell.
    ///
    /// Validates the S1 supported-state profile, then classifies the current
    /// temporal position. The Untap position executes ordinary untap and
    /// advances to Upkeep; every other position is classified at its
    /// downstream boundary and returns a typed `Err`. No state mutation
    /// occurs on any rejection path.
    pub(crate) fn advance_forced_progress(
        &mut self,
        state: &EngineState,
    ) -> Result<TransitionResult, KernelExecutionError> {
        validate_engine_state(state).map_err(KernelExecutionError::BeforeState)?;
        let profile =
            validate_turn_structure_support(state).map_err(KernelExecutionError::TurnStructure)?;
        let position = profile.position();

        if matches!(
            position,
            TurnPosition::Beginning {
                step: BeginningStep::Untap,
            }
        ) {
            return self.ordinary_untap(state, &profile);
        }

        if matches!(
            position,
            TurnPosition::Ending {
                step: EndingStep::Cleanup,
            }
        ) {
            return self.advance_quiescent_cleanup(state, &profile);
        }

        match unsupported_rules_boundary(position) {
            Some(boundary) => Err(KernelExecutionError::UnsupportedRulesBoundary(boundary)),
            None => Err(KernelExecutionError::UnsupportedStagePath),
        }
    }

    /// Ordinary untap: derives the complete affected set from authoritative
    /// object snapshots, clears `tapped` for every member simultaneously, emits
    /// `UntapCompleted` and `TurnPositionChanged`, and advances to Upkeep.
    fn ordinary_untap(
        &mut self,
        state: &EngineState,
        profile: &TurnStructureSupportProfile,
    ) -> Result<TransitionResult, KernelExecutionError> {
        let active_player = profile.active_player();
        let from = profile.position();
        let to = temporal_successor(from);

        let snapshots = crate::snapshots::object_snapshots(state)
            .map_err(KernelExecutionError::TransitionContract)?;
        let affected = derive_ordinary_untap_affected_objects(&snapshots, active_player);

        let next_revision = StateRevision(
            state
                .revision
                .0
                .checked_add(1)
                .ok_or(KernelExecutionError::RevisionOverflow)?,
        );

        let second_event_id = RuleEventId(
            state
                .allocators
                .next_rule_event_id
                .0
                .checked_add(1)
                .ok_or(KernelExecutionError::RuleEventIdOverflow)?,
        );

        let events = vec![
            AuthoritativeRuleEvent {
                event_id: state.allocators.next_rule_event_id,
                state_revision: next_revision,
                event: AuthoritativeRuleEventKind::UntapCompleted {
                    affected_objects: affected.clone(),
                },
            },
            AuthoritativeRuleEvent {
                event_id: second_event_id,
                state_revision: next_revision,
                event: AuthoritativeRuleEventKind::TurnPositionChanged { from, to },
            },
        ];

        let mut next = state.clone();
        next.revision = next_revision;

        build_accepted_product(state, next, events, |workspace| {
            workspace.core.position = to;
            for object_id in &affected {
                if let Some(object) = workspace.zones.objects.get_mut(object_id) {
                    object.tapped = false;
                }
            }
            Ok(())
        })
    }

    /// Quiescent Cleanup boundary: switches active player to the unique other
    /// declared player, increments turn_number with checked arithmetic, and
    /// advances position to the next player's Beginning(Untap). Priority remains
    /// None. No damage removal, discard, duration expiry, or cleanup trigger
    /// work is performed.
    fn advance_quiescent_cleanup(
        &mut self,
        state: &EngineState,
        profile: &TurnStructureSupportProfile,
    ) -> Result<TransitionResult, KernelExecutionError> {
        validate_quiescent_cleanup_boundary(state, profile.active_player()).map_err(|_| {
            KernelExecutionError::UnsupportedRulesBoundary(UnsupportedRulesBoundary::CleanupReset)
        })?;

        let old_turn = profile.turn_number();
        let old_active = profile.active_player();
        let new_active = profile.other_player();
        let new_turn = old_turn
            .checked_add(1)
            .ok_or(KernelExecutionError::TurnStructure(
                TurnStructureError::TurnNumberOverflow,
            ))?;
        let from = profile.position();
        let to = temporal_successor(from);

        let first_event_id = state.allocators.next_rule_event_id;
        let second_event_id = RuleEventId(
            state
                .allocators
                .next_rule_event_id
                .0
                .checked_add(1)
                .ok_or(KernelExecutionError::RuleEventIdOverflow)?,
        );
        let third_event_id = RuleEventId(
            state
                .allocators
                .next_rule_event_id
                .0
                .checked_add(2)
                .ok_or(KernelExecutionError::RuleEventIdOverflow)?,
        );

        let next_revision = StateRevision(
            state
                .revision
                .0
                .checked_add(1)
                .ok_or(KernelExecutionError::RevisionOverflow)?,
        );

        let events = vec![
            AuthoritativeRuleEvent {
                event_id: first_event_id,
                state_revision: next_revision,
                event: AuthoritativeRuleEventKind::TurnNumberChanged {
                    from: old_turn,
                    to: new_turn,
                },
            },
            AuthoritativeRuleEvent {
                event_id: second_event_id,
                state_revision: next_revision,
                event: AuthoritativeRuleEventKind::ActivePlayerChanged {
                    from: old_active,
                    to: new_active,
                },
            },
            AuthoritativeRuleEvent {
                event_id: third_event_id,
                state_revision: next_revision,
                event: AuthoritativeRuleEventKind::TurnPositionChanged { from, to },
            },
        ];

        let mut next = state.clone();
        next.revision = next_revision;
        next.core.turn_number = new_turn;
        next.core.active_player = new_active;
        next.core.position = to;

        build_accepted_product(state, next, events, |workspace| {
            workspace.core.position = to;
            Ok(())
        })
    }

    /// Construct a bare shell instance for crate-internal tests only.
    ///
    /// This is NOT a production constructor and carries no semantic-admission
    /// authority. `Task 8` owns admitted construction.
    pub(crate) fn new() -> Self {
        MagicRulesKernel
    }
}
