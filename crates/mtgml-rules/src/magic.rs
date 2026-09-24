//! Durable `MagicRulesKernel` owner.
//!
//! This module establishes the milestone-free, future-authoritative owner of
//! Magic execution inside `mtgml-rules`. Production execution is reachable
//! through `ProgramKernelV1::for_admitted_execution` once V6 admission confirms
//! the exact S1, S3.A, or S3.B contract. Separate fixed S3.A/S3.B
//! conformance-candidate profile exists only behind the non-default
//! `m3-conformance-testkit` feature and carries no production identity.
//!
//! The execution profile is provided by the semantic execution catalog
//! (`semantic_execution_generated`) and is validated via `magic_execution_profile()`.
//! Its capability predicates are exact for the admitted contract: an S1
//! checkpoint cannot execute S3.A behavior
//! merely because the same `MagicRulesKernel` type later gains more capabilities.

use std::collections::BTreeMap;

use mtgml_decision::{
    AuthoritativeCandidateV2, CandidateIntent, CandidateOrderingV1, DecisionAnswerV2,
    DecisionDomainV2, DecisionResponseV2, DecisionVisibility, EngineCandidateBinding,
};
use mtgml_model::{
    CandidateIdV1, DecisionId, EpisodeStatus, PlayerId, PlayerOutcome, PlayerResult, RuleEventId,
    StateRevision, TerminalReason, VisibleSequence, ZoneKind,
};
use mtgml_state::{
    validate_engine_state, BeginningStep, EndingStep, EngineState, PerspectiveLifecycleAuditV1,
    PerspectiveLifecycleMutationV1, SbaSelectedActionV1, TurnPosition, ZonePosition,
};
use mtgml_state::{
    ContinuationPayloadV2, ContinuationRecordV2, PendingDecisionRecordV2, SbaGraveyardOwnerOrderV1,
    VisibilityPartition, ZoneLocation,
};

use crate::errors::KernelExecutionError;
use crate::events::{
    AuthoritativeRuleEvent, AuthoritativeRuleEventKind, PerspectiveObservationPolicyV1,
};
use crate::product::{build_accepted_product, build_accepted_product_with_status};
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
/// `ProgramKernelV1::for_admitted_execution` with the exact V6-admitted
/// semantic contract. The non-default conformance testkit has a separate
/// fixed constructor that does not participate in production admission.
pub(crate) struct MagicRulesKernel {
    profile: MagicKernelProfile,
}

/// The kernel's execution context is not itself a semantic contract. The
/// admitted S1, S3.A, and S3.B production profiles are selected only by their
/// exact V6 semantic identities. S3.A/S3.B candidate profiles carry no
/// production identity and exist only in test/conformance builds.
enum MagicKernelProfile {
    AdmittedS1(MagicExecutionProfile),
    AdmittedS3A(MagicExecutionProfile),
    AdmittedS3B(MagicExecutionProfile),
    #[cfg(test)]
    UnitTest(MagicExecutionProfile),
    #[cfg(any(test, feature = "m3-conformance-testkit"))]
    S3AConformanceCandidate,
    #[cfg(test)]
    S3BConformanceCandidate,
}

impl MagicKernelProfile {
    fn allows_s1_turn_structure(&self) -> bool {
        match self {
            Self::AdmittedS1(profile) => profile.allows_turn_structure_0_1_0(),
            Self::AdmittedS3A(_) => false,
            Self::AdmittedS3B(_) => false,
            #[cfg(test)]
            Self::UnitTest(profile) => profile.allows_turn_structure_0_1_0(),
            #[cfg(any(test, feature = "m3-conformance-testkit"))]
            Self::S3AConformanceCandidate => false,
            #[cfg(test)]
            Self::S3BConformanceCandidate => false,
        }
    }

    fn allows_s3_a(&self) -> bool {
        match self {
            Self::AdmittedS1(_) => false,
            Self::AdmittedS3A(profile) | Self::AdmittedS3B(profile) => {
                profile.allows_state_based_actions_combat_0_1_0()
            }
            #[cfg(test)]
            Self::UnitTest(_) => false,
            #[cfg(any(test, feature = "m3-conformance-testkit"))]
            Self::S3AConformanceCandidate => true,
            #[cfg(test)]
            Self::S3BConformanceCandidate => true,
        }
    }

    fn allows_s3_b(&self) -> bool {
        match self {
            Self::AdmittedS3B(profile) => profile.allows_basic_priority_0_1_0(),
            #[cfg(test)]
            Self::S3BConformanceCandidate => true,
            _ => false,
        }
    }
}

