use std::collections::BTreeMap;

use mtgml_model::PlayerId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerState {
    pub life: i64,
    pub has_lost: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoreRulesState {
    pub players: BTreeMap<PlayerId, PlayerState>,
    pub active_player: PlayerId,
    pub priority_player: PlayerId,
    pub turn_number: u64,
}
