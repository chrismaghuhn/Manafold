//! Ownership: synthetic replay manifest identity assembly only. The
//! environment transaction itself does NOT live here.

use std::collections::BTreeSet;

use mtgml_replay::{
    InitialEnvironmentIdentityV3, RandomnessIdentityV2, ReplayManifestV3, REPLAY_MANIFEST_SCHEMA_V3,
};

use super::SyntheticM1EnvironmentConfig;
use crate::checkpoint::EnvironmentCheckpointV3;
use crate::errors::ControllerError;

pub(super) fn build_manifest(
    config: &SyntheticM1EnvironmentConfig,
    checkpoint: &EnvironmentCheckpointV3,
) -> Result<ReplayManifestV3, ControllerError> {
    let manifest = ReplayManifestV3 {
        schema_version: REPLAY_MANIFEST_SCHEMA_V3.into(),
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
        decks: config.replay.decks.clone(),
        initial_identity: InitialEnvironmentIdentityV3 {
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