impl MagicRulesKernel {
    /// Construct from an admitted execution profile.
    ///
    /// This is the production admission authority for Magic execution.
    /// The profile MUST carry the exact supported semantic contract ID;
    /// the V6 admission layer guarantees this before construction.
    pub(crate) fn from_admitted_profile(profile: MagicExecutionProfile) -> Self {
        let profile = if profile.allows_basic_priority_0_1_0() {
            MagicKernelProfile::AdmittedS3B(profile)
        } else if profile.allows_state_based_actions_combat_0_1_0() {
            MagicKernelProfile::AdmittedS3A(profile)
        } else {
            MagicKernelProfile::AdmittedS1(profile)
        };
        Self { profile }
    }

    /// Construct the single prospective S3.A candidate profile for isolated
    /// conformance. It carries no SemanticContractId and cannot be admitted
    /// from a production checkpoint or replay.
    #[cfg(any(test, feature = "m3-conformance-testkit"))]
    pub(crate) fn s3_a_conformance_candidate() -> Self {
        Self {
            profile: MagicKernelProfile::S3AConformanceCandidate,
        }
    }

    #[cfg(test)]
    pub(crate) fn s3_b_conformance_candidate() -> Self {
        Self {
            profile: MagicKernelProfile::S3BConformanceCandidate,
        }
    }

    /// Read-only conformance hook for Task 6 continuation semantic evidence.
    #[cfg(feature = "m3-conformance-testkit")]
    pub(crate) fn validate_s3_a_conformance_continuation(
        &self,
        state: &EngineState,
    ) -> Result<(), crate::SbaContinuationValidationError> {
        if !matches!(self.profile, MagicKernelProfile::S3AConformanceCandidate) {
            return Err(crate::SbaContinuationValidationError::NotS3AConformanceCandidate);
        }
        crate::state_based_actions::validate_sba_order_continuation(state)
    }

    /// Construct a bare shell instance for crate-internal tests only.
    ///
    /// This is NOT a production constructor and carries no semantic-admission
    /// authority. V6 admission constructs the production kernel.
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
    /// Admitted S1 has no player Decision surface and rejects every response
    /// without inspecting or mutating it. The fixed non-production S3.A
    /// candidate accepts only a nonfinal SBA Order stage; its final Order
    /// remains unaccepted until Task 9 can apply the complete round atomically.
    fn apply(
        &mut self,
        state: &EngineState,
        trusted_actor: PlayerId,
        response: &DecisionResponseV2,
    ) -> Result<TransitionResult, KernelExecutionError> {
        if self.profile.allows_s3_a() {
            if state
                .execution
                .pending_decision
                .as_ref()
                .is_some_and(|pending| {
                    matches!(pending.request.decision, DecisionDomainV2::Order { .. })
                })
            {
                return self.apply_s3_a_order_response(state, trusted_actor, response);
            }
            if self.profile.allows_s3_b() {
                return self.apply_priority_response(state, trusted_actor, response);
            }
        }
        let _ = (state, trusted_actor, response);
        Err(KernelExecutionError::UnsupportedPlayerResponse)
    }
}

