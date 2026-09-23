//! Compatibility delegation for the shared player-safe projection owner.
//!
//! SyntheticRulesCompat retains its historical associated-function surface;
//! the implementation lives in `player_projection.rs` so reference and synthetic
//! transactions validate identical observation/information products.

use mtgml_decision::PlayerDecisionRequestV2;
use mtgml_model::{EpisodeStatus, PlayerId};
use mtgml_observation::{
    ObservationEnvelope, PlayerInformationStateV2, PlayerStepSubmissionV1, PlayerStepV2,
};
use mtgml_state::EngineState;

use super::SyntheticM1EnvironmentBackend;
use crate::endpoint::PlayerEndpointError;

impl SyntheticM1EnvironmentBackend {
    pub(super) fn require_player(&self, perspective: PlayerId) -> Result<(), PlayerEndpointError> {
        self.state
            .core
            .players
            .contains_key(&perspective)
            .then_some(())
            .ok_or(PlayerEndpointError::ServiceUnavailable)
    }

    pub(crate) fn synthetic_observation(
        state: &EngineState,
        perspective: PlayerId,
    ) -> Result<ObservationEnvelope, PlayerEndpointError> {
        crate::player_projection::project_observation(state, perspective)
    }

    pub(crate) fn player_information_state_from_state(
        state: &EngineState,
        perspective: PlayerId,
    ) -> Result<PlayerInformationStateV2, PlayerEndpointError> {
        crate::player_projection::project_information_state(state, perspective)
    }

    pub(crate) fn visible_decision_from_state(
        state: &EngineState,
        perspective: PlayerId,
    ) -> Result<Option<PlayerDecisionRequestV2>, PlayerEndpointError> {
        crate::player_projection::project_visible_decision(state, perspective)
    }

    pub(crate) fn player_step_from_state(
        state: &EngineState,
        perspective: PlayerId,
        status: EpisodeStatus,
        submission: PlayerStepSubmissionV1,
    ) -> Result<PlayerStepV2, PlayerEndpointError> {
        crate::player_projection::project_player_step(state, perspective, status, submission)
    }
}
