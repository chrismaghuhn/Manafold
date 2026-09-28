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

pub fn canonical_state_bytes_v7_with_profile_domain_context(
    state: &EngineStatePartsV3,
    context: &mtgml_decision::ProfileDecisionDomainContextV1,
    active_execution_identity: &mtgml_model::ExecutionIdentityV1,
    active_rules_contract_id: &mtgml_model::RulesContractIdV1,
) -> Result<Vec<u8>, StateDigestError> {
    FullStateDigestInputV7::from_successor_with_profile_domain_context(
        state,
        context,
        active_execution_identity,
        active_rules_contract_id,
    )?
    .canonical_payload()
}

pub fn calculate_full_state_digest_v7(
    state: &EngineStatePartsV3,
) -> Result<FullStateDigestV7, StateDigestError> {
    let payload = canonical_state_bytes_v7(state)?;
    calculate_full_state_digest_v7_payload(&payload)
}

pub fn calculate_full_state_digest_v7_with_profile_domain_context(
    state: &EngineStatePartsV3,
    context: &mtgml_decision::ProfileDecisionDomainContextV1,
    active_execution_identity: &mtgml_model::ExecutionIdentityV1,
    active_rules_contract_id: &mtgml_model::RulesContractIdV1,
) -> Result<FullStateDigestV7, StateDigestError> {
    let payload = canonical_state_bytes_v7_with_profile_domain_context(
        state,
        context,
        active_execution_identity,
        active_rules_contract_id,
    )?;
    calculate_full_state_digest_v7_payload(&payload)
}

pub fn calculate_full_state_digest_v7_payload(
    payload: &[u8],
) -> Result<FullStateDigestV7, StateDigestError> {
    // This verifies canonical V7 bytes and computes their digest only. It does
    // not admit those bytes as authoritative profile-dependent state; callers
    // must use the typed state/context API for state admission.
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

pub fn verify_full_state_digest_v7_with_profile_domain_context(
    state: &EngineStatePartsV3,
    expected: FullStateDigestV7,
    context: &mtgml_decision::ProfileDecisionDomainContextV1,
    active_execution_identity: &mtgml_model::ExecutionIdentityV1,
    active_rules_contract_id: &mtgml_model::RulesContractIdV1,
) -> Result<(), StateDigestError> {
    if calculate_full_state_digest_v7_with_profile_domain_context(
        state,
        context,
        active_execution_identity,
        active_rules_contract_id,
    )? == expected
    {
        Ok(())
    } else {
        Err(StateDigestError::StateInvariant)
    }
}
