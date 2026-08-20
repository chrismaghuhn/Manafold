pub mod hmac_counter;
pub mod sampling;
pub mod seed;
pub mod state;
pub mod stream_key;

pub use seed::{
    encode_lower_hex, validate_seed_hex, RandomValidationError, RootSeed256, MTGML_RNG_V1,
};
pub use state::{CanonicalRandomStreamEntryV1, RandomStateV1, RandomStreamCursorV1};
pub use stream_key::{RandomStreamKeyV1, RandomStreamKindV1, RandomStreamScopeV1};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum LegacyRandomValidationError {
    #[error("randomness algorithm and derivation identifiers must be non-empty")]
    EmptyIdentity,
    #[error("root seed must be exactly 64 lowercase hexadecimal characters")]
    InvalidSeedHex,
    #[error("random stream names must be non-empty")]
    EmptyStreamName,
}
