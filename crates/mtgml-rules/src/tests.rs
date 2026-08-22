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

#[test]
fn synthetic_rejection_matrix_preserves_complete_nonmutation() {
    let state = synthetic_state();
    let wrong_domain = {
        let mut changed = state.clone();
        let pending = changed.execution.pending_decision.as_mut().unwrap();
        pending.request.decision = mtgml_decision::DecisionDomainV2::ChooseMany {
            minimum: 1,
            maximum: 1,
        };
        changed
    };

    let cases = vec![
        response(1, 0),
        DecisionResponseV2 {
            schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
            player_decision_id: PlayerDecisionIdV1(1),
            state_revision: StateRevision(0),
            answer: DecisionAnswerV2::SelectMany {
                candidate_ids: vec![CandidateIdV1(0)],
            },
        },
        DecisionResponseV2 {
            schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
            player_decision_id: PlayerDecisionIdV1(1),
            state_revision: StateRevision(0),
            answer: DecisionAnswerV2::ChooseNumber { value: 0 },
        },
        DecisionResponseV2 {
            schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
            player_decision_id: PlayerDecisionIdV1(2),
            state_revision: StateRevision(0),
            answer: DecisionAnswerV2::SelectOne {
                candidate_id: CandidateIdV1(0),
            },
        },
        response(0, 9),
    ];

    for case in &cases {
        for state in [&state, &wrong_domain] {
            let mut kernel = SyntheticM1RulesKernel;
            let result = kernel.apply(state, PlayerId(1), case).unwrap();
            assert!(!result.accepted);
            assert_eq!(result.next_state, *state);
            assert!(result.events.is_empty());
            assert!(result.delta.audit.is_empty());
            assert_eq!(result.delta.before_revision, result.delta.after_revision);
            assert_eq!(result.delta.before_digest, result.delta.after_digest);
            validate_transition_contract(state, &result).unwrap();
        }
    }
}

#[test]
fn deterministic_services_repeat_exact_transition_result() {
    let first = synthetic_state();
    let second = synthetic_state();
    let mut kernel_a = SyntheticM1RulesKernel;
    let mut kernel_b = SyntheticM1RulesKernel;
    let left = kernel_a
        .apply(&first, PlayerId(1), &response(0, 0))
        .unwrap();
    let right = kernel_b
        .apply(&second, PlayerId(1), &response(0, 0))
        .unwrap();
    assert_eq!(left, right);
    assert_eq!(
        left.next_state.digest().unwrap(),
        right.next_state.digest().unwrap()
    );
}

#[test]
fn deterministic_services_isolate_unrelated_stream_cursors() {
    use mtgml_random::{RandomStreamCursorV1, RandomStreamKeyV1, RandomStreamKindV1};
    let baseline_state = synthetic_state();
    let mut isolated_state = synthetic_state();
    isolated_state.random.streams.insert(
        RandomStreamKeyV1::player_scoped(RandomStreamKindV1::SyntheticM1, 2),
        RandomStreamCursorV1 {
            next_raw_u64: 987_654_321,
        },
    );
    let mut kernel_a = SyntheticM1RulesKernel;
    let mut kernel_b = SyntheticM1RulesKernel;
    let baseline = kernel_a
        .apply(&baseline_state, PlayerId(1), &response(0, 0))
        .unwrap();
    let isolated = kernel_b
        .apply(&isolated_state, PlayerId(1), &response(0, 0))
        .unwrap();
    // The unrelated player-scoped cursor must not influence the transition:
    // identical events, audit trace, and consumed global-stream progression.
    assert_eq!(baseline.events, isolated.events);
    assert_eq!(baseline.delta.audit, isolated.delta.audit);
    let global =
        mtgml_random::RandomStreamKeyV1::global(mtgml_random::RandomStreamKindV1::SyntheticM1);
    assert_eq!(
        baseline
            .next_state
            .random
            .lookup_stream(&global)
            .unwrap()
            .next_raw_u64,
        isolated
            .next_state
            .random
            .lookup_stream(&global)
            .unwrap()
            .next_raw_u64
    );
}

