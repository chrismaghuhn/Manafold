use std::cmp::Ordering;

use mtgml_model::CandidateIdV1;

use crate::error::DecisionValidationError;
use crate::v4::{
    AuthoritativeCandidate, CandidateIntent, EngineCandidateBinding, VisibleCandidate,
};

const CANDIDATE_ID_COUNT_CAPACITY: u64 = (u32::MAX as u64) + 1;

pub(crate) fn validate_candidate_capacity(
    candidate_count: usize,
) -> Result<(), DecisionValidationError> {
    let count = u64::try_from(candidate_count)
        .map_err(|_| DecisionValidationError::CandidateCapacityExceeded)?;
    if count > CANDIDATE_ID_COUNT_CAPACITY {
        return Err(DecisionValidationError::CandidateCapacityExceeded);
    }
    Ok(())
}

pub struct CandidateOrdering;

impl CandidateOrdering {
    /// Orders trusted candidates by their public intent alone and assigns
    /// dense request-local ids. Two candidates with one public key fail
    /// closed, whatever their trusted bindings.
    pub fn assign_dense(
        candidates: Vec<(CandidateIntent, EngineCandidateBinding)>,
    ) -> Result<Vec<AuthoritativeCandidate>, DecisionValidationError> {
        validate_candidate_capacity(candidates.len())?;
        let mut candidates = candidates;
        candidates.sort_by(|left, right| left.0.compare(&right.0));
        if candidates
            .windows(2)
            .any(|pair| pair[0].0.compare(&pair[1].0) == Ordering::Equal)
        {
            return Err(DecisionValidationError::DuplicateOrderingKey);
        }
        candidates
            .into_iter()
            .enumerate()
            .map(|(index, (visible_intent, trusted_binding))| {
                Ok(AuthoritativeCandidate {
                    candidate_id: CandidateIdV1(
                        u32::try_from(index)
                            .map_err(|_| DecisionValidationError::CandidateCapacityExceeded)?,
                    ),
                    visible_intent,
                    trusted_binding,
                })
            })
            .collect()
    }

    pub fn validate_public(candidates: &[VisibleCandidate]) -> Result<(), DecisionValidationError> {
        let count = u64::try_from(candidates.len())
            .map_err(|_| DecisionValidationError::CandidateCapacityExceeded)?;
        if count > u64::from(u32::MAX) + 1 {
            return Err(DecisionValidationError::CandidateCapacityExceeded);
        }
        for (index, candidate) in candidates.iter().enumerate() {
            if candidate.candidate_id.0 != index as u32 {
                return Err(DecisionValidationError::CandidateIdsNotDense);
            }
            candidate.intent.validate()?;
        }
        if candidates
            .windows(2)
            .any(|pair| pair[0].intent.compare(&pair[1].intent) != Ordering::Less)
        {
            return Err(DecisionValidationError::NoncanonicalCandidateOrder);
        }
        Ok(())
    }
}
