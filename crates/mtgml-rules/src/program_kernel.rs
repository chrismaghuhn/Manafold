//! Program-owned kernel boundary (spec §23a.1, ADR 0055
//! `PROGRAM_OWNS_ALL_KERNEL_ENTRYPOINTS`).
//!
//! `ProgramKernelV1` is the production construction path for rules kernels: an
//! opaque public adapter struct wrapping a PRIVATE inner enum, so
//! `SyntheticM1RulesKernel` stays a private implementation detail of
//! `mtgml-rules` while both mandatory entry points (trusted response
//! execution and forced-progress execution) remain program-owned. There is
//! deliberately NO `Default`. A fixed S3.A conformance constructor is
//! available only with the non-default `m3-conformance-testkit` feature; it
//! is not a production admission route.
//!
//! Production Magic admission: `for_admitted_execution` receives the
//! semantic contract ID after the V5 catalog confirms support for the exact
//! contract. Only that production constructor uses `magic_execution_profile()`.
//! The testkit constructor uses a fixed prospective profile without any
//! SemanticContractId.

use crate::magic::MagicRulesKernel;
use crate::semantic_execution_generated::magic_execution_profile;
use crate::synthetic::{validate_synthetic_runtime_state, SyntheticM1RulesKernel};
use crate::turn_structure::validate_turn_structure_support;
use crate::{KernelExecutionError, RulesKernel, TransitionResult};
use mtgml_decision::DecisionResponseV2;
use mtgml_model::PlayerId;
use mtgml_model::{ExecutionProgramV1, SemanticContractIdV1};
use mtgml_state::EngineState;

/// Opaque public kernel adapter. Production construction flows through
/// [`ProgramKernelV1::for_program`] or
/// [`ProgramKernelV1::for_admitted_execution`]. A separate constructor is
/// available only with the non-default conformance-testkit feature.
pub struct ProgramKernelV1 {
    inner: ProgramKernelInner,
}

/// Private inner dispatch. Production Magic execution is reachable only
/// through V5 admission; the fixed conformance candidate is feature-gated.
enum ProgramKernelInner {
    SyntheticLegacy(SyntheticM1RulesKernel),
    Magic(MagicRulesKernel),
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
            ProgramKernelInner::Magic(_) => f.write_str("ProgramKernelV1(Magic)"),
        }
    }
}

impl ProgramKernelV1 {
    /// The single named construction path. Behavior is unchanged for the
    /// synthetic program: the wrapped kernel is the existing
    /// `SyntheticM1RulesKernel`. `MagicRules` remains fail-closed;
    /// production Magic construction goes through
    /// `for_admitted_execution` only.
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

    /// Contract-aware admitted construction for Magic execution.
    ///
    /// Requires a semantic contract ID that the V5 admission layer has
    /// already confirmed is the exact supported contract via
    /// `catalog.supported(id, MagicRules) == true`. Program kind alone is
    /// never sufficient to construct Magic runtime.
    ///
    /// Admission is validated through `magic_execution_profile()`: only
    /// the exact supported contract ID maps to a profile. Any other
    /// ID returns `None` and is rejected with `UnsupportedProgram`.
    pub fn for_admitted_execution(
        program_kind: ExecutionProgramV1,
        semantic_contract_id: SemanticContractIdV1,
    ) -> Result<Self, ProgramKernelConstructionErrorV1> {
        match program_kind {
            ExecutionProgramV1::MagicRules => {
                let profile = magic_execution_profile(semantic_contract_id)
                    .ok_or(ProgramKernelConstructionErrorV1::UnsupportedProgram)?;
                Ok(Self {
                    inner: ProgramKernelInner::Magic(MagicRulesKernel::from_admitted_profile(
                        profile,
                    )),
                })
            }
            ExecutionProgramV1::SyntheticRulesCompat => {
                Err(ProgramKernelConstructionErrorV1::UnsupportedProgram)
            }
        }
    }

    /// Construct the real Magic kernel implementation under the fixed S3.A
    /// conformance-candidate profile. This non-default testkit entry is not a
    /// production admission path and carries no `SemanticContractId`; only
    /// the conformance crate enables `m3-conformance-testkit`.
    #[cfg(feature = "m3-conformance-testkit")]
    pub fn for_s3_a_conformance_testkit() -> Self {
        Self {
            inner: ProgramKernelInner::Magic(MagicRulesKernel::s3_a_conformance_candidate()),
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
            ProgramKernelInner::Magic(kernel) => kernel.apply(state, trusted_actor, response),
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
            ProgramKernelInner::Magic(kernel) => kernel.advance_forced_progress(state),
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
/// For MagicRules: state admission only — generic EngineState validation
/// followed by S1 profile validation. Kernel execution of S1 semantics
/// requires V6 admission via `ProgramKernelV1::for_admitted_execution`.
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
