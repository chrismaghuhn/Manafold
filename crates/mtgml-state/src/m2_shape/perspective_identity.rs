//! Ownership: detached V2 perspective-local opaque identity shapes and
//! only their existing local shape validation.

use std::collections::{BTreeMap, BTreeSet};

use mtgml_model::{
    AbilityInstanceId, GameObjectId, OpaqueAbilityId, OpaqueObjectId, PlayerDecisionIdV1, PlayerId,
};
use serde::{Deserialize, Serialize};

use crate::m2_shape::M2ShapeViolation;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct PerspectiveIdentityRecordV2 {
    /// This is the one canonical persisted mapping direction.
    pub opaque_to_object: BTreeMap<OpaqueObjectId, GameObjectId>,
    pub opaque_to_ability: BTreeMap<OpaqueAbilityId, AbilityInstanceId>,
    /// Reverse maps are runtime validation indexes and are rebuilt/checked.
    #[serde(default)]
    pub object_to_opaque: BTreeMap<GameObjectId, OpaqueObjectId>,
    #[serde(default)]
    pub ability_to_opaque: BTreeMap<AbilityInstanceId, OpaqueAbilityId>,
    pub next_opaque_object_id: OpaqueObjectId,
    pub next_opaque_ability_id: OpaqueAbilityId,
    pub next_player_decision_id: PlayerDecisionIdV1,
    pub retired_object_ids: BTreeSet<OpaqueObjectId>,
    pub retired_ability_ids: BTreeSet<OpaqueAbilityId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct PerspectiveIdentityStateV2 {
    pub players: BTreeMap<PlayerId, PerspectiveIdentityRecordV2>,
}

pub(super) fn validate_identity(
    identity: &PerspectiveIdentityRecordV2,
) -> Result<(), M2ShapeViolation> {
    if identity.next_opaque_object_id.0 == 0
        || identity.next_opaque_ability_id.0 == 0
        || identity.next_player_decision_id.0 == 0
    {
        return Err(M2ShapeViolation::Allocator);
    }
    if identity.object_to_opaque.len() != identity.opaque_to_object.len()
        || identity.ability_to_opaque.len() != identity.opaque_to_ability.len()
    {
        return Err(M2ShapeViolation::IdentityMapping);
    }
    if identity
        .opaque_to_object
        .keys()
        .any(|opaque| identity.retired_object_ids.contains(opaque) || opaque.0 == 0)
        || identity
            .opaque_to_ability
            .keys()
            .any(|opaque| identity.retired_ability_ids.contains(opaque) || opaque.0 == 0)
        || identity
            .retired_object_ids
            .iter()
            .any(|opaque| opaque.0 == 0)
        || identity
            .retired_ability_ids
            .iter()
            .any(|opaque| opaque.0 == 0)
    {
        return Err(M2ShapeViolation::RetiredIdentity);
    }
    if identity
        .opaque_to_object
        .keys()
        .any(|opaque| opaque.0 >= identity.next_opaque_object_id.0)
        || identity
            .opaque_to_ability
            .keys()
            .any(|opaque| opaque.0 >= identity.next_opaque_ability_id.0)
    {
        return Err(M2ShapeViolation::Allocator);
    }
    for (object, opaque) in &identity.object_to_opaque {
        if identity.opaque_to_object.get(opaque) != Some(object) {
            return Err(M2ShapeViolation::IdentityMapping);
        }
    }
    for (ability, opaque) in &identity.ability_to_opaque {
        if identity.opaque_to_ability.get(opaque) != Some(ability) {
            return Err(M2ShapeViolation::IdentityMapping);
        }
    }
    Ok(())
}
