//! Attacking (CR 508.1), declaring blockers (CR 509.1), combat damage
//! (CR 510.1, 510.2) and removal from combat (CR 506.4).
//!
//! The only creatures of this slice are vanilla creatures, so combat reads of
//! a creature only its controller, whether it is tapped, since when it has
//! been under its controller's control, its power and its toughness. The
//! defending player declares blocks one untapped creature at a time, in a
//! continuation that only they can see. Combat damage is dealt to players and
//! marked on creatures; an attacker with two or more blockers has to divide
//! its damage among them, which is not supported yet. A creature dealt lethal
//! damage is destroyed by the state-based actions that follow
//! (`crate::state_based_actions`), which remove it from combat first.

use std::collections::BTreeMap;

use mtgml_card_ir::ExecutableProfileAdmissionV1;
use mtgml_decision::{
    AuthoritativeCandidate, CandidateIntent, CandidateOrdering, DecisionDomainV2,
    DecisionPurposeV4, DecisionVisibility, EngineCandidateBinding,
};
use mtgml_model::{CandidateIdV1, ContinuationId, GameObjectId, PlayerId};
use mtgml_state::{
    CombatBlockerAssignmentV1, CombatState, CombatStep, ContinuationPayload, ContinuationRecord,
    DamageAssignmentV1, DamageRecipientV1, EndingStep, EngineState, PriorityState, TurnPosition,
};

use crate::turn_progression::{
    admits, battlefield_objects, observe_public, record_unobserved, Facts, RequestShape,
};
use crate::{AuthoritativeRuleEventKind, BasicLandTransitionError as Error, S1QueryAuthority};

/// A creature on the battlefield and what combat reads of it.
pub(crate) struct Creature {
    pub(crate) object: GameObjectId,
    pub(crate) controller: PlayerId,
    pub(crate) power: i64,
    pub(crate) toughness: i64,
}

