use super::*;
use mtgml_decision::{
    ActionCandidate, CandidateAssignment, CandidateIntent, DecisionKind, DecisionResponse,
    DecisionVisibility, EngineCandidateBinding, PlayerDecisionRequest, DECISION_RESPONSE_SCHEMA,
};
use mtgml_model::{
    AbilityInstanceId, CardDefinitionId, ContinuationId, DecisionId, EffectInstanceId,
    EpisodeStatus, GameObjectId, OpaqueAbilityId, OpaqueObjectId, PhysicalCardId, PlayerId,
    RuleEventId, StackObjectId, StateRevision, TriggerInstanceId, ZoneKind,
};
use mtgml_random::RootSeed256;
use mtgml_random::{
    RandomStateV1, RandomStreamCursorV1, RandomStreamKeyV1, RandomStreamKindV1,
    RandomValidationError,
};
use mtgml_state::{
    construct_synthetic_engine_state, ContinuationRecord, CoreRulesState, EngineState,
    EngineStateViolation, ExecutionState, FormatState, GameObject, IdentityAllocationError,
    IdentityAllocatorState, KnowledgeState, ObjectSnapshot, PendingDecisionRecord,
    PerspectiveIdentityMap, PerspectiveIdentityState, PlayerKnowledgeState, PlayerState,
    SemanticDeltaOperation, StateDelta, SyntheticResetInputs, VisibilityPartition, ZoneLocation,
    ZonePosition, ZoneState, ZoneTransition,
};
use std::collections::BTreeMap;

fn request(id: u64, revision: u64) -> PlayerDecisionRequest {
    PlayerDecisionRequest {
        schema_version: "player-decision-request.v1".into(),
        decision_id: DecisionId(id),
        state_revision: StateRevision(revision),
        actor: PlayerId(1),
        visibility: DecisionVisibility::Public,
        decision: DecisionKind::ChooseNumber {
            minimum: 0,
            maximum: 0,
        },
        candidates: vec![],
    }
}

fn state() -> EngineState {
    let p1 = PlayerId(1);
    let p2 = PlayerId(2);
    EngineState {
        revision: StateRevision(0),
        core: CoreRulesState {
            players: BTreeMap::from([
                (
                    p1,
                    PlayerState {
                        life: 40,
                        has_lost: false,
                    },
                ),
                (
                    p2,
                    PlayerState {
                        life: 40,
                        has_lost: false,
                    },
                ),
            ]),
            active_player: p1,
            priority_player: p1,
            turn_number: 1,
        },
        zones: ZoneState::default(),
        allocators: IdentityAllocatorState {
            next_object_id: GameObjectId(1),
            next_ability_id: AbilityInstanceId(1),
            next_stack_object_id: StackObjectId(1),
            next_effect_id: EffectInstanceId(1),
            next_trigger_id: TriggerInstanceId(1),
            next_decision_id: DecisionId(1),
            next_continuation_id: ContinuationId(1),
            next_rule_event_id: RuleEventId(1),
            next_opaque_object_id: BTreeMap::from([
                (p1, OpaqueObjectId(1)),
                (p2, OpaqueObjectId(1)),
            ]),
            next_opaque_ability_id: BTreeMap::from([
                (p1, OpaqueAbilityId(1)),
                (p2, OpaqueAbilityId(1)),
            ]),
        },
        execution: ExecutionState::default(),
        random: RandomStateV1::default(),
        knowledge: KnowledgeState {
            players: BTreeMap::from([
                (p1, PlayerKnowledgeState::default()),
                (p2, PlayerKnowledgeState::default()),
            ]),
        },
        perspective_identities: PerspectiveIdentityState {
            players: BTreeMap::from([
                (p1, PerspectiveIdentityMap::default()),
                (p2, PerspectiveIdentityMap::default()),
            ]),
        },
        format: FormatState::None,
    }
}

fn synthetic_state() -> EngineState {
    construct_synthetic_engine_state(SyntheticResetInputs {
        players: [PlayerId(1), PlayerId(2)],
        root_seed: RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap(),
    })
    .unwrap()
}

fn synthetic_response(state: &EngineState) -> DecisionResponse {
    response_for_candidate(state, "select_public_object")
}

fn response_for_candidate(state: &EngineState, candidate_id: &str) -> DecisionResponse {
    let request = &state.execution.pending_decision.as_ref().unwrap().request;
    DecisionResponse {
        schema_version: DECISION_RESPONSE_SCHEMA.to_owned(),
        decision_id: request.decision_id,
        state_revision: request.state_revision,
        assignments: vec![CandidateAssignment {
            candidate_id: candidate_id.to_owned(),
            ordinal: None,
        }],
    }
}

fn response_for_state_or_default(state: &EngineState) -> DecisionResponse {
    state
        .execution
        .pending_decision
        .as_ref()
        .map(|_| response_for_candidate(state, "select_public_object"))
        .unwrap_or_else(|| DecisionResponse {
            schema_version: DECISION_RESPONSE_SCHEMA.to_owned(),
            decision_id: DecisionId(1),
            state_revision: state.revision,
            assignments: vec![CandidateAssignment {
                candidate_id: "select_public_object".to_owned(),
                ordinal: None,
            }],
        })
}

