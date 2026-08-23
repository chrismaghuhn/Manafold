//! Neutral accepted-product epilogue shared by the synthetic decision kernel
//! and the M2.E conformance fixture support.

use mtgml_model::{EpisodeStatus, RuleEventId};
use mtgml_state::{validate_engine_state, EngineState, StateDelta};

use crate::contract::validate_transition_contract;
use crate::errors::KernelExecutionError;
use crate::events::AuthoritativeRuleEvent;
use crate::transition::TransitionResult;

/// Shared accepted-product epilogue: applies the workspace mutation, closes
/// the event cursor, builds the exact delta, and validates the complete
/// product before returning it for atomic commit.
pub(crate) fn build_accepted_product(
    state: &EngineState,
    mut next: EngineState,
    events: Vec<AuthoritativeRuleEvent>,
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
        status: EpisodeStatus::Running,
        next_state: next,
        delta,
        events,
    };
    validate_engine_state(&result.next_state).map_err(KernelExecutionError::AfterState)?;
    validate_transition_contract(state, &result)
        .map_err(KernelExecutionError::TransitionContract)?;
    Ok(result)
}
