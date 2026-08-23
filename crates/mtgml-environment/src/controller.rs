use mtgml_decision::{DecisionResponseV2, PlayerDecisionRequestV2};
use mtgml_model::PlayerId;
use mtgml_observation::{ObservationEnvelope, PlayerInformationStateV2, PlayerStepV2};
use mtgml_replay::AuthoritativeReplayV3;
use std::sync::{Arc, Mutex, MutexGuard};

use crate::checkpoint::EnvironmentCheckpointV3;
use crate::endpoint::PlayerEndpointHandle;
use crate::errors::ControllerError;

pub trait EnvironmentBackend: Send {
    fn players(&self) -> Vec<PlayerId>;
    fn checkpoint(&self) -> Result<EnvironmentCheckpointV3, ControllerError>;
    fn restore(&mut self, checkpoint: EnvironmentCheckpointV3) -> Result<(), ControllerError>;
    fn fork_boxed(&self) -> Result<Box<dyn EnvironmentBackend>, ControllerError>;
    fn export_replay(&self) -> Result<AuthoritativeReplayV3, ControllerError>;
    fn execute_trusted_response(
        &mut self,
        _actor: PlayerId,
        _response: DecisionResponseV2,
    ) -> Result<mtgml_rules::TransitionResult, ControllerError> {
        Err(ControllerError::Backend(
            "trusted execution is unavailable".into(),
        ))
    }

    fn player_observation(
        &self,
        perspective: PlayerId,
    ) -> Result<ObservationEnvelope, crate::endpoint::PlayerEndpointError>;
    fn player_information_state(
        &self,
        perspective: PlayerId,
    ) -> Result<PlayerInformationStateV2, crate::endpoint::PlayerEndpointError>;
    fn player_visible_decision(
        &self,
        perspective: PlayerId,
    ) -> Result<Option<PlayerDecisionRequestV2>, crate::endpoint::PlayerEndpointError>;
    fn submit_player_response(
        &mut self,
        perspective: PlayerId,
        response: DecisionResponseV2,
    ) -> Result<PlayerStepV2, crate::endpoint::PlayerEndpointError>;
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

    pub fn bind_player(&self, player: PlayerId) -> Result<PlayerEndpointHandle, ControllerError> {
        if !self.lock()?.players().contains(&player) {
            return Err(ControllerError::UnknownPlayer);
        }
        Ok(PlayerEndpointHandle {
            perspective: player,
            inner: Arc::clone(&self.inner),
        })
    }

    pub fn checkpoint(&self) -> Result<EnvironmentCheckpointV3, ControllerError> {
        self.lock()?.checkpoint()
    }

    pub fn restore(&self, checkpoint: EnvironmentCheckpointV3) -> Result<(), ControllerError> {
        checkpoint
            .validate()
            .map_err(ControllerError::CheckpointValidation)?;
        self.lock()?.restore(checkpoint)
    }

    pub fn fork(&self) -> Result<Self, ControllerError> {
        let backend = self.lock()?.fork_boxed()?;
        Ok(Self {
            inner: Arc::new(Mutex::new(backend)),
        })
    }

    pub fn export_replay(&self) -> Result<AuthoritativeReplayV3, ControllerError> {
        self.lock()?.export_replay()
    }

    pub fn execute_trusted_response(
        &self,
        actor: PlayerId,
        response: DecisionResponseV2,
    ) -> Result<mtgml_rules::TransitionResult, ControllerError> {
        self.lock()?.execute_trusted_response(actor, response)
    }

    pub fn execute_replay_from_checkpoint(
        &self,
        checkpoint: EnvironmentCheckpointV3,
        replay: AuthoritativeReplayV3,
    ) -> Result<crate::replay::ReplayExecutionReport, ControllerError> {
        checkpoint
            .validate()
            .map_err(ControllerError::CheckpointValidation)?;
        let mut backend = self.lock()?.fork_boxed()?;
        backend.restore(checkpoint)?;
        crate::replay::execute_replay(&mut *backend, replay)
    }

    pub(crate) fn lock(
        &self,
    ) -> Result<MutexGuard<'_, Box<dyn EnvironmentBackend>>, ControllerError> {
        self.inner.lock().map_err(|_| ControllerError::Poisoned)
    }
}
