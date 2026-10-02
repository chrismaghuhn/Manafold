//! FullStateDigest: validate the state once, encode its canonical
//! restricted-CBOR preimage in one pass from the typed state, and hash it
//! once. The conversion does not use Rust/Serde output; every field is mapped
//! to the fixed semantic CBOR layout from STATE_HASHING.md.

use mtgml_decision::{
    AuthoritativeCandidate, AuthoritativeDecisionRequest, CandidateIntent,
    CostFactsV1 as DecisionCostFactsV1, DecisionDomainV2, DecisionPurposeV4, DecisionVisibility,
    EngineCandidateBinding, SafeDamageRecipientV1, SafeTargetDescriptorV1, SafeTriggerDescriptorV1,
    SafeTriggerSubjectV1, SafeZoneKindV1,
};
use mtgml_model::FullStateDigest;
use mtgml_persistence::cbor::{self, Value};
use mtgml_persistence::envelope;
use thiserror::Error;

use crate::core::{BeginningStep, CombatStep, EndingStep, PriorityState, TurnPosition};
use crate::engine_state_shape::{
    KnowledgeInvalidationV2, KnowledgeRecordV2, KnownLocationFactV2, RetiredKnowledgeRecordV2,
};
use crate::format::FormatState;
use crate::knowledge::{KnowledgeAcquisitionReason, KnowledgeInvalidationReason};
use crate::zones::{VisibilityPartition, ZoneKey, ZoneLocation, ZonePosition};
use crate::{
    AbilitySourceContext, ActionCostFacts, AssemblyStageV2, AttackerFact, CastContinuation,
    CastContinuationStage, ContinuationPayload, ContinuationRecord, CostFacts, CostRoute,
    DamageKind, DamageRecipient, EffectExpiry, EffectTimestamp, EngineState, ExecutionState,
    LifeChangeCause, ManaCost, ManaPaymentStage, ManaPaymentStaging, ManaSourceActivation,
    ManaSourceActivationCost, ModeBinding, NonManaActivationContinuation, NonManaActivationStage,
    PendingTriggerRecord, ReservedNonManaCost, SelectedCostOperand, SourceContext,
    StackItemPayload, StackResolutionContinuation, StackResolutionStage, TargetBinding, TargetRef,
    TemporaryEffectRecord, TemporaryKeyword, TemporaryOperation, TriggerActorRequestRoot,
    TriggerEventSnapshot, TriggerPlacementContinuation, TriggerTargetTiming,
    FULL_STATE_DIGEST_DOMAIN_V7, FULL_STATE_DIGEST_INPUT_SCHEMA_V7,
};

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum StateDigestError {
    #[error("canonical full-state digest serialization failed")]
    Serialization,
    #[error("persisted full-state digest encoding failed: {0}")]
    Persistence(mtgml_persistence::PersistenceDecodeErrorV1),
    #[error("state violates the authoritative current digest preconditions")]
    StateInvariant,
}

pub fn canonical_state_bytes(state: &EngineState) -> Result<Vec<u8>, StateDigestError> {
    state
        .validate()
        .map_err(|_| StateDigestError::StateInvariant)?;
    encode(state)
}

/// Canonicalizes a structurally valid state after a RulesKernel-owned exact
/// profile-domain check at the containing runtime boundary. This encoder does
/// not itself prove or admit a Decision domain.
pub(crate) fn canonical_state_bytes_structural_only(
    state: &EngineState,
) -> Result<Vec<u8>, StateDigestError> {
    state
        .validate_structure()
        .map_err(|_| StateDigestError::StateInvariant)?;
    encode(state)
}

fn encode(state: &EngineState) -> Result<Vec<u8>, StateDigestError> {
    cbor::encode_canonical(&state_value(state)?).map_err(StateDigestError::Persistence)
}

pub fn calculate_full_state_digest(
    state: &EngineState,
) -> Result<FullStateDigest, StateDigestError> {
    full_state_digest_from_payload(&canonical_state_bytes(state)?)
}

#[doc(hidden)]
pub fn calculate_full_state_digest_structural_only(
    state: &EngineState,
) -> Result<FullStateDigest, StateDigestError> {
    full_state_digest_from_payload(&canonical_state_bytes_structural_only(state)?)
}

