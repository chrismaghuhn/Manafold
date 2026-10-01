//! Ownership: current-M2 synthetic assembly continuation program material
//! (records, stages, payload, and its shape/coherence validation).

use std::collections::{BTreeMap, BTreeSet};

use mtgml_model::{GameObjectId, PlayerId, StateRevision};
use serde::{Deserialize, Serialize};

use crate::engine_state_shape::{EngineStateShapeViolation, SYNTHETIC_COUNT_MAX};
use crate::zones::GameObject;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssemblyStageV2 {
    ChooseCount,
    ChooseMembers,
    OrderMembers,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SbaObjectCauseV1 {
    ZeroToughness,
    LethalDamage,
}

/// Complete selected action family for the bounded SBA continuation.
/// Variant order is canonical: player losses by PlayerId, then Graveyard
/// object actions by GameObjectId.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SbaSelectedActionV1 {
    PlayerLoses {
        player: PlayerId,
    },
    ObjectToOwnerGraveyard {
        object: GameObjectId,
        causes: Vec<SbaObjectCauseV1>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SbaGraveyardOwnerOrderV1 {
    pub owner: PlayerId,
    /// First member is the topmost new Graveyard card.
    pub top_to_bottom: Vec<GameObjectId>,
}

/// Stage-payload invariants of the one frozen synthetic assembly payload.
///
/// Every reachable continuation state must have exactly one unambiguous
/// semantic interpretation:
///
/// - `ChooseCount`: nothing decided yet;
/// - `ChooseMembers`: the numeric count is decided, the member set is not;
/// - `OrderMembers`: the member set is decided in canonical set form, the
///   semantic order is not (it lives only in the pending stage answer).
///
/// Ordered partial data never persists: completion removes the continuation.
pub(super) fn validate_synthetic_assembly(
    stage: AssemblyStageV2,
    selected_count: Option<u32>,
    selected_piece_keys: &[u32],
    ordered_piece_keys: &[u32],
) -> Result<(), EngineStateShapeViolation> {
    let canonical_set = |values: &[u32]| values.windows(2).all(|window| window[0] < window[1]);
    match stage {
        AssemblyStageV2::ChooseCount => {
            if selected_count.is_some()
                || !selected_piece_keys.is_empty()
                || !ordered_piece_keys.is_empty()
            {
                return Err(EngineStateShapeViolation::Knowledge);
            }
        }
        AssemblyStageV2::ChooseMembers => {
            if selected_count.is_none()
                || !selected_piece_keys.is_empty()
                || !ordered_piece_keys.is_empty()
            {
                return Err(EngineStateShapeViolation::Knowledge);
            }
        }
        AssemblyStageV2::OrderMembers => {
            let Some(count) = selected_count else {
                return Err(EngineStateShapeViolation::Knowledge);
            };
            if !ordered_piece_keys.is_empty()
                || selected_piece_keys.len() != count as usize
                || !canonical_set(selected_piece_keys)
            {
                return Err(EngineStateShapeViolation::Knowledge);
            }
        }
    }
    // A decided count can only originate from the supported ChooseCount
    // interval; anything else was never offered by this program.
    if let Some(count) = selected_count {
        if count > SYNTHETIC_COUNT_MAX {
            return Err(EngineStateShapeViolation::Knowledge);
        }
    }
    Ok(())
}

pub(super) struct MagicSbaGraveyardOrderValidation<'a> {
    pub round_start_revision: StateRevision,
    pub continuation_created_at_revision: StateRevision,
    pub selected_sba_actions: &'a [SbaSelectedActionV1],
    pub apnap_owners: &'a [PlayerId],
    pub next_owner_index: u32,
    pub completed_owner_orders: &'a [SbaGraveyardOwnerOrderV1],
    pub current_revision: StateRevision,
    pub players: &'a BTreeSet<PlayerId>,
    pub objects: &'a BTreeMap<GameObjectId, GameObject>,
}

pub(super) fn validate_magic_sba_graveyard_order(
    validation: MagicSbaGraveyardOrderValidation<'_>,
) -> Result<BTreeMap<PlayerId, Vec<GameObjectId>>, EngineStateShapeViolation> {
    let MagicSbaGraveyardOrderValidation {
        round_start_revision,
        continuation_created_at_revision,
        selected_sba_actions,
        apnap_owners,
        next_owner_index,
        completed_owner_orders,
        current_revision,
        players,
        objects,
    } = validation;
    let expected_created_revision = round_start_revision
        .0
        .checked_add(1)
        .ok_or(EngineStateShapeViolation::ContinuationRevision)?;
    let stage_count = u64::from(next_owner_index);
    let expected_current_revision = expected_created_revision
        .checked_add(stage_count)
        .ok_or(EngineStateShapeViolation::ContinuationRevision)?;
    if continuation_created_at_revision.0 != expected_created_revision
        || current_revision.0 != expected_current_revision
    {
        return Err(EngineStateShapeViolation::ContinuationRevision);
    }
    if selected_sba_actions.is_empty()
        || selected_sba_actions
            .windows(2)
            .any(|window| window[0] >= window[1])
    {
        return Err(EngineStateShapeViolation::MagicContinuation);
    }

    let mut losing_players = BTreeSet::new();
    let mut graveyard_objects = BTreeSet::new();
    for action in selected_sba_actions {
        match action {
            SbaSelectedActionV1::PlayerLoses { player } => {
                if !players.contains(player) || !losing_players.insert(*player) {
                    return Err(EngineStateShapeViolation::MagicContinuation);
                }
            }
            SbaSelectedActionV1::ObjectToOwnerGraveyard { object, causes } => {
                if !graveyard_objects.insert(*object)
                    || causes.is_empty()
                    || causes.windows(2).any(|window| window[0] >= window[1])
                    || !objects.contains_key(object)
                {
                    return Err(EngineStateShapeViolation::MagicContinuation);
                }
            }
        }
    }

    let next_owner_index_usize = usize::try_from(next_owner_index)
        .map_err(|_| EngineStateShapeViolation::MagicContinuation)?;
    if apnap_owners.is_empty()
        || apnap_owners.iter().any(|owner| !players.contains(owner))
        || apnap_owners.iter().copied().collect::<BTreeSet<_>>().len() != apnap_owners.len()
        || next_owner_index_usize != completed_owner_orders.len()
        || next_owner_index_usize >= apnap_owners.len()
    {
        return Err(EngineStateShapeViolation::MagicContinuation);
    }

    let mut objects_by_owner = BTreeMap::<PlayerId, Vec<GameObjectId>>::new();
    for action in selected_sba_actions {
        if let SbaSelectedActionV1::ObjectToOwnerGraveyard { object, .. } = action {
            let game_object = objects
                .get(object)
                .ok_or(EngineStateShapeViolation::MagicContinuation)?;
            if !players.contains(&game_object.owner) {
                return Err(EngineStateShapeViolation::MagicContinuation);
            }
            objects_by_owner
                .entry(game_object.owner)
                .or_default()
                .push(*object);
        }
    }

    let required_owners: BTreeSet<_> = objects_by_owner
        .iter()
        .filter_map(|(owner, members)| (members.len() >= 2).then_some(*owner))
        .collect();
    if required_owners.len() != apnap_owners.len()
        || apnap_owners.iter().copied().collect::<BTreeSet<_>>() != required_owners
    {
        return Err(EngineStateShapeViolation::MagicContinuation);
    }

    for (index, order) in completed_owner_orders.iter().enumerate() {
        let owner = apnap_owners[index];
        let expected = objects_by_owner
            .get(&owner)
            .ok_or(EngineStateShapeViolation::MagicContinuation)?;
        let actual: BTreeSet<_> = order.top_to_bottom.iter().copied().collect();
        if order.owner != owner
            || order.top_to_bottom.len() != expected.len()
            || actual.len() != order.top_to_bottom.len()
            || actual != expected.iter().copied().collect()
        {
            return Err(EngineStateShapeViolation::MagicContinuation);
        }
    }
    Ok(objects_by_owner)
}
