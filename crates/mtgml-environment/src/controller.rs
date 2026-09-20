use mtgml_decision::{DecisionResponseV2, PlayerDecisionRequestV2};
use mtgml_model::PlayerId;
use mtgml_observation::{ObservationEnvelope, PlayerInformationStateV2, PlayerStepV2};
use mtgml_replay::AuthoritativeReplayV5;
use std::sync::{Arc, Mutex, MutexGuard};

use crate::checkpoint::EnvironmentCheckpointV5;
use crate::endpoint::PlayerEndpointHandle;
use crate::errors::ControllerError;
use crate::semantic_catalog::{admit_restore, RuntimeSemanticCatalog};

pub trait EnvironmentBackend: Send {
    fn players(&self) -> Vec<PlayerId>;
    fn checkpoint(&self) -> Result<EnvironmentCheckpointV5, ControllerError>;
    fn restore(&mut self, checkpoint: EnvironmentCheckpointV5) -> Result<(), ControllerError>;
    fn fork_boxed(&self) -> Result<Box<dyn EnvironmentBackend>, ControllerError>;
    fn export_replay(&self) -> Result<AuthoritativeReplayV5, ControllerError>;
    fn execute_trusted_response(
        &mut self,
        _actor: PlayerId,
        _response: DecisionResponseV2,
    ) -> Result<mtgml_rules::TransitionResult, ControllerError> {
        Err(ControllerError::Backend(
            "trusted execution is unavailable".into(),
        ))
    }

    /// Rules-owned forced progress without any player response. Backends
    /// without forced-progress support reject with a backend error.
    fn execute_forced_progress(
        &mut self,
    ) -> Result<mtgml_rules::TransitionResult, ControllerError> {
        Err(ControllerError::Backend(
            "forced progress is unavailable".into(),
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

    pub fn checkpoint(&self) -> Result<EnvironmentCheckpointV5, ControllerError> {
        self.lock()?.checkpoint()
    }

    pub fn restore(&self, checkpoint: EnvironmentCheckpointV5) -> Result<(), ControllerError> {
        let catalog = RuntimeSemanticCatalog::production();
        admit_restore(&catalog, &checkpoint)?;
        self.lock()?.restore(checkpoint)
    }

    pub fn fork(&self) -> Result<Self, ControllerError> {
        let backend = self.lock()?.fork_boxed()?;
        Ok(Self {
            inner: Arc::new(Mutex::new(backend)),
        })
    }

    pub fn export_replay(&self) -> Result<AuthoritativeReplayV5, ControllerError> {
        self.lock()?.export_replay()
    }

    pub fn execute_trusted_response(
        &self,
        actor: PlayerId,
        response: DecisionResponseV2,
    ) -> Result<mtgml_rules::TransitionResult, ControllerError> {
        self.lock()?.execute_trusted_response(actor, response)
    }

    pub fn execute_forced_progress(
        &self,
    ) -> Result<mtgml_rules::TransitionResult, ControllerError> {
        self.lock()?.execute_forced_progress()
    }

    /// Executes detached replay input on an internal backend fork and returns
    /// backend/checkpoint-verified traces. Detached replay validation alone does
    /// not establish these execution facts.
    pub fn execute_replay_from_checkpoint(
        &self,
        checkpoint: EnvironmentCheckpointV5,
        replay: AuthoritativeReplayV5,
    ) -> Result<crate::replay::ReplayExecutionReport, ControllerError> {
        let catalog = RuntimeSemanticCatalog::production();
        admit_restore(&catalog, &checkpoint)?;
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
