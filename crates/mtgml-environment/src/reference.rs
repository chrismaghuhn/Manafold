//! Shared reference-environment mechanics and the durable MagicRules backend.
//!
//! This module owns environment transaction mechanics that are independent of
//! rules meaning. The rules product is always supplied by `ProgramKernelV1`;
//! this module only validates, projects, checkpoints, rebases replay, and
//! commits the complete candidate atomically.

use mtgml_decision::{DecisionResponseV2, PlayerDecisionRequestV2};
use mtgml_model::{
    CheckpointCodecIdentity, ContentDigest, EnvironmentLimitCounters, EpisodeStatus,
    ExecutionIdentityV1, ExecutionProgramV1, PlayerId,
};
use mtgml_observation::{
    ObservationEnvelope, PlayerInformationStateV2, PlayerStepSubmissionV1, PlayerStepV2,
    INFORMATION_STATE_SCHEMA_V2, OBSERVATION_SCHEMA, OBSERVED_EVENT_SCHEMA_V2,
    PLAYER_STEP_SCHEMA_V2,
};
use mtgml_random::MTGML_RNG_V1;
use mtgml_replay::{
    AuthoritativeReplayV5, DeckIdentityV1, InitialEnvironmentIdentityV5, KernelIdentityV1,
    RandomnessIdentityV2, ReplayManifestV5, ReplayRecorderV5, ReplaySchemaVersionsV5,
    SemanticContractMaterialV5, REPLAY_MANIFEST_SCHEMA_V5, REPLAY_STEP_SCHEMA_V5,
};
use mtgml_rules::{validate_transition_contract, ProgramKernelV1, TransitionResult};
use mtgml_state::EngineState;

use crate::checkpoint::{EnvironmentCheckpointV5, EnvironmentLimitCounters as CheckpointCounters};
use crate::controller::EnvironmentBackend;
use crate::endpoint::PlayerEndpointError;
use crate::errors::{ControllerError, EnvironmentCommitError};
use crate::semantic_catalog::{admit_restore, RuntimeSemanticCatalog};
use crate::semantic_catalog_generated::{
    magic_turn_structure_0_1_0_rules_manifest, magic_turn_structure_0_1_0_semantic_contract_id,
    magic_turn_structure_0_1_0_semantic_manifest,
};

pub const REFERENCE_SCENARIO_ID: &str = "rules/turn-structure@0.1.0:non-card-reference";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceEnvironmentReplayConfig {
    pub scenario_id: String,
    pub engine_build: String,
    pub kernel: KernelIdentityV1,
    pub rules_snapshot: String,
    pub format_policy_snapshot: String,
    pub oracle_snapshot: String,
    pub schemas: ReplaySchemaVersionsV5,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceEnvironmentConfig {
    pub state: EngineState,
    pub status: EpisodeStatus,
    pub limit_counters: EnvironmentLimitCounters,
    pub codec: CheckpointCodecIdentity,
    pub execution_identity: ExecutionIdentityV1,
    pub replay: ReferenceEnvironmentReplayConfig,
}

pub struct ReferenceEnvironmentBackend {
    state: EngineState,
    status: EpisodeStatus,
    limit_counters: EnvironmentLimitCounters,
    codec: CheckpointCodecIdentity,
    execution_identity: ExecutionIdentityV1,
    replay_config: ReferenceEnvironmentReplayConfig,
    replay: ReplayRecorderV5,
    kernel: ProgramKernelV1,
}

fn magic_execution_identity() -> ExecutionIdentityV1 {
    ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::MagicRules,
        semantic_contract_id: magic_turn_structure_0_1_0_semantic_contract_id(),
    }
}

fn current_v5_schema_versions() -> ReplaySchemaVersionsV5 {
    ReplaySchemaVersionsV5 {
        observation: OBSERVATION_SCHEMA.into(),
        observation_payload_codec: "synthetic-m3-observation.v1".into(),
        information_state: INFORMATION_STATE_SCHEMA_V2.into(),
        decision: mtgml_decision::PLAYER_DECISION_REQUEST_V2_SCHEMA.into(),
        decision_response: mtgml_decision::DECISION_RESPONSE_V2_SCHEMA.into(),
        observed_event: OBSERVED_EVENT_SCHEMA_V2.into(),
        player_step: PLAYER_STEP_SCHEMA_V2.into(),
        replay_step: REPLAY_STEP_SCHEMA_V5.into(),
    }
}

