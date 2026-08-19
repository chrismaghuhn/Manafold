//! Normative replay wire contracts and internal replay identity.

use mtgml_decision::DecisionResponse;
use mtgml_model::{ContentDigest, FullStateDigest, FullStateDigestV2, PlayerId, StateRevision};
use mtgml_random::types::validate_seed_hex;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeSet;
use thiserror::Error;

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

pub const REPLAY_MANIFEST_SCHEMA: &str = "replay-manifest.v1";
pub const REPLAY_FILE_SCHEMA: &str = "authoritative-replay.v1";
pub const REPLAY_MANIFEST_SCHEMA_V2: &str = "replay-manifest.v2";
pub const REPLAY_FILE_SCHEMA_V2: &str = "authoritative-replay.v2";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KernelIdentityV1 {
    pub implementation_id: String,
    pub semantic_version: String,
    pub build_profile: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplaySchemaVersionsV1 {
    pub observation: String,
    pub information_state: String,
    pub decision: String,
    pub decision_response: String,
    pub observed_event: String,
    pub player_step: String,
    pub replay_step: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RandomnessIdentityV1 {
    pub algorithm_id: String,
    pub derivation_version: String,
    pub root_seed_hex: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeckIdentityV1 {
    pub player: PlayerId,
    pub deck_id: String,
    pub digest: ContentDigest,
}

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

/// Internal domain identity. It deliberately carries every field required to
/// construct the normative wire manifest; no hidden enrichment step is allowed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayIdentity {
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

impl TryFrom<&ReplayIdentity> for ReplayManifestV1 {
    type Error = ReplayValidationError;

    fn try_from(identity: &ReplayIdentity) -> Result<Self, Self::Error> {
        let manifest = Self {
            schema_version: REPLAY_MANIFEST_SCHEMA.into(),
            engine_build: identity.engine_build.clone(),
            kernel: identity.kernel.clone(),
            rules_snapshot: identity.rules_snapshot.clone(),
            format_policy_snapshot: identity.format_policy_snapshot.clone(),
            oracle_snapshot: identity.oracle_snapshot.clone(),
            card_bundle: identity.card_bundle.clone(),
            schemas: identity.schemas.clone(),
            randomness: identity.randomness.clone(),
            decks: identity.decks.clone(),
            initial_state_revision: identity.initial_state_revision,
            initial_state_digest: identity.initial_state_digest.clone(),
        };
        manifest.validate()?;
        Ok(manifest)
    }
}

impl TryFrom<ReplayManifestV1> for ReplayIdentity {
    type Error = ReplayValidationError;

    fn try_from(manifest: ReplayManifestV1) -> Result<Self, Self::Error> {
        manifest.validate()?;
        Ok(Self {
            engine_build: manifest.engine_build,
            kernel: manifest.kernel,
            rules_snapshot: manifest.rules_snapshot,
            format_policy_snapshot: manifest.format_policy_snapshot,
            oracle_snapshot: manifest.oracle_snapshot,
            card_bundle: manifest.card_bundle,
            schemas: manifest.schemas,
            randomness: manifest.randomness,
            decks: manifest.decks,
            initial_state_revision: manifest.initial_state_revision,
            initial_state_digest: manifest.initial_state_digest,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayStepV1 {
    pub step_index: u64,
    pub state_revision_before: StateRevision,
    pub response: DecisionResponse,
    pub accepted: bool,
    pub state_revision_after: StateRevision,
    pub state_digest_after: FullStateDigest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoritativeReplayV1 {
    pub schema_version: String,
    pub manifest: ReplayManifestV1,
    pub steps: Vec<ReplayStepV1>,
    pub final_state_revision: StateRevision,
    pub final_state_digest: FullStateDigest,
}

impl AuthoritativeReplayV1 {
    pub fn validate(&self) -> Result<(), ReplayValidationError> {
        if self.schema_version != REPLAY_FILE_SCHEMA {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ReplayValidationError {
    #[error("unsupported replay schema version")]
    SchemaVersion,
    #[error("replay identity fields must be non-empty")]
    EmptyIdentity,
    #[error("root seed is not canonical lowercase hexadecimal")]
    Seed,
    #[error("replay manifest must identify at least one deck")]
    MissingDecks,
    #[error("each player must have exactly one deck identity")]
    DuplicateDeckPlayer,
    #[error("replay revisions are not contiguous")]
    RevisionDiscontinuity,
    #[error("rejected response mutated the authoritative revision or full-state identity")]
    RejectedMutation,
    #[error("decision response is invalid")]
    Response,
    #[error("final replay identity does not match its steps")]
    FinalIdentity,
    #[error("an empty replay must end at its initial identity")]
    EmptyReplayIdentity,
    #[error("unsupported RNG contract in replay")]
    UnsupportedRngContract,
    #[error("replay-step schema identity must be replay-step.v2")]
    ReplayStepIdentity,
}

// === V2 replay types ===

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

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(text: char) -> FullStateDigest {
        FullStateDigest::parse(text.to_string().repeat(64)).unwrap()
    }

    fn manifest() -> ReplayManifestV1 {
        ReplayManifestV1 {
            schema_version: REPLAY_MANIFEST_SCHEMA.into(),
            engine_build: "build".into(),
            kernel: KernelIdentityV1 {
                implementation_id: "reference".into(),
                semantic_version: "0.2.2".into(),
                build_profile: "test".into(),
            },
            rules_snapshot: "rules".into(),
            format_policy_snapshot: "format".into(),
            oracle_snapshot: "oracle".into(),
            card_bundle: "bundle".into(),
            schemas: ReplaySchemaVersionsV1 {
                observation: "observation-envelope.v1".into(),
                information_state: "information-state-envelope.v1".into(),
                decision: "player-decision-request.v1".into(),
                decision_response: "decision-response.v1".into(),
                observed_event: "observed-event-envelope.v1".into(),
                player_step: "player-step.v1".into(),
                replay_step: "replay-step.v1".into(),
            },
            randomness: RandomnessIdentityV1 {
                algorithm_id: "counter".into(),
                derivation_version: "v1".into(),
                root_seed_hex: "00".repeat(32),
            },
            decks: vec![DeckIdentityV1 {
                player: PlayerId(1),
                deck_id: "deck".into(),
                digest: ContentDigest::parse("11".repeat(32)).unwrap(),
            }],
            initial_state_revision: StateRevision(0),
            initial_state_digest: digest('0'),
        }
    }

    #[test]
    fn replay_schema_version_fields_must_all_be_non_empty() {
        let mut invalid = manifest();
        invalid.schemas.observed_event.clear();
        assert_eq!(
            invalid.validate(),
            Err(ReplayValidationError::EmptyIdentity)
        );
    }

    #[test]
    fn rejected_replay_step_must_preserve_the_full_state_digest() {
        let mut replay = AuthoritativeReplayV1 {
            schema_version: REPLAY_FILE_SCHEMA.into(),
            manifest: manifest(),
            steps: vec![ReplayStepV1 {
                step_index: 0,
                state_revision_before: StateRevision(0),
                response: DecisionResponse {
                    schema_version: "decision-response.v1".into(),
                    decision_id: mtgml_model::DecisionId(1),
                    state_revision: StateRevision(0),
                    assignments: vec![],
                },
                accepted: false,
                state_revision_after: StateRevision(0),
                state_digest_after: digest('1'),
            }],
            final_state_revision: StateRevision(0),
            final_state_digest: digest('1'),
        };
        assert_eq!(
            replay.validate(),
            Err(ReplayValidationError::RejectedMutation)
        );
        replay.steps[0].state_digest_after = digest('0');
        replay.final_state_digest = digest('0');
        replay.validate().unwrap();
    }
}
