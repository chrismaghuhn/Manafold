//! Native turn progression for the land-only slice.
//!
//! Passing priority and every turn-based action run directly on V3 state:
//! step changes, untap, draw, the combat skeleton, cleanup and the turn
//! change. One response is one transition (`StateRevision` +1, one
//! `StateDelta`). V3 validates turn-position, priority, active-player and
//! turn-number events against the transition's endpoints, so each changed
//! aspect gets exactly one net event.
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
    BeginningStep, CombatState, CombatStep, EndingStep, EngineState, ManaPoolChangeCauseV1,
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
/// declaring attackers run the turn progression.
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
            reject_attackers_while_creatures_cannot_attack(admission, state)?;
            validate_magic_pending_request(admission, state, status)
                .map_err(|_| Error::InvalidSelection)?;
            match &response.answer {
                DecisionAnswerV2::SelectMany { candidate_ids } if candidate_ids.is_empty() => {
                    Answer::NoAttackers
                }
                // Attacking creatures are not supported yet.
                _ => return Err(Error::TurnProgressUnsupported),
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
    if !hands_within_slice(state) {
        return Err(BasicLandCandidateError::InvalidState);
    }
    let Some(request) = state.execution.pending_decision.as_ref().filter(|request| {
        matches!(
            request.purpose,
            DecisionPurposeV4::AttackerDeclaration
                | DecisionPurposeV4::HandSizeDiscard
                | DecisionPurposeV4::ManaPayment
        )
    }) else {
        return crate::validate_basic_land_pending_request(admission, state, status);
    };
    if request.purpose == DecisionPurposeV4::HandSizeDiscard {
        return validate_discard_request(admission, state, request, status);
    }
    if request.purpose == DecisionPurposeV4::ManaPayment {
        return validate_payment_request(admission, state, request, status);
    }
    state
        .validate_structure()
        .map_err(|_| BasicLandCandidateError::InvalidState)?;
    validate_slice(admission, state).map_err(|_| BasicLandCandidateError::InvalidState)?;
    reject_attackers_while_creatures_cannot_attack(admission, state)
        .map_err(|_| BasicLandCandidateError::InvalidState)?;
    // Only an admission with the rule that creates this request accepts it.
    admits(admission, "rules/declare-attackers")
        .map_err(|_| BasicLandCandidateError::PendingCandidateSetMismatch)?;
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
                maximum: 0,
            })
        || !request.candidates.is_empty()
        || !actor_only_request_matches(state, request)
    {
        return Err(BasicLandCandidateError::PendingCandidateSetMismatch);
    }
    Ok(())
}

enum Answer {
    Pass,
    NoAttackers,
    Discard(GameObjectId),
}

pub(crate) enum NextDecision {
    Priority(PlayerId),
    /// The caster of the spell on the stack chooses how to pay for it.
    Payment,
    Attackers,
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
    attackers_declared: bool,
    combat_skipped: bool,
    combat_ended: bool,
    /// Zone moves, shuffles and public events in the order they happened,
    /// each followed by the occurrences it caused.
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

/// The slice this progression can evaluate (D13): exactly two players, only
/// admitted lands and vanilla creatures on the battlefield, a stack that is
/// empty or holds one creature spell, no continuation but the payment of that
/// spell, and none of the state no rule of this slice can evaluate. Creatures
/// may be on the battlefield but cannot attack yet (see
/// `reject_attackers_while_creatures_cannot_attack`), so no combat damage is
/// dealt and no state-based action can apply.
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
            && crate::casting::pending_payment(admission, state).is_err())
        || !execution.effects.is_empty()
        || !execution.waiting_triggers.is_empty()
        || !execution.delayed_effects.is_empty()
        || !crate::casting::stack_within_profile(admission, state)
        || parts.combat.as_ref().is_some_and(|combat| {
            !combat.attackers.is_empty()
                || !combat.blockers.is_empty()
                || !combat.blocked_attackers.is_empty()
        })
    {
        return Err(Error::TurnProgressUnsupported);
    }
    if !hands_within_slice(state) {
        return Err(Error::TurnProgressUnsupported);
    }
    crate::S1QueryAuthority::for_objects(admission, state, &battlefield_objects(state))
        .map_err(|_| Error::TurnProgressUnsupported)?;
    Ok(())
}

fn battlefield_objects(state: &EngineState) -> Vec<GameObjectId> {
    state
        .zones
        .locations
        .iter()
        .filter(|(_, location)| location.zone == ZoneKind::Battlefield)
        .map(|(object, _)| *object)
        .collect()
}

