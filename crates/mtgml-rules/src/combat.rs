//! Attacking (CR 508.1) and unblocked combat damage (CR 510.1, 510.2).
//!
//! The only creatures of this slice are vanilla creatures, so combat reads of
//! a creature only its controller, whether it is tapped, since when it has
//! been under its controller's control and its power. A defender who controls
//! an untapped creature could block (CR 509.1); blocking arrives with a later
//! rule, so such a combat fails closed before the declare blockers step.

use mtgml_card_ir::ExecutableProfileAdmissionV1;
use mtgml_decision::{AuthoritativeCandidate, CandidateIntent, EngineCandidateBinding};
use mtgml_model::{CandidateIdV1, GameObjectId, PlayerId};
use mtgml_state::{
    CombatState, CombatStep, DamageAssignmentV1, DamageRecipientV1, EngineState, TurnPosition,
};

use crate::turn_progression::{
    admits, battlefield_objects, observe_public, record_unobserved, Facts,
};
use crate::{AuthoritativeRuleEventKind, BasicLandTransitionError as Error, S1QueryAuthority};

/// A creature on the battlefield and what combat reads of it.
struct Creature {
    object: GameObjectId,
    controller: PlayerId,
    power: i64,
}

/// The creatures on the battlefield, in object order.
fn battlefield_creatures(
    admission: &ExecutableProfileAdmissionV1,
    state: &EngineState,
) -> Result<Vec<Creature>, Error> {
    let queries = S1QueryAuthority::for_objects(admission, state, &battlefield_objects(state))
        .map_err(|_| Error::TurnProgressUnsupported)?;
    let mut creatures = Vec::new();
    for query in &queries {
        let base = query.derive_base_characteristics();
        if !base
            .card_types
            .iter()
            .any(|card_type| card_type == "Creature")
        {
            continue;
        }
        // Every creature has a power and a toughness (CR 208.1).
        let (power, _) = base
            .base_power_toughness
            .ok_or(Error::TurnProgressUnsupported)?;
        creatures.push(Creature {
            object: base.queried.object,
            controller: base.queried.controller,
            power,
        });
    }
    Ok(creatures)
}

/// CR 508.1a, 302.6: the creatures the active player may declare as
/// attackers are the untapped creatures they control that have been under
/// their control continuously since their turn began, which holds exactly when
/// control began on an earlier turn. A candidate is one creature, in the order
/// of the active player's opaque identities.
pub(crate) fn attacker_candidates(
    admission: &ExecutableProfileAdmissionV1,
    state: &EngineState,
) -> Result<Vec<AuthoritativeCandidate>, Error> {
    let active = state.core.active_player;
    let creatures: Vec<_> = battlefield_creatures(admission, state)?
        .into_iter()
        .filter(|creature| creature.controller == active)
        .collect();
    if creatures.is_empty() {
        return Ok(Vec::new());
    }
    admits(admission, "rules/summoning-sickness")?;
    let identity = state
        .perspective_identities
        .players
        .get(&active)
        .ok_or(Error::InvalidResult)?;
    let mut able = Vec::new();
    for creature in creatures {
        let permanent = state
            .card_rules
            .permanents
            .permanents
            .get(&creature.object)
            .ok_or(Error::InvalidResult)?;
        let tapped = state
            .zones
            .objects
            .get(&creature.object)
            .ok_or(Error::InvalidResult)?
            .tapped;
        if tapped || permanent.controlled_since_turn >= state.core.turn_number {
            continue;
        }
        let opaque = identity
            .object_to_opaque
            .get(&creature.object)
            .copied()
            .ok_or(Error::InvalidResult)?;
        able.push((opaque, creature.object));
    }
    able.sort();
    Ok(able
        .into_iter()
        .enumerate()
        .map(|(index, (opaque, object))| AuthoritativeCandidate {
            candidate_id: CandidateIdV1(index as u32),
            visible_intent: CandidateIntent::SelectObject { object: opaque },
            trusted_binding: EngineCandidateBinding::SelectObject { object },
        })
        .collect())
}

