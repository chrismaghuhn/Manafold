//! Ownership: synthetic replay manifest identity assembly only. The
//! environment transaction itself does NOT live here.

use std::collections::BTreeSet;

use mtgml_decision::{DECISION_RESPONSE_V2_SCHEMA, PLAYER_DECISION_REQUEST_V2_SCHEMA};
use mtgml_observation::{
    INFORMATION_STATE_SCHEMA_V2, OBSERVATION_SCHEMA, OBSERVED_EVENT_SCHEMA_V2,
    PLAYER_STEP_SCHEMA_V2,
};
use mtgml_random::MTGML_RNG_V1;
use mtgml_replay::{
    InitialEnvironmentIdentityV4, RandomnessIdentityV2, ReplayManifestV4, ReplaySchemaVersionsV4,
    REPLAY_MANIFEST_SCHEMA_V4, REPLAY_STEP_SCHEMA_V4,
};

use super::SyntheticM1EnvironmentConfig;
use crate::checkpoint::EnvironmentCheckpointV4;
use crate::errors::ControllerError;

fn current_v4_schema_versions() -> ReplaySchemaVersionsV4 {
    ReplaySchemaVersionsV4 {
        observation: OBSERVATION_SCHEMA.into(),
        observation_payload_codec: "synthetic-m3-observation.v1".into(),
        information_state: INFORMATION_STATE_SCHEMA_V2.into(),
        decision: PLAYER_DECISION_REQUEST_V2_SCHEMA.into(),
        decision_response: DECISION_RESPONSE_V2_SCHEMA.into(),
        observed_event: OBSERVED_EVENT_SCHEMA_V2.into(),
        player_step: PLAYER_STEP_SCHEMA_V2.into(),
        replay_step: REPLAY_STEP_SCHEMA_V4.into(),
    }
}

fn validate_current_producer_identity(
    config: &SyntheticM1EnvironmentConfig,
) -> Result<(), ControllerError> {
    if config.replay.randomness_contract_id != MTGML_RNG_V1
        || config.replay.schemas != current_v4_schema_versions()
    {
        return Err(ControllerError::ReplayIdentityMismatch);
    }
    Ok(())
}

pub(super) fn build_manifest(
    config: &SyntheticM1EnvironmentConfig,
    checkpoint: &EnvironmentCheckpointV4,
) -> Result<ReplayManifestV4, ControllerError> {
    validate_current_producer_identity(config)?;
    let mut decks = config.replay.decks.clone();
    decks.sort_by_key(|deck| deck.player);
    let manifest = ReplayManifestV4 {
        schema_version: REPLAY_MANIFEST_SCHEMA_V4.into(),
        engine_build: config.replay.engine_build.clone(),
        kernel: config.replay.kernel.clone(),
        rules_snapshot: config.replay.rules_snapshot.clone(),
        format_policy_snapshot: config.replay.format_policy_snapshot.clone(),
        oracle_snapshot: config.replay.oracle_snapshot.clone(),
        card_bundle: config.replay.card_bundle.clone(),
        schemas: config.replay.schemas.clone(),
        randomness: RandomnessIdentityV2 {
            contract_id: config.replay.randomness_contract_id.clone(),
            root_seed_hex: checkpoint.state.random.root_seed.to_lower_hex(),
        },
        decks,
        initial_identity: InitialEnvironmentIdentityV4 {
            state_revision: checkpoint.state.revision,
            full_state_digest: checkpoint.state_digest.clone(),
            episode_status: checkpoint.status.clone(),
            environment_limit_counters: checkpoint.limit_counters.clone(),
            checkpoint_codec_identity: checkpoint.codec.clone(),
            checkpoint_digest: checkpoint.checkpoint_digest.clone(),
        },
    };
    manifest.validate()?;

    let state_players: BTreeSet<_> = checkpoint.state.core.players.keys().copied().collect();
    let manifest_players: BTreeSet<_> = manifest.decks.iter().map(|deck| deck.player).collect();
    if state_players.len() != 2
        || manifest.decks.len() != 2
        || state_players != manifest_players
        || state_players.len() != manifest.decks.len()
    {
        return Err(ControllerError::ReplayIdentityMismatch);
    }
    Ok(manifest)
}
