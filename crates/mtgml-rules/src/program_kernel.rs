//! Program-owned kernel boundary (spec §23a.1, ADR 0055
//! `PROGRAM_OWNS_ALL_KERNEL_ENTRYPOINTS`).
//!
//! `ProgramKernelV1` is the ONLY construction path for any rules kernel: an
//! opaque public adapter struct wrapping a PRIVATE inner enum, so
//! `SyntheticM1RulesKernel` stays a private implementation detail of
//! `mtgml-rules` while both mandatory entry points (trusted response
//! execution and forced-progress execution) remain program-owned. There is
//! deliberately NO `Default` and no other constructor: ambient or
//! silently-selecting kernel construction is an architectural violation.
//! Pre-S1 there is no production Magic semantic contract, so
//! `ExecutionProgramV1::MagicRules` fails closed with
//! `ProgramKernelConstructionErrorV1::UnsupportedProgram`.

use crate::synthetic::{validate_synthetic_runtime_state, SyntheticM1RulesKernel};
use crate::turn_structure::validate_turn_structure_support;
use crate::{KernelExecutionError, RulesKernel, TransitionResult};
use mtgml_decision::DecisionResponseV2;
use mtgml_model::ExecutionProgramV1;
use mtgml_model::PlayerId;
use mtgml_state::EngineState;

/// Opaque public kernel adapter. Construction flows exclusively through
/// [`ProgramKernelV1::for_program`].
pub struct ProgramKernelV1 {
    inner: ProgramKernelInner,
}

/// Private inner dispatch. NO Magic variant may exist pre-S1: adding one is
/// an S1-gated architectural decision, not an implementation detail.
enum ProgramKernelInner {
    SyntheticLegacy(SyntheticM1RulesKernel),
}

/// Typed construction failure of the program-owned kernel boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgramKernelConstructionErrorV1 {
    /// The requested execution program has no production kernel contract in
    /// the current slice (e.g. `MagicRules` before S1).
    UnsupportedProgram,
}

impl std::fmt::Debug for ProgramKernelV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.inner {
            ProgramKernelInner::SyntheticLegacy(_) => {
                f.write_str("ProgramKernelV1(SyntheticLegacy)")
            }
        }
    }
}

impl ProgramKernelV1 {
    /// The single named construction path. Behavior is unchanged for the
    /// synthetic program: the wrapped kernel is the existing
    /// `SyntheticM1RulesKernel`.
    pub fn for_program(
        program_kind: ExecutionProgramV1,
    ) -> Result<Self, ProgramKernelConstructionErrorV1> {
        match program_kind {
            ExecutionProgramV1::SyntheticRulesCompat => Ok(Self {
                inner: ProgramKernelInner::SyntheticLegacy(SyntheticM1RulesKernel),
            }),
            ExecutionProgramV1::MagicRules => {
                Err(ProgramKernelConstructionErrorV1::UnsupportedProgram)
            }
        }
    }

    /// Mandatory entry point 1: trusted response execution, dispatched to
    /// the wrapped kernel.
    pub fn apply(
        &mut self,
        state: &EngineState,
        trusted_actor: PlayerId,
        response: &DecisionResponseV2,
    ) -> Result<TransitionResult, KernelExecutionError> {
        match &mut self.inner {
            ProgramKernelInner::SyntheticLegacy(kernel) => {
                kernel.apply(state, trusted_actor, response)
            }
        }
    }

    /// Mandatory entry point 2: rules-owned forced-progress execution,
    /// dispatched to the wrapped kernel's inherent primitive.
    pub fn advance_forced_progress(
        &mut self,
        state: &EngineState,
    ) -> Result<TransitionResult, KernelExecutionError> {
        match &mut self.inner {
            ProgramKernelInner::SyntheticLegacy(kernel) => kernel.advance_forced_progress(state),
        }
    }
}

/// Program-aware runtime-state validation boundary (spec §8).
///
/// This is the mtgml-rules-owned validator that sits ABOVE `EngineState` and
/// dispatches to per-program runtime-state semantics. The generic
/// `EngineState` remains free of execution-program identity; program-awareness
/// lives here, in the rules layer.
///
/// Pre-S1:
/// - `SyntheticRulesCompat` → reuses the existing synthetic runtime-state
///   validation semantics (never weakened).
/// - `MagicRules` → state admission only: generic EngineState validation
///   followed by S1 profile validation. Execution remains unsupported.
pub fn validate_runtime_state(
    program_kind: ExecutionProgramV1,
    state: &EngineState,
) -> Result<(), KernelExecutionError> {
    match program_kind {
        ExecutionProgramV1::SyntheticRulesCompat => validate_synthetic_runtime_state(state),
        ExecutionProgramV1::MagicRules => {
            mtgml_state::validate_engine_state(state).map_err(KernelExecutionError::BeforeState)?;
            let _ = validate_turn_structure_support(state)
                .map_err(KernelExecutionError::TurnStructure)?;
            Ok(())
        }
    }
}
