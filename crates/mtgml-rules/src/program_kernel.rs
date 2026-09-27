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
//! Successor Magic admission: `for_executable_profile` receives the verified
//! Phase-9 content/profile admission and constructs the one executable bounded
//! Magic kernel. Historical semantic-ID constructors are test/conformance-only.
//! The testkit constructor uses a fixed prospective profile without any
//! SemanticContractId.

use crate::magic::MagicRulesKernel;
use crate::semantic_execution_generated::magic_execution_profile;
use crate::synthetic::{validate_synthetic_runtime_state, SyntheticLegacyRulesKernel};
use crate::turn_structure::validate_turn_structure_support;
#[cfg(any(test, feature = "historical-runtime-testkit"))]
use crate::PredecessorTransitionResult;
use crate::{KernelExecutionError, RulesKernel, TransitionResult};
use mtgml_card_ir::ExecutableProfileAdmissionV1;
use mtgml_decision::DecisionResponseV2;
use mtgml_model::PlayerId;
use mtgml_model::{
    EpisodeStatus, ExecutionProgramV1, PlayerOutcome, PlayerResult, SemanticContractIdV1,
    TerminalReason,
};
use mtgml_state::EngineState;

/// Opaque public kernel adapter. Production construction flows through
/// [`ProgramKernelV1::for_executable_profile`]. Predecessor constructors are
/// available only with the explicit historical testkit feature.
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
    /// The immutable admission token does not describe the closed executable
    /// basic-land profile required by this constructor.
    InvalidExecutableAdmission,
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
    /// The exact immutable content/semantic admission used by the executable
    /// Magic transition path. Synthetic and historical kernels have none.
    pub fn executable_profile_admission(&self) -> Option<&ExecutableProfileAdmissionV1> {
        match &self.inner {
            ProgramKernelInner::Magic(kernel) => kernel.executable_admission(),
            ProgramKernelInner::SyntheticLegacy(_) => None,
        }
    }

    /// Historical predecessor construction path. It is unavailable in the
    /// default successor runtime and exists only for archived conformance tools.
    #[cfg(any(test, feature = "historical-runtime-testkit"))]
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

    /// Historical contract-aware construction for predecessor Magic execution.
    ///
    /// Requires a semantic contract ID that the V6 admission layer has
    /// already confirmed is the exact supported contract via
    /// `catalog.supported(id, MagicRules) == true`. Program kind alone is
    /// never sufficient to construct Magic runtime.
    ///
    /// Admission is validated through `magic_execution_profile()`: only
    /// the exact supported contract ID maps to a profile. Any other
    /// ID returns `None` and is rejected with `UnsupportedProgram`.
    #[cfg(any(test, feature = "historical-runtime-testkit"))]
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

    /// Construct the bounded executable Magic kernel from the Phase-9
    /// identity/provenance/requirement admission. Unlike the historical
    /// semantic-ID constructor, this path retains the exact verified catalog
    /// that was included in admission.
    pub fn for_executable_profile(
        admission: ExecutableProfileAdmissionV1,
    ) -> Result<Self, ProgramKernelConstructionErrorV1> {
        use mtgml_model::ExecutionProgramV1;

        let identity = admission.execution_identity();
        if identity.program_kind != ExecutionProgramV1::MagicRules
            || identity.semantic_contract_id != *admission.semantic_contract_id()
            || admission.content_contract_id() != admission.verified_catalog().content_contract_id()
        {
            return Err(ProgramKernelConstructionErrorV1::InvalidExecutableAdmission);
        }
        let roots = admission
            .direct_requirement_roots()
            .iter()
            .map(|requirement| requirement.key.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        if roots
            != [
                "rules/basic-land-mana",
                "rules/land-play",
                "rules/mana-pool",
            ]
            .into_iter()
            .collect()
        {
            return Err(ProgramKernelConstructionErrorV1::InvalidExecutableAdmission);
        }
        Ok(Self {
            inner: ProgramKernelInner::Magic(MagicRulesKernel::from_executable_admission(
                admission,
            )),
        })
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
    #[cfg(any(test, feature = "historical-runtime-testkit"))]
    pub fn apply_predecessor(
        &mut self,
        state: &EngineState,
        trusted_actor: PlayerId,
        response: &DecisionResponseV2,
    ) -> Result<PredecessorTransitionResult, KernelExecutionError> {
        match &mut self.inner {
            ProgramKernelInner::SyntheticLegacy(kernel) => {
                kernel.apply_legacy(state, trusted_actor, response)
            }
            ProgramKernelInner::Magic(kernel) => {
                if kernel.is_successor_profile() {
                    return Err(KernelExecutionError::UnsupportedPlayerResponse);
                }
                kernel.apply_legacy(state, trusted_actor, response)
            }
        }
    }

    /// Return the current bounded Magic candidate surface from complete
    /// successor state. Only an immutable Phase-9 executable admission can
    /// reach the Basic-Land producer; the synthetic and historical
    /// semantic-ID kernels fail closed.
    pub fn successor_candidates(
        &self,
        state: &mtgml_state::EngineStatePartsV2,
        actor: PlayerId,
        status: &EpisodeStatus,
    ) -> Result<Vec<mtgml_decision::AuthoritativeCandidateV3>, crate::BasicLandCandidateError> {
        match &self.inner {
            ProgramKernelInner::Magic(kernel) => {
                kernel.derive_basic_land_candidates(state, actor, status)
            }
            ProgramKernelInner::SyntheticLegacy(_) => {
                Err(crate::BasicLandCandidateError::WrongExecutionIdentity)
            }
        }
    }

    /// Install a rules-derived successor request into a revisioned transition
    /// workspace. The surrounding RulesKernel product owns revision advance,
    /// event creation, and atomic commit.
    pub fn install_successor_request(
        &self,
        state: &mut mtgml_state::EngineStatePartsV2,
        actor: PlayerId,
        status: &EpisodeStatus,
    ) -> Result<mtgml_decision::AuthoritativeDecisionRequestV3, crate::BasicLandCandidateError>
    {
        match &self.inner {
            ProgramKernelInner::Magic(kernel) => {
                kernel.install_basic_land_request(state, actor, status)
            }
            ProgramKernelInner::SyntheticLegacy(_) => {
                Err(crate::BasicLandCandidateError::WrongExecutionIdentity)
            }
        }
    }

    /// Resolve a response to the exact trusted PlayLand or mana-ability
    /// binding stored in successor state. This is still read-only; execution
    /// and product construction remain inside the kernel transition.
    pub fn selected_successor_action(
        &self,
        state: &mtgml_state::EngineStatePartsV2,
        actor: PlayerId,
        response: &DecisionResponseV2,
        status: &EpisodeStatus,
    ) -> Result<crate::MagicActionRequestV1, crate::BasicLandCandidateError> {
        match &self.inner {
            ProgramKernelInner::Magic(kernel) => {
                kernel.selected_basic_land_action(state, actor, response, status)
            }
            ProgramKernelInner::SyntheticLegacy(_) => {
                Err(crate::BasicLandCandidateError::WrongExecutionIdentity)
            }
        }
    }

    /// Execute one exact response against the complete successor state using
    /// the immutable Phase-9 admission held by this program kernel.
    fn apply_admitted_successor(
        &self,
        state: &mtgml_state::EngineStatePartsV2,
        actor: PlayerId,
        response: &DecisionResponseV2,
        status: &EpisodeStatus,
    ) -> Result<TransitionResult, KernelExecutionError> {
        match &self.inner {
            ProgramKernelInner::Magic(kernel) => {
                let product = kernel.execute_basic_land_response(state, actor, response, status)?;
                Ok(TransitionResult {
                    accepted: product.accepted,
                    next_state: product.next_state,
                    delta: product.delta,
                    events: product.events,
                    next_decision: product.next_decision,
                    status: product.status,
                })
            }
            ProgramKernelInner::SyntheticLegacy(_) => {
                Err(KernelExecutionError::UnsupportedPlayerResponse)
            }
        }
    }

    /// Mandatory entry point 2: rules-owned forced-progress execution,
    /// dispatched to the wrapped kernel's inherent primitive.
    #[cfg(any(test, feature = "historical-runtime-testkit"))]
    pub fn advance_forced_progress(
        &mut self,
        state: &EngineState,
    ) -> Result<PredecessorTransitionResult, KernelExecutionError> {
        match &mut self.inner {
            ProgramKernelInner::SyntheticLegacy(kernel) => kernel.advance_forced_progress(state),
            ProgramKernelInner::Magic(kernel) => kernel.advance_forced_progress(state),
        }
    }

    /// Contract-aware admission for a forced product that closes an accepted
    /// player response. Magic uses this seam to preserve historical profile
    /// behavior while authorizing the cumulative bounded-turn continuation.
    #[cfg(any(test, feature = "historical-runtime-testkit"))]
    pub fn authorize_response_progress(
        &self,
        before: &EngineState,
        result: &PredecessorTransitionResult,
    ) -> Result<(), crate::TransitionViolation> {
        match &self.inner {
            ProgramKernelInner::SyntheticLegacy(_) => Ok(()),
            ProgramKernelInner::Magic(kernel) => kernel.authorize_response_progress(before, result),
        }
    }
}

impl RulesKernel for ProgramKernelV1 {
    fn apply(
        &mut self,
        state: &mtgml_state::EngineStatePartsV2,
        trusted_actor: PlayerId,
        response: &DecisionResponseV2,
        status: &EpisodeStatus,
    ) -> Result<TransitionResult, KernelExecutionError> {
        ProgramKernelV1::apply_admitted_successor(self, state, trusted_actor, response, status)
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
