//! Detached Replay V8 contracts. Execution and digest production remain G0h.

use std::collections::BTreeSet;

use mtgml_decision::DecisionResponseV3;
use mtgml_model::{
    execution_program_matches_rules_authority, CheckpointCodecIdentity, CheckpointDigestV8,
    EnvironmentLimitCounters, EpisodeStatus, ExecutionIdentityV1, FullStateDigestV7, PlayerId,
    RulesAuthorityV1, StateRevision,
};
use mtgml_random::types::validate_seed_hex;
use serde::{Deserialize, Serialize};

use crate::{
    identity::{DeckIdentityV1, KernelIdentityV1},
    v2::RandomnessIdentityV2,
    v7::SemanticContractMaterialV7,
    validation::ReplayValidationError,
};

pub const REPLAY_MANIFEST_SCHEMA_V8: &str = "replay-manifest.v8";
pub const REPLAY_FILE_SCHEMA_V8: &str = "authoritative-replay.v8";
pub const REPLAY_STEP_SCHEMA_V8: &str = "replay-step.v8";
pub const CHECKPOINT_CODEC_ID_V8: &str = "in-memory-reference";
pub const CHECKPOINT_CODEC_VERSION_V8: &str = "8";
const SHARED_OBSERVATION_CODEC_V1: &str = "magic-shared-execution-observation.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplaySchemaVersionsV8 {
    pub observation: String,
    pub observation_payload_codec: String,
    pub information_state: String,
    pub decision: String,
    pub decision_response: String,
    pub observed_event: String,
    pub player_step: String,
    pub replay_step: String,
}

