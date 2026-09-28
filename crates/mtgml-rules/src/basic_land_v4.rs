//! Rules-owned V4 admission for the already accepted bounded Basic Land slice.
//!
//! This is the G0j bridge only. It projects the existing Basic Land rules
//! candidate owner into the frozen V4 request vocabulary; it adds no actions.

use mtgml_card_ir::ExecutableProfileAdmissionV1;
use mtgml_decision::{
    AuthoritativeCandidateV3, AuthoritativeCandidateV4, AuthoritativeDecisionRequestV3,
    AuthoritativeDecisionRequestV4, CandidateIntentV3, CandidateIntentV4, DecisionAnswerV2,
    DecisionDomainV2, DecisionPurposeV4, DecisionResponseV2, DecisionResponseV3,
    DecisionVisibility, EngineCandidateBindingV3, EngineCandidateBindingV4,
    DECISION_RESPONSE_V2_SCHEMA,
};
use mtgml_model::{DecisionId, EpisodeStatus, PlayerDecisionIdV1, PlayerId, RuleEventId};
use mtgml_state::{
    EngineStatePartsV2, EngineStatePartsV3, ExecutionStateV3, SemanticDeltaOperationV2,
    SemanticDeltaOperationV3, StateDeltaV3,
};

use crate::BasicLandCandidateError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasicLandTransitionProductV4 {
    pub accepted: bool,
    pub next_state: EngineStatePartsV3,
    pub delta: StateDeltaV3,
    pub events: Vec<crate::AuthoritativeRuleEventV3>,
    pub next_decision: Option<AuthoritativeDecisionRequestV4>,
    pub status: EpisodeStatus,
}

fn candidate_state_v2(
    state: &EngineStatePartsV3,
) -> Result<EngineStatePartsV2, BasicLandCandidateError> {
    let mut state = state.clone();
    state.execution_v4.pending_decision = None;
    state
        .validate_structure()
        .map_err(|_| BasicLandCandidateError::InvalidState)?;
    if !state.execution_v4.continuations.is_empty()
        || !state.execution_v4.effects.is_empty()
        || !state.execution_v4.waiting_triggers.is_empty()
        || !state.execution_v4.delayed_effects.is_empty()
        || state
            .predecessor_v5
            .zones
            .stack_records
            .values()
            .any(|record| record.payload.is_some())
    {
        return Err(BasicLandCandidateError::InvalidState);
    }
    let state = EngineStatePartsV2 {
        predecessor_v5: state.predecessor_v5.clone(),
        execution_v3: ExecutionStateV3::default(),
        card_rules_state: state.card_rules_state.clone(),
    };
    state
        .validate()
        .map_err(|_| BasicLandCandidateError::InvalidState)?;
    Ok(state)
}

pub fn derive_basic_land_candidates_v4(
    admission: &ExecutableProfileAdmissionV1,
    state: &EngineStatePartsV3,
    actor: PlayerId,
    status: &EpisodeStatus,
) -> Result<Vec<AuthoritativeCandidateV4>, BasicLandCandidateError> {
    let candidate_state = candidate_state_v2(state)?;
    let candidates =
        crate::derive_basic_land_candidates(admission, &candidate_state, actor, status)?;
    candidates
        .into_iter()
        .map(|candidate| {
            let visible_intent = match candidate.visible_intent {
                CandidateIntentV3::PassPriority => CandidateIntentV4::PassPriority,
                CandidateIntentV3::PlayLand { object } => CandidateIntentV4::PlayLand { object },
                CandidateIntentV3::ActivateAbility { ability } => {
                    CandidateIntentV4::ActivateAbility { ability }
                }
                _ => return Err(BasicLandCandidateError::CandidateOrdering),
            };
            let trusted_binding = match candidate.trusted_binding {
                EngineCandidateBindingV3::PassPriority => EngineCandidateBindingV4::PassPriority,
                EngineCandidateBindingV3::PlayLand { object } => {
                    EngineCandidateBindingV4::PlayLand { object }
                }
                EngineCandidateBindingV3::ActivateAbility { ability } => {
                    EngineCandidateBindingV4::ActivateAbility { ability }
                }
                _ => return Err(BasicLandCandidateError::CandidateOrdering),
            };
            Ok(AuthoritativeCandidateV4 {
                candidate_id: candidate.candidate_id,
                visible_intent,
                trusted_binding,
            })
        })
        .collect()
}

