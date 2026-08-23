use crate::events::AuthoritativeRuleEventKind;
use mtgml_model::{EpisodeStatus, PlayerId};
use mtgml_state::PerspectiveIdentityRecordV2;
use mtgml_state::{validate_engine_state, EngineState, ZoneTransition};
use std::collections::BTreeMap;
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

    let transitions: Vec<ZoneTransition> = result
        .events
        .iter()
        .filter_map(|event| match &event.event {
            AuthoritativeRuleEventKind::ZoneTransition { transition } => {
                Some((**transition).clone())
            }
            _ => None,
        })
        .collect();
    let event_audit: Vec<_> = result
        .events
        .iter()
        .map(|event| event.event.semantic_delta())
        .collect();
    if event_audit != result.delta.audit {
        return Err(TransitionViolation::EventDeltaMismatch);
    }

    let mut running_identities: BTreeMap<PlayerId, PerspectiveIdentityRecordV2> =
        before.perspective_identities.players.clone();
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
            crate::events::validate_occurrence_pairing(lifecycle, observation, &transitions)
                .map_err(|_| TransitionViolation::OccurrencePairing)?;
            // Causal knowledge binding against the SEQUENTIAL identity
            // snapshot: old-side references resolve pre-occurrence, new-side
            // post-occurrence.
            let bound = match observation {
                crate::events::PerspectiveObservationPolicyV1::MovedInSight {
                    from_zone,
                    to_zone,
                    old_object,
                    new_object,
                    ..
                } => transitions.iter().find(|transition| {
                    transition.old_object == *old_object
                        && transition.new_object == *new_object
                        && transition.from.zone == *from_zone
                        && transition.to.zone == *to_zone
                }),
                crate::events::PerspectiveObservationPolicyV1::Appeared {
                    from_zone,
                    to_zone,
                    new_object,
                } => transitions.iter().find(|transition| {
                    transition.new_object == *new_object
                        && transition.from.zone == *from_zone
                        && transition.to.zone == *to_zone
                }),
                _ => None,
            };
            // The sequential identity snapshot ALWAYS advances for every
            // perspective occurrence, even when no physical observation
            // exists (NoEnvelope with Allocate/Remap/Retire).
            let (record_pre, record_post) = {
                let _ = lifecycle;
                let pre = running_identities.get(&lifecycle.perspective).cloned();
                let mut post = pre.clone().unwrap_or_default();
                mtgml_state::advance_identity_record(&mut post, &lifecycle.mutation.identity);
                running_identities.insert(lifecycle.perspective, post.clone());
                (pre, post)
            };
            if let Some(transition) = bound {
                use mtgml_state::KnowledgeMutationV1;
                let resolves = |record: Option<PerspectiveIdentityRecordV2>,
                                opaque: &mtgml_model::OpaqueObjectId,
                                object: &mtgml_model::GameObjectId| {
                    record
                        .map(|record| record.opaque_to_object.get(opaque) == Some(object))
                        .unwrap_or(false)
                };
                if let Some(knowledge) = &lifecycle.mutation.knowledge {
                    let causally_bound = match knowledge {
                        KnowledgeMutationV1::CurrentToHistory { opaque, .. } => {
                            resolves(record_pre, opaque, &transition.old_object)
                        }
                        KnowledgeMutationV1::UpdateLocation { opaque, fact } => {
                            resolves(Some(record_post.clone()), opaque, &transition.new_object)
                                && fact.location == transition.to
                        }
                        KnowledgeMutationV1::Invalidate { opaque, .. } => {
                            resolves(record_pre, opaque, &transition.old_object)
                                || resolves(
                                    Some(record_post.clone()),
                                    opaque,
                                    &transition.new_object,
                                )
                        }
                        KnowledgeMutationV1::Acquire { opaque, .. } => {
                            resolves(Some(record_post.clone()), opaque, &transition.new_object)
                        }
                    };
                    if !causally_bound {
                        return Err(TransitionViolation::OccurrencePairing);
                    }
                }
                running_identities.insert(lifecycle.perspective, record_post);
            }
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
