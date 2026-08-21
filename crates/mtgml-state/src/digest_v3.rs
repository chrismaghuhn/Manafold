//! The single typed producer for the current full-state V3 digest.
//!
//! The conversion below deliberately does not use Rust/Serde output.  Every
//! field is mapped to the fixed semantic CBOR layout from STATE_HASHING.md.

use std::cmp::Ordering;

use mtgml_decision::{
    AuthoritativeCandidateV2, AuthoritativeDecisionRequestV2, CandidateIntent, DecisionDomainV2,
    DecisionVisibility, EngineCandidateBinding,
};
use mtgml_model::FullStateDigestV3;
use mtgml_persistence::{
    cbor::{self, Value},
    envelope, PersistenceDecodeErrorV1,
};
use mtgml_random::{RandomStreamKeyV1, RandomStreamScopeV1};

use crate::digest::StateDigestError;
use crate::engine::EngineState;
use crate::format::FormatState;
use crate::knowledge::{
    KnowledgeAcquisitionCause, KnowledgeAcquisitionReason, KnowledgeHistoryChannel,
    KnowledgeInvalidationReason, KnowledgePoint,
};
use crate::m2_shape::{
    AssemblyStageV2, ContinuationPayloadV2, KnowledgeRecordV2, RetiredKnowledgeRecordV2,
};
use crate::zones::{VisibilityPartition, ZoneKey, ZoneLocation, ZonePosition};

pub const FULL_STATE_DIGEST_DOMAIN_V3: &str = "mtgml.full-state-digest.v3";
pub const FULL_STATE_DIGEST_INPUT_SCHEMA_V3: &str = "full-state-digest-input.v3";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FullStateDigestInputV3 {
    pub revision: u64,
    pub core: Value,
    pub zones: Value,
    pub allocators: Value,
    pub execution: Value,
    pub random: Value,
    pub knowledge: Value,
    pub perspective_identities: Value,
    pub format: Value,
}

impl FullStateDigestInputV3 {
    pub fn canonical_value(&self) -> Result<Value, StateDigestError> {
        let Value::Array(execution) = &self.execution else {
            return Err(semantic_error());
        };
        if execution.len() != 5
            || !matches!(execution.get(2), Some(Value::Array(values)) if values.is_empty())
            || !matches!(execution.get(3), Some(Value::Array(values)) if values.is_empty())
            || !matches!(execution.get(4), Some(Value::Array(values)) if values.is_empty())
        {
            return Err(semantic_error());
        }
        Ok(Value::Array(vec![
            Value::Text(FULL_STATE_DIGEST_INPUT_SCHEMA_V3.to_owned()),
            Value::Text(FULL_STATE_DIGEST_DOMAIN_V3.to_owned()),
            Value::Unsigned(self.revision),
            self.core.clone(),
            self.zones.clone(),
            self.allocators.clone(),
            self.execution.clone(),
            self.random.clone(),
            self.knowledge.clone(),
            self.perspective_identities.clone(),
            self.format.clone(),
        ]))
    }

    pub fn canonical_payload(&self) -> Result<Vec<u8>, StateDigestError> {
        cbor::encode_canonical(&self.canonical_value()?).map_err(StateDigestError::Persistence)
    }
}

pub(crate) fn calculate_full_state_digest_v3(
    input: &FullStateDigestInputV3,
) -> Result<FullStateDigestV3, StateDigestError> {
    let payload = input.canonical_payload()?;
    let envelope = envelope::encode_envelope(
        FULL_STATE_DIGEST_DOMAIN_V3,
        FULL_STATE_DIGEST_INPUT_SCHEMA_V3,
        &payload,
    )
    .map_err(StateDigestError::Persistence)?;
    Ok(FullStateDigestV3::from_digest_bytes(
        envelope::hash_envelope(&envelope),
    ))
}

pub(crate) fn calculate_full_state_digest_v3_for_state(
    state: &EngineState,
) -> Result<FullStateDigestV3, StateDigestError> {
    calculate_full_state_digest_v3(&full_state_digest_input(state)?)
}

pub(crate) fn full_state_digest_input(
    state: &EngineState,
) -> Result<FullStateDigestInputV3, StateDigestError> {
    crate::validation::validate_engine_state(state)
        .map_err(|_| StateDigestError::StateInvariant)?;
    Ok(FullStateDigestInputV3 {
        revision: state.revision.0,
        core: core_value(state),
        zones: zones_value(state)?,
        allocators: allocators_value(state),
        execution: execution_value(state)?,
        random: random_value(state),
        knowledge: knowledge_value(state)?,
        perspective_identities: perspective_identities_value(state)?,
        format: format_value(&state.format)?,
    })
}

