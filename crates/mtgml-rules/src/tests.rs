use super::*;
use mtgml_decision::{DecisionKind, DecisionVisibility, PlayerDecisionRequest};
use mtgml_model::{
    AbilityInstanceId, CardDefinitionId, ContinuationId, DecisionId, EffectInstanceId,
    EpisodeStatus, GameObjectId, OpaqueAbilityId, OpaqueObjectId, PhysicalCardId, PlayerId,
    RuleEventId, StackObjectId, StateRevision, TriggerInstanceId, ZoneKind,
};
use mtgml_random::RandomStateV1;
use mtgml_state::{
    CoreRulesState, EngineState, ExecutionState, FormatState, GameObject, IdentityAllocatorState,
    KnowledgeState, ObjectSnapshot, PendingDecisionRecord, PerspectiveIdentityMap,
    PerspectiveIdentityState, PlayerKnowledgeState, PlayerState, StateDelta, VisibilityPartition,
    ZoneLocation, ZonePosition, ZoneState, ZoneTransition,
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
    state
        .zones
        .ordered_zones
        .entry(location.key())
        .or_default()
        .push(id);
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
    let objects = state.zones.ordered_zones.get_mut(&key).unwrap();
    objects.retain(|current| *current != object);
    if objects.is_empty() {
        state.zones.ordered_zones.remove(&key);
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
