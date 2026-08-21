use std::collections::BTreeSet;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use mtgml_decision::DecisionResponse;
use mtgml_model::{EpisodeStatus, InformationStateDigest, ObservationDigest, PlayerId};
use mtgml_observation::{
    InformationStateEnvelope, ObservationEnvelope, PlayerStep, INFORMATION_STATE_SCHEMA,
    OBSERVATION_SCHEMA, PLAYER_STEP_SCHEMA,
};
use mtgml_random::RootSeed256;
use mtgml_replay::{
    AuthoritativeReplayV2, DeckIdentityV1, KernelIdentityV1, RandomnessIdentityV2,
    ReplayManifestV2, ReplayRecorderV2, ReplaySchemaVersionsV1, ReplayStepV2,
    REPLAY_MANIFEST_SCHEMA_V2,
};
use mtgml_rules::{
    validate_transition_contract, RulesKernel, SyntheticM1RulesKernel, TransitionResult,
};
use mtgml_state::{construct_synthetic_engine_state, EngineState, SyntheticResetInputs};

use crate::checkpoint::{
    CheckpointCodecIdentity, EnvironmentCheckpointV2, EnvironmentLimitCounters,
};
use crate::controller::EnvironmentBackend;
use crate::endpoint::PlayerApiError;
use crate::errors::{ControllerError, EnvironmentCommitError};

const SYNTHETIC_M1_OBSERVATION_CODEC: &str = "synthetic-m1-observation.v1";
const SYNTHETIC_M1_INFORMATION_DIGEST_INPUT: &str = "synthetic-m1-information-state.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntheticM1EnvironmentConfig {
    pub codec: CheckpointCodecIdentity,
    pub replay: SyntheticM1ReplayConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntheticM1ReplayConfig {
    pub engine_build: String,
    pub kernel: KernelIdentityV1,
    pub rules_snapshot: String,
    pub format_policy_snapshot: String,
    pub oracle_snapshot: String,
    pub card_bundle: String,
    pub randomness_contract_id: String,
    pub schemas: ReplaySchemaVersionsV1,
    pub decks: Vec<DeckIdentityV1>,
}

pub struct SyntheticM1EnvironmentBackend {
    state: EngineState,
    status: EpisodeStatus,
    limit_counters: EnvironmentLimitCounters,
    codec: CheckpointCodecIdentity,
    config: SyntheticM1EnvironmentConfig,
    replay: ReplayRecorderV2,
    kernel: SyntheticM1RulesKernel,
}

impl SyntheticM1EnvironmentBackend {
    pub fn new(
        players: [PlayerId; 2],
        root_seed: RootSeed256,
        config: SyntheticM1EnvironmentConfig,
    ) -> Result<Self, ControllerError> {
        let state = construct_synthetic_engine_state(SyntheticResetInputs { players, root_seed })?;
        let status = EpisodeStatus::Running;
        let limit_counters = EnvironmentLimitCounters::default();
        let checkpoint = EnvironmentCheckpointV2::new(
            state.clone(),
            status.clone(),
            limit_counters.clone(),
            config.codec.clone(),
        )?;
        let replay = ReplayRecorderV2::new(build_manifest(&config, &checkpoint)?)?;
        Ok(Self {
            state,
            status,
            limit_counters,
            codec: config.codec.clone(),
            config,
            replay,
            kernel: SyntheticM1RulesKernel,
        })
    }

    pub fn from_checkpoint(
        checkpoint: EnvironmentCheckpointV2,
        config: SyntheticM1EnvironmentConfig,
    ) -> Result<Self, ControllerError> {
        checkpoint.validate()?;
        if checkpoint.codec != config.codec {
            return Err(ControllerError::UnsupportedCheckpointCodec);
        }
        let replay = ReplayRecorderV2::new(build_manifest(&config, &checkpoint)?)?;
        Ok(Self {
            state: checkpoint.state,
            status: checkpoint.status,
            limit_counters: checkpoint.limit_counters,
            codec: checkpoint.codec,
            config,
            replay,
            kernel: SyntheticM1RulesKernel,
        })
    }

    fn current_checkpoint(&self) -> Result<EnvironmentCheckpointV2, ControllerError> {
        Ok(EnvironmentCheckpointV2::new(
            self.state.clone(),
            self.status.clone(),
            self.limit_counters.clone(),
            self.codec.clone(),
        )?)
    }

    fn checked_add_counter(
        value: u64,
        increment: u64,
        counter: &'static str,
    ) -> Result<u64, ControllerError> {
        value
            .checked_add(increment)
            .ok_or(ControllerError::CounterOverflow { counter })
    }

