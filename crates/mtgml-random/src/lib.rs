pub mod hmac_counter;
pub mod sampling;
pub mod types;

pub use types::{
    encode_lower_hex, CanonicalRandomStreamEntryV1, RandomStateV1, RandomStreamCursorV1,
    RandomStreamKeyV1, RandomStreamKindV1, RandomStreamScopeV1, RandomValidationError, RootSeed256,
    MTGML_RNG_V1,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum LegacyRandomValidationError {
    #[error("randomness algorithm and derivation identifiers must be non-empty")]
    EmptyIdentity,
    #[error("root seed must be exactly 64 lowercase hexadecimal characters")]
    InvalidSeedHex,
    #[error("random stream names must be non-empty")]
    EmptyStreamName,
}
