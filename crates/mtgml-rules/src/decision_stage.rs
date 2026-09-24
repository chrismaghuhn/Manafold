//! Shared checked mechanics for allocating a fresh Decision stage.
//!
//! This module owns no program semantics; Synthetic and Magic callers supply
//! the actor and construct their own typed requests and continuations.

use mtgml_model::{DecisionId, EpisodeStatus, PlayerDecisionIdV1, PlayerId, StateRevision};
use mtgml_state::{EngineState, EngineStateViolation, StateDelta};

use crate::errors::KernelExecutionError;
use crate::transition::TransitionResult;
use crate::validate_transition_contract;

pub(crate) struct StageIdentity {
    pub(crate) revision: StateRevision,
    pub(crate) decision_id: DecisionId,
    pub(crate) player_decision_id: PlayerDecisionIdV1,
}

pub(crate) fn next_revision(state: &EngineState) -> Result<StateRevision, KernelExecutionError> {
    Ok(StateRevision(
        state
            .revision
            .0
            .checked_add(1)
            .ok_or(KernelExecutionError::RevisionOverflow)?,
    ))
}

/// Checks every identity cursor before a caller creates a transition workspace.
pub(crate) fn fresh_stage_identity(
    state: &EngineState,
    actor: PlayerId,
) -> Result<StageIdentity, KernelExecutionError> {
    let revision = next_revision(state)?;
    let decision_id = state.allocators.next_decision_id;
    if decision_id.0 == u64::MAX {
        return Err(KernelExecutionError::Exhaustion("decision"));
    }
    let identity = state
        .perspective_identities
        .players
        .get(&actor)
        .ok_or(KernelExecutionError::Exhaustion("perspective"))?;
    let player_decision_id = identity.next_player_decision_id;
    if player_decision_id.0 == u64::MAX {
        return Err(KernelExecutionError::Exhaustion("player_decision"));
    }
    Ok(StageIdentity {
        revision,
        decision_id,
        player_decision_id,
    })
}

pub(crate) fn advance_player_allocator(
    workspace: &mut EngineState,
    actor: PlayerId,
    issued: PlayerDecisionIdV1,
) -> Result<(), KernelExecutionError> {
    let next = issued
        .0
        .checked_add(1)
        .ok_or(KernelExecutionError::Exhaustion("player_decision"))?;
    let identity = workspace
        .perspective_identities
        .players
        .get_mut(&actor)
        .ok_or(KernelExecutionError::AfterState(
            EngineStateViolation::MissingTurnPlayer,
        ))?;
    identity.next_player_decision_id = PlayerDecisionIdV1(next);
    Ok(())
}

pub(crate) fn rejected(state: &EngineState) -> Result<TransitionResult, KernelExecutionError> {
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
