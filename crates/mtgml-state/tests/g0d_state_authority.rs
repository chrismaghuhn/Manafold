use mtgml_card_ir::{CardSemanticProfileId, FaceKey};
use mtgml_model::{
    AbilityInstanceId, CandidateIdV1, CardDefinitionId, ContinuationId, DecisionId, GameObjectId,
    PhysicalCardId, PlayerDecisionIdV1, PlayerId, StackObjectId, StateRevision, VisibleSequence,
    ZoneKind,
};
use mtgml_random::RootSeed256;
use mtgml_state::{
    construct_synthetic_engine_state, AbilitySourceContext, CardRulesAuthoritativeStateV1,
    EngineStatePartsV2, EngineStatePartsV3, ExecutionStateV4, ManaCost, ManaPaymentStage,
    ManaPaymentStaging, ManaPoolV1, ManaSourceActivation, ManaSourceActivationCost,
    NonManaActivationContinuation, NonManaActivationStage, PlayerTurnHistoryV1,
    ReservedNonManaCost, SelectedCostOperand, StackItemPayload, StackRecord, SyntheticResetInputs,
    SyntheticV4Setup, VisibilityPartition, ZoneLocation, ZonePosition,
};

fn root() -> EngineStatePartsV3 {
    let mut state = construct_synthetic_engine_state(SyntheticResetInputs {
        players: [PlayerId(1), PlayerId(2)],
        root_seed: RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap(),
        setup: SyntheticV4Setup::synthetic_compatibility(),
    })
    .unwrap();
    state.execution = Default::default();
    let mut card_rules_state = CardRulesAuthoritativeStateV1::default();
    for player in state.core.players.keys().copied() {
        card_rules_state
            .mana
            .pools
            .insert(player, ManaPoolV1::default());
        card_rules_state
            .turn_history
            .players
            .insert(player, PlayerTurnHistoryV1::default());
    }
    card_rules_state.turn_history.turn_number = state.core.turn_number;
    EngineStatePartsV3::new(state.parts(), ExecutionStateV4::default(), card_rules_state).unwrap()
}

