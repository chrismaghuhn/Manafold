//! Detached FullStateDigestV6 producer and verifier.
//!
//! The current `EngineState::digest()` remains V5. This module composes a V6
//! identity only when explicitly requested with a separate V6 state-family
//! value and never changes current runtime aliases.

use mtgml_model::FullStateDigestV6;
use mtgml_persistence::{
    cbor::{self, Value},
    envelope,
};

use crate::{
    digest::StateDigestError,
    engine::EngineState,
    persisted_v6::{CardRulesAuthoritativeStateV1, FullStateDigestInputV6, PersistedExecutionV3},
};

pub const FULL_STATE_DIGEST_DOMAIN_V6: &str = "mtgml.full-state-digest.v6";
pub const FULL_STATE_DIGEST_INPUT_SCHEMA_V6: &str = "full-state-digest-input.v6";

pub fn canonical_state_bytes_v6(
    state: &EngineState,
    card_rules_state: CardRulesAuthoritativeStateV1,
) -> Result<Vec<u8>, StateDigestError> {
    let predecessor = crate::digest_v5::full_state_digest_input_v5(state)?.canonical_value()?;
    let Value::Array(fields) = predecessor else {
        return Err(StateDigestError::StateInvariant);
    };
    if fields.len() != 13 {
        return Err(StateDigestError::StateInvariant);
    }
    let Value::Unsigned(revision) = &fields[2] else {
        return Err(StateDigestError::StateInvariant);
    };
    let input = FullStateDigestInputV6 {
        revision: *revision,
        core_v1: fields[3].clone(),
        zones_v1: fields[4].clone(),
        allocators_v3: fields[5].clone(),
        execution_v3: PersistedExecutionV3::from_value(fields[6].clone())
            .map_err(|_| StateDigestError::StateInvariant)?,
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

pub fn calculate_full_state_digest_v6(
    state: &EngineState,
    card_rules_state: CardRulesAuthoritativeStateV1,
) -> Result<FullStateDigestV6, StateDigestError> {
    let payload = canonical_state_bytes_v6(state, card_rules_state)?;
    calculate_full_state_digest_v6_payload(&payload)
}

pub(crate) fn calculate_full_state_digest_v6_payload(
    payload: &[u8],
) -> Result<FullStateDigestV6, StateDigestError> {
    let encoded = envelope::encode_envelope(
        FULL_STATE_DIGEST_DOMAIN_V6,
        FULL_STATE_DIGEST_INPUT_SCHEMA_V6,
        payload,
    )
    .map_err(StateDigestError::Persistence)?;
    Ok(FullStateDigestV6::from_digest_bytes(
        envelope::hash_envelope(&encoded),
    ))
}

/// Verify canonical V6 bytes without constructing an executable EngineState.
/// This checks the versioned input and all six successor state families; the
/// later restore boundary remains responsible for cross-state/catalog joins.
pub fn verify_full_state_digest_v6(
    canonical_payload: &[u8],
    expected: &FullStateDigestV6,
) -> Result<(), StateDigestError> {
    let value = cbor::decode_canonical(canonical_payload).map_err(StateDigestError::Persistence)?;
    let input = FullStateDigestInputV6::from_canonical_value(&value)
        .map_err(|_| StateDigestError::StateInvariant)?;
    let reencoded = input
        .canonical_payload()
        .map_err(|_| StateDigestError::StateInvariant)?;
    if reencoded != canonical_payload {
        return Err(StateDigestError::StateInvariant);
    }
    let encoded = envelope::encode_envelope(
        FULL_STATE_DIGEST_DOMAIN_V6,
        FULL_STATE_DIGEST_INPUT_SCHEMA_V6,
        &reencoded,
    )
    .map_err(StateDigestError::Persistence)?;
    let actual = FullStateDigestV6::from_digest_bytes(envelope::hash_envelope(&encoded));
    if &actual != expected {
        return Err(StateDigestError::StateInvariant);
    }
    Ok(())
}
