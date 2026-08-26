// Ownership fragment: retained-knowledge and opaque-identity evidence. Included lexically by tests.rs so
// every identity remains tests::<name>.

#[test]
fn active_knowledge_requires_exactly_one_live_mapping() {
    let mut state = synthetic_state();
    state
        .perspective_identities
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .opaque_to_object
        .remove(&OpaqueObjectId(1));
    state
        .perspective_identities
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .object_to_opaque
        .remove(&GameObjectId(1));
    assert!(matches!(
        validate_engine_state(&state),
        Err(EngineStateViolation::KnowledgeMismatch)
    ));
}

#[test]
fn reverse_identity_maps_must_agree() {
    let mut state = synthetic_state();
    state
        .perspective_identities
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .object_to_opaque
        .insert(GameObjectId(1), OpaqueObjectId(9));
    assert!(matches!(
        validate_engine_state(&state),
        Err(EngineStateViolation::M2Shape(
            M2ShapeViolation::IdentityMapping
        ))
    ));
}

#[test]
fn retired_opaque_identity_must_not_stay_active() {
    let mut state = synthetic_state();
    let identity = state
        .perspective_identities
        .players
        .get_mut(&PlayerId(1))
        .unwrap();
    identity.retired_object_ids.insert(OpaqueObjectId(1));
    assert!(matches!(
        validate_engine_state(&state),
        Err(EngineStateViolation::M2Shape(
            M2ShapeViolation::RetiredIdentity
        ))
    ));
}

#[test]
fn opaque_allocator_must_stay_ahead_of_issued_ids() {
    let mut state = synthetic_state();
    state
        .perspective_identities
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .next_opaque_object_id = OpaqueObjectId(1);
    assert!(matches!(
        validate_engine_state(&state),
        Err(EngineStateViolation::M2Shape(M2ShapeViolation::Allocator))
    ));
}

#[test]
fn retired_knowledge_must_not_keep_a_live_mapping() {
    let mut state = synthetic_state();
    let identity = state
        .perspective_identities
        .players
        .get_mut(&PlayerId(1))
        .unwrap();
    identity.next_opaque_object_id = OpaqueObjectId(6);
    identity.retired_object_ids.insert(OpaqueObjectId(5));
    let knowledge = state.knowledge.players.get_mut(&PlayerId(1)).unwrap();
    knowledge
        .retired
        .insert(OpaqueObjectId(5), retired_record(OpaqueObjectId(5)));
    validate_engine_state(&state).unwrap();

    // The same opaque identity cannot simultaneously be retired and active.
    let identity = state
        .perspective_identities
        .players
        .get_mut(&PlayerId(1))
        .unwrap();
    identity
        .opaque_to_object
        .insert(OpaqueObjectId(5), GameObjectId(2));
    identity
        .object_to_opaque
        .insert(GameObjectId(2), OpaqueObjectId(5));
    assert!(matches!(
        validate_engine_state(&state),
        Err(EngineStateViolation::M2Shape(
            M2ShapeViolation::RetiredIdentity
        ))
    ));
}

#[test]
fn known_location_must_match_the_live_association() {
    let mut state = synthetic_state();
    let knowledge = state.knowledge.players.get_mut(&PlayerId(1)).unwrap();
    knowledge
        .active
        .get_mut(&OpaqueObjectId(1))
        .unwrap()
        .known_location = Some(KnownLocationFactV2 {
        location: ZoneLocation {
            zone: ZoneKind::Exile,
            ..public_location()
        },
        provenance: KnowledgeAcquisitionReason::InitialConfiguration,
    });
    assert!(matches!(
        validate_engine_state(&state),
        Err(EngineStateViolation::KnowledgeMismatch)
    ));
}

#[test]
fn historical_location_sequences_must_increase() {
    let mut state = synthetic_state();
    let fact_at = |sequence: u64| {
        fact(
            public_location(),
            observed(
                KnowledgeHistoryChannel::Public,
                sequence,
                KnowledgeAcquisitionCause::PublicEvent,
            ),
        )
    };
    {
        let knowledge = state.knowledge.players.get_mut(&PlayerId(1)).unwrap();
        let record = knowledge.active.get_mut(&OpaqueObjectId(1)).unwrap();
        record.historical_locations = vec![fact_at(0), fact_at(0)];
    }
    assert!(matches!(
        validate_engine_state(&state),
        Err(EngineStateViolation::M2Shape(
            M2ShapeViolation::VisibleSequence
        ))
    ));
    {
        let knowledge = state.knowledge.players.get_mut(&PlayerId(1)).unwrap();
        let record = knowledge.active.get_mut(&OpaqueObjectId(1)).unwrap();
        record.historical_locations = vec![fact_at(1), fact_at(0)];
    }
    assert!(matches!(
        validate_engine_state(&state),
        Err(EngineStateViolation::M2Shape(
            M2ShapeViolation::VisibleSequence
        ))
    ));
}

