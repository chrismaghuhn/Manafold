//! Capability-separated environment APIs.
//!
//! `TrustedEnvironmentController` owns checkpoint/fork/replay capabilities.
//! `PlayerEndpointHandle` is permanently perspective-bound and exposes only
//! projected information. Multiple player handles may coexist.

mod boundary;
mod checkpoint;
mod controller;
mod endpoint;
mod errors;
pub mod lifecycle_projection;
mod replay;
#[cfg(test)]
mod semantic_catalog_kat;
mod synthetic;
#[cfg(test)]
mod tests;

pub use boundary::{submit_response_bytes, PlayerBoundaryError};
pub use checkpoint::{
    CheckpointValidationError, EnvironmentCheckpointV4, CHECKPOINT_CODEC_ID_V4,
    CHECKPOINT_CODEC_SEMANTIC_VERSION_V4, ENVIRONMENT_CHECKPOINT_SCHEMA,
};
pub use controller::{EnvironmentBackend, TrustedEnvironmentController};
pub use endpoint::{PlayerEndpoint, PlayerEndpointError, PlayerEndpointHandle};
pub use errors::ControllerError;
pub use errors::ReplayExecutionError;
pub use mtgml_model::{CheckpointCodecIdentity, EnvironmentLimitCounters};
// Task 3 generated catalog module: wired ONLY into test builds until Task 8
// introduces the runtime catalog consumer. Keeping the declaration cfg(test)
// preserves the pre-Task-8 production compile surface exactly.
#[cfg(test)]
mod semantic_catalog_generated;
pub use replay::{ReplayExecutionReport, ReplayExecutionTrace};
pub use synthetic::{
    SyntheticM1EnvironmentBackend, SyntheticM1EnvironmentConfig, SyntheticM1ReplayConfig,
};
