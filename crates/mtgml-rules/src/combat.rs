//! Attacking (CR 508.1), declaring blockers (CR 509.1), combat damage
//! (CR 510.1, 510.2) and removal from combat (CR 506.4).
//!
//! The only creatures of this slice are vanilla creatures, so combat reads of
//! a creature only its controller, whether it is tapped, since when it has
//! been under its controller's control, its power and its toughness. The
//! defending player declares blocks one untapped creature at a time, in a
//! continuation that only they can see. Combat damage is dealt to players and
//! marked on creatures; an attacker with two or more blockers has its
//! controller divide its damage among them, one blocker at a time, in a
//! continuation too (CR 510.1c). A creature dealt lethal damage is destroyed by
//! the state-based actions that follow (`crate::state_based_actions`), which
//! remove it from combat first. The damage that stays marked is removed in the
//! cleanup step (CR 514.2).

use std::collections::{BTreeMap, BTreeSet};

use mtgml_card_ir::ExecutableProfileAdmissionV1;
use mtgml_decision::{
    AuthoritativeCandidate, CandidateIntent, CandidateOrdering, DecisionDomainV2,
    DecisionPurposeV4, DecisionVisibility, EngineCandidateBinding,
};
use mtgml_model::{CandidateIdV1, ContinuationId, GameObjectId, PlayerId, StateRevision};
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
/// without one, nothing is asked and the caller declares no blocks
/// (`declare_no_blockers`).
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
    created_at_revision: StateRevision,
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
        created_at_revision: record.created_at_revision,
        defender: *defender,
        pending_blockers,
        declared,
    })
}

/// Whether the block declaration `state` waits on is the one the battlefield
/// calls for: exactly one BlockDeclaration continuation, for the defending
/// player in the declare blockers step of an attack, before any block is
/// recorded and while no player has priority (CR 509.1, 509.2). It was created
/// by the transition that opened the step and gained one answer with each
/// revision since, so its `created_at_revision` is the state's less the
/// creatures answered. Its creatures
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
    let created_when_it_was_opened = u64::try_from(answered)
        .ok()
        .and_then(|answered| state.revision.0.checked_sub(answered))
        == Some(declaration.created_at_revision.0);
    let in_order = expected.len() == answered + declaration.pending_blockers.len()
        && expected[answered..] == *declaration.pending_blockers
        && expected[..answered]
            .iter()
            .all(|blocker| declaration.declared.contains_key(blocker));
    if !in_order
        || !created_when_it_was_opened
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

/// CR 509.1: the defending player controls no untapped creature, so the
/// declare blockers step's turn-based action declares no blockers: one empty
/// `BlockersDeclared`, which both players observe, as for any other declaration.
/// It happens whenever there are attackers (CR 508.8).
pub(crate) fn declare_no_blockers(next: &mut EngineState, facts: &mut Facts) -> Result<(), Error> {
    observe_public(
        next,
        facts,
        AuthoritativeRuleEventKind::BlockersDeclared {
            assignments: Vec::new(),
        },
    )
}

/// CR 509.1a: the creature the pending declaration asks about, `blocker`,
/// blocks `attacker`, or nothing. With creatures still to ask, the declaration
/// goes on and this returns `false`. After the last answer the declaration is
/// complete: every chosen creature becomes a blocking creature and each
/// attacker with a blocker becomes blocked (CR 509.1g, 509.1h), in one
/// `BlockersDeclared`, which both players observe and which is emitted even
/// when no creature blocks. The continuation ends and this returns `true`; the
/// caller gives the active player priority (CR 509.2).
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
            .insert(assignment.blocker, Some(assignment.attacker));
        combat.blocked_attackers.insert(assignment.attacker);
    }
    observe_public(
        next,
        facts,
        AuthoritativeRuleEventKind::BlockersDeclared { assignments },
    )?;
    Ok(true)
}

