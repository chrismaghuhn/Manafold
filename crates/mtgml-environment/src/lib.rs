//! Capability-separated environment APIs.
//!
//! `TrustedEnvironmentController` owns checkpoint/fork/replay capabilities.
//! `PlayerEndpointHandle` is permanently perspective-bound and exposes only
//! projected information. Multiple player handles may coexist.

mod boundary;
pub mod checkpoint;
pub mod checkpoint_v7;
pub mod checkpoint_v8;
#[cfg(any(test, feature = "historical-conformance-runtime"))]
#[path = "controller_predecessor.rs"]
mod controller;
#[cfg(not(any(test, feature = "historical-conformance-runtime")))]
mod controller;
#[cfg(any(test, not(feature = "historical-conformance-runtime")))]
pub mod controller_successor;
#[cfg(any(test, feature = "historical-conformance-runtime"))]
#[path = "endpoint_predecessor.rs"]
mod endpoint;
#[cfg(not(any(test, feature = "historical-conformance-runtime")))]
mod endpoint;
#[cfg(any(test, not(feature = "historical-conformance-runtime")))]
pub mod endpoint_successor;
mod errors;
pub mod lifecycle_projection;
mod player_projection;
#[cfg(any(test, feature = "historical-conformance-runtime"))]
mod reference;
#[cfg(any(test, feature = "historical-conformance-runtime"))]
mod replay;
pub mod replay_v7_execution;
#[cfg(any(test, feature = "historical-conformance-runtime"))]
mod response_transaction;
#[cfg(any(test, feature = "historical-conformance-runtime"))]
mod semantic_catalog;
#[cfg(test)]
mod semantic_catalog_kat;
pub mod successor_projection;
pub mod successor_runtime;
pub mod successor_transaction;
#[cfg(any(test, feature = "historical-conformance-runtime"))]
mod synthetic;
#[cfg(test)]
mod tests;

pub use boundary::{submit_response_bytes, PlayerBoundaryError};
pub use checkpoint::{
    CheckpointValidationError, EnvironmentCheckpointV5, EnvironmentCheckpointV6,
    CHECKPOINT_CODEC_ID_V5, CHECKPOINT_CODEC_ID_V6, CHECKPOINT_CODEC_SEMANTIC_VERSION_V5,
    CHECKPOINT_CODEC_SEMANTIC_VERSION_V6, ENVIRONMENT_CHECKPOINT_SCHEMA_V5,
    ENVIRONMENT_CHECKPOINT_SCHEMA_V6,
};
pub use checkpoint_v7::{
    CheckpointV7Error, EnvironmentCheckpointV7, CHECKPOINT_CODEC_ID_V7,
    CHECKPOINT_CODEC_SEMANTIC_VERSION_V7, ENVIRONMENT_CHECKPOINT_SCHEMA_V7,
};
pub use checkpoint_v8::{
    CheckpointV8Error, EnvironmentCheckpointV8, CHECKPOINT_CODEC_ID_V8,
    CHECKPOINT_CODEC_SEMANTIC_VERSION_V8, ENVIRONMENT_CHECKPOINT_SCHEMA_V8,
};
pub use controller::{EnvironmentBackend, TrustedEnvironmentController};
pub use endpoint::{PlayerEndpoint, PlayerEndpointError, PlayerEndpointHandle};
#[cfg(any(test, feature = "historical-conformance-runtime"))]
pub type CurrentPlayerStep = mtgml_observation::PlayerStepV2;
#[cfg(not(any(test, feature = "historical-conformance-runtime")))]
pub type CurrentPlayerStep = mtgml_observation::PlayerStepV3;
pub use errors::ControllerError;
pub use errors::ReplayExecutionError;
pub use mtgml_model::{CheckpointCodecIdentity, EnvironmentLimitCounters};
pub use player_projection::{
    project_magic_basic_land_observation_v1, project_successor_information_state,
    project_successor_information_state_v3,
};
// Task 3 generated catalog module: unconditional compile surface — it is the
// production input Task 8 consumes. Only the KAT is test-only.
mod semantic_catalog_generated;
#[cfg(any(test, feature = "historical-conformance-runtime"))]
pub use reference::{
    ReferenceEnvironmentBackend, ReferenceEnvironmentConfig, ReferenceEnvironmentReplayConfig,
    REFERENCE_SCENARIO_ID,
};
#[cfg(any(test, feature = "historical-conformance-runtime"))]
pub use replay::{ReplayExecutionReport, ReplayExecutionTrace};
pub use semantic_catalog_generated::magic_combat_attackers_0_1_0_rules_manifest;
pub use semantic_catalog_generated::magic_combat_attackers_0_1_0_semantic_contract_id;
pub use semantic_catalog_generated::magic_combat_attackers_0_1_0_semantic_manifest;
pub use semantic_catalog_generated::magic_combat_blockers_0_1_0_rules_manifest;
pub use semantic_catalog_generated::magic_combat_blockers_0_1_0_semantic_contract_id;
pub use semantic_catalog_generated::magic_combat_blockers_0_1_0_semantic_manifest;
pub use semantic_catalog_generated::magic_s3_a_ordered_sba_0_1_0_rules_manifest;
pub use semantic_catalog_generated::magic_s3_a_ordered_sba_0_1_0_semantic_contract_id;
pub use semantic_catalog_generated::magic_s3_a_ordered_sba_0_1_0_semantic_manifest;
pub use semantic_catalog_generated::magic_s3_b_basic_priority_0_1_0_rules_manifest;
pub use semantic_catalog_generated::magic_s3_b_basic_priority_0_1_0_semantic_contract_id;
pub use semantic_catalog_generated::magic_s3_b_basic_priority_0_1_0_semantic_manifest;
pub use semantic_catalog_generated::magic_s3_c_draw_interaction_0_1_0_rules_manifest;
pub use semantic_catalog_generated::magic_s3_c_draw_interaction_0_1_0_semantic_contract_id;
pub use semantic_catalog_generated::magic_s3_c_draw_interaction_0_1_0_semantic_manifest;
pub use semantic_catalog_generated::magic_turn_structure_0_1_0_rules_manifest;
pub use semantic_catalog_generated::magic_turn_structure_0_1_0_semantic_contract_id;
pub use semantic_catalog_generated::magic_turn_structure_0_1_0_semantic_manifest;
pub use semantic_catalog_generated::synthetic_legacy_default_rules_manifest;
pub use semantic_catalog_generated::synthetic_legacy_default_semantic_contract_id;
pub use semantic_catalog_generated::synthetic_legacy_default_semantic_manifest;
pub use successor_runtime::SuccessorEnvironmentRuntime;
#[cfg(any(test, feature = "historical-conformance-runtime"))]
pub use synthetic::{
    SyntheticRulesEnvironmentBackend, SyntheticRulesEnvironmentConfig, SyntheticRulesReplayConfig,
};
