use std::collections::BTreeSet;

use mtgml_model::{CheckpointDigestV3, EpisodeStatus, FullStateDigestV3, PlayerId};
use mtgml_state::{validate_engine_state, EngineState};
use thiserror::Error;

pub use mtgml_model::{CheckpointCodecIdentity, EnvironmentLimitCounters};

pub const ENVIRONMENT_CHECKPOINT_SCHEMA: &str = "environment-checkpoint.v3";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentCheckpointV3 {
    pub schema_version: String,
    pub state: EngineState,
    pub state_digest: FullStateDigestV3,
    pub status: EpisodeStatus,
    pub limit_counters: EnvironmentLimitCounters,
    pub codec: CheckpointCodecIdentity,
    pub checkpoint_digest: CheckpointDigestV3,
}

fn validate_v3_status_for_players(
    status: &EpisodeStatus,
    expected: &BTreeSet<PlayerId>,
) -> Result<(), CheckpointValidationError> {
    let outcomes = match status {
        EpisodeStatus::Running => return Ok(()),
        EpisodeStatus::Terminal { players, .. } | EpisodeStatus::Truncated { players, .. } => {
            players
        }
    };
    if outcomes
        .windows(2)
        .any(|window| window[0].player >= window[1].player)
    {
        return Err(CheckpointValidationError::NoncanonicalStatusOrder);
    }
    let actual: BTreeSet<_> = outcomes.iter().map(|outcome| outcome.player).collect();
    if actual != *expected {
        return Err(CheckpointValidationError::StatusPlayerUniverse);
    }
    Ok(())
}

impl EnvironmentCheckpointV3 {
    pub fn new(
        state: EngineState,
        status: EpisodeStatus,
        limit_counters: EnvironmentLimitCounters,
        codec: CheckpointCodecIdentity,
    ) -> Result<Self, CheckpointValidationError> {
        let state_digest = state
            .digest()
            .map_err(|_| CheckpointValidationError::StateDigest)?;
        limit_counters
            .validate()
            .map_err(|_| CheckpointValidationError::LimitCounters)?;
        let checkpoint_digest =
            calculate_checkpoint_digest(&state_digest, &status, &limit_counters, &codec)?;
        let checkpoint = Self {
            schema_version: ENVIRONMENT_CHECKPOINT_SCHEMA.into(),
            state,
            state_digest,
            status,
            limit_counters,
            codec,
            checkpoint_digest,
        };
        checkpoint.validate()?;
        Ok(checkpoint)
    }

    pub fn validate(&self) -> Result<(), CheckpointValidationError> {
        if self.schema_version != ENVIRONMENT_CHECKPOINT_SCHEMA
            || self.codec.codec_id.is_empty()
            || self.codec.semantic_version.is_empty()
        {
            return Err(CheckpointValidationError::Identity);
        }
        validate_engine_state(&self.state)
            .map_err(|_| CheckpointValidationError::StateInvariant)?;
        self.status
            .validate()
            .map_err(|_| CheckpointValidationError::EpisodeStatus)?;
        let players: BTreeSet<_> = self.state.core.players.keys().copied().collect();
        validate_v3_status_for_players(&self.status, &players)?;
        let state_digest = self
            .state
            .digest()
            .map_err(|_| CheckpointValidationError::StateDigest)?;
        if state_digest != self.state_digest {
            return Err(CheckpointValidationError::StateDigest);
        }
        self.limit_counters
            .validate()
            .map_err(|_| CheckpointValidationError::LimitCounters)?;
        let checkpoint_digest = calculate_checkpoint_digest(
            &self.state_digest,
            &self.status,
            &self.limit_counters,
            &self.codec,
        )?;
        if checkpoint_digest != self.checkpoint_digest {
            return Err(CheckpointValidationError::CheckpointDigest);
        }
        if !matches!(self.status, EpisodeStatus::Running)
            && self.state.execution.pending_decision.is_some()
        {
            return Err(CheckpointValidationError::CompletedWithDecision);
        }
        Ok(())
    }
}

fn calculate_checkpoint_digest(
    state_digest: &FullStateDigestV3,
    status: &EpisodeStatus,
    counters: &EnvironmentLimitCounters,
    codec: &CheckpointCodecIdentity,
) -> Result<CheckpointDigestV3, CheckpointValidationError> {
    mtgml_persistence::checkpoint_digest::calculate_checkpoint_digest_v3(
        &state_digest.as_digest_reference(),
        status,
        counters,
        codec,
    )
    .map_err(|_| CheckpointValidationError::CheckpointDigest)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum CheckpointValidationError {
    #[error("unsupported or empty checkpoint identity")]
    Identity,
    #[error("checkpoint EngineState violates cross-component invariants")]
    StateInvariant,
    #[error("checkpoint full-state digest does not match its state")]
    StateDigest,
    #[error("checkpoint digest does not match status, limits, codec, and state identity")]
    CheckpointDigest,
    #[error("checkpoint episode status is invalid")]
    EpisodeStatus,
    #[error("checkpoint episode status player order is not canonical")]
    NoncanonicalStatusOrder,
    #[error("checkpoint episode status does not cover the authoritative player universe")]
    StatusPlayerUniverse,
    #[error("checkpoint limit counters are inconsistent")]
    LimitCounters,
    #[error("completed checkpoint retains a pending player decision")]
    CompletedWithDecision,
}
