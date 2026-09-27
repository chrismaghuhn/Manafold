use mtgml_card_ir::{AbilityKey, CardSemanticProfileId, FaceKey};
use mtgml_model::{
    AbilityInstanceId, CardDefinitionId, GameObjectId, PlayerId, StackObjectId, TriggerInstanceId,
};
use mtgml_state::{
    AbilitySourceContext, ActionCostFacts, CostFacts, CostRoute, DamageKind, DamageRecipient,
    EffectExpiry, LifeChangeCause, ManaCost, ManaPaymentStage, ManaPaymentStaging,
    ManaSourceActivation, ManaSourceActivationCost, ModeBinding, ReservedNonManaCost,
    SelectedCostOperand, SourceContext, StackItemPayload, StackResolutionContinuation,
    StackResolutionStage, TargetBinding, TargetRef, TemporaryEffectRecord, TemporaryKeyword,
    TemporaryOperation, TriggerEventSnapshot,
};

fn snapshot(zone: mtgml_model::ZoneKind) -> mtgml_state::ObjectSnapshot {
    mtgml_state::ObjectSnapshot {
        object: GameObjectId(10),
        physical_card: None,
        card_definition: CardDefinitionId(4),
        owner: PlayerId(0),
        controller: PlayerId(0),
        tapped: false,
        face_down: false,
        location: mtgml_state::ZoneLocation {
            zone,
            player: Some(PlayerId(0)),
            position: mtgml_state::ZonePosition::Unordered,
            visibility: mtgml_state::VisibilityPartition::Public,
            partition: None,
        },
    }
}

fn profile() -> CardSemanticProfileId {
    CardSemanticProfileId::parse("test/profile@1.0.0").unwrap()
}

fn ability_source() -> AbilitySourceContext {
    AbilitySourceContext {
        source: SourceContext {
            snapshot: snapshot(mtgml_model::ZoneKind::Battlefield),
            face_key: FaceKey(0),
            semantic_profile_id: profile(),
        },
        ability_instance_id: AbilityInstanceId(1),
        ability_key: AbilityKey(0),
    }
}

#[test]
fn typed_stack_payload_keeps_source_context_without_generic_resolution_blob() {
    let payload = StackItemPayload::TriggeredAbility {
        originating_trigger: TriggerInstanceId(1),
        source_context: ability_source(),
        captured_trigger_context: TriggerEventSnapshot::TargetBecame {
            actor: PlayerId(1),
            source_stack_item: StackObjectId(3),
            target: TargetRef::Object(GameObjectId(10)),
        },
        targets: vec![TargetBinding {
            target_slot: 0,
            target: TargetRef::StackItem(StackObjectId(3)),
        }],
    };

    assert!(matches!(payload, StackItemPayload::TriggeredAbility { .. }));

    let activated = StackItemPayload::ActivatedAbility {
        source_context: ability_source(),
        targets: vec![],
        cost_facts: CostFacts {
            selected_route: Some(CostRoute::Alternative {
                profile_local_route_id: 1,
            }),
            paid_additional_cost_ids: vec![2],
        },
    };
    assert!(matches!(
        activated,
        StackItemPayload::ActivatedAbility { .. }
    ));

    let spell = StackItemPayload::Spell {
        stack_card_object: GameObjectId(11),
        card_definition_id: CardDefinitionId(5),
        face_key: FaceKey(0),
        semantic_profile_id: profile(),
        modes: vec![ModeBinding {
            mode_slot: 0,
            selected_mode: 1,
        }],
        targets: vec![],
        cost_facts: CostFacts::default(),
    };
    assert!(matches!(spell, StackItemPayload::Spell { .. }));
}

#[test]
fn trigger_event_vocabulary_captures_cast_combat_saga_damage_and_life_facts() {
    let facts = [
        TriggerEventSnapshot::SpellCast {
            actor: PlayerId(0),
            stack_item: StackObjectId(2),
            spell: SourceContext {
                snapshot: snapshot(mtgml_model::ZoneKind::Stack),
                face_key: FaceKey(0),
                semantic_profile_id: profile(),
            },
            is_creature_spell: false,
            cost_facts: CostFacts {
                selected_route: Some(CostRoute::Normal),
                paid_additional_cost_ids: vec![],
            },
        },
        TriggerEventSnapshot::AbilityActivated {
            actor: PlayerId(0),
            stack_item: StackObjectId(4),
            source: ability_source(),
            targets: vec![],
            cost_facts: CostFacts::default(),
        },
        TriggerEventSnapshot::TargetBecame {
            actor: PlayerId(0),
            source_stack_item: StackObjectId(5),
            target: TargetRef::Object(GameObjectId(10)),
        },
        TriggerEventSnapshot::ObjectEntered {
            object: snapshot(mtgml_model::ZoneKind::Battlefield),
        },
        TriggerEventSnapshot::ObjectLeftOrDied {
            last_known: snapshot(mtgml_model::ZoneKind::Battlefield),
            destination: mtgml_state::ZoneLocation {
                zone: mtgml_model::ZoneKind::Graveyard,
                player: Some(PlayerId(0)),
                position: mtgml_state::ZonePosition::Unordered,
                visibility: mtgml_state::VisibilityPartition::Public,
                partition: None,
            },
        },
        TriggerEventSnapshot::BeginningOfCombat {
            active_player: PlayerId(0),
            turn_number: 1,
        },
        TriggerEventSnapshot::AttackDeclared {
            controller: PlayerId(0),
            attackers: vec![],
        },
        TriggerEventSnapshot::CardDrawn {
            player: PlayerId(0),
        },
        TriggerEventSnapshot::CounterChanged {
            object: GameObjectId(10),
            kind: mtgml_state::CounterKindV1::Lore,
            before: 0,
            after: 1,
        },
        TriggerEventSnapshot::DamageApplied {
            source: Some(SourceContext {
                snapshot: snapshot(mtgml_model::ZoneKind::Stack),
                face_key: FaceKey(0),
                semantic_profile_id: profile(),
            }),
            recipient: DamageRecipient::Player(PlayerId(1)),
            amount: 3,
            damage_kind: DamageKind::Noncombat,
        },
        TriggerEventSnapshot::LifeChanged {
            player: PlayerId(1),
            before: 20,
            after: 17,
            cause: LifeChangeCause::Damage,
        },
    ];

    assert_eq!(facts.len(), 11);
    assert!(facts
        .iter()
        .any(|fact| matches!(fact, TriggerEventSnapshot::BeginningOfCombat { .. })));
    assert!(facts
        .iter()
        .any(|fact| matches!(fact, TriggerEventSnapshot::CounterChanged { .. })));
    let _mode = ModeBinding {
        mode_slot: 0,
        selected_mode: 0,
    };
}