pub fn validate_basic_land_pending_request_v4(
    admission: &ExecutableProfileAdmissionV1,
    state: &EngineStatePartsV3,
    status: &EpisodeStatus,
) -> Result<(), BasicLandCandidateError> {
    state
        .validate_structure()
        .map_err(|_| BasicLandCandidateError::InvalidState)?;
    let Some(request) = state.execution_v4.pending_decision.as_ref() else {
        return match status {
            EpisodeStatus::Terminal { .. } | EpisodeStatus::Truncated { .. } => Ok(()),
            EpisodeStatus::Running => Err(BasicLandCandidateError::PendingCandidateSetMismatch),
        };
    };
    request
        .project_player_request()
        .map_err(|_| BasicLandCandidateError::PendingCandidateSetMismatch)?;
    let expected = derive_basic_land_candidates_v4(admission, state, request.actor, status)?;
    let view_sequence = state
        .predecessor_v5
        .knowledge
        .players
        .get(&request.actor)
        .ok_or(BasicLandCandidateError::InvalidState)?
        .next_visible_sequence;
    let identity = state
        .predecessor_v5
        .perspective_identities
        .players
        .get(&request.actor)
        .ok_or(BasicLandCandidateError::InvalidState)?;
    if request.decision_id.0.checked_add(1)
        != Some(state.predecessor_v5.allocators.next_decision_id.0)
        || request.player_decision_id.0.checked_add(1) != Some(identity.next_player_decision_id.0)
        || request.state_revision != state.predecessor_v5.revision
        || request.view_sequence != view_sequence
        || request.visibility != DecisionVisibility::Public
        || request.decision_domain_v2 != DecisionDomainV2::ChooseOne
        || request.purpose != DecisionPurposeV4::PriorityAction
        || request.parent_player_decision_id.is_some()
        || request.continuation_id.is_some()
        || request.candidates != expected
        || !matches!(status, EpisodeStatus::Running)
    {
        return Err(BasicLandCandidateError::PendingCandidateSetMismatch);
    }
    Ok(())
}

pub fn install_basic_land_request_v4(
    admission: &ExecutableProfileAdmissionV1,
    state: &mut EngineStatePartsV3,
    actor: PlayerId,
    status: &EpisodeStatus,
) -> Result<AuthoritativeDecisionRequestV4, BasicLandCandidateError> {
    if state.execution_v4.pending_decision.is_some() {
        return Err(BasicLandCandidateError::InvalidState);
    }
    let candidates = derive_basic_land_candidates_v4(admission, state, actor, status)?;
    if candidates.is_empty() {
        return Err(BasicLandCandidateError::NoCandidates);
    }
    let view_sequence = state
        .predecessor_v5
        .knowledge
        .players
        .get(&actor)
        .ok_or(BasicLandCandidateError::InvalidState)?
        .next_visible_sequence;
    let identity = state
        .predecessor_v5
        .perspective_identities
        .players
        .get(&actor)
        .ok_or(BasicLandCandidateError::InvalidState)?;
    let decision_id = state.predecessor_v5.allocators.next_decision_id;
    let player_decision_id = identity.next_player_decision_id;
    let next_decision_id = DecisionId(
        decision_id
            .0
            .checked_add(1)
            .ok_or(BasicLandCandidateError::IdentityExhausted)?,
    );
    let next_player_decision_id = PlayerDecisionIdV1(
        player_decision_id
            .0
            .checked_add(1)
            .ok_or(BasicLandCandidateError::IdentityExhausted)?,
    );
    let request = AuthoritativeDecisionRequestV4 {
        decision_id,
        player_decision_id,
        state_revision: state.predecessor_v5.revision,
        view_sequence,
        actor,
        visibility: DecisionVisibility::Public,
        decision_domain_v2: DecisionDomainV2::ChooseOne,
        purpose: DecisionPurposeV4::PriorityAction,
        parent_player_decision_id: None,
        continuation_id: None,
        candidates,
    };
    request
        .project_player_request()
        .map_err(|_| BasicLandCandidateError::CandidateOrdering)?;

    let mut candidate_state = state.clone();
    candidate_state.predecessor_v5.allocators.next_decision_id = next_decision_id;
    candidate_state
        .predecessor_v5
        .perspective_identities
        .players
        .get_mut(&actor)
        .ok_or(BasicLandCandidateError::InvalidState)?
        .next_player_decision_id = next_player_decision_id;
    candidate_state.execution_v4.pending_decision = Some(request.clone());
    candidate_state
        .validate_structure()
        .map_err(|_| BasicLandCandidateError::InvalidState)?;
    *state = candidate_state;
    Ok(request)
}

