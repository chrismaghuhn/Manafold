use crate::events::AuthoritativeRuleEventKind;
use mtgml_model::{EpisodeStatus, GameObjectId, PlayerId, ZoneKind};
use mtgml_state::{
    validate_engine_state, BeginningStep, EndingStep, EngineState, GameObject, IdentityMutationV1,
    KnowledgeAcquisitionCause, KnowledgeAcquisitionReason, KnowledgeHistoryChannel,
    KnowledgeMutationV1, KnownLocationFactV2, PerspectiveIdentityRecordV2,
    PerspectiveLifecycleAuditV1, PerspectiveLifecycleMutationV1, SbaSelectedActionV1, TurnPosition,
    VisibilityPartition, ZoneLocation, ZonePosition, ZoneTransition,
};
use mtgml_state::{ContinuationPayloadV2, ContinuationRecordV2, SbaGraveyardOwnerOrderV1};
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::convert::TryFrom;

use crate::semantic_cursor::SemanticValidationCursor;
use crate::transition::TransitionResult;
use crate::turn_structure::validate_quiescent_cleanup_boundary;
use crate::validation::TransitionViolation;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectedZoneFamily {
    BattlefieldToGraveyard,
    LibraryToHand,
}

fn selected_zone_family(transition: &ZoneTransition) -> Option<SelectedZoneFamily> {
    match (transition.from.zone, transition.to.zone) {
        (ZoneKind::Battlefield, ZoneKind::Graveyard) => {
            Some(SelectedZoneFamily::BattlefieldToGraveyard)
        }
        (ZoneKind::Library, ZoneKind::Hand) => Some(SelectedZoneFamily::LibraryToHand),
        _ => None,
    }
}

fn expected_s2_occurrences(
    before: &EngineState,
    transition: &ZoneTransition,
    family: SelectedZoneFamily,
) -> Result<
    Vec<(
        PerspectiveLifecycleAuditV1,
        crate::events::PerspectiveObservationPolicyV1,
    )>,
    TransitionViolation,
> {
    use crate::events::PerspectiveObservationPolicyV1 as Observation;
    let old = transition.old_object;
    let new = transition.new_object;
    let owner = transition.last_known.owner;
    let mut expected = Vec::new();

    match family {
        SelectedZoneFamily::BattlefieldToGraveyard => {
            for perspective in before.core.players.keys().copied() {
                let identity = before
                    .perspective_identities
                    .players
                    .get(&perspective)
                    .ok_or(TransitionViolation::OccurrencePairing)?;
                let Some(opaque) = identity.object_to_opaque.get(&old).copied() else {
                    continue;
                };
                let knowledge = before
                    .knowledge
                    .players
                    .get(&perspective)
                    .ok_or(TransitionViolation::OccurrencePairing)?;
                if !knowledge.active.contains_key(&opaque) {
                    return Err(TransitionViolation::OccurrencePairing);
                }
                let sequence = knowledge.next_visible_sequence;
                let provenance = KnowledgeAcquisitionReason::Observed {
                    channel: KnowledgeHistoryChannel::Public,
                    sequence,
                    cause: KnowledgeAcquisitionCause::PublicEvent,
                };
                expected.push((
                    PerspectiveLifecycleAuditV1 {
                        perspective,
                        sequence,
                        mutation: PerspectiveLifecycleMutationV1 {
                            identity: IdentityMutationV1::Remap {
                                opaque,
                                from_object: old,
                                to_object: new,
                            },
                            knowledge: Some(KnowledgeMutationV1::UpdateLocation {
                                opaque,
                                fact: KnownLocationFactV2 {
                                    location: transition.to.clone(),
                                    provenance,
                                },
                            }),
                        },
                    },
                    Observation::MovedInSight {
                        from_zone: ZoneKind::Battlefield,
                        to_zone: ZoneKind::Graveyard,
                        old_object: old,
                        new_object: new,
                        reveals_old: true,
                        reveals_new: true,
                    },
                ));
            }
        }
        SelectedZoneFamily::LibraryToHand => {
            for (perspective, identity) in &before.perspective_identities.players {
                if *perspective != owner && identity.object_to_opaque.contains_key(&old) {
                    return Err(TransitionViolation::OldReferenceClosure);
                }
            }
            let identity = before
                .perspective_identities
                .players
                .get(&owner)
                .ok_or(TransitionViolation::OccurrencePairing)?;
            let knowledge = before
                .knowledge
                .players
                .get(&owner)
                .ok_or(TransitionViolation::OccurrencePairing)?;
            let sequence = knowledge.next_visible_sequence;
            let (identity_mutation, knowledge_mutation, observation) =
                if let Some(opaque) = identity.object_to_opaque.get(&old).copied() {
                    let record = knowledge
                        .active
                        .get(&opaque)
                        .ok_or(TransitionViolation::OccurrencePairing)?;
                    if record.card_definition != Some(transition.last_known.card_definition) {
                        return Err(TransitionViolation::OccurrencePairing);
                    }
                    let provenance = KnowledgeAcquisitionReason::Observed {
                        channel: KnowledgeHistoryChannel::Private,
                        sequence,
                        cause: KnowledgeAcquisitionCause::OwnPrivateIdentity,
                    };
                    (
                        IdentityMutationV1::Remap {
                            opaque,
                            from_object: old,
                            to_object: new,
                        },
                        Some(KnowledgeMutationV1::UpdateLocation {
                            opaque,
                            fact: KnownLocationFactV2 {
                                location: transition.to.clone(),
                                provenance,
                            },
                        }),
                        Observation::MovedInSight {
                            from_zone: ZoneKind::Library,
                            to_zone: ZoneKind::Hand,
                            old_object: old,
                            new_object: new,
                            reveals_old: true,
                            reveals_new: true,
                        },
                    )
                } else {
                    let opaque = identity.next_opaque_object_id;
                    let acquisition = KnowledgeAcquisitionReason::Observed {
                        channel: KnowledgeHistoryChannel::Private,
                        sequence,
                        cause: KnowledgeAcquisitionCause::OwnPrivateIdentity,
                    };
                    (
                        IdentityMutationV1::Allocate {
                            opaque,
                            object: new,
                        },
                        Some(KnowledgeMutationV1::Acquire {
                            opaque,
                            definition: Some(transition.last_known.card_definition),
                            location: Some(transition.to.clone()),
                            acquisition,
                        }),
                        Observation::NoEnvelope,
                    )
                };
            expected.push((
                PerspectiveLifecycleAuditV1 {
                    perspective: owner,
                    sequence,
                    mutation: PerspectiveLifecycleMutationV1 {
                        identity: identity_mutation,
                        knowledge: knowledge_mutation,
                    },
                },
                observation,
            ));
        }
    }
    Ok(expected)
}

