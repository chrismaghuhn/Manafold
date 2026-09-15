use crate::authoritative::{AuthoritativeCandidateV2, EngineCandidateBinding};
use crate::common::CandidateIntent;
use crate::error::DecisionValidationError;
use crate::v2::VisibleCandidateV2;
use mtgml_model::CandidateIdV1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum CandidatePayloadKey {
    None,
    U64(u64),
    U32(u32),
    Bool(bool),
    I64(i64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct CandidateOrderingKey {
    rank: u8,
    payload: CandidatePayloadKey,
}

pub struct CandidateOrderingV1;

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

impl CandidateOrderingV1 {
    pub fn assign_dense(
        candidates: Vec<(CandidateIntent, EngineCandidateBinding)>,
    ) -> Result<Vec<AuthoritativeCandidateV2>, DecisionValidationError> {
        validate_candidate_capacity(candidates.len())?;
        let mut keyed = candidates
            .into_iter()
            .map(|(visible_intent, trusted_binding)| {
                let key = ordering_key(&visible_intent)?;
                Ok((key, visible_intent, trusted_binding))
            })
            .collect::<Result<Vec<_>, DecisionValidationError>>()?;
        keyed.sort_by_key(|(key, _, _)| *key);
        if keyed.windows(2).any(|window| window[0].0 == window[1].0) {
            return Err(DecisionValidationError::DuplicateOrderingKey);
        }
        let assigned = keyed
            .into_iter()
            .enumerate()
            .map(|(index, (_, visible_intent, trusted_binding))| {
                let candidate_id = u32::try_from(index)
                    .map_err(|_| DecisionValidationError::CandidateCapacityExceeded)?;
                Ok(AuthoritativeCandidateV2 {
                    candidate_id: CandidateIdV1(candidate_id),
                    visible_intent,
                    trusted_binding,
                })
            })
            .collect::<Result<Vec<_>, DecisionValidationError>>()?;
        Ok(assigned)
    }

    pub fn validate_public(
        candidates: &[VisibleCandidateV2],
    ) -> Result<(), DecisionValidationError> {
        validate_candidate_capacity(candidates.len())?;
        for (index, candidate) in candidates.iter().enumerate() {
            let expected = u32::try_from(index)
                .map_err(|_| DecisionValidationError::CandidateCapacityExceeded)?;
            if candidate.candidate_id.0 != expected {
                return Err(DecisionValidationError::CandidateIdsNotDense);
            }
            if let Some(previous) = candidates.get(index.wrapping_sub(1)) {
                if ordering_key(&previous.intent)? >= ordering_key(&candidate.intent)? {
                    return Err(DecisionValidationError::NoncanonicalCandidateOrder);
                }
            }
        }
        Ok(())
    }
}

fn ordering_key(intent: &CandidateIntent) -> Result<CandidateOrderingKey, DecisionValidationError> {
    let (rank, payload) = match intent {
        CandidateIntent::PassPriority => (0, CandidatePayloadKey::None),
        CandidateIntent::CastSpell { object } => (1, CandidatePayloadKey::U64(object.0)),
        CandidateIntent::ActivateAbility { ability } => (2, CandidatePayloadKey::U64(ability.0)),
        CandidateIntent::SelectObject { object } => (3, CandidatePayloadKey::U64(object.0)),
        CandidateIntent::SelectPlayer { player } => (4, CandidatePayloadKey::U64(player.0)),
        CandidateIntent::SelectMode { mode_index } => (5, CandidatePayloadKey::U32(*mode_index)),
        CandidateIntent::ChooseBoolean { value } => (6, CandidatePayloadKey::Bool(*value)),
        CandidateIntent::DeclareNumber { value } => (7, CandidatePayloadKey::I64(*value)),
        CandidateIntent::Confirm => (8, CandidatePayloadKey::None),
    };
    Ok(CandidateOrderingKey { rank, payload })
}
