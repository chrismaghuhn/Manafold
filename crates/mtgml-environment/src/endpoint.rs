use mtgml_decision::{DecisionResponseV2, PlayerDecisionRequestV2};
use mtgml_model::PlayerId;
use mtgml_observation::{ObservationEnvelope, PlayerInformationStateV2, PlayerStepV2};
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
    fn information_state(&self) -> Result<PlayerInformationStateV2, PlayerApiError>;
    fn visible_decision(&self) -> Result<Option<PlayerDecisionRequestV2>, PlayerApiError>;
    fn submit(&self, response: DecisionResponseV2) -> Result<PlayerStepV2, PlayerApiError>;
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

    fn information_state(&self) -> Result<PlayerInformationStateV2, PlayerApiError> {
        self.lock()?.player_information_state(self.perspective)
    }

    fn visible_decision(&self) -> Result<Option<PlayerDecisionRequestV2>, PlayerApiError> {
        self.lock()?.player_visible_decision(self.perspective)
    }

    fn submit(&self, response: DecisionResponseV2) -> Result<PlayerStepV2, PlayerApiError> {
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
