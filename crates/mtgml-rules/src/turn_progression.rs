//! Native turn progression for the slice of lands and vanilla creatures.
//!
//! Passing priority and every turn-based action run directly on V3 state:
//! step changes, untap, draw, attackers, blockers and combat damage, cleanup
//! and the turn change. One response is one transition
//! (`StateRevision` +1, one `StateDelta`). V3 validates turn-position,
//! priority, active-player and turn-number events against the transition's
//! endpoints, so each changed aspect gets exactly one net event.
//!
//! What a step reports, and what it does not:
//! - A step that crosses several positions reports one
//!   `TurnPositionChanged` from the first to the last. The positions in
//!   between are not reported as positions, only through their effects:
//!   end step to upkeep shows cleanup only as a discard and the turn change,
//!   untap only as `UntapCompleted`.
//! - Events are grouped by kind, not ordered by time: turn and combat events
//!   first, then zone changes, then emptied mana pools, then the next
//!   decision. A cleanup discard is therefore listed after the next turn's
//!   untap, and a draw before the pools that emptied at the end of upkeep.

use mtgml_card_ir::ExecutableProfileAdmissionV1;
use mtgml_decision::{
    AuthoritativeCandidate, AuthoritativeDecisionRequest, CandidateIntent, DecisionAnswerV2,
    DecisionDomainV2, DecisionPurposeV4, DecisionResponseV3, DecisionVisibility,
    EngineCandidateBinding,
};
use mtgml_model::{
    DecisionId, EpisodeStatus, GameObjectId, PlayerDecisionIdV1, PlayerId, PlayerOutcome,
    PlayerResult, RuleEventId, StateRevision, TerminalReason, ZoneKind,
};
use mtgml_state::{
    BeginningStep, CombatStep, EndingStep, EngineState, ManaPoolChangeCauseV1,
    PerspectiveLifecycleAuditV1, PriorityState, SbaSelectedActionV1, SemanticDeltaOperation,
    StateDelta, TurnHistoryStateV1, TurnPosition, VisibilityPartition, ZoneKey, ZoneLocation,
    ZonePosition,
};

use crate::{
    AuthoritativeRuleEvent, AuthoritativeRuleEventKind, BasicLandCandidateError,
    BasicLandTransitionError as Error, BasicLandTransitionProduct, MagicActionRequestV1,
    SelectedSuccessorDecisionV1,
};

/// Executes one V4 response. Land plays and mana abilities use the
/// basic-land path; passing priority, casting a spell, paying for it and
/// declaring attackers and blockers run the turn progression.
pub fn execute_magic_response(
    admission: &ExecutableProfileAdmissionV1,
    state: &EngineState,
    actor: PlayerId,
    response: &DecisionResponseV3,
    status: &EpisodeStatus,
) -> Result<BasicLandTransitionProduct, Error> {
    let request = state
        .execution
        .pending_decision
        .as_ref()
        .ok_or(Error::InvalidSelection)?;
    if request.actor != actor || request.validate_response(response).is_err() {
        return Err(Error::InvalidSelection);
    }
    if crate::game_start::is_pregame_purpose(&request.purpose) {
        return crate::game_start::execute_pregame_response(
            admission, state, request, response, status,
        );
    }
    validate_slice(admission, state)?;
    let answer = match request.purpose {
        DecisionPurposeV4::PriorityAction => {
            match crate::selected_basic_land_action(admission, state, actor, response, status)
                .map_err(|_| Error::InvalidSelection)?
            {
                SelectedSuccessorDecisionV1::MagicAction(MagicActionRequestV1::CastSpell {
                    actor,
                    object,
                }) => {
                    if !matches!(status, EpisodeStatus::Running) {
                        return Err(Error::InvalidSelection);
                    }
                    return cast(admission, state, request, actor, object);
                }
                SelectedSuccessorDecisionV1::MagicAction(_) => {
                    return crate::execute_basic_land_response(
                        admission, state, actor, response, status,
                    );
                }
                SelectedSuccessorDecisionV1::PassPriority => Answer::Pass,
            }
        }
        // CR 601.2h: the caster chose how to pay for the spell on the stack.
        DecisionPurposeV4::ManaPayment => {
            validate_magic_pending_request(admission, state, status)
                .map_err(|_| Error::InvalidSelection)?;
            let DecisionAnswerV2::SelectOne { candidate_id } = &response.answer else {
                return Err(Error::InvalidSelection);
            };
            let Some(EngineCandidateBinding::SelectManaPayment { spent_buckets }) = request
                .candidates
                .iter()
                .find(|candidate| candidate.candidate_id == *candidate_id)
                .map(|candidate| &candidate.trusted_binding)
            else {
                return Err(Error::InvalidSelection);
            };
            if !matches!(status, EpisodeStatus::Running) {
                return Err(Error::InvalidSelection);
            }
            return pay(admission, state, request, *spent_buckets);
        }
        DecisionPurposeV4::AttackerDeclaration => {
            validate_magic_pending_request(admission, state, status)
                .map_err(|_| Error::InvalidSelection)?;
            let DecisionAnswerV2::SelectMany { candidate_ids } = &response.answer else {
                return Err(Error::InvalidSelection);
            };
            let mut attackers = candidate_ids
                .iter()
                .map(|chosen| {
                    match request
                        .candidates
                        .iter()
                        .find(|candidate| candidate.candidate_id == *chosen)
                        .map(|candidate| &candidate.trusted_binding)
                    {
                        Some(EngineCandidateBinding::SelectObject { object }) => Ok(*object),
                        _ => Err(Error::InvalidSelection),
                    }
                })
                .collect::<Result<Vec<_>, _>>()?;
            // Candidates are in opaque order; the combat state is in object order.
            attackers.sort();
            Answer::Attackers(attackers)
        }
        // CR 509.1a: the defending player chose, for one creature, which
        // attacker it blocks, or none.
        DecisionPurposeV4::BlockerDeclaration => {
            validate_magic_pending_request(admission, state, status)
                .map_err(|_| Error::InvalidSelection)?;
            let DecisionAnswerV2::SelectOne { candidate_id } = &response.answer else {
                return Err(Error::InvalidSelection);
            };
            let Some(EngineCandidateBinding::DeclareBlock { blocker, attacker }) = request
                .candidates
                .iter()
                .find(|candidate| candidate.candidate_id == *candidate_id)
                .map(|candidate| &candidate.trusted_binding)
            else {
                return Err(Error::InvalidSelection);
            };
            Answer::Block {
                blocker: *blocker,
                attacker: *attacker,
            }
        }
        DecisionPurposeV4::HandSizeDiscard => {
            validate_magic_pending_request(admission, state, status)
                .map_err(|_| Error::InvalidSelection)?;
            let DecisionAnswerV2::SelectMany { candidate_ids } = &response.answer else {
                return Err(Error::InvalidSelection);
            };
            let [chosen] = candidate_ids.as_slice() else {
                return Err(Error::InvalidSelection);
            };
            match request
                .candidates
                .iter()
                .find(|candidate| candidate.candidate_id == *chosen)
                .map(|candidate| &candidate.trusted_binding)
            {
                Some(EngineCandidateBinding::SelectObject { object }) => Answer::Discard(*object),
                _ => return Err(Error::InvalidSelection),
            }
        }
        _ => return Err(Error::InvalidSelection),
    };
    if !matches!(status, EpisodeStatus::Running) {
        return Err(Error::InvalidSelection);
    }
    progress(admission, state, request, answer)
}

/// Validates the pending V4 request of a restored or committed state: priority
/// windows through the basic-land candidate owner, the other requests against
/// the request this progression creates.
pub fn validate_magic_pending_request(
    admission: &ExecutableProfileAdmissionV1,
    state: &EngineState,
    status: &EpisodeStatus,
) -> Result<(), BasicLandCandidateError> {
    if let Some(request) = state
        .execution
        .pending_decision
        .as_ref()
        .filter(|request| crate::game_start::is_pregame_purpose(&request.purpose))
    {
        return crate::game_start::validate_pregame_request(admission, state, request, status);
    }
    // Priority windows are validated by the basic-land owner, which does not
    // know combat: a restored combat is checked here, for every request and
    // every status.
    if !hands_within_slice(state)
        || !permanents_controlled_by_their_owners(state)
        || (matches!(status, EpisodeStatus::Running) && state_based_action_pending(state))
        || crate::combat::validate_reachable_combat(admission, state).is_err()
        || crate::combat::validate_marked_damage(admission, state).is_err()
    {
        return Err(BasicLandCandidateError::InvalidState);
    }
    let Some(request) = state.execution.pending_decision.as_ref().filter(|request| {
        matches!(
            request.purpose,
            DecisionPurposeV4::AttackerDeclaration
                | DecisionPurposeV4::BlockerDeclaration
                | DecisionPurposeV4::HandSizeDiscard
                | DecisionPurposeV4::ManaPayment
        )
    }) else {
        return crate::validate_basic_land_pending_request(admission, state, status);
    };
    if request.purpose == DecisionPurposeV4::HandSizeDiscard {
        return validate_discard_request(admission, state, request, status);
    }
    if request.purpose == DecisionPurposeV4::BlockerDeclaration {
        return validate_block_request(admission, state, request, status);
    }
    if request.purpose == DecisionPurposeV4::ManaPayment {
        return validate_payment_request(admission, state, request, status);
    }
    validate_attacker_request(admission, state, request, status)
}

enum Answer {
    Pass,
    /// The creatures the active player declares as attackers, in object order.
    Attackers(Vec<GameObjectId>),
    /// The defending player's choice for one creature: it blocks `attacker`,
    /// or nothing.
    Block {
        blocker: GameObjectId,
        attacker: Option<GameObjectId>,
    },
    Discard(GameObjectId),
}

pub(crate) enum NextDecision {
    Priority(PlayerId),
    /// The caster of the spell on the stack chooses how to pay for it.
    Payment,
    Attackers,
    /// The defending player is asked about the next creature of a block
    /// declaration.
    Blockers,
    Discard,
    /// The next request of the start of the game (CR 103).
    Pregame,
    /// The game ended: `loser` lost to a state-based action.
    GameOver {
        loser: PlayerId,
    },
}

/// What changed during the transition beyond the endpoint fields.
#[derive(Default)]
pub(crate) struct Facts {
    pub(crate) untapped: Option<Vec<GameObjectId>>,
    combat_skipped: bool,
    combat_ended: bool,
    /// Zone moves, shuffles and public events in the order they happened,
    /// each followed by the occurrences it caused (none, for an event no
    /// player observes).
    pub(crate) zone_events: Vec<crate::zone_incarnation::ZoneMoveEvent>,
}

/// Records a public rule event where it happens: every player observes it,
/// so each perspective's visible sequence advances now, before whatever
/// happens next in the same transition.
pub(crate) fn observe_public(
    next: &mut EngineState,
    facts: &mut Facts,
    event: AuthoritativeRuleEventKind,
) -> Result<(), Error> {
    facts
        .zone_events
        .push(crate::zone_incarnation::ZoneMoveEvent::Public(Box::new(
            event,
        )));
    let perspectives: Vec<PlayerId> = next.core.players.keys().copied().collect();
    for perspective in perspectives {
        let lifecycle = PerspectiveLifecycleAuditV1 {
            perspective,
            sequence: next
                .knowledge
                .players
                .get(&perspective)
                .ok_or(Error::InvalidResult)?
                .next_visible_sequence,
            mutation: Default::default(),
        };
        mtgml_state::apply_perspective_lifecycle(next, &lifecycle)
            .map_err(|_| Error::InvalidResult)?;
        facts
            .zone_events
            .push(crate::zone_incarnation::ZoneMoveEvent::Occurrence(
                lifecycle,
            ));
    }
    Ok(())
}

/// Records a rule event where it happens that no player observes: no
/// observation occurrence follows it.
pub(crate) fn record_unobserved(facts: &mut Facts, event: AuthoritativeRuleEventKind) {
    facts
        .zone_events
        .push(crate::zone_incarnation::ZoneMoveEvent::Public(Box::new(
            event,
        )));
}

/// The slice this progression can evaluate (D13): exactly two players, only
/// admitted lands and vanilla creatures on the battlefield, each controlled by
/// its owner, a stack that is empty or holds one creature spell that the active
/// player cast in a main phase (see `crate::casting::stack_within_profile`), no
/// continuation but the payment of that spell or the defending player's block
/// declaration (see `crate::combat::validate_pending_block_declaration`), a combat that
/// this slice could have produced (see `crate::combat::validate_reachable_combat`),
/// and none of the state no rule of this slice can evaluate. State-based actions
/// are checked before a player would receive priority (CR 704.3), so a decision
/// is never pending while one applies: a player at 0 or less life who has not
/// lost is not a state of this slice (CR 704.5a), nor is a creature with lethal
/// damage marked on it (CR 704.5g; see `crate::combat::validate_marked_damage`).
fn validate_slice(
    admission: &ExecutableProfileAdmissionV1,
    state: &EngineState,
) -> Result<(), Error> {
    let parts = state;
    let execution = &state.execution;
    let cards = &state.card_rules;
    if parts.core.players.len() != 2
        || cards.counters != Default::default()
        || cards.attachments != Default::default()
        || (!execution.continuations.is_empty()
            && crate::casting::pending_payment(admission, state).is_err()
            && crate::combat::validate_pending_block_declaration(admission, state).is_err())
        || !execution.effects.is_empty()
        || !execution.waiting_triggers.is_empty()
        || !execution.delayed_effects.is_empty()
        || !crate::casting::stack_within_profile(admission, state)
        || state_based_action_pending(state)
    {
        return Err(Error::TurnProgressUnsupported);
    }
    if !hands_within_slice(state) || !permanents_controlled_by_their_owners(state) {
        return Err(Error::TurnProgressUnsupported);
    }
    crate::S1QueryAuthority::for_objects(admission, state, &battlefield_objects(state))
        .map_err(|_| Error::TurnProgressUnsupported)?;
    crate::combat::validate_reachable_combat(admission, state)?;
    crate::combat::validate_marked_damage(admission, state)
}

/// CR 704.5a: a player who has not lost and is at 0 or less life loses the
/// game the next time state-based actions are checked, before any player would
/// receive priority (CR 704.3).
fn state_based_action_pending(state: &EngineState) -> bool {
    state
        .core
        .players
        .values()
        .any(|player| !player.has_lost && player.life <= 0)
}

/// A permanent's controller is by default the player under whose control it
/// entered the battlefield (CR 110.2), and a permanent spell enters under its
/// controller's control (CR 608.3a), who is its owner (CR 112.2, 302.1). No
/// card of this slice changes control, so every permanent is controlled by its
/// owner; a state in which one is not is not one this slice can have made.
fn permanents_controlled_by_their_owners(state: &EngineState) -> bool {
    battlefield_objects(state).iter().all(|object| {
        state
            .zones
            .objects
            .get(object)
            .is_some_and(|object| object.controller == object.owner)
    })
}

pub(crate) fn battlefield_objects(state: &EngineState) -> Vec<GameObjectId> {
    state
        .zones
        .locations
        .iter()
        .filter(|(_, location)| location.zone == ZoneKind::Battlefield)
        .map(|(object, _)| *object)
        .collect()
}

/// One draw per turn (CR 504.1) and the discard to maximum hand size at each
/// cleanup (CR 514.1) keep the non-active player at seven cards or fewer and
/// the active player at seven before their draw and eight after it. A larger
/// hand could only reach a cleanup with several simultaneous discards, which
/// need the owner's graveyard order that this slice does not offer.
fn hands_within_slice(state: &EngineState) -> bool {
    let core = &state.core;
    let before_draw = matches!(
        core.position,
        TurnPosition::Beginning {
            step: BeginningStep::Untap | BeginningStep::Upkeep
        }
    );
    core.players.keys().all(|player| {
        let hand = state
            .zones
            .locations
            .values()
            .filter(|location| location.zone == ZoneKind::Hand && location.player == Some(*player))
            .count();
        let limit = if *player == core.active_player && !before_draw {
            crate::turn_structure::ORDINARY_MAXIMUM_HAND_SIZE + 1
        } else {
            crate::turn_structure::ORDINARY_MAXIMUM_HAND_SIZE
        };
        hand <= limit
    })
}

pub(crate) fn admits(admission: &ExecutableProfileAdmissionV1, key: &str) -> Result<(), Error> {
    if admission
        .resolved_capabilities()
        .iter()
        .any(|capability| capability.key == key)
    {
        Ok(())
    } else {
        Err(Error::TurnProgressUnsupported)
    }
}

fn other_player(state: &EngineState) -> Result<PlayerId, Error> {
    let core = &state.core;
    core.players
        .keys()
        .copied()
        .find(|player| *player != core.active_player)
        .ok_or(Error::InvalidResult)
}