fn assert_exact_rejected_product(before: &EngineState, result: TransitionResult) {
    let before_digest = before.digest().unwrap();
    let expected_next_decision = before
        .execution
        .pending_decision
        .as_ref()
        .map(|record| record.request.clone());

    assert!(!result.accepted);
    assert_eq!(result.next_state, *before);
    assert_eq!(
        before.digest().unwrap(),
        result.next_state.digest().unwrap()
    );
    assert_eq!(result.next_state.revision, before.revision);
    assert_eq!(
        result.next_state.execution.pending_decision,
        before.execution.pending_decision
    );
    assert_eq!(
        result.next_state.execution.continuations,
        before.execution.continuations
    );
    assert_eq!(result.next_state.random.root_seed, before.random.root_seed);
    assert_eq!(result.next_state.random.streams, before.random.streams);
    assert_eq!(result.next_state.allocators, before.allocators);
    assert_eq!(result.next_state.zones, before.zones);
    assert_eq!(result.next_state.core, before.core);
    assert_eq!(result.next_state.knowledge, before.knowledge);
    assert_eq!(
        result.next_state.perspective_identities,
        before.perspective_identities
    );
    assert_eq!(result.next_state.format, before.format);
    assert!(result.events.is_empty());
    assert!(result.delta.audit.is_empty());
    assert_eq!(result.delta.replacement, before.parts());
    assert_eq!(result.delta.before_revision, before.revision);
    assert_eq!(result.delta.after_revision, before.revision);
    assert_eq!(result.delta.before_digest, before_digest);
    assert_eq!(result.delta.after_digest, before_digest);
    assert_eq!(result.next_decision, expected_next_decision);
    assert_eq!(result.status, EpisodeStatus::Running);
    assert_eq!(result.delta.apply(before).unwrap(), *before);
    assert_eq!(
        before.digest().unwrap(),
        result.next_state.digest().unwrap()
    );
    validate_transition_contract(before, &result).unwrap();
}

#[test]
fn synthetic_kernel_boundary_carries_trusted_actor() {
    let state = synthetic_state();
    let response = synthetic_response(&state);
    let mut kernel = SyntheticM1RulesKernel;
    let result = kernel.apply(&state, PlayerId(1), &response);
    assert!(result.is_ok());
}

#[test]
fn synthetic_m1_acceptance_returns_exact_transition_product() {
    let before = synthetic_state();
    let response = synthetic_response(&before);
    let before_digest = before.digest().unwrap();
    let pending = before.execution.pending_decision.as_ref().unwrap();

    assert_eq!(pending.request.actor, PlayerId(1));
    assert_eq!(pending.request.decision, DecisionKind::ChooseOne);
    assert_eq!(pending.request.candidates.len(), 1);
    assert_eq!(
        pending.request.candidates[0].candidate_id,
        "select_public_object"
    );
    assert_eq!(
        pending.candidate_bindings["select_public_object"],
        EngineCandidateBinding::SelectObject {
            object: GameObjectId(1)
        }
    );

    let mut expected_after = before.clone();
    expected_after.revision = StateRevision(1);
    expected_after
        .core
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .life = 38;
    expected_after.execution.pending_decision = None;
    expected_after
        .random
        .set_cursor(
            &RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1),
            RandomStreamCursorV1 { next_raw_u64: 1 },
        )
        .unwrap();
    expected_after.allocators.next_effect_id = EffectInstanceId(2);
    expected_after.allocators.next_rule_event_id = RuleEventId(5);

    let expected_events = vec![
        AuthoritativeRuleEvent {
            event_id: RuleEventId(1),
            state_revision: StateRevision(1),
            event: AuthoritativeRuleEventKind::LifeChanged {
                player: PlayerId(1),
                from: 40,
                to: 39,
            },
        },
        AuthoritativeRuleEvent {
            event_id: RuleEventId(2),
            state_revision: StateRevision(1),
            event: AuthoritativeRuleEventKind::LifeChanged {
                player: PlayerId(1),
                from: 39,
                to: 38,
            },
        },
        AuthoritativeRuleEvent {
            event_id: RuleEventId(3),
            state_revision: StateRevision(1),
            event: AuthoritativeRuleEventKind::RandomValueSampled {
                stream: RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1),
                bound: 10,
                value: 1,
                raw_words_consumed: 1,
                cursor_before: 0,
                cursor_after: 1,
            },
        },
        AuthoritativeRuleEvent {
            event_id: RuleEventId(4),
            state_revision: StateRevision(1),
            event: AuthoritativeRuleEventKind::DecisionCleared {
                decision: DecisionId(1),
            },
        },
    ];
    let expected_audit = expected_events
        .iter()
        .map(|event| event.event.semantic_delta())
        .collect::<Vec<_>>();
    let expected_delta =
        StateDelta::between(&before, &expected_after, expected_audit.clone()).unwrap();

    let mut kernel = SyntheticM1RulesKernel;
    let result = kernel.apply(&before, PlayerId(1), &response).unwrap();

    assert!(result.accepted);
    assert_eq!(result.next_state, expected_after);
    assert_eq!(result.events, expected_events);
    assert_eq!(result.delta, expected_delta);
    assert_eq!(result.delta.audit, expected_audit);
    assert_eq!(result.next_decision, None);
    assert_eq!(result.status, EpisodeStatus::Running);
    assert_eq!(result.next_state.revision, StateRevision(1));
    assert_eq!(result.next_state.core.players[&PlayerId(1)].life, 38);
    assert_eq!(result.next_state.core.players[&PlayerId(2)].life, 40);
    assert_eq!(
        result.next_state.allocators.next_rule_event_id,
        RuleEventId(5)
    );
    assert_eq!(result.next_state.zones, before.zones);
    assert_eq!(result.next_state.random.root_seed, before.random.root_seed);
    assert_eq!(
        result
            .next_state
            .random
            .lookup_stream(&RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1))
            .unwrap()
            .next_raw_u64,
        1
    );
    assert_eq!(result.next_state.knowledge, before.knowledge);
    assert_eq!(
        result.next_state.perspective_identities,
        before.perspective_identities
    );
    assert_eq!(result.next_state.format, before.format);
    assert_eq!(
        result.next_state.allocators.next_object_id,
        before.allocators.next_object_id
    );
    assert_eq!(
        result.next_state.allocators.next_decision_id,
        before.allocators.next_decision_id
    );
    assert_eq!(
        result.next_state.allocators.next_continuation_id,
        before.allocators.next_continuation_id
    );
    assert_eq!(
        result.next_state.allocators.next_effect_id,
        EffectInstanceId(2)
    );
    assert_eq!(result.delta.before_revision, StateRevision(0));
    assert_eq!(result.delta.after_revision, StateRevision(1));
    assert_eq!(result.delta.before_digest, before_digest);
    assert_eq!(
        result.delta.after_digest,
        result.next_state.digest().unwrap()
    );
    assert_ne!(before_digest, result.next_state.digest().unwrap());

    let reapplied = result.delta.apply(&before).unwrap();
    assert_eq!(reapplied, result.next_state);
    assert_eq!(
        reapplied.digest().unwrap(),
        result.next_state.digest().unwrap()
    );
    validate_transition_contract(&before, &result).unwrap();
}

