//! Ownership: implementations of the ChooseMembers and Order program
//! stages. Not generalized; no stage trait, no generic continuation
//! dispatch.

use mtgml_decision::DecisionDomainV2;
use mtgml_model::{CandidateIdV1, DecisionId};
use mtgml_state::{AssemblyStageV2, ContinuationPayloadV2, EngineState, PendingDecisionRecordV2};

use super::helpers::{
    advance_player_allocator, answered_piece, cleared_event, created_event, fresh_stage_identity,
    next_revision, piece_candidates_from, rejected,
};
use super::SyntheticM1RulesKernel;
use crate::errors::KernelExecutionError;
use crate::product::build_accepted_product;
use crate::transition::TransitionResult;

impl SyntheticM1RulesKernel {
    /// Stage 1: ChooseMembers fixes the unordered member set.
    pub(super) fn apply_members_stage(
        state: &EngineState,
        candidate_ids: &[CandidateIdV1],
    ) -> Result<TransitionResult, KernelExecutionError> {
        let pending = state.execution.pending_decision.as_ref().expect("checked");
        let request = &pending.request;
        let actor = request.actor;
        let continuation_id = match request.continuation_id {
            Some(id) => id,
            None => return rejected(state),
        };
        let selected_count = match state
            .execution
            .continuations
            .get(&continuation_id)
            .map(|record| &record.payload)
        {
            Some(ContinuationPayloadV2::SyntheticM2Assembly {
                stage: AssemblyStageV2::ChooseMembers,
                selected_count: Some(count),
                selected_piece_keys,
                ordered_piece_keys,
            }) if selected_piece_keys.is_empty() && ordered_piece_keys.is_empty() => *count,
            _ => return rejected(state),
        };
        // The pending request bounds equal the authoritative partial count
        // (state-level program coherence), and the answer was validated
        // against them before dispatch - so the answer length must equal the
        // persisted authoritative count.
        if candidate_ids.len() != selected_count as usize {
            return rejected(state);
        }
        let mut selected_piece_keys = Vec::with_capacity(candidate_ids.len());
        for candidate_id in candidate_ids {
            selected_piece_keys.push(answered_piece(request, *candidate_id)?);
        }
        selected_piece_keys.sort_unstable();

        let identity = fresh_stage_identity(state, actor)?;
        let mut next = state.clone();
        next.revision = identity.revision;
        let events = vec![
            cleared_event(state, identity.revision, request.decision_id)?,
            created_event(state, identity.revision, identity.decision_id)?,
        ];
        let candidates = piece_candidates_from(&selected_piece_keys);

        build_accepted_product(state, next, events, move |workspace| {
            let record = workspace
                .execution
                .continuations
                .get_mut(&continuation_id)
                .expect("validated above");
            record.stage_index = AssemblyStageV2::OrderMembers.stage_index();
            record.payload = ContinuationPayloadV2::SyntheticM2Assembly {
                stage: AssemblyStageV2::OrderMembers,
                // Persist the authoritative partial value unchanged.
                selected_count: Some(selected_count),
                selected_piece_keys: selected_piece_keys.clone(),
                ordered_piece_keys: Vec::new(),
            };
            workspace.execution.pending_decision = Some(PendingDecisionRecordV2 {
                request: mtgml_decision::AuthoritativeDecisionRequestV2 {
                    decision_id: identity.decision_id,
                    player_decision_id: identity.player_decision_id,
                    state_revision: workspace.revision,
                    actor,
                    visibility: request.visibility,
                    decision: DecisionDomainV2::Order {
                        minimum: selected_count,
                        maximum: selected_count,
                    },
                    candidates,
                    continuation_id: Some(continuation_id),
                },
            });
            workspace.allocators.next_decision_id = DecisionId(identity.decision_id.0 + 1);
            advance_player_allocator(workspace, actor, identity.player_decision_id)?;
            Ok(())
        })
    }
    /// Stage 2: Order supplies the semantic sequence and completes the
    /// continuation.
    pub(super) fn apply_order_stage(
        state: &EngineState,
        candidate_ids: &[CandidateIdV1],
    ) -> Result<TransitionResult, KernelExecutionError> {
        let pending = state.execution.pending_decision.as_ref().expect("checked");
        let request = &pending.request;
        let continuation_id = match request.continuation_id {
            Some(id) => id,
            None => return rejected(state),
        };
        let selected_piece_keys = match state
            .execution
            .continuations
            .get(&continuation_id)
            .map(|record| &record.payload)
        {
            Some(ContinuationPayloadV2::SyntheticM2Assembly {
                stage: AssemblyStageV2::OrderMembers,
                selected_piece_keys,
                ordered_piece_keys: persisted,
                ..
            }) if persisted.is_empty() => selected_piece_keys.clone(),
            _ => return rejected(state),
        };
        let mut ordered_piece_keys = Vec::with_capacity(candidate_ids.len());
        for candidate_id in candidate_ids {
            ordered_piece_keys.push(answered_piece(request, *candidate_id)?);
        }
        let mut answered_set = ordered_piece_keys.clone();
        answered_set.sort_unstable();
        answered_set.dedup();
        if answered_set != selected_piece_keys || ordered_piece_keys.len() != answered_set.len() {
            return rejected(state);
        }

        // Completion consumes no fresh decision or visible identity: it must
        // remain possible even when both allocator cursors are exhausted.
        let revision = next_revision(state)?;
        let mut next = state.clone();
        next.revision = revision;
        let events = vec![cleared_event(state, revision, request.decision_id)?];

        build_accepted_product(state, next, events, move |workspace| {
            workspace.execution.continuations.remove(&continuation_id);
            workspace.execution.pending_decision = None;
            Ok(())
        })
    }
}