/// An attacking creature that is blocked by two or more creatures and has damage
/// to assign: its controller divides that damage among the blockers (CR 510.1c,
/// 510.1a). A creature that would assign 0 or less damage assigns none, so it
/// has nothing to divide.
struct Division {
    attacker: GameObjectId,
    power: u64,
    /// Its blockers in the order of the active player's opaque identities,
    /// which is the order they are asked in.
    blockers: Vec<GameObjectId>,
}

/// The attackers that divide their damage, in the order of the active player's
/// opaque identities: that order means something to the player, where the
/// engine's object order does not.
fn divisions(
    admission: &ExecutableProfileAdmissionV1,
    state: &EngineState,
) -> Result<Vec<Division>, Error> {
    let combat = state.combat.as_ref().ok_or(Error::InvalidResult)?;
    let active = state.core.active_player;
    let identity = state
        .perspective_identities
        .players
        .get(&active)
        .ok_or(Error::InvalidResult)?;
    let opaque = |object: &GameObjectId| {
        identity
            .object_to_opaque
            .get(object)
            .copied()
            .ok_or(Error::InvalidResult)
    };
    let creatures = battlefield_creatures(admission, state)?;
    let mut divisions = Vec::new();
    for attacker in &combat.attackers {
        let power = creatures
            .iter()
            .find(|creature| creature.object == *attacker && creature.controller == active)
            .map(|creature| u64::try_from(creature.power).unwrap_or(0))
            .ok_or(Error::TurnProgressUnsupported)?;
        let mut blockers = combat
            .blockers
            .iter()
            .filter(|(_, blocked)| **blocked == Some(*attacker))
            .map(|(blocker, _)| Ok((opaque(blocker)?, *blocker)))
            .collect::<Result<Vec<_>, Error>>()?;
        if blockers.len() < 2 || power == 0 {
            continue;
        }
        blockers.sort();
        divisions.push((
            opaque(attacker)?,
            Division {
                attacker: *attacker,
                power,
                blockers: blockers.into_iter().map(|(_, blocker)| blocker).collect(),
            },
        ));
    }
    divisions.sort_by_key(|(opaque, _)| *opaque);
    Ok(divisions
        .into_iter()
        .map(|(_, division)| division)
        .collect())
}

/// How far the answers of a division have got.
enum Progress {
    /// The blocker at `blocker` of the attacker at `attacker` (positions in the
    /// divisions) is asked next, with `remaining` damage left to give.
    Asking {
        attacker: usize,
        blocker: usize,
        remaining: u64,
    },
    /// Every attacker is finished: the amount each blocker of a divided
    /// attacker is assigned, the amounts the rules force included.
    Done(BTreeMap<GameObjectId, u64>),
}

/// Plays `assigned`, the answered amounts by blocker, through `divisions`
/// attacker by attacker and blocker by blocker. A blocker is asked while two or
/// more of the attacker's blockers have no amount and damage is left to give
/// (CR 510.1c). Otherwise the rest is forced: the last blocker is assigned what
/// is left, and when nothing is left every blocker still without an amount is
/// assigned none. Fails closed on an answer that no game gives: more than the
/// attacker has left, one for a blocker the rules force or have not reached, or
/// one for a creature that is not divided.
fn progress_of(
    divisions: &[Division],
    assigned: &BTreeMap<GameObjectId, u64>,
) -> Result<Progress, Error> {
    let mut divided = BTreeMap::new();
    let mut used = 0;
    for (index, division) in divisions.iter().enumerate() {
        let mut remaining = division.power;
        let mut next = 0;
        while division.blockers.len() - next >= 2 && remaining >= 1 {
            let blocker = division.blockers[next];
            let Some(amount) = assigned.get(&blocker) else {
                return if used == assigned.len() {
                    Ok(Progress::Asking {
                        attacker: index,
                        blocker: next,
                        remaining,
                    })
                } else {
                    Err(Error::InvalidResult)
                };
            };
            remaining = remaining.checked_sub(*amount).ok_or(Error::InvalidResult)?;
            divided.insert(blocker, *amount);
            used += 1;
            next += 1;
        }
        // An answer for one of these is never consumed, so it fails the count.
        let forced = &division.blockers[next..];
        for (position, blocker) in forced.iter().enumerate() {
            let last = position + 1 == forced.len();
            divided.insert(*blocker, if last { remaining } else { 0 });
        }
    }
    if used != assigned.len() {
        return Err(Error::InvalidResult);
    }
    Ok(Progress::Done(divided))
}

