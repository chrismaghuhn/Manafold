#![cfg(not(feature = "historical-conformance-runtime"))]

use mtgml_decision::DecisionResponseV3;
use mtgml_environment::{
    submit_response_bytes, BasicLandReplayV8ExecutionReport, BasicLandRuntimeOutputV8,
    ControllerError, CurrentPlayerStep, EnvironmentBackend, EnvironmentCheckpointV8,
    PlayerEndpoint, PlayerEndpointError, TrustedEnvironmentController,
};
use mtgml_model::PlayerId;
use mtgml_observation::{ObservationEnvelopeV2, PlayerInformationStateV3, PlayerStepV4};
use mtgml_replay::AuthoritativeReplayV8;

struct ProductionAliasProbe {
    step: PlayerStepV4,
}

impl EnvironmentBackend for ProductionAliasProbe {
    fn players(&self) -> Vec<PlayerId> {
        vec![self.step.information_state.perspective]
    }

    fn checkpoint(&self) -> Result<EnvironmentCheckpointV8, ControllerError> {
        Err(ControllerError::SemanticContractUnsupported)
    }

    fn restore(&mut self, _: EnvironmentCheckpointV8) -> Result<(), ControllerError> {
        Err(ControllerError::SemanticContractUnsupported)
    }

    fn fork_boxed(&self) -> Result<Box<dyn EnvironmentBackend>, ControllerError> {
        Err(ControllerError::SemanticContractUnsupported)
    }

    fn export_replay(&self) -> Result<AuthoritativeReplayV8, ControllerError> {
        Err(ControllerError::SemanticContractUnsupported)
    }

    fn execute_replay(
        &self,
        _: AuthoritativeReplayV8,
    ) -> Result<BasicLandReplayV8ExecutionReport, ControllerError> {
        Err(ControllerError::SemanticContractUnsupported)
    }

    fn player_observation(
        &self,
        perspective: PlayerId,
    ) -> Result<ObservationEnvelopeV2, PlayerEndpointError> {
        if perspective != self.step.information_state.perspective {
            return Err(PlayerEndpointError::ServiceUnavailable);
        }
        Ok(self.step.information_state.current_observation.clone())
    }

    fn player_information_state(
        &self,
        perspective: PlayerId,
    ) -> Result<PlayerInformationStateV3, PlayerEndpointError> {
        if perspective != self.step.information_state.perspective {
            return Err(PlayerEndpointError::ServiceUnavailable);
        }
        Ok(self.step.information_state.clone())
    }

    fn player_visible_decision(
        &self,
        perspective: PlayerId,
    ) -> Result<Option<mtgml_decision::PlayerDecisionRequestV4>, PlayerEndpointError> {
        if perspective != self.step.information_state.perspective {
            return Err(PlayerEndpointError::ServiceUnavailable);
        }
        Ok(self.step.next_decision.clone())
    }

    fn submit_player_response(
        &mut self,
        perspective: PlayerId,
        _: DecisionResponseV3,
    ) -> Result<PlayerStepV4, PlayerEndpointError> {
        if perspective != self.step.information_state.perspective {
            return Err(PlayerEndpointError::ServiceUnavailable);
        }
        Ok(self.step.clone())
    }

    fn execute_transition(
        &mut self,
        _: PlayerId,
        _: DecisionResponseV3,
    ) -> Result<BasicLandRuntimeOutputV8, PlayerEndpointError> {
        Err(PlayerEndpointError::ServiceUnavailable)
    }
}

#[test]
fn public_current_endpoint_and_wire_boundary_return_player_step_v4() {
    let step: PlayerStepV4 = serde_json::from_str(include_str!(
        "../../../schemas/examples/player-step-v4.json"
    ))
    .unwrap();
    let expected = step.clone();
    let perspective = step.information_state.perspective;
    let controller = TrustedEnvironmentController::new(ProductionAliasProbe { step });
    let endpoint = controller.bind_player(perspective).unwrap();
    let _: &dyn PlayerEndpoint = &endpoint;

    let response: DecisionResponseV3 = serde_json::from_str(include_str!(
        "../../../schemas/examples/decision-response-v3.json"
    ))
    .unwrap();
    let bytes = mtgml_wire::encode_canonical(&response).unwrap();
    let current_step: CurrentPlayerStep = submit_response_bytes(&endpoint, &bytes).unwrap();
    let successor_step: PlayerStepV4 = current_step;
    assert_eq!(successor_step, expected);
}
