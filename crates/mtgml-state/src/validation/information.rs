//! Ownership: cross-component information checks. Retained knowledge vs
//! live state and knowledge provenance/history form one authority;
//! perspective identity maps, retired/active relationships, and opaque
//! allocator relationships form the other. They are separate functions
//! because the pending-decision segment sits between them in the frozen
//! coordinator order.

use std::collections::BTreeSet;

use mtgml_model::PlayerId;

use super::EngineStateViolation;
use crate::engine::EngineState;
use crate::knowledge::KnowledgeAcquisitionReason;
use crate::m2_shape::{KnownLocationFactV2, PlayerKnowledgeStateV2};

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

pub(super) fn validate_retained_knowledge_against_live_state(
    state: &EngineState,
    players: &BTreeSet<PlayerId>,
) -> Result<(), EngineStateViolation> {
    for player in players {
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
            // Retirement must carry an observed invalidation sequence
            // (INFORMATION_MODEL.md: invalidation reason *and visible
            // sequence*); an unsequenced initial configuration cannot
            // invalidate anything.
            let invalidation_is_observed =
                record.invalidation.provenance.observed_sequence().is_some();
            if identity
                .opaque_to_object
                .contains_key(&record.opaque_object)
                || !invalidation_is_observed
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
    Ok(())
}

pub(super) fn validate_perspective_identity_relationships(
    state: &EngineState,
) -> Result<(), EngineStateViolation> {
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
    Ok(())
}