/// CR 510.1c: the combat damage step begins. When an attacker has two or more
/// blockers and damage to assign, a division starts: its controller is asked
/// about the blockers one at a time. Returns the request that asks about the
/// first, for the caller to install; without a division, nothing is asked and
/// the damage is dealt (`deal_combat_damage`). The creatures are read before the
/// continuation exists: a state with a continuation and no request is not one
/// they can be queried in.
pub(crate) fn begin_damage_division(
    admission: &ExecutableProfileAdmissionV1,
    next: &mut EngineState,
) -> Result<Option<RequestShape>, Error> {
    let divisions = divisions(admission, next)?;
    let Some(first) = divisions.first() else {
        return Ok(None);
    };
    let player = next.core.active_player;
    let continuation = next.allocators.next_continuation_id;
    next.allocators.next_continuation_id = ContinuationId(
        continuation
            .0
            .checked_add(1)
            .ok_or(Error::IdentityExhausted)?,
    );
    let assigned = BTreeMap::new();
    let shape = request_shape(next, &divisions, continuation, player, &assigned)?;
    next.execution.continuations.insert(
        continuation,
        ContinuationRecord {
            id: continuation,
            created_at_revision: next.revision,
            payload: ContinuationPayload::CombatDamageAssignment {
                player,
                pending_attackers: divisions.iter().map(|division| division.attacker).collect(),
                pending_blockers: first.blockers.clone(),
                assigned,
            },
        },
    );
    Ok(Some(shape))
}

/// The division in `state`'s one continuation, as written.
struct PendingDivision<'a> {
    continuation: ContinuationId,
    created_at_revision: StateRevision,
    player: PlayerId,
    pending_attackers: &'a [GameObjectId],
    pending_blockers: &'a [GameObjectId],
    assigned: &'a BTreeMap<GameObjectId, u64>,
}

fn pending_division_of(state: &EngineState) -> Result<PendingDivision<'_>, Error> {
    let [(continuation, record)] = state.execution.continuations.iter().collect::<Vec<_>>()[..]
    else {
        return Err(Error::InvalidResult);
    };
    let ContinuationPayload::CombatDamageAssignment {
        player,
        pending_attackers,
        pending_blockers,
        assigned,
    } = &record.payload
    else {
        return Err(Error::InvalidResult);
    };
    Ok(PendingDivision {
        continuation: *continuation,
        created_at_revision: record.created_at_revision,
        player: *player,
        pending_attackers,
        pending_blockers,
        assigned,
    })
}

/// Whether the division `state` waits on is the one the battlefield calls for:
/// exactly one CombatDamageAssignment continuation, for the active player in the
/// combat damage step of an attack, before the damage is dealt and while no
/// player has priority (CR 510.1, 510.2). It was created by the transition that
/// opened the step and gained one answer with each revision since, so its
/// `created_at_revision` is the state's less the blockers answered. The answers
/// are ones the division could have been given (see `progress_of`) and leave a
/// blocker to ask; the attackers it holds are the divided attackers not yet
/// finished, in the order of the active player's opaque identities, and the
/// blockers are those of the first, from the one asked next.
pub(crate) fn validate_pending_damage_division(
    admission: &ExecutableProfileAdmissionV1,
    state: &EngineState,
) -> Result<(), Error> {
    let pending = pending_division_of(state)?;
    let combat = state.combat.as_ref().ok_or(Error::InvalidResult)?;
    let divisions = divisions(admission, state)?;
    let Progress::Asking {
        attacker, blocker, ..
    } = progress_of(&divisions, pending.assigned)?
    else {
        return Err(Error::InvalidResult);
    };
    let created_when_it_was_opened = u64::try_from(pending.assigned.len())
        .ok()
        .and_then(|answered| state.revision.0.checked_sub(answered))
        == Some(pending.created_at_revision.0);
    if !created_when_it_was_opened
        || !divisions[attacker..]
            .iter()
            .map(|division| division.attacker)
            .eq(pending.pending_attackers.iter().copied())
        || pending.pending_blockers != &divisions[attacker].blockers[blocker..]
        || pending.player != state.core.active_player
        || state.core.position
            != (TurnPosition::Combat {
                step: CombatStep::CombatDamage,
            })
        || state.core.priority != PriorityState::None
        || combat.damage_step_completed
    {
        return Err(Error::InvalidResult);
    }
    Ok(())
}

