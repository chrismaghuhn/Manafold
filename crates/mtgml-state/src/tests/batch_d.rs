// Ownership fragment: Pre-M3 Batch-D RED characterization and focused
// chronology/ordered-zone evidence. Included lexically by tests.rs.

fn observed_public_at(sequence: u64) -> KnowledgeAcquisitionReason {
    observed(
        KnowledgeHistoryChannel::Public,
        sequence,
        KnowledgeAcquisitionCause::PublicEvent,
    )
}

fn observed_public_fact(sequence: u64) -> KnownLocationFactV2 {
    fact(public_location(), observed_public_at(sequence))
}

fn active_chronology_state() -> EngineState {
    let mut state = synthetic_state();
    let knowledge = state.knowledge.players.get_mut(&PlayerId(1)).unwrap();
    knowledge.next_visible_sequence = VisibleSequence(20);
    let record = knowledge.active.get_mut(&OpaqueObjectId(1)).unwrap();
    record.acquisition = KnowledgeAcquisitionReason::InitialConfiguration;
    record.known_location = None;
    record.historical_locations.clear();
    state
}

fn retired_chronology_state() -> EngineState {
    let mut state = synthetic_state();
    let identity = state
        .perspective_identities
        .players
        .get_mut(&PlayerId(1))
        .unwrap();
    identity.next_opaque_object_id = OpaqueObjectId(6);
    identity.retired_object_ids.insert(OpaqueObjectId(5));
    let knowledge = state.knowledge.players.get_mut(&PlayerId(1)).unwrap();
    knowledge.next_visible_sequence = VisibleSequence(20);
    knowledge
        .retired
        .insert(OpaqueObjectId(5), retired_record(OpaqueObjectId(5)));
    knowledge
        .retired
        .get_mut(&OpaqueObjectId(5))
        .unwrap()
        .invalidation
        .provenance = observed_public_at(2);
    state
}

fn two_object_ordered_state() -> EngineState {
    let mut state = synthetic_state();
    let mut object = state.zones.objects[&GameObjectId(2)].clone();
    object.id = GameObjectId(3);
    object.physical_card = Some(PhysicalCardId(3));
    object.card_definition = CardDefinitionId(3);
    state.zones.objects.insert(object.id, object);

    let mut location = state.zones.locations[&GameObjectId(2)].clone();
    location.position = ZonePosition::Top { offset: 1 };
    state.zones.locations.insert(GameObjectId(3), location);

    let key = state.zones.locations[&GameObjectId(2)].key();
    state
        .zones
        .ordered_zones
        .get_mut(&key)
        .unwrap()
        .push(GameObjectId(3));
    state.allocators.next_object_id = GameObjectId(4);
    state
}

fn swap_canonical_two_object_order(state: &mut EngineState) {
    set_object_two_position(state, ZonePosition::Top { offset: 1 });
    state
        .zones
        .locations
        .get_mut(&GameObjectId(3))
        .unwrap()
        .position = ZonePosition::Top { offset: 0 };
    let key = state.zones.locations[&GameObjectId(2)].key();
    state
        .zones
        .ordered_zones
        .get_mut(&key)
        .unwrap()
        .reverse();
}

fn set_object_two_position(state: &mut EngineState, position: ZonePosition) {
    state
        .zones
        .locations
        .get_mut(&GameObjectId(2))
        .unwrap()
        .position = position;
    state
        .knowledge
        .players
        .get_mut(&PlayerId(2))
        .unwrap()
        .active
        .get_mut(&OpaqueObjectId(2))
        .unwrap()
        .known_location
        .as_mut()
        .unwrap()
        .location
        .position = position;
}

fn assert_rejected_without_mutation(state: EngineState) {
    let before = state.clone();
    assert!(validate_engine_state(&state).is_err());
    assert_eq!(state, before);
}

#[test]
fn fnd_002_rejects_acquisition_newer_than_history() {
    let mut state = active_chronology_state();
    let record = state
        .knowledge
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .active
        .get_mut(&OpaqueObjectId(1))
        .unwrap();
    record.acquisition = observed_public_at(4);
    record.historical_locations.push(observed_public_fact(3));
    assert_rejected_without_mutation(state);
}

#[test]
fn fnd_002_rejects_locally_increasing_history_before_acquisition() {
    let mut state = active_chronology_state();
    let record = state
        .knowledge
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .active
        .get_mut(&OpaqueObjectId(1))
        .unwrap();
    record.acquisition = observed_public_at(4);
    record.historical_locations = vec![observed_public_fact(3), observed_public_fact(5)];
    assert_rejected_without_mutation(state);
}

