//! Passive public DTO for the M3 synthetic observation payload.

use mtgml_model::{parse_canonical_u64, PlayerId};
use serde::{Deserialize, Serialize};

use crate::error::ObservationValidationError;

pub const SYNTHETIC_M3_OBSERVATION_SCHEMA: &str = "synthetic-m3-observation.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyntheticM3BeginningStep {
    Untap,
    Upkeep,
    Draw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyntheticM3CombatStep {
    BeginningOfCombat,
    DeclareAttackers,
    DeclareBlockers,
    CombatDamage,
    EndOfCombat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyntheticM3EndingStep {
    EndStep,
    Cleanup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SyntheticM3TurnPosition {
    Beginning { step: SyntheticM3BeginningStep },
    PrecombatMain,
    Combat { step: SyntheticM3CombatStep },
    PostcombatMain,
    Ending { step: SyntheticM3EndingStep },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SyntheticM3Priority {
    None,
    HeldBy { player: PlayerId },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SyntheticM3Observation {
    pub schema_version: String,
    pub active_player: PlayerId,
    pub turn_number: String,
    pub turn_position: SyntheticM3TurnPosition,
    pub priority: SyntheticM3Priority,
}

impl SyntheticM3Observation {
    pub fn validate(&self) -> Result<(), ObservationValidationError> {
        if self.schema_version != SYNTHETIC_M3_OBSERVATION_SCHEMA {
            return Err(ObservationValidationError::M3Payload);
        }
        parse_canonical_u64(&self.turn_number)
            .map_err(|_| ObservationValidationError::M3Payload)?;
        Ok(())
    }
}