#[test]
fn synthetic_m1_acceptance_uses_the_first_player_for_non_default_ids() {
    let first_player = PlayerId(7);
    let second_player = PlayerId(9);
    let before = construct_synthetic_engine_state(SyntheticResetInputs {
        players: [first_player, second_player],
        root_seed: RootSeed256::from_lower_hex(&"22".repeat(32)).unwrap(),
    })
    .unwrap();
    let response = synthetic_response(&before);

    assert_eq!(before.revision, StateRevision(0));
    assert_eq!(before.core.players[&first_player].life, 40);
    assert_eq!(before.core.players[&second_player].life, 40);
    assert_eq!(
        before
            .execution
            .pending_decision
            .as_ref()
            .unwrap()
            .request
            .actor,
        first_player
    );

    let mut kernel = SyntheticM1RulesKernel;
    let result = kernel.apply(&before, first_player, &response).unwrap();

    assert!(result.accepted);
    assert_eq!(result.next_state.revision, StateRevision(1));
    assert_eq!(result.next_state.core.players[&first_player].life, 38);
    assert_eq!(result.next_state.core.players[&second_player].life, 40);
    assert_eq!(result.next_state.execution.pending_decision, None);
    assert_eq!(
        result.next_state.allocators.next_rule_event_id,
        RuleEventId(5)
    );
    assert_eq!(
        result.events,
        vec![
            AuthoritativeRuleEvent {
                event_id: RuleEventId(1),
                state_revision: StateRevision(1),
                event: AuthoritativeRuleEventKind::LifeChanged {
                    player: first_player,
                    from: 40,
                    to: 39,
                },
            },
            AuthoritativeRuleEvent {
                event_id: RuleEventId(2),
                state_revision: StateRevision(1),
                event: AuthoritativeRuleEventKind::LifeChanged {
                    player: first_player,
                    from: 39,
                    to: 38,
                },
            },
            AuthoritativeRuleEvent {
                event_id: RuleEventId(3),
                state_revision: StateRevision(1),
                event: AuthoritativeRuleEventKind::RandomValueSampled {
                    stream: RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1),
                    bound: 10,
                    value: 1,
                    raw_words_consumed: 1,
                    cursor_before: 0,
                    cursor_after: 1,
                },
            },
            AuthoritativeRuleEvent {
                event_id: RuleEventId(4),
                state_revision: StateRevision(1),
                event: AuthoritativeRuleEventKind::DecisionCleared {
                    decision: DecisionId(1),
                },
            },
        ]
    );

    let reapplied = result.delta.apply(&before).unwrap();
    assert_eq!(reapplied, result.next_state);
    assert_eq!(
        reapplied.digest().unwrap(),
        result.next_state.digest().unwrap()
    );
    validate_transition_contract(&before, &result).unwrap();
}

#[test]
fn second_life_event_must_use_cursor_life_after_first_event() {
    let before = state();
    let mut after = before.clone();
    after.revision = StateRevision(1);
    after.core.players.get_mut(&PlayerId(1)).unwrap().life = 38;
    after.allocators.next_rule_event_id = RuleEventId(3);
    let events = vec![
        AuthoritativeRuleEvent {
            event_id: RuleEventId(1),
            state_revision: StateRevision(1),
            event: AuthoritativeRuleEventKind::LifeChanged {
                player: PlayerId(1),
                from: 40,
                to: 39,
            },
        },
        AuthoritativeRuleEvent {
            event_id: RuleEventId(2),
            state_revision: StateRevision(1),
            event: AuthoritativeRuleEventKind::LifeChanged {
                player: PlayerId(1),
                from: 40,
                to: 38,
            },
        },
    ];
    let transition = result(&before, after, events, None);

    assert!(matches!(
        validate_transition_contract(&before, &transition),
        Err(TransitionViolation::LifeChange)
    ));
}

