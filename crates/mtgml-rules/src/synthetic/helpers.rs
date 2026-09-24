//! Ownership: mechanical helpers of the current synthetic program. Fresh
//! stage identities and rejection products are re-exported from the
//! program-neutral decision-stage module.

use mtgml_decision::{CandidateIntent, EngineCandidateBinding};
use mtgml_model::{CandidateIdV1, DecisionId, RuleEventId, StateRevision};
use mtgml_random::{RandomStreamKeyV1, RandomStreamKindV1};
use mtgml_state::EngineState;

use crate::errors::KernelExecutionError;
use crate::events::{AuthoritativeRuleEvent, AuthoritativeRuleEventKind};

pub(super) use crate::decision_stage::{
    advance_player_allocator, fresh_stage_identity, next_revision, rejected,
};

pub(super) fn global_stream() -> RandomStreamKeyV1 {
    RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1)
}

pub(super) fn answered_piece(
    request: &mtgml_decision::AuthoritativeDecisionRequestV2,
    candidate_id: CandidateIdV1,
) -> Result<u32, KernelExecutionError> {
    let candidate = request.candidates.get(candidate_id.0 as usize);
    let piece = candidate.and_then(|candidate| {
        if candidate.candidate_id != candidate_id {
            return None;
        }
        match &candidate.visible_intent {
            CandidateIntent::SelectMode { mode_index } => Some(*mode_index),
            _ => None,
        }
    });
    // A dense authoritative request always contains every answered ID; a
    // miss means the engine-offered path is inconsistent (internal).
    piece.ok_or(KernelExecutionError::UnsupportedStagePath)
}

pub(super) fn piece_candidates_from(
    pieces: &[u32],
) -> Vec<mtgml_decision::AuthoritativeCandidateV2> {
    let pairs = pieces
        .iter()
        .map(|piece| {
            (
                CandidateIntent::SelectMode { mode_index: *piece },
                EngineCandidateBinding::SelectMode { mode_index: *piece },
            )
        })
        .collect();
    mtgml_decision::CandidateOrderingV1::assign_dense(pairs)
        .expect("selected pieces are distinct public ordering keys")
}

pub(super) fn piece_candidates(count: u32) -> Vec<mtgml_decision::AuthoritativeCandidateV2> {
    let pairs = (0..count)
        .map(|piece| {
            (
                CandidateIntent::SelectMode { mode_index: piece },
                EngineCandidateBinding::SelectMode { mode_index: piece },
            )
        })
        .collect();
    mtgml_decision::CandidateOrderingV1::assign_dense(pairs)
        .expect("generated pieces are distinct public ordering keys")
}

pub(super) fn bound_event(
    state: &EngineState,
    offset: u64,
    revision: StateRevision,
    kind: AuthoritativeRuleEventKind,
) -> Result<AuthoritativeRuleEvent, KernelExecutionError> {
    Ok(AuthoritativeRuleEvent {
        event_id: RuleEventId(
            state
                .allocators
                .next_rule_event_id
                .0
                .checked_add(offset)
                .ok_or(KernelExecutionError::RuleEventIdOverflow)?,
        ),
        state_revision: revision,
        event: kind,
    })
}

pub(super) fn cleared_event(
    state: &EngineState,
    revision: StateRevision,
    decision: DecisionId,
) -> Result<AuthoritativeRuleEvent, KernelExecutionError> {
    bound_event(
        state,
        0,
        revision,
        AuthoritativeRuleEventKind::DecisionCleared { decision },
    )
}

pub(super) fn created_event(
    state: &EngineState,
    revision: StateRevision,
    decision: DecisionId,
) -> Result<AuthoritativeRuleEvent, KernelExecutionError> {
    bound_event(
        state,
        1,
        revision,
        AuthoritativeRuleEventKind::DecisionCreated { decision },
    )
}
