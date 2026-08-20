use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

pub const MTGML_RNG_V1: &str = "mtgml.rng.v1";

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RandomValidationError {
    #[error("unsupported RNG contract")]
    UnsupportedRngContract,
    #[error("root seed must be exactly 64 lowercase hexadecimal characters")]
    InvalidSeedHex,
    #[error("unknown stream-key codec version: {0}")]
    UnknownKeyVersion(u8),
    #[error("reserved stream kind code: {0}")]
    ReservedKind(u16),
    #[error("unknown stream kind code: {0}")]
    UnknownKind(u16),
    #[error("unknown scope tag: {0}")]
    UnknownScopeTag(u8),
    #[error("malformed stream key bytes")]
    MalformedStreamKey,
    #[error("duplicate stream key")]
    DuplicateStreamKey,
    #[error("stream entries do not match stream map")]
    StreamEntryMismatch,
    #[error("stream entries are not in canonical key-byte order")]
    UnorderedStreamEntries,
    #[error("player-scoped stream references an absent player")]
    PlayerScopeMismatch,
    #[error("invalid random bound")]
    InvalidRandomBound,
    #[error("random stream exhausted")]
    StreamExhausted,
    #[error("requested stream not found")]
    StreamNotFound,
    #[error("too many streams")]
    TooManyStreams,
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

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RootSeed256(pub [u8; 32]);

impl RootSeed256 {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn from_lower_hex(hex: &str) -> Result<Self, RandomValidationError> {
        if hex.len() != 64 {
            return Err(RandomValidationError::InvalidSeedHex);
        }
        let bytes = hex.as_bytes();
        let mut result = [0u8; 32];
        for i in 0..32 {
            let hi = hex_byte(bytes[i * 2])?;
            let lo = hex_byte(bytes[i * 2 + 1])?;
            result[i] = (hi << 4) | lo;
        }
        Ok(Self(result))
    }

    pub fn to_lower_hex(&self) -> String {
        encode_lower_hex(&self.0)
    }
}

impl fmt::Debug for RootSeed256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RootSeed256(***)")
    }
}

impl Serialize for RootSeed256 {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_lower_hex())
    }
}

impl<'de> Deserialize<'de> for RootSeed256 {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        RootSeed256::from_lower_hex(&text).map_err(serde::de::Error::custom)
    }
}

fn hex_byte(ch: u8) -> Result<u8, RandomValidationError> {
    match ch {
        b'0'..=b'9' => Ok(ch - b'0'),
        b'a'..=b'f' => Ok(ch - b'a' + 10),
        _ => Err(RandomValidationError::InvalidSeedHex),
    }
}

pub fn encode_lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_ZERO_SEED: &str = "0000000000000000000000000000000000000000000000000000000000000000";

    #[test]
    fn root_seed_roundtrip() {
        let seed = RootSeed256::from_lower_hex(ALL_ZERO_SEED).unwrap();
        assert_eq!(seed.to_lower_hex(), ALL_ZERO_SEED);
        assert_eq!(seed.as_bytes(), &[0u8; 32]);
    }

    #[test]
    fn root_seed_rejects_wrong_length() {
        assert!(RootSeed256::from_lower_hex("00").is_err());
        assert!(RootSeed256::from_lower_hex(&"a".repeat(65)).is_err());
    }

    #[test]
    fn root_seed_rejects_uppercase() {
        let mut hex = ALL_ZERO_SEED.to_owned();
        hex.replace_range(..1, "A");
        assert!(RootSeed256::from_lower_hex(&hex).is_err());
    }

    #[test]
    fn root_seed_rejects_nonhex() {
        let mut hex = ALL_ZERO_SEED.to_owned();
        hex.replace_range(..1, "g");
        assert!(RootSeed256::from_lower_hex(&hex).is_err());
    }
}
