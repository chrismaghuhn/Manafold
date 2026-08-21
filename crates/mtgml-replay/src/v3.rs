use std::collections::BTreeSet;

use mtgml_decision::DecisionResponseV2;
use mtgml_model::{
    CheckpointCodecIdentity, CheckpointDigestV3, ContentDigest, EnvironmentLimitCounters,
    EpisodeStatus, FullStateDigestV3, PlayerId, StateRevision,
};
use mtgml_random::types::validate_seed_hex;
use serde::{Deserialize, Serialize};

use crate::identity::{DeckIdentityV1, KernelIdentityV1, ReplaySchemaVersionsV1};
use crate::v2::RandomnessIdentityV2;
use crate::validation::ReplayValidationError;

pub const REPLAY_MANIFEST_SCHEMA_V3: &str = "replay-manifest.v3";
pub const REPLAY_FILE_SCHEMA_V3: &str = "authoritative-replay.v3";
pub const REPLAY_STEP_SCHEMA_V3: &str = "replay-step.v3";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InitialEnvironmentIdentityV3 {
    pub state_revision: StateRevision,
    pub full_state_digest: FullStateDigestV3,
    pub episode_status: EpisodeStatus,
    pub environment_limit_counters: EnvironmentLimitCounters,
    pub checkpoint_codec_identity: CheckpointCodecIdentity,
    pub checkpoint_digest: CheckpointDigestV3,
}