fn object_references_old(state: &EngineState, object: GameObjectId) -> bool {
    state.combat.as_ref().is_some_and(|combat| {
        combat.attackers.contains(&object)
            || combat.blockers.contains_key(&object)
            || combat
                .blockers
                .values()
                .any(|blocker| *blocker == Some(object))
    }) || state
        .zones
        .stack_records
        .values()
        .any(|record| record.source_object == Some(object))
        || state
            .execution
            .pending_decision
            .as_ref()
            .is_some_and(|pending| {
                pending.request.candidates.iter().any(|candidate| {
                    matches!(
                        candidate.trusted_binding,
                        mtgml_decision::EngineCandidateBinding::CastSpell { object: bound }
                            | mtgml_decision::EngineCandidateBinding::SelectObject { object: bound }
                            if bound == object
                    )
                })
            })
}

fn validate_s2_zone_transition_product(
    before: &EngineState,
    result: &TransitionResult,
) -> Result<(), TransitionViolation> {
    if result.events.iter().any(|event| {
        matches!(
            event.event,
            AuthoritativeRuleEventKind::StateBasedActionsApplied { .. }
        )
    }) {
        return validate_sba_batch_product(before, result);
    }
    let transitions: Vec<_> = result
        .events
        .iter()
        .filter_map(|event| match &event.event {
            AuthoritativeRuleEventKind::ZoneTransition { transition }
                if selected_zone_family(transition).is_some() =>
            {
                Some(transition.as_ref())
            }
            _ => None,
        })
        .collect();
    if transitions.is_empty() {
        return Ok(());
    }
    let transition = *transitions
        .first()
        .ok_or(TransitionViolation::ZoneTransition)?;
    let family = selected_zone_family(transition).ok_or(TransitionViolation::ZoneTransition)?;
    if transitions.len() != 1
        || result
            .events
            .iter()
            .filter(|event| {
                matches!(
                    event.event,
                    AuthoritativeRuleEventKind::ZoneTransition { .. }
                )
            })
            .count()
            != 1
        || !matches!(
            result.events.first().map(|event| &event.event),
            Some(AuthoritativeRuleEventKind::ZoneTransition { .. })
        )
    {
        return Err(TransitionViolation::ZoneTransition);
    }

    let old = transition.old_object;
    let new = transition.new_object;
    let old_object = before
        .zones
        .objects
        .get(&old)
        .ok_or(TransitionViolation::ZoneTransition)?;
    let old_snapshot = crate::snapshots::object_snapshots(before)?
        .remove(&old)
        .ok_or(TransitionViolation::ZoneTransition)?;
    if transition.last_known != old_snapshot
        || old == new
        || transition.physical_card != old_object.physical_card
        || transition.physical_card.is_none()
        || old_object.face_down
    {
        return Err(TransitionViolation::ZoneTransition);
    }

    let source = match family {
        SelectedZoneFamily::BattlefieldToGraveyard => ZoneLocation {
            zone: ZoneKind::Battlefield,
            player: None,
            position: ZonePosition::Unordered,
            visibility: VisibilityPartition::Public,
            partition: None,
        },
        SelectedZoneFamily::LibraryToHand => ZoneLocation {
            zone: ZoneKind::Library,
            player: Some(old_object.owner),
            position: ZonePosition::Top { offset: 0 },
            visibility: VisibilityPartition::FaceDown,
            partition: None,
        },
    };
    let destination = match family {
        SelectedZoneFamily::BattlefieldToGraveyard => ZoneLocation {
            zone: ZoneKind::Graveyard,
            player: Some(old_object.owner),
            position: ZonePosition::Top { offset: 0 },
            visibility: VisibilityPartition::Public,
            partition: None,
        },
        SelectedZoneFamily::LibraryToHand => ZoneLocation {
            zone: ZoneKind::Hand,
            player: Some(old_object.owner),
            position: ZonePosition::Unordered,
            visibility: VisibilityPartition::OwnerOnly,
            partition: None,
        },
    };
    if transition.from != source || transition.to != destination {
        return Err(TransitionViolation::ZoneTransition);
    }
    if object_references_old(before, old) {
        return Err(TransitionViolation::OldReferenceClosure);
    }

    let expected_new = GameObject {
        id: new,
        physical_card: old_object.physical_card,
        card_definition: old_object.card_definition,
        owner: old_object.owner,
        controller: old_object.owner,
        tapped: false,
        face_down: false,
    };
    let expected_snapshot = mtgml_state::ObjectSnapshot {
        object: new,
        physical_card: old_object.physical_card,
        card_definition: old_object.card_definition,
        owner: old_object.owner,
        controller: old_object.owner,
        tapped: false,
        face_down: false,
        location: destination.clone(),
    };
    if transition.new_snapshot != expected_snapshot {
        return Err(TransitionViolation::ZoneTransition);
    }

    let expected_next_object_id = before
        .allocators
        .next_object_id
        .0
        .checked_add(1)
        .ok_or(TransitionViolation::ObjectAllocatorProgression)?;
    if new != before.allocators.next_object_id
        || result.next_state.allocators.next_object_id.0 != expected_next_object_id
    {
        return Err(TransitionViolation::ObjectAllocatorProgression);
    }
    let before_allocators = &before.allocators;
    let after_allocators = &result.next_state.allocators;
    if after_allocators.next_ability_id != before_allocators.next_ability_id
        || after_allocators.next_stack_object_id != before_allocators.next_stack_object_id
        || after_allocators.next_effect_id != before_allocators.next_effect_id
        || after_allocators.next_trigger_id != before_allocators.next_trigger_id
        || after_allocators.next_decision_id != before_allocators.next_decision_id
        || after_allocators.next_continuation_id != before_allocators.next_continuation_id
    {
        return Err(TransitionViolation::UnrelatedAllocatorProgression);
    }
    if result.next_state.random != before.random
        || result.events.iter().any(|event| {
            matches!(
                event.event,
                AuthoritativeRuleEventKind::RandomValueSampled { .. }
            )
        })
    {
        return Err(TransitionViolation::Randomness);
    }
    if result.next_state.core != before.core
        || result.next_state.execution != before.execution
        || result.next_state.format != before.format
        || result.next_state.combat != before.combat
    {
        return Err(TransitionViolation::UnexplainedMutation);
    }

    let mut expected_zones = before.zones.clone();
    expected_zones.objects.remove(&old);
    expected_zones.locations.remove(&old);
    match family {
        SelectedZoneFamily::BattlefieldToGraveyard => {
            let destination_key = destination.key();
            let existing = before
                .zones
                .ordered_zones
                .get(&destination_key)
                .cloned()
                .unwrap_or_default();
            for member in &existing {
                let location = expected_zones
                    .locations
                    .get_mut(member)
                    .ok_or(TransitionViolation::ZoneOrderProgression)?;
                let ZonePosition::Top { offset } = location.position else {
                    return Err(TransitionViolation::ZoneOrderProgression);
                };
                location.position = ZonePosition::Top {
                    offset: offset
                        .checked_add(1)
                        .ok_or(TransitionViolation::ZoneOrderProgression)?,
                };
            }
            expected_zones
                .ordered_zones
                .entry(destination_key)
                .or_default()
                .insert(0, new);
        }
        SelectedZoneFamily::LibraryToHand => {
            let source_key = source.key();
            let ordered = expected_zones
                .ordered_zones
                .get_mut(&source_key)
                .ok_or(TransitionViolation::ZoneOrderProgression)?;
            if ordered.first() != Some(&old) {
                return Err(TransitionViolation::ZoneOrderProgression);
            }
            ordered.remove(0);
            let remaining = ordered.clone();
            if ordered.is_empty() {
                expected_zones.ordered_zones.remove(&source_key);
            }
            for member in remaining {
                let location = expected_zones
                    .locations
                    .get_mut(&member)
                    .ok_or(TransitionViolation::ZoneOrderProgression)?;
                let ZonePosition::Top { offset } = location.position else {
                    return Err(TransitionViolation::ZoneOrderProgression);
                };
                location.position = ZonePosition::Top {
                    offset: offset
                        .checked_sub(1)
                        .ok_or(TransitionViolation::ZoneOrderProgression)?,
                };
            }
        }
    }
    expected_zones.objects.insert(new, expected_new);
    expected_zones.locations.insert(new, destination.clone());
    if result.next_state.zones.ordered_zones != expected_zones.ordered_zones {
        return Err(TransitionViolation::ZoneOrderProgression);
    }
    if result.next_state.zones != expected_zones {
        return Err(TransitionViolation::ZoneTransition);
    }

    if result.next_state.zones.objects.contains_key(&old)
        || result.next_state.zones.locations.contains_key(&old)
        || result
            .next_state
            .zones
            .ordered_zones
            .values()
            .any(|objects| objects.contains(&old))
        || result.next_state.foundation_sources.contains_key(&old)
        || result.next_state.foundation_sources.contains_key(&new)
        || object_references_old(&result.next_state, old)
        || result
            .next_state
            .perspective_identities
            .players
            .values()
            .any(|identity| identity.object_to_opaque.contains_key(&old))
    {
        return Err(TransitionViolation::OldReferenceClosure);
    }

    let expected_occurrences = expected_s2_occurrences(before, transition, family)?;
    if result.events.len() != expected_occurrences.len() + 1 {
        return Err(TransitionViolation::OccurrencePairing);
    }
    for (event, (lifecycle, observation)) in result
        .events
        .iter()
        .skip(1)
        .zip(expected_occurrences.iter())
    {
        match &event.event {
            AuthoritativeRuleEventKind::PerspectiveOccurrence {
                lifecycle: actual_lifecycle,
                observation: actual_observation,
            } if actual_lifecycle == lifecycle && actual_observation == observation => {}
            _ => return Err(TransitionViolation::OccurrencePairing),
        }
    }
    Ok(())
}

