//! Detached public G0 observed-event V4 contract.

use mtgml_model::{OpaqueObjectId, PlayerId, VisibleSequence, ZoneKind};
use serde::{Deserialize, Serialize};

use crate::{
    error::ObservationValidationError, PublicStackItemV1, PublicTemporaryEffectV1,
    OBSERVED_EVENT_SCHEMA_V4,
};

fn deserialize_required_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::deserialize(deserializer)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservedCounterKindV3 {
    PlusOnePlusOne,
    MinusOneMinusOne,
    Lore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManaPoolAfterV1 {
    pub unrestricted: [u32; 6],
    pub creature_spell_only: [u32; 6],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservedFaceV1 {
    Front,
    Back,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManaPoolChangeCauseV2 {
    Produced,
    Emptied,
    Spent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StackItemRemovalCauseV1 {
    Resolved,
    Countered,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ObservedEventKindV4 {
    ObjectMoved {
        #[serde(deserialize_with = "deserialize_required_option")]
        old_object: Option<OpaqueObjectId>,
        #[serde(deserialize_with = "deserialize_required_option")]
        new_object: Option<OpaqueObjectId>,
        from: ZoneKind,
        to: ZoneKind,
        #[serde(deserialize_with = "deserialize_required_option")]
        entering_face: Option<ObservedFaceV1>,
        #[serde(deserialize_with = "deserialize_required_option")]
        tapped: Option<bool>,
    },
    ObjectCeasedToExist {
        object: OpaqueObjectId,
    },
    LifeChanged {
        player: PlayerId,
        from: i64,
        to: i64,
    },
    ObjectTapped {
        object: OpaqueObjectId,
        tapped: bool,
    },
    DecisionAvailable {
        actor: PlayerId,
    },
    RandomOutcomeVisible {
        label: String,
        exclusive_upper_bound: u64,
        value: u64,
    },
    PublicOutcome {
        code: String,
    },
    ManaPoolChanged {
        player: PlayerId,
        pool_after: ManaPoolAfterV1,
        cause: ManaPoolChangeCauseV2,
    },
    CountersChanged {
        object: OpaqueObjectId,
        counter_kind: ObservedCounterKindV3,
        from: u32,
        to: u32,
    },
    AttachmentChanged {
        source: OpaqueObjectId,
        #[serde(deserialize_with = "deserialize_required_option")]
        old_target: Option<OpaqueObjectId>,
        #[serde(deserialize_with = "deserialize_required_option")]
        new_target: Option<OpaqueObjectId>,
    },
    ObjectFaceChanged {
        object: OpaqueObjectId,
        face: ObservedFaceV1,
    },
    StackItemAdded {
        stack_position_from_top: u32,
        item: PublicStackItemV1,
    },
    StackItemRemoved {
        stack_position_from_top: u32,
        item: PublicStackItemV1,
        cause: StackItemRemovalCauseV1,
    },
    TemporaryEffectCreated {
        effect: PublicTemporaryEffectV1,
    },
    TemporaryEffectExpired {
        effect: PublicTemporaryEffectV1,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservedEventEnvelopeV4 {
    pub schema_version: String,
    pub sequence: VisibleSequence,
    pub event: ObservedEventKindV4,
}

impl ObservedEventEnvelopeV4 {
    pub fn validate(&self) -> Result<(), ObservationValidationError> {
        if self.schema_version != OBSERVED_EVENT_SCHEMA_V4 {
            return Err(ObservationValidationError::SchemaOrCodec);
        }
        match &self.event {
            ObservedEventKindV4::ObjectMoved {
                old_object,
                new_object,
                to,
                entering_face,
                tapped,
                ..
            } => {
                if old_object.is_none() && new_object.is_none() {
                    return Err(ObservationValidationError::ObjectMovedIdentity);
                }
                if (entering_face.is_some() || tapped.is_some())
                    && (*to != ZoneKind::Battlefield || new_object.is_none())
                {
                    return Err(ObservationValidationError::ObservationPayload);
                }
            }
            ObservedEventKindV4::RandomOutcomeVisible {
                label,
                exclusive_upper_bound,
                value,
            } => {
                if label.is_empty() {
                    return Err(ObservationValidationError::EmptyEventText);
                }
                if *exclusive_upper_bound == 0 || *value >= *exclusive_upper_bound {
                    return Err(ObservationValidationError::RandomOutcome);
                }
            }
            ObservedEventKindV4::PublicOutcome { code } if code.is_empty() => {
                return Err(ObservationValidationError::EmptyEventText);
            }
            ObservedEventKindV4::CountersChanged { from, to, .. } if from == to => {
                return Err(ObservationValidationError::ObservationPayload);
            }
            ObservedEventKindV4::AttachmentChanged {
                old_target,
                new_target,
                ..
            } if old_target == new_target => {
                return Err(ObservationValidationError::ObservationPayload);
            }
            ObservedEventKindV4::ManaPoolChanged {
                cause: ManaPoolChangeCauseV2::Emptied,
                pool_after,
                ..
            } if pool_after
                .unrestricted
                .iter()
                .chain(pool_after.creature_spell_only.iter())
                .any(|amount| *amount != 0) =>
            {
                return Err(ObservationValidationError::ObservationPayload);
            }
            ObservedEventKindV4::StackItemAdded { item, .. }
            | ObservedEventKindV4::StackItemRemoved { item, .. } => item.validate_public()?,
            ObservedEventKindV4::TemporaryEffectCreated { effect }
            | ObservedEventKindV4::TemporaryEffectExpired { effect } => effect.validate_public()?,
            _ => {}
        }
        Ok(())
    }
}
