use mtgml_model::{OpaqueAbilityId, OpaqueObjectId, PlayerId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionVisibility {
    Public,
    ActingPlayerOnly,
    Mixed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CandidateIntent {
    PassPriority,
    CastSpell { object: OpaqueObjectId },
    ActivateAbility { ability: OpaqueAbilityId },
    SelectObject { object: OpaqueObjectId },
    SelectPlayer { player: PlayerId },
    SelectMode { mode_index: u32 },
    ChooseBoolean { value: bool },
    DeclareNumber { value: i64 },
    Confirm,
}