pub fn selected_basic_land_action_v4(
    admission: &ExecutableProfileAdmissionV1,
    state: &EngineStatePartsV3,
    actor: PlayerId,
    response: &DecisionResponseV3,
    status: &EpisodeStatus,
) -> Result<crate::SelectedSuccessorDecisionV1, BasicLandCandidateError> {
    validate_basic_land_pending_request_v4(admission, state, status)?;
    let request = state
        .execution_v4
        .pending_decision
        .as_ref()
        .ok_or(BasicLandCandidateError::InvalidState)?;
    if request.actor != actor {
        return Err(BasicLandCandidateError::InvalidState);
    }
    request
        .validate_response(response)
        .map_err(|_| BasicLandCandidateError::InvalidState)?;
    let selected_ids = match &response.answer {
        DecisionAnswerV2::SelectOne { candidate_id } => vec![*candidate_id],
        DecisionAnswerV2::SelectMany { candidate_ids }
        | DecisionAnswerV2::Order { candidate_ids } => candidate_ids.clone(),
        DecisionAnswerV2::ChooseNumber { .. } => Vec::new(),
    };
    let selected = selected_ids
        .iter()
        .map(|candidate_id| {
            request
                .candidates
                .iter()
                .find(|candidate| candidate.candidate_id == *candidate_id)
                .map(|candidate| &candidate.trusted_binding)
                .ok_or(BasicLandCandidateError::InvalidState)
        })
        .collect::<Result<Vec<_>, _>>()?;
    if selected.len() != 1 {
        return Err(BasicLandCandidateError::UnsupportedSelectedAction);
    }
    match selected[0] {
        EngineCandidateBindingV4::PassPriority => {
            Ok(crate::SelectedSuccessorDecisionV1::PassPriority)
        }
        EngineCandidateBindingV4::PlayLand { object } => {
            Ok(crate::SelectedSuccessorDecisionV1::MagicAction(
                crate::MagicActionRequestV1::PlayLand {
                    actor,
                    object: *object,
                },
            ))
        }
        EngineCandidateBindingV4::ActivateAbility { ability } => {
            Ok(crate::SelectedSuccessorDecisionV1::MagicAction(
                crate::MagicActionRequestV1::ActivateManaAbility {
                    actor,
                    ability: *ability,
                },
            ))
        }
        _ => Err(BasicLandCandidateError::UnsupportedSelectedAction),
    }
}