fn add_stack_spell(state: &mut EngineStatePartsV3, object_id: u64, stack_id: u64) {
    let object = GameObjectId(object_id);
    let card_definition = CardDefinitionId(object_id);
    state.predecessor_v5.zones.objects.insert(
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
    state.predecessor_v5.zones.locations.insert(
        object,
        ZoneLocation {
            zone: ZoneKind::Stack,
            player: None,
            position: ZonePosition::Unordered,
            visibility: VisibilityPartition::Public,
            partition: None,
        },
    );
    state.predecessor_v5.allocators.next_object_id = GameObjectId(object_id + 1);
    state.predecessor_v5.allocators.next_stack_object_id = StackObjectId(stack_id + 1);
    state.predecessor_v5.zones.stack_records.insert(
        StackObjectId(stack_id),
        StackRecord {
            id: StackObjectId(stack_id),
            controller: PlayerId(1),
            source_object: None,
            source_ability: None,
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
    state
        .predecessor_v5
        .zones
        .stack_order
        .push(StackObjectId(stack_id));
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

fn staged_blight_activation() -> (EngineStatePartsV3, ContinuationId) {
    let mut state = root();
    let source_object = GameObjectId(3);
    state.predecessor_v5.zones.objects.insert(
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
    state.predecessor_v5.zones.locations.insert(
        source_object,
        ZoneLocation {
            zone: ZoneKind::Battlefield,
            player: None,
            position: ZonePosition::Unordered,
            visibility: VisibilityPartition::Public,
            partition: None,
        },
    );
    state.predecessor_v5.allocators.next_object_id = GameObjectId(4);
    state.predecessor_v5.allocators.next_ability_id = AbilityInstanceId(3);
    state.predecessor_v5.allocators.next_continuation_id = ContinuationId(2);
    state.predecessor_v5.allocators.next_decision_id = DecisionId(3);
    state.card_rules_state.abilities.by_instance.insert(
        AbilityInstanceId(1),
        mtgml_state::AbilityAuthorityV1 {
            source: source_object,
            ability_key: 4,
        },
    );
    state.card_rules_state.abilities.by_instance.insert(
        AbilityInstanceId(2),
        mtgml_state::AbilityAuthorityV1 {
            source: GameObjectId(1),
            ability_key: 9,
        },
    );
    for object in state.predecessor_v5.zones.objects.keys().copied() {
        state.card_rules_state.faces.faces.insert(object, 0);
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
    state.execution_v4.continuations.insert(
        continuation_id,
        mtgml_state::ContinuationRecordV3 {
            id: continuation_id,
            created_at_revision: state.predecessor_v5.revision,
            payload: mtgml_state::ContinuationPayloadV3::NonManaActivation(
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
    let view_sequence = state.predecessor_v5.knowledge.players[&PlayerId(1)].next_visible_sequence;
    state.execution_v4.pending_decision = Some(mtgml_decision::AuthoritativeDecisionRequestV4 {
        decision_id: DecisionId(2),
        player_decision_id: PlayerDecisionIdV1(1),
        state_revision: StateRevision(state.predecessor_v5.revision.0),
        view_sequence,
        actor: PlayerId(1),
        visibility: mtgml_decision::DecisionVisibility::ActingPlayerOnly,
        decision_domain_v2: mtgml_decision::DecisionDomainV2::ChooseOne,
        purpose: mtgml_decision::DecisionPurposeV4::ManaProductionChoice,
        parent_player_decision_id: None,
        continuation_id: Some(continuation_id),
        candidates: vec![mtgml_decision::AuthoritativeCandidateV4 {
            candidate_id: CandidateIdV1(0),
            visible_intent: mtgml_decision::CandidateIntentV4::FinalizeManaProduction,
            trusted_binding: mtgml_decision::EngineCandidateBindingV4::FinalizeManaProduction {
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
    assert_eq!(state.execution_v4, ExecutionStateV4::default());
}

#[test]
fn predecessor_execution_cannot_duplicate_the_successor_execution_owner() {
    let state = root();
    let mut predecessor = state.predecessor_v5;
    predecessor.execution.effects.insert(
        mtgml_model::EffectInstanceId(1),
        mtgml_state::EffectRecord {
            id: mtgml_model::EffectInstanceId(1),
            label: "legacy-placeholder".to_owned(),
        },
    );
    assert_eq!(
        EngineStatePartsV3::new(
            predecessor,
            ExecutionStateV4::default(),
            state.card_rules_state,
        ),
        Err(mtgml_state::EngineStatePartsV3Error::DuplicateExecutionAuthority)
    );
}

#[test]
fn successor_stack_records_require_one_typed_payload_owner() {
    let mut state = root();
    state.predecessor_v5.zones.stack_records.insert(
        StackObjectId(1),
        StackRecord {
            id: StackObjectId(1),
            controller: PlayerId(1),
            source_object: None,
            source_ability: None,
            payload: None,
        },
    );
    state
        .predecessor_v5
        .zones
        .stack_order
        .push(StackObjectId(1));
    state.predecessor_v5.allocators.next_stack_object_id = StackObjectId(2);
    assert_eq!(
        state.validate(),
        Err(mtgml_state::EngineStatePartsV3Error::MissingStackPayload)
    );
}

#[test]
fn stack_zone_card_requires_exactly_one_spell_payload() {
    let mut state = root();
    let card = GameObjectId(3);
    state.predecessor_v5.zones.objects.insert(
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
    state.predecessor_v5.zones.locations.insert(
        card,
        ZoneLocation {
            zone: ZoneKind::Stack,
            player: None,
            position: ZonePosition::Unordered,
            visibility: VisibilityPartition::Public,
            partition: None,
        },
    );
    state.predecessor_v5.allocators.next_object_id = GameObjectId(4);
    assert_eq!(
        state.validate(),
        Err(mtgml_state::EngineStatePartsV3Error::StackCardReference)
    );
}

#[test]
fn paused_stack_resolution_must_reference_the_current_top_item() {
    let mut state = root();
    add_stack_spell(&mut state, 3, 1);
    add_stack_spell(&mut state, 4, 2);
    let id = ContinuationId(1);
    state.predecessor_v5.allocators.next_continuation_id = ContinuationId(2);
    state.execution_v4.continuations.insert(
        id,
        mtgml_state::ContinuationRecordV3 {
            id,
            created_at_revision: state.predecessor_v5.revision,
            payload: mtgml_state::ContinuationPayloadV3::StackResolution(
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
        Err(mtgml_state::EngineStatePartsV3Error::StackResolution)
    );
}

#[test]
fn stack_resolution_continuation_must_keep_the_resolving_item_in_the_stack_owner() {
    let mut state = root();
    let id = mtgml_model::ContinuationId(1);
    state.predecessor_v5.allocators.next_continuation_id = mtgml_model::ContinuationId(2);
    state.execution_v4.continuations.insert(
        id,
        mtgml_state::ContinuationRecordV3 {
            id,
            created_at_revision: state.predecessor_v5.revision,
            payload: mtgml_state::ContinuationPayloadV3::StackResolution(
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
        Err(mtgml_state::EngineStatePartsV3Error::StackResolution)
    );
}

#[test]
fn v3_continuation_preserves_frozen_synthetic_and_sba_stage_invariants() {
    let mut synthetic = root();
    let synthetic_id = ContinuationId(1);
    synthetic.predecessor_v5.allocators.next_continuation_id = ContinuationId(2);
    synthetic.execution_v4.continuations.insert(
        synthetic_id,
        mtgml_state::ContinuationRecordV3 {
            id: synthetic_id,
            created_at_revision: synthetic.predecessor_v5.revision,
            payload: mtgml_state::ContinuationPayloadV3::SyntheticAssembly {
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
        Err(mtgml_state::EngineStatePartsV3Error::ContinuationRecord)
    );

    let mut sba = root();
    let sba_id = ContinuationId(1);
    sba.predecessor_v5.allocators.next_continuation_id = ContinuationId(2);
    sba.execution_v4.continuations.insert(
        sba_id,
        mtgml_state::ContinuationRecordV3 {
            id: sba_id,
            created_at_revision: sba.predecessor_v5.revision,
            payload: mtgml_state::ContinuationPayloadV3::MagicSbaGraveyardOrderV1 {
                round_start_revision: sba.predecessor_v5.revision,
                selected_sba_actions: vec![],
                apnap_owners: vec![PlayerId(1)],
                next_owner_index: 0,
                completed_owner_orders: vec![],
            },
        },
    );
    assert_eq!(
        sba.validate(),
        Err(mtgml_state::EngineStatePartsV3Error::ContinuationRecord)
    );
}

#[test]
fn temporary_effect_expiry_is_bound_to_the_authoritative_turn() {
    let mut state = root();
    let id = mtgml_model::EffectInstanceId(1);
    state.predecessor_v5.allocators.next_effect_id = mtgml_model::EffectInstanceId(2);
    let effect = mtgml_state::TemporaryEffectRecord {
        id,
        affected_objects: vec![GameObjectId(1)],
        operation: mtgml_state::TemporaryOperation::GrantKeyword {
            keyword: mtgml_state::TemporaryKeyword::DoubleStrike,
        },
        expiry: mtgml_state::EffectExpiry::UntilEndOfTurn {
            turn_number: state.predecessor_v5.core.turn_number,
        },
        timestamp: Some(mtgml_state::EffectTimestamp {
            creation_revision: state.predecessor_v5.revision,
            operation_ordinal: 0,
        }),
    };
    state.execution_v4.effects.insert(id, effect.clone());
    state.validate().unwrap();

    let mut expired_later = root();
    expired_later.predecessor_v5.allocators.next_effect_id = mtgml_model::EffectInstanceId(2);
    let mut invalid = effect;
    invalid.expiry = mtgml_state::EffectExpiry::UntilEndOfTurn {
        turn_number: expired_later.predecessor_v5.core.turn_number + 1,
    };
    expired_later.execution_v4.effects.insert(id, invalid);
    assert_eq!(
        expired_later.validate(),
        Err(mtgml_state::EngineStatePartsV3Error::TemporaryEffectExpiry)
    );
}

#[test]
fn selected_blight_operand_request_is_not_admitted_until_rules_domain_generation_exists() {
    let (state, _) = staged_blight_activation();
    assert_eq!(
        state.validate(),
        Err(mtgml_state::EngineStatePartsV3Error::ProfileDependentDecisionNotAdmitted)
    );
}

#[test]
fn blight_operand_must_be_a_controlled_battlefield_incarnation() {
    let (mut state, continuation_id) = staged_blight_activation();
    let operand = GameObjectId(4);
    state.predecessor_v5.zones.objects.insert(
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
    state.predecessor_v5.zones.locations.insert(
        operand,
        ZoneLocation {
            zone: ZoneKind::Graveyard,
            player: Some(PlayerId(1)),
            position: ZonePosition::Unordered,
            visibility: VisibilityPartition::Public,
            partition: None,
        },
    );
    state.predecessor_v5.allocators.next_object_id = GameObjectId(5);
    state.card_rules_state.faces.faces.insert(operand, 0);
    let record = state
        .execution_v4
        .continuations
        .get_mut(&continuation_id)
        .unwrap();
    let mtgml_state::ContinuationPayloadV3::NonManaActivation(activation) = &mut record.payload
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
        Err(mtgml_state::EngineStatePartsV3Error::SelectedCostOperand)
    );
}

#[test]
fn tap_reserved_activation_source_is_not_a_mana_source() {
    let (mut state, continuation_id) = staged_blight_activation();
    let record = state
        .execution_v4
        .continuations
        .get_mut(&continuation_id)
        .unwrap();
    let mtgml_state::ContinuationPayloadV3::NonManaActivation(activation) = &mut record.payload
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
        Err(mtgml_state::EngineStatePartsV3Error::ManaPaymentStaging)
    );
}

#[test]
fn provisional_source_output_overflow_fails_closed() {
    let (mut state, _) = staged_blight_activation();
    state
        .card_rules_state
        .mana
        .pools
        .get_mut(&PlayerId(1))
        .unwrap()
        .unrestricted[3] = u32::MAX;
    assert_eq!(
        state.validate(),
        Err(mtgml_state::EngineStatePartsV3Error::ProvisionalManaOverflow)
    );
}

#[test]
fn pending_v4_request_must_match_its_perspective_local_visible_cursor() {
    let (mut state, _) = staged_blight_activation();
    let stale = VisibleSequence(
        state.predecessor_v5.knowledge.players[&PlayerId(1)]
            .next_visible_sequence
            .0
            + 1,
    );
    state
        .execution_v4
        .pending_decision
        .as_mut()
        .unwrap()
        .view_sequence = stale;
    assert_eq!(
        state.validate(),
        Err(mtgml_state::EngineStatePartsV3Error::PendingDecision)
    );
}

#[test]
fn profile_dependent_pending_request_is_fail_closed_at_state_admission() {
    let (state, _) = staged_blight_activation();
    assert_eq!(
        state.validate(),
        Err(mtgml_state::EngineStatePartsV3Error::ProfileDependentDecisionNotAdmitted)
    );
    assert_eq!(
        EngineStatePartsV3::new(
            state.predecessor_v5.clone(),
            state.execution_v4.clone(),
            state.card_rules_state.clone(),
        ),
        Err(mtgml_state::EngineStatePartsV3Error::ProfileDependentDecisionNotAdmitted)
    );
}

#[test]
fn pending_request_purpose_must_match_mana_staging_stage() {
    let (mut state, _) = staged_blight_activation();
    let request = state.execution_v4.pending_decision.as_mut().unwrap();
    request.purpose = mtgml_decision::DecisionPurposeV4::ManaPayment;
    request.candidates = vec![mtgml_decision::AuthoritativeCandidateV4 {
        candidate_id: CandidateIdV1(0),
        visible_intent: mtgml_decision::CandidateIntentV4::SelectManaPayment {
            spent_buckets: [0; 12],
        },
        trusted_binding: mtgml_decision::EngineCandidateBindingV4::SelectManaPayment {
            spent_buckets: [0; 12],
        },
    }];
    assert_eq!(
        state.validate(),
        Err(mtgml_state::EngineStatePartsV3Error::ContinuationRequestMismatch)
    );
}

#[test]
fn pending_finalize_binding_must_name_its_exact_continuation() {
    let (mut state, _) = staged_blight_activation();
    state
        .execution_v4
        .pending_decision
        .as_mut()
        .unwrap()
        .candidates[0]
        .trusted_binding = mtgml_decision::EngineCandidateBindingV4::FinalizeManaProduction {
        continuation: ContinuationId(99),
    };
    assert_eq!(
        state.validate(),
        Err(mtgml_state::EngineStatePartsV3Error::PendingCandidateBinding)
    );
}

#[test]
fn mana_source_candidate_rejects_the_tap_reserved_activation_source() {
    let (mut state, _) = staged_blight_activation();
    let location = state.predecessor_v5.zones.locations[&GameObjectId(3)].clone();
    let opaque_object = mtgml_model::OpaqueObjectId(2);
    let opaque_ability = mtgml_model::OpaqueAbilityId(1);
    let identity = state
        .predecessor_v5
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
        .predecessor_v5
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
    let request = state.execution_v4.pending_decision.as_mut().unwrap();
    request.candidates = vec![mtgml_decision::AuthoritativeCandidateV4 {
        candidate_id: CandidateIdV1(0),
        visible_intent: mtgml_decision::CandidateIntentV4::SelectManaSource {
            source: opaque_object,
            ability: opaque_ability,
            produced_buckets: output,
        },
        trusted_binding: mtgml_decision::EngineCandidateBindingV4::SelectManaSource {
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
        Err(mtgml_state::EngineStatePartsV3Error::PendingCandidateBinding)
    );
}

#[test]
fn profile_dependent_v4_response_is_not_selected_without_rules_domain_generation() {
    let (state, _) = staged_blight_activation();
    let before = state.clone();
    let request = state.execution_v4.pending_decision.as_ref().unwrap();
    let response = mtgml_decision::DecisionResponseV3 {
        schema_version: mtgml_decision::DECISION_RESPONSE_V3_SCHEMA.to_owned(),
        player_decision_id: request.player_decision_id,
        view_sequence: request.view_sequence,
        answer: mtgml_decision::DecisionAnswerV2::SelectOne {
            candidate_id: CandidateIdV1(0),
        },
    };
    assert_eq!(
        state.selected_bindings_v4(PlayerId(1), &response),
        Err(mtgml_state::EngineStatePartsV3Error::ProfileDependentDecisionNotAdmitted)
    );
    assert_eq!(state, before);
}

#[test]
fn v4_parent_decision_must_precede_its_child_identity() {
    let (mut state, _) = staged_blight_activation();
    state
        .execution_v4
        .pending_decision
        .as_mut()
        .unwrap()
        .parent_player_decision_id = Some(PlayerDecisionIdV1(2));
    assert_eq!(
        state.validate(),
        Err(mtgml_state::EngineStatePartsV3Error::PendingCandidateBinding)
    );
}

#[test]
fn trigger_placement_progress_must_follow_apnap_and_completed_prefix() {
    let make_state = |apnap_actors: Vec<PlayerId>, current_actor_index| {
        let mut state = root();
        state.predecessor_v5.allocators.next_object_id = GameObjectId(4);
        state.predecessor_v5.allocators.next_ability_id = AbilityInstanceId(2);
        state.predecessor_v5.allocators.next_trigger_id = mtgml_model::TriggerInstanceId(3);
        let id = ContinuationId(1);
        state.predecessor_v5.allocators.next_continuation_id = ContinuationId(2);
        state.execution_v4.waiting_triggers.insert(
            mtgml_model::TriggerInstanceId(1),
            pending_trigger(1, PlayerId(1)),
        );
        state.execution_v4.waiting_triggers.insert(
            mtgml_model::TriggerInstanceId(2),
            pending_trigger(2, PlayerId(2)),
        );
        state.execution_v4.continuations.insert(
            id,
            mtgml_state::ContinuationRecordV3 {
                id,
                created_at_revision: state.predecessor_v5.revision,
                payload: mtgml_state::ContinuationPayloadV3::TriggerPlacement(
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
        Err(mtgml_state::EngineStatePartsV3Error::TriggerPlacement)
    );

    let skipped_owner = make_state(vec![PlayerId(1), PlayerId(2)], 1);
    assert_eq!(
        skipped_owner.validate(),
        Err(mtgml_state::EngineStatePartsV3Error::TriggerPlacement)
    );
}

#[test]
fn trigger_order_request_is_bound_to_the_current_apnap_group_not_trigger_ids() {
    let mut state = root();
    state.predecessor_v5.allocators.next_object_id = GameObjectId(4);
    state.predecessor_v5.allocators.next_ability_id = AbilityInstanceId(2);
    state.predecessor_v5.allocators.next_trigger_id = mtgml_model::TriggerInstanceId(3);
    state.predecessor_v5.allocators.next_continuation_id = ContinuationId(2);
    state.predecessor_v5.allocators.next_decision_id = DecisionId(3);
    let trigger_source = pending_trigger(1, PlayerId(1)).source_context;
    for (id, drawn_player) in [(1, PlayerId(1)), (2, PlayerId(2))] {
        state.execution_v4.waiting_triggers.insert(
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
    state.execution_v4.continuations.insert(
        continuation_id,
        mtgml_state::ContinuationRecordV3 {
            id: continuation_id,
            created_at_revision: state.predecessor_v5.revision,
            payload: mtgml_state::ContinuationPayloadV3::TriggerPlacement(
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
    state.execution_v4.pending_decision = Some(mtgml_decision::AuthoritativeDecisionRequestV4 {
        decision_id: DecisionId(2),
        player_decision_id: PlayerDecisionIdV1(1),
        state_revision: StateRevision(state.predecessor_v5.revision.0),
        view_sequence: state.predecessor_v5.knowledge.players[&PlayerId(1)].next_visible_sequence,
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
            mtgml_decision::AuthoritativeCandidateV4 {
                candidate_id: CandidateIdV1(0),
                visible_intent: mtgml_decision::CandidateIntentV4::SelectTrigger {
                    trigger: card_drawn(PlayerId(1)),
                },
                trusted_binding: mtgml_decision::EngineCandidateBindingV4::SelectTrigger {
                    trigger: mtgml_model::TriggerInstanceId(1),
                },
            },
            mtgml_decision::AuthoritativeCandidateV4 {
                candidate_id: CandidateIdV1(1),
                visible_intent: mtgml_decision::CandidateIntentV4::SelectTrigger {
                    trigger: card_drawn(PlayerId(2)),
                },
                trusted_binding: mtgml_decision::EngineCandidateBindingV4::SelectTrigger {
                    trigger: mtgml_model::TriggerInstanceId(2),
                },
            },
        ],
    });
    state.validate().unwrap();

    let mut rebound = state.clone();
    rebound
        .execution_v4
        .pending_decision
        .as_mut()
        .unwrap()
        .candidates[0]
        .trusted_binding = mtgml_decision::EngineCandidateBindingV4::SelectTrigger {
        trigger: mtgml_model::TriggerInstanceId(2),
    };
    assert_eq!(
        rebound.validate(),
        Err(mtgml_state::EngineStatePartsV3Error::PendingCandidateBinding)
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
    state
        .predecessor_v5
        .zones
        .objects
        .insert(object, game_object.clone());
    state
        .predecessor_v5
        .zones
        .locations
        .insert(object, stack_location);
    state.predecessor_v5.allocators.next_object_id = GameObjectId(4);
    let payload = StackItemPayload::Spell {
        stack_card_object: object,
        card_definition_id: game_object.card_definition,
        face_key: FaceKey(0),
        semantic_profile_id: CardSemanticProfileId::parse("basic-land@1.0.0").unwrap(),
        modes: vec![],
        targets: vec![],
        cost_facts: Default::default(),
    };
    state.predecessor_v5.zones.stack_records.insert(
        StackObjectId(1),
        StackRecord {
            id: StackObjectId(1),
            controller: PlayerId(1),
            source_object: None,
            source_ability: None,
            payload: Some(payload),
        },
    );
    state
        .predecessor_v5
        .zones
        .stack_order
        .push(StackObjectId(1));
    state.predecessor_v5.allocators.next_stack_object_id = StackObjectId(2);
    assert!(mtgml_state::validate_engine_state(&state.predecessor_v5.clone().into()).is_ok());
    state.validate().unwrap();
    let mut zero_target = state.clone();
    let Some(StackItemPayload::Spell { targets, .. }) = zero_target
        .predecessor_v5
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
        Err(mtgml_state::EngineStatePartsV3Error::TargetReference)
    );
    let legacy: mtgml_state::EngineState = state.predecessor_v5.clone().into();
    assert!(legacy.digest().is_err());
    assert!(mtgml_state::calculate_full_state_digest_v4_historical(&legacy).is_err());
    let old_v6 = EngineStatePartsV2::from_state(&legacy, state.card_rules_state.clone());
    assert_eq!(
        old_v6.validate(),
        Err(mtgml_state::EngineStatePartsV2Error::UnsupportedStackPayload)
    );
}

#[test]
fn activated_stack_payload_keeps_lki_after_its_source_has_left() {
    let mut state = root();
    state.predecessor_v5.allocators.next_object_id = GameObjectId(4);
    state.predecessor_v5.allocators.next_ability_id = AbilityInstanceId(2);
    state.predecessor_v5.allocators.next_stack_object_id = StackObjectId(2);
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
    state.predecessor_v5.zones.stack_records.insert(
        StackObjectId(1),
        StackRecord {
            id: StackObjectId(1),
            controller: PlayerId(1),
            source_object: None,
            source_ability: None,
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
    state
        .predecessor_v5
        .zones
        .stack_order
        .push(StackObjectId(1));
    state.validate().unwrap();
    assert!(matches!(
        &state.predecessor_v5.zones.stack_records[&StackObjectId(1)].payload,
        Some(StackItemPayload::ActivatedAbility { modes, .. })
            if modes == &[mtgml_state::ModeBinding { mode_slot: 0, selected_mode: 1 }]
    ));
}

#[test]
fn triggered_stack_payload_captures_every_closed_event_fact_across_source_departure() {
    let mut state = root();
    state.predecessor_v5.allocators.next_object_id = GameObjectId(4);
    state.predecessor_v5.allocators.next_ability_id = AbilityInstanceId(2);
    state.predecessor_v5.allocators.next_stack_object_id = StackObjectId(2);
    state.predecessor_v5.allocators.next_trigger_id = mtgml_model::TriggerInstanceId(2);
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
            turn_number: state.predecessor_v5.core.turn_number,
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
        state.predecessor_v5.zones.stack_records.insert(
            StackObjectId(1),
            StackRecord {
                id: StackObjectId(1),
                controller: PlayerId(1),
                source_object: None,
                source_ability: None,
                payload: Some(StackItemPayload::TriggeredAbility {
                    originating_trigger: mtgml_model::TriggerInstanceId(1),
                    source_context: source_context.clone(),
                    captured_trigger_context: Box::new(trigger_context),
                    targets: vec![],
                }),
            },
        );
        state.predecessor_v5.zones.stack_order = vec![StackObjectId(1)];
        state.validate().unwrap();
    }
    state.predecessor_v5.zones.stack_records.clear();
    state.predecessor_v5.zones.stack_order.clear();
    state.execution_v4.waiting_triggers.insert(
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
        Err(mtgml_state::EngineStatePartsV3Error::MissingTriggerPlacement)
    );
}

#[test]
fn typed_stack_payload_cannot_be_serialized_through_the_legacy_serde_shape() {
    let payload = StackItemPayload::ActivatedAbility {
        source_context: AbilitySourceContext {
            source: mtgml_state::SourceContext {
                snapshot: mtgml_state::ObjectSnapshot {
                    object: GameObjectId(3),
                    physical_card: None,
                    card_definition: CardDefinitionId(2),
                    owner: PlayerId(1),
                    controller: PlayerId(1),
                    tapped: false,
                    face_down: false,
                    location: ZoneLocation {
                        zone: ZoneKind::Battlefield,
                        player: None,
                        position: ZonePosition::Unordered,
                        visibility: VisibilityPartition::Public,
                        partition: None,
                    },
                },
                face_key: FaceKey(0),
                semantic_profile_id: CardSemanticProfileId::parse("test/profile@1.0.0").unwrap(),
            },
            ability_instance_id: mtgml_model::AbilityInstanceId(1),
            ability_key: mtgml_card_ir::AbilityKey(1),
        },
        modes: vec![],
        targets: vec![],
        cost_facts: Default::default(),
    };
    let record = StackRecord {
        id: StackObjectId(1),
        controller: PlayerId(1),
        source_object: None,
        source_ability: None,
        payload: Some(payload),
    };
    assert!(serde_json::to_value(record).is_err());

    let mut unknown_successor_field = serde_json::json!({
        "id": 1,
        "controller": 1,
        "payload": {"kind": "spell"}
    });
    unknown_successor_field["source_object"] = serde_json::Value::Null;
    unknown_successor_field["source_ability"] = serde_json::Value::Null;
    assert!(serde_json::from_value::<StackRecord>(unknown_successor_field).is_err());

    let legacy_record = StackRecord {
        id: StackObjectId(1),
        controller: PlayerId(1),
        source_object: None,
        source_ability: None,
        payload: None,
    };
    let legacy_value = serde_json::to_value(legacy_record).unwrap();
    assert!(legacy_value.get("payload").is_none());
    assert!(legacy_value.get("source_object").is_none());
    assert!(legacy_value.get("source_ability").is_none());
}