#[test]
fn knowledge_provenance_must_not_be_future_dated() {
    let mut state = synthetic_state();
    let knowledge = state.knowledge.players.get_mut(&PlayerId(1)).unwrap();
    let record = knowledge.active.get_mut(&OpaqueObjectId(1)).unwrap();
    record.acquisition = observed(
        KnowledgeHistoryChannel::Public,
        5,
        KnowledgeAcquisitionCause::PublicEvent,
    );
    assert!(matches!(
        validate_engine_state(&state),
        Err(EngineStateViolation::M2Shape(
            M2ShapeViolation::VisibleSequence
        ))
    ));
}

#[test]
fn invalid_knowledge_cause_channel_combination_is_rejected() {
    let mut state = synthetic_state();
    let knowledge = state.knowledge.players.get_mut(&PlayerId(1)).unwrap();
    let record = knowledge.active.get_mut(&OpaqueObjectId(1)).unwrap();
    record.acquisition = observed(
        KnowledgeHistoryChannel::Private,
        0,
        KnowledgeAcquisitionCause::PublicEvent,
    );
    assert_eq!(
        validate_engine_state(&state),
        Err(EngineStateViolation::M2Shape(
            M2ShapeViolation::VisibleSequence
        ))
    );
}

#[test]
fn retired_knowledge_provenance_is_validated() {
    let mut state = synthetic_state();
    let identity = state
        .perspective_identities
        .players
        .get_mut(&PlayerId(1))
        .unwrap();
    identity.next_opaque_object_id = OpaqueObjectId(6);
    identity.retired_object_ids.insert(OpaqueObjectId(5));
    let knowledge = state.knowledge.players.get_mut(&PlayerId(1)).unwrap();
    let mut record = retired_record(OpaqueObjectId(5));
    record.acquisition = observed(
        KnowledgeHistoryChannel::Public,
        9,
        KnowledgeAcquisitionCause::PublicEvent,
    );
    knowledge.retired.insert(OpaqueObjectId(5), record);
    assert!(matches!(
        validate_engine_state(&state),
        Err(EngineStateViolation::M2Shape(
            M2ShapeViolation::VisibleSequence
        ))
    ));
}

#[test]
fn invalidation_must_carry_an_observed_visible_sequence() {
    let mut state = synthetic_state();
    let identity = state
        .perspective_identities
        .players
        .get_mut(&PlayerId(1))
        .unwrap();
    identity.next_opaque_object_id = OpaqueObjectId(6);
    identity.retired_object_ids.insert(OpaqueObjectId(5));
    let knowledge = state.knowledge.players.get_mut(&PlayerId(1)).unwrap();
    let mut record = retired_record(OpaqueObjectId(5));
    record.invalidation.provenance = KnowledgeAcquisitionReason::InitialConfiguration;
    knowledge.retired.insert(OpaqueObjectId(5), record);
    assert_eq!(
        validate_engine_state(&state),
        Err(EngineStateViolation::M2Shape(M2ShapeViolation::Knowledge))
    );
}

#[test]
fn tracked_incarnation_remap_keeps_opaque_and_allocator() {
    let mut state = lifecycle_fixture();
    // First reveal GO3 to P1 as opaque 2.
    let reveal = PerspectiveLifecycleAuditV1 {
        perspective: PlayerId(1),
        sequence: VisibleSequence(1),
        mutation: PerspectiveLifecycleMutationV1 {
            identity: IdentityMutationV1::Allocate {
                opaque: OpaqueObjectId(2),
                object: GameObjectId(3),
            },
            knowledge: Some(KnowledgeMutationV1::Acquire {
                opaque: OpaqueObjectId(2),
                definition: Some(CardDefinitionId(3)),
                location: Some(state.zones.locations[&GameObjectId(3)].clone()),
                acquisition: observed_at(
                    1,
                    crate::knowledge::KnowledgeHistoryChannel::Public,
                    crate::knowledge::KnowledgeAcquisitionCause::ExplicitReveal,
                ),
            }),
        },
    };
    apply_perspective_lifecycle(&mut state, &reveal).unwrap();
    // Tracked incarnation change GO3 -> GO4: same opaque, no allocator advance.
    let remap = PerspectiveLifecycleAuditV1 {
        perspective: PlayerId(1),
        sequence: VisibleSequence(2),
        mutation: PerspectiveLifecycleMutationV1 {
            identity: IdentityMutationV1::Remap {
                opaque: OpaqueObjectId(2),
                from_object: GameObjectId(3),
                to_object: GameObjectId(4),
            },
            knowledge: Some(KnowledgeMutationV1::CurrentToHistory {
                opaque: OpaqueObjectId(2),
                observed_definition: Some(CardDefinitionId(4)),
            }),
        },
    };
    apply_perspective_lifecycle(&mut state, &remap).unwrap();
    validate_engine_state(&state).unwrap();
    let identity = &state.perspective_identities.players[&PlayerId(1)];
    assert_eq!(
        identity.opaque_to_object.get(&OpaqueObjectId(2)),
        Some(&GameObjectId(4))
    );
    assert_eq!(identity.next_opaque_object_id, OpaqueObjectId(3));
    assert!(!identity.object_to_opaque.contains_key(&GameObjectId(3)));
    let record = &state.knowledge.players[&PlayerId(1)].active[&OpaqueObjectId(2)];
    assert!(record.known_location.is_none());
    assert_eq!(record.historical_locations.len(), 1);
}