/// The hash step alone: envelope the preimage bytes and hash them. It does not
/// decode or check the preimage, so it is only meaningful for bytes produced
/// by `canonical_state_bytes` (or a known-answer vector of them).
pub fn full_state_digest_from_payload(payload: &[u8]) -> Result<FullStateDigest, StateDigestError> {
    let encoded = envelope::encode_envelope(
        FULL_STATE_DIGEST_DOMAIN_V7,
        FULL_STATE_DIGEST_INPUT_SCHEMA_V7,
        payload,
    )
    .map_err(StateDigestError::Persistence)?;
    Ok(FullStateDigest::from_digest_bytes(envelope::hash_envelope(
        &encoded,
    )))
}

/// The 14-element FullStateDigest preimage. Keep both destructures
/// exhaustive: a new state field must fail to compile here until its digest
/// encoding is decided. The legacy execution slot is always empty (validated)
/// and is not encoded.
pub(crate) fn state_value(state: &EngineState) -> Result<Value, crate::StateDigestError> {
    let EngineState {
        revision,
        core: _,
        combat: _,
        zones: _,
        allocators: _,
        execution,
        random: _,
        knowledge: _,
        perspective_identities: _,
        format,
        card_rules,
    } = state;
    Ok(array([
        text(FULL_STATE_DIGEST_INPUT_SCHEMA_V7),
        text(FULL_STATE_DIGEST_DOMAIN_V7),
        u(revision.0),
        core_value(state),
        zones_v2_value(state)?,
        allocators_value(state),
        execution_v4_value(execution)?,
        random_value(state),
        knowledge_value(state)?,
        perspective_identities_value(state)?,
        combat_value(state),
        // The foundation-source slot of the preimage is always empty.
        array([]),
        format_value(format)?,
        card_rules
            .canonical_value()
            .map_err(|_| crate::StateDigestError::StateInvariant)?,
    ]))
}