#[test]
fn fnd_002_rejects_current_older_than_newest_history() {
    let mut state = active_chronology_state();
    let record = state
        .knowledge
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .active
        .get_mut(&OpaqueObjectId(1))
        .unwrap();
    record.acquisition = observed_public_at(2);
    record.historical_locations = vec![observed_public_fact(4), observed_public_fact(6)];
    record.known_location = Some(fact(public_location(), observed_public_at(5)));
    assert_rejected_without_mutation(state);
}

#[test]
fn fnd_002_rejects_retired_last_known_older_than_newest_history() {
    let mut state = retired_chronology_state();
    let record = state
        .knowledge
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .retired
        .get_mut(&OpaqueObjectId(5))
        .unwrap();
    record.historical_locations = vec![observed_public_fact(5), observed_public_fact(7)];
    record.last_known_location = Some(fact(public_location(), observed_public_at(6)));
    record.invalidation.provenance = observed_public_at(8);
    assert_rejected_without_mutation(state);
}

#[test]
fn fnd_002_rejects_invalidation_earlier_than_acquisition() {
    let mut state = retired_chronology_state();
    let record = state
        .knowledge
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .retired
        .get_mut(&OpaqueObjectId(5))
        .unwrap();
    record.acquisition = observed_public_at(5);
    record.invalidation.provenance = observed_public_at(4);
    assert_rejected_without_mutation(state);
}

#[test]
fn fnd_002_rejects_invalidation_earlier_than_newest_history() {
    let mut state = retired_chronology_state();
    let record = state
        .knowledge
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .retired
        .get_mut(&OpaqueObjectId(5))
        .unwrap();
    record.historical_locations = vec![observed_public_fact(5)];
    record.invalidation.provenance = observed_public_at(4);
    assert_rejected_without_mutation(state);
}

#[test]
fn fnd_002_rejects_invalidation_earlier_than_last_known() {
    let mut state = retired_chronology_state();
    let record = state
        .knowledge
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .retired
        .get_mut(&OpaqueObjectId(5))
        .unwrap();
    record.last_known_location = Some(fact(public_location(), observed_public_at(5)));
    record.invalidation.provenance = observed_public_at(4);
    assert_rejected_without_mutation(state);
}

#[test]
fn fnd_002_same_acquire_provenance_for_current_location_is_accepted() {
    let mut state = active_chronology_state();
    let acquisition = observed_public_at(4);
    let record = state
        .knowledge
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .active
        .get_mut(&OpaqueObjectId(1))
        .unwrap();
    record.acquisition = acquisition;
    record.known_location = Some(fact(public_location(), acquisition));
    state.knowledge.players.get_mut(&PlayerId(1)).unwrap().next_visible_sequence =
        VisibleSequence(5);
    assert_eq!(validate_engine_state(&state), Ok(()));
}

#[test]
fn fnd_002_initial_acquisition_with_initial_prefix_and_observed_history_is_accepted() {
    let mut state = active_chronology_state();
    let record = state
        .knowledge
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .active
        .get_mut(&OpaqueObjectId(1))
        .unwrap();
    record.historical_locations = vec![
        fact(public_location(), KnowledgeAcquisitionReason::InitialConfiguration),
        observed_public_fact(3),
    ];
    assert_eq!(validate_engine_state(&state), Ok(()));
}

#[test]
fn fnd_002_sequence_gaps_are_accepted() {
    let mut state = active_chronology_state();
    let record = state
        .knowledge
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .active
        .get_mut(&OpaqueObjectId(1))
        .unwrap();
    record.historical_locations = vec![observed_public_fact(2), observed_public_fact(5)];
    assert_eq!(validate_engine_state(&state), Ok(()));
}

#[test]
fn fnd_002_rejects_multiple_initial_location_facts() {
    let mut state = active_chronology_state();
    let record = state
        .knowledge
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .active
        .get_mut(&OpaqueObjectId(1))
        .unwrap();
    record.historical_locations = vec![
        fact(public_location(), KnowledgeAcquisitionReason::InitialConfiguration),
        fact(public_location(), KnowledgeAcquisitionReason::InitialConfiguration),
    ];
    assert_rejected_without_mutation(state);
}

