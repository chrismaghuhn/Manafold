//! Current V8 environment/controller boundary.

use std::sync::{Arc, Mutex, MutexGuard};

use mtgml_model::PlayerId;
use mtgml_replay::AuthoritativeReplayV8;

use crate::{
    BasicLandEnvironmentRuntimeV8, BasicLandReplayV8ExecutionReport, BasicLandRuntimeOutputV8,
    ControllerError, EnvironmentCheckpointV8, PlayerEndpointError,
};

pub trait EnvironmentBackend: Send {
    fn players(&self) -> Vec<PlayerId>;
    fn checkpoint(&self) -> Result<EnvironmentCheckpointV8, ControllerError>;
    fn restore(&mut self, checkpoint: EnvironmentCheckpointV8) -> Result<(), ControllerError>;
    fn fork_boxed(&self) -> Result<Box<dyn EnvironmentBackend>, ControllerError>;
    fn export_replay(&self) -> Result<AuthoritativeReplayV8, ControllerError>;
    fn execute_replay(
        &self,
        replay: AuthoritativeReplayV8,
    ) -> Result<BasicLandReplayV8ExecutionReport, ControllerError>;
    fn player_observation(
        &self,
        perspective: PlayerId,
    ) -> Result<mtgml_observation::ObservationEnvelope, PlayerEndpointError>;
    fn player_information_state(
        &self,
        perspective: PlayerId,
    ) -> Result<mtgml_observation::PlayerInformationState, PlayerEndpointError>;
    fn player_visible_decision(
        &self,
        perspective: PlayerId,
    ) -> Result<Option<mtgml_decision::PlayerDecisionRequestV4>, PlayerEndpointError>;
    fn submit_player_response(
        &mut self,
        perspective: PlayerId,
        response: mtgml_decision::DecisionResponseV3,
    ) -> Result<mtgml_observation::PlayerStepV4, PlayerEndpointError>;
    fn execute_transition(
        &mut self,
        actor: PlayerId,
        response: mtgml_decision::DecisionResponseV3,
    ) -> Result<BasicLandRuntimeOutputV8, PlayerEndpointError>;
}

impl EnvironmentBackend for BasicLandEnvironmentRuntimeV8 {
    fn players(&self) -> Vec<PlayerId> {
        BasicLandEnvironmentRuntimeV8::players(self)
    }

    fn checkpoint(&self) -> Result<EnvironmentCheckpointV8, ControllerError> {
        BasicLandEnvironmentRuntimeV8::checkpoint(self)
    }

    fn restore(&mut self, checkpoint: EnvironmentCheckpointV8) -> Result<(), ControllerError> {
        BasicLandEnvironmentRuntimeV8::restore(self, checkpoint)
    }

    fn fork_boxed(&self) -> Result<Box<dyn EnvironmentBackend>, ControllerError> {
        Ok(Box::new(BasicLandEnvironmentRuntimeV8::fork(self)?))
    }

    fn export_replay(&self) -> Result<AuthoritativeReplayV8, ControllerError> {
        BasicLandEnvironmentRuntimeV8::export_replay(self)
    }

    fn execute_replay(
        &self,
        replay: AuthoritativeReplayV8,
    ) -> Result<BasicLandReplayV8ExecutionReport, ControllerError> {
        BasicLandEnvironmentRuntimeV8::execute_replay(self, replay)
    }

    fn player_observation(
        &self,
        perspective: PlayerId,
    ) -> Result<mtgml_observation::ObservationEnvelope, PlayerEndpointError> {
        BasicLandEnvironmentRuntimeV8::information_state(self, perspective)
            .map(|state| state.current_observation)
    }

    fn player_information_state(
        &self,
        perspective: PlayerId,
    ) -> Result<mtgml_observation::PlayerInformationState, PlayerEndpointError> {
        BasicLandEnvironmentRuntimeV8::information_state(self, perspective)
    }

    fn player_visible_decision(
        &self,
        perspective: PlayerId,
    ) -> Result<Option<mtgml_decision::PlayerDecisionRequestV4>, PlayerEndpointError> {
        BasicLandEnvironmentRuntimeV8::visible_decision(self, perspective)
    }

    fn submit_player_response(
        &mut self,
        perspective: PlayerId,
        response: mtgml_decision::DecisionResponseV3,
    ) -> Result<mtgml_observation::PlayerStepV4, PlayerEndpointError> {
        BasicLandEnvironmentRuntimeV8::submit(self, perspective, response)?
            .player_steps
            .get(&perspective)
            .cloned()
            .ok_or(PlayerEndpointError::ServiceUnavailable)
    }

    fn execute_transition(
        &mut self,
        actor: PlayerId,
        response: mtgml_decision::DecisionResponseV3,
    ) -> Result<BasicLandRuntimeOutputV8, PlayerEndpointError> {
        BasicLandEnvironmentRuntimeV8::submit(self, actor, response)
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

    pub fn checkpoint(&self) -> Result<EnvironmentCheckpointV8, ControllerError> {
        self.lock()?.checkpoint()
    }

    pub fn restore(&self, checkpoint: EnvironmentCheckpointV8) -> Result<(), ControllerError> {
        self.lock()?.restore(checkpoint)
    }

    pub fn fork(&self) -> Result<Self, ControllerError> {
        Ok(Self {
            inner: Arc::new(Mutex::new(self.lock()?.fork_boxed()?)),
        })
    }

    pub fn export_replay(&self) -> Result<AuthoritativeReplayV8, ControllerError> {
        self.lock()?.export_replay()
    }

    pub fn execute_replay(
        &self,
        replay: AuthoritativeReplayV8,
    ) -> Result<BasicLandReplayV8ExecutionReport, ControllerError> {
        self.lock()?.execute_replay(replay)
    }

    pub(crate) fn lock(
        &self,
    ) -> Result<MutexGuard<'_, Box<dyn EnvironmentBackend>>, ControllerError> {
        self.inner.lock().map_err(|_| ControllerError::Poisoned)
    }
}
