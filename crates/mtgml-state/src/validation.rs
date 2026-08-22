use std::collections::BTreeSet;

use mtgml_decision::{
    validate_candidate_binding, ActionCandidate, EngineCandidateBinding, VisibleCandidateV2,
};
use mtgml_model::{PhysicalCardId, PlayerId};
use thiserror::Error;

use crate::engine::EngineState;
use crate::format::FormatState;
use crate::knowledge::KnowledgeAcquisitionReason;
use crate::m2_shape::{validate_m2_shape, KnownLocationFactV2, PlayerKnowledgeStateV2};
use crate::zones::ZonePosition;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EngineStateViolation {
    #[error("active or priority player is absent")]
    MissingTurnPlayer,
    #[error("object map key does not equal object identity")]
    ObjectKeyMismatch,
    #[error("object owner/controller or zone player is absent")]
    ObjectPlayerMismatch,
    #[error("objects and locations are not bijective")]
    ObjectLocationMismatch,
    #[error("a physical card identifies more than one live game-object incarnation")]
    DuplicatePhysicalCard,
    #[error("ordered zones contain a missing, duplicated, or wrongly located object")]
    OrderedZoneMismatch,
    #[error("stack records and stack order are not bijective")]
    StackMismatch,
    #[error("an identity allocator does not exceed every allocated identity")]
    AllocatorBehind,
    #[error("pending decision is invalid for this state")]
    PendingDecisionMismatch,
    #[error("continuation reference is missing")]
    MissingContinuation,
    #[error("execution record keys do not match their embedded identities")]
    ExecutionMismatch,
    #[error("perspective identities are not bijective or reference missing objects")]
    PerspectiveIdentityMismatch,
    #[error("knowledge state references an absent player/object or has invalid provenance")]
    KnowledgeMismatch,
    #[error("format state references absent players or undesignated commanders")]
    FormatMismatch,
    #[error("random state is invalid")]
    RandomState,
    #[error("M2 state shape is invalid: {0}")]
    M2Shape(#[from] crate::m2_shape::M2ShapeViolation),
}

fn provenance_is_valid(
    provenance: &KnowledgeAcquisitionReason,
    knowledge: &PlayerKnowledgeStateV2,
) -> bool {
    provenance.has_accepted_channel_cause()
        && provenance.is_within_visible_sequence(knowledge.next_visible_sequence)
}

fn fact_is_valid(fact: &KnownLocationFactV2, knowledge: &PlayerKnowledgeStateV2) -> bool {
    provenance_is_valid(&fact.provenance, knowledge)
}

pub fn validate_engine_state(state: &EngineState) -> Result<(), EngineStateViolation> {
    if !state.core.players.contains_key(&state.core.active_player)
        || !state.core.players.contains_key(&state.core.priority_player)
    {
        return Err(EngineStateViolation::MissingTurnPlayer);
    }
    if state
        .zones
        .objects
        .iter()
        .any(|(id, object)| id != &object.id)
    {
        return Err(EngineStateViolation::ObjectKeyMismatch);
    }
    if state.zones.objects.values().any(|object| {
        !state.core.players.contains_key(&object.owner)
            || !state.core.players.contains_key(&object.controller)
    }) || state
        .zones
        .locations
        .values()
        .filter_map(|location| location.player)
        .any(|player| !state.core.players.contains_key(&player))
    {
        return Err(EngineStateViolation::ObjectPlayerMismatch);
    }
    let mut live_physical_cards = BTreeSet::<PhysicalCardId>::new();
    for object in state.zones.objects.values() {
        if let Some(physical_card) = object.physical_card {
            if !live_physical_cards.insert(physical_card) {
                return Err(EngineStateViolation::DuplicatePhysicalCard);
            }
        }
    }
    let object_ids: BTreeSet<_> = state.zones.objects.keys().copied().collect();
    let location_ids: BTreeSet<_> = state.zones.locations.keys().copied().collect();
    if object_ids != location_ids {
        return Err(EngineStateViolation::ObjectLocationMismatch);
    }
    let expected_ordered: BTreeSet<_> = state
        .zones
        .locations
        .iter()
        .filter_map(|(object, location)| {
            (!matches!(location.position, ZonePosition::Unordered)).then_some(*object)
        })
        .collect();
    let mut ordered_seen = BTreeSet::new();
    for (key, objects) in &state.zones.ordered_zones {
        for object in objects {
            let Some(location) = state.zones.locations.get(object) else {
                return Err(EngineStateViolation::OrderedZoneMismatch);
            };
            if matches!(location.position, ZonePosition::Unordered)
                || &location.key() != key
                || !ordered_seen.insert(*object)
            {
                return Err(EngineStateViolation::OrderedZoneMismatch);
            }
        }
    }
    if ordered_seen != expected_ordered {
        return Err(EngineStateViolation::OrderedZoneMismatch);
    }
    let stack_record_ids: BTreeSet<_> = state.zones.stack_records.keys().copied().collect();
    let stack_order_ids: BTreeSet<_> = state.zones.stack_order.iter().copied().collect();
    if stack_record_ids != stack_order_ids || stack_order_ids.len() != state.zones.stack_order.len()
    {
        return Err(EngineStateViolation::StackMismatch);
    }
    if state.zones.stack_records.iter().any(|(id, record)| {
        id != &record.id
            || !state.core.players.contains_key(&record.controller)
            || record
                .source_object
                .is_some_and(|object| !state.zones.objects.contains_key(&object))
    }) {
        return Err(EngineStateViolation::StackMismatch);
    }

    let max_object = state.zones.objects.keys().map(|id| id.0).max().unwrap_or(0);
    let max_stack = state
        .zones
        .stack_records
        .keys()
        .map(|id| id.0)
        .max()
        .unwrap_or(0);
    let max_effect = state
        .execution
        .effects
        .keys()
        .chain(state.execution.delayed_effects.keys())
        .map(|id| id.0)
        .max()
        .unwrap_or(0);
    let max_trigger = state
        .execution
        .waiting_triggers
        .keys()
        .map(|id| id.0)
        .max()
        .unwrap_or(0);
    let max_continuation = state
        .execution
        .continuations
        .keys()
        .map(|id| id.0)
        .max()
        .unwrap_or(0);
    if state.allocators.next_object_id.0 <= max_object
        || state.allocators.next_stack_object_id.0 <= max_stack
        || state.allocators.next_effect_id.0 <= max_effect
        || state.allocators.next_trigger_id.0 <= max_trigger
        || state.allocators.next_continuation_id.0 <= max_continuation
        || state.allocators.next_rule_event_id.0 == 0
    {
        return Err(EngineStateViolation::AllocatorBehind);
    }

    // Trusted decision identities are issued to authoritative pending
    // requests; every issued trusted identity must stay strictly below the
    // global allocator cursor.
    let issued_decision_id = state
        .execution
        .pending_decision
        .as_ref()
        .map(|record| record.request.decision_id.0)
        .unwrap_or(0);
    if state.allocators.next_decision_id.0 <= issued_decision_id {
        return Err(EngineStateViolation::AllocatorBehind);
    }
    // Trusted ability identities are reachable through stack records and the
    // perspective-local opaque ability mappings.
    let issued_ability_id = state
        .zones
        .stack_records
        .values()
        .filter_map(|record| record.source_ability)
        .chain(
            state
                .perspective_identities
                .players
                .values()
                .flat_map(|identity| identity.opaque_to_ability.values().copied()),
        )
        .map(|ability| ability.0)
        .max()
        .unwrap_or(0);
    if state.allocators.next_ability_id.0 <= issued_ability_id {
        return Err(EngineStateViolation::AllocatorBehind);
    }
    if state.execution.effects.is_empty()
        && state.execution.waiting_triggers.is_empty()
        && state.execution.delayed_effects.is_empty()
    {
        // M2.B explicitly has no executable effect/trigger machinery.
    } else {
        return Err(EngineStateViolation::ExecutionMismatch);
    }
    if state
        .execution
        .continuations
        .iter()
        .any(|(id, record)| id != &record.id || !state.core.players.contains_key(&record.actor))
    {
        return Err(EngineStateViolation::ExecutionMismatch);
    }

    let players: BTreeSet<_> = state.core.players.keys().copied().collect();
    validate_m2_shape(
        state.revision,
        &players,
        state.execution.pending_decision.as_ref(),
        &state.execution.continuations,
        &state.knowledge,
        &state.perspective_identities,
    )?;

    for player in &players {
        let identity = state
            .perspective_identities
            .players
            .get(player)
            .ok_or(EngineStateViolation::PerspectiveIdentityMismatch)?;
        let knowledge = state
            .knowledge
            .players
            .get(player)
            .ok_or(EngineStateViolation::KnowledgeMismatch)?;
        for (opaque, record) in &knowledge.active {
            let object = identity
                .opaque_to_object
                .get(opaque)
                .ok_or(EngineStateViolation::KnowledgeMismatch)?;
            let live = state
                .zones
                .objects
                .get(object)
                .ok_or(EngineStateViolation::KnowledgeMismatch)?;
            let known_fact_matches_live = record
                .known_location
                .as_ref()
                .is_some_and(|fact| state.zones.locations.get(object) != Some(&fact.location));
            let observed: Vec<_> = record
                .historical_locations
                .iter()
                .filter_map(|fact| {
                    fact.provenance
                        .observed_sequence()
                        .map(|sequence| sequence.0)
                })
                .collect();
            let history_is_increasing = observed.windows(2).any(|window| window[0] >= window[1]);
            if known_fact_matches_live
                || record
                    .physical_card
                    .is_some_and(|physical| Some(physical) != live.physical_card)
                || record
                    .card_definition
                    .is_some_and(|definition| definition != live.card_definition)
                || !provenance_is_valid(&record.acquisition, knowledge)
                || record
                    .known_location
                    .as_ref()
                    .is_some_and(|fact| !fact_is_valid(fact, knowledge))
                || !record
                    .historical_locations
                    .iter()
                    .all(|fact| fact_is_valid(fact, knowledge))
                || history_is_increasing
            {
                return Err(EngineStateViolation::KnowledgeMismatch);
            }
        }
        for record in knowledge.retired.values() {
            if identity
                .opaque_to_object
                .contains_key(&record.opaque_object)
                || !provenance_is_valid(&record.acquisition, knowledge)
                || !provenance_is_valid(&record.invalidation.provenance, knowledge)
                || record
                    .last_known_location
                    .as_ref()
                    .is_some_and(|fact| !fact_is_valid(fact, knowledge))
                || !record
                    .historical_locations
                    .iter()
                    .all(|fact| fact_is_valid(fact, knowledge))
                || {
                    let observed: Vec<_> = record
                        .historical_locations
                        .iter()
                        .filter_map(|fact| {
                            fact.provenance
                                .observed_sequence()
                                .map(|sequence| sequence.0)
                        })
                        .collect();
                    observed.windows(2).any(|window| window[0] >= window[1])
                }
            {
                return Err(EngineStateViolation::KnowledgeMismatch);
            }
        }
    }
    if let Some(pending) = &state.execution.pending_decision {
        let request = &pending.request;
        request
            .validate()
            .map_err(|_| EngineStateViolation::PendingDecisionMismatch)?;
        if request.state_revision != state.revision
            || !state.core.players.contains_key(&request.actor)
            || state
                .perspective_identities
                .players
                .get(&request.actor)
                .is_none_or(|identity| {
                    identity.next_player_decision_id.0 <= request.player_decision_id.0
                })
        {
            return Err(EngineStateViolation::PendingDecisionMismatch);
        }
        for candidate in &request.candidates {
            let visible = ActionCandidate {
                candidate_id: candidate.candidate_id.to_string(),
                semantic_key: format!("candidate.{}", candidate.candidate_id.0),
                intent: candidate.visible_intent.clone(),
            };
            if !candidate
                .trusted_binding
                .same_variant_as(&candidate.visible_intent)
                || validate_candidate_binding(
                    &visible,
                    &candidate.trusted_binding,
                    request.actor,
                    &state.perspective_identities,
                )
                .is_err()
            {
                return Err(EngineStateViolation::PendingDecisionMismatch);
            }
        }
    }

    for (player, identities) in &state.perspective_identities.players {
        if !state.core.players.contains_key(player)
            || identities.object_to_opaque.len() != identities.opaque_to_object.len()
            || identities.ability_to_opaque.len() != identities.opaque_to_ability.len()
        {
            return Err(EngineStateViolation::PerspectiveIdentityMismatch);
        }
        for (opaque, object) in &identities.opaque_to_object {
            if identities.object_to_opaque.get(object) != Some(opaque)
                || !state.zones.objects.contains_key(object)
                || identities.retired_object_ids.contains(opaque)
            {
                return Err(EngineStateViolation::PerspectiveIdentityMismatch);
            }
        }
        for (object, opaque) in &identities.object_to_opaque {
            if identities.opaque_to_object.get(opaque) != Some(object) {
                return Err(EngineStateViolation::PerspectiveIdentityMismatch);
            }
        }
        for (opaque, ability) in &identities.opaque_to_ability {
            if identities.ability_to_opaque.get(ability) != Some(opaque)
                || identities.retired_ability_ids.contains(opaque)
            {
                return Err(EngineStateViolation::PerspectiveIdentityMismatch);
            }
        }
        if identities
            .opaque_to_object
            .keys()
            .any(|id| id.0 >= identities.next_opaque_object_id.0)
            || identities
                .opaque_to_ability
                .keys()
                .any(|id| id.0 >= identities.next_opaque_ability_id.0)
            || identities
                .retired_object_ids
                .iter()
                .any(|id| id.0 >= identities.next_opaque_object_id.0)
            || identities
                .retired_ability_ids
                .iter()
                .any(|id| id.0 >= identities.next_opaque_ability_id.0)
        {
            return Err(EngineStateViolation::AllocatorBehind);
        }
    }

    if let FormatState::Commander { state: commander } = &state.format {
        let mut designated = BTreeSet::new();
        for (player, cards) in &commander.designations {
            if !state.core.players.contains_key(player) || cards.is_empty() {
                return Err(EngineStateViolation::FormatMismatch);
            }
            if cards.iter().any(|card| !designated.insert(*card)) {
                return Err(EngineStateViolation::FormatMismatch);
            }
            if cards.iter().any(|card| {
                !state
                    .zones
                    .objects
                    .values()
                    .any(|object| object.physical_card == Some(*card) && object.owner == *player)
            }) {
                return Err(EngineStateViolation::FormatMismatch);
            }
        }
        if commander
            .cast_counts
            .keys()
            .chain(commander.damage.keys())
            .any(|card| !designated.contains(card))
        {
            return Err(EngineStateViolation::FormatMismatch);
        }
        if commander.damage.values().any(|targets| {
            targets
                .keys()
                .any(|player| !state.core.players.contains_key(player))
        }) {
            return Err(EngineStateViolation::FormatMismatch);
        }
    }

    state
        .random
        .validate()
        .map_err(|_| EngineStateViolation::RandomState)?;
    for key in state.random.streams.keys() {
        if let Some(player_raw) = key.player() {
            if !state.core.players.contains_key(&PlayerId(player_raw)) {
                return Err(EngineStateViolation::RandomState);
            }
        }
    }
    Ok(())
}

#[allow(dead_code)]
fn _binding_type_marker(_: &EngineCandidateBinding, _: &VisibleCandidateV2) {}
