use std::collections::BTreeSet;

use mtgml_model::{
    CheckpointDigestV4, CheckpointDigestV5, EpisodeStatus, ExecutionIdentityV1, FullStateDigestV4,
    PlayerId,
};
use mtgml_state::{validate_engine_state, EngineState};
use thiserror::Error;

pub use mtgml_model::{CheckpointCodecIdentity, EnvironmentLimitCounters};

pub const ENVIRONMENT_CHECKPOINT_SCHEMA: &str = "environment-checkpoint.v4";
pub const CHECKPOINT_CODEC_ID_V4: &str = "in-memory-reference";
pub const CHECKPOINT_CODEC_SEMANTIC_VERSION_V4: &str = "4";

// === V5 (spec §11; ADR 0055 §2.7/§2.8) — beside the retained V4 surface ===
pub const ENVIRONMENT_CHECKPOINT_SCHEMA_V5: &str = "environment-checkpoint.v5";
pub const CHECKPOINT_CODEC_ID_V5: &str = "in-memory-reference";
pub const CHECKPOINT_CODEC_SEMANTIC_VERSION_V5: &str = "5";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentCheckpointV4 {
    pub schema_version: String,
    pub state: EngineState,
    pub state_digest: FullStateDigestV4,
    pub status: EpisodeStatus,
    pub limit_counters: EnvironmentLimitCounters,
    pub codec: CheckpointCodecIdentity,
    pub checkpoint_digest: CheckpointDigestV4,
}

fn validate_v4_status_for_players(
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

impl EnvironmentCheckpointV4 {
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
            || self.codec.codec_id != CHECKPOINT_CODEC_ID_V4
            || self.codec.semantic_version != CHECKPOINT_CODEC_SEMANTIC_VERSION_V4
        {
            return Err(CheckpointValidationError::Identity);
        }
        validate_engine_state(&self.state)
            .map_err(|_| CheckpointValidationError::StateInvariant)?;
        self.status
            .validate()
            .map_err(|_| CheckpointValidationError::EpisodeStatus)?;
        let players: BTreeSet<_> = self.state.core.players.keys().copied().collect();
        validate_v4_status_for_players(&self.status, &players)?;
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
    state_digest: &FullStateDigestV4,
    status: &EpisodeStatus,
    counters: &EnvironmentLimitCounters,
    codec: &CheckpointCodecIdentity,
) -> Result<CheckpointDigestV4, CheckpointValidationError> {
    mtgml_persistence::checkpoint_digest::calculate_checkpoint_digest_v4(
        &state_digest.as_digest_reference(),
        status,
        counters,
        codec,
    )
    .map_err(|_| CheckpointValidationError::CheckpointDigest)
}

fn calculate_checkpoint_digest_v5(
    state_digest: &FullStateDigestV4,
    status: &EpisodeStatus,
    counters: &EnvironmentLimitCounters,
    codec: &CheckpointCodecIdentity,
    execution_identity: &ExecutionIdentityV1,
) -> Result<CheckpointDigestV5, CheckpointValidationError> {
    mtgml_persistence::checkpoint_digest::calculate_checkpoint_digest_v5(
        &state_digest.as_digest_reference(),
        status,
        counters,
        codec,
        execution_identity,
    )
    .map_err(|_| CheckpointValidationError::CheckpointDigest)
}

/// V5 environment checkpoint (spec §11): the UNCHANGED V4 `EngineState` and
/// `FullStateDigestV4` surfaces plus the two NEW identity-bound fields.
/// `ExecutionIdentityV1` is the full identity struct — the dispatch tag
/// alone is never sufficient — and `CheckpointDigestV5` is computed with the
/// identity as its final input, so identity and digest cannot diverge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentCheckpointV5 {
    pub schema_version: String,
    pub state: EngineState,
    pub state_digest: FullStateDigestV4,
    pub status: EpisodeStatus,
    pub limit_counters: EnvironmentLimitCounters,
    pub codec: CheckpointCodecIdentity,
    pub execution_identity: ExecutionIdentityV1,
    pub checkpoint_digest: CheckpointDigestV5,
}

impl EnvironmentCheckpointV5 {
    /// Mirrors V4: compute the state digest, compute the checkpoint digest
    /// via `calculate_checkpoint_digest_v5` with the identity as the final
    /// input, then `validate()`.
    pub fn new(
        state: EngineState,
        status: EpisodeStatus,
        limit_counters: EnvironmentLimitCounters,
        codec: CheckpointCodecIdentity,
        execution_identity: ExecutionIdentityV1,
    ) -> Result<Self, CheckpointValidationError> {
        let state_digest = state
            .digest()
            .map_err(|_| CheckpointValidationError::StateDigest)?;
        limit_counters
            .validate()
            .map_err(|_| CheckpointValidationError::LimitCounters)?;
        let checkpoint_digest = calculate_checkpoint_digest_v5(
            &state_digest,
            &status,
            &limit_counters,
            &codec,
            &execution_identity,
        )?;
        let checkpoint = Self {
            schema_version: ENVIRONMENT_CHECKPOINT_SCHEMA_V5.into(),
            state,
            state_digest,
            status,
            limit_counters,
            codec,
            execution_identity,
            checkpoint_digest,
        };
        checkpoint.validate()?;
        Ok(checkpoint)
    }

    /// Validation obligations (spec §11): schema/codec identity equality
    /// (codec `"5"`); `validate_engine_state(&state)`; status + canonical
    /// status/player-universe checks; state digest recompute and equality;
    /// limit counters validate; checkpoint digest recompute FROM THE STORED
    /// `execution_identity` and equality; completed-checkpoint/pending-
    /// decision rule. NO child manifests are embedded.
    pub fn validate(&self) -> Result<(), CheckpointValidationError> {
        if self.schema_version != ENVIRONMENT_CHECKPOINT_SCHEMA_V5
            || self.codec.codec_id != CHECKPOINT_CODEC_ID_V5
            || self.codec.semantic_version != CHECKPOINT_CODEC_SEMANTIC_VERSION_V5
        {
            return Err(CheckpointValidationError::Identity);
        }
        validate_engine_state(&self.state)
            .map_err(|_| CheckpointValidationError::StateInvariant)?;
        self.status
            .validate()
            .map_err(|_| CheckpointValidationError::EpisodeStatus)?;
        let players: BTreeSet<_> = self.state.core.players.keys().copied().collect();
        validate_v4_status_for_players(&self.status, &players)?;
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
        let checkpoint_digest = calculate_checkpoint_digest_v5(
            &self.state_digest,
            &self.status,
            &self.limit_counters,
            &self.codec,
            &self.execution_identity,
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
