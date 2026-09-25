//! Ownership: V4 temporal, priority, combat, and foundation-source
//! structural validation. This module checks representation and references;
//! it does not decide Magic legality.

use std::collections::BTreeSet;

use mtgml_model::{GameObjectId, PlayerId};

use super::EngineStateViolation;
use crate::core::{ControlHistory, PriorityState, TurnPosition};
use crate::engine::EngineState;

pub(super) fn validate_core_structure(state: &EngineState) -> Result<(), EngineStateViolation> {
    let players: BTreeSet<_> = state.core.players.keys().copied().collect();
    if !players.contains(&state.core.active_player) {
        return Err(EngineStateViolation::MissingTurnPlayer);
    }
    if let PriorityState::HeldBy {
        player,
        consecutive_passes,
    } = state.core.priority
    {
        if !players.contains(&player) || consecutive_passes > 1 {
            return Err(EngineStateViolation::PriorityState);
        }
    }
    if let Some(combat) = &state.combat {
        validate_combat(state, combat, &players)?;
    }
    for (object, source) in &state.foundation_sources {
        if !state.zones.objects.contains_key(object)
            || !control_history_is_coherent(
                &source.control_history,
                state.core.turn_number,
                state.core.position,
            )
        {
            return Err(EngineStateViolation::FoundationSource);
        }
    }
    Ok(())
}

fn validate_combat(
    state: &EngineState,
    combat: &crate::core::CombatState,
    players: &BTreeSet<PlayerId>,
) -> Result<(), EngineStateViolation> {
    let damage_boundary = matches!(
        state.core.position,
        TurnPosition::Combat {
            step: crate::core::CombatStep::CombatDamage | crate::core::CombatStep::EndOfCombat
        }
    );
    if (combat.damage_step_completed && !damage_boundary)
        || (state.core.position
            == (TurnPosition::Combat {
                step: crate::core::CombatStep::EndOfCombat,
            })
            && !combat.attackers.is_empty()
            && !combat.damage_step_completed)
    {
        return Err(EngineStateViolation::CombatState);
    }
    if !players.contains(&combat.defending_player)
        || combat.attackers.len() > 8
        || combat
            .attackers
            .windows(2)
            .any(|window| window[0] >= window[1])
        || combat
            .attackers
            .iter()
            .any(|attacker| !state.zones.objects.contains_key(attacker))
    {
        return Err(EngineStateViolation::CombatState);
    }
    let attacker_ids: BTreeSet<GameObjectId> = combat.attackers.iter().copied().collect();
    if attacker_ids.len() != combat.attackers.len()
        || attacker_ids != combat.blockers.keys().copied().collect()
        || !combat.blocked_attackers.is_subset(&attacker_ids)
    {
        return Err(EngineStateViolation::CombatState);
    }
    let mut blockers = BTreeSet::new();
    for (attacker, blocker) in &combat.blockers {
        if blocker.is_some() && !combat.blocked_attackers.contains(attacker) {
            return Err(EngineStateViolation::CombatState);
        }
        if blocker.is_some_and(|blocker| {
            !state.zones.objects.contains_key(&blocker) || !blockers.insert(blocker)
        }) {
            return Err(EngineStateViolation::CombatState);
        }
    }
    Ok(())
}

fn control_history_is_coherent(
    history: &ControlHistory,
    current_turn: u64,
    current_position: TurnPosition,
) -> bool {
    match history {
        ControlHistory::BeforeTurnStart { turn_number } => *turn_number <= current_turn,
        ControlHistory::DuringTurn {
            turn_number,
            boundary,
        } => {
            *turn_number <= current_turn
                && (*turn_number < current_turn
                    || boundary.canonical_rank() <= current_position.canonical_rank())
        }
    }
}