fn progress(
    admission: &ExecutableProfileAdmissionV1,
    before: &EngineState,
    request: &AuthoritativeDecisionRequest,
    answer: Answer,
) -> Result<BasicLandTransitionProduct, Error> {
    let mut next = before.clone();
    next.execution.pending_decision = None;
    next.revision = StateRevision(
        before
            .revision
            .0
            .checked_add(1)
            .ok_or(Error::IdentityExhausted)?,
    );
    let active = next.core.active_player;
    let other = other_player(&next)?;
    let mut facts = Facts::default();
    let next_decision = match answer {
        // CR 117.3d, 117.4: priority passes to the next player; when all
        // players pass in succession the step ends. An action resets the
        // succession (see the basic-land path).
        Answer::Pass => match next.core.priority {
            PriorityState::HeldBy {
                player,
                consecutive_passes: 0,
            } if player == request.actor => {
                let receiver = if player == active { other } else { active };
                next.core.priority = PriorityState::HeldBy {
                    player: receiver,
                    consecutive_passes: 1,
                };
                NextDecision::Priority(receiver)
            }
            PriorityState::HeldBy {
                player,
                consecutive_passes: 1,
            } if player == request.actor => {
                if next.zones.stack_order.is_empty() {
                    next.core.priority = PriorityState::None;
                    advance(admission, &mut next, &mut facts)?
                } else {
                    // CR 608.1, 117.4: with an object on the stack, the top
                    // object resolves instead of the step ending. The active
                    // player then receives priority (CR 117.3b).
                    crate::casting::resolve_top(admission, &mut next, &mut facts)?;
                    open_priority(&mut next)
                }
            }
            _ => return Err(Error::TurnProgressUnsupported),
        },
        // CR 508.1, 508.2: the active player declares attackers (possibly
        // none) and then receives priority in the declare attackers step.
        Answer::Attackers(attackers) => {
            crate::combat::declare_attackers(&mut next, &mut facts, other, attackers)?;
            next.core.priority = PriorityState::HeldBy {
                player: active,
                consecutive_passes: 0,
            };
            NextDecision::Priority(active)
        }
        // CR 509.1a: the defending player answers for one creature. After the
        // last one the active player receives priority (CR 509.2).
        Answer::Block { blocker, attacker } => {
            if crate::combat::declare_block(&mut next, &mut facts, blocker, attacker)? {
                open_priority(&mut next)
            } else {
                NextDecision::Blockers
            }
        }
        // CR 514.1: the discard ends cleanup; the turn then ends (CR 514.3).
        Answer::Discard(object) => {
            move_card(
                &mut next,
                object,
                crate::zone_incarnation::SelectedZoneTransitionKind::HandToOwnerGraveyard,
                ZoneLocation {
                    zone: ZoneKind::Graveyard,
                    player: Some(active),
                    position: ZonePosition::Top { offset: 0 },
                    visibility: VisibilityPartition::Public,
                    partition: None,
                },
                &mut facts,
            )?;
            advance(admission, &mut next, &mut facts)?
        }
    };
    finish(admission, before, request, next, facts, next_decision)
}

/// CR 601.2: `caster` casts `card`. With one way to pay, the cast is complete
/// and the caster receives priority (CR 117.3c). With several, the card is on
/// the stack and the caster is asked how to pay (CR 601.2h).
fn cast(
    admission: &ExecutableProfileAdmissionV1,
    before: &EngineState,
    request: &AuthoritativeDecisionRequest,
    caster: PlayerId,
    card: GameObjectId,
) -> Result<BasicLandTransitionProduct, Error> {
    let options = crate::casting::payment_options_of(admission, before, caster, card)?;
    let mut next = before.clone();
    next.execution.pending_decision = None;
    next.revision = StateRevision(
        before
            .revision
            .0
            .checked_add(1)
            .ok_or(Error::IdentityExhausted)?,
    );
    let mut facts = Facts::default();
    let next_decision = match options.as_slice() {
        [] => return Err(Error::InvalidSelection),
        [spent] => {
            crate::casting::cast_spell(admission, &mut next, caster, card, *spent, &mut facts)?;
            hold_priority(&mut next, caster)
        }
        _ => {
            crate::casting::begin_cast(admission, &mut next, caster, card, &mut facts)?;
            NextDecision::Payment
        }
    };
    finish(admission, before, request, next, facts, next_decision)
}

/// CR 601.2h, 601.2i: the caster of the spell on the stack pays for it with
/// `spent`, and then receives priority (CR 117.3c).
fn pay(
    admission: &ExecutableProfileAdmissionV1,
    before: &EngineState,
    request: &AuthoritativeDecisionRequest,
    spent: [u32; 12],
) -> Result<BasicLandTransitionProduct, Error> {
    let mut next = before.clone();
    next.execution.pending_decision = None;
    next.revision = StateRevision(
        before
            .revision
            .0
            .checked_add(1)
            .ok_or(Error::IdentityExhausted)?,
    );
    let mut facts = Facts::default();
    crate::casting::complete_cast(&mut next, spent, &mut facts)?;
    let next_decision = hold_priority(&mut next, request.actor);
    finish(admission, before, request, next, facts, next_decision)
}

/// An action ends any succession of passes (CR 117.4): `player`, who acted,
/// holds priority again.
fn hold_priority(next: &mut EngineState, player: PlayerId) -> NextDecision {
    next.core.priority = PriorityState::HeldBy {
        player,
        consecutive_passes: 0,
    };
    NextDecision::Priority(player)
}

/// Ends the current step and performs turn-based actions until a step in
/// which a player must act (CR 500.2, 500.3).
fn advance(
    admission: &ExecutableProfileAdmissionV1,
    next: &mut EngineState,
    facts: &mut Facts,
) -> Result<NextDecision, Error> {
    loop {
        let from = next.core.position;
        if from
            == (TurnPosition::Combat {
                step: CombatStep::EndOfCombat,
            })
        {
            // CR 511.3: creatures stop being attacking at end of combat.
            next.combat = None;
            facts.combat_ended = true;
        }
        let mut to = crate::temporal_successor(from);
        // CR 508.8: with no attackers, skip declare blockers and damage.
        if to
            == (TurnPosition::Combat {
                step: CombatStep::DeclareBlockers,
            })
            && next
                .combat
                .as_ref()
                .is_some_and(|combat| combat.attackers.is_empty())
        {
            to = TurnPosition::Combat {
                step: CombatStep::EndOfCombat,
            };
            facts.combat_skipped = true;
        }
        next.core.position = to;
        let active = next.core.active_player;
        match to {
            // CR 502.2-502.4: a new turn begins; the active player untaps;
            // no player receives priority in the untap step.
            TurnPosition::Beginning {
                step: BeginningStep::Untap,
            } => begin_turn(next, facts)?,
            TurnPosition::Beginning {
                step: BeginningStep::Upkeep,
            }
            | TurnPosition::PrecombatMain
            | TurnPosition::PostcombatMain
            | TurnPosition::Ending {
                step: EndingStep::EndStep,
            } => return Ok(open_priority(next)),
            TurnPosition::Beginning {
                step: BeginningStep::Draw,
            } => {
                admits(admission, "rules/draw-card")?;
                // CR 103.8a, 500.11: the starting player skips the draw step
                // of turn 1; the turn proceeds as though it did not exist.
                if next.core.turn_number == 1 {
                    continue;
                }
                if library_top(next, active).is_none() {
                    // CR 121.4, 704.5b: drawing from an empty library
                    // loses the game when state-based actions are next
                    // checked, before anyone receives priority (CR 117.5).
                    admits(admission, "rules/state-based-actions-empty-library")?;
                    next.core
                        .players
                        .get_mut(&active)
                        .ok_or(Error::InvalidResult)?
                        .has_lost = true;
                    return Ok(NextDecision::GameOver { loser: active });
                }
                draw(next, active, facts)?;
                return Ok(open_priority(next));
            }
            // The beginning of combat is part of the turn structure every
            // admission has (as in the accepted basic-land slice); declaring
            // attackers and the rest of combat need their own capabilities.
            TurnPosition::Combat {
                step: CombatStep::BeginningOfCombat,
            } => return Ok(open_priority(next)),
            TurnPosition::Combat {
                step: CombatStep::EndOfCombat,
            } => {
                admits(admission, "rules/combat-phase")?;
                return Ok(open_priority(next));
            }
            TurnPosition::Combat {
                step: CombatStep::DeclareAttackers,
            } => {
                admits(admission, "rules/declare-attackers")?;
                return Ok(NextDecision::Attackers);
            }
            // Only reachable with declared attackers (CR 508.8).
            TurnPosition::Combat {
                step: CombatStep::DeclareBlockers,
            } => {
                admits(admission, "rules/declare-blockers")?;
                // CR 509.1a: a defender with an untapped creature declares
                // blockers, one creature at a time.
                if crate::combat::begin_block_declaration(admission, next)? {
                    return Ok(NextDecision::Blockers);
                }
                // CR 509.2: with nothing to declare, the active player gets
                // priority.
                return Ok(open_priority(next));
            }
            TurnPosition::Combat {
                step: CombatStep::CombatDamage,
            } => {
                admits(admission, "rules/combat-damage")?;
                admits(admission, "rules/damage-and-life")?;
                crate::combat::deal_combat_damage(admission, next, facts)?;
                // CR 704.3: state-based actions are checked before the active
                // player gets priority (CR 510.3). A creature dealt lethal
                // damage would be destroyed (CR 704.5g), which is not
                // supported yet: the step fails closed.
                crate::combat::validate_marked_damage(admission, next)?;
                return Ok(
                    match crate::combat::player_at_zero_life_loses(admission, next)? {
                        Some(loser) => NextDecision::GameOver { loser },
                        None => open_priority(next),
                    },
                );
            }
            // CR 514.1-514.3: the active player discards to maximum hand
            // size, then the turn ends without priority.
            TurnPosition::Ending {
                step: EndingStep::Cleanup,
            } => {
                admits(admission, "rules/cleanup-reset")?;
                let hand = next
                    .zones
                    .locations
                    .values()
                    .filter(|location| {
                        location.zone == ZoneKind::Hand && location.player == Some(active)
                    })
                    .count();
                match hand.checked_sub(crate::turn_structure::ORDINARY_MAXIMUM_HAND_SIZE) {
                    None | Some(0) => {}
                    Some(1) => return Ok(NextDecision::Discard),
                    // Two or more simultaneous discards need the owner's
                    // graveyard order, which this slice does not offer.
                    Some(_) => return Err(Error::TurnProgressUnsupported),
                }
            }
        }
    }
}

pub(crate) fn open_priority(next: &mut EngineState) -> NextDecision {
    let active = next.core.active_player;
    next.core.priority = PriorityState::HeldBy {
        player: active,
        consecutive_passes: 0,
    };
    NextDecision::Priority(active)
}

fn begin_turn(next: &mut EngineState, facts: &mut Facts) -> Result<(), Error> {
    let new_active = other_player(next)?;
    let core = &mut next.core;
    core.active_player = new_active;
    core.turn_number = core
        .turn_number
        .checked_add(1)
        .ok_or(Error::IdentityExhausted)?;
    let turn_number = core.turn_number;
    let snapshots = crate::snapshots::object_snapshots(next).map_err(|_| Error::InvalidResult)?;
    let affected =
        crate::turn_structure::derive_ordinary_untap_affected_objects(&snapshots, new_active);
    for object in &affected {
        next.zones
            .objects
            .get_mut(object)
            .ok_or(Error::InvalidResult)?
            .tapped = false;
    }
    facts.untapped = Some(affected);
    let history = &mut next.card_rules.turn_history;
    *history = TurnHistoryStateV1 {
        turn_number,
        players: history
            .players
            .keys()
            .map(|player| (*player, Default::default()))
            .collect(),
        ..TurnHistoryStateV1::default()
    };
    Ok(())
}

fn library_top(state: &EngineState, owner: PlayerId) -> Option<GameObjectId> {
    let library = ZoneLocation {
        zone: ZoneKind::Library,
        player: Some(owner),
        position: ZonePosition::Top { offset: 0 },
        visibility: VisibilityPartition::FaceDown,
        partition: None,
    };
    let key: ZoneKey = library.key();
    state
        .zones
        .ordered_zones
        .get(&key)
        .and_then(|objects| objects.first())
        .copied()
}

/// CR 504.1: the active player draws the top card of their library. The
/// caller handles an empty library.
pub(crate) fn draw(
    next: &mut EngineState,
    owner: PlayerId,
    facts: &mut Facts,
) -> Result<(), Error> {
    let top = library_top(next, owner).ok_or(Error::InvalidResult)?;
    move_card(
        next,
        top,
        crate::zone_incarnation::SelectedZoneTransitionKind::LibraryTopToOwnerHand,
        ZoneLocation {
            zone: ZoneKind::Hand,
            player: Some(owner),
            position: ZonePosition::Unordered,
            visibility: VisibilityPartition::OwnerOnly,
            partition: None,
        },
        facts,
    )?;
    Ok(())
}

/// Moves one card through the shared zone-incarnation authority (new
/// incarnation, knowledge and identity updates) and carries its face over.
pub(crate) fn move_card(
    next: &mut EngineState,
    object: GameObjectId,
    kind: crate::zone_incarnation::SelectedZoneTransitionKind,
    claimed_to: ZoneLocation,
    facts: &mut Facts,
) -> Result<GameObjectId, Error> {
    let claimed_from = next
        .zones
        .locations
        .get(&object)
        .cloned()
        .ok_or(Error::InvalidResult)?;
    let mut events = Vec::new();
    let transition = crate::zone_incarnation::apply_selected_zone_transition_in_workspace(
        next,
        &crate::zone_incarnation::SelectedZoneTransitionRequest {
            object,
            kind,
            claimed_from,
            claimed_to,
        },
        &mut events,
    )
    .map_err(|_| Error::TurnProgressUnsupported)?;
    // The new incarnation shows the same face as the card it came from.
    let face = next
        .card_rules
        .faces
        .faces
        .remove(&transition.old_object)
        .ok_or(Error::InvalidResult)?;
    next.card_rules
        .faces
        .faces
        .insert(transition.new_object, face);
    // A card that leaves the battlefield is no longer a permanent: only the
    // objects on the battlefield keep their entry.
    let battlefield = battlefield_objects(next).into_iter().collect();
    next.card_rules
        .permanents
        .prune_departed_objects(&battlefield);
    facts.zone_events.extend(events);
    Ok(transition.new_object)
}

/// Emits one net event per changed aspect, installs the next decision and
/// builds the validated V3 product.
pub(crate) fn finish(
    admission: &ExecutableProfileAdmissionV1,
    before: &EngineState,
    answered: &AuthoritativeDecisionRequest,
    mut next: EngineState,
    facts: Facts,
    next_decision: NextDecision,
) -> Result<BasicLandTransitionProduct, Error> {
    enum Pending {
        Kind(Box<AuthoritativeRuleEventKind>),
        Occurrence {
            lifecycle: PerspectiveLifecycleAuditV1,
            source: usize,
        },
    }
    let kind = |event: AuthoritativeRuleEventKind| Pending::Kind(Box::new(event));
    let old = &before.core;
    let new = next.core.clone();
    let mut pending = vec![kind(AuthoritativeRuleEventKind::DecisionCleared {
        decision: answered.decision_id,
    })];
    if old.priority != new.priority {
        pending.push(kind(AuthoritativeRuleEventKind::PriorityChanged {
            from: old.priority,
            to: new.priority,
        }));
    }
    if old.position != new.position {
        pending.push(kind(AuthoritativeRuleEventKind::TurnPositionChanged {
            from: old.position,
            to: new.position,
        }));
    }
    if old.active_player != new.active_player {
        pending.push(kind(AuthoritativeRuleEventKind::ActivePlayerChanged {
            from: old.active_player,
            to: new.active_player,
        }));
    }
    if old.turn_number != new.turn_number {
        pending.push(kind(AuthoritativeRuleEventKind::TurnNumberChanged {
            from: old.turn_number,
            to: new.turn_number,
        }));
    }
    if let Some(affected_objects) = facts.untapped {
        pending.push(kind(AuthoritativeRuleEventKind::UntapCompleted {
            affected_objects,
        }));
    }
    if facts.combat_skipped {
        pending.push(kind(AuthoritativeRuleEventKind::EmptyCombatStepsSkipped));
    }
    if facts.combat_ended {
        pending.push(kind(AuthoritativeRuleEventKind::CombatEnded));
    }
    let mut transition_index = None;
    for event in facts.zone_events {
        match event {
            crate::zone_incarnation::ZoneMoveEvent::Public(event) => {
                transition_index = Some(pending.len());
                pending.push(Pending::Kind(event));
            }
            crate::zone_incarnation::ZoneMoveEvent::Shuffle(audit) => {
                let crate::zone_incarnation::LibraryShuffleAudit {
                    player,
                    stream,
                    cursor_before,
                    cursor_after,
                    raw_words_consumed,
                    top_to_bottom,
                } = *audit;
                pending.push(kind(AuthoritativeRuleEventKind::LibraryShuffled {
                    player,
                    stream,
                    cursor_before,
                    cursor_after,
                    raw_words_consumed,
                    top_to_bottom,
                }));
            }
            crate::zone_incarnation::ZoneMoveEvent::Occurrence(lifecycle) => {
                pending.push(Pending::Occurrence {
                    lifecycle,
                    source: transition_index.ok_or(Error::InvalidResult)?,
                });
            }
            crate::zone_incarnation::ZoneMoveEvent::Transition(transition) => {
                transition_index = Some(pending.len());
                pending.push(kind(AuthoritativeRuleEventKind::ZoneTransition {
                    transition,
                }));
            }
        }
    }
    // CR 500.4: mana empties from each player's pool at the end of each
    // step. Pools are public: every player observes each change.
    if old.position != new.position {
        let pools = next.card_rules.mana.pools.clone();
        for (player, pool) in pools {
            if pool == Default::default() {
                continue;
            }
            next.card_rules
                .mana
                .pools
                .insert(player, Default::default());
            let source = pending.len();
            pending.push(Pending::Kind(Box::new(
                AuthoritativeRuleEventKind::ManaPoolChanged {
                    player,
                    before: pool,
                    after: Default::default(),
                    cause: ManaPoolChangeCauseV1::Emptied,
                },
            )));
            let perspectives: Vec<PlayerId> = next.core.players.keys().copied().collect();
            for perspective in perspectives {
                let lifecycle = PerspectiveLifecycleAuditV1 {
                    perspective,
                    sequence: next
                        .knowledge
                        .players
                        .get(&perspective)
                        .ok_or(Error::InvalidResult)?
                        .next_visible_sequence,
                    mutation: Default::default(),
                };
                mtgml_state::apply_perspective_lifecycle(&mut next, &lifecycle)
                    .map_err(|_| Error::InvalidResult)?;
                pending.push(Pending::Occurrence { lifecycle, source });
            }
        }
    }
    let running = EpisodeStatus::Running;
    let (status, request) = match next_decision {
        NextDecision::Priority(actor) => (
            running.clone(),
            Some(
                crate::install_basic_land_request(admission, &mut next, actor, &running)
                    .map_err(|_| Error::InvalidResult)?,
            ),
        ),
        NextDecision::Payment => (
            running,
            Some(crate::casting::install_payment_request(
                admission, &mut next,
            )?),
        ),
        NextDecision::Attackers => (
            running,
            Some(install_attacker_request(admission, &mut next)?),
        ),
        NextDecision::Blockers => (
            running,
            Some(crate::combat::install_block_request(&mut next)?),
        ),
        NextDecision::Pregame => (
            running,
            Some(crate::game_start::install_pregame_request(&mut next)?),
        ),
        NextDecision::Discard => (running, Some(install_discard_request(&mut next)?)),
        // CR 104.2a: in a two-player game the other player wins.
        NextDecision::GameOver { loser } => {
            pending.push(kind(AuthoritativeRuleEventKind::StateBasedActionsApplied {
                actions: vec![SbaSelectedActionV1::PlayerLoses { player: loser }],
            }));
            let players = next
                .core
                .players
                .keys()
                .map(|player| PlayerOutcome {
                    player: *player,
                    result: if *player == loser {
                        PlayerResult::Loss
                    } else {
                        PlayerResult::Win
                    },
                })
                .collect();
            let status = EpisodeStatus::Terminal {
                reason: TerminalReason::RulesLoss,
                players,
            };
            (status, None)
        }
    };
    if let Some(request) = &request {
        pending.push(kind(AuthoritativeRuleEventKind::DecisionCreated {
            decision: request.decision_id,
        }));
    }

    let revision = next.revision;
    let first = before.allocators.next_rule_event_id;
    let mut events = Vec::with_capacity(pending.len());
    for (index, item) in pending.into_iter().enumerate() {
        let event_id = RuleEventId(
            first
                .0
                .checked_add(index as u64)
                .ok_or(Error::IdentityExhausted)?,
        );
        let event = match item {
            Pending::Kind(event) => *event,
            Pending::Occurrence { lifecycle, source } => {
                AuthoritativeRuleEventKind::PerspectiveObservationOccurrence {
                    lifecycle: Box::new(lifecycle),
                    source_event_id: RuleEventId(first.0 + source as u64),
                }
            }
        };
        events.push(AuthoritativeRuleEvent {
            event_id,
            state_revision: revision,
            event,
        });
    }
    next.allocators.next_rule_event_id = RuleEventId(
        first
            .0
            .checked_add(events.len() as u64)
            .ok_or(Error::IdentityExhausted)?,
    );
    let mut operations: Vec<SemanticDeltaOperation> = events
        .iter()
        .flat_map(|event| event.event.semantic_operations())
        .collect();
    // Continuations are private bookkeeping with no rule event of their own.
    let continuations: std::collections::BTreeSet<_> = before
        .execution
        .continuations
        .keys()
        .chain(next.execution.continuations.keys())
        .copied()
        .collect();
    for continuation in continuations {
        let from = before.execution.continuations.get(&continuation);
        let to = next.execution.continuations.get(&continuation);
        if from != to {
            operations.push(SemanticDeltaOperation::ContinuationChanged {
                continuation,
                from: from.map(|record| Box::new(record.payload.clone())),
                to: to.map(|record| Box::new(record.payload.clone())),
            });
        }
    }
    // Resolving or casting changes the stack order. The order is not derived
    // from an event: it is one operation of its own.
    if before.zones.stack_order != next.zones.stack_order {
        operations.push(SemanticDeltaOperation::StackOrderChanged {
            from: before.zones.stack_order.clone(),
            to: next.zones.stack_order.clone(),
        });
    }
    operations.push(SemanticDeltaOperation::PendingRequestChanged {
        from: Some(Box::new(answered.clone())),
        to: request.clone().map(Box::new),
    });
    next.validate_structure()
        .map_err(|_| Error::InvalidResult)?;
    let delta =
        StateDelta::between_structural_only(before, &next, operations).map_err(|_| Error::Delta)?;
    crate::events::validate_events_for_built_delta_v3(before, &next, &events, &delta)
        .map_err(|_| Error::InvalidResult)?;
    Ok(BasicLandTransitionProduct {
        accepted: true,
        next_state: next,
        delta,
        events,
        next_decision: request,
        status,
    })
}