#[test]
fn reversed_dependent_life_events_fail() {
    let before = state();
    let mut after = before.clone();
    after.revision = StateRevision(1);
    after.core.players.get_mut(&PlayerId(1)).unwrap().life = 38;
    after.allocators.next_rule_event_id = RuleEventId(3);
    let events = vec![
        AuthoritativeRuleEvent {
            event_id: RuleEventId(1),
            state_revision: StateRevision(1),
            event: AuthoritativeRuleEventKind::LifeChanged {
                player: PlayerId(1),
                from: 39,
                to: 38,
            },
        },
        AuthoritativeRuleEvent {
            event_id: RuleEventId(2),
            state_revision: StateRevision(1),
            event: AuthoritativeRuleEventKind::LifeChanged {
                player: PlayerId(1),
                from: 40,
                to: 39,
            },
        },
    ];
    let transition = result(&before, after, events, None);

    assert!(matches!(
        validate_transition_contract(&before, &transition),
        Err(TransitionViolation::LifeChange)
    ));
}

#[test]
fn incomplete_life_trace_fails_final_projection() {
    let before = state();
    let mut after = before.clone();
    after.revision = StateRevision(1);
    after.core.players.get_mut(&PlayerId(1)).unwrap().life = 38;
    after.allocators.next_rule_event_id = RuleEventId(2);
    let events = vec![AuthoritativeRuleEvent {
        event_id: RuleEventId(1),
        state_revision: StateRevision(1),
        event: AuthoritativeRuleEventKind::LifeChanged {
            player: PlayerId(1),
            from: 40,
            to: 39,
        },
    }];
    let transition = result(&before, after, events, None);

    assert!(matches!(
        validate_transition_contract(&before, &transition),
        Err(TransitionViolation::LifeChange)
    ));
}

