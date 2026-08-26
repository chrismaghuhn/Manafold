// Ownership fragment: M2.E lifecycle fixture evidence. Included lexically by tests.rs so
// every identity remains tests::<name>.

#[test]
fn reveal_occurrence_allocates_and_acquires_with_bound_provenance() {
    let mut state = lifecycle_fixture();
    let audit = PerspectiveLifecycleAuditV1 {
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
                location: Some(crate::zones::ZoneLocation {
                    zone: ZoneKind::Exile,
                    player: None,
                    position: crate::zones::ZonePosition::Unordered,
                    visibility: crate::zones::VisibilityPartition::Public,
                    partition: None,
                }),
                acquisition: observed_at(
                    1,
                    crate::knowledge::KnowledgeHistoryChannel::Public,
                    crate::knowledge::KnowledgeAcquisitionCause::ExplicitReveal,
                ),
            }),
        },
    };
    apply_perspective_lifecycle(&mut state, &audit).unwrap();
    validate_engine_state(&state).unwrap();
    let identity = &state.perspective_identities.players[&PlayerId(1)];
    assert_eq!(
        identity.opaque_to_object.get(&OpaqueObjectId(2)),
        Some(&GameObjectId(3))
    );
    assert_eq!(identity.next_opaque_object_id, OpaqueObjectId(3));
    assert!(identity.retired_object_ids.is_empty());
    let knowledge = &state.knowledge.players[&PlayerId(1)];
    assert_eq!(knowledge.next_visible_sequence, VisibleSequence(2));
    let record = &knowledge.active[&OpaqueObjectId(2)];
    assert_eq!(record.card_definition, Some(CardDefinitionId(3)));
}

#[test]
fn provenance_sequence_mismatch_is_rejected_without_mutation() {
    let mut state = lifecycle_fixture();
    let before = state.clone();
    let audit = PerspectiveLifecycleAuditV1 {
        perspective: PlayerId(1),
        sequence: VisibleSequence(1),
        mutation: PerspectiveLifecycleMutationV1 {
            identity: IdentityMutationV1::Allocate {
                opaque: OpaqueObjectId(2),
                object: GameObjectId(3),
            },
            knowledge: Some(KnowledgeMutationV1::Acquire {
                opaque: OpaqueObjectId(2),
                definition: None,
                location: None,
                acquisition: observed_at(
                    2,
                    crate::knowledge::KnowledgeHistoryChannel::Public,
                    crate::knowledge::KnowledgeAcquisitionCause::ExplicitReveal,
                ),
            }),
        },
    };
    let error = apply_perspective_lifecycle(&mut state, &audit).unwrap_err();
    assert_eq!(error, LifecycleApplicationError::ProvenanceSequence);
    assert_eq!(state, before);
}

#[test]
fn cursor_mismatch_is_rejected() {
    let mut state = lifecycle_fixture();
    let audit = PerspectiveLifecycleAuditV1 {
        perspective: PlayerId(1),
        sequence: VisibleSequence(5),
        mutation: PerspectiveLifecycleMutationV1::default(),
    };
    assert_eq!(
        apply_perspective_lifecycle(&mut state, &audit).unwrap_err(),
        LifecycleApplicationError::CursorMismatch
    );
}

#[test]
fn unsequenced_provenance_is_rejected() {
    let mut state = lifecycle_fixture();
    let audit = PerspectiveLifecycleAuditV1 {
        perspective: PlayerId(2),
        sequence: VisibleSequence(1),
        mutation: PerspectiveLifecycleMutationV1 {
            identity: IdentityMutationV1::None,
            knowledge: Some(KnowledgeMutationV1::UpdateLocation {
                opaque: OpaqueObjectId(2),
                fact: KnownLocationFactV2 {
                    location: crate::zones::ZoneLocation {
                        zone: ZoneKind::Library,
                        player: Some(PlayerId(2)),
                        position: crate::zones::ZonePosition::Top { offset: 0 },
                        visibility: crate::zones::VisibilityPartition::FaceDown,
                        partition: None,
                    },
                    provenance: crate::knowledge::KnowledgeAcquisitionReason::InitialConfiguration,
                },
            }),
        },
    };
    assert_eq!(
        apply_perspective_lifecycle(&mut state, &audit).unwrap_err(),
        LifecycleApplicationError::UnsequencedProvenance
    );
}
