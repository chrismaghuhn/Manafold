//! Ownership: V4 temporal, priority and combat structural validation. This module checks representation and references;
//! it does not decide Magic legality.

use std::collections::BTreeSet;

use mtgml_model::{GameObjectId, PlayerId, ZoneKind};

use super::EngineStateViolation;
use crate::core::{PriorityState, TurnPosition};
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
    // CR 508.1a: the active player may declare any number of attackers.
    if !players.contains(&combat.defending_player)
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
        || !combat.blocked_attackers.is_subset(&attacker_ids)
    {
        return Err(EngineStateViolation::CombatState);
    }
    // CR 509.1a: the defending player chooses which creatures they control
    // block, and for each the one attacker it blocks. CR 509.1h, 510.1c: an
    // attacker may have several blockers, so there is no cap. A state holds
    // only that a blocker is a permanent on the battlefield under the defending
    // player's control; that it is a creature is a rule of the card pool.
    for (blocker, attacker) in &combat.blockers {
        let controlled_by_the_defender = state
            .zones
            .objects
            .get(blocker)
            .is_some_and(|object| object.controller == combat.defending_player);
        let on_the_battlefield = state
            .zones
            .locations
            .get(blocker)
            .is_some_and(|location| location.zone == ZoneKind::Battlefield);
        // CR 509.1h: the attacker is blocked, and stays so without a blocker.
        // Every blocked attacker is one of `attackers` (checked above), so the
        // blocker blocks an attacker. CR 509.1g, 506.4: a blocker whose attacker
        // was removed from combat blocks nothing, and an attacker is removed by
        // dying in the state-based actions after the combat damage step, so
        // there is no such blocker before the damage is dealt.
        let blocks_a_blocked_attacker = match attacker {
            Some(attacker) => combat.blocked_attackers.contains(attacker),
            None => combat.damage_step_completed,
        };
        if !controlled_by_the_defender || !on_the_battlefield || !blocks_a_blocked_attacker {
            return Err(EngineStateViolation::CombatState);
        }
    }
    Ok(())
}
