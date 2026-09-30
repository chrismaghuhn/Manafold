//! Detached full-replacement StateDeltaV2 bound to FullStateDigestV6.

use mtgml_model::StateRevision;

use crate::{CounterKindV1, ManaColorV1, ManaPoolV1, ManaRestrictionV1, SemanticDeltaOperation};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemanticDeltaOperationV2 {
    Existing {
        operation: Box<SemanticDeltaOperation>,
    },
    LandPlayCountChanged {
        player: mtgml_model::PlayerId,
        from: u8,
        to: u8,
    },
    ManaAdded {
        player: mtgml_model::PlayerId,
        color: ManaColorV1,
        restriction: ManaRestrictionV1,
        amount: u32,
        source: mtgml_model::GameObjectId,
        ability_key: u32,
    },
    ManaPoolEmptied {
        player: mtgml_model::PlayerId,
        previous_pool: ManaPoolV1,
    },
    ObjectTapped {
        object: mtgml_model::GameObjectId,
        from: bool,
        to: bool,
    },
    AbilityIdentityChanged {
        perspective: mtgml_model::PlayerId,
        instance: mtgml_model::AbilityInstanceId,
        from: Option<mtgml_model::OpaqueAbilityId>,
        to: Option<mtgml_model::OpaqueAbilityId>,
    },
    CounterChanged {
        object: mtgml_model::GameObjectId,
        kind: CounterKindV1,
        from: u32,
        to: u32,
        cause: mtgml_model::RuleEventId,
    },
    AttachmentChanged {
        source: mtgml_model::GameObjectId,
        from_target: Option<mtgml_model::GameObjectId>,
        to_target: Option<mtgml_model::GameObjectId>,
        timestamp_revision: StateRevision,
        operation_ordinal: u32,
    },
    ObjectFaceChanged {
        object: mtgml_model::GameObjectId,
        from_face: u32,
        to_face: u32,
    },
    ObjectEntered {
        old_object: Option<mtgml_model::GameObjectId>,
        new_object: mtgml_model::GameObjectId,
        from_zone: mtgml_model::ZoneKind,
        to_zone: mtgml_model::ZoneKind,
        tapped: bool,
        face: u32,
    },
    AbilityAuthorityAdded {
        instance: mtgml_model::AbilityInstanceId,
        source: mtgml_model::GameObjectId,
        ability_key: u32,
    },
    AbilityAuthorityRemoved {
        instance: mtgml_model::AbilityInstanceId,
        source: mtgml_model::GameObjectId,
        ability_key: u32,
    },
}