fn validate_sba_batch_product(
    before: &EngineState,
    result: &TransitionResult,
) -> Result<(), TransitionViolation> {
    let batches: Vec<_> = result
        .events
        .iter()
        .enumerate()
        .filter_map(|(index, event)| match &event.event {
            AuthoritativeRuleEventKind::StateBasedActionsApplied { actions } => {
                Some((index, actions))
            }
            _ => None,
        })
        .collect();
    if batches.len() != 1 {
        return Err(TransitionViolation::SbaBatch);
    }
    let (batch_index, actions) = batches[0];
    let plan = crate::state_based_actions::derive_bounded_sba_round_plan(before)
        .map_err(|_| TransitionViolation::SbaBatch)?;
    if actions != &plan.selected_sba_actions || actions.is_empty() {
        return Err(TransitionViolation::SbaBatch);
    }

    let mut orders = Vec::new();
    let continuation_id = if let Some(continuation) = magic_sba_order_continuation(before) {
        if batch_index != 2 {
            return Err(TransitionViolation::SbaBatch);
        }
        crate::state_based_actions::validate_sba_order_continuation(before)
            .map_err(|_| TransitionViolation::SbaBatch)?;
        let ContinuationPayloadV2::MagicSbaGraveyardOrderV1 {
            completed_owner_orders,
            ..
        } = &continuation.payload
        else {
            return Err(TransitionViolation::SbaBatch);
        };
        orders = completed_owner_orders.clone();
        let (decision, order) = match result.events.get(..batch_index) {
            Some(
                [crate::events::AuthoritativeRuleEvent {
                    event: AuthoritativeRuleEventKind::DecisionCleared { decision },
                    ..
                }, crate::events::AuthoritativeRuleEvent {
                    event:
                        AuthoritativeRuleEventKind::SbaGraveyardOrderChosen {
                            continuation: chosen_continuation,
                            owner,
                            top_to_bottom,
                        },
                    ..
                }],
            ) if *chosen_continuation == continuation.id => (
                *decision,
                SbaGraveyardOwnerOrderV1 {
                    owner: *owner,
                    top_to_bottom: top_to_bottom.clone(),
                },
            ),
            _ => return Err(TransitionViolation::SbaBatch),
        };
        if before
            .execution
            .pending_decision
            .as_ref()
            .map(|pending| pending.request.decision_id)
            != Some(decision)
        {
            return Err(TransitionViolation::SbaBatch);
        }
        orders.push(order);
        Some(continuation.id)
    } else {
        if batch_index != 0
            || before.execution.pending_decision.is_some()
            || !before.execution.continuations.is_empty()
        {
            return Err(TransitionViolation::SbaBatch);
        }
        None
    };
    if continuation_id.is_none() && !plan.apnap_owners.is_empty() {
        return Err(TransitionViolation::SbaBatch);
    }
    let invocation_order =
        crate::state_based_actions::ordered_sba_objects_for_s2(before, &plan, &orders)
            .map_err(|_| TransitionViolation::SbaBatch)?;

    let zone_event_index = result
        .events
        .iter()
        .position(|event| {
            matches!(
                event.event,
                AuthoritativeRuleEventKind::ZoneTransition { .. }
            )
        })
        .unwrap_or(result.events.len());
    if zone_event_index < batch_index + 1 {
        return Err(TransitionViolation::SbaBatch);
    }
    let prefix_len = if continuation_id.is_some() { 3 } else { 1 };
    if zone_event_index < prefix_len || result.events.len() < prefix_len {
        return Err(TransitionViolation::SbaBatch);
    }
    let mut expected_events = result.events[..prefix_len].to_vec();
    let mut candidate = before.clone();
    candidate.revision = result.next_state.revision;
    candidate.execution.pending_decision = None;
    if let Some(continuation) = continuation_id {
        candidate.execution.continuations.remove(&continuation);
    }
    let selected_objects: BTreeSet<_> = actions
        .iter()
        .filter_map(|action| match action {
            SbaSelectedActionV1::ObjectToOwnerGraveyard { object, .. } => Some(*object),
            SbaSelectedActionV1::PlayerLoses { .. } => None,
        })
        .collect();
    for action in actions {
        if let SbaSelectedActionV1::PlayerLoses { player } = action {
            let status = candidate
                .core
                .players
                .get_mut(player)
                .ok_or(TransitionViolation::SbaBatch)?;
            if status.life > 0 || status.has_lost {
                return Err(TransitionViolation::SbaBatch);
            }
            status.has_lost = true;
        }
    }
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
                before.core.position,
                TurnPosition::Combat {
                    step: mtgml_state::CombatStep::CombatDamage
                }
            )
        {
            return Err(TransitionViolation::SbaBatch);
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
    for object in invocation_order {
        let owner = candidate
            .zones
            .objects
            .get(&object)
            .ok_or(TransitionViolation::SbaBatch)?
            .owner;
        let from = candidate
            .zones
            .locations
            .get(&object)
            .cloned()
            .ok_or(TransitionViolation::SbaBatch)?;
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
            before.allocators.next_rule_event_id,
            &mut expected_events,
        )
        .map_err(|_| TransitionViolation::ZoneTransition)?;
    }
    if expected_events != result.events {
        return Err(TransitionViolation::SbaBatch);
    }
    candidate.allocators.next_rule_event_id = mtgml_model::RuleEventId(
        before
            .allocators
            .next_rule_event_id
            .0
            .checked_add(
                u64::try_from(result.events.len())
                    .map_err(|_| TransitionViolation::EventIdentity)?,
            )
            .ok_or(TransitionViolation::EventIdentity)?,
    );
    if candidate != result.next_state {
        return Err(TransitionViolation::SbaBatch);
    }

    let losers: Vec<_> = actions
        .iter()
        .filter_map(|action| match action {
            SbaSelectedActionV1::PlayerLoses { player } => Some(*player),
            SbaSelectedActionV1::ObjectToOwnerGraveyard { .. } => None,
        })
        .collect();
    let expected_status = match losers.as_slice() {
        [] => EpisodeStatus::Running,
        [loser] => EpisodeStatus::Terminal {
            reason: mtgml_model::TerminalReason::RulesLoss,
            players: before
                .core
                .players
                .keys()
                .copied()
                .map(|player| mtgml_model::PlayerOutcome {
                    player,
                    result: if player == *loser {
                        mtgml_model::PlayerResult::Loss
                    } else {
                        mtgml_model::PlayerResult::Win
                    },
                })
                .collect(),
        },
        _ if losers.len() == before.core.players.len() => EpisodeStatus::Terminal {
            reason: mtgml_model::TerminalReason::SimultaneousOutcome,
            players: before
                .core
                .players
                .keys()
                .copied()
                .map(|player| mtgml_model::PlayerOutcome {
                    player,
                    result: mtgml_model::PlayerResult::Draw,
                })
                .collect(),
        },
        _ => return Err(TransitionViolation::SbaBatch),
    };
    if result.status != expected_status || result.next_decision.is_some() {
        return Err(TransitionViolation::SbaBatch);
    }
    Ok(())
}

