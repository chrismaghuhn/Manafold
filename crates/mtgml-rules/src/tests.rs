use super::*;
use mtgml_decision::{DecisionAnswerV2, DecisionResponseV2, DECISION_RESPONSE_V2_SCHEMA};
use mtgml_model::{CandidateIdV1, PlayerDecisionIdV1, PlayerId, StateRevision};
use mtgml_random::RootSeed256;
use mtgml_state::{construct_synthetic_engine_state, SyntheticResetInputs};

fn synthetic_state() -> mtgml_state::EngineState {
    construct_synthetic_engine_state(SyntheticResetInputs {
        players: [PlayerId(1), PlayerId(2)],
        root_seed: RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap(),
    })
    .unwrap()
}

fn response(candidate_id: u32, revision: u64) -> DecisionResponseV2 {
    DecisionResponseV2 {
        schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
        player_decision_id: PlayerDecisionIdV1(1),
        state_revision: StateRevision(revision),
        answer: DecisionAnswerV2::SelectOne {
            candidate_id: CandidateIdV1(candidate_id),
        },
    }
}

#[test]
fn synthetic_m2_choose_one_returns_authoritative_transition_product() {
    let state = synthetic_state();
    let mut kernel = SyntheticM1RulesKernel;
    let result = kernel.apply(&state, PlayerId(1), &response(0, 0)).unwrap();

    assert!(result.accepted);
    assert_eq!(result.next_state.revision, StateRevision(1));
    assert_eq!(result.next_state.core.players[&PlayerId(1)].life, 38);
    assert!(result.next_state.execution.pending_decision.is_none());
    assert!(result.next_decision.is_none());
    assert_eq!(result.events.len(), 4);
    assert_eq!(result.delta.apply(&state).unwrap(), result.next_state);
    assert_eq!(result.delta.before_digest, state.digest().unwrap());
    assert_eq!(
        result.delta.after_digest,
        result.next_state.digest().unwrap()
    );
    validate_transition_contract(&state, &result).unwrap();
}

#[test]
fn invalid_v2_answer_is_rejected_without_state_mutation() {
    let state = synthetic_state();
    let mut kernel = SyntheticM1RulesKernel;
    let result = kernel.apply(&state, PlayerId(1), &response(1, 0)).unwrap();

    assert!(!result.accepted);
    assert_eq!(result.next_state, state);
    assert!(result.events.is_empty());
    assert_eq!(
        result.next_decision,
        state.execution.pending_decision.clone().map(|p| p.request)
    );
    validate_transition_contract(&state, &result).unwrap();
}

#[test]
fn wrong_actor_and_stale_revision_fail_closed() {
    let state = synthetic_state();
    let mut kernel = SyntheticM1RulesKernel;
    let wrong_actor = kernel.apply(&state, PlayerId(2), &response(0, 0)).unwrap();
    assert!(!wrong_actor.accepted);
    assert_eq!(wrong_actor.next_state, state);

    let stale = kernel.apply(&state, PlayerId(1), &response(0, 1)).unwrap();
    assert!(!stale.accepted);
    assert_eq!(stale.next_state, state);
}
