//! Program-owned kernel boundary (spec §23a.1, ADR 0055
//! `PROGRAM_OWNS_ALL_KERNEL_ENTRYPOINTS`).
//!
//! `ProgramKernelV1` is the production construction path for rules kernels: an
//! opaque public adapter struct wrapping a PRIVATE inner enum, so
//! `SyntheticLegacyRulesKernel` stays a private implementation detail of
//! `mtgml-rules` while both mandatory entry points (trusted response
//! execution and forced-progress execution) remain program-owned. There is
//! deliberately NO `Default`. A fixed state-based-actions conformance constructor is
//! available only with the non-default `magic-conformance-testkit` feature; it
//! is not a production admission route.
//!
//! Production Magic admission: `for_admitted_execution` receives the
//! semantic contract ID after the V6 catalog confirms support for the exact
//! an exact catalog-recognized semantic contract. Only that production constructor uses
//! `magic_execution_profile()`.
//! The testkit constructor uses a fixed prospective profile without any
//! SemanticContractId.

use crate::magic::MagicRulesKernel;
use crate::semantic_execution_generated::magic_execution_profile;
use crate::synthetic::{validate_synthetic_runtime_state, SyntheticLegacyRulesKernel};
use crate::turn_structure::validate_turn_structure_support;
use crate::{KernelExecutionError, RulesKernel, TransitionResult};
use mtgml_decision::DecisionResponseV2;
use mtgml_model::PlayerId;
use mtgml_model::{
    EpisodeStatus, ExecutionProgramV1, PlayerOutcome, PlayerResult, SemanticContractIdV1,
    TerminalReason,
};
use mtgml_state::EngineState;

/// Opaque public kernel adapter. Production construction flows through
/// [`ProgramKernelV1::for_program`] or
/// [`ProgramKernelV1::for_admitted_execution`]. A separate constructor is
/// available only with the non-default conformance-testkit feature.
pub struct ProgramKernelV1 {
    inner: ProgramKernelInner,
}

/// Private inner dispatch. Production Magic execution is reachable only
/// through V6 admission; fixed conformance candidates are feature-gated.
enum ProgramKernelInner {
    SyntheticLegacy(SyntheticLegacyRulesKernel),
    Magic(MagicRulesKernel),
}

