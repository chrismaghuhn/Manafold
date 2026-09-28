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
    /// Validates the identity shape and recomputes its V8 checkpoint digest.
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
            .map_err(|_| ReplayValidationError::CounterProgression)?;
        let actual = mtgml_persistence::checkpoint_digest::calculate_checkpoint_digest_v8(
            &self.full_state_digest.as_digest_reference(),
            &self.episode_status,
            &self.environment_limit_counters,
            &self.checkpoint_codec_identity,
            &self.execution_identity,
        )
        .map_err(|_| ReplayValidationError::CheckpointIdentity)?;
        if actual != self.checkpoint_digest {
            return Err(ReplayValidationError::CheckpointIdentity);
        }
        Ok(())
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
            previous = validate_replay_step_transition(&previous, step, index as u64, &players)?;
        }
        if self.final_identity != previous {
            return Err(ReplayValidationError::FinalIdentity);
        }
        Ok(())
    }
}

fn validate_replay_step_transition(
    previous: &InitialEnvironmentIdentityV8,
    step: &ReplayStepV8,
    expected_index: u64,
    players: &BTreeSet<PlayerId>,
) -> Result<InitialEnvironmentIdentityV8, ReplayValidationError> {
    if !matches!(previous.episode_status, EpisodeStatus::Running) {
        return Err(ReplayValidationError::TransitionAfterEpisodeClosed);
    }
    if step.step_index != expected_index
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
    validate_status_for_players(&next.episode_status, players)?;
    next.validate()?;
    Ok(next)
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayRecorderV8 {
    manifest: ReplayManifestV8,
    steps: Vec<ReplayStepV8>,
    final_identity: InitialEnvironmentIdentityV8,
}

impl ReplayRecorderV8 {
    pub fn new(manifest: ReplayManifestV8) -> Result<Self, ReplayValidationError> {
        manifest.validate()?;
        Ok(Self {
            final_identity: manifest.initial_identity.clone(),
            manifest,
            steps: Vec::new(),
        })
    }

    /// Appends a structurally valid authoritative replay product. This
    /// detached recorder does not execute the response; G0j owns the single
    /// current runtime integration that supplies such products.
    pub fn append(&mut self, step: ReplayStepV8) -> Result<(), ReplayValidationError> {
        let final_identity = self.validate_next(&step)?;
        self.steps.push(step);
        self.final_identity = final_identity;
        Ok(())
    }

    /// Validates one appended step against the recorder's already validated
    /// prefix without rescanning or cloning that prefix.
    fn validate_next(
        &self,
        step: &ReplayStepV8,
    ) -> Result<InitialEnvironmentIdentityV8, ReplayValidationError> {
        let expected_index =
            u64::try_from(self.steps.len()).map_err(|_| ReplayValidationError::FinalIdentity)?;
        let players = self.manifest.decks.iter().map(|deck| deck.player).collect();
        validate_replay_step_transition(&self.final_identity, step, expected_index, &players)
    }

    pub fn export(&self) -> Result<AuthoritativeReplayV8, ReplayValidationError> {
        let replay = AuthoritativeReplayV8 {
            schema_version: REPLAY_FILE_SCHEMA_V8.to_owned(),
            manifest: self.manifest.clone(),
            steps: self.steps.clone(),
            final_identity: self.final_identity.clone(),
        };
        replay.validate()?;
        Ok(replay)
    }

    pub fn manifest(&self) -> &ReplayManifestV8 {
        &self.manifest
    }

    pub fn step_count(&self) -> usize {
        self.steps.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> ReplayManifestV8 {
        let manifest: ReplayManifestV8 = serde_json::from_str(include_str!(
            "../../../schemas/examples/replay-manifest-v8.json"
        ))
        .unwrap();
        manifest
    }

    #[test]
    fn replay_v8_recorder_roundtrips_empty_and_rejected_steps() {
        let manifest = manifest();
        manifest.validate().unwrap();
        assert_eq!(
            manifest.initial_identity.checkpoint_digest.to_string(),
            "d11105b35123d4da63d810049a27c8d3ca8f5d6beb389378033c2b3b726b4870"
        );
        let mut recorder = ReplayRecorderV8::new(manifest.clone()).unwrap();
        assert_eq!(recorder.export().unwrap().manifest, manifest);
        assert_eq!(recorder.step_count(), 0);

        let mut step: ReplayStepV8 = serde_json::from_str(include_str!(
            "../../../schemas/examples/replay-step-v8.json"
        ))
        .unwrap();
        step.checkpoint_digest_before = manifest.initial_identity.checkpoint_digest.clone();
        step.checkpoint_digest_after = manifest.initial_identity.checkpoint_digest.clone();
        step.full_state_digest_after = manifest.initial_identity.full_state_digest.clone();
        step.episode_status_after = manifest.initial_identity.episode_status.clone();
        step.environment_limit_counters_after =
            manifest.initial_identity.environment_limit_counters.clone();
        step.state_revision_before = manifest.initial_identity.state_revision;
        step.state_revision_after = manifest.initial_identity.state_revision;
        step.accepted = false;
        recorder.append(step.clone()).unwrap();
        let replay = recorder.export().unwrap();
        assert_eq!(replay.steps, vec![step]);
        assert_eq!(replay.final_identity, manifest.initial_identity);
        assert_eq!(recorder.step_count(), 1);
    }

    #[test]
    fn replay_v8_rejected_append_is_total_nonmutation() {
        let mut recorder = ReplayRecorderV8::new(manifest()).unwrap();
        let before = recorder.clone();
        let mut step: ReplayStepV8 = serde_json::from_str(include_str!(
            "../../../schemas/examples/replay-step-v8.json"
        ))
        .unwrap();
        step.step_index = 1;
        assert!(recorder.append(step).is_err());
        assert_eq!(recorder, before);
    }

    #[test]
    fn replay_v8_recorder_accepts_complete_successor_identity_progression() {
        let mut recorder = ReplayRecorderV8::new(manifest()).unwrap();
        let mut step: ReplayStepV8 = serde_json::from_str(include_str!(
            "../../../schemas/examples/replay-step-v8.json"
        ))
        .unwrap();
        let before = recorder.manifest().initial_identity.clone();
        let mut counters = before.environment_limit_counters.clone();
        counters.decisions_submitted = 1;
        counters.accepted_transitions = 1;
        let after_digest = FullStateDigestV7::from_digest_bytes([0x79; 32]);
        let checkpoint_digest =
            mtgml_persistence::checkpoint_digest::calculate_checkpoint_digest_v8(
                &after_digest.as_digest_reference(),
                &before.episode_status,
                &counters,
                &before.checkpoint_codec_identity,
                &before.execution_identity,
            )
            .unwrap();
        step.checkpoint_digest_before = before.checkpoint_digest.clone();
        step.state_revision_before = before.state_revision;
        step.state_revision_after = StateRevision(before.state_revision.0 + 1);
        step.full_state_digest_after = after_digest.clone();
        step.episode_status_after = before.episode_status.clone();
        step.environment_limit_counters_after = counters;
        step.checkpoint_digest_after = checkpoint_digest;
        step.accepted = true;

        recorder.append(step.clone()).unwrap();
        let replay = recorder.export().unwrap();
        assert_eq!(replay.steps, vec![step]);
        assert_eq!(
            replay.final_identity.state_revision.0,
            before.state_revision.0 + 1
        );
        assert_eq!(replay.final_identity.full_state_digest, after_digest);
    }

    #[test]
    fn replay_v8_rejects_transitions_after_terminal_status() {
        let mut recorder = ReplayRecorderV8::new(manifest()).unwrap();
        let before = recorder.manifest().initial_identity.clone();
        let players = recorder
            .manifest()
            .decks
            .iter()
            .map(|deck| deck.player)
            .collect::<Vec<_>>();
        let terminal = EpisodeStatus::Terminal {
            reason: mtgml_model::TerminalReason::Concession,
            players: players
                .iter()
                .map(|player| mtgml_model::PlayerOutcome {
                    player: *player,
                    result: mtgml_model::PlayerResult::Win,
                })
                .collect(),
        };
        let mut counters = before.environment_limit_counters.clone();
        counters.decisions_submitted += 1;
        counters.accepted_transitions += 1;
        let after_digest = FullStateDigestV7::from_digest_bytes([0x4a; 32]);
        let terminal_checkpoint_digest =
            mtgml_persistence::checkpoint_digest::calculate_checkpoint_digest_v8(
                &after_digest.as_digest_reference(),
                &terminal,
                &counters,
                &before.checkpoint_codec_identity,
                &before.execution_identity,
            )
            .unwrap();
        let mut terminal_step: ReplayStepV8 = serde_json::from_str(include_str!(
            "../../../schemas/examples/replay-step-v8.json"
        ))
        .unwrap();
        terminal_step.checkpoint_digest_before = before.checkpoint_digest.clone();
        terminal_step.state_revision_before = before.state_revision;
        terminal_step.state_revision_after = StateRevision(before.state_revision.0 + 1);
        terminal_step.full_state_digest_after = after_digest.clone();
        terminal_step.episode_status_after = terminal.clone();
        terminal_step.environment_limit_counters_after = counters.clone();
        terminal_step.checkpoint_digest_after = terminal_checkpoint_digest.clone();
        terminal_step.accepted = true;
        recorder.append(terminal_step).unwrap();

        let terminal_identity = recorder.export().unwrap().final_identity;
        let mut after_close: ReplayStepV8 = serde_json::from_str(include_str!(
            "../../../schemas/examples/replay-step-v8.json"
        ))
        .unwrap();
        after_close.step_index = 1;
        after_close.checkpoint_digest_before = terminal_identity.checkpoint_digest.clone();
        after_close.state_revision_before = terminal_identity.state_revision;
        after_close.state_revision_after = terminal_identity.state_revision;
        after_close.full_state_digest_after = terminal_identity.full_state_digest.clone();
        after_close.episode_status_after = terminal_identity.episode_status.clone();
        after_close.environment_limit_counters_after =
            terminal_identity.environment_limit_counters.clone();
        after_close.checkpoint_digest_after = terminal_identity.checkpoint_digest.clone();
        after_close.accepted = false;

        let recorder_before = recorder.clone();
        assert_eq!(
            recorder.append(after_close.clone()),
            Err(ReplayValidationError::TransitionAfterEpisodeClosed)
        );
        assert_eq!(recorder, recorder_before);

        let mut malformed_replay = recorder.export().unwrap();
        malformed_replay.steps.push(after_close);
        assert_eq!(
            malformed_replay.validate(),
            Err(ReplayValidationError::TransitionAfterEpisodeClosed)
        );
    }

    #[test]
    fn replay_v8_identity_rejects_wrong_checkpoint_digest() {
        let mut manifest = manifest();
        manifest.initial_identity.checkpoint_digest =
            CheckpointDigestV8::from_digest_bytes([0; 32]);
        assert_eq!(
            manifest.initial_identity.validate(),
            Err(ReplayValidationError::CheckpointIdentity)
        );
    }

    #[test]
    fn replay_v8_example_fixture_is_semantically_valid() {
        let replay: AuthoritativeReplayV8 = serde_json::from_str(include_str!(
            "../../../schemas/examples/authoritative-replay-v8.json"
        ))
        .unwrap();
        replay.validate().unwrap();
    }
}
