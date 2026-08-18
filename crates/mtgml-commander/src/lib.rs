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
