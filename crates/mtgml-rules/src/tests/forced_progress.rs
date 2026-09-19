// Ownership fragment: rules-owned forced-progress evidence. Included
// lexically by tests.rs so every identity remains tests::<name>.

use mtgml_decision::{
    AuthoritativeCandidateV2, AuthoritativeDecisionRequestV2, CandidateIntent, DecisionVisibility,
    EngineCandidateBinding,
};
use mtgml_model::{EffectInstanceId, GameObjectId, OpaqueObjectId, RuleEventId};
use mtgml_state::SemanticDeltaOperation;

fn advance_forced_progress(
    state: &EngineState,
) -> Result<TransitionResult, KernelExecutionError> {
    let mut kernel = boundary_kernel();
    kernel.advance_forced_progress(state)
}

/// The independently authored expected entry request installed by
/// stabilization: ChooseOne over the fixture public object, with identities
/// derived from the decision-less setup allocator heads.
fn expected_stabilized_request() -> AuthoritativeDecisionRequestV2 {
    AuthoritativeDecisionRequestV2 {
        decision_id: DecisionId(2),
        player_decision_id: PlayerDecisionIdV1(2),
        state_revision: StateRevision(1),
        actor: PlayerId(1),
        visibility: DecisionVisibility::Public,
        decision: DecisionDomainV2::ChooseOne,
        candidates: vec![AuthoritativeCandidateV2 {
            candidate_id: CandidateIdV1(0),
            visible_intent: CandidateIntent::SelectObject {
                object: OpaqueObjectId(1),
            },
            trusted_binding: EngineCandidateBinding::SelectObject {
                object: GameObjectId(1),
            },
        }],
        continuation_id: None,
    }
}

fn expected_stabilized_events() -> Vec<AuthoritativeRuleEvent> {
    vec![AuthoritativeRuleEvent {
        event_id: RuleEventId(1),
        state_revision: StateRevision(1),
        event: AuthoritativeRuleEventKind::DecisionCreated {
            decision: DecisionId(2),
        },
    }]
}

fn expected_stabilized_delta_operations() -> Vec<SemanticDeltaOperation> {
    vec![SemanticDeltaOperation::DecisionCreated {
        decision: DecisionId(2),
    }]
}

#[test]
fn advance_forced_progress_installs_entry_decision_from_decision_less_setup() {
    let setup = state_without_pending_decision();
    assert!(setup.execution.pending_decision.is_none());
    assert!(setup.execution.continuations.is_empty());

    let product = advance_forced_progress(&setup).expect("stabilization must succeed");

    assert!(product.accepted);
    assert_eq!(product.next_state.revision, StateRevision(1));
    assert_eq!(
        product.next_decision,
        Some(expected_stabilized_request()),
        "stabilization must stop at the derived entry decision"
    );
    assert_eq!(product.events, expected_stabilized_events());
    assert_eq!(
        product.delta.audit,
        expected_stabilized_delta_operations()
    );
    assert_eq!(product.delta.before_revision, StateRevision(0));
    assert_eq!(product.delta.after_revision, StateRevision(1));
    assert_eq!(product.status, mtgml_model::EpisodeStatus::Running);

    // Allocators: only the decision cursors advance; effect, continuation
    // and (zero emitted events beyond the single DecisionCreated) rule-event
    // heads move exactly as authored.
    assert_eq!(
        product.next_state.allocators.next_decision_id,
        DecisionId(3)
    );
    assert_eq!(
        product.next_state.allocators.next_rule_event_id,
        RuleEventId(2)
    );
    assert_eq!(
        product.next_state.allocators.next_effect_id,
        EffectInstanceId(1)
    );
    assert_eq!(
        product
            .next_state
            .perspective_identities
            .players
            .get(&PlayerId(1))
            .expect("P1 identity")
            .next_player_decision_id,
        PlayerDecisionIdV1(3)
    );

    // Nothing else moves: life, RNG, knowledge, zones, continuations.
    assert_eq!(product.next_state.core, setup.core);
    assert_eq!(product.next_state.random, setup.random);
    assert_eq!(product.next_state.knowledge, setup.knowledge);
    assert_eq!(product.next_state.zones, setup.zones);
    assert!(product.next_state.execution.continuations.is_empty());

    // The independently expected stabilized state digests mechanically.
    let mut expected_state = setup.clone();
    expected_state.revision = StateRevision(1);
    expected_state.execution.pending_decision = Some(mtgml_state::PendingDecisionRecordV2 {
        request: expected_stabilized_request(),
    });
    expected_state.allocators.next_decision_id = DecisionId(3);
    expected_state.allocators.next_rule_event_id = RuleEventId(2);
    expected_state
        .perspective_identities
        .players
        .get_mut(&PlayerId(1))
        .expect("P1 identity")
        .next_player_decision_id = PlayerDecisionIdV1(3);
    assert_eq!(product.next_state, expected_state);
    assert_eq!(
        product.next_state.digest().expect("digest"),
        expected_state.digest().expect("digest")
    );
}

