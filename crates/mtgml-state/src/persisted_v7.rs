//! Canonical restricted-CBOR preimage of FullStateDigestV7, written in one
//! pass from the typed state. The component encoders of the former V5 input
//! live in `digest_v5.rs`; `zones_v2` and `execution_v4` are encoded here.

use mtgml_decision::{
    AuthoritativeCandidateV4, AuthoritativeDecisionRequestV4, CandidateIntentV4,
    CostFactsV1 as DecisionCostFactsV1, DecisionDomainV2, DecisionPurposeV4, DecisionVisibility,
    EngineCandidateBindingV4, SafeDamageRecipientV1, SafeTargetDescriptorV1,
    SafeTriggerDescriptorV1, SafeTriggerSubjectV1, SafeZoneKindV1,
};
use mtgml_persistence::cbor::Value;

use crate::digest_v5;

use crate::{
    AbilitySourceContext, ActionCostFacts, AssemblyStageV2, AttackerFact, CastContinuation,
    CastContinuationStage, ContinuationPayloadV3, ContinuationRecordV3, CostFacts, CostRoute,
    DamageKind, DamageRecipient, EffectExpiry, EffectTimestamp, EngineStateParts,
    EngineStatePartsV3, ExecutionStateV4, LifeChangeCause, ManaCost, ManaPaymentStage,
    ManaPaymentStaging, ManaSourceActivation, ManaSourceActivationCost, ModeBinding,
    NonManaActivationContinuation, NonManaActivationStage, PendingTriggerRecord,
    ReservedNonManaCost, SelectedCostOperand, SourceContext, StackItemPayload,
    StackResolutionContinuation, StackResolutionStage, TargetBinding, TargetRef,
    TemporaryEffectRecord, TemporaryKeyword, TemporaryOperation, TriggerActorRequestRoot,
    TriggerEventSnapshot, TriggerPlacementContinuation, TriggerTargetTiming,
    FULL_STATE_DIGEST_DOMAIN_V7, FULL_STATE_DIGEST_INPUT_SCHEMA_V7,
};

fn u(value: u64) -> Value {
    Value::Unsigned(value)
}

fn i(value: i64) -> Value {
    Value::Signed(value)
}

fn text(value: impl Into<String>) -> Value {
    Value::Text(value.into())
}

fn array(values: impl IntoIterator<Item = Value>) -> Value {
    Value::Array(values.into_iter().collect())
}

fn optional(value: Option<Value>) -> Value {
    value.unwrap_or(Value::Null)
}

/// The 14-element FullStateDigestV7 preimage. Keep both destructures
/// exhaustive: a new state field must fail to compile here until its digest
/// encoding is decided. The legacy execution slot is always empty (validated)
/// and is not encoded.
pub(crate) fn state_value(state: &EngineStatePartsV3) -> Result<Value, crate::StateDigestError> {
    let EngineStatePartsV3 {
        predecessor_v5: parts,
        execution_v4,
        card_rules_state,
    } = state;
    let EngineStateParts {
        revision,
        core: _,
        combat: _,
        foundation_sources: _,
        zones: _,
        allocators: _,
        execution: _,
        random: _,
        knowledge: _,
        perspective_identities: _,
        format,
    } = parts;
    Ok(array([
        text(FULL_STATE_DIGEST_INPUT_SCHEMA_V7),
        text(FULL_STATE_DIGEST_DOMAIN_V7),
        u(revision.0),
        digest_v5::core_value(parts),
        zones_v2_value(parts)?,
        digest_v5::allocators_value(parts),
        execution_v4_value(execution_v4)?,
        digest_v5::random_value(parts),
        digest_v5::knowledge_value(parts)?,
        digest_v5::perspective_identities_value(parts)?,
        digest_v5::combat_value(parts),
        digest_v5::foundation_sources_value(parts),
        digest_v5::format_value(format)?,
        card_rules_state
            .canonical_value()
            .map_err(|_| crate::StateDigestError::StateInvariant)?,
    ]))
}

fn zones_v2_value(state: &EngineStateParts) -> Result<Value, crate::StateDigestError> {
    let [objects, locations, ordered] = digest_v5::zone_contents_values(state)?;
    Ok(array([
        text("zones_v2"),
        objects,
        locations,
        ordered,
        array(
            state
                .zones
                .stack_records
                .values()
                .map(|record| {
                    let payload = record
                        .payload
                        .as_ref()
                        .ok_or(crate::StateDigestError::StateInvariant)?;
                    Ok(array([
                        u(record.id.0),
                        u(record.controller.0),
                        stack_payload_value(payload),
                    ]))
                })
                .collect::<Result<Vec<_>, crate::StateDigestError>>()?,
        ),
        array(state.zones.stack_order.iter().map(|id| u(id.0))),
    ]))
}