impl MagicRulesKernel {
    fn advance_s3_a_round(
        &mut self,
        state: &EngineState,
    ) -> Result<TransitionResult, KernelExecutionError> {
        validate_engine_state(state).map_err(KernelExecutionError::BeforeState)?;
        if state.execution.pending_decision.is_some() || !state.execution.continuations.is_empty() {
            return Err(KernelExecutionError::UnsupportedStagePath);
        }
        if matches!(
            state.core.position,
            TurnPosition::Beginning {
                step: BeginningStep::Untap
            }
        ) {
            let profile = validate_turn_structure_support(state)
                .map_err(KernelExecutionError::TurnStructure)?;
            return self.ordinary_untap(state, &profile);
        }
        if matches!(
            state.core.position,
            TurnPosition::Ending {
                step: EndingStep::Cleanup
            }
        ) {
            let profile = validate_turn_structure_support(state)
                .map_err(KernelExecutionError::TurnStructure)?;
            return self.advance_quiescent_cleanup(state, &profile);
        }
        let plan = crate::state_based_actions::derive_bounded_sba_round_plan(state)
            .map_err(|_| KernelExecutionError::UnsupportedStagePath)?;
        if plan.selected_sba_actions.is_empty() {
            if self.profile.allows_s3_b() {
                return crate::basic_priority::open_priority_window(state);
            }
            return match unsupported_rules_boundary(state.core.position) {
                Some(boundary) => Err(KernelExecutionError::UnsupportedRulesBoundary(boundary)),
                None => Err(KernelExecutionError::UnsupportedStagePath),
            };
        }
        let Some(actor) = plan.apnap_owners.first().copied() else {
            return self.apply_s3_a_batch(
                state,
                None,
                plan.selected_sba_actions,
                self.profile.allows_s3_b(),
            );
        };

        let continuation_id = state.allocators.next_continuation_id;
        let next_continuation = continuation_id
            .0
            .checked_add(1)
            .ok_or(KernelExecutionError::Exhaustion("continuation"))?;
        let identity = crate::decision_stage::fresh_stage_identity(state, actor)?;
        let candidates = Self::s3_a_order_candidates(state, &plan.selected_sba_actions, actor)?;
        let cardinality = u32::try_from(candidates.len())
            .map_err(|_| KernelExecutionError::Exhaustion("order_cardinality"))?;
        let request = mtgml_decision::AuthoritativeDecisionRequestV2 {
            decision_id: identity.decision_id,
            player_decision_id: identity.player_decision_id,
            state_revision: identity.revision,
            actor,
            visibility: DecisionVisibility::ActingPlayerOnly,
            decision: DecisionDomainV2::Order {
                minimum: cardinality,
                maximum: cardinality,
            },
            candidates,
            continuation_id: Some(continuation_id),
        };
        let payload = ContinuationPayloadV2::MagicSbaGraveyardOrderV1 {
            round_start_revision: state.revision,
            selected_sba_actions: plan.selected_sba_actions,
            apnap_owners: plan.apnap_owners,
            next_owner_index: 0,
            completed_owner_orders: Vec::new(),
        };
        let continuation = ContinuationRecordV2 {
            id: continuation_id,
            actor,
            created_at_revision: identity.revision,
            stage_index: payload.stage_index(),
            payload,
        };
        let event = Self::s3_a_bound_event(
            state,
            0,
            identity.revision,
            AuthoritativeRuleEventKind::DecisionCreated {
                decision: identity.decision_id,
            },
        )?;

        let mut next = state.clone();
        next.revision = identity.revision;
        build_accepted_product(state, next, vec![event], |workspace| {
            workspace.execution.pending_decision = Some(PendingDecisionRecordV2 { request });
            workspace
                .execution
                .continuations
                .insert(continuation_id, continuation);
            workspace.allocators.next_decision_id = DecisionId(
                identity
                    .decision_id
                    .0
                    .checked_add(1)
                    .ok_or(KernelExecutionError::Exhaustion("decision"))?,
            );
            workspace.allocators.next_continuation_id =
                mtgml_model::ContinuationId(next_continuation);
            crate::decision_stage::advance_player_allocator(
                workspace,
                actor,
                identity.player_decision_id,
            )?;
            Ok(())
        })
    }

