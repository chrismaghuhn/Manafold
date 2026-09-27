//! Admitted bounded-Magic environment execution over the successor state.
//!
//! This is the concrete owner of successor restore, fork, player projection,
//! response commit, and Replay V7 capture. It accepts no detached state until
//! the exact Phase-9 executable admission and Replay V7 identity agree.

use mtgml_card_ir::ExecutableProfileAdmissionV1;
use mtgml_decision::{DecisionResponseV2, PlayerDecisionRequestV3};
use mtgml_model::{EnvironmentLimitCounters, EpisodeStatus, PlayerId};
use mtgml_observation::{ObservationEnvelope, PlayerInformationStateV2};
use mtgml_replay::{
    AuthoritativeReplayV7, InitialEnvironmentIdentityV7, ReplayManifestV7, ReplayRecorderV7,
};
use mtgml_rules::ProgramKernelV1;
use mtgml_state::EngineStatePartsV2;

use crate::replay_v7_execution::ReplayV7ExecutionReport;
use crate::successor_transaction::SuccessorTransactionOutput;
use crate::{ControllerError, EnvironmentCheckpointV7};

pub struct SuccessorEnvironmentRuntime {
    admission: ExecutableProfileAdmissionV1,
    state: EngineStatePartsV2,
    status: EpisodeStatus,
    limit_counters: EnvironmentLimitCounters,
    replay: ReplayRecorderV7,
    replay_origin: EnvironmentCheckpointV7,
    kernel: ProgramKernelV1,
}

impl SuccessorEnvironmentRuntime {
    pub fn new(
        admission: ExecutableProfileAdmissionV1,
        state: EngineStatePartsV2,
        status: EpisodeStatus,
        limit_counters: EnvironmentLimitCounters,
        manifest: ReplayManifestV7,
    ) -> Result<Self, ControllerError> {
        let checkpoint = EnvironmentCheckpointV7::new(
            state.clone(),
            status.clone(),
            limit_counters.clone(),
            admission.execution_identity().clone(),
        )?;
        verify_admission(&admission, &manifest)?;
        if manifest.initial_identity != identity(&checkpoint) {
            return Err(crate::ReplayExecutionError::ReplayV7BeforeIdentity.into());
        }
        checkpoint.restore_with_verified_contracts(
            admission.semantic_contract_manifest(),
            admission.rules_contract_manifest(),
            Some(admission.verified_catalog()),
        )?;
        validate_executable_candidate_set(&admission, &state, &status)?;
        Ok(Self {
            kernel: ProgramKernelV1::for_executable_profile(admission.clone())
                .map_err(|_| ControllerError::SemanticContractUnsupported)?,
            admission,
            state,
            status,
            limit_counters,
            replay: ReplayRecorderV7::new(manifest)?,
            replay_origin: checkpoint,
        })
    }

    pub fn players(&self) -> Vec<PlayerId> {
        self.state
            .predecessor_v5
            .core
            .players
            .keys()
            .copied()
            .collect()
    }

    pub fn checkpoint(&self) -> Result<EnvironmentCheckpointV7, ControllerError> {
        Ok(EnvironmentCheckpointV7::new(
            self.state.clone(),
            self.status.clone(),
            self.limit_counters.clone(),
            self.admission.execution_identity().clone(),
        )?)
    }

    pub fn fork(&self) -> Result<Self, ControllerError> {
        Ok(Self {
            admission: self.admission.clone(),
            state: self.state.clone(),
            status: self.status.clone(),
            limit_counters: self.limit_counters.clone(),
            replay: self.replay.clone(),
            replay_origin: self.replay_origin.clone(),
            kernel: ProgramKernelV1::for_executable_profile(self.admission.clone())
                .map_err(|_| ControllerError::SemanticContractUnsupported)?,
        })
    }

    /// Restores a verified checkpoint and starts a new replay segment rooted
    /// at that exact identity. The checkpoint itself contains all decision,
    /// continuation, and card-rules state; no backend-side request cache is
    /// restored or synthesized.
    pub fn restore(&mut self, checkpoint: EnvironmentCheckpointV7) -> Result<(), ControllerError> {
        verify_checkpoint_admission(&self.admission, &checkpoint)?;
        checkpoint.restore_with_verified_contracts(
            self.admission.semantic_contract_manifest(),
            self.admission.rules_contract_manifest(),
            Some(self.admission.verified_catalog()),
        )?;
        validate_executable_candidate_set(&self.admission, &checkpoint.state, &checkpoint.status)?;
        let mut manifest = self.replay.manifest().clone();
        manifest.initial_identity = identity(&checkpoint);
        manifest.validate()?;
        let replay = ReplayRecorderV7::new(manifest)?;
        let kernel = ProgramKernelV1::for_executable_profile(self.admission.clone())
            .map_err(|_| ControllerError::SemanticContractUnsupported)?;
        self.replay_origin = checkpoint.clone();
        self.state = checkpoint.state;
        self.status = checkpoint.status;
        self.limit_counters = checkpoint.limit_counters;
        self.replay = replay;
        self.kernel = kernel;
        Ok(())
    }