fn execution_v4_value(state: &ExecutionStateV4) -> Result<Value, crate::StateDigestError> {
    Ok(array([
        text("execution_v4"),
        optional(state.pending_decision.as_ref().map(decision_request_value)),
        array(
            state
                .continuations
                .values()
                .map(continuation_value)
                .collect::<Result<Vec<_>, _>>()?,
        ),
        array(state.effects.values().map(temporary_effect_value)),
        array(state.waiting_triggers.values().map(pending_trigger_value)),
        array([]),
    ]))
}

fn stack_payload_value(value: &StackItemPayload) -> Value {
    match value {
        StackItemPayload::Spell {
            stack_card_object,
            card_definition_id,
            face_key,
            semantic_profile_id,
            modes,
            targets,
            cost_facts,
        } => array([
            text("spell"),
            u(stack_card_object.0),
            u(card_definition_id.0),
            u(u64::from(face_key.0)),
            text(semantic_profile_id.as_str()),
            array(modes.iter().map(mode_value)),
            array(targets.iter().map(target_binding_value)),
            cost_facts_value(cost_facts),
        ]),
        StackItemPayload::ActivatedAbility {
            source_context,
            modes,
            targets,
            cost_facts,
        } => array([
            text("activated_ability"),
            ability_source_context_value(source_context),
            array(modes.iter().map(mode_value)),
            array(targets.iter().map(target_binding_value)),
            cost_facts_value(cost_facts),
        ]),
        StackItemPayload::TriggeredAbility {
            originating_trigger,
            source_context,
            captured_trigger_context,
            targets,
        } => array([
            text("triggered_ability"),
            u(originating_trigger.0),
            ability_source_context_value(source_context),
            trigger_event_value(captured_trigger_context),
            array(targets.iter().map(target_binding_value)),
        ]),
    }
}

fn source_context_value(value: &SourceContext) -> Value {
    array([
        object_snapshot_value(&value.snapshot),
        u(u64::from(value.face_key.0)),
        text(value.semantic_profile_id.as_str()),
    ])
}

fn ability_source_context_value(value: &AbilitySourceContext) -> Value {
    array([
        source_context_value(&value.source),
        u(value.ability_instance_id.0),
        u(u64::from(value.ability_key.0)),
    ])
}

fn object_snapshot_value(value: &crate::ObjectSnapshot) -> Value {
    array([
        u(value.object.0),
        optional(value.physical_card.map(|id| u(id.0))),
        u(value.card_definition.0),
        u(value.owner.0),
        u(value.controller.0),
        Value::Bool(value.tapped),
        Value::Bool(value.face_down),
        crate::digest_v5::zone_location_value(&value.location),
    ])
}

fn target_ref_value(value: TargetRef) -> Value {
    match value {
        TargetRef::Object(id) => array([text("object"), u(id.0)]),
        TargetRef::Player(id) => array([text("player"), u(id.0)]),
        TargetRef::StackItem(id) => array([text("stack_item"), u(id.0)]),
    }
}

fn target_binding_value(value: &TargetBinding) -> Value {
    array([
        u(u64::from(value.target_slot)),
        target_ref_value(value.target),
    ])
}

fn mode_value(value: &ModeBinding) -> Value {
    array([
        u(u64::from(value.mode_slot)),
        u(u64::from(value.selected_mode)),
    ])
}

fn route_value(value: Option<CostRoute>) -> Value {
    match value {
        None => Value::Null,
        Some(CostRoute::Normal) => array([text("normal")]),
        Some(CostRoute::Alternative {
            profile_local_route_id,
        }) => array([text("alternative"), u(u64::from(profile_local_route_id))]),
    }
}

fn cost_facts_value(value: &CostFacts) -> Value {
    array([
        route_value(value.selected_route),
        array(
            value
                .paid_additional_cost_ids
                .iter()
                .map(|id| u(u64::from(*id))),
        ),
    ])
}

fn action_cost_facts_value(value: &ActionCostFacts) -> Value {
    array([
        optional(value.mana_cost.map(mana_cost_value)),
        array(value.reserved_nonmana_costs.iter().map(reserved_cost_value)),
        array(value.selected_cost_operands.iter().map(cost_operand_value)),
    ])
}

fn mana_cost_value(value: ManaCost) -> Value {
    array([
        array(
            value
                .colored_wubrg_counts
                .iter()
                .map(|amount| u(u64::from(*amount))),
        ),
        u(u64::from(value.colorless_count)),
        u(u64::from(value.generic_count)),
    ])
}

fn reserved_cost_value(value: &ReservedNonManaCost) -> Value {
    match value {
        ReservedNonManaCost::TapSource => array([text("tap_source")]),
        ReservedNonManaCost::SacrificeSource => array([text("sacrifice_source")]),
    }
}