fn zones_v2_value(state: &EngineState) -> Result<Value, crate::StateDigestError> {
    let [objects, locations, ordered] = zone_contents_values(state)?;
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

fn execution_v4_value(state: &ExecutionState) -> Result<Value, crate::StateDigestError> {
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
        zone_location_value(&value.location),
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

fn game_start_value(value: &crate::GameStartContinuation) -> Value {
    let players = |players: &std::collections::BTreeSet<mtgml_model::PlayerId>| {
        array(players.iter().map(|player| u(player.0)))
    };
    array([
        text("game_start"),
        u(value.chooser.0),
        optional(value.starting_player.map(|player| u(player.0))),
        match value.stage {
            crate::GameStartStage::ChoosingStartingPlayer => {
                array([text("choosing_starting_player"), Value::Null])
            }
            crate::GameStartStage::Declaring { player } => array([text("declaring"), u(player.0)]),
            crate::GameStartStage::Bottoming { player } => array([text("bottoming"), u(player.0)]),
        },
        array(
            value
                .mulligans_taken
                .iter()
                .map(|(player, count)| array([u(player.0), u(u64::from(*count))])),
        ),
        players(&value.kept),
        players(&value.round_mulligans),
    ])
}

fn continuation_value(value: &ContinuationRecord) -> Result<Value, crate::StateDigestError> {
    let payload = match &value.payload {
        ContinuationPayload::SyntheticAssembly {
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
        ContinuationPayload::MagicSbaGraveyardOrderV1 {
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
        ContinuationPayload::Cast(value) => cast_continuation_value(value),
        ContinuationPayload::NonManaActivation(value) => activation_continuation_value(value),
        ContinuationPayload::TriggerPlacement(value) => trigger_placement_value(value),
        ContinuationPayload::StackResolution(value) => stack_resolution_value(value),
        ContinuationPayload::GameStart(value) => game_start_value(value),
        ContinuationPayload::BlockDeclaration {
            defender,
            pending_blockers,
            declared,
        } => array([
            text("block_declaration"),
            u(defender.0),
            array(pending_blockers.iter().map(|blocker| u(blocker.0))),
            array(declared.iter().map(|(blocker, attacker)| {
                array([
                    u(blocker.0),
                    optional(attacker.map(|attacker| u(attacker.0))),
                ])
            })),
        ]),
        ContinuationPayload::CombatDamageAssignment {
            player,
            pending_attackers,
            pending_blockers,
            assigned,
        } => array([
            text("combat_damage_assignment"),
            u(player.0),
            array(pending_attackers.iter().map(|attacker| u(attacker.0))),
            array(pending_blockers.iter().map(|blocker| u(blocker.0))),
            array(
                assigned
                    .iter()
                    .map(|(blocker, amount)| array([u(blocker.0), u(*amount)])),
            ),
        ]),
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
            zone_location_value(destination),
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

fn decision_request_value(value: &AuthoritativeDecisionRequest) -> Value {
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
        DecisionPurposeV4::BlockerDeclaration => array([text("blocker_declaration")]),
        DecisionPurposeV4::CombatDamageAssignment => array([text("combat_damage_assignment")]),
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
        DecisionPurposeV4::StartingPlayer => array([text("starting_player")]),
        DecisionPurposeV4::MulliganDeclaration => array([text("mulligan_declaration")]),
        DecisionPurposeV4::MulliganBottom => array([text("mulligan_bottom")]),
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

fn candidate_value(value: &AuthoritativeCandidate) -> Value {
    array([
        u(u64::from(value.candidate_id.0)),
        candidate_intent_value(&value.visible_intent),
        candidate_binding_value(&value.trusted_binding),
    ])
}

fn candidate_intent_value(value: &CandidateIntent) -> Value {
    match value {
        CandidateIntent::PassPriority => array([text("pass_priority"), Value::Null]),
        CandidateIntent::PlayLand { object } => array([text("play_land"), u(object.0)]),
        CandidateIntent::CastSpell { object } => array([text("cast_spell"), u(object.0)]),
        CandidateIntent::ActivateAbility { ability } => {
            array([text("activate_ability"), u(ability.0)])
        }
        CandidateIntent::SelectObject { object } => array([text("select_object"), u(object.0)]),
        CandidateIntent::SelectPlayer { player } => array([text("select_player"), u(player.0)]),
        CandidateIntent::SelectMode { mode_index } => {
            array([text("select_mode"), u(u64::from(*mode_index))])
        }
        CandidateIntent::ChooseBoolean { value } => {
            array([text("choose_boolean"), Value::Bool(*value)])
        }
        CandidateIntent::DeclareNumber { value } => array([text("declare_number"), i(*value)]),
        CandidateIntent::Confirm => array([text("confirm"), Value::Null]),
        CandidateIntent::SelectCostRoute { descriptor } => array([
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
        CandidateIntent::SelectManaSource {
            source,
            ability,
            produced_buckets,
        } => array([
            text("select_mana_source"),
            u(source.0),
            u(ability.0),
            array(produced_buckets.iter().map(|amount| u(u64::from(*amount)))),
        ]),
        CandidateIntent::FinalizeManaProduction => {
            array([text("finalize_mana_production"), Value::Null])
        }
        CandidateIntent::SelectManaPayment { spent_buckets } => array([
            text("select_mana_payment"),
            array(spent_buckets.iter().map(|amount| u(u64::from(*amount)))),
        ]),
        CandidateIntent::SelectTrigger { trigger } => array([
            text("select_trigger"),
            safe_trigger_descriptor_value(trigger),
        ]),
        CandidateIntent::DeclareBlock { blocker, attacker } => array([
            text("declare_block"),
            u(blocker.0),
            optional(attacker.map(|attacker| u(attacker.0))),
        ]),
        CandidateIntent::AssignCombatDamage {
            attacker,
            recipient,
            amount,
        } => array([
            text("assign_combat_damage"),
            u(attacker.0),
            u(recipient.0),
            u(*amount),
        ]),
    }
}

fn candidate_binding_value(value: &EngineCandidateBinding) -> Value {
    match value {
        EngineCandidateBinding::PassPriority => array([text("pass_priority"), Value::Null]),
        EngineCandidateBinding::PlayLand { object } => array([text("play_land"), u(object.0)]),
        EngineCandidateBinding::CastSpell { object } => array([text("cast_spell"), u(object.0)]),
        EngineCandidateBinding::ActivateAbility { ability } => {
            array([text("activate_ability"), u(ability.0)])
        }
        EngineCandidateBinding::SelectObject { object } => {
            array([text("select_object"), u(object.0)])
        }
        EngineCandidateBinding::SelectPlayer { player } => {
            array([text("select_player"), u(player.0)])
        }
        EngineCandidateBinding::SelectMode { mode_index } => {
            array([text("select_mode"), u(u64::from(*mode_index))])
        }
        EngineCandidateBinding::ChooseBoolean { value } => {
            array([text("choose_boolean"), Value::Bool(*value)])
        }
        EngineCandidateBinding::DeclareNumber { value } => {
            array([text("declare_number"), i(*value)])
        }
        EngineCandidateBinding::Confirm => array([text("confirm"), Value::Null]),
        EngineCandidateBinding::SelectCostRoute { route } => array([
            text("select_cost_route"),
            match route {
                mtgml_decision::CostRouteV1::Normal => array([text("normal")]),
                mtgml_decision::CostRouteV1::Alternative { route_id } => {
                    array([text("alternative"), u(u64::from(*route_id))])
                }
            },
        ]),
        EngineCandidateBinding::SelectManaSource {
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
        EngineCandidateBinding::FinalizeManaProduction { continuation } => {
            array([text("finalize_mana_production"), u(continuation.0)])
        }
        EngineCandidateBinding::SelectManaPayment { spent_buckets } => array([
            text("select_mana_payment"),
            array(spent_buckets.iter().map(|amount| u(u64::from(*amount)))),
        ]),
        EngineCandidateBinding::SelectTrigger { trigger } => {
            array([text("select_trigger"), u(trigger.0)])
        }
        EngineCandidateBinding::DeclareBlock { blocker, attacker } => array([
            text("declare_block"),
            u(blocker.0),
            optional(attacker.map(|attacker| u(attacker.0))),
        ]),
        EngineCandidateBinding::AssignCombatDamage {
            attacker,
            recipient,
            amount,
        } => array([
            text("assign_combat_damage"),
            u(attacker.0),
            u(recipient.0),
            u(*amount),
        ]),
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

fn u(value: u64) -> Value {
    Value::Unsigned(value)
}

fn u32_value(value: u32) -> Value {
    Value::Unsigned(u64::from(value))
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

pub(crate) fn core_value(state: &EngineState) -> Value {
    let players =
        state.core.players.iter().map(|(player, value)| {
            array([u(player.0), i(value.life), Value::Bool(value.has_lost)])
        });
    array([
        array(players),
        u(state.core.active_player.0),
        u(state.core.turn_number),
        turn_position_value(state.core.position),
        priority_value(state.core.priority),
    ])
}

pub(crate) fn turn_position_value(position: TurnPosition) -> Value {
    match position {
        TurnPosition::Beginning { step } => array([text("beginning"), text(beginning_step(step))]),
        TurnPosition::PrecombatMain => array([text("precombat_main"), Value::Null]),
        TurnPosition::Combat { step } => array([text("combat"), text(combat_step(step))]),
        TurnPosition::PostcombatMain => array([text("postcombat_main"), Value::Null]),
        TurnPosition::Ending { step } => array([text("ending"), text(ending_step(step))]),
    }
}

fn beginning_step(step: BeginningStep) -> &'static str {
    match step {
        BeginningStep::Untap => "untap",
        BeginningStep::Upkeep => "upkeep",
        BeginningStep::Draw => "draw",
    }
}

fn combat_step(step: CombatStep) -> &'static str {
    match step {
        CombatStep::BeginningOfCombat => "beginning_of_combat",
        CombatStep::DeclareAttackers => "declare_attackers",
        CombatStep::DeclareBlockers => "declare_blockers",
        CombatStep::CombatDamage => "combat_damage",
        CombatStep::EndOfCombat => "end_of_combat",
    }
}

fn ending_step(step: EndingStep) -> &'static str {
    match step {
        EndingStep::EndStep => "end_step",
        EndingStep::Cleanup => "cleanup",
    }
}

fn priority_value(priority: PriorityState) -> Value {
    match priority {
        PriorityState::None => array([text("none"), Value::Null]),
        PriorityState::HeldBy {
            player,
            consecutive_passes,
        } => array([
            text("held_by"),
            array([u(player.0), u(u64::from(consecutive_passes))]),
        ]),
    }
}

pub(crate) fn combat_value(state: &EngineState) -> Value {
    let Some(combat) = &state.combat else {
        return Value::Null;
    };
    array([
        u(combat.defending_player.0),
        array(combat.attackers.iter().map(|object| u(object.0))),
        array(combat.blockers.iter().map(|(blocker, attacker)| {
            array([u(blocker.0), optional(attacker.map(|object| u(object.0)))])
        })),
        array(combat.blocked_attackers.iter().map(|object| u(object.0))),
        Value::Bool(combat.damage_step_completed),
    ])
}

/// Objects, locations and ordered zones: the first three `zones_v2` elements.
pub(crate) fn zone_contents_values(state: &EngineState) -> Result<[Value; 3], StateDigestError> {
    let objects = state.zones.objects.values().map(|object| {
        array([
            u(object.id.0),
            optional(object.physical_card.map(|value| u(value.0))),
            u(object.card_definition.0),
            u(object.owner.0),
            u(object.controller.0),
            Value::Bool(object.tapped),
            Value::Bool(object.face_down),
        ])
    });
    let locations = state
        .zones
        .locations
        .iter()
        .map(|(object, location)| array([u(object.0), zone_location_value(location)]));

    let mut ordered = state
        .zones
        .ordered_zones
        .iter()
        .map(|(key, objects)| {
            let key_value = zone_key_value(key);
            Ok((
                cbor::encode_canonical(&key_value).map_err(StateDigestError::Persistence)?,
                array([key_value, array(objects.iter().map(|object| u(object.0)))]),
            ))
        })
        .collect::<Result<Vec<_>, StateDigestError>>()?;
    ordered.sort_by(|left, right| left.0.cmp(&right.0));

    Ok([
        array(objects),
        array(locations),
        array(ordered.into_iter().map(|(_, value)| value)),
    ])
}

pub(crate) fn zone_key_value(key: &ZoneKey) -> Value {
    array([
        text(zone_kind(key.zone)),
        optional(key.player.map(|player| u(player.0))),
        text(visibility(key.visibility)),
        optional(key.partition.clone().map(Value::Text)),
    ])
}

pub(crate) fn zone_location_value(location: &ZoneLocation) -> Value {
    array([
        text(zone_kind(location.zone)),
        optional(location.player.map(|player| u(player.0))),
        zone_position_value(location.position),
        text(visibility(location.visibility)),
        optional(location.partition.clone().map(Value::Text)),
    ])
}

fn zone_position_value(position: ZonePosition) -> Value {
    match position {
        ZonePosition::Unordered => array([text("unordered"), Value::Null]),
        ZonePosition::Top { offset } => array([text("top"), u32_value(offset)]),
        ZonePosition::Bottom { offset } => array([text("bottom"), u32_value(offset)]),
        ZonePosition::Index { index } => array([text("index"), u32_value(index)]),
    }
}

fn zone_kind(kind: mtgml_model::ZoneKind) -> &'static str {
    match kind {
        mtgml_model::ZoneKind::Library => "library",
        mtgml_model::ZoneKind::Hand => "hand",
        mtgml_model::ZoneKind::Battlefield => "battlefield",
        mtgml_model::ZoneKind::Graveyard => "graveyard",
        mtgml_model::ZoneKind::Exile => "exile",
        mtgml_model::ZoneKind::Stack => "stack",
        mtgml_model::ZoneKind::Command => "command",
        mtgml_model::ZoneKind::Ante => "ante",
        mtgml_model::ZoneKind::Outside => "outside",
    }
}

fn visibility(value: VisibilityPartition) -> &'static str {
    match value {
        VisibilityPartition::Public => "public",
        VisibilityPartition::OwnerOnly => "owner_only",
        VisibilityPartition::FaceDown => "face_down",
        VisibilityPartition::PrivateGroup => "private_group",
    }
}

pub(crate) fn allocators_value(state: &EngineState) -> Value {
    let a = &state.allocators;
    array([
        u(a.next_object_id.0),
        u(a.next_ability_id.0),
        u(a.next_stack_object_id.0),
        u(a.next_effect_id.0),
        u(a.next_trigger_id.0),
        u(a.next_decision_id.0),
        u(a.next_continuation_id.0),
        u(a.next_rule_event_id.0),
    ])
}

pub(crate) fn random_value(state: &EngineState) -> Value {
    let mut streams: Vec<_> = state
        .random
        .streams
        .iter()
        .map(|(key, cursor)| (*key, cursor.next_raw_u64))
        .collect();
    streams.sort_by(|left, right| {
        left.0
            .to_canonical_bytes()
            .cmp(&right.0.to_canonical_bytes())
    });
    array([
        text(state.random.contract_id.clone()),
        Value::Bytes(state.random.root_seed.as_bytes().to_vec()),
        array(
            streams
                .into_iter()
                .map(|(key, cursor)| array([Value::Bytes(key.to_canonical_bytes()), u(cursor)])),
        ),
    ])
}

pub(crate) fn knowledge_value(state: &EngineState) -> Result<Value, StateDigestError> {
    let players = state
        .knowledge
        .players
        .iter()
        .map(|(player, knowledge)| {
            let active = knowledge
                .active
                .values()
                .map(active_knowledge_value)
                .collect::<Vec<_>>();
            let retired = knowledge
                .retired
                .values()
                .map(retired_knowledge_value)
                .collect::<Result<Vec<_>, _>>()?;
            Ok(array([
                u(player.0),
                u(knowledge.next_visible_sequence.0),
                array(active),
                array(retired),
            ]))
        })
        .collect::<Result<Vec<_>, StateDigestError>>()?;
    Ok(array(players))
}

fn active_knowledge_value(record: &KnowledgeRecordV2) -> Value {
    array([
        u(record.opaque_object.0),
        optional(record.physical_card.map(|value| u(value.0))),
        optional(record.card_definition.map(|value| u(value.0))),
        optional(record.known_location.as_ref().map(location_fact_value)),
        array(record.historical_locations.iter().map(location_fact_value)),
        provenance_value(&record.acquisition),
    ])
}

fn retired_knowledge_value(record: &RetiredKnowledgeRecordV2) -> Result<Value, StateDigestError> {
    Ok(array([
        u(record.opaque_object.0),
        optional(record.physical_card.map(|value| u(value.0))),
        optional(record.card_definition.map(|value| u(value.0))),
        optional(record.last_known_location.as_ref().map(location_fact_value)),
        array(record.historical_locations.iter().map(location_fact_value)),
        provenance_value(&record.acquisition),
        invalidation_value(&record.invalidation),
    ]))
}

fn location_fact_value(fact: &KnownLocationFactV2) -> Value {
    array([
        zone_location_value(&fact.location),
        provenance_value(&fact.provenance),
    ])
}

fn invalidation_value(invalidation: &KnowledgeInvalidationV2) -> Value {
    array([
        provenance_value(&invalidation.provenance),
        text(invalidation_reason(invalidation.reason)),
    ])
}

fn provenance_value(reason: &KnowledgeAcquisitionReason) -> Value {
    match reason {
        KnowledgeAcquisitionReason::InitialConfiguration => {
            array([text("initial_configuration"), Value::Null])
        }
        KnowledgeAcquisitionReason::Observed {
            channel,
            sequence,
            cause,
        } => array([
            text("observed"),
            array([
                text(channel_name(*channel)),
                u(sequence.0),
                text(cause_name(*cause)),
            ]),
        ]),
    }
}

fn channel_name(value: crate::knowledge::KnowledgeHistoryChannel) -> &'static str {
    match value {
        crate::knowledge::KnowledgeHistoryChannel::Public => "public",
        crate::knowledge::KnowledgeHistoryChannel::Private => "private",
    }
}

fn cause_name(value: crate::knowledge::KnowledgeAcquisitionCause) -> &'static str {
    match value {
        crate::knowledge::KnowledgeAcquisitionCause::PublicEvent => "public_event",
        crate::knowledge::KnowledgeAcquisitionCause::PrivateLook => "private_look",
        crate::knowledge::KnowledgeAcquisitionCause::ExplicitReveal => "explicit_reveal",
        crate::knowledge::KnowledgeAcquisitionCause::OwnPrivateIdentity => "own_private_identity",
    }
}

fn invalidation_reason(value: KnowledgeInvalidationReason) -> &'static str {
    match value {
        KnowledgeInvalidationReason::HiddenTransition => "hidden_transition",
        KnowledgeInvalidationReason::Randomization => "randomization",
        KnowledgeInvalidationReason::Shuffle => "shuffle",
        KnowledgeInvalidationReason::ExplicitForget => "explicit_forget",
    }
}

pub(crate) fn perspective_identities_value(state: &EngineState) -> Result<Value, StateDigestError> {
    let players = state
        .perspective_identities
        .players
        .iter()
        .map(|(player, record)| {
            let object_mappings = record
                .opaque_to_object
                .iter()
                .map(|(opaque, object)| array([u(opaque.0), u(object.0)]));
            let ability_mappings = record
                .opaque_to_ability
                .iter()
                .map(|(opaque, ability)| array([u(opaque.0), u(ability.0)]));
            Ok(array([
                u(player.0),
                array(object_mappings),
                array(ability_mappings),
                u(record.next_opaque_object_id.0),
                u(record.next_opaque_ability_id.0),
                u(record.next_player_decision_id.0),
                array(record.retired_object_ids.iter().map(|id| u(id.0))),
                array(record.retired_ability_ids.iter().map(|id| u(id.0))),
            ]))
        })
        .collect::<Result<Vec<_>, StateDigestError>>()?;
    Ok(array(players))
}

pub(crate) fn format_value(format: &FormatState) -> Result<Value, StateDigestError> {
    match format {
        FormatState::None => Ok(array([text("none"), Value::Null])),
        FormatState::Commander { state } => {
            let designations = state.designations.iter().map(|(player, cards)| {
                array([u(player.0), array(cards.iter().map(|card| u(card.0)))])
            });
            let cast_counts = state
                .cast_counts
                .iter()
                .map(|(card, count)| array([u(card.0), u32_value(*count)]));
            let damage = state.damage.iter().map(|(card, players)| {
                array([
                    u(card.0),
                    array(
                        players
                            .iter()
                            .map(|(player, value)| array([u(player.0), u32_value(*value)])),
                    ),
                ])
            });
            Ok(array([
                text("commander"),
                array([array(designations), array(cast_counts), array(damage)]),
            ]))
        }
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

#[cfg(test)]
mod block_declaration_codec_tests {
    use super::*;
    use mtgml_model::{GameObjectId, OpaqueObjectId, StateRevision};

    fn declaration(defender: u64, pending: &[u64], declared: &[(u64, Option<u64>)]) -> Value {
        continuation_value(&ContinuationRecord {
            id: mtgml_model::ContinuationId(1),
            created_at_revision: StateRevision(0),
            payload: ContinuationPayload::BlockDeclaration {
                defender: mtgml_model::PlayerId(defender),
                pending_blockers: pending.iter().copied().map(GameObjectId).collect(),
                declared: declared
                    .iter()
                    .map(|(blocker, attacker)| (GameObjectId(*blocker), attacker.map(GameObjectId)))
                    .collect(),
            },
        })
        .unwrap()
    }

    #[test]
    fn the_block_declaration_digest_binds_every_field() {
        let variants = [
            declaration(2, &[5, 6], &[]),
            declaration(1, &[5, 6], &[]),
            declaration(2, &[6, 5], &[]),
            declaration(2, &[5], &[]),
            declaration(2, &[6], &[(5, None)]),
            declaration(2, &[6], &[(5, Some(3))]),
            declaration(2, &[6], &[(5, Some(4))]),
            declaration(2, &[5], &[(6, None)]),
            declaration(2, &[], &[(5, None), (6, None)]),
            declaration(2, &[], &[(5, Some(3)), (6, None)]),
            declaration(2, &[], &[(5, None), (6, Some(3))]),
        ];
        for (index, variant) in variants.iter().enumerate() {
            for (other, other_variant) in variants.iter().enumerate().skip(index + 1) {
                assert_ne!(variant, other_variant, "{index} / {other}");
            }
        }
    }

    #[test]
    fn a_block_has_its_own_purpose_and_candidate_forms() {
        assert_eq!(
            decision_purpose_value(&DecisionPurposeV4::BlockerDeclaration),
            array([text("blocker_declaration")])
        );
        let intent = |attacker: Option<u64>| {
            candidate_intent_value(&CandidateIntent::DeclareBlock {
                blocker: OpaqueObjectId(7),
                attacker: attacker.map(OpaqueObjectId),
            })
        };
        let binding = |attacker: Option<u64>| {
            candidate_binding_value(&EngineCandidateBinding::DeclareBlock {
                blocker: GameObjectId(5),
                attacker: attacker.map(GameObjectId),
            })
        };
        assert_eq!(intent(Some(8)), array([text("declare_block"), u(7), u(8)]));
        assert_eq!(
            intent(None),
            array([text("declare_block"), u(7), Value::Null])
        );
        assert_ne!(intent(None), intent(Some(0)));
        assert_ne!(binding(None), binding(Some(0)));
        assert_eq!(binding(Some(3)), array([text("declare_block"), u(5), u(3)]));
    }
}

#[cfg(test)]
mod combat_damage_assignment_codec_tests {
    use super::*;
    use mtgml_model::{GameObjectId, OpaqueObjectId, StateRevision};

    fn division(
        player: u64,
        attackers: &[u64],
        blockers: &[u64],
        assigned: &[(u64, u64)],
    ) -> Value {
        continuation_value(&ContinuationRecord {
            id: mtgml_model::ContinuationId(1),
            created_at_revision: StateRevision(0),
            payload: ContinuationPayload::CombatDamageAssignment {
                player: mtgml_model::PlayerId(player),
                pending_attackers: attackers.iter().copied().map(GameObjectId).collect(),
                pending_blockers: blockers.iter().copied().map(GameObjectId).collect(),
                assigned: assigned
                    .iter()
                    .map(|(blocker, amount)| (GameObjectId(*blocker), *amount))
                    .collect(),
            },
        })
        .unwrap()
    }

    #[test]
    fn the_combat_damage_assignment_digest_binds_every_field() {
        let variants = [
            division(1, &[3], &[5, 6], &[]),
            division(2, &[3], &[5, 6], &[]),
            division(1, &[4], &[5, 6], &[]),
            division(1, &[3, 4], &[5, 6], &[]),
            division(1, &[4, 3], &[5, 6], &[]),
            division(1, &[3], &[6, 5], &[]),
            division(1, &[3], &[6], &[(5, 0)]),
            division(1, &[3], &[6], &[(5, 1)]),
            division(1, &[3], &[6], &[(5, 2)]),
            division(1, &[3], &[5], &[(6, 1)]),
            division(1, &[3], &[], &[(5, 1), (6, 1)]),
            division(1, &[3], &[], &[(5, 1), (6, 2)]),
            division(1, &[3], &[], &[(5, 2), (6, 1)]),
        ];
        for (index, variant) in variants.iter().enumerate() {
            for (other, other_variant) in variants.iter().enumerate().skip(index + 1) {
                assert_ne!(variant, other_variant, "{index} / {other}");
            }
        }
        // The pair is [blocker, amount], and the amount is a number.
        assert_eq!(
            division(1, &[3], &[6], &[(5, 2)]),
            array([
                u(1),
                u(0),
                array([
                    text("combat_damage_assignment"),
                    u(1),
                    array([u(3)]),
                    array([u(6)]),
                    array([array([u(5), u(2)])]),
                ])
            ])
        );
    }

    #[test]
    fn an_amount_of_combat_damage_has_its_own_purpose_and_candidate_forms() {
        assert_eq!(
            decision_purpose_value(&DecisionPurposeV4::CombatDamageAssignment),
            array([text("combat_damage_assignment")])
        );
        let intent = |amount: u64| {
            candidate_intent_value(&CandidateIntent::AssignCombatDamage {
                attacker: OpaqueObjectId(7),
                recipient: OpaqueObjectId(8),
                amount,
            })
        };
        let binding = |amount: u64| {
            candidate_binding_value(&EngineCandidateBinding::AssignCombatDamage {
                attacker: GameObjectId(3),
                recipient: GameObjectId(5),
                amount,
            })
        };
        assert_eq!(
            intent(2),
            array([text("assign_combat_damage"), u(7), u(8), u(2)])
        );
        assert_eq!(
            binding(2),
            array([text("assign_combat_damage"), u(3), u(5), u(2)])
        );
        assert_ne!(intent(0), intent(1));
        assert_ne!(binding(0), binding(1));
    }
}
