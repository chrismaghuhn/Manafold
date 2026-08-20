use mtgml_decision::DecisionResponse;
use mtgml_model::{FullStateDigestV2, StateRevision};
use mtgml_random::types::validate_seed_hex;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeSet;

use crate::identity::{DeckIdentityV1, KernelIdentityV1, ReplaySchemaVersionsV1};
use crate::validation::ReplayValidationError;

pub const REPLAY_MANIFEST_SCHEMA_V2: &str = "replay-manifest.v2";
pub const REPLAY_FILE_SCHEMA_V2: &str = "authoritative-replay.v2";

fn deserialize_root_seed_hex<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    validate_seed_hex(&s).map_err(|_| {
        serde::de::Error::custom("root seed is not canonical lowercase hexadecimal")
    })?;
    Ok(s)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RandomnessIdentityV2 {
    pub contract_id: String,
    #[serde(deserialize_with = "deserialize_root_seed_hex")]
    pub root_seed_hex: String,
}

/// Exact normative object described by `schemas/replay-manifest.v2.schema.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayManifestV2 {
    pub schema_version: String,
    pub engine_build: String,
    pub kernel: KernelIdentityV1,
    pub rules_snapshot: String,
    pub format_policy_snapshot: String,
    pub oracle_snapshot: String,
    pub card_bundle: String,
    pub schemas: ReplaySchemaVersionsV1,
    pub randomness: RandomnessIdentityV2,
    pub decks: Vec<DeckIdentityV1>,
    pub initial_state_revision: StateRevision,
    pub initial_state_digest: FullStateDigestV2,
}

impl ReplayManifestV2 {
    pub fn validate(&self) -> Result<(), ReplayValidationError> {
        if self.schema_version != REPLAY_MANIFEST_SCHEMA_V2 {
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
            self.randomness.contract_id.as_str(),
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
        if self.randomness.contract_id != "mtgml.rng.v1" {
            return Err(ReplayValidationError::UnsupportedRngContract);
        }
        if self.schemas.replay_step != "replay-step.v2" {
            return Err(ReplayValidationError::ReplayStepIdentity);
        }
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

/// Exact normative object described by `schemas/authoritative-replay.v2.schema.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoritativeReplayV2 {
    pub schema_version: String,
    pub manifest: ReplayManifestV2,
    pub steps: Vec<ReplayStepV2>,
    pub final_state_revision: StateRevision,
    pub final_state_digest: FullStateDigestV2,
}

impl AuthoritativeReplayV2 {
    pub fn validate(&self) -> Result<(), ReplayValidationError> {
        if self.schema_version != REPLAY_FILE_SCHEMA_V2 {
            return Err(ReplayValidationError::SchemaVersion);
        }
        self.manifest.validate()?;
        let mut revision = self.manifest.initial_state_revision;
        let mut state_digest = self.manifest.initial_state_digest.clone();
        for (index, step) in self.steps.iter().enumerate() {
            if step.step_index != index as u64
                || step.state_revision_before != revision
                || step.response.state_revision != revision
            {
                return Err(ReplayValidationError::RevisionDiscontinuity);
            }
            step.response
                .validate()
                .map_err(|_| ReplayValidationError::Response)?;
            if step.accepted {
                if step.state_revision_after.0 <= step.state_revision_before.0 {
                    return Err(ReplayValidationError::RevisionDiscontinuity);
                }
            } else if step.state_revision_after != step.state_revision_before
                || step.state_digest_after != state_digest
            {
                return Err(ReplayValidationError::RejectedMutation);
            }
            revision = step.state_revision_after;
            state_digest = step.state_digest_after.clone();
        }
        if self.final_state_revision != revision {
            return Err(ReplayValidationError::FinalIdentity);
        }
        if let Some(last) = self.steps.last() {
            if self.final_state_digest != last.state_digest_after {
                return Err(ReplayValidationError::FinalIdentity);
            }
        } else if self.final_state_digest != self.manifest.initial_state_digest
            || self.final_state_revision != self.manifest.initial_state_revision
        {
            return Err(ReplayValidationError::EmptyReplayIdentity);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayStepV2 {
    pub step_index: u64,
    pub state_revision_before: StateRevision,
    pub response: DecisionResponse,
    pub accepted: bool,
    pub state_revision_after: StateRevision,
    pub state_digest_after: FullStateDigestV2,
}