fn cost_operand_value(value: &SelectedCostOperand) -> Value {
    match value {
        SelectedCostOperand::PutCounters {
            object,
            counter_kind,
            count,
        } => array([
            text("put_counters"),
            u(object.0),
            text(match counter_kind {
                crate::CounterKindV1::PlusOnePlusOne => "plus_one_plus_one",
                crate::CounterKindV1::MinusOneMinusOne => "minus_one_minus_one",
                crate::CounterKindV1::Lore => "lore",
            }),
            u(u64::from(*count)),
        ]),
    }
}

fn mana_staging_value(value: &ManaPaymentStaging) -> Value {
    array([
        text("mana_payment_staging"),
        text(match value.stage {
            ManaPaymentStage::SelectingSources => "selecting_sources",
            ManaPaymentStage::AwaitingFinalAllocation => "awaiting_final_allocation",
        }),
        array(value.mana_source_activations.iter().map(mana_source_value)),
    ])
}

fn mana_source_value(value: &ManaSourceActivation) -> Value {
    array([
        u(value.source_object.0),
        u(value.source_ability_instance.0),
        u(u64::from(value.ability_key.0)),
        text(value.semantic_profile_id.as_str()),
        match value.activation_cost_receipt {
            ManaSourceActivationCost::TapSource => array([text("tap_source")]),
        },
        array(
            value
                .produced_buckets
                .iter()
                .map(|amount| u(u64::from(*amount))),
        ),
    ])
}

fn cast_continuation_value(value: &CastContinuation) -> Value {
    array([
        text("cast"),
        u(value.actor.0),
        u(value.spell_object.0),
        u(value.card_definition_id.0),
        u(u64::from(value.face_key.0)),
        text(value.semantic_profile_id.as_str()),
        text(match value.stage {
            CastContinuationStage::SelectingCostRoute => "selecting_cost_route",
            CastContinuationStage::SelectingModes => "selecting_modes",
            CastContinuationStage::SelectingTargets => "selecting_targets",
            CastContinuationStage::SelectingAdditionalCosts => "selecting_additional_costs",
            CastContinuationStage::SelectingCostOperands => "selecting_cost_operands",
            CastContinuationStage::PayingMana => "paying_mana",
        }),
        route_value(value.selected_route),
        array(value.modes.iter().map(mode_value)),
        array(value.targets.iter().map(target_binding_value)),
        array(value.paid_cost_choices.iter().map(|id| u(u64::from(*id)))),
        action_cost_facts_value(&value.action_cost_facts),
        optional(value.mana_payment_staging.as_ref().map(mana_staging_value)),
    ])
}

fn activation_continuation_value(value: &NonManaActivationContinuation) -> Value {
    array([
        text("nonmana_activation"),
        u(value.actor.0),
        u(value.source_object.0),
        u(value.source_ability_instance.0),
        u(u64::from(value.ability_key.0)),
        text(value.semantic_profile_id.as_str()),
        text(match value.stage {
            NonManaActivationStage::SelectingModes => "selecting_modes",
            NonManaActivationStage::SelectingTargets => "selecting_targets",
            NonManaActivationStage::SelectingCostOperands => "selecting_cost_operands",
            NonManaActivationStage::PayingMana => "paying_mana",
        }),
        array(value.modes.iter().map(mode_value)),
        array(value.targets.iter().map(target_binding_value)),
        action_cost_facts_value(&value.action_cost_facts),
        optional(value.mana_payment_staging.as_ref().map(mana_staging_value)),
    ])
}

fn stack_resolution_value(value: &StackResolutionContinuation) -> Value {
    array([
        text("stack_resolution"),
        u(value.resolving_stack_object.0),
        text(match value.stage {
            StackResolutionStage::AwaitingOptionalPayment => "awaiting_optional_payment",
            StackResolutionStage::PayingMana => "paying_mana",
        }),
        optional(
            value
                .action_cost_facts
                .as_ref()
                .map(action_cost_facts_value),
        ),
        optional(value.mana_payment_staging.as_ref().map(mana_staging_value)),
    ])
}

