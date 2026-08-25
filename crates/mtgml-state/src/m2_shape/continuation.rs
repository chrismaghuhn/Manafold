//! Ownership: current-M2 synthetic assembly continuation program material
//! (records, stages, payload, and its shape/coherence validation).

use std::collections::BTreeMap;

use mtgml_decision::{AuthoritativeDecisionRequestV2, CandidateIntent};
use mtgml_model::{ContinuationId, PlayerId, StateRevision};
use serde::{Deserialize, Serialize};

use crate::m2_shape::{M2ShapeViolation, SYNTHETIC_COUNT_MAX, SYNTHETIC_COUNT_MIN};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PendingDecisionRecordV2 {
    pub request: AuthoritativeDecisionRequestV2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssemblyStageV2 {
    ChooseCount,
    ChooseMembers,
    OrderMembers,
}

impl AssemblyStageV2 {
    pub fn stage_index(self) -> u16 {
        match self {
            Self::ChooseCount => 0,
            Self::ChooseMembers => 1,
            Self::OrderMembers => 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ContinuationPayloadV2 {
    SyntheticM2Assembly {
        stage: AssemblyStageV2,
        selected_count: Option<u32>,
        selected_piece_keys: Vec<u32>,
        ordered_piece_keys: Vec<u32>,
    },
}

impl ContinuationPayloadV2 {
    pub fn stage_index(&self) -> u16 {
        match self {
            Self::SyntheticM2Assembly { stage, .. } => stage.stage_index(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContinuationRecordV2 {
    pub id: ContinuationId,
    pub actor: PlayerId,
    pub created_at_revision: StateRevision,
    pub stage_index: u16,
    pub payload: ContinuationPayloadV2,
}

/// Stage-payload invariants of the one frozen synthetic assembly payload.
///
/// Every reachable continuation state must have exactly one unambiguous
/// semantic interpretation:
///
/// - `ChooseCount`: nothing decided yet;
/// - `ChooseMembers`: the numeric count is decided, the member set is not;
/// - `OrderMembers`: the member set is decided in canonical set form, the
///   semantic order is not (it lives only in the pending stage answer).
///
/// Ordered partial data never persists: completion removes the continuation.
pub(super) fn validate_synthetic_assembly(
    stage: AssemblyStageV2,
    selected_count: Option<u32>,
    selected_piece_keys: &[u32],
    ordered_piece_keys: &[u32],
) -> Result<(), M2ShapeViolation> {
    let canonical_set = |values: &[u32]| values.windows(2).all(|window| window[0] < window[1]);
    match stage {
        AssemblyStageV2::ChooseCount => {
            if selected_count.is_some()
                || !selected_piece_keys.is_empty()
                || !ordered_piece_keys.is_empty()
            {
                return Err(M2ShapeViolation::Knowledge);
            }
        }
        AssemblyStageV2::ChooseMembers => {
            if selected_count.is_none()
                || !selected_piece_keys.is_empty()
                || !ordered_piece_keys.is_empty()
            {
                return Err(M2ShapeViolation::Knowledge);
            }
        }
        AssemblyStageV2::OrderMembers => {
            let Some(count) = selected_count else {
                return Err(M2ShapeViolation::Knowledge);
            };
            if !ordered_piece_keys.is_empty()
                || selected_piece_keys.len() != count as usize
                || !canonical_set(selected_piece_keys)
            {
                return Err(M2ShapeViolation::Knowledge);
            }
        }
    }
    // A decided count can only originate from the supported ChooseCount
    // interval; anything else was never offered by this program.
    if let Some(count) = selected_count {
        if count > SYNTHETIC_COUNT_MAX {
            return Err(M2ShapeViolation::Knowledge);
        }
    }
    Ok(())
}

/// The one linear M2 program binds an active continuation and its pending
/// request into a single authoritative semantic unit: the pending request
/// must express exactly the referenced stage's program, and an active
/// continuation must always be resumable.
pub(super) fn validate_program_coherence(
    pending: Option<&PendingDecisionRecordV2>,
    continuations: &BTreeMap<ContinuationId, ContinuationRecordV2>,
) -> Result<(), M2ShapeViolation> {
    if continuations.len() > 1 {
        return Err(M2ShapeViolation::ContinuationReference);
    }
    let Some(record) = continuations.values().next() else {
        return Ok(());
    };
    let Some(pending) = pending else {
        // An active continuation without its next stage request is not
        // resumable and can never become checkpointable state.
        return Err(M2ShapeViolation::ContinuationReference);
    };
    if pending.request.continuation_id != Some(record.id) {
        return Err(M2ShapeViolation::ContinuationReference);
    }
    let ContinuationPayloadV2::SyntheticM2Assembly {
        stage,
        selected_count,
        selected_piece_keys,
        ..
    } = &record.payload;
    let candidates_express = |expected_pieces: &[u32]| -> bool {
        pending.request.candidates.len() == expected_pieces.len()
            && pending
                .request
                .candidates
                .iter()
                .enumerate()
                .all(|(index, candidate)| {
                    candidate.candidate_id.0 == index as u32
                        && matches!(
                            &candidate.visible_intent,
                            CandidateIntent::SelectMode { mode_index }
                                if *mode_index == expected_pieces[index]
                        )
                })
    };
    match (stage, &pending.request.decision) {
        (
            AssemblyStageV2::ChooseCount,
            mtgml_decision::DecisionDomainV2::ChooseNumber { minimum, maximum },
        ) => {
            // The engine may offer exactly the supported program interval.
            if *minimum != i64::from(SYNTHETIC_COUNT_MIN)
                || *maximum != i64::from(SYNTHETIC_COUNT_MAX)
                || !pending.request.candidates.is_empty()
            {
                return Err(M2ShapeViolation::PendingDecision);
            }
        }
        (
            AssemblyStageV2::ChooseMembers,
            mtgml_decision::DecisionDomainV2::ChooseMany { minimum, maximum },
        ) => {
            let count = selected_count.ok_or(M2ShapeViolation::Knowledge)?;
            if count > SYNTHETIC_COUNT_MAX || *minimum != count || *maximum != count {
                return Err(M2ShapeViolation::PendingDecision);
            }
            // Stage members are the fixed synthetic piece surface 0..count.
            let expected: Vec<u32> = (0..count).collect();
            if !candidates_express(&expected) {
                return Err(M2ShapeViolation::PendingDecision);
            }
        }
        (
            AssemblyStageV2::OrderMembers,
            mtgml_decision::DecisionDomainV2::Order { minimum, maximum },
        ) => {
            let count = selected_count.ok_or(M2ShapeViolation::Knowledge)?;
            if count > SYNTHETIC_COUNT_MAX || *minimum != count || *maximum != count {
                return Err(M2ShapeViolation::PendingDecision);
            }
            // ChooseMembers offers exactly pieces 0..count and requires
            // exactly count selections: the only reachable member set is the
            // full prefix. Anything else is an unreachable history.
            if *selected_piece_keys != (0..count).collect::<Vec<u32>>() {
                return Err(M2ShapeViolation::Knowledge);
            }
            if !candidates_express(selected_piece_keys) {
                return Err(M2ShapeViolation::PendingDecision);
            }
        }
        _ => return Err(M2ShapeViolation::PendingDecision),
    }
    Ok(())
}
