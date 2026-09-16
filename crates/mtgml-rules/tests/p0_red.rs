use mtgml_decision::{DecisionAnswerV2, DecisionResponseV2, DECISION_RESPONSE_V2_SCHEMA};
use mtgml_model::{CandidateIdV1, PlayerDecisionIdV1, PlayerId, StateRevision};
use mtgml_random::RootSeed256;
use mtgml_rules::{
    validate_transition_contract, RulesKernel, SyntheticM1RulesKernel, TransitionViolation,
};
use mtgml_state::{
    construct_synthetic_engine_state, StateDelta, SyntheticResetInputs, SyntheticV4Setup,
    TurnPosition,
};

fn state() -> mtgml_state::EngineState {
    construct_synthetic_engine_state(SyntheticResetInputs {
        players: [PlayerId(1), PlayerId(2)],
        root_seed: RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap(),
        setup: SyntheticV4Setup::m2_compatibility(),
    })
    .unwrap()
}

fn response() -> DecisionResponseV2 {
    DecisionResponseV2 {
        schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
        player_decision_id: PlayerDecisionIdV1(1),
        state_revision: StateRevision(0),
        answer: DecisionAnswerV2::SelectOne {
            candidate_id: CandidateIdV1(0),
        },
    }
}

#[test]
fn p0_normal_transition_rejects_unexplained_turn_position_mutation() {
    let before = state();
    let mut kernel = SyntheticM1RulesKernel;
    let mut transition = kernel.apply(&before, PlayerId(1), &response()).unwrap();
    transition.next_state.core.position = TurnPosition::PrecombatMain;
    transition.delta = StateDelta::between(
        &before,
        &transition.next_state,
        transition.delta.audit.clone(),
    )
    .unwrap();

    let error = validate_transition_contract(&before, &transition).unwrap_err();
    assert!(matches!(error, TransitionViolation::UnexplainedMutation));
}
