use mtgml_model::{AbilityInstanceId, GameObjectId, PlayerId};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::m2_shape::PerspectiveIdentityStateV2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum IdentityAllocationError {
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
