//! Rules-neutral persisted semantic codec and digest-envelope primitives.

pub mod authority;
pub mod cbor;
pub mod checkpoint_digest;
pub mod envelope;
pub mod error;

pub use error::{PersistenceDecodeErrorV1, PersistenceErrorCategory};

#[cfg(test)]
mod tests;
