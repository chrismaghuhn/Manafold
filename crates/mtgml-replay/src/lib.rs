//! Normative replay wire contracts and internal replay identity.

mod identity;
mod manifest;
mod v1;
mod v2;
mod validation;

#[cfg(test)]
mod tests;

pub use identity::{
    DeckIdentityV1, KernelIdentityV1, RandomnessIdentityV1, ReplayIdentity, ReplaySchemaVersionsV1,
};
pub use manifest::ReplayManifestV1;
pub use validation::ReplayValidationError;
pub use v1::{AuthoritativeReplayV1, ReplayStepV1};
pub use v2::{AuthoritativeReplayV2, RandomnessIdentityV2, ReplayManifestV2, ReplayStepV2};

pub const REPLAY_MANIFEST_SCHEMA: &str = manifest::REPLAY_MANIFEST_SCHEMA;
pub const REPLAY_FILE_SCHEMA: &str = v1::REPLAY_FILE_SCHEMA;
pub const REPLAY_MANIFEST_SCHEMA_V2: &str = v2::REPLAY_MANIFEST_SCHEMA_V2;
pub const REPLAY_FILE_SCHEMA_V2: &str = v2::REPLAY_FILE_SCHEMA_V2;
