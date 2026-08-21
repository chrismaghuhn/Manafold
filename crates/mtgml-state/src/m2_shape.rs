//! Private, detached M2 state shapes prepared before the current-runtime cut.
//!
//! Nothing in this module is reachable through the current `EngineState` until
//! the coordinated Tasks 7–11 cut. The types deliberately do not adapt or
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
    AwaitingSelectOne,
}

impl AssemblyStageV2 {
    fn index(self) -> u16 {
        match self {
            Self::AwaitingSelectOne => 0,
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
    fn stage_index(&self) -> u16 {
        match self {
            Self::SyntheticM2Assembly { stage, .. } => stage.index(),
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeRecordV2 {
    pub opaque_object: OpaqueObjectId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub physical_card: Option<PhysicalCardId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card_definition: Option<CardDefinitionId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub known_location: Option<ZoneLocation>,
    pub learned_at: crate::knowledge::KnowledgePoint,
    pub learned_via: KnowledgeAcquisitionReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeHistoryRecordV2 {
    pub opaque_object: OpaqueObjectId,
    pub location: Option<ZoneLocation>,
    pub observed_at: crate::knowledge::KnowledgePoint,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetiredKnowledgeRecordV2 {
    pub opaque_object: OpaqueObjectId,
    pub invalidated_at: crate::knowledge::KnowledgePoint,
    pub reason: KnowledgeInvalidationReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct PlayerKnowledgeStateV2 {
    pub active: BTreeMap<OpaqueObjectId, KnowledgeRecordV2>,
    pub retired: BTreeMap<OpaqueObjectId, RetiredKnowledgeRecordV2>,
    pub history: Vec<KnowledgeHistoryRecordV2>,
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
    #[error("M2 continuation revision is stale or future-dated")]
    ContinuationRevision,
    #[error("M2 continuation stage is invalid")]
    ContinuationStage,
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
            .any(|object| identity.object_to_opaque.get(object).is_none())
            || identity
                .opaque_to_ability
                .values()
                .any(|ability| identity.ability_to_opaque.get(ability).is_none())
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
    for (opaque, record) in &knowledge.active {
        if opaque != &record.opaque_object || opaque.0 == 0 {
            return Err(M2ShapeViolation::Knowledge);
        }
    }
    for (opaque, record) in &knowledge.retired {
        if opaque != &record.opaque_object || opaque.0 == 0 || knowledge.active.contains_key(opaque)
        {
            return Err(M2ShapeViolation::Knowledge);
        }
    }
    for window in knowledge.history.windows(2) {
        if window[0].observed_at.sequence >= window[1].observed_at.sequence {
            return Err(M2ShapeViolation::VisibleSequence);
        }
    }
    if knowledge
        .history
        .last()
        .is_some_and(|record| record.observed_at.sequence.0 >= knowledge.next_visible_sequence.0)
    {
        return Err(M2ShapeViolation::VisibleSequence);
    }
    Ok(())
}