/// CR 508.1f, 508.1k: the active player's chosen creatures tap and become
/// attacking creatures, attacking `defending_player` (CR 506.2). The tap and
/// the declaration are public; `attackers` may be empty.
pub(crate) fn declare_attackers(
    next: &mut EngineState,
    facts: &mut Facts,
    defending_player: PlayerId,
    attackers: Vec<GameObjectId>,
) -> Result<(), Error> {
    for object in &attackers {
        let tapped = &mut next
            .zones
            .objects
            .get_mut(object)
            .ok_or(Error::InvalidResult)?
            .tapped;
        if *tapped {
            return Err(Error::InvalidResult);
        }
        *tapped = true;
        observe_public(
            next,
            facts,
            AuthoritativeRuleEventKind::ObjectTapped {
                object: *object,
                from: false,
                to: true,
            },
        )?;
    }
    next.combat = Some(CombatState {
        defending_player,
        attackers: attackers.clone(),
        damage_step_completed: false,
        blocked_attackers: Default::default(),
        blockers: Default::default(),
    });
    observe_public(
        next,
        facts,
        AuthoritativeRuleEventKind::AttackersDeclared {
            defending_player,
            attackers,
        },
    )
}

/// CR 509.1a: whether the defending player controls an untapped creature,
/// which could block.
pub(crate) fn defender_could_block(
    admission: &ExecutableProfileAdmissionV1,
    state: &EngineState,
) -> Result<bool, Error> {
    could_block(state, &battlefield_creatures(admission, state)?)
}

/// `defender_could_block` over the creatures already read.
fn could_block(state: &EngineState, creatures: &[Creature]) -> Result<bool, Error> {
    let defender = state
        .combat
        .as_ref()
        .ok_or(Error::InvalidResult)?
        .defending_player;
    for creature in creatures {
        let tapped = state
            .zones
            .objects
            .get(&creature.object)
            .ok_or(Error::InvalidResult)?
            .tapped;
        if creature.controller == defender && !tapped {
            return Ok(true);
        }
    }
    Ok(false)
}

/// A combat in a restored or committed state is one this slice could have
/// produced; anything else fails closed:
/// - it exists from the declaration to the end of combat;
/// - the attackers are creatures on the battlefield that the active player
///   controls (CR 508.1a), and they attack the other player (CR 506.2);
/// - each attacker is tapped (CR 508.1f) and has been under its controller's
///   control since the turn began (CR 302.6, 508.1a);
/// - the damage step with attackers has dealt its damage (CR 510.1, 510.3):
///   the turn-based action runs on entering the step, so no game rests there
///   before it;
/// - no attacker is blocked while attackers are still being declared
///   (CR 509.1, 508.2), and every blocker is a creature the defending player
///   controls (CR 509.1a);
/// - from the declare blockers step on, with attackers, the defending player
///   controls no untapped creature (CR 509.1a): the game stops before that step
///   when one could block, because blocks are not supported yet.
pub(crate) fn validate_reachable_combat(
    admission: &ExecutableProfileAdmissionV1,
    state: &EngineState,
) -> Result<(), Error> {
    let Some(combat) = state.combat.as_ref() else {
        return Ok(());
    };
    let TurnPosition::Combat { step } = state.core.position else {
        return Err(Error::TurnProgressUnsupported);
    };
    let active = state.core.active_player;
    if step == CombatStep::BeginningOfCombat || combat.defending_player == active {
        return Err(Error::TurnProgressUnsupported);
    }
    let creatures = battlefield_creatures(admission, state)?;
    if combat.attackers.iter().any(|attacker| {
        !creatures
            .iter()
            .any(|creature| creature.object == *attacker && creature.controller == active)
    }) {
        return Err(Error::TurnProgressUnsupported);
    }
    // Declaring an attacker taps it, and nothing untaps it before the combat
    // ends. Only a creature that is not summoning sick can be declared.
    for attacker in &combat.attackers {
        let tapped = state
            .zones
            .objects
            .get(attacker)
            .map(|object| object.tapped);
        let controlled_since = state
            .card_rules
            .permanents
            .permanents
            .get(attacker)
            .map(|permanent| permanent.controlled_since_turn);
        if tapped != Some(true)
            || controlled_since.is_none_or(|since| since >= state.core.turn_number)
        {
            return Err(Error::TurnProgressUnsupported);
        }
    }
    if step == CombatStep::CombatDamage
        && !combat.attackers.is_empty()
        && !combat.damage_step_completed
    {
        return Err(Error::TurnProgressUnsupported);
    }
    if step == CombatStep::DeclareAttackers
        && (!combat.blockers.is_empty() || !combat.blocked_attackers.is_empty())
    {
        return Err(Error::TurnProgressUnsupported);
    }
    if combat.blockers.keys().any(|blocker| {
        !creatures.iter().any(|creature| {
            creature.object == *blocker && creature.controller == combat.defending_player
        })
    }) {
        return Err(Error::TurnProgressUnsupported);
    }
    let blocking_step = matches!(
        step,
        CombatStep::DeclareBlockers | CombatStep::CombatDamage | CombatStep::EndOfCombat
    );
    if blocking_step && !combat.attackers.is_empty() && could_block(state, &creatures)? {
        return Err(Error::TurnProgressUnsupported);
    }
    Ok(())
}

