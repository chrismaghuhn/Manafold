use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum StateDigestError {
    #[error("canonical full-state digest serialization failed")]
    Serialization,
    #[error("persisted full-state digest encoding failed: {0}")]
    Persistence(mtgml_persistence::PersistenceDecodeErrorV1),
    #[error("state violates the authoritative V3 digest preconditions")]
    StateInvariant,
}