fn validate_reference_replay_config(
    config: &ReferenceEnvironmentReplayConfig,
) -> Result<(), ControllerError> {
    if config.scenario_id.is_empty()
        || config.rules_snapshot.is_empty()
        || config.schemas != current_v5_schema_versions()
    {
        return Err(ControllerError::ReplayIdentityMismatch);
    }
    Ok(())
}

pub(crate) fn build_reference_manifest(
    config: &ReferenceEnvironmentReplayConfig,
    checkpoint: &EnvironmentCheckpointV5,
) -> Result<ReplayManifestV5, ControllerError> {
    validate_reference_replay_config(config)?;
    let players: Vec<_> = checkpoint.state.core.players.keys().copied().collect();
    if players.len() != 2 {
        return Err(ControllerError::ProgramStateIncompatible);
    }

    // V5 requires participant/deck-shaped provenance fields. These values are
    // explicit non-card scenario markers, never deck or card-bundle support
    // claims, and are derived deterministically from the scenario ID/player.
    let decks = players
        .iter()
        .map(|player| {
            let marker = format!("scenario:{}:participant:{}", config.scenario_id, player.0);
            DeckIdentityV1 {
                player: *player,
                deck_id: format!("not-a-deck:{marker}"),
                digest: ContentDigest::from_canonical_bytes(marker.as_bytes()),
            }
        })
        .collect::<Vec<_>>();
    let manifest = ReplayManifestV5 {
        schema_version: REPLAY_MANIFEST_SCHEMA_V5.into(),
        engine_build: config.engine_build.clone(),
        kernel: config.kernel.clone(),
        rules_snapshot: config.rules_snapshot.clone(),
        format_policy_snapshot: config.format_policy_snapshot.clone(),
        oracle_snapshot: config.oracle_snapshot.clone(),
        card_bundle: format!(
            "not-a-card-bundle:{scenario}",
            scenario = config.scenario_id
        ),
        schemas: config.schemas.clone(),
        randomness: RandomnessIdentityV2 {
            contract_id: MTGML_RNG_V1.into(),
            root_seed_hex: checkpoint.state.random.root_seed.to_lower_hex(),
        },
        decks,
        initial_identity: identity_from_checkpoint(checkpoint),
        execution_identity: checkpoint.execution_identity.clone(),
        semantic_contract: SemanticContractMaterialV5 {
            semantic_contract_id: magic_turn_structure_0_1_0_semantic_contract_id(),
            manifest: magic_turn_structure_0_1_0_semantic_manifest(),
            rules_manifest: magic_turn_structure_0_1_0_rules_manifest(),
        },
    };
    manifest.validate()?;
    Ok(manifest)
}

fn identity_from_checkpoint(checkpoint: &EnvironmentCheckpointV5) -> InitialEnvironmentIdentityV5 {
    InitialEnvironmentIdentityV5 {
        state_revision: checkpoint.state.revision,
        full_state_digest: checkpoint.state_digest.clone(),
        episode_status: checkpoint.status.clone(),
        environment_limit_counters: checkpoint.limit_counters.clone(),
        checkpoint_codec_identity: checkpoint.codec.clone(),
        checkpoint_digest: checkpoint.checkpoint_digest.clone(),
        execution_identity: checkpoint.execution_identity.clone(),
    }
}

pub(crate) fn current_checkpoint(
    state: &EngineState,
    status: &EpisodeStatus,
    limit_counters: &EnvironmentLimitCounters,
    codec: &CheckpointCodecIdentity,
    execution_identity: &ExecutionIdentityV1,
) -> Result<EnvironmentCheckpointV5, ControllerError> {
    Ok(EnvironmentCheckpointV5::new(
        state.clone(),
        status.clone(),
        limit_counters.clone(),
        codec.clone(),
        execution_identity.clone(),
    )?)
}

// Player-safe products are shared across reference and compatibility backends.

pub(crate) struct ReferenceEnvironmentTransaction<'a> {
    pub(crate) state: &'a mut EngineState,
    pub(crate) status: &'a mut EpisodeStatus,
    pub(crate) limit_counters: &'a mut EnvironmentLimitCounters,
    pub(crate) codec: &'a CheckpointCodecIdentity,
    pub(crate) execution_identity: &'a ExecutionIdentityV1,
    pub(crate) replay: &'a mut ReplayRecorderV5,
    pub(crate) kernel: &'a mut ProgramKernelV1,
}

