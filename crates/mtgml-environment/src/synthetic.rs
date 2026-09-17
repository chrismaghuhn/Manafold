use mtgml_decision::{DecisionResponseV2, PlayerDecisionRequestV2};
use mtgml_model::{EpisodeStatus, PlayerId};
use mtgml_observation::{
    ObservationEnvelope, PlayerInformationStateV2, PlayerStepV2, PlayerSubmissionCodeV1,
};
use mtgml_random::RootSeed256;
use mtgml_replay::{
    AuthoritativeReplayV4, DeckIdentityV1, KernelIdentityV1, ReplayRecorderV4,
    ReplaySchemaVersionsV4,
};
use mtgml_rules::{SyntheticM1RulesKernel, TransitionResult};
use mtgml_state::{
    construct_synthetic_engine_state, EngineState, SyntheticResetInputs, SyntheticV4Setup,
};

use crate::checkpoint::{
    CheckpointCodecIdentity, EnvironmentCheckpointV4, EnvironmentLimitCounters,
};
use crate::controller::EnvironmentBackend;
use crate::endpoint::PlayerEndpointError;
use crate::errors::{ControllerError, EnvironmentCommitError};

mod commit;
#[cfg(test)]
#[path = "tests/eventful.rs"]
mod eventful;
mod projection;
mod replay;

// G.6 Node A: declared as a child module of this file so the historical
// reprojection evidence reuses THE private production projection/step-
// assembly functions instead of any second projector.
#[cfg(test)]
#[path = "replay_parity_tests.rs"]
mod replay_parity_tests;

use replay::build_manifest;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntheticM1EnvironmentConfig {
    pub codec: CheckpointCodecIdentity,
    pub setup: SyntheticV4Setup,
    pub replay: SyntheticM1ReplayConfig,
}

impl SyntheticM1EnvironmentConfig {
    /// Builds the current rules-free M2 compatibility configuration while
    /// keeping the V4 state setup behind the Environment ownership boundary.
    pub fn m2_compatibility(
        codec: CheckpointCodecIdentity,
        replay: SyntheticM1ReplayConfig,
    ) -> Self {
        Self {
            codec,
            setup: SyntheticV4Setup::m2_compatibility(),
            replay,
        }
    }
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
    pub schemas: ReplaySchemaVersionsV4,
    pub decks: Vec<DeckIdentityV1>,
}

pub struct SyntheticM1EnvironmentBackend {
    state: EngineState,
    status: EpisodeStatus,
    limit_counters: EnvironmentLimitCounters,
    codec: CheckpointCodecIdentity,
    config: SyntheticM1EnvironmentConfig,
    replay: ReplayRecorderV4,
    kernel: SyntheticM1RulesKernel,
    #[cfg(test)]
    eventful_fixture: bool,
}

impl SyntheticM1EnvironmentBackend {
    pub fn new(
        players: [PlayerId; 2],
        root_seed: RootSeed256,
        config: SyntheticM1EnvironmentConfig,
    ) -> Result<Self, ControllerError> {
        let state = construct_synthetic_engine_state(SyntheticResetInputs {
            players,
            root_seed,
            setup: config.setup.clone(),
        })?;
        let status = EpisodeStatus::Running;
        let limit_counters = EnvironmentLimitCounters::default();
        let checkpoint = EnvironmentCheckpointV4::new(
            state.clone(),
            status.clone(),
            limit_counters.clone(),
            config.codec.clone(),
        )?;
        let replay = ReplayRecorderV4::new(build_manifest(&config, &checkpoint)?)?;
        Ok(Self {
            state,
            status,
            limit_counters,
            codec: config.codec.clone(),
            config,
            replay,
            kernel: SyntheticM1RulesKernel,
            #[cfg(test)]
            eventful_fixture: false,
        })
    }

    pub fn from_checkpoint(
        checkpoint: EnvironmentCheckpointV4,
        config: SyntheticM1EnvironmentConfig,
    ) -> Result<Self, ControllerError> {
        checkpoint.validate()?;
        if checkpoint.codec != config.codec {
            return Err(ControllerError::UnsupportedCheckpointCodec);
        }
        // A structurally valid generic EngineState may still express
        // decisions this synthetic kernel cannot execute; such checkpoints
        // are rejected before any player projection can expose them.
        mtgml_rules::validate_synthetic_runtime_state(&checkpoint.state)
            .map_err(|_| ControllerError::UnsupportedSyntheticState)?;
        let replay = ReplayRecorderV4::new(build_manifest(&config, &checkpoint)?)?;
        Ok(Self {
            state: checkpoint.state,
            status: checkpoint.status,
            limit_counters: checkpoint.limit_counters,
            codec: checkpoint.codec,
            config,
            replay,
            kernel: SyntheticM1RulesKernel,
            #[cfg(test)]
            eventful_fixture: false,
        })
    }
}

