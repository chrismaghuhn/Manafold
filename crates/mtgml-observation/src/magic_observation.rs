//! Perspective-safe current-state view of public turn and APNAP graveyard ordering.

use mtgml_model::{parse_canonical_u64, OpaqueObjectId, PlayerId};
use serde::{Deserialize, Serialize};

use crate::{ObservationValidationError, SyntheticPriority, SyntheticTurnPosition};

pub const MAGIC_OBSERVATION_SCHEMA_V1: &str = "magic-m3-observation.v1";
pub const MAGIC_OBSERVATION_SCHEMA_V2: &str = "magic-combat-observation.v2";
pub const MAGIC_OBSERVATION_SCHEMA_V3: &str = "magic-combat-observation.v3";
pub const MAGIC_OBSERVATION_SCHEMA_V4: &str = "magic-combat-observation.v4";

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MagicObservation {
    pub schema_version: String,
    pub active_player: PlayerId,
    pub turn_number: String,
    pub turn_position: SyntheticTurnPosition,
    pub priority: SyntheticPriority,
    #[serde(deserialize_with = "deserialize_required_pending_ordering")]
    pub pending_sba_ordering: Option<MagicPendingSbaOrdering>,
}

fn deserialize_required_pending_ordering<'de, D>(
    deserializer: D,
) -> Result<Option<MagicPendingSbaOrdering>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::deserialize(deserializer)
}