    fn apply_s3_a_batch(
        &mut self,
        state: &EngineState,
        final_order: Option<(mtgml_model::ContinuationId, Vec<SbaGraveyardOwnerOrderV1>)>,
        selected_sba_actions: Vec<SbaSelectedActionV1>,
        open_priority_if_stable: bool,
    ) -> Result<TransitionResult, KernelExecutionError> {
        validate_engine_state(state).map_err(KernelExecutionError::BeforeState)?;
        let plan = crate::state_based_actions::derive_bounded_sba_round_plan(state)
            .map_err(|_| KernelExecutionError::UnsupportedStagePath)?;
        if plan.selected_sba_actions != selected_sba_actions || selected_sba_actions.is_empty() {
            return Err(KernelExecutionError::UnsupportedStagePath);
        }
        match &final_order {
            Some((continuation_id, orders)) => {
                crate::state_based_actions::validate_sba_order_continuation(state)
                    .map_err(|_| KernelExecutionError::UnsupportedStagePath)?;
                let continuation = state
                    .execution
                    .continuations
                    .get(continuation_id)
                    .ok_or(KernelExecutionError::UnsupportedStagePath)?;
                let ContinuationPayloadV2::MagicSbaGraveyardOrderV1 {
                    apnap_owners,
                    next_owner_index,
                    completed_owner_orders,
                    ..
                } = &continuation.payload
                else {
                    return Err(KernelExecutionError::UnsupportedStagePath);
                };
                if orders.len() != apnap_owners.len()
                    || orders.len() != completed_owner_orders.len() + 1
                    || orders
                        .iter()
                        .zip(apnap_owners)
                        .any(|(order, expected)| order.owner != *expected)
                    || usize::try_from(*next_owner_index).ok() != Some(completed_owner_orders.len())
                {
                    return Err(KernelExecutionError::UnsupportedStagePath);
                }
                let final_owner = orders
                    .last()
                    .ok_or(KernelExecutionError::UnsupportedStagePath)?
                    .owner;
                if plan.apnap_owners != *apnap_owners
                    || apnap_owners.get(*next_owner_index as usize) != Some(&final_owner)
                {
                    return Err(KernelExecutionError::UnsupportedStagePath);
                }
            }
            None if !plan.apnap_owners.is_empty()
                || state.execution.pending_decision.is_some()
                || !state.execution.continuations.is_empty() =>
            {
                return Err(KernelExecutionError::UnsupportedStagePath)
            }
            None => {}
        }

        let revision = StateRevision(
            state
                .revision
                .0
                .checked_add(1)
                .ok_or(KernelExecutionError::RevisionOverflow)?,
        );
        let mut events = Vec::new();
        if let Some((continuation_id, orders)) = &final_order {
            let pending = state
                .execution
                .pending_decision
                .as_ref()
                .ok_or(KernelExecutionError::UnsupportedStagePath)?;
            let final_order = orders
                .last()
                .ok_or(KernelExecutionError::UnsupportedStagePath)?;
            events.push(Self::s3_a_bound_event(
                state,
                events.len() as u64,
                revision,
                AuthoritativeRuleEventKind::DecisionCleared {
                    decision: pending.request.decision_id,
                },
            )?);
            events.push(Self::s3_a_bound_event(
                state,
                events.len() as u64,
                revision,
                AuthoritativeRuleEventKind::SbaGraveyardOrderChosen {
                    continuation: *continuation_id,
                    owner: final_order.owner,
                    top_to_bottom: final_order.top_to_bottom.clone(),
                },
            )?);
        }
        events.push(Self::s3_a_bound_event(
            state,
            events.len() as u64,
            revision,
            AuthoritativeRuleEventKind::StateBasedActionsApplied {
                actions: selected_sba_actions.clone(),
            },
        )?);

        let losing_players: Vec<_> = selected_sba_actions
            .iter()
            .filter_map(|action| match action {
                SbaSelectedActionV1::PlayerLoses { player } => Some(*player),
                SbaSelectedActionV1::ObjectToOwnerGraveyard { .. } => None,
            })
            .collect();
        let status = match losing_players.as_slice() {
            [] => EpisodeStatus::Running,
            [loser] => EpisodeStatus::Terminal {
                reason: TerminalReason::RulesLoss,
                players: state
                    .core
                    .players
                    .keys()
                    .copied()
                    .map(|player| PlayerOutcome {
                        player,
                        result: if player == *loser {
                            PlayerResult::Loss
                        } else {
                            PlayerResult::Win
                        },
                    })
                    .collect(),
            },
            _ => EpisodeStatus::Terminal {
                reason: TerminalReason::SimultaneousOutcome,
                players: state
                    .core
                    .players
                    .keys()
                    .copied()
                    .map(|player| PlayerOutcome {
                        player,
                        result: PlayerResult::Draw,
                    })
                    .collect(),
            },
        };

        let mut candidate = state.clone();
        candidate.revision = revision;
        candidate.execution.pending_decision = None;
        if let Some((continuation_id, _)) = &final_order {
            if candidate
                .execution
                .continuations
                .remove(continuation_id)
                .is_none()
            {
                return Err(KernelExecutionError::UnsupportedStagePath);
            }
        }
        for action in &selected_sba_actions {
            match action {
                SbaSelectedActionV1::PlayerLoses { player } => {
                    let player_state = candidate
                        .core
                        .players
                        .get_mut(player)
                        .ok_or(KernelExecutionError::UnsupportedStagePath)?;
                    if player_state.has_lost || player_state.life > 0 {
                        return Err(KernelExecutionError::UnsupportedStagePath);
                    }
                    player_state.has_lost = true;
                }
                SbaSelectedActionV1::ObjectToOwnerGraveyard { .. } => {}
            }
        }

        let selected_objects: std::collections::BTreeSet<_> = selected_sba_actions
            .iter()
            .filter_map(|action| match action {
                SbaSelectedActionV1::ObjectToOwnerGraveyard { object, .. } => Some(*object),
                SbaSelectedActionV1::PlayerLoses { .. } => None,
            })
            .collect();
        if let Some(combat) = &mut candidate.combat {
            let participant_selected = combat
                .attackers
                .iter()
                .any(|object| selected_objects.contains(object))
                || combat.blockers.iter().any(|(attacker, blocker)| {
                    selected_objects.contains(attacker)
                        || blocker.is_some_and(|object| selected_objects.contains(&object))
                });
            if participant_selected
                && !matches!(
                    state.core.position,
                    TurnPosition::Combat {
                        step: mtgml_state::CombatStep::CombatDamage
                    }
                )
            {
                return Err(KernelExecutionError::UnsupportedStagePath);
            }
            if participant_selected {
                combat
                    .attackers
                    .retain(|attacker| !selected_objects.contains(attacker));
                combat
                    .blockers
                    .retain(|attacker, _| !selected_objects.contains(attacker));
                for blocker in combat.blockers.values_mut() {
                    if blocker.is_some_and(|object| selected_objects.contains(&object)) {
                        *blocker = None;
                    }
                }
            }
        }

        let orders: Vec<SbaGraveyardOwnerOrderV1> = final_order
            .as_ref()
            .map(|(_, orders)| orders.clone())
            .unwrap_or_default();
        let invocation_order =
            crate::state_based_actions::ordered_sba_objects_for_s2(state, &plan, &orders)
                .map_err(|_| KernelExecutionError::UnsupportedStagePath)?;
        for object in invocation_order {
            let owner = candidate
                .zones
                .objects
                .get(&object)
                .ok_or(KernelExecutionError::UnsupportedStagePath)?
                .owner;
            let from = candidate
                .zones
                .locations
                .get(&object)
                .cloned()
                .ok_or(KernelExecutionError::UnsupportedStagePath)?;
            let to = ZoneLocation {
                zone: ZoneKind::Graveyard,
                player: Some(owner),
                position: ZonePosition::Top { offset: 0 },
                visibility: VisibilityPartition::Public,
                partition: None,
            };
            crate::zone_incarnation::apply_selected_zone_transition_in_sba_batch_workspace(
                &mut candidate,
                &crate::zone_incarnation::SelectedZoneTransitionRequest {
                    object,
                    kind: crate::zone_incarnation::SelectedZoneTransitionKind::BattlefieldToOwnerGraveyard,
                    claimed_from: from,
                    claimed_to: to,
                },
                state.allocators.next_rule_event_id,
                &mut events,
            )?;
        }

        validate_engine_state(&candidate).map_err(KernelExecutionError::AfterState)?;

        if matches!(status, EpisodeStatus::Running) {
            let next_plan = crate::state_based_actions::derive_bounded_sba_round_plan(&candidate)
                .map_err(|_| KernelExecutionError::UnsupportedStagePath)?;
            // The selected closed profile has no effects, triggers, tokens,
            // or characteristic-changing machinery that could create a new
            // SBA fact after this complete simultaneous batch. A nonempty
            // re-derivation therefore means the profile contract was violated.
            if !next_plan.selected_sba_actions.is_empty() {
                return Err(KernelExecutionError::UnsupportedStagePath);
            }
        }

        if open_priority_if_stable && matches!(status, EpisodeStatus::Running) {
            crate::basic_priority::validate_pass_only_state(&candidate, false)?;
            let actor = candidate.core.active_player;
            let identity = crate::decision_stage::fresh_stage_identity(state, actor)?;
            let request = crate::basic_priority::make_pass_request(
                actor,
                revision,
                identity.decision_id,
                identity.player_decision_id,
            )?;
            let next_decision = DecisionId(
                identity
                    .decision_id
                    .0
                    .checked_add(1)
                    .ok_or(KernelExecutionError::Exhaustion("decision"))?,
            );
            events.push(Self::s3_a_bound_event(
                state,
                u64::try_from(events.len())
                    .map_err(|_| KernelExecutionError::RuleEventIdOverflow)?,
                revision,
                AuthoritativeRuleEventKind::PriorityChanged {
                    from: mtgml_state::PriorityState::None,
                    to: mtgml_state::PriorityState::HeldBy {
                        player: actor,
                        consecutive_passes: 0,
                    },
                },
            )?);
            events.push(Self::s3_a_bound_event(
                state,
                u64::try_from(events.len())
                    .map_err(|_| KernelExecutionError::RuleEventIdOverflow)?,
                revision,
                AuthoritativeRuleEventKind::DecisionCreated {
                    decision: identity.decision_id,
                },
            )?);
            candidate.core.priority = mtgml_state::PriorityState::HeldBy {
                player: actor,
                consecutive_passes: 0,
            };
            candidate.execution.pending_decision = Some(PendingDecisionRecordV2 { request });
            candidate.allocators.next_decision_id = next_decision;
            crate::decision_stage::advance_player_allocator(
                &mut candidate,
                actor,
                identity.player_decision_id,
            )?;
        }

        build_accepted_product_with_status(state, candidate, events, status, |_| Ok(()))
    }

