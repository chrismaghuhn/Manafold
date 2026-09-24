//! Closed two-player, pass-only Basic Priority semantics for S3.B.

use mtgml_decision::{
    AuthoritativeDecisionRequestV2, CandidateIntent, CandidateOrderingV1, DecisionDomainV2,
    DecisionVisibility, EngineCandidateBinding,
};
use mtgml_model::{DecisionId, PlayerId, RuleEventId, StateRevision};
use mtgml_state::{
    validate_engine_state, EngineState, PendingDecisionRecordV2, PriorityState, TurnPosition,
};

use crate::decision_stage::{advance_player_allocator, fresh_stage_identity};
use crate::errors::KernelExecutionError;
use crate::events::{AuthoritativeRuleEvent, AuthoritativeRuleEventKind};
use crate::product::build_accepted_product;
use crate::transition::TransitionResult;
use crate::validation::TransitionViolation;

pub(crate) fn is_priority_bearing_position(position: TurnPosition) -> bool {
    matches!(
        position,
        TurnPosition::Beginning {
            step: mtgml_state::BeginningStep::Upkeep
        } | TurnPosition::PrecombatMain
            | TurnPosition::PostcombatMain
            | TurnPosition::Ending {
                step: mtgml_state::EndingStep::EndStep
            }
    )
}

/// Proves the currently representable pass-only S3.B state from authoritative
/// state. SBA semantics remain owned by the S3.A Rules authority.
pub(crate) fn validate_pass_only_state(
    state: &EngineState,
    require_held_decision: bool,
) -> Result<(), KernelExecutionError> {
    validate_engine_state(state).map_err(KernelExecutionError::BeforeState)?;
    if !is_priority_bearing_position(state.core.position)
        || !state.execution.continuations.is_empty()
        || state.combat.is_some()
        || state.core.players.values().any(|player| player.has_lost)
    {
        return Err(KernelExecutionError::UnsupportedStagePath);
    }

    let mut stable = state.clone();
    stable.core.priority = PriorityState::None;
    stable.execution.pending_decision = None;
    crate::state_based_actions::validate_s3_a_support_profile(&stable)
        .map_err(|_| KernelExecutionError::UnsupportedStagePath)?;
    let plan = crate::state_based_actions::derive_bounded_sba_round_plan(&stable)
        .map_err(|_| KernelExecutionError::UnsupportedStagePath)?;
    if !plan.selected_sba_actions.is_empty() || !plan.apnap_owners.is_empty() {
        return Err(KernelExecutionError::UnsupportedStagePath);
    }

    match (
        state.core.priority,
        state.execution.pending_decision.as_ref(),
    ) {
        (PriorityState::None, None) if !require_held_decision => Ok(()),
        (
            PriorityState::HeldBy {
                player,
                consecutive_passes,
            },
            Some(pending),
        ) if require_held_decision
            && ((player == state.core.active_player && consecutive_passes == 0)
                || (player != state.core.active_player && consecutive_passes == 1))
            && pending.request.actor == player
            && pending.request.state_revision == state.revision
            && pending.request.visibility == DecisionVisibility::ActingPlayerOnly
            && pending.request.continuation_id.is_none()
            && pending.request.decision == DecisionDomainV2::ChooseOne
            && pending.request.candidates.len() == 1
            && pending.request.candidates[0].candidate_id.0 == 0
            && pending.request.candidates[0].visible_intent == CandidateIntent::PassPriority
            && pending.request.candidates[0].trusted_binding
                == EngineCandidateBinding::PassPriority
            && pending.request.project_player_request().is_ok() =>
        {
            Ok(())
        }
        _ => Err(KernelExecutionError::UnsupportedStagePath),
    }
}

