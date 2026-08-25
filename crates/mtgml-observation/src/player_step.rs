//! Ownership: player-step DTOs (V1 and V2), submission/service codes, and
//! their existing validation / `code()` logic.

use mtgml_decision::{PlayerDecisionRequest, PlayerDecisionRequestV2};
use mtgml_model::EpisodeStatus;
use serde::{Deserialize, Serialize};

use crate::error::ObservationValidationError;
use crate::information::{InformationStateEnvelope, PlayerInformationStateV2};
use crate::observation::ObservationEnvelope;
use crate::observed_event::{ObservedEventEnvelope, ObservedEventEnvelopeV2};
use crate::{PLAYER_STEP_SCHEMA, PLAYER_STEP_SCHEMA_V2};

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

/// Versioned closed codes for typed player submission rejections
/// (ERROR_MODEL, layer B). Wire representation: snake_case strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayerSubmissionCodeV1 {
    StaleDecision,
    UnavailableDecision,
    InvalidAnswer,
    InvalidCandidate,
    DuplicateAssignment,
    InvalidCardinality,
    InvalidNumber,
    InvalidOrder,
    EpisodeClosed,
}

/// Versioned closed service-failure code (ERROR_MODEL, layer C).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayerServiceErrorCodeV1 {
    ServiceUnavailable,
}

/// Versioned submission outcome carried by `PlayerStepV2`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PlayerStepSubmissionV1 {
    Accepted,
    Rejected { code: PlayerSubmissionCodeV1 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerStepV2 {
    pub schema_version: String,
    pub information_state: PlayerInformationStateV2,
    pub observed_events: Vec<ObservedEventEnvelopeV2>,
    pub next_decision: Option<PlayerDecisionRequestV2>,
    pub status: EpisodeStatus,
    pub submission: PlayerStepSubmissionV1,
}

impl PlayerStepV2 {
    pub fn validate(&self) -> Result<(), ObservationValidationError> {
        if self.schema_version != PLAYER_STEP_SCHEMA_V2 {
            return Err(ObservationValidationError::SchemaOrCodec);
        }
        self.information_state.validate()?;
        self.status
            .validate()
            .map_err(|_| ObservationValidationError::EpisodeStatus)?;
        for (index, event) in self.observed_events.iter().enumerate() {
            event.validate()?;
            // One accepted transition owns exactly one revision: every
            // observed envelope of a step belongs to that step's revision.
            if event.state_revision != self.information_state.state_revision {
                return Err(ObservationValidationError::FutureEvent);
            }
            // Perspective-local visible sequences are strictly increasing and
            // never reach the step's own next-unused cursor.
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
                || decision.state_revision != self.information_state.state_revision
            {
                return Err(ObservationValidationError::Decision);
            }
        }
        if !matches!(self.status, EpisodeStatus::Running) && self.next_decision.is_some() {
            return Err(ObservationValidationError::Decision);
        }
        // ML_ENVIRONMENT.md: a typed semantic rejection mirrors the unchanged
        // product with an empty event batch; only the outcome code differs.
        if let PlayerStepSubmissionV1::Rejected { code } = &self.submission {
            if !self.observed_events.is_empty() {
                return Err(ObservationValidationError::Submission);
            }
            if *code == PlayerSubmissionCodeV1::EpisodeClosed {
                // An episode_closed rejection requires a non-Running status.
                if matches!(self.status, EpisodeStatus::Running) {
                    return Err(ObservationValidationError::Submission);
                }
            } else if !matches!(self.status, EpisodeStatus::Running) {
                // Every other typed rejection mirrors a live Running
                // episode; a closed episode surfaces as episode_closed.
                return Err(ObservationValidationError::Submission);
            }
        }
        Ok(())
    }
}

impl PlayerServiceErrorCodeV1 {
    /// Single versioned authority for the public service-failure string.
    pub fn code(self) -> &'static str {
        match self {
            Self::ServiceUnavailable => "service_unavailable",
        }
    }
}
