//! Ownership: allocator-behind checks, issued trusted decision/ability
//! identity checks, unsupported effect/trigger machinery rejection, and
//! continuation record key/player structural consistency.

use super::EngineStateViolation;
use crate::engine::EngineState;

pub(super) fn validate_allocators_and_execution(
    state: &EngineState,
) -> Result<(), EngineStateViolation> {
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
    Ok(())
}