#[test]
fn rng_exhaustion_is_a_typed_internal_failure_without_input_mutation() {
    use mtgml_random::{RandomStreamCursorV1, RandomStreamKeyV1, RandomStreamKindV1};
    let mut state = synthetic_state();
    let key = RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1);
    state
        .random
        .set_cursor(
            &key,
            RandomStreamCursorV1 {
                next_raw_u64: u64::MAX,
            },
        )
        .unwrap();
    let before = state.clone();
    let mut kernel = SyntheticM1RulesKernel;
    let error = kernel
        .apply(&state, PlayerId(1), &response(0, 0))
        .unwrap_err();
    assert!(matches!(error, KernelExecutionError::Random(_)));
    assert_eq!(state, before, "kernel input must never be mutated");
}

#[test]
fn effect_allocator_exhaustion_is_a_typed_internal_failure_before_rng() {
    use mtgml_model::EffectInstanceId;
    use mtgml_random::{RandomStreamKeyV1, RandomStreamKindV1};
    let mut state = synthetic_state();
    state.allocators.next_effect_id = EffectInstanceId(u64::MAX);
    let key = RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1);
    let cursor_before = state.random.lookup_stream(&key).unwrap().next_raw_u64;
    let mut kernel = SyntheticM1RulesKernel;
    let error = kernel
        .apply(&state, PlayerId(1), &response(0, 0))
        .unwrap_err();
    assert!(matches!(error, KernelExecutionError::IdentityAllocation(_)));
    assert_eq!(
        state.random.lookup_stream(&key).unwrap().next_raw_u64,
        cursor_before,
        "exhaustion must fail before any randomness is consumed"
    );
}

#[test]
fn sequential_event_delta_audit_rejects_tampered_products() {
    use crate::{AuthoritativeRuleEventKind, TransitionViolation};
    use mtgml_model::RuleEventId;

    let state = synthetic_state();
    let mut kernel = SyntheticM1RulesKernel;
    let result = kernel.apply(&state, PlayerId(1), &response(0, 0)).unwrap();
    assert_eq!(result.events.len(), 4);
    validate_transition_contract(&state, &result).unwrap();

    // Dropping an event breaks event/delta audit agreement.
    let mut tampered = result.clone();
    tampered.events.remove(0);
    assert!(matches!(
        validate_transition_contract(&state, &tampered),
        Err(TransitionViolation::EventDeltaMismatch)
            | Err(TransitionViolation::LifeChange)
            | Err(TransitionViolation::EventIdentity)
    ));

    // Reordering the life trace violates cursor progression.
    let mut tampered = result.clone();
    tampered.events.swap(0, 1);
    assert!(validate_transition_contract(&state, &tampered).is_err());

    // Event identity must be dense from the allocator cursor.
    let mut tampered = result.clone();
    tampered.events[2].event_id = RuleEventId(99);
    assert!(matches!(
        validate_transition_contract(&state, &tampered),
        Err(TransitionViolation::EventIdentity)
    ));

    // A divergent audit trace is rejected.
    let mut tampered = result.clone();
    tampered.delta.audit.clear();
    assert!(matches!(
        validate_transition_contract(&state, &tampered),
        Err(TransitionViolation::EventDeltaMismatch)
    ));

    // A random sample event must match the authoritative sampler.
    let mut tampered = result.clone();
    let mut second_kernel = SyntheticM1RulesKernel;
    let fresh = second_kernel
        .apply(&synthetic_state(), PlayerId(1), &response(0, 0))
        .unwrap();
    tampered.events = fresh.events.clone();
    if let AuthoritativeRuleEventKind::RandomValueSampled { value, .. } =
        &mut tampered.events[2].event
    {
        *value = value.wrapping_add(1);
    }
    // Keep the audit consistent so the sampler itself is what fails.
    tampered.delta.audit = tampered
        .events
        .iter()
        .map(|event| event.event.semantic_delta())
        .collect();
    assert!(matches!(
        validate_transition_contract(&state, &tampered),
        Err(TransitionViolation::Randomness)
    ));

    // A final state that disagrees with the event trace is rejected.
    let mut tampered = result.clone();
    tampered
        .next_state
        .core
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .life = 37;
    tampered.delta = mtgml_state::StateDelta::between(
        &state,
        &tampered.next_state,
        result
            .events
            .iter()
            .map(|event| event.event.semantic_delta())
            .collect(),
    )
    .unwrap();
    assert!(validate_transition_contract(&state, &tampered).is_err());

    // A rejected product that claims mutation is rejected.
    let mut tampered = result.clone();
    tampered.accepted = false;
    assert!(matches!(
        validate_transition_contract(&state, &tampered),
        Err(TransitionViolation::RejectedMutation)
    ));
}
