//! Ownership: authoritative random-state validation only.

use mtgml_model::PlayerId;

use super::EngineStateViolation;
use crate::engine::EngineState;

pub(super) fn validate_authoritative_random_state(
    state: &EngineState,
) -> Result<(), EngineStateViolation> {
    state
        .random
        .validate()
        .map_err(|_| EngineStateViolation::RandomState)?;
    for key in state.random.streams.keys() {
        if let Some(player_raw) = key.player() {
            if !state.core.players.contains_key(&PlayerId(player_raw)) {
                return Err(EngineStateViolation::RandomState);
            }
        }
    }
    Ok(())
}