pub fn execute_basic_land_response_v4(
    admission: &ExecutableProfileAdmissionV1,
    state: &EngineStatePartsV3,
    actor: PlayerId,
    response: &DecisionResponseV3,
    status: &EpisodeStatus,
) -> Result<BasicLandTransitionProductV4, crate::BasicLandTransitionError> {
    let request = state
        .execution_v4
        .pending_decision
        .as_ref()
        .ok_or(crate::BasicLandTransitionError::InvalidSelection)?;
    validate_basic_land_pending_request_v4(admission, state, status)
        .map_err(|_| crate::BasicLandTransitionError::InvalidSelection)?;
    request
        .validate_response(response)
        .map_err(|_| crate::BasicLandTransitionError::InvalidSelection)?;

    let mut before_v2 =
        candidate_state_v2(state).map_err(|_| crate::BasicLandTransitionError::InvalidResult)?;
    before_v2.execution_v3.pending_decision = Some(
        request_v3_from_v4(request)
            .map_err(|_| crate::BasicLandTransitionError::InvalidSelection)?,
    );
    before_v2
        .validate()
        .map_err(|_| crate::BasicLandTransitionError::InvalidResult)?;
    let response_v2 = DecisionResponseV2 {
        schema_version: DECISION_RESPONSE_V2_SCHEMA.to_owned(),
        player_decision_id: response.player_decision_id,
        state_revision: request.state_revision,
        answer: response.answer.clone(),
    };
    let legacy = crate::basic_land::execute_basic_land_response(
        admission,
        &before_v2,
        actor,
        &response_v2,
        status,
    )?;
    let next_request = legacy
        .next_decision
        .as_ref()
        .map(|request| request_v4_from_v3(&legacy.next_state, request))
        .transpose()
        .map_err(|_| crate::BasicLandTransitionError::InvalidResult)?;
    let mut next_state = EngineStatePartsV3 {
        predecessor_v5: legacy.next_state.predecessor_v5.clone(),
        execution_v4: mtgml_state::ExecutionStateV4 {
            pending_decision: next_request.clone(),
            ..Default::default()
        },
        card_rules_state: legacy.next_state.card_rules_state.clone(),
    };
    let events = convert_events_v2_to_v3(
        state.predecessor_v5.allocators.next_rule_event_id,
        next_state.predecessor_v5.revision,
        &legacy.events,
        &mut next_state,
    )?;
    let mut operations = convert_operations_v2_to_v3(&legacy.delta.operations, &legacy.events)?;
    operations.push(SemanticDeltaOperationV3::PendingRequestChanged {
        from: Some(Box::new(request.clone())),
        to: next_request.clone().map(Box::new),
    });
    next_state
        .validate_structure()
        .map_err(|_| crate::BasicLandTransitionError::InvalidResult)?;
    let delta = StateDeltaV3::between_after_rules_domain_validation(state, &next_state, operations)
        .map_err(|_| crate::BasicLandTransitionError::Delta)?;
    crate::validate_event_delta_state_v3_after_rules_domain_validation(
        state,
        &next_state,
        &events,
        &delta,
    )
    .map_err(|_| crate::BasicLandTransitionError::InvalidResult)?;
    Ok(BasicLandTransitionProductV4 {
        accepted: legacy.accepted,
        next_state,
        delta,
        events,
        next_decision: next_request,
        status: legacy.status,
    })
}