/// The discard request's candidates: every card in the active player's
/// hand, in canonical order of the player's opaque identities.
fn discard_candidates(state: &EngineState) -> Result<Vec<AuthoritativeCandidate>, Error> {
    let parts = state;
    let actor = parts.core.active_player;
    let identity = parts
        .perspective_identities
        .players
        .get(&actor)
        .ok_or(Error::InvalidResult)?;
    let mut cards = parts
        .zones
        .locations
        .iter()
        .filter(|(_, location)| location.zone == ZoneKind::Hand && location.player == Some(actor))
        .map(|(object, _)| {
            identity
                .object_to_opaque
                .get(object)
                .map(|opaque| (*opaque, *object))
                .ok_or(Error::InvalidResult)
        })
        .collect::<Result<Vec<_>, _>>()?;
    cards.sort();
    Ok(cards
        .into_iter()
        .enumerate()
        .map(|(index, (opaque, object))| AuthoritativeCandidate {
            candidate_id: mtgml_model::CandidateIdV1(index as u32),
            visible_intent: CandidateIntent::SelectObject { object: opaque },
            trusted_binding: EngineCandidateBinding::SelectObject { object },
        })
        .collect())
}

/// CR 514.1: the active player chooses the card to discard.
fn install_discard_request(next: &mut EngineState) -> Result<AuthoritativeDecisionRequest, Error> {
    let candidates = discard_candidates(next)?;
    install_actor_only_request(
        next,
        DecisionPurposeV4::HandSizeDiscard,
        DecisionDomainV2::ChooseMany {
            minimum: 1,
            maximum: 1,
        },
        candidates,
    )
}

fn validate_discard_request(
    admission: &ExecutableProfileAdmissionV1,
    state: &EngineState,
    request: &AuthoritativeDecisionRequest,
    status: &EpisodeStatus,
) -> Result<(), BasicLandCandidateError> {
    state
        .validate_structure()
        .map_err(|_| BasicLandCandidateError::InvalidState)?;
    validate_slice(admission, state).map_err(|_| BasicLandCandidateError::InvalidState)?;
    // Only an admission with the rule that creates this request accepts it.
    admits(admission, "rules/cleanup-reset")
        .map_err(|_| BasicLandCandidateError::PendingCandidateSetMismatch)?;
    let parts = state;
    let hand = parts
        .zones
        .locations
        .values()
        .filter(|location| {
            location.zone == ZoneKind::Hand && location.player == Some(parts.core.active_player)
        })
        .count();
    let expected = discard_candidates(state).map_err(|_| BasicLandCandidateError::InvalidState)?;
    if !matches!(status, EpisodeStatus::Running)
        || parts.core.position
            != (TurnPosition::Ending {
                step: EndingStep::Cleanup,
            })
        || parts.core.priority != PriorityState::None
        || hand != crate::turn_structure::ORDINARY_MAXIMUM_HAND_SIZE + 1
        || request.decision_domain_v2
            != (DecisionDomainV2::ChooseMany {
                minimum: 1,
                maximum: 1,
            })
        || request.candidates != expected
        || !actor_only_request_matches(state, request)
    {
        return Err(BasicLandCandidateError::PendingCandidateSetMismatch);
    }
    Ok(())
}

/// The request is the latest one: the identities, revision and view sequence
/// the installer allocates, and a projection a player can receive.
fn request_is_current(state: &EngineState, request: &AuthoritativeDecisionRequest) -> bool {
    let Some(knowledge) = state.knowledge.players.get(&request.actor) else {
        return false;
    };
    let Some(identity) = state.perspective_identities.players.get(&request.actor) else {
        return false;
    };
    request.decision_id.0.checked_add(1) == Some(state.allocators.next_decision_id.0)
        && request.player_decision_id.0.checked_add(1) == Some(identity.next_player_decision_id.0)
        && request.state_revision == state.revision
        && request.view_sequence == knowledge.next_visible_sequence
        && request.parent_player_decision_id.is_none()
        && request.project_player_request().is_ok()
}

/// Identity, sequence and visibility fields every actor-only request shares.
fn actor_only_request_matches(state: &EngineState, request: &AuthoritativeDecisionRequest) -> bool {
    request.actor == state.core.active_player
        && request_is_current(state, request)
        && request.visibility == DecisionVisibility::ActingPlayerOnly
        && request.continuation_id.is_none()
}

/// CR 601.2h: a restored or committed payment request is exactly the one the
/// pending cast calls for, with the identities the installer allocates.
fn validate_payment_request(
    admission: &ExecutableProfileAdmissionV1,
    state: &EngineState,
    request: &AuthoritativeDecisionRequest,
    status: &EpisodeStatus,
) -> Result<(), BasicLandCandidateError> {
    state
        .validate_structure()
        .map_err(|_| BasicLandCandidateError::InvalidState)?;
    validate_slice(admission, state).map_err(|_| BasicLandCandidateError::InvalidState)?;
    // Only an admission with the rule that creates this request accepts it.
    admits(admission, "rules/cast-creature-spell")
        .map_err(|_| BasicLandCandidateError::PendingCandidateSetMismatch)?;
    let mismatch = BasicLandCandidateError::PendingCandidateSetMismatch;
    let expected = crate::casting::payment_request_shape(admission, state).map_err(|_| mismatch)?;
    let shape = RequestShape {
        actor: request.actor,
        visibility: request.visibility,
        continuation_id: request.continuation_id,
        purpose: request.purpose.clone(),
        decision_domain_v2: request.decision_domain_v2.clone(),
        candidates: request.candidates.clone(),
    };
    if !matches!(status, EpisodeStatus::Running)
        || shape != expected
        || !request_is_current(state, request)
    {
        return Err(mismatch);
    }
    Ok(())
}

/// CR 508.1a: the active player declares any subset of the creatures that can
/// attack (none, when none can).
fn install_attacker_request(
    admission: &ExecutableProfileAdmissionV1,
    next: &mut EngineState,
) -> Result<AuthoritativeDecisionRequest, Error> {
    let candidates = crate::combat::attacker_candidates(admission, next)?;
    let maximum = u32::try_from(candidates.len()).map_err(|_| Error::InvalidResult)?;
    install_actor_only_request(
        next,
        DecisionPurposeV4::AttackerDeclaration,
        DecisionDomainV2::ChooseMany {
            minimum: 0,
            maximum,
        },
        candidates,
    )
}

/// CR 508.1a: a restored or committed attacker declaration is exactly the one
/// the state calls for: every creature that can attack, and any subset of them.
fn validate_attacker_request(
    admission: &ExecutableProfileAdmissionV1,
    state: &EngineState,
    request: &AuthoritativeDecisionRequest,
    status: &EpisodeStatus,
) -> Result<(), BasicLandCandidateError> {
    state
        .validate_structure()
        .map_err(|_| BasicLandCandidateError::InvalidState)?;
    validate_slice(admission, state).map_err(|_| BasicLandCandidateError::InvalidState)?;
    // Only an admission with the rule that creates this request accepts it.
    admits(admission, "rules/declare-attackers")
        .map_err(|_| BasicLandCandidateError::PendingCandidateSetMismatch)?;
    let expected = crate::combat::attacker_candidates(admission, state)
        .map_err(|_| BasicLandCandidateError::InvalidState)?;
    let maximum =
        u32::try_from(expected.len()).map_err(|_| BasicLandCandidateError::InvalidState)?;
    let parts = state;
    if !matches!(status, EpisodeStatus::Running)
        || parts.core.position
            != (TurnPosition::Combat {
                step: CombatStep::DeclareAttackers,
            })
        || parts.combat.is_some()
        || parts.core.priority != PriorityState::None
        || request.decision_domain_v2
            != (DecisionDomainV2::ChooseMany {
                minimum: 0,
                maximum,
            })
        || request.candidates != expected
        || !actor_only_request_matches(state, request)
    {
        return Err(BasicLandCandidateError::PendingCandidateSetMismatch);
    }
    Ok(())
}

/// CR 509.1a: a restored or committed block request is exactly the one the
/// pending declaration calls for: for the defending player, who is not the
/// active player, about the next creature, with the identities the installer
/// allocates.
fn validate_block_request(
    admission: &ExecutableProfileAdmissionV1,
    state: &EngineState,
    request: &AuthoritativeDecisionRequest,
    status: &EpisodeStatus,
) -> Result<(), BasicLandCandidateError> {
    state
        .validate_structure()
        .map_err(|_| BasicLandCandidateError::InvalidState)?;
    validate_slice(admission, state).map_err(|_| BasicLandCandidateError::InvalidState)?;
    // Only an admission with the rule that creates this request accepts it.
    admits(admission, "rules/declare-blockers")
        .map_err(|_| BasicLandCandidateError::PendingCandidateSetMismatch)?;
    let mismatch = BasicLandCandidateError::PendingCandidateSetMismatch;
    // `validate_slice` has checked the continuation against the battlefield.
    let expected = crate::combat::block_request_shape(state).map_err(|_| mismatch)?;
    let shape = RequestShape {
        actor: request.actor,
        visibility: request.visibility,
        continuation_id: request.continuation_id,
        purpose: request.purpose.clone(),
        decision_domain_v2: request.decision_domain_v2.clone(),
        candidates: request.candidates.clone(),
    };
    if !matches!(status, EpisodeStatus::Running)
        || shape != expected
        || !request_is_current(state, request)
    {
        return Err(mismatch);
    }
    Ok(())
}

/// Allocates the next decision identity (D5) for an active-player request.
fn install_actor_only_request(
    next: &mut EngineState,
    purpose: DecisionPurposeV4,
    decision_domain_v2: DecisionDomainV2,
    candidates: Vec<AuthoritativeCandidate>,
) -> Result<AuthoritativeDecisionRequest, Error> {
    let actor = next.core.active_player;
    install_request(
        next,
        RequestShape {
            actor,
            visibility: DecisionVisibility::ActingPlayerOnly,
            continuation_id: None,
            purpose,
            decision_domain_v2,
            candidates,
        },
    )
}

/// Everything about a request except the identities it is allocated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RequestShape {
    pub(crate) actor: PlayerId,
    pub(crate) visibility: DecisionVisibility,
    pub(crate) continuation_id: Option<mtgml_model::ContinuationId>,
    pub(crate) purpose: DecisionPurposeV4,
    pub(crate) decision_domain_v2: DecisionDomainV2,
    pub(crate) candidates: Vec<AuthoritativeCandidate>,
}