/// The request that asks the attacking player how much of the damage left the
/// next blocker is assigned (CR 510.1c), for the answers `assigned` in the
/// division `continuation` of `player` over `divisions`: one candidate for each
/// amount from 0 to what is left, in ascending order.
fn request_shape(
    state: &EngineState,
    divisions: &[Division],
    continuation: ContinuationId,
    player: PlayerId,
    assigned: &BTreeMap<GameObjectId, u64>,
) -> Result<RequestShape, Error> {
    let Progress::Asking {
        attacker,
        blocker,
        remaining,
    } = progress_of(divisions, assigned)?
    else {
        return Err(Error::InvalidResult);
    };
    let (attacker, recipient) = (
        divisions[attacker].attacker,
        divisions[attacker].blockers[blocker],
    );
    let identity = state
        .perspective_identities
        .players
        .get(&player)
        .ok_or(Error::InvalidResult)?;
    let opaque = |object: &GameObjectId| {
        identity
            .object_to_opaque
            .get(object)
            .copied()
            .ok_or(Error::InvalidResult)
    };
    let (visible_attacker, visible_recipient) = (opaque(&attacker)?, opaque(&recipient)?);
    let raw = (0..=remaining)
        .map(|amount| {
            (
                CandidateIntent::AssignCombatDamage {
                    attacker: visible_attacker,
                    recipient: visible_recipient,
                    amount,
                },
                EngineCandidateBinding::AssignCombatDamage {
                    attacker,
                    recipient,
                    amount,
                },
            )
        })
        .collect();
    Ok(RequestShape {
        actor: player,
        visibility: DecisionVisibility::ActingPlayerOnly,
        continuation_id: Some(continuation),
        purpose: DecisionPurposeV4::CombatDamageAssignment,
        decision_domain_v2: DecisionDomainV2::ChooseOne,
        candidates: CandidateOrdering::assign_dense(raw).map_err(|_| Error::InvalidResult)?,
    })
}

/// The request a state with a pending division calls for. It follows from the
/// continuation and the battlefield; `validate_pending_damage_division` is what
/// says the continuation is right.
pub(crate) fn damage_request_shape(
    admission: &ExecutableProfileAdmissionV1,
    state: &EngineState,
) -> Result<RequestShape, Error> {
    let pending = pending_division_of(state)?;
    let divisions = divisions(admission, state)?;
    request_shape(
        state,
        &divisions,
        pending.continuation,
        pending.player,
        pending.assigned,
    )
}

/// What an answer to the pending division makes of it.
pub(crate) enum DamageAnswer {
    /// The division goes on: this request asks about the next blocker.
    Asking(RequestShape),
    /// The division is complete and the damage is dealt; the caller checks the
    /// state-based actions.
    Dealt,
}

