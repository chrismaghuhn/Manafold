//! Private rules-owned entry seam for the selected zone-incarnation capability.
//!
//! This module owns the single future production implementation point. The
//! conformance facade below only translates its closed test vocabulary and
//! delegates here; it contains no transition behavior.

use mtgml_model::{GameObjectId, RuleEventId, StateRevision};
use mtgml_state::{validate_engine_state, EngineState, ZoneLocation, ZonePosition, ZoneTransition};

use crate::errors::ZoneIncarnationError;
use crate::events::{AuthoritativeRuleEvent, AuthoritativeRuleEventKind};
use crate::product::build_accepted_product;
use crate::{KernelExecutionError, TransitionResult};
use mtgml_state::{
    IdentityMutationV1, KnowledgeAcquisitionCause, KnowledgeAcquisitionReason,
    KnowledgeHistoryChannel, KnowledgeMutationV1, KnownLocationFactV2, PerspectiveLifecycleAuditV1,
    PerspectiveLifecycleMutationV1,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SelectedZoneTransitionKind {
    BattlefieldToOwnerGraveyard,
    LibraryTopToOwnerHand,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SelectedZoneTransitionRequest {
    pub object: GameObjectId,
    pub kind: SelectedZoneTransitionKind,
    pub claimed_from: ZoneLocation,
    pub claimed_to: ZoneLocation,
}

fn validate_old_references(
    state: &EngineState,
    object: GameObjectId,
    kind: SelectedZoneTransitionKind,
    owner: mtgml_model::PlayerId,
) -> Result<(), KernelExecutionError> {
    if state.combat.as_ref().is_some_and(|combat| {
        combat.attackers.contains(&object)
            || combat.blockers.contains_key(&object)
            || combat
                .blockers
                .values()
                .any(|blocker| *blocker == Some(object))
    }) {
        return Err(KernelExecutionError::ZoneIncarnation(
            ZoneIncarnationError::CombatReference,
        ));
    }
    if state
        .zones
        .stack_records
        .values()
        .any(|record| record.source_object == Some(object))
    {
        return Err(KernelExecutionError::ZoneIncarnation(
            ZoneIncarnationError::StackSourceReference,
        ));
    }
    if state
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
    {
        return Err(KernelExecutionError::ZoneIncarnation(
            ZoneIncarnationError::PendingDecisionReference,
        ));
    }
    if matches!(kind, SelectedZoneTransitionKind::LibraryTopToOwnerHand)
        && state.foundation_sources.contains_key(&object)
    {
        return Err(KernelExecutionError::ZoneIncarnation(
            ZoneIncarnationError::UnsupportedSourceProfile,
        ));
    }
    if matches!(kind, SelectedZoneTransitionKind::LibraryTopToOwnerHand)
        && state
            .perspective_identities
            .players
            .iter()
            .any(|(perspective, identity)| {
                *perspective != owner && identity.object_to_opaque.contains_key(&object)
            })
    {
        return Err(KernelExecutionError::ZoneIncarnation(
            ZoneIncarnationError::NonOwnerTracksHiddenSource,
        ));
    }
    Ok(())
}

fn emit_perspective_occurrence(
    event_origin: RuleEventId,
    prior_event_count: usize,
    candidate: &mut EngineState,
    events: &mut Vec<AuthoritativeRuleEvent>,
    lifecycle: PerspectiveLifecycleAuditV1,
    observation: crate::PerspectiveObservationPolicyV1,
) -> Result<(), KernelExecutionError> {
    mtgml_state::apply_perspective_lifecycle(candidate, &lifecycle)?;
    let local_offset =
        u64::try_from(events.len()).map_err(|_| KernelExecutionError::RuleEventIdOverflow)?;
    let prior_offset =
        u64::try_from(prior_event_count).map_err(|_| KernelExecutionError::RuleEventIdOverflow)?;
    let event_offset = prior_offset
        .checked_add(local_offset)
        .ok_or(KernelExecutionError::RuleEventIdOverflow)?;
    let event_id = event_origin
        .0
        .checked_add(event_offset)
        .ok_or(KernelExecutionError::RuleEventIdOverflow)?;
    events.push(AuthoritativeRuleEvent {
        event_id: mtgml_model::RuleEventId(event_id),
        state_revision: candidate.revision,
        event: AuthoritativeRuleEventKind::PerspectiveOccurrence {
            lifecycle,
            observation,
        },
    });
    Ok(())
}

/// Standalone S2 transition wrapper. It owns the one-move revision policy,
/// complete before/after validation, delta, and accepted product.
pub(crate) fn execute_selected_zone_transition(
    state: &EngineState,
    request: &SelectedZoneTransitionRequest,
) -> Result<TransitionResult, KernelExecutionError> {
    validate_engine_state(state).map_err(KernelExecutionError::BeforeState)?;
    let revision = StateRevision(
        state
            .revision
            .0
            .checked_add(1)
            .ok_or(KernelExecutionError::RevisionOverflow)?,
    );
    let mut candidate = state.clone();
    candidate.revision = revision;
    let mut events = Vec::new();
    apply_selected_zone_transition_in_workspace(
        &mut candidate,
        request,
        state.allocators.next_rule_event_id,
        &mut events,
    )?;
    build_accepted_product(state, candidate, events, |_| Ok(()))
}

/// Apply one selected S2 move to a caller-owned scratch workspace.
///
/// The candidate revision and outer event cursor belong to the coordinator.
/// This primitive never creates a `TransitionResult`, increments revision, or
/// validates a potentially incomplete intermediate workspace. It stages all
/// work in a private clone so an error leaves both the candidate and its event
/// vector untouched.
pub(crate) fn apply_selected_zone_transition_in_workspace(
    candidate: &mut EngineState,
    request: &SelectedZoneTransitionRequest,
    event_origin: RuleEventId,
    events: &mut Vec<AuthoritativeRuleEvent>,
) -> Result<ZoneTransition, KernelExecutionError> {
    let state = &*candidate;

    let old_object =
        state
            .zones
            .objects
            .get(&request.object)
            .ok_or(KernelExecutionError::ZoneIncarnation(
                ZoneIncarnationError::ObjectNotLive,
            ))?;
    let actual_from =
        state
            .zones
            .locations
            .get(&request.object)
            .ok_or(KernelExecutionError::ZoneIncarnation(
                ZoneIncarnationError::ObjectNotLive,
            ))?;
    if actual_from != &request.claimed_from {
        return Err(KernelExecutionError::ZoneIncarnation(
            ZoneIncarnationError::ClaimedSourceLocationMismatch,
        ));
    }

    let source_family_matches = match request.kind {
        SelectedZoneTransitionKind::BattlefieldToOwnerGraveyard => {
            actual_from
                == &ZoneLocation {
                    zone: mtgml_model::ZoneKind::Battlefield,
                    player: None,
                    position: ZonePosition::Unordered,
                    visibility: mtgml_state::VisibilityPartition::Public,
                    partition: None,
                }
        }
        SelectedZoneTransitionKind::LibraryTopToOwnerHand => {
            actual_from.zone == mtgml_model::ZoneKind::Library
                && actual_from.player == Some(old_object.owner)
                && matches!(actual_from.position, ZonePosition::Top { .. })
                && actual_from.visibility == mtgml_state::VisibilityPartition::FaceDown
                && actual_from.partition.is_none()
        }
    };
    if !source_family_matches {
        return Err(KernelExecutionError::ZoneIncarnation(
            ZoneIncarnationError::UnadmittedSourceFamily,
        ));
    }
    if matches!(
        request.kind,
        SelectedZoneTransitionKind::LibraryTopToOwnerHand
    ) && actual_from.position != (ZonePosition::Top { offset: 0 })
    {
        return Err(KernelExecutionError::ZoneIncarnation(
            ZoneIncarnationError::LibrarySourceNotTop,
        ));
    }

    let required_to = match request.kind {
        SelectedZoneTransitionKind::BattlefieldToOwnerGraveyard => ZoneLocation {
            zone: mtgml_model::ZoneKind::Graveyard,
            player: Some(old_object.owner),
            position: ZonePosition::Top { offset: 0 },
            visibility: mtgml_state::VisibilityPartition::Public,
            partition: None,
        },
        SelectedZoneTransitionKind::LibraryTopToOwnerHand => ZoneLocation {
            zone: mtgml_model::ZoneKind::Hand,
            player: Some(old_object.owner),
            position: ZonePosition::Unordered,
            visibility: mtgml_state::VisibilityPartition::OwnerOnly,
            partition: None,
        },
    };
    if request.claimed_to != required_to {
        return Err(KernelExecutionError::ZoneIncarnation(
            ZoneIncarnationError::DestinationMismatch,
        ));
    }
    if old_object.physical_card.is_none() {
        return Err(KernelExecutionError::ZoneIncarnation(
            ZoneIncarnationError::PhysicalCardRequired,
        ));
    }
    if old_object.face_down {
        return Err(KernelExecutionError::ZoneIncarnation(
            ZoneIncarnationError::UnsupportedSourceProfile,
        ));
    }

    let graveyard_key = required_to.key();
    if matches!(
        request.kind,
        SelectedZoneTransitionKind::BattlefieldToOwnerGraveyard
    ) {
        if let Some(existing) = state.zones.ordered_zones.get(&graveyard_key) {
            for member in existing {
                if state
                    .zones
                    .objects
                    .get(member)
                    .is_none_or(|object| object.face_down)
                {
                    return Err(KernelExecutionError::ZoneIncarnation(
                        ZoneIncarnationError::UnsupportedSourceProfile,
                    ));
                }
            }
        }
    }

    if matches!(
        request.kind,
        SelectedZoneTransitionKind::LibraryTopToOwnerHand
    ) {
        let ordered = state.zones.ordered_zones.get(&actual_from.key()).ok_or(
            KernelExecutionError::ZoneIncarnation(ZoneIncarnationError::LibrarySourceNotTop),
        )?;
        if ordered.first() != Some(&request.object) {
            return Err(KernelExecutionError::ZoneIncarnation(
                ZoneIncarnationError::LibrarySourceNotTop,
            ));
        }
    }

    validate_old_references(state, request.object, request.kind, old_object.owner)?;

    let last_known = crate::snapshots::object_snapshots(state)
        .map_err(KernelExecutionError::TransitionContract)?
        .remove(&request.object)
        .ok_or(KernelExecutionError::ZoneIncarnation(
            ZoneIncarnationError::ObjectNotLive,
        ))?;

    let mut next = state.clone();
    let new_object_id = next.allocators.allocate_object_id()?;
    if next.zones.objects.contains_key(&new_object_id) {
        return Err(KernelExecutionError::ZoneIncarnation(
            ZoneIncarnationError::ObjectIdCollision,
        ));
    }

    match request.kind {
        SelectedZoneTransitionKind::BattlefieldToOwnerGraveyard => {
            next.foundation_sources.remove(&request.object);
            if let Some(existing) = next.zones.ordered_zones.get(&graveyard_key).cloned() {
                for member in existing {
                    let location = next.zones.locations.get_mut(&member).ok_or(
                        KernelExecutionError::ZoneIncarnation(
                            ZoneIncarnationError::UnadmittedSourceFamily,
                        ),
                    )?;
                    let ZonePosition::Top { offset } = location.position else {
                        return Err(KernelExecutionError::ZoneIncarnation(
                            ZoneIncarnationError::UnadmittedSourceFamily,
                        ));
                    };
                    location.position = ZonePosition::Top {
                        offset: offset.checked_add(1).ok_or(
                            KernelExecutionError::ZoneIncarnation(
                                ZoneIncarnationError::GraveyardOffsetOverflow,
                            ),
                        )?,
                    };
                }
            }
            next.zones
                .ordered_zones
                .entry(graveyard_key.clone())
                .or_default()
                .insert(0, new_object_id);
        }
        SelectedZoneTransitionKind::LibraryTopToOwnerHand => {
            let source_key = actual_from.key();
            let ordered = next.zones.ordered_zones.get_mut(&source_key).ok_or(
                KernelExecutionError::ZoneIncarnation(ZoneIncarnationError::LibrarySourceNotTop),
            )?;
            if ordered.first() != Some(&request.object) {
                return Err(KernelExecutionError::ZoneIncarnation(
                    ZoneIncarnationError::LibrarySourceNotTop,
                ));
            }
            ordered.remove(0);
            if ordered.is_empty() {
                next.zones.ordered_zones.remove(&source_key);
            } else {
                for member in ordered.iter() {
                    let location = next.zones.locations.get_mut(member).ok_or(
                        KernelExecutionError::ZoneIncarnation(
                            ZoneIncarnationError::LibrarySourceNotTop,
                        ),
                    )?;
                    let ZonePosition::Top { offset } = location.position else {
                        return Err(KernelExecutionError::ZoneIncarnation(
                            ZoneIncarnationError::LibrarySourceNotTop,
                        ));
                    };
                    location.position = ZonePosition::Top {
                        offset: offset.checked_sub(1).ok_or(
                            KernelExecutionError::ZoneIncarnation(
                                ZoneIncarnationError::LibrarySourceNotTop,
                            ),
                        )?,
                    };
                }
            }
        }
    }

    next.zones
        .objects
        .remove(&request.object)
        .ok_or(KernelExecutionError::ZoneIncarnation(
            ZoneIncarnationError::ObjectNotLive,
        ))?;
    next.zones
        .locations
        .remove(&request.object)
        .ok_or(KernelExecutionError::ZoneIncarnation(
            ZoneIncarnationError::ObjectNotLive,
        ))?;
    next.zones.objects.insert(
        new_object_id,
        mtgml_state::GameObject {
            id: new_object_id,
            physical_card: old_object.physical_card,
            card_definition: old_object.card_definition,
            owner: old_object.owner,
            controller: old_object.owner,
            tapped: false,
            face_down: false,
        },
    );
    next.zones
        .locations
        .insert(new_object_id, required_to.clone());

    let new_snapshot = crate::snapshots::object_snapshots(&next)
        .map_err(KernelExecutionError::TransitionContract)?
        .remove(&new_object_id)
        .ok_or(KernelExecutionError::ZoneIncarnation(
            ZoneIncarnationError::ObjectNotLive,
        ))?;
    let transition = ZoneTransition {
        old_object: request.object,
        new_object: new_object_id,
        physical_card: old_object.physical_card,
        from: actual_from.clone(),
        to: required_to.clone(),
        last_known,
        new_snapshot,
    };
    let prior_event_count =
        u64::try_from(events.len()).map_err(|_| KernelExecutionError::RuleEventIdOverflow)?;
    let event_id = event_origin
        .0
        .checked_add(prior_event_count)
        .ok_or(KernelExecutionError::RuleEventIdOverflow)?;
    let event = AuthoritativeRuleEvent {
        event_id: mtgml_model::RuleEventId(event_id),
        state_revision: next.revision,
        event: AuthoritativeRuleEventKind::ZoneTransition {
            transition: Box::new(transition.clone()),
        },
    };
    let mut staged_events = vec![event];
    let new_object = new_object_id;
    let to_location = transition.to.clone();
    match request.kind {
        SelectedZoneTransitionKind::BattlefieldToOwnerGraveyard => {
            for perspective in state.core.players.keys().copied() {
                let identity = state
                    .perspective_identities
                    .players
                    .get(&perspective)
                    .ok_or(KernelExecutionError::ZoneIncarnation(
                        ZoneIncarnationError::PerspectiveKnowledgeMismatch,
                    ))?;
                let Some(opaque) = identity.object_to_opaque.get(&request.object).copied() else {
                    continue;
                };
                let knowledge = state.knowledge.players.get(&perspective).ok_or(
                    KernelExecutionError::ZoneIncarnation(
                        ZoneIncarnationError::PerspectiveKnowledgeMismatch,
                    ),
                )?;
                if !knowledge.active.contains_key(&opaque) {
                    return Err(KernelExecutionError::ZoneIncarnation(
                        ZoneIncarnationError::PerspectiveKnowledgeMismatch,
                    ));
                }
                let sequence = knowledge.next_visible_sequence;
                let provenance = KnowledgeAcquisitionReason::Observed {
                    channel: KnowledgeHistoryChannel::Public,
                    sequence,
                    cause: KnowledgeAcquisitionCause::PublicEvent,
                };
                let lifecycle = PerspectiveLifecycleAuditV1 {
                    perspective,
                    sequence,
                    mutation: PerspectiveLifecycleMutationV1 {
                        identity: IdentityMutationV1::Remap {
                            opaque,
                            from_object: request.object,
                            to_object: new_object,
                        },
                        knowledge: Some(KnowledgeMutationV1::UpdateLocation {
                            opaque,
                            fact: KnownLocationFactV2 {
                                location: to_location.clone(),
                                provenance,
                            },
                        }),
                    },
                };
                emit_perspective_occurrence(
                    event_origin,
                    events.len(),
                    &mut next,
                    &mut staged_events,
                    lifecycle,
                    crate::PerspectiveObservationPolicyV1::MovedInSight {
                        from_zone: transition.from.zone,
                        to_zone: transition.to.zone,
                        old_object: request.object,
                        new_object,
                        reveals_old: true,
                        reveals_new: true,
                    },
                )?;
            }
        }
        SelectedZoneTransitionKind::LibraryTopToOwnerHand => {
            let owner = old_object.owner;
            let identity = state.perspective_identities.players.get(&owner).ok_or(
                KernelExecutionError::ZoneIncarnation(
                    ZoneIncarnationError::PerspectiveKnowledgeMismatch,
                ),
            )?;
            let knowledge = state.knowledge.players.get(&owner).ok_or(
                KernelExecutionError::ZoneIncarnation(
                    ZoneIncarnationError::PerspectiveKnowledgeMismatch,
                ),
            )?;
            let sequence = knowledge.next_visible_sequence;
            let owner_opaque = identity.object_to_opaque.get(&request.object).copied();
            let (identity_mutation, knowledge_mutation, observation) =
                if let Some(opaque) = owner_opaque {
                    let record = knowledge.active.get(&opaque).ok_or(
                        KernelExecutionError::ZoneIncarnation(
                            ZoneIncarnationError::PerspectiveKnowledgeMismatch,
                        ),
                    )?;
                    if record.card_definition != Some(old_object.card_definition) {
                        return Err(KernelExecutionError::ZoneIncarnation(
                            ZoneIncarnationError::PerspectiveKnowledgeMismatch,
                        ));
                    }
                    let provenance = KnowledgeAcquisitionReason::Observed {
                        channel: KnowledgeHistoryChannel::Private,
                        sequence,
                        cause: KnowledgeAcquisitionCause::OwnPrivateIdentity,
                    };
                    (
                        IdentityMutationV1::Remap {
                            opaque,
                            from_object: request.object,
                            to_object: new_object,
                        },
                        Some(KnowledgeMutationV1::UpdateLocation {
                            opaque,
                            fact: KnownLocationFactV2 {
                                location: to_location.clone(),
                                provenance,
                            },
                        }),
                        crate::PerspectiveObservationPolicyV1::MovedInSight {
                            from_zone: transition.from.zone,
                            to_zone: transition.to.zone,
                            old_object: request.object,
                            new_object,
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
                            object: new_object,
                        },
                        Some(KnowledgeMutationV1::Acquire {
                            opaque,
                            definition: Some(old_object.card_definition),
                            location: Some(to_location.clone()),
                            acquisition,
                        }),
                        crate::PerspectiveObservationPolicyV1::NoEnvelope,
                    )
                };
            let lifecycle = PerspectiveLifecycleAuditV1 {
                perspective: owner,
                sequence,
                mutation: PerspectiveLifecycleMutationV1 {
                    identity: identity_mutation,
                    knowledge: knowledge_mutation,
                },
            };
            emit_perspective_occurrence(
                event_origin,
                events.len(),
                &mut next,
                &mut staged_events,
                lifecycle,
                observation,
            )?;
        }
    }
    *candidate = next;
    events.extend(staged_events);
    Ok(transition)
}

/// Closed family vocabulary available only when the conformance testkit feature
/// is explicitly enabled. It is not a runtime request or environment API.
#[cfg(feature = "m3-conformance-testkit")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConformanceZoneTransitionKind {
    BattlefieldToOwnerGraveyard,
    LibraryTopToOwnerHand,
}

/// Narrow conformance bridge to the private production-owned seam.
#[cfg(feature = "m3-conformance-testkit")]
pub fn execute_selected_zone_transition_for_conformance(
    state: &EngineState,
    object: GameObjectId,
    claimed_from: ZoneLocation,
    claimed_to: ZoneLocation,
    kind: ConformanceZoneTransitionKind,
) -> Result<TransitionResult, KernelExecutionError> {
    let kind = match kind {
        ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard => {
            SelectedZoneTransitionKind::BattlefieldToOwnerGraveyard
        }
        ConformanceZoneTransitionKind::LibraryTopToOwnerHand => {
            SelectedZoneTransitionKind::LibraryTopToOwnerHand
        }
    };
    execute_selected_zone_transition(
        state,
        &SelectedZoneTransitionRequest {
            object,
            kind,
            claimed_from,
            claimed_to,
        },
    )
}
