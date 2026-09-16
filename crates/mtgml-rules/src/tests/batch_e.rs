// Ownership fragment: Batch-E FND-008 mutation-family and FND-012B
// characterization. Included lexically by tests.rs so existing transition
// product helpers remain the single test authority.

fn assert_fnd_008_unexplained_core_mutation(
    mutate: impl FnOnce(&mut EngineState),
) {
    let before = state_without_pending_decision();
    let mut after = before.clone();
    after.revision = StateRevision(before.revision.0 + 1);
    mutate(&mut after);
    let result = accepted_product_for_contract(&before, after, Vec::new());
    assert_contract_rejects_without_mutation(&before, &result);
}

#[test]
fn fnd_008_has_lost_mutation_requires_current_semantic_ownership() {
    assert_fnd_008_unexplained_core_mutation(|after| {
        after.core.players.get_mut(&PlayerId(1)).unwrap().has_lost = true;
    });
}

#[test]
fn fnd_008_active_player_mutation_requires_current_semantic_ownership() {
    assert_fnd_008_unexplained_core_mutation(|after| {
        after.core.active_player = PlayerId(2);
    });
}

#[test]
fn fnd_008_priority_mutation_requires_current_semantic_ownership() {
    assert_fnd_008_unexplained_core_mutation(|after| {
        after.core.priority = mtgml_state::PriorityState::HeldBy {
            player: PlayerId(2),
            consecutive_passes: 0,
        };
    });
}

#[test]
fn fnd_008_turn_number_mutation_requires_current_semantic_ownership() {
    assert_fnd_008_unexplained_core_mutation(|after| {
        after.core.turn_number += 1;
    });
}

#[test]
fn fnd_008_player_universe_mutation_requires_current_semantic_ownership() {
    let before = state_without_pending_decision();
    let mut after = before.clone();
    after.revision = StateRevision(before.revision.0 + 1);
    after.core.players.insert(
        PlayerId(3),
        mtgml_state::PlayerState {
            life: 40,
            has_lost: false,
        },
    );
    after
        .knowledge
        .players
        .insert(PlayerId(3), Default::default());
    after.perspective_identities.players.insert(
        PlayerId(3),
        mtgml_state::PerspectiveIdentityRecordV2 {
            next_opaque_object_id: mtgml_model::OpaqueObjectId(1),
            next_opaque_ability_id: mtgml_model::OpaqueAbilityId(1),
            next_player_decision_id: PlayerDecisionIdV1(1),
            ..Default::default()
        },
    );
    let result = accepted_product_for_contract(&before, after, Vec::new());
    assert_contract_rejects_without_mutation(&before, &result);
}

#[test]
fn fnd_012b_announced_outcome_is_its_separate_presentation_occurrence() {
    let (before, result) = outcome_occurrence_product(
        PerspectiveObservationPolicyV1::AnnouncedOutcome {
            code: "arbitrary-public-code".into(),
        },
    );
    validate_transition_contract(&before, &result).unwrap();
}

#[test]
fn fnd_012b_empty_announced_outcome_fails_closed() {
    let (before, result) = outcome_occurrence_product(
        PerspectiveObservationPolicyV1::AnnouncedOutcome {
            code: String::new(),
        },
    );
    assert_contract_rejects_without_mutation(&before, &result);
}