impl EnvironmentBackend for SyntheticM1EnvironmentBackend {
    fn players(&self) -> Vec<PlayerId> {
        self.state.core.players.keys().copied().collect()
    }

    fn checkpoint(&self) -> Result<EnvironmentCheckpointV4, ControllerError> {
        self.current_checkpoint()
    }

    fn restore(&mut self, checkpoint: EnvironmentCheckpointV4) -> Result<(), ControllerError> {
        #[cfg(test)]
        let eventful_fixture = self.eventful_fixture;
        let candidate = Self::from_checkpoint(checkpoint, self.config.clone())?;
        self.state = candidate.state;
        self.status = candidate.status;
        self.limit_counters = candidate.limit_counters;
        self.codec = candidate.codec;
        self.replay = candidate.replay;
        self.kernel = SyntheticM1RulesKernel;
        #[cfg(test)]
        {
            self.eventful_fixture = eventful_fixture;
        }
        Ok(())
    }

    fn fork_boxed(&self) -> Result<Box<dyn EnvironmentBackend>, ControllerError> {
        let checkpoint = self.current_checkpoint()?;
        #[cfg(test)]
        let mut child = Self::from_checkpoint(checkpoint, self.config.clone())?;
        #[cfg(not(test))]
        let child = Self::from_checkpoint(checkpoint, self.config.clone())?;
        #[cfg(test)]
        {
            child.eventful_fixture = self.eventful_fixture;
        }
        Ok(Box::new(child))
    }

    fn export_replay(&self) -> Result<AuthoritativeReplayV4, ControllerError> {
        Ok(self.replay.export()?)
    }

    fn execute_trusted_response(
        &mut self,
        actor: PlayerId,
        response: DecisionResponseV2,
    ) -> Result<TransitionResult, ControllerError> {
        self.execute_response(
            actor,
            response,
            |_candidate, _transition, _envelopes| Ok(()),
        )
    }

    fn execute_forced_progress(&mut self) -> Result<TransitionResult, ControllerError> {
        self.execute_forced_progress()
    }

    fn player_observation(
        &self,
        perspective: PlayerId,
    ) -> Result<ObservationEnvelope, PlayerEndpointError> {
        self.require_player(perspective)?;
        Self::synthetic_observation(&self.state, perspective)
    }

    fn player_information_state(
        &self,
        perspective: PlayerId,
    ) -> Result<PlayerInformationStateV2, PlayerEndpointError> {
        self.require_player(perspective)?;
        Self::player_information_state_from_state(&self.state, perspective)
    }

    fn player_visible_decision(
        &self,
        perspective: PlayerId,
    ) -> Result<Option<PlayerDecisionRequestV2>, PlayerEndpointError> {
        self.require_player(perspective)?;
        Self::visible_decision_from_state(&self.state, perspective)
    }