/// Allocates the next decision identity (D5) and installs the request.
pub(crate) fn install_request(
    next: &mut EngineState,
    shape: RequestShape,
) -> Result<AuthoritativeDecisionRequest, Error> {
    let RequestShape {
        actor,
        visibility,
        continuation_id,
        purpose,
        decision_domain_v2,
        candidates,
    } = shape;
    let parts = &mut *next;
    let view_sequence = parts
        .knowledge
        .players
        .get(&actor)
        .ok_or(Error::InvalidResult)?
        .next_visible_sequence;
    let identity = parts
        .perspective_identities
        .players
        .get_mut(&actor)
        .ok_or(Error::InvalidResult)?;
    let decision_id = parts.allocators.next_decision_id;
    let player_decision_id = identity.next_player_decision_id;
    parts.allocators.next_decision_id = DecisionId(
        decision_id
            .0
            .checked_add(1)
            .ok_or(Error::IdentityExhausted)?,
    );
    identity.next_player_decision_id = PlayerDecisionIdV1(
        player_decision_id
            .0
            .checked_add(1)
            .ok_or(Error::IdentityExhausted)?,
    );
    let request = AuthoritativeDecisionRequest {
        decision_id,
        player_decision_id,
        state_revision: parts.revision,
        view_sequence,
        actor,
        visibility,
        decision_domain_v2,
        purpose,
        parent_player_decision_id: None,
        continuation_id,
        candidates,
    };
    // A zero-candidate attacker declaration must be representable too.
    request
        .project_player_request()
        .map_err(|_| Error::TurnProgressUnsupported)?;
    next.execution.pending_decision = Some(request.clone());
    Ok(request)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mtgml_card_ir::ExecutableProfileAdmissionV1;
    use mtgml_decision::{
        AuthoritativeDecisionRequest, DecisionAnswerV2, DecisionDomainV2, DecisionPurposeV4,
        DecisionResponseV3, EngineCandidateBinding, DECISION_RESPONSE_V3_SCHEMA,
    };
    use mtgml_model::{EpisodeStatus, GameObjectId, PhysicalCardId, PlayerId, ZoneKind};
    use mtgml_state::{
        BeginningStep, CombatStep, EndingStep, EngineState, GameObject, PriorityState,
        TurnPosition, VisibilityPartition, ZoneLocation, ZonePosition,
    };

    pub(super) const P1: PlayerId = PlayerId(1);
    pub(super) const P2: PlayerId = PlayerId(2);

    /// The synthetic reset puts a face-down (morph-like) card on top of P2's
    /// library; the draw profile admits only ordinary face-up cards.
    pub(super) fn make_synthetic_library_card_ordinary(state: &mut EngineState) {
        state
            .zones
            .objects
            .get_mut(&GameObjectId(2))
            .unwrap()
            .face_down = false;
    }

    pub(super) fn add_library_cards(state: &mut EngineState, owner: PlayerId, count: u64) {
        let definition = crate::basic_land::s1_b_state_with_two_lands_fixture()
            .zones
            .objects
            .values()
            .next()
            .unwrap()
            .card_definition;
        for _ in 0..count {
            let id = state.allocators.next_object_id;
            state.allocators.next_object_id = GameObjectId(id.0 + 1);
            let base = ZoneLocation {
                zone: ZoneKind::Library,
                player: Some(owner),
                position: ZonePosition::Top { offset: 0 },
                visibility: VisibilityPartition::FaceDown,
                partition: None,
            };
            let order = state.zones.ordered_zones.entry(base.key()).or_default();
            let location = ZoneLocation {
                position: ZonePosition::Top {
                    offset: order.len() as u32,
                },
                ..base
            };
            order.push(id);
            state.zones.objects.insert(
                id,
                GameObject {
                    id,
                    physical_card: Some(PhysicalCardId(1_000 + id.0)),
                    card_definition: definition,
                    owner,
                    controller: owner,
                    tapped: false,
                    face_down: false,
                },
            );
            state.zones.locations.insert(id, location);
            state.card_rules.faces.faces.insert(id, 0);
        }
    }

    /// Adds cards to `owner`'s hand that only the owner tracks, with the
    /// owner's knowledge record, as a real game's opening hand has.
    pub(super) fn add_hand_cards(state: &mut EngineState, owner: PlayerId, count: u64) {
        let definition = state.zones.objects[&GameObjectId(2)].card_definition;
        for _ in 0..count {
            let parts = &mut *state;
            let id = parts.allocators.next_object_id;
            parts.allocators.next_object_id = GameObjectId(id.0 + 1);
            let location = ZoneLocation {
                zone: ZoneKind::Hand,
                player: Some(owner),
                position: ZonePosition::Unordered,
                visibility: VisibilityPartition::OwnerOnly,
                partition: None,
            };
            let physical_card = Some(PhysicalCardId(2_000 + id.0));
            parts.zones.objects.insert(
                id,
                GameObject {
                    id,
                    physical_card,
                    card_definition: definition,
                    owner,
                    controller: owner,
                    tapped: false,
                    face_down: false,
                },
            );
            parts.zones.locations.insert(id, location.clone());
            let identity = parts
                .perspective_identities
                .players
                .get_mut(&owner)
                .unwrap();
            let opaque = identity.next_opaque_object_id;
            identity.next_opaque_object_id = mtgml_model::OpaqueObjectId(opaque.0 + 1);
            identity.object_to_opaque.insert(id, opaque);
            identity.opaque_to_object.insert(opaque, id);
            parts
                .knowledge
                .players
                .get_mut(&owner)
                .unwrap()
                .active
                .insert(
                    opaque,
                    mtgml_state::KnowledgeRecordV2 {
                        opaque_object: opaque,
                        physical_card,
                        card_definition: Some(definition),
                        known_location: Some(mtgml_state::KnownLocationFactV2 {
                            location,
                            provenance:
                                mtgml_state::KnowledgeAcquisitionReason::InitialConfiguration,
                        }),
                        acquisition: mtgml_state::KnowledgeAcquisitionReason::InitialConfiguration,
                        historical_locations: Vec::new(),
                    },
                );
            state.card_rules.faces.faces.insert(id, 0);
        }
    }

    /// Two players, P1 active with priority in precombat main, Mountain and
    /// Plains in P1's hand, one untapped Mountain with its mana ability on
    /// P1's battlefield and `library` face-down cards in each library.
    fn game_with(
        admission: ExecutableProfileAdmissionV1,
        library: u64,
    ) -> (ExecutableProfileAdmissionV1, EngineState) {
        game_with_hands(admission, library, 0, 0)
    }

    /// As `game_with`, with extra owner-tracked cards in each hand.
    fn game_with_hands(
        admission: ExecutableProfileAdmissionV1,
        library: u64,
        p1_extra: u64,
        p2_extra: u64,
    ) -> (ExecutableProfileAdmissionV1, EngineState) {
        let v2 = crate::basic_land::s1_b_state_with_two_lands_fixture();
        let mut state = v2;
        make_synthetic_library_card_ordinary(&mut state);
        add_library_cards(&mut state, P1, library);
        add_library_cards(&mut state, P2, library);
        add_hand_cards(&mut state, P1, p1_extra);
        add_hand_cards(&mut state, P2, p2_extra);
        crate::install_basic_land_request(&admission, &mut state, P1, &EpisodeStatus::Running)
            .unwrap();
        (admission, state)
    }

    fn game(library: u64) -> (ExecutableProfileAdmissionV1, EngineState) {
        game_with(crate::basic_land::basic_land_admission_fixture(), library)
    }

    /// As `game`, under the admission with vanilla creatures, with a
    /// creature on `controller`'s battlefield.
    fn game_with_creature(
        library: u64,
        controller: PlayerId,
    ) -> (ExecutableProfileAdmissionV1, EngineState) {
        game_with_creatures(library, &[controller])
    }

    /// As `game_with_creature`, with one creature for each of `controllers`.
    fn game_with_creatures(
        library: u64,
        controllers: &[PlayerId],
    ) -> (ExecutableProfileAdmissionV1, EngineState) {
        let admission = crate::basic_land::vanilla_creature_admission_fixture();
        let mut state = crate::basic_land::s1_b_state_with_two_lands_fixture();
        make_synthetic_library_card_ordinary(&mut state);
        add_library_cards(&mut state, P1, library);
        add_library_cards(&mut state, P2, library);
        for controller in controllers {
            crate::basic_land::put_vanilla_creature_on_battlefield(&mut state, *controller);
        }
        crate::install_basic_land_request(&admission, &mut state, P1, &EpisodeStatus::Running)
            .unwrap();
        (admission, state)
    }

    fn at_attacker_declaration(state: &EngineState) -> bool {
        state
            .execution
            .pending_decision
            .as_ref()
            .is_some_and(|request| request.purpose == DecisionPurposeV4::AttackerDeclaration)
    }

    pub(super) fn pending(state: &EngineState) -> &AuthoritativeDecisionRequest {
        state.execution.pending_decision.as_ref().unwrap()
    }

    fn candidate(
        request: &AuthoritativeDecisionRequest,
        pick: impl Fn(&EngineCandidateBinding) -> bool,
    ) -> Option<mtgml_model::CandidateIdV1> {
        request
            .candidates
            .iter()
            .find(|candidate| pick(&candidate.trusted_binding))
            .map(|candidate| candidate.candidate_id)
    }

    pub(super) fn submit(
        admission: &ExecutableProfileAdmissionV1,
        state: &EngineState,
        answer: DecisionAnswerV2,
    ) -> Result<crate::BasicLandTransitionProduct, crate::BasicLandTransitionError> {
        let request = pending(state);
        let response = DecisionResponseV3 {
            schema_version: DECISION_RESPONSE_V3_SCHEMA.to_owned(),
            player_decision_id: request.player_decision_id,
            view_sequence: request.view_sequence,
            answer,
        };
        execute_magic_response(
            admission,
            state,
            request.actor,
            &response,
            &EpisodeStatus::Running,
        )
    }

    /// Checks a product is a complete, valid V3 step and returns its state.
    pub(super) fn apply(
        before: &EngineState,
        product: &crate::BasicLandTransitionProduct,
    ) -> EngineState {
        assert!(product.accepted);
        assert_eq!(product.next_state.revision.0, before.revision.0 + 1);
        crate::events::validate_events_for_built_delta_v3(
            before,
            &product.next_state,
            &product.events,
            &product.delta,
        )
        .unwrap();
        assert_eq!(
            product.delta.apply_structural_only(before).unwrap(),
            product.next_state
        );
        assert_eq!(
            product.next_state.execution.pending_decision,
            product.next_decision
        );
        product.next_state.clone()
    }

    pub(super) fn pass_answer(request: &AuthoritativeDecisionRequest) -> DecisionAnswerV2 {
        if request.purpose == DecisionPurposeV4::AttackerDeclaration {
            DecisionAnswerV2::SelectMany {
                candidate_ids: Vec::new(),
            }
        } else if request.purpose == DecisionPurposeV4::HandSizeDiscard {
            DecisionAnswerV2::SelectMany {
                candidate_ids: vec![request.candidates.last().unwrap().candidate_id],
            }
        } else {
            DecisionAnswerV2::SelectOne {
                candidate_id: candidate(request, |binding| {
                    matches!(binding, EngineCandidateBinding::PassPriority)
                })
                .unwrap(),
            }
        }
    }

    pub(super) fn pass(
        admission: &ExecutableProfileAdmissionV1,
        state: &EngineState,
    ) -> (EngineState, crate::BasicLandTransitionProduct) {
        let product = submit(admission, state, pass_answer(pending(state))).unwrap();
        (apply(state, &product), product)
    }

    fn take_action(
        admission: &ExecutableProfileAdmissionV1,
        state: &EngineState,
        pick: impl Fn(&EngineCandidateBinding) -> bool,
    ) -> EngineState {
        let id = candidate(pending(state), pick).unwrap();
        let product = submit(
            admission,
            state,
            DecisionAnswerV2::SelectOne { candidate_id: id },
        )
        .unwrap();
        apply(state, &product)
    }

    fn play_first_land(
        admission: &ExecutableProfileAdmissionV1,
        state: &EngineState,
    ) -> EngineState {
        take_action(admission, state, |binding| {
            matches!(binding, EngineCandidateBinding::PlayLand { .. })
        })
    }

    fn tap_first_mana_source(
        admission: &ExecutableProfileAdmissionV1,
        state: &EngineState,
    ) -> EngineState {
        take_action(admission, state, |binding| {
            matches!(binding, EngineCandidateBinding::ActivateAbility { .. })
        })
    }

    /// Passes (and declares no attackers) until `until` holds.
    pub(super) fn pass_until(
        admission: &ExecutableProfileAdmissionV1,
        mut state: EngineState,
        until: impl Fn(&EngineState) -> bool,
    ) -> EngineState {
        for _ in 0..200 {
            if until(&state) {
                return state;
            }
            state = pass(admission, &state).0;
        }
        panic!("condition not reached within 200 responses");
    }

    pub(super) fn at(position: TurnPosition, turn: u64) -> impl Fn(&EngineState) -> bool {
        move |state| {
            state.core.position == position
                && state.core.turn_number == turn
                && state
                    .execution
                    .pending_decision
                    .as_ref()
                    .is_some_and(|request| {
                        request.actor == state.core.active_player
                            && request.purpose == DecisionPurposeV4::PriorityAction
                    })
        }
    }

    pub(super) const UPKEEP: TurnPosition = TurnPosition::Beginning {
        step: BeginningStep::Upkeep,
    };
    const DRAW: TurnPosition = TurnPosition::Beginning {
        step: BeginningStep::Draw,
    };
    const BEGIN_COMBAT: TurnPosition = TurnPosition::Combat {
        step: CombatStep::BeginningOfCombat,
    };
    pub(super) const END_STEP: TurnPosition = TurnPosition::Ending {
        step: EndingStep::EndStep,
    };

    pub(super) fn zone_count(state: &EngineState, owner: PlayerId, zone: ZoneKind) -> usize {
        state
            .zones
            .locations
            .values()
            .filter(|location| location.zone == zone && location.player == Some(owner))
            .count()
    }

    fn battlefield_tapped(state: &EngineState, controller: PlayerId) -> Vec<bool> {
        state
            .zones
            .objects
            .values()
            .filter(|object| {
                object.controller == controller
                    && state.zones.locations[&object.id].zone == ZoneKind::Battlefield
            })
            .map(|object| object.tapped)
            .collect()
    }

    fn mana_source(state: &EngineState) -> GameObjectId {
        state
            .card_rules
            .abilities
            .by_instance
            .values()
            .next()
            .unwrap()
            .source
    }

    fn has_play_land(state: &EngineState) -> bool {
        candidate(pending(state), |binding| {
            matches!(binding, EngineCandidateBinding::PlayLand { .. })
        })
        .is_some()
    }

    #[test]
    fn first_pass_only_transfers_priority() {
        let (admission, state) = game(3);
        let after = pass(&admission, &state).0;

        assert_eq!(after.core.position, TurnPosition::PrecombatMain);
        let request = pending(&after);
        assert_eq!(request.actor, P2);
        assert_eq!(request.purpose, DecisionPurposeV4::PriorityAction);
        assert!(!has_play_land(&after));
    }

    /// One transition in which the active player draws `count` cards, built
    /// the way `progress` builds every transition.
    fn draw_in_one_transition(
        admission: &ExecutableProfileAdmissionV1,
        state: &EngineState,
        count: usize,
    ) -> Result<crate::BasicLandTransitionProduct, Error> {
        let request = pending(state).clone();
        let mut next = state.clone();
        next.execution.pending_decision = None;
        next.revision = StateRevision(state.revision.0 + 1);
        let active = next.core.active_player;
        let mut facts = Facts::default();
        for _ in 0..count {
            draw(&mut next, active, &mut facts)?;
        }
        finish(
            admission,
            state,
            &request,
            next,
            facts,
            NextDecision::Priority(active),
        )
    }

    #[test]
    fn two_draws_in_one_transition_validate() {
        let (admission, state) = game(3);
        let state = pass_until(&admission, state, at(UPKEEP, 2));
        let hand_before = zone_count(&state, P2, ZoneKind::Hand);
        let product = draw_in_one_transition(&admission, &state, 2).unwrap();
        let after = apply(&state, &product);
        assert_eq!(zone_count(&after, P2, ZoneKind::Hand), hand_before + 2);
    }

    #[test]
    fn a_stale_second_draw_location_is_rejected() {
        let (admission, state) = game(3);
        let state = pass_until(&admission, state, at(UPKEEP, 2));
        let mut product = draw_in_one_transition(&admission, &state, 2).unwrap();
        // The second draw claims the card was where it lay before the first
        // draw moved it up: true of `before`, false when the draw happened.
        let second = product
            .events
            .iter()
            .filter_map(|event| match &event.event {
                AuthoritativeRuleEventKind::ZoneTransition { transition } => {
                    Some(transition.old_object)
                }
                _ => None,
            })
            .nth(1)
            .unwrap();
        let stale = state.zones.locations[&second].clone();
        assert_ne!(stale.position, ZonePosition::Top { offset: 0 });
        for event in &mut product.events {
            if let AuthoritativeRuleEventKind::ZoneTransition { transition } = &mut event.event {
                if transition.old_object == second {
                    transition.last_known.location = stale.clone();
                    transition.from = stale.clone();
                }
            }
        }
        for operation in &mut product.delta.operations {
            if let SemanticDeltaOperation::ZoneTransition { transition } = operation {
                if transition.old_object == second {
                    transition.last_known.location = stale.clone();
                    transition.from = stale.clone();
                }
            }
        }
        assert!(crate::events::validate_events_for_built_delta_v3(
            &state,
            &product.next_state,
            &product.events,
            &product.delta,
        )
        .is_err());
    }

    #[test]
    fn built_delta_check_rejects_another_after_state() {
        let (admission, state) = game(3);
        let product = submit(&admission, &state, pass_answer(pending(&state))).unwrap();
        crate::events::validate_events_for_built_delta_v3(
            &state,
            &product.next_state,
            &product.events,
            &product.delta,
        )
        .unwrap();
        let mut other = product.next_state.clone();
        other.core.turn_number += 1;
        assert!(crate::events::validate_events_for_built_delta_v3(
            &state,
            &other,
            &product.events,
            &product.delta
        )
        .is_err());
    }

    #[test]
    fn main_passes_open_beginning_of_combat_priority() {
        let (admission, state) = game(3);
        let state = pass(&admission, &state).0;
        let after = pass(&admission, &state).0;

        assert_eq!(after.core.position, BEGIN_COMBAT);
        assert_eq!(pending(&after).actor, P1);
        assert_eq!(pending(&after).purpose, DecisionPurposeV4::PriorityAction);
    }

    #[test]
    fn mana_pools_empty_when_the_step_changes() {
        let (admission, state) = game(3);
        let state = tap_first_mana_source(&admission, &state);
        assert_ne!(state.card_rules.mana.pools[&P1], Default::default());
        let state = pass(&admission, &state).0;
        let (after, product) = pass(&admission, &state);

        assert_eq!(after.core.position, BEGIN_COMBAT);
        assert_eq!(after.card_rules.mana.pools[&P1], Default::default());
        assert!(product.events.iter().any(|event| matches!(
            event.event,
            crate::AuthoritativeRuleEventKind::ManaPoolChanged {
                player: P1,
                cause: mtgml_state::ManaPoolChangeCauseV1::Emptied,
                ..
            }
        )));
    }

    #[test]
    fn acting_after_a_pass_restarts_the_pass_succession() {
        // CR 117.3c, 117.4: a player who acts keeps priority, and only
        // passes in succession with no action between them end the step.
        let held = |player| PriorityState::HeldBy {
            player,
            consecutive_passes: 0,
        };
        let (admission, state) = game(3);
        let state = pass_until(&admission, state, at(TurnPosition::PrecombatMain, 2));
        let state = pass(&admission, &state).0;
        assert_eq!(pending(&state).actor, P1);

        let state = tap_first_mana_source(&admission, &state);
        assert_eq!(state.core.priority, held(P1));
        let state = pass(&admission, &state).0;
        assert_eq!(state.core.position, TurnPosition::PrecombatMain);
        assert_eq!(pending(&state).actor, P2);

        let state = play_first_land(&admission, &state);
        assert_eq!(state.core.priority, held(P2));
        let state = pass(&admission, &state).0;
        assert_eq!(pending(&state).actor, P1);
        let after = pass(&admission, &state).0;
        assert_eq!(after.core.position, BEGIN_COMBAT);
        assert_eq!(pending(&after).actor, P2);
    }

    #[test]
    fn beginning_of_combat_passes_reach_empty_attacker_declaration() {
        let (admission, state) = game(3);
        let state = pass_until(&admission, state, at(BEGIN_COMBAT, 1));
        let state = pass(&admission, &state).0;
        let after = pass(&admission, &state).0;

        let request = pending(&after);
        assert_eq!(request.purpose, DecisionPurposeV4::AttackerDeclaration);
        assert_eq!(request.actor, P1);
        assert_eq!(
            request.decision_domain_v2,
            DecisionDomainV2::ChooseMany {
                minimum: 0,
                maximum: 0
            }
        );
        assert!(request.candidates.is_empty());
    }

    #[test]
    fn a_creature_on_the_battlefield_does_not_stop_the_turn_before_combat() {
        // Creatures may be on the battlefield; only attacking with them is
        // missing. Every step up to the beginning of combat still runs.
        let (admission, state) = game_with_creature(3, P1);
        let state = pass_until(&admission, state, at(BEGIN_COMBAT, 1));
        assert_eq!(zone_count(&state, P1, ZoneKind::Battlefield), 2);
    }

    /// The battlefield creatures of the fixtures (Savannah Lions), in object order.
    fn battlefield_creatures(state: &EngineState) -> Vec<GameObjectId> {
        state
            .zones
            .objects
            .values()
            .filter(|object| {
                object.card_definition == mtgml_model::CardDefinitionId(3)
                    && state.zones.locations[&object.id].zone == ZoneKind::Battlefield
            })
            .map(|object| object.id)
            .collect()
    }

    /// The attacker declaration of `turn`, before it is answered.
    fn at_attackers(turn: u64) -> impl Fn(&EngineState) -> bool {
        move |state| at_attacker_declaration(state) && state.core.turn_number == turn
    }

    /// The objects the attacker declaration offers, in the order offered.
    fn offered_attackers(state: &EngineState) -> Vec<GameObjectId> {
        pending(state)
            .candidates
            .iter()
            .map(|candidate| match candidate.trusted_binding {
                EngineCandidateBinding::SelectObject { object } => object,
                ref other => panic!("{other:?}"),
            })
            .collect()
    }

    /// Declares the first `count` offered attackers.
    fn declare(
        admission: &ExecutableProfileAdmissionV1,
        state: &EngineState,
        count: usize,
    ) -> Result<crate::BasicLandTransitionProduct, crate::BasicLandTransitionError> {
        let candidate_ids = pending(state)
            .candidates
            .iter()
            .take(count)
            .map(|candidate| candidate.candidate_id)
            .collect();
        submit(
            admission,
            state,
            DecisionAnswerV2::SelectMany { candidate_ids },
        )
    }

    #[test]
    fn a_creature_cannot_attack_the_turn_it_arrives() {
        // CR 302.6: the creature came under P1's control on turn 1.
        let (admission, state) = game_with_creature(3, P1);
        let state = pass_until(&admission, state, at_attackers(1));
        let request = pending(&state);
        assert_eq!(
            request.decision_domain_v2,
            DecisionDomainV2::ChooseMany {
                minimum: 0,
                maximum: 0
            }
        );
        assert!(request.candidates.is_empty());
        validate_magic_pending_request(&admission, &state, &EpisodeStatus::Running).unwrap();
        // The only answer is no attackers.
        assert_eq!(
            submit(
                &admission,
                &state,
                DecisionAnswerV2::SelectMany {
                    candidate_ids: vec![mtgml_model::CandidateIdV1(0)]
                }
            ),
            Err(crate::BasicLandTransitionError::InvalidSelection)
        );
        let after = declare(&admission, &state, 0).unwrap().next_state;
        assert!(after.combat.as_ref().unwrap().attackers.is_empty());
    }

    #[test]
    fn a_creature_can_attack_from_its_controllers_next_turn() {
        let (admission, state) = game_with_creature(3, P1);
        let creature = battlefield_creatures(&state)[0];
        let state = pass_until(&admission, state, at_attackers(3));
        let request = pending(&state);
        assert_eq!(request.actor, P1);
        assert_eq!(
            request.decision_domain_v2,
            DecisionDomainV2::ChooseMany {
                minimum: 0,
                maximum: 1
            }
        );
        assert_eq!(offered_attackers(&state), vec![creature]);
        let opaque = state.perspective_identities.players[&P1].object_to_opaque[&creature];
        assert_eq!(
            request.candidates[0].visible_intent,
            mtgml_decision::CandidateIntent::SelectObject { object: opaque }
        );
        validate_magic_pending_request(&admission, &state, &EpisodeStatus::Running).unwrap();
    }

    #[test]
    fn a_tapped_creature_is_not_offered_as_an_attacker() {
        // CR 508.1a: the chosen creatures must be untapped.
        let (admission, state) = game_with_creature(3, P1);
        let creature = battlefield_creatures(&state)[0];
        let mut state = pass_until(&admission, state, at(BEGIN_COMBAT, 3));
        state.zones.objects.get_mut(&creature).unwrap().tapped = true;
        let state = pass(&admission, &state).0;
        let state = pass(&admission, &state).0;
        assert!(at_attacker_declaration(&state));
        assert!(pending(&state).candidates.is_empty());
    }

    #[test]
    fn a_creature_is_offered_only_to_its_controller_on_their_own_turn() {
        // P2's creature came under its control on turn 1: it cannot attack on
        // P1's turn 1, and is not P1's to attack with.
        let (admission, state) = game_with_creature(3, P2);
        let creature = battlefield_creatures(&state)[0];
        let state = pass_until(&admission, state, at_attackers(1));
        assert!(pending(&state).candidates.is_empty());
        validate_magic_pending_request(&admission, &state, &EpisodeStatus::Running).unwrap();
        let after = declare(&admission, &state, 0).unwrap().next_state;
        assert!(after.combat.as_ref().unwrap().attackers.is_empty());

        // On P2's turn 2 it has been under P2's control since the turn began.
        let state = pass_until(&admission, after, at_attackers(2));
        assert_eq!(pending(&state).actor, P2);
        assert_eq!(offered_attackers(&state), vec![creature]);
    }

    #[test]
    fn a_wrong_candidate_set_for_the_declaration_is_refused() {
        let (admission, state) = game_with_creature(3, P1);
        let state = pass_until(&admission, state, at_attackers(3));
        let status = EpisodeStatus::Running;
        validate_magic_pending_request(&admission, &state, &status).unwrap();
        // Without its candidate, or with a wrong domain, the request is not
        // the one the state calls for.
        let mut without = state.clone();
        let request = without.execution.pending_decision.as_mut().unwrap();
        request.candidates.clear();
        request.decision_domain_v2 = DecisionDomainV2::ChooseMany {
            minimum: 0,
            maximum: 0,
        };
        assert!(validate_magic_pending_request(&admission, &without, &status).is_err());
        let mut wide = state.clone();
        wide.execution
            .pending_decision
            .as_mut()
            .unwrap()
            .decision_domain_v2 = DecisionDomainV2::ChooseMany {
            minimum: 0,
            maximum: 2,
        };
        assert!(validate_magic_pending_request(&admission, &wide, &status).is_err());
        // The creature tapped since the request was made: it is no attacker.
        let mut tapped = state.clone();
        let creature = offered_attackers(&state)[0];
        tapped.zones.objects.get_mut(&creature).unwrap().tapped = true;
        assert!(validate_magic_pending_request(&admission, &tapped, &status).is_err());
    }

    #[test]
    fn declaring_an_attacker_taps_it_and_makes_the_attack_public() {
        let (admission, state) = game_with_creature(3, P1);
        let creature = battlefield_creatures(&state)[0];
        let state = pass_until(&admission, state, at_attackers(3));
        let product = declare(&admission, &state, 1).unwrap();
        let after = apply(&state, &product);

        // CR 508.1f: the attacker taps.
        assert!(after.zones.objects[&creature].tapped);
        // CR 508.1k: it is an attacking creature, unblocked so far.
        let combat = after.combat.as_ref().unwrap();
        assert_eq!(combat.defending_player, P2);
        assert_eq!(combat.attackers, vec![creature]);
        assert!(combat.blockers.is_empty());
        assert!(combat.blocked_attackers.is_empty() && !combat.damage_step_completed);
        // CR 508.2: the active player receives priority.
        assert_eq!(pending(&after).actor, P1);
        assert_eq!(pending(&after).purpose, DecisionPurposeV4::PriorityAction);
        // The tap, then the declaration, are public: each is followed by one
        // observation occurrence per player.
        let kinds: Vec<_> = product
            .events
            .iter()
            .filter_map(|event| match &event.event {
                AuthoritativeRuleEventKind::ObjectTapped { object, from, to } => {
                    Some(format!("tap {} {from} {to}", object.0))
                }
                AuthoritativeRuleEventKind::AttackersDeclared {
                    defending_player,
                    attackers,
                } => Some(format!("declared {} {attackers:?}", defending_player.0)),
                _ => None,
            })
            .collect();
        assert_eq!(
            kinds,
            vec![
                format!("tap {} false true", creature.0),
                format!("declared 2 {:?}", vec![creature]),
            ]
        );
        let occurrences = product
            .events
            .iter()
            .filter(|event| {
                matches!(
                    event.event,
                    AuthoritativeRuleEventKind::PerspectiveObservationOccurrence { .. }
                )
            })
            .count();
        assert!(occurrences >= 4, "{occurrences}");
        validate_magic_pending_request(&admission, &after, &EpisodeStatus::Running).unwrap();
    }

    #[test]
    fn an_empty_declaration_is_public_too() {
        let (admission, state) = game(3);
        let state = pass_until(&admission, state, at_attacker_declaration);
        let product = declare(&admission, &state, 0).unwrap();
        apply(&state, &product);
        assert!(product.events.iter().any(|event| matches!(
            &event.event,
            AuthoritativeRuleEventKind::AttackersDeclared { attackers, .. } if attackers.is_empty()
        )));
        let sources: Vec<_> = product
            .events
            .iter()
            .filter_map(|event| match &event.event {
                AuthoritativeRuleEventKind::PerspectiveObservationOccurrence {
                    source_event_id,
                    ..
                } => Some(*source_event_id),
                _ => None,
            })
            .collect();
        let declared = product
            .events
            .iter()
            .find(|event| {
                matches!(
                    event.event,
                    AuthoritativeRuleEventKind::AttackersDeclared { .. }
                )
            })
            .unwrap()
            .event_id;
        assert_eq!(
            sources.iter().filter(|source| **source == declared).count(),
            2
        );
    }

    /// P1's Savannah Lions (2/1) attacks P2 on turn 3 and the declaration is
    /// made: the state is P1's priority in the declare attackers step.
    fn after_declaring_an_attacker(
        admission: &ExecutableProfileAdmissionV1,
        state: EngineState,
    ) -> EngineState {
        let state = pass_until(admission, state, at_attackers(3));
        let product = declare(admission, &state, 1).unwrap();
        apply(&state, &product)
    }

    #[test]
    fn unblocked_attackers_deal_combat_damage_to_the_defending_player() {
        let (admission, state) = game_with_creature(3, P1);
        let creature = battlefield_creatures(&state)[0];
        let state = after_declaring_an_attacker(&admission, state);
        // Both players pass: the declare blockers step, in which P2 has no
        // creature to block with, so there is no declaration.
        let state = pass(&admission, &state).0;
        let state = pass(&admission, &state).0;
        assert_eq!(
            state.core.position,
            TurnPosition::Combat {
                step: CombatStep::DeclareBlockers
            }
        );
        assert_eq!(pending(&state).purpose, DecisionPurposeV4::PriorityAction);
        assert_eq!(pending(&state).actor, P1);
        let state = pass(&admission, &state).0;
        // P2's pass opens the combat damage step (CR 510.1a, 510.2).
        let (p1_life, p2_life) = (state.core.players[&P1].life, state.core.players[&P2].life);
        let (after, product) = pass(&admission, &state);
        assert_eq!(
            after.core.position,
            TurnPosition::Combat {
                step: CombatStep::CombatDamage
            }
        );
        assert_eq!(after.core.players[&P2].life, p2_life - 2);
        assert_eq!(after.core.players[&P1].life, p1_life);
        assert!(after.combat.as_ref().unwrap().damage_step_completed);
        assert!(after.card_rules.turn_history.players[&P2].lost_life_this_turn);
        // CR 510.3: the active player receives priority.
        assert_eq!(pending(&after).actor, P1);
        assert_eq!(pending(&after).purpose, DecisionPurposeV4::PriorityAction);
        let kinds: Vec<_> = product
            .events
            .iter()
            .filter_map(|event| match &event.event {
                AuthoritativeRuleEventKind::CombatDamageDealt { assignments } => {
                    assert_eq!(
                        assignments,
                        &vec![mtgml_state::DamageAssignmentV1 {
                            source: creature,
                            recipient: mtgml_state::DamageRecipientV1::Player { player: P2 },
                            amount: 2,
                        }]
                    );
                    Some("damage")
                }
                AuthoritativeRuleEventKind::LifeChanged { player, from, to } => {
                    assert_eq!((*player, *from, *to), (P2, p2_life, p2_life - 2));
                    Some("life")
                }
                AuthoritativeRuleEventKind::CombatDamageStepCompleted => Some("completed"),
                _ => None,
            })
            .collect();
        assert_eq!(kinds, ["damage", "life", "completed"]);

        // The step ends, combat ends, and the damage stays dealt.
        let end = pass_until(&admission, after, at(TurnPosition::PostcombatMain, 3));
        assert!(end.combat.is_none());
        assert_eq!(end.core.players[&P2].life, p2_life - 2);
    }

    #[test]
    fn nine_attackers_deal_their_damage_together() {
        // CR 508.1a: there is no limit on the number of attackers.
        let (admission, state) = game_with_creatures(3, &[P1; 9]);
        let creatures = battlefield_creatures(&state);
        assert_eq!(creatures.len(), 9);
        let state = pass_until(&admission, state, at_attackers(3));
        assert_eq!(offered_attackers(&state).len(), 9);
        assert_eq!(
            pending(&state).decision_domain_v2,
            DecisionDomainV2::ChooseMany {
                minimum: 0,
                maximum: 9
            }
        );
        validate_magic_pending_request(&admission, &state, &EpisodeStatus::Running).unwrap();
        let product = declare(&admission, &state, 9).unwrap();
        let state = apply(&state, &product);
        assert_eq!(state.combat.as_ref().unwrap().attackers, creatures);
        assert!(creatures
            .iter()
            .all(|creature| state.zones.objects[creature].tapped));

        let state = pass(&admission, &state).0;
        let state = pass(&admission, &state).0;
        let state = pass(&admission, &state).0;
        let life = state.core.players[&P2].life;
        let (after, product) = pass(&admission, &state);
        // Nine Savannah Lions deal 2 damage each, as one life change.
        assert_eq!(after.core.players[&P2].life, life - 18);
        let assignments: Vec<_> = product
            .events
            .iter()
            .filter_map(|event| match &event.event {
                AuthoritativeRuleEventKind::CombatDamageDealt { assignments } => {
                    Some(assignments.clone())
                }
                _ => None,
            })
            .collect();
        assert_eq!(assignments.len(), 1);
        assert_eq!(
            assignments[0]
                .iter()
                .map(|assignment| (assignment.source, assignment.amount))
                .collect::<Vec<_>>(),
            creatures
                .iter()
                .map(|creature| (*creature, 2))
                .collect::<Vec<_>>()
        );
        assert_eq!(
            product
                .events
                .iter()
                .filter(|event| matches!(
                    event.event,
                    AuthoritativeRuleEventKind::LifeChanged { .. }
                ))
                .count(),
            1
        );
    }

    /// The answer in `request` in which its creature blocks `attacker`, or
    /// nothing.
    fn block_answer(
        request: &AuthoritativeDecisionRequest,
        attacker: Option<GameObjectId>,
    ) -> DecisionAnswerV2 {
        DecisionAnswerV2::SelectOne {
            candidate_id: candidate(request, |binding| {
                matches!(binding,
                    EngineCandidateBinding::DeclareBlock { attacker: bound, .. }
                        if *bound == attacker)
            })
            .unwrap(),
        }
    }

    /// The block declaration in progress: the defender, the creatures still to
    /// ask, and the answers so far.
    fn block_declaration(
        state: &EngineState,
    ) -> (
        PlayerId,
        Vec<GameObjectId>,
        std::collections::BTreeMap<GameObjectId, Option<GameObjectId>>,
    ) {
        let [record] = state
            .execution
            .continuations
            .values()
            .collect::<Vec<_>>()
            .try_into()
            .expect("one continuation");
        match &record.payload {
            mtgml_state::ContinuationPayload::BlockDeclaration {
                defender,
                pending_blockers,
                declared,
            } => (*defender, pending_blockers.clone(), declared.clone()),
            other => panic!("{other:?}"),
        }
    }

    /// The block declarations the product's events make.
    fn declarations(
        product: &crate::BasicLandTransitionProduct,
    ) -> Vec<Vec<mtgml_state::CombatBlockerAssignmentV1>> {
        product
            .events
            .iter()
            .filter_map(|event| match &event.event {
                AuthoritativeRuleEventKind::BlockersDeclared { assignments } => {
                    Some(assignments.clone())
                }
                _ => None,
            })
            .collect()
    }

    /// `declared` with P2 holding priority in the declare attackers step:
    /// P1's pass has been made.
    fn defender_to_pass(
        admission: &ExecutableProfileAdmissionV1,
        declared: &EngineState,
    ) -> EngineState {
        let state = pass(admission, declared).0;
        assert_eq!(pending(&state).actor, P2);
        state
    }

    #[test]
    fn a_defender_with_an_untapped_creature_is_asked_to_block() {
        // CR 509.1a: P2 may block with its creature, so the declare blockers
        // step begins with P2's declaration, which nobody has priority for.
        let (admission, state) = game_with_creatures(3, &[P1, P2]);
        let attacking = after_declaring_an_attacker(&admission, state);
        let attacker = attacking.combat.as_ref().unwrap().attackers[0];
        let blocker = *battlefield_creatures(&attacking)
            .iter()
            .find(|creature| attacking.zones.objects[creature].controller == P2)
            .unwrap();
        let at_priority = defender_to_pass(&admission, &attacking);
        let product = submit(&admission, &at_priority, pass_answer(pending(&at_priority))).unwrap();
        assert_eq!(declarations(&product), Vec::<Vec<_>>::new());
        let asked = apply(&at_priority, &product);

        let request = pending(&asked);
        assert_eq!(request.actor, P2);
        assert_eq!(request.purpose, DecisionPurposeV4::BlockerDeclaration);
        assert_eq!(request.decision_domain_v2, DecisionDomainV2::ChooseOne);
        assert_eq!(request.visibility, DecisionVisibility::ActingPlayerOnly);
        assert_eq!(asked.core.priority, PriorityState::None);
        assert_eq!(
            block_declaration(&asked),
            (P2, vec![blocker], std::collections::BTreeMap::new())
        );
        assert_eq!(
            request.continuation_id,
            asked.execution.continuations.keys().next().copied()
        );
        // The creature does not block, or blocks the attacker.
        let bindings: Vec<_> = request
            .candidates
            .iter()
            .map(|candidate| candidate.trusted_binding.clone())
            .collect();
        assert_eq!(
            bindings,
            vec![
                EngineCandidateBinding::DeclareBlock {
                    blocker,
                    attacker: None
                },
                EngineCandidateBinding::DeclareBlock {
                    blocker,
                    attacker: Some(attacker)
                },
            ]
        );
        validate_magic_pending_request(&admission, &asked, &EpisodeStatus::Running).unwrap();

        // No block: one declaration with nothing in it. P1 has priority.
        let product = submit(&admission, &asked, block_answer(request, None)).unwrap();
        assert_eq!(declarations(&product), vec![Vec::new()]);
        let unblocked = apply(&asked, &product);
        assert!(unblocked.execution.continuations.is_empty());
        let combat = unblocked.combat.as_ref().unwrap();
        assert!(combat.blockers.is_empty() && combat.blocked_attackers.is_empty());
        assert_eq!(
            unblocked.core.priority,
            PriorityState::HeldBy {
                player: P1,
                consecutive_passes: 0
            }
        );
        assert_eq!(pending(&unblocked).actor, P1);
        assert_eq!(
            pending(&unblocked).purpose,
            DecisionPurposeV4::PriorityAction
        );
        validate_magic_pending_request(&admission, &unblocked, &EpisodeStatus::Running).unwrap();

        // A block: the attacker is blocked (CR 509.1g, 509.1h).
        let product = submit(&admission, &asked, block_answer(request, Some(attacker))).unwrap();
        assert_eq!(
            declarations(&product),
            vec![vec![mtgml_state::CombatBlockerAssignmentV1 {
                blocker,
                attacker
            }]]
        );
        let blocked = apply(&asked, &product);
        let combat = blocked.combat.as_ref().unwrap();
        assert_eq!(
            combat.blockers,
            std::collections::BTreeMap::from([(blocker, attacker)])
        );
        assert_eq!(
            combat.blocked_attackers,
            std::collections::BTreeSet::from([attacker])
        );
        validate_magic_pending_request(&admission, &blocked, &EpisodeStatus::Running).unwrap();

        // CR 509.1a: a tapped creature cannot block, so nothing is asked.
        let mut tapped = at_priority.clone();
        tapped.zones.objects.get_mut(&blocker).unwrap().tapped = true;
        let product = submit(&admission, &tapped, pass_answer(pending(&tapped))).unwrap();
        assert_eq!(declarations(&product), Vec::<Vec<_>>::new());
        let next = apply(&tapped, &product);
        assert!(next.execution.continuations.is_empty());
        assert_eq!(pending(&next).actor, P1);
        assert_eq!(pending(&next).purpose, DecisionPurposeV4::PriorityAction);
    }

    #[test]
    fn creatures_are_asked_in_the_order_of_the_defenders_opaque_ids() {
        // Requests and candidates follow the actor's opaque ids, never the
        // engine's object ids (INFORMATION_MODEL, Noninterference).
        let (admission, state) = game_with_creatures(3, &[P1, P2, P2]);
        let attacking = after_declaring_an_attacker(&admission, state);
        let [first, second]: [GameObjectId; 2] = battlefield_creatures(&attacking)
            .into_iter()
            .filter(|creature| attacking.zones.objects[creature].controller == P2)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        assert!(first < second);
        let at_priority = defender_to_pass(&admission, &attacking);
        let ask = |state: &EngineState| {
            apply(
                state,
                &submit(&admission, state, pass_answer(pending(state))).unwrap(),
            )
        };

        // The fixture's opaque ids rise with the object ids.
        let asked = ask(&at_priority);
        assert_eq!(
            block_declaration(&asked),
            (P2, vec![first, second], std::collections::BTreeMap::new())
        );

        // With P2's opaque ids of the two creatures swapped, the second object
        // is asked about first, and the request shows its opaque id.
        let mut swapped = at_priority.clone();
        let identity = swapped.perspective_identities.players.get_mut(&P2).unwrap();
        let (low, high) = (
            identity.object_to_opaque[&first],
            identity.object_to_opaque[&second],
        );
        assert!(low < high);
        identity.object_to_opaque.insert(first, high);
        identity.object_to_opaque.insert(second, low);
        identity.opaque_to_object.insert(low, second);
        identity.opaque_to_object.insert(high, first);
        let asked = ask(&swapped);
        assert_eq!(
            block_declaration(&asked),
            (P2, vec![second, first], std::collections::BTreeMap::new())
        );
        assert!(pending(&asked).candidates.iter().all(|candidate| matches!(
            candidate.visible_intent,
            CandidateIntent::DeclareBlock { blocker, .. } if blocker == low
        )));
        validate_magic_pending_request(&admission, &asked, &EpisodeStatus::Running).unwrap();

        // The first answer leaves the other creature to ask.
        let attacker = asked.combat.as_ref().unwrap().attackers[0];
        let product = submit(
            &admission,
            &asked,
            block_answer(pending(&asked), Some(attacker)),
        )
        .unwrap();
        assert_eq!(declarations(&product), Vec::<Vec<_>>::new());
        let half = apply(&asked, &product);
        assert_eq!(
            block_declaration(&half),
            (
                P2,
                vec![first],
                std::collections::BTreeMap::from([(second, Some(attacker))])
            )
        );
        assert!(half.combat.as_ref().unwrap().blockers.is_empty());
        validate_magic_pending_request(&admission, &half, &EpisodeStatus::Running).unwrap();
    }

    /// `state` with its combat moved to `step`, as a restored checkpoint could
    /// claim: an attack that has been declared, in a later step.
    fn in_combat_step(state: &EngineState, step: CombatStep) -> EngineState {
        let mut moved = state.clone();
        moved.core.position = TurnPosition::Combat { step };
        if matches!(step, CombatStep::CombatDamage | CombatStep::EndOfCombat) {
            // An attack that reached the damage step has dealt its damage.
            moved.combat.as_mut().unwrap().damage_step_completed = true;
        }
        moved
    }

    /// A restored state is one the game could have reached: it validates as a
    /// pending request, and answering it is not refused as unsupported.
    fn is_refused(admission: &ExecutableProfileAdmissionV1, state: &EngineState) -> bool {
        validate_magic_pending_request(admission, state, &EpisodeStatus::Running).is_err()
            && submit(admission, state, pass_answer(pending(state)))
                == Err(crate::BasicLandTransitionError::TurnProgressUnsupported)
    }

    #[test]
    fn a_restored_state_after_blocks_were_declared_may_hold_an_unblocking_creature() {
        // CR 509.1a: blocking is a choice. Once the declaration is over, in the
        // declare blockers step or after it, P2's untapped creature that did
        // not block is legal.
        let (admission, state) = game_with_creatures(3, &[P1, P2, P2]);
        let declared = after_declaring_an_attacker(&admission, state);
        validate_magic_pending_request(&admission, &declared, &EpisodeStatus::Running).unwrap();
        let defenders: Vec<_> = battlefield_creatures(&declared)
            .into_iter()
            .filter(|creature| declared.zones.objects[creature].controller == P2)
            .collect();
        assert!(defenders
            .iter()
            .all(|defender| !declared.zones.objects[defender].tapped));
        let running = EpisodeStatus::Running;

        // Neither creature blocked.
        for step in [
            CombatStep::DeclareBlockers,
            CombatStep::CombatDamage,
            CombatStep::EndOfCombat,
        ] {
            let restored = in_combat_step(&declared, step);
            validate_magic_pending_request(&admission, &restored, &running)
                .unwrap_or_else(|error| panic!("{step:?}: {error:?}"));
        }

        // One creature blocked and the other did not.
        let blocked = with_the_attacker_blocked(&declared, CombatStep::DeclareBlockers);
        assert_eq!(blocked.combat.as_ref().unwrap().blockers.len(), 1);
        validate_magic_pending_request(&admission, &blocked, &running).unwrap();
        // Damage dealt with a block is damage the game can have dealt.
        for step in [CombatStep::CombatDamage, CombatStep::EndOfCombat] {
            let restored = with_the_attacker_blocked(&declared, step);
            validate_magic_pending_request(&admission, &restored, &running)
                .unwrap_or_else(|error| panic!("{step:?}: {error:?}"));
        }
    }

    #[test]
    fn a_restored_damage_step_has_dealt_its_damage() {
        // CR 510.1, 510.3: the combat damage turn-based action happens on
        // entering the step, before any player has priority. The game never
        // rests in the step with attackers and the damage undealt.
        let (admission, state) = game_with_creature(3, P1);
        let declared = after_declaring_an_attacker(&admission, state);
        let dealt = in_combat_step(&declared, CombatStep::CombatDamage);
        assert!(dealt.combat.as_ref().unwrap().damage_step_completed);
        validate_magic_pending_request(&admission, &dealt, &EpisodeStatus::Running).unwrap();

        let mut undealt = dealt.clone();
        undealt.combat.as_mut().unwrap().damage_step_completed = false;
        assert!(is_refused(&admission, &undealt));
    }

    /// `declared` with P2's creature blocking P1's attacker, in `step`.
    fn with_the_attacker_blocked(declared: &EngineState, step: CombatStep) -> EngineState {
        let mut blocked = in_combat_step(declared, step);
        let blocker = *battlefield_creatures(declared)
            .iter()
            .find(|creature| declared.zones.objects[creature].controller == P2)
            .unwrap();
        let combat = blocked.combat.as_mut().unwrap();
        let attacker = combat.attackers[0];
        combat.blockers.insert(blocker, attacker);
        combat.blocked_attackers.insert(attacker);
        blocked
    }

    #[test]
    fn a_restored_combat_has_no_blocks_before_the_declare_blockers_step() {
        // CR 509.1: blockers are declared in the declare blockers step, which
        // comes after the attackers are declared (CR 508.2): there is no
        // block, and no blocked attacker, while priority is in the declare
        // attackers step.
        let (admission, state) = game_with_creatures(3, &[P1, P2]);
        let declared = after_declaring_an_attacker(&admission, state);
        let blocked = with_the_attacker_blocked(&declared, CombatStep::DeclareAttackers);
        // The state itself is well formed; only its timing is not.
        mtgml_state::validate_engine_state(&blocked).unwrap();
        assert_eq!(
            crate::combat::validate_reachable_combat(&admission, &blocked),
            Err(crate::BasicLandTransitionError::TurnProgressUnsupported)
        );
        // An attacker that is blocked without a blocker is as unreachable
        // (CR 509.1h: it becomes blocked in the declaration).
        let mut history = declared.clone();
        let combat = history.combat.as_mut().unwrap();
        combat.blocked_attackers.insert(combat.attackers[0]);
        mtgml_state::validate_engine_state(&history).unwrap();
        assert_eq!(
            crate::combat::validate_reachable_combat(&admission, &history),
            Err(crate::BasicLandTransitionError::TurnProgressUnsupported)
        );
        assert_eq!(
            crate::combat::validate_reachable_combat(&admission, &declared),
            Ok(())
        );
    }

    #[test]
    fn a_restored_blocker_is_a_creature_the_defending_player_controls() {
        // CR 509.1a: only creatures block. P2's creature is tapped here, so
        // it is no possible blocker (CR 509.1a) and the step is reachable
        // without any block.
        let (admission, state) = game_with_creatures(3, &[P1, P2]);
        let declared = after_declaring_an_attacker(&admission, state);
        let creature = *battlefield_creatures(&declared)
            .iter()
            .find(|creature| declared.zones.objects[creature].controller == P2)
            .unwrap();
        // A land under P2's control (the fixture's lands are P1's).
        let land = declared
            .zones
            .objects
            .values()
            .find(|object| {
                declared.zones.locations[&object.id].zone == ZoneKind::Battlefield
                    && object.id != creature
                    && battlefield_creatures(&declared)
                        .iter()
                        .all(|creature| *creature != object.id)
            })
            .unwrap()
            .id;
        let mut tapped = declared.clone();
        tapped.zones.objects.get_mut(&creature).unwrap().tapped = true;
        tapped.zones.objects.get_mut(&land).unwrap().controller = P2;
        let mut land_blocks = with_the_attacker_blocked(&tapped, CombatStep::DeclareBlockers);
        let combat = land_blocks.combat.as_mut().unwrap();
        combat.blockers = std::collections::BTreeMap::from([(land, combat.attackers[0])]);
        mtgml_state::validate_engine_state(&land_blocks).unwrap();
        assert_eq!(
            crate::combat::validate_reachable_combat(&admission, &land_blocks),
            Err(crate::BasicLandTransitionError::TurnProgressUnsupported)
        );
    }

    #[test]
    fn a_restored_combat_without_attackers_may_have_a_defender_with_a_creature() {
        // With no attackers the blockers step is skipped (CR 508.8): P2's
        // untapped creature is no possible blocker of anything.
        let (admission, state) = game_with_creatures(3, &[P2]);
        let state = pass_until(&admission, state, at_attackers(1));
        let declared = apply(&state, &declare(&admission, &state, 0).unwrap());
        for step in [CombatStep::DeclareBlockers, CombatStep::EndOfCombat] {
            let restored = in_combat_step(&declared, step);
            validate_magic_pending_request(&admission, &restored, &EpisodeStatus::Running)
                .unwrap_or_else(|error| panic!("{step:?}: {error:?}"));
        }
    }

    #[test]
    fn a_restored_combat_cannot_have_the_active_player_as_its_defender() {
        // The attacker would damage its own controller.
        let (admission, state) = game_with_creature(3, P1);
        let declared = after_declaring_an_attacker(&admission, state);
        let mut forged = declared.clone();
        forged.combat.as_mut().unwrap().defending_player = P1;
        for step in [
            CombatStep::DeclareAttackers,
            CombatStep::DeclareBlockers,
            CombatStep::CombatDamage,
        ] {
            let restored = in_combat_step(&forged, step);
            assert!(is_refused(&admission, &restored), "{step:?}");
        }
        assert!(!is_refused(&admission, &declared));
    }

    #[test]
    fn a_restored_combat_needs_attackers_the_active_player_controls() {
        let (admission, state) = game_with_creature(3, P1);
        let declared = after_declaring_an_attacker(&admission, state);
        let attacker = declared.combat.as_ref().unwrap().attackers[0];
        let land = declared
            .zones
            .objects
            .values()
            .find(|object| {
                object.controller == P1
                    && object.id != attacker
                    && declared.zones.locations[&object.id].zone == ZoneKind::Battlefield
            })
            .unwrap()
            .id;

        // An attacker the other player controls.
        let mut other_players = declared.clone();
        other_players
            .zones
            .objects
            .get_mut(&attacker)
            .unwrap()
            .controller = P2;
        // An attacker that is a land, not a creature.
        let mut not_a_creature = declared.clone();
        let combat = not_a_creature.combat.as_mut().unwrap();
        combat.attackers = vec![land];
        // An attacker that is not on the battlefield.
        let in_the_library = *declared
            .zones
            .locations
            .iter()
            .find(|(_, location)| location.zone == ZoneKind::Library)
            .unwrap()
            .0;
        let mut not_on_the_battlefield = declared.clone();
        let combat = not_on_the_battlefield.combat.as_mut().unwrap();
        combat.attackers = vec![in_the_library];

        for (name, forged) in [
            ("controlled by the other player", other_players),
            ("a land", not_a_creature),
            ("not on the battlefield", not_on_the_battlefield),
        ] {
            for step in [CombatStep::DeclareAttackers, CombatStep::CombatDamage] {
                let restored = in_combat_step(&forged, step);
                assert!(is_refused(&admission, &restored), "{name}, {step:?}");
            }
        }
    }

    #[test]
    fn a_restored_combat_is_in_a_combat_step_it_could_be_in() {
        // Combat state exists from the declaration to the end of combat.
        let (admission, state) = game_with_creature(3, P1);
        let declared = after_declaring_an_attacker(&admission, state);
        let restored = in_combat_step(&declared, CombatStep::BeginningOfCombat);
        assert!(is_refused(&admission, &restored));
    }

    #[test]
    fn every_boundary_of_an_attack_validates() {
        // The game itself reaches states the check must accept: the
        // declaration, the blockers step, the damage step, the end of combat.
        let (admission, state) = game_with_creature(3, P1);
        let mut state = after_declaring_an_attacker(&admission, state);
        let status = EpisodeStatus::Running;
        let mut steps = Vec::new();
        while state.combat.is_some() {
            validate_magic_pending_request(&admission, &state, &status).unwrap();
            steps.push(state.core.position);
            state = pass(&admission, &state).0;
        }
        for step in [
            CombatStep::DeclareAttackers,
            CombatStep::DeclareBlockers,
            CombatStep::CombatDamage,
            CombatStep::EndOfCombat,
        ] {
            assert!(steps.contains(&TurnPosition::Combat { step }), "{step:?}");
        }
    }

    #[test]
    fn lethal_combat_damage_ends_the_game_before_anyone_receives_priority() {
        // CR 704.5a, 704.3, 104.2a.
        let (admission, state) = game_with_creature(3, P1);
        let mut state = after_declaring_an_attacker(&admission, state);
        state.core.players.get_mut(&P2).unwrap().life = 2;
        let state = pass(&admission, &state).0;
        let state = pass(&admission, &state).0;
        let state = pass(&admission, &state).0;
        let product = submit(&admission, &state, pass_answer(pending(&state))).unwrap();
        let after = apply(&state, &product);

        assert_eq!(after.core.players[&P2].life, 0);
        assert!(after.core.players[&P2].has_lost && !after.core.players[&P1].has_lost);
        assert_eq!(after.core.priority, PriorityState::None);
        assert_eq!(product.next_decision, None);
        assert_eq!(
            product.status,
            EpisodeStatus::Terminal {
                reason: mtgml_model::TerminalReason::RulesLoss,
                players: vec![
                    mtgml_model::PlayerOutcome {
                        player: P1,
                        result: mtgml_model::PlayerResult::Win,
                    },
                    mtgml_model::PlayerOutcome {
                        player: P2,
                        result: mtgml_model::PlayerResult::Loss,
                    },
                ],
            }
        );
        assert!(product.events.iter().any(|event| event.event
            == AuthoritativeRuleEventKind::StateBasedActionsApplied {
                actions: vec![mtgml_state::SbaSelectedActionV1::PlayerLoses { player: P2 }],
            }));
        validate_magic_pending_request(&admission, &after, &product.status).unwrap();
    }

    #[test]
    fn a_decision_state_with_a_player_at_zero_life_who_has_not_lost_is_rejected() {
        // CR 704.3: state-based actions are checked before a player would
        // receive priority, so no decision is pending while one applies.
        let (admission, mut state) = game(3);
        let status = EpisodeStatus::Running;
        validate_magic_pending_request(&admission, &state, &status).unwrap();
        state.core.players.get_mut(&P2).unwrap().life = 0;
        assert!(validate_magic_pending_request(&admission, &state, &status).is_err());
        assert_eq!(
            submit(&admission, &state, pass_answer(pending(&state))),
            Err(crate::BasicLandTransitionError::TurnProgressUnsupported)
        );
        // Below zero too, and for the active player.
        state.core.players.get_mut(&P2).unwrap().life = 5;
        state.core.players.get_mut(&P1).unwrap().life = -3;
        assert!(validate_magic_pending_request(&admission, &state, &status).is_err());
    }

    #[test]
    fn empty_attack_declaration_ends_combat_without_damage() {
        let (admission, state) = game(3);
        let state = pass_until(&admission, state, |state| {
            state
                .execution
                .pending_decision
                .as_ref()
                .is_some_and(|request| request.purpose == DecisionPurposeV4::AttackerDeclaration)
        });
        let after = pass(&admission, &state).0;
        // Priority in declare attackers with an empty combat, then
        // end of combat (blockers and damage skipped), then postcombat main.
        assert!(after
            .combat
            .as_ref()
            .is_some_and(|combat| combat.attackers.is_empty()));
        assert_eq!(
            after.core.priority,
            PriorityState::HeldBy {
                player: P1,
                consecutive_passes: 0
            }
        );
        let state = pass(&admission, &after).0;
        let end_of_combat = pass(&admission, &state).0;
        assert_eq!(
            end_of_combat.core.position,
            TurnPosition::Combat {
                step: CombatStep::EndOfCombat
            }
        );
        let state = pass(&admission, &end_of_combat).0;
        let postcombat = pass(&admission, &state).0;
        assert_eq!(postcombat.core.position, TurnPosition::PostcombatMain);
        assert!(postcombat.combat.is_none());
    }

    #[test]
    fn end_step_passes_run_cleanup_and_open_next_upkeep() {
        let (admission, state) = game(3);
        let mountain = mana_source(&state);
        let state = tap_first_mana_source(&admission, &state);
        let state = pass_until(&admission, state, at(END_STEP, 1));
        let state = pass(&admission, &state).0;
        let after = pass(&admission, &state).0;

        let core = &after.core;
        assert_eq!(core.turn_number, 2);
        assert_eq!(core.active_player, P2);
        assert_eq!(core.position, UPKEEP);
        assert_eq!(
            core.priority,
            PriorityState::HeldBy {
                player: P2,
                consecutive_passes: 0
            }
        );
        assert_eq!(pending(&after).actor, P2);
        assert!(
            after.zones.objects[&mountain].tapped,
            "only the active player's permanents untap"
        );
        assert_eq!(after.card_rules.turn_history.turn_number, 2);
    }

    #[test]
    fn upkeep_passes_draw_one_card_for_the_active_player() {
        let (admission, state) = game(3);
        let state = pass_until(&admission, state, at(UPKEEP, 2));
        let hand_before = zone_count(&state, P2, ZoneKind::Hand);
        let library_before = zone_count(&state, P2, ZoneKind::Library);
        let state = pass(&admission, &state).0;
        let after = pass(&admission, &state).0;

        assert_eq!(after.core.position, DRAW);
        assert_eq!(zone_count(&after, P2, ZoneKind::Hand), hand_before + 1);
        assert_eq!(
            zone_count(&after, P2, ZoneKind::Library),
            library_before - 1
        );
        assert_eq!(pending(&after).actor, P2);
    }

    #[test]
    fn land_play_entitlement_resets_on_new_turn() {
        let (admission, state) = game(3);
        let state = tap_first_mana_source(&admission, &state);
        let state = play_first_land(&admission, &state);
        assert!(!has_play_land(&state));
        let state = pass_until(&admission, state, at(TurnPosition::PrecombatMain, 3));

        assert!(has_play_land(&state));
        assert!(battlefield_tapped(&state, P1).iter().all(|tapped| !tapped));
        assert_eq!(state.card_rules.turn_history.turn_number, 3);
    }

    #[test]
    fn drawing_from_an_empty_library_loses_the_game() {
        // CR 121.4, 704.5b, 104.2a: P2 draws the synthetic library card on
        // turn 2; P1's library is empty when P1 draws on turn 3, so P1 loses
        // before anyone receives priority in the draw step, and P2 wins.
        let (admission, state) = game(0);
        let state = pass_until(&admission, state, at(UPKEEP, 3));
        let state = pass(&admission, &state).0;
        let product = submit(&admission, &state, pass_answer(pending(&state))).unwrap();
        let after = apply(&state, &product);

        assert_eq!(
            product.status,
            EpisodeStatus::Terminal {
                reason: mtgml_model::TerminalReason::RulesLoss,
                players: vec![
                    mtgml_model::PlayerOutcome {
                        player: P1,
                        result: mtgml_model::PlayerResult::Loss,
                    },
                    mtgml_model::PlayerOutcome {
                        player: P2,
                        result: mtgml_model::PlayerResult::Win,
                    },
                ],
            }
        );
        let core = &after.core;
        assert!(core.players[&P1].has_lost && !core.players[&P2].has_lost);
        assert_eq!(core.position, DRAW);
        assert_eq!(core.priority, PriorityState::None);
        assert_eq!(product.next_decision, None);
        assert!(product.events.iter().any(|event| matches!(
            &event.event,
            event if *event == AuthoritativeRuleEventKind::StateBasedActionsApplied {
                actions: vec![mtgml_state::SbaSelectedActionV1::PlayerLoses { player: P1 }],
            }
        )));
        assert_eq!(
            zone_count(&after, P1, ZoneKind::Hand),
            zone_count(&state, P1, ZoneKind::Hand)
        );
        validate_magic_pending_request(&admission, &after, &product.status).unwrap();
    }

    #[test]
    fn unsupported_state_fails_closed() {
        let (admission, mut state) = game(3);
        let land = *state
            .card_rules
            .abilities
            .by_instance
            .values()
            .next()
            .map(|authority| &authority.source)
            .unwrap();
        // A counter is state the land-only slice cannot evaluate.
        state.card_rules.counters.counters.insert(
            land,
            std::collections::BTreeMap::from([(mtgml_state::CounterKindV1::PlusOnePlusOne, 1)]),
        );

        assert_eq!(
            submit(&admission, &state, pass_answer(pending(&state))),
            Err(crate::BasicLandTransitionError::TurnProgressUnsupported)
        );
    }

    #[test]
    fn products_validate_as_v3_deltas_for_two_full_turns() {
        let (admission, state) = game(3);
        let state = play_first_land(&admission, &state);
        // `pass` validates every product's delta, events and replacement.
        let state = pass_until(&admission, state, at(UPKEEP, 3));
        assert_eq!(state.core.active_player, P1);
    }

    fn discard_request(
        admission: &ExecutableProfileAdmissionV1,
        state: EngineState,
    ) -> EngineState {
        let state = pass_until(admission, state, at(END_STEP, 1));
        let state = pass(admission, &state).0;
        pass(admission, &state).0
    }

    fn discard(
        admission: &ExecutableProfileAdmissionV1,
        state: &EngineState,
        candidate_ids: Vec<mtgml_model::CandidateIdV1>,
    ) -> Result<crate::BasicLandTransitionProduct, crate::BasicLandTransitionError> {
        submit(
            admission,
            state,
            DecisionAnswerV2::SelectMany { candidate_ids },
        )
    }

    fn last_candidate(state: &EngineState) -> mtgml_model::CandidateIdV1 {
        pending(state).candidates.last().unwrap().candidate_id
    }

    #[test]
    fn cleanup_with_eight_cards_asks_active_player_to_discard_one() {
        let (admission, state) =
            game_with_hands(crate::basic_land::basic_land_admission_fixture(), 3, 6, 0);
        assert_eq!(zone_count(&state, P1, ZoneKind::Hand), 8);
        let state = discard_request(&admission, state);

        let request = pending(&state);
        assert_eq!(request.purpose, DecisionPurposeV4::HandSizeDiscard);
        assert_eq!(
            request.decision_domain_v2,
            DecisionDomainV2::ChooseMany {
                minimum: 1,
                maximum: 1
            }
        );
        assert_eq!(request.candidates.len(), 8);
        assert_eq!(request.actor, P1);
        assert_eq!(
            state.core.position,
            TurnPosition::Ending {
                step: EndingStep::Cleanup
            }
        );
        assert_eq!(state.core.turn_number, 1);
    }

    #[test]
    fn hands_the_slice_cannot_reach_are_rejected_before_play() {
        // Discarding two or more cards at once needs the owner's graveyard
        // order, which this slice does not offer. One draw per turn and the
        // discard to seven keep every reachable cleanup at one discard, so
        // larger hands are rejected before any step instead of mid-game.
        let admission = crate::basic_land::basic_land_admission_fixture();
        let status = EpisodeStatus::Running;
        for (p1_extra, p2_extra) in [(7, 0), (0, 8)] {
            let (_, state) = game_with_hands(admission.clone(), 3, p1_extra, p2_extra);
            assert!(
                zone_count(&state, P1, ZoneKind::Hand) == 9
                    || zone_count(&state, P2, ZoneKind::Hand) == 8
            );
            assert!(validate_magic_pending_request(&admission, &state, &status).is_err());
            assert_eq!(
                submit(&admission, &state, pass_answer(pending(&state))),
                Err(crate::BasicLandTransitionError::TurnProgressUnsupported)
            );
        }
        // The active player may hold eight after their draw, the other seven.
        let (_, state) = game_with_hands(admission.clone(), 3, 6, 7);
        assert_eq!(zone_count(&state, P1, ZoneKind::Hand), 8);
        assert_eq!(zone_count(&state, P2, ZoneKind::Hand), 7);
        validate_magic_pending_request(&admission, &state, &status).unwrap();
    }

    #[test]
    fn restored_attacker_and_discard_requests_validate() {
        let status = EpisodeStatus::Running;
        let (admission, state) = game(3);
        let state = pass_until(&admission, state, at(BEGIN_COMBAT, 1));
        let state = pass(&admission, &state).0;
        let attackers = pass(&admission, &state).0;
        let (_, state) =
            game_with_hands(crate::basic_land::basic_land_admission_fixture(), 3, 6, 0);
        let discard = discard_request(&admission, state);

        for (state, purpose) in [
            (attackers, DecisionPurposeV4::AttackerDeclaration),
            (discard, DecisionPurposeV4::HandSizeDiscard),
        ] {
            assert_eq!(pending(&state).purpose, purpose);
            validate_magic_pending_request(&admission, &state, &status).unwrap();
        }
    }

    #[test]
    fn discard_moves_chosen_card_then_next_turn_starts() {
        let (admission, state) =
            game_with_hands(crate::basic_land::basic_land_admission_fixture(), 3, 6, 0);
        let state = discard_request(&admission, state);
        let chosen = pending(&state)
            .candidates
            .last()
            .map(|candidate| match candidate.trusted_binding {
                EngineCandidateBinding::SelectObject { object } => object,
                _ => unreachable!(),
            })
            .unwrap();
        let product = discard(&admission, &state, vec![last_candidate(&state)]).unwrap();
        let after = apply(&state, &product);

        assert!(!after.zones.objects.contains_key(&chosen));
        assert_eq!(zone_count(&after, P1, ZoneKind::Graveyard), 1);
        assert_eq!(zone_count(&after, P1, ZoneKind::Hand), 7);
        assert_eq!(after.core.turn_number, 2);
        assert_eq!(after.core.active_player, P2);
        assert_eq!(after.core.position, UPKEEP);
        assert_eq!(pending(&after).actor, P2);
    }

    #[test]
    fn discard_with_wrong_count_is_rejected() {
        let (admission, state) =
            game_with_hands(crate::basic_land::basic_land_admission_fixture(), 3, 6, 0);
        let state = discard_request(&admission, state);
        let first = pending(&state).candidates[0].candidate_id;

        assert_eq!(
            discard(&admission, &state, Vec::new()),
            Err(crate::BasicLandTransitionError::InvalidSelection)
        );
        assert_eq!(
            discard(&admission, &state, vec![first, last_candidate(&state)]),
            Err(crate::BasicLandTransitionError::InvalidSelection)
        );
    }

    #[test]
    fn second_discard_shifts_the_known_graveyard_card() {
        // P1 discards on turn 1 and again after drawing on turn 3; the second
        // card lands on top and both players' knowledge of the first card
        // follows it down the graveyard.
        let (admission, state) =
            game_with_hands(crate::basic_land::basic_land_admission_fixture(), 3, 6, 0);
        let state = pass_until(&admission, state, at(END_STEP, 3));
        assert_eq!(zone_count(&state, P1, ZoneKind::Graveyard), 1);
        let state = pass(&admission, &state).0;
        let state = pass(&admission, &state).0;
        assert_eq!(pending(&state).purpose, DecisionPurposeV4::HandSizeDiscard);
        let after = pass(&admission, &state).0;

        assert_eq!(zone_count(&after, P1, ZoneKind::Graveyard), 2);
        assert_eq!(after.core.turn_number, 4);
    }

    // Combat damage with blocks (CR 510.1, 510.2). The vanilla creatures are
    // Savannah Lions (2/1), Gray Ogre (2/2) and Hill Giant (3/3), so every
    // fight between two of them is lethal for at least one of the two. The
    // damage step runs up to the state-based actions (`damage_step`), where
    // the marks and events of a fight can be seen; the whole step, which does
    // not support lethal damage yet, is run by `submit`.
    use crate::basic_land::{GRAY_OGRE, HILL_GIANT, SAVANNAH_LIONS};

    /// As `game_with_creatures`, with each creature made from the definition
    /// given with its controller. Returns the creatures in the order given.
    fn game_with_creature_cards(
        cards: &[(PlayerId, mtgml_model::CardDefinitionId)],
    ) -> (ExecutableProfileAdmissionV1, EngineState, Vec<GameObjectId>) {
        let admission = crate::basic_land::vanilla_creature_admission_fixture();
        let mut state = crate::basic_land::s1_b_state_with_two_lands_fixture();
        make_synthetic_library_card_ordinary(&mut state);
        add_library_cards(&mut state, P1, 3);
        add_library_cards(&mut state, P2, 3);
        let creatures = cards
            .iter()
            .map(|(controller, definition)| {
                crate::basic_land::put_creature_card_on_battlefield(
                    &mut state,
                    *controller,
                    *definition,
                )
            })
            .collect();
        crate::install_basic_land_request(&admission, &mut state, P1, &EpisodeStatus::Running)
            .unwrap();
        (admission, state, creatures)
    }

    /// P1 attacks with the creatures `attackers` on turn 3, and P2 blocks
    /// with its creatures as `blocks` says (each pair is a blocker of P2 and
    /// the attacker it blocks; any other creature of P2 does not block). The
    /// result is P2 holding priority in the declare blockers step, before the
    /// pass that opens the combat damage step.
    fn blocks_declared(
        admission: &ExecutableProfileAdmissionV1,
        state: EngineState,
        attackers: &[GameObjectId],
        blocks: &[(GameObjectId, GameObjectId)],
    ) -> EngineState {
        let state = pass_until(admission, state, at_attackers(3));
        let candidate_ids: Vec<_> = pending(&state)
            .candidates
            .iter()
            .filter(|candidate| match candidate.trusted_binding {
                EngineCandidateBinding::SelectObject { object } => attackers.contains(&object),
                _ => false,
            })
            .map(|candidate| candidate.candidate_id)
            .collect();
        assert_eq!(candidate_ids.len(), attackers.len());
        let product = submit(
            admission,
            &state,
            DecisionAnswerV2::SelectMany { candidate_ids },
        )
        .unwrap();
        let mut state = apply(&state, &product);
        // P1 and P2 pass in the declare attackers step; P2 declares blocks.
        state = pass(admission, &state).0;
        state = pass(admission, &state).0;
        while pending(&state).purpose == DecisionPurposeV4::BlockerDeclaration {
            let asked = block_declaration(&state).1[0];
            let attacker = blocks
                .iter()
                .find(|(blocker, _)| *blocker == asked)
                .map(|(_, attacker)| *attacker);
            let product =
                submit(admission, &state, block_answer(pending(&state), attacker)).unwrap();
            state = apply(&state, &product);
        }
        // P1 passes: P2 holds priority.
        let state = pass(admission, &state).0;
        assert_eq!(pending(&state).actor, P2);
        assert_eq!(
            state.core.position,
            TurnPosition::Combat {
                step: CombatStep::DeclareBlockers
            }
        );
        state
    }

    /// The combat damage step that P2's pass opens in `before`, without the
    /// state-based actions that follow it: the damage is dealt and the
    /// transition is finished and validated as `progress` does.
    fn damage_step(
        admission: &ExecutableProfileAdmissionV1,
        before: &EngineState,
    ) -> Result<crate::BasicLandTransitionProduct, crate::BasicLandTransitionError> {
        let mut next = before.clone();
        next.execution.pending_decision = None;
        next.revision = StateRevision(before.revision.0 + 1);
        next.core.priority = PriorityState::None;
        next.core.position = TurnPosition::Combat {
            step: CombatStep::CombatDamage,
        };
        let mut facts = Facts::default();
        crate::combat::deal_combat_damage(admission, &mut next, &mut facts)?;
        let decision = open_priority(&mut next);
        finish(admission, before, pending(before), next, facts, decision)
    }

    /// The marked damage of `object` in `state`.
    fn marked(state: &EngineState, object: GameObjectId) -> u64 {
        state.card_rules.permanents.permanents[&object].marked_damage
    }

    /// What the events of a damage step say, in order: the damage dealt, the
    /// life lost, the damage marked and the completion of the step.
    fn damage_events(product: &crate::BasicLandTransitionProduct) -> Vec<String> {
        product
            .events
            .iter()
            .filter_map(|event| match &event.event {
                AuthoritativeRuleEventKind::CombatDamageDealt { assignments } => Some(format!(
                    "damage {}",
                    assignments
                        .iter()
                        .map(|assignment| match assignment.recipient {
                            mtgml_state::DamageRecipientV1::Player { player } => format!(
                                "{}>P{}:{}",
                                assignment.source.0, player.0, assignment.amount
                            ),
                            mtgml_state::DamageRecipientV1::Creature { object } => format!(
                                "{}>{}:{}",
                                assignment.source.0, object.0, assignment.amount
                            ),
                        })
                        .collect::<Vec<_>>()
                        .join(" ")
                )),
                AuthoritativeRuleEventKind::LifeChanged { player, from, to } => {
                    Some(format!("life P{} {from} {to}", player.0))
                }
                AuthoritativeRuleEventKind::MarkedDamageChanged { creature, from, to } => {
                    Some(format!("marked {} {from} {to}", creature.0))
                }
                AuthoritativeRuleEventKind::CombatDamageStepCompleted => {
                    Some("completed".to_owned())
                }
                _ => None,
            })
            .collect()
    }

    /// The events of the product that a player observes: each one that an
    /// observation occurrence follows.
    fn observed_events(
        product: &crate::BasicLandTransitionProduct,
    ) -> Vec<&AuthoritativeRuleEventKind> {
        let sources: Vec<_> = product
            .events
            .iter()
            .filter_map(|event| match &event.event {
                AuthoritativeRuleEventKind::PerspectiveObservationOccurrence {
                    source_event_id,
                    ..
                } => Some(*source_event_id),
                _ => None,
            })
            .collect();
        product
            .events
            .iter()
            .filter(|event| sources.contains(&event.event_id))
            .map(|event| &event.event)
            .collect()
    }

    #[test]
    fn the_vanilla_creatures_have_the_powers_and_toughnesses_the_fights_assume() {
        let (admission, state, creatures) =
            game_with_creature_cards(&[(P1, SAVANNAH_LIONS), (P1, GRAY_OGRE), (P1, HILL_GIANT)]);
        let queries = crate::S1QueryAuthority::for_objects(&admission, &state, &creatures).unwrap();
        let stats: Vec<_> = queries
            .iter()
            .map(|query| query.derive_base_characteristics().base_power_toughness)
            .collect();
        assert_eq!(stats, [Some((2, 1)), Some((2, 2)), Some((3, 3))]);
    }

    #[test]
    fn a_blocked_attacker_and_its_blocker_damage_each_other() {
        // CR 510.1c, 510.1d, 510.2: Gray Ogre (2/2) is blocked by Savannah
        // Lions (2/1). Each deals its power to the other at once, and the
        // damage is marked (CR 120.3e). The defending player is not damaged.
        let (admission, state, creatures) =
            game_with_creature_cards(&[(P1, GRAY_OGRE), (P2, SAVANNAH_LIONS)]);
        let [ogre, lions] = creatures[..] else {
            panic!("two creatures")
        };
        let before = blocks_declared(&admission, state, &[ogre], &[(lions, ogre)]);
        let life = (before.core.players[&P1].life, before.core.players[&P2].life);

        let product = damage_step(&admission, &before).unwrap();
        let after = apply(&before, &product);
        assert_eq!((marked(&after, ogre), marked(&after, lions)), (2, 2));
        assert_eq!(
            (after.core.players[&P1].life, after.core.players[&P2].life),
            life
        );
        assert!(after.combat.as_ref().unwrap().damage_step_completed);
        // No player lost life, so no life loss is recorded.
        assert!(!after.card_rules.turn_history.players[&P2].lost_life_this_turn);
        assert_eq!(
            damage_events(&product),
            [
                format!("damage {}>{}:2 {}>{}:2", ogre.0, lions.0, lions.0, ogre.0),
                format!("marked {} 0 2", ogre.0),
                format!("marked {} 0 2", lions.0),
                "completed".to_owned(),
            ]
        );
        // The blocks stay: the creatures are still blocking and blocked.
        let combat = after.combat.as_ref().unwrap();
        assert_eq!(
            combat.blockers,
            std::collections::BTreeMap::from([(lions, ogre)])
        );
        // Marked damage is shown to no player yet, and nothing else of the
        // step is public: no life changed.
        assert_eq!(
            observed_events(&product),
            Vec::<&AuthoritativeRuleEventKind>::new()
        );
    }

    #[test]
    fn an_unblocked_attacker_still_hits_the_player_while_another_is_blocked() {
        // Hill Giant (3/3) is blocked by Savannah Lions; P1's other Savannah
        // Lions is not, and deals its 2 damage to P2 (CR 510.1b).
        let (admission, state, creatures) = game_with_creature_cards(&[
            (P1, HILL_GIANT),
            (P1, SAVANNAH_LIONS),
            (P2, SAVANNAH_LIONS),
        ]);
        let [giant, free, blocker] = creatures[..] else {
            panic!("three creatures")
        };
        let before = blocks_declared(&admission, state, &[giant, free], &[(blocker, giant)]);
        let p2_life = before.core.players[&P2].life;

        let product = damage_step(&admission, &before).unwrap();
        let after = apply(&before, &product);
        assert_eq!(after.core.players[&P2].life, p2_life - 2);
        assert_eq!(after.core.players[&P1].life, before.core.players[&P1].life);
        assert!(after.card_rules.turn_history.players[&P2].lost_life_this_turn);
        assert_eq!(
            (
                marked(&after, giant),
                marked(&after, free),
                marked(&after, blocker)
            ),
            (2, 0, 3)
        );
        assert_eq!(
            damage_events(&product),
            [
                format!(
                    "damage {}>{}:3 {}>P2:2 {}>{}:2",
                    giant.0, blocker.0, free.0, blocker.0, giant.0
                ),
                format!("life P2 {p2_life} {}", p2_life - 2),
                format!("marked {} 0 2", giant.0),
                format!("marked {} 0 3", blocker.0),
                "completed".to_owned(),
            ]
        );
        // The life lost is public and the marked damage is not shown yet.
        assert_eq!(
            observed_events(&product),
            [&AuthoritativeRuleEventKind::LifeChanged {
                player: P2,
                from: p2_life,
                to: p2_life - 2
            }]
        );
    }

    #[test]
    fn an_attacker_whose_blocker_is_gone_deals_no_damage() {
        // CR 510.1c, 509.1h: a creature stays blocked when its blockers are
        // gone, and then assigns no combat damage. It does not hit the player.
        let (admission, state, creatures) =
            game_with_creature_cards(&[(P1, SAVANNAH_LIONS), (P2, SAVANNAH_LIONS)]);
        let [attacker, blocker] = creatures[..] else {
            panic!("two creatures")
        };
        let mut before = blocks_declared(&admission, state, &[attacker], &[(blocker, attacker)]);
        // The blocker leaves combat (a state only a death could make).
        before.combat.as_mut().unwrap().blockers.clear();
        assert!(before
            .combat
            .as_ref()
            .unwrap()
            .blocked_attackers
            .contains(&attacker));
        validate_magic_pending_request(&admission, &before, &EpisodeStatus::Running).unwrap();

        // The whole step, through the production path.
        let product = submit(&admission, &before, pass_answer(pending(&before))).unwrap();
        let after = apply(&before, &product);
        assert_eq!(
            after.core.position,
            TurnPosition::Combat {
                step: CombatStep::CombatDamage
            }
        );
        assert_eq!(after.core.players, before.core.players);
        assert_eq!((marked(&after, attacker), marked(&after, blocker)), (0, 0));
        assert!(after.combat.as_ref().unwrap().damage_step_completed);
        assert_eq!(damage_events(&product), ["completed"]);
        assert!(!after.card_rules.turn_history.players[&P2].lost_life_this_turn);
    }

    #[test]
    fn two_blockers_on_one_attacker_fail_closed_until_damage_can_be_divided() {
        // CR 510.1c: an attacker blocked by two creatures divides its damage
        // between them as its controller chooses, which is not supported yet.
        let (admission, state, creatures) = game_with_creature_cards(&[
            (P1, HILL_GIANT),
            (P2, SAVANNAH_LIONS),
            (P2, SAVANNAH_LIONS),
        ]);
        let [giant, first, second] = creatures[..] else {
            panic!("three creatures")
        };
        let before = blocks_declared(
            &admission,
            state.clone(),
            &[giant],
            &[(first, giant), (second, giant)],
        );
        assert_eq!(before.combat.as_ref().unwrap().blockers.len(), 2);
        assert_eq!(
            damage_step(&admission, &before),
            Err(crate::BasicLandTransitionError::TurnProgressUnsupported)
        );
        assert_eq!(
            submit(&admission, &before, pass_answer(pending(&before))),
            Err(crate::BasicLandTransitionError::TurnProgressUnsupported)
        );

        // With one of them blocking, the same fight is supported: it is the
        // two blockers that fail closed, not the damage.
        let one = blocks_declared(&admission, state, &[giant], &[(first, giant)]);
        assert!(damage_step(&admission, &one).is_ok());
    }

    #[test]
    fn lethal_damage_fails_closed_until_creatures_can_die() {
        // CR 704.5g: Gray Ogre (2/2) and Savannah Lions (2/1) kill each
        // other. Death is not supported yet, so the damage step is refused
        // and the state it was asked about is unchanged.
        let (admission, state, creatures) =
            game_with_creature_cards(&[(P1, GRAY_OGRE), (P2, SAVANNAH_LIONS)]);
        let [ogre, lions] = creatures[..] else {
            panic!("two creatures")
        };
        let before = blocks_declared(&admission, state, &[ogre], &[(lions, ogre)]);
        let kept = before.clone();
        assert_eq!(
            submit(&admission, &before, pass_answer(pending(&before))),
            Err(crate::BasicLandTransitionError::TurnProgressUnsupported)
        );
        assert_eq!(before, kept);
        // The same step without a block is supported: it is the lethal
        // damage that is not.
        let (_, unblocked_state, creatures) = game_with_creature_cards(&[(P1, GRAY_OGRE)]);
        let unblocked = blocks_declared(&admission, unblocked_state, &[creatures[0]], &[]);
        assert!(submit(&admission, &unblocked, pass_answer(pending(&unblocked))).is_ok());
    }

    #[test]
    fn a_restored_combat_after_damage_with_a_block_is_accepted() {
        // Hill Giant (3/3) blocked by Savannah Lions: the Giant has 2 damage
        // marked, which is not lethal. The Lions' 3 would be, so the state
        // restored has only the Giant's damage.
        let (admission, state, creatures) =
            game_with_creature_cards(&[(P1, HILL_GIANT), (P2, SAVANNAH_LIONS)]);
        let [giant, lions] = creatures[..] else {
            panic!("two creatures")
        };
        let before = blocks_declared(&admission, state, &[giant], &[(lions, giant)]);
        let mut restored = apply(&before, &damage_step(&admission, &before).unwrap());
        assert_eq!((marked(&restored, giant), marked(&restored, lions)), (2, 3));
        restored
            .card_rules
            .permanents
            .permanents
            .get_mut(&lions)
            .unwrap()
            .marked_damage = 0;
        let running = EpisodeStatus::Running;
        for step in [CombatStep::CombatDamage, CombatStep::EndOfCombat] {
            let mut restored = restored.clone();
            restored.core.position = TurnPosition::Combat { step };
            validate_magic_pending_request(&admission, &restored, &running)
                .unwrap_or_else(|error| panic!("{step:?}: {error:?}"));
        }
    }

    #[test]
    fn a_restored_blocked_attacker_without_a_blocker_is_accepted() {
        // CR 509.1h: the attacker stays blocked after its blockers are gone.
        let (admission, state, creatures) =
            game_with_creature_cards(&[(P1, SAVANNAH_LIONS), (P2, SAVANNAH_LIONS)]);
        let [attacker, blocker] = creatures[..] else {
            panic!("two creatures")
        };
        let mut restored = blocks_declared(&admission, state, &[attacker], &[(blocker, attacker)]);
        restored.combat.as_mut().unwrap().blockers.clear();
        for step in [
            CombatStep::DeclareBlockers,
            CombatStep::CombatDamage,
            CombatStep::EndOfCombat,
        ] {
            let restored = in_combat_step(&restored, step);
            validate_magic_pending_request(&admission, &restored, &EpisodeStatus::Running)
                .unwrap_or_else(|error| panic!("{step:?}: {error:?}"));
        }
    }

    /// The states of a game on turn 3 that a checkpoint is restored in, of the
    /// two kinds validation reaches in different ways: a priority window and
    /// the attacker declaration.
    fn restore_points(
        admission: &ExecutableProfileAdmissionV1,
        state: EngineState,
    ) -> Vec<EngineState> {
        vec![
            pass_until(admission, state.clone(), at(TurnPosition::PrecombatMain, 3)),
            pass_until(admission, state, at_attackers(3)),
        ]
    }

    #[test]
    fn a_restored_creature_with_lethal_damage_is_refused() {
        // CR 704.5g: a creature with damage marked at least equal to its
        // toughness is destroyed before any player has priority, so no game
        // rests there. Hill Giant (3/3), Gray Ogre (2/2), Savannah Lions (2/1).
        let (admission, state, creatures) =
            game_with_creature_cards(&[(P1, HILL_GIANT), (P1, GRAY_OGRE), (P1, SAVANNAH_LIONS)]);
        let running = EpisodeStatus::Running;
        for mut state in restore_points(&admission, state) {
            for (creature, lethal) in creatures.iter().zip([3, 2, 1]) {
                let mark = |state: &mut EngineState, damage| {
                    state
                        .card_rules
                        .permanents
                        .permanents
                        .get_mut(creature)
                        .unwrap()
                        .marked_damage = damage;
                };
                for damage in 0..lethal {
                    mark(&mut state, damage);
                    validate_magic_pending_request(&admission, &state, &running)
                        .unwrap_or_else(|error| panic!("{creature:?} {damage}: {error:?}"));
                }
                mark(&mut state, lethal);
                assert!(
                    validate_magic_pending_request(&admission, &state, &running).is_err(),
                    "{creature:?} {lethal} at {:?}",
                    pending(&state).purpose
                );
                assert_eq!(
                    submit(&admission, &state, pass_answer(pending(&state))),
                    Err(crate::BasicLandTransitionError::TurnProgressUnsupported),
                    "{creature:?} {lethal} at {:?}",
                    pending(&state).purpose
                );
                mark(&mut state, 0);
            }
        }
    }

    #[test]
    fn a_restored_land_with_marked_damage_is_refused() {
        // Damage is marked on creatures (CR 120.3e): a land has none.
        let (admission, state, _) = game_with_creature_cards(&[(P1, HILL_GIANT)]);
        let running = EpisodeStatus::Running;
        for state in restore_points(&admission, state) {
            let land = state
                .card_rules
                .permanents
                .permanents
                .keys()
                .copied()
                .find(|object| state.zones.objects[object].card_definition != HILL_GIANT)
                .unwrap();
            validate_magic_pending_request(&admission, &state, &running).unwrap();
            let mut damaged = state.clone();
            damaged
                .card_rules
                .permanents
                .permanents
                .get_mut(&land)
                .unwrap()
                .marked_damage = 1;
            assert!(
                validate_magic_pending_request(&admission, &damaged, &running).is_err(),
                "{:?}",
                pending(&damaged).purpose
            );
            assert_eq!(
                submit(&admission, &damaged, pass_answer(pending(&damaged))),
                Err(crate::BasicLandTransitionError::TurnProgressUnsupported),
                "{:?}",
                pending(&damaged).purpose
            );
        }
    }

    #[test]
    fn a_restored_tapped_blocker_is_refused() {
        // CR 509.1a: only an untapped creature blocks, and nothing taps a
        // blocker afterwards.
        let (admission, state, creatures) =
            game_with_creature_cards(&[(P1, SAVANNAH_LIONS), (P2, SAVANNAH_LIONS)]);
        let [attacker, blocker] = creatures[..] else {
            panic!("two creatures")
        };
        let blocked = blocks_declared(&admission, state, &[attacker], &[(blocker, attacker)]);
        let running = EpisodeStatus::Running;
        validate_magic_pending_request(&admission, &blocked, &running).unwrap();
        for step in [
            CombatStep::DeclareBlockers,
            CombatStep::CombatDamage,
            CombatStep::EndOfCombat,
        ] {
            let mut tapped = in_combat_step(&blocked, step);
            tapped.zones.objects.get_mut(&blocker).unwrap().tapped = true;
            mtgml_state::validate_engine_state(&tapped).unwrap();
            assert_eq!(
                crate::combat::validate_reachable_combat(&admission, &tapped),
                Err(crate::BasicLandTransitionError::TurnProgressUnsupported),
                "{step:?}"
            );
            assert!(is_refused(&admission, &tapped), "{step:?}");
        }
    }
}
