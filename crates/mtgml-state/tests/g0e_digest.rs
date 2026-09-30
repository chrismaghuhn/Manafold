use mtgml_card_ir::{CardSemanticProfileId, FaceKey};
use mtgml_model::{
    CardDefinitionId, GameObjectId, PhysicalCardId, PlayerId, StackObjectId, ZoneKind,
};
use mtgml_random::RootSeed256;
use mtgml_state::{
    calculate_full_state_digest_v7, canonical_state_bytes_v7, construct_synthetic_engine_state,
    full_state_digest_v7_from_payload, ActionCostFacts, CardRulesAuthoritativeStateV1,
    EffectExpiry, EngineStatePartsV3, ExecutionStateV4, ManaCost, ManaPaymentStage,
    ManaPaymentStaging, ManaSourceActivation, ManaSourceActivationCost,
    NonManaActivationContinuation, NonManaActivationStage, ReservedNonManaCost,
    SelectedCostOperand, StackItemPayload, StackRecord, StateDeltaV3, SyntheticResetInputs,
    SyntheticV4Setup, TemporaryEffectRecord, TemporaryOperation, VisibilityPartition, ZoneLocation,
    ZonePosition, FULL_STATE_DIGEST_DOMAIN_V7, FULL_STATE_DIGEST_INPUT_SCHEMA_V7,
};

fn state() -> EngineStatePartsV3 {
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
    EngineStatePartsV3::new(
        predecessor.parts(),
        ExecutionStateV4::default(),
        card_rules_state,
    )
    .unwrap()
}

fn staged_activation_state() -> EngineStatePartsV3 {
    let mut state = state();
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
    state.predecessor_v5.allocators.next_ability_id = mtgml_model::AbilityInstanceId(3);
    state.predecessor_v5.allocators.next_continuation_id = mtgml_model::ContinuationId(2);
    state.predecessor_v5.allocators.next_decision_id = mtgml_model::DecisionId(3);
    state.card_rules_state.abilities.by_instance.insert(
        mtgml_model::AbilityInstanceId(1),
        mtgml_state::AbilityAuthorityV1 {
            source: source_object,
            ability_key: 4,
        },
    );
    state.card_rules_state.abilities.by_instance.insert(
        mtgml_model::AbilityInstanceId(2),
        mtgml_state::AbilityAuthorityV1 {
            source: GameObjectId(1),
            ability_key: 9,
        },
    );
    for object in state.predecessor_v5.zones.objects.keys().copied() {
        state.card_rules_state.faces.faces.insert(object, 0);
    }
    let continuation_id = mtgml_model::ContinuationId(1);
    let action_cost_facts = ActionCostFacts {
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
                    source_ability_instance: mtgml_model::AbilityInstanceId(1),
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
                            source_ability_instance: mtgml_model::AbilityInstanceId(2),
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
        decision_id: mtgml_model::DecisionId(2),
        player_decision_id: mtgml_model::PlayerDecisionIdV1(1),
        state_revision: state.predecessor_v5.revision,
        view_sequence,
        actor: PlayerId(1),
        visibility: mtgml_decision::DecisionVisibility::ActingPlayerOnly,
        decision_domain_v2: mtgml_decision::DecisionDomainV2::ChooseOne,
        purpose: mtgml_decision::DecisionPurposeV4::ManaProductionChoice,
        parent_player_decision_id: None,
        continuation_id: Some(continuation_id),
        candidates: vec![mtgml_decision::AuthoritativeCandidateV4 {
            candidate_id: mtgml_model::CandidateIdV1(0),
            visible_intent: mtgml_decision::CandidateIntentV4::FinalizeManaProduction,
            trusted_binding: mtgml_decision::EngineCandidateBindingV4::FinalizeManaProduction {
                continuation: continuation_id,
            },
        }],
    });
    state
}