impl ReplaySchemaVersionsV8 {
    fn validate(&self) -> Result<(), ReplayValidationError> {
        if self.observation != "observation-envelope.v2"
            || self.observation_payload_codec != SHARED_OBSERVATION_CODEC_V1
            || self.information_state != "information-state-envelope.v3"
            || self.decision != "player-decision-request.v4"
            || self.decision_response != "decision-response.v3"
            || self.observed_event != "observed-event-envelope.v4"
            || self.player_step != "player-step.v4"
            || self.replay_step != REPLAY_STEP_SCHEMA_V8
        {
            return Err(ReplayValidationError::ReplayStepIdentity);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InitialEnvironmentIdentityV8 {
    pub state_revision: StateRevision,
    pub full_state_digest: FullStateDigestV7,
    pub episode_status: EpisodeStatus,
    pub environment_limit_counters: EnvironmentLimitCounters,
    pub checkpoint_codec_identity: CheckpointCodecIdentity,
    pub checkpoint_digest: CheckpointDigestV8,
    pub execution_identity: ExecutionIdentityV1,
}

impl InitialEnvironmentIdentityV8 {
    /// G0c checks the detached identity shape only. Digest recomputation and
    /// checkpoint admission are implemented and proven by G0h.
    pub fn validate(&self) -> Result<(), ReplayValidationError> {
        self.episode_status
            .validate()
            .map_err(|_| ReplayValidationError::CheckpointIdentity)?;
        if self.checkpoint_codec_identity.codec_id != CHECKPOINT_CODEC_ID_V8
            || self.checkpoint_codec_identity.semantic_version != CHECKPOINT_CODEC_VERSION_V8
        {
            return Err(ReplayValidationError::CheckpointIdentity);
        }
        self.environment_limit_counters
            .validate()
            .map_err(|_| ReplayValidationError::CounterProgression)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayManifestV8 {
    pub schema_version: String,
    pub engine_build: String,
    pub kernel: KernelIdentityV1,
    pub rules_snapshot: String,
    pub format_policy_snapshot: String,
    pub oracle_snapshot: String,
    pub card_bundle: String,
    pub schemas: ReplaySchemaVersionsV8,
    pub randomness: RandomnessIdentityV2,
    pub decks: Vec<DeckIdentityV1>,
    pub initial_identity: InitialEnvironmentIdentityV8,
    pub execution_identity: ExecutionIdentityV1,
    pub semantic_contract: SemanticContractMaterialV7,
}

impl ReplayManifestV8 {
    pub fn validate(&self) -> Result<(), ReplayValidationError> {
        if self.schema_version != REPLAY_MANIFEST_SCHEMA_V8 {
            return Err(ReplayValidationError::SchemaVersion);
        }
        self.semantic_contract.validate()?;
        if !execution_program_matches_rules_authority(
            self.execution_identity.program_kind,
            &self.semantic_contract.rules_manifest.rules_authority,
        ) {
            return Err(ReplayValidationError::SemanticContractMismatch);
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
            self.schemas.observation_payload_codec.as_str(),
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
        if self.randomness.contract_id != "mtgml.rng.v1" {
            return Err(ReplayValidationError::UnsupportedRngContract);
        }
        self.schemas.validate()?;
        validate_seed_hex(&self.randomness.root_seed_hex)
            .map_err(|_| ReplayValidationError::Seed)?;
        if self.decks.is_empty() {
            return Err(ReplayValidationError::MissingDecks);
        }
        let mut players = BTreeSet::new();
        let mut previous_player = None;
        for deck in &self.decks {
            if deck.deck_id.is_empty() || !players.insert(deck.player) {
                return Err(ReplayValidationError::DuplicateDeckPlayer);
            }
            if previous_player.is_some_and(|previous| previous >= deck.player) {
                return Err(ReplayValidationError::NoncanonicalKeyOrder);
            }
            previous_player = Some(deck.player);
        }
        validate_status_for_players(&self.initial_identity.episode_status, &players)?;
        if self.execution_identity.semantic_contract_id
            != self.semantic_contract.semantic_contract_id
            || self.initial_identity.execution_identity != self.execution_identity
            || self
                .initial_identity
                .execution_identity
                .semantic_contract_id
                != self.semantic_contract.semantic_contract_id
        {
            return Err(ReplayValidationError::SemanticContractMismatch);
        }
        if let RulesAuthorityV1::ComprehensiveRules { snapshot_id } =
            &self.semantic_contract.rules_manifest.rules_authority
        {
            if self.rules_snapshot != *snapshot_id {
                return Err(ReplayValidationError::RulesSnapshotMismatch);
            }
        }
        self.initial_identity.validate()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayStepV8 {
    pub step_index: u64,
    pub actor: PlayerId,
    pub checkpoint_digest_before: CheckpointDigestV8,
    pub state_revision_before: StateRevision,
    pub response: DecisionResponseV3,
    pub accepted: bool,
    pub state_revision_after: StateRevision,
    pub full_state_digest_after: FullStateDigestV7,
    pub episode_status_after: EpisodeStatus,
    pub environment_limit_counters_after: EnvironmentLimitCounters,
    pub checkpoint_digest_after: CheckpointDigestV8,
}

impl ReplayStepV8 {
    pub fn validate(&self) -> Result<(), ReplayValidationError> {
        self.response
            .validate()
            .map_err(|_| ReplayValidationError::Response)?;
        self.episode_status_after
            .validate()
            .map_err(|_| ReplayValidationError::StatusPlayerUniverse)?;
        self.environment_limit_counters_after
            .validate()
            .map_err(|_| ReplayValidationError::CounterProgression)?;
        if self.accepted && self.state_revision_after.0 <= self.state_revision_before.0 {
            return Err(ReplayValidationError::RevisionDiscontinuity);
        }
        if !self.accepted && self.state_revision_after != self.state_revision_before {
            return Err(ReplayValidationError::RejectedMutation);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoritativeReplayV8 {
    pub schema_version: String,
    pub manifest: ReplayManifestV8,
    pub steps: Vec<ReplayStepV8>,
    pub final_identity: InitialEnvironmentIdentityV8,
}

impl AuthoritativeReplayV8 {
    pub fn validate(&self) -> Result<(), ReplayValidationError> {
        if self.schema_version != REPLAY_FILE_SCHEMA_V8 {
            return Err(ReplayValidationError::SchemaVersion);
        }
        self.manifest.validate()?;
        let initial = &self.manifest.initial_identity;
        if initial.execution_identity != self.manifest.execution_identity
            || initial.execution_identity != self.final_identity.execution_identity
        {
            return Err(ReplayValidationError::SemanticContractMismatch);
        }
        let players: BTreeSet<_> = self.manifest.decks.iter().map(|deck| deck.player).collect();
        let mut previous = initial.clone();
        for (index, step) in self.steps.iter().enumerate() {
            if step.step_index != index as u64
                || !players.contains(&step.actor)
                || step.checkpoint_digest_before != previous.checkpoint_digest
                || step.state_revision_before != previous.state_revision
            {
                return Err(ReplayValidationError::RevisionDiscontinuity);
            }
            step.validate()?;
            step.environment_limit_counters_after
                .validate()
                .map_err(|_| ReplayValidationError::CounterProgression)?;
            if !step.accepted {
                if step.state_revision_after != previous.state_revision
                    || step.full_state_digest_after != previous.full_state_digest
                    || step.episode_status_after != previous.episode_status
                    || step.environment_limit_counters_after != previous.environment_limit_counters
                    || step.checkpoint_digest_after != previous.checkpoint_digest
                {
                    return Err(ReplayValidationError::RejectedMutation);
                }
            } else {
                if step.state_revision_after.0 <= previous.state_revision.0 {
                    return Err(ReplayValidationError::RevisionDiscontinuity);
                }
                validate_accepted_counter_progression(
                    &previous.environment_limit_counters,
                    &step.environment_limit_counters_after,
                )?;
            }
            let next = InitialEnvironmentIdentityV8 {
                state_revision: step.state_revision_after,
                full_state_digest: step.full_state_digest_after.clone(),
                episode_status: step.episode_status_after.clone(),
                environment_limit_counters: step.environment_limit_counters_after.clone(),
                checkpoint_codec_identity: previous.checkpoint_codec_identity.clone(),
                checkpoint_digest: step.checkpoint_digest_after.clone(),
                execution_identity: previous.execution_identity.clone(),
            };
            validate_status_for_players(&next.episode_status, &players)?;
            next.validate()?;
            previous = next;
        }
        if self.final_identity != previous {
            return Err(ReplayValidationError::FinalIdentity);
        }
        Ok(())
    }
}

fn validate_accepted_counter_progression(
    before: &EnvironmentLimitCounters,
    after: &EnvironmentLimitCounters,
) -> Result<(), ReplayValidationError> {
    let submitted = before
        .decisions_submitted
        .checked_add(1)
        .ok_or(ReplayValidationError::CounterProgression)?;
    let accepted = before
        .accepted_transitions
        .checked_add(1)
        .ok_or(ReplayValidationError::CounterProgression)?;
    if after.decisions_submitted != submitted
        || after.accepted_transitions != accepted
        || after.rule_events_emitted < before.rule_events_emitted
        || after.resource_units_consumed < before.resource_units_consumed
        || after.wall_clock_elapsed_millis < before.wall_clock_elapsed_millis
    {
        return Err(ReplayValidationError::CounterProgression);
    }
    Ok(())
}

fn validate_status_for_players(
    status: &EpisodeStatus,
    expected: &BTreeSet<PlayerId>,
) -> Result<(), ReplayValidationError> {
    let outcomes = match status {
        EpisodeStatus::Running => return Ok(()),
        EpisodeStatus::Terminal { players, .. } | EpisodeStatus::Truncated { players, .. } => {
            players
        }
    };
    status
        .validate()
        .map_err(|_| ReplayValidationError::StatusPlayerUniverse)?;
    if outcomes
        .windows(2)
        .any(|window| window[0].player >= window[1].player)
        || outcomes
            .iter()
            .map(|outcome| outcome.player)
            .collect::<BTreeSet<_>>()
            != *expected
    {
        return Err(ReplayValidationError::StatusPlayerUniverse);
    }
    Ok(())
}

// The V8 recorder is intentionally only an identity-bearing detached owner in G0c.
// G0h owns append/export behavior and digest recomputation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayRecorderV8 {
    pub manifest: ReplayManifestV8,
    pub steps: Vec<ReplayStepV8>,
    pub final_identity: InitialEnvironmentIdentityV8,
}