/// Declaring attackers fails closed while the active player controls a
/// creature (CR 302.6, 508.1a): the request cannot yet offer it as an
/// attacker, and offering no candidate would hide a legal choice. Attacks
/// arrive with a later rule; until then such a state is unsupported rather
/// than played as though the creature could not attack. Creatures the
/// non-active player controls may stay: they cannot attack on this turn.
fn reject_attackers_while_creatures_cannot_attack(
    admission: &ExecutableProfileAdmissionV1,
    state: &EngineState,
) -> Result<(), Error> {
    let queries =
        crate::S1QueryAuthority::for_objects(admission, state, &battlefield_objects(state))
            .map_err(|_| Error::TurnProgressUnsupported)?;
    let controls_creature = queries.iter().any(|query| {
        query.queried_object().controller == state.core.active_player
            && query
                .derive_base_characteristics()
                .card_types
                .iter()
                .any(|card_type| card_type == "Creature")
    });
    if controls_creature {
        return Err(Error::TurnProgressUnsupported);
    }
    Ok(())
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
        // CR 508.1, 508.2: no attackers are declared; the active player then
        // receives priority in the declare attackers step.
        Answer::NoAttackers => {
            next.combat = Some(CombatState {
                defending_player: other,
                attackers: Vec::new(),
                damage_step_completed: false,
                blocked_attackers: Default::default(),
                blockers: Default::default(),
            });
            facts.attackers_declared = true;
            next.core.priority = PriorityState::HeldBy {
                player: active,
                consecutive_passes: 0,
            };
            NextDecision::Priority(active)
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
                reject_attackers_while_creatures_cannot_attack(admission, next)?;
                return Ok(NextDecision::Attackers);
            }
            // Only reachable with declared attackers, which this slice has not.
            TurnPosition::Combat {
                step: CombatStep::DeclareBlockers | CombatStep::CombatDamage,
            } => return Err(Error::TurnProgressUnsupported),
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
    if facts.attackers_declared {
        let combat = next.combat.as_ref().ok_or(Error::InvalidResult)?;
        pending.push(kind(AuthoritativeRuleEventKind::AttackersDeclared {
            defending_player: combat.defending_player,
            attackers: combat.attackers.clone(),
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
        NextDecision::Attackers => (running, Some(install_attacker_request(&mut next)?)),
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

/// CR 508.1: the active player declares attackers. Creatures cannot attack
/// yet, and the active player controls none here (the step before this
/// refuses otherwise), so the request offers no candidates.
fn install_attacker_request(next: &mut EngineState) -> Result<AuthoritativeDecisionRequest, Error> {
    install_actor_only_request(
        next,
        DecisionPurposeV4::AttackerDeclaration,
        DecisionDomainV2::ChooseMany {
            minimum: 0,
            maximum: 0,
        },
        Vec::new(),
    )
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
        let admission = crate::basic_land::vanilla_creature_admission_fixture();
        let mut state = crate::basic_land::s1_b_state_with_two_lands_fixture();
        make_synthetic_library_card_ordinary(&mut state);
        add_library_cards(&mut state, P1, library);
        add_library_cards(&mut state, P2, library);
        crate::basic_land::put_vanilla_creature_on_battlefield(&mut state, controller);
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

    #[test]
    fn declaring_attackers_fails_closed_while_the_active_player_controls_a_creature() {
        let (admission, state) = game_with_creature(3, P1);
        let state = pass_until(&admission, state, at(BEGIN_COMBAT, 1));
        let state = pass(&admission, &state).0;
        // The second pass would open the declaration of attackers, which
        // cannot offer the creature yet and must not offer none.
        assert_eq!(
            submit(&admission, &state, pass_answer(pending(&state))),
            Err(crate::BasicLandTransitionError::TurnProgressUnsupported)
        );
    }

    #[test]
    fn a_pending_attacker_declaration_is_refused_while_the_active_player_controls_a_creature() {
        let (admission, state) =
            game_with(crate::basic_land::vanilla_creature_admission_fixture(), 3);
        let mut state = pass_until(&admission, state, at_attacker_declaration);
        let status = EpisodeStatus::Running;
        assert!(validate_magic_pending_request(&admission, &state, &status).is_ok());

        // The same pending request in a restored state that also holds a
        // creature of the active player is refused, and so is its answer.
        crate::basic_land::put_vanilla_creature_on_battlefield(&mut state, P1);
        assert!(validate_magic_pending_request(&admission, &state, &status).is_err());
        assert_eq!(
            submit(&admission, &state, pass_answer(pending(&state))),
            Err(crate::BasicLandTransitionError::TurnProgressUnsupported)
        );
    }

    #[test]
    fn a_creature_only_the_other_player_controls_does_not_stop_the_declaration() {
        let (admission, state) = game_with_creature(3, P2);
        let state = pass_until(&admission, state, at_attacker_declaration);
        let status = EpisodeStatus::Running;
        assert!(validate_magic_pending_request(&admission, &state, &status).is_ok());
        let after = pass(&admission, &state).0;
        assert!(after
            .combat
            .as_ref()
            .is_some_and(|combat| combat.attackers.is_empty()));

        // On its controller's own turn the declaration fails closed.
        let state = pass_until(&admission, after, at(BEGIN_COMBAT, 2));
        let state = pass(&admission, &state).0;
        assert_eq!(state.core.active_player, P2);
        assert_eq!(
            submit(&admission, &state, pass_answer(pending(&state))),
            Err(crate::BasicLandTransitionError::TurnProgressUnsupported)
        );
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
}
