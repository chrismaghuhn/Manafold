//! Rules-owned read-only validation for the bounded S3.A SBA-order continuation.
//!
//! This module derives the currently applicable action set from immutable
//! Magic state. It does not produce decisions or transitions and never mutates
//! the state.

use std::collections::BTreeMap;

use mtgml_model::{PlayerId, ZoneKind};
use mtgml_state::{
    validate_engine_state, BaseCharacteristics, ContinuationPayloadV2, EngineState,
    FoundationSourceKind, SbaObjectCauseV1, SbaSelectedActionV1,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SbaContinuationValidationError {
    NotS3AConformanceCandidate,
    NoActiveSbaContinuation,
    UnsupportedSbaProfile,
    SelectedActionSetMismatch,
    ApnapOwnersMismatch,
}

pub(crate) fn validate_sba_order_continuation(
    state: &EngineState,
) -> Result<(), SbaContinuationValidationError> {
    validate_engine_state(state)
        .map_err(|_| SbaContinuationValidationError::UnsupportedSbaProfile)?;
    if !matches!(state.core.priority, mtgml_state::PriorityState::None) {
        return Err(SbaContinuationValidationError::UnsupportedSbaProfile);
    }
    if state.core.players.len() != 2 {
        return Err(SbaContinuationValidationError::UnsupportedSbaProfile);
    }

    let Some(continuation) = state.execution.continuations.values().next() else {
        return Err(SbaContinuationValidationError::NoActiveSbaContinuation);
    };
    let ContinuationPayloadV2::MagicSbaGraveyardOrderV1 {
        selected_sba_actions,
        apnap_owners,
        next_owner_index,
        ..
    } = &continuation.payload
    else {
        return Err(SbaContinuationValidationError::NoActiveSbaContinuation);
    };

    let current_actions = derive_bounded_sba_actions(state)?;
    if &current_actions != selected_sba_actions {
        return Err(SbaContinuationValidationError::SelectedActionSetMismatch);
    }

    let required_owners = derive_order_owners(state, &current_actions);
    if &required_owners != apnap_owners
        || required_owners.get(*next_owner_index as usize) != Some(&continuation.actor)
    {
        return Err(SbaContinuationValidationError::ApnapOwnersMismatch);
    }
    Ok(())
}

fn derive_bounded_sba_actions(
    state: &EngineState,
) -> Result<Vec<SbaSelectedActionV1>, SbaContinuationValidationError> {
    let mut actions = state
        .core
        .players
        .iter()
        .filter_map(|(player, status)| {
            (status.life <= 0 && !status.has_lost)
                .then_some(SbaSelectedActionV1::PlayerLoses { player: *player })
        })
        .collect::<Vec<_>>();

    let mut object_actions = Vec::new();
    for (object_id, object) in &state.zones.objects {
        let location = state
            .zones
            .locations
            .get(object_id)
            .ok_or(SbaContinuationValidationError::UnsupportedSbaProfile)?;
        if location.zone != ZoneKind::Battlefield {
            continue;
        }
        if object.face_down {
            return Err(SbaContinuationValidationError::UnsupportedSbaProfile);
        }
        let source = state
            .foundation_sources
            .get(object_id)
            .ok_or(SbaContinuationValidationError::UnsupportedSbaProfile)?;
        if source.source_kind != FoundationSourceKind::Creature {
            return Err(SbaContinuationValidationError::UnsupportedSbaProfile);
        }
        let BaseCharacteristics::Simple { toughness, .. } = source.base_characteristics;
        let mut causes = Vec::with_capacity(2);
        if toughness <= 0 {
            causes.push(SbaObjectCauseV1::ZeroToughness);
        }
        if toughness > 0 && source.marked_damage >= toughness as u64 {
            causes.push(SbaObjectCauseV1::LethalDamage);
        }
        if !causes.is_empty() {
            object_actions.push(SbaSelectedActionV1::ObjectToOwnerGraveyard {
                object: *object_id,
                causes,
            });
        }
    }
    actions.extend(object_actions);
    Ok(actions)
}

fn derive_order_owners(state: &EngineState, actions: &[SbaSelectedActionV1]) -> Vec<PlayerId> {
    let mut counts = BTreeMap::<PlayerId, usize>::new();
    for action in actions {
        if let SbaSelectedActionV1::ObjectToOwnerGraveyard { object, .. } = action {
            if let Some(owner) = state.zones.objects.get(object).map(|object| object.owner) {
                *counts.entry(owner).or_default() += 1;
            }
        }
    }
    let active = state.core.active_player;
    let mut owners = Vec::new();
    if counts.get(&active).copied().unwrap_or_default() >= 2 {
        owners.push(active);
    }
    let mut remaining = counts
        .into_iter()
        .filter_map(|(owner, count)| (owner != active && count >= 2).then_some(owner))
        .collect::<Vec<_>>();
    remaining.sort_unstable();
    owners.extend(remaining);
    owners
}
