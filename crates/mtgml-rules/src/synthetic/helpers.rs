//! Ownership: mechanical helpers of the current synthetic program (fresh
//! stage identity, revision/allocator advancement, candidate construction,
//! authoritative event constructors, rejection product, global stream key).

use mtgml_decision::{CandidateIntent, EngineCandidateBinding};
use mtgml_model::{
    CandidateIdV1, DecisionId, EpisodeStatus, PlayerDecisionIdV1, PlayerId, RuleEventId,
    StateRevision,
};
use mtgml_random::{RandomStreamKeyV1, RandomStreamKindV1};
use mtgml_state::{EngineState, EngineStateViolation, StateDelta};

use crate::errors::KernelExecutionError;
use crate::events::{AuthoritativeRuleEvent, AuthoritativeRuleEventKind};
use crate::transition::TransitionResult;
use crate::validate_transition_contract;

pub(super) fn global_stream() -> RandomStreamKeyV1 {
    RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1)
}

pub(super) struct StageIdentity {
    pub(super) revision: StateRevision,
    pub(super) decision_id: DecisionId,
    pub(super) player_decision_id: PlayerDecisionIdV1,
}

/// Advances only the authoritative revision; completion needs nothing else.
pub(super) fn next_revision(state: &EngineState) -> Result<StateRevision, KernelExecutionError> {
    Ok(StateRevision(
        state
            .revision
            .0
            .checked_add(1)
            .ok_or(KernelExecutionError::RevisionOverflow)?,
    ))
}

/// Allocates every fresh identity up front: exhaustion must be detected
/// before a workspace is created or mutated. Only stages that actually
/// expose a new decision may call this.
pub(super) fn fresh_stage_identity(
    state: &EngineState,
    actor: PlayerId,
) -> Result<StageIdentity, KernelExecutionError> {
    let revision = next_revision(state)?;
    let decision_id = state.allocators.next_decision_id;
    if state.allocators.next_decision_id.0 == u64::MAX {
        return Err(KernelExecutionError::Exhaustion("decision"));
    }
    let identity = state
        .perspective_identities
        .players
        .get(&actor)
        .ok_or(KernelExecutionError::Exhaustion("perspective"))?;
    let player_decision_id = identity.next_player_decision_id;
    if identity.next_player_decision_id.0 == u64::MAX {
        return Err(KernelExecutionError::Exhaustion("player_decision"));
    }
    Ok(StageIdentity {
        revision,
        decision_id,
        player_decision_id,
    })
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

pub(super) fn advance_player_allocator(
    workspace: &mut EngineState,
    actor: PlayerId,
    issued: PlayerDecisionIdV1,
) -> Result<(), KernelExecutionError> {
    let identity = workspace
        .perspective_identities
        .players
        .get_mut(&actor)
        .ok_or(KernelExecutionError::AfterState(
            EngineStateViolation::MissingTurnPlayer,
        ))?;
    identity.next_player_decision_id = PlayerDecisionIdV1(issued.0 + 1);
    Ok(())
}

pub(super) fn rejected(state: &EngineState) -> Result<TransitionResult, KernelExecutionError> {
    let result = TransitionResult {
        accepted: false,
        next_state: state.clone(),
        delta: StateDelta::between(state, state, vec![]).map_err(KernelExecutionError::Delta)?,
        events: vec![],
        next_decision: state
            .execution
            .pending_decision
            .as_ref()
            .map(|record| record.request.clone()),
        status: EpisodeStatus::Running,
    };
    validate_transition_contract(state, &result)
        .map_err(KernelExecutionError::TransitionContract)?;
    Ok(result)
}
