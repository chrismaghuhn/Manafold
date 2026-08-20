use std::collections::BTreeMap;

use mtgml_model::{
    CardDefinitionId, EventSequence, GameObjectId, PhysicalCardId, PlayerId, RuleEventId,
};
use serde::{Deserialize, Serialize};

use crate::zones::ZoneLocation;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeHistoryChannel {
    Public,
    Private,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnowledgePoint {
    pub channel: KnowledgeHistoryChannel,
    pub sequence: EventSequence,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum KnowledgeAcquisitionReason {
    InitialConfiguration,
    PublicEvent { event: RuleEventId },
    PrivateEvent { event: RuleEventId },
    OwnZoneIdentity,
    ExplicitReveal,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeInvalidationReason {
    Shuffle,
    HiddenZoneTransition,
    Randomization,
    ExplicitForget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnownObjectIdentity {
    pub object: GameObjectId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub physical_card: Option<PhysicalCardId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card_definition: Option<CardDefinitionId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub known_location: Option<ZoneLocation>,
    pub learned_at: KnowledgePoint,
    pub learned_via: KnowledgeAcquisitionReason,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeInvalidationRecord {
    pub object: GameObjectId,
    pub invalidated_at: KnowledgePoint,
    pub reason: KnowledgeInvalidationReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct PlayerKnowledgeState {
    pub known_objects: BTreeMap<GameObjectId, KnownObjectIdentity>,
    pub public_history_length: u64,
    pub private_history_length: u64,
    pub invalidations: Vec<KnowledgeInvalidationRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeState {
    pub players: BTreeMap<PlayerId, PlayerKnowledgeState>,
}
