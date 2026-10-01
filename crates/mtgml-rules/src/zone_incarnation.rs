//! The one zone-move primitive: a card leaves one zone and a new incarnation
//! enters another, with the identity and knowledge updates every perspective
//! observes. Callers stage moves in a workspace they own.

use mtgml_model::GameObjectId;
use mtgml_state::{EngineState, ZoneLocation, ZonePosition, ZoneTransition};

use crate::errors::ZoneIncarnationError;
use crate::KernelExecutionError;
use mtgml_state::{
    IdentityMutationV1, KnowledgeAcquisitionCause, KnowledgeAcquisitionReason,
    KnowledgeHistoryChannel, KnowledgeMutationV1, KnownLocationFactV2, PerspectiveLifecycleAuditV1,
    PerspectiveLifecycleMutationV1,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SelectedZoneTransitionKind {
    LibraryTopToOwnerHand,
    /// A discard (CR 701.9a): hand to the top of the owner's public graveyard.
    HandToOwnerGraveyard,
    /// A hand card to the bottom of its owner's face-down library (CR 103.5).
    /// The owner stops tracking it, for `reason`.
    HandToOwnerLibraryBottom {
        reason: mtgml_state::KnowledgeInvalidationReason,
    },
    /// Casting a spell (CR 601.2a): a hand card to the stack, as a new public
    /// incarnation. Every player sees it appear.
    HandToStack,
    /// A permanent spell resolving (CR 608.3a): the card on the stack enters
    /// the battlefield as a new incarnation under `controller`'s control.
    StackToBattlefield {
        controller: mtgml_model::PlayerId,
    },
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
    if matches!(
        kind,
        SelectedZoneTransitionKind::LibraryTopToOwnerHand
            | SelectedZoneTransitionKind::HandToOwnerLibraryBottom { .. }
    ) && state
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

/// One observable step of a zone move, in emission order: the transition
/// itself, then each perspective's lifecycle occurrence of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ZoneMoveEvent {
    Transition(Box<ZoneTransition>),
    Occurrence(PerspectiveLifecycleAuditV1),
    /// One library was shuffled (CR 701.24); the trusted audit of the draw.
    Shuffle(Box<LibraryShuffleAudit>),
    /// A rule event of the transition. Events every player observes are
    /// followed by each player's occurrence of it; one that no player
    /// observes (see `record_unobserved`) has none.
    Public(Box<crate::AuthoritativeRuleEventKind>),
}

/// The draw of one library shuffle: the stream, its cursors, the words it
/// consumed and the resulting order, top to bottom.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LibraryShuffleAudit {
    pub player: mtgml_model::PlayerId,
    pub stream: mtgml_random::RandomStreamKeyV1,
    pub cursor_before: u64,
    pub cursor_after: u64,
    pub raw_words_consumed: u64,
    pub top_to_bottom: Vec<GameObjectId>,
}

fn emit_perspective_occurrence(
    candidate: &mut EngineState,
    events: &mut Vec<ZoneMoveEvent>,
    lifecycle: PerspectiveLifecycleAuditV1,
) -> Result<(), KernelExecutionError> {
    mtgml_state::apply_perspective_lifecycle(candidate, &lifecycle)?;
    events.push(ZoneMoveEvent::Occurrence(lifecycle));
    Ok(())
}