/// The creatures on the battlefield, in object order.
pub(crate) fn battlefield_creatures(
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
        let (power, toughness) = base
            .base_power_toughness
            .ok_or(Error::TurnProgressUnsupported)?;
        creatures.push(Creature {
            object: base.queried.object,
            controller: base.queried.controller,
            power,
            toughness,
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

/// CR 509.1a: the untapped creatures the defending player controls, which can
/// block, in the order of the defending player's opaque identities. That order
/// is the order they are asked in: it means something to the player, where the
/// engine's object order does not.
fn possible_blockers(
    state: &EngineState,
    creatures: &[Creature],
) -> Result<Vec<GameObjectId>, Error> {
    let defender = state
        .combat
        .as_ref()
        .ok_or(Error::InvalidResult)?
        .defending_player;
    let identity = state
        .perspective_identities
        .players
        .get(&defender)
        .ok_or(Error::InvalidResult)?;
    let mut able = Vec::new();
    for creature in creatures {
        let tapped = state
            .zones
            .objects
            .get(&creature.object)
            .ok_or(Error::InvalidResult)?
            .tapped;
        if creature.controller == defender && !tapped {
            let opaque = identity
                .object_to_opaque
                .get(&creature.object)
                .copied()
                .ok_or(Error::InvalidResult)?;
            able.push((opaque, creature.object));
        }
    }
    able.sort();
    Ok(able.into_iter().map(|(_, object)| object).collect())
}

/// CR 509.1a: the declare blockers step begins. When the defending player
/// controls an untapped creature, a block declaration starts: the creatures are
/// asked about one at a time, and the caller asks about the first
/// (`install_block_request`). Returns whether there is such a declaration;
/// without one, nothing is asked.
pub(crate) fn begin_block_declaration(
    admission: &ExecutableProfileAdmissionV1,
    next: &mut EngineState,
) -> Result<bool, Error> {
    let creatures = battlefield_creatures(admission, next)?;
    let pending_blockers = possible_blockers(next, &creatures)?;
    if pending_blockers.is_empty() {
        return Ok(false);
    }
    let defender = next
        .combat
        .as_ref()
        .ok_or(Error::InvalidResult)?
        .defending_player;
    let continuation = next.allocators.next_continuation_id;
    next.allocators.next_continuation_id = ContinuationId(
        continuation
            .0
            .checked_add(1)
            .ok_or(Error::IdentityExhausted)?,
    );
    next.execution.continuations.insert(
        continuation,
        ContinuationRecord {
            id: continuation,
            created_at_revision: next.revision,
            payload: ContinuationPayload::BlockDeclaration {
                defender,
                pending_blockers,
                declared: Default::default(),
            },
        },
    );
    Ok(true)
}

/// The block declaration in `state`'s one continuation, as written.
struct BlockDeclaration<'a> {
    continuation: ContinuationId,
    defender: PlayerId,
    pending_blockers: &'a [GameObjectId],
    declared: &'a BTreeMap<GameObjectId, Option<GameObjectId>>,
}

fn block_declaration_of(state: &EngineState) -> Result<BlockDeclaration<'_>, Error> {
    let [(continuation, record)] = state.execution.continuations.iter().collect::<Vec<_>>()[..]
    else {
        return Err(Error::InvalidResult);
    };
    let ContinuationPayload::BlockDeclaration {
        defender,
        pending_blockers,
        declared,
    } = &record.payload
    else {
        return Err(Error::InvalidResult);
    };
    Ok(BlockDeclaration {
        continuation: *continuation,
        defender: *defender,
        pending_blockers,
        declared,
    })
}

/// Whether the block declaration `state` waits on is the one the battlefield
/// calls for: exactly one BlockDeclaration continuation, for the defending
/// player in the declare blockers step of an attack, before any block is
/// recorded and while no player has priority (CR 509.1, 509.2). Its creatures
/// are all the untapped creatures the defender controls (CR 509.1a), in the
/// order of the defender's opaque identities: the answered ones first, then the
/// ones still to ask, with at least one of those. Each answered creature
/// blocks one of the attackers or none.
pub(crate) fn validate_pending_block_declaration(
    admission: &ExecutableProfileAdmissionV1,
    state: &EngineState,
) -> Result<(), Error> {
    let declaration = block_declaration_of(state)?;
    let combat = state.combat.as_ref().ok_or(Error::InvalidResult)?;
    let expected = possible_blockers(state, &battlefield_creatures(admission, state)?)?;
    let answered = declaration.declared.len();
    let in_order = expected.len() == answered + declaration.pending_blockers.len()
        && expected[answered..] == *declaration.pending_blockers
        && expected[..answered]
            .iter()
            .all(|blocker| declaration.declared.contains_key(blocker));
    if !in_order
        || declaration.pending_blockers.is_empty()
        || declaration.defender != combat.defending_player
        || state.core.position
            != (TurnPosition::Combat {
                step: CombatStep::DeclareBlockers,
            })
        || state.core.priority != PriorityState::None
        || combat.attackers.is_empty()
        || !combat.blockers.is_empty()
        || !combat.blocked_attackers.is_empty()
        || declaration
            .declared
            .values()
            .flatten()
            .any(|attacker| !combat.attackers.contains(attacker))
    {
        return Err(Error::InvalidResult);
    }
    Ok(())
}

/// The request that asks the defending player whether the next creature of the
/// pending declaration blocks (CR 509.1a): one candidate for each attacker it
/// could block, and one for no block, which comes first. It follows from the
/// continuation alone; `validate_pending_block_declaration` is what says the
/// continuation is right.
pub(crate) fn block_request_shape(state: &EngineState) -> Result<RequestShape, Error> {
    let declaration = block_declaration_of(state)?;
    let blocker = *declaration
        .pending_blockers
        .first()
        .ok_or(Error::InvalidResult)?;
    let combat = state.combat.as_ref().ok_or(Error::InvalidResult)?;
    let identity = state
        .perspective_identities
        .players
        .get(&declaration.defender)
        .ok_or(Error::InvalidResult)?;
    let opaque = |object: &GameObjectId| {
        identity
            .object_to_opaque
            .get(object)
            .copied()
            .ok_or(Error::InvalidResult)
    };
    let visible_blocker = opaque(&blocker)?;
    let mut raw = vec![(
        CandidateIntent::DeclareBlock {
            blocker: visible_blocker,
            attacker: None,
        },
        EngineCandidateBinding::DeclareBlock {
            blocker,
            attacker: None,
        },
    )];
    for attacker in &combat.attackers {
        raw.push((
            CandidateIntent::DeclareBlock {
                blocker: visible_blocker,
                attacker: Some(opaque(attacker)?),
            },
            EngineCandidateBinding::DeclareBlock {
                blocker,
                attacker: Some(*attacker),
            },
        ));
    }
    Ok(RequestShape {
        actor: declaration.defender,
        visibility: DecisionVisibility::ActingPlayerOnly,
        continuation_id: Some(declaration.continuation),
        purpose: DecisionPurposeV4::BlockerDeclaration,
        decision_domain_v2: DecisionDomainV2::ChooseOne,
        candidates: CandidateOrdering::assign_dense(raw).map_err(|_| Error::InvalidResult)?,
    })
}

/// Installs the request that asks the defending player about the next creature.
pub(crate) fn install_block_request(
    next: &mut EngineState,
) -> Result<mtgml_decision::AuthoritativeDecisionRequest, Error> {
    let shape = block_request_shape(next)?;
    crate::turn_progression::install_request(next, shape)
}

/// CR 509.1a: the creature the pending declaration asks about, `blocker`,
/// blocks `attacker`, or nothing. With creatures still to ask, the declaration
/// goes on and this returns `false`. After the last answer the declaration is
/// complete: every chosen creature becomes a blocking creature and each
/// attacker with a blocker becomes blocked (CR 509.1g, 509.1h), in one
/// `BlockersDeclared`, which is emitted even when no creature blocks. The
/// continuation ends and this returns `true`; the caller gives the active
/// player priority (CR 509.2).
///
/// Every combination of answers is a legal declaration for vanilla creatures:
/// no restriction or requirement applies to blocking (CR 509.1b, 509.1c).
/// Evasion and block requirements would need a check over the whole
/// declaration before it is applied.
pub(crate) fn declare_block(
    next: &mut EngineState,
    facts: &mut Facts,
    blocker: GameObjectId,
    attacker: Option<GameObjectId>,
) -> Result<bool, Error> {
    if next.execution.continuations.len() != 1 {
        return Err(Error::InvalidResult);
    }
    let (continuation, record) = next
        .execution
        .continuations
        .iter_mut()
        .next()
        .ok_or(Error::InvalidResult)?;
    let continuation = *continuation;
    let ContinuationPayload::BlockDeclaration {
        pending_blockers,
        declared,
        ..
    } = &mut record.payload
    else {
        return Err(Error::InvalidResult);
    };
    if pending_blockers.first() != Some(&blocker) {
        return Err(Error::InvalidResult);
    }
    pending_blockers.remove(0);
    declared.insert(blocker, attacker);
    if !pending_blockers.is_empty() {
        return Ok(false);
    }
    // The assignments are in order of blocker: the map is.
    let assignments: Vec<CombatBlockerAssignmentV1> = declared
        .iter()
        .filter_map(|(blocker, attacker)| {
            attacker.map(|attacker| CombatBlockerAssignmentV1 {
                blocker: *blocker,
                attacker,
            })
        })
        .collect();
    next.execution.continuations.remove(&continuation);
    let combat = next.combat.as_mut().ok_or(Error::InvalidResult)?;
    for assignment in &assignments {
        combat
            .blockers
            .insert(assignment.blocker, assignment.attacker);
        combat.blocked_attackers.insert(assignment.attacker);
    }
    // No observed event or observation field shows blocks yet, so no player
    // observes this rule event.
    record_unobserved(
        facts,
        AuthoritativeRuleEventKind::BlockersDeclared { assignments },
    );
    Ok(true)
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
///   (CR 509.1, 508.2), and every blocker is an untapped creature the defending
///   player controls (CR 509.1a): nothing taps a blocker once it blocks. An
///   attacker may be blocked with no blocker left (CR 509.1h);
/// - a block declaration in progress is the one the battlefield calls for (see
///   `validate_pending_block_declaration`). Once the declaration is complete, in the
///   declare blockers step with priority or in a later step, an untapped
///   creature of the defending player that does not block is legal: blocking is
///   a choice (CR 509.1a).
pub(crate) fn validate_reachable_combat(
    admission: &ExecutableProfileAdmissionV1,
    state: &EngineState,
) -> Result<(), Error> {
    let declaring = state
        .execution
        .continuations
        .values()
        .any(|record| matches!(record.payload, ContinuationPayload::BlockDeclaration { .. }));
    let Some(combat) = state.combat.as_ref() else {
        return if declaring {
            Err(Error::TurnProgressUnsupported)
        } else {
            Ok(())
        };
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
        }) || state
            .zones
            .objects
            .get(blocker)
            .is_none_or(|object| object.tapped)
    }) {
        return Err(Error::TurnProgressUnsupported);
    }
    if declaring && validate_pending_block_declaration(admission, state).is_err() {
        return Err(Error::TurnProgressUnsupported);
    }
    Ok(())
}

