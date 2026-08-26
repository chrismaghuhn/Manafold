//! Ownership: proof that the committed state is executable by this kernel
//! (context legality), including the standalone synthetic entry support.

use mtgml_decision::{
    validate_candidate_binding, ActionCandidate, CandidateIntent, DecisionDomainV2,
    EngineCandidateBinding, PerspectiveIdentityResolver,
};
use mtgml_model::GameObjectId;
use mtgml_state::EngineState;

use super::helpers::global_stream;
use crate::errors::KernelExecutionError;

/// Context legality of the current synthetic program over a structurally
/// valid `EngineState`.
///
/// `validate_engine_state()` proves generic authoritative invariants; this
/// proof additionally guarantees that every decision this environment can
/// offer is actually executable by the kernel Ã¢â‚¬â€ including the standalone
/// root decision. An unsupported engine-offered path is an internal
/// soundness failure, never a player rejection.
pub fn validate_synthetic_runtime_state(state: &EngineState) -> Result<(), KernelExecutionError> {
    mtgml_state::validate_engine_state(state).map_err(KernelExecutionError::BeforeState)?;
    match state.execution.pending_decision.as_ref() {
        None => {
            // A completed environment holds neither continuation nor request.
            if !state.execution.continuations.is_empty() {
                Err(KernelExecutionError::UnsupportedStagePath)
            } else {
                Ok(())
            }
        }
        Some(pending) => {
            if pending.request.continuation_id.is_some() {
                // Stage requests are fully bound by state-level program
                // coherence, and every reachable stage is supported here.
                Ok(())
            } else {
                // Standalone decisions are only supported as the exact
                // synthetic entry program.
                entry_supported(state)
            }
        }
    }
}

/// Whether the committed state offers the one supported synthetic entry
/// decision with all preconditions the kernel needs to accept its answer.
pub(super) fn entry_supported(state: &EngineState) -> Result<(), KernelExecutionError> {
    let Some(pending) = state.execution.pending_decision.as_ref() else {
        return Err(KernelExecutionError::UnsupportedStagePath);
    };
    let request = &pending.request;
    let actor = request.actor;
    if !matches!(request.decision, DecisionDomainV2::ChooseOne) || request.candidates.len() != 1 {
        return Err(KernelExecutionError::UnsupportedStagePath);
    }
    let candidate = &request.candidates[0];
    let CandidateIntent::SelectObject {
        object: opaque_object,
    } = &candidate.visible_intent
    else {
        return Err(KernelExecutionError::UnsupportedStagePath);
    };
    if state
        .perspective_identities
        .resolve_object(actor, *opaque_object)
        != Some(GameObjectId(1))
        || candidate.trusted_binding
            != (EngineCandidateBinding::SelectObject {
                object: GameObjectId(1),
            })
    {
        return Err(KernelExecutionError::UnsupportedStagePath);
    }
    let visible = ActionCandidate {
        candidate_id: candidate.candidate_id.to_string(),
        semantic_key: "candidate.0".into(),
        intent: candidate.visible_intent.clone(),
    };
    if validate_candidate_binding(
        &visible,
        &candidate.trusted_binding,
        actor,
        &state.perspective_identities,
    )
    .is_err()
    {
        return Err(KernelExecutionError::UnsupportedStagePath);
    }
    let stream = global_stream();
    if !state.random.streams.contains_key(&stream)
        || state.core.players.get(&actor).map(|player| player.life) != Some(40)
    {
        return Err(KernelExecutionError::UnsupportedStagePath);
    }
    Ok(())
}