fn continuation_value(value: &ContinuationRecordV3) -> Result<Value, crate::StateDigestError> {
    let payload = match &value.payload {
        ContinuationPayloadV3::SyntheticAssembly {
            actor,
            stage,
            selected_count,
            selected_piece_keys,
            ordered_piece_keys,
        } => array([
            text("synthetic_assembly"),
            u(actor.0),
            text(match stage {
                AssemblyStageV2::ChooseCount => "choose_count",
                AssemblyStageV2::ChooseMembers => "choose_members",
                AssemblyStageV2::OrderMembers => "order_members",
            }),
            optional(selected_count.map(|count| u(u64::from(count)))),
            array(selected_piece_keys.iter().map(|key| u(u64::from(*key)))),
            array(ordered_piece_keys.iter().map(|key| u(u64::from(*key)))),
        ]),
        ContinuationPayloadV3::MagicSbaGraveyardOrderV1 {
            round_start_revision,
            selected_sba_actions,
            apnap_owners,
            next_owner_index,
            completed_owner_orders,
        } => array([
            text("magic_sba_graveyard_order_v1"),
            u(round_start_revision.0),
            array(selected_sba_actions.iter().map(|action| match action {
                crate::SbaSelectedActionV1::PlayerLoses { player } => {
                    array([text("player_loses"), u(player.0)])
                }
                crate::SbaSelectedActionV1::ObjectToOwnerGraveyard { object, causes } => array([
                    text("object_to_owner_graveyard"),
                    u(object.0),
                    array(causes.iter().map(|cause| {
                        text(match cause {
                            crate::SbaObjectCauseV1::ZeroToughness => "zero_toughness",
                            crate::SbaObjectCauseV1::LethalDamage => "lethal_damage",
                        })
                    })),
                ]),
            })),
            array(apnap_owners.iter().map(|player| u(player.0))),
            u(u64::from(*next_owner_index)),
            array(completed_owner_orders.iter().map(|order| {
                array([
                    u(order.owner.0),
                    array(order.top_to_bottom.iter().map(|id| u(id.0))),
                ])
            })),
        ]),
        ContinuationPayloadV3::Cast(value) => cast_continuation_value(value),
        ContinuationPayloadV3::NonManaActivation(value) => activation_continuation_value(value),
        ContinuationPayloadV3::TriggerPlacement(value) => trigger_placement_value(value),
        ContinuationPayloadV3::StackResolution(value) => stack_resolution_value(value),
    };
    Ok(array([
        u(value.id.0),
        u(value.created_at_revision.0),
        payload,
    ]))
}

fn trigger_placement_value(value: &TriggerPlacementContinuation) -> Value {
    array([
        text("trigger_placement"),
        array(value.apnap_actors.iter().map(|player| u(player.0))),
        u(u64::from(value.current_actor_index)),
        array(value.pending_trigger_ids.iter().map(|id| u(id.0))),
        array(value.completed_orders.iter().map(|order| {
            array([
                u(order.actor.0),
                array(order.ordered_trigger_ids.iter().map(|id| u(id.0))),
            ])
        })),
        array(
            value.selected_trigger_targets.iter().map(|target| {
                array([u(target.trigger_id.0), target_binding_value(&target.target)])
            }),
        ),
        array(
            value
                .actor_request_roots
                .iter()
                .map(trigger_actor_root_value),
        ),
    ])
}

fn trigger_actor_root_value(value: &TriggerActorRequestRoot) -> Value {
    array([u(value.actor.0), u(value.first_decision_id.0)])
}

fn pending_trigger_value(value: &PendingTriggerRecord) -> Value {
    array([
        u(value.id.0),
        u(value.controller.0),
        ability_source_context_value(&value.source_context),
        trigger_event_value(&value.trigger_context),
        text(match value.target_timing {
            TriggerTargetTiming::NoTargets => "no_targets",
            TriggerTargetTiming::CapturedFromEvent => "captured_from_event",
            TriggerTargetTiming::ChooseOnPlacement => "choose_on_placement",
        }),
    ])
}

fn temporary_effect_value(value: &TemporaryEffectRecord) -> Value {
    array([
        u(value.id.0),
        array(value.affected_objects.iter().map(|id| u(id.0))),
        match value.operation {
            TemporaryOperation::PowerToughnessDelta { power, toughness } => array([
                text("power_toughness_delta"),
                i(i64::from(power)),
                i(i64::from(toughness)),
            ]),
            TemporaryOperation::GrantKeyword { keyword } => array([
                text("grant_keyword"),
                text(match keyword {
                    TemporaryKeyword::Haste => "haste",
                    TemporaryKeyword::DoubleStrike => "double_strike",
                }),
            ]),
        },
        match value.expiry {
            EffectExpiry::UntilEndOfTurn { turn_number } => {
                array([text("until_end_of_turn"), u(turn_number)])
            }
        },
        optional(value.timestamp.map(effect_timestamp_value)),
    ])
}

fn effect_timestamp_value(value: EffectTimestamp) -> Value {
    array([
        u(value.creation_revision.0),
        u(u64::from(value.operation_ordinal)),
    ])
}