fn foundation_sources_match_selected_zone_transitions(
    before: &EngineState,
    after: &EngineState,
    events: &[crate::events::AuthoritativeRuleEvent],
) -> bool {
    let battlefield = ZoneLocation {
        zone: ZoneKind::Battlefield,
        player: None,
        position: ZonePosition::Unordered,
        visibility: VisibilityPartition::Public,
        partition: None,
    };
    let mut expected = before.foundation_sources.clone();
    for event in events {
        let crate::events::AuthoritativeRuleEventKind::ZoneTransition { transition } = &event.event
        else {
            continue;
        };
        let is_selected_battlefield_graveyard = transition.from == battlefield
            && transition.to
                == (ZoneLocation {
                    zone: ZoneKind::Graveyard,
                    player: Some(transition.last_known.owner),
                    position: ZonePosition::Top { offset: 0 },
                    visibility: VisibilityPartition::Public,
                    partition: None,
                });
        if is_selected_battlefield_graveyard {
            expected.remove(&transition.old_object);
            if after
                .foundation_sources
                .contains_key(&transition.new_object)
            {
                return false;
            }
        }
    }
    expected == after.foundation_sources
}

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
    //
    // `core.position` is intentionally NOT in this blanket: it is proven
    // event-by-event through the `TurnPositionChanged` cursor arm (Task 6),
    // which requires `temporal_successor(from) == to`.
    let has_lost_changed = before.core.players.iter().any(|(player, state)| {
        after
            .core
            .players
            .get(player)
            .is_none_or(|other| other.has_lost != state.has_lost)
    });

    // Task 7: detect the quiescent Cleanup boundary. A product that
    // transitions Ending(Cleanup) -> Beginning(Untap) MUST produce the
    // exact three-event shape. This check runs BEFORE the blanket mutation
    // check so that every Cleanup boundary violation returns TurnStructure
    // (not UnexplainedMutation). Conversely, Task-7 event families cannot
    // justify an unrelated transition because the before/after position
    // guard gates this rule entirely.
    let is_cleanup_boundary = before.core.position
        == TurnPosition::Ending {
            step: EndingStep::Cleanup,
        }
        && after.core.position
            == TurnPosition::Beginning {
                step: BeginningStep::Untap,
            };

    if is_cleanup_boundary {
        let next_turn = before.core.turn_number.checked_add(1);
        let turn_ok = next_turn == Some(after.core.turn_number);
        let player_ok = after.core.active_player != before.core.active_player
            && before.core.players.contains_key(&after.core.active_player)
            && before.core.players.len() == 2;
        let events_ok = result.events.len() == 3
            && matches!(
                &result.events[0].event,
                AuthoritativeRuleEventKind::TurnNumberChanged { from, to }
                    if *from == before.core.turn_number
                        && *to == after.core.turn_number
            )
            && matches!(
                &result.events[1].event,
                AuthoritativeRuleEventKind::ActivePlayerChanged { from, to }
                    if *from == before.core.active_player
                        && *to == after.core.active_player
                        && from != to
            )
            && matches!(
                &result.events[2].event,
                AuthoritativeRuleEventKind::TurnPositionChanged { from, to }
                    if *from == TurnPosition::Ending {
                        step: EndingStep::Cleanup,
                    }
                    && *to == TurnPosition::Beginning {
                        step: BeginningStep::Untap,
                    }
            );
        if !turn_ok || !player_ok || !events_ok {
            return Err(TransitionViolation::TurnStructure);
        }

        validate_quiescent_cleanup_boundary(before, before.core.active_player)
            .map_err(|_| TransitionViolation::TurnStructure)?;

        // Exact Cleanup event shape validated: active_player and turn_number
        // changes are expected and proven by events. Continue to blanket
        // check which will pass for all non-position/non-priority fields.
    } else {
        // Task 7 turn-switch events (TurnNumberChanged, ActivePlayerChanged)
        // are valid for the Cleanup -> next-Untap transition only. Outside
        // that boundary they must never justify any transition, because the
        // blanket mutation check sees only net Before/After equality and
        // would permit transient switches (e.g. P1->P2->P1) that net to
        // the original state.
        let has_turn_switch_event = result.events.iter().any(|event| {
            matches!(
                event.event,
                AuthoritativeRuleEventKind::TurnNumberChanged { .. }
                    | AuthoritativeRuleEventKind::ActivePlayerChanged { .. }
            )
        });
        if has_turn_switch_event {
            return Err(TransitionViolation::TurnStructure);
        }
    }

    // Task 9: if this accepted product performs the ordinary untap boundary
    // (Untap -> Upkeep), enforce the exact ordinary-untap event shape with
    // public tap observations.
    //
    // Required order:
    //   1. UntapCompleted
    //   2. PerspectiveOccurrence per (perspective ascending, object ascending)
    //      with ObjectTapped observation, one per affected object × perspective
    //   3. TurnPositionChanged(Untap -> Upkeep)
    //
    // UntapCompleted must be first so the cursor derives its expected set
    // from the before-state. Every middle event must be a causally bound
    // ObjectTapped occurrence. No free-floating observation policies are
    // accepted at this boundary.
    let has_untap = result.events.iter().any(|event| {
        matches!(
            event.event,
            AuthoritativeRuleEventKind::UntapCompleted { .. }
        )
    });

    let is_ordinary_untap_transition = before.core.position
        == TurnPosition::Beginning {
            step: BeginningStep::Untap,
        }
        && after.core.position
            == TurnPosition::Beginning {
                step: BeginningStep::Upkeep,
            };

    if is_ordinary_untap_transition {
        if result.events.len() < 2 {
            return Err(TransitionViolation::TurnStructure);
        }
        // UntapCompleted must be first.
        if !matches!(
            &result.events[0].event,
            AuthoritativeRuleEventKind::UntapCompleted { .. }
        ) {
            return Err(TransitionViolation::TurnStructure);
        }

        let untap_completed = match &result.events[0].event {
            AuthoritativeRuleEventKind::UntapCompleted { affected_objects } => affected_objects,
            _ => unreachable!(),
        };

        // TurnPositionChanged(Untap -> Upkeep) must be last.
        if !matches!(
            &result.events[result.events.len() - 1].event,
            AuthoritativeRuleEventKind::TurnPositionChanged { from, to }
                if *from == TurnPosition::Beginning { step: BeginningStep::Untap }
                    && *to == TurnPosition::Beginning { step: BeginningStep::Upkeep }
        ) {
            return Err(TransitionViolation::TurnStructure);
        }

        let middle_events = &result.events[1..result.events.len() - 1];

        // Every middle event must be a PerspectiveOccurrence with ObjectTapped.
        let mut seen_pairs: BTreeSet<(PlayerId, GameObjectId)> = BTreeSet::new();
        let mut prev_perspective: Option<PlayerId> = None;
        let mut prev_object: Option<GameObjectId> = None;

        for event in middle_events {
            let lifecycle = match &event.event {
                AuthoritativeRuleEventKind::PerspectiveOccurrence { lifecycle, .. } => lifecycle,
                _ => return Err(TransitionViolation::TurnStructure),
            };
            let observation = match &event.event {
                AuthoritativeRuleEventKind::PerspectiveOccurrence { observation, .. } => {
                    observation
                }
                _ => return Err(TransitionViolation::TurnStructure),
            };

            let object = match observation {
                crate::events::PerspectiveObservationPolicyV1::ObjectTapped { object, tapped } => {
                    // Object must be in UntapCompleted affected set.
                    if !untap_completed.contains(object) {
                        return Err(TransitionViolation::TurnStructure);
                    }
                    // tapped must be false (untap clears tapped=true to false).
                    if *tapped {
                        return Err(TransitionViolation::TurnStructure);
                    }
                    *object
                }
                _ => return Err(TransitionViolation::TurnStructure),
            };

            // No duplicate (perspective, object) pair.
            let pair = (lifecycle.perspective, object);
            if !seen_pairs.insert(pair) {
                return Err(TransitionViolation::TurnStructure);
            }

            // Canonical order: perspective ascending, then object ascending.
            if let Some(prev_p) = prev_perspective {
                if lifecycle.perspective < prev_p {
                    return Err(TransitionViolation::TurnStructure);
                }
                if lifecycle.perspective == prev_p {
                    if let Some(prev_o) = prev_object {
                        if object < prev_o {
                            return Err(TransitionViolation::TurnStructure);
                        }
                    }
                }
            }
            prev_perspective = Some(lifecycle.perspective);
            prev_object = Some(object);
        }

        // Complete set: every (affected_object, perspective) pair must have
        // exactly one occurrence. No missing, no extra.
        let expected_count = untap_completed.len() * before.core.players.len();
        if middle_events.len() != expected_count {
            return Err(TransitionViolation::TurnStructure);
        }
        for object in untap_completed {
            for player in before.core.players.keys() {
                if !seen_pairs.contains(&(*player, *object)) {
                    return Err(TransitionViolation::TurnStructure);
                }
            }
        }
    } else if has_untap {
        // UntapCompleted outside an ordinary untap transition is invalid.
        return Err(TransitionViolation::TurnStructure);
    }

    // Blanket mutation check. active_player and turn_number changes
    // are already validated above for Cleanup boundaries. All other
    // fields must not change for any accepted transition.
    let has_sba_application = result.events.iter().any(|event| {
        matches!(
            event.event,
            AuthoritativeRuleEventKind::StateBasedActionsApplied { .. }
        )
    });
    if (before.core.active_player != after.core.active_player
        || before.core.turn_number != after.core.turn_number)
        && !is_cleanup_boundary
        || before.core.priority != after.core.priority
        || before.core.players.len() != after.core.players.len()
        || (has_lost_changed && !has_sba_application)
        || (before.combat != after.combat && !has_sba_application)
    {
        return Err(TransitionViolation::UnexplainedMutation);
    }
    if !foundation_sources_match_selected_zone_transitions(before, after, &result.events) {
        let selected_battlefield_graveyard = result.events.iter().any(|event| {
            matches!(
                &event.event,
                AuthoritativeRuleEventKind::ZoneTransition { transition }
                    if transition.from.zone == ZoneKind::Battlefield
                        && transition.to.zone == ZoneKind::Graveyard
            )
        });
        return Err(if selected_battlefield_graveyard {
            TransitionViolation::FoundationSourceProgression
        } else {
            TransitionViolation::UnexplainedMutation
        });
    }
    Ok(())
}

