//! Rules-owned read-only validation for the bounded S3.A SBA-order continuation.
//!
//! This module derives the currently applicable action set from immutable
//! Magic state. It does not produce decisions or transitions and never mutates
//! the state.

use std::collections::BTreeMap;

use mtgml_model::{PlayerId, ZoneKind};
use mtgml_state::{
    validate_engine_state, BaseCharacteristics, ContinuationPayloadV2, EngineState, FormatState,
    FoundationSourceKind, PriorityState, SbaGraveyardOwnerOrderV1, SbaObjectCauseV1,
    SbaSelectedActionV1, TurnPosition,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // Used by the non-default conformance-testkit adapter.
pub enum SbaContinuationValidationError {
    NotS3AConformanceCandidate,
    NoActiveSbaContinuation,
    UnsupportedSbaProfile,
    SelectedActionSetMismatch,
    ApnapOwnersMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SbaOrderRoundPlan {
    pub(crate) selected_sba_actions: Vec<SbaSelectedActionV1>,
    pub(crate) apnap_owners: Vec<PlayerId>,
}

/// Resolve only already-authorized owner permutations into deterministic S2
/// invocation order. This never chooses a player's order: multi-card groups
/// require the exact persisted/accepted permutation; singleton groups have a
/// unique order. S2 inserts at top, so each chosen top-to-bottom group is
/// returned in reverse invocation order.
pub(crate) fn ordered_sba_objects_for_s2(
    state: &EngineState,
    plan: &SbaOrderRoundPlan,
    orders: &[SbaGraveyardOwnerOrderV1],
) -> Result<Vec<mtgml_model::GameObjectId>, SbaContinuationValidationError> {
    let mut groups = BTreeMap::<PlayerId, Vec<mtgml_model::GameObjectId>>::new();
    for action in &plan.selected_sba_actions {
        if let SbaSelectedActionV1::ObjectToOwnerGraveyard { object, .. } = action {
            let owner = state
                .zones
                .objects
                .get(object)
                .ok_or(SbaContinuationValidationError::UnsupportedSbaProfile)?
                .owner;
            groups.entry(owner).or_default().push(*object);
        }
    }

    let mut explicit = BTreeMap::new();
    for order in orders {
        if explicit
            .insert(order.owner, order.top_to_bottom.clone())
            .is_some()
        {
            return Err(SbaContinuationValidationError::SelectedActionSetMismatch);
        }
    }
    let required: std::collections::BTreeSet<_> = groups
        .iter()
        .filter_map(|(owner, objects)| (objects.len() >= 2).then_some(*owner))
        .collect();
    let plan_owners: std::collections::BTreeSet<_> = plan.apnap_owners.iter().copied().collect();
    let explicit_owners: std::collections::BTreeSet<_> = explicit.keys().copied().collect();
    if plan_owners.len() != plan.apnap_owners.len()
        || required != plan_owners
        || explicit.len() != plan.apnap_owners.len()
        || explicit_owners != plan_owners
    {
        return Err(SbaContinuationValidationError::ApnapOwnersMismatch);
    }

    let mut owners = Vec::new();
    if groups.contains_key(&state.core.active_player) {
        owners.push(state.core.active_player);
    }
    owners.extend(
        groups
            .keys()
            .copied()
            .filter(|owner| *owner != state.core.active_player),
    );
    let mut invocation_order = Vec::new();
    for owner in owners {
        let objects = groups
            .get(&owner)
            .ok_or(SbaContinuationValidationError::UnsupportedSbaProfile)?;
        let top_to_bottom = if objects.len() >= 2 {
            let chosen = explicit
                .get(&owner)
                .ok_or(SbaContinuationValidationError::ApnapOwnersMismatch)?;
            let expected: std::collections::BTreeSet<_> = objects.iter().copied().collect();
            let actual: std::collections::BTreeSet<_> = chosen.iter().copied().collect();
            if actual.len() != chosen.len() || actual != expected {
                return Err(SbaContinuationValidationError::SelectedActionSetMismatch);
            }
            chosen.as_slice()
        } else {
            objects.as_slice()
        };
        invocation_order.extend(top_to_bottom.iter().rev().copied());
    }
    Ok(invocation_order)
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

    let current_plan = derive_bounded_sba_round_plan(state)?;
    if current_plan.selected_sba_actions != *selected_sba_actions {
        return Err(SbaContinuationValidationError::SelectedActionSetMismatch);
    }

    if current_plan.apnap_owners != *apnap_owners
        || current_plan.apnap_owners.get(*next_owner_index as usize) != Some(&continuation.actor)
    {
        return Err(SbaContinuationValidationError::ApnapOwnersMismatch);
    }
    Ok(())
}

/// Sole derivation entry for both fresh SBA ordering stages and saved-plan
/// revalidation. The result contains no cached or caller-supplied facts.
pub(crate) fn derive_bounded_sba_round_plan(
    state: &EngineState,
) -> Result<SbaOrderRoundPlan, SbaContinuationValidationError> {
    validate_s3_a_support_profile(state)?;
    validate_engine_state(state)
        .map_err(|_| SbaContinuationValidationError::UnsupportedSbaProfile)?;
    let selected_sba_actions = derive_bounded_sba_actions(state)?;
    validate_selected_combat_boundary(state, &selected_sba_actions)?;
    let apnap_owners = derive_order_owners(state, &selected_sba_actions);
    Ok(SbaOrderRoundPlan {
        selected_sba_actions,
        apnap_owners,
    })
}

fn validate_selected_combat_boundary(
    state: &EngineState,
    actions: &[SbaSelectedActionV1],
) -> Result<(), SbaContinuationValidationError> {
    let Some(combat) = &state.combat else {
        return Ok(());
    };
    let selected = actions.iter().filter_map(|action| match action {
        SbaSelectedActionV1::ObjectToOwnerGraveyard { object, .. } => Some(*object),
        SbaSelectedActionV1::PlayerLoses { .. } => None,
    });
    let selected: std::collections::BTreeSet<_> = selected.collect();
    let participant_selected = combat
        .attackers
        .iter()
        .any(|object| selected.contains(object))
        || combat.blockers.iter().any(|(attacker, blocker)| {
            selected.contains(attacker) || blocker.is_some_and(|object| selected.contains(&object))
        });
    if participant_selected
        && !matches!(
            state.core.position,
            TurnPosition::Combat {
                step: mtgml_state::CombatStep::CombatDamage
            }
        )
    {
        return Err(SbaContinuationValidationError::UnsupportedSbaProfile);
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
    validate_s3_a_state_profile(state, false)
}

pub(crate) fn validate_s3_a_state_profile(
    state: &EngineState,
    allow_lost_players: bool,
) -> Result<(), SbaContinuationValidationError> {
    if state.core.players.len() != 2
        || !state.core.players.contains_key(&state.core.active_player)
        || !matches!(state.format, FormatState::None)
        || !matches!(state.core.priority, PriorityState::None)
        || !is_supported_sba_boundary(state.core.position)
        || (!allow_lost_players && state.core.players.values().any(|player| player.has_lost))
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

pub(crate) fn derive_bounded_sba_actions(
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