    fn apply_s3_a_order_response(
        &mut self,
        state: &EngineState,
        trusted_actor: PlayerId,
        response: &DecisionResponseV2,
    ) -> Result<TransitionResult, KernelExecutionError> {
        validate_engine_state(state).map_err(KernelExecutionError::BeforeState)?;
        let Some(pending) = state.execution.pending_decision.as_ref() else {
            return crate::decision_stage::rejected(state);
        };
        let request = &pending.request;
        let Ok(visible_request) = request.project_player_request() else {
            return crate::decision_stage::rejected(state);
        };
        if trusted_actor != request.actor
            || response.validate_for(&visible_request).is_err()
            || response.state_revision != state.revision
        {
            return crate::decision_stage::rejected(state);
        }
        let DecisionAnswerV2::Order { candidate_ids } = &response.answer else {
            return crate::decision_stage::rejected(state);
        };
        let Some(continuation_id) = request.continuation_id else {
            return crate::decision_stage::rejected(state);
        };
        crate::state_based_actions::validate_sba_order_continuation(state)
            .map_err(|_| KernelExecutionError::UnsupportedStagePath)?;
        let continuation = state
            .execution
            .continuations
            .get(&continuation_id)
            .cloned()
            .ok_or(KernelExecutionError::UnsupportedStagePath)?;
        let ContinuationPayloadV2::MagicSbaGraveyardOrderV1 {
            selected_sba_actions,
            apnap_owners,
            next_owner_index,
            ..
        } = &continuation.payload
        else {
            return Err(KernelExecutionError::UnsupportedStagePath);
        };
        let owner = *apnap_owners
            .get(*next_owner_index as usize)
            .ok_or(KernelExecutionError::UnsupportedStagePath)?;
        if owner != trusted_actor || owner != request.actor {
            return crate::decision_stage::rejected(state);
        }

        let top_to_bottom = Self::s3_a_resolve_order_answer(
            state,
            request,
            owner,
            selected_sba_actions,
            candidate_ids,
        )?;
        let next_index = next_owner_index
            .checked_add(1)
            .ok_or(KernelExecutionError::Exhaustion("continuation_stage"))?;
        let next_owner = apnap_owners
            .get(
                usize::try_from(next_index)
                    .map_err(|_| KernelExecutionError::Exhaustion("continuation_stage"))?,
            )
            .copied();
        if next_owner.is_none() {
            let ContinuationPayloadV2::MagicSbaGraveyardOrderV1 {
                completed_owner_orders,
                ..
            } = &continuation.payload
            else {
                return Err(KernelExecutionError::UnsupportedStagePath);
            };
            let mut orders = completed_owner_orders.clone();
            orders.push(SbaGraveyardOwnerOrderV1 {
                owner,
                top_to_bottom,
            });
            return self.apply_s3_a_batch(
                state,
                Some((continuation_id, orders)),
                selected_sba_actions.clone(),
                false,
            );
        }
        let next_owner = next_owner.ok_or(KernelExecutionError::UnsupportedStagePath)?;

        let identity = crate::decision_stage::fresh_stage_identity(state, next_owner)?;
        let candidates = Self::s3_a_order_candidates(state, selected_sba_actions, next_owner)?;
        let cardinality = u32::try_from(candidates.len())
            .map_err(|_| KernelExecutionError::Exhaustion("order_cardinality"))?;
        let next_request = mtgml_decision::AuthoritativeDecisionRequestV2 {
            decision_id: identity.decision_id,
            player_decision_id: identity.player_decision_id,
            state_revision: identity.revision,
            actor: next_owner,
            visibility: DecisionVisibility::ActingPlayerOnly,
            decision: DecisionDomainV2::Order {
                minimum: cardinality,
                maximum: cardinality,
            },
            candidates,
            continuation_id: Some(continuation_id),
        };
        let next_decision = DecisionId(
            identity
                .decision_id
                .0
                .checked_add(1)
                .ok_or(KernelExecutionError::Exhaustion("decision"))?,
        );
        let events = vec![
            Self::s3_a_bound_event(
                state,
                0,
                identity.revision,
                AuthoritativeRuleEventKind::DecisionCleared {
                    decision: request.decision_id,
                },
            )?,
            Self::s3_a_bound_event(
                state,
                1,
                identity.revision,
                AuthoritativeRuleEventKind::SbaGraveyardOrderChosen {
                    continuation: continuation_id,
                    owner,
                    top_to_bottom: top_to_bottom.clone(),
                },
            )?,
            Self::s3_a_bound_event(
                state,
                2,
                identity.revision,
                AuthoritativeRuleEventKind::DecisionCreated {
                    decision: identity.decision_id,
                },
            )?,
        ];

        let mut next = state.clone();
        next.revision = identity.revision;
        build_accepted_product(state, next, events, |workspace| {
            workspace.execution.pending_decision = Some(PendingDecisionRecordV2 {
                request: next_request,
            });
            let record = workspace
                .execution
                .continuations
                .get_mut(&continuation_id)
                .ok_or(KernelExecutionError::UnsupportedStagePath)?;
            let ContinuationPayloadV2::MagicSbaGraveyardOrderV1 {
                next_owner_index,
                completed_owner_orders,
                ..
            } = &mut record.payload
            else {
                return Err(KernelExecutionError::UnsupportedStagePath);
            };
            completed_owner_orders.push(SbaGraveyardOwnerOrderV1 {
                owner,
                top_to_bottom,
            });
            *next_owner_index = next_index;
            record.actor = next_owner;
            record.stage_index = u16::try_from(next_index)
                .map_err(|_| KernelExecutionError::Exhaustion("continuation_stage"))?;
            workspace.allocators.next_decision_id = next_decision;
            crate::decision_stage::advance_player_allocator(
                workspace,
                next_owner,
                identity.player_decision_id,
            )?;
            Ok(())
        })
    }

