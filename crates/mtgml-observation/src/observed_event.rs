//! Ownership: observed-event DTOs for both current generations (V1 and V2)
//! and their existing validation.

use mtgml_model::{
    EventSequence, OpaqueObjectId, PlayerId, StateRevision, VisibleSequence, ZoneKind,
};
use serde::{Deserialize, Serialize};

use crate::error::ObservationValidationError;
use crate::{OBSERVED_EVENT_SCHEMA, OBSERVED_EVENT_SCHEMA_V2};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ObservedEventKind {
    ObjectMoved {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        old_object: Option<OpaqueObjectId>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        new_object: Option<OpaqueObjectId>,
        from: ZoneKind,
        to: ZoneKind,
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservedEventEnvelope {
    pub schema_version: String,
    pub sequence: EventSequence,
    pub state_revision: StateRevision,
    pub event: ObservedEventKind,
}

impl ObservedEventEnvelope {
    pub fn validate(&self) -> Result<(), ObservationValidationError> {
        if self.schema_version != OBSERVED_EVENT_SCHEMA {
            return Err(ObservationValidationError::SchemaOrCodec);
        }
        match &self.event {
            ObservedEventKind::RandomOutcomeVisible {
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
            ObservedEventKind::PublicOutcome { code } if code.is_empty() => {
                return Err(ObservationValidationError::EmptyEventText);
            }
            _ => {}
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ObservedEventKindV2 {
    ObjectMoved {
        old_object: Option<OpaqueObjectId>,
        new_object: Option<OpaqueObjectId>,
        from: ZoneKind,
        to: ZoneKind,
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservedEventEnvelopeV2 {
    pub schema_version: String,
    pub sequence: VisibleSequence,
    pub state_revision: StateRevision,
    pub event: ObservedEventKindV2,
}

impl ObservedEventEnvelopeV2 {
    pub fn validate(&self) -> Result<(), ObservationValidationError> {
        if self.schema_version != OBSERVED_EVENT_SCHEMA_V2 {
            return Err(ObservationValidationError::SchemaOrCodec);
        }
        match &self.event {
            ObservedEventKindV2::RandomOutcomeVisible {
                label,
                exclusive_upper_bound,
                value,
            } if label.is_empty()
                || *exclusive_upper_bound == 0
                || *value >= *exclusive_upper_bound =>
            {
                Err(ObservationValidationError::RandomOutcome)
            }
            ObservedEventKindV2::PublicOutcome { code } if code.is_empty() => {
                Err(ObservationValidationError::EmptyEventText)
            }
            _ => Ok(()),
        }
    }
}
