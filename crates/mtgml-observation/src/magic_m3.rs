//! Perspective-safe current-state view of public APNAP Graveyard ordering.

use mtgml_model::{parse_canonical_u64, OpaqueObjectId, PlayerId};
use serde::{Deserialize, Serialize};

use crate::{ObservationValidationError, SyntheticM3Priority, SyntheticM3TurnPosition};

pub const MAGIC_M3_OBSERVATION_SCHEMA: &str = "magic-m3-observation.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MagicM3CompletedOrder {
    pub owner: PlayerId,
    pub ordered_objects: Vec<OpaqueObjectId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MagicM3PendingSbaOrdering {
    pub completed_orders: Vec<MagicM3CompletedOrder>,
    pub next_order_owner: PlayerId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MagicM3Observation {
    pub schema_version: String,
    pub active_player: PlayerId,
    pub turn_number: String,
    pub turn_position: SyntheticM3TurnPosition,
    pub priority: SyntheticM3Priority,
    #[serde(deserialize_with = "deserialize_required_pending_ordering")]
    pub pending_sba_ordering: Option<MagicM3PendingSbaOrdering>,
}

fn deserialize_required_pending_ordering<'de, D>(
    deserializer: D,
) -> Result<Option<MagicM3PendingSbaOrdering>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::deserialize(deserializer)
}

impl MagicM3Observation {
    pub fn validate(&self) -> Result<(), ObservationValidationError> {
        if self.schema_version != MAGIC_M3_OBSERVATION_SCHEMA
            || parse_canonical_u64(&self.turn_number).is_err()
        {
            return Err(ObservationValidationError::M3Payload);
        }
        // Turn position and priority reuse closed, already-versioned enums.
        if let Some(progress) = &self.pending_sba_ordering {
            let mut owners = std::collections::BTreeSet::new();
            for order in &progress.completed_orders {
                if order.ordered_objects.len() < 2
                    || !owners.insert(order.owner)
                    || order.owner == progress.next_order_owner
                {
                    return Err(ObservationValidationError::M3Payload);
                }
                let mut objects = std::collections::BTreeSet::new();
                if order
                    .ordered_objects
                    .iter()
                    .any(|object| !objects.insert(*object))
                {
                    return Err(ObservationValidationError::M3Payload);
                }
            }
        }
        Ok(())
    }
}