    fn apply_priority_response(
        &mut self,
        state: &EngineState,
        trusted_actor: PlayerId,
        response: &DecisionResponseV2,
    ) -> Result<TransitionResult, KernelExecutionError> {
        if state.execution.pending_decision.is_none() {
            return crate::decision_stage::rejected(state);
        }
        crate::basic_priority::validate_pass_only_state(state, true)?;
        let Some(pending) = state.execution.pending_decision.as_ref() else {
            return crate::decision_stage::rejected(state);
        };
        let request = &pending.request;
        let Ok(visible_request) = request.project_player_request() else {
            return crate::decision_stage::rejected(state);
        };
        if trusted_actor != request.actor
            || response.validate_for(&visible_request).is_err()
            || response.state_revision != state.revision
            || !matches!(&response.answer, DecisionAnswerV2::SelectOne { candidate_id } if candidate_id.0 == 0)
        {
            return crate::decision_stage::rejected(state);
        }
        let unique_other = state
            .core
            .players
            .keys()
            .copied()
            .find(|player| *player != state.core.active_player)
            .ok_or(KernelExecutionError::UnsupportedStagePath)?;
        match state.core.priority {
            mtgml_state::PriorityState::HeldBy {
                player,
                consecutive_passes: 0,
            } if player == state.core.active_player && request.actor == player => {
                let identity = crate::decision_stage::fresh_stage_identity(state, unique_other)?;
                let next_request = crate::basic_priority::make_pass_request(
                    unique_other,
                    identity.revision,
                    identity.decision_id,
                    identity.player_decision_id,
                )?;
                let next_decision = DecisionId(
                    identity
                        .decision_id
                        .0
                        .checked_add(1)
                        .ok_or(KernelExecutionError::Exhaustion("decision"))?,
                );
                let events = vec![
                    crate::basic_priority::bound_event(
                        state,
                        0,
                        identity.revision,
                        AuthoritativeRuleEventKind::DecisionCleared {
                            decision: request.decision_id,
                        },
                    )?,
                    crate::basic_priority::bound_event(
                        state,
                        1,
                        identity.revision,
                        AuthoritativeRuleEventKind::PriorityChanged {
                            from: mtgml_state::PriorityState::HeldBy {
                                player,
                                consecutive_passes: 0,
                            },
                            to: mtgml_state::PriorityState::HeldBy {
                                player: unique_other,
                                consecutive_passes: 1,
                            },
                        },
                    )?,
                    crate::basic_priority::bound_event(
                        state,
                        2,
                        identity.revision,
                        AuthoritativeRuleEventKind::DecisionCreated {
                            decision: identity.decision_id,
                        },
                    )?,
                ];
                let mut next = state.clone();
                next.revision = identity.revision;
                build_accepted_product(state, next, events, |workspace| {
                    workspace.core.priority = mtgml_state::PriorityState::HeldBy {
                        player: unique_other,
                        consecutive_passes: 1,
                    };
                    workspace.execution.pending_decision = Some(PendingDecisionRecordV2 {
                        request: next_request,
                    });
                    workspace.allocators.next_decision_id = next_decision;
                    crate::decision_stage::advance_player_allocator(
                        workspace,
                        unique_other,
                        identity.player_decision_id,
                    )?;
                    Ok(())
                })
            }
            mtgml_state::PriorityState::HeldBy {
                player,
                consecutive_passes: 1,
            } if player != state.core.active_player && request.actor == player => {
                let revision = crate::decision_stage::next_revision(state)?;
                let successor = temporal_successor(state.core.position);
                let events = vec![
                    crate::basic_priority::bound_event(
                        state,
                        0,
                        revision,
                        AuthoritativeRuleEventKind::DecisionCleared {
                            decision: request.decision_id,
                        },
                    )?,
                    crate::basic_priority::bound_event(
                        state,
                        1,
                        revision,
                        AuthoritativeRuleEventKind::PriorityChanged {
                            from: mtgml_state::PriorityState::HeldBy {
                                player,
                                consecutive_passes: 1,
                            },
                            to: mtgml_state::PriorityState::None,
                        },
                    )?,
                    crate::basic_priority::bound_event(
                        state,
                        2,
                        revision,
                        AuthoritativeRuleEventKind::TurnPositionChanged {
                            from: state.core.position,
                            to: successor,
                        },
                    )?,
                ];
                let mut next = state.clone();
                next.revision = revision;
                build_accepted_product(state, next, events, |workspace| {
                    workspace.core.priority = mtgml_state::PriorityState::None;
                    workspace.core.position = successor;
                    workspace.execution.pending_decision = None;
                    Ok(())
                })
            }
            _ => crate::decision_stage::rejected(state),
        }
    }

