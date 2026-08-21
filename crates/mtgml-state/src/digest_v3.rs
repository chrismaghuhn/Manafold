//! Detached full-state V3 input and calculator.
//!
//! This module is intentionally not wired into `EngineState::digest()` until
//! the coordinated runtime cut. It owns only the fixed semantic field layout;
//! the CBOR and envelope implementation remains in `mtgml-persistence`.

use mtgml_model::FullStateDigestV3;
use mtgml_persistence::{
    cbor::{self, Value},
    envelope, PersistenceDecodeErrorV1,
};

use crate::digest::StateDigestError;

pub const FULL_STATE_DIGEST_DOMAIN_V3: &str = "mtgml.full-state-digest.v3";
pub const FULL_STATE_DIGEST_INPUT_SCHEMA_V3: &str = "full-state-digest-input.v3";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FullStateDigestInputV3 {
    pub revision: u64,
    pub core: Value,
    pub zones: Value,
    pub allocators: Value,
    pub execution: Value,
    pub random: Value,
    pub knowledge: Value,
    pub perspective_identities: Value,
    pub format: Value,
}

impl FullStateDigestInputV3 {
    pub fn canonical_value(&self) -> Result<Value, StateDigestError> {
        let Value::Array(execution) = &self.execution else {
            return Err(StateDigestError::Persistence(
                PersistenceDecodeErrorV1::SemanticValidation,
            ));
        };
        if execution.len() != 5
            || !matches!(execution.get(2), Some(Value::Array(values)) if values.is_empty())
            || !matches!(execution.get(3), Some(Value::Array(values)) if values.is_empty())
            || !matches!(execution.get(4), Some(Value::Array(values)) if values.is_empty())
        {
            return Err(StateDigestError::Persistence(
                PersistenceDecodeErrorV1::SemanticValidation,
            ));
        }
        Ok(Value::Array(vec![
            Value::Text(FULL_STATE_DIGEST_INPUT_SCHEMA_V3.to_owned()),
            Value::Text(FULL_STATE_DIGEST_DOMAIN_V3.to_owned()),
            Value::Unsigned(self.revision),
            self.core.clone(),
            self.zones.clone(),
            self.allocators.clone(),
            self.execution.clone(),
            self.random.clone(),
            self.knowledge.clone(),
            self.perspective_identities.clone(),
            self.format.clone(),
        ]))
    }

    pub fn canonical_payload(&self) -> Result<Vec<u8>, StateDigestError> {
        let value = self.canonical_value()?;
        cbor::encode_canonical(&value).map_err(StateDigestError::Persistence)
    }
}

pub(crate) fn calculate_full_state_digest_v3(
    input: &FullStateDigestInputV3,
) -> Result<FullStateDigestV3, StateDigestError> {
    let payload = input.canonical_payload()?;
    let envelope = envelope::encode_envelope(
        FULL_STATE_DIGEST_DOMAIN_V3,
        FULL_STATE_DIGEST_INPUT_SCHEMA_V3,
        &payload,
    )
    .map_err(StateDigestError::Persistence)?;
    Ok(FullStateDigestV3::from_digest_bytes(
        envelope::hash_envelope(&envelope),
    ))
}