#[test]
fn event_and_delta_audit_disagreement_fails() {
    let before = state();
    let mut after = before.clone();
    after.revision = StateRevision(1);
    after.core.players.get_mut(&PlayerId(1)).unwrap().life = 38;
    after.allocators.next_rule_event_id = RuleEventId(3);
    let events = vec![
        AuthoritativeRuleEvent {
            event_id: RuleEventId(1),
            state_revision: StateRevision(1),
            event: AuthoritativeRuleEventKind::LifeChanged {
                player: PlayerId(1),
                from: 40,
                to: 39,
            },
        },
        AuthoritativeRuleEvent {
            event_id: RuleEventId(2),
            state_revision: StateRevision(1),
            event: AuthoritativeRuleEventKind::LifeChanged {
                player: PlayerId(1),
                from: 39,
                to: 38,
            },
        },
    ];
    let mut transition = result(&before, after, events, None);
    transition.delta.audit[1] = SemanticDeltaOperation::LifeChanged {
        player: PlayerId(1),
        from: 39,
        to: 37,
    };

    assert!(matches!(
        validate_transition_contract(&before, &transition),
        Err(TransitionViolation::EventDeltaMismatch)
    ));
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RejectionClassification {
    PlayerSubmission,
    TrustedBeforeState(EngineStateViolation),
}

#[derive(Clone)]
struct RejectionCase {
    name: &'static str,
    mutate_state: fn(&mut EngineState),
    mutate_response: fn(&EngineState, &mut DecisionResponse),
    trusted_actor: PlayerId,
    classification: RejectionClassification,
}

fn no_state_mutation(_state: &mut EngineState) {}

fn no_response_mutation(_state: &EngineState, _response: &mut DecisionResponse) {}

fn response_with_stale_revision(_state: &EngineState, response: &mut DecisionResponse) {
    response.state_revision = StateRevision(response.state_revision.0 + 1);
}

fn response_with_wrong_decision_id(_state: &EngineState, response: &mut DecisionResponse) {
    response.decision_id = DecisionId(99);
}

fn response_with_wrong_schema(_state: &EngineState, response: &mut DecisionResponse) {
    response.schema_version = "decision-response.invalid".to_owned();
}

fn response_with_empty_candidate(_state: &EngineState, response: &mut DecisionResponse) {
    response.assignments[0].candidate_id.clear();
}

fn response_with_duplicate_assignment(_state: &EngineState, response: &mut DecisionResponse) {
    let candidate_id = response.assignments[0].candidate_id.clone();
    response.assignments.push(CandidateAssignment {
        candidate_id,
        ordinal: None,
    });
}

fn response_with_zero_assignments(_state: &EngineState, response: &mut DecisionResponse) {
    response.assignments.clear();
}

fn response_with_multiple_assignments(_state: &EngineState, response: &mut DecisionResponse) {
    response.assignments.push(CandidateAssignment {
        candidate_id: "second-selection".to_owned(),
        ordinal: None,
    });
}

fn response_with_unknown_candidate(_state: &EngineState, response: &mut DecisionResponse) {
    response.assignments[0].candidate_id = "unknown-candidate".to_owned();
}

fn response_with_unsupported_ordinal(_state: &EngineState, response: &mut DecisionResponse) {
    response.assignments[0].ordinal = Some(0);
}

fn state_with_confirm_candidate(state: &mut EngineState) {
    let pending = state.execution.pending_decision.as_mut().unwrap();
    pending.request.candidates[0] = ActionCandidate {
        candidate_id: "select_public_object".to_owned(),
        semantic_key: "synthetic.select_public_object".to_owned(),
        intent: CandidateIntent::Confirm,
    };
    pending.candidate_bindings.insert(
        "select_public_object".to_owned(),
        EngineCandidateBinding::Confirm,
    );
}

fn state_with_multiple_candidates(state: &mut EngineState) {
    let pending = state.execution.pending_decision.as_mut().unwrap();
    pending.request.candidates.push(ActionCandidate {
        candidate_id: "confirm".to_owned(),
        semantic_key: "synthetic.confirm".to_owned(),
        intent: CandidateIntent::Confirm,
    });
    pending
        .candidate_bindings
        .insert("confirm".to_owned(), EngineCandidateBinding::Confirm);
}

fn state_with_continuation(state: &mut EngineState) {
    let continuation = ContinuationId(1);
    state.execution.continuations.insert(
        continuation,
        ContinuationRecord {
            id: continuation,
            label: "synthetic continuation".to_owned(),
        },
    );
    state
        .execution
        .pending_decision
        .as_mut()
        .unwrap()
        .continuation = Some(continuation);
    state.allocators.next_continuation_id = ContinuationId(2);
}

fn state_with_choose_many(state: &mut EngineState) {
    state
        .execution
        .pending_decision
        .as_mut()
        .unwrap()
        .request
        .decision = DecisionKind::ChooseMany {
        minimum: 1,
        maximum: 1,
    };
}

fn state_without_pending_decision(state: &mut EngineState) {
    state.execution.pending_decision = None;
}

fn state_with_invalid_rule_event_allocator(state: &mut EngineState) {
    state.allocators.next_rule_event_id = RuleEventId(0);
}

fn state_with_binding_mismatch(state: &mut EngineState) {
    state
        .execution
        .pending_decision
        .as_mut()
        .unwrap()
        .candidate_bindings
        .insert(
            "select_public_object".to_owned(),
            EngineCandidateBinding::SelectObject {
                object: GameObjectId(2),
            },
        );
}

fn rejection_cases() -> Vec<RejectionCase> {
    let player_rejection = RejectionClassification::PlayerSubmission;
    vec![
        RejectionCase {
            name: "wrong trusted actor",
            mutate_state: no_state_mutation,
            mutate_response: no_response_mutation,
            trusted_actor: PlayerId(2),
            classification: player_rejection.clone(),
        },
        RejectionCase {
            name: "stale state revision",
            mutate_state: no_state_mutation,
            mutate_response: response_with_stale_revision,
            trusted_actor: PlayerId(1),
            classification: player_rejection.clone(),
        },
        RejectionCase {
            name: "wrong decision ID",
            mutate_state: no_state_mutation,
            mutate_response: response_with_wrong_decision_id,
            trusted_actor: PlayerId(1),
            classification: player_rejection.clone(),
        },
        RejectionCase {
            name: "wrong schema version",
            mutate_state: no_state_mutation,
            mutate_response: response_with_wrong_schema,
            trusted_actor: PlayerId(1),
            classification: player_rejection.clone(),
        },
        RejectionCase {
            name: "empty candidate ID",
            mutate_state: no_state_mutation,
            mutate_response: response_with_empty_candidate,
            trusted_actor: PlayerId(1),
            classification: player_rejection.clone(),
        },
        RejectionCase {
            name: "duplicate assignment",
            mutate_state: no_state_mutation,
            mutate_response: response_with_duplicate_assignment,
            trusted_actor: PlayerId(1),
            classification: player_rejection.clone(),
        },
        RejectionCase {
            name: "zero assignments",
            mutate_state: no_state_mutation,
            mutate_response: response_with_zero_assignments,
            trusted_actor: PlayerId(1),
            classification: player_rejection.clone(),
        },
        RejectionCase {
            name: "more than one assignment for ChooseOne",
            mutate_state: no_state_mutation,
            mutate_response: response_with_multiple_assignments,
            trusted_actor: PlayerId(1),
            classification: player_rejection.clone(),
        },
        RejectionCase {
            name: "unknown candidate ID",
            mutate_state: no_state_mutation,
            mutate_response: response_with_unknown_candidate,
            trusted_actor: PlayerId(1),
            classification: player_rejection.clone(),
        },
        RejectionCase {
            name: "unsupported ordinal",
            mutate_state: no_state_mutation,
            mutate_response: response_with_unsupported_ordinal,
            trusted_actor: PlayerId(1),
            classification: player_rejection.clone(),
        },
        RejectionCase {
            name: "otherwise-valid Confirm candidate and binding",
            mutate_state: state_with_confirm_candidate,
            mutate_response: no_response_mutation,
            trusted_actor: PlayerId(1),
            classification: player_rejection.clone(),
        },
        RejectionCase {
            name: "more than one valid candidate",
            mutate_state: state_with_multiple_candidates,
            mutate_response: no_response_mutation,
            trusted_actor: PlayerId(1),
            classification: player_rejection.clone(),
        },
        RejectionCase {
            name: "pending decision with continuation",
            mutate_state: state_with_continuation,
            mutate_response: no_response_mutation,
            trusted_actor: PlayerId(1),
            classification: player_rejection.clone(),
        },
        RejectionCase {
            name: "otherwise-valid ChooseMany decision",
            mutate_state: state_with_choose_many,
            mutate_response: no_response_mutation,
            trusted_actor: PlayerId(1),
            classification: player_rejection.clone(),
        },
        RejectionCase {
            name: "no pending decision",
            mutate_state: state_without_pending_decision,
            mutate_response: no_response_mutation,
            trusted_actor: PlayerId(1),
            classification: player_rejection,
        },
        RejectionCase {
            name: "invalid rule-event allocator",
            mutate_state: state_with_invalid_rule_event_allocator,
            mutate_response: no_response_mutation,
            trusted_actor: PlayerId(1),
            classification: RejectionClassification::TrustedBeforeState(
                EngineStateViolation::AllocatorBehind,
            ),
        },
        RejectionCase {
            name: "visible and authoritative binding mismatch",
            mutate_state: state_with_binding_mismatch,
            mutate_response: no_response_mutation,
            trusted_actor: PlayerId(1),
            classification: RejectionClassification::TrustedBeforeState(
                EngineStateViolation::PendingDecisionMismatch,
            ),
        },
    ]
}

#[test]
fn synthetic_rejection_matrix_preserves_complete_nonmutation() {
    for case in rejection_cases() {
        let mut before = synthetic_state();
        (case.mutate_state)(&mut before);
        if matches!(
            case.classification,
            RejectionClassification::PlayerSubmission
        ) {
            assert!(
                mtgml_state::validate_engine_state(&before).is_ok(),
                "{}: player-rejection fixture must be a valid before-state",
                case.name
            );
        }

        let mut response = response_for_state_or_default(&before);
        (case.mutate_response)(&before, &mut response);
        let mut kernel = SyntheticM1RulesKernel;
        let result = kernel.apply(&before, case.trusted_actor, &response);

        match (&case.classification, result) {
            (RejectionClassification::PlayerSubmission, Ok(result)) => {
                assert_exact_rejected_product(&before, result);
            }
            (
                RejectionClassification::TrustedBeforeState(expected),
                Err(KernelExecutionError::BeforeState(actual)),
            ) => {
                assert_eq!(
                    &actual, expected,
                    "{}: trusted before-state error differed",
                    case.name
                );
            }
            (expected, actual) => panic!("{}: expected {expected:?}, got {actual:?}", case.name),
        }
    }
}

#[test]
fn invalid_before_state_is_a_kernel_execution_error() {
    let before = synthetic_state();
    let response = synthetic_response(&before);
    let mut invalid = before.clone();
    invalid.allocators.next_rule_event_id = RuleEventId(0);
    let mut kernel = SyntheticM1RulesKernel;

    assert!(matches!(
        kernel.apply(&invalid, PlayerId(1), &response),
        Err(KernelExecutionError::BeforeState(
            EngineStateViolation::AllocatorBehind
        ))
    ));
}

fn insert_object(
    state: &mut EngineState,
    object_id: u64,
    zone: ZoneKind,
    tapped: bool,
) -> ObjectSnapshot {
    let id = GameObjectId(object_id);
    let location = ZoneLocation {
        zone,
        player: Some(PlayerId(1)),
        position: ZonePosition::Unordered,
        visibility: if zone == ZoneKind::Battlefield || zone == ZoneKind::Graveyard {
            VisibilityPartition::Public
        } else {
            VisibilityPartition::OwnerOnly
        },
        partition: None,
    };
    let object = GameObject {
        id,
        physical_card: Some(PhysicalCardId(1)),
        card_definition: CardDefinitionId(1),
        owner: PlayerId(1),
        controller: PlayerId(1),
        tapped,
        face_down: false,
    };
    state.zones.objects.insert(id, object.clone());
    state.zones.locations.insert(id, location.clone());
    state.allocators.next_object_id = GameObjectId(object_id + 1);
    ObjectSnapshot {
        object: id,
        physical_card: object.physical_card,
        card_definition: object.card_definition,
        owner: object.owner,
        controller: object.controller,
        tapped: object.tapped,
        face_down: object.face_down,
        location,
    }
}

fn remove_object(state: &mut EngineState, object: GameObjectId) {
    let location = state.zones.locations.remove(&object).unwrap();
    state.zones.objects.remove(&object).unwrap();
    let key = location.key();
    if let Some(objects) = state.zones.ordered_zones.get_mut(&key) {
        objects.retain(|current| *current != object);
        if objects.is_empty() {
            state.zones.ordered_zones.remove(&key);
        }
    }
}

fn result(
    before: &EngineState,
    after: EngineState,
    events: Vec<AuthoritativeRuleEvent>,
    next_decision: Option<PlayerDecisionRequest>,
) -> TransitionResult {
    let audit = events
        .iter()
        .map(|event| event.event.semantic_delta())
        .collect();
    TransitionResult {
        accepted: true,
        delta: StateDelta::between(before, &after, audit).unwrap(),
        next_state: after,
        events,
        next_decision,
        status: EpisodeStatus::Running,
    }
}

fn random_before_and_stream() -> (EngineState, RandomStreamKeyV1) {
    let stream = RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1);
    let mut before = state();
    before.random = RandomStateV1::new(RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap());
    before
        .random
        .add_stream(stream, RandomStreamCursorV1::default())
        .unwrap();
    (before, stream)
}

