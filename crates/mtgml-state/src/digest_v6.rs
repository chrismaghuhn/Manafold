//! Detached FullStateDigestV6 producer and verifier.
//!
//! The current `EngineState::digest()` remains V5. This module composes a V6
//! identity only when explicitly requested with a separate V6 state-family
//! value and never changes current runtime aliases.

use mtgml_persistence::cbor::Value;

use crate::{
    digest::StateDigestError,
    engine::EngineState,
    persisted_v6::{CardRulesAuthoritativeStateV1, FullStateDigestInputV6, PersistedExecutionV3},
};

pub const FULL_STATE_DIGEST_DOMAIN_V6: &str = "mtgml.full-state-digest.v6";
pub const FULL_STATE_DIGEST_INPUT_SCHEMA_V6: &str = "full-state-digest-input.v6";

/// Successor producer that binds typed V3 execution state directly. The
/// predecessor carrier must not contain a V2 pending request, preventing two
/// pending-decision authorities in one semantic snapshot.
pub fn canonical_state_bytes_v6_with_execution_v3(
    state: &EngineState,
    execution: &crate::ExecutionStateV3,
    card_rules_state: CardRulesAuthoritativeStateV1,
) -> Result<Vec<u8>, StateDigestError> {
    if state.execution.pending_decision.is_some() {
        return Err(StateDigestError::StateInvariant);
    }
    let predecessor = crate::digest_v5::full_state_digest_input_v5(state)?.canonical_value()?;
    let Value::Array(fields) = predecessor else {
        return Err(StateDigestError::StateInvariant);
    };
    if fields.len() != 13 {
        return Err(StateDigestError::StateInvariant);
    }
    let execution_v3 = PersistedExecutionV3::from_successor(execution)?;
    let Value::Unsigned(revision) = &fields[2] else {
        return Err(StateDigestError::StateInvariant);
    };
    let input = FullStateDigestInputV6 {
        revision: *revision,
        core_v1: fields[3].clone(),
        zones_v1: fields[4].clone(),
        allocators_v3: fields[5].clone(),
        execution_v3,
        random_v1: fields[7].clone(),
        knowledge_v2: fields[8].clone(),
        perspective_identities_v2: fields[9].clone(),
        combat: fields[10].clone(),
        foundation_sources: fields[11].clone(),
        format_v1: fields[12].clone(),
        card_rules_state,
    };
    input
        .canonical_payload()
        .map_err(|_| StateDigestError::StateInvariant)
}