/// CR 510.1a, 510.2, 120.3a: every attacking creature deals damage equal to
/// its power to the defending player, simultaneously, and the player loses
/// that much life. A creature that would assign 0 or less damage assigns none.
/// No creature is blocked in this slice.
pub(crate) fn deal_unblocked_combat_damage(
    admission: &ExecutableProfileAdmissionV1,
    next: &mut EngineState,
    facts: &mut Facts,
) -> Result<(), Error> {
    let combat = next.combat.clone().ok_or(Error::InvalidResult)?;
    if combat.damage_step_completed
        || !combat.blocked_attackers.is_empty()
        || !combat.blockers.is_empty()
    {
        return Err(Error::TurnProgressUnsupported);
    }
    let creatures = battlefield_creatures(admission, next)?;
    let mut assignments = Vec::new();
    let mut total: u64 = 0;
    for attacker in &combat.attackers {
        let creature = creatures
            .iter()
            .find(|creature| {
                creature.object == *attacker && creature.controller == next.core.active_player
            })
            .ok_or(Error::TurnProgressUnsupported)?;
        let amount = u64::try_from(creature.power).unwrap_or(0);
        if amount == 0 {
            continue;
        }
        total = total.checked_add(amount).ok_or(Error::InvalidResult)?;
        assignments.push(DamageAssignmentV1 {
            source: *attacker,
            recipient: DamageRecipientV1::Player {
                player: combat.defending_player,
            },
            amount,
        });
    }
    if !assignments.is_empty() {
        let player = combat.defending_player;
        let from = next
            .core
            .players
            .get(&player)
            .ok_or(Error::InvalidResult)?
            .life;
        let to = i64::try_from(total)
            .ok()
            .and_then(|total| from.checked_sub(total))
            .ok_or(Error::InvalidResult)?;
        record_unobserved(
            facts,
            AuthoritativeRuleEventKind::CombatDamageDealt { assignments },
        );
        next.core
            .players
            .get_mut(&player)
            .ok_or(Error::InvalidResult)?
            .life = to;
        next.card_rules
            .turn_history
            .record_life_loss(player)
            .map_err(|_| Error::InvalidResult)?;
        observe_public(
            next,
            facts,
            AuthoritativeRuleEventKind::LifeChanged { player, from, to },
        )?;
    }
    next.combat
        .as_mut()
        .ok_or(Error::InvalidResult)?
        .damage_step_completed = true;
    record_unobserved(facts, AuthoritativeRuleEventKind::CombatDamageStepCompleted);
    Ok(())
}

/// CR 704.5a, 704.3: a player with 0 or less life loses the game. Returns the
/// player who lost. Both players losing at once would be a draw (CR 104.4a),
/// which this slice does not model.
pub(crate) fn player_at_zero_life_loses(
    admission: &ExecutableProfileAdmissionV1,
    next: &mut EngineState,
) -> Result<Option<PlayerId>, Error> {
    let losing: Vec<PlayerId> = next
        .core
        .players
        .iter()
        .filter(|(_, player)| !player.has_lost && player.life <= 0)
        .map(|(id, _)| *id)
        .collect();
    let [loser] = losing.as_slice() else {
        return if losing.is_empty() {
            Ok(None)
        } else {
            Err(Error::TurnProgressUnsupported)
        };
    };
    admits(admission, "rules/state-based-actions-combat")?;
    next.core
        .players
        .get_mut(loser)
        .ok_or(Error::InvalidResult)?
        .has_lost = true;
    Ok(Some(*loser))
}