fn random_transition(
    event: AuthoritativeRuleEventKind,
    after_cursor: u64,
) -> (EngineState, TransitionResult) {
    let (before, stream) = random_before_and_stream();
    let mut after = before.clone();
    after.revision = StateRevision(1);
    after
        .random
        .set_cursor(
            &stream,
            RandomStreamCursorV1 {
                next_raw_u64: after_cursor,
            },
        )
        .unwrap();
    after.allocators.next_rule_event_id = RuleEventId(2);
    let transition = result(
        &before,
        after,
        vec![AuthoritativeRuleEvent {
            event_id: RuleEventId(1),
            state_revision: StateRevision(1),
            event,
        }],
        None,
    );
    (before, transition)
}

#[test]
fn random_sample_event_is_validated_against_authoritative_sampler() {
    let stream = RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1);
    let (before, transition) = random_transition(
        AuthoritativeRuleEventKind::RandomValueSampled {
            stream,
            bound: 10,
            value: 1,
            raw_words_consumed: 1,
            cursor_before: 0,
            cursor_after: 1,
        },
        1,
    );

    validate_transition_contract(&before, &transition).unwrap();
}

#[test]
fn random_sample_event_rejects_wrong_cursor_precondition() {
    let stream = RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1);
    let (before, transition) = random_transition(
        AuthoritativeRuleEventKind::RandomValueSampled {
            stream,
            bound: 10,
            value: 1,
            raw_words_consumed: 1,
            cursor_before: 1,
            cursor_after: 2,
        },
        1,
    );

    assert!(matches!(
        validate_transition_contract(&before, &transition),
        Err(TransitionViolation::Randomness)
    ));
}

