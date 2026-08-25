//! Ownership: pending authoritative request validation and candidate
//! binding cross-checks.

use mtgml_decision::{validate_candidate_binding, ActionCandidate};

use super::EngineStateViolation;
use crate::engine::EngineState;

pub(super) fn validate_pending_authoritative_request(
    state: &EngineState,
) -> Result<(), EngineStateViolation> {
    if let Some(pending) = &state.execution.pending_decision {
        let request = &pending.request;
        request
            .validate()
            .map_err(|_| EngineStateViolation::PendingDecisionMismatch)?;
        if request.state_revision != state.revision
            || !state.core.players.contains_key(&request.actor)
            || state
                .perspective_identities
                .players
                .get(&request.actor)
                .is_none_or(|identity| {
                    identity.next_player_decision_id.0 <= request.player_decision_id.0
                })
        {
            return Err(EngineStateViolation::PendingDecisionMismatch);
        }
        for candidate in &request.candidates {
            let visible = ActionCandidate {
                candidate_id: candidate.candidate_id.to_string(),
                semantic_key: format!("candidate.{}", candidate.candidate_id.0),
                intent: candidate.visible_intent.clone(),
            };
            if !candidate
                .trusted_binding
                .same_variant_as(&candidate.visible_intent)
                || validate_candidate_binding(
                    &visible,
                    &candidate.trusted_binding,
                    request.actor,
                    &state.perspective_identities,
                )
                .is_err()
            {
                return Err(EngineStateViolation::PendingDecisionMismatch);
            }
        }
    }
    Ok(())
}