fn semantic_error() -> StateDigestError {
    StateDigestError::Persistence(PersistenceDecodeErrorV1::SemanticValidation)
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

fn core_value(state: &EngineState) -> Value {
    let players =
        state.core.players.iter().map(|(player, value)| {
            array([u(player.0), i(value.life), Value::Bool(value.has_lost)])
        });
    array([
        array(players),
        u(state.core.active_player.0),
        u(state.core.priority_player.0),
        u(state.core.turn_number),
    ])
}

fn zones_value(state: &EngineState) -> Result<Value, StateDigestError> {
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

    let stack_records = state.zones.stack_records.values().map(|record| {
        array([
            u(record.id.0),
            u(record.controller.0),
            optional(record.source_object.map(|value| u(value.0))),
            optional(record.source_ability.map(|value| u(value.0))),
        ])
    });
    Ok(array([
        array(objects),
        array(locations),
        array(ordered.into_iter().map(|(_, value)| value)),
        array(stack_records),
        array(state.zones.stack_order.iter().map(|id| u(id.0))),
    ]))
}

fn zone_key_value(key: &ZoneKey) -> Value {
    array([
        text(zone_kind(key.zone)),
        optional(key.player.map(|player| u(player.0))),
        text(visibility(key.visibility)),
        optional(key.partition.clone().map(Value::Text)),
    ])
}

fn zone_location_value(location: &ZoneLocation) -> Value {
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

fn allocators_value(state: &EngineState) -> Value {
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

fn execution_value(state: &EngineState) -> Result<Value, StateDigestError> {
    let pending = state
        .execution
        .pending_decision
        .as_ref()
        .map(|pending| decision_value(&pending.request));
    let continuations = state
        .execution
        .continuations
        .values()
        .map(continuation_value);
    if !state.execution.effects.is_empty()
        || !state.execution.waiting_triggers.is_empty()
        || !state.execution.delayed_effects.is_empty()
    {
        return Err(semantic_error());
    }
    Ok(array([
        optional(pending),
        array(continuations),
        array([]),
        array([]),
        array([]),
    ]))
}

fn decision_value(request: &AuthoritativeDecisionRequestV2) -> Value {
    array([
        u(request.decision_id.0),
        u(request.player_decision_id.0),
        u(request.state_revision.0),
        u(request.actor.0),
        text(decision_visibility(request.visibility)),
        decision_domain(&request.decision),
        array(request.candidates.iter().map(candidate_value)),
        optional(request.continuation_id.map(|value| u(value.0))),
    ])
}

fn decision_visibility(value: DecisionVisibility) -> &'static str {
    match value {
        DecisionVisibility::Public => "public",
        DecisionVisibility::ActingPlayerOnly => "acting_player_only",
        DecisionVisibility::Mixed => "mixed",
    }
}

fn decision_domain(value: &DecisionDomainV2) -> Value {
    match value {
        DecisionDomainV2::ChooseOne => array([text("choose_one"), Value::Null]),
        DecisionDomainV2::ChooseMany { minimum, maximum } => array([
            text("choose_many"),
            array([u32_value(*minimum), u32_value(*maximum)]),
        ]),
        DecisionDomainV2::ChooseNumber { minimum, maximum } => {
            array([text("choose_number"), array([i(*minimum), i(*maximum)])])
        }
        DecisionDomainV2::Order { minimum, maximum } => array([
            text("order"),
            array([u32_value(*minimum), u32_value(*maximum)]),
        ]),
    }
}

fn candidate_value(candidate: &AuthoritativeCandidateV2) -> Value {
    array([
        u32_value(candidate.candidate_id.0),
        visible_intent(&candidate.visible_intent),
        trusted_binding(&candidate.trusted_binding),
    ])
}

fn visible_intent(value: &CandidateIntent) -> Value {
    match value {
        CandidateIntent::PassPriority => array([text("pass_priority"), Value::Null]),
        CandidateIntent::CastSpell { object } => array([text("cast_spell"), u(object.0)]),
        CandidateIntent::ActivateAbility { ability } => {
            array([text("activate_ability"), u(ability.0)])
        }
        CandidateIntent::SelectObject { object } => array([text("select_object"), u(object.0)]),
        CandidateIntent::SelectPlayer { player } => array([text("select_player"), u(player.0)]),
        CandidateIntent::SelectMode { mode_index } => {
            array([text("select_mode"), u32_value(*mode_index)])
        }
        CandidateIntent::ChooseBoolean { value } => {
            array([text("choose_boolean"), Value::Bool(*value)])
        }
        CandidateIntent::DeclareNumber { value } => array([text("declare_number"), i(*value)]),
        CandidateIntent::Confirm => array([text("confirm"), Value::Null]),
    }
}

fn trusted_binding(value: &EngineCandidateBinding) -> Value {
    match value {
        EngineCandidateBinding::PassPriority => array([text("pass_priority"), Value::Null]),
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
            array([text("select_mode"), u32_value(*mode_index)])
        }
        EngineCandidateBinding::ChooseBoolean { value } => {
            array([text("choose_boolean"), Value::Bool(*value)])
        }
        EngineCandidateBinding::DeclareNumber { value } => {
            array([text("declare_number"), i(*value)])
        }
        EngineCandidateBinding::Confirm => array([text("confirm"), Value::Null]),
    }
}

fn continuation_value(record: &crate::m2_shape::ContinuationRecordV2) -> Value {
    let payload = match &record.payload {
        ContinuationPayloadV2::SyntheticM2Assembly {
            stage,
            selected_count,
            selected_piece_keys,
            ordered_piece_keys,
        } => array([
            text("synthetic_m2_assembly"),
            array([
                array([text(assembly_stage(*stage)), Value::Null]),
                optional(selected_count.map(u32_value)),
                array(selected_piece_keys.iter().copied().map(u32_value)),
                array(ordered_piece_keys.iter().copied().map(u32_value)),
            ]),
        ]),
    };
    array([
        u(record.id.0),
        u(record.actor.0),
        u(record.created_at_revision.0),
        u(u64::from(record.stage_index)),
        payload,
    ])
}

fn assembly_stage(value: AssemblyStageV2) -> &'static str {
    match value {
        AssemblyStageV2::ChooseCount => "choose_count",
        AssemblyStageV2::ChooseMembers => "choose_members",
        AssemblyStageV2::OrderMembers => "order_members",
    }
}