impl InitialEnvironmentIdentityV3 {
    pub fn validate(&self) -> Result<(), ReplayValidationError> {
        self.episode_status
            .validate()
            .map_err(|_| ReplayValidationError::CheckpointIdentity)?;
        if self.checkpoint_codec_identity.codec_id.is_empty()
            || self.checkpoint_codec_identity.semantic_version.is_empty()
        {
            return Err(ReplayValidationError::EmptyIdentity);
        }
        if self.checkpoint_digest != calculate_checkpoint_digest(self)? {
            return Err(ReplayValidationError::CheckpointIdentity);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayManifestV3 {
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
    pub initial_identity: InitialEnvironmentIdentityV3,
}

impl ReplayManifestV3 {
    pub fn validate(&self) -> Result<(), ReplayValidationError> {
        if self.schema_version != REPLAY_MANIFEST_SCHEMA_V3 {
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
        if self.randomness.contract_id != "mtgml.rng.v1"
            || self.schemas.decision != "player-decision-request.v2"
            || self.schemas.decision_response != "decision-response.v2"
            || self.schemas.information_state != "information-state-envelope.v2"
            || self.schemas.observed_event != "observed-event-envelope.v2"
            || self.schemas.player_step != "player-step.v2"
            || self.schemas.replay_step != REPLAY_STEP_SCHEMA_V3
        {
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
        self.initial_identity.validate()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayStepV3 {
    pub step_index: u64,
    pub actor: PlayerId,
    pub checkpoint_digest_before: CheckpointDigestV3,
    pub state_revision_before: StateRevision,
    pub response: DecisionResponseV2,
    pub accepted: bool,
    pub state_revision_after: StateRevision,
    pub full_state_digest_after: FullStateDigestV3,
    pub episode_status_after: EpisodeStatus,
    pub environment_limit_counters_after: EnvironmentLimitCounters,
    pub checkpoint_digest_after: CheckpointDigestV3,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoritativeReplayV3 {
    pub schema_version: String,
    pub manifest: ReplayManifestV3,
    pub steps: Vec<ReplayStepV3>,
    pub final_identity: InitialEnvironmentIdentityV3,
}

impl AuthoritativeReplayV3 {
    pub fn validate(&self) -> Result<(), ReplayValidationError> {
        if self.schema_version != REPLAY_FILE_SCHEMA_V3 {
            return Err(ReplayValidationError::SchemaVersion);
        }
        self.manifest.validate()?;
        let mut previous = self.manifest.initial_identity.clone();
        for (index, step) in self.steps.iter().enumerate() {
            if step.step_index != index as u64
                || step.checkpoint_digest_before != previous.checkpoint_digest
                || step.state_revision_before != previous.state_revision
                || step.response.state_revision != previous.state_revision
                || step.actor.0 == 0
            {
                return Err(ReplayValidationError::RevisionDiscontinuity);
            }
            step.response
                .validate()
                .map_err(|_| ReplayValidationError::Response)?;
            if !step.accepted {
                if step.state_revision_after != previous.state_revision
                    || step.full_state_digest_after != previous.full_state_digest
                    || step.episode_status_after != previous.episode_status
                    || step.environment_limit_counters_after != previous.environment_limit_counters
                    || step.checkpoint_digest_after != previous.checkpoint_digest
                {
                    return Err(ReplayValidationError::RejectedMutation);
                }
            } else if step.state_revision_after.0 <= previous.state_revision.0 {
                return Err(ReplayValidationError::RevisionDiscontinuity);
            }
            let next = InitialEnvironmentIdentityV3 {
                state_revision: step.state_revision_after,
                full_state_digest: step.full_state_digest_after.clone(),
                episode_status: step.episode_status_after.clone(),
                environment_limit_counters: step.environment_limit_counters_after.clone(),
                checkpoint_codec_identity: previous.checkpoint_codec_identity.clone(),
                checkpoint_digest: step.checkpoint_digest_after.clone(),
            };
            next.validate()?;
            previous = next;
        }
        if self.final_identity != previous {
            return Err(ReplayValidationError::FinalIdentity);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayRecorderV3 {
    manifest: ReplayManifestV3,
    steps: Vec<ReplayStepV3>,
    final_identity: InitialEnvironmentIdentityV3,
}

impl ReplayRecorderV3 {
    pub fn new(manifest: ReplayManifestV3) -> Result<Self, ReplayValidationError> {
        manifest.validate()?;
        Ok(Self {
            final_identity: manifest.initial_identity.clone(),
            manifest,
            steps: Vec::new(),
        })
    }

    pub fn append(&mut self, step: ReplayStepV3) -> Result<(), ReplayValidationError> {
        let mut steps = self.steps.clone();
        steps.push(step);
        let candidate = AuthoritativeReplayV3 {
            schema_version: REPLAY_FILE_SCHEMA_V3.into(),
            manifest: self.manifest.clone(),
            steps,
            final_identity: self.final_identity.clone(),
        };
        // The final identity is set from the proposed step before validation.
        let step = candidate.steps.last().expect("just appended");
        let final_identity = InitialEnvironmentIdentityV3 {
            state_revision: step.state_revision_after,
            full_state_digest: step.full_state_digest_after.clone(),
            episode_status: step.episode_status_after.clone(),
            environment_limit_counters: step.environment_limit_counters_after.clone(),
            checkpoint_codec_identity: self.final_identity.checkpoint_codec_identity.clone(),
            checkpoint_digest: step.checkpoint_digest_after.clone(),
        };
        let candidate = AuthoritativeReplayV3 {
            final_identity,
            ..candidate
        };
        candidate.validate()?;
        self.steps = candidate.steps;
        self.final_identity = candidate.final_identity;
        Ok(())
    }

    pub fn export(&self) -> Result<AuthoritativeReplayV3, ReplayValidationError> {
        let replay = AuthoritativeReplayV3 {
            schema_version: REPLAY_FILE_SCHEMA_V3.into(),
            manifest: self.manifest.clone(),
            steps: self.steps.clone(),
            final_identity: self.final_identity.clone(),
        };
        replay.validate()?;
        Ok(replay)
    }

    pub fn manifest(&self) -> &ReplayManifestV3 {
        &self.manifest
    }

    pub fn step_count(&self) -> usize {
        self.steps.len()
    }
}

fn calculate_checkpoint_digest(
    identity: &InitialEnvironmentIdentityV3,
) -> Result<CheckpointDigestV3, ReplayValidationError> {
    let reference = identity.full_state_digest.as_digest_reference();
    mtgml_persistence::checkpoint_digest::calculate_checkpoint_digest_v3(
        &reference,
        &identity.episode_status,
        &identity.environment_limit_counters,
        &identity.checkpoint_codec_identity,
    )
    .map_err(|_| ReplayValidationError::CheckpointIdentity)
}

#[allow(dead_code)]
fn _digest_type_marker(_: &ContentDigest) {}
