//! Rules-neutral persisted semantic codec and digest-envelope primitives.

pub mod cbor;
pub mod checkpoint_digest;
pub mod content_contract_digest;
pub mod envelope;
pub mod error;
pub mod semantic_contract_digest;

pub use error::{PersistenceDecodeErrorV1, PersistenceErrorCategory};

#[cfg(test)]
mod tests;
