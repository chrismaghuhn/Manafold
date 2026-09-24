//! Durable `MagicRulesKernel` owner.
//!
//! This module establishes the milestone-free, future-authoritative owner of
//! Magic execution inside `mtgml-rules`. Production execution is reachable
//! through `ProgramKernelV1::for_admitted_execution` once V5 admission confirms
//! the exact `rules/turn-structure@0.1.0` contract. A separate fixed S3.A
//! conformance-candidate profile exists only behind the non-default
//! `m3-conformance-testkit` feature and carries no production identity.
//!
//! The execution profile is provided by the semantic execution catalog
//! (`semantic_execution_generated`) and is validated via `magic_execution_profile()`.
//! Its capability predicates are exact for the admitted contract: an old
//! checkpoint admitted under Contract A cannot execute Contract B behavior
//! merely because the same `MagicRulesKernel` type later gains more capabilities.

use std::collections::BTreeMap;

use mtgml_decision::DecisionResponseV2;
use mtgml_model::{PlayerId, RuleEventId, StateRevision, VisibleSequence};
use mtgml_state::{
    validate_engine_state, BeginningStep, EndingStep, EngineState, PerspectiveLifecycleAuditV1,
    PerspectiveLifecycleMutationV1, TurnPosition,
};

use crate::errors::KernelExecutionError;
use crate::events::{
    AuthoritativeRuleEvent, AuthoritativeRuleEventKind, PerspectiveObservationPolicyV1,
};
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
/// Production construction is reachable only through
/// `ProgramKernelV1::for_admitted_execution` with the exact V5-admitted
/// semantic contract. The non-default conformance testkit has a separate
/// fixed constructor that does not participate in production admission.
pub(crate) struct MagicRulesKernel {
    profile: MagicKernelProfile,
}

/// The kernel's execution context is not itself a semantic contract. The
/// admitted production profile remains the frozen S1 contract; S3.A RED/GREEN
/// execution uses one fixed, non-production candidate profile behind the
/// non-default conformance-testkit feature.
enum MagicKernelProfile {
    AdmittedS1(MagicExecutionProfile),
    #[cfg(test)]
    UnitTest(MagicExecutionProfile),
    #[cfg(feature = "m3-conformance-testkit")]
    S3AConformanceCandidate,
}

impl MagicKernelProfile {
    fn allows_s1_turn_structure(&self) -> bool {
        match self {
            Self::AdmittedS1(profile) => profile.allows_turn_structure_0_1_0(),
            #[cfg(test)]
            Self::UnitTest(profile) => profile.allows_turn_structure_0_1_0(),
            #[cfg(feature = "m3-conformance-testkit")]
            Self::S3AConformanceCandidate => true,
        }
    }
}

impl MagicRulesKernel {
    /// Construct from an admitted execution profile.
    ///
    /// This is the production admission authority for Magic execution.
    /// The profile MUST carry the exact supported semantic contract ID;
    /// the V5 admission layer guarantees this before construction.
    pub(crate) fn from_admitted_profile(profile: MagicExecutionProfile) -> Self {
        Self {
            profile: MagicKernelProfile::AdmittedS1(profile),
        }
    }

    /// Construct the single prospective S3.A candidate profile for isolated
    /// conformance. It carries no SemanticContractId and cannot be admitted
    /// from a production checkpoint or replay.
    #[cfg(feature = "m3-conformance-testkit")]
    pub(crate) fn s3_a_conformance_candidate() -> Self {
        Self {
            profile: MagicKernelProfile::S3AConformanceCandidate,
        }
    }