#[test]
fn fnd_002_rejects_initial_location_after_observed_history() {
    let mut state = active_chronology_state();
    let record = state
        .knowledge
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .active
        .get_mut(&OpaqueObjectId(1))
        .unwrap();
    record.historical_locations = vec![
        observed_public_fact(2),
        fact(public_location(), KnowledgeAcquisitionReason::InitialConfiguration),
    ];
    assert_rejected_without_mutation(state);
}

#[test]
fn fnd_002_rejects_current_older_than_acquisition() {
    let mut state = active_chronology_state();
    let record = state
        .knowledge
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .active
        .get_mut(&OpaqueObjectId(1))
        .unwrap();
    record.acquisition = observed_public_at(5);
    record.known_location = Some(fact(public_location(), observed_public_at(4)));
    assert_rejected_without_mutation(state);
}

#[test]
fn fnd_002_rejects_initial_current_after_observed_history() {
    let mut state = active_chronology_state();
    let record = state
        .knowledge
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .active
        .get_mut(&OpaqueObjectId(1))
        .unwrap();
    record.historical_locations = vec![observed_public_fact(2)];
    record.known_location = Some(fact(
        public_location(),
        KnowledgeAcquisitionReason::InitialConfiguration,
    ));
    assert_rejected_without_mutation(state);
}

#[test]
fn fnd_002_future_provenance_equal_to_next_is_rejected() {
    let mut state = active_chronology_state();
    state.knowledge.players.get_mut(&PlayerId(1)).unwrap().next_visible_sequence =
        VisibleSequence(4);
    state
        .knowledge
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .active
        .get_mut(&OpaqueObjectId(1))
        .unwrap()
        .acquisition = observed_public_at(4);
    assert!(validate_engine_state(&state).is_err());
}

#[test]
fn fnd_002_future_provenance_beyond_next_is_rejected() {
    let mut state = active_chronology_state();
    state.knowledge.players.get_mut(&PlayerId(1)).unwrap().next_visible_sequence =
        VisibleSequence(4);
    state
        .knowledge
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .active
        .get_mut(&OpaqueObjectId(1))
        .unwrap()
        .acquisition = observed_public_at(5);
    assert!(validate_engine_state(&state).is_err());
}

#[test]
fn fnd_002_lifecycle_retains_acquire_location_with_original_sequence() {
    let mut state = lifecycle_fixture();
    let location = ZoneLocation {
        zone: ZoneKind::Exile,
        player: None,
        position: ZonePosition::Unordered,
        visibility: VisibilityPartition::Public,
        partition: None,
    };
    let acquisition = observed_at(
        1,
        KnowledgeHistoryChannel::Public,
        KnowledgeAcquisitionCause::ExplicitReveal,
    );
    apply_perspective_lifecycle(
        &mut state,
        &PerspectiveLifecycleAuditV1 {
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
                    location: Some(location.clone()),
                    acquisition,
                }),
            },
        },
    )
    .unwrap();
    let update = observed_at(
        2,
        KnowledgeHistoryChannel::Public,
        KnowledgeAcquisitionCause::PublicEvent,
    );
    apply_perspective_lifecycle(
        &mut state,
        &PerspectiveLifecycleAuditV1 {
            perspective: PlayerId(1),
            sequence: VisibleSequence(2),
            mutation: PerspectiveLifecycleMutationV1 {
                identity: IdentityMutationV1::None,
                knowledge: Some(KnowledgeMutationV1::UpdateLocation {
                    opaque: OpaqueObjectId(2),
                    fact: KnownLocationFactV2 {
                        location,
                        provenance: update,
                    },
                }),
            },
        },
    )
    .unwrap();
    validate_engine_state(&state).unwrap();
    let record = &state.knowledge.players[&PlayerId(1)].active[&OpaqueObjectId(2)];
    assert_eq!(record.acquisition, acquisition);
    assert_eq!(record.historical_locations[0].provenance, acquisition);
    assert_eq!(record.known_location.as_ref().unwrap().provenance, update);
}

#[test]
fn fnd_006b_rejects_swapped_index_values() {
    let mut state = two_object_ordered_state();
    set_object_two_position(&mut state, ZonePosition::Index { index: 1 });
    state
        .zones
        .locations
        .get_mut(&GameObjectId(3))
        .unwrap()
        .position = ZonePosition::Index { index: 0 };
    assert_rejected_without_mutation(state);
}

