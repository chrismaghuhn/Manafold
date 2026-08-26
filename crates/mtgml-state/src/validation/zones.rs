//! Ownership: zone/object structural validation segment (active/priority
//! player presence; object map identity; owner/controller and zone-player
//! references; duplicate live physical-card identity; object/location
//! bijection; ordered-zone consistency; stack consistency).

use std::collections::BTreeSet;

use mtgml_model::PhysicalCardId;

use super::EngineStateViolation;
use crate::engine::EngineState;
use crate::zones::ZonePosition;

pub(super) fn validate_zone_structure(state: &EngineState) -> Result<(), EngineStateViolation> {
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
    Ok(())
}
