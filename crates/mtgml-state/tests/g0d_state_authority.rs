use mtgml_card_ir::{CardSemanticProfileId, FaceKey};
use mtgml_model::{
    AbilityInstanceId, CandidateIdV1, CardDefinitionId, ContinuationId, DecisionId, GameObjectId,
    PhysicalCardId, PlayerDecisionIdV1, PlayerId, StackObjectId, StateRevision, VisibleSequence,
    ZoneKind,
};
use mtgml_random::RootSeed256;
use mtgml_state::{
    construct_synthetic_engine_state, AbilitySourceContext, EngineState, ExecutionState, ManaCost,
    ManaPaymentStage, ManaPaymentStaging, ManaSourceActivation, ManaSourceActivationCost,
    NonManaActivationContinuation, NonManaActivationStage, ReservedNonManaCost,
    SelectedCostOperand, StackItemPayload, StackRecord, SyntheticResetInputs, SyntheticV4Setup,
    VisibilityPartition, ZoneLocation, ZonePosition,
};

fn root() -> EngineState {
    construct_synthetic_engine_state(SyntheticResetInputs {
        players: [PlayerId(1), PlayerId(2)],
        root_seed: RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap(),
        setup: SyntheticV4Setup::synthetic_compatibility(),
    })
    .unwrap()
}

fn add_stack_spell(state: &mut EngineState, object_id: u64, stack_id: u64) {
    let object = GameObjectId(object_id);
    let card_definition = CardDefinitionId(object_id);
    state.zones.objects.insert(
        object,
        mtgml_state::GameObject {
            id: object,
            physical_card: Some(PhysicalCardId(object_id)),
            card_definition,
            owner: PlayerId(1),
            controller: PlayerId(1),
            tapped: false,
            face_down: false,
        },
    );
    state.zones.locations.insert(
        object,
        ZoneLocation {
            zone: ZoneKind::Stack,
            player: None,
            position: ZonePosition::Unordered,
            visibility: VisibilityPartition::Public,
            partition: None,
        },
    );
    state.allocators.next_object_id = GameObjectId(object_id + 1);
    state.allocators.next_stack_object_id = StackObjectId(stack_id + 1);
    state.zones.stack_records.insert(
        StackObjectId(stack_id),
        StackRecord {
            id: StackObjectId(stack_id),
            controller: PlayerId(1),
            payload: Some(StackItemPayload::Spell {
                stack_card_object: object,
                card_definition_id: card_definition,
                face_key: FaceKey(0),
                semantic_profile_id: CardSemanticProfileId::parse("test/spell@1.0.0").unwrap(),
                modes: vec![],
                targets: vec![],
                cost_facts: Default::default(),
            }),
        },
    );
    state.zones.stack_order.push(StackObjectId(stack_id));
}

fn pending_trigger(id: u64, controller: PlayerId) -> mtgml_state::PendingTriggerRecord {
    mtgml_state::PendingTriggerRecord {
        id: mtgml_model::TriggerInstanceId(id),
        controller,
        source_context: AbilitySourceContext {
            source: mtgml_state::SourceContext {
                snapshot: mtgml_state::ObjectSnapshot {
                    object: GameObjectId(3),
                    physical_card: Some(PhysicalCardId(3)),
                    card_definition: CardDefinitionId(3),
                    owner: PlayerId(1),
                    controller: PlayerId(1),
                    tapped: false,
                    face_down: false,
                    location: ZoneLocation {
                        zone: ZoneKind::Graveyard,
                        player: Some(PlayerId(1)),
                        position: ZonePosition::Unordered,
                        visibility: VisibilityPartition::Public,
                        partition: None,
                    },
                },
                face_key: FaceKey(0),
                semantic_profile_id: CardSemanticProfileId::parse("test/trigger@1.0.0").unwrap(),
            },
            ability_instance_id: AbilityInstanceId(1),
            ability_key: mtgml_card_ir::AbilityKey(4),
        },
        trigger_context: mtgml_state::TriggerEventSnapshot::CardDrawn { player: controller },
        target_timing: mtgml_state::TriggerTargetTiming::NoTargets,
    }
}

fn staged_blight_activation() -> (EngineState, ContinuationId) {
    let mut state = root();
    let source_object = GameObjectId(3);
    state.zones.objects.insert(
        source_object,
        mtgml_state::GameObject {
            id: source_object,
            physical_card: Some(PhysicalCardId(3)),
            card_definition: CardDefinitionId(3),
            owner: PlayerId(1),
            controller: PlayerId(1),
            tapped: false,
            face_down: false,
        },
    );
    state.zones.locations.insert(
        source_object,
        ZoneLocation {
            zone: ZoneKind::Battlefield,
            player: None,
            position: ZonePosition::Unordered,
            visibility: VisibilityPartition::Public,
            partition: None,
        },
    );
    state.allocators.next_object_id = GameObjectId(4);
    state.allocators.next_ability_id = AbilityInstanceId(3);
    state.allocators.next_continuation_id = ContinuationId(2);
    state.allocators.next_decision_id = DecisionId(3);
    state.card_rules.abilities.by_instance.insert(
        AbilityInstanceId(1),
        mtgml_state::AbilityAuthorityV1 {
            source: source_object,
            ability_key: 4,
        },
    );
    state.card_rules.abilities.by_instance.insert(
        AbilityInstanceId(2),
        mtgml_state::AbilityAuthorityV1 {
            source: GameObjectId(1),
            ability_key: 9,
        },
    );
    for object in state.zones.objects.keys().copied() {
        state.card_rules.faces.faces.insert(object, 0);
        if state.zones.locations[&object].zone == ZoneKind::Battlefield {
            state.card_rules.permanents.enter(object, 1).unwrap();
        }
    }
    let continuation_id = ContinuationId(1);
    let action_cost_facts = mtgml_state::ActionCostFacts {
        mana_cost: Some(ManaCost {
            colored_wubrg_counts: [0; 5],
            colorless_count: 0,
            generic_count: 1,
        }),
        reserved_nonmana_costs: vec![ReservedNonManaCost::TapSource],
        selected_cost_operands: vec![SelectedCostOperand::PutCounters {
            object: GameObjectId(1),
            counter_kind: mtgml_state::CounterKindV1::MinusOneMinusOne,
            count: 2,
        }],
    };
    state.execution.continuations.insert(
        continuation_id,
        mtgml_state::ContinuationRecord {
            id: continuation_id,
            created_at_revision: state.revision,
            payload: mtgml_state::ContinuationPayload::NonManaActivation(
                NonManaActivationContinuation {
                    actor: PlayerId(1),
                    source_object,
                    source_ability_instance: AbilityInstanceId(1),
                    ability_key: mtgml_card_ir::AbilityKey(4),
                    semantic_profile_id: CardSemanticProfileId::parse("test/blight@1.0.0").unwrap(),
                    stage: NonManaActivationStage::PayingMana,
                    modes: vec![],
                    targets: vec![],
                    action_cost_facts,
                    mana_payment_staging: Some(ManaPaymentStaging {
                        stage: ManaPaymentStage::SelectingSources,
                        mana_source_activations: vec![ManaSourceActivation {
                            source_object: GameObjectId(1),
                            source_ability_instance: AbilityInstanceId(2),
                            ability_key: mtgml_card_ir::AbilityKey(9),
                            semantic_profile_id: CardSemanticProfileId::parse(
                                "test/mana-source@1.0.0",
                            )
                            .unwrap(),
                            activation_cost_receipt: ManaSourceActivationCost::TapSource,
                            produced_buckets: [0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0],
                        }],
                    }),
                },
            ),
        },
    );
    let view_sequence = state.knowledge.players[&PlayerId(1)].next_visible_sequence;
    state.execution.pending_decision = Some(mtgml_decision::AuthoritativeDecisionRequest {
        decision_id: DecisionId(2),
        player_decision_id: PlayerDecisionIdV1(1),
        state_revision: StateRevision(state.revision.0),
        view_sequence,
        actor: PlayerId(1),
        visibility: mtgml_decision::DecisionVisibility::ActingPlayerOnly,
        decision_domain_v2: mtgml_decision::DecisionDomainV2::ChooseOne,
        purpose: mtgml_decision::DecisionPurposeV4::ManaProductionChoice,
        parent_player_decision_id: None,
        continuation_id: Some(continuation_id),
        candidates: vec![mtgml_decision::AuthoritativeCandidate {
            candidate_id: CandidateIdV1(0),
            visible_intent: mtgml_decision::CandidateIntent::FinalizeManaProduction,
            trusted_binding: mtgml_decision::EngineCandidateBinding::FinalizeManaProduction {
                continuation: continuation_id,
            },
        }],
    });
    (state, continuation_id)
}

#[test]
fn successor_authority_has_one_v3_root_and_v4_execution_owner() {
    let state = root();
    state.validate().unwrap();
    assert_eq!(state.execution, ExecutionState::default());
}

