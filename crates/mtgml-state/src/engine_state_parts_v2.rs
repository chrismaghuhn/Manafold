//! Detached complete replacement state for FullStateDigestV6.
//!
//! The embedded V5 `EngineStateParts` is preserved as-is. This successor
//! value adds the six V6 authoritative families without changing the current
//! EngineState or its digest path.

use std::collections::BTreeSet;

use crate::{validate_engine_state, CardRulesAuthoritativeStateV1, EngineState, EngineStateParts};
use mtgml_model::{GameObjectId, ZoneKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineStatePartsV2 {
    pub predecessor_v5: EngineStateParts,
    pub card_rules_state: CardRulesAuthoritativeStateV1,
}

impl EngineStatePartsV2 {
    pub fn from_state(
        state: &EngineState,
        card_rules_state: CardRulesAuthoritativeStateV1,
    ) -> Self {
        Self {
            predecessor_v5: state.parts(),
            card_rules_state,
        }
    }

    pub fn materialize(&self) -> EngineState {
        self.predecessor_v5.clone().into()
    }

    pub fn validate(&self) -> Result<(), EngineStatePartsV2Error> {
        let state = self.materialize();
        validate_engine_state(&state).map_err(|_| EngineStatePartsV2Error::PredecessorState)?;
        self.card_rules_state
            .validate()
            .map_err(|_| EngineStatePartsV2Error::CardRulesState)?;

        let players: BTreeSet<_> = state.core.players.keys().copied().collect();
        let mana_players: BTreeSet<_> = self.card_rules_state.mana.pools.keys().copied().collect();
        let history_players: BTreeSet<_> = self
            .card_rules_state
            .turn_history
            .players
            .keys()
            .copied()
            .collect();
        if mana_players != players || history_players != players {
            return Err(EngineStatePartsV2Error::PlayerUniverse);
        }
        if self.card_rules_state.turn_history.turn_number != state.core.turn_number {
            return Err(EngineStatePartsV2Error::TurnNumber);
        }

        let live: BTreeSet<GameObjectId> = state.zones.objects.keys().copied().collect();
        let on_battlefield = |object: &GameObjectId| {
            live.contains(object)
                && state
                    .zones
                    .locations
                    .get(object)
                    .is_some_and(|location| location.zone == ZoneKind::Battlefield)
        };
        if self
            .card_rules_state
            .counters
            .counters
            .keys()
            .any(|object| !on_battlefield(object))
            || self
                .card_rules_state
                .attachments
                .by_source
                .iter()
                .any(|(source, edge)| !on_battlefield(source) || !on_battlefield(&edge.target))
            || self
                .card_rules_state
                .faces
                .faces
                .keys()
                .any(|object| !live.contains(object))
            || self
                .card_rules_state
                .abilities
                .by_instance
                .values()
                .any(|ability| !live.contains(&ability.source))
        {
            return Err(EngineStatePartsV2Error::ObjectReference);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EngineStatePartsV2Error {
    #[error("predecessor EngineState is invalid")]
    PredecessorState,
    #[error("V6 card-rules state is invalid")]
    CardRulesState,
    #[error("V6 per-player state does not match the EngineState player universe")]
    PlayerUniverse,
    #[error("V6 turn history does not match the current turn number")]
    TurnNumber,
    #[error("V6 object family references a non-live GameObject incarnation")]
    ObjectReference,
}