fn magic_sba_order_continuation(state: &EngineState) -> Option<&ContinuationRecordV2> {
    state.execution.continuations.values().find(|record| {
        matches!(
            &record.payload,
            ContinuationPayloadV2::MagicSbaGraveyardOrderV1 { .. }
        )
    })
}

fn validate_sba_order_stage_world(
    before: &EngineState,
    after: &EngineState,
    actor_with_new_request: PlayerId,
    expected_next_continuation: mtgml_model::ContinuationId,
    expected_event_count: u64,
) -> Result<(), TransitionViolation> {
    if after.revision.0
        != before
            .revision
            .0
            .checked_add(1)
            .ok_or(TransitionViolation::RevisionDidNotAdvance)?
        || before.core != after.core
        || before.combat != after.combat
        || before.foundation_sources != after.foundation_sources
        || before.zones != after.zones
        || before.random != after.random
        || before.knowledge != after.knowledge
        || before.format != after.format
        || before.execution.effects != after.execution.effects
        || before.execution.waiting_triggers != after.execution.waiting_triggers
        || before.execution.delayed_effects != after.execution.delayed_effects
    {
        return Err(TransitionViolation::SbaOrder);
    }

    let mut expected_allocators = before.allocators.clone();
    expected_allocators.next_decision_id = mtgml_model::DecisionId(
        before
            .allocators
            .next_decision_id
            .0
            .checked_add(1)
            .ok_or(TransitionViolation::DecisionProgression)?,
    );
    expected_allocators.next_continuation_id = expected_next_continuation;
    expected_allocators.next_rule_event_id = mtgml_model::RuleEventId(
        before
            .allocators
            .next_rule_event_id
            .0
            .checked_add(expected_event_count)
            .ok_or(TransitionViolation::EventIdentity)?,
    );
    if after.allocators != expected_allocators {
        return Err(TransitionViolation::SbaOrder);
    }

    let mut expected_identities = before.perspective_identities.clone();
    let expected_actor_identity = expected_identities
        .players
        .get_mut(&actor_with_new_request)
        .ok_or(TransitionViolation::DecisionProgression)?;
    expected_actor_identity.next_player_decision_id = mtgml_model::PlayerDecisionIdV1(
        expected_actor_identity
            .next_player_decision_id
            .0
            .checked_add(1)
            .ok_or(TransitionViolation::DecisionProgression)?,
    );
    if after.perspective_identities != expected_identities {
        return Err(TransitionViolation::SbaOrder);
    }
    Ok(())
}

