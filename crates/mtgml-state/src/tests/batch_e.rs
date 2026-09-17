// Ownership fragment: Batch-E FND-007 lifecycle staging characterization.
// Included lexically by tests.rs so existing state fixture helpers remain
// available without creating a second test fixture authority.

#[test]
fn fnd_007_lifecycle_seam_can_stage_before_physical_state_completion() {
    let mut state = synthetic_state();
    let before = state.clone();
    let audit = PerspectiveLifecycleAuditV1 {
        perspective: PlayerId(1),
        sequence: VisibleSequence(1),
        mutation: PerspectiveLifecycleMutationV1 {
            identity: IdentityMutationV1::None,
            knowledge: Some(KnowledgeMutationV1::UpdateLocation {
                opaque: OpaqueObjectId(1),
                fact: KnownLocationFactV2 {
                    location: ZoneLocation {
                        zone: ZoneKind::Hand,
                        player: Some(PlayerId(1)),
                        position: ZonePosition::Unordered,
                        visibility: VisibilityPartition::OwnerOnly,
                        partition: None,
                    },
                    provenance: KnowledgeAcquisitionReason::Observed {
                        channel: KnowledgeHistoryChannel::Public,
                        sequence: VisibleSequence(1),
                        cause: KnowledgeAcquisitionCause::PublicEvent,
                    },
                },
            }),
        },
    };

    assert_eq!(
        apply_perspective_lifecycle(&mut state, &audit),
        Ok(()),
        "the lower-level lifecycle seam is a staging primitive"
    );
    assert_ne!(state, before);
    assert_eq!(
        validate_engine_state(&state),
        Err(EngineStateViolation::KnowledgeMismatch)
    );
}
