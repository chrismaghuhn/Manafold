use mtgml_decision::{validate_candidate_binding, DecisionKind, DecisionResponse};
use mtgml_model::{EpisodeStatus, PlayerId, RuleEventId, StateRevision};
use mtgml_state::{validate_engine_state, EngineState, EngineStateViolation, StateDelta};

use crate::errors::KernelExecutionError;
use crate::events::{AuthoritativeRuleEvent, AuthoritativeRuleEventKind};
use crate::transition::{RulesKernel, TransitionResult};
use crate::validate_transition_contract;

#[derive(Debug, Default)]
pub struct SyntheticM1RulesKernel;

impl RulesKernel for SyntheticM1RulesKernel {
    fn apply(
        &mut self,
        state: &EngineState,
        trusted_actor: PlayerId,
        response: &DecisionResponse,
    ) -> Result<TransitionResult, KernelExecutionError> {
        validate_engine_state(state).map_err(KernelExecutionError::BeforeState)?;

        let Some(pending) = state.execution.pending_decision.as_ref() else {
            return rejected(state);
        };
        let request = &pending.request;
        if trusted_actor != request.actor
            || response.validate().is_err()
            || response.decision_id != request.decision_id
            || response.state_revision != state.revision
            || !matches!(request.decision, DecisionKind::ChooseOne)
            || response.assignments.len() != 1
        {
            return rejected(state);
        }

        let assignment = &response.assignments[0];
        let Some(candidate) = request
            .candidates
            .iter()
            .find(|candidate| candidate.candidate_id == assignment.candidate_id)
        else {
            return rejected(state);
        };
        let binding = pending
            .candidate_bindings
            .get(&assignment.candidate_id)
            .ok_or(KernelExecutionError::BeforeState(
                EngineStateViolation::PendingDecisionMismatch,
            ))?;
        if validate_candidate_binding(
            candidate,
            binding,
            trusted_actor,
            &state.perspective_identities,
        )
        .is_err()
        {
            return rejected(state);
        }

        let mut next_state = state.clone();
        let next_revision = state
            .revision
            .0
            .checked_add(1)
            .ok_or(KernelExecutionError::RevisionOverflow)?;
        let event_id = state.allocators.next_rule_event_id;
        let next_event_id = event_id
            .0
            .checked_add(1)
            .ok_or(KernelExecutionError::RuleEventIdOverflow)?;

        next_state.revision = StateRevision(next_revision);
        next_state.execution.pending_decision = None;
        next_state.allocators.next_rule_event_id = RuleEventId(next_event_id);

        let events = vec![AuthoritativeRuleEvent {
            event_id,
            state_revision: next_state.revision,
            event: AuthoritativeRuleEventKind::DecisionCleared {
                decision: request.decision_id,
            },
        }];
        let audit = events
            .iter()
            .map(|event| event.event.semantic_delta())
            .collect();
        let delta =
            StateDelta::between(state, &next_state, audit).map_err(KernelExecutionError::Delta)?;
        let result = TransitionResult {
            accepted: true,
            next_state,
            delta,
            events,
            next_decision: None,
            status: EpisodeStatus::Running,
        };

        validate_engine_state(&result.next_state).map_err(KernelExecutionError::AfterState)?;
        validate_transition_contract(state, &result)
            .map_err(KernelExecutionError::TransitionContract)?;
        Ok(result)
    }
}

fn rejected(state: &EngineState) -> Result<TransitionResult, KernelExecutionError> {
    let result = TransitionResult {
        accepted: false,
        next_state: state.clone(),
        delta: StateDelta::between(state, state, vec![]).map_err(KernelExecutionError::Delta)?,
        events: vec![],
        next_decision: state
            .execution
            .pending_decision
            .as_ref()
            .map(|record| record.request.clone()),
        status: EpisodeStatus::Running,
    };
    validate_transition_contract(state, &result)
        .map_err(KernelExecutionError::TransitionContract)?;
    Ok(result)
}
