//! Capability-separated environment APIs.
//!
//! `TrustedEnvironmentController` owns checkpoint/fork/replay capabilities.
//! `PlayerEndpointHandle` is permanently perspective-bound and exposes only
//! projected information. Multiple player handles may coexist.

pub mod basic_land_runtime_v8;
mod boundary;
pub mod checkpoint_v8;
mod controller;
pub mod controller_successor;
mod endpoint;
pub mod endpoint_successor;
mod errors;
mod player_projection;
pub mod successor_projection;
#[cfg(test)]
mod tests;

pub use basic_land_runtime_v8::{
    BasicLandEnvironmentRuntimeV8, BasicLandEnvironmentRuntimeV8 as SuccessorEnvironmentRuntime,
    BasicLandReplayV8ExecutionReport, BasicLandRuntimeOutputV8,
};
pub use boundary::{submit_response_bytes, PlayerBoundaryError};
pub use checkpoint_v8::{
    CheckpointV8Error, EnvironmentCheckpointV8, CHECKPOINT_CODEC_ID_V8,
    CHECKPOINT_CODEC_SEMANTIC_VERSION_V8, ENVIRONMENT_CHECKPOINT_SCHEMA_V8,
};
pub use controller::{EnvironmentBackend, TrustedEnvironmentController};
pub use endpoint::{PlayerEndpoint, PlayerEndpointError, PlayerEndpointHandle};
pub type CurrentPlayerStep = mtgml_observation::PlayerStepV4;
pub use errors::ControllerError;
pub use errors::ReplayExecutionError;
pub use mtgml_model::{CheckpointCodecIdentity, EnvironmentLimitCounters};
pub use player_projection::project_successor_information_state;
