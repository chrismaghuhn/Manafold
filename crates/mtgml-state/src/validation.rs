use std::collections::BTreeSet;

use mtgml_decision::{validate_candidate_binding, EngineCandidateBinding};
use mtgml_model::{PhysicalCardId, PlayerId, RuleEventId};
use thiserror::Error;

use crate::engine::EngineState;
use crate::format::FormatState;
use crate::knowledge::{
    KnowledgeAcquisitionReason, KnowledgeHistoryChannel, KnowledgePoint, PlayerKnowledgeState,
};
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
    #[error("an opaque identity allocator references an absent player")]
    AllocatorPlayerMismatch,
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
}

fn knowledge_point_is_valid(point: KnowledgePoint, state: &PlayerKnowledgeState) -> bool {
    match point.channel {
        KnowledgeHistoryChannel::Public => point.sequence.0 <= state.public_history_length,
        KnowledgeHistoryChannel::Private => point.sequence.0 <= state.private_history_length,
    }
}

fn acquisition_matches_channel(
    reason: &KnowledgeAcquisitionReason,
    channel: KnowledgeHistoryChannel,
) -> bool {
    match reason {
        KnowledgeAcquisitionReason::PublicEvent { .. }
        | KnowledgeAcquisitionReason::ExplicitReveal => channel == KnowledgeHistoryChannel::Public,
        KnowledgeAcquisitionReason::PrivateEvent { .. }
        | KnowledgeAcquisitionReason::OwnZoneIdentity => {
            channel == KnowledgeHistoryChannel::Private
        }
        KnowledgeAcquisitionReason::InitialConfiguration => true,
    }
}