#[test]
fn temporary_operation_vocabulary_is_bounded_to_locked_operations() {
    let effects = [
        TemporaryEffectRecord {
            id: mtgml_model::EffectInstanceId(1),
            affected_objects: vec![GameObjectId(10)],
            operation: TemporaryOperation::PowerToughnessDelta {
                power: 1,
                toughness: 1,
            },
            expiry: EffectExpiry::UntilEndOfTurn { turn_number: 1 },
            timestamp: None,
        },
        TemporaryEffectRecord {
            id: mtgml_model::EffectInstanceId(2),
            affected_objects: vec![GameObjectId(10)],
            operation: TemporaryOperation::GrantKeyword {
                keyword: TemporaryKeyword::Haste,
            },
            expiry: EffectExpiry::UntilEndOfTurn { turn_number: 1 },
            timestamp: None,
        },
        TemporaryEffectRecord {
            id: mtgml_model::EffectInstanceId(3),
            affected_objects: vec![GameObjectId(10)],
            operation: TemporaryOperation::GrantKeyword {
                keyword: TemporaryKeyword::DoubleStrike,
            },
            expiry: EffectExpiry::UntilEndOfTurn { turn_number: 1 },
            timestamp: None,
        },
    ];

    assert_eq!(effects.len(), 3);
}

#[test]
fn cost_facts_and_source_payment_staging_have_one_typed_owner() {
    let facts = ActionCostFacts {
        mana_cost: Some(ManaCost {
            colored_wubrg_counts: [0, 0, 0, 1, 0],
            colorless_count: 0,
            generic_count: 0,
        }),
        reserved_nonmana_costs: vec![ReservedNonManaCost::TapSource],
        selected_cost_operands: vec![SelectedCostOperand::PutCounters {
            object: GameObjectId(11),
            counter_kind: mtgml_state::CounterKindV1::MinusOneMinusOne,
            count: 2,
        }],
    };
    let staging = ManaPaymentStaging {
        stage: ManaPaymentStage::SelectingSources,
        mana_source_activations: vec![ManaSourceActivation {
            source_object: GameObjectId(12),
            source_ability_instance: AbilityInstanceId(3),
            ability_key: AbilityKey(1),
            semantic_profile_id: profile(),
            activation_cost_receipt: ManaSourceActivationCost::TapSource,
            produced_buckets: [0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0],
        }],
    };

    assert_eq!(facts.mana_cost.unwrap().colored_wubrg_counts[3], 1);
    assert_eq!(facts.selected_cost_operands.len(), 1);
    assert_eq!(staging.mana_source_activations.len(), 1);
    assert_eq!(staging.mana_source_activations[0].produced_buckets[3], 1);
}

#[test]
fn paused_stack_payment_has_one_authoritative_stage_owner() {
    let waiting_for_ward_choice = StackResolutionContinuation {
        id: mtgml_model::ContinuationId(1),
        resolving_stack_object: StackObjectId(3),
        stage: StackResolutionStage::AwaitingOptionalPayment,
        action_cost_facts: None,
        mana_payment_staging: None,
    };
    assert!(matches!(
        waiting_for_ward_choice.stage,
        StackResolutionStage::AwaitingOptionalPayment
    ));

    let paying_ward_cost = StackResolutionContinuation {
        id: mtgml_model::ContinuationId(1),
        resolving_stack_object: StackObjectId(3),
        stage: StackResolutionStage::PayingMana,
        action_cost_facts: Some(ActionCostFacts {
            mana_cost: Some(ManaCost {
                colored_wubrg_counts: [0; 5],
                colorless_count: 0,
                generic_count: 2,
            }),
            ..ActionCostFacts::default()
        }),
        mana_payment_staging: Some(ManaPaymentStaging {
            stage: ManaPaymentStage::SelectingSources,
            mana_source_activations: vec![],
        }),
    };
    assert!(matches!(
        paying_ward_cost.stage,
        StackResolutionStage::PayingMana
    ));
    assert!(matches!(
        paying_ward_cost.mana_payment_staging.unwrap().stage,
        ManaPaymentStage::SelectingSources
    ));
}