/// CR 510.1c: the blocker the pending division asks about, `blocker`, of the
/// attacker `attacker` that is being divided, is assigned `amount` of its
/// damage. With a blocker still to ask, the continuation holds the answer and
/// the request about the next is returned. After the last answer the division is
/// complete: the continuation ends and the damage of every creature is dealt at
/// once (see `deal_combat_damage`). The continuation is out of the state while
/// the creatures are read, as in `begin_damage_division`.
pub(crate) fn assign_combat_damage(
    admission: &ExecutableProfileAdmissionV1,
    next: &mut EngineState,
    facts: &mut Facts,
    attacker: GameObjectId,
    blocker: GameObjectId,
    amount: u64,
) -> Result<DamageAnswer, Error> {
    let continuation = pending_division_of(next)?.continuation;
    let record = next
        .execution
        .continuations
        .remove(&continuation)
        .ok_or(Error::InvalidResult)?;
    let ContinuationPayload::CombatDamageAssignment {
        player,
        pending_attackers,
        pending_blockers,
        mut assigned,
    } = record.payload.clone()
    else {
        return Err(Error::InvalidResult);
    };
    if pending_attackers.first() != Some(&attacker) || pending_blockers.first() != Some(&blocker) {
        return Err(Error::InvalidResult);
    }
    assigned.insert(blocker, amount);
    let divisions = divisions(admission, next)?;
    match progress_of(&divisions, &assigned)? {
        Progress::Asking {
            attacker: first,
            blocker: next_blocker,
            ..
        } => {
            let shape = request_shape(next, &divisions, continuation, player, &assigned)?;
            next.execution.continuations.insert(
                continuation,
                ContinuationRecord {
                    payload: ContinuationPayload::CombatDamageAssignment {
                        player,
                        pending_attackers: divisions[first..]
                            .iter()
                            .map(|division| division.attacker)
                            .collect(),
                        pending_blockers: divisions[first].blockers[next_blocker..].to_vec(),
                        assigned,
                    },
                    ..record
                },
            );
            Ok(DamageAnswer::Asking(shape))
        }
        Progress::Done(divided) => {
            deal_combat_damage(admission, next, facts, &divided)?;
            Ok(DamageAnswer::Dealt)
        }
    }
}

