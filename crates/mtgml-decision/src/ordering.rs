use crate::error::DecisionValidationError;

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