    fn s3_a_order_candidates(
        state: &EngineState,
        selected_actions: &[mtgml_state::SbaSelectedActionV1],
        actor: PlayerId,
    ) -> Result<Vec<AuthoritativeCandidateV2>, KernelExecutionError> {
        let identity = state
            .perspective_identities
            .players
            .get(&actor)
            .ok_or(KernelExecutionError::UnsupportedStagePath)?;
        let mut candidates = Vec::new();
        for action in selected_actions {
            let mtgml_state::SbaSelectedActionV1::ObjectToOwnerGraveyard { object, .. } = action
            else {
                continue;
            };
            let game_object = state
                .zones
                .objects
                .get(object)
                .ok_or(KernelExecutionError::UnsupportedStagePath)?;
            if game_object.owner != actor {
                continue;
            }
            let opaque = identity
                .object_to_opaque
                .get(object)
                .copied()
                .ok_or(KernelExecutionError::UnsupportedStagePath)?;
            candidates.push((
                CandidateIntent::SelectObject { object: opaque },
                EngineCandidateBinding::SelectObject { object: *object },
            ));
        }
        let ordered = CandidateOrderingV1::assign_dense(candidates)
            .map_err(|_| KernelExecutionError::UnsupportedStagePath)?;
        if ordered.len() < 2 {
            return Err(KernelExecutionError::UnsupportedStagePath);
        }
        Ok(ordered)
    }

