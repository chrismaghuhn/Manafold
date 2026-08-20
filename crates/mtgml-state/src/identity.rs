use std::collections::BTreeMap;

use mtgml_decision::PerspectiveIdentityResolver;
use mtgml_model::{AbilityInstanceId, GameObjectId, OpaqueAbilityId, OpaqueObjectId, PlayerId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityAllocatorState {
    pub next_object_id: mtgml_model::GameObjectId,
    pub next_ability_id: AbilityInstanceId,
    pub next_stack_object_id: mtgml_model::StackObjectId,
    pub next_effect_id: mtgml_model::EffectInstanceId,
    pub next_trigger_id: mtgml_model::TriggerInstanceId,
    pub next_decision_id: mtgml_model::DecisionId,
    pub next_continuation_id: mtgml_model::ContinuationId,
    pub next_rule_event_id: mtgml_model::RuleEventId,
    pub next_opaque_object_id: BTreeMap<PlayerId, OpaqueObjectId>,
    pub next_opaque_ability_id: BTreeMap<PlayerId, OpaqueAbilityId>,
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
            next_opaque_object_id: BTreeMap::new(),
            next_opaque_ability_id: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct PerspectiveIdentityMap {
    pub object_to_opaque: BTreeMap<GameObjectId, OpaqueObjectId>,
    pub opaque_to_object: BTreeMap<OpaqueObjectId, GameObjectId>,
    pub ability_to_opaque: BTreeMap<AbilityInstanceId, OpaqueAbilityId>,
    pub opaque_to_ability: BTreeMap<OpaqueAbilityId, AbilityInstanceId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct PerspectiveIdentityState {
    pub players: BTreeMap<PlayerId, PerspectiveIdentityMap>,
}

impl PerspectiveIdentityResolver for PerspectiveIdentityState {
    fn resolve_object(
        &self,
        perspective: PlayerId,
        opaque: OpaqueObjectId,
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
        opaque: OpaqueAbilityId,
    ) -> Option<AbilityInstanceId> {
        self.players
            .get(&perspective)?
            .opaque_to_ability
            .get(&opaque)
            .copied()
    }
}