fn knowledge_event_is_from_the_future(
    reason: &KnowledgeAcquisitionReason,
    next_rule_event_id: RuleEventId,
) -> bool {
    match reason {
        KnowledgeAcquisitionReason::PublicEvent { event }
        | KnowledgeAcquisitionReason::PrivateEvent { event } => event.0 >= next_rule_event_id.0,
        KnowledgeAcquisitionReason::InitialConfiguration
        | KnowledgeAcquisitionReason::OwnZoneIdentity
        | KnowledgeAcquisitionReason::ExplicitReveal => false,
    }
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
    if state
        .execution
        .continuations
        .iter()
        .any(|(id, record)| id != &record.id)
        || state
            .execution
            .effects
            .iter()
            .any(|(id, record)| id != &record.id)
        || state
            .execution
            .delayed_effects
            .iter()
            .any(|(id, record)| id != &record.id)
        || state.execution.waiting_triggers.iter().any(|(id, record)| {
            id != &record.id || !state.core.players.contains_key(&record.controller)
        })
    {
        return Err(EngineStateViolation::ExecutionMismatch);
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
    let max_ability = state
        .zones
        .stack_records
        .values()
        .filter_map(|record| record.source_ability)
        .map(|id| id.0)
        .chain(
            state
                .perspective_identities
                .players
                .values()
                .flat_map(|identities| identities.ability_to_opaque.keys().map(|id| id.0)),
        )
        .chain(
            state
                .execution
                .pending_decision
                .iter()
                .flat_map(|pending| pending.candidate_bindings.values())
                .filter_map(|binding| match binding {
                    EngineCandidateBinding::ActivateAbility { ability } => Some(ability.0),
                    _ => None,
                }),
        )
        .max()
        .unwrap_or(0);
    let pending_decision_id = state
        .execution
        .pending_decision
        .as_ref()
        .map(|record| record.request.decision_id.0)
        .unwrap_or(0);
    if state.allocators.next_object_id.0 <= max_object
        || state.allocators.next_ability_id.0 <= max_ability
        || state.allocators.next_stack_object_id.0 <= max_stack
        || state.allocators.next_effect_id.0 <= max_effect
        || state.allocators.next_trigger_id.0 <= max_trigger
        || state.allocators.next_continuation_id.0 <= max_continuation
        || state.allocators.next_decision_id.0 <= pending_decision_id
        || state.allocators.next_rule_event_id.0 == 0
    {
        return Err(EngineStateViolation::AllocatorBehind);
    }
    if state
        .allocators
        .next_opaque_object_id
        .keys()
        .chain(state.allocators.next_opaque_ability_id.keys())
        .any(|player| !state.core.players.contains_key(player))
    {
        return Err(EngineStateViolation::AllocatorPlayerMismatch);
    }

    if let Some(pending) = &state.execution.pending_decision {
        pending
            .request
            .validate()
            .map_err(|_| EngineStateViolation::PendingDecisionMismatch)?;
        if pending.request.state_revision != state.revision
            || !state.core.players.contains_key(&pending.request.actor)
        {
            return Err(EngineStateViolation::PendingDecisionMismatch);
        }
        let candidate_ids: BTreeSet<_> = pending
            .request
            .candidates
            .iter()
            .map(|candidate| candidate.candidate_id.as_str())
            .collect();
        let binding_ids: BTreeSet<_> = pending
            .candidate_bindings
            .keys()
            .map(String::as_str)
            .collect();
        if candidate_ids != binding_ids {
            return Err(EngineStateViolation::PendingDecisionMismatch);
        }
        for candidate in &pending.request.candidates {
            let Some(binding) = pending.candidate_bindings.get(&candidate.candidate_id) else {
                return Err(EngineStateViolation::PendingDecisionMismatch);
            };
            if validate_candidate_binding(
                candidate,
                binding,
                pending.request.actor,
                &state.perspective_identities,
            )
            .is_err()
            {
                return Err(EngineStateViolation::PendingDecisionMismatch);
            }
        }
        if let Some(continuation) = pending.continuation {
            if !state.execution.continuations.contains_key(&continuation) {
                return Err(EngineStateViolation::MissingContinuation);
            }
        }
    }

    for player in state.core.players.keys() {
        if !state.perspective_identities.players.contains_key(player)
            || !state.knowledge.players.contains_key(player)
        {
            return Err(EngineStateViolation::PerspectiveIdentityMismatch);
        }
    }
    for (player, identities) in &state.perspective_identities.players {
        if !state.core.players.contains_key(player)
            || identities.object_to_opaque.len() != identities.opaque_to_object.len()
            || identities.ability_to_opaque.len() != identities.opaque_to_ability.len()
        {
            return Err(EngineStateViolation::PerspectiveIdentityMismatch);
        }
        for (object, opaque) in &identities.object_to_opaque {
            if !state.zones.objects.contains_key(object)
                || identities.opaque_to_object.get(opaque) != Some(object)
            {
                return Err(EngineStateViolation::PerspectiveIdentityMismatch);
            }
        }
        for (opaque, object) in &identities.opaque_to_object {
            if identities.object_to_opaque.get(object) != Some(opaque) {
                return Err(EngineStateViolation::PerspectiveIdentityMismatch);
            }
        }
        for (ability, opaque) in &identities.ability_to_opaque {
            if identities.opaque_to_ability.get(opaque) != Some(ability) {
                return Err(EngineStateViolation::PerspectiveIdentityMismatch);
            }
        }
        for (opaque, ability) in &identities.opaque_to_ability {
            if identities.ability_to_opaque.get(ability) != Some(opaque) {
                return Err(EngineStateViolation::PerspectiveIdentityMismatch);
            }
        }
        let max_opaque_object = identities
            .opaque_to_object
            .keys()
            .map(|id| id.0)
            .max()
            .unwrap_or(0);
        let max_opaque_ability = identities
            .opaque_to_ability
            .keys()
            .map(|id| id.0)
            .max()
            .unwrap_or(0);
        let next_object = state
            .allocators
            .next_opaque_object_id
            .get(player)
            .map(|id| id.0)
            .unwrap_or(1);
        let next_ability = state
            .allocators
            .next_opaque_ability_id
            .get(player)
            .map(|id| id.0)
            .unwrap_or(1);
        if next_object <= max_opaque_object || next_ability <= max_opaque_ability {
            return Err(EngineStateViolation::AllocatorBehind);
        }
    }

    for (player, knowledge) in &state.knowledge.players {
        if !state.core.players.contains_key(player) {
            return Err(EngineStateViolation::KnowledgeMismatch);
        }
        let Some(identities) = state.perspective_identities.players.get(player) else {
            return Err(EngineStateViolation::KnowledgeMismatch);
        };
        if knowledge.known_objects.iter().any(|(id, known)| {
            let Some(object) = state.zones.objects.get(id) else {
                return true;
            };
            let Some(location) = state.zones.locations.get(id) else {
                return true;
            };
            id != &known.object
                || !identities.object_to_opaque.contains_key(id)
                || known
                    .physical_card
                    .is_some_and(|physical| Some(physical) != object.physical_card)
                || known
                    .card_definition
                    .is_some_and(|definition| definition != object.card_definition)
                || known
                    .known_location
                    .as_ref()
                    .is_some_and(|known_location| known_location != location)
                || !knowledge_point_is_valid(known.learned_at, knowledge)
                || !acquisition_matches_channel(&known.learned_via, known.learned_at.channel)
                || knowledge_event_is_from_the_future(
                    &known.learned_via,
                    state.allocators.next_rule_event_id,
                )
        }) {
            return Err(EngineStateViolation::KnowledgeMismatch);
        }
        let mut invalidations = BTreeSet::new();
        if knowledge.invalidations.iter().any(|record| {
            !knowledge_point_is_valid(record.invalidated_at, knowledge)
                || !invalidations.insert(record)
        }) {
            return Err(EngineStateViolation::KnowledgeMismatch);
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
        for targets in commander.damage.values() {
            if targets
                .keys()
                .any(|player| !state.core.players.contains_key(player))
            {
                return Err(EngineStateViolation::FormatMismatch);
            }
        }
    }

    state
        .random
        .validate()
        .map_err(|_| EngineStateViolation::RandomState)?;

    for key in state.random.streams.keys() {
        if let Some(player_raw) = key.player() {
            let player = PlayerId(player_raw);
            if !state.core.players.contains_key(&player) {
                return Err(EngineStateViolation::RandomState);
            }
        }
    }
    Ok(())
}
