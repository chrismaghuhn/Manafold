use std::collections::BTreeMap;

use mtgml_model::PlayerId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerState {
    pub life: i64,
    pub has_lost: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BeginningStep {
    Untap,
    Upkeep,
    Draw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatStep {
    BeginningOfCombat,
    DeclareAttackers,
    DeclareBlockers,
    CombatDamage,
    EndOfCombat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EndingStep {
    EndStep,
    Cleanup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TurnPosition {
    Beginning { step: BeginningStep },
    PrecombatMain,
    Combat { step: CombatStep },
    PostcombatMain,
    Ending { step: EndingStep },
}

impl TurnPosition {
    pub const fn canonical_rank(self) -> u8 {
        match self {
            Self::Beginning {
                step: BeginningStep::Untap,
            } => 0,
            Self::Beginning {
                step: BeginningStep::Upkeep,
            } => 1,
            Self::Beginning {
                step: BeginningStep::Draw,
            } => 2,
            Self::PrecombatMain => 3,
            Self::Combat {
                step: CombatStep::BeginningOfCombat,
            } => 4,
            Self::Combat {
                step: CombatStep::DeclareAttackers,
            } => 5,
            Self::Combat {
                step: CombatStep::DeclareBlockers,
            } => 6,
            Self::Combat {
                step: CombatStep::CombatDamage,
            } => 7,
            Self::Combat {
                step: CombatStep::EndOfCombat,
            } => 8,
            Self::PostcombatMain => 9,
            Self::Ending {
                step: EndingStep::EndStep,
            } => 10,
            Self::Ending {
                step: EndingStep::Cleanup,
            } => 11,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PriorityState {
    None,
    HeldBy {
        player: PlayerId,
        consecutive_passes: u8,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CombatState {
    pub defending_player: PlayerId,
    pub attackers: Vec<mtgml_model::GameObjectId>,
    pub blockers: BTreeMap<mtgml_model::GameObjectId, Option<mtgml_model::GameObjectId>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FoundationSourceKind {
    Creature,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum BaseCharacteristics {
    Simple { power: i64, toughness: i64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ControlHistory {
    BeforeTurnStart {
        turn_number: u64,
    },
    DuringTurn {
        turn_number: u64,
        boundary: TurnPosition,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FoundationCreatureSource {
    pub source_kind: FoundationSourceKind,
    pub base_characteristics: BaseCharacteristics,
    pub marked_damage: u64,
    pub control_history: ControlHistory,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoreRulesState {
    pub players: BTreeMap<PlayerId, PlayerState>,
    pub active_player: PlayerId,
    pub turn_number: u64,
    pub position: TurnPosition,
    pub priority: PriorityState,
}
