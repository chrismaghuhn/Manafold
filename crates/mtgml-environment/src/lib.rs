//! Capability-separated environment APIs.
//!
//! `TrustedEnvironmentController` owns checkpoint/fork/replay capabilities.
//! `PlayerEndpointHandle` is permanently perspective-bound and exposes only
//! projected information. Multiple player handles may coexist.

mod checkpoint;
mod controller;
mod endpoint;
mod errors;
mod replay;
mod synthetic;

#[cfg(test)]
mod tests;

pub use checkpoint::{
    CheckpointValidationError, EnvironmentCheckpointV3, ENVIRONMENT_CHECKPOINT_SCHEMA,
};
pub use controller::{EnvironmentBackend, TrustedEnvironmentController};
pub use endpoint::{PlayerApiError, PlayerEndpoint, PlayerEndpointHandle};
pub use errors::ControllerError;
pub use errors::ReplayExecutionError;
pub use mtgml_model::{CheckpointCodecIdentity, EnvironmentLimitCounters};
pub use replay::{ReplayExecutionReport, ReplayExecutionTrace};
pub use synthetic::{
    SyntheticM1EnvironmentBackend, SyntheticM1EnvironmentConfig, SyntheticM1ReplayConfig,
};