fn trigger_event_value(value: &TriggerEventSnapshot) -> Value {
    match value {
        TriggerEventSnapshot::SpellCast {
            actor,
            stack_item,
            spell,
            is_creature_spell,
            cost_facts,
        } => array([
            text("spell_cast"),
            u(actor.0),
            u(stack_item.0),
            source_context_value(spell),
            Value::Bool(*is_creature_spell),
            cost_facts_value(cost_facts),
        ]),
        TriggerEventSnapshot::AbilityActivated {
            actor,
            stack_item,
            source,
            targets,
            cost_facts,
        } => array([
            text("ability_activated"),
            u(actor.0),
            u(stack_item.0),
            ability_source_context_value(source),
            array(targets.iter().map(target_binding_value)),
            cost_facts_value(cost_facts),
        ]),
        TriggerEventSnapshot::TargetBecame {
            actor,
            source_stack_item,
            target,
        } => array([
            text("target_became"),
            u(actor.0),
            u(source_stack_item.0),
            target_ref_value(*target),
        ]),
        TriggerEventSnapshot::ObjectEntered { object } => {
            array([text("object_entered"), object_snapshot_value(object)])
        }
        TriggerEventSnapshot::ObjectLeftOrDied {
            last_known,
            destination,
        } => array([
            text("object_left_or_died"),
            object_snapshot_value(last_known),
            crate::digest_v5::zone_location_value(destination),
        ]),
        TriggerEventSnapshot::BeginningOfCombat {
            active_player,
            turn_number,
        } => array([
            text("beginning_of_combat"),
            u(active_player.0),
            u(*turn_number),
        ]),
        TriggerEventSnapshot::AttackDeclared {
            controller,
            attackers,
        } => array([
            text("attack_declared"),
            u(controller.0),
            array(attackers.iter().map(attacker_fact_value)),
        ]),
        TriggerEventSnapshot::CardDrawn { player } => array([text("card_drawn"), u(player.0)]),
        TriggerEventSnapshot::CounterChanged {
            object,
            kind,
            before,
            after,
        } => array([
            text("counter_changed"),
            u(object.0),
            text(match kind {
                crate::CounterKindV1::PlusOnePlusOne => "plus_one_plus_one",
                crate::CounterKindV1::MinusOneMinusOne => "minus_one_minus_one",
                crate::CounterKindV1::Lore => "lore",
            }),
            u(u64::from(*before)),
            u(u64::from(*after)),
        ]),
        TriggerEventSnapshot::DamageApplied {
            source,
            recipient,
            amount,
            damage_kind,
        } => array([
            text("damage_applied"),
            optional(source.as_ref().map(source_context_value)),
            damage_recipient_value(*recipient),
            u(u64::from(*amount)),
            text(match damage_kind {
                DamageKind::Combat => "combat",
                DamageKind::Noncombat => "noncombat",
            }),
        ]),
        TriggerEventSnapshot::LifeChanged {
            player,
            before,
            after,
            cause,
        } => array([
            text("life_changed"),
            u(player.0),
            i(*before),
            i(*after),
            text(match cause {
                LifeChangeCause::Damage => "damage",
                LifeChangeCause::NonDamage => "non_damage",
            }),
        ]),
    }
}

fn attacker_fact_value(value: &AttackerFact) -> Value {
    array([u(value.object.0), u(value.defending_player.0)])
}

fn damage_recipient_value(value: DamageRecipient) -> Value {
    match value {
        DamageRecipient::Object(id) => array([text("object"), u(id.0)]),
        DamageRecipient::Player(id) => array([text("player"), u(id.0)]),
    }
}

fn decision_request_value(value: &AuthoritativeDecisionRequestV4) -> Value {
    array([
        u(value.decision_id.0),
        u(value.player_decision_id.0),
        u(value.state_revision.0),
        u(value.view_sequence.0),
        u(value.actor.0),
        text(match value.visibility {
            DecisionVisibility::Public => "public",
            DecisionVisibility::ActingPlayerOnly => "acting_player_only",
            DecisionVisibility::Mixed => "mixed",
        }),
        decision_domain_value(&value.decision_domain_v2),
        decision_purpose_value(&value.purpose),
        optional(value.parent_player_decision_id.map(|id| u(id.0))),
        optional(value.continuation_id.map(|id| u(id.0))),
        array(value.candidates.iter().map(candidate_value)),
    ])
}

fn decision_domain_value(value: &DecisionDomainV2) -> Value {
    match value {
        DecisionDomainV2::ChooseOne => array([text("choose_one")]),
        DecisionDomainV2::ChooseMany { minimum, maximum } => array([
            text("choose_many"),
            u(u64::from(*minimum)),
            u(u64::from(*maximum)),
        ]),
        DecisionDomainV2::ChooseNumber { minimum, maximum } => {
            array([text("choose_number"), i(*minimum), i(*maximum)])
        }
        DecisionDomainV2::Order { minimum, maximum } => array([
            text("order"),
            u(u64::from(*minimum)),
            u(u64::from(*maximum)),
        ]),
    }
}

