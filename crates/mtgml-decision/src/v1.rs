use crate::common::{CandidateIntent, DecisionVisibility};
use crate::error::DecisionValidationError;
use mtgml_model::{DecisionId, PlayerId, StateRevision};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const PLAYER_DECISION_REQUEST_SCHEMA: &str = "player-decision-request.v1";
pub const DECISION_RESPONSE_SCHEMA: &str = "decision-response.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DecisionKind {
    ChooseOne,
    ChooseMany { minimum: u32, maximum: u32 },
    ChooseNumber { minimum: i64, maximum: i64 },
    Order { minimum: u32, maximum: u32 },
}

impl DecisionKind {
    pub fn selection_bounds(&self) -> Option<(u32, u32)> {
        match self {
            Self::ChooseOne => Some((1, 1)),
            Self::ChooseMany { minimum, maximum } | Self::Order { minimum, maximum } => {
                Some((*minimum, *maximum))
            }
            Self::ChooseNumber { .. } => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionCandidate {
    pub candidate_id: String,
    pub semantic_key: String,
    pub intent: CandidateIntent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerDecisionRequest {
    pub schema_version: String,
    pub decision_id: DecisionId,
    pub state_revision: StateRevision,
    pub actor: PlayerId,
    pub visibility: DecisionVisibility,
    pub decision: DecisionKind,
    pub candidates: Vec<ActionCandidate>,
}

impl PlayerDecisionRequest {
    pub fn validate(&self) -> Result<(), DecisionValidationError> {
        if self.schema_version != PLAYER_DECISION_REQUEST_SCHEMA {
            return Err(DecisionValidationError::SchemaVersion);
        }
        let mut candidate_ids = BTreeSet::new();
        let mut semantic_keys = BTreeSet::new();
        for candidate in &self.candidates {
            if candidate.candidate_id.is_empty() || candidate.semantic_key.is_empty() {
                return Err(DecisionValidationError::EmptyCandidateIdentity);
            }
            if !candidate_ids.insert(candidate.candidate_id.as_str()) {
                return Err(DecisionValidationError::DuplicateCandidateId);
            }
            if !semantic_keys.insert(candidate.semantic_key.as_str()) {
                return Err(DecisionValidationError::DuplicateSemanticKey);
            }
        }
        match self.decision {
            DecisionKind::ChooseMany { minimum, maximum }
            | DecisionKind::Order { minimum, maximum } => {
                if minimum > maximum {
                    return Err(DecisionValidationError::InvertedBounds);
                }
                if usize::try_from(minimum).unwrap_or(usize::MAX) > self.candidates.len() {
                    return Err(DecisionValidationError::ImpossibleMinimum);
                }
            }
            DecisionKind::ChooseNumber { minimum, maximum } if minimum > maximum => {
                return Err(DecisionValidationError::InvertedBounds)
            }
            DecisionKind::ChooseOne if self.candidates.is_empty() => {
                return Err(DecisionValidationError::ImpossibleMinimum)
            }
            _ => {}
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateAssignment {
    pub candidate_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ordinal: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionResponse {
    pub schema_version: String,
    pub decision_id: DecisionId,
    pub state_revision: StateRevision,
    pub assignments: Vec<CandidateAssignment>,
}

impl DecisionResponse {
    pub fn validate(&self) -> Result<(), DecisionValidationError> {
        if self.schema_version != DECISION_RESPONSE_SCHEMA {
            return Err(DecisionValidationError::SchemaVersion);
        }
        let mut ids = BTreeSet::new();
        if self
            .assignments
            .iter()
            .any(|assignment| assignment.candidate_id.is_empty())
        {
            return Err(DecisionValidationError::EmptyCandidateIdentity);
        }
        if self
            .assignments
            .iter()
            .any(|assignment| !ids.insert(assignment.candidate_id.as_str()))
        {
            return Err(DecisionValidationError::DuplicateAssignment);
        }
        Ok(())
    }
}