/// Issues one pending pass-priority request to player 1 and returns its
/// trusted decision identity.
fn issue_pass_priority_request(state: &mut EngineState) -> DecisionId {
    let actor = PlayerId(1);
    let decision_id = state.allocators.next_decision_id;
    state.allocators.next_decision_id = DecisionId(decision_id.0.checked_add(1).unwrap());
    let identity = state
        .perspective_identities
        .players
        .get_mut(&actor)
        .unwrap();
    let player_decision_id = identity.next_player_decision_id;
    identity.next_player_decision_id =
        PlayerDecisionIdV1(player_decision_id.0.checked_add(1).unwrap());
    let view_sequence = state.knowledge.players[&actor].next_visible_sequence;
    state.execution.pending_decision = Some(mtgml_decision::AuthoritativeDecisionRequest {
        decision_id,
        player_decision_id,
        state_revision: state.revision,
        view_sequence,
        actor,
        visibility: mtgml_decision::DecisionVisibility::Public,
        decision_domain_v2: mtgml_decision::DecisionDomainV2::ChooseOne,
        purpose: mtgml_decision::DecisionPurposeV4::PriorityAction,
        parent_player_decision_id: None,
        continuation_id: None,
        candidates: vec![mtgml_decision::AuthoritativeCandidate {
            candidate_id: CandidateIdV1(0),
            visible_intent: mtgml_decision::CandidateIntent::PassPriority,
            trusted_binding: mtgml_decision::EngineCandidateBinding::PassPriority,
        }],
    });
    decision_id
}

#[test]
fn m42_priority_candidate_is_structural_but_not_rules_admitted() {
    let mut state = root();
    issue_pass_priority_request(&mut state);
    assert!(state.validate_structure().is_ok());
    assert_eq!(
        state.validate(),
        Err(mtgml_state::EngineStateError::ProfileDependentDecisionNotAdmitted)
    );
}

#[test]
fn pending_request_decision_identity_must_stay_below_the_allocator() {
    let build = |offset: i64| {
        let mut state = root();
        let issued = issue_pass_priority_request(&mut state);
        state.allocators.next_decision_id =
            DecisionId(issued.0.checked_add_signed(offset).unwrap());
        state
    };
    // next == issued must fail closed.
    assert_eq!(
        build(0).validate_structure(),
        Err(mtgml_state::EngineStateError::PendingDecision)
    );
    // next < issued must fail closed.
    assert_eq!(
        build(-1).validate_structure(),
        Err(mtgml_state::EngineStateError::PendingDecision)
    );
    // A strictly greater cursor is accepted.
    build(1).validate_structure().unwrap();
}

#[test]
fn successor_stack_records_require_one_typed_payload_owner() {
    let mut state = root();
    state.zones.stack_records.insert(
        StackObjectId(1),
        StackRecord {
            id: StackObjectId(1),
            controller: PlayerId(1),
            payload: None,
        },
    );
    state.zones.stack_order.push(StackObjectId(1));
    state.allocators.next_stack_object_id = StackObjectId(2);
    assert_eq!(
        state.validate(),
        Err(mtgml_state::EngineStateError::MissingStackPayload)
    );
}

#[test]
fn stack_zone_card_requires_exactly_one_spell_payload() {
    let mut state = root();
    let card = GameObjectId(3);
    state.zones.objects.insert(
        card,
        mtgml_state::GameObject {
            id: card,
            physical_card: Some(PhysicalCardId(3)),
            card_definition: CardDefinitionId(3),
            owner: PlayerId(1),
            controller: PlayerId(1),
            tapped: false,
            face_down: false,
        },
    );
    state.zones.locations.insert(
        card,
        ZoneLocation {
            zone: ZoneKind::Stack,
            player: None,
            position: ZonePosition::Unordered,
            visibility: VisibilityPartition::Public,
            partition: None,
        },
    );
    state.allocators.next_object_id = GameObjectId(4);
    assert_eq!(
        state.validate(),
        Err(mtgml_state::EngineStateError::StackCardReference)
    );
}

#[test]
fn paused_stack_resolution_must_reference_the_current_top_item() {
    let mut state = root();
    add_stack_spell(&mut state, 3, 1);
    add_stack_spell(&mut state, 4, 2);
    let id = ContinuationId(1);
    state.allocators.next_continuation_id = ContinuationId(2);
    state.execution.continuations.insert(
        id,
        mtgml_state::ContinuationRecord {
            id,
            created_at_revision: state.revision,
            payload: mtgml_state::ContinuationPayload::StackResolution(
                mtgml_state::StackResolutionContinuation {
                    resolving_stack_object: StackObjectId(1),
                    stage: mtgml_state::StackResolutionStage::AwaitingOptionalPayment,
                    action_cost_facts: None,
                    mana_payment_staging: None,
                },
            ),
        },
    );
    assert_eq!(
        state.validate(),
        Err(mtgml_state::EngineStateError::StackResolution)
    );
}

#[test]
fn stack_resolution_continuation_must_keep_the_resolving_item_in_the_stack_owner() {
    let mut state = root();
    let id = mtgml_model::ContinuationId(1);
    state.allocators.next_continuation_id = mtgml_model::ContinuationId(2);
    state.execution.continuations.insert(
        id,
        mtgml_state::ContinuationRecord {
            id,
            created_at_revision: state.revision,
            payload: mtgml_state::ContinuationPayload::StackResolution(
                mtgml_state::StackResolutionContinuation {
                    resolving_stack_object: StackObjectId(1),
                    stage: mtgml_state::StackResolutionStage::AwaitingOptionalPayment,
                    action_cost_facts: None,
                    mana_payment_staging: None,
                },
            ),
        },
    );
    assert_eq!(
        state.validate(),
        Err(mtgml_state::EngineStateError::StackResolution)
    );
}

#[test]
fn v3_continuation_preserves_frozen_synthetic_and_sba_stage_invariants() {
    let mut synthetic = root();
    let synthetic_id = ContinuationId(1);
    synthetic.allocators.next_continuation_id = ContinuationId(2);
    synthetic.execution.continuations.insert(
        synthetic_id,
        mtgml_state::ContinuationRecord {
            id: synthetic_id,
            created_at_revision: synthetic.revision,
            payload: mtgml_state::ContinuationPayload::SyntheticAssembly {
                actor: PlayerId(1),
                stage: mtgml_state::AssemblyStageV2::ChooseCount,
                selected_count: Some(u32::MAX),
                selected_piece_keys: vec![],
                ordered_piece_keys: vec![],
            },
        },
    );
    assert_eq!(
        synthetic.validate(),
        Err(mtgml_state::EngineStateError::ContinuationRecord)
    );

    let mut sba = root();
    let sba_id = ContinuationId(1);
    sba.allocators.next_continuation_id = ContinuationId(2);
    sba.execution.continuations.insert(
        sba_id,
        mtgml_state::ContinuationRecord {
            id: sba_id,
            created_at_revision: sba.revision,
            payload: mtgml_state::ContinuationPayload::MagicSbaGraveyardOrderV1 {
                round_start_revision: sba.revision,
                selected_sba_actions: vec![],
                apnap_owners: vec![PlayerId(1)],
                next_owner_index: 0,
                completed_owner_orders: vec![],
            },
        },
    );
    assert_eq!(
        sba.validate(),
        Err(mtgml_state::EngineStateError::ContinuationRecord)
    );
}

#[test]
fn temporary_effect_expiry_is_bound_to_the_authoritative_turn() {
    let mut state = root();
    let id = mtgml_model::EffectInstanceId(1);
    state.allocators.next_effect_id = mtgml_model::EffectInstanceId(2);
    let effect = mtgml_state::TemporaryEffectRecord {
        id,
        affected_objects: vec![GameObjectId(1)],
        operation: mtgml_state::TemporaryOperation::GrantKeyword {
            keyword: mtgml_state::TemporaryKeyword::DoubleStrike,
        },
        expiry: mtgml_state::EffectExpiry::UntilEndOfTurn {
            turn_number: state.core.turn_number,
        },
        timestamp: Some(mtgml_state::EffectTimestamp {
            creation_revision: state.revision,
            operation_ordinal: 0,
        }),
    };
    state.execution.effects.insert(id, effect.clone());
    state.validate().unwrap();

    let mut expired_later = root();
    expired_later.allocators.next_effect_id = mtgml_model::EffectInstanceId(2);
    let mut invalid = effect;
    invalid.expiry = mtgml_state::EffectExpiry::UntilEndOfTurn {
        turn_number: expired_later.core.turn_number + 1,
    };
    expired_later.execution.effects.insert(id, invalid);
    assert_eq!(
        expired_later.validate(),
        Err(mtgml_state::EngineStateError::TemporaryEffectExpiry)
    );
}

#[test]
fn selected_blight_operand_request_is_not_admitted_until_rules_domain_generation_exists() {
    let (state, _) = staged_blight_activation();
    assert_eq!(
        state.validate(),
        Err(mtgml_state::EngineStateError::ProfileDependentDecisionNotAdmitted)
    );
}

#[test]
fn blight_operand_must_be_a_controlled_battlefield_incarnation() {
    let (mut state, continuation_id) = staged_blight_activation();
    let operand = GameObjectId(4);
    state.zones.objects.insert(
        operand,
        mtgml_state::GameObject {
            id: operand,
            physical_card: Some(PhysicalCardId(4)),
            card_definition: CardDefinitionId(4),
            owner: PlayerId(1),
            controller: PlayerId(1),
            tapped: false,
            face_down: false,
        },
    );
    state.zones.locations.insert(
        operand,
        ZoneLocation {
            zone: ZoneKind::Graveyard,
            player: Some(PlayerId(1)),
            position: ZonePosition::Unordered,
            visibility: VisibilityPartition::Public,
            partition: None,
        },
    );
    state.allocators.next_object_id = GameObjectId(5);
    state.card_rules.faces.faces.insert(operand, 0);
    let record = state
        .execution
        .continuations
        .get_mut(&continuation_id)
        .unwrap();
    let mtgml_state::ContinuationPayload::NonManaActivation(activation) = &mut record.payload
    else {
        unreachable!()
    };
    activation.action_cost_facts.selected_cost_operands[0] = SelectedCostOperand::PutCounters {
        object: operand,
        counter_kind: mtgml_state::CounterKindV1::MinusOneMinusOne,
        count: 2,
    };
    assert_eq!(
        state.validate(),
        Err(mtgml_state::EngineStateError::SelectedCostOperand)
    );
}

