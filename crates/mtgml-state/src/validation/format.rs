//! Ownership: current Commander structural-reference validation only.

use std::collections::BTreeSet;

use super::EngineStateViolation;
use crate::engine::EngineState;
use crate::format::FormatState;

pub(super) fn validate_commander_format_references(
    state: &EngineState,
) -> Result<(), EngineStateViolation> {
    if let FormatState::Commander { state: commander } = &state.format {
        let mut designated = BTreeSet::new();
        for (player, cards) in &commander.designations {
            if !state.core.players.contains_key(player) || cards.is_empty() {
                return Err(EngineStateViolation::FormatMismatch);
            }
            if cards.iter().any(|card| !designated.insert(*card)) {
                return Err(EngineStateViolation::FormatMismatch);
            }
            if cards.iter().any(|card| {
                !state
                    .zones
                    .objects
                    .values()
                    .any(|object| object.physical_card == Some(*card) && object.owner == *player)
            }) {
                return Err(EngineStateViolation::FormatMismatch);
            }
        }
        if commander
            .cast_counts
            .keys()
            .chain(commander.damage.keys())
            .any(|card| !designated.contains(card))
        {
            return Err(EngineStateViolation::FormatMismatch);
        }
        if commander.damage.values().any(|targets| {
            targets
                .keys()
                .any(|player| !state.core.players.contains_key(player))
        }) {
            return Err(EngineStateViolation::FormatMismatch);
        }
    }
    Ok(())
}
