//! Shared reference-environment mechanics and the durable MagicRules backend.
//!
//! This module owns environment transaction mechanics that are independent of
//! rules meaning. The rules product is always supplied by `ProgramKernelV1`;
//! this module only validates, projects, checkpoints, rebases replay, and
//! commits the complete candidate atomically.

use mtgml_decision::{DecisionResponseV2, PlayerDecisionRequestV2};
use mtgml_model::{
    CheckpointCodecIdentity, ContentDigest, EnvironmentLimitCounters, EpisodeStatus,
    ExecutionIdentityV1, ExecutionProgramV1, PlayerId, SemanticContractIdV1,
};
use mtgml_observation::{
    ObservationEnvelope, ObservedEventEnvelopeV2, PlayerInformationStateV2, PlayerStepSubmissionV1,
    PlayerStepV2, INFORMATION_STATE_SCHEMA_V2, OBSERVATION_SCHEMA, OBSERVED_EVENT_SCHEMA_V2,
    PLAYER_STEP_SCHEMA_V2,
};
use mtgml_random::MTGML_RNG_V1;
use mtgml_replay::{
    AuthoritativeReplayV6, DeckIdentityV1, InitialEnvironmentIdentityV6, KernelIdentityV1,
    RandomnessIdentityV2, ReplayManifestV6, ReplayRecorderV6, ReplaySchemaVersionsV6,
    SemanticContractMaterialV5, REPLAY_MANIFEST_SCHEMA_V6, REPLAY_STEP_SCHEMA_V6,
};
use mtgml_rules::{validate_transition_contract, ProgramKernelV1, TransitionResult};
use mtgml_state::EngineState;

use crate::checkpoint::{EnvironmentCheckpointV6, EnvironmentLimitCounters as CheckpointCounters};
use crate::controller::EnvironmentBackend;
use crate::endpoint::PlayerEndpointError;
use crate::errors::{ControllerError, EnvironmentCommitError};
use crate::semantic_catalog::{admit_restore, RuntimeSemanticCatalog};
use crate::semantic_catalog_generated::{
    magic_s3_a_ordered_sba_0_1_0_rules_manifest, magic_s3_a_ordered_sba_0_1_0_semantic_contract_id,
    magic_s3_a_ordered_sba_0_1_0_semantic_manifest, magic_s3_b_basic_priority_0_1_0_rules_manifest,
    magic_s3_b_basic_priority_0_1_0_semantic_contract_id,
    magic_s3_b_basic_priority_0_1_0_semantic_manifest,
    magic_s3_c_draw_interaction_0_1_0_rules_manifest,
    magic_s3_c_draw_interaction_0_1_0_semantic_contract_id,
    magic_s3_c_draw_interaction_0_1_0_semantic_manifest, magic_turn_structure_0_1_0_rules_manifest,
    magic_turn_structure_0_1_0_semantic_contract_id, magic_turn_structure_0_1_0_semantic_manifest,
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
    pub schemas: ReplaySchemaVersionsV6,
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
    replay: ReplayRecorderV6,
    kernel: ProgramKernelV1,
}

fn magic_execution_identity() -> ExecutionIdentityV1 {
    ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::MagicRules,
        semantic_contract_id: magic_turn_structure_0_1_0_semantic_contract_id(),
    }
}

fn magic_state_based_actions_execution_identity() -> ExecutionIdentityV1 {
    ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::MagicRules,
        semantic_contract_id: magic_s3_a_ordered_sba_0_1_0_semantic_contract_id(),
    }
}

fn magic_basic_priority_execution_identity() -> ExecutionIdentityV1 {
    ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::MagicRules,
        semantic_contract_id: magic_s3_b_basic_priority_0_1_0_semantic_contract_id(),
    }
}

fn magic_draw_execution_identity() -> ExecutionIdentityV1 {
    ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::MagicRules,
        semantic_contract_id: magic_s3_c_draw_interaction_0_1_0_semantic_contract_id(),
    }
}

