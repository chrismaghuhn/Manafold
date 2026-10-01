//! Perspective-safe current-state view of public turn and APNAP graveyard ordering.

use mtgml_model::{parse_canonical_u64, OpaqueObjectId, PlayerId};
use serde::{Deserialize, Serialize};

use crate::MAGIC_BASIC_LAND_OBSERVATION_SCHEMA_V1;
use crate::{ObservationValidationError, SyntheticPriority, SyntheticTurnPosition};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PublicCounterKindV1 {
    PlusOnePlusOne,
    MinusOneMinusOne,
    Lore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PublicFaceV1 {
    Front,
    Back,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManaPoolObservationV1 {
    pub player: PlayerId,
    pub unrestricted: [u32; 6],
    pub creature_spell_only: [u32; 6],
}

/// A player's public totals: life (CR 119), hand size (CR 402.3) and
/// library size (CR 401.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerObservationV1 {
    pub player: PlayerId,
    pub life: i64,
    pub hand_count: u32,
    pub library_count: u32,
}

/// A creature on the battlefield: who controls it, its printed power and
/// toughness (CR 208.1) and the turn since which that player has controlled it
/// (CR 302.6). Counters on it are listed in `counters`, not added here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreatureObservationV1 {
    pub object: OpaqueObjectId,
    pub controller: PlayerId,
    pub power: i64,
    pub toughness: i64,
    pub controlled_since_turn: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CounterObservationV1 {
    pub object: OpaqueObjectId,
    pub counter_kind: PublicCounterKindV1,
    pub count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttachmentObservationV1 {
    pub source: OpaqueObjectId,
    pub target: OpaqueObjectId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FaceObservationV1 {
    pub object: OpaqueObjectId,
    pub face: PublicFaceV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MagicBasicLandObservationV1 {
    pub schema_version: String,
    pub active_player: PlayerId,
    pub turn_number: String,
    pub turn_position: SyntheticTurnPosition,
    pub priority: SyntheticPriority,
    pub players: Vec<PlayerObservationV1>,
    #[serde(deserialize_with = "deserialize_required_pending_ordering")]
    pub pending_sba_ordering: Option<MagicPendingSbaOrdering>,
    pub mana_pools: Vec<ManaPoolObservationV1>,
    pub counters: Vec<CounterObservationV1>,
    pub attachments: Vec<AttachmentObservationV1>,
    pub faces: Vec<FaceObservationV1>,
    /// Tapped permanents on the battlefield (CR 110.5), by opaque id.
    pub tapped: Vec<OpaqueObjectId>,
    /// The creatures on the battlefield, ascending by opaque id.
    pub creatures: Vec<CreatureObservationV1>,
    /// The creatures that are attacking (CR 508.1k), ascending by opaque id.
    /// Each is one of `creatures`.
    pub attacking: Vec<OpaqueObjectId>,
}

impl MagicBasicLandObservationV1 {
    pub fn validate(&self) -> Result<(), ObservationValidationError> {
        if self.schema_version != MAGIC_BASIC_LAND_OBSERVATION_SCHEMA_V1
            || parse_canonical_u64(&self.turn_number).is_err()
        {
            return Err(ObservationValidationError::ObservationPayload);
        }
        if self
            .players
            .windows(2)
            .any(|pair| pair[0].player >= pair[1].player)
            || !self
                .players
                .iter()
                .any(|entry| entry.player == self.active_player)
            || self.tapped.windows(2).any(|pair| pair[0] >= pair[1])
            || self
                .creatures
                .windows(2)
                .any(|pair| pair[0].object >= pair[1].object)
            || self.attacking.windows(2).any(|pair| pair[0] >= pair[1])
            || self.attacking.iter().any(|attacker| {
                self.creatures
                    .binary_search_by_key(attacker, |creature| creature.object)
                    .is_err()
            })
            || self
                .mana_pools
                .windows(2)
                .any(|pair| pair[0].player >= pair[1].player)
            || self.counters.windows(2).any(|pair| {
                (pair[0].object, pair[0].counter_kind) >= (pair[1].object, pair[1].counter_kind)
            })
            || self.counters.iter().any(|entry| entry.count == 0)
            || self
                .attachments
                .windows(2)
                .any(|pair| pair[0].source >= pair[1].source)
            || self
                .faces
                .windows(2)
                .any(|pair| pair[0].object >= pair[1].object)
        {
            return Err(ObservationValidationError::ObservationPayload);
        }
        if let Some(progress) = &self.pending_sba_ordering {
            let mut owners = std::collections::BTreeSet::new();
            for order in &progress.completed_orders {
                if order.ordered_objects.len() < 2
                    || !owners.insert(order.owner)
                    || order.owner == progress.next_order_owner
                    || order
                        .ordered_objects
                        .windows(2)
                        .any(|pair| pair[0] == pair[1])
                {
                    return Err(ObservationValidationError::ObservationPayload);
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MagicCompletedOrder {
    pub owner: PlayerId,
    pub ordered_objects: Vec<OpaqueObjectId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MagicPendingSbaOrdering {
    pub completed_orders: Vec<MagicCompletedOrder>,
    pub next_order_owner: PlayerId,
}

fn deserialize_required_pending_ordering<'de, D>(
    deserializer: D,
) -> Result<Option<MagicPendingSbaOrdering>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::deserialize(deserializer)
}