pub(crate) fn execute_forced_progress_transaction<F>(
    transaction: &mut ReferenceEnvironmentTransaction<'_>,
    build_manifest: F,
) -> Result<TransitionResult, ControllerError>
where
    F: FnOnce(&EnvironmentCheckpointV5) -> Result<ReplayManifestV5, ControllerError>,
{
    let state = &mut *transaction.state;
    let status = &mut *transaction.status;
    let limit_counters = &mut *transaction.limit_counters;
    let codec = transaction.codec;
    let execution_identity = transaction.execution_identity;
    let replay = &mut *transaction.replay;
    let kernel = &mut *transaction.kernel;
    let before = current_checkpoint(state, status, limit_counters, codec, execution_identity)?;
    let transition = match kernel.advance_forced_progress(&before.state) {
        Ok(transition) => transition,
        Err(error) => {
            let after =
                current_checkpoint(state, status, limit_counters, codec, execution_identity)?;
            if after != before {
                return Err(EnvironmentCommitError::RejectedMutation.into());
            }
            return Err(error.into());
        }
    };
    validate_transition_contract(&before.state, &transition)?;
    crate::lifecycle_projection::project_occurrence_envelopes(
        &before.state,
        &transition.next_state,
        &transition.events,
    )
    .map_err(|_| {
        ControllerError::EnvironmentCommit(EnvironmentCommitError::PlayerProjectionInvalid)
    })?;
    crate::player_projection::validate_candidate_projections(&transition.next_state)?;

    let event_count =
        u64::try_from(transition.events.len()).map_err(|_| ControllerError::CounterOverflow {
            counter: "rule_events_emitted",
        })?;
    let candidate_counters = CheckpointCounters {
        decisions_submitted: before.limit_counters.decisions_submitted,
        accepted_transitions: before.limit_counters.accepted_transitions,
        rule_events_emitted: before
            .limit_counters
            .rule_events_emitted
            .checked_add(event_count)
            .ok_or(ControllerError::CounterOverflow {
                counter: "rule_events_emitted",
            })?,
        resource_units_consumed: before.limit_counters.resource_units_consumed,
        wall_clock_elapsed_millis: before.limit_counters.wall_clock_elapsed_millis,
    };
    if candidate_counters.decisions_submitted != before.limit_counters.decisions_submitted
        || candidate_counters.accepted_transitions != before.limit_counters.accepted_transitions
    {
        return Err(ControllerError::Backend(
            "forced progress changed player-submission counters".into(),
        ));
    }
    let candidate = EnvironmentCheckpointV5::new(
        transition.next_state.clone(),
        transition.status.clone(),
        candidate_counters,
        before.codec.clone(),
        before.execution_identity.clone(),
    )?;
    if candidate.state != transition.next_state || candidate.status != transition.status {
        return Err(EnvironmentCommitError::CandidateMismatch.into());
    }
    if replay.step_count() != 0 {
        return Err(ControllerError::Backend(
            "forced progress cannot rebase a non-empty replay history".into(),
        ));
    }
    let rebased_replay = ReplayRecorderV5::new(build_manifest(&candidate)?)?;

    *state = candidate.state;
    *status = candidate.status;
    *limit_counters = candidate.limit_counters;
    *replay = rebased_replay;
    Ok(transition)
}

impl ReferenceEnvironmentBackend {
    pub fn new(config: ReferenceEnvironmentConfig) -> Result<Self, ControllerError> {
        let expected_identity = magic_execution_identity();
        if config.execution_identity != expected_identity {
            return Err(ControllerError::ProgramAuthorityMismatch);
        }
        let checkpoint = EnvironmentCheckpointV5::new(
            config.state,
            config.status,
            config.limit_counters,
            config.codec.clone(),
            config.execution_identity,
        )?;
        admit_restore(&RuntimeSemanticCatalog::production(), &checkpoint)?;
        Self::from_admitted_checkpoint(checkpoint, config.codec, config.replay)
    }

    fn from_admitted_checkpoint(
        checkpoint: EnvironmentCheckpointV5,
        codec: CheckpointCodecIdentity,
        replay_config: ReferenceEnvironmentReplayConfig,
    ) -> Result<Self, ControllerError> {
        checkpoint.validate()?;
        if checkpoint.codec != codec {
            return Err(ControllerError::UnsupportedCheckpointCodec);
        }
        if checkpoint.execution_identity != magic_execution_identity() {
            return Err(ControllerError::ProgramAuthorityMismatch);
        }
        crate::player_projection::validate_candidate_projections(&checkpoint.state)?;
        let kernel = ProgramKernelV1::for_admitted_execution(
            checkpoint.execution_identity.program_kind,
            checkpoint.execution_identity.semantic_contract_id.clone(),
        )?;
        let replay = ReplayRecorderV5::new(build_reference_manifest(&replay_config, &checkpoint)?)?;
        Ok(Self {
            state: checkpoint.state,
            status: checkpoint.status,
            limit_counters: checkpoint.limit_counters,
            codec,
            execution_identity: magic_execution_identity(),
            replay_config,
            replay,
            kernel,
        })
    }

