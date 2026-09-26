use crate::common::{CandidateIntent, DecisionVisibility};
use crate::error::DecisionValidationError;
use crate::ordering::CandidateOrderingV1;
use mtgml_model::{CandidateIdV1, PlayerDecisionIdV1, PlayerId, StateRevision};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const PLAYER_DECISION_REQUEST_V2_SCHEMA: &str = "player-decision-request.v2";
pub const DECISION_RESPONSE_V2_SCHEMA: &str = "decision-response.v2";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DecisionDomainV2 {
    ChooseOne,
    ChooseMany { minimum: u32, maximum: u32 },
    ChooseNumber { minimum: i64, maximum: i64 },
    Order { minimum: u32, maximum: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DecisionAnswerV2 {
    SelectOne { candidate_id: CandidateIdV1 },
    SelectMany { candidate_ids: Vec<CandidateIdV1> },
    ChooseNumber { value: i64 },
    Order { candidate_ids: Vec<CandidateIdV1> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VisibleCandidateV2 {
    pub candidate_id: CandidateIdV1,
    pub intent: CandidateIntent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerDecisionRequestV2 {
    pub schema_version: String,
    pub player_decision_id: PlayerDecisionIdV1,
    pub state_revision: StateRevision,
    pub actor: PlayerId,
    pub visibility: DecisionVisibility,
    pub decision: DecisionDomainV2,
    pub candidates: Vec<VisibleCandidateV2>,
}

impl DecisionDomainV2 {
    pub(crate) fn validate_candidates(
        &self,
        candidate_count: usize,
    ) -> Result<(), DecisionValidationError> {
        match self {
            Self::ChooseOne if candidate_count == 0 => {
                Err(DecisionValidationError::ImpossibleMinimum)
            }
            Self::ChooseMany { minimum, maximum } | Self::Order { minimum, maximum } => {
                if minimum > maximum {
                    return Err(DecisionValidationError::InvertedBounds);
                }
                // Only an unsatisfiable MINIMUM invalidates the request:
                // the candidate set itself bounds how many distinct
                // candidates are actually reachable, so a larger inclusive
                // maximum still leaves legal answers.
                if usize::try_from(*minimum).unwrap_or(usize::MAX) > candidate_count {
                    return Err(DecisionValidationError::ImpossibleMinimum);
                }
                Ok(())
            }
            Self::ChooseNumber { minimum, maximum } => {
                if minimum > maximum {
                    return Err(DecisionValidationError::InvertedBounds);
                }
                if candidate_count != 0 {
                    return Err(DecisionValidationError::CandidatesNotAllowed);
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

impl PlayerDecisionRequestV2 {
    pub fn validate(&self) -> Result<(), DecisionValidationError> {
        if self.schema_version != PLAYER_DECISION_REQUEST_V2_SCHEMA {
            return Err(DecisionValidationError::SchemaVersion);
        }
        self.decision.validate_candidates(self.candidates.len())?;
        CandidateOrderingV1::validate_public(&self.candidates)
    }

    pub fn answer(&self, answer: &DecisionAnswerV2) -> Result<(), DecisionValidationError> {
        self.validate()?;
        answer.validate_for(&self.decision, &self.candidates)
    }
}

impl DecisionAnswerV2 {
    pub fn validate_for(
        &self,
        domain: &DecisionDomainV2,
        candidates: &[VisibleCandidateV2],
    ) -> Result<(), DecisionValidationError> {
        let ids = candidates
            .iter()
            .map(|candidate| candidate.candidate_id)
            .collect::<Vec<_>>();
        Self::validate_for_candidate_ids(self, domain, &ids)
    }

    pub(crate) fn validate_for_candidate_ids(
        answer: &Self,
        domain: &DecisionDomainV2,
        candidate_ids: &[CandidateIdV1],
    ) -> Result<(), DecisionValidationError> {
        let ids: BTreeSet<_> = candidate_ids.iter().copied().collect();
        match (domain, answer) {
            (DecisionDomainV2::ChooseOne, Self::SelectOne { candidate_id }) => {
                if ids.contains(candidate_id) {
                    Ok(())
                } else {
                    Err(DecisionValidationError::UnknownCandidate)
                }
            }
            (
                DecisionDomainV2::ChooseMany { minimum, maximum },
                Self::SelectMany { candidate_ids },
            ) => {
                validate_selection_ids(candidate_ids, &ids, true)?;
                validate_cardinality(candidate_ids.len(), *minimum, *maximum)
            }
            (DecisionDomainV2::ChooseNumber { minimum, maximum }, Self::ChooseNumber { value })
                if value >= minimum && value <= maximum =>
            {
                Ok(())
            }
            (DecisionDomainV2::Order { minimum, maximum }, Self::Order { candidate_ids }) => {
                validate_selection_ids(candidate_ids, &ids, false)?;
                validate_cardinality(candidate_ids.len(), *minimum, *maximum)
            }
            (DecisionDomainV2::ChooseNumber { .. }, Self::ChooseNumber { .. }) => {
                Err(DecisionValidationError::NumericOutOfBounds)
            }
            _ => Err(DecisionValidationError::AnswerDomainMismatch),
        }
    }
}

/// Normative answer-set precedence (DECISION_PROTOCOL steps 7-8):
/// membership, then uniqueness, then canonical set/order representation.
fn validate_selection_ids(
    candidate_ids: &[CandidateIdV1],
    available: &BTreeSet<CandidateIdV1>,
    require_ascending: bool,
) -> Result<(), DecisionValidationError> {
    for candidate_id in candidate_ids {
        if !available.contains(candidate_id) {
            return Err(DecisionValidationError::UnknownCandidate);
        }
    }
    let mut seen = BTreeSet::new();
    for candidate_id in candidate_ids {
        if !seen.insert(*candidate_id) {
            return Err(DecisionValidationError::DuplicateAnswerCandidate);
        }
    }
    if require_ascending {
        for window in candidate_ids.windows(2) {
            if window[0] >= window[1] {
                return Err(DecisionValidationError::NoncanonicalAnswer);
            }
        }
    }
    Ok(())
}

fn validate_cardinality(
    actual: usize,
    minimum: u32,
    maximum: u32,
) -> Result<(), DecisionValidationError> {
    let actual = u32::try_from(actual).map_err(|_| DecisionValidationError::ValueOutOfRange)?;
    if actual < minimum || actual > maximum {
        return Err(DecisionValidationError::AnswerCardinality);
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionResponseV2 {
    pub schema_version: String,
    pub player_decision_id: PlayerDecisionIdV1,
    pub state_revision: StateRevision,
    pub answer: DecisionAnswerV2,
}

impl DecisionResponseV2 {
    pub fn validate(&self) -> Result<(), DecisionValidationError> {
        if self.schema_version != DECISION_RESPONSE_V2_SCHEMA {
            return Err(DecisionValidationError::SchemaVersion);
        }
        match &self.answer {
            // Standalone response-local semantics: uniqueness precedes the
            // canonical set representation (request-relative membership is
            // checked against a visible request by `validate_for`).
            DecisionAnswerV2::SelectMany { candidate_ids } => {
                let mut seen = BTreeSet::new();
                if candidate_ids
                    .iter()
                    .any(|candidate_id| !seen.insert(*candidate_id))
                {
                    return Err(DecisionValidationError::DuplicateAnswerCandidate);
                }
                if candidate_ids
                    .windows(2)
                    .any(|window| window[0] >= window[1])
                {
                    return Err(DecisionValidationError::NoncanonicalAnswer);
                }
            }
            DecisionAnswerV2::Order { candidate_ids } => {
                let mut seen = BTreeSet::new();
                if candidate_ids
                    .iter()
                    .any(|candidate_id| !seen.insert(*candidate_id))
                {
                    return Err(DecisionValidationError::DuplicateAnswerCandidate);
                }
            }
            DecisionAnswerV2::SelectOne { .. } | DecisionAnswerV2::ChooseNumber { .. } => {}
        }
        Ok(())
    }

    /// Request-relative submission checks in normative order. Deliberately
    /// does NOT run the standalone monolithic [`Self::validate`] first:
    /// endpoint availability, visible-request identity, and revision must be
    /// resolved before uniqueness/canonicality so compound failures classify
    /// deterministically (stale beats duplicate/canonical defects).
    /// Wire/schema shape identity is enforced separately at decode time.
    pub fn validate_for(
        &self,
        request: &PlayerDecisionRequestV2,
    ) -> Result<(), DecisionValidationError> {
        if self.schema_version != DECISION_RESPONSE_V2_SCHEMA {
            return Err(DecisionValidationError::SchemaVersion);
        }
        if self.player_decision_id != request.player_decision_id {
            return Err(DecisionValidationError::DecisionIdentityMismatch);
        }
        if self.state_revision != request.state_revision {
            return Err(DecisionValidationError::StateRevisionMismatch);
        }
        // Answer variant first, then membership/uniqueness/canonical/bounds.
        if !matches!(
            (&request.decision, &self.answer),
            (
                DecisionDomainV2::ChooseOne,
                DecisionAnswerV2::SelectOne { .. }
            ) | (
                DecisionDomainV2::ChooseMany { .. },
                DecisionAnswerV2::SelectMany { .. }
            ) | (
                DecisionDomainV2::ChooseNumber { .. },
                DecisionAnswerV2::ChooseNumber { .. }
            ) | (
                DecisionDomainV2::Order { .. },
                DecisionAnswerV2::Order { .. }
            )
        ) {
            return Err(DecisionValidationError::AnswerDomainMismatch);
        }
        self.answer
            .validate_for(&request.decision, &request.candidates)
    }
}
