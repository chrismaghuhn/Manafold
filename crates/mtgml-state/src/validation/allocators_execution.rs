//! Ownership: allocator-behind checks for the identities the state itself
//! issues. The execution records check their own allocators.

use super::EngineStateViolation;
use crate::engine::EngineState;

pub(super) fn validate_allocators(state: &EngineState) -> Result<(), EngineStateViolation> {
    let max_object = state.zones.objects.keys().map(|id| id.0).max().unwrap_or(0);
    let max_stack = state
        .zones
        .stack_records
        .keys()
        .map(|id| id.0)
        .max()
        .unwrap_or(0);
    if state.allocators.next_object_id.0 <= max_object
        || state.allocators.next_stack_object_id.0 <= max_stack
        || state.allocators.next_rule_event_id.0 == 0
    {
        return Err(EngineStateViolation::AllocatorBehind);
    }
    // Trusted ability identities are reachable through the perspective-local
    // opaque ability mappings.
    let issued_ability_id = state
        .perspective_identities
        .players
        .values()
        .flat_map(|identity| identity.opaque_to_ability.values().copied())
        .map(|ability| ability.0)
        .max()
        .unwrap_or(0);
    if state.allocators.next_ability_id.0 <= issued_ability_id {
        return Err(EngineStateViolation::AllocatorBehind);
    }
    Ok(())
}