#[test]
fn tap_reserved_activation_source_is_not_a_mana_source() {
    let (mut state, continuation_id) = staged_blight_activation();
    let record = state
        .execution
        .continuations
        .get_mut(&continuation_id)
        .unwrap();
    let mtgml_state::ContinuationPayload::NonManaActivation(activation) = &mut record.payload
    else {
        unreachable!()
    };
    let source = &mut activation
        .mana_payment_staging
        .as_mut()
        .unwrap()
        .mana_source_activations[0];
    source.source_object = GameObjectId(3);
    source.source_ability_instance = AbilityInstanceId(1);
    source.ability_key = mtgml_card_ir::AbilityKey(4);
    assert_eq!(
        state.validate(),
        Err(mtgml_state::EngineStateError::ManaPaymentStaging)
    );
}

#[test]
fn provisional_source_output_overflow_fails_closed() {
    let (mut state, _) = staged_blight_activation();
    state
        .card_rules
        .mana
        .pools
        .get_mut(&PlayerId(1))
        .unwrap()
        .unrestricted[3] = u32::MAX;
    assert_eq!(
        state.validate(),
        Err(mtgml_state::EngineStateError::ProvisionalManaOverflow)
    );
}

#[test]
fn pending_v4_request_must_match_its_perspective_local_visible_cursor() {
    let (mut state, _) = staged_blight_activation();
    let stale = VisibleSequence(
        state.knowledge.players[&PlayerId(1)]
            .next_visible_sequence
            .0
            + 1,
    );
    state
        .execution
        .pending_decision
        .as_mut()
        .unwrap()
        .view_sequence = stale;
    assert_eq!(
        state.validate(),
        Err(mtgml_state::EngineStateError::PendingDecision)
    );
}

#[test]
fn profile_dependent_pending_request_is_fail_closed_at_state_admission() {
    let (state, _) = staged_blight_activation();
    assert_eq!(
        state.validate(),
        Err(mtgml_state::EngineStateError::ProfileDependentDecisionNotAdmitted)
    );
}

#[test]
fn pending_request_purpose_must_match_mana_staging_stage() {
    let (mut state, _) = staged_blight_activation();
    let request = state.execution.pending_decision.as_mut().unwrap();
    request.purpose = mtgml_decision::DecisionPurposeV4::ManaPayment;
    request.candidates = vec![mtgml_decision::AuthoritativeCandidate {
        candidate_id: CandidateIdV1(0),
        visible_intent: mtgml_decision::CandidateIntent::SelectManaPayment {
            spent_buckets: [0; 12],
        },
        trusted_binding: mtgml_decision::EngineCandidateBinding::SelectManaPayment {
            spent_buckets: [0; 12],
        },
    }];
    assert_eq!(
        state.validate(),
        Err(mtgml_state::EngineStateError::ContinuationRequestMismatch)
    );
}

#[test]
fn pending_finalize_binding_must_name_its_exact_continuation() {
    let (mut state, _) = staged_blight_activation();
    state
        .execution
        .pending_decision
        .as_mut()
        .unwrap()
        .candidates[0]
        .trusted_binding = mtgml_decision::EngineCandidateBinding::FinalizeManaProduction {
        continuation: ContinuationId(99),
    };
    assert_eq!(
        state.validate(),
        Err(mtgml_state::EngineStateError::PendingCandidateBinding)
    );
}

#[test]
fn mana_source_candidate_rejects_the_tap_reserved_activation_source() {
    let (mut state, _) = staged_blight_activation();
    let location = state.zones.locations[&GameObjectId(3)].clone();
    let opaque_object = mtgml_model::OpaqueObjectId(2);
    let opaque_ability = mtgml_model::OpaqueAbilityId(1);
    let identity = state
        .perspective_identities
        .players
        .get_mut(&PlayerId(1))
        .unwrap();
    identity
        .opaque_to_object
        .insert(opaque_object, GameObjectId(3));
    identity
        .object_to_opaque
        .insert(GameObjectId(3), opaque_object);
    identity.next_opaque_object_id = mtgml_model::OpaqueObjectId(3);
    identity
        .opaque_to_ability
        .insert(opaque_ability, AbilityInstanceId(1));
    identity
        .ability_to_opaque
        .insert(AbilityInstanceId(1), opaque_ability);
    identity.next_opaque_ability_id = mtgml_model::OpaqueAbilityId(2);
    state
        .knowledge
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .active
        .insert(
            opaque_object,
            mtgml_state::KnowledgeRecordV2 {
                opaque_object,
                physical_card: Some(PhysicalCardId(3)),
                card_definition: Some(CardDefinitionId(3)),
                known_location: Some(mtgml_state::KnownLocationFactV2 {
                    location,
                    provenance: mtgml_state::KnowledgeAcquisitionReason::InitialConfiguration,
                }),
                acquisition: mtgml_state::KnowledgeAcquisitionReason::InitialConfiguration,
                historical_locations: vec![],
            },
        );
    let output = [0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0];
    let request = state.execution.pending_decision.as_mut().unwrap();
    request.candidates = vec![mtgml_decision::AuthoritativeCandidate {
        candidate_id: CandidateIdV1(0),
        visible_intent: mtgml_decision::CandidateIntent::SelectManaSource {
            source: opaque_object,
            ability: opaque_ability,
            produced_buckets: output,
        },
        trusted_binding: mtgml_decision::EngineCandidateBinding::SelectManaSource {
            source: GameObjectId(3),
            ability: AbilityInstanceId(1),
            ability_key: mtgml_card_ir::AbilityKey(4),
            semantic_profile_id: CardSemanticProfileId::parse("test/blight@1.0.0").unwrap(),
            activation_cost: mtgml_decision::ManaSourceActivationCostV1::TapSource,
            produced_buckets: output,
        },
    }];
    assert_eq!(
        state.validate(),
        Err(mtgml_state::EngineStateError::PendingCandidateBinding)
    );
}

#[test]
fn profile_dependent_v4_response_is_not_selected_without_rules_domain_generation() {
    let (state, _) = staged_blight_activation();
    let before = state.clone();
    let request = state.execution.pending_decision.as_ref().unwrap();
    let response = mtgml_decision::DecisionResponseV3 {
        schema_version: mtgml_decision::DECISION_RESPONSE_V3_SCHEMA.to_owned(),
        player_decision_id: request.player_decision_id,
        view_sequence: request.view_sequence,
        answer: mtgml_decision::DecisionAnswerV2::SelectOne {
            candidate_id: CandidateIdV1(0),
        },
    };
    assert_eq!(
        state.selected_bindings(PlayerId(1), &response),
        Err(mtgml_state::EngineStateError::ProfileDependentDecisionNotAdmitted)
    );
    assert_eq!(state, before);
}

#[test]
fn v4_parent_decision_must_precede_its_child_identity() {
    let (mut state, _) = staged_blight_activation();
    state
        .execution
        .pending_decision
        .as_mut()
        .unwrap()
        .parent_player_decision_id = Some(PlayerDecisionIdV1(2));
    assert_eq!(
        state.validate(),
        Err(mtgml_state::EngineStateError::PendingCandidateBinding)
    );
}

#[test]
fn trigger_placement_progress_must_follow_apnap_and_completed_prefix() {
    let make_state = |apnap_actors: Vec<PlayerId>, current_actor_index| {
        let mut state = root();
        state.allocators.next_object_id = GameObjectId(4);
        state.allocators.next_ability_id = AbilityInstanceId(2);
        state.allocators.next_trigger_id = mtgml_model::TriggerInstanceId(3);
        let id = ContinuationId(1);
        state.allocators.next_continuation_id = ContinuationId(2);
        state.execution.waiting_triggers.insert(
            mtgml_model::TriggerInstanceId(1),
            pending_trigger(1, PlayerId(1)),
        );
        state.execution.waiting_triggers.insert(
            mtgml_model::TriggerInstanceId(2),
            pending_trigger(2, PlayerId(2)),
        );
        state.execution.continuations.insert(
            id,
            mtgml_state::ContinuationRecord {
                id,
                created_at_revision: state.revision,
                payload: mtgml_state::ContinuationPayload::TriggerPlacement(
                    mtgml_state::TriggerPlacementContinuation {
                        apnap_actors,
                        current_actor_index,
                        pending_trigger_ids: vec![
                            mtgml_model::TriggerInstanceId(1),
                            mtgml_model::TriggerInstanceId(2),
                        ],
                        completed_orders: vec![],
                        selected_trigger_targets: vec![],
                        actor_request_roots: vec![],
                    },
                ),
            },
        );
        state
    };

    let apnap = make_state(vec![PlayerId(2), PlayerId(1)], 0);
    assert_eq!(
        apnap.validate(),
        Err(mtgml_state::EngineStateError::TriggerPlacement)
    );

    let skipped_owner = make_state(vec![PlayerId(1), PlayerId(2)], 1);
    assert_eq!(
        skipped_owner.validate(),
        Err(mtgml_state::EngineStateError::TriggerPlacement)
    );
}

