pub mod hmac_counter;
pub mod sampling;

mod seed;
mod state;
mod stream_key;

pub mod types {
    pub use crate::seed::{
        encode_lower_hex, validate_seed_hex, RandomValidationError, RootSeed256, MTGML_RNG_V1,
    };
    pub use crate::state::{CanonicalRandomStreamEntryV1, RandomStateV1, RandomStreamCursorV1};
    pub use crate::stream_key::{RandomStreamKeyV1, RandomStreamKindV1, RandomStreamScopeV1};
}

pub use types::{
    encode_lower_hex, CanonicalRandomStreamEntryV1, RandomStateV1, RandomStreamCursorV1,
    RandomStreamKeyV1, RandomStreamKindV1, RandomStreamScopeV1, RandomValidationError, RootSeed256,
    MTGML_RNG_V1,
};
