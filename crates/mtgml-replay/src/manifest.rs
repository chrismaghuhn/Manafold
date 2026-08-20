use mtgml_model::{FullStateDigest, StateRevision};
use mtgml_random::types::validate_seed_hex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use crate::identity::{
    DeckIdentityV1, KernelIdentityV1, RandomnessIdentityV1, ReplaySchemaVersionsV1,
};
use crate::validation::ReplayValidationError;

pub const REPLAY_MANIFEST_SCHEMA: &str = "replay-manifest.v1";

/// Exact normative object described by `schemas/replay-manifest.v1.schema.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayManifestV1 {
    pub schema_version: String,
    pub engine_build: String,
    pub kernel: KernelIdentityV1,
    pub rules_snapshot: String,
    pub format_policy_snapshot: String,
    pub oracle_snapshot: String,
    pub card_bundle: String,
    pub schemas: ReplaySchemaVersionsV1,
    pub randomness: RandomnessIdentityV1,
    pub decks: Vec<DeckIdentityV1>,
    pub initial_state_revision: StateRevision,
    pub initial_state_digest: FullStateDigest,
}

impl ReplayManifestV1 {
    pub fn validate(&self) -> Result<(), ReplayValidationError> {
        if self.schema_version != REPLAY_MANIFEST_SCHEMA {
            return Err(ReplayValidationError::SchemaVersion);
        }
        let required = [
            self.engine_build.as_str(),
            self.kernel.implementation_id.as_str(),
            self.kernel.semantic_version.as_str(),
            self.kernel.build_profile.as_str(),
            self.rules_snapshot.as_str(),
            self.format_policy_snapshot.as_str(),
            self.oracle_snapshot.as_str(),
            self.card_bundle.as_str(),
            self.randomness.algorithm_id.as_str(),
            self.randomness.derivation_version.as_str(),
            self.schemas.observation.as_str(),
            self.schemas.information_state.as_str(),
            self.schemas.decision.as_str(),
            self.schemas.decision_response.as_str(),
            self.schemas.observed_event.as_str(),
            self.schemas.player_step.as_str(),
            self.schemas.replay_step.as_str(),
        ];
        if required.iter().any(|value| value.is_empty()) {
            return Err(ReplayValidationError::EmptyIdentity);
        }
        validate_seed_hex(&self.randomness.root_seed_hex)
            .map_err(|_| ReplayValidationError::Seed)?;
        if self.decks.is_empty() {
            return Err(ReplayValidationError::MissingDecks);
        }
        let mut players = BTreeSet::new();
        if self
            .decks
            .iter()
            .any(|deck| deck.deck_id.is_empty() || !players.insert(deck.player))
        {
            return Err(ReplayValidationError::DuplicateDeckPlayer);
        }
        Ok(())
    }
}
