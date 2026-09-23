use mtgml_model::{AbilityInstanceId, GameObjectId, PlayerId};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::m2_shape::PerspectiveIdentityStateV2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum IdentityAllocationError {
    #[error("game object identity is exhausted")]
    GameObjectIdExhausted,
    #[error("effect instance identity is exhausted")]
    EffectInstanceIdExhausted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityAllocatorState {
    pub next_object_id: GameObjectId,
    pub next_ability_id: AbilityInstanceId,
    pub next_stack_object_id: mtgml_model::StackObjectId,
    pub next_effect_id: mtgml_model::EffectInstanceId,
    pub next_trigger_id: mtgml_model::TriggerInstanceId,
    pub next_decision_id: mtgml_model::DecisionId,
    pub next_continuation_id: mtgml_model::ContinuationId,
    pub next_rule_event_id: mtgml_model::RuleEventId,
}

impl IdentityAllocatorState {
    pub fn allocate_object_id(&mut self) -> Result<GameObjectId, IdentityAllocationError> {
        let allocated = self.next_object_id;
        let next = allocated
            .0
            .checked_add(1)
            .map(GameObjectId)
            .ok_or(IdentityAllocationError::GameObjectIdExhausted)?;
        self.next_object_id = next;
        Ok(allocated)
    }

    pub fn allocate_effect_id(
        &mut self,
    ) -> Result<mtgml_model::EffectInstanceId, IdentityAllocationError> {
        let allocated = self.next_effect_id;
        if allocated.0 == u64::MAX {
            return Err(IdentityAllocationError::EffectInstanceIdExhausted);
        }
        self.next_effect_id = mtgml_model::EffectInstanceId(
            allocated
                .0
                .checked_add(1)
                .ok_or(IdentityAllocationError::EffectInstanceIdExhausted)?,
        );
        Ok(allocated)
    }
}

#[cfg(test)]
mod tests {
    use super::{IdentityAllocationError, IdentityAllocatorState};
    use mtgml_model::GameObjectId;

    #[test]
    fn object_id_allocation_advances_exactly_once() {
        let mut allocators = IdentityAllocatorState {
            next_object_id: GameObjectId(41),
            ..IdentityAllocatorState::default()
        };

        assert_eq!(allocators.allocate_object_id(), Ok(GameObjectId(41)));
        assert_eq!(allocators.next_object_id, GameObjectId(42));
    }

    #[test]
    fn object_id_exhaustion_preserves_allocator() {
        let mut allocators = IdentityAllocatorState {
            next_object_id: GameObjectId(u64::MAX),
            ..IdentityAllocatorState::default()
        };
        let before = allocators.next_object_id;

        assert_eq!(
            allocators.allocate_object_id(),
            Err(IdentityAllocationError::GameObjectIdExhausted)
        );
        assert_eq!(allocators.next_object_id, before);
    }
}

impl Default for IdentityAllocatorState {
    fn default() -> Self {
        Self {
            next_object_id: GameObjectId(1),
            next_ability_id: AbilityInstanceId(1),
            next_stack_object_id: mtgml_model::StackObjectId(1),
            next_effect_id: mtgml_model::EffectInstanceId(1),
            next_trigger_id: mtgml_model::TriggerInstanceId(1),
            next_decision_id: mtgml_model::DecisionId(1),
            next_continuation_id: mtgml_model::ContinuationId(1),
            next_rule_event_id: mtgml_model::RuleEventId(1),
        }
    }
}

impl mtgml_decision::PerspectiveIdentityResolver for PerspectiveIdentityStateV2 {
    fn resolve_object(
        &self,
        perspective: PlayerId,
        opaque: mtgml_model::OpaqueObjectId,
    ) -> Option<GameObjectId> {
        self.players
            .get(&perspective)?
            .opaque_to_object
            .get(&opaque)
            .copied()
    }

    fn resolve_ability(
        &self,
        perspective: PlayerId,
        opaque: mtgml_model::OpaqueAbilityId,
    ) -> Option<AbilityInstanceId> {
        self.players
            .get(&perspective)?
            .opaque_to_ability
            .get(&opaque)
            .copied()
    }
}
