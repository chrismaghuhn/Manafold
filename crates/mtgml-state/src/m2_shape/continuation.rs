//! Ownership: current-M2 synthetic assembly continuation program material
//! (records, stages, payload, and its shape/coherence validation).

use std::collections::{BTreeMap, BTreeSet};

use mtgml_decision::{
    AuthoritativeDecisionRequestV2, CandidateIntent, DecisionDomainV2, DecisionVisibility,
    EngineCandidateBinding,
};
use mtgml_model::{ContinuationId, GameObjectId, PlayerId, StateRevision};
use serde::{Deserialize, Serialize};

use super::PerspectiveIdentityStateV2;
use crate::m2_shape::{M2ShapeViolation, SYNTHETIC_COUNT_MAX, SYNTHETIC_COUNT_MIN};
use crate::zones::GameObject;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PendingDecisionRecordV2 {
    pub request: AuthoritativeDecisionRequestV2,
}

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

impl AssemblyStageV2 {
    pub fn stage_index(self) -> u16 {
        match self {
            Self::ChooseCount => 0,
            Self::ChooseMembers => 1,
            Self::OrderMembers => 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ContinuationPayloadV2 {
    SyntheticM2Assembly {
        stage: AssemblyStageV2,
        selected_count: Option<u32>,
        selected_piece_keys: Vec<u32>,
        ordered_piece_keys: Vec<u32>,
    },
    MagicSbaGraveyardOrderV1 {
        round_start_revision: StateRevision,
        selected_sba_actions: Vec<SbaSelectedActionV1>,
        apnap_owners: Vec<PlayerId>,
        next_owner_index: u32,
        completed_owner_orders: Vec<SbaGraveyardOwnerOrderV1>,
    },
}

impl ContinuationPayloadV2 {
    pub fn stage_index(&self) -> u16 {
        match self {
            Self::SyntheticM2Assembly { stage, .. } => stage.stage_index(),
            Self::MagicSbaGraveyardOrderV1 {
                next_owner_index, ..
            } => u16::try_from(*next_owner_index).unwrap_or(u16::MAX),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContinuationRecordV2 {
    pub id: ContinuationId,
    pub actor: PlayerId,
    pub created_at_revision: StateRevision,
    pub stage_index: u16,
    pub payload: ContinuationPayloadV2,
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
) -> Result<(), M2ShapeViolation> {
    let canonical_set = |values: &[u32]| values.windows(2).all(|window| window[0] < window[1]);
    match stage {
        AssemblyStageV2::ChooseCount => {
            if selected_count.is_some()
                || !selected_piece_keys.is_empty()
                || !ordered_piece_keys.is_empty()
            {
                return Err(M2ShapeViolation::Knowledge);
            }
        }
        AssemblyStageV2::ChooseMembers => {
            if selected_count.is_none()
                || !selected_piece_keys.is_empty()
                || !ordered_piece_keys.is_empty()
            {
                return Err(M2ShapeViolation::Knowledge);
            }
        }
        AssemblyStageV2::OrderMembers => {
            let Some(count) = selected_count else {
                return Err(M2ShapeViolation::Knowledge);
            };
            if !ordered_piece_keys.is_empty()
                || selected_piece_keys.len() != count as usize
                || !canonical_set(selected_piece_keys)
            {
                return Err(M2ShapeViolation::Knowledge);
            }
        }
    }
    // A decided count can only originate from the supported ChooseCount
    // interval; anything else was never offered by this program.
    if let Some(count) = selected_count {
        if count > SYNTHETIC_COUNT_MAX {
            return Err(M2ShapeViolation::Knowledge);
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
) -> Result<BTreeMap<PlayerId, Vec<GameObjectId>>, M2ShapeViolation> {
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
        .ok_or(M2ShapeViolation::ContinuationRevision)?;
    let stage_count = u64::from(next_owner_index);
    let expected_current_revision = expected_created_revision
        .checked_add(stage_count)
        .ok_or(M2ShapeViolation::ContinuationRevision)?;
    if continuation_created_at_revision.0 != expected_created_revision
        || current_revision.0 != expected_current_revision
    {
        return Err(M2ShapeViolation::ContinuationRevision);
    }
    if selected_sba_actions.is_empty()
        || selected_sba_actions
            .windows(2)
            .any(|window| window[0] >= window[1])
    {
        return Err(M2ShapeViolation::MagicContinuation);
    }

    let mut losing_players = BTreeSet::new();
    let mut graveyard_objects = BTreeSet::new();
    for action in selected_sba_actions {
        match action {
            SbaSelectedActionV1::PlayerLoses { player } => {
                if !players.contains(player) || !losing_players.insert(*player) {
                    return Err(M2ShapeViolation::MagicContinuation);
                }
            }
            SbaSelectedActionV1::ObjectToOwnerGraveyard { object, causes } => {
                if !graveyard_objects.insert(*object)
                    || causes.is_empty()
                    || causes.windows(2).any(|window| window[0] >= window[1])
                    || !objects.contains_key(object)
                {
                    return Err(M2ShapeViolation::MagicContinuation);
                }
            }
        }
    }

    let next_owner_index_usize =
        usize::try_from(next_owner_index).map_err(|_| M2ShapeViolation::MagicContinuation)?;
    if apnap_owners.is_empty()
        || apnap_owners.iter().any(|owner| !players.contains(owner))
        || apnap_owners.iter().copied().collect::<BTreeSet<_>>().len() != apnap_owners.len()
        || next_owner_index_usize != completed_owner_orders.len()
        || next_owner_index_usize >= apnap_owners.len()
    {
        return Err(M2ShapeViolation::MagicContinuation);
    }

    let mut objects_by_owner = BTreeMap::<PlayerId, Vec<GameObjectId>>::new();
    for action in selected_sba_actions {
        if let SbaSelectedActionV1::ObjectToOwnerGraveyard { object, .. } = action {
            let game_object = objects
                .get(object)
                .ok_or(M2ShapeViolation::MagicContinuation)?;
            if !players.contains(&game_object.owner) {
                return Err(M2ShapeViolation::MagicContinuation);
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
        return Err(M2ShapeViolation::MagicContinuation);
    }

    for (index, order) in completed_owner_orders.iter().enumerate() {
        let owner = apnap_owners[index];
        let expected = objects_by_owner
            .get(&owner)
            .ok_or(M2ShapeViolation::MagicContinuation)?;
        let actual: BTreeSet<_> = order.top_to_bottom.iter().copied().collect();
        if order.owner != owner
            || order.top_to_bottom.len() != expected.len()
            || actual.len() != order.top_to_bottom.len()
            || actual != expected.iter().copied().collect()
        {
            return Err(M2ShapeViolation::MagicContinuation);
        }
    }
    Ok(objects_by_owner)
}

/// The one linear M2 program binds an active continuation and its pending
/// request into a single authoritative semantic unit: the pending request
/// must express exactly the referenced stage's program, and an active
/// continuation must always be resumable.
pub(super) fn validate_program_coherence(
    pending: Option<&PendingDecisionRecordV2>,
    continuations: &BTreeMap<ContinuationId, ContinuationRecordV2>,
    players: &BTreeSet<PlayerId>,
    objects: &BTreeMap<GameObjectId, GameObject>,
    identities: &PerspectiveIdentityStateV2,
) -> Result<(), M2ShapeViolation> {
    if continuations.len() > 1 {
        return Err(M2ShapeViolation::ContinuationReference);
    }
    let Some(record) = continuations.values().next() else {
        return Ok(());
    };
    let Some(pending) = pending else {
        // An active continuation without its next stage request is not
        // resumable and can never become checkpointable state.
        return Err(M2ShapeViolation::ContinuationReference);
    };
    if pending.request.continuation_id != Some(record.id) {
        return Err(M2ShapeViolation::ContinuationReference);
    }
    let ContinuationPayloadV2::SyntheticM2Assembly {
        stage,
        selected_count,
        selected_piece_keys,
        ..
    } = &record.payload
    else {
        let ContinuationPayloadV2::MagicSbaGraveyardOrderV1 {
            round_start_revision,
            selected_sba_actions,
            apnap_owners,
            next_owner_index,
            completed_owner_orders,
        } = &record.payload
        else {
            unreachable!("ContinuationPayloadV2 is a closed enum")
        };
        let objects_by_owner =
            validate_magic_sba_graveyard_order(MagicSbaGraveyardOrderValidation {
                round_start_revision: *round_start_revision,
                continuation_created_at_revision: record.created_at_revision,
                selected_sba_actions,
                apnap_owners,
                next_owner_index: *next_owner_index,
                completed_owner_orders,
                current_revision: pending.request.state_revision,
                players,
                objects,
            })?;
        let current_owner = apnap_owners
            .get(*next_owner_index as usize)
            .copied()
            .ok_or(M2ShapeViolation::MagicContinuation)?;
        let expected = objects_by_owner
            .get(&current_owner)
            .ok_or(M2ShapeViolation::MagicContinuation)?;
        let request = &pending.request;
        let domain_matches = matches!(
            &request.decision,
            DecisionDomainV2::Order { minimum, maximum }
                if *minimum as usize == expected.len() && *maximum as usize == expected.len()
        );
        let mut bound = Vec::with_capacity(request.candidates.len());
        let actor_identities = identities
            .players
            .get(&current_owner)
            .ok_or(M2ShapeViolation::MagicContinuation)?;
        let bindings_match = request.candidates.iter().all(|candidate| {
            if let (
                EngineCandidateBinding::SelectObject { object },
                CandidateIntent::SelectObject { object: opaque },
            ) = (&candidate.trusted_binding, &candidate.visible_intent)
            {
                bound.push(*object);
                actor_identities.opaque_to_object.get(opaque) == Some(object)
                    && actor_identities.object_to_opaque.get(object) == Some(opaque)
            } else {
                false
            }
        });
        bound.sort_unstable();
        let mut expected = expected.clone();
        expected.sort_unstable();
        if record.actor != current_owner
            || request.actor != current_owner
            || request.visibility != DecisionVisibility::ActingPlayerOnly
            || !domain_matches
            || !bindings_match
            || bound != expected
        {
            return Err(M2ShapeViolation::MagicContinuation);
        }
        return Ok(());
    };
    let candidates_express = |expected_pieces: &[u32]| -> bool {
        pending.request.candidates.len() == expected_pieces.len()
            && pending
                .request
                .candidates
                .iter()
                .enumerate()
                .all(|(index, candidate)| {
                    candidate.candidate_id.0 == index as u32
                        && matches!(
                            &candidate.visible_intent,
                            CandidateIntent::SelectMode { mode_index }
                                if *mode_index == expected_pieces[index]
                        )
                })
    };
    match (stage, &pending.request.decision) {
        (
            AssemblyStageV2::ChooseCount,
            mtgml_decision::DecisionDomainV2::ChooseNumber { minimum, maximum },
        ) => {
            // The engine may offer exactly the supported program interval.
            if *minimum != i64::from(SYNTHETIC_COUNT_MIN)
                || *maximum != i64::from(SYNTHETIC_COUNT_MAX)
                || !pending.request.candidates.is_empty()
            {
                return Err(M2ShapeViolation::PendingDecision);
            }
        }
        (
            AssemblyStageV2::ChooseMembers,
            mtgml_decision::DecisionDomainV2::ChooseMany { minimum, maximum },
        ) => {
            let count = selected_count.ok_or(M2ShapeViolation::Knowledge)?;
            if count > SYNTHETIC_COUNT_MAX || *minimum != count || *maximum != count {
                return Err(M2ShapeViolation::PendingDecision);
            }
            // Stage members are the fixed synthetic piece surface 0..count.
            let expected: Vec<u32> = (0..count).collect();
            if !candidates_express(&expected) {
                return Err(M2ShapeViolation::PendingDecision);
            }
        }
        (
            AssemblyStageV2::OrderMembers,
            mtgml_decision::DecisionDomainV2::Order { minimum, maximum },
        ) => {
            let count = selected_count.ok_or(M2ShapeViolation::Knowledge)?;
            if count > SYNTHETIC_COUNT_MAX || *minimum != count || *maximum != count {
                return Err(M2ShapeViolation::PendingDecision);
            }
            // ChooseMembers offers exactly pieces 0..count and requires
            // exactly count selections: the only reachable member set is the
            // full prefix. Anything else is an unreachable history.
            if *selected_piece_keys != (0..count).collect::<Vec<u32>>() {
                return Err(M2ShapeViolation::Knowledge);
            }
            if !candidates_express(selected_piece_keys) {
                return Err(M2ShapeViolation::PendingDecision);
            }
        }
        _ => return Err(M2ShapeViolation::PendingDecision),
    }
    Ok(())
}