impl MagicObservation {
    pub fn validate(&self) -> Result<(), ObservationValidationError> {
        if self.schema_version != MAGIC_OBSERVATION_SCHEMA_V1
            || parse_canonical_u64(&self.turn_number).is_err()
        {
            return Err(ObservationValidationError::ObservationPayload);
        }
        // Turn position and priority reuse closed, already-versioned enums.
        if let Some(progress) = &self.pending_sba_ordering {
            let mut owners = std::collections::BTreeSet::new();
            for order in &progress.completed_orders {
                if order.ordered_objects.len() < 2
                    || !owners.insert(order.owner)
                    || order.owner == progress.next_order_owner
                {
                    return Err(ObservationValidationError::ObservationPayload);
                }
                let mut objects = std::collections::BTreeSet::new();
                if order
                    .ordered_objects
                    .iter()
                    .any(|object| !objects.insert(*object))
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
pub struct MagicCombatBlockerAssignmentV2 {
    pub attacker: OpaqueObjectId,
    pub blocker: Option<OpaqueObjectId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MagicCombatParticipationV2 {
    pub defending_player: PlayerId,
    pub attackers: Vec<OpaqueObjectId>,
    pub blockers: Vec<MagicCombatBlockerAssignmentV2>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MagicObservationV2 {
    pub schema_version: String,
    pub active_player: PlayerId,
    pub turn_number: String,
    pub turn_position: SyntheticTurnPosition,
    pub priority: SyntheticPriority,
    #[serde(deserialize_with = "deserialize_required_pending_ordering")]
    pub pending_sba_ordering: Option<MagicPendingSbaOrdering>,
    #[serde(deserialize_with = "deserialize_required_combat")]
    pub combat: Option<MagicCombatParticipationV2>,
}

fn deserialize_required_combat<'de, D>(
    deserializer: D,
) -> Result<Option<MagicCombatParticipationV2>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::deserialize(deserializer)
}

impl MagicObservationV2 {
    pub fn validate(&self) -> Result<(), ObservationValidationError> {
        if self.schema_version != MAGIC_OBSERVATION_SCHEMA_V2
            || parse_canonical_u64(&self.turn_number).is_err()
        {
            return Err(ObservationValidationError::ObservationPayload);
        }
        if let Some(combat) = &self.combat {
            let mut attackers = std::collections::BTreeSet::new();
            if combat.defending_player == self.active_player
                || !matches!(
                    self.turn_position,
                    SyntheticTurnPosition::Combat {
                        step: crate::SyntheticCombatStep::DeclareAttackers
                            | crate::SyntheticCombatStep::EndOfCombat
                    }
                )
                || (matches!(
                    self.turn_position,
                    SyntheticTurnPosition::Combat {
                        step: crate::SyntheticCombatStep::EndOfCombat
                    }
                ) && !combat.attackers.is_empty())
                || combat
                    .attackers
                    .iter()
                    .any(|attacker| !attackers.insert(*attacker))
                || combat.attackers.windows(2).any(|pair| pair[0] >= pair[1])
                || combat.blockers.len() != combat.attackers.len()
                || combat
                    .blockers
                    .iter()
                    .zip(&combat.attackers)
                    .any(|(assignment, attacker)| {
                        assignment.attacker != *attacker || assignment.blocker.is_some()
                    })
            {
                return Err(ObservationValidationError::ObservationPayload);
            }
        }
        if let Some(progress) = &self.pending_sba_ordering {
            let mut owners = std::collections::BTreeSet::new();
            for order in &progress.completed_orders {
                if order.ordered_objects.len() < 2
                    || !owners.insert(order.owner)
                    || order.owner == progress.next_order_owner
                {
                    return Err(ObservationValidationError::ObservationPayload);
                }
                let mut objects = std::collections::BTreeSet::new();
                if order
                    .ordered_objects
                    .iter()
                    .any(|object| !objects.insert(*object))
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
pub struct MagicCombatBlockerAssignmentV3 {
    pub attacker: OpaqueObjectId,
    pub blocker: Option<OpaqueObjectId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MagicCombatParticipationV3 {
    pub defending_player: PlayerId,
    pub attackers: Vec<OpaqueObjectId>,
    pub blockers: Vec<MagicCombatBlockerAssignmentV3>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MagicObservationV3 {
    pub schema_version: String,
    pub active_player: PlayerId,
    pub turn_number: String,
    pub turn_position: SyntheticTurnPosition,
    pub priority: SyntheticPriority,
    #[serde(deserialize_with = "deserialize_required_pending_ordering")]
    pub pending_sba_ordering: Option<MagicPendingSbaOrdering>,
    #[serde(deserialize_with = "deserialize_required_combat_v3")]
    pub combat: Option<MagicCombatParticipationV3>,
}

fn deserialize_required_combat_v3<'de, D>(
    deserializer: D,
) -> Result<Option<MagicCombatParticipationV3>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::deserialize(deserializer)
}

impl MagicObservationV3 {
    pub fn validate(&self) -> Result<(), ObservationValidationError> {
        if self.schema_version != MAGIC_OBSERVATION_SCHEMA_V3
            || parse_canonical_u64(&self.turn_number).is_err()
        {
            return Err(ObservationValidationError::ObservationPayload);
        }
        if let Some(combat) = &self.combat {
            let mut attackers = std::collections::BTreeSet::new();
            let mut blockers = std::collections::BTreeSet::new();
            let assigned_count = combat
                .blockers
                .iter()
                .filter(|entry| entry.blocker.is_some())
                .count();
            if combat.defending_player == self.active_player
                || !matches!(
                    self.turn_position,
                    SyntheticTurnPosition::Combat {
                        step: crate::SyntheticCombatStep::DeclareAttackers
                            | crate::SyntheticCombatStep::DeclareBlockers
                            | crate::SyntheticCombatStep::EndOfCombat
                    }
                )
                || (matches!(
                    self.turn_position,
                    SyntheticTurnPosition::Combat {
                        step: crate::SyntheticCombatStep::DeclareAttackers
                    }
                ) && combat.blockers.iter().any(|entry| entry.blocker.is_some()))
                || (matches!(
                    self.turn_position,
                    SyntheticTurnPosition::Combat {
                        step: crate::SyntheticCombatStep::EndOfCombat
                    }
                ) && !combat.attackers.is_empty())
                || combat.blockers.len() != combat.attackers.len()
                || combat
                    .attackers
                    .iter()
                    .any(|attacker| !attackers.insert(*attacker))
                || combat.attackers.windows(2).any(|pair| pair[0] >= pair[1])
                || combat
                    .blockers
                    .iter()
                    .zip(&combat.attackers)
                    .any(|(assignment, attacker)| assignment.attacker != *attacker)
                || assigned_count > 1
                || combat
                    .blockers
                    .iter()
                    .filter_map(|assignment| assignment.blocker)
                    .any(|blocker| !blockers.insert(blocker) || attackers.contains(&blocker))
            {
                return Err(ObservationValidationError::ObservationPayload);
            }
        }
        if let Some(progress) = &self.pending_sba_ordering {
            let mut owners = std::collections::BTreeSet::new();
            for order in &progress.completed_orders {
                if order.ordered_objects.len() < 2
                    || !owners.insert(order.owner)
                    || order.owner == progress.next_order_owner
                {
                    return Err(ObservationValidationError::ObservationPayload);
                }
                let mut objects = std::collections::BTreeSet::new();
                if order
                    .ordered_objects
                    .iter()
                    .any(|object| !objects.insert(*object))
                {
                    return Err(ObservationValidationError::ObservationPayload);
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MagicBlockedStatusV4 {
    Blocked,
    Unblocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MagicCombatBlockerAssignmentV4 {
    pub attacker: OpaqueObjectId,
    pub status: MagicBlockedStatusV4,
    pub blocker: Option<OpaqueObjectId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MagicCombatParticipationV4 {
    pub defending_player: PlayerId,
    pub attackers: Vec<OpaqueObjectId>,
    pub blockers: Vec<MagicCombatBlockerAssignmentV4>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MagicPlayerLifeV4 {
    pub player: PlayerId,
    pub life: i64,
    pub has_lost: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MagicMarkedDamageV4 {
    pub creature: OpaqueObjectId,
    pub amount: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MagicObservationV4 {
    pub schema_version: String,
    pub active_player: PlayerId,
    pub turn_number: String,
    pub turn_position: SyntheticTurnPosition,
    pub priority: SyntheticPriority,
    pub player_life: Vec<MagicPlayerLifeV4>,
    pub marked_damage: Vec<MagicMarkedDamageV4>,
    #[serde(deserialize_with = "deserialize_required_pending_ordering")]
    pub pending_sba_ordering: Option<MagicPendingSbaOrdering>,
    #[serde(deserialize_with = "deserialize_required_combat_v4")]
    pub combat: Option<MagicCombatParticipationV4>,
}

fn deserialize_required_combat_v4<'de, D>(
    deserializer: D,
) -> Result<Option<MagicCombatParticipationV4>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::deserialize(deserializer)
}

impl MagicObservationV4 {
    pub fn validate(&self) -> Result<(), ObservationValidationError> {
        if self.schema_version != MAGIC_OBSERVATION_SCHEMA_V4
            || parse_canonical_u64(&self.turn_number).is_err()
            || self.player_life.len() != 2
        {
            return Err(ObservationValidationError::ObservationPayload);
        }
        let mut players = std::collections::BTreeSet::new();
        if self
            .player_life
            .iter()
            .any(|item| !players.insert(item.player) || (item.has_lost && item.life > 0))
            || self
                .player_life
                .windows(2)
                .any(|pair| pair[0].player >= pair[1].player)
        {
            return Err(ObservationValidationError::ObservationPayload);
        }
        let mut marked = std::collections::BTreeSet::new();
        if self.marked_damage.iter().any(|item| {
            !marked.insert(item.creature)
                || parse_canonical_u64(&item.amount).is_err()
                || item.amount == "0"
        }) || self
            .marked_damage
            .windows(2)
            .any(|pair| pair[0].creature >= pair[1].creature)
        {
            return Err(ObservationValidationError::ObservationPayload);
        }
        if let Some(combat) = &self.combat {
            let attackers: std::collections::BTreeSet<_> =
                combat.attackers.iter().copied().collect();
            let mut blockers = std::collections::BTreeSet::new();
            if combat.defending_player == self.active_player
                || attackers.len() != combat.attackers.len()
                || combat.attackers.windows(2).any(|pair| pair[0] >= pair[1])
                || combat.blockers.len() != combat.attackers.len()
                || combat
                    .blockers
                    .iter()
                    .zip(&combat.attackers)
                    .any(|(entry, attacker)| {
                        entry.attacker != *attacker
                            || matches!(entry.status, MagicBlockedStatusV4::Unblocked)
                                && entry.blocker.is_some()
                            || entry.blocker.is_some_and(|blocker| {
                                !blockers.insert(blocker) || attackers.contains(&blocker)
                            })
                    })
            {
                return Err(ObservationValidationError::ObservationPayload);
            }
        }
        if let Some(progress) = &self.pending_sba_ordering {
            let mut owners = std::collections::BTreeSet::new();
            for order in &progress.completed_orders {
                if order.ordered_objects.len() < 2
                    || !owners.insert(order.owner)
                    || order.owner == progress.next_order_owner
                {
                    return Err(ObservationValidationError::ObservationPayload);
                }
                let mut objects = std::collections::BTreeSet::new();
                if order
                    .ordered_objects
                    .iter()
                    .any(|object| !objects.insert(*object))
                {
                    return Err(ObservationValidationError::ObservationPayload);
                }
            }
        }
        Ok(())
    }
}
