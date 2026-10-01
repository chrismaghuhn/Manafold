//! Perspective-safe current-state view of public turn and APNAP graveyard ordering.

use mtgml_model::{parse_canonical_u64, OpaqueObjectId, PlayerId};
use serde::{Deserialize, Serialize};

/// A `u64` on the wire as a canonical decimal string, as `turn_number` is: no
/// sign, no leading zeros, and a JSON number is refused.
mod canonical_u64_string {
    use mtgml_model::parse_canonical_u64;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(value: &u64, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u64, D::Error>
    where
        D: Deserializer<'de>,
    {
        let text = String::deserialize(deserializer)?;
        parse_canonical_u64(&text).map_err(serde::de::Error::custom)
    }
}

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

/// A permanent on the battlefield: who controls it and the turn since which
/// that player has controlled it (CR 302.6) and, for a creature, its power
/// and toughness (CR 208.1); both are null for any other permanent.
///
/// A creature's power and toughness are its printed ones, which are also its
/// current ones: the projection is not made for a creature that an effect or a
/// +1/+1 or -1/-1 counter could change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PermanentObservationV1 {
    pub object: OpaqueObjectId,
    pub controller: PlayerId,
    #[serde(with = "canonical_u64_string")]
    pub controlled_since_turn: u64,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub power: Option<i64>,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub toughness: Option<i64>,
}

impl PermanentObservationV1 {
    fn is_creature(&self) -> bool {
        self.power.is_some()
    }
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
    /// Every permanent on the battlefield, lands included, ascending by
    /// opaque id.
    pub permanents: Vec<PermanentObservationV1>,
    /// The creatures that are attacking (CR 508.1k), ascending by opaque id.
    /// Each is a creature among `permanents`.
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
                .permanents
                .windows(2)
                .any(|pair| pair[0].object >= pair[1].object)
            || self
                .permanents
                .iter()
                .any(|permanent| permanent.power.is_some() != permanent.toughness.is_some())
            || self.attacking.windows(2).any(|pair| pair[0] >= pair[1])
            || self.attacking.iter().any(|attacker| {
                !self
                    .permanents
                    .binary_search_by_key(attacker, |permanent| permanent.object)
                    .is_ok_and(|index| self.permanents[index].is_creature())
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

fn deserialize_required_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::deserialize(deserializer)
}

fn deserialize_required_pending_ordering<'de, D>(
    deserializer: D,
) -> Result<Option<MagicPendingSbaOrdering>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::deserialize(deserializer)
}