#[test]
fn random_sample_event_rejects_wrong_value() {
    let stream = RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1);
    let (before, transition) = random_transition(
        AuthoritativeRuleEventKind::RandomValueSampled {
            stream,
            bound: 10,
            value: 2,
            raw_words_consumed: 1,
            cursor_before: 0,
            cursor_after: 1,
        },
        1,
    );

    assert!(matches!(
        validate_transition_contract(&before, &transition),
        Err(TransitionViolation::Randomness)
    ));
}

#[test]
fn random_sample_event_rejects_wrong_consumption_and_cursor_after() {
    let stream = RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1);
    let (before, transition) = random_transition(
        AuthoritativeRuleEventKind::RandomValueSampled {
            stream,
            bound: 10,
            value: 1,
            raw_words_consumed: 2,
            cursor_before: 0,
            cursor_after: 2,
        },
        1,
    );

    assert!(matches!(
        validate_transition_contract(&before, &transition),
        Err(TransitionViolation::Randomness)
    ));
}

#[test]
fn random_sample_event_is_required_for_final_cursor_progression() {
    let (before, stream) = random_before_and_stream();
    let mut after = before.clone();
    after.revision = StateRevision(1);
    after
        .random
        .set_cursor(&stream, RandomStreamCursorV1 { next_raw_u64: 1 })
        .unwrap();
    let transition = result(&before, after, vec![], None);

    assert!(matches!(
        validate_transition_contract(&before, &transition),
        Err(TransitionViolation::Randomness)
    ));
}

#[test]
fn random_sample_event_and_delta_audit_must_agree() {
    let stream = RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1);
    let (before, mut transition) = random_transition(
        AuthoritativeRuleEventKind::RandomValueSampled {
            stream,
            bound: 10,
            value: 1,
            raw_words_consumed: 1,
            cursor_before: 0,
            cursor_after: 1,
        },
        1,
    );
    transition.delta.audit = vec![SemanticDeltaOperation::PublicOutcome {
        code: "mismatch".into(),
    }];

    assert!(matches!(
        validate_transition_contract(&before, &transition),
        Err(TransitionViolation::EventDeltaMismatch)
    ));
}

#[test]
fn two_life_changes_in_one_atomic_transition_are_compositional() {
    let before = state();
    let mut after = before.clone();
    after.revision = StateRevision(1);
    after.core.players.get_mut(&PlayerId(1)).unwrap().life = 38;
    after.allocators.next_rule_event_id = RuleEventId(3);
    let events = vec![
        AuthoritativeRuleEvent {
            event_id: RuleEventId(1),
            state_revision: StateRevision(1),
            event: AuthoritativeRuleEventKind::LifeChanged {
                player: PlayerId(1),
                from: 40,
                to: 39,
            },
        },
        AuthoritativeRuleEvent {
            event_id: RuleEventId(2),
            state_revision: StateRevision(1),
            event: AuthoritativeRuleEventKind::LifeChanged {
                player: PlayerId(1),
                from: 39,
                to: 38,
            },
        },
    ];
    validate_transition_contract(&before, &result(&before, after, events, None)).unwrap();
}