pub(crate) fn open_priority_window(
    state: &EngineState,
) -> Result<TransitionResult, KernelExecutionError> {
    validate_pass_only_state(state, false)?;
    let actor = state.core.active_player;
    let identity = fresh_stage_identity(state, actor)?;
    let request = make_pass_request(
        actor,
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
    let event = bound_event(
        state,
        0,
        identity.revision,
        AuthoritativeRuleEventKind::PriorityChanged {
            from: PriorityState::None,
            to: PriorityState::HeldBy {
                player: actor,
                consecutive_passes: 0,
            },
        },
    )?;
    let created = bound_event(
        state,
        1,
        identity.revision,
        AuthoritativeRuleEventKind::DecisionCreated {
            decision: identity.decision_id,
        },
    )?;
    let mut next = state.clone();
    next.revision = identity.revision;
    build_accepted_product(state, next, vec![event, created], |workspace| {
        workspace.core.priority = PriorityState::HeldBy {
            player: actor,
            consecutive_passes: 0,
        };
        workspace.execution.pending_decision = Some(PendingDecisionRecordV2 { request });
        workspace.allocators.next_decision_id = next_decision;
        advance_player_allocator(workspace, actor, identity.player_decision_id)?;
        Ok(())
    })
}

pub(crate) fn validate_priority_transition(
    before: &EngineState,
    result: &TransitionResult,
) -> Result<(), TransitionViolation> {
    let after = &result.next_state;
    let priority_events: Vec<_> = result
        .events
        .iter()
        .filter_map(|event| match &event.event {
            AuthoritativeRuleEventKind::PriorityChanged { from, to } => Some((*from, *to)),
            _ => None,
        })
        .collect();
    if before.core.priority == after.core.priority {
        if !priority_events.is_empty() {
            return Err(TransitionViolation::Priority);
        }
        return Ok(());
    }
    if priority_events.len() != 1
        || priority_events[0] != (before.core.priority, after.core.priority)
    {
        return Err(TransitionViolation::Priority);
    }
    match (before.core.priority, after.core.priority) {
        (
            PriorityState::None,
            PriorityState::HeldBy {
                player,
                consecutive_passes: 0,
            },
        ) => {
            if player != before.core.active_player
                || after.core.position != before.core.position
                || after.execution.pending_decision.is_none()
                || !matches!(
                    result
                        .events
                        .get(result.events.len().saturating_sub(2))
                        .map(|e| &e.event),
                    Some(AuthoritativeRuleEventKind::PriorityChanged { .. })
                )
            {
                return Err(TransitionViolation::Priority);
            }
            validate_pass_only_state(after, true).map_err(|_| TransitionViolation::Priority)?;
        }
        (
            PriorityState::HeldBy {
                player: from_player,
                consecutive_passes: 0,
            },
            PriorityState::HeldBy {
                player: to_player,
                consecutive_passes: 1,
            },
        ) => {
            if from_player != before.core.active_player
                || to_player == from_player
                || result.events.len() != 3
                || !matches!(
                    result.events[0].event,
                    AuthoritativeRuleEventKind::DecisionCleared { .. }
                )
                || !matches!(
                    result.events[1].event,
                    AuthoritativeRuleEventKind::PriorityChanged { .. }
                )
                || !matches!(
                    result.events[2].event,
                    AuthoritativeRuleEventKind::DecisionCreated { .. }
                )
                || after.core.position != before.core.position
            {
                return Err(TransitionViolation::Priority);
            }
            validate_pass_only_state(after, true).map_err(|_| TransitionViolation::Priority)?;
        }
        (
            PriorityState::HeldBy {
                player: from_player,
                consecutive_passes: 1,
            },
            PriorityState::None,
        ) => {
            let cleanup_composition =
                crate::contract::is_second_pass_cleanup_composition(before, result);
            let expected_position = if cleanup_composition {
                TurnPosition::Beginning {
                    step: mtgml_state::BeginningStep::Untap,
                }
            } else {
                crate::turn_structure::temporal_successor(before.core.position)
            };
            if from_player == before.core.active_player
                || result.events.len() != if cleanup_composition { 6 } else { 3 }
                || !matches!(
                    result.events[0].event,
                    AuthoritativeRuleEventKind::DecisionCleared { .. }
                )
                || !matches!(
                    result.events[1].event,
                    AuthoritativeRuleEventKind::PriorityChanged { .. }
                )
                || !matches!(
                    result.events[2].event,
                    AuthoritativeRuleEventKind::TurnPositionChanged { from, to }
                        if from == before.core.position
                            && to == crate::turn_structure::temporal_successor(from)
                )
                || after.core.position != expected_position
                || after.execution.pending_decision.is_some()
            {
                return Err(TransitionViolation::Priority);
            }
        }
        _ => return Err(TransitionViolation::Priority),
    }
    Ok(())
}

pub(crate) fn make_pass_request(
    actor: PlayerId,
    revision: StateRevision,
    decision_id: DecisionId,
    player_decision_id: mtgml_model::PlayerDecisionIdV1,
) -> Result<AuthoritativeDecisionRequestV2, KernelExecutionError> {
    let candidates = CandidateOrderingV1::assign_dense(vec![(
        CandidateIntent::PassPriority,
        EngineCandidateBinding::PassPriority,
    )])
    .map_err(|_| KernelExecutionError::UnsupportedStagePath)?;
    Ok(AuthoritativeDecisionRequestV2 {
        decision_id,
        player_decision_id,
        state_revision: revision,
        actor,
        visibility: DecisionVisibility::ActingPlayerOnly,
        decision: DecisionDomainV2::ChooseOne,
        candidates,
        continuation_id: None,
    })
}

pub(crate) fn bound_event(
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
