use mtgml_model::{CheckpointDigestV2, EpisodeStatus, FullStateDigestV2};
use mtgml_state::{validate_engine_state, EngineState};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const ENVIRONMENT_CHECKPOINT_SCHEMA: &str = "environment-checkpoint.v2";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentLimitCounters {
    pub decisions_submitted: u64,
    pub accepted_transitions: u64,
    pub rule_events_emitted: u64,
    pub resource_units_consumed: u64,
    pub wall_clock_elapsed_millis: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckpointCodecIdentity {
    pub codec_id: String,
    pub semantic_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentCheckpointV2 {
    pub schema_version: String,
    pub state: EngineState,
    pub state_digest: FullStateDigestV2,
    pub status: EpisodeStatus,
    pub limit_counters: EnvironmentLimitCounters,
    pub codec: CheckpointCodecIdentity,
    pub checkpoint_digest: CheckpointDigestV2,
}

#[derive(Serialize)]
pub(crate) struct CheckpointDigestInputV2<'a> {
    pub schema_version: &'a str,
    pub domain: &'static str,
    pub state_digest: &'a FullStateDigestV2,
    pub status: &'a EpisodeStatus,
    pub limit_counters: &'a EnvironmentLimitCounters,
    pub codec: &'a CheckpointCodecIdentity,
}

impl EnvironmentCheckpointV2 {
    pub fn new(
        state: EngineState,
        status: EpisodeStatus,
        limit_counters: EnvironmentLimitCounters,
        codec: CheckpointCodecIdentity,
    ) -> Result<Self, CheckpointValidationError> {
        let state_digest = state
            .digest()
            .map_err(|_| CheckpointValidationError::StateDigest)?;
        let schema_version = ENVIRONMENT_CHECKPOINT_SCHEMA.to_owned();
        let checkpoint_digest = Self::calculate_digest(
            &schema_version,
            &state_digest,
            &status,
            &limit_counters,
            &codec,
        )?;
        let checkpoint = Self {
            schema_version,
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

    fn calculate_digest(
        schema_version: &str,
        state_digest: &FullStateDigestV2,
        status: &EpisodeStatus,
        limit_counters: &EnvironmentLimitCounters,
        codec: &CheckpointCodecIdentity,
    ) -> Result<CheckpointDigestV2, CheckpointValidationError> {
        let input = CheckpointDigestInputV2 {
            schema_version,
            domain: CheckpointDigestV2::DOMAIN,
            state_digest,
            status,
            limit_counters,
            codec,
        };
        let bytes =
            serde_json::to_vec(&input).map_err(|_| CheckpointValidationError::CheckpointDigest)?;
        Ok(CheckpointDigestV2::from_canonical_bytes(&bytes))
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
        let digest = self
            .state
            .digest()
            .map_err(|_| CheckpointValidationError::StateDigest)?;
        if digest != self.state_digest {
            return Err(CheckpointValidationError::StateDigest);
        }
        let checkpoint_digest = Self::calculate_digest(
            &self.schema_version,
            &self.state_digest,
            &self.status,
            &self.limit_counters,
            &self.codec,
        )?;
        if checkpoint_digest != self.checkpoint_digest {
            return Err(CheckpointValidationError::CheckpointDigest);
        }
        self.status
            .validate()
            .map_err(|_| CheckpointValidationError::EpisodeStatus)?;
        if self.limit_counters.accepted_transitions > self.limit_counters.decisions_submitted {
            return Err(CheckpointValidationError::LimitCounters);
        }
        if !matches!(&self.status, EpisodeStatus::Running)
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
    #[error("checkpoint limit counters are inconsistent")]
    LimitCounters,
    #[error("completed checkpoint retains a pending player decision")]
    CompletedWithDecision,
}
