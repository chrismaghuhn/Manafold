//! Passive public DTO for the synthetic turn and priority observation payload.

use mtgml_model::{parse_canonical_u64, PlayerId};
use serde::{Deserialize, Deserializer, Serialize};

use crate::error::ObservationValidationError;

pub const SYNTHETIC_OBSERVATION_SCHEMA_V1: &str = "synthetic-m3-observation.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyntheticBeginningStep {
    Untap,
    Upkeep,
    Draw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyntheticCombatStep {
    BeginningOfCombat,
    DeclareAttackers,
    DeclareBlockers,
    CombatDamage,
    EndOfCombat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyntheticEndingStep {
    EndStep,
    Cleanup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SyntheticTurnPosition {
    Beginning { step: SyntheticBeginningStep },
    PrecombatMain,
    Combat { step: SyntheticCombatStep },
    PostcombatMain,
    Ending { step: SyntheticEndingStep },
}

impl<'de> Deserialize<'de> for SyntheticTurnPosition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{Error as _, MapAccess, Visitor};
        use std::fmt;

        struct TurnPositionVisitor;

        impl<'de> Visitor<'de> for TurnPositionVisitor {
            type Value = SyntheticTurnPosition;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a closed synthetic turn position")
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
                        let step = SyntheticBeginningStep::deserialize(
                            serde::de::value::StringDeserializer::<A::Error>::new(step),
                        )
                        .map_err(A::Error::custom)?;
                        Ok(SyntheticTurnPosition::Beginning { step })
                    }
                    "precombat_main" => {
                        if step.is_some() {
                            return Err(A::Error::unknown_field("step", &["kind"]));
                        }
                        Ok(SyntheticTurnPosition::PrecombatMain)
                    }
                    "combat" => {
                        let step = step.ok_or_else(|| A::Error::missing_field("step"))?;
                        let step = SyntheticCombatStep::deserialize(
                            serde::de::value::StringDeserializer::<A::Error>::new(step),
                        )
                        .map_err(A::Error::custom)?;
                        Ok(SyntheticTurnPosition::Combat { step })
                    }
                    "postcombat_main" => {
                        if step.is_some() {
                            return Err(A::Error::unknown_field("step", &["kind"]));
                        }
                        Ok(SyntheticTurnPosition::PostcombatMain)
                    }
                    "ending" => {
                        let step = step.ok_or_else(|| A::Error::missing_field("step"))?;
                        let step = SyntheticEndingStep::deserialize(
                            serde::de::value::StringDeserializer::<A::Error>::new(step),
                        )
                        .map_err(A::Error::custom)?;
                        Ok(SyntheticTurnPosition::Ending { step })
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
pub enum SyntheticPriority {
    None,
    HeldBy { player: PlayerId },
}

impl<'de> Deserialize<'de> for SyntheticPriority {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{Error as _, MapAccess, Visitor};
        use std::fmt;

        struct PriorityVisitor;

        impl<'de> Visitor<'de> for PriorityVisitor {
            type Value = SyntheticPriority;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a closed synthetic priority")
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
                        Ok(SyntheticPriority::None)
                    }
                    "held_by" => {
                        let player = player.ok_or_else(|| A::Error::missing_field("player"))?;
                        let player = player.parse::<PlayerId>().map_err(A::Error::custom)?;
                        Ok(SyntheticPriority::HeldBy { player })
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
pub struct SyntheticObservation {
    pub schema_version: String,
    pub active_player: PlayerId,
    pub turn_number: String,
    pub turn_position: SyntheticTurnPosition,
    pub priority: SyntheticPriority,
}

impl SyntheticObservation {
    pub fn validate(&self) -> Result<(), ObservationValidationError> {
        if self.schema_version != SYNTHETIC_OBSERVATION_SCHEMA_V1 {
            return Err(ObservationValidationError::ObservationPayload);
        }
        parse_canonical_u64(&self.turn_number)
            .map_err(|_| ObservationValidationError::ObservationPayload)?;
        Ok(())
    }
}
