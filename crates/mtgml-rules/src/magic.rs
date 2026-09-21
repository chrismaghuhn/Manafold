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
use mtgml_model::PlayerId;
use mtgml_state::{validate_engine_state, EngineState};

use crate::errors::KernelExecutionError;
use crate::transition::{RulesKernel, TransitionResult};
use crate::turn_structure::{unsupported_rules_boundary, validate_turn_structure_support};

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
    /// temporal position. Positions whose mandatory downstream work requires an
    /// unsupported capability return
    /// [`KernelExecutionError::UnsupportedRulesBoundary`]. Positions whose own
    /// semantic operation (ordinary untap, quiescent Cleanup) is not yet
    /// implemented return
    /// [`KernelExecutionError::UnsupportedStagePath`]. No state mutation
    /// occurs on any path.
    pub(crate) fn advance_forced_progress(
        &mut self,
        state: &EngineState,
    ) -> Result<TransitionResult, KernelExecutionError> {
        validate_engine_state(state).map_err(KernelExecutionError::BeforeState)?;
        let profile =
            validate_turn_structure_support(state).map_err(KernelExecutionError::TurnStructure)?;
        let position = profile.position();
        match unsupported_rules_boundary(position) {
            Some(boundary) => Err(KernelExecutionError::UnsupportedRulesBoundary(boundary)),
            None => Err(KernelExecutionError::UnsupportedStagePath),
        }
    }

    /// Construct a bare shell instance for crate-internal tests only.
    ///
    /// This is NOT a production constructor and carries no semantic-admission
    /// authority. `Task 8` owns admitted construction.
    pub(crate) fn new() -> Self {
        MagicRulesKernel
    }
}
