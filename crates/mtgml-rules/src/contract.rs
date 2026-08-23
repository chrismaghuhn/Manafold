use mtgml_model::EpisodeStatus;
use mtgml_state::{validate_engine_state, EngineState};
use std::collections::BTreeSet;
use std::convert::TryFrom;

use crate::semantic_cursor::SemanticValidationCursor;
use crate::transition::TransitionResult;
use crate::validation::TransitionViolation;

pub fn validate_transition_contract(
    before: &EngineState,
    result: &TransitionResult,
) -> Result<(), TransitionViolation> {
    validate_engine_state(before).map_err(TransitionViolation::BeforeState)?;
    validate_engine_state(&result.next_state).map_err(TransitionViolation::AfterState)?;

    let reapplied = result
        .delta
        .apply(before)
        .map_err(|_| TransitionViolation::DeltaReapplication)?;
    if reapplied != result.next_state {
        return Err(TransitionViolation::DeltaReapplication);
    }

    if !result.accepted {
        if &result.next_state != before
            || !result.events.is_empty()
            || !result.delta.audit.is_empty()
            || result.delta.before_revision != result.delta.after_revision
            || result.delta.before_digest != result.delta.after_digest
            || !matches!(&result.status, EpisodeStatus::Running)
        {
            return Err(TransitionViolation::RejectedMutation);
        }
    } else if result.next_state.revision.0 <= before.revision.0 {
        return Err(TransitionViolation::RevisionDidNotAdvance);
    } else {
        let before_decision = before
            .execution
            .pending_decision
            .as_ref()
            .map(|record| record.request.decision_id);
        let after_decision = result
            .next_state
            .execution
            .pending_decision
            .as_ref()
            .map(|record| record.request.decision_id);
        if before_decision.is_some() && before_decision == after_decision {
            return Err(TransitionViolation::DecisionIdentityReused);
        }
    }

    let event_audit: Vec<_> = result
        .events
        .iter()
        .map(|event| event.event.semantic_delta())
        .collect();
    if event_audit != result.delta.audit {
        return Err(TransitionViolation::EventDeltaMismatch);
    }

    let mut seen = BTreeSet::new();
    let mut cursor = SemanticValidationCursor::from_state(before)?;
    for (offset, event) in result.events.iter().enumerate() {
        let offset = u64::try_from(offset).map_err(|_| TransitionViolation::EventIdentity)?;
        let expected = before
            .allocators
            .next_rule_event_id
            .0
            .checked_add(offset)
            .ok_or(TransitionViolation::EventIdentity)?;
        if event.event_id.0 != expected
            || event.state_revision != result.next_state.revision
            || !seen.insert(event.event_id)
        {
            return Err(TransitionViolation::EventIdentity);
        }
        if let crate::events::AuthoritativeRuleEventKind::PerspectiveOccurrence {
            lifecycle,
            observation,
        } = &event.event
        {
            crate::events::validate_occurrence_pairing(lifecycle, observation)
                .map_err(|_| TransitionViolation::OccurrencePairing)?;
        }
        cursor.apply(&event.event)?;
    }
    cursor.validate_final_state(&result.next_state)?;

    let event_count =
        u64::try_from(result.events.len()).map_err(|_| TransitionViolation::EventIdentity)?;
    let expected_next_event = before
        .allocators
        .next_rule_event_id
        .0
        .checked_add(event_count)
        .ok_or(TransitionViolation::EventIdentity)?;
    if result.next_state.allocators.next_rule_event_id.0 != expected_next_event {
        return Err(TransitionViolation::EventIdentity);
    }

    let pending = result
        .next_state
        .execution
        .pending_decision
        .as_ref()
        .map(|record| &record.request);
    if pending != result.next_decision.as_ref() {
        return Err(TransitionViolation::NextDecisionMismatch);
    }
    if let Some(decision) = &result.next_decision {
        if decision.state_revision != result.next_state.revision {
            return Err(TransitionViolation::NextDecisionMismatch);
        }
    }
    if !matches!(&result.status, EpisodeStatus::Running) && result.next_decision.is_some() {
        return Err(TransitionViolation::TerminalDecision);
    }
    result
        .status
        .validate()
        .map_err(|_| TransitionViolation::EpisodeStatus)?;
    Ok(())
}
