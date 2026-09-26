//! Detached PlayerStep V3 composition and predecessor-equivalent validation.

use mtgml_decision::PlayerDecisionRequestV3;
use mtgml_model::EpisodeStatus;
use serde::{Deserialize, Serialize};

use crate::error::ObservationValidationError;
use crate::information::PlayerInformationStateV2;
use crate::observed_event_v3::ObservedEventEnvelopeV3;
use crate::player_step::{PlayerStepSubmissionV1, PlayerSubmissionCodeV1};
use crate::PLAYER_STEP_SCHEMA_V3;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerStepV3 {
    pub schema_version: String,
    pub information_state: PlayerInformationStateV2,
    pub observed_events: Vec<ObservedEventEnvelopeV3>,
    pub next_decision: Option<PlayerDecisionRequestV3>,
    pub status: EpisodeStatus,
    pub submission: PlayerStepSubmissionV1,
}

impl PlayerStepV3 {
    pub fn validate(&self) -> Result<(), ObservationValidationError> {
        if self.schema_version != PLAYER_STEP_SCHEMA_V3 {
            return Err(ObservationValidationError::SchemaOrCodec);
        }
        self.information_state.validate()?;
        self.status
            .validate()
            .map_err(|_| ObservationValidationError::EpisodeStatus)?;

        let revision = self.information_state.state_revision;
        let mut previous_event_revision = None;
        for (index, event) in self.observed_events.iter().enumerate() {
            event.validate()?;
            if event.state_revision > revision
                || previous_event_revision.is_some_and(|previous| event.state_revision < previous)
            {
                return Err(ObservationValidationError::FutureEvent);
            }
            if event.sequence.0 >= self.information_state.next_visible_sequence.0
                || index > 0 && event.sequence <= self.observed_events[index - 1].sequence
            {
                return Err(ObservationValidationError::VisibleSequence);
            }
            previous_event_revision = Some(event.state_revision);
        }

        if let Some(decision) = &self.next_decision {
            decision
                .validate()
                .map_err(|_| ObservationValidationError::Decision)?;
            if decision.actor != self.information_state.perspective
                || decision.state_revision != revision
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
