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
        let abilities = &self.card_rules_state.abilities.by_instance;
        if abilities
            .keys()
            .any(|ability| ability.0 >= state.allocators.next_ability_id.0)
        {
            return Err(EngineStatePartsV2Error::AbilityAllocator);
        }
        if self
            .card_rules_state
            .turn_history
            .target_occurrences
            .iter()
            .any(|(object, player)| !live.contains(object) || !players.contains(player))
            || self
                .card_rules_state
                .turn_history
                .once_ability_used
                .iter()
                .any(|(object, key)| {
                    !live.contains(object)
                        || !abilities.values().any(|authority| {
                            authority.source == *object && authority.ability_key == *key
                        })
                })
        {
            return Err(EngineStatePartsV2Error::HistoryReference);
        }
        if state
            .perspective_identities
            .players
            .values()
            .flat_map(|identity| identity.opaque_to_ability.values())
            .any(|ability| !abilities.contains_key(ability))
        {
            return Err(EngineStatePartsV2Error::AbilityAuthorityReference);
        }
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
    #[error("V6 ability authority is not below the predecessor ability allocator")]
    AbilityAllocator,
    #[error("V6 turn history references a non-live object or unknown ability")]
    HistoryReference,
    #[error("active opaque ability identity has no V6 ability authority")]
    AbilityAuthorityReference,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        construct_synthetic_engine_state, AbilityAuthorityV1, CardRulesAuthoritativeStateV1,
        ManaPoolV1, ManaStateV1, PlayerTurnHistoryV1, SyntheticResetInputs, SyntheticV4Setup,
        TurnHistoryStateV1,
    };
    use mtgml_model::{AbilityInstanceId, OpaqueAbilityId, PlayerId};
    use mtgml_random::RootSeed256;
    use std::collections::{BTreeMap, BTreeSet};

    fn parts() -> EngineStatePartsV2 {
        let engine = construct_synthetic_engine_state(SyntheticResetInputs {
            players: [PlayerId(1), PlayerId(2)],
            root_seed: RootSeed256::from_lower_hex(&"74".repeat(32)).unwrap(),
            setup: SyntheticV4Setup::synthetic_compatibility(),
        })
        .unwrap();
        let mut mana = ManaStateV1::default();
        let mut players = BTreeMap::new();
        for player in engine.core.players.keys().copied() {
            mana.pools.insert(player, ManaPoolV1::default());
            players.insert(player, PlayerTurnHistoryV1::default());
        }
        EngineStatePartsV2::from_state(
            &engine,
            CardRulesAuthoritativeStateV1 {
                mana,
                turn_history: TurnHistoryStateV1 {
                    turn_number: engine.core.turn_number,
                    players,
                    ..TurnHistoryStateV1::default()
                },
                ..CardRulesAuthoritativeStateV1::default()
            },
        )
    }

    #[test]
    fn cross_state_authority_and_history_references_fail_closed() {
        let mut allocator = parts();
        allocator.card_rules_state.abilities.by_instance.insert(
            AbilityInstanceId(1),
            AbilityAuthorityV1 {
                source: GameObjectId(1),
                ability_key: 0,
            },
        );
        allocator.predecessor_v5.allocators.next_ability_id = AbilityInstanceId(1);
        assert_eq!(
            allocator.validate(),
            Err(EngineStatePartsV2Error::AbilityAllocator)
        );

        let mut opaque = parts();
        let identity = opaque
            .predecessor_v5
            .perspective_identities
            .players
            .get_mut(&PlayerId(1))
            .unwrap();
        identity
            .opaque_to_ability
            .insert(OpaqueAbilityId(1), AbilityInstanceId(999));
        identity
            .ability_to_opaque
            .insert(AbilityInstanceId(999), OpaqueAbilityId(1));
        identity.next_opaque_ability_id = OpaqueAbilityId(2);
        opaque.predecessor_v5.allocators.next_ability_id = AbilityInstanceId(1000);
        assert_eq!(
            opaque.validate(),
            Err(EngineStatePartsV2Error::AbilityAuthorityReference)
        );

        let mut target = parts();
        target.card_rules_state.turn_history.target_occurrences =
            BTreeSet::from([(GameObjectId(999), PlayerId(1))]);
        assert_eq!(
            target.validate(),
            Err(EngineStatePartsV2Error::HistoryReference)
        );

        let before = parts();
        let mut invalid_delta = crate::StateDeltaV2::between(&before, &before, Vec::new()).unwrap();
        invalid_delta
            .replacement
            .card_rules_state
            .turn_history
            .target_occurrences = BTreeSet::from([(GameObjectId(999), PlayerId(1))]);
        assert_eq!(
            invalid_delta.apply(&before),
            Err(crate::DeltaApplicationV2Error::InvalidReplacement(
                EngineStatePartsV2Error::HistoryReference
            ))
        );

        let mut used = parts();
        used.card_rules_state.turn_history.once_ability_used =
            BTreeSet::from([(GameObjectId(1), 7)]);
        assert_eq!(
            used.validate(),
            Err(EngineStatePartsV2Error::HistoryReference)
        );
    }

    #[test]
    fn cross_state_live_ability_authority_relationships_validate() {
        let mut state = parts();
        state.card_rules_state.abilities.by_instance.insert(
            AbilityInstanceId(1),
            AbilityAuthorityV1 {
                source: GameObjectId(1),
                ability_key: 7,
            },
        );
        state.predecessor_v5.allocators.next_ability_id = AbilityInstanceId(2);
        state.card_rules_state.turn_history.once_ability_used =
            BTreeSet::from([(GameObjectId(1), 7)]);
        let identity = state
            .predecessor_v5
            .perspective_identities
            .players
            .get_mut(&PlayerId(1))
            .unwrap();
        identity
            .opaque_to_ability
            .insert(OpaqueAbilityId(1), AbilityInstanceId(1));
        identity
            .ability_to_opaque
            .insert(AbilityInstanceId(1), OpaqueAbilityId(1));
        identity.next_opaque_ability_id = OpaqueAbilityId(2);
        state.validate().unwrap();
    }
}
