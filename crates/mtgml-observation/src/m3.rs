//! Passive public DTO for the M3 synthetic observation payload.

use mtgml_model::{parse_canonical_u64, PlayerId};
use serde::{Deserialize, Deserializer, Serialize};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SyntheticM3TurnPosition {
    Beginning { step: SyntheticM3BeginningStep },
    PrecombatMain,
    Combat { step: SyntheticM3CombatStep },
    PostcombatMain,
    Ending { step: SyntheticM3EndingStep },
}

impl<'de> Deserialize<'de> for SyntheticM3TurnPosition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{Error as _, MapAccess, Visitor};
        use std::fmt;

        struct TurnPositionVisitor;

        impl<'de> Visitor<'de> for TurnPositionVisitor {
            type Value = SyntheticM3TurnPosition;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a closed synthetic M3 turn position")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut kind: Option<String> = None;
                let mut step: Option<String> = None;
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "kind" => {
                            if kind.is_some() {
                                return Err(A::Error::duplicate_field("kind"));
                            }
                            kind = Some(map.next_value()?);
                        }
                        "step" => {
                            if step.is_some() {
                                return Err(A::Error::duplicate_field("step"));
                            }
                            step = Some(map.next_value()?);
                        }
                        other => {
                            return Err(A::Error::unknown_field(other, &["kind", "step"]));
                        }
                    }
                }
                let kind = kind.ok_or_else(|| A::Error::missing_field("kind"))?;
                match kind.as_str() {
                    "beginning" => {
                        let step = step.ok_or_else(|| A::Error::missing_field("step"))?;
                        // Delegate to the derived step decoder so the closed
                        // step vocabulary stays single-sourced (no duplicated
                        // wire literals here).
                        let step = SyntheticM3BeginningStep::deserialize(
                            serde::de::value::StringDeserializer::<A::Error>::new(step),
                        )
                        .map_err(A::Error::custom)?;
                        Ok(SyntheticM3TurnPosition::Beginning { step })
                    }
                    "precombat_main" => {
                        if step.is_some() {
                            return Err(A::Error::unknown_field("step", &["kind"]));
                        }
                        Ok(SyntheticM3TurnPosition::PrecombatMain)
                    }
                    "combat" => {
                        let step = step.ok_or_else(|| A::Error::missing_field("step"))?;
                        let step = SyntheticM3CombatStep::deserialize(
                            serde::de::value::StringDeserializer::<A::Error>::new(step),
                        )
                        .map_err(A::Error::custom)?;
                        Ok(SyntheticM3TurnPosition::Combat { step })
                    }
                    "postcombat_main" => {
                        if step.is_some() {
                            return Err(A::Error::unknown_field("step", &["kind"]));
                        }
                        Ok(SyntheticM3TurnPosition::PostcombatMain)
                    }
                    "ending" => {
                        let step = step.ok_or_else(|| A::Error::missing_field("step"))?;
                        let step = SyntheticM3EndingStep::deserialize(
                            serde::de::value::StringDeserializer::<A::Error>::new(step),
                        )
                        .map_err(A::Error::custom)?;
                        Ok(SyntheticM3TurnPosition::Ending { step })
                    }
                    _ => Err(A::Error::unknown_variant(
                        &kind,
                        &[
                            "beginning",
                            "precombat_main",
                            "combat",
                            "postcombat_main",
                            "ending",
                        ],
                    )),
                }
            }
        }

        deserializer.deserialize_map(TurnPositionVisitor)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SyntheticM3Priority {
    None,
    HeldBy { player: PlayerId },
}

impl<'de> Deserialize<'de> for SyntheticM3Priority {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{Error as _, MapAccess, Visitor};
        use std::fmt;

        struct PriorityVisitor;

        impl<'de> Visitor<'de> for PriorityVisitor {
            type Value = SyntheticM3Priority;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a closed synthetic M3 priority")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut kind: Option<String> = None;
                let mut player: Option<String> = None;
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "kind" => {
                            if kind.is_some() {
                                return Err(A::Error::duplicate_field("kind"));
                            }
                            kind = Some(map.next_value()?);
                        }
                        "player" => {
                            if player.is_some() {
                                return Err(A::Error::duplicate_field("player"));
                            }
                            player = Some(map.next_value()?);
                        }
                        other => {
                            return Err(A::Error::unknown_field(other, &["kind", "player"]));
                        }
                    }
                }
                let kind = kind.ok_or_else(|| A::Error::missing_field("kind"))?;
                match kind.as_str() {
                    "none" => {
                        if player.is_some() {
                            return Err(A::Error::unknown_field("player", &["kind"]));
                        }
                        Ok(SyntheticM3Priority::None)
                    }
                    "held_by" => {
                        let player = player.ok_or_else(|| A::Error::missing_field("player"))?;
                        let player = player.parse::<PlayerId>().map_err(A::Error::custom)?;
                        Ok(SyntheticM3Priority::HeldBy { player })
                    }
                    _ => Err(A::Error::unknown_variant(&kind, &["none", "held_by"])),
                }
            }
        }

        deserializer.deserialize_map(PriorityVisitor)
    }
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
