use mtgml_model::{
    AbilityInstanceId, GameObjectId, PhysicalCardId, PlayerId, StackObjectId, ZoneKind,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisibilityPartition {
    Public,
    OwnerOnly,
    FaceDown,
    PrivateGroup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ZonePosition {
    Unordered,
    Top { offset: u32 },
    Bottom { offset: u32 },
    Index { index: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ZoneLocation {
    pub zone: ZoneKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerId>,
    pub position: ZonePosition,
    pub visibility: VisibilityPartition,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub partition: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ZoneKey {
    pub zone: ZoneKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerId>,
    pub visibility: VisibilityPartition,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub partition: Option<String>,
}

impl ZoneLocation {
    pub fn key(&self) -> ZoneKey {
        ZoneKey {
            zone: self.zone,
            player: self.player,
            visibility: self.visibility,
            partition: self.partition.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameObject {
    /// Identity of this incarnation. A zone transition creates another ID.
    pub id: GameObjectId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub physical_card: Option<PhysicalCardId>,
    pub card_definition: mtgml_model::CardDefinitionId,
    pub owner: PlayerId,
    pub controller: PlayerId,
    pub tapped: bool,
    pub face_down: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectSnapshot {
    pub object: GameObjectId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub physical_card: Option<PhysicalCardId>,
    pub card_definition: mtgml_model::CardDefinitionId,
    pub owner: PlayerId,
    pub controller: PlayerId,
    pub tapped: bool,
    pub face_down: bool,
    pub location: ZoneLocation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ZoneTransition {
    pub old_object: GameObjectId,
    pub new_object: GameObjectId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub physical_card: Option<PhysicalCardId>,
    pub from: ZoneLocation,
    pub to: ZoneLocation,
    pub last_known: ObjectSnapshot,
    /// Complete authoritative identity of the new incarnation. Carrying this in
    /// the semantic event makes consecutive zone transitions compositional.
    pub new_snapshot: ObjectSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StackRecord {
    pub id: StackObjectId,
    pub controller: PlayerId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_object: Option<GameObjectId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_ability: Option<AbilityInstanceId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ZoneState {
    pub objects: BTreeMap<GameObjectId, GameObject>,
    pub locations: BTreeMap<GameObjectId, ZoneLocation>,
    pub ordered_zones: BTreeMap<ZoneKey, Vec<GameObjectId>>,
    pub stack_records: BTreeMap<StackObjectId, StackRecord>,
    pub stack_order: Vec<StackObjectId>,
}

#[derive(Serialize)]
pub(crate) struct CanonicalOrderedZoneEntryV1<'a> {
    pub key: &'a ZoneKey,
    pub objects: &'a [GameObjectId],
}

#[derive(Serialize)]
pub(crate) struct CanonicalZoneStateV1<'a> {
    pub objects: &'a BTreeMap<GameObjectId, GameObject>,
    pub locations: &'a BTreeMap<GameObjectId, ZoneLocation>,
    pub ordered_zones: Vec<CanonicalOrderedZoneEntryV1<'a>>,
    pub stack_records: &'a BTreeMap<StackObjectId, StackRecord>,
    pub stack_order: &'a [StackObjectId],
}
