//! Durable `MagicRulesKernel` owner.
//!
//! This module establishes the milestone-free, future-authoritative owner of
//! Magic execution inside `mtgml-rules`. The kernel is structurally present and
//! reachable through `ProgramKernelV1::for_admitted_execution` once the V5
//! admission layer has confirmed `catalog.supported(semantic_contract_id,
//! MagicRules) == true` for the exact `rules/turn-structure@0.1.0` contract.
//!
//! The execution profile is provided by the semantic execution catalog
//! (`semantic_execution_generated`) and is validated via `magic_execution_profile()`.
//! Its capability predicates are exact for the admitted contract: an old
//! checkpoint admitted under Contract A cannot execute Contract B behavior
//! merely because the same `MagicRulesKernel` type later gains more capabilities.

use mtgml_decision::DecisionResponseV2;
use mtgml_model::{PlayerId, RuleEventId, StateRevision};
use mtgml_state::{validate_engine_state, BeginningStep, EndingStep, EngineState, TurnPosition};

use crate::errors::KernelExecutionError;
use crate::events::{AuthoritativeRuleEvent, AuthoritativeRuleEventKind};
use crate::product::build_accepted_product;
use crate::semantic_execution_generated::MagicExecutionProfile;
#[cfg(test)]
use crate::semantic_execution_generated::{
    magic_turn_structure_0_1_0_semantic_contract_id, test_only_magic_execution_profile,
};
use crate::transition::{RulesKernel, TransitionResult};
use crate::turn_structure::{
    derive_ordinary_untap_affected_objects, temporal_successor, unsupported_rules_boundary,
    validate_quiescent_cleanup_boundary, validate_turn_structure_support, TurnStructureError,
    TurnStructureSupportProfile, UnsupportedRulesBoundary,
};

/// Durable, milestone-free owner of Magic execution.
///
/// Reachable only through `ProgramKernelV1::for_admitted_execution`
/// with a contract ID that the V5 admission layer has confirmed is the
/// exact supported semantic contract.
pub(crate) struct MagicRulesKernel {
    profile: MagicExecutionProfile,
}

impl MagicRulesKernel {
    /// Construct from an admitted execution profile.
    ///
    /// This is the production admission authority for Magic execution.
    /// The profile MUST carry the exact supported semantic contract ID;
    /// the V5 admission layer guarantees this before construction.
    pub(crate) fn from_admitted_profile(profile: MagicExecutionProfile) -> Self {
        Self { profile }
    }

    /// Construct a bare shell instance for crate-internal tests only.
    ///
    /// This is NOT a production constructor and carries no semantic-admission
    /// authority. V5 admission constructs the production kernel.
    #[cfg(test)]
    pub(crate) fn new() -> Self {
        Self {
            profile: test_only_magic_execution_profile(
                magic_turn_structure_0_1_0_semantic_contract_id(),
                true,
            ),
        }
    }
}

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

impl MagicRulesKernel {
    /// Rules-owned forced-progress shell.
    ///
    /// The profile's `turn_structure_0_1_0` capability flag is checked
    /// at the entry point as a real capability gate before any S1
    /// semantics execute. The catalog admission layer has already
    /// confirmed that the profile's admitted contract is the exact
    /// supported semantic contract; the capability flag confirms the
    /// contract carries the turn-structure capability.
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
        if !self.profile.allows_turn_structure_0_1_0() {
            return Err(KernelExecutionError::UnsupportedStagePath);
        }

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
}