fn semantic_material(
    id: &SemanticContractIdV1,
) -> Result<SemanticContractMaterialV5, ControllerError> {
    if *id == magic_execution_identity().semantic_contract_id {
        return Ok(SemanticContractMaterialV5 {
            semantic_contract_id: id.clone(),
            manifest: magic_turn_structure_0_1_0_semantic_manifest(),
            rules_manifest: magic_turn_structure_0_1_0_rules_manifest(),
        });
    }
    if *id == magic_state_based_actions_execution_identity().semantic_contract_id {
        return Ok(SemanticContractMaterialV5 {
            semantic_contract_id: id.clone(),
            manifest: magic_s3_a_ordered_sba_0_1_0_semantic_manifest(),
            rules_manifest: magic_s3_a_ordered_sba_0_1_0_rules_manifest(),
        });
    }
    if *id == magic_basic_priority_execution_identity().semantic_contract_id {
        return Ok(SemanticContractMaterialV5 {
            semantic_contract_id: id.clone(),
            manifest: magic_s3_b_basic_priority_0_1_0_semantic_manifest(),
            rules_manifest: magic_s3_b_basic_priority_0_1_0_rules_manifest(),
        });
    }
    if *id == magic_draw_execution_identity().semantic_contract_id {
        return Ok(SemanticContractMaterialV5 {
            semantic_contract_id: id.clone(),
            manifest: magic_s3_c_draw_interaction_0_1_0_semantic_manifest(),
            rules_manifest: magic_s3_c_draw_interaction_0_1_0_rules_manifest(),
        });
    }
    Err(ControllerError::SemanticContractUnsupported)
}

fn current_v6_schema_versions() -> ReplaySchemaVersionsV6 {
    ReplaySchemaVersionsV6 {
        observation: OBSERVATION_SCHEMA.into(),
        observation_payload_codec: "synthetic-m3-observation.v1".into(),
        information_state: INFORMATION_STATE_SCHEMA_V2.into(),
        decision: mtgml_decision::PLAYER_DECISION_REQUEST_V2_SCHEMA.into(),
        decision_response: mtgml_decision::DECISION_RESPONSE_V2_SCHEMA.into(),
        observed_event: OBSERVED_EVENT_SCHEMA_V2.into(),
        player_step: PLAYER_STEP_SCHEMA_V2.into(),
        replay_step: REPLAY_STEP_SCHEMA_V6.into(),
    }
}

fn magic_v6_schema_versions() -> ReplaySchemaVersionsV6 {
    ReplaySchemaVersionsV6 {
        observation: OBSERVATION_SCHEMA.into(),
        observation_payload_codec: "magic-m3-observation.v1".into(),
        information_state: INFORMATION_STATE_SCHEMA_V2.into(),
        decision: mtgml_decision::PLAYER_DECISION_REQUEST_V2_SCHEMA.into(),
        decision_response: mtgml_decision::DECISION_RESPONSE_V2_SCHEMA.into(),
        observed_event: OBSERVED_EVENT_SCHEMA_V2.into(),
        player_step: PLAYER_STEP_SCHEMA_V2.into(),
        replay_step: REPLAY_STEP_SCHEMA_V6.into(),
    }
}

fn validate_reference_replay_config(
    config: &ReferenceEnvironmentReplayConfig,
) -> Result<(), ControllerError> {
    if config.scenario_id.is_empty()
        || config.rules_snapshot.is_empty()
        || (config.schemas != current_v6_schema_versions()
            && config.schemas != magic_v6_schema_versions())
    {
        return Err(ControllerError::ReplayIdentityMismatch);
    }
    Ok(())
}

