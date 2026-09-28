//! Detached FullStateDigestV7 producer for the accepted G0 successor state.
//!
//! G0j alone changes the current writer alias. These APIs require an explicit
//! successor aggregate and do not route through the current engine.

use mtgml_model::FullStateDigestV7;
use mtgml_persistence::envelope;

use crate::{
    digest::StateDigestError, EngineStatePartsV3, FullStateDigestInputV7,
    FULL_STATE_DIGEST_DOMAIN_V7, FULL_STATE_DIGEST_INPUT_SCHEMA_V7,
};

pub fn canonical_state_bytes_v7(state: &EngineStatePartsV3) -> Result<Vec<u8>, StateDigestError> {
    FullStateDigestInputV7::from_successor(state)?.canonical_payload()
}

pub fn calculate_full_state_digest_v7(
    state: &EngineStatePartsV3,
) -> Result<FullStateDigestV7, StateDigestError> {
    let payload = canonical_state_bytes_v7(state)?;
    calculate_full_state_digest_v7_payload(&payload)
}

pub fn calculate_full_state_digest_v7_payload(
    payload: &[u8],
) -> Result<FullStateDigestV7, StateDigestError> {
    // This verifies canonical V7 bytes and computes their digest only. It is
    // never proof that profile-dependent requests were admitted by the
    // RulesKernel/Environment boundary.
    FullStateDigestInputV7::from_canonical_payload(payload)?;
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

pub fn verify_full_state_digest_v7(
    state: &EngineStatePartsV3,
    expected: FullStateDigestV7,
) -> Result<(), StateDigestError> {
    if calculate_full_state_digest_v7(state)? == expected {
        Ok(())
    } else {
        Err(StateDigestError::StateInvariant)
    }
}