fn insert_spell(state: &mut EngineStatePartsV3, object_id: u64, stack_id: u64) {
    let object = GameObjectId(object_id);
    let definition = CardDefinitionId(object_id);
    state.predecessor_v5.zones.objects.insert(
        object,
        mtgml_state::GameObject {
            id: object,
            physical_card: Some(PhysicalCardId(object_id)),
            card_definition: definition,
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
    state.predecessor_v5.allocators.next_object_id.0 = state
        .predecessor_v5
        .allocators
        .next_object_id
        .0
        .max(object_id + 1);
    state.predecessor_v5.allocators.next_stack_object_id.0 = state
        .predecessor_v5
        .allocators
        .next_stack_object_id
        .0
        .max(stack_id + 1);
    state.predecessor_v5.zones.stack_records.insert(
        StackObjectId(stack_id),
        StackRecord {
            id: StackObjectId(stack_id),
            controller: PlayerId(1),
            source_object: None,
            source_ability: None,
            payload: Some(StackItemPayload::Spell {
                stack_card_object: object,
                card_definition_id: definition,
                face_key: FaceKey(0),
                semantic_profile_id: CardSemanticProfileId::parse("test/spell@1.0.0").unwrap(),
                modes: vec![],
                targets: vec![],
                cost_facts: Default::default(),
            }),
        },
    );
}

#[test]
fn v7_digest_has_the_frozen_14_field_domain_envelope() {
    let state = state();
    let payload = canonical_state_bytes_v7(&state).unwrap();
    let value = mtgml_persistence::cbor::decode_canonical(&payload).unwrap();
    let mtgml_persistence::cbor::Value::Array(fields) = value else {
        panic!("V7 input must be an array")
    };
    assert_eq!(fields.len(), 14);
    assert_eq!(
        fields[0],
        mtgml_persistence::cbor::Value::Text(FULL_STATE_DIGEST_INPUT_SCHEMA_V7.to_owned())
    );
    assert_eq!(
        fields[1],
        mtgml_persistence::cbor::Value::Text(FULL_STATE_DIGEST_DOMAIN_V7.to_owned())
    );
    assert!(matches!(
        &fields[4],
        mtgml_persistence::cbor::Value::Array(zones)
            if zones.len() == 6
                && zones[0] == mtgml_persistence::cbor::Value::Text("zones_v2".to_owned())
    ));
    assert!(matches!(
        &fields[6],
        mtgml_persistence::cbor::Value::Array(execution)
            if execution.len() == 6
                && execution[0] == mtgml_persistence::cbor::Value::Text("execution_v4".to_owned())
    ));
    assert_eq!(
        calculate_full_state_digest_v7(&state).unwrap().as_str(),
        "9e80644042e5316842cac990ee05371faf21e900b96d28ae3ff59d43193a2489"
    );
}

#[test]
fn v7_detached_g0c_identity_fixture_matches_its_domain_kat() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../persistence/golden/full-state-digest-v7-kat.v1.json"
    ))
    .unwrap();
    let hex = fixture["canonical_payload_hex"].as_str().unwrap();
    let payload = hex
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let pair = std::str::from_utf8(pair).unwrap();
            u8::from_str_radix(pair, 16).unwrap()
        })
        .collect::<Vec<_>>();
    let expected =
        mtgml_model::FullStateDigestV7::parse(fixture["expected_digest"].as_str().unwrap())
            .unwrap();
    assert_eq!(
        full_state_digest_v7_from_payload(&payload).unwrap(),
        expected
    );
}

#[test]
fn v7_digest_changes_when_a_shared_card_rule_value_changes() {
    let state = state();
    let mut changed = state.clone();
    changed
        .card_rules_state
        .turn_history
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .land_plays_used = 1;
    assert_ne!(
        calculate_full_state_digest_v7(&state).unwrap(),
        calculate_full_state_digest_v7(&changed).unwrap()
    );
}