fn decision_purpose_value(value: &DecisionPurposeV4) -> Value {
    match value {
        DecisionPurposeV4::PriorityAction => array([text("priority_action")]),
        DecisionPurposeV4::AttackerDeclaration => array([text("attacker_declaration")]),
        DecisionPurposeV4::HandSizeDiscard => array([text("hand_size_discard")]),
        DecisionPurposeV4::SbaGraveyardOrder => array([text("sba_graveyard_order")]),
        DecisionPurposeV4::CastCostRoute => array([text("cast_cost_route")]),
        DecisionPurposeV4::ModeSelection { mode_slot } => {
            array([text("mode_selection"), u(u64::from(*mode_slot))])
        }
        DecisionPurposeV4::TargetSelection { target_slot } => {
            array([text("target_selection"), u(u64::from(*target_slot))])
        }
        DecisionPurposeV4::CostOperandSelection {
            cost_slot,
            operation,
            counter_kind,
            count,
        } => array([
            text("cost_operand_selection"),
            u(u64::from(*cost_slot)),
            text(match operation {
                mtgml_decision::CostOperandOperationV1::PutCounters => "put_counters",
            }),
            text(match counter_kind {
                mtgml_decision::CounterKindV1::PlusOnePlusOne => "plus_one_plus_one",
                mtgml_decision::CounterKindV1::MinusOneMinusOne => "minus_one_minus_one",
                mtgml_decision::CounterKindV1::Lore => "lore",
            }),
            u(u64::from(*count)),
        ]),
        DecisionPurposeV4::ManaProductionChoice => array([text("mana_production_choice")]),
        DecisionPurposeV4::ManaPayment => array([text("mana_payment")]),
        DecisionPurposeV4::OptionalCostPayment {
            profile_local_cost_id,
        } => array([
            text("optional_cost_payment"),
            u(u64::from(*profile_local_cost_id)),
        ]),
        DecisionPurposeV4::AbilityAction => array([text("ability_action")]),
        DecisionPurposeV4::TriggerOrder => array([text("trigger_order")]),
        DecisionPurposeV4::TriggerTarget { target_slot } => {
            array([text("trigger_target"), u(u64::from(*target_slot))])
        }
        DecisionPurposeV4::SyntheticAssembly { stage } => array([
            text("synthetic_assembly"),
            text(match stage {
                mtgml_decision::SyntheticAssemblyStageV1::Entry => "entry",
                mtgml_decision::SyntheticAssemblyStageV1::ChooseCount => "choose_count",
                mtgml_decision::SyntheticAssemblyStageV1::ChooseMembers => "choose_members",
                mtgml_decision::SyntheticAssemblyStageV1::OrderMembers => "order_members",
            }),
        ]),
    }
}

fn candidate_value(value: &AuthoritativeCandidateV4) -> Value {
    array([
        u(u64::from(value.candidate_id.0)),
        candidate_intent_value(&value.visible_intent),
        candidate_binding_value(&value.trusted_binding),
    ])
}

fn candidate_intent_value(value: &CandidateIntentV4) -> Value {
    match value {
        CandidateIntentV4::PassPriority => array([text("pass_priority"), Value::Null]),
        CandidateIntentV4::PlayLand { object } => array([text("play_land"), u(object.0)]),
        CandidateIntentV4::CastSpell { object } => array([text("cast_spell"), u(object.0)]),
        CandidateIntentV4::ActivateAbility { ability } => {
            array([text("activate_ability"), u(ability.0)])
        }
        CandidateIntentV4::SelectObject { object } => array([text("select_object"), u(object.0)]),
        CandidateIntentV4::SelectPlayer { player } => array([text("select_player"), u(player.0)]),
        CandidateIntentV4::SelectMode { mode_index } => {
            array([text("select_mode"), u(u64::from(*mode_index))])
        }
        CandidateIntentV4::ChooseBoolean { value } => {
            array([text("choose_boolean"), Value::Bool(*value)])
        }
        CandidateIntentV4::DeclareNumber { value } => array([text("declare_number"), i(*value)]),
        CandidateIntentV4::Confirm => array([text("confirm"), Value::Null]),
        CandidateIntentV4::SelectCostRoute { descriptor } => array([
            text("select_cost_route"),
            text(match descriptor.route_class {
                mtgml_decision::CostRouteClassV1::Normal => "normal",
                mtgml_decision::CostRouteClassV1::Alternative => "alternative",
            }),
            printed_symbols_value(descriptor.printed_mana_symbols),
            optional(
                descriptor
                    .profile_local_option_ordinal
                    .map(|id| u(u64::from(id))),
            ),
        ]),
        CandidateIntentV4::SelectManaSource {
            source,
            ability,
            produced_buckets,
        } => array([
            text("select_mana_source"),
            u(source.0),
            u(ability.0),
            array(produced_buckets.iter().map(|amount| u(u64::from(*amount)))),
        ]),
        CandidateIntentV4::FinalizeManaProduction => {
            array([text("finalize_mana_production"), Value::Null])
        }
        CandidateIntentV4::SelectManaPayment { spent_buckets } => array([
            text("select_mana_payment"),
            array(spent_buckets.iter().map(|amount| u(u64::from(*amount)))),
        ]),
        CandidateIntentV4::SelectTrigger { trigger } => array([
            text("select_trigger"),
            safe_trigger_descriptor_value(trigger),
        ]),
    }
}

