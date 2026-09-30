//! FullStateDigestV7: validate the state once, encode its preimage once, and
//! hash it once.

use mtgml_model::FullStateDigestV7;
use mtgml_persistence::{cbor, envelope};

use crate::{
    digest::StateDigestError, persisted_v7::state_value, EngineStatePartsV3,
    FULL_STATE_DIGEST_DOMAIN_V7, FULL_STATE_DIGEST_INPUT_SCHEMA_V7,
};

pub fn canonical_state_bytes_v7(state: &EngineStatePartsV3) -> Result<Vec<u8>, StateDigestError> {
    state
        .validate()
        .map_err(|_| StateDigestError::StateInvariant)?;
    encode(state)
}

/// Canonicalizes a structurally valid state after a RulesKernel-owned exact
/// profile-domain check at the containing runtime boundary. This encoder does
/// not itself prove or admit a Decision domain.
pub(crate) fn canonical_state_bytes_v7_structural_only(
    state: &EngineStatePartsV3,
) -> Result<Vec<u8>, StateDigestError> {
    state
        .validate_structure()
        .map_err(|_| StateDigestError::StateInvariant)?;
    encode(state)
}

fn encode(state: &EngineStatePartsV3) -> Result<Vec<u8>, StateDigestError> {
    cbor::encode_canonical(&state_value(state)?).map_err(StateDigestError::Persistence)
}

pub fn calculate_full_state_digest_v7(
    state: &EngineStatePartsV3,
) -> Result<FullStateDigestV7, StateDigestError> {
    full_state_digest_v7_from_payload(&canonical_state_bytes_v7(state)?)
}

#[doc(hidden)]
pub fn calculate_full_state_digest_v7_structural_only(
    state: &EngineStatePartsV3,
) -> Result<FullStateDigestV7, StateDigestError> {
    full_state_digest_v7_from_payload(&canonical_state_bytes_v7_structural_only(state)?)
}

/// The hash step alone: envelope the preimage bytes and hash them. It does not
/// decode or check the preimage, so it is only meaningful for bytes produced
/// by `canonical_state_bytes_v7` (or a known-answer vector of them).
pub fn full_state_digest_v7_from_payload(
    payload: &[u8],
) -> Result<FullStateDigestV7, StateDigestError> {
    let encoded = envelope::encode_envelope(
        FULL_STATE_DIGEST_DOMAIN_V7,
        FULL_STATE_DIGEST_INPUT_SCHEMA_V7,
        payload,
    )
    .map_err(StateDigestError::Persistence)?;
    Ok(FullStateDigestV7::from_digest_bytes(
        envelope::hash_envelope(&encoded),
    ))
}
