//! Rules-owned read-only validation for the bounded S3.A SBA-order continuation.
//!
//! This module derives the currently applicable action set from immutable
//! Magic state. It does not produce decisions or transitions and never mutates
//! the state.

use std::collections::BTreeMap;

use mtgml_model::{PlayerId, ZoneKind};
use mtgml_state::{
    validate_engine_state, BaseCharacteristics, ContinuationPayloadV2, EngineState, FormatState,
    FoundationSourceKind, PriorityState, SbaObjectCauseV1, SbaSelectedActionV1, TurnPosition,
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

    validate_s3_a_support_profile(state)?;
    validate_engine_state(state)
        .map_err(|_| SbaContinuationValidationError::UnsupportedSbaProfile)?;

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

/// Proves the closed S3.A semantic support profile before Foundation source
/// facts are treated as current characteristics. This is deliberately
/// separate from S1 turn-structure admission: an active Order continuation
/// and bounded combat state are valid S3.A inputs.
pub(crate) fn validate_s3_a_support_profile(
    state: &EngineState,
) -> Result<(), SbaContinuationValidationError> {
    if state.core.players.len() != 2
        || !state.core.players.contains_key(&state.core.active_player)
        || !matches!(state.format, FormatState::None)
        || !matches!(state.core.priority, PriorityState::None)
        || !is_supported_sba_boundary(state.core.position)
        || state.core.players.values().any(|player| player.has_lost)
        || !state.execution.effects.is_empty()
        || !state.execution.waiting_triggers.is_empty()
        || !state.execution.delayed_effects.is_empty()
        || !state.zones.stack_records.is_empty()
        || !state.zones.stack_order.is_empty()
        || state
            .zones
            .locations
            .values()
            .any(|location| location.zone == ZoneKind::Stack)
        || state
            .perspective_identities
            .players
            .values()
            .any(|identity| {
                !identity.opaque_to_ability.is_empty() || !identity.ability_to_opaque.is_empty()
            })
    {
        return Err(SbaContinuationValidationError::UnsupportedSbaProfile);
    }

    // EngineState carries opaque CardDefinitionId values, not a Card IR or
    // any card-rule/action payload. The only currently representable live
    // ability surface is the perspective ability mapping checked above;
    // layers, copy effects, keywords, replacement/prevention, mana, and
    // special actions have no state fields. Do not infer those semantics from
    // an ID or add a second registry here.
    for (object_id, object) in &state.zones.objects {
        if object.physical_card.is_none() {
            // The selected profile has no token semantics.
            return Err(SbaContinuationValidationError::UnsupportedSbaProfile);
        }
        let location = state
            .zones
            .locations
            .get(object_id)
            .ok_or(SbaContinuationValidationError::UnsupportedSbaProfile)?;
        if location.zone == ZoneKind::Battlefield {
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
            // Base values are usable only because this closed state model has
            // no layer, copy, continuous-effect, or characteristic modifier
            // representation; effects and all executable action surfaces were
            // checked absent above.
            let BaseCharacteristics::Simple { .. } = source.base_characteristics;
        } else if state.foundation_sources.contains_key(object_id) {
            return Err(SbaContinuationValidationError::UnsupportedSbaProfile);
        }
    }

    Ok(())
}

fn is_supported_sba_boundary(position: TurnPosition) -> bool {
    // Foundation V2 runs SBA before each priority window and at the selected
    // cleanup check. Match the closed enum exhaustively so adding a temporal
    // state cannot silently widen this semantic profile.
    match position {
        TurnPosition::Beginning {
            step: mtgml_state::BeginningStep::Untap,
        } => false,
        TurnPosition::Beginning {
            step: mtgml_state::BeginningStep::Upkeep | mtgml_state::BeginningStep::Draw,
        }
        | TurnPosition::PrecombatMain
        | TurnPosition::Combat {
            step:
                mtgml_state::CombatStep::BeginningOfCombat
                | mtgml_state::CombatStep::DeclareAttackers
                | mtgml_state::CombatStep::DeclareBlockers
                | mtgml_state::CombatStep::CombatDamage
                | mtgml_state::CombatStep::EndOfCombat,
        }
        | TurnPosition::PostcombatMain
        | TurnPosition::Ending {
            step: mtgml_state::EndingStep::EndStep | mtgml_state::EndingStep::Cleanup,
        } => true,
    }
}

fn derive_bounded_sba_actions(
    state: &EngineState,
) -> Result<Vec<SbaSelectedActionV1>, SbaContinuationValidationError> {
    let mut actions = state
        .core
        .players
        .iter()
        .filter_map(|(player, status)| {
            (status.life <= 0).then_some(SbaSelectedActionV1::PlayerLoses { player: *player })
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