#[test]
fn v7_digest_binds_typed_stack_payload_and_mode_order() {
    let mut state = state();
    let object = GameObjectId(50);
    let definition = CardDefinitionId(50);
    state.predecessor_v5.zones.objects.insert(
        object,
        mtgml_state::GameObject {
            id: object,
            physical_card: Some(PhysicalCardId(50)),
            card_definition: definition,
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
    state.predecessor_v5.allocators.next_object_id = GameObjectId(51);
    state.predecessor_v5.allocators.next_stack_object_id = StackObjectId(2);
    state.predecessor_v5.zones.stack_records.insert(
        StackObjectId(1),
        StackRecord {
            id: StackObjectId(1),
            controller: PlayerId(1),
            source_object: None,
            source_ability: None,
            payload: Some(StackItemPayload::Spell {
                stack_card_object: object,
                card_definition_id: definition,
                face_key: FaceKey(0),
                semantic_profile_id: CardSemanticProfileId::parse("test/spell@1.0.0").unwrap(),
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

    let original = calculate_full_state_digest_v7(&state).unwrap();
    if let Some(StackItemPayload::Spell { modes, .. }) = state
        .predecessor_v5
        .zones
        .stack_records
        .get_mut(&StackObjectId(1))
        .and_then(|record| record.payload.as_mut())
    {
        modes[0].selected_mode = 2;
    }
    assert_ne!(original, calculate_full_state_digest_v7(&state).unwrap());
}

#[test]
fn v7_digest_ignores_stack_map_insertion_order_but_binds_stack_order() {
    let mut first = state();
    insert_spell(&mut first, 50, 1);
    insert_spell(&mut first, 51, 2);
    first
        .predecessor_v5
        .zones
        .stack_order
        .extend([StackObjectId(1), StackObjectId(2)]);
    first.validate().unwrap();

    let mut reverse_insert = state();
    insert_spell(&mut reverse_insert, 51, 2);
    insert_spell(&mut reverse_insert, 50, 1);
    reverse_insert
        .predecessor_v5
        .zones
        .stack_order
        .extend([StackObjectId(1), StackObjectId(2)]);
    reverse_insert.validate().unwrap();
    assert_eq!(
        calculate_full_state_digest_v7(&first).unwrap(),
        calculate_full_state_digest_v7(&reverse_insert).unwrap()
    );

    reverse_insert.predecessor_v5.zones.stack_order.swap(0, 1);
    assert_ne!(
        calculate_full_state_digest_v7(&first).unwrap(),
        calculate_full_state_digest_v7(&reverse_insert).unwrap()
    );
}

#[test]
fn v7_digest_binds_temporary_effect_operation_and_lifetime_record() {
    let mut state = state();
    let object = GameObjectId(50);
    state.predecessor_v5.zones.objects.insert(
        object,
        mtgml_state::GameObject {
            id: object,
            physical_card: None,
            card_definition: CardDefinitionId(50),
            owner: PlayerId(1),
            controller: PlayerId(1),
            tapped: false,
            face_down: false,
        },
    );
    state.predecessor_v5.zones.locations.insert(
        object,
        ZoneLocation {
            zone: ZoneKind::Battlefield,
            player: None,
            position: ZonePosition::Unordered,
            visibility: VisibilityPartition::Public,
            partition: None,
        },
    );
    state.predecessor_v5.allocators.next_object_id = GameObjectId(51);
    state.predecessor_v5.allocators.next_effect_id = mtgml_model::EffectInstanceId(2);
    state.execution_v4.effects.insert(
        mtgml_model::EffectInstanceId(1),
        TemporaryEffectRecord {
            id: mtgml_model::EffectInstanceId(1),
            affected_objects: vec![object],
            operation: TemporaryOperation::PowerToughnessDelta {
                power: 1,
                toughness: 0,
            },
            expiry: EffectExpiry::UntilEndOfTurn {
                turn_number: state.predecessor_v5.core.turn_number,
            },
            timestamp: None,
        },
    );
    state.validate().unwrap();
    let original = calculate_full_state_digest_v7(&state).unwrap();

    let mut changed = state.clone();
    changed
        .execution_v4
        .effects
        .get_mut(&mtgml_model::EffectInstanceId(1))
        .unwrap()
        .operation = TemporaryOperation::PowerToughnessDelta {
        power: 2,
        toughness: 0,
    };
    assert_ne!(original, calculate_full_state_digest_v7(&changed).unwrap());
}

#[test]
fn v7_typed_digest_rejects_profile_dependent_pending_activation() {
    let state = staged_activation_state();
    assert!(state.validate().is_err());
    assert!(canonical_state_bytes_v7(&state).is_err());
    assert!(calculate_full_state_digest_v7(&state).is_err());
}

#[test]
fn v7_digest_binds_pending_trigger_context_and_exact_candidate_binding() {
    let mut state = state();
    state.predecessor_v5.allocators.next_object_id = GameObjectId(4);
    state.predecessor_v5.allocators.next_ability_id = mtgml_model::AbilityInstanceId(2);
    state.predecessor_v5.allocators.next_trigger_id = mtgml_model::TriggerInstanceId(3);
    state.predecessor_v5.allocators.next_continuation_id = mtgml_model::ContinuationId(2);
    state.predecessor_v5.allocators.next_decision_id = mtgml_model::DecisionId(3);
    let source = mtgml_state::AbilitySourceContext {
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
        ability_instance_id: mtgml_model::AbilityInstanceId(1),
        ability_key: mtgml_card_ir::AbilityKey(4),
    };
    for (id, drawn_player) in [(1, PlayerId(1)), (2, PlayerId(2))] {
        let id = mtgml_model::TriggerInstanceId(id);
        state.execution_v4.waiting_triggers.insert(
            id,
            mtgml_state::PendingTriggerRecord {
                id,
                controller: PlayerId(1),
                source_context: source.clone(),
                trigger_context: mtgml_state::TriggerEventSnapshot::CardDrawn {
                    player: drawn_player,
                },
                target_timing: mtgml_state::TriggerTargetTiming::NoTargets,
            },
        );
    }
    let continuation_id = mtgml_model::ContinuationId(1);
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
                        first_decision_id: mtgml_model::PlayerDecisionIdV1(1),
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
        decision_id: mtgml_model::DecisionId(2),
        player_decision_id: mtgml_model::PlayerDecisionIdV1(1),
        state_revision: state.predecessor_v5.revision,
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
                candidate_id: mtgml_model::CandidateIdV1(0),
                visible_intent: mtgml_decision::CandidateIntentV4::SelectTrigger {
                    trigger: card_drawn(PlayerId(1)),
                },
                trusted_binding: mtgml_decision::EngineCandidateBindingV4::SelectTrigger {
                    trigger: mtgml_model::TriggerInstanceId(1),
                },
            },
            mtgml_decision::AuthoritativeCandidateV4 {
                candidate_id: mtgml_model::CandidateIdV1(1),
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
    let original = calculate_full_state_digest_v7(&state).unwrap();
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
    assert!(rebound.validate().is_err());
    assert!(calculate_full_state_digest_v7(&rebound).is_err());
    state
        .execution_v4
        .waiting_triggers
        .get_mut(&mtgml_model::TriggerInstanceId(1))
        .unwrap()
        .trigger_context = mtgml_state::TriggerEventSnapshot::CardDrawn {
        player: PlayerId(2),
    };
    state
        .execution_v4
        .waiting_triggers
        .get_mut(&mtgml_model::TriggerInstanceId(2))
        .unwrap()
        .trigger_context = mtgml_state::TriggerEventSnapshot::CardDrawn {
        player: PlayerId(1),
    };
    let request = state.execution_v4.pending_decision.as_mut().unwrap();
    request.candidates[0].trusted_binding =
        mtgml_decision::EngineCandidateBindingV4::SelectTrigger {
            trigger: mtgml_model::TriggerInstanceId(2),
        };
    request.candidates[1].trusted_binding =
        mtgml_decision::EngineCandidateBindingV4::SelectTrigger {
            trigger: mtgml_model::TriggerInstanceId(1),
        };
    assert_ne!(original, calculate_full_state_digest_v7(&state).unwrap());
}

#[test]
fn v7_typed_writer_rejects_profile_dependent_pending_activation() {
    let state = staged_activation_state();
    assert!(canonical_state_bytes_v7(&state).is_err());
    assert!(calculate_full_state_digest_v7(&state).is_err());
}

#[test]
fn state_delta_v3_rejects_profile_dependent_pending_requests() {
    let before = staged_activation_state();
    let mut after = before.clone();
    after.predecessor_v5.revision =
        mtgml_model::StateRevision(before.predecessor_v5.revision.0.checked_add(1).unwrap());
    assert!(StateDeltaV3::between(&before, &after, vec![]).is_err());
    assert!(before.validate().is_err());
    assert!(after.validate().is_err());
}

#[test]
fn state_delta_v3_reapplies_the_exact_successor_replacement() {
    let before = state();
    let mut after = before.clone();
    after.predecessor_v5.revision =
        mtgml_model::StateRevision(after.predecessor_v5.revision.0.checked_add(1).unwrap());
    after.validate().unwrap();
    let delta = StateDeltaV3::between(&before, &after, vec![]).unwrap();
    assert_eq!(delta.apply(&before).unwrap(), after);

    let unchanged = before.clone();
    let mut stale = delta;
    stale.before_digest = mtgml_model::FullStateDigestV7::from_digest_bytes([0; 32]);
    assert_eq!(
        stale.apply(&before),
        Err(mtgml_state::DeltaApplicationV3Error::BeforeMismatch)
    );
    assert_eq!(before, unchanged);
}

#[test]
fn state_delta_v3_requires_exact_revision_step_and_covers_life_mutation() {
    let before = state();
    let mut unchanged_revision = before.clone();
    assert_eq!(
        StateDeltaV3::between(&before, &unchanged_revision, vec![]),
        Err(mtgml_state::DeltaApplicationV3Error::RevisionProgression)
    );

    unchanged_revision.predecessor_v5.revision =
        mtgml_model::StateRevision(before.predecessor_v5.revision.0.checked_add(1).unwrap());
    unchanged_revision
        .predecessor_v5
        .core
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .life -= 1;
    unchanged_revision.validate().unwrap();
    assert_eq!(
        StateDeltaV3::between(&before, &unchanged_revision, vec![]),
        Err(mtgml_state::DeltaApplicationV3Error::UncoveredMutation)
    );
}

#[test]
fn state_delta_v3_rejects_unrelated_turn_history_mutation_even_with_cast_operation() {
    let before = state();
    let mut after = before.clone();
    after.predecessor_v5.revision =
        mtgml_model::StateRevision(before.predecessor_v5.revision.0.checked_add(1).unwrap());
    let object = *after.predecessor_v5.zones.objects.keys().next().unwrap();
    after
        .card_rules_state
        .turn_history
        .target_occurrences
        .insert((object, PlayerId(1)));
    after.validate().unwrap();

    let unrelated_cast = mtgml_state::SemanticDeltaOperationV3::SpellCast {
        stack_object: StackObjectId(1),
        spell_object: object,
        card_definition: CardDefinitionId(1),
        face_key: FaceKey(0),
        semantic_profile_id: CardSemanticProfileId::parse("test/spell@1.0.0").unwrap(),
        is_creature_spell: false,
        cost_facts: Default::default(),
    };
    assert_eq!(
        StateDeltaV3::between(&before, &after, vec![unrelated_cast]),
        Err(mtgml_state::DeltaApplicationV3Error::UncoveredMutation)
    );
}

#[test]
fn state_delta_v3_rejects_cast_event_against_preexisting_stack_item() {
    let mut before = state();
    insert_spell(&mut before, 50, 1);
    before
        .predecessor_v5
        .zones
        .stack_order
        .push(StackObjectId(1));
    before.validate().unwrap();
    let mut after = before.clone();
    after.predecessor_v5.revision =
        mtgml_model::StateRevision(before.predecessor_v5.revision.0.checked_add(1).unwrap());
    let operation = mtgml_state::SemanticDeltaOperationV3::SpellCast {
        stack_object: StackObjectId(1),
        spell_object: GameObjectId(50),
        card_definition: CardDefinitionId(50),
        face_key: FaceKey(0),
        semantic_profile_id: CardSemanticProfileId::parse("test/spell@1.0.0").unwrap(),
        is_creature_spell: false,
        cost_facts: Default::default(),
    };
    assert_eq!(
        StateDeltaV3::between(&before, &after, vec![operation]),
        Err(mtgml_state::DeltaApplicationV3Error::UncoveredMutation)
    );
}

#[test]
fn state_delta_v3_rejects_duplicate_cast_occurrences_for_one_stack_creation() {
    let before = state();
    let mut after = before.clone();
    insert_spell(&mut after, 50, 1);
    after
        .predecessor_v5
        .zones
        .stack_order
        .push(StackObjectId(1));
    after.predecessor_v5.revision =
        mtgml_model::StateRevision(before.predecessor_v5.revision.0.checked_add(1).unwrap());
    after
        .card_rules_state
        .turn_history
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .spells_cast_total = 2;
    after
        .card_rules_state
        .turn_history
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .noncreature_spells_cast = 2;
    after.validate().unwrap();
    let payload = after.predecessor_v5.zones.stack_records[&StackObjectId(1)]
        .payload
        .clone()
        .unwrap();
    let spell = || mtgml_state::SemanticDeltaOperationV3::SpellCast {
        stack_object: StackObjectId(1),
        spell_object: GameObjectId(50),
        card_definition: CardDefinitionId(50),
        face_key: FaceKey(0),
        semantic_profile_id: CardSemanticProfileId::parse("test/spell@1.0.0").unwrap(),
        is_creature_spell: false,
        cost_facts: Default::default(),
    };
    let operations = vec![
        mtgml_state::SemanticDeltaOperationV3::ObjectEntered {
            old_object: None,
            new_object: GameObjectId(50),
            from_zone: ZoneKind::Hand,
            to_zone: ZoneKind::Stack,
            tapped: false,
            face: 0,
        },
        mtgml_state::SemanticDeltaOperationV3::StackItemCreated {
            stack_object: StackObjectId(1),
            payload: Box::new(payload),
        },
        mtgml_state::SemanticDeltaOperationV3::StackOrderChanged {
            from: vec![],
            to: vec![StackObjectId(1)],
        },
        spell(),
        spell(),
    ];
    assert_eq!(
        StateDeltaV3::between(&before, &after, operations),
        Err(mtgml_state::DeltaApplicationV3Error::UncoveredMutation)
    );
}

#[test]
fn state_delta_v3_requires_a_cast_operation_for_a_new_spell_stack_item() {
    let before = state();
    let mut after = before.clone();
    insert_spell(&mut after, 50, 1);
    after
        .predecessor_v5
        .zones
        .stack_order
        .push(StackObjectId(1));
    after.predecessor_v5.revision =
        mtgml_model::StateRevision(before.predecessor_v5.revision.0.checked_add(1).unwrap());
    after.validate().unwrap();
    let payload = after.predecessor_v5.zones.stack_records[&StackObjectId(1)]
        .payload
        .clone()
        .unwrap();
    let operations = vec![
        mtgml_state::SemanticDeltaOperationV3::ObjectEntered {
            old_object: None,
            new_object: GameObjectId(50),
            from_zone: ZoneKind::Hand,
            to_zone: ZoneKind::Stack,
            tapped: false,
            face: 0,
        },
        mtgml_state::SemanticDeltaOperationV3::StackItemCreated {
            stack_object: StackObjectId(1),
            payload: Box::new(payload),
        },
        mtgml_state::SemanticDeltaOperationV3::StackOrderChanged {
            from: vec![],
            to: vec![StackObjectId(1)],
        },
    ];
    assert_eq!(
        StateDeltaV3::between(&before, &after, operations),
        Err(mtgml_state::DeltaApplicationV3Error::UncoveredMutation)
    );
}

#[test]
fn state_delta_v3_rejects_duplicate_stack_creation_for_triggered_payload() {
    let mut before = state();
    before.predecessor_v5.allocators.next_object_id = GameObjectId(4);
    before.predecessor_v5.allocators.next_ability_id = mtgml_model::AbilityInstanceId(2);
    before.predecessor_v5.allocators.next_trigger_id = mtgml_model::TriggerInstanceId(2);
    let payload = StackItemPayload::TriggeredAbility {
        originating_trigger: mtgml_model::TriggerInstanceId(1),
        source_context: mtgml_state::AbilitySourceContext {
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
            ability_instance_id: mtgml_model::AbilityInstanceId(1),
            ability_key: mtgml_card_ir::AbilityKey(4),
        },
        captured_trigger_context: Box::new(mtgml_state::TriggerEventSnapshot::CardDrawn {
            player: PlayerId(1),
        }),
        targets: vec![],
    };
    before.validate().unwrap();

    let mut after = before.clone();
    after.predecessor_v5.allocators.next_stack_object_id = StackObjectId(2);
    after.predecessor_v5.zones.stack_records.insert(
        StackObjectId(1),
        StackRecord {
            id: StackObjectId(1),
            controller: PlayerId(1),
            source_object: None,
            source_ability: None,
            payload: Some(payload.clone()),
        },
    );
    after
        .predecessor_v5
        .zones
        .stack_order
        .push(StackObjectId(1));
    after.predecessor_v5.revision =
        mtgml_model::StateRevision(before.predecessor_v5.revision.0.checked_add(1).unwrap());
    after.validate().unwrap();

    let operations = vec![
        mtgml_state::SemanticDeltaOperationV3::StackItemCreated {
            stack_object: StackObjectId(1),
            payload: Box::new(payload.clone()),
        },
        mtgml_state::SemanticDeltaOperationV3::StackItemCreated {
            stack_object: StackObjectId(1),
            payload: Box::new(payload.clone()),
        },
        mtgml_state::SemanticDeltaOperationV3::TriggerPlaced {
            trigger: mtgml_model::TriggerInstanceId(1),
            stack_object: StackObjectId(1),
            payload: Box::new(payload),
        },
        mtgml_state::SemanticDeltaOperationV3::StackOrderChanged {
            from: vec![],
            to: vec![StackObjectId(1)],
        },
    ];
    assert_eq!(
        StateDeltaV3::between(&before, &after, operations),
        Err(mtgml_state::DeltaApplicationV3Error::UncoveredMutation)
    );
}
