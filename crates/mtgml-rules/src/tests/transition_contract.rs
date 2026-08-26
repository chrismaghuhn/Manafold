// Ownership fragment: transition-contract and nonmutation evidence. Included lexically by tests.rs so
// every identity remains tests::<name>.

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
        let mut kernel = SyntheticM1RulesKernel;
        let result = kernel.apply(&state, PlayerId(1), case).unwrap();
        assert!(!result.accepted);
        assert_eq!(result.next_state, state);
        assert!(result.events.is_empty());
        assert!(result.delta.audit.is_empty());
        assert_eq!(result.delta.before_revision, result.delta.after_revision);
        assert_eq!(result.delta.before_digest, result.delta.after_digest);
        validate_transition_contract(&state, &result).unwrap();
    }

    // A standalone ChooseMany pending request is not part of the supported
    // program: offering it is an internal soundness failure, not a player
    // rejection.
    let mut kernel = SyntheticM1RulesKernel;
    assert!(matches!(
        kernel.apply(&wrong_domain, PlayerId(1), &cases[0]),
        Err(KernelExecutionError::UnsupportedStagePath)
    ));
}

#[test]
fn sequential_event_delta_audit_rejects_tampered_products() {
    use crate::{AuthoritativeRuleEventKind, TransitionViolation};
    use mtgml_model::RuleEventId;

    let state = synthetic_state();
    let mut kernel = SyntheticM1RulesKernel;
    let result = kernel.apply(&state, PlayerId(1), &response(0, 0)).unwrap();
    assert_eq!(result.events.len(), 5);
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
