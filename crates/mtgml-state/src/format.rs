use std::collections::BTreeMap;

use mtgml_model::{PhysicalCardId, PlayerId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct CommanderState {
    pub designations: BTreeMap<PlayerId, Vec<PhysicalCardId>>,
    pub cast_counts: BTreeMap<PhysicalCardId, u32>,
    pub damage: BTreeMap<PhysicalCardId, BTreeMap<PlayerId, u32>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FormatState {
    None,
    Commander { state: CommanderState },
}

impl Default for FormatState {
    fn default() -> Self {
        Self::None
    }
}
