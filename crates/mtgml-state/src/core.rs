use std::collections::{BTreeMap, BTreeSet};

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
    /// True only after the combat-damage turn-based action and its bounded
    /// simultaneous package have completed.
    pub damage_step_completed: bool,
    /// Attackers which became blocked during blocker declaration. This fact
    /// survives removal of every live blocker (CR 509.1h).
    pub blocked_attackers: BTreeSet<mtgml_model::GameObjectId>,
    pub blockers: BTreeMap<mtgml_model::GameObjectId, Option<mtgml_model::GameObjectId>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CombatBlockerAssignmentV1 {
    pub attacker: mtgml_model::GameObjectId,
    pub blocker: Option<mtgml_model::GameObjectId>,
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

#[cfg(test)]
mod blocked_history_characterization {
    use super::{CombatState, PlayerId};
    use mtgml_model::GameObjectId;
    use std::collections::{BTreeMap, BTreeSet};

    #[test]
    fn unblocked_and_blocked_with_removed_blocker_are_distinguishable() {
        // These are two different rule histories. The second is explicitly
        // covered by CR 509.1h / 510.1c: it remains blocked but has no live
        // blocker, so it must not assign damage to the defending player.
        let never_blocked = CombatState {
            defending_player: PlayerId(2),
            attackers: vec![GameObjectId(10)],
            damage_step_completed: false,
            blocked_attackers: BTreeSet::new(),
            blockers: BTreeMap::from([(GameObjectId(10), None)]),
        };
        let was_blocked_then_blocker_left = CombatState {
            defending_player: PlayerId(2),
            attackers: vec![GameObjectId(10)],
            damage_step_completed: false,
            blocked_attackers: BTreeSet::from([GameObjectId(10)]),
            blockers: BTreeMap::from([(GameObjectId(10), None)]),
        };

        assert_ne!(never_blocked, was_blocked_then_blocker_left);
    }
}