pub(crate) fn build_reference_manifest(
    config: &ReferenceEnvironmentReplayConfig,
    checkpoint: &EnvironmentCheckpointV6,
) -> Result<ReplayManifestV6, ControllerError> {
    validate_reference_replay_config(config)?;
    let expected_schemas = if checkpoint.execution_identity == magic_execution_identity() {
        current_v6_schema_versions()
    } else if checkpoint.execution_identity == magic_state_based_actions_execution_identity()
        || checkpoint.execution_identity == magic_basic_priority_execution_identity()
        || checkpoint.execution_identity == magic_draw_execution_identity()
    {
        magic_v6_schema_versions()
    } else {
        return Err(ControllerError::ProgramAuthorityMismatch);
    };
    if config.schemas != expected_schemas {
        return Err(ControllerError::ReplayIdentityMismatch);
    }
    let players: Vec<_> = checkpoint.state.core.players.keys().copied().collect();
    if players.len() != 2 {
        return Err(ControllerError::ProgramStateIncompatible);
    }

    // V6 requires participant/deck-shaped provenance fields. These values are
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
    let manifest = ReplayManifestV6 {
        schema_version: REPLAY_MANIFEST_SCHEMA_V6.into(),
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
        semantic_contract: semantic_material(&checkpoint.execution_identity.semantic_contract_id)?,
    };
    manifest.validate()?;
    Ok(manifest)
}