fn random_value(state: &EngineState) -> Value {
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

fn knowledge_value(state: &EngineState) -> Result<Value, StateDigestError> {
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
        optional(record.known_location.as_ref().map(|location| {
            array([
                zone_location_value(location),
                provenance_value(&record.learned_at, &record.learned_via),
            ])
        })),
        array(record.historical_locations.iter().map(history_value)),
        provenance_value(&record.learned_at, &record.learned_via),
    ])
}

fn retired_knowledge_value(record: &RetiredKnowledgeRecordV2) -> Result<Value, StateDigestError> {
    Ok(array([
        u(record.opaque_object.0),
        optional(record.physical_card.map(|value| u(value.0))),
        optional(record.card_definition.map(|value| u(value.0))),
        optional(record.last_known_location.as_ref().map(history_value)),
        array(record.historical_locations.iter().map(history_value)),
        provenance_value(&record.learned_at, &record.learned_via),
        array([
            provenance_value(
                &record.invalidated_at,
                &KnowledgeAcquisitionReason::Observed {
                    channel: record.invalidated_at.channel,
                    sequence: record.invalidated_at.sequence,
                    cause: KnowledgeAcquisitionCause::ExplicitReveal,
                },
            ),
            text(invalidation_reason(record.reason.clone())),
        ]),
    ]))
}

fn history_value(record: &crate::m2_shape::KnowledgeHistoryRecordV2) -> Value {
    array([
        optional(record.location.as_ref().map(zone_location_value)),
        provenance_value(
            &record.observed_at,
            &KnowledgeAcquisitionReason::Observed {
                channel: record.observed_at.channel,
                sequence: record.observed_at.sequence,
                cause: KnowledgeAcquisitionCause::PublicEvent,
            },
        ),
    ])
}

fn provenance_value(point: &KnowledgePoint, reason: &KnowledgeAcquisitionReason) -> Value {
    match reason {
        KnowledgeAcquisitionReason::InitialConfiguration => {
            array([text("initial_configuration"), Value::Null])
        }
        KnowledgeAcquisitionReason::Observed { cause, .. } => array([
            text("observed"),
            array([
                text(channel(point.channel)),
                u(point.sequence.0),
                text(cause_name(cause.clone())),
            ]),
        ]),
    }
}

fn channel(value: KnowledgeHistoryChannel) -> &'static str {
    match value {
        KnowledgeHistoryChannel::Public => "public",
        KnowledgeHistoryChannel::Private => "private",
    }
}

fn cause_name(value: KnowledgeAcquisitionCause) -> &'static str {
    match value {
        KnowledgeAcquisitionCause::PublicEvent => "public_event",
        KnowledgeAcquisitionCause::PrivateLook => "private_look",
        KnowledgeAcquisitionCause::ExplicitReveal => "explicit_reveal",
        KnowledgeAcquisitionCause::OwnPrivateIdentity => "own_private_identity",
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

fn perspective_identities_value(state: &EngineState) -> Result<Value, StateDigestError> {
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

fn format_value(format: &FormatState) -> Result<Value, StateDigestError> {
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

#[allow(dead_code)]
fn compare_cbor(left: &Value, right: &Value) -> Ordering {
    let left = cbor::encode_canonical(left).expect("semantic V3 values are encodable");
    let right = cbor::encode_canonical(right).expect("semantic V3 values are encodable");
    left.cmp(&right)
}

#[allow(dead_code)]
fn _identity_key(_key: &RandomStreamKeyV1) -> Option<RandomStreamScopeV1> {
    None
}
