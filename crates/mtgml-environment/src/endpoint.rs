use mtgml_decision::{DecisionResponse, PlayerDecisionRequest};
use mtgml_model::PlayerId;
use mtgml_observation::{InformationStateEnvelope, ObservationEnvelope, PlayerStep};
use std::sync::MutexGuard;
use thiserror::Error;

use crate::controller::SharedBackend;

#[derive(Clone)]
pub struct PlayerEndpointHandle {
    pub(crate) perspective: PlayerId,
    pub(crate) inner: SharedBackend,
}

pub trait PlayerEndpoint: Send + Sync {
    fn perspective(&self) -> PlayerId;
    fn observation(&self) -> Result<ObservationEnvelope, PlayerApiError>;
    fn information_state(&self) -> Result<InformationStateEnvelope, PlayerApiError>;
    fn visible_decision(&self) -> Result<Option<PlayerDecisionRequest>, PlayerApiError>;
    fn submit(&self, response: DecisionResponse) -> Result<PlayerStep, PlayerApiError>;
}

impl PlayerEndpointHandle {
    fn lock(
        &self,
    ) -> Result<MutexGuard<'_, Box<dyn crate::controller::EnvironmentBackend>>, PlayerApiError>
    {
        self.inner.lock().map_err(|_| PlayerApiError::Unavailable)
    }
}

impl PlayerEndpoint for PlayerEndpointHandle {
    fn perspective(&self) -> PlayerId {
        self.perspective
    }

    fn observation(&self) -> Result<ObservationEnvelope, PlayerApiError> {
        self.lock()?.player_observation(self.perspective)
    }

    fn information_state(&self) -> Result<InformationStateEnvelope, PlayerApiError> {
        self.lock()?.player_information_state(self.perspective)
    }

    fn visible_decision(&self) -> Result<Option<PlayerDecisionRequest>, PlayerApiError> {
        self.lock()?.player_visible_decision(self.perspective)
    }

    fn submit(&self, response: DecisionResponse) -> Result<PlayerStep, PlayerApiError> {
        self.lock()?
            .submit_player_response(self.perspective, response)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum PlayerApiError {
    #[error("no decision is available to this player")]
    NoVisibleDecision,
    #[error("the response is stale")]
    StaleResponse,
    #[error("the selection is invalid")]
    InvalidSelection,
    #[error("the episode is complete")]
    EpisodeComplete,
    #[error("the endpoint is unavailable")]
    Unavailable,
}