fn candidate_binding_value(value: &EngineCandidateBindingV4) -> Value {
    match value {
        EngineCandidateBindingV4::PassPriority => array([text("pass_priority"), Value::Null]),
        EngineCandidateBindingV4::PlayLand { object } => array([text("play_land"), u(object.0)]),
        EngineCandidateBindingV4::CastSpell { object } => array([text("cast_spell"), u(object.0)]),
        EngineCandidateBindingV4::ActivateAbility { ability } => {
            array([text("activate_ability"), u(ability.0)])
        }
        EngineCandidateBindingV4::SelectObject { object } => {
            array([text("select_object"), u(object.0)])
        }
        EngineCandidateBindingV4::SelectPlayer { player } => {
            array([text("select_player"), u(player.0)])
        }
        EngineCandidateBindingV4::SelectMode { mode_index } => {
            array([text("select_mode"), u(u64::from(*mode_index))])
        }
        EngineCandidateBindingV4::ChooseBoolean { value } => {
            array([text("choose_boolean"), Value::Bool(*value)])
        }
        EngineCandidateBindingV4::DeclareNumber { value } => {
            array([text("declare_number"), i(*value)])
        }
        EngineCandidateBindingV4::Confirm => array([text("confirm"), Value::Null]),
        EngineCandidateBindingV4::SelectCostRoute { route } => array([
            text("select_cost_route"),
            match route {
                mtgml_decision::CostRouteV1::Normal => array([text("normal")]),
                mtgml_decision::CostRouteV1::Alternative { route_id } => {
                    array([text("alternative"), u(u64::from(*route_id))])
                }
            },
        ]),
        EngineCandidateBindingV4::SelectManaSource {
            source,
            ability,
            ability_key,
            semantic_profile_id,
            activation_cost,
            produced_buckets,
        } => array([
            text("select_mana_source"),
            u(source.0),
            u(ability.0),
            u(u64::from(ability_key.0)),
            text(semantic_profile_id.as_str()),
            match activation_cost {
                mtgml_decision::ManaSourceActivationCostV1::TapSource => {
                    array([text("tap_source")])
                }
            },
            array(produced_buckets.iter().map(|amount| u(u64::from(*amount)))),
        ]),
        EngineCandidateBindingV4::FinalizeManaProduction { continuation } => {
            array([text("finalize_mana_production"), u(continuation.0)])
        }
        EngineCandidateBindingV4::SelectManaPayment { spent_buckets } => array([
            text("select_mana_payment"),
            array(spent_buckets.iter().map(|amount| u(u64::from(*amount)))),
        ]),
        EngineCandidateBindingV4::SelectTrigger { trigger } => {
            array([text("select_trigger"), u(trigger.0)])
        }
    }
}

fn printed_symbols_value(value: mtgml_decision::PrintedManaSymbolsV1) -> Value {
    array([
        array(
            value
                .colored_wubrg_counts
                .iter()
                .map(|amount| u(u64::from(*amount))),
        ),
        u(u64::from(value.colorless_count)),
        u(u64::from(value.generic_count)),
    ])
}

fn safe_trigger_descriptor_value(value: &SafeTriggerDescriptorV1) -> Value {
    array([
        optional(value.source_object.map(|id| u(id.0))),
        optional(value.source_ability.map(|id| u(id.0))),
        text(match value.event_kind {
            mtgml_decision::TriggerEventKindV1::SpellCast => "spell_cast",
            mtgml_decision::TriggerEventKindV1::AbilityActivated => "ability_activated",
            mtgml_decision::TriggerEventKindV1::TargetBecame => "target_became",
            mtgml_decision::TriggerEventKindV1::ObjectEntered => "object_entered",
            mtgml_decision::TriggerEventKindV1::ObjectLeftOrDied => "object_left_or_died",
            mtgml_decision::TriggerEventKindV1::BeginningOfCombat => "beginning_of_combat",
            mtgml_decision::TriggerEventKindV1::AttackDeclared => "attack_declared",
            mtgml_decision::TriggerEventKindV1::CardDrawn => "card_drawn",
            mtgml_decision::TriggerEventKindV1::CounterChanged => "counter_changed",
            mtgml_decision::TriggerEventKindV1::DamageApplied => "damage_applied",
            mtgml_decision::TriggerEventKindV1::LifeChanged => "life_changed",
        }),
        safe_trigger_subject_value(&value.subject),
    ])
}

