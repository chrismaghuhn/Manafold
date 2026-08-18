//! Checkpointable deterministic randomness contracts.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RandomStreamState {
    pub counter: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RandomState {
    pub algorithm_id: String,
    pub derivation_version: String,
    pub root_seed_hex: String,
    pub streams: BTreeMap<String, RandomStreamState>,
}

impl RandomState {
    pub fn validate(&self) -> Result<(), RandomValidationError> {
        if self.algorithm_id.is_empty() || self.derivation_version.is_empty() {
            return Err(RandomValidationError::EmptyIdentity);
        }
        validate_seed_hex(&self.root_seed_hex)?;
        if self.streams.keys().any(|name| name.is_empty()) {
            return Err(RandomValidationError::EmptyStreamName);
        }
        Ok(())
    }
}

pub fn validate_seed_hex(value: &str) -> Result<(), RandomValidationError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(RandomValidationError::InvalidSeedHex);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum RandomValidationError {
    #[error("randomness algorithm and derivation identifiers must be non-empty")]
    EmptyIdentity,
    #[error("root seed must be exactly 64 lowercase hexadecimal characters")]
    InvalidSeedHex,
    #[error("random stream names must be non-empty")]
    EmptyStreamName,
}
