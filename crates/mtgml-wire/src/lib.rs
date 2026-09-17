//! Canonical JSON writer/reader for every public v1 wire contract.
//!
//! JSON Schema validates shape. This crate additionally enforces closed Rust
//! variants, integer ranges, canonical bytes, and cross-field semantics.

mod canonical_json;
mod contract;
mod decision;
mod error;
mod fixtures;
mod observation;
mod replay;

pub use canonical_json::{decode_canonical, encode_canonical};
pub use contract::WireContract;
pub use decision::decision_response_v2;
pub use error::{PlayerWireErrorCodeV1, WireError};
pub use fixtures::{
    verify_golden_fixture_directory, verify_negative_fixture_directory, FixtureVerificationError,
};
pub use observation::compute_information_state_digest_v2;

// The shared negative fixture test remains in the root tests module as
// every_shared_negative_fixture; this marker keeps the repository guard aware
// of the mechanically moved test body.
#[cfg(test)]
pub(crate) mod tests;

#[cfg(test)]
mod constructive_producer_tests;
