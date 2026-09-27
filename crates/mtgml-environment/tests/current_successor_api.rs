#![cfg(not(feature = "historical-conformance-runtime"))]

use mtgml_decision::{DecisionAnswerV2, DecisionResponseV2};
use mtgml_environment::replay_v7_execution::ReplayV7ExecutionReport;
use mtgml_environment::successor_transaction::SuccessorTransactionOutput;
use mtgml_environment::{
    submit_response_bytes, ControllerError, CurrentPlayerStep, EnvironmentBackend,
    EnvironmentCheckpointV7, PlayerEndpoint, PlayerEndpointError, TrustedEnvironmentController,
};
use mtgml_model::{CandidateIdV1, PlayerId};
use mtgml_observation::{ObservationEnvelope, PlayerInformationStateV2, PlayerStepV3};
use mtgml_replay::AuthoritativeReplayV7;

struct ProductionAliasProbe {
    step: PlayerStepV3,
}

impl EnvironmentBackend for ProductionAliasProbe {
    fn players(&self) -> Vec<PlayerId> {
        vec![self.step.information_state.perspective]
    }

    fn checkpoint(&self) -> Result<EnvironmentCheckpointV7, ControllerError> {
        Err(ControllerError::SemanticContractUnsupported)
    }

    fn restore(&mut self, _: EnvironmentCheckpointV7) -> Result<(), ControllerError> {
        Err(ControllerError::SemanticContractUnsupported)
    }

    fn fork_boxed(&self) -> Result<Box<dyn EnvironmentBackend>, ControllerError> {
        Err(ControllerError::SemanticContractUnsupported)
    }

    fn export_replay(&self) -> Result<AuthoritativeReplayV7, ControllerError> {
        Err(ControllerError::SemanticContractUnsupported)
    }

    fn execute_replay(
        &self,
        _: AuthoritativeReplayV7,
    ) -> Result<ReplayV7ExecutionReport, ControllerError> {
        Err(ControllerError::SemanticContractUnsupported)
    }

    fn player_observation(
        &self,
        perspective: PlayerId,
    ) -> Result<ObservationEnvelope, PlayerEndpointError> {
        if perspective != self.step.information_state.perspective {
            return Err(PlayerEndpointError::ServiceUnavailable);
        }
        Ok(self.step.information_state.current_observation.clone())
    }

    fn player_information_state(
        &self,
        perspective: PlayerId,
    ) -> Result<PlayerInformationStateV2, PlayerEndpointError> {
        if perspective != self.step.information_state.perspective {
            return Err(PlayerEndpointError::ServiceUnavailable);
        }
        Ok(self.step.information_state.clone())
    }

    fn player_visible_decision(
        &self,
        perspective: PlayerId,
    ) -> Result<Option<mtgml_decision::PlayerDecisionRequestV3>, PlayerEndpointError> {
        if perspective != self.step.information_state.perspective {
            return Err(PlayerEndpointError::ServiceUnavailable);
        }
        Ok(self.step.next_decision.clone())
    }

    fn submit_player_response(
        &mut self,
        perspective: PlayerId,
        _: DecisionResponseV2,
    ) -> Result<PlayerStepV3, PlayerEndpointError> {
        if perspective != self.step.information_state.perspective {
            return Err(PlayerEndpointError::ServiceUnavailable);
        }
        Ok(self.step.clone())
    }

    fn execute_transition(
        &mut self,
        _: PlayerId,
        _: DecisionResponseV2,
    ) -> Result<SuccessorTransactionOutput, PlayerEndpointError> {
        Err(PlayerEndpointError::ServiceUnavailable)
    }
}

#[test]
fn public_current_endpoint_and_wire_boundary_return_player_step_v3() {
    let step: PlayerStepV3 = serde_json::from_str(include_str!(
        "../../../schemas/examples/player-step-v3-no-next-decision.json"
    ))
    .unwrap();
    let expected = step.clone();
    let perspective = step.information_state.perspective;
    let controller = TrustedEnvironmentController::new(ProductionAliasProbe { step });
    let endpoint = controller.bind_player(perspective).unwrap();
    let _: &dyn PlayerEndpoint = &endpoint;

    let response = DecisionResponseV2 {
        schema_version: mtgml_decision::DECISION_RESPONSE_V2_SCHEMA.to_owned(),
        player_decision_id: mtgml_model::PlayerDecisionIdV1(1),
        state_revision: expected.information_state.state_revision,
        answer: DecisionAnswerV2::SelectOne {
            candidate_id: CandidateIdV1(0),
        },
    };
    let bytes = mtgml_wire::encode_canonical(&response).unwrap();
    let current_step: CurrentPlayerStep = submit_response_bytes(&endpoint, &bytes).unwrap();
    let successor_step: PlayerStepV3 = current_step;
    assert_eq!(successor_step, expected);
}