#[test]
fn trigger_order_request_is_bound_to_the_current_apnap_group_not_trigger_ids() {
    let mut state = root();
    state.allocators.next_object_id = GameObjectId(4);
    state.allocators.next_ability_id = AbilityInstanceId(2);
    state.allocators.next_trigger_id = mtgml_model::TriggerInstanceId(3);
    state.allocators.next_continuation_id = ContinuationId(2);
    state.allocators.next_decision_id = DecisionId(3);
    let trigger_source = pending_trigger(1, PlayerId(1)).source_context;
    for (id, drawn_player) in [(1, PlayerId(1)), (2, PlayerId(2))] {
        state.execution.waiting_triggers.insert(
            mtgml_model::TriggerInstanceId(id),
            mtgml_state::PendingTriggerRecord {
                id: mtgml_model::TriggerInstanceId(id),
                controller: PlayerId(1),
                source_context: trigger_source.clone(),
                trigger_context: mtgml_state::TriggerEventSnapshot::CardDrawn {
                    player: drawn_player,
                },
                target_timing: mtgml_state::TriggerTargetTiming::NoTargets,
            },
        );
    }
    let continuation_id = ContinuationId(1);
    state.execution.continuations.insert(
        continuation_id,
        mtgml_state::ContinuationRecord {
            id: continuation_id,
            created_at_revision: state.revision,
            payload: mtgml_state::ContinuationPayload::TriggerPlacement(
                mtgml_state::TriggerPlacementContinuation {
                    apnap_actors: vec![PlayerId(1)],
                    current_actor_index: 0,
                    pending_trigger_ids: vec![
                        mtgml_model::TriggerInstanceId(2),
                        mtgml_model::TriggerInstanceId(1),
                    ],
                    completed_orders: vec![],
                    selected_trigger_targets: vec![],
                    actor_request_roots: vec![mtgml_state::TriggerActorRequestRoot {
                        actor: PlayerId(1),
                        first_decision_id: PlayerDecisionIdV1(1),
                    }],
                },
            ),
        },
    );
    let card_drawn = |player| mtgml_decision::SafeTriggerDescriptorV1 {
        source_object: None,
        source_ability: None,
        event_kind: mtgml_decision::TriggerEventKindV1::CardDrawn,
        subject: mtgml_decision::SafeTriggerSubjectV1::CardDrawn { player },
    };
    state.execution.pending_decision = Some(mtgml_decision::AuthoritativeDecisionRequest {
        decision_id: DecisionId(2),
        player_decision_id: PlayerDecisionIdV1(1),
        state_revision: StateRevision(state.revision.0),
        view_sequence: state.knowledge.players[&PlayerId(1)].next_visible_sequence,
        actor: PlayerId(1),
        visibility: mtgml_decision::DecisionVisibility::ActingPlayerOnly,
        decision_domain_v2: mtgml_decision::DecisionDomainV2::Order {
            minimum: 2,
            maximum: 2,
        },
        purpose: mtgml_decision::DecisionPurposeV4::TriggerOrder,
        parent_player_decision_id: None,
        continuation_id: Some(continuation_id),
        candidates: vec![
            mtgml_decision::AuthoritativeCandidate {
                candidate_id: CandidateIdV1(0),
                visible_intent: mtgml_decision::CandidateIntent::SelectTrigger {
                    trigger: card_drawn(PlayerId(1)),
                },
                trusted_binding: mtgml_decision::EngineCandidateBinding::SelectTrigger {
                    trigger: mtgml_model::TriggerInstanceId(1),
                },
            },
            mtgml_decision::AuthoritativeCandidate {
                candidate_id: CandidateIdV1(1),
                visible_intent: mtgml_decision::CandidateIntent::SelectTrigger {
                    trigger: card_drawn(PlayerId(2)),
                },
                trusted_binding: mtgml_decision::EngineCandidateBinding::SelectTrigger {
                    trigger: mtgml_model::TriggerInstanceId(2),
                },
            },
        ],
    });
    state.validate().unwrap();

    let mut rebound = state.clone();
    rebound
        .execution
        .pending_decision
        .as_mut()
        .unwrap()
        .candidates[0]
        .trusted_binding = mtgml_decision::EngineCandidateBinding::SelectTrigger {
        trigger: mtgml_model::TriggerInstanceId(2),
    };
    assert_eq!(
        rebound.validate(),
        Err(mtgml_state::EngineStateError::PendingCandidateBinding)
    );
}

#[test]
fn typed_spell_stack_payload_matches_the_live_stack_card_incarnation() {
    let mut state = root();
    let object = GameObjectId(3);
    let game_object = mtgml_state::GameObject {
        id: object,
        physical_card: Some(PhysicalCardId(3)),
        card_definition: CardDefinitionId(1),
        owner: PlayerId(1),
        controller: PlayerId(1),
        tapped: false,
        face_down: false,
    };
    let stack_location = ZoneLocation {
        zone: ZoneKind::Stack,
        player: None,
        position: ZonePosition::Unordered,
        visibility: VisibilityPartition::Public,
        partition: None,
    };
    state.zones.objects.insert(object, game_object.clone());
    state.zones.locations.insert(object, stack_location);
    state.allocators.next_object_id = GameObjectId(4);
    let payload = StackItemPayload::Spell {
        stack_card_object: object,
        card_definition_id: game_object.card_definition,
        face_key: FaceKey(0),
        semantic_profile_id: CardSemanticProfileId::parse("basic-land@1.0.0").unwrap(),
        modes: vec![],
        targets: vec![],
        cost_facts: Default::default(),
    };
    state.zones.stack_records.insert(
        StackObjectId(1),
        StackRecord {
            id: StackObjectId(1),
            controller: PlayerId(1),
            payload: Some(payload),
        },
    );
    state.zones.stack_order.push(StackObjectId(1));
    state.allocators.next_stack_object_id = StackObjectId(2);
    assert!(mtgml_state::validate_engine_state(&state.clone()).is_ok());
    state.validate().unwrap();
    let mut zero_target = state.clone();
    let Some(StackItemPayload::Spell { targets, .. }) = zero_target
        .zones
        .stack_records
        .get_mut(&StackObjectId(1))
        .and_then(|record| record.payload.as_mut())
    else {
        unreachable!()
    };
    targets.push(mtgml_state::TargetBinding {
        target_slot: 0,
        target: mtgml_state::TargetRef::Object(GameObjectId(0)),
    });
    assert_eq!(
        zero_target.validate(),
        Err(mtgml_state::EngineStateError::TargetReference)
    );
}

#[test]
fn activated_stack_payload_keeps_lki_after_its_source_has_left() {
    let mut state = root();
    state.allocators.next_object_id = GameObjectId(4);
    state.allocators.next_ability_id = AbilityInstanceId(2);
    state.allocators.next_stack_object_id = StackObjectId(2);
    let source_context = AbilitySourceContext {
        source: mtgml_state::SourceContext {
            snapshot: mtgml_state::ObjectSnapshot {
                object: GameObjectId(3),
                physical_card: Some(PhysicalCardId(3)),
                card_definition: CardDefinitionId(3),
                owner: PlayerId(1),
                controller: PlayerId(1),
                tapped: true,
                face_down: false,
                location: ZoneLocation {
                    zone: ZoneKind::Graveyard,
                    player: Some(PlayerId(1)),
                    position: ZonePosition::Unordered,
                    visibility: VisibilityPartition::Public,
                    partition: None,
                },
            },
            face_key: FaceKey(0),
            semantic_profile_id: CardSemanticProfileId::parse("test/leaves-play@1.0.0").unwrap(),
        },
        ability_instance_id: AbilityInstanceId(1),
        ability_key: mtgml_card_ir::AbilityKey(4),
    };
    state.zones.stack_records.insert(
        StackObjectId(1),
        StackRecord {
            id: StackObjectId(1),
            controller: PlayerId(1),
            payload: Some(StackItemPayload::ActivatedAbility {
                source_context,
                modes: vec![mtgml_state::ModeBinding {
                    mode_slot: 0,
                    selected_mode: 1,
                }],
                targets: vec![],
                cost_facts: Default::default(),
            }),
        },
    );
    state.zones.stack_order.push(StackObjectId(1));
    state.validate().unwrap();
    assert!(matches!(
        &state.zones.stack_records[&StackObjectId(1)].payload,
        Some(StackItemPayload::ActivatedAbility { modes, .. })
            if modes == &[mtgml_state::ModeBinding { mode_slot: 0, selected_mode: 1 }]
    ));
}