fn safe_trigger_subject_value(value: &SafeTriggerSubjectV1) -> Value {
    match value {
        SafeTriggerSubjectV1::SpellCast {
            actor,
            spell_source_object,
            creature_spell,
            cost_facts,
        } => array([
            text("spell_cast"),
            u(actor.0),
            optional(spell_source_object.map(|id| u(id.0))),
            Value::Bool(*creature_spell),
            decision_cost_facts_value(cost_facts),
        ]),
        SafeTriggerSubjectV1::AbilityActivated {
            actor,
            activated_source_object,
            activated_source_ability,
            cost_facts,
            targets,
        } => array([
            text("ability_activated"),
            u(actor.0),
            u(activated_source_object.0),
            u(activated_source_ability.0),
            decision_cost_facts_value(cost_facts),
            array(targets.iter().map(safe_target_value)),
        ]),
        SafeTriggerSubjectV1::TargetBecame { actor, target } => {
            array([text("target_became"), u(actor.0), safe_target_value(target)])
        }
        SafeTriggerSubjectV1::ObjectEntered { object } => {
            array([text("object_entered"), optional(object.map(|id| u(id.0)))])
        }
        SafeTriggerSubjectV1::ObjectLeftOrDied {
            last_known_object,
            destination,
        } => array([
            text("object_left_or_died"),
            optional(last_known_object.map(|id| u(id.0))),
            text(safe_zone_name(*destination)),
        ]),
        SafeTriggerSubjectV1::BeginningOfCombat {
            active_player,
            turn_number,
        } => array([
            text("beginning_of_combat"),
            u(active_player.0),
            u(*turn_number),
        ]),
        SafeTriggerSubjectV1::AttackDeclared {
            controller,
            attackers,
        } => {
            array([
                text("attack_declared"),
                u(controller.0),
                array(attackers.iter().map(|attacker| {
                    array([u(attacker.attacker.0), u(attacker.defending_player.0)])
                })),
            ])
        }
        SafeTriggerSubjectV1::CardDrawn { player } => array([text("card_drawn"), u(player.0)]),
        SafeTriggerSubjectV1::CounterChanged {
            object,
            counter_kind,
            before,
            after,
        } => array([
            text("counter_changed"),
            optional(object.map(|id| u(id.0))),
            text(match counter_kind {
                mtgml_decision::CounterKindV1::PlusOnePlusOne => "plus_one_plus_one",
                mtgml_decision::CounterKindV1::MinusOneMinusOne => "minus_one_minus_one",
                mtgml_decision::CounterKindV1::Lore => "lore",
            }),
            u(u64::from(*before)),
            u(u64::from(*after)),
        ]),
        SafeTriggerSubjectV1::DamageApplied {
            source_object,
            recipient,
            amount,
            damage_kind,
        } => array([
            text("damage_applied"),
            optional(source_object.map(|id| u(id.0))),
            match recipient {
                SafeDamageRecipientV1::Object { object } => array([text("object"), u(object.0)]),
                SafeDamageRecipientV1::Player { player } => array([text("player"), u(player.0)]),
            },
            u(u64::from(*amount)),
            text(match damage_kind {
                mtgml_decision::DamageKindV1::Combat => "combat",
                mtgml_decision::DamageKindV1::Noncombat => "noncombat",
            }),
        ]),
        SafeTriggerSubjectV1::LifeChanged {
            player,
            before,
            after,
            cause,
        } => array([
            text("life_changed"),
            u(player.0),
            i(*before),
            i(*after),
            text(match cause {
                mtgml_decision::LifeChangeCauseV1::Damage => "damage",
                mtgml_decision::LifeChangeCauseV1::NonDamage => "non_damage",
            }),
        ]),
    }
}

fn safe_target_value(value: &SafeTargetDescriptorV1) -> Value {
    match value {
        SafeTargetDescriptorV1::Object { object } => array([text("object"), u(object.0)]),
        SafeTargetDescriptorV1::Player { player } => array([text("player"), u(player.0)]),
        SafeTargetDescriptorV1::StackItem {
            stack_position_from_top,
        } => array([text("stack_item"), u(u64::from(*stack_position_from_top))]),
    }
}

fn decision_cost_facts_value(value: &DecisionCostFactsV1) -> Value {
    array([
        match &value.selected_route {
            None => Value::Null,
            Some(mtgml_decision::CostRouteV1::Normal) => array([text("normal")]),
            Some(mtgml_decision::CostRouteV1::Alternative { route_id }) => {
                array([text("alternative"), u(u64::from(*route_id))])
            }
        },
        array(
            value
                .paid_additional_cost_ids
                .iter()
                .map(|id| u(u64::from(*id))),
        ),
    ])
}

fn safe_zone_name(value: SafeZoneKindV1) -> &'static str {
    match value {
        SafeZoneKindV1::Library => "library",
        SafeZoneKindV1::Hand => "hand",
        SafeZoneKindV1::Battlefield => "battlefield",
        SafeZoneKindV1::Graveyard => "graveyard",
        SafeZoneKindV1::Exile => "exile",
        SafeZoneKindV1::Stack => "stack",
        SafeZoneKindV1::Command => "command",
        SafeZoneKindV1::Ante => "ante",
        SafeZoneKindV1::Outside => "outside",
    }
}

#[cfg(test)]
mod decision_purpose_codec_tests {
    use super::*;

    #[test]
    fn hand_size_discard_purpose_round_trips_through_the_persisted_codec() {
        let value = decision_purpose_value(&DecisionPurposeV4::HandSizeDiscard);
        assert_eq!(value, array([text("hand_size_discard")]));
    }
}
