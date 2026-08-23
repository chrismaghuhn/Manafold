//! Private, detached M2 state shapes prepared before the current-runtime cut.
//!
//! Nothing in this module is reachable through the current `EngineState` until
//! the coordinated Tasks 7Ã¢â‚¬â€œ11 cut. The types deliberately do not adapt or
//! reinterpret any historical V1/V2 value.

use std::collections::{BTreeMap, BTreeSet};

use mtgml_decision::AuthoritativeDecisionRequestV2;
use mtgml_model::{
    AbilityInstanceId, CardDefinitionId, ContinuationId, GameObjectId, OpaqueAbilityId,
    OpaqueObjectId, PhysicalCardId, PlayerDecisionIdV1, PlayerId, StateRevision, VisibleSequence,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::knowledge::{KnowledgeAcquisitionReason, KnowledgeInvalidationReason};
use crate::zones::ZoneLocation;
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
}

impl ContinuationPayloadV2 {
    pub fn stage_index(&self) -> u16 {
        match self {
            Self::SyntheticM2Assembly { stage, .. } => stage.stage_index(),
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

/// One retained known-location fact. The fact owns its complete typed
/// provenance; no downstream layer may infer or synthesize it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnownLocationFactV2 {
    pub location: ZoneLocation,
    pub provenance: KnowledgeAcquisitionReason,
}

/// Typed invalidation of a retired knowledge record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeInvalidationV2 {
    pub provenance: KnowledgeAcquisitionReason,
    pub reason: KnowledgeInvalidationReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeRecordV2 {
    pub opaque_object: OpaqueObjectId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub physical_card: Option<PhysicalCardId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card_definition: Option<CardDefinitionId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub known_location: Option<KnownLocationFactV2>,
    #[serde(default)]
    pub historical_locations: Vec<KnownLocationFactV2>,
    pub acquisition: KnowledgeAcquisitionReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetiredKnowledgeRecordV2 {
    pub opaque_object: OpaqueObjectId,
    #[serde(default)]
    pub physical_card: Option<PhysicalCardId>,
    #[serde(default)]
    pub card_definition: Option<CardDefinitionId>,
    #[serde(default)]
    pub last_known_location: Option<KnownLocationFactV2>,
    #[serde(default)]
    pub historical_locations: Vec<KnownLocationFactV2>,
    pub acquisition: KnowledgeAcquisitionReason,
    pub invalidation: KnowledgeInvalidationV2,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct PlayerKnowledgeStateV2 {
    pub active: BTreeMap<OpaqueObjectId, KnowledgeRecordV2>,
    pub retired: BTreeMap<OpaqueObjectId, RetiredKnowledgeRecordV2>,
    pub next_visible_sequence: VisibleSequence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeStateV2 {
    pub players: BTreeMap<PlayerId, PlayerKnowledgeStateV2>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct PerspectiveIdentityRecordV2 {
    /// This is the one canonical persisted mapping direction.
    pub opaque_to_object: BTreeMap<OpaqueObjectId, GameObjectId>,
    pub opaque_to_ability: BTreeMap<OpaqueAbilityId, AbilityInstanceId>,
    /// Reverse maps are runtime validation indexes and are rebuilt/checked.
    #[serde(default)]
    pub object_to_opaque: BTreeMap<GameObjectId, OpaqueObjectId>,
    #[serde(default)]
    pub ability_to_opaque: BTreeMap<AbilityInstanceId, OpaqueAbilityId>,
    pub next_opaque_object_id: OpaqueObjectId,
    pub next_opaque_ability_id: OpaqueAbilityId,
    pub next_player_decision_id: PlayerDecisionIdV1,
    pub retired_object_ids: BTreeSet<OpaqueObjectId>,
    pub retired_ability_ids: BTreeSet<OpaqueAbilityId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct PerspectiveIdentityStateV2 {
    pub players: BTreeMap<PlayerId, PerspectiveIdentityRecordV2>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum M2ShapeViolation {
    #[error("M2 state does not cover exactly the declared players")]
    PlayerCoverage,
    #[error("M2 perspective-local allocator is missing or behind")]
    Allocator,
    #[error("M2 opaque mapping is not bijective")]
    IdentityMapping,
    #[error("M2 opaque identity is active and retired simultaneously")]
    RetiredIdentity,
    #[error("M2 retained knowledge shape is invalid")]
    Knowledge,
    #[error("M2 visible sequence is not strictly monotonic")]
    VisibleSequence,
    #[error("M2 pending decision is invalid")]
    PendingDecision,
    #[error("M2 pending decision references a missing continuation")]
    ContinuationReference,
    #[error("M2 continuation stage is owned by a different actor than its request")]
    ContinuationActor,
    #[error("M2 continuation revision is stale or future-dated")]
    ContinuationRevision,
    #[error("M2 continuation stage is invalid")]
    ContinuationStage,
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
fn validate_synthetic_assembly(
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
    Ok(())
}

pub fn validate_m2_shape(
    current_revision: StateRevision,
    players: &BTreeSet<PlayerId>,
    pending: Option<&PendingDecisionRecordV2>,
    continuations: &BTreeMap<ContinuationId, ContinuationRecordV2>,
    knowledge: &KnowledgeStateV2,
    identities: &PerspectiveIdentityStateV2,
) -> Result<(), M2ShapeViolation> {
    if knowledge.players.keys().copied().collect::<BTreeSet<_>>() != *players
        || identities.players.keys().copied().collect::<BTreeSet<_>>() != *players
    {
        return Err(M2ShapeViolation::PlayerCoverage);
    }

    for (player, identity) in &identities.players {
        validate_identity(identity)?;
        if identity
            .opaque_to_object
            .values()
            .any(|object| !identity.object_to_opaque.contains_key(object))
            || identity
                .opaque_to_ability
                .values()
                .any(|ability| !identity.ability_to_opaque.contains_key(ability))
        {
            return Err(M2ShapeViolation::IdentityMapping);
        }
        let player_knowledge = knowledge
            .players
            .get(player)
            .ok_or(M2ShapeViolation::PlayerCoverage)?;
        validate_knowledge(player_knowledge)?;
    }

    for continuation in continuations.values() {
        if !players.contains(&continuation.actor) {
            return Err(M2ShapeViolation::PlayerCoverage);
        }
        if continuation.created_at_revision > current_revision {
            return Err(M2ShapeViolation::ContinuationRevision);
        }
        if continuation.stage_index != continuation.payload.stage_index() {
            return Err(M2ShapeViolation::ContinuationStage);
        }
        match &continuation.payload {
            ContinuationPayloadV2::SyntheticM2Assembly {
                stage,
                selected_count,
                selected_piece_keys,
                ordered_piece_keys,
            } => {
                validate_synthetic_assembly(
                    *stage,
                    *selected_count,
                    selected_piece_keys,
                    ordered_piece_keys,
                )?;
            }
        }
    }

    if let Some(pending) = pending {
        pending
            .request
            .validate()
            .map_err(|_| M2ShapeViolation::PendingDecision)?;
        if pending.request.state_revision != current_revision
            || !players.contains(&pending.request.actor)
        {
            return Err(M2ShapeViolation::PendingDecision);
        }
        if let Some(continuation_id) = pending.request.continuation_id {
            let continuation = continuations
                .get(&continuation_id)
                .ok_or(M2ShapeViolation::ContinuationReference)?;
            // DECISION_PROTOCOL.md: the endpoint bound to the actor projects
            // the request; ADR 0039 serializes the owning actor as
            // continuation state. M2 has no accepted stage-transfer
            // semantics, so a referenced continuation must belong to the
            // pending request's actor.
            if continuation.actor != pending.request.actor {
                return Err(M2ShapeViolation::ContinuationActor);
            }
            if continuation.created_at_revision > pending.request.state_revision {
                return Err(M2ShapeViolation::ContinuationRevision);
            }
            if continuation.stage_index != continuation.payload.stage_index() {
                return Err(M2ShapeViolation::ContinuationStage);
            }
        }
    }
    Ok(())
}

fn validate_identity(identity: &PerspectiveIdentityRecordV2) -> Result<(), M2ShapeViolation> {
    if identity.next_opaque_object_id.0 == 0
        || identity.next_opaque_ability_id.0 == 0
        || identity.next_player_decision_id.0 == 0
    {
        return Err(M2ShapeViolation::Allocator);
    }
    if identity.object_to_opaque.len() != identity.opaque_to_object.len()
        || identity.ability_to_opaque.len() != identity.opaque_to_ability.len()
    {
        return Err(M2ShapeViolation::IdentityMapping);
    }
    if identity
        .opaque_to_object
        .keys()
        .any(|opaque| identity.retired_object_ids.contains(opaque) || opaque.0 == 0)
        || identity
            .opaque_to_ability
            .keys()
            .any(|opaque| identity.retired_ability_ids.contains(opaque) || opaque.0 == 0)
        || identity
            .retired_object_ids
            .iter()
            .any(|opaque| opaque.0 == 0)
        || identity
            .retired_ability_ids
            .iter()
            .any(|opaque| opaque.0 == 0)
    {
        return Err(M2ShapeViolation::RetiredIdentity);
    }
    if identity
        .opaque_to_object
        .keys()
        .any(|opaque| opaque.0 >= identity.next_opaque_object_id.0)
        || identity
            .opaque_to_ability
            .keys()
            .any(|opaque| opaque.0 >= identity.next_opaque_ability_id.0)
    {
        return Err(M2ShapeViolation::Allocator);
    }
    for (object, opaque) in &identity.object_to_opaque {
        if identity.opaque_to_object.get(opaque) != Some(object) {
            return Err(M2ShapeViolation::IdentityMapping);
        }
    }
    for (ability, opaque) in &identity.ability_to_opaque {
        if identity.opaque_to_ability.get(opaque) != Some(ability) {
            return Err(M2ShapeViolation::IdentityMapping);
        }
    }
    Ok(())
}

fn validate_knowledge(knowledge: &PlayerKnowledgeStateV2) -> Result<(), M2ShapeViolation> {
    let provenance_is_valid =
        |provenance: &KnowledgeAcquisitionReason| -> Result<(), M2ShapeViolation> {
            if !provenance.has_accepted_channel_cause()
                || !provenance.is_within_visible_sequence(knowledge.next_visible_sequence)
            {
                return Err(M2ShapeViolation::VisibleSequence);
            }
            Ok(())
        };
    let history_is_valid = |records: &[KnownLocationFactV2]| -> Result<(), M2ShapeViolation> {
        for fact in records {
            provenance_is_valid(&fact.provenance)?;
        }
        // Only observed facts carry visible sequences; the strictly
        // increasing rule compares those observed sequences.
        let observed: Vec<_> = records
            .iter()
            .filter_map(|fact| {
                fact.provenance
                    .observed_sequence()
                    .map(|sequence| sequence.0)
            })
            .collect();
        for window in observed.windows(2) {
            if window[0] >= window[1] {
                return Err(M2ShapeViolation::VisibleSequence);
            }
        }
        Ok(())
    };
    for (opaque, record) in &knowledge.active {
        if opaque != &record.opaque_object || opaque.0 == 0 {
            return Err(M2ShapeViolation::Knowledge);
        }
        provenance_is_valid(&record.acquisition)?;
        if let Some(fact) = record.known_location.as_ref() {
            provenance_is_valid(&fact.provenance)?;
        }
        history_is_valid(&record.historical_locations)?;
    }
    for (opaque, record) in &knowledge.retired {
        if opaque != &record.opaque_object || opaque.0 == 0 || knowledge.active.contains_key(opaque)
        {
            return Err(M2ShapeViolation::Knowledge);
        }
        // INFORMATION_MODEL.md: retirement records an explicit invalidation
        // reason *and visible sequence* â€” an unsequenced initial
        // configuration cannot invalidate anything.
        if record.invalidation.provenance.observed_sequence().is_none() {
            return Err(M2ShapeViolation::Knowledge);
        }
        provenance_is_valid(&record.acquisition)?;
        provenance_is_valid(&record.invalidation.provenance)?;
        if let Some(fact) = record.last_known_location.as_ref() {
            provenance_is_valid(&fact.provenance)?;
        }
        history_is_valid(&record.historical_locations)?;
    }
    Ok(())
}
