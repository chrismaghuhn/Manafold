//! Commander-format helpers over checkpointed `FormatState`.

use mtgml_model::{PhysicalCardId, PlayerId};
use mtgml_state::{CommanderState, FormatState};
use thiserror::Error;

pub fn commander_state(format: &FormatState) -> Result<&CommanderState, CommanderError> {
    match format {
        FormatState::Commander { state } => Ok(state),
        FormatState::None => Err(CommanderError::WrongFormat),
    }
}

pub fn additional_cast_cost(
    format: &FormatState,
    commander: PhysicalCardId,
) -> Result<u32, CommanderError> {
    let casts = commander_state(format)?
        .cast_counts
        .get(&commander)
        .copied()
        .ok_or(CommanderError::NotDesignated)?;
    Ok(casts.saturating_mul(2))
}

pub fn commander_damage(
    format: &FormatState,
    commander: PhysicalCardId,
    defending_player: PlayerId,
) -> Result<u32, CommanderError> {
    Ok(commander_state(format)?
        .damage
        .get(&commander)
        .and_then(|targets| targets.get(&defending_player))
        .copied()
        .unwrap_or(0))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum CommanderError {
    #[error("engine state is not configured for Commander")]
    WrongFormat,
    #[error("physical card is not a designated commander")]
    NotDesignated,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn commander_format(
        designations: Vec<PhysicalCardId>,
        cast_counts: Vec<(PhysicalCardId, u32)>,
    ) -> FormatState {
        FormatState::Commander {
            state: CommanderState {
                designations: BTreeMap::from([(PlayerId(1), designations)]),
                cast_counts: cast_counts.into_iter().collect(),
                damage: BTreeMap::new(),
            },
        }
    }

    #[test]
    fn fnd_032_designated_without_ledger_entry_has_zero_additional_cost() {
        let format = commander_format(vec![PhysicalCardId(7)], vec![]);
        assert_eq!(additional_cast_cost(&format, PhysicalCardId(7)), Ok(0));
    }

    #[test]
    fn fnd_032_designated_with_one_ledger_entry_has_existing_cost() {
        let format = commander_format(vec![PhysicalCardId(7)], vec![(PhysicalCardId(7), 1)]);
        assert_eq!(additional_cast_cost(&format, PhysicalCardId(7)), Ok(2));
    }

    #[test]
    fn fnd_032_ledger_entry_without_designation_is_not_designated() {
        let format = commander_format(vec![], vec![(PhysicalCardId(7), 1)]);
        assert_eq!(
            additional_cast_cost(&format, PhysicalCardId(7)),
            Err(CommanderError::NotDesignated)
        );
    }

    #[test]
    fn fnd_032_missing_designation_is_not_designated() {
        let format = commander_format(vec![PhysicalCardId(8)], vec![]);
        assert_eq!(
            additional_cast_cost(&format, PhysicalCardId(7)),
            Err(CommanderError::NotDesignated)
        );
    }
}
