use mtgml_model::{GameObjectId, OpaqueObjectId, PlayerId};
use mtgml_random::RootSeed256;
use mtgml_state::{
    apply_perspective_lifecycle, construct_synthetic_engine_state, EngineState, IdentityMutationV1,
    KnowledgeAcquisitionCause, KnowledgeAcquisitionReason, KnowledgeHistoryChannel,
    KnowledgeLocationUpdateV1, KnowledgeMutationV1, KnownLocationFactV2, LifecycleApplicationError,
    PerspectiveLifecycleAuditV1, PerspectiveLifecycleMutationV1, SyntheticResetInputs,
    SyntheticV4Setup,
};

const P1: PlayerId = PlayerId(1);

/// P1 knows the public object 1 and not the hidden object 2.
fn state() -> EngineState {
    construct_synthetic_engine_state(SyntheticResetInputs {
        players: [P1, PlayerId(2)],
        root_seed: RootSeed256::from_lower_hex(&"22".repeat(32)).unwrap(),
        setup: SyntheticV4Setup::synthetic_compatibility(),
    })
    .unwrap()
}

fn audit(
    state: &EngineState,
    updates: Vec<KnowledgeLocationUpdateV1>,
) -> PerspectiveLifecycleAuditV1 {
    let sequence = state.knowledge.players[&P1].next_visible_sequence;
    let provenance = KnowledgeAcquisitionReason::Observed {
        channel: KnowledgeHistoryChannel::Public,
        sequence,
        cause: KnowledgeAcquisitionCause::PublicEvent,
    };
    let acquired = state.perspective_identities.players[&P1].next_opaque_object_id;
    PerspectiveLifecycleAuditV1 {
        perspective: P1,
        sequence,
        mutation: PerspectiveLifecycleMutationV1 {
            identity: IdentityMutationV1::Allocate {
                opaque: acquired,
                object: GameObjectId(2),
            },
            knowledge: Some(KnowledgeMutationV1::AcquireShiftingKnownMembers {
                opaque: acquired,
                definition: Some(state.zones.objects[&GameObjectId(2)].card_definition),
                location: Some(state.zones.locations[&GameObjectId(2)].clone()),
                acquisition: provenance,
                updates: updates
                    .into_iter()
                    .map(|update| KnowledgeLocationUpdateV1 {
                        fact: KnownLocationFactV2 {
                            provenance,
                            ..update.fact
                        },
                        ..update
                    })
                    .collect(),
            }),
        },
    }
}

fn public_member_update(state: &EngineState, opaque: OpaqueObjectId) -> KnowledgeLocationUpdateV1 {
    KnowledgeLocationUpdateV1 {
        opaque,
        fact: KnownLocationFactV2 {
            location: state.zones.locations[&GameObjectId(1)].clone(),
            provenance: KnowledgeAcquisitionReason::InitialConfiguration,
        },
    }
}

#[test]
fn acquire_shifting_known_members_acquires_and_refreshes_in_one_occurrence() {
    let mut state = state();
    let known = state.perspective_identities.players[&P1].object_to_opaque[&GameObjectId(1)];
    let lifecycle = audit(&state, vec![public_member_update(&state, known)]);
    let acquired = state.perspective_identities.players[&P1].next_opaque_object_id;

    apply_perspective_lifecycle(&mut state, &lifecycle).unwrap();

    let knowledge = &state.knowledge.players[&P1];
    assert!(knowledge.active.contains_key(&acquired));
    assert_eq!(knowledge.active[&known].historical_locations.len(), 1);
    assert_eq!(
        state.perspective_identities.players[&P1].opaque_to_object[&acquired],
        GameObjectId(2)
    );
}

#[test]
fn acquire_shifting_known_members_rejects_empty_or_self_updates() {
    let state = state();
    let acquired = state.perspective_identities.players[&P1].next_opaque_object_id;

    let mut empty = state.clone();
    assert_eq!(
        apply_perspective_lifecycle(&mut empty, &audit(&state, Vec::new())),
        Err(LifecycleApplicationError::InvalidState)
    );
    assert_eq!(empty, state);

    let mut includes_acquired = state.clone();
    assert_eq!(
        apply_perspective_lifecycle(
            &mut includes_acquired,
            &audit(&state, vec![public_member_update(&state, acquired)])
        ),
        Err(LifecycleApplicationError::InvalidState)
    );
    assert_eq!(includes_acquired, state);
}
