//! Perspective-safe observations, information states, and observed events.

use base64::{engine::general_purpose::STANDARD, Engine as _};
use mtgml_decision::PlayerDecisionRequest;
use mtgml_model::{
    EpisodeStatus, EventSequence, InformationStateDigest, ObservationDigest, OpaqueObjectId,
    PlayerId, StateRevision, ZoneKind,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const OBSERVATION_SCHEMA: &str = "observation-envelope.v1";
pub const INFORMATION_STATE_SCHEMA: &str = "information-state-envelope.v1";
pub const OBSERVED_EVENT_SCHEMA: &str = "observed-event-envelope.v1";
pub const PLAYER_STEP_SCHEMA: &str = "player-step.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationEnvelope {
    pub schema_version: String,
    pub perspective: PlayerId,
    pub state_revision: StateRevision,
    pub payload_codec: String,
    pub payload_base64: String,
    pub digest: ObservationDigest,
}

impl ObservationEnvelope {
    pub fn validate(&self) -> Result<(), ObservationValidationError> {
        if self.schema_version != OBSERVATION_SCHEMA || self.payload_codec.is_empty() {
            return Err(ObservationValidationError::SchemaOrCodec);
        }
        let decoded = STANDARD
            .decode(&self.payload_base64)
            .map_err(|_| ObservationValidationError::Base64)?;
        if STANDARD.encode(decoded) != self.payload_base64 {
            return Err(ObservationValidationError::Base64);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InformationStateEnvelope {
    pub schema_version: String,
    pub perspective: PlayerId,
    pub state_revision: StateRevision,
    pub current_observation: ObservationEnvelope,
    pub public_history_length: u64,
    pub private_history_length: u64,
    pub digest: InformationStateDigest,
}

impl InformationStateEnvelope {
    pub fn validate(&self) -> Result<(), ObservationValidationError> {
        if self.schema_version != INFORMATION_STATE_SCHEMA {
            return Err(ObservationValidationError::SchemaOrCodec);
        }
        self.current_observation.validate()?;
        if self.current_observation.perspective != self.perspective
            || self.current_observation.state_revision != self.state_revision
        {
            return Err(ObservationValidationError::InformationStateMismatch);
        }
        Ok(())
    }
}

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
#[serde(deny_unknown_fields)]
pub struct PlayerStep {
    pub schema_version: String,
    pub information_state: InformationStateEnvelope,
    pub observed_events: Vec<ObservedEventEnvelope>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_decision: Option<PlayerDecisionRequest>,
    pub status: EpisodeStatus,
}

impl PlayerStep {
    pub fn validate(&self) -> Result<(), ObservationValidationError> {
        if self.schema_version != PLAYER_STEP_SCHEMA {
            return Err(ObservationValidationError::SchemaOrCodec);
        }
        self.information_state.validate()?;
        self.status
            .validate()
            .map_err(|_| ObservationValidationError::EpisodeStatus)?;
        let perspective = self.information_state.perspective;
        let revision = self.information_state.state_revision;
        for event in &self.observed_events {
            event.validate()?;
            if event.state_revision > revision {
                return Err(ObservationValidationError::FutureEvent);
            }
        }
        if let Some(decision) = &self.next_decision {
            decision
                .validate()
                .map_err(|_| ObservationValidationError::Decision)?;
            if decision.actor != perspective || decision.state_revision != revision {
                return Err(ObservationValidationError::Decision);
            }
        }
        if !matches!(&self.status, EpisodeStatus::Running) && self.next_decision.is_some() {
            return Err(ObservationValidationError::Decision);
        }
        Ok(())
    }

    pub fn observation(&self) -> &ObservationEnvelope {
        &self.information_state.current_observation
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ObservationValidationError {
    #[error("unsupported schema version or empty codec")]
    SchemaOrCodec,
    #[error("payload is not canonical base64")]
    Base64,
    #[error("information state and current observation disagree")]
    InformationStateMismatch,
    #[error("observed event label/code must be non-empty")]
    EmptyEventText,
    #[error("random outcome is outside its declared range")]
    RandomOutcome,
    #[error("observed event belongs to a future revision")]
    FutureEvent,
    #[error("next decision is invalid for this endpoint")]
    Decision,
    #[error("episode status is invalid")]
    EpisodeStatus,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_seven_observed_event_variants_deserialize() {
        let events = [
            concat!(
                r#"{"kind":"object_moved","old_object":"1","new_object":"2","#,
                r#""from":"battlefield","to":"graveyard"}"#,
            ),
            r#"{"kind":"object_ceased_to_exist","object":"1"}"#,
            r#"{"kind":"life_changed","player":"1","from":40,"to":39}"#,
            r#"{"kind":"object_tapped","object":"1","tapped":true}"#,
            r#"{"kind":"decision_available","actor":"1"}"#,
            concat!(
                r#"{"kind":"random_outcome_visible","label":"die","#,
                r#""exclusive_upper_bound":6,"value":2}"#,
            ),
            r#"{"kind":"public_outcome","code":"draw"}"#,
        ];
        for event in events {
            serde_json::from_str::<ObservedEventKind>(event).unwrap();
        }
    }

    #[test]
    fn observed_event_text_fields_are_closed_like_python_and_schema() {
        let empty_label = ObservedEventEnvelope {
            schema_version: OBSERVED_EVENT_SCHEMA.into(),
            sequence: EventSequence(0),
            state_revision: StateRevision(0),
            event: ObservedEventKind::RandomOutcomeVisible {
                label: String::new(),
                exclusive_upper_bound: 2,
                value: 0,
            },
        };
        assert_eq!(
            empty_label.validate(),
            Err(ObservationValidationError::EmptyEventText)
        );
        let empty_code = ObservedEventEnvelope {
            schema_version: OBSERVED_EVENT_SCHEMA.into(),
            sequence: EventSequence(0),
            state_revision: StateRevision(0),
            event: ObservedEventKind::PublicOutcome {
                code: String::new(),
            },
        };
        assert_eq!(
            empty_code.validate(),
            Err(ObservationValidationError::EmptyEventText)
        );
    }
}
