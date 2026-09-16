use crate::events::AuthoritativeRuleEventKind;
use mtgml_model::{EpisodeStatus, PlayerId};
use mtgml_state::PerspectiveIdentityRecordV2;
use mtgml_state::{validate_engine_state, EngineState};
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::convert::TryFrom;

use crate::semantic_cursor::SemanticValidationCursor;
use crate::transition::TransitionResult;
use crate::validation::TransitionViolation;

fn validate_accepted_progression(
    before: &EngineState,
    result: &TransitionResult,
) -> Result<(), TransitionViolation> {
    let after = &result.next_state;

    if after.allocators.next_object_id.0 < before.allocators.next_object_id.0
        || after.allocators.next_ability_id.0 < before.allocators.next_ability_id.0
        || after.allocators.next_stack_object_id.0 < before.allocators.next_stack_object_id.0
        || after.allocators.next_effect_id.0 < before.allocators.next_effect_id.0
        || after.allocators.next_trigger_id.0 < before.allocators.next_trigger_id.0
        || after.allocators.next_continuation_id.0 < before.allocators.next_continuation_id.0
        || after.allocators.next_rule_event_id.0 < before.allocators.next_rule_event_id.0
    {
        return Err(TransitionViolation::AllocatorProgression);
    }

    let before_request = before
        .execution
        .pending_decision
        .as_ref()
        .map(|record| &record.request);
    let after_request = after
        .execution
        .pending_decision
        .as_ref()
        .map(|record| &record.request);
    if let (Some(before_request), Some(after_request)) = (before_request, after_request) {
        if before_request.continuation_id.is_some()
            && after_request.continuation_id.is_some()
            && before_request.continuation_id != after_request.continuation_id
        {
            return Err(TransitionViolation::ContinuationIdentity);
        }
    }

    let decision_created = match (before_request, after_request) {
        (None, Some(_)) => true,
        (Some(before), Some(after)) => before.decision_id != after.decision_id,
        _ => false,
    };
    let expected_next_decision = before
        .allocators
        .next_decision_id
        .0
        .checked_add(u64::from(decision_created))
        .ok_or(TransitionViolation::DecisionProgression)?;
    if after.allocators.next_decision_id.0 != expected_next_decision {
        return Err(TransitionViolation::DecisionProgression);
    }

    let actor = after_request
        .filter(|_| decision_created)
        .map(|request| request.actor);
    for (player, before_identity) in &before.perspective_identities.players {
        let after_identity = after
            .perspective_identities
            .players
            .get(player)
            .ok_or(TransitionViolation::DecisionProgression)?;
        let expected = if Some(*player) == actor {
            before_identity
                .next_player_decision_id
                .0
                .checked_add(1)
                .ok_or(TransitionViolation::DecisionProgression)?
        } else {
            before_identity.next_player_decision_id.0
        };
        if after_identity.next_player_decision_id.0 != expected {
            return Err(TransitionViolation::DecisionProgression);
        }
    }
    if let Some(request) = after_request.filter(|_| decision_created) {
        let before_identity = before
            .perspective_identities
            .players
            .get(&request.actor)
            .ok_or(TransitionViolation::DecisionProgression)?;
        if request.decision_id.0 != before.allocators.next_decision_id.0
            || request.player_decision_id.0 != before_identity.next_player_decision_id.0
        {
            return Err(TransitionViolation::DecisionProgression);
        }
    }

    // M2/P0 have no event families for these core semantic fields. Fail
    // closed until a reviewed current contract defines their event/cursor
    // proof. Compare complete values, not only map presence or keys.
    let has_lost_changed = before.core.players.iter().any(|(player, state)| {
        after
            .core
            .players
            .get(player)
            .is_none_or(|other| other.has_lost != state.has_lost)
    });
    if before.core.active_player != after.core.active_player
        || before.core.turn_number != after.core.turn_number
        || before.core.priority != after.core.priority
        || before.core.position != after.core.position
        || before.core.players.len() != after.core.players.len()
        || has_lost_changed
        || before.combat != after.combat
        || before.foundation_sources != after.foundation_sources
    {
        return Err(TransitionViolation::UnexplainedMutation);
    }
    Ok(())
}

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
    } else if result.next_state.revision.0
        != before
            .revision
            .0
            .checked_add(1)
            .ok_or(TransitionViolation::RevisionDidNotAdvance)?
    {
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

    if result.accepted {
        validate_accepted_progression(before, result)?;
    }

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
    let mut seen_transitions = Vec::new();
    let mut previous_random_sample = None;
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
            crate::events::validate_occurrence_pairing(lifecycle, observation, &seen_transitions)
                .map_err(|_| TransitionViolation::OccurrencePairing)?;
            if let crate::events::PerspectiveObservationPolicyV1::SawRandomOutcome {
                exclusive_upper_bound,
                value,
                ..
            } = observation
            {
                if previous_random_sample != Some((*exclusive_upper_bound, *value)) {
                    return Err(TransitionViolation::OccurrencePairing);
                }
            }
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
                } => seen_transitions.iter().find(|transition| {
                    transition.old_object == *old_object
                        && transition.new_object == *new_object
                        && transition.from.zone == *from_zone
                        && transition.to.zone == *to_zone
                }),
                crate::events::PerspectiveObservationPolicyV1::Appeared {
                    from_zone,
                    to_zone,
                    new_object,
                } => seen_transitions.iter().find(|transition| {
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
        previous_random_sample = match &event.event {
            AuthoritativeRuleEventKind::RandomValueSampled { bound, value, .. } => {
                Some((*bound, *value))
            }
            _ => None,
        };
        if let AuthoritativeRuleEventKind::ZoneTransition { transition } = &event.event {
            seen_transitions.push((**transition).clone());
        }
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
