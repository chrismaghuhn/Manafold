// Ownership fragment: continuation payload invariants, checked directly on the
// validators the execution records use. Included lexically by tests.rs so
// every identity remains tests::<name>.

#[test]
fn assembly_payload_stage_invariants_are_enforced() {
    use crate::engine_state_shape::validate_successor_synthetic_assembly as validate;
    use AssemblyStageV2::{ChooseCount, ChooseMembers, OrderMembers};

    // ChooseCount must carry no partial values.
    assert!(validate(ChooseCount, None, &[], &[]).is_ok());
    assert!(validate(ChooseCount, Some(2), &[], &[]).is_err());

    // ChooseMembers carries only the decided count.
    assert!(validate(ChooseMembers, Some(2), &[], &[]).is_ok());
    assert!(validate(ChooseMembers, None, &[], &[]).is_err());
    assert!(validate(ChooseMembers, Some(2), &[0, 1], &[]).is_err());

    // OrderMembers carries exactly the canonical member set and no order.
    assert!(validate(OrderMembers, Some(2), &[0, 1], &[]).is_ok());
    // Member-set size disagrees with the decided count.
    assert!(validate(OrderMembers, Some(3), &[0, 1], &[]).is_err());
    // Noncanonical set representation.
    assert!(validate(OrderMembers, Some(2), &[1, 0], &[]).is_err());
    // A persisted order is never a valid partial value.
    assert!(validate(OrderMembers, Some(2), &[0, 1], &[1, 0]).is_err());
}

/// The Magic SBA Graveyard-order stage for four dying objects (1 and 3 owned
/// by P1, 2 and 4 by P2), with P1 ordering first.
fn magic_sba_order_stage(
    current_revision: u64,
    created_at_revision: u64,
    round_start_revision: u64,
    next_owner_index: u32,
) -> Result<(), EngineStateShapeViolation> {
    use crate::engine_state_shape::{
        validate_successor_magic_sba_graveyard_order, SuccessorMagicSbaGraveyardOrderValidation,
    };
    use crate::{GameObject, SbaGraveyardOwnerOrderV1, SbaObjectCauseV1, SbaSelectedActionV1};

    let (p1, p2) = (PlayerId(1), PlayerId(2));
    let players = BTreeSet::from([p1, p2]);
    let objects: BTreeMap<GameObjectId, GameObject> = [(1, p1), (2, p2), (3, p1), (4, p2)]
        .into_iter()
        .map(|(id, owner)| {
            (
                GameObjectId(id),
                GameObject {
                    id: GameObjectId(id),
                    physical_card: Some(PhysicalCardId(id)),
                    card_definition: CardDefinitionId(id),
                    owner,
                    controller: owner,
                    tapped: false,
                    face_down: false,
                },
            )
        })
        .collect();
    let selected_sba_actions = [
        (1, SbaObjectCauseV1::LethalDamage),
        (2, SbaObjectCauseV1::LethalDamage),
        (3, SbaObjectCauseV1::ZeroToughness),
        (4, SbaObjectCauseV1::ZeroToughness),
    ]
    .into_iter()
    .map(|(id, cause)| SbaSelectedActionV1::ObjectToOwnerGraveyard {
        object: GameObjectId(id),
        causes: vec![cause],
    })
    .collect::<Vec<_>>();
    let apnap_owners = [p1, p2];
    let completed_owner_orders = if next_owner_index == 0 {
        Vec::new()
    } else {
        vec![SbaGraveyardOwnerOrderV1 {
            owner: p1,
            top_to_bottom: vec![GameObjectId(1), GameObjectId(3)],
        }]
    };
    validate_successor_magic_sba_graveyard_order(SuccessorMagicSbaGraveyardOrderValidation {
        round_start_revision: StateRevision(round_start_revision),
        continuation_created_at_revision: StateRevision(created_at_revision),
        selected_sba_actions: &selected_sba_actions,
        apnap_owners: &apnap_owners,
        next_owner_index,
        completed_owner_orders: &completed_owner_orders,
        current_revision: StateRevision(current_revision),
        players: &players,
        objects: &objects,
    })
}

#[test]
fn magic_sba_order_initial_and_second_apnap_stages_have_exact_valid_revisions() {
    assert_eq!(magic_sba_order_stage(1, 1, 0, 0), Ok(()));
    assert_eq!(magic_sba_order_stage(2, 1, 0, 1), Ok(()));
}

#[test]
fn magic_sba_created_revision_must_follow_round_start_revision() {
    assert_eq!(
        magic_sba_order_stage(1, 0, 0, 0),
        Err(EngineStateShapeViolation::ContinuationRevision)
    );
}

#[test]
fn magic_sba_current_revision_must_count_each_completed_owner_order() {
    assert_eq!(
        magic_sba_order_stage(1, 1, 0, 1),
        Err(EngineStateShapeViolation::ContinuationRevision)
    );
}

#[test]
fn magic_sba_stage_revision_relation_uses_checked_arithmetic() {
    assert_eq!(
        magic_sba_order_stage(u64::MAX, u64::MAX, u64::MAX, 0),
        Err(EngineStateShapeViolation::ContinuationRevision)
    );
}

#[test]
fn a_player_with_seven_mulligans_is_never_asked_to_declare() {
    use crate::engine::game_start_shape_is_valid as valid;
    let players = BTreeSet::from([PlayerId(1), PlayerId(2)]);
    let declaring = |taken: u32| crate::GameStartContinuation {
        chooser: PlayerId(1),
        starting_player: Some(PlayerId(1)),
        stage: crate::GameStartStage::Declaring {
            player: PlayerId(1),
        },
        mulligans_taken: BTreeMap::from([(PlayerId(1), taken), (PlayerId(2), 0)]),
        kept: BTreeSet::from([PlayerId(2)]),
        round_mulligans: BTreeSet::new(),
    };
    assert!(valid(&declaring(6), &players));
    // CR 103.5: with an opening hand of zero cards no further mulligan may
    // be taken, so there is nothing to declare.
    assert!(!valid(&declaring(7), &players));
}