#[test]
fn fnd_006b_rejects_duplicate_top_zero_offsets() {
    let mut state = two_object_ordered_state();
    set_object_two_position(&mut state, ZonePosition::Top { offset: 0 });
    state
        .zones
        .locations
        .get_mut(&GameObjectId(3))
        .unwrap()
        .position = ZonePosition::Top { offset: 0 };
    assert_rejected_without_mutation(state);
}

#[test]
fn fnd_006b_rejects_out_of_range_index() {
    let mut state = two_object_ordered_state();
    set_object_two_position(&mut state, ZonePosition::Index { index: 9 });
    assert_rejected_without_mutation(state);
}

#[test]
fn fnd_006b_rejects_out_of_range_top_offset() {
    let mut state = two_object_ordered_state();
    set_object_two_position(&mut state, ZonePosition::Top { offset: 9 });
    assert_rejected_without_mutation(state);
}

#[test]
fn fnd_006b_rejects_out_of_range_bottom_offset() {
    let mut state = two_object_ordered_state();
    set_object_two_position(&mut state, ZonePosition::Bottom { offset: 9 });
    assert_rejected_without_mutation(state);
}

#[test]
fn fnd_006b_rejects_equivalent_index_spelling() {
    let mut state = two_object_ordered_state();
    set_object_two_position(&mut state, ZonePosition::Index { index: 0 });
    state
        .zones
        .locations
        .get_mut(&GameObjectId(3))
        .unwrap()
        .position = ZonePosition::Index { index: 1 };
    assert_rejected_without_mutation(state);
}

#[test]
fn fnd_006b_rejects_unrelated_empty_declared_key() {
    let mut state = two_object_ordered_state();
    state.zones.ordered_zones.insert(
        ZoneKey {
            zone: ZoneKind::Library,
            player: Some(PlayerId(1)),
            visibility: VisibilityPartition::FaceDown,
            partition: None,
        },
        Vec::new(),
    );
    assert_rejected_without_mutation(state);
}

#[test]
fn fnd_006b_rejects_empty_key_alongside_valid_key() {
    let mut state = two_object_ordered_state();
    state.zones.ordered_zones.insert(
        ZoneKey {
            zone: ZoneKind::Graveyard,
            player: Some(PlayerId(2)),
            visibility: VisibilityPartition::FaceDown,
            partition: None,
        },
        Vec::new(),
    );
    assert_rejected_without_mutation(state);
}

#[test]
fn fnd_006b_rejects_bottom_in_retained_history() {
    let mut state = active_chronology_state();
    state
        .knowledge
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .active
        .get_mut(&OpaqueObjectId(1))
        .unwrap()
        .historical_locations
        .push(fact(
            ZoneLocation {
                position: ZonePosition::Bottom { offset: 0 },
                ..public_location()
            },
            observed_public_at(2),
        ));
    assert_rejected_without_mutation(state);
}

#[test]
fn fnd_006b_rejects_index_in_retired_last_known() {
    let mut state = retired_chronology_state();
    state
        .knowledge
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .retired
        .get_mut(&OpaqueObjectId(5))
        .unwrap()
        .last_known_location = Some(fact(
            ZoneLocation {
                position: ZonePosition::Index { index: 0 },
                ..public_location()
            },
            observed_public_at(1),
        ));
    assert_rejected_without_mutation(state);
}

#[test]
fn fnd_002_invalid_chronology_cannot_obtain_a_v4_digest() {
    let mut state = active_chronology_state();
    let record = state
        .knowledge
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .active
        .get_mut(&OpaqueObjectId(1))
        .unwrap();
    record.acquisition = observed_public_at(4);
    record.historical_locations.push(observed_public_fact(3));
    assert_eq!(state.digest(), Err(StateDigestError::StateInvariant));
}

#[test]
fn fnd_006b_noncanonical_position_cannot_obtain_a_v4_digest() {
    let mut state = synthetic_state();
    set_object_two_position(&mut state, ZonePosition::Bottom { offset: 0 });
    assert_eq!(state.digest(), Err(StateDigestError::StateInvariant));
}

#[test]
fn fnd_006b_valid_canonical_reorder_changes_the_v4_digest() {
    let baseline = two_object_ordered_state();
    let mut reordered = baseline.clone();
    swap_canonical_two_object_order(&mut reordered);
    validate_engine_state(&reordered).unwrap();
    assert_ne!(baseline.digest().unwrap(), reordered.digest().unwrap());
}
