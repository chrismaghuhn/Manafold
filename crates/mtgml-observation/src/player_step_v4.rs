//! Detached PlayerStep V4 composition without global state revision.

use mtgml_decision::PlayerDecisionRequestV4;
use mtgml_model::EpisodeStatus;
use serde::{Deserialize, Serialize};

use crate::{
    error::ObservationValidationError,
    observed_event_v4::ObservedEventEnvelopeV4,
    player_step::{PlayerStepSubmissionV1, PlayerSubmissionCodeV1},
    PlayerInformationState, PLAYER_STEP_SCHEMA_V4,
};

fn deserialize_required_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::deserialize(deserializer)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerStepV4 {
    pub schema_version: String,
    pub information_state: PlayerInformationState,
    pub observed_events: Vec<ObservedEventEnvelopeV4>,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub next_decision: Option<PlayerDecisionRequestV4>,
    pub status: EpisodeStatus,
    pub submission: PlayerStepSubmissionV1,
}

impl PlayerStepV4 {
    pub fn validate(&self) -> Result<(), ObservationValidationError> {
        if self.schema_version != PLAYER_STEP_SCHEMA_V4 {
            return Err(ObservationValidationError::SchemaOrCodec);
        }
        self.information_state.validate()?;
        self.status
            .validate()
            .map_err(|_| ObservationValidationError::EpisodeStatus)?;

        for (index, event) in self.observed_events.iter().enumerate() {
            event.validate()?;
            if event.sequence.0 >= self.information_state.next_visible_sequence.0 {
                return Err(ObservationValidationError::FutureEvent);
            }
            if index > 0 && event.sequence <= self.observed_events[index - 1].sequence {
                return Err(ObservationValidationError::VisibleSequence);
            }
        }
        if let Some(decision) = &self.next_decision {
            decision
                .validate()
                .map_err(|_| ObservationValidationError::Decision)?;
            if decision.actor != self.information_state.perspective
                || decision.view_sequence != self.information_state.next_visible_sequence
            {
                return Err(ObservationValidationError::Decision);
            }
        }
        if !matches!(self.status, EpisodeStatus::Running) && self.next_decision.is_some() {
            return Err(ObservationValidationError::Decision);
        }
        if let PlayerStepSubmissionV1::Rejected { code } = &self.submission {
            if !self.observed_events.is_empty() {
                return Err(ObservationValidationError::Submission);
            }
            match code {
                PlayerSubmissionCodeV1::EpisodeClosed => {
                    if matches!(self.status, EpisodeStatus::Running) || self.next_decision.is_some()
                    {
                        return Err(ObservationValidationError::Submission);
                    }
                }
                PlayerSubmissionCodeV1::UnavailableDecision => {
                    if !matches!(self.status, EpisodeStatus::Running)
                        || self.next_decision.is_some()
                    {
                        return Err(ObservationValidationError::Submission);
                    }
                }
                PlayerSubmissionCodeV1::StaleDecision
                | PlayerSubmissionCodeV1::InvalidAnswer
                | PlayerSubmissionCodeV1::InvalidCandidate
                | PlayerSubmissionCodeV1::DuplicateAssignment
                | PlayerSubmissionCodeV1::InvalidCardinality
                | PlayerSubmissionCodeV1::InvalidNumber
                | PlayerSubmissionCodeV1::InvalidOrder => {
                    if !matches!(self.status, EpisodeStatus::Running)
                        || self.next_decision.is_none()
                    {
                        return Err(ObservationValidationError::Submission);
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const FIXTURE: &str = include_str!("../../../schemas/examples/player-step-v4.json");

    #[test]
    fn v4_step_fixture_has_revision_free_public_products_and_safe_event_cursor() {
        let step: PlayerStepV4 = serde_json::from_str(FIXTURE).unwrap();
        step.validate().unwrap();
        let value = serde_json::to_value(step).unwrap();
        assert!(value.get("state_revision").is_none());
        assert!(value["information_state"].get("state_revision").is_none());
        assert!(value["information_state"]["current_observation"]
            .get("state_revision")
            .is_none());
        assert!(value["observed_events"][0].get("state_revision").is_none());
    }

    #[test]
    fn v4_step_rejects_global_revision_and_event_after_cursor() {
        let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
        value["state_revision"] = Value::from("9");
        assert!(serde_json::from_value::<PlayerStepV4>(value).is_err());

        let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
        value["observed_events"][0]["sequence"] = Value::from("5");
        let step: PlayerStepV4 = serde_json::from_value(value).unwrap();
        assert_eq!(
            step.validate(),
            Err(ObservationValidationError::FutureEvent)
        );
    }
}
