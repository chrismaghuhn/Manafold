//! Component encoders of the FullStateDigestV7 preimage that were introduced
//! with the V5 input (core, zones, allocators, random, knowledge, identities,
//! combat, foundation sources, format), plus the V3-execution encoders the V2
//! state parts still validate with.
//!
//! The conversion deliberately does not use Rust/Serde output. Every field is
//! mapped to the fixed semantic CBOR layout from STATE_HASHING.md.

use mtgml_persistence::cbor::{self, Value};

use crate::core::{BeginningStep, CombatStep, EndingStep, PriorityState, TurnPosition};
use crate::digest::StateDigestError;
use crate::engine::EngineState;
use crate::engine_state_shape::{
    KnowledgeInvalidationV2, KnowledgeRecordV2, KnownLocationFactV2, RetiredKnowledgeRecordV2,
};
use crate::format::FormatState;
use crate::knowledge::{KnowledgeAcquisitionReason, KnowledgeInvalidationReason};
use crate::zones::{VisibilityPartition, ZoneKey, ZoneLocation, ZonePosition};

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
    let blockers = combat
        .blockers
        .iter()
        .map(|(attacker, blocker)| array([u(attacker.0), optional(blocker.map(|id| u(id.0)))]));
    let live_blocked: std::collections::BTreeSet<_> = combat
        .blockers
        .iter()
        .filter_map(|(attacker, blocker)| blocker.map(|_| *attacker))
        .collect();
    let attackers = array(combat.attackers.iter().map(|object| u(object.0)));
    let blockers = array(blockers);
    if !combat.damage_step_completed && combat.blocked_attackers == live_blocked {
        // The 3-element form keeps the bytes of every CombatState that the
        // original three fields can represent. The extended form below is emitted only
        // when blocked history or completed-damage state adds information.
        array([u(combat.defending_player.0), attackers, blockers])
    } else {
        array([
            u(combat.defending_player.0),
            attackers,
            blockers,
            array(combat.blocked_attackers.iter().map(|object| u(object.0))),
            Value::Bool(combat.damage_step_completed),
        ])
    }
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
