//! Current player endpoint over PlayerStep V4 and Decision V4.

use mtgml_decision::{DecisionResponseV3, PlayerDecisionRequestV4};
use mtgml_model::PlayerId;
use mtgml_observation::{ObservationEnvelopeV2, PlayerInformationStateV3, PlayerStepV4};
use std::sync::MutexGuard;

use crate::controller_successor::SharedBackend;
pub use crate::errors::PlayerEndpointError;

#[derive(Clone)]
pub struct PlayerEndpointHandle {
    pub(crate) perspective: PlayerId,
    pub(crate) inner: SharedBackend,
}

pub trait PlayerEndpoint: Send + Sync {
    fn perspective(&self) -> PlayerId;
    fn observation(&self) -> Result<ObservationEnvelopeV2, PlayerEndpointError>;
    fn information_state(&self) -> Result<PlayerInformationStateV3, PlayerEndpointError>;
    fn visible_decision(&self) -> Result<Option<PlayerDecisionRequestV4>, PlayerEndpointError>;
    fn submit(&self, response: DecisionResponseV3) -> Result<PlayerStepV4, PlayerEndpointError>;
}

impl PlayerEndpointHandle {
    fn lock(
        &self,
    ) -> Result<
        MutexGuard<'_, Box<dyn crate::controller_successor::EnvironmentBackend>>,
        PlayerEndpointError,
    > {
        self.inner
            .lock()
            .map_err(|_| PlayerEndpointError::ServiceUnavailable)
    }
}

impl PlayerEndpoint for PlayerEndpointHandle {
    fn perspective(&self) -> PlayerId {
        self.perspective
    }

    fn observation(&self) -> Result<ObservationEnvelopeV2, PlayerEndpointError> {
        self.lock()?.player_observation(self.perspective)
    }

    fn information_state(&self) -> Result<PlayerInformationStateV3, PlayerEndpointError> {
        self.lock()?.player_information_state(self.perspective)
    }

    fn visible_decision(&self) -> Result<Option<PlayerDecisionRequestV4>, PlayerEndpointError> {
        self.lock()?.player_visible_decision(self.perspective)
    }

    fn submit(&self, response: DecisionResponseV3) -> Result<PlayerStepV4, PlayerEndpointError> {
        self.lock()?
            .submit_player_response(self.perspective, response)
    }
}