/// A combat in a restored or committed state is one this slice could have
/// produced; anything else fails closed:
/// - it exists from the declaration to the end of combat;
/// - the attackers are creatures on the battlefield that the active player
///   controls (CR 508.1a), and they attack the other player (CR 506.2);
/// - each attacker is tapped (CR 508.1f) and has been under its controller's
///   control since the turn began (CR 302.6, 508.1a);
/// - a combat with attackers has dealt its damage from the damage step on
///   (CR 508.8, 510.1, 510.3): the turn-based action runs on entering the step,
///   so no game rests there before it, and the end of combat step is reached
///   only through it. The one exception is the division of an attacker's
///   damage among its blockers (CR 510.1c), which the player is asked for in the
///   step: the damage step with the damage undealt is accepted only while that
///   division is pending and is the one the battlefield calls for (see
///   `validate_pending_damage_division`). The damage step is checked here. The
///   end of combat step is checked by `validate_combat` in `mtgml-state`, which
///   every restore and every commit runs. With no attackers the step is
///   skipped (CR 508.8) and the flag stays false;
/// - no attacker is blocked while attackers are still being declared
///   (CR 509.1, 508.2), and every blocker is an untapped creature the defending
///   player controls (CR 509.1a): nothing taps a blocker once it blocks. An
///   attacker may be blocked with no blocker left (CR 509.1h), and a blocker
///   may block no attacker (CR 509.1g), but only once the damage is dealt: a
///   creature leaves combat only by dying in the state-based actions after the
///   damage step (CR 704.5g, 506.4), so before it every blocker blocks an
///   attacker, and the blocked attackers are exactly the ones a blocker names;
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
    let dividing = state.execution.continuations.values().any(|record| {
        matches!(
            record.payload,
            ContinuationPayload::CombatDamageAssignment { .. }
        )
    });
    let Some(combat) = state.combat.as_ref() else {
        return if declaring || dividing {
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
        && !dividing
    {
        return Err(Error::TurnProgressUnsupported);
    }
    if step == CombatStep::DeclareAttackers
        && (!combat.blockers.is_empty() || !combat.blocked_attackers.is_empty())
    {
        return Err(Error::TurnProgressUnsupported);
    }
    // A creature leaves combat only by dying in the state-based actions after
    // the combat damage step, so until the damage is dealt every blocker has an
    // attacker, and the blocked attackers are exactly the ones a blocker names.
    if !combat.damage_step_completed {
        // `None` when some blocker has no attacker.
        let named_by_blockers = combat
            .blockers
            .values()
            .copied()
            .collect::<Option<BTreeSet<_>>>();
        if named_by_blockers.as_ref() != Some(&combat.blocked_attackers) {
            return Err(Error::TurnProgressUnsupported);
        }
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
    if dividing && validate_pending_damage_division(admission, state).is_err() {
        return Err(Error::TurnProgressUnsupported);
    }
    Ok(())
}

/// CR 506.4: the creature `object`, which leaves the battlefield, is removed
/// from combat and stops being an attacking, blocking, blocked creature.
/// - An attacker leaves `attackers` and `blocked_attackers`. The creatures that
///   blocked it stay blocking creatures until combat ends (CR 509.1g): each is
///   recorded as blocking no attacker.
/// - A blocker leaves the blocks. The attacker it blocked stays blocked with no
///   blocker left (CR 509.1h): it stays in `blocked_attackers`.
pub(crate) fn remove_from_combat(next: &mut EngineState, object: GameObjectId) {
    let Some(combat) = next.combat.as_mut() else {
        return;
    };
    combat.attackers.retain(|attacker| *attacker != object);
    combat.blocked_attackers.remove(&object);
    combat.blockers.remove(&object);
    for attacker in combat.blockers.values_mut() {
        if *attacker == Some(object) {
            *attacker = None;
        }
    }
}

/// CR 510.1, 510.2: every attacking and blocking creature deals combat damage
/// equal to its power, all at once:
/// - an unblocked attacker deals it to the defending player (CR 510.1b), who
///   loses that much life (CR 120.3a);
/// - an attacker with exactly one blocker deals all of it to that blocker; one
///   that is blocked with no blocker left deals none (CR 510.1c, 509.1h);
/// - an attacker with two or more blockers deals it to them divided as its
///   controller chose (CR 510.1c): `divided` gives each blocker of such an
///   attacker its amount, the ones the rules force included (see
///   `progress_of`), and they add up to the attacker's power;
/// - a blocker deals it to the attacker it blocks (CR 510.1d); one whose
///   attacker was removed from combat blocks nothing and deals none.
///
/// Damage dealt to a creature is marked on it (CR 120.3e). A creature that
/// would assign 0 or less damage assigns none (CR 510.1a), and nothing is
/// dealt in an amount of 0. A `divided` that does not fit the combat fails
/// closed. Whether a creature has been dealt lethal damage is for the
/// state-based actions that follow the step (see `crate::state_based_actions`).
pub(crate) fn deal_combat_damage(
    admission: &ExecutableProfileAdmissionV1,
    next: &mut EngineState,
    facts: &mut Facts,
    divided: &BTreeMap<GameObjectId, u64>,
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
    let mut divided_blockers = 0;
    for attacker in &combat.attackers {
        let amount = assigned(attacker, next.core.active_player)?;
        let blockers: Vec<GameObjectId> = combat
            .blockers
            .iter()
            .filter(|(_, blocked)| **blocked == Some(*attacker))
            .map(|(blocker, _)| *blocker)
            .collect();
        let recipient = match blockers[..] {
            [] if combat.blocked_attackers.contains(attacker) => continue,
            [] => DamageRecipientV1::Player {
                player: combat.defending_player,
            },
            [blocker] => DamageRecipientV1::Creature { object: blocker },
            _ => {
                // CR 510.1c, 510.1a: the damage is divided among the blockers,
                // all of it; with none to assign, none is divided.
                if amount > 0 {
                    let mut total: u64 = 0;
                    for blocker in &blockers {
                        let share = *divided.get(blocker).ok_or(Error::InvalidResult)?;
                        total = total.checked_add(share).ok_or(Error::InvalidResult)?;
                        divided_blockers += 1;
                        if share > 0 {
                            assignments.push(DamageAssignmentV1 {
                                source: *attacker,
                                recipient: DamageRecipientV1::Creature { object: *blocker },
                                amount: share,
                            });
                        }
                    }
                    if total != amount {
                        return Err(Error::InvalidResult);
                    }
                }
                continue;
            }
        };
        if amount > 0 {
            assignments.push(DamageAssignmentV1 {
                source: *attacker,
                recipient,
                amount,
            });
        }
    }
    if divided_blockers != divided.len() {
        return Err(Error::InvalidResult);
    }
    for (blocker, attacker) in &combat.blockers {
        // A blocker whose attacker was removed from combat blocks no creature,
        // so it assigns no damage (CR 510.1d).
        let Some(attacker) = attacker else {
            continue;
        };
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
        // Combat damage is public (CR 510.2): both players observe all of it,
        // dealt at once, before the life lost and the creatures that die.
        observe_public(
            next,
            facts,
            AuthoritativeRuleEventKind::CombatDamageDealt { assignments },
        )?;
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
    // The observation shows the damage marked on every permanent, so no
    // observed event repeats it: no player observes these rule events.
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

/// CR 514.2, 120.6: all damage marked on permanents is removed, all at once,
/// in the cleanup step. Each creature that had some has one
/// `MarkedDamageChanged` to 0, in object order. No player observes the event:
/// the observation shows the damage marked, so the removal is seen as the mark
/// going to 0.
pub(crate) fn remove_marked_damage(next: &mut EngineState, facts: &mut Facts) {
    for (creature, from) in next.card_rules.permanents.remove_marked_damage() {
        record_unobserved(
            facts,
            AuthoritativeRuleEventKind::MarkedDamageChanged {
                creature,
                from,
                to: 0,
            },
        );
    }
}

/// CR 120.6, 514.2: damage is marked by combat damage and stays marked until
/// the cleanup step removes it, so some can exist from the combat damage step,
/// once its damage is dealt, through the end of combat, the postcombat main
/// phase and the end step. The cleanup step first has the player discard to
/// their maximum hand size (CR 514.1), and only then removes the damage
/// (CR 514.2): while that discard is asked it is still marked. The cleanup
/// step has no other decision, so damage is nowhere else in it.
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
        TurnPosition::Ending {
            step: EndingStep::Cleanup,
        } => state
            .execution
            .pending_decision
            .as_ref()
            .is_some_and(|request| request.purpose == DecisionPurposeV4::HandSizeDiscard),
        _ => false,
    }
}

/// The damage marked on the permanents is damage this slice can have made:
/// - it is marked only in the window of `marked_damage_may_exist`: none in the
///   beginning phase, the precombat main phase, the combat steps before the
///   damage is dealt, or the cleanup step but for the discard that comes
///   before the damage is removed;
/// - only a creature has any (CR 120.3e);
/// - no creature has been dealt lethal damage: marked damage at least equal to
///   its toughness (CR 704.5g; with toughness 0 or less it is destroyed as
///   well, CR 704.5f, which no card of this slice has). It is destroyed by the
///   state-based actions before any player has priority (CR 704.3), so no
///   game rests there. The one exception is a decision made in the middle of
///   those actions: while an owner is asked for the order of their cards
///   (CR 404.3), the creatures that die with that order are still on the
///   battlefield with their lethal damage (see
///   `crate::state_based_actions::creatures_awaiting_the_order`), and only
///   those.
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
    let dying = crate::state_based_actions::creatures_awaiting_the_order(state);
    for creature in &creatures {
        let permanent = permanents
            .get(&creature.object)
            .ok_or(Error::InvalidResult)?;
        if i128::from(permanent.marked_damage) >= i128::from(creature.toughness)
            && !dying.contains(&creature.object)
        {
            return Err(Error::TurnProgressUnsupported);
        }
    }
    Ok(())
}