#[test]
fn advance_forced_progress_rejects_state_with_pending_decision() {
    let state = synthetic_state();
    assert!(state.execution.pending_decision.is_some());
    let before = state.clone();
    let error = advance_forced_progress(&state).unwrap_err();
    assert!(matches!(
        error,
        KernelExecutionError::UnsupportedStagePath
    ));
    assert_eq!(state, before, "failed progress must not mutate input");
}

#[test]
fn advance_forced_progress_rejects_unsupported_setup_without_mutation() {
    // Life 39 is structurally valid but the synthetic entry program requires
    // the exact fixture life total: derivation must fail closed with the
    // existing unsupported-path signal.
    let mut setup = state_without_pending_decision();
    setup
        .core
        .players
        .get_mut(&PlayerId(1))
        .expect("P1 state")
        .life = 39;
    let before = setup.clone();
    let error = advance_forced_progress(&setup).unwrap_err();
    assert!(matches!(
        error,
        KernelExecutionError::UnsupportedStagePath
    ));
    assert_eq!(setup, before, "failed progress must not mutate input");
}

#[test]
fn advance_forced_progress_rejects_broken_identity_mapping() {
    let mut setup = state_without_pending_decision();
    setup
        .perspective_identities
        .players
        .get_mut(&PlayerId(1))
        .expect("P1 identity")
        .opaque_to_object
        .remove(&OpaqueObjectId(1));
    // A broken identity mapping fails closed (structural validation rejects
    // the setup before any derivation work begins).
    assert!(advance_forced_progress(&setup).is_err());
    // Restoring the mapping restores progress.
    setup
        .perspective_identities
        .players
        .get_mut(&PlayerId(1))
        .expect("P1 identity")
        .opaque_to_object
        .insert(OpaqueObjectId(1), GameObjectId(1));
    assert!(advance_forced_progress(&setup).is_ok());
}

/// A genuine completed assembly: entry, count 2, members, and order all
/// accepted, leaving no pending Decision and no continuations.
fn completed_assembly_state() -> EngineState {
    let stage1 = apply(&synthetic_state(), &select_one_response(0, 0)).next_state;
    let stage2 = apply(&stage1, &number_response(2, 2, 1)).next_state;
    let stage3 = apply(&stage2, &many_response(3, &[0, 1], 2)).next_state;
    let completed = apply(&stage3, &order_response(4, &[0, 1], 3)).next_state;
    assert!(completed.execution.pending_decision.is_none());
    assert!(completed.execution.continuations.is_empty());
    completed
}

#[test]
fn advance_forced_progress_on_completed_state_is_no_work_without_resurrection() {
    let completed = completed_assembly_state();
    assert_ne!(
        completed.revision,
        StateRevision(0),
        "completed assembly must sit past revision 0"
    );
    let before = completed.clone();
    let product = advance_forced_progress(&completed).expect("completed advance must succeed");
    // No work available: unchanged state, no events, no decision — and in
    // particular no resurrected entry decision.
    assert!(!product.accepted);
    assert_eq!(product.next_state, before);
    assert!(product.events.is_empty());
    assert_eq!(product.next_decision, None);
    assert_eq!(product.status, mtgml_model::EpisodeStatus::Running);
}

#[test]
fn advance_forced_progress_is_deterministic() {
    let setup = state_without_pending_decision();
    let first = advance_forced_progress(&setup).expect("first run");
    let second = advance_forced_progress(&setup).expect("second run");
    assert_eq!(first.next_state, second.next_state);
    assert_eq!(first.events, second.events);
    assert_eq!(first.delta, second.delta);
    assert_eq!(first.next_decision, second.next_decision);
    assert_eq!(first.status, second.status);
}
