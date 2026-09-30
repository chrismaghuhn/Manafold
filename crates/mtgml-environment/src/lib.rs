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
#[cfg(test)]
mod semantic_catalog_kat;
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
pub use player_projection::{
    project_magic_basic_land_observation_v1, project_successor_information_state_v3,
};
mod semantic_catalog_generated;
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