fn identity_from_checkpoint(checkpoint: &EnvironmentCheckpointV6) -> InitialEnvironmentIdentityV6 {
    InitialEnvironmentIdentityV6 {
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
) -> Result<EnvironmentCheckpointV6, ControllerError> {
    Ok(EnvironmentCheckpointV6::new(
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
    pub(crate) replay: &'a mut ReplayRecorderV6,
    pub(crate) kernel: &'a mut ProgramKernelV1,
}

pub(crate) fn execute_forced_progress_transaction<F>(
    transaction: &mut ReferenceEnvironmentTransaction<'_>,
    build_manifest: F,
) -> Result<TransitionResult, ControllerError>
where
    F: FnOnce(&EnvironmentCheckpointV6) -> Result<ReplayManifestV6, ControllerError>,
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
    let projection_profile =
        crate::player_projection::profile_for_execution_identity(execution_identity)?;
    crate::player_projection::validate_candidate_projections_with_profile(
        &transition.next_state,
        projection_profile,
    )?;

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
    let candidate = EnvironmentCheckpointV6::new(
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
    let rebased_replay = ReplayRecorderV6::new(build_manifest(&candidate)?)?;

    *state = candidate.state;
    *status = candidate.status;
    *limit_counters = candidate.limit_counters;
    *replay = rebased_replay;
    Ok(transition)
}

impl ReferenceEnvironmentBackend {
    pub fn new(config: ReferenceEnvironmentConfig) -> Result<Self, ControllerError> {
        if config.execution_identity != magic_execution_identity()
            && config.execution_identity != magic_state_based_actions_execution_identity()
            && config.execution_identity != magic_basic_priority_execution_identity()
            && config.execution_identity != magic_draw_execution_identity()
        {
            return Err(ControllerError::ProgramAuthorityMismatch);
        }
        let checkpoint = EnvironmentCheckpointV6::new(
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
        checkpoint: EnvironmentCheckpointV6,
        codec: CheckpointCodecIdentity,
        replay_config: ReferenceEnvironmentReplayConfig,
    ) -> Result<Self, ControllerError> {
        checkpoint.validate()?;
        if checkpoint.codec != codec {
            return Err(ControllerError::UnsupportedCheckpointCodec);
        }
        if checkpoint.execution_identity != magic_execution_identity()
            && checkpoint.execution_identity != magic_state_based_actions_execution_identity()
            && checkpoint.execution_identity != magic_basic_priority_execution_identity()
            && checkpoint.execution_identity != magic_draw_execution_identity()
        {
            return Err(ControllerError::ProgramAuthorityMismatch);
        }
        let projection_profile = crate::player_projection::profile_for_execution_identity(
            &checkpoint.execution_identity,
        )?;
        crate::player_projection::validate_candidate_projections_with_profile(
            &checkpoint.state,
            projection_profile,
        )?;
        let kernel = ProgramKernelV1::for_admitted_execution(
            checkpoint.execution_identity.program_kind,
            checkpoint.execution_identity.semantic_contract_id.clone(),
        )?;
        let replay = ReplayRecorderV6::new(build_reference_manifest(&replay_config, &checkpoint)?)?;
        Ok(Self {
            state: checkpoint.state,
            status: checkpoint.status,
            limit_counters: checkpoint.limit_counters,
            codec,
            execution_identity: checkpoint.execution_identity.clone(),
            replay_config,
            replay,
            kernel,
        })
    }

    pub fn magic_execution_identity() -> ExecutionIdentityV1 {
        magic_execution_identity()
    }

    pub fn magic_state_based_actions_execution_identity() -> ExecutionIdentityV1 {
        magic_state_based_actions_execution_identity()
    }

    pub fn magic_basic_priority_execution_identity() -> ExecutionIdentityV1 {
        magic_basic_priority_execution_identity()
    }

    pub fn magic_draw_execution_identity() -> ExecutionIdentityV1 {
        magic_draw_execution_identity()
    }

    fn projection_profile(
        &self,
    ) -> Result<crate::player_projection::ObservationProjectionProfile, ControllerError> {
        crate::player_projection::profile_for_execution_identity(&self.execution_identity)
    }

    fn execute_response_transaction<F>(
        &mut self,
        actor: PlayerId,
        response: DecisionResponseV2,
        before_commit: F,
    ) -> Result<TransitionResult, ControllerError>
    where
        F: FnOnce(
            &EnvironmentCheckpointV6,
            &TransitionResult,
            &std::collections::BTreeMap<PlayerId, Vec<ObservedEventEnvelopeV2>>,
        ) -> Result<(), ControllerError>,
    {
        let transaction = crate::response_transaction::ResponseTransaction {
            state: &mut self.state,
            status: &mut self.status,
            limit_counters: &mut self.limit_counters,
            codec: &self.codec,
            execution_identity: &self.execution_identity,
            replay: &mut self.replay,
            kernel: &mut self.kernel,
        };
        crate::response_transaction::execute_response_transaction(
            transaction,
            actor,
            response,
            before_commit,
            #[cfg(test)]
            None,
            #[cfg(test)]
            None,
        )
    }
}

impl EnvironmentBackend for ReferenceEnvironmentBackend {
    fn players(&self) -> Vec<PlayerId> {
        self.state.core.players.keys().copied().collect()
    }

    fn checkpoint(&self) -> Result<EnvironmentCheckpointV6, ControllerError> {
        current_checkpoint(
            &self.state,
            &self.status,
            &self.limit_counters,
            &self.codec,
            &self.execution_identity,
        )
    }

    fn restore(&mut self, checkpoint: EnvironmentCheckpointV6) -> Result<(), ControllerError> {
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

    fn export_replay(&self) -> Result<AuthoritativeReplayV6, ControllerError> {
        Ok(self.replay.export()?)
    }

    fn execute_trusted_response(
        &mut self,
        actor: PlayerId,
        response: DecisionResponseV2,
    ) -> Result<TransitionResult, ControllerError> {
        self.execute_response_transaction(actor, response, |_, _, _| Ok(()))
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
        crate::player_projection::project_observation_with_profile(
            &self.state,
            perspective,
            self.projection_profile()
                .map_err(|_| PlayerEndpointError::ServiceUnavailable)?,
        )
    }

    fn player_information_state(
        &self,
        perspective: PlayerId,
    ) -> Result<PlayerInformationStateV2, PlayerEndpointError> {
        self.require_player(perspective)?;
        crate::player_projection::project_information_state_with_profile(
            &self.state,
            perspective,
            self.projection_profile()
                .map_err(|_| PlayerEndpointError::ServiceUnavailable)?,
        )
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
        response: DecisionResponseV2,
    ) -> Result<PlayerStepV2, PlayerEndpointError> {
        self.require_player(perspective)?;
        let s3_magic = self.execution_identity == magic_state_based_actions_execution_identity()
            || self.execution_identity == magic_basic_priority_execution_identity()
            || self.execution_identity == magic_draw_execution_identity();
        let code = if !matches!(self.status, EpisodeStatus::Running) {
            Some(mtgml_observation::PlayerSubmissionCodeV1::EpisodeClosed)
        } else if !s3_magic {
            Some(mtgml_observation::PlayerSubmissionCodeV1::UnavailableDecision)
        } else if let Some(pending) = self.state.execution.pending_decision.as_ref() {
            if pending.request.actor != perspective {
                Some(mtgml_observation::PlayerSubmissionCodeV1::UnavailableDecision)
            } else {
                let visible = pending
                    .request
                    .project_player_request()
                    .map_err(|_| PlayerEndpointError::ServiceUnavailable)?;
                match response.validate_for(&visible) {
                    Ok(()) => None,
                    Err(mtgml_decision::DecisionValidationError::DecisionIdentityMismatch)
                    | Err(mtgml_decision::DecisionValidationError::StateRevisionMismatch) => {
                        Some(mtgml_observation::PlayerSubmissionCodeV1::StaleDecision)
                    }
                    Err(mtgml_decision::DecisionValidationError::AnswerDomainMismatch) => {
                        Some(mtgml_observation::PlayerSubmissionCodeV1::InvalidAnswer)
                    }
                    Err(mtgml_decision::DecisionValidationError::UnknownCandidate) => {
                        Some(mtgml_observation::PlayerSubmissionCodeV1::InvalidCandidate)
                    }
                    Err(mtgml_decision::DecisionValidationError::DuplicateAnswerCandidate) => {
                        Some(mtgml_observation::PlayerSubmissionCodeV1::DuplicateAssignment)
                    }
                    Err(mtgml_decision::DecisionValidationError::NoncanonicalAnswer) => {
                        Some(mtgml_observation::PlayerSubmissionCodeV1::InvalidOrder)
                    }
                    Err(mtgml_decision::DecisionValidationError::AnswerCardinality) => {
                        Some(mtgml_observation::PlayerSubmissionCodeV1::InvalidCardinality)
                    }
                    Err(mtgml_decision::DecisionValidationError::NumericOutOfBounds) => {
                        Some(mtgml_observation::PlayerSubmissionCodeV1::InvalidNumber)
                    }
                    Err(_) => Some(mtgml_observation::PlayerSubmissionCodeV1::InvalidAnswer),
                }
            }
        } else {
            Some(mtgml_observation::PlayerSubmissionCodeV1::UnavailableDecision)
        };
        if let Some(code) = code {
            return crate::player_projection::project_player_step_with_profile(
                &self.state,
                perspective,
                self.status.clone(),
                PlayerStepSubmissionV1::Rejected { code },
                self.projection_profile()
                    .map_err(|_| PlayerEndpointError::ServiceUnavailable)?,
            );
        }
        let profile = self
            .projection_profile()
            .map_err(|_| PlayerEndpointError::ServiceUnavailable)?;
        let mut accepted_step = None;
        let transition = self
            .execute_response_transaction(
                perspective,
                response,
                |candidate, transition, occurrences| {
                    let mut step = crate::player_projection::project_player_step_with_profile(
                        &candidate.state,
                        perspective,
                        transition.status.clone(),
                        PlayerStepSubmissionV1::Accepted,
                        profile,
                    )
                    .map_err(|_| {
                        ControllerError::EnvironmentCommit(
                            crate::errors::EnvironmentCommitError::PlayerProjectionInvalid,
                        )
                    })?;
                    if let Some(events) = occurrences.get(&perspective) {
                        step.observed_events = events.clone();
                    }
                    step.validate().map_err(|_| {
                        ControllerError::EnvironmentCommit(
                            crate::errors::EnvironmentCommitError::PlayerProjectionInvalid,
                        )
                    })?;
                    accepted_step = Some(step);
                    Ok(())
                },
            )
            .map_err(|_| PlayerEndpointError::ServiceUnavailable)?;
        if !transition.accepted {
            return Err(PlayerEndpointError::ServiceUnavailable);
        }
        accepted_step.ok_or(PlayerEndpointError::ServiceUnavailable)
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