#[test]
fn explicit_forget_retires_mapping_and_knowledge_together() {
    let mut state = lifecycle_fixture();
    let reveal = PerspectiveLifecycleAuditV1 {
        perspective: PlayerId(1),
        sequence: VisibleSequence(1),
        mutation: PerspectiveLifecycleMutationV1 {
            identity: IdentityMutationV1::Allocate {
                opaque: OpaqueObjectId(2),
                object: GameObjectId(3),
            },
            knowledge: Some(KnowledgeMutationV1::Acquire {
                opaque: OpaqueObjectId(2),
                definition: Some(CardDefinitionId(3)),
                location: Some(state.zones.locations[&GameObjectId(3)].clone()),
                acquisition: observed_at(
                    1,
                    crate::knowledge::KnowledgeHistoryChannel::Public,
                    crate::knowledge::KnowledgeAcquisitionCause::ExplicitReveal,
                ),
            }),
        },
    };
    apply_perspective_lifecycle(&mut state, &reveal).unwrap();
    let forget = PerspectiveLifecycleAuditV1 {
        perspective: PlayerId(1),
        sequence: VisibleSequence(2),
        mutation: PerspectiveLifecycleMutationV1 {
            identity: IdentityMutationV1::Retire {
                opaque: OpaqueObjectId(2),
                object: GameObjectId(3),
            },
            knowledge: Some(KnowledgeMutationV1::Invalidate {
                opaque: OpaqueObjectId(2),
                reason: crate::knowledge::KnowledgeInvalidationReason::ExplicitForget,
                invalidation_provenance: observed_at(
                    2,
                    crate::knowledge::KnowledgeHistoryChannel::Public,
                    crate::knowledge::KnowledgeAcquisitionCause::PublicEvent,
                ),
            }),
        },
    };
    apply_perspective_lifecycle(&mut state, &forget).unwrap();
    validate_engine_state(&state).unwrap();
    let identity = &state.perspective_identities.players[&PlayerId(1)];
    assert!(!identity.opaque_to_object.contains_key(&OpaqueObjectId(2)));
    assert!(!identity.object_to_opaque.contains_key(&GameObjectId(3)));
    assert!(identity.retired_object_ids.contains(&OpaqueObjectId(2)));
    assert_eq!(identity.next_opaque_object_id, OpaqueObjectId(3));
    let knowledge = &state.knowledge.players[&PlayerId(1)];
    assert!(!knowledge.active.contains_key(&OpaqueObjectId(2)));
    let retired = &knowledge.retired[&OpaqueObjectId(2)];
    assert!(retired.last_known_location.is_some());
    assert_eq!(
        retired.invalidation.reason,
        crate::knowledge::KnowledgeInvalidationReason::ExplicitForget
    );
}

#[test]
fn retired_opaque_cannot_be_reallocated_even_by_cursor_accident() {
    let mut state = lifecycle_fixture();
    // Drive P1's allocator forward so it would collide with a retired id.
    state
        .perspective_identities
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .next_opaque_object_id = OpaqueObjectId(2);
    state
        .perspective_identities
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .retired_object_ids
        .insert(OpaqueObjectId(2));
    let audit = PerspectiveLifecycleAuditV1 {
        perspective: PlayerId(1),
        sequence: VisibleSequence(1),
        mutation: PerspectiveLifecycleMutationV1 {
            identity: IdentityMutationV1::Allocate {
                opaque: OpaqueObjectId(2),
                object: GameObjectId(3),
            },
            knowledge: None,
        },
    };
    assert_eq!(
        apply_perspective_lifecycle(&mut state, &audit).unwrap_err(),
        LifecycleApplicationError::OpaqueRetired
    );
}
