use mtgml_model::{CardDefinitionId, GameObjectId, PhysicalCardId, PlayerId, ZoneKind};
use mtgml_random::RootSeed256;
use mtgml_state::{
    construct_synthetic_engine_state, CardRulesAuthoritativeStateV1, DeltaApplicationV3Error,
    EngineStatePartsV3, ExecutionStateV4, SemanticDeltaOperation, SemanticDeltaOperationV2,
    SemanticDeltaOperationV3, StateDeltaV3, SyntheticResetInputs, SyntheticV4Setup,
    VisibilityPartition, ZoneLocation, ZonePosition,
};

fn state_with_tapped_permanents(objects: &[u64]) -> EngineStatePartsV3 {
    let mut predecessor = construct_synthetic_engine_state(SyntheticResetInputs {
        players: [PlayerId(1), PlayerId(2)],
        root_seed: RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap(),
        setup: SyntheticV4Setup::synthetic_compatibility(),
    })
    .unwrap();
    predecessor.execution = Default::default();
    let mut card_rules_state = CardRulesAuthoritativeStateV1::default();
    for player in predecessor.core.players.keys().copied() {
        card_rules_state
            .mana
            .pools
            .insert(player, Default::default());
        card_rules_state
            .turn_history
            .players
            .insert(player, Default::default());
    }
    card_rules_state.turn_history.turn_number = predecessor.core.turn_number;
    let mut state = EngineStatePartsV3::new(
        predecessor.parts(),
        ExecutionStateV4::default(),
        card_rules_state,
    )
    .unwrap();
    for raw in objects {
        let id = GameObjectId(*raw);
        state.predecessor_v5.zones.objects.insert(
            id,
            mtgml_state::GameObject {
                id,
                physical_card: Some(PhysicalCardId(*raw)),
                card_definition: CardDefinitionId(1),
                owner: PlayerId(1),
                controller: PlayerId(1),
                tapped: true,
                face_down: false,
            },
        );
        state.predecessor_v5.zones.locations.insert(
            id,
            ZoneLocation {
                zone: ZoneKind::Battlefield,
                player: None,
                position: ZonePosition::Unordered,
                visibility: VisibilityPartition::Public,
                partition: None,
            },
        );
    }
    for object in state.predecessor_v5.zones.objects.keys().copied() {
        state.card_rules_state.faces.faces.insert(object, 0);
    }
    let next = objects.iter().max().map_or(1, |max| max + 1);
    state.predecessor_v5.allocators.next_object_id = GameObjectId(next);
    state
}

fn untapped(before: &EngineStatePartsV3, objects: &[u64]) -> EngineStatePartsV3 {
    let mut after = before.clone();
    after.predecessor_v5.revision.0 += 1;
    for raw in objects {
        after
            .predecessor_v5
            .zones
            .objects
            .get_mut(&GameObjectId(*raw))
            .unwrap()
            .tapped = false;
    }
    after
}

fn untap_completed(objects: &[u64]) -> SemanticDeltaOperationV3 {
    SemanticDeltaOperationV3::Existing {
        operation: Box::new(SemanticDeltaOperationV2::Existing {
            operation: Box::new(SemanticDeltaOperation::UntapCompleted {
                affected_objects: objects.iter().copied().map(GameObjectId).collect(),
            }),
        }),
    }
}

#[test]
fn untap_completed_covers_exactly_its_affected_objects() {
    let before = state_with_tapped_permanents(&[10, 11]);
    let after = untapped(&before, &[10, 11]);

    let delta =
        StateDeltaV3::between_structural_only(&before, &after, vec![untap_completed(&[10, 11])])
            .expect("UntapCompleted covers the permanents it untapped");

    assert_eq!(delta.apply_structural_only(&before).unwrap(), after);
}

#[test]
fn untap_completed_does_not_cover_unlisted_objects() {
    let before = state_with_tapped_permanents(&[10, 11, 12]);
    let after = untapped(&before, &[10, 11, 12]);

    assert_eq!(
        StateDeltaV3::between_structural_only(&before, &after, vec![untap_completed(&[10, 11])])
            .unwrap_err(),
        DeltaApplicationV3Error::UncoveredMutation
    );
}

#[test]
fn untap_completed_does_not_cover_tapping() {
    let mut before = state_with_tapped_permanents(&[10]);
    before
        .predecessor_v5
        .zones
        .objects
        .get_mut(&GameObjectId(10))
        .unwrap()
        .tapped = false;
    let mut after = before.clone();
    after.predecessor_v5.revision.0 += 1;
    after
        .predecessor_v5
        .zones
        .objects
        .get_mut(&GameObjectId(10))
        .unwrap()
        .tapped = true;

    assert_eq!(
        StateDeltaV3::between_structural_only(&before, &after, vec![untap_completed(&[10])])
            .unwrap_err(),
        DeltaApplicationV3Error::UncoveredMutation
    );
}

#[test]
fn untap_completed_claims_only_objects_it_untapped() {
    // 10 untaps, 11 stays tapped, 12 does not exist: listing 11 or 12 claims
    // untaps that did not happen.
    let before = state_with_tapped_permanents(&[10, 11]);
    let after = untapped(&before, &[10]);

    for claimed in [&[10, 11][..], &[10, 12][..]] {
        assert_eq!(
            StateDeltaV3::between_structural_only(&before, &after, vec![untap_completed(claimed)])
                .unwrap_err(),
            DeltaApplicationV3Error::UncoveredMutation
        );
    }
}
