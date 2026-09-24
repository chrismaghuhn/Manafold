//! Perspective-safe current-state view of public turn and APNAP graveyard ordering.

use mtgml_model::{parse_canonical_u64, OpaqueObjectId, PlayerId};
use serde::{Deserialize, Serialize};

use crate::{ObservationValidationError, SyntheticPriority, SyntheticTurnPosition};

pub const MAGIC_OBSERVATION_SCHEMA_V1: &str = "magic-m3-observation.v1";

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
