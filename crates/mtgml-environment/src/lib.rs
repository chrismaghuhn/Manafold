//! Capability-separated environment APIs.
//!
//! `TrustedEnvironmentController` owns checkpoint/fork/replay capabilities.
//! `PlayerEndpointHandle` is permanently perspective-bound and exposes only
//! projected information. Multiple player handles may coexist.

mod checkpoint;
mod controller;
mod endpoint;
mod errors;

#[cfg(test)]
mod tests;

pub use checkpoint::{
    CheckpointCodecIdentity, CheckpointValidationError, EnvironmentCheckpointV2,
    EnvironmentLimitCounters, ENVIRONMENT_CHECKPOINT_SCHEMA,
};
pub use controller::{EnvironmentBackend, TrustedEnvironmentController};
pub use endpoint::{PlayerEndpoint, PlayerEndpointHandle, PlayerApiError};
pub use errors::ControllerError;
