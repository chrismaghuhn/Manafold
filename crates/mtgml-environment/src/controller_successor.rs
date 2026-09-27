//! Current successor environment/controller boundary.

use std::sync::{Arc, Mutex, MutexGuard};

use mtgml_decision::DecisionResponseV2;
use mtgml_model::PlayerId;
use mtgml_observation::{ObservationEnvelope, PlayerInformationStateV2, PlayerStepV3};
use mtgml_replay::AuthoritativeReplayV7;

use crate::replay_v7_execution::ReplayV7ExecutionReport;
use crate::successor_transaction::SuccessorTransactionOutput;
use crate::{
    ControllerError, EnvironmentCheckpointV7, PlayerEndpointError, SuccessorEnvironmentRuntime,
};

pub trait EnvironmentBackend: Send {
    fn players(&self) -> Vec<PlayerId>;
    fn checkpoint(&self) -> Result<EnvironmentCheckpointV7, ControllerError>;
    fn restore(&mut self, checkpoint: EnvironmentCheckpointV7) -> Result<(), ControllerError>;
    fn fork_boxed(&self) -> Result<Box<dyn EnvironmentBackend>, ControllerError>;
    fn export_replay(&self) -> Result<AuthoritativeReplayV7, ControllerError>;
    fn execute_replay(
        &self,
        replay: AuthoritativeReplayV7,
    ) -> Result<ReplayV7ExecutionReport, ControllerError>;
    fn player_observation(
        &self,
        perspective: PlayerId,
    ) -> Result<ObservationEnvelope, PlayerEndpointError>;
    fn player_information_state(
        &self,
        perspective: PlayerId,
    ) -> Result<PlayerInformationStateV2, PlayerEndpointError>;
    fn player_visible_decision(
        &self,
        perspective: PlayerId,
    ) -> Result<Option<mtgml_decision::PlayerDecisionRequestV3>, PlayerEndpointError>;
    fn submit_player_response(
        &mut self,
        perspective: PlayerId,
        response: DecisionResponseV2,
    ) -> Result<PlayerStepV3, PlayerEndpointError>;
    fn execute_transition(
        &mut self,
        actor: PlayerId,
        response: DecisionResponseV2,
    ) -> Result<SuccessorTransactionOutput, PlayerEndpointError>;
}

impl EnvironmentBackend for SuccessorEnvironmentRuntime {
    fn players(&self) -> Vec<PlayerId> {
        SuccessorEnvironmentRuntime::players(self)
    }

    fn checkpoint(&self) -> Result<EnvironmentCheckpointV7, ControllerError> {
        SuccessorEnvironmentRuntime::checkpoint(self)
    }

    fn restore(&mut self, checkpoint: EnvironmentCheckpointV7) -> Result<(), ControllerError> {
        SuccessorEnvironmentRuntime::restore(self, checkpoint)
    }

    fn fork_boxed(&self) -> Result<Box<dyn EnvironmentBackend>, ControllerError> {
        Ok(Box::new(SuccessorEnvironmentRuntime::fork(self)?))
    }

    fn export_replay(&self) -> Result<AuthoritativeReplayV7, ControllerError> {
        SuccessorEnvironmentRuntime::export_replay(self)
    }

    fn execute_replay(
        &self,
        replay: AuthoritativeReplayV7,
    ) -> Result<ReplayV7ExecutionReport, ControllerError> {
        SuccessorEnvironmentRuntime::execute_replay(self, replay)
    }

    fn player_observation(
        &self,
        perspective: PlayerId,
    ) -> Result<ObservationEnvelope, PlayerEndpointError> {
        SuccessorEnvironmentRuntime::observation(self, perspective)
    }

    fn player_information_state(
        &self,
        perspective: PlayerId,
    ) -> Result<PlayerInformationStateV2, PlayerEndpointError> {
        SuccessorEnvironmentRuntime::information_state(self, perspective)
    }

    fn player_visible_decision(
        &self,
        perspective: PlayerId,
    ) -> Result<Option<mtgml_decision::PlayerDecisionRequestV3>, PlayerEndpointError> {
        SuccessorEnvironmentRuntime::visible_decision(self, perspective)
    }

    fn submit_player_response(
        &mut self,
        perspective: PlayerId,
        response: DecisionResponseV2,
    ) -> Result<PlayerStepV3, PlayerEndpointError> {
        let output = SuccessorEnvironmentRuntime::submit(self, perspective, response)?;
        output
            .player_steps
            .get(&perspective)
            .cloned()
            .ok_or(PlayerEndpointError::ServiceUnavailable)
    }

    fn execute_transition(
        &mut self,
        actor: PlayerId,
        response: DecisionResponseV2,
    ) -> Result<SuccessorTransactionOutput, PlayerEndpointError> {
        SuccessorEnvironmentRuntime::submit(self, actor, response)
    }
}

pub(crate) type SharedBackend = Arc<Mutex<Box<dyn EnvironmentBackend>>>;

#[derive(Clone)]
pub struct TrustedEnvironmentController {
    inner: SharedBackend,
}

impl TrustedEnvironmentController {
    pub fn new(backend: impl EnvironmentBackend + 'static) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Box::new(backend))),
        }
    }

    pub fn bind_player(
        &self,
        player: PlayerId,
    ) -> Result<crate::endpoint_successor::PlayerEndpointHandle, ControllerError> {
        if !self.lock()?.players().contains(&player) {
            return Err(ControllerError::UnknownPlayer);
        }
        Ok(crate::endpoint_successor::PlayerEndpointHandle {
            perspective: player,
            inner: Arc::clone(&self.inner),
        })
    }

    pub fn checkpoint(&self) -> Result<EnvironmentCheckpointV7, ControllerError> {
        self.lock()?.checkpoint()
    }

    pub fn restore(&self, checkpoint: EnvironmentCheckpointV7) -> Result<(), ControllerError> {
        self.lock()?.restore(checkpoint)
    }

    pub fn fork(&self) -> Result<Self, ControllerError> {
        Ok(Self {
            inner: Arc::new(Mutex::new(self.lock()?.fork_boxed()?)),
        })
    }

    pub fn export_replay(&self) -> Result<AuthoritativeReplayV7, ControllerError> {
        self.lock()?.export_replay()
    }

    pub fn execute_replay(
        &self,
        replay: AuthoritativeReplayV7,
    ) -> Result<ReplayV7ExecutionReport, ControllerError> {
        self.lock()?.execute_replay(replay)
    }

    pub(crate) fn lock(
        &self,
    ) -> Result<MutexGuard<'_, Box<dyn EnvironmentBackend>>, ControllerError> {
        self.inner.lock().map_err(|_| ControllerError::Poisoned)
    }
}
