//! Normative replay wire contracts and internal replay identity.
//!
//! Historical support classification (docs/REPLAY_AND_DETERMINISM.md): Replay
//! V2 is `READABLE_VERIFIABLE_ONLY`. There is no current V2 writer on the
//! public API; the detached V2 recorder survives only as historical-evidence
//! tooling for the immutable M1 regression tests.

mod identity;
mod manifest;
#[cfg(test)]
mod recorder;
mod v1;
mod v2;
mod v3;
mod validation;

#[cfg(test)]
mod tests;

pub use identity::{
    DeckIdentityV1, KernelIdentityV1, RandomnessIdentityV1, ReplayIdentity, ReplaySchemaVersionsV1,
};
pub use manifest::ReplayManifestV1;
pub use v1::{AuthoritativeReplayV1, ReplayStepV1};
pub use v2::{AuthoritativeReplayV2, RandomnessIdentityV2, ReplayManifestV2, ReplayStepV2};
pub use v3::{
    AuthoritativeReplayV3, InitialEnvironmentIdentityV3, ReplayManifestV3, ReplayRecorderV3,
    ReplayStepV3, REPLAY_FILE_SCHEMA_V3, REPLAY_MANIFEST_SCHEMA_V3, REPLAY_STEP_SCHEMA_V3,
};
pub use validation::ReplayValidationError;

pub const REPLAY_MANIFEST_SCHEMA: &str = manifest::REPLAY_MANIFEST_SCHEMA;
pub const REPLAY_FILE_SCHEMA: &str = v1::REPLAY_FILE_SCHEMA;
pub const REPLAY_MANIFEST_SCHEMA_V2: &str = v2::REPLAY_MANIFEST_SCHEMA_V2;
pub const REPLAY_FILE_SCHEMA_V2: &str = v2::REPLAY_FILE_SCHEMA_V2;
