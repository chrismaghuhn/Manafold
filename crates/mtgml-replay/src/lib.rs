//! Normative replay wire contracts: Replay V8 and the identities it embeds.

mod contract_material;
mod identity;
mod randomness;
mod v8;
mod validation;

pub use contract_material::{ContentContractMaterialV1, SemanticContractMaterialV7};
pub use identity::{DeckIdentityV1, KernelIdentityV1};
pub use randomness::RandomnessIdentityV2;
pub use v8::{
    AuthoritativeReplayV8, InitialEnvironmentIdentityV8, ReplayManifestV8, ReplayRecorderV8,
    ReplaySchemaVersionsV8, ReplayStepV8, REPLAY_FILE_SCHEMA_V8, REPLAY_MANIFEST_SCHEMA_V8,
    REPLAY_STEP_SCHEMA_V8,
};
pub use validation::ReplayValidationError;
