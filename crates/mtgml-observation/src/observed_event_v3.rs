//! Detached M4 observed-event V3 public wire contract.

use mtgml_model::{OpaqueObjectId, PlayerId, StateRevision, VisibleSequence, ZoneKind};
use serde::{Deserialize, Serialize};

use crate::error::ObservationValidationError;
use crate::OBSERVED_EVENT_SCHEMA_V3;

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ObservedEventKindV3 {
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
        cause: ManaPoolChangeCauseV1,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManaPoolChangeCauseV1 {
    Produced,
    Emptied,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservedFaceV1 {
    Front,
    Back,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservedEventEnvelopeV3 {
    pub schema_version: String,
    pub sequence: VisibleSequence,
    pub state_revision: StateRevision,
    pub event: ObservedEventKindV3,
}

impl ObservedEventEnvelopeV3 {
    pub fn validate(&self) -> Result<(), ObservationValidationError> {
        if self.schema_version != OBSERVED_EVENT_SCHEMA_V3 {
            return Err(ObservationValidationError::SchemaOrCodec);
        }
        match &self.event {
            ObservedEventKindV3::ObjectMoved {
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
            ObservedEventKindV3::RandomOutcomeVisible {
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
            ObservedEventKindV3::PublicOutcome { code } if code.is_empty() => {
                return Err(ObservationValidationError::EmptyEventText);
            }
            ObservedEventKindV3::CountersChanged { from, to, .. } if from == to => {
                return Err(ObservationValidationError::ObservationPayload);
            }
            ObservedEventKindV3::AttachmentChanged {
                old_target,
                new_target,
                ..
            } if old_target == new_target => {
                return Err(ObservationValidationError::ObservationPayload);
            }
            ObservedEventKindV3::ManaPoolChanged {
                cause: ManaPoolChangeCauseV1::Emptied,
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
            _ => {}
        }
        Ok(())
    }
}

impl TryFrom<crate::ObservedEventEnvelopeV2> for ObservedEventEnvelopeV3 {
    type Error = ObservationValidationError;

    fn try_from(value: crate::ObservedEventEnvelopeV2) -> Result<Self, Self::Error> {
        use crate::{ObservedEventKindV2 as Old, ObservedEventKindV3 as New};
        let event = match value.event {
            Old::ObjectMoved {
                old_object,
                new_object,
                from,
                to,
            } => New::ObjectMoved {
                old_object,
                new_object,
                from,
                to,
                entering_face: None,
                tapped: None,
            },
            Old::ObjectCeasedToExist { object } => New::ObjectCeasedToExist { object },
            Old::LifeChanged { player, from, to } => New::LifeChanged { player, from, to },
            Old::ObjectTapped { object, tapped } => New::ObjectTapped { object, tapped },
            Old::DecisionAvailable { actor } => New::DecisionAvailable { actor },
            Old::RandomOutcomeVisible {
                label,
                exclusive_upper_bound,
                value,
            } => New::RandomOutcomeVisible {
                label,
                exclusive_upper_bound,
                value,
            },
            Old::PublicOutcome { code } => New::PublicOutcome { code },
        };
        let result = Self {
            schema_version: OBSERVED_EVENT_SCHEMA_V3.into(),
            sequence: value.sequence,
            state_revision: value.state_revision,
            event,
        };
        result.validate()?;
        Ok(result)
    }
}