/// Apply one selected S2 move to a caller-owned scratch workspace.
///
/// The candidate revision and outer event cursor belong to the coordinator.
/// This primitive never creates a `PredecessorTransitionResult`, increments revision, or
/// validates a potentially incomplete intermediate workspace. It stages all
/// work in a private clone so an error leaves both the candidate and its event
/// vector untouched.
pub(crate) fn apply_selected_zone_transition_in_workspace(
    candidate: &mut EngineState,
    request: &SelectedZoneTransitionRequest,
    events: &mut Vec<ZoneMoveEvent>,
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
        SelectedZoneTransitionKind::LibraryTopToOwnerHand => {
            actual_from.zone == mtgml_model::ZoneKind::Library
                && actual_from.player == Some(old_object.owner)
                && matches!(actual_from.position, ZonePosition::Top { .. })
                && actual_from.visibility == mtgml_state::VisibilityPartition::FaceDown
                && actual_from.partition.is_none()
        }
        SelectedZoneTransitionKind::HandToOwnerGraveyard
        | SelectedZoneTransitionKind::HandToOwnerLibraryBottom { .. }
        | SelectedZoneTransitionKind::HandToStack => {
            actual_from.zone == mtgml_model::ZoneKind::Hand
                && actual_from.player == Some(old_object.owner)
                && matches!(
                    actual_from.position,
                    ZonePosition::Unordered | ZonePosition::Top { .. }
                )
                && actual_from.visibility == mtgml_state::VisibilityPartition::OwnerOnly
                && actual_from.partition.is_none()
        }
        SelectedZoneTransitionKind::StackToBattlefield { .. } => {
            actual_from.zone == mtgml_model::ZoneKind::Stack
                && actual_from.player.is_none()
                && actual_from.position == ZonePosition::Unordered
                && actual_from.visibility == mtgml_state::VisibilityPartition::Public
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
        SelectedZoneTransitionKind::HandToStack => ZoneLocation {
            zone: mtgml_model::ZoneKind::Stack,
            player: None,
            position: ZonePosition::Unordered,
            visibility: mtgml_state::VisibilityPartition::Public,
            partition: None,
        },
        SelectedZoneTransitionKind::StackToBattlefield { .. } => ZoneLocation {
            zone: mtgml_model::ZoneKind::Battlefield,
            player: None,
            position: ZonePosition::Unordered,
            visibility: mtgml_state::VisibilityPartition::Public,
            partition: None,
        },
        SelectedZoneTransitionKind::HandToOwnerGraveyard => ZoneLocation {
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
        SelectedZoneTransitionKind::HandToOwnerLibraryBottom { .. } => {
            let library = ZoneLocation {
                zone: mtgml_model::ZoneKind::Library,
                player: Some(old_object.owner),
                position: ZonePosition::Top { offset: 0 },
                visibility: mtgml_state::VisibilityPartition::FaceDown,
                partition: None,
            };
            let below = state
                .zones
                .ordered_zones
                .get(&library.key())
                .map_or(0, Vec::len);
            ZoneLocation {
                position: ZonePosition::Top {
                    offset: u32::try_from(below).map_err(|_| {
                        KernelExecutionError::ZoneIncarnation(
                            ZoneIncarnationError::GraveyardOffsetOverflow,
                        )
                    })?,
                },
                ..library
            }
        }
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
        SelectedZoneTransitionKind::HandToOwnerGraveyard
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

    if let (
        SelectedZoneTransitionKind::HandToOwnerGraveyard
        | SelectedZoneTransitionKind::HandToOwnerLibraryBottom { .. }
        | SelectedZoneTransitionKind::HandToStack,
        ZonePosition::Top { offset },
    ) = (request.kind, actual_from.position)
    {
        // An ordered hand closes the gap the discarded card leaves.
        let source_key = actual_from.key();
        let ordered = next.zones.ordered_zones.get_mut(&source_key).ok_or(
            KernelExecutionError::ZoneIncarnation(ZoneIncarnationError::UnadmittedSourceFamily),
        )?;
        let index = ordered
            .iter()
            .position(|member| *member == request.object)
            .ok_or(KernelExecutionError::ZoneIncarnation(
                ZoneIncarnationError::UnadmittedSourceFamily,
            ))?;
        ordered.remove(index);
        let members = ordered.clone();
        if members.is_empty() {
            next.zones.ordered_zones.remove(&source_key);
        }
        for member in members {
            let location = next.zones.locations.get_mut(&member).ok_or(
                KernelExecutionError::ZoneIncarnation(ZoneIncarnationError::UnadmittedSourceFamily),
            )?;
            if let ZonePosition::Top {
                offset: member_offset,
            } = location.position
            {
                if member_offset > offset {
                    location.position = ZonePosition::Top {
                        offset: member_offset - 1,
                    };
                }
            }
        }
    }
    match request.kind {
        SelectedZoneTransitionKind::HandToOwnerGraveyard => {
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
        SelectedZoneTransitionKind::HandToOwnerLibraryBottom { .. } => {
            next.zones
                .ordered_zones
                .entry(required_to.key())
                .or_default()
                .push(new_object_id);
        }
        // The stack and the battlefield are unordered zones.
        SelectedZoneTransitionKind::HandToStack
        | SelectedZoneTransitionKind::StackToBattlefield { .. } => {}
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
            controller: match request.kind {
                SelectedZoneTransitionKind::StackToBattlefield { controller } => controller,
                _ => old_object.owner,
            },
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
    let mut staged_events = vec![ZoneMoveEvent::Transition(Box::new(transition.clone()))];
    let new_object = new_object_id;
    let to_location = transition.to.clone();
    match request.kind {
        SelectedZoneTransitionKind::HandToOwnerGraveyard
        | SelectedZoneTransitionKind::HandToStack
        | SelectedZoneTransitionKind::StackToBattlefield { .. } => {
            // The destination is public: every perspective learns the card. A
            // perspective that tracked the source card follows it; any other
            // perspective sees it appear. In the graveyard, known members
            // shifted down by the new top card get their exact new locations
            // in the same occurrence; the stack and the battlefield have no
            // order to shift.
            let shifted_members = if matches!(
                request.kind,
                SelectedZoneTransitionKind::HandToOwnerGraveyard
            ) {
                state
                    .zones
                    .ordered_zones
                    .get(&graveyard_key)
                    .cloned()
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
            for perspective in state.core.players.keys().copied() {
                let identity = state
                    .perspective_identities
                    .players
                    .get(&perspective)
                    .ok_or(KernelExecutionError::ZoneIncarnation(
                        ZoneIncarnationError::PerspectiveKnowledgeMismatch,
                    ))?;
                let knowledge = state.knowledge.players.get(&perspective).ok_or(
                    KernelExecutionError::ZoneIncarnation(
                        ZoneIncarnationError::PerspectiveKnowledgeMismatch,
                    ),
                )?;
                let sequence = knowledge.next_visible_sequence;
                let provenance = KnowledgeAcquisitionReason::Observed {
                    channel: KnowledgeHistoryChannel::Public,
                    sequence,
                    cause: KnowledgeAcquisitionCause::PublicEvent,
                };
                let mut updates = Vec::new();
                for member in &shifted_members {
                    let Some(opaque) = identity.object_to_opaque.get(member).copied() else {
                        continue;
                    };
                    let record = knowledge.active.get(&opaque).ok_or(
                        KernelExecutionError::ZoneIncarnation(
                            ZoneIncarnationError::PerspectiveKnowledgeMismatch,
                        ),
                    )?;
                    if record.known_location.is_some() {
                        let location = next.zones.locations.get(member).cloned().ok_or(
                            KernelExecutionError::ZoneIncarnation(
                                ZoneIncarnationError::PerspectiveKnowledgeMismatch,
                            ),
                        )?;
                        updates.push(mtgml_state::KnowledgeLocationUpdateV1 {
                            opaque,
                            fact: KnownLocationFactV2 {
                                location,
                                provenance,
                            },
                        });
                    }
                }
                let mutation = match identity.object_to_opaque.get(&request.object).copied() {
                    Some(opaque) => {
                        if !knowledge.active.contains_key(&opaque) {
                            return Err(KernelExecutionError::ZoneIncarnation(
                                ZoneIncarnationError::PerspectiveKnowledgeMismatch,
                            ));
                        }
                        let fact = KnownLocationFactV2 {
                            location: to_location.clone(),
                            provenance,
                        };
                        let knowledge = if updates.is_empty() {
                            KnowledgeMutationV1::UpdateLocation { opaque, fact }
                        } else {
                            updates.push(mtgml_state::KnowledgeLocationUpdateV1 { opaque, fact });
                            updates.sort_by_key(|update| update.opaque);
                            KnowledgeMutationV1::UpdateLocations { updates }
                        };
                        PerspectiveLifecycleMutationV1 {
                            identity: IdentityMutationV1::Remap {
                                opaque,
                                from_object: request.object,
                                to_object: new_object,
                            },
                            knowledge: Some(knowledge),
                        }
                    }
                    None => {
                        let opaque = identity.next_opaque_object_id;
                        let definition = Some(old_object.card_definition);
                        let location = Some(to_location.clone());
                        let knowledge = if updates.is_empty() {
                            KnowledgeMutationV1::Acquire {
                                opaque,
                                definition,
                                location,
                                acquisition: provenance,
                            }
                        } else {
                            updates.sort_by_key(|update| update.opaque);
                            KnowledgeMutationV1::AcquireShiftingKnownMembers {
                                opaque,
                                definition,
                                location,
                                acquisition: provenance,
                                updates,
                            }
                        };
                        PerspectiveLifecycleMutationV1 {
                            identity: IdentityMutationV1::Allocate {
                                opaque,
                                object: new_object,
                            },
                            knowledge: Some(knowledge),
                        }
                    }
                };
                emit_perspective_occurrence(
                    &mut next,
                    &mut staged_events,
                    PerspectiveLifecycleAuditV1 {
                        perspective,
                        sequence,
                        mutation,
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
            let (identity_mutation, knowledge_mutation) =
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
            emit_perspective_occurrence(&mut next, &mut staged_events, lifecycle)?;
        }
        // The card disappears face down into the library: its owner, who
        // knew it in hand, sees it go and no longer tracks it. No other
        // perspective tracked it (checked above).
        SelectedZoneTransitionKind::HandToOwnerLibraryBottom { reason } => {
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
            let opaque = identity
                .object_to_opaque
                .get(&request.object)
                .copied()
                .ok_or(KernelExecutionError::ZoneIncarnation(
                    ZoneIncarnationError::PerspectiveKnowledgeMismatch,
                ))?;
            let sequence = knowledge.next_visible_sequence;
            let lifecycle = PerspectiveLifecycleAuditV1 {
                perspective: owner,
                sequence,
                mutation: PerspectiveLifecycleMutationV1 {
                    identity: IdentityMutationV1::Retire {
                        opaque,
                        object: request.object,
                    },
                    knowledge: Some(KnowledgeMutationV1::Invalidate {
                        opaque,
                        reason,
                        invalidation_provenance: KnowledgeAcquisitionReason::Observed {
                            channel: KnowledgeHistoryChannel::Private,
                            sequence,
                            cause: KnowledgeAcquisitionCause::OwnPrivateIdentity,
                        },
                    }),
                },
            };
            emit_perspective_occurrence(&mut next, &mut staged_events, lifecycle)?;
        }
    }
    *candidate = next;
    events.extend(staged_events);
    Ok(transition)
}