#[test]
fn triggered_stack_payload_captures_every_closed_event_fact_across_source_departure() {
    let mut state = root();
    state.allocators.next_object_id = GameObjectId(4);
    state.allocators.next_ability_id = AbilityInstanceId(2);
    state.allocators.next_stack_object_id = StackObjectId(2);
    state.allocators.next_trigger_id = mtgml_model::TriggerInstanceId(2);
    let source_context = AbilitySourceContext {
        source: mtgml_state::SourceContext {
            snapshot: mtgml_state::ObjectSnapshot {
                object: GameObjectId(3),
                physical_card: Some(PhysicalCardId(3)),
                card_definition: CardDefinitionId(3),
                owner: PlayerId(1),
                controller: PlayerId(1),
                tapped: false,
                face_down: false,
                location: ZoneLocation {
                    zone: ZoneKind::Graveyard,
                    player: Some(PlayerId(1)),
                    position: ZonePosition::Unordered,
                    visibility: VisibilityPartition::Public,
                    partition: None,
                },
            },
            face_key: FaceKey(0),
            semantic_profile_id: CardSemanticProfileId::parse("test/trigger-source@1.0.0").unwrap(),
        },
        ability_instance_id: AbilityInstanceId(1),
        ability_key: mtgml_card_ir::AbilityKey(4),
    };
    let snapshot = source_context.source.snapshot.clone();
    let events = [
        mtgml_state::TriggerEventSnapshot::SpellCast {
            actor: PlayerId(1),
            stack_item: StackObjectId(1),
            spell: source_context.source.clone(),
            is_creature_spell: true,
            cost_facts: Default::default(),
        },
        mtgml_state::TriggerEventSnapshot::AbilityActivated {
            actor: PlayerId(1),
            stack_item: StackObjectId(1),
            source: source_context.clone(),
            targets: vec![],
            cost_facts: Default::default(),
        },
        mtgml_state::TriggerEventSnapshot::TargetBecame {
            actor: PlayerId(1),
            source_stack_item: StackObjectId(1),
            target: mtgml_state::TargetRef::Object(GameObjectId(1)),
        },
        mtgml_state::TriggerEventSnapshot::ObjectEntered {
            object: snapshot.clone(),
        },
        mtgml_state::TriggerEventSnapshot::ObjectLeftOrDied {
            last_known: snapshot,
            destination: ZoneLocation {
                zone: ZoneKind::Graveyard,
                player: Some(PlayerId(1)),
                position: ZonePosition::Unordered,
                visibility: VisibilityPartition::Public,
                partition: None,
            },
        },
        mtgml_state::TriggerEventSnapshot::BeginningOfCombat {
            active_player: PlayerId(1),
            turn_number: state.core.turn_number,
        },
        mtgml_state::TriggerEventSnapshot::AttackDeclared {
            controller: PlayerId(1),
            attackers: vec![mtgml_state::AttackerFact {
                object: GameObjectId(1),
                defending_player: PlayerId(2),
            }],
        },
        mtgml_state::TriggerEventSnapshot::CardDrawn {
            player: PlayerId(1),
        },
        mtgml_state::TriggerEventSnapshot::CounterChanged {
            object: GameObjectId(1),
            kind: mtgml_state::CounterKindV1::Lore,
            before: 0,
            after: 1,
        },
        mtgml_state::TriggerEventSnapshot::DamageApplied {
            source: Some(source_context.source.clone()),
            recipient: mtgml_state::DamageRecipient::Player(PlayerId(2)),
            amount: 1,
            damage_kind: mtgml_state::DamageKind::Noncombat,
        },
        mtgml_state::TriggerEventSnapshot::LifeChanged {
            player: PlayerId(1),
            before: 20,
            after: 19,
            cause: mtgml_state::LifeChangeCause::NonDamage,
        },
    ];
    for trigger_context in events {
        state.zones.stack_records.insert(
            StackObjectId(1),
            StackRecord {
                id: StackObjectId(1),
                controller: PlayerId(1),
                payload: Some(StackItemPayload::TriggeredAbility {
                    originating_trigger: mtgml_model::TriggerInstanceId(1),
                    source_context: source_context.clone(),
                    captured_trigger_context: Box::new(trigger_context),
                    targets: vec![],
                }),
            },
        );
        state.zones.stack_order = vec![StackObjectId(1)];
        state.validate().unwrap();
    }
    state.zones.stack_records.clear();
    state.zones.stack_order.clear();
    state.execution.waiting_triggers.insert(
        mtgml_model::TriggerInstanceId(1),
        mtgml_state::PendingTriggerRecord {
            id: mtgml_model::TriggerInstanceId(1),
            controller: PlayerId(1),
            source_context,
            trigger_context: mtgml_state::TriggerEventSnapshot::CardDrawn {
                player: PlayerId(1),
            },
            target_timing: mtgml_state::TriggerTargetTiming::NoTargets,
        },
    );
    assert_eq!(
        state.validate(),
        Err(mtgml_state::EngineStateError::MissingTriggerPlacement)
    );
}

#[test]
fn flat_state_exposes_every_component_at_the_top_level() {
    let state = construct_synthetic_engine_state(SyntheticResetInputs {
        players: [PlayerId(1), PlayerId(2)],
        root_seed: RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap(),
        setup: SyntheticV4Setup::synthetic_compatibility(),
    })
    .unwrap();
    assert_eq!(state.card_rules.mana.pools.len(), 2);
    assert!(state.execution.pending_decision.is_none());
    assert_eq!(state.allocators.next_decision_id, DecisionId(2));
    state.validate_structure().unwrap();
}

