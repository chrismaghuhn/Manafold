use mtgml_model::{GameObjectId, PlayerId};
use serde::{Deserialize, Serialize};

/// Public damage recipient in the selected player/creature damage profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DamageRecipientV1 {
    Player { player: PlayerId },
    Creature { object: GameObjectId },
}

/// One source-bound assignment inside a complete simultaneous damage package.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DamageAssignmentV1 {
    pub source: GameObjectId,
    pub recipient: DamageRecipientV1,
    pub amount: u64,
}