    fn candidate_counters(
        before: &EnvironmentLimitCounters,
        event_count: usize,
    ) -> Result<EnvironmentLimitCounters, ControllerError> {
        let event_count =
            u64::try_from(event_count).map_err(|_| ControllerError::CounterOverflow {
                counter: "rule_events_emitted",
            })?;
        Ok(EnvironmentLimitCounters {
            decisions_submitted: Self::checked_add_counter(
                before.decisions_submitted,
                1,
                "decisions_submitted",
            )?,
            accepted_transitions: Self::checked_add_counter(
                before.accepted_transitions,
                1,
                "accepted_transitions",
            )?,
            rule_events_emitted: Self::checked_add_counter(
                before.rule_events_emitted,
                event_count,
                "rule_events_emitted",
            )?,
            resource_units_consumed: before.resource_units_consumed,
            wall_clock_elapsed_millis: before.wall_clock_elapsed_millis,
        })
    }

    fn execute_response(
        &mut self,
        actor: PlayerId,
        response: DecisionResponse,
    ) -> Result<TransitionResult, ControllerError> {
        let before = self.current_checkpoint()?;
        let transition = self.kernel.apply(&before.state, actor, &response)?;
        validate_transition_contract(&before.state, &transition)?;

        if !transition.accepted {
            let after = self.current_checkpoint()?;
            if after != before {
                return Err(EnvironmentCommitError::RejectedMutation.into());
            }
            return Ok(transition);
        }

        let candidate_counters =
            Self::candidate_counters(&before.limit_counters, transition.events.len())?;
        let candidate = EnvironmentCheckpointV2::new(
            transition.next_state.clone(),
            transition.status.clone(),
            candidate_counters,
            before.codec.clone(),
        )?;
        if candidate.state != transition.next_state || candidate.status != transition.status {
            return Err(EnvironmentCommitError::CandidateMismatch.into());
        }

        let step_index = u64::try_from(self.replay.step_count()).map_err(|_| {
            ControllerError::CounterOverflow {
                counter: "replay_step_index",
            }
        })?;
        let step = ReplayStepV2 {
            step_index,
            state_revision_before: before.state.revision,
            response,
            accepted: true,
            state_revision_after: candidate.state.revision,
            state_digest_after: candidate.state_digest.clone(),
        };
        let mut candidate_replay = self.replay.clone();
        candidate_replay.append(step)?;
        candidate_replay.export()?;

        self.state = candidate.state;
        self.status = candidate.status;
        self.limit_counters = candidate.limit_counters;
        self.replay = candidate_replay;
        Ok(transition)
    }

    fn require_player(&self, perspective: PlayerId) -> Result<(), PlayerApiError> {
        self.state
            .core
            .players
            .contains_key(&perspective)
            .then_some(())
            .ok_or(PlayerApiError::Unavailable)
    }

    fn synthetic_observation(
        perspective: PlayerId,
        revision: mtgml_model::StateRevision,
    ) -> Result<ObservationEnvelope, PlayerApiError> {
        let payload = format!(
            "{SYNTHETIC_M1_OBSERVATION_CODEC}|perspective={}|state-revision={}",
            perspective.0, revision.0
        )
        .into_bytes();
        let observation = ObservationEnvelope {
            schema_version: OBSERVATION_SCHEMA.into(),
            perspective,
            state_revision: revision,
            payload_codec: SYNTHETIC_M1_OBSERVATION_CODEC.into(),
            payload_base64: STANDARD.encode(&payload),
            digest: ObservationDigest::from_canonical_bytes(&payload),
        };
        observation
            .validate()
            .map_err(|_| PlayerApiError::Unavailable)?;
        Ok(observation)
    }
}

impl EnvironmentBackend for SyntheticM1EnvironmentBackend {
    fn players(&self) -> Vec<PlayerId> {
        self.state.core.players.keys().copied().collect()
    }

    fn checkpoint(&self) -> Result<EnvironmentCheckpointV2, ControllerError> {
        self.current_checkpoint()
    }

    fn restore(&mut self, checkpoint: EnvironmentCheckpointV2) -> Result<(), ControllerError> {
        let candidate = Self::from_checkpoint(checkpoint, self.config.clone())?;
        self.state = candidate.state;
        self.status = candidate.status;
        self.limit_counters = candidate.limit_counters;
        self.codec = candidate.codec;
        self.replay = candidate.replay;
        self.kernel = SyntheticM1RulesKernel;
        Ok(())
    }

    fn fork_boxed(&self) -> Result<Box<dyn EnvironmentBackend>, ControllerError> {
        let checkpoint = self.current_checkpoint()?;
        Ok(Box::new(Self::from_checkpoint(
            checkpoint,
            self.config.clone(),
        )?))
    }

    fn export_replay(&self) -> Result<AuthoritativeReplayV2, ControllerError> {
        Ok(self.replay.export()?)
    }

    fn execute_trusted_response(
        &mut self,
        actor: PlayerId,
        response: DecisionResponse,
    ) -> Result<TransitionResult, ControllerError> {
        self.execute_response(actor, response)
    }

    fn player_observation(
        &self,
        perspective: PlayerId,
    ) -> Result<ObservationEnvelope, PlayerApiError> {
        self.require_player(perspective)?;
        Self::synthetic_observation(perspective, self.state.revision)
    }