/// Typed construction failure of the program-owned kernel boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgramKernelConstructionErrorV1 {
    /// The requested execution program has no production kernel contract in
    /// the current slice.
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
    /// `SyntheticLegacyRulesKernel`. `MagicRules` remains fail-closed;
    /// production Magic construction goes through
    /// `for_admitted_execution` only.
    pub fn for_program(
        program_kind: ExecutionProgramV1,
    ) -> Result<Self, ProgramKernelConstructionErrorV1> {
        match program_kind {
            ExecutionProgramV1::SyntheticRulesCompat => Ok(Self {
                inner: ProgramKernelInner::SyntheticLegacy(SyntheticLegacyRulesKernel),
            }),
            ExecutionProgramV1::MagicRules => {
                Err(ProgramKernelConstructionErrorV1::UnsupportedProgram)
            }
        }
    }

    /// Contract-aware admitted construction for Magic execution.
    ///
    /// Requires a semantic contract ID that the V6 admission layer has
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

    /// Construct the real Magic kernel implementation under the fixed state-based-actions
    /// conformance-candidate profile. This non-default testkit entry is not a
    /// production admission path and carries no `SemanticContractId`; only
    /// the conformance crate enables `magic-conformance-testkit`.
    #[cfg(feature = "magic-conformance-testkit")]
    pub fn for_state_based_actions_conformance_testkit() -> Self {
        Self {
            inner: ProgramKernelInner::Magic(
                MagicRulesKernel::state_based_actions_conformance_candidate(),
            ),
        }
    }

    /// Validate a persisted SBA plan using the fixed conformance candidate.
    /// This is read-only testkit access, not production restore admission.
    #[cfg(feature = "magic-conformance-testkit")]
    pub fn validate_state_based_actions_conformance_continuation(
        &self,
        state: &EngineState,
    ) -> Result<(), crate::SbaContinuationValidationError> {
        match &self.inner {
            ProgramKernelInner::Magic(kernel) => {
                kernel.validate_state_based_actions_conformance_continuation(state)
            }
            ProgramKernelInner::SyntheticLegacy(_) => {
                Err(crate::SbaContinuationValidationError::NotStateBasedActionsConformanceCandidate)
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
/// For MagicRules: legacy program-only validation remains the turn-structure-only
/// validator. Production capability-profile restore admission uses
/// `validate_runtime_state_for_contract` so the content-addressed identity
/// selects one exact profile.
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

/// Contract-aware restore admission. The old program-only validator remains
/// frozen as the turn-structure entry point; broader runtime admission is
/// selected only by its exact generated SemanticContractId and capability closure.
pub fn validate_runtime_state_for_contract(
    program_kind: ExecutionProgramV1,
    semantic_contract_id: SemanticContractIdV1,
    state: &EngineState,
    status: &EpisodeStatus,
) -> Result<(), KernelExecutionError> {
    match program_kind {
        ExecutionProgramV1::SyntheticRulesCompat => validate_runtime_state(program_kind, state),
        ExecutionProgramV1::MagicRules => {
            let profile = magic_execution_profile(semantic_contract_id)
                .ok_or(KernelExecutionError::UnsupportedStagePath)?;
            mtgml_state::validate_engine_state(state).map_err(KernelExecutionError::BeforeState)?;
            if profile.is_turn_structure_only_profile() {
                let _ = validate_turn_structure_support(state)
                    .map_err(KernelExecutionError::TurnStructure)?;
                return Ok(());
            }
            if profile.allows_cleanup_reset_0_1_0()
                && state.core.position
                    == (mtgml_state::TurnPosition::Ending {
                        step: mtgml_state::EndingStep::Cleanup,
                    })
            {
                if !matches!(status, EpisodeStatus::Running)
                    || state.core.players.values().any(|player| player.has_lost)
                    || state.core.priority != mtgml_state::PriorityState::None
                    || state.execution.pending_decision.is_some()
                    || !state.execution.continuations.is_empty()
                    || state.combat.is_some()
                {
                    return Err(KernelExecutionError::UnsupportedStagePath);
                }
                let turn_profile = validate_turn_structure_support(state)
                    .map_err(KernelExecutionError::TurnStructure)?;
                crate::turn_structure::derive_cleanup_damage_reset_objects(
                    state,
                    turn_profile.active_player(),
                )
                .map_err(|_| KernelExecutionError::UnsupportedStagePath)?;
                let sba = crate::state_based_actions::derive_bounded_sba_round_plan(state)
                    .map_err(|_| KernelExecutionError::UnsupportedStagePath)?;
                if !sba.selected_sba_actions.is_empty() || !sba.apnap_owners.is_empty() {
                    return Err(KernelExecutionError::UnsupportedStagePath);
                }
                return Ok(());
            }
            if profile.allows_combat_damage_0_1_0()
                && matches!(
                    state.core.position,
                    mtgml_state::TurnPosition::Combat {
                        step: mtgml_state::CombatStep::CombatDamage
                            | mtgml_state::CombatStep::EndOfCombat
                    }
                )
            {
                return MagicRulesKernel::validate_combat_damage_runtime_state(state, status);
            }
            if profile.allows_combat_blockers_0_1_0() {
                return MagicRulesKernel::validate_combat_blockers_runtime_state(state, status);
            }
            if profile.allows_combat_attackers_0_1_0() {
                return MagicRulesKernel::validate_combat_runtime_state(state, status);
            }
            if matches!(
                state.core.position,
                mtgml_state::TurnPosition::Beginning {
                    step: mtgml_state::BeginningStep::Draw
                }
            ) && !profile.allows_draw_card_0_1_0()
            {
                return Err(KernelExecutionError::UnsupportedStagePath);
            }
            if profile.allows_draw_card_0_1_0() {
                return validate_draw_runtime_state(state, status);
            }
            if profile.allows_basic_priority_0_1_0() {
                return validate_basic_priority_runtime_state(state, status);
            }
            if profile.allows_state_based_actions_combat_0_1_0() {
                return validate_state_based_actions_runtime_state(state, status);
            }
            Err(KernelExecutionError::UnsupportedStagePath)
        }
    }
}

pub(crate) fn validate_draw_runtime_state(
    state: &EngineState,
    status: &EpisodeStatus,
) -> Result<(), KernelExecutionError> {
    if matches!(
        state.core.position,
        mtgml_state::TurnPosition::Beginning {
            step: mtgml_state::BeginningStep::Draw
        }
    ) {
        if state.core.turn_number < 2 {
            return Err(KernelExecutionError::UnsupportedStagePath);
        }
        if matches!(
            state.core.priority,
            mtgml_state::PriorityState::HeldBy { .. }
        ) {
            return validate_basic_priority_runtime_state(state, status);
        }
        if state.execution.pending_decision.is_some() {
            return validate_state_based_actions_runtime_state(state, status);
        }
        // This pre-action boundary is only kernel-local; accepted response
        // transactions finish forced progress before exposing a checkpoint.
        return Err(KernelExecutionError::UnsupportedStagePath);
    }
    validate_basic_priority_runtime_state(state, status)
}

fn validate_basic_priority_runtime_state(
    state: &EngineState,
    status: &EpisodeStatus,
) -> Result<(), KernelExecutionError> {
    if matches!(
        state.core.priority,
        mtgml_state::PriorityState::HeldBy { .. }
    ) {
        if !matches!(status, EpisodeStatus::Running) {
            return Err(KernelExecutionError::UnsupportedStagePath);
        }
        crate::basic_priority::validate_pass_only_state(state, true)?;
        return Ok(());
    }
    validate_state_based_actions_runtime_state(state, status)
}

fn validate_state_based_actions_runtime_state(
    state: &EngineState,
    status: &EpisodeStatus,
) -> Result<(), KernelExecutionError> {
    if matches!(
        state.core.position,
        mtgml_state::TurnPosition::Beginning {
            step: mtgml_state::BeginningStep::Untap
        }
    ) {
        if !matches!(
            status,
            EpisodeStatus::Running | EpisodeStatus::Truncated { .. }
        ) || state.core.players.values().any(|player| player.has_lost)
            || state.execution.pending_decision.is_some()
            || !state.execution.continuations.is_empty()
        {
            return Err(KernelExecutionError::UnsupportedStagePath);
        }
        let _ =
            validate_turn_structure_support(state).map_err(KernelExecutionError::TurnStructure)?;
        return Ok(());
    }
    crate::state_based_actions::validate_state_based_actions_state_profile(state, true)
        .map_err(|_| KernelExecutionError::UnsupportedStagePath)?;
    let lost: Vec<_> = state
        .core
        .players
        .iter()
        .filter_map(|(player, value)| value.has_lost.then_some(*player))
        .collect();
    match status {
        EpisodeStatus::Running | EpisodeStatus::Truncated { .. } => {
            if !lost.is_empty() {
                return Err(KernelExecutionError::UnsupportedStagePath);
            }
        }
        EpisodeStatus::Terminal { reason, players } => {
            if state.execution.pending_decision.is_some()
                || !state.execution.continuations.is_empty()
                || state.core.players.iter().any(|(player, value)| {
                    value.has_lost != (value.life <= 0)
                        || (value.has_lost && !lost.contains(player))
                })
            {
                return Err(KernelExecutionError::UnsupportedStagePath);
            }
            let expected = if lost.len() == 1 {
                let loser = lost[0];
                Some((
                    TerminalReason::RulesLoss,
                    state
                        .core
                        .players
                        .keys()
                        .copied()
                        .map(|player| PlayerOutcome {
                            player,
                            result: if player == loser {
                                PlayerResult::Loss
                            } else {
                                PlayerResult::Win
                            },
                        })
                        .collect::<Vec<_>>(),
                ))
            } else if lost.len() == 2 {
                Some((
                    TerminalReason::SimultaneousOutcome,
                    state
                        .core
                        .players
                        .keys()
                        .copied()
                        .map(|player| PlayerOutcome {
                            player,
                            result: PlayerResult::Draw,
                        })
                        .collect::<Vec<_>>(),
                ))
            } else {
                None
            }
            .ok_or(KernelExecutionError::UnsupportedStagePath)?;
            if *reason != expected.0 || *players != expected.1 {
                return Err(KernelExecutionError::UnsupportedStagePath);
            }
        }
    }
    if let Some(continuation) = state.execution.continuations.values().next() {
        if !matches!(
            continuation.payload,
            mtgml_state::ContinuationPayloadV2::MagicSbaGraveyardOrderV1 { .. }
        ) {
            return Err(KernelExecutionError::UnsupportedStagePath);
        }
        crate::state_based_actions::validate_sba_order_continuation(state)
            .map_err(|_| KernelExecutionError::UnsupportedStagePath)?;
    } else if state.execution.pending_decision.is_some()
        || !state.execution.continuations.is_empty()
    {
        return Err(KernelExecutionError::UnsupportedStagePath);
    }
    Ok(())
}