#[test]
fn decision_clear_then_create_composes_to_the_next_decision() {
    let mut before = state();
    before.execution.pending_decision = Some(PendingDecisionRecord {
        request: request(1, 0),
        candidate_bindings: BTreeMap::new(),
        continuation: None,
    });
    before.allocators.next_decision_id = DecisionId(2);

    let mut after = before.clone();
    after.revision = StateRevision(1);
    let next = request(2, 1);
    after.execution.pending_decision = Some(PendingDecisionRecord {
        request: next.clone(),
        candidate_bindings: BTreeMap::new(),
        continuation: None,
    });
    after.allocators.next_decision_id = DecisionId(3);
    after.allocators.next_rule_event_id = RuleEventId(3);
    let events = vec![
        AuthoritativeRuleEvent {
            event_id: RuleEventId(1),
            state_revision: StateRevision(1),
            event: AuthoritativeRuleEventKind::DecisionCleared {
                decision: DecisionId(1),
            },
        },
        AuthoritativeRuleEvent {
            event_id: RuleEventId(2),
            state_revision: StateRevision(1),
            event: AuthoritativeRuleEventKind::DecisionCreated {
                decision: DecisionId(2),
            },
        },
    ];
    validate_transition_contract(&before, &result(&before, after, events, Some(next))).unwrap();
}

#[test]
fn repeated_tap_changes_in_one_atomic_transition_are_compositional() {
    let mut before = state();
    insert_object(&mut before, 1, ZoneKind::Battlefield, false);
    let mut after = before.clone();
    after.revision = StateRevision(1);
    after.allocators.next_rule_event_id = RuleEventId(3);
    let events = vec![
        AuthoritativeRuleEvent {
            event_id: RuleEventId(1),
            state_revision: StateRevision(1),
            event: AuthoritativeRuleEventKind::ObjectTapped {
                object: GameObjectId(1),
                from: false,
                to: true,
            },
        },
        AuthoritativeRuleEvent {
            event_id: RuleEventId(2),
            state_revision: StateRevision(1),
            event: AuthoritativeRuleEventKind::ObjectTapped {
                object: GameObjectId(1),
                from: true,
                to: false,
            },
        },
    ];
    validate_transition_contract(&before, &result(&before, after, events, None)).unwrap();
}

#[test]
fn consecutive_zone_incarnations_in_one_transition_are_compositional() {
    let mut before = state();
    let first = insert_object(&mut before, 1, ZoneKind::Battlefield, false);

    let second_location = ZoneLocation {
        zone: ZoneKind::Exile,
        player: Some(PlayerId(1)),
        position: ZonePosition::Unordered,
        visibility: VisibilityPartition::Public,
        partition: Some("linked-test".into()),
    };
    let second = ObjectSnapshot {
        object: GameObjectId(2),
        location: second_location.clone(),
        ..first.clone()
    };
    let third_location = ZoneLocation {
        zone: ZoneKind::Graveyard,
        player: Some(PlayerId(1)),
        position: ZonePosition::Unordered,
        visibility: VisibilityPartition::Public,
        partition: None,
    };
    let third = ObjectSnapshot {
        object: GameObjectId(3),
        location: third_location.clone(),
        ..first.clone()
    };

    let mut after = before.clone();
    remove_object(&mut after, GameObjectId(1));
    insert_object(&mut after, 3, ZoneKind::Graveyard, false);
    after.revision = StateRevision(1);
    after.allocators.next_object_id = GameObjectId(4);
    after.allocators.next_rule_event_id = RuleEventId(3);

    let events = vec![
        AuthoritativeRuleEvent {
            event_id: RuleEventId(1),
            state_revision: StateRevision(1),
            event: AuthoritativeRuleEventKind::ZoneTransition {
                transition: Box::new(ZoneTransition {
                    old_object: GameObjectId(1),
                    new_object: GameObjectId(2),
                    physical_card: Some(PhysicalCardId(1)),
                    from: first.location.clone(),
                    to: second_location,
                    last_known: first,
                    new_snapshot: second.clone(),
                }),
            },
        },
        AuthoritativeRuleEvent {
            event_id: RuleEventId(2),
            state_revision: StateRevision(1),
            event: AuthoritativeRuleEventKind::ZoneTransition {
                transition: Box::new(ZoneTransition {
                    old_object: GameObjectId(2),
                    new_object: GameObjectId(3),
                    physical_card: Some(PhysicalCardId(1)),
                    from: second.location.clone(),
                    to: third_location,
                    last_known: second,
                    new_snapshot: third,
                }),
            },
        },
    ];
    validate_transition_contract(&before, &result(&before, after, events, None)).unwrap();
}

#[test]
fn accepted_transition_cannot_reuse_the_consumed_decision_id() {
    let mut before = state();
    before.execution.pending_decision = Some(PendingDecisionRecord {
        request: request(1, 0),
        candidate_bindings: BTreeMap::new(),
        continuation: None,
    });
    before.allocators.next_decision_id = DecisionId(2);

    let mut after = before.clone();
    after.revision = StateRevision(1);
    after
        .execution
        .pending_decision
        .as_mut()
        .unwrap()
        .request
        .state_revision = StateRevision(1);
    let transition = result(&before, after, vec![], Some(request(1, 1)));
    assert!(matches!(
        validate_transition_contract(&before, &transition),
        Err(TransitionViolation::DecisionIdentityReused)
    ));
}