    fn player_information_state(
        &self,
        perspective: PlayerId,
    ) -> Result<InformationStateEnvelope, PlayerApiError> {
        self.require_player(perspective)?;
        let current_observation = self.player_observation(perspective)?;
        let knowledge = self
            .state
            .knowledge
            .players
            .get(&perspective)
            .ok_or(PlayerApiError::Unavailable)?;
        let digest_input = format!(
            "{SYNTHETIC_M1_INFORMATION_DIGEST_INPUT}|perspective={}|state-revision={}|public-history-length={}|private-history-length={}|observation-payload={}",
            perspective.0,
            self.state.revision.0,
            knowledge.public_history_length,
            knowledge.private_history_length,
            current_observation.payload_base64,
        );
        let information_state = InformationStateEnvelope {
            schema_version: INFORMATION_STATE_SCHEMA.into(),
            perspective,
            state_revision: self.state.revision,
            current_observation,
            public_history_length: knowledge.public_history_length,
            private_history_length: knowledge.private_history_length,
            digest: InformationStateDigest::from_canonical_bytes(digest_input.as_bytes()),
        };
        information_state
            .validate()
            .map_err(|_| PlayerApiError::Unavailable)?;
        Ok(information_state)
    }

    fn player_visible_decision(
        &self,
        perspective: PlayerId,
    ) -> Result<Option<mtgml_decision::PlayerDecisionRequest>, PlayerApiError> {
        self.require_player(perspective)?;
        let Some(pending) = self.state.execution.pending_decision.as_ref() else {
            return Ok(None);
        };
        if pending.request.actor != perspective {
            return Ok(None);
        }
        pending
            .request
            .validate()
            .map_err(|_| PlayerApiError::Unavailable)?;
        Ok(Some(pending.request.clone()))
    }

    fn submit_player_response(
        &mut self,
        perspective: PlayerId,
        response: DecisionResponse,
    ) -> Result<PlayerStep, PlayerApiError> {
        self.require_player(perspective)?;
        if !matches!(&self.status, EpisodeStatus::Running) {
            return Err(PlayerApiError::EpisodeComplete);
        }
        let Some(pending) = self.state.execution.pending_decision.as_ref() else {
            return Err(PlayerApiError::NoVisibleDecision);
        };
        if pending.request.actor != perspective {
            return Err(PlayerApiError::NoVisibleDecision);
        }
        if response.validate().is_err() {
            return Err(PlayerApiError::InvalidSelection);
        }
        if response.decision_id != pending.request.decision_id
            || response.state_revision != pending.request.state_revision
            || response.state_revision != self.state.revision
        {
            return Err(PlayerApiError::StaleResponse);
        }

        let transition = self
            .execute_response(perspective, response)
            .map_err(|_| PlayerApiError::Unavailable)?;
        if !transition.accepted {
            return Err(PlayerApiError::InvalidSelection);
        }

        let step = PlayerStep {
            schema_version: PLAYER_STEP_SCHEMA.into(),
            information_state: self.player_information_state(perspective)?,
            observed_events: vec![],
            next_decision: None,
            status: transition.status,
        };
        step.validate().map_err(|_| PlayerApiError::Unavailable)?;
        Ok(step)
    }
}

fn build_manifest(
    config: &SyntheticM1EnvironmentConfig,
    checkpoint: &EnvironmentCheckpointV2,
) -> Result<ReplayManifestV2, ControllerError> {
    let manifest = ReplayManifestV2 {
        schema_version: REPLAY_MANIFEST_SCHEMA_V2.into(),
        engine_build: config.replay.engine_build.clone(),
        kernel: config.replay.kernel.clone(),
        rules_snapshot: config.replay.rules_snapshot.clone(),
        format_policy_snapshot: config.replay.format_policy_snapshot.clone(),
        oracle_snapshot: config.replay.oracle_snapshot.clone(),
        card_bundle: config.replay.card_bundle.clone(),
        schemas: config.replay.schemas.clone(),
        randomness: RandomnessIdentityV2 {
            contract_id: config.replay.randomness_contract_id.clone(),
            root_seed_hex: checkpoint.state.random.root_seed.to_lower_hex(),
        },
        decks: config.replay.decks.clone(),
        initial_state_revision: checkpoint.state.revision,
        initial_state_digest: checkpoint.state_digest.clone(),
    };
    manifest.validate()?;

    let state_players: BTreeSet<_> = checkpoint.state.core.players.keys().copied().collect();
    let manifest_players: BTreeSet<_> = manifest.decks.iter().map(|deck| deck.player).collect();
    if state_players.len() != 2
        || manifest.decks.len() != 2
        || state_players != manifest_players
        || state_players.len() != manifest.decks.len()
    {
        return Err(ControllerError::ReplayIdentityMismatch);
    }
    Ok(manifest)
}
