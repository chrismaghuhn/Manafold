//! Neutral accepted-product epilogue shared by the synthetic decision kernel
//! and the M2.E conformance fixture support.

use mtgml_model::{EpisodeStatus, RuleEventId};
use mtgml_state::{validate_engine_state, EngineState, StateDelta};

use crate::contract::validate_transition_contract;
use crate::errors::KernelExecutionError;
use crate::events::AuthoritativeRuleEvent;
use crate::transition::TransitionResult;

/// Compose two already validated Rules products into one externally atomic
/// product. The intermediate state remains kernel-local; the second product's
/// final revision is folded into the first product's revision, while event
/// order and allocator progress remain contiguous. Callers must run the
/// composed transition contract before returning it.
pub(crate) fn compose_atomic_products(
    before: &EngineState,
    first: TransitionResult,
    second: TransitionResult,
) -> Result<TransitionResult, KernelExecutionError> {
    if !first.accepted
        || !second.accepted
        || first.next_state.revision.0
            != before
                .revision
                .0
                .checked_add(1)
                .ok_or(KernelExecutionError::RevisionOverflow)?
        || second.delta.before_revision != first.next_state.revision
        || second.delta.before_digest
            != first
                .next_state
                .digest()
                .map_err(KernelExecutionError::Delta)?
        || second.next_state.revision.0
            != first
                .next_state
                .revision
                .0
                .checked_add(1)
                .ok_or(KernelExecutionError::RevisionOverflow)?
        || !matches!(&first.status, EpisodeStatus::Running)
        || first.next_decision.is_some()
    {
        return Err(KernelExecutionError::UnsupportedStagePath);
    }

    let target_revision = first.next_state.revision;
    let old_final_revision = second.next_state.revision;
    let mut next_state = second.next_state;
    next_state.revision = target_revision;
    if let Some(pending) = next_state.execution.pending_decision.as_mut() {
        pending.request.state_revision = target_revision;
    }
    for continuation in next_state.execution.continuations.values_mut() {
        if continuation.created_at_revision == old_final_revision {
            continuation.created_at_revision = target_revision;
        }
    }

    let mut events = first.events;
    events.extend(second.events);
    for event in &mut events {
        event.state_revision = target_revision;
    }
    let audit = events
        .iter()
        .map(|event| event.event.semantic_delta())
        .collect();
    let delta =
        StateDelta::between(before, &next_state, audit).map_err(KernelExecutionError::Delta)?;
    let next_decision = next_state
        .execution
        .pending_decision
        .as_ref()
        .map(|pending| pending.request.clone());
    Ok(TransitionResult {
        accepted: true,
        next_state,
        delta,
        events,
        next_decision,
        status: second.status,
    })
}

/// Compose two Rules products while retaining both externally meaningful
/// revisions. This is required when the second product persists a
/// continuation whose creation revision must remain strictly after the
/// round-start revision.
pub(crate) fn compose_sequential_products(
    before: &EngineState,
    first: TransitionResult,
    second: TransitionResult,
) -> Result<TransitionResult, KernelExecutionError> {
    if !first.accepted
        || !second.accepted
        || first.next_state.revision.0
            != before
                .revision
                .0
                .checked_add(1)
                .ok_or(KernelExecutionError::RevisionOverflow)?
        || second.delta.before_revision != first.next_state.revision
        || second.delta.before_digest
            != first
                .next_state
                .digest()
                .map_err(KernelExecutionError::Delta)?
        || second.next_state.revision.0
            != first
                .next_state
                .revision
                .0
                .checked_add(1)
                .ok_or(KernelExecutionError::RevisionOverflow)?
        || !matches!(&first.status, EpisodeStatus::Running)
        || first.next_decision.is_some()
    {
        return Err(KernelExecutionError::UnsupportedStagePath);
    }
    let mut events = first.events;
    events.extend(second.events);
    let audit = events
        .iter()
        .map(|event| event.event.semantic_delta())
        .collect();
    let delta = StateDelta::between(before, &second.next_state, audit)
        .map_err(KernelExecutionError::Delta)?;
    let next_decision = second
        .next_state
        .execution
        .pending_decision
        .as_ref()
        .map(|pending| pending.request.clone());
    Ok(TransitionResult {
        accepted: true,
        next_state: second.next_state,
        delta,
        events,
        next_decision,
        status: second.status,
    })
}

/// Shared accepted-product epilogue: applies the workspace mutation, closes
/// the event cursor, builds the exact delta, and validates the complete
/// product before returning it for atomic commit.
pub(crate) fn build_accepted_product(
    state: &EngineState,
    next: EngineState,
    events: Vec<AuthoritativeRuleEvent>,
    mutate: impl FnOnce(&mut EngineState) -> Result<(), KernelExecutionError>,
) -> Result<TransitionResult, KernelExecutionError> {
    build_accepted_product_with_status(state, next, events, EpisodeStatus::Running, mutate)
}

pub(crate) fn build_accepted_product_with_status(
    state: &EngineState,
    mut next: EngineState,
    events: Vec<AuthoritativeRuleEvent>,
    status: EpisodeStatus,
    mutate: impl FnOnce(&mut EngineState) -> Result<(), KernelExecutionError>,
) -> Result<TransitionResult, KernelExecutionError> {
    let next_rule_event_id = state
        .allocators
        .next_rule_event_id
        .0
        .checked_add(events.len() as u64)
        .ok_or(KernelExecutionError::RuleEventIdOverflow)?;
    mutate(&mut next)?;
    next.allocators.next_rule_event_id = RuleEventId(next_rule_event_id);

    let audit = events
        .iter()
        .map(|event| event.event.semantic_delta())
        .collect();
    let delta = StateDelta::between(state, &next, audit).map_err(KernelExecutionError::Delta)?;
    let result = TransitionResult {
        accepted: true,
        next_decision: next
            .execution
            .pending_decision
            .as_ref()
            .map(|record| record.request.clone()),
        status,
        next_state: next,
        delta,
        events,
    };
    validate_engine_state(&result.next_state).map_err(KernelExecutionError::AfterState)?;
    validate_transition_contract(state, &result)
        .map_err(KernelExecutionError::TransitionContract)?;
    Ok(result)
}
