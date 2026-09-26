//! Capability-separated environment APIs.
//!
//! `TrustedEnvironmentController` owns checkpoint/fork/replay capabilities.
//! `PlayerEndpointHandle` is permanently perspective-bound and exposes only
//! projected information. Multiple player handles may coexist.

mod boundary;
pub mod checkpoint;
pub mod checkpoint_v7;
mod controller;
mod endpoint;
mod errors;
pub mod lifecycle_projection;
mod player_projection;
mod reference;
mod replay;
mod response_transaction;
mod semantic_catalog;
#[cfg(test)]
mod semantic_catalog_kat;
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
pub use controller::{EnvironmentBackend, TrustedEnvironmentController};
pub use endpoint::{PlayerEndpoint, PlayerEndpointError, PlayerEndpointHandle};
pub use errors::ControllerError;
pub use errors::ReplayExecutionError;
pub use mtgml_model::{CheckpointCodecIdentity, EnvironmentLimitCounters};
pub use player_projection::project_magic_basic_land_observation_v1;
// Task 3 generated catalog module: unconditional compile surface — it is the
// production input Task 8 consumes. Only the KAT is test-only.
mod semantic_catalog_generated;
pub use reference::{
    ReferenceEnvironmentBackend, ReferenceEnvironmentConfig, ReferenceEnvironmentReplayConfig,
    REFERENCE_SCENARIO_ID,
};
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
pub use synthetic::{
    SyntheticRulesEnvironmentBackend, SyntheticRulesEnvironmentConfig, SyntheticRulesReplayConfig,
};
