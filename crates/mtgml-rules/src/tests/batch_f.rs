fn single_event_product(
    before: &EngineState,
    after: EngineState,
    event: AuthoritativeRuleEvent,
) -> TransitionResult {
    let audit = vec![event.event.semantic_delta()];
    let delta = mtgml_state::StateDelta::between(before, &after, audit).unwrap();
    TransitionResult {
        accepted: true,
        next_state: after.clone(),
        delta,
        events: vec![event],
        next_decision: None,
        status: mtgml_model::EpisodeStatus::Running,
    }
}

#[test]
fn fnd_009_noop_life_change_is_rejected_by_the_transition_contract() {
    let before = state_without_pending_decision();
    let mut after = before.clone();
    after.revision = StateRevision(1);
    after.allocators.next_rule_event_id = mtgml_model::RuleEventId(2);
    let event = AuthoritativeRuleEvent {
        event_id: mtgml_model::RuleEventId(1),
        state_revision: StateRevision(1),
        event: AuthoritativeRuleEventKind::LifeChanged {
            player: PlayerId(1),
            from: 40,
            to: 40,
        },
    };
    let result = single_event_product(&before, after, event);
    let before_snapshot = before.clone();
    let result_snapshot = result.clone();
    assert!(matches!(
        validate_transition_contract(&before, &result),
        Err(TransitionViolation::LifeChange)
    ));
    assert_eq!(before, before_snapshot);
    assert_eq!(result, result_snapshot);
}

#[test]
fn fnd_009_noop_object_tap_is_rejected_by_the_transition_contract() {
    let before = state_without_pending_decision();
    let mut after = before.clone();
    after.revision = StateRevision(1);
    after.allocators.next_rule_event_id = mtgml_model::RuleEventId(2);
    let event = AuthoritativeRuleEvent {
        event_id: mtgml_model::RuleEventId(1),
        state_revision: StateRevision(1),
        event: AuthoritativeRuleEventKind::ObjectTapped {
            object: mtgml_model::GameObjectId(1),
            from: false,
            to: false,
        },
    };
    let result = single_event_product(&before, after, event);
    let before_snapshot = before.clone();
    let result_snapshot = result.clone();
    assert!(matches!(
        validate_transition_contract(&before, &result),
        Err(TransitionViolation::TapChange)
    ));
    assert_eq!(before, before_snapshot);
    assert_eq!(result, result_snapshot);
}

#[test]
fn fnd_009_real_life_and_tap_mutations_remain_valid() {
    let before = state_without_pending_decision();

    let mut life_after = before.clone();
    life_after.revision = StateRevision(1);
    life_after.allocators.next_rule_event_id = mtgml_model::RuleEventId(2);
    life_after.core.players.get_mut(&PlayerId(1)).unwrap().life = 39;
    let life_event = AuthoritativeRuleEvent {
        event_id: mtgml_model::RuleEventId(1),
        state_revision: StateRevision(1),
        event: AuthoritativeRuleEventKind::LifeChanged {
            player: PlayerId(1),
            from: 40,
            to: 39,
        },
    };
    assert!(validate_transition_contract(
        &before,
        &single_event_product(&before, life_after, life_event)
    )
    .is_ok());

    let mut tap_after = before.clone();
    tap_after.revision = StateRevision(1);
    tap_after.allocators.next_rule_event_id = mtgml_model::RuleEventId(2);
    tap_after
        .zones
        .objects
        .get_mut(&mtgml_model::GameObjectId(1))
        .unwrap()
        .tapped = true;
    let tap_event = AuthoritativeRuleEvent {
        event_id: mtgml_model::RuleEventId(1),
        state_revision: StateRevision(1),
        event: AuthoritativeRuleEventKind::ObjectTapped {
            object: mtgml_model::GameObjectId(1),
            from: false,
            to: true,
        },
    };
    assert!(validate_transition_contract(
        &before,
        &single_event_product(&before, tap_after, tap_event)
    )
    .is_ok());
}

#[test]
fn fnd_009_zone_transition_same_incarnation_is_rejected() {
    let before = state_without_pending_decision();
    let snapshot = crate::snapshots::object_snapshots(&before)
        .unwrap()
        .get(&mtgml_model::GameObjectId(1))
        .unwrap()
        .clone();
    let mut after = before.clone();
    after.revision = StateRevision(1);
    after.allocators.next_rule_event_id = mtgml_model::RuleEventId(2);
    let transition = mtgml_state::ZoneTransition {
        old_object: mtgml_model::GameObjectId(1),
        new_object: mtgml_model::GameObjectId(1),
        physical_card: snapshot.physical_card,
        from: snapshot.location.clone(),
        to: snapshot.location.clone(),
        last_known: snapshot.clone(),
        new_snapshot: snapshot,
    };
    let event = AuthoritativeRuleEvent {
        event_id: mtgml_model::RuleEventId(1),
        state_revision: StateRevision(1),
        event: AuthoritativeRuleEventKind::ZoneTransition {
            transition: Box::new(transition),
        },
    };
    let result = single_event_product(&before, after, event);
    assert!(matches!(
        validate_transition_contract(&before, &result),
        Err(TransitionViolation::ZoneTransition)
    ));
}

#[test]
fn fnd_027_final_identity_mismatch_is_rejected_by_the_rules_cursor() {
    let (before, result) = outcome_occurrence_product(
        PerspectiveObservationPolicyV1::AnnouncedOutcome {
            code: "identity-proof".into(),
        },
    );
    let mut tampered = result;
    tampered
        .next_state
        .perspective_identities
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .next_opaque_object_id = mtgml_model::OpaqueObjectId(3);
    assert!(mtgml_state::validate_engine_state(&tampered.next_state).is_ok());
    tampered.delta = mtgml_state::StateDelta::between(
        &before,
        &tampered.next_state,
        tampered
            .events
            .iter()
            .map(|event| event.event.semantic_delta())
            .collect(),
    )
    .unwrap();
    assert!(matches!(
        validate_transition_contract(&before, &tampered),
        Err(TransitionViolation::OccurrencePairing)
    ));
}
