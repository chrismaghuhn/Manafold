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