fn request_v3_from_v4(
    request: &AuthoritativeDecisionRequestV4,
) -> Result<AuthoritativeDecisionRequestV3, BasicLandCandidateError> {
    let candidates = request
        .candidates
        .iter()
        .map(|candidate| {
            let visible_intent = match candidate.visible_intent {
                CandidateIntentV4::PassPriority => CandidateIntentV3::PassPriority,
                CandidateIntentV4::PlayLand { object } => CandidateIntentV3::PlayLand { object },
                CandidateIntentV4::ActivateAbility { ability } => {
                    CandidateIntentV3::ActivateAbility { ability }
                }
                _ => return Err(BasicLandCandidateError::CandidateOrdering),
            };
            let trusted_binding = match candidate.trusted_binding {
                EngineCandidateBindingV4::PassPriority => EngineCandidateBindingV3::PassPriority,
                EngineCandidateBindingV4::PlayLand { object } => {
                    EngineCandidateBindingV3::PlayLand { object }
                }
                EngineCandidateBindingV4::ActivateAbility { ability } => {
                    EngineCandidateBindingV3::ActivateAbility { ability }
                }
                _ => return Err(BasicLandCandidateError::CandidateOrdering),
            };
            Ok(AuthoritativeCandidateV3 {
                candidate_id: candidate.candidate_id,
                visible_intent,
                trusted_binding,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(AuthoritativeDecisionRequestV3 {
        decision_id: request.decision_id,
        player_decision_id: request.player_decision_id,
        state_revision: request.state_revision,
        actor: request.actor,
        visibility: request.visibility,
        decision: request.decision_domain_v2.clone(),
        candidates,
        continuation_id: request.continuation_id,
    })
}

fn request_v4_from_v3(
    state: &EngineStatePartsV2,
    request: &AuthoritativeDecisionRequestV3,
) -> Result<AuthoritativeDecisionRequestV4, BasicLandCandidateError> {
    let candidate_state = EngineStatePartsV3 {
        predecessor_v5: state.predecessor_v5.clone(),
        execution_v4: Default::default(),
        card_rules_state: state.card_rules_state.clone(),
    };
    let view_sequence = candidate_state
        .predecessor_v5
        .knowledge
        .players
        .get(&request.actor)
        .ok_or(BasicLandCandidateError::InvalidState)?
        .next_visible_sequence;
    let candidates = request
        .candidates
        .iter()
        .map(|candidate| {
            let visible_intent = match candidate.visible_intent {
                CandidateIntentV3::PassPriority => CandidateIntentV4::PassPriority,
                CandidateIntentV3::PlayLand { object } => CandidateIntentV4::PlayLand { object },
                CandidateIntentV3::ActivateAbility { ability } => {
                    CandidateIntentV4::ActivateAbility { ability }
                }
                _ => return Err(BasicLandCandidateError::CandidateOrdering),
            };
            let trusted_binding = match candidate.trusted_binding {
                EngineCandidateBindingV3::PassPriority => EngineCandidateBindingV4::PassPriority,
                EngineCandidateBindingV3::PlayLand { object } => {
                    EngineCandidateBindingV4::PlayLand { object }
                }
                EngineCandidateBindingV3::ActivateAbility { ability } => {
                    EngineCandidateBindingV4::ActivateAbility { ability }
                }
                _ => return Err(BasicLandCandidateError::CandidateOrdering),
            };
            Ok(AuthoritativeCandidateV4 {
                candidate_id: candidate.candidate_id,
                visible_intent,
                trusted_binding,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let next_player_decision_id = candidate_state
        .predecessor_v5
        .perspective_identities
        .players
        .get(&request.actor)
        .ok_or(BasicLandCandidateError::InvalidState)?
        .next_player_decision_id;
    if next_player_decision_id.0
        != request
            .player_decision_id
            .0
            .checked_add(1)
            .ok_or(BasicLandCandidateError::InvalidState)?
    {
        return Err(BasicLandCandidateError::PendingCandidateSetMismatch);
    }
    let result = AuthoritativeDecisionRequestV4 {
        decision_id: request.decision_id,
        player_decision_id: request.player_decision_id,
        state_revision: request.state_revision,
        view_sequence,
        actor: request.actor,
        visibility: request.visibility,
        decision_domain_v2: request.decision.clone(),
        purpose: DecisionPurposeV4::PriorityAction,
        parent_player_decision_id: None,
        continuation_id: request.continuation_id,
        candidates,
    };
    result
        .project_player_request()
        .map_err(|_| BasicLandCandidateError::CandidateOrdering)?;
    Ok(result)
}

fn convert_events_v2_to_v3(
    first_event_id: RuleEventId,
    revision: mtgml_model::StateRevision,
    source: &[crate::AuthoritativeRuleEventV2],
    after: &mut EngineStatePartsV3,
) -> Result<Vec<crate::AuthoritativeRuleEventV3>, crate::BasicLandTransitionError> {
    use crate::{
        AuthoritativeRuleEventKindV2 as Old, AuthoritativeRuleEventKindV3 as New,
        SuccessorObservationPolicyV1 as OldPolicy,
    };

    let source_order = causal_event_order_v2_to_v3(source)?;
    let mut new_ids = std::collections::BTreeMap::new();
    let mut next_id = first_event_id;
    for source_index in &source_order {
        let record = &source[*source_index];
        new_ids.insert(record.event_id, next_id);
        next_id = RuleEventId(
            next_id
                .0
                .checked_add(1)
                .ok_or(crate::BasicLandTransitionError::IdentityExhausted)?,
        );
    }
    let mut events = Vec::new();
    for source_index in source_order {
        let record = &source[source_index];
        let Some(event_id) = new_ids.get(&record.event_id).copied() else {
            continue;
        };
        let event = match &record.event {
            Old::DecisionCleared { decision } => Some(New::Existing {
                event: Box::new(crate::AuthoritativeRuleEventKind::DecisionCleared {
                    decision: *decision,
                }),
            }),
            Old::DecisionCreated { decision } => Some(New::Existing {
                event: Box::new(crate::AuthoritativeRuleEventKind::DecisionCreated {
                    decision: *decision,
                }),
            }),
            Old::PriorityChanged { from, to } => Some(New::Existing {
                event: Box::new(crate::AuthoritativeRuleEventKind::PriorityChanged {
                    from: *from,
                    to: *to,
                }),
            }),
            Old::TurnPositionChanged { from, to } => Some(New::Existing {
                event: Box::new(crate::AuthoritativeRuleEventKind::TurnPositionChanged {
                    from: *from,
                    to: *to,
                }),
            }),
            Old::ObjectMoved { .. } | Old::LandPlayed { .. } => continue,
            Old::ObjectTapped { object, from, to } => Some(New::Existing {
                event: Box::new(crate::AuthoritativeRuleEventKind::ObjectTapped {
                    object: *object,
                    from: *from,
                    to: *to,
                }),
            }),
            Old::ManaPoolChanged {
                player,
                previous,
                pool_after,
                color,
                amount,
            } => {
                let cause = if color.is_some() && *amount > 0 {
                    mtgml_state::ManaPoolChangeCauseV1::Produced
                } else if color.is_none() && *amount == 0 {
                    mtgml_state::ManaPoolChangeCauseV1::Emptied
                } else {
                    return Err(crate::BasicLandTransitionError::InvalidResult);
                };
                Some(New::ManaPoolChanged {
                    player: *player,
                    before: *previous,
                    after: *pool_after,
                    cause,
                })
            }
            Old::PerspectiveOccurrence {
                lifecycle,
                observation,
            } => match observation {
                OldPolicy::MovedInSight {
                    old_object,
                    new_object,
                    from,
                    to,
                    ..
                } => {
                    let source_event = source
                        .iter()
                        .find(|candidate| {
                            matches!(&candidate.event,
                            Old::ZoneTransition(transition)
                                if transition.old_object == *old_object
                                    && transition.new_object == *new_object
                                    && transition.from.zone == *from
                                    && transition.to.zone == *to)
                        })
                        .ok_or(crate::BasicLandTransitionError::InvalidResult)?;
                    let source_event_id = *new_ids
                        .get(&source_event.event_id)
                        .ok_or(crate::BasicLandTransitionError::InvalidResult)?;
                    Some(New::PerspectiveObservationOccurrence {
                        lifecycle: Box::new(lifecycle.as_ref().clone()),
                        source_event_id,
                    })
                }
                OldPolicy::ObjectTapped { object, .. } => {
                    let source_event = source
                        .iter()
                        .find(|candidate| {
                            matches!(&candidate.event,
                            Old::ObjectTapped { object: event_object, .. }
                                if event_object == object)
                        })
                        .ok_or(crate::BasicLandTransitionError::InvalidResult)?;
                    let source_event_id = *new_ids
                        .get(&source_event.event_id)
                        .ok_or(crate::BasicLandTransitionError::InvalidResult)?;
                    Some(New::PerspectiveObservationOccurrence {
                        lifecycle: Box::new(lifecycle.as_ref().clone()),
                        source_event_id,
                    })
                }
                OldPolicy::ManaPoolChanged { player } => {
                    let source_event = source[..source_index]
                        .iter()
                        .rev()
                        .find(|candidate| {
                            matches!(&candidate.event,
                            Old::ManaPoolChanged { player: event_player, .. }
                                if event_player == player)
                        })
                        .ok_or(crate::BasicLandTransitionError::InvalidResult)?;
                    let source_event_id = *new_ids
                        .get(&source_event.event_id)
                        .ok_or(crate::BasicLandTransitionError::InvalidResult)?;
                    Some(New::PerspectiveObservationOccurrence {
                        lifecycle: Box::new(lifecycle.as_ref().clone()),
                        source_event_id,
                    })
                }
            },
            Old::ZoneTransition(transition) => Some(New::Existing {
                event: Box::new(crate::AuthoritativeRuleEventKind::ZoneTransition {
                    transition: transition.clone(),
                }),
            }),
        };
        events.push(crate::AuthoritativeRuleEventV3 {
            event_id,
            state_revision: revision,
            event: event.ok_or(crate::BasicLandTransitionError::InvalidResult)?,
        });
    }
    after.predecessor_v5.allocators.next_rule_event_id = next_id;
    Ok(events)
}

/// Preserves the source order except where an observation occurrence names a
/// public source event that the legacy M4.2 producer emitted later in the same
/// atomic transition. The V3 contract requires the source event to precede its
/// occurrence, so this stable topological order moves only the required source
/// ahead while retaining the order among otherwise independent events.
fn causal_event_order_v2_to_v3(
    source: &[crate::AuthoritativeRuleEventV2],
) -> Result<Vec<usize>, crate::BasicLandTransitionError> {
    use crate::{AuthoritativeRuleEventKindV2 as Old, SuccessorObservationPolicyV1 as OldPolicy};

    let included = |index: usize| {
        !matches!(
            source[index].event,
            Old::ObjectMoved { .. } | Old::LandPlayed { .. }
        )
    };
    let nodes = (0..source.len())
        .filter(|index| included(*index))
        .collect::<Vec<_>>();
    let mut outgoing = vec![Vec::new(); source.len()];
    let mut indegree = vec![0usize; source.len()];

    for (occurrence_index, record) in source.iter().enumerate() {
        let Old::PerspectiveOccurrence { observation, .. } = &record.event else {
            continue;
        };
        let source_index = match observation {
            OldPolicy::MovedInSight {
                old_object,
                new_object,
                from,
                to,
                ..
            } => source.iter().position(|candidate| {
                matches!(&candidate.event,
                    Old::ZoneTransition(transition)
                        if transition.old_object == *old_object
                            && transition.new_object == *new_object
                            && transition.from.zone == *from
                            && transition.to.zone == *to)
            }),
            OldPolicy::ObjectTapped { object, .. } => source.iter().position(|candidate| {
                matches!(&candidate.event,
                    Old::ObjectTapped { object: event_object, .. }
                        if event_object == object)
            }),
            // The legacy producer already chooses a prior mana event for this
            // occurrence. Preserve and validate that source-order relation.
            OldPolicy::ManaPoolChanged { player } => {
                source[..occurrence_index].iter().rposition(|candidate| {
                    matches!(&candidate.event,
                        Old::ManaPoolChanged { player: event_player, .. }
                            if event_player == player)
                })
            }
        }
        .ok_or(crate::BasicLandTransitionError::InvalidResult)?;
        if source_index == occurrence_index || !included(source_index) {
            return Err(crate::BasicLandTransitionError::InvalidResult);
        }
        outgoing[source_index].push(occurrence_index);
        indegree[occurrence_index] = indegree[occurrence_index]
            .checked_add(1)
            .ok_or(crate::BasicLandTransitionError::InvalidResult)?;
    }

    let mut ready = std::collections::BTreeSet::new();
    for index in &nodes {
        if indegree[*index] == 0 {
            ready.insert(*index);
        }
    }
    let mut ordered = Vec::with_capacity(nodes.len());
    while let Some(index) = ready.pop_first() {
        ordered.push(index);
        for dependent in &outgoing[index] {
            indegree[*dependent] -= 1;
            if indegree[*dependent] == 0 {
                ready.insert(*dependent);
            }
        }
    }
    if ordered.len() != nodes.len() {
        return Err(crate::BasicLandTransitionError::InvalidResult);
    }
    Ok(ordered)
}

fn convert_operations_v2_to_v3(
    operations: &[SemanticDeltaOperationV2],
    events: &[crate::AuthoritativeRuleEventV2],
) -> Result<Vec<SemanticDeltaOperationV3>, crate::BasicLandTransitionError> {
    use crate::AuthoritativeRuleEventKindV2 as Event;
    use mtgml_state::SemanticDeltaOperation;
    use SemanticDeltaOperationV2 as Old;
    use SemanticDeltaOperationV3 as New;

    let mut converted = Vec::new();
    let mut used_mana_events = std::collections::BTreeSet::new();
    for operation in operations {
        match operation {
            Old::ManaAdded {
                player,
                color,
                amount,
                ..
            } => {
                let (index, event) = events
                    .iter()
                    .enumerate()
                    .find(|(index, event)| {
                        !used_mana_events.contains(index)
                            && matches!(&event.event, Event::ManaPoolChanged {
                                player: event_player,
                                color: Some(event_color),
                                amount: event_amount,
                                ..
                            } if event_player == player && event_color == color && event_amount == amount)
                    })
                    .ok_or(crate::BasicLandTransitionError::InvalidResult)?;
                let Event::ManaPoolChanged {
                    previous,
                    pool_after,
                    ..
                } = &event.event
                else {
                    return Err(crate::BasicLandTransitionError::InvalidResult);
                };
                used_mana_events.insert(index);
                converted.push(New::ManaPoolChanged {
                    player: *player,
                    from: *previous,
                    to: *pool_after,
                    cause: mtgml_state::ManaPoolChangeCauseV1::Produced,
                });
            }
            Old::ManaPoolEmptied {
                player,
                previous_pool,
            } => {
                let (index, _) = events
                    .iter()
                    .enumerate()
                    .find(|(index, event)| {
                        !used_mana_events.contains(index)
                            && matches!(&event.event, Event::ManaPoolChanged {
                                player: event_player,
                                previous,
                                color: None,
                                amount: 0,
                                ..
                            } if event_player == player && previous == previous_pool)
                    })
                    .ok_or(crate::BasicLandTransitionError::InvalidResult)?;
                used_mana_events.insert(index);
                converted.push(New::ManaPoolChanged {
                    player: *player,
                    from: *previous_pool,
                    to: Default::default(),
                    cause: mtgml_state::ManaPoolChangeCauseV1::Emptied,
                });
            }
            Old::ObjectTapped { object, from, to } => {
                converted.push(New::Existing {
                    operation: Box::new(Old::Existing {
                        operation: Box::new(SemanticDeltaOperation::ObjectTapped {
                            object: *object,
                            from: *from,
                            to: *to,
                        }),
                    }),
                });
            }
            other => converted.push(New::Existing {
                operation: Box::new(other.clone()),
            }),
        }
    }
    Ok(converted)
}