/// CR 506.4: the creature `object`, which leaves the battlefield, is removed
/// from combat and stops being an attacking, blocking, blocked creature.
/// - An attacker leaves `attackers` and `blocked_attackers`. The creatures that
///   blocked it remain blocking creatures (CR 509.1g) with nothing to block,
///   and a creature that blocks nothing assigns no combat damage (CR 510.1d)
///   as one that never blocked does. The combat records a block as the
///   attacker its blocker blocks, so they leave the blocks with it.
/// - A blocker leaves the blocks. The attacker it blocked stays blocked with no
///   blocker left (CR 509.1h): it stays in `blocked_attackers`.
pub(crate) fn remove_from_combat(next: &mut EngineState, object: GameObjectId) {
    let Some(combat) = next.combat.as_mut() else {
        return;
    };
    combat.attackers.retain(|attacker| *attacker != object);
    combat.blocked_attackers.remove(&object);
    combat.blockers.remove(&object);
    combat.blockers.retain(|_, attacker| *attacker != object);
}

/// CR 510.1, 510.2: every attacking and blocking creature deals combat damage
/// equal to its power, all at once:
/// - an unblocked attacker deals it to the defending player (CR 510.1b), who
///   loses that much life (CR 120.3a);
/// - an attacker with exactly one blocker deals all of it to that blocker; one
///   that is blocked with no blocker left deals none (CR 510.1c, 509.1h);
/// - a blocker deals it to the attacker it blocks (CR 510.1d).
///
/// Damage dealt to a creature is marked on it (CR 120.3e). A creature that
/// would assign 0 or less damage assigns none (CR 510.1a). An attacker with two
/// or more blockers divides its damage among them (CR 510.1c), which is not
/// supported yet, so the step fails closed. Whether a creature has been dealt
/// lethal damage is for the state-based actions that follow the step (see
/// `crate::state_based_actions`).
pub(crate) fn deal_combat_damage(
    admission: &ExecutableProfileAdmissionV1,
    next: &mut EngineState,
    facts: &mut Facts,
) -> Result<(), Error> {
    let combat = next.combat.clone().ok_or(Error::InvalidResult)?;
    if combat.damage_step_completed {
        return Err(Error::TurnProgressUnsupported);
    }
    let creatures = battlefield_creatures(admission, next)?;
    // The damage `object`, a creature `controller` controls, assigns (CR 510.1a).
    let assigned = |object: &GameObjectId, controller: PlayerId| {
        creatures
            .iter()
            .find(|creature| creature.object == *object && creature.controller == controller)
            .map(|creature| u64::try_from(creature.power).unwrap_or(0))
            .ok_or(Error::TurnProgressUnsupported)
    };
    let mut assignments = Vec::new();
    for attacker in &combat.attackers {
        let amount = assigned(attacker, next.core.active_player)?;
        let mut blockers = combat
            .blockers
            .iter()
            .filter(|(_, blocked)| *blocked == attacker)
            .map(|(blocker, _)| *blocker);
        let recipient = match (blockers.next(), blockers.next()) {
            (None, _) if combat.blocked_attackers.contains(attacker) => continue,
            (None, _) => DamageRecipientV1::Player {
                player: combat.defending_player,
            },
            (Some(blocker), None) => DamageRecipientV1::Creature { object: blocker },
            (Some(_), Some(_)) => return Err(Error::TurnProgressUnsupported),
        };
        if amount > 0 {
            assignments.push(DamageAssignmentV1 {
                source: *attacker,
                recipient,
                amount,
            });
        }
    }
    for (blocker, attacker) in &combat.blockers {
        let amount = assigned(blocker, combat.defending_player)?;
        if amount > 0 {
            assignments.push(DamageAssignmentV1 {
                source: *blocker,
                recipient: DamageRecipientV1::Creature { object: *attacker },
                amount,
            });
        }
    }
    let mut to_player: u64 = 0;
    let mut to_creatures: BTreeMap<GameObjectId, u64> = BTreeMap::new();
    for assignment in &assignments {
        let total = match assignment.recipient {
            DamageRecipientV1::Player { .. } => &mut to_player,
            DamageRecipientV1::Creature { object } => to_creatures.entry(object).or_default(),
        };
        *total = total
            .checked_add(assignment.amount)
            .ok_or(Error::InvalidResult)?;
    }
    if !assignments.is_empty() {
        record_unobserved(
            facts,
            AuthoritativeRuleEventKind::CombatDamageDealt { assignments },
        );
    }
    if to_player > 0 {
        let player = combat.defending_player;
        let from = next
            .core
            .players
            .get(&player)
            .ok_or(Error::InvalidResult)?
            .life;
        let to = i64::try_from(to_player)
            .ok()
            .and_then(|total| from.checked_sub(total))
            .ok_or(Error::InvalidResult)?;
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
    // No observed event or observation field shows marked damage yet, so no
    // player observes these rule events.
    for (creature, amount) in to_creatures {
        let (from, to) = next
            .card_rules
            .permanents
            .mark_damage(creature, amount)
            .map_err(|_| Error::InvalidResult)?;
        record_unobserved(
            facts,
            AuthoritativeRuleEventKind::MarkedDamageChanged { creature, from, to },
        );
    }
    next.combat
        .as_mut()
        .ok_or(Error::InvalidResult)?
        .damage_step_completed = true;
    record_unobserved(facts, AuthoritativeRuleEventKind::CombatDamageStepCompleted);
    Ok(())
}

/// Whether damage is marked on any permanent.
pub(crate) fn damage_is_marked(state: &EngineState) -> bool {
    state
        .card_rules
        .permanents
        .permanents
        .values()
        .any(|permanent| permanent.marked_damage != 0)
}

/// CR 120.6, 514.2: damage is marked by combat damage and stays marked until
/// the cleanup step removes it, so some can exist from the combat damage step,
/// once its damage is dealt, through the end of combat, the postcombat main
/// phase and the end step. Removing it at cleanup is not supported yet: no
/// game enters the cleanup step with any marked.
fn marked_damage_may_exist(state: &EngineState) -> bool {
    match state.core.position {
        TurnPosition::Combat {
            step: CombatStep::CombatDamage | CombatStep::EndOfCombat,
        } => state
            .combat
            .as_ref()
            .is_some_and(|combat| combat.damage_step_completed),
        TurnPosition::PostcombatMain
        | TurnPosition::Ending {
            step: EndingStep::EndStep,
        } => true,
        _ => false,
    }
}

/// The damage marked on the permanents is damage this slice can have made:
/// - it is marked only in the window of `marked_damage_may_exist`: none in the
///   beginning phase, the precombat main phase, the combat steps before the
///   damage is dealt, or the cleanup step;
/// - only a creature has any (CR 120.3e);
/// - no creature has been dealt lethal damage: marked damage at least equal to
///   its toughness (CR 704.5g; with toughness 0 or less it is destroyed as
///   well, CR 704.5f, which no card of this slice has). It is destroyed by the
///   state-based actions before any player has priority (CR 704.3), so no
///   game rests there.
pub(crate) fn validate_marked_damage(
    admission: &ExecutableProfileAdmissionV1,
    state: &EngineState,
) -> Result<(), Error> {
    if damage_is_marked(state) && !marked_damage_may_exist(state) {
        return Err(Error::TurnProgressUnsupported);
    }
    let creatures = battlefield_creatures(admission, state)?;
    let permanents = &state.card_rules.permanents.permanents;
    if permanents.iter().any(|(object, permanent)| {
        permanent.marked_damage != 0 && !creatures.iter().any(|creature| creature.object == *object)
    }) {
        return Err(Error::TurnProgressUnsupported);
    }
    for creature in &creatures {
        let permanent = permanents
            .get(&creature.object)
            .ok_or(Error::InvalidResult)?;
        if i128::from(permanent.marked_damage) >= i128::from(creature.toughness) {
            return Err(Error::TurnProgressUnsupported);
        }
    }
    Ok(())
}