    /// Ordered typed-submission pipeline (DECISION_PROTOCOL validation
    /// order). Layer-B rejections return `Ok` with a mirrored unchanged
    /// product carrying only the closed rejected outcome; anything else maps
    /// to the closed service failure.
    fn submit_player_response(
        &mut self,
        perspective: PlayerId,
        response: DecisionResponseV2,
    ) -> Result<PlayerStepV2, PlayerEndpointError> {
        use mtgml_observation::PlayerStepSubmissionV1 as Submission;

        self.require_player(perspective)?;
        // 2. episode availability.
        if !matches!(&self.status, EpisodeStatus::Running) {
            let step = Self::player_step_from_state(
                &self.state,
                perspective,
                self.status.clone(),
                Submission::Rejected {
                    code: PlayerSubmissionCodeV1::EpisodeClosed,
                },
            )?;
            step.validate()
                .map_err(|_| PlayerEndpointError::ServiceUnavailable)?;
            return Ok(step);
        }
        // 3. request availability for this perspective (non-disclosing).
        let Some(pending) = self.state.execution.pending_decision.as_ref() else {
            let step = Self::player_step_from_state(
                &self.state,
                perspective,
                self.status.clone(),
                Submission::Rejected {
                    code: PlayerSubmissionCodeV1::UnavailableDecision,
                },
            )?;
            step.validate()
                .map_err(|_| PlayerEndpointError::ServiceUnavailable)?;
            return Ok(step);
        };
        if pending.request.actor != perspective {
            let step = Self::player_step_from_state(
                &self.state,
                perspective,
                self.status.clone(),
                Submission::Rejected {
                    code: PlayerSubmissionCodeV1::UnavailableDecision,
                },
            )?;
            step.validate()
                .map_err(|_| PlayerEndpointError::ServiceUnavailable)?;
            return Ok(step);
        }
        // 4. visible request projection.
        let visible_request = pending
            .request
            .project_player_request()
            .map_err(|_| PlayerEndpointError::ServiceUnavailable)?;

        // 5.-11. identity/revision/variant/membership/uniqueness/canonical/
        // cardinality/numeric classification via the decision authority.
        let code = match response.validate_for(&visible_request) {
            Ok(()) => None,
            Err(mtgml_decision::DecisionValidationError::DecisionIdentityMismatch)
            | Err(mtgml_decision::DecisionValidationError::StateRevisionMismatch) => {
                Some(PlayerSubmissionCodeV1::StaleDecision)
            }
            Err(mtgml_decision::DecisionValidationError::AnswerDomainMismatch) => {
                Some(PlayerSubmissionCodeV1::InvalidAnswer)
            }
            Err(mtgml_decision::DecisionValidationError::UnknownCandidate) => {
                Some(PlayerSubmissionCodeV1::InvalidCandidate)
            }
            Err(mtgml_decision::DecisionValidationError::DuplicateAnswerCandidate) => {
                Some(PlayerSubmissionCodeV1::DuplicateAssignment)
            }
            Err(mtgml_decision::DecisionValidationError::NoncanonicalAnswer) => {
                Some(PlayerSubmissionCodeV1::InvalidOrder)
            }
            Err(mtgml_decision::DecisionValidationError::AnswerCardinality) => {
                Some(PlayerSubmissionCodeV1::InvalidCardinality)
            }
            Err(mtgml_decision::DecisionValidationError::NumericOutOfBounds) => {
                Some(PlayerSubmissionCodeV1::InvalidNumber)
            }
            Err(_) => Some(PlayerSubmissionCodeV1::InvalidAnswer),
        };
        if let Some(code) = code {
            // Layer B: mirror the unchanged committed product.
            let step = Self::player_step_from_state(
                &self.state,
                perspective,
                self.status.clone(),
                Submission::Rejected { code },
            )?;
            step.validate()
                .map_err(|_| PlayerEndpointError::ServiceUnavailable)?;
            return Ok(step);
        }

        // 12./13. trusted binding/context and kernel execution. A kernel that
        // still reports `accepted=false` after a fully accepted public
        // submission is an internal soundness failure, not player illegality.
        let mut projected_step = None;
        let transition = self
            .execute_response(
                perspective,
                response,
                |candidate, transition, occurrence_envelopes| {
                    let step = Self::player_step_from_state(
                        &candidate.state,
                        perspective,
                        transition.status.clone(),
                        Submission::Accepted,
                    )
                    .map_err(|_| {
                        ControllerError::EnvironmentCommit(
                            EnvironmentCommitError::PlayerProjectionInvalid,
                        )
                    })?;
                    let mut step = step;
                    // Attach the per-perspective observed batch and re-validate
                    // the COMPLETE player product before the atomic commit.
                    if let Some(envelopes) = occurrence_envelopes.get(&perspective) {
                        step.observed_events = envelopes.clone();
                    }
                    step.validate().map_err(|_| {
                        ControllerError::EnvironmentCommit(
                            EnvironmentCommitError::PlayerProjectionInvalid,
                        )
                    })?;
                    projected_step = Some(step);
                    Ok(())
                },
            )
            .map_err(|_| PlayerEndpointError::ServiceUnavailable)?;
        if !transition.accepted {
            return Err(PlayerEndpointError::ServiceUnavailable);
        }
        projected_step.ok_or(PlayerEndpointError::ServiceUnavailable)
    }
}
