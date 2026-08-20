use mtgml_model::{ContentDigest, FullStateDigest, PlayerId, StateRevision};
use serde::{Deserialize, Serialize};

use crate::manifest::ReplayManifestV1;
use crate::validation::ReplayValidationError;

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
            schema_version: crate::REPLAY_MANIFEST_SCHEMA.into(),
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