    fn s3_a_resolve_order_answer(
        state: &EngineState,
        request: &mtgml_decision::AuthoritativeDecisionRequestV2,
        owner: PlayerId,
        selected_actions: &[mtgml_state::SbaSelectedActionV1],
        candidate_ids: &[CandidateIdV1],
    ) -> Result<Vec<mtgml_model::GameObjectId>, KernelExecutionError> {
        let identity = state
            .perspective_identities
            .players
            .get(&owner)
            .ok_or(KernelExecutionError::UnsupportedStagePath)?;
        let mut resolved = Vec::with_capacity(candidate_ids.len());
        for candidate_id in candidate_ids {
            let candidate_index = usize::try_from(candidate_id.0)
                .map_err(|_| KernelExecutionError::UnsupportedStagePath)?;
            let candidate = request
                .candidates
                .get(candidate_index)
                .filter(|candidate| candidate.candidate_id == *candidate_id)
                .ok_or(KernelExecutionError::UnsupportedStagePath)?;
            let (
                CandidateIntent::SelectObject { object: opaque },
                EngineCandidateBinding::SelectObject { object },
            ) = (&candidate.visible_intent, &candidate.trusted_binding)
            else {
                return Err(KernelExecutionError::UnsupportedStagePath);
            };
            if identity.opaque_to_object.get(opaque) != Some(object)
                || identity.object_to_opaque.get(object) != Some(opaque)
                || !selected_actions.iter().any(|action| {
                    matches!(
                        action,
                        mtgml_state::SbaSelectedActionV1::ObjectToOwnerGraveyard {
                            object: selected,
                            ..
                        } if selected == object
                    )
                })
            {
                return Err(KernelExecutionError::UnsupportedStagePath);
            }
            resolved.push(*object);
        }
        Ok(resolved)
    }

    fn s3_a_bound_event(
        state: &EngineState,
        offset: u64,
        revision: StateRevision,
        event: AuthoritativeRuleEventKind,
    ) -> Result<AuthoritativeRuleEvent, KernelExecutionError> {
        Ok(AuthoritativeRuleEvent {
            event_id: RuleEventId(
                state
                    .allocators
                    .next_rule_event_id
                    .0
                    .checked_add(offset)
                    .ok_or(KernelExecutionError::RuleEventIdOverflow)?,
            ),
            state_revision: revision,
            event,
        })
    }
}

impl MagicRulesKernel {
    /// Rules-owned forced-progress shell.
    ///
    /// The fixed non-production S3.A candidate takes its separate typed SBA
    /// Order staging path. Admitted S1 validates its exact execution profile,
    /// then classifies the current temporal position: Untap advances to
    /// Upkeep, and downstream boundaries remain typed failures. No production
    /// S1 accepted state set changes here.
    pub(crate) fn advance_forced_progress(
        &mut self,
        state: &EngineState,
    ) -> Result<TransitionResult, KernelExecutionError> {
        if self.profile.allows_s3_a() {
            return self.advance_s3_a_round(state);
        }
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