fn validate_sba_order_transition(
    before: &EngineState,
    result: &TransitionResult,
) -> Result<(), TransitionViolation> {
    use crate::events::AuthoritativeRuleEventKind as Event;
    let after = &result.next_state;
    let before_record = magic_sba_order_continuation(before);
    let after_record = magic_sba_order_continuation(after);
    let has_order_event = result
        .events
        .iter()
        .any(|event| matches!(&event.event, Event::SbaGraveyardOrderChosen { .. }));

    match (before_record, after_record) {
        (None, None) => {
            if has_order_event {
                return Err(TransitionViolation::SbaOrder);
            }
            Ok(())
        }
        (None, Some(after_continuation)) => {
            if before.execution.pending_decision.is_some()
                || !before.execution.continuations.is_empty()
                || result.events.len() != 1
                || !matches!(
                    &result.events[0].event,
                    Event::DecisionCreated { decision }
                        if after.execution.pending_decision.as_ref().is_some_and(|pending|
                            pending.request.decision_id == *decision)
                )
                || !matches!(result.status, EpisodeStatus::Running)
            {
                return Err(TransitionViolation::SbaOrder);
            }
            let ContinuationPayloadV2::MagicSbaGraveyardOrderV1 {
                round_start_revision,
                selected_sba_actions,
                apnap_owners,
                next_owner_index,
                completed_owner_orders,
            } = &after_continuation.payload
            else {
                return Err(TransitionViolation::SbaOrder);
            };
            let plan = crate::state_based_actions::derive_bounded_sba_round_plan(before)
                .map_err(|_| TransitionViolation::SbaOrder)?;
            let first_owner = *plan
                .apnap_owners
                .first()
                .ok_or(TransitionViolation::SbaOrder)?;
            let pending = after
                .execution
                .pending_decision
                .as_ref()
                .ok_or(TransitionViolation::SbaOrder)?;
            let request = &pending.request;
            let expected_continuation = before.allocators.next_continuation_id;
            if after.execution.continuations.len() != 1
                || after_continuation.id != expected_continuation
                || after_continuation.actor != first_owner
                || after_continuation.created_at_revision != after.revision
                || after_continuation.stage_index != 0
                || *round_start_revision != before.revision
                || *selected_sba_actions != plan.selected_sba_actions
                || *apnap_owners != plan.apnap_owners
                || *next_owner_index != 0
                || !completed_owner_orders.is_empty()
                || request.continuation_id != Some(expected_continuation)
                || request.actor != first_owner
                || request.visibility != mtgml_decision::DecisionVisibility::ActingPlayerOnly
                || request.decision_id != before.allocators.next_decision_id
                || request.state_revision != after.revision
            {
                return Err(TransitionViolation::SbaOrder);
            }
            let next_continuation = mtgml_model::ContinuationId(
                expected_continuation
                    .0
                    .checked_add(1)
                    .ok_or(TransitionViolation::AllocatorProgression)?,
            );
            validate_sba_order_stage_world(before, after, first_owner, next_continuation, 1)
        }
        (Some(before_continuation), None) => {
            if !has_order_event
                || result.events.len() < 3
                || !matches!(
                    &result.events[0].event,
                    Event::DecisionCleared { decision }
                        if before.execution.pending_decision.as_ref().is_some_and(|pending|
                            pending.request.decision_id == *decision)
                )
                || !matches!(
                    &result.events[2].event,
                    Event::StateBasedActionsApplied { .. }
                )
                || after.execution.pending_decision.is_some()
                || after.execution.continuations.values().any(|record| {
                    matches!(
                        record.payload,
                        ContinuationPayloadV2::MagicSbaGraveyardOrderV1 { .. }
                    )
                })
            {
                return Err(TransitionViolation::SbaOrder);
            }
            let Event::SbaGraveyardOrderChosen {
                continuation,
                owner,
                ..
            } = &result.events[1].event
            else {
                return Err(TransitionViolation::SbaOrder);
            };
            let ContinuationPayloadV2::MagicSbaGraveyardOrderV1 {
                apnap_owners,
                next_owner_index,
                ..
            } = &before_continuation.payload
            else {
                return Err(TransitionViolation::SbaOrder);
            };
            if *continuation != before_continuation.id
                || apnap_owners.get(*next_owner_index as usize) != Some(owner)
            {
                return Err(TransitionViolation::SbaOrder);
            }
            Ok(())
        }
        (Some(before_continuation), Some(after_continuation)) => {
            if !has_order_event
                || result.events.len() != 3
                || !matches!(
                    &result.events[0].event,
                    Event::DecisionCleared { decision }
                        if before.execution.pending_decision.as_ref().is_some_and(|pending|
                            pending.request.decision_id == *decision)
                )
                || !matches!(&result.events[2].event, Event::DecisionCreated { .. })
                || !matches!(result.status, EpisodeStatus::Running)
            {
                return Err(TransitionViolation::SbaOrder);
            }
            let Event::SbaGraveyardOrderChosen {
                continuation,
                owner,
                top_to_bottom,
            } = &result.events[1].event
            else {
                return Err(TransitionViolation::SbaOrder);
            };
            let ContinuationPayloadV2::MagicSbaGraveyardOrderV1 {
                round_start_revision: before_round_start,
                selected_sba_actions: before_actions,
                apnap_owners: before_owners,
                next_owner_index: before_index,
                completed_owner_orders: before_orders,
            } = &before_continuation.payload
            else {
                return Err(TransitionViolation::SbaOrder);
            };
            let ContinuationPayloadV2::MagicSbaGraveyardOrderV1 {
                round_start_revision: after_round_start,
                selected_sba_actions: after_actions,
                apnap_owners: after_owners,
                next_owner_index: after_index,
                completed_owner_orders: _after_orders,
            } = &after_continuation.payload
            else {
                return Err(TransitionViolation::SbaOrder);
            };
            let old_index =
                usize::try_from(*before_index).map_err(|_| TransitionViolation::SbaOrder)?;
            let expected_owner = *before_owners
                .get(old_index)
                .ok_or(TransitionViolation::SbaOrder)?;
            let next_index = before_index
                .checked_add(1)
                .ok_or(TransitionViolation::SbaOrder)?;
            let next_actor = *before_owners
                .get(usize::try_from(next_index).map_err(|_| TransitionViolation::SbaOrder)?)
                .ok_or(TransitionViolation::SbaOrder)?;
            let before_pending = before
                .execution
                .pending_decision
                .as_ref()
                .ok_or(TransitionViolation::SbaOrder)?;
            let after_pending = after
                .execution
                .pending_decision
                .as_ref()
                .ok_or(TransitionViolation::SbaOrder)?;
            let expected_order = SbaGraveyardOwnerOrderV1 {
                owner: expected_owner,
                top_to_bottom: top_to_bottom.clone(),
            };
            let mut expected_continuation = before_continuation.clone();
            let ContinuationPayloadV2::MagicSbaGraveyardOrderV1 {
                next_owner_index,
                completed_owner_orders,
                ..
            } = &mut expected_continuation.payload
            else {
                unreachable!()
            };
            completed_owner_orders.push(expected_order);
            *next_owner_index = next_index;
            expected_continuation.actor = next_actor;
            expected_continuation.stage_index =
                u16::try_from(next_index).map_err(|_| TransitionViolation::SbaOrder)?;

            if *continuation != before_continuation.id
                || *owner != expected_owner
                || before_orders.len() != old_index
                || *after_round_start != *before_round_start
                || *after_actions != *before_actions
                || *after_owners != *before_owners
                || *after_index != next_index
                || after_continuation != &expected_continuation
                || after.execution.continuations.len() != 1
                || before_pending.request.continuation_id != Some(before_continuation.id)
                || after_pending.request.continuation_id != Some(before_continuation.id)
                || after_pending.request.actor != next_actor
                || after_pending.request.decision_id == before_pending.request.decision_id
                || after_pending.request.state_revision != after.revision
                || !matches!(result.events[2].event, Event::DecisionCreated { decision }
                    if decision == after_pending.request.decision_id)
            {
                return Err(TransitionViolation::SbaOrder);
            }
            let expected_next_continuation = before.allocators.next_continuation_id;
            validate_sba_order_stage_world(before, after, next_actor, expected_next_continuation, 3)
        }
    }
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
        validate_s2_zone_transition_product(before, result)?;
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
                        KnowledgeMutationV1::UpdateLocations { updates } => {
                            !updates.is_empty()
                                && updates
                                    .windows(2)
                                    .all(|pair| pair[0].opaque < pair[1].opaque)
                                && updates.iter().all(|update| {
                                    record_post.opaque_to_object.contains_key(&update.opaque)
                                        && update.fact.location.zone == transition.to.zone
                                        && update.fact.location.player == transition.to.player
                                })
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

    if result.accepted {
        validate_sba_order_transition(before, result)?;
    }

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