/// P1 pays for a spell on the stack (object 4, stack object 1) that costs
/// {1}: its Cast continuation is paying mana, awaiting the final allocation
/// with no source activations, and P1 is asked how to pay. P1 also has a land
/// (object 3) with a mana ability, untapped.
fn paying_for_a_spell() -> EngineState {
    let mut state = root();
    let land = GameObjectId(3);
    state.zones.objects.insert(
        land,
        mtgml_state::GameObject {
            id: land,
            physical_card: Some(PhysicalCardId(3)),
            card_definition: CardDefinitionId(3),
            owner: PlayerId(1),
            controller: PlayerId(1),
            tapped: false,
            face_down: false,
        },
    );
    state.zones.locations.insert(
        land,
        ZoneLocation {
            zone: ZoneKind::Battlefield,
            player: None,
            position: ZonePosition::Unordered,
            visibility: VisibilityPartition::Public,
            partition: None,
        },
    );
    state.allocators.next_object_id = GameObjectId(4);
    state.allocators.next_ability_id = AbilityInstanceId(2);
    state.card_rules.abilities.by_instance.insert(
        AbilityInstanceId(1),
        mtgml_state::AbilityAuthorityV1 {
            source: land,
            ability_key: 0,
        },
    );
    add_stack_spell(&mut state, 4, 1);
    for object in state.zones.objects.keys().copied() {
        state.card_rules.faces.faces.insert(object, 0);
        if state.zones.locations[&object].zone == ZoneKind::Battlefield {
            state.card_rules.permanents.enter(object, 1).unwrap();
        }
    }
    let continuation_id = ContinuationId(1);
    state.allocators.next_continuation_id = ContinuationId(2);
    state.allocators.next_decision_id = DecisionId(3);
    state.execution.continuations.insert(
        continuation_id,
        mtgml_state::ContinuationRecord {
            id: continuation_id,
            created_at_revision: state.revision,
            payload: mtgml_state::ContinuationPayload::Cast(mtgml_state::CastContinuation {
                actor: PlayerId(1),
                spell_object: GameObjectId(4),
                card_definition_id: CardDefinitionId(4),
                face_key: FaceKey(0),
                semantic_profile_id: CardSemanticProfileId::parse("test/spell@1.0.0").unwrap(),
                stage: mtgml_state::CastContinuationStage::PayingMana,
                selected_route: Some(mtgml_state::CostRoute::Normal),
                modes: vec![],
                targets: vec![],
                paid_cost_choices: vec![],
                action_cost_facts: mtgml_state::ActionCostFacts {
                    mana_cost: Some(ManaCost {
                        colored_wubrg_counts: [0; 5],
                        colorless_count: 0,
                        generic_count: 1,
                    }),
                    reserved_nonmana_costs: vec![],
                    selected_cost_operands: vec![],
                },
                mana_payment_staging: Some(ManaPaymentStaging {
                    stage: ManaPaymentStage::AwaitingFinalAllocation,
                    mana_source_activations: vec![],
                }),
            }),
        },
    );
    let view_sequence = state.knowledge.players[&PlayerId(1)].next_visible_sequence;
    state.execution.pending_decision = Some(mtgml_decision::AuthoritativeDecisionRequest {
        decision_id: DecisionId(2),
        player_decision_id: PlayerDecisionIdV1(1),
        state_revision: state.revision,
        view_sequence,
        actor: PlayerId(1),
        visibility: mtgml_decision::DecisionVisibility::ActingPlayerOnly,
        decision_domain_v2: mtgml_decision::DecisionDomainV2::ChooseOne,
        purpose: mtgml_decision::DecisionPurposeV4::ManaPayment,
        parent_player_decision_id: None,
        continuation_id: Some(continuation_id),
        candidates: [
            [0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0],
            [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        ]
        .into_iter()
        .enumerate()
        .map(
            |(index, spent_buckets)| mtgml_decision::AuthoritativeCandidate {
                candidate_id: CandidateIdV1(index as u32),
                visible_intent: mtgml_decision::CandidateIntent::SelectManaPayment {
                    spent_buckets,
                },
                trusted_binding: mtgml_decision::EngineCandidateBinding::SelectManaPayment {
                    spent_buckets,
                },
            },
        )
        .collect(),
    });
    state
}

fn cast_continuation(state: &mut EngineState) -> &mut mtgml_state::CastContinuation {
    let record = state
        .execution
        .continuations
        .get_mut(&ContinuationId(1))
        .unwrap();
    let mtgml_state::ContinuationPayload::Cast(cast) = &mut record.payload else {
        unreachable!()
    };
    cast
}

#[test]
fn a_cast_continuation_awaiting_the_final_allocation_is_a_valid_payment() {
    paying_for_a_spell().validate_structure().unwrap();
}

#[test]
fn a_cast_continuation_paying_mana_has_no_source_activation() {
    // The source is a live untapped land that could be activated, so only the
    // cast's own rule rejects the activation: this slice pays from the pool
    // (a design choice of the vanilla-creatures spec §3, not a requirement of
    // the rules, which let mana abilities be activated while casting, CR
    // 601.2g and 605.3a), so the payment stages no mana source of its own.
    let mut state = paying_for_a_spell();
    cast_continuation(&mut state)
        .mana_payment_staging
        .as_mut()
        .unwrap()
        .mana_source_activations
        .push(ManaSourceActivation {
            source_object: GameObjectId(3),
            source_ability_instance: AbilityInstanceId(1),
            ability_key: mtgml_card_ir::AbilityKey(0),
            semantic_profile_id: CardSemanticProfileId::parse("test/mana-source@1.0.0").unwrap(),
            activation_cost_receipt: ManaSourceActivationCost::TapSource,
            produced_buckets: [0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0],
        });
    assert_eq!(
        state.validate_structure(),
        Err(mtgml_state::EngineStateError::ManaPaymentStaging)
    );
}

#[test]
fn a_cast_continuation_paying_mana_awaits_the_final_allocation() {
    let mut state = paying_for_a_spell();
    cast_continuation(&mut state)
        .mana_payment_staging
        .as_mut()
        .unwrap()
        .stage = ManaPaymentStage::SelectingSources;
    assert_eq!(
        state.validate_structure(),
        Err(mtgml_state::EngineStateError::ManaPaymentStaging)
    );
}

#[test]
fn a_cast_continuation_names_a_spell_on_the_stack() {
    // The card the continuation names is on the battlefield, not on the
    // stack.
    let mut state = paying_for_a_spell();
    let cast = cast_continuation(&mut state);
    cast.spell_object = GameObjectId(3);
    cast.card_definition_id = CardDefinitionId(3);
    assert_eq!(
        state.validate_structure(),
        Err(mtgml_state::EngineStateError::ContinuationRecord)
    );

    // The card is on the stack, but its record is another player's spell.
    let mut state = paying_for_a_spell();
    state
        .zones
        .stack_records
        .get_mut(&StackObjectId(1))
        .unwrap()
        .controller = PlayerId(2);
    assert_eq!(
        state.validate_structure(),
        Err(mtgml_state::EngineStateError::ContinuationRecord)
    );

    // The continuation names another profile than the spell's record has.
    let mut state = paying_for_a_spell();
    cast_continuation(&mut state).semantic_profile_id =
        CardSemanticProfileId::parse("test/other@1.0.0").unwrap();
    assert_eq!(
        state.validate_structure(),
        Err(mtgml_state::EngineStateError::ContinuationRecord)
    );
}

/// P1 attacks P2 with object 3 in the declare blockers step. P2 controls the
/// untapped objects 5 and 6 and is asked about 5 first (CR 509.1a); object 4
/// is P1's, and did not attack. P2 knows all four by opaque identities.
fn blocks_being_declared() -> EngineState {
    let mut state = root();
    for (object, controller) in [(3, 1), (4, 1), (5, 2), (6, 2)] {
        let id = GameObjectId(object);
        state.zones.objects.insert(
            id,
            mtgml_state::GameObject {
                id,
                physical_card: Some(PhysicalCardId(object)),
                card_definition: CardDefinitionId(3),
                owner: PlayerId(controller),
                controller: PlayerId(controller),
                tapped: object == 3,
                face_down: false,
            },
        );
        state.zones.locations.insert(
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
    state.allocators.next_object_id = GameObjectId(7);
    for object in state.zones.objects.keys().copied().collect::<Vec<_>>() {
        state.card_rules.faces.faces.insert(object, 0);
        if state.zones.locations[&object].zone == ZoneKind::Battlefield {
            state.card_rules.permanents.enter(object, 1).unwrap();
        }
    }
    for object in 3..=6 {
        let (id, opaque) = (
            GameObjectId(object),
            mtgml_model::OpaqueObjectId(object + 10),
        );
        let location = state.zones.locations[&id].clone();
        let identity = state
            .perspective_identities
            .players
            .get_mut(&PlayerId(2))
            .unwrap();
        identity.object_to_opaque.insert(id, opaque);
        identity.opaque_to_object.insert(opaque, id);
        identity.next_opaque_object_id = mtgml_model::OpaqueObjectId(opaque.0 + 1);
        state
            .knowledge
            .players
            .get_mut(&PlayerId(2))
            .unwrap()
            .active
            .insert(
                opaque,
                mtgml_state::KnowledgeRecordV2 {
                    opaque_object: opaque,
                    physical_card: Some(PhysicalCardId(object)),
                    card_definition: Some(CardDefinitionId(3)),
                    known_location: Some(mtgml_state::KnownLocationFactV2 {
                        location,
                        provenance: mtgml_state::KnowledgeAcquisitionReason::InitialConfiguration,
                    }),
                    acquisition: mtgml_state::KnowledgeAcquisitionReason::InitialConfiguration,
                    historical_locations: vec![],
                },
            );
    }
    state.core.position = mtgml_state::TurnPosition::Combat {
        step: mtgml_state::CombatStep::DeclareBlockers,
    };
    state.core.priority = mtgml_state::PriorityState::None;
    state.combat = Some(mtgml_state::CombatState {
        defending_player: PlayerId(2),
        attackers: vec![GameObjectId(3)],
        damage_step_completed: false,
        blocked_attackers: Default::default(),
        blockers: Default::default(),
    });
    state.allocators.next_continuation_id = ContinuationId(2);
    state.allocators.next_decision_id = DecisionId(3);
    state.execution.continuations.insert(
        ContinuationId(1),
        mtgml_state::ContinuationRecord {
            id: ContinuationId(1),
            created_at_revision: state.revision,
            payload: mtgml_state::ContinuationPayload::BlockDeclaration {
                defender: PlayerId(2),
                pending_blockers: vec![GameObjectId(5), GameObjectId(6)],
                declared: Default::default(),
            },
        },
    );
    let view_sequence = state.knowledge.players[&PlayerId(2)].next_visible_sequence;
    let candidate = |index, attacker: Option<u64>| mtgml_decision::AuthoritativeCandidate {
        candidate_id: CandidateIdV1(index),
        visible_intent: mtgml_decision::CandidateIntent::DeclareBlock {
            blocker: mtgml_model::OpaqueObjectId(15),
            attacker: attacker.map(|attacker| mtgml_model::OpaqueObjectId(attacker + 10)),
        },
        trusted_binding: mtgml_decision::EngineCandidateBinding::DeclareBlock {
            blocker: GameObjectId(5),
            attacker: attacker.map(GameObjectId),
        },
    };
    state.execution.pending_decision = Some(mtgml_decision::AuthoritativeDecisionRequest {
        decision_id: DecisionId(2),
        player_decision_id: PlayerDecisionIdV1(1),
        state_revision: state.revision,
        view_sequence,
        actor: PlayerId(2),
        visibility: mtgml_decision::DecisionVisibility::ActingPlayerOnly,
        decision_domain_v2: mtgml_decision::DecisionDomainV2::ChooseOne,
        purpose: mtgml_decision::DecisionPurposeV4::BlockerDeclaration,
        parent_player_decision_id: None,
        continuation_id: Some(ContinuationId(1)),
        candidates: vec![candidate(0, None), candidate(1, Some(3))],
    });
    state
}

fn block_declaration(
    state: &mut EngineState,
) -> (
    &mut PlayerId,
    &mut Vec<GameObjectId>,
    &mut std::collections::BTreeMap<GameObjectId, Option<GameObjectId>>,
) {
    let record = state
        .execution
        .continuations
        .get_mut(&ContinuationId(1))
        .unwrap();
    match &mut record.payload {
        mtgml_state::ContinuationPayload::BlockDeclaration {
            defender,
            pending_blockers,
            declared,
        } => (defender, pending_blockers, declared),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_block_declaration_names_the_defender_and_the_untapped_creatures_it_controls() {
    use mtgml_state::EngineStateError::BlockDeclaration;
    let state = blocks_being_declared();
    state.validate_structure().unwrap();
    // The request names a creature of its own, which the G0 runtime boundary
    // does not admit.
    assert_eq!(
        state.validate(),
        Err(mtgml_state::EngineStateError::ProfileDependentDecisionNotAdmitted)
    );

    let refused = |what: &str, edit: &dyn Fn(&mut EngineState)| {
        let mut forged = state.clone();
        edit(&mut forged);
        assert_eq!(forged.validate_structure(), Err(BlockDeclaration), "{what}");
    };
    // CR 509.1a: the defending player declares the blockers.
    refused("the attacking player", &|state| {
        *block_declaration(state).0 = PlayerId(1);
    });
    refused("nobody asked", &|state| block_declaration(state).1.clear());
    // The creatures are permanents that player controls, and they are untapped.
    refused("a creature of the attacking player", &|state| {
        block_declaration(state).1[1] = GameObjectId(4);
    });
    refused("a tapped creature that is also the attacker", &|state| {
        block_declaration(state).1[1] = GameObjectId(3);
    });
    refused("a tapped creature of the defender", &|state| {
        state
            .zones
            .objects
            .get_mut(&GameObjectId(6))
            .unwrap()
            .tapped = true;
    });
    // Object 2 is a card in P2s library.
    refused("a card that is not on the battlefield", &|state| {
        block_declaration(state).1[1] = GameObjectId(2);
    });
    refused("a creature asked twice", &|state| {
        block_declaration(state).1[1] = GameObjectId(5);
    });
    refused("a creature that is asked and answered", &|state| {
        block_declaration(state).2.insert(GameObjectId(5), None);
    });
    // Every answered creature blocks a real attacker, or nothing.
    refused("a block of a creature that did not attack", &|state| {
        let (_, pending, declared) = block_declaration(state);
        declared.insert(pending.remove(0), Some(GameObjectId(4)));
    });
    refused("a block of the defenders own creature", &|state| {
        let (_, pending, declared) = block_declaration(state);
        declared.insert(pending.remove(0), Some(GameObjectId(6)));
    });
    // It belongs to the declare blockers step of an attack, before any block,
    // and nobody has priority.
    refused("another step", &|state| {
        state.core.position = mtgml_state::TurnPosition::Combat {
            step: mtgml_state::CombatStep::DeclareAttackers,
        };
    });
    refused("priority", &|state| {
        state.core.priority = mtgml_state::PriorityState::HeldBy {
            player: PlayerId(1),
            consecutive_passes: 0,
        };
    });
    refused("no combat", &|state| state.combat = None);
    refused("a recorded block", &|state| {
        let combat = state.combat.as_mut().unwrap();
        combat
            .blockers
            .insert(GameObjectId(5), Some(GameObjectId(3)));
        combat.blocked_attackers.insert(GameObjectId(3));
    });
    refused("blocked attackers", &|state| {
        state
            .combat
            .as_mut()
            .unwrap()
            .blocked_attackers
            .insert(GameObjectId(3));
    });
    refused("no attackers", &|state| {
        state.combat.as_mut().unwrap().attackers.clear();
    });

    // A half-declared block: the answered creature blocks the attacker, and
    // the other is asked next.
    let mut half = state.clone();
    let (_, pending, declared) = block_declaration(&mut half);
    declared.insert(pending.remove(0), Some(GameObjectId(3)));
    let request = half.execution.pending_decision.as_mut().unwrap();
    for candidate in &mut request.candidates {
        let mtgml_decision::CandidateIntent::DeclareBlock { blocker, .. } =
            &mut candidate.visible_intent
        else {
            unreachable!()
        };
        *blocker = mtgml_model::OpaqueObjectId(16);
        let mtgml_decision::EngineCandidateBinding::DeclareBlock { blocker, .. } =
            &mut candidate.trusted_binding
        else {
            unreachable!()
        };
        *blocker = GameObjectId(6);
    }
    half.validate_structure().unwrap();
}

#[test]
fn a_block_request_asks_about_the_next_creature_and_the_real_attackers() {
    use mtgml_decision::{CandidateIntent as Intent, EngineCandidateBinding as Binding};
    use mtgml_state::EngineStateError::{ContinuationRequestMismatch, PendingCandidateBinding};
    let state = blocks_being_declared();
    let refused =
        |what: &str, error, edit: &dyn Fn(&mut mtgml_decision::AuthoritativeDecisionRequest)| {
            let mut forged = state.clone();
            edit(forged.execution.pending_decision.as_mut().unwrap());
            assert_eq!(forged.validate_structure(), Err(error), "{what}");
        };
    // The request is the continuations: the defenders, one answer.
    refused("another purpose", ContinuationRequestMismatch, &|request| {
        request.purpose = mtgml_decision::DecisionPurposeV4::PriorityAction;
        request.candidates = vec![mtgml_decision::AuthoritativeCandidate {
            candidate_id: CandidateIdV1(0),
            visible_intent: Intent::PassPriority,
            trusted_binding: Binding::PassPriority,
        }];
    });
    // The request itself does not allow several answers.
    refused(
        "several answers",
        mtgml_state::EngineStateError::PendingDecision,
        &|request| {
            request.decision_domain_v2 = mtgml_decision::DecisionDomainV2::ChooseMany {
                minimum: 0,
                maximum: 2,
            };
        },
    );
    refused(
        "the attacking player",
        ContinuationRequestMismatch,
        &|request| {
            request.actor = PlayerId(1);
        },
    );
    // The candidates are the next creature blocking a real attacker, or nothing.
    refused(
        "another creature of the declaration",
        PendingCandidateBinding,
        &|request| {
            for candidate in &mut request.candidates {
                candidate.visible_intent = Intent::DeclareBlock {
                    blocker: mtgml_model::OpaqueObjectId(16),
                    attacker: match candidate.visible_intent {
                        Intent::DeclareBlock { attacker, .. } => attacker,
                        _ => unreachable!(),
                    },
                };
                candidate.trusted_binding = Binding::DeclareBlock {
                    blocker: GameObjectId(6),
                    attacker: match candidate.trusted_binding {
                        Binding::DeclareBlock { attacker, .. } => attacker,
                        _ => unreachable!(),
                    },
                };
            }
        },
    );
    refused(
        "a creature of the attacking player",
        PendingCandidateBinding,
        &|request| {
            request.candidates[0].visible_intent = Intent::DeclareBlock {
                blocker: mtgml_model::OpaqueObjectId(14),
                attacker: None,
            };
            request.candidates[0].trusted_binding = Binding::DeclareBlock {
                blocker: GameObjectId(4),
                attacker: None,
            };
        },
    );
    refused(
        "a block of a creature that did not attack",
        PendingCandidateBinding,
        &|request| {
            request.candidates[1].visible_intent = Intent::DeclareBlock {
                blocker: mtgml_model::OpaqueObjectId(15),
                attacker: Some(mtgml_model::OpaqueObjectId(14)),
            };
            request.candidates[1].trusted_binding = Binding::DeclareBlock {
                blocker: GameObjectId(5),
                attacker: Some(GameObjectId(4)),
            };
        },
    );
    refused(
        "an attacker the identities do not bind",
        PendingCandidateBinding,
        &|request| {
            request.candidates[1].visible_intent = Intent::DeclareBlock {
                blocker: mtgml_model::OpaqueObjectId(15),
                attacker: Some(mtgml_model::OpaqueObjectId(16)),
            };
        },
    );
    refused(
        "a block of an attacker, with no attacker visible",
        PendingCandidateBinding,
        &|request| {
            request.candidates.remove(0);
            request.candidates[0].candidate_id = CandidateIdV1(0);
            request.candidates[0].visible_intent = Intent::DeclareBlock {
                blocker: mtgml_model::OpaqueObjectId(15),
                attacker: None,
            };
        },
    );
    refused(
        "no block, with an attacker bound",
        PendingCandidateBinding,
        &|request| {
            request.candidates[0].trusted_binding = Binding::DeclareBlock {
                blocker: GameObjectId(5),
                attacker: Some(GameObjectId(3)),
            };
        },
    );
    refused(
        "an opaque identity that is not the blockers",
        PendingCandidateBinding,
        &|request| {
            for candidate in &mut request.candidates {
                candidate.visible_intent = Intent::DeclareBlock {
                    blocker: mtgml_model::OpaqueObjectId(16),
                    attacker: match candidate.visible_intent {
                        Intent::DeclareBlock { attacker, .. } => attacker,
                        _ => unreachable!(),
                    },
                };
            }
        },
    );
    // A request about no declaration at all.
    let mut orphan = state.clone();
    orphan.execution.continuations.clear();
    orphan
        .execution
        .pending_decision
        .as_mut()
        .unwrap()
        .continuation_id = None;
    assert_eq!(orphan.validate_structure(), Err(PendingCandidateBinding));
}

/// P1 attacks P2 with object 3, which P2's objects 5 and 6 block, and is in the
/// combat damage step, dividing the damage of the attacker (CR 510.1c): it is
/// asked how much of it object 5 is assigned, from 0 to 3. P1 knows 3, 5 and 6
/// by opaque identities (the object's plus 20); object 4 is P1's and did not
/// attack.
fn damage_being_divided() -> EngineState {
    use mtgml_decision::{AuthoritativeCandidate, CandidateIntent, EngineCandidateBinding};
    let mut state = blocks_being_declared();
    let identity = state
        .perspective_identities
        .players
        .get_mut(&PlayerId(1))
        .unwrap();
    for object in [3, 5, 6] {
        let (id, opaque) = (
            GameObjectId(object),
            mtgml_model::OpaqueObjectId(object + 20),
        );
        identity.object_to_opaque.insert(id, opaque);
        identity.opaque_to_object.insert(opaque, id);
    }
    identity.next_opaque_object_id = mtgml_model::OpaqueObjectId(100);
    identity.next_player_decision_id = PlayerDecisionIdV1(2);
    state.core.position = mtgml_state::TurnPosition::Combat {
        step: mtgml_state::CombatStep::CombatDamage,
    };
    let combat = state.combat.as_mut().unwrap();
    combat.blocked_attackers.insert(GameObjectId(3));
    for blocker in [5, 6] {
        combat
            .blockers
            .insert(GameObjectId(blocker), Some(GameObjectId(3)));
    }
    state
        .execution
        .continuations
        .get_mut(&ContinuationId(1))
        .unwrap()
        .payload = mtgml_state::ContinuationPayload::CombatDamageAssignment {
        player: PlayerId(1),
        pending_attackers: vec![GameObjectId(3)],
        pending_blockers: vec![GameObjectId(5), GameObjectId(6)],
        assigned: Default::default(),
    };
    let view_sequence = state.knowledge.players[&PlayerId(1)].next_visible_sequence;
    state.execution.pending_decision = Some(mtgml_decision::AuthoritativeDecisionRequest {
        decision_id: DecisionId(2),
        player_decision_id: PlayerDecisionIdV1(1),
        state_revision: state.revision,
        view_sequence,
        actor: PlayerId(1),
        visibility: mtgml_decision::DecisionVisibility::ActingPlayerOnly,
        decision_domain_v2: mtgml_decision::DecisionDomainV2::ChooseOne,
        purpose: mtgml_decision::DecisionPurposeV4::CombatDamageAssignment,
        parent_player_decision_id: None,
        continuation_id: Some(ContinuationId(1)),
        candidates: (0..=3)
            .map(|amount| AuthoritativeCandidate {
                candidate_id: CandidateIdV1(amount as u32),
                visible_intent: CandidateIntent::AssignCombatDamage {
                    attacker: mtgml_model::OpaqueObjectId(23),
                    recipient: mtgml_model::OpaqueObjectId(25),
                    amount,
                },
                trusted_binding: EngineCandidateBinding::AssignCombatDamage {
                    attacker: GameObjectId(3),
                    recipient: GameObjectId(5),
                    amount,
                },
            })
            .collect(),
    });
    state
}

fn damage_division(
    state: &mut EngineState,
) -> (
    &mut PlayerId,
    &mut Vec<GameObjectId>,
    &mut Vec<GameObjectId>,
    &mut std::collections::BTreeMap<GameObjectId, u64>,
) {
    let record = state
        .execution
        .continuations
        .get_mut(&ContinuationId(1))
        .unwrap();
    match &mut record.payload {
        mtgml_state::ContinuationPayload::CombatDamageAssignment {
            player,
            pending_attackers,
            pending_blockers,
            assigned,
        } => (player, pending_attackers, pending_blockers, assigned),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_combat_damage_assignment_names_the_attacking_player_and_the_blockers_of_its_attacker() {
    use mtgml_state::EngineStateError::CombatDamageAssignment;
    let state = damage_being_divided();
    state.validate_structure().unwrap();
    // The request names creatures of its own, which the G0 runtime boundary
    // does not admit.
    assert_eq!(
        state.validate(),
        Err(mtgml_state::EngineStateError::ProfileDependentDecisionNotAdmitted)
    );

    let refused = |what: &str, edit: &dyn Fn(&mut EngineState)| {
        let mut forged = state.clone();
        edit(&mut forged);
        assert_eq!(
            forged.validate_structure(),
            Err(CombatDamageAssignment),
            "{what}"
        );
    };
    // CR 510.1c: the attacking player divides the damage.
    refused("the defending player", &|state| {
        *damage_division(state).0 = PlayerId(2);
    });
    // The attackers are attacking creatures, each once, with one being divided.
    refused("no attacker", &|state| damage_division(state).1.clear());
    refused("an attacker that does not attack", &|state| {
        damage_division(state).1[0] = GameObjectId(4);
    });
    refused("an attacker twice", &|state| {
        let attackers = damage_division(state).1;
        attackers.push(attackers[0]);
    });
    // The blockers asked block the attacker being divided, each once.
    refused("nobody asked", &|state| damage_division(state).2.clear());
    refused("a creature that does not block", &|state| {
        damage_division(state).2[1] = GameObjectId(4);
    });
    refused("a creature of the attacking player", &|state| {
        damage_division(state).2[1] = GameObjectId(3);
    });
    refused("a blocker asked twice", &|state| {
        damage_division(state).2[1] = GameObjectId(5);
    });
    refused("a blocker of another attacker", &|state| {
        let combat = state.combat.as_mut().unwrap();
        combat.attackers.push(GameObjectId(4));
        combat.attackers.sort();
        combat.blocked_attackers.insert(GameObjectId(4));
        combat
            .blockers
            .insert(GameObjectId(6), Some(GameObjectId(4)));
    });
    refused("a blocker that is asked and answered", &|state| {
        damage_division(state).3.insert(GameObjectId(5), 1);
    });
    refused("an answer for a creature that does not block", &|state| {
        damage_division(state).3.insert(GameObjectId(4), 1);
    });
    // It belongs to the combat damage step of an attack, before the damage is
    // dealt, and nobody has priority.
    refused("another step", &|state| {
        state.core.position = mtgml_state::TurnPosition::Combat {
            step: mtgml_state::CombatStep::DeclareBlockers,
        };
    });
    refused("priority", &|state| {
        state.core.priority = mtgml_state::PriorityState::HeldBy {
            player: PlayerId(1),
            consecutive_passes: 0,
        };
    });
    refused("the damage dealt", &|state| {
        state.combat.as_mut().unwrap().damage_step_completed = true;
    });
    refused("no combat", &|state| state.combat = None);

    // A half-divided damage: object 5 has been answered, and 6 is asked next.
    let mut half = state.clone();
    {
        let (_, _, pending, assigned) = damage_division(&mut half);
        assigned.insert(pending.remove(0), 1);
    }
    let request = half.execution.pending_decision.as_mut().unwrap();
    for candidate in &mut request.candidates {
        let mtgml_decision::CandidateIntent::AssignCombatDamage { recipient, .. } =
            &mut candidate.visible_intent
        else {
            unreachable!()
        };
        *recipient = mtgml_model::OpaqueObjectId(26);
        let mtgml_decision::EngineCandidateBinding::AssignCombatDamage { recipient, .. } =
            &mut candidate.trusted_binding
        else {
            unreachable!()
        };
        *recipient = GameObjectId(6);
    }
    half.validate_structure().unwrap();
    // An answered blocker of an attacker that has not been reached yet is
    // refused: object 4 attacks too, and is divided after object 3.
    let mut later = half.clone();
    let combat = later.combat.as_mut().unwrap();
    combat.attackers.push(GameObjectId(4));
    combat.attackers.sort();
    damage_division(&mut later).1.push(GameObjectId(4));
    later.validate_structure().unwrap();
    let combat = later.combat.as_mut().unwrap();
    combat
        .blockers
        .insert(GameObjectId(5), Some(GameObjectId(4)));
    combat.blocked_attackers.insert(GameObjectId(4));
    assert_eq!(later.validate_structure(), Err(CombatDamageAssignment));
}

#[test]
fn a_combat_damage_request_asks_about_the_next_blocker_and_an_amount_each() {
    use mtgml_decision::{CandidateIntent as Intent, EngineCandidateBinding as Binding};
    use mtgml_state::EngineStateError::{ContinuationRequestMismatch, PendingCandidateBinding};
    let state = damage_being_divided();
    let refused =
        |what: &str, error, edit: &dyn Fn(&mut mtgml_decision::AuthoritativeDecisionRequest)| {
            let mut forged = state.clone();
            edit(forged.execution.pending_decision.as_mut().unwrap());
            assert_eq!(forged.validate_structure(), Err(error), "{what}");
        };
    // The request is the continuation's: the attacking player's, one answer.
    refused("another purpose", ContinuationRequestMismatch, &|request| {
        request.purpose = mtgml_decision::DecisionPurposeV4::PriorityAction;
        request.candidates = vec![mtgml_decision::AuthoritativeCandidate {
            candidate_id: CandidateIdV1(0),
            visible_intent: Intent::PassPriority,
            trusted_binding: Binding::PassPriority,
        }];
    });
    refused(
        "the defending player",
        ContinuationRequestMismatch,
        &|request| {
            request.actor = PlayerId(2);
        },
    );
    refused(
        "several answers",
        mtgml_state::EngineStateError::PendingDecision,
        &|request| {
            request.decision_domain_v2 = mtgml_decision::DecisionDomainV2::ChooseMany {
                minimum: 0,
                maximum: 2,
            };
        },
    );
    // Each candidate is an amount for the next blocker of the attacker.
    refused(
        "the blocker after the next",
        PendingCandidateBinding,
        &|request| {
            for candidate in &mut request.candidates {
                let Intent::AssignCombatDamage { recipient, .. } = &mut candidate.visible_intent
                else {
                    unreachable!()
                };
                *recipient = mtgml_model::OpaqueObjectId(26);
                let Binding::AssignCombatDamage { recipient, .. } = &mut candidate.trusted_binding
                else {
                    unreachable!()
                };
                *recipient = GameObjectId(6);
            }
        },
    );
    refused("another attacker", PendingCandidateBinding, &|request| {
        for candidate in &mut request.candidates {
            let Intent::AssignCombatDamage { attacker, .. } = &mut candidate.visible_intent else {
                unreachable!()
            };
            *attacker = mtgml_model::OpaqueObjectId(24);
            let Binding::AssignCombatDamage { attacker, .. } = &mut candidate.trusted_binding
            else {
                unreachable!()
            };
            *attacker = GameObjectId(4);
        }
    });
    refused(
        "an opaque identity that is not the blockers",
        PendingCandidateBinding,
        &|request| {
            for candidate in &mut request.candidates {
                let Intent::AssignCombatDamage { recipient, .. } = &mut candidate.visible_intent
                else {
                    unreachable!()
                };
                *recipient = mtgml_model::OpaqueObjectId(26);
            }
        },
    );
    refused(
        "an amount that is not the bound one",
        PendingCandidateBinding,
        &|request| {
            let Intent::AssignCombatDamage { amount, .. } =
                &mut request.candidates[3].visible_intent
            else {
                unreachable!()
            };
            *amount = 4;
        },
    );
    // A request about no division at all.
    let mut orphan = state.clone();
    orphan.execution.continuations.clear();
    orphan
        .execution
        .pending_decision
        .as_mut()
        .unwrap()
        .continuation_id = None;
    assert_eq!(orphan.validate_structure(), Err(PendingCandidateBinding));
}
