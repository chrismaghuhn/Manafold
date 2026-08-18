//! Experimental card-IR vocabulary.
//!
//! ADR-0004 fixes the direction—typed, inspectable, serializable IR—but **not**
//! these concrete variants. The vocabulary remains unstable until M2.5 locks
//! the first exact deck closure.

use mtgml_model::CardDefinitionId;
use serde::{Deserialize, Serialize};

pub const CARD_IR_STABILITY: &str = "experimental-not-a-stable-semantic-contract";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ExperimentalEffect {
    NoOp,
    Draw { count: u32 },
    LoseLife { amount: i64 },
    MoveZonePrototype { referenced_card: CardDefinitionId },
}