    pub fn magic_execution_identity() -> ExecutionIdentityV1 {
        magic_execution_identity()
    }
}

impl EnvironmentBackend for ReferenceEnvironmentBackend {
    fn players(&self) -> Vec<PlayerId> {
        self.state.core.players.keys().copied().collect()
    }

    fn checkpoint(&self) -> Result<EnvironmentCheckpointV5, ControllerError> {
        current_checkpoint(
            &self.state,
            &self.status,
            &self.limit_counters,
            &self.codec,
            &self.execution_identity,
        )
    }

    fn restore(&mut self, checkpoint: EnvironmentCheckpointV5) -> Result<(), ControllerError> {
        admit_restore(&RuntimeSemanticCatalog::production(), &checkpoint)?;
        let candidate = Self::from_admitted_checkpoint(
            checkpoint,
            self.codec.clone(),
            self.replay_config.clone(),
        )?;
        self.state = candidate.state;
        self.status = candidate.status;
        self.limit_counters = candidate.limit_counters;
        self.codec = candidate.codec;
        self.execution_identity = candidate.execution_identity;
        self.replay = candidate.replay;
        self.kernel = candidate.kernel;
        Ok(())
    }

    fn fork_boxed(&self) -> Result<Box<dyn EnvironmentBackend>, ControllerError> {
        let checkpoint = self.checkpoint()?;
        Ok(Box::new(Self::from_admitted_checkpoint(
            checkpoint,
            self.codec.clone(),
            self.replay_config.clone(),
        )?))
    }

    fn export_replay(&self) -> Result<AuthoritativeReplayV5, ControllerError> {
        Ok(self.replay.export()?)
    }

    fn execute_trusted_response(
        &mut self,
        actor: PlayerId,
        response: DecisionResponseV2,
    ) -> Result<TransitionResult, ControllerError> {
        Ok(self.kernel.apply(&self.state, actor, &response)?)
    }

    fn execute_forced_progress(&mut self) -> Result<TransitionResult, ControllerError> {
        let replay_config = self.replay_config.clone();
        let mut transaction = ReferenceEnvironmentTransaction {
            state: &mut self.state,
            status: &mut self.status,
            limit_counters: &mut self.limit_counters,
            codec: &self.codec,
            execution_identity: &self.execution_identity,
            replay: &mut self.replay,
            kernel: &mut self.kernel,
        };
        execute_forced_progress_transaction(&mut transaction, move |checkpoint| {
            build_reference_manifest(&replay_config, checkpoint)
        })
    }

    fn player_observation(
        &self,
        perspective: PlayerId,
    ) -> Result<ObservationEnvelope, PlayerEndpointError> {
        self.require_player(perspective)?;
        crate::player_projection::project_observation(&self.state, perspective)
    }

    fn player_information_state(
        &self,
        perspective: PlayerId,
    ) -> Result<PlayerInformationStateV2, PlayerEndpointError> {
        self.require_player(perspective)?;
        crate::player_projection::project_information_state(&self.state, perspective)
    }

    fn player_visible_decision(
        &self,
        perspective: PlayerId,
    ) -> Result<Option<PlayerDecisionRequestV2>, PlayerEndpointError> {
        self.require_player(perspective)?;
        crate::player_projection::project_visible_decision(&self.state, perspective)
    }

    fn submit_player_response(
        &mut self,
        perspective: PlayerId,
        _response: DecisionResponseV2,
    ) -> Result<PlayerStepV2, PlayerEndpointError> {
        self.require_player(perspective)?;
        let code = if matches!(self.status, EpisodeStatus::Running) {
            mtgml_observation::PlayerSubmissionCodeV1::UnavailableDecision
        } else {
            mtgml_observation::PlayerSubmissionCodeV1::EpisodeClosed
        };
        crate::player_projection::project_player_step(
            &self.state,
            perspective,
            self.status.clone(),
            PlayerStepSubmissionV1::Rejected { code },
        )
    }
}

impl ReferenceEnvironmentBackend {
    fn require_player(&self, perspective: PlayerId) -> Result<(), PlayerEndpointError> {
        self.state
            .core
            .players
            .contains_key(&perspective)
            .then_some(())
            .ok_or(PlayerEndpointError::ServiceUnavailable)
    }
}