    pub fn information_state(
        &self,
        perspective: PlayerId,
    ) -> Result<PlayerInformationStateV2, crate::PlayerEndpointError> {
        crate::project_successor_information_state(
            &self.state,
            perspective,
            self.admission.execution_identity(),
            self.admission.semantic_contract_manifest(),
            self.admission.rules_contract_manifest(),
            self.admission.verified_catalog(),
        )
    }

    pub fn observation(
        &self,
        perspective: PlayerId,
    ) -> Result<ObservationEnvelope, crate::PlayerEndpointError> {
        Ok(self.information_state(perspective)?.current_observation)
    }

    pub fn visible_decision(
        &self,
        perspective: PlayerId,
    ) -> Result<Option<PlayerDecisionRequestV3>, crate::PlayerEndpointError> {
        if !self
            .state
            .predecessor_v5
            .core
            .players
            .contains_key(&perspective)
        {
            return Err(crate::PlayerEndpointError::ServiceUnavailable);
        }
        self.state
            .execution_v3
            .pending_decision
            .as_ref()
            .filter(|request| request.actor == perspective)
            .map(|request| request.project_player_request())
            .transpose()
            .map_err(|_| crate::PlayerEndpointError::ServiceUnavailable)
    }

    pub fn submit(
        &mut self,
        perspective: PlayerId,
        response: DecisionResponseV2,
    ) -> Result<SuccessorTransactionOutput, crate::PlayerEndpointError> {
        if !self
            .state
            .predecessor_v5
            .core
            .players
            .contains_key(&perspective)
        {
            return Err(crate::PlayerEndpointError::ServiceUnavailable);
        }
        let output = crate::successor_transaction::execute_successor_response_transaction(
            crate::successor_transaction::SuccessorResponseTransaction {
                state: &mut self.state,
                status: &mut self.status,
                limit_counters: &mut self.limit_counters,
                execution_identity: self.admission.execution_identity(),
                replay: &mut self.replay,
                kernel: &mut self.kernel,
            },
            perspective,
            response,
        )
        .map_err(|_| crate::PlayerEndpointError::ServiceUnavailable)?;
        Ok(output)
    }

    pub fn export_replay(&self) -> Result<AuthoritativeReplayV7, ControllerError> {
        Ok(self.replay.export()?)
    }

    pub fn execute_replay(
        &self,
        replay: AuthoritativeReplayV7,
    ) -> Result<ReplayV7ExecutionReport, ControllerError> {
        crate::replay_v7_execution::execute_authoritative_replay_v7(
            self.admission.clone(),
            self.replay_origin.clone(),
            replay,
        )
    }

    pub fn state(&self) -> &EngineStatePartsV2 {
        &self.state
    }
}

pub(crate) fn validate_executable_candidate_set(
    admission: &ExecutableProfileAdmissionV1,
    state: &EngineStatePartsV2,
    status: &EpisodeStatus,
) -> Result<(), ControllerError> {
    mtgml_rules::validate_basic_land_pending_request(admission, state, status)
        .map_err(|_| crate::CheckpointV7Error::CandidateSet.into())
}

fn identity(checkpoint: &EnvironmentCheckpointV7) -> InitialEnvironmentIdentityV7 {
    InitialEnvironmentIdentityV7 {
        state_revision: checkpoint.state.predecessor_v5.revision,
        full_state_digest: checkpoint.state_digest.clone(),
        episode_status: checkpoint.status.clone(),
        environment_limit_counters: checkpoint.limit_counters.clone(),
        checkpoint_codec_identity: checkpoint.codec.clone(),
        checkpoint_digest: checkpoint.checkpoint_digest.clone(),
        execution_identity: checkpoint.execution_identity.clone(),
    }
}

fn verify_admission(
    admission: &ExecutableProfileAdmissionV1,
    manifest: &ReplayManifestV7,
) -> Result<(), ControllerError> {
    manifest.validate()?;
    if manifest.execution_identity != *admission.execution_identity()
        || manifest.semantic_contract.semantic_contract_id != *admission.semantic_contract_id()
        || manifest.semantic_contract.manifest != *admission.semantic_contract_manifest()
        || manifest.semantic_contract.rules_manifest != *admission.rules_contract_manifest()
        || manifest
            .semantic_contract
            .content_contract
            .as_ref()
            .is_none_or(|material| material.content_contract_id != *admission.content_contract_id())
    {
        return Err(crate::ReplayExecutionError::ReplayV7Identity.into());
    }
    Ok(())
}

fn verify_checkpoint_admission(
    admission: &ExecutableProfileAdmissionV1,
    checkpoint: &EnvironmentCheckpointV7,
) -> Result<(), ControllerError> {
    checkpoint.validate()?;
    if checkpoint.execution_identity != *admission.execution_identity() {
        return Err(crate::ReplayExecutionError::ReplayV7Identity.into());
    }
    Ok(())
}
