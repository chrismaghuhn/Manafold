//! Normative replay wire contracts: Replay V8 and the identities it embeds.

mod identity;
mod v2;
mod v7;
mod v8;
mod validation;

pub use identity::{DeckIdentityV1, KernelIdentityV1};
pub use v2::RandomnessIdentityV2;
pub use v7::{ContentContractMaterialV1, SemanticContractMaterialV7};
pub use v8::{
    AuthoritativeReplayV8, InitialEnvironmentIdentityV8, ReplayManifestV8, ReplayRecorderV8,
    ReplaySchemaVersionsV8, ReplayStepV8, REPLAY_FILE_SCHEMA_V8, REPLAY_MANIFEST_SCHEMA_V8,
    REPLAY_STEP_SCHEMA_V8,
};
pub use validation::ReplayValidationError;