    /// Construct a bare shell instance for crate-internal tests only.
    ///
    /// This is NOT a production constructor and carries no semantic-admission
    /// authority. V5 admission constructs the production kernel.
    #[cfg(test)]
    pub(crate) fn new() -> Self {
        Self {
            profile: MagicKernelProfile::UnitTest(test_only_magic_execution_profile(
                magic_turn_structure_0_1_0_semantic_contract_id(),
                true,
            )),
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
    /// The profile gate checks both the turn-structure capability and the
    /// exact admitted semantic identity before any S1 semantics execute.
    /// This prevents a capability bit from authorizing a different contract.
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
        if !self.profile.allows_s1_turn_structure() {
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

        let perspective_count = u64::try_from(state.knowledge.players.len())
            .map_err(|_| KernelExecutionError::RuleEventIdOverflow)?;
        let affected_count = u64::try_from(affected.len())
            .map_err(|_| KernelExecutionError::VisibleSequenceOverflow)?;
        let occurrence_count = perspective_count
            .checked_mul(affected_count)
            .ok_or(KernelExecutionError::RuleEventIdOverflow)?;
        let total_events = 2u64
            .checked_add(occurrence_count)
            .ok_or(KernelExecutionError::RuleEventIdOverflow)?;

        let first_event_id = state.allocators.next_rule_event_id.0;
        let last_event_id = first_event_id
            .checked_add(total_events)
            .ok_or(KernelExecutionError::RuleEventIdOverflow)?
            - 1;
        let _ = last_event_id;

        for knowledge in state.knowledge.players.values() {
            knowledge
                .next_visible_sequence
                .0
                .checked_add(affected_count)
                .ok_or(KernelExecutionError::VisibleSequenceOverflow)?;
        }

        let mut events: Vec<AuthoritativeRuleEvent> =
            Vec::with_capacity(2 + affected.len() * state.knowledge.players.len());

        events.push(AuthoritativeRuleEvent {
            event_id: state.allocators.next_rule_event_id,
            state_revision: next_revision,
            event: AuthoritativeRuleEventKind::UntapCompleted {
                affected_objects: affected.clone(),
            },
        });

        let mut occurrence_event_id = state.allocators.next_rule_event_id.0 + 1;
        let mut occurrence_sequence: BTreeMap<PlayerId, u64> = BTreeMap::new();
        for (player, knowledge) in &state.knowledge.players {
            occurrence_sequence.insert(*player, knowledge.next_visible_sequence.0);
        }

        for player_id in state.knowledge.players.keys().copied().collect::<Vec<_>>() {
            for object_id in &affected {
                let sequence = VisibleSequence(*occurrence_sequence.get(&player_id).unwrap());
                events.push(AuthoritativeRuleEvent {
                    event_id: RuleEventId(occurrence_event_id),
                    state_revision: next_revision,
                    event: AuthoritativeRuleEventKind::PerspectiveOccurrence {
                        lifecycle: PerspectiveLifecycleAuditV1 {
                            perspective: player_id,
                            sequence,
                            mutation: PerspectiveLifecycleMutationV1::default(),
                        },
                        observation: PerspectiveObservationPolicyV1::ObjectTapped {
                            object: *object_id,
                            tapped: false,
                        },
                    },
                });
                occurrence_event_id += 1;
                *occurrence_sequence.get_mut(&player_id).unwrap() += 1;
            }
        }

        let turn_event_id = RuleEventId(occurrence_event_id);
        events.push(AuthoritativeRuleEvent {
            event_id: turn_event_id,
            state_revision: next_revision,
            event: AuthoritativeRuleEventKind::TurnPositionChanged { from, to },
        });

        let mut next = state.clone();
        next.revision = next_revision;

        build_accepted_product(state, next, events, |workspace| {
            workspace.core.position = to;
            for object_id in &affected {
                if let Some(object) = workspace.zones.objects.get_mut(object_id) {
                    object.tapped = false;
                }
            }
            let advance = u64::try_from(affected.len())
                .map_err(|_| KernelExecutionError::VisibleSequenceOverflow)?;
            for knowledge in workspace.knowledge.players.values_mut() {
                knowledge.next_visible_sequence =
                    VisibleSequence(knowledge.next_visible_sequence.0 + advance);
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
