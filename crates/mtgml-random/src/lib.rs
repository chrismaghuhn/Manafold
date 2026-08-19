pub mod hmac_counter;
pub mod sampling;
pub mod types;

pub use types::{
    encode_lower_hex, CanonicalRandomStreamEntryV1, RandomStateV1, RandomStreamCursorV1,
    RandomStreamKindV1, RandomStreamKeyV1, RandomStreamScopeV1, RandomValidationError,
    RootSeed256, MTGML_RNG_V1,
};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RandomStreamState {
    pub counter: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RandomState {
    pub algorithm_id: String,
    pub derivation_version: String,
    pub root_seed_hex: String,
    pub streams: std::collections::BTreeMap<String, RandomStreamState>,
}

impl RandomState {
    pub fn validate(&self) -> Result<(), LegacyRandomValidationError> {
        if self.algorithm_id.is_empty() || self.derivation_version.is_empty() {
            return Err(LegacyRandomValidationError::EmptyIdentity);
        }
        validate_seed_hex(&self.root_seed_hex)?;
        if self.streams.keys().any(|name| name.is_empty()) {
            return Err(LegacyRandomValidationError::EmptyStreamName);
        }
        Ok(())
    }
}

pub fn validate_seed_hex(value: &str) -> Result<(), LegacyRandomValidationError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(LegacyRandomValidationError::InvalidSeedHex);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum LegacyRandomValidationError {
    #[error("randomness algorithm and derivation identifiers must be non-empty")]
    EmptyIdentity,
    #[error("root seed must be exactly 64 lowercase hexadecimal characters")]
    InvalidSeedHex,
    #[error("random stream names must be non-empty")]
    EmptyStreamName,
}
