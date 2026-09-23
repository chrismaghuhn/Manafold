//! Private rules-owned entry seam for the selected zone-incarnation capability.
//!
//! This module owns the single future production implementation point. The
//! conformance facade below only translates its closed test vocabulary and
//! delegates here; it contains no transition behavior.

use mtgml_model::{GameObjectId, StateRevision};
use mtgml_state::{validate_engine_state, EngineState, ZoneLocation, ZonePosition, ZoneTransition};

use crate::errors::ZoneIncarnationError;
use crate::events::{AuthoritativeRuleEvent, AuthoritativeRuleEventKind};
use crate::product::build_accepted_product;
use crate::{KernelExecutionError, TransitionResult};

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

/// The one future implementation point for both selected transition families.
///
/// Executes one admitted selected zone transition through the ordinary
/// accepted-product path. Task-3 lifecycle/reference closures remain owned by
/// their later integration step; this core executor requires the candidate to
/// satisfy the existing complete EngineState validator.
pub(crate) fn execute_selected_zone_transition(
    state: &EngineState,
    request: &SelectedZoneTransitionRequest,
) -> Result<TransitionResult, KernelExecutionError> {
    validate_engine_state(state).map_err(KernelExecutionError::BeforeState)?;

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
                for member in ordered.iter().copied() {
                    let location = next.zones.locations.get_mut(&member).ok_or(
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
    next.revision = StateRevision(
        state
            .revision
            .0
            .checked_add(1)
            .ok_or(KernelExecutionError::RevisionOverflow)?,
    );
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
        to: required_to,
        last_known,
        new_snapshot,
    };
    let event = AuthoritativeRuleEvent {
        event_id: state.allocators.next_rule_event_id,
        state_revision: next.revision,
        event: AuthoritativeRuleEventKind::ZoneTransition {
            transition: Box::new(transition),
        },
    };
    build_accepted_product(state, next, vec![event], |_| Ok(()))
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
