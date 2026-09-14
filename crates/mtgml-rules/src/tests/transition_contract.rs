// Ownership fragment: transition-contract and nonmutation evidence. Included lexically by tests.rs so
// every identity remains tests::<name>.

fn accepted_product_for_contract(
    before: &EngineState,
    after: EngineState,
    events: Vec<AuthoritativeRuleEvent>,
) -> TransitionResult {
    let audit = events
        .iter()
        .map(|event| event.event.semantic_delta())
        .collect();
    let delta = mtgml_state::StateDelta::between(before, &after, audit).unwrap();
    TransitionResult {
        accepted: true,
        next_state: after.clone(),
        delta,
        events,
        next_decision: after
            .execution
            .pending_decision
            .as_ref()
            .map(|record| record.request.clone()),
        status: mtgml_model::EpisodeStatus::Running,
    }
}

fn assert_contract_rejects_without_mutation(before: &EngineState, result: &TransitionResult) {
    let before_snapshot = before.clone();
    let result_snapshot = result.clone();
    assert!(validate_transition_contract(before, result).is_err());
    assert_eq!(before, &before_snapshot);
    assert_eq!(result, &result_snapshot);
}

fn decision_creation_product(
    decision_id: DecisionId,
    player_decision_id: PlayerDecisionIdV1,
    allocator_cursor: u64,
    perspective_cursor: u64,
) -> (EngineState, TransitionResult) {
    let mut before = state_without_pending_decision();
    before.allocators.next_decision_id = DecisionId(allocator_cursor);
    before
        .perspective_identities
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .next_player_decision_id = PlayerDecisionIdV1(perspective_cursor);

    let mut after = before.clone();
    after.revision = StateRevision(1);
    let mut request = synthetic_state()
        .execution
        .pending_decision
        .unwrap()
        .request;
    request.decision_id = decision_id;
    request.player_decision_id = player_decision_id;
    request.state_revision = after.revision;
    after.execution.pending_decision = Some(mtgml_state::PendingDecisionRecordV2 { request });
    after.allocators.next_decision_id = DecisionId(allocator_cursor);
    after
        .perspective_identities
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .next_player_decision_id = PlayerDecisionIdV1(perspective_cursor);
    after.allocators.next_rule_event_id = mtgml_model::RuleEventId(2);
    let event = AuthoritativeRuleEvent {
        event_id: mtgml_model::RuleEventId(1),
        state_revision: after.revision,
        event: AuthoritativeRuleEventKind::DecisionCreated {
            decision: decision_id,
        },
    };
    let result = accepted_product_for_contract(&before, after, vec![event]);
    (before, result)
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

#[test]
fn unexplained_turn_mutation_must_not_pass_transition_contract() {
    let before = state_without_pending_decision();
    let mut after = before.clone();
    after.revision = StateRevision(before.revision.0 + 1);
    after.core.turn_number += 1;
    let result = accepted_product_for_contract(&before, after, Vec::new());
    assert_contract_rejects_without_mutation(&before, &result);
}

#[test]
fn accepted_revision_must_advance_exactly_once() {
    let before = state_without_pending_decision();
    let mut after = before.clone();
    after.revision = StateRevision(before.revision.0 + 2);
    let result = accepted_product_for_contract(&before, after, Vec::new());
    assert_contract_rejects_without_mutation(&before, &result);
}

#[test]
fn global_allocator_rewind_must_not_pass_transition_contract() {
    let mut before = state_without_pending_decision();
    before.allocators.next_object_id = mtgml_model::GameObjectId(5);
    let mut after = before.clone();
    after.revision = StateRevision(1);
    after.allocators.next_object_id = mtgml_model::GameObjectId(3);
    let result = accepted_product_for_contract(&before, after, Vec::new());
    assert_contract_rejects_without_mutation(&before, &result);
}

#[test]
fn trusted_decision_identity_reuse_must_not_pass_transition_contract() {
    let (before, result) = decision_creation_product(DecisionId(1), PlayerDecisionIdV1(1), 5, 2);
    assert_contract_rejects_without_mutation(&before, &result);
}

#[test]
fn perspective_decision_identity_reuse_must_not_pass_transition_contract() {
    let (before, result) = decision_creation_product(DecisionId(5), PlayerDecisionIdV1(1), 6, 5);
    assert_contract_rejects_without_mutation(&before, &result);
}

#[test]
fn continuation_identity_must_persist_across_staged_decisions() {
    let before = continuation_state();
    let mut after = before.clone();
    after.revision = StateRevision(1);
    let mut continuation = after
        .execution
        .continuations
        .remove(&mtgml_model::ContinuationId(1))
        .unwrap();
    continuation.id = mtgml_model::ContinuationId(2);
    after.execution.continuations.insert(continuation.id, continuation);
    let request = &mut after.execution.pending_decision.as_mut().unwrap().request;
    request.decision_id = DecisionId(10);
    request.player_decision_id = PlayerDecisionIdV1(10);
    request.state_revision = after.revision;
    request.continuation_id = Some(mtgml_model::ContinuationId(2));
    after.allocators.next_decision_id = DecisionId(11);
    after.allocators.next_continuation_id = mtgml_model::ContinuationId(3);
    after
        .perspective_identities
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .next_player_decision_id = PlayerDecisionIdV1(11);
    after.allocators.next_rule_event_id = mtgml_model::RuleEventId(3);
    let events = vec![
        AuthoritativeRuleEvent {
            event_id: mtgml_model::RuleEventId(1),
            state_revision: after.revision,
            event: AuthoritativeRuleEventKind::DecisionCleared {
                decision: DecisionId(9),
            },
        },
        AuthoritativeRuleEvent {
            event_id: mtgml_model::RuleEventId(2),
            state_revision: after.revision,
            event: AuthoritativeRuleEventKind::DecisionCreated {
                decision: DecisionId(10),
            },
        },
    ];
    let result = accepted_product_for_contract(&before, after, events);
    assert_contract_rejects_without_mutation(&before, &result);
}

#[test]
fn cross_perspective_decision_cursor_inheritance_must_not_pass() {
    let mut before = state_without_pending_decision();
    before
        .perspective_identities
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .next_player_decision_id = PlayerDecisionIdV1(10);
    let mut after = before.clone();
    after.revision = StateRevision(1);
    let mut request = synthetic_state()
        .execution
        .pending_decision
        .unwrap()
        .request;
    request.actor = PlayerId(2);
    request.decision_id = DecisionId(2);
    request.player_decision_id = PlayerDecisionIdV1(10);
    request.state_revision = after.revision;
    after.execution.pending_decision = Some(mtgml_state::PendingDecisionRecordV2 { request });
    after.allocators.next_decision_id = DecisionId(3);
    after
        .perspective_identities
        .players
        .get_mut(&PlayerId(2))
        .unwrap()
        .next_player_decision_id = PlayerDecisionIdV1(11);
    after.allocators.next_rule_event_id = mtgml_model::RuleEventId(2);
    let event = AuthoritativeRuleEvent {
        event_id: mtgml_model::RuleEventId(1),
        state_revision: after.revision,
        event: AuthoritativeRuleEventKind::DecisionCreated {
            decision: DecisionId(2),
        },
    };
    let result = accepted_product_for_contract(&before, after, vec![event]);
    assert_contract_rejects_without_mutation(&before, &result);
}

fn outcome_occurrence_product(
    policy: PerspectiveObservationPolicyV1,
) -> (EngineState, TransitionResult) {
    let before = state_without_pending_decision();
    let mut after = before.clone();
    after.revision = StateRevision(1);
    after
        .knowledge
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .next_visible_sequence = mtgml_model::VisibleSequence(2);
    after.allocators.next_rule_event_id = mtgml_model::RuleEventId(2);
    let event = AuthoritativeRuleEvent {
        event_id: mtgml_model::RuleEventId(1),
        state_revision: after.revision,
        event: AuthoritativeRuleEventKind::PerspectiveOccurrence {
            lifecycle: mtgml_state::PerspectiveLifecycleAuditV1 {
                perspective: PlayerId(1),
                sequence: mtgml_model::VisibleSequence(1),
                mutation: mtgml_state::PerspectiveLifecycleMutationV1::default(),
            },
            observation: policy,
        },
    };
    let result = accepted_product_for_contract(&before, after, vec![event]);
    (before, result)
}

#[test]
fn visible_random_outcome_requires_authoritative_rng_provenance() {
    let (before, result) = outcome_occurrence_product(
        PerspectiveObservationPolicyV1::SawRandomOutcome {
            label: "die".into(),
            exclusive_upper_bound: 6,
            value: 5,
        },
    );
    assert_contract_rejects_without_mutation(&before, &result);
}

#[test]
fn announced_outcome_is_characterized_separately_from_rng() {
    let (before, result) = outcome_occurrence_product(
        PerspectiveObservationPolicyV1::AnnouncedOutcome {
            code: "declared".into(),
        },
    );
    assert!(validate_transition_contract(&before, &result).is_ok());
}

#[test]
fn occurrence_must_not_bind_to_a_future_zone_transition() {
    let mut before = synthetic_state();
    before.execution.pending_decision = None;
    let old_location = before.zones.locations[&mtgml_model::GameObjectId(2)].clone();
    let new_location = mtgml_state::ZoneLocation {
        zone: mtgml_model::ZoneKind::Exile,
        player: None,
        position: mtgml_state::ZonePosition::Unordered,
        visibility: mtgml_state::VisibilityPartition::Public,
        partition: None,
    };
    let old_object = before.zones.objects[&mtgml_model::GameObjectId(2)].clone();
    let new_object = mtgml_state::GameObject {
        id: mtgml_model::GameObjectId(3),
        physical_card: old_object.physical_card,
        card_definition: old_object.card_definition,
        owner: old_object.owner,
        controller: old_object.controller,
        tapped: old_object.tapped,
        face_down: old_object.face_down,
    };
    let audit = mtgml_state::PerspectiveLifecycleAuditV1 {
        perspective: PlayerId(2),
        sequence: mtgml_model::VisibleSequence(1),
        mutation: mtgml_state::PerspectiveLifecycleMutationV1 {
            identity: mtgml_state::IdentityMutationV1::Retire {
                opaque: mtgml_model::OpaqueObjectId(2),
                object: mtgml_model::GameObjectId(2),
            },
            knowledge: Some(mtgml_state::KnowledgeMutationV1::Invalidate {
                opaque: mtgml_model::OpaqueObjectId(2),
                reason: mtgml_state::KnowledgeInvalidationReason::ExplicitForget,
                invalidation_provenance: mtgml_state::KnowledgeAcquisitionReason::Observed {
                    channel: mtgml_state::KnowledgeHistoryChannel::Public,
                    sequence: mtgml_model::VisibleSequence(1),
                    cause: mtgml_state::KnowledgeAcquisitionCause::PublicEvent,
                },
            }),
        },
    };
    let mut after = before.clone();
    mtgml_state::apply_perspective_lifecycle(&mut after, &audit).unwrap();
    after.zones.objects.remove(&mtgml_model::GameObjectId(2));
    after.zones.locations.remove(&mtgml_model::GameObjectId(2));
    let old_zone_key = old_location.key();
    let remove_old_zone_key = if let Some(objects) =
        after.zones.ordered_zones.get_mut(&old_zone_key)
    {
        objects.retain(|object| *object != mtgml_model::GameObjectId(2));
        objects.is_empty()
    } else {
        false
    };
    if remove_old_zone_key {
        after.zones.ordered_zones.remove(&old_zone_key);
    }
    after
        .zones
        .objects
        .insert(mtgml_model::GameObjectId(3), new_object.clone());
    after
        .zones
        .locations
        .insert(mtgml_model::GameObjectId(3), new_location.clone());
    after.allocators.next_object_id = mtgml_model::GameObjectId(4);
    after.revision = StateRevision(1);
    after.allocators.next_rule_event_id = mtgml_model::RuleEventId(3);
    let events = vec![
        AuthoritativeRuleEvent {
            event_id: mtgml_model::RuleEventId(1),
            state_revision: after.revision,
            event: AuthoritativeRuleEventKind::PerspectiveOccurrence {
                lifecycle: audit,
                observation: PerspectiveObservationPolicyV1::MovedInSight {
                    from_zone: mtgml_model::ZoneKind::Library,
                    to_zone: mtgml_model::ZoneKind::Exile,
                    old_object: mtgml_model::GameObjectId(2),
                    new_object: mtgml_model::GameObjectId(3),
                    reveals_old: true,
                    reveals_new: false,
                },
            },
        },
        AuthoritativeRuleEvent {
            event_id: mtgml_model::RuleEventId(2),
            state_revision: after.revision,
            event: AuthoritativeRuleEventKind::ZoneTransition {
                transition: Box::new(mtgml_state::ZoneTransition {
                    old_object: mtgml_model::GameObjectId(2),
                    new_object: mtgml_model::GameObjectId(3),
                    physical_card: old_object.physical_card,
                    from: old_location.clone(),
                    to: new_location,
                    last_known: mtgml_state::ObjectSnapshot {
                        object: old_object.id,
                        physical_card: old_object.physical_card,
                        card_definition: old_object.card_definition,
                        owner: old_object.owner,
                        controller: old_object.controller,
                        tapped: old_object.tapped,
                        face_down: old_object.face_down,
                        location: old_location,
                    },
                    new_snapshot: mtgml_state::ObjectSnapshot {
                        object: new_object.id,
                        physical_card: new_object.physical_card,
                        card_definition: new_object.card_definition,
                        owner: new_object.owner,
                        controller: new_object.controller,
                        tapped: new_object.tapped,
                        face_down: new_object.face_down,
                        location: mtgml_state::ZoneLocation {
                            zone: mtgml_model::ZoneKind::Exile,
                            player: None,
                            position: mtgml_state::ZonePosition::Unordered,
                            visibility: mtgml_state::VisibilityPartition::Public,
                            partition: None,
                        },
                    },
                }),
            },
        },
    ];
    let result = accepted_product_for_contract(&before, after, events);
    assert_contract_rejects_without_mutation(&before, &result);
}
