//! Native turn progression for the land-only slice.
//!
//! Passing priority and every turn-based action run directly on V3 state:
//! step changes, untap, draw, the combat skeleton, cleanup and the turn
//! change. One response is one transition (`StateRevision` +1, one
//! `StateDeltaV3`). V3 validates turn-position, priority, active-player and
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
    AuthoritativeCandidateV4, AuthoritativeDecisionRequestV4, CandidateIntentV4, DecisionAnswerV2,
    DecisionDomainV2, DecisionPurposeV4, DecisionResponseV3, DecisionVisibility,
    EngineCandidateBindingV4,
};
use mtgml_model::{
    DecisionId, EpisodeStatus, GameObjectId, PlayerDecisionIdV1, PlayerId, PlayerOutcome,
    PlayerResult, RuleEventId, StateRevision, TerminalReason, ZoneKind,
};
use mtgml_state::{
    BeginningStep, CombatState, CombatStep, EndingStep, EngineState, EngineStatePartsV3,
    ManaPoolChangeCauseV1, PerspectiveLifecycleAuditV1, PriorityState, SbaSelectedActionV1,
    SemanticDeltaOperationV3, StateDeltaV3, TurnHistoryStateV1, TurnPosition, VisibilityPartition,
    ZoneKey, ZoneLocation, ZonePosition,
};

use crate::{
    AuthoritativeRuleEventKind, AuthoritativeRuleEventKindV3, AuthoritativeRuleEventV3,
    BasicLandCandidateError, BasicLandTransitionError as Error, BasicLandTransitionProductV4,
    SelectedSuccessorDecisionV1,
};

/// Executes one V4 response. Land plays and mana abilities use the
/// basic-land path; passing priority and declaring attackers run the turn
/// progression.
pub fn execute_magic_response_v4(
    admission: &ExecutableProfileAdmissionV1,
    state: &EngineStatePartsV3,
    actor: PlayerId,
    response: &DecisionResponseV3,
    status: &EpisodeStatus,
) -> Result<BasicLandTransitionProductV4, Error> {
    let request = state
        .execution_v4
        .pending_decision
        .as_ref()
        .ok_or(Error::InvalidSelection)?;
    if request.actor != actor || request.validate_response(response).is_err() {
        return Err(Error::InvalidSelection);
    }
    validate_slice(admission, state)?;
    let answer = match request.purpose {
        DecisionPurposeV4::PriorityAction => {
            match crate::selected_basic_land_action_v4(admission, state, actor, response, status)
                .map_err(|_| Error::InvalidSelection)?
            {
                SelectedSuccessorDecisionV1::MagicAction(_) => {
                    return crate::execute_basic_land_response_v4(
                        admission, state, actor, response, status,
                    );
                }
                SelectedSuccessorDecisionV1::PassPriority => Answer::Pass,
            }
        }
        DecisionPurposeV4::AttackerDeclaration => {
            validate_magic_pending_request_v4(admission, state, status)
                .map_err(|_| Error::InvalidSelection)?;
            match &response.answer {
                DecisionAnswerV2::SelectMany { candidate_ids } if candidate_ids.is_empty() => {
                    Answer::NoAttackers
                }
                // Attacking creatures are outside the land-only slice.
                _ => return Err(Error::TurnProgressUnsupported),
            }
        }
        DecisionPurposeV4::HandSizeDiscard => {
            validate_magic_pending_request_v4(admission, state, status)
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
                Some(EngineCandidateBindingV4::SelectObject { object }) => Answer::Discard(*object),
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
/// windows through the basic-land candidate owner, the attacker declaration
/// against the request this progression creates.
pub fn validate_magic_pending_request_v4(
    admission: &ExecutableProfileAdmissionV1,
    state: &EngineStatePartsV3,
    status: &EpisodeStatus,
) -> Result<(), BasicLandCandidateError> {
    if !hands_within_slice(state) {
        return Err(BasicLandCandidateError::InvalidState);
    }
    let Some(request) = state
        .execution_v4
        .pending_decision
        .as_ref()
        .filter(|request| {
            matches!(
                request.purpose,
                DecisionPurposeV4::AttackerDeclaration | DecisionPurposeV4::HandSizeDiscard
            )
        })
    else {
        return crate::validate_basic_land_pending_request_v4(admission, state, status);
    };
    if request.purpose == DecisionPurposeV4::HandSizeDiscard {
        return validate_discard_request(admission, state, request, status);
    }
    state
        .validate_structure()
        .map_err(|_| BasicLandCandidateError::InvalidState)?;
    validate_slice(admission, state).map_err(|_| BasicLandCandidateError::InvalidState)?;
    // Only an admission with the rule that creates this request accepts it.
    admits(admission, "rules/declare-attackers")
        .map_err(|_| BasicLandCandidateError::PendingCandidateSetMismatch)?;
    let parts = &state.predecessor_v5;
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

enum NextDecision {
    Priority(PlayerId),
    Attackers,
    Discard,
    /// The game ended: `loser` lost to a state-based action.
    GameOver {
        loser: PlayerId,
    },
}

/// What changed during the transition beyond the endpoint fields.
#[derive(Default)]
struct Facts {
    untapped: Option<Vec<GameObjectId>>,
    attackers_declared: bool,
    combat_skipped: bool,
    combat_ended: bool,
    zone_events: Vec<crate::AuthoritativeRuleEvent>,
}

/// The land-only slice (D13): exactly two players, only admitted basic lands
/// on the battlefield, and none of the state no rule of this slice can
/// evaluate. Under it no state-based action can apply.
fn validate_slice(
    admission: &ExecutableProfileAdmissionV1,
    state: &EngineStatePartsV3,
) -> Result<(), Error> {
    let parts = &state.predecessor_v5;
    let execution = &state.execution_v4;
    let cards = &state.card_rules_state;
    if parts.core.players.len() != 2
        || !parts.foundation_sources.is_empty()
        || cards.counters != Default::default()
        || cards.attachments != Default::default()
        || !execution.continuations.is_empty()
        || !execution.effects.is_empty()
        || !execution.waiting_triggers.is_empty()
        || !execution.delayed_effects.is_empty()
        || !parts.zones.stack_order.is_empty()
        || !parts.zones.stack_records.is_empty()
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
    let battlefield: Vec<GameObjectId> = parts
        .zones
        .locations
        .iter()
        .filter(|(_, location)| location.zone == ZoneKind::Battlefield)
        .map(|(object, _)| *object)
        .collect();
    crate::S1QueryAuthority::for_objects(admission, state, &battlefield)
        .map_err(|_| Error::TurnProgressUnsupported)?;
    Ok(())
}

/// One draw per turn (CR 504.1) and the discard to maximum hand size at each
/// cleanup (CR 514.1) keep the non-active player at seven cards or fewer and
/// the active player at seven before their draw and eight after it. A larger
/// hand could only reach a cleanup with several simultaneous discards, which
/// need the owner's graveyard order that this slice does not offer.
fn hands_within_slice(state: &EngineStatePartsV3) -> bool {
    let core = &state.predecessor_v5.core;
    let before_draw = matches!(
        core.position,
        TurnPosition::Beginning {
            step: BeginningStep::Untap | BeginningStep::Upkeep
        }
    );
    core.players.keys().all(|player| {
        let hand = state
            .predecessor_v5
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

fn admits(admission: &ExecutableProfileAdmissionV1, key: &str) -> Result<(), Error> {
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

fn other_player(state: &EngineStatePartsV3) -> Result<PlayerId, Error> {
    let core = &state.predecessor_v5.core;
    core.players
        .keys()
        .copied()
        .find(|player| *player != core.active_player)
        .ok_or(Error::InvalidResult)
}

fn progress(
    admission: &ExecutableProfileAdmissionV1,
    before: &EngineStatePartsV3,
    request: &AuthoritativeDecisionRequestV4,
    answer: Answer,
) -> Result<BasicLandTransitionProductV4, Error> {
    let mut next = before.clone();
    next.execution_v4.pending_decision = None;
    next.predecessor_v5.revision = StateRevision(
        before
            .predecessor_v5
            .revision
            .0
            .checked_add(1)
            .ok_or(Error::IdentityExhausted)?,
    );
    let active = next.predecessor_v5.core.active_player;
    let other = other_player(&next)?;
    let mut facts = Facts::default();
    let next_decision = match answer {
        // CR 117.3d, 117.4: priority passes to the next player; when all
        // players pass in succession the step ends. An action resets the
        // succession (see the basic-land path).
        Answer::Pass => match next.predecessor_v5.core.priority {
            PriorityState::HeldBy {
                player,
                consecutive_passes: 0,
            } if player == request.actor => {
                let receiver = if player == active { other } else { active };
                next.predecessor_v5.core.priority = PriorityState::HeldBy {
                    player: receiver,
                    consecutive_passes: 1,
                };
                NextDecision::Priority(receiver)
            }
            PriorityState::HeldBy {
                player,
                consecutive_passes: 1,
            } if player == request.actor => {
                next.predecessor_v5.core.priority = PriorityState::None;
                advance(admission, &mut next, &mut facts)?
            }
            _ => return Err(Error::TurnProgressUnsupported),
        },
        // CR 508.1, 508.2: no attackers are declared; the active player then
        // receives priority in the declare attackers step.
        Answer::NoAttackers => {
            next.predecessor_v5.combat = Some(CombatState {
                defending_player: other,
                attackers: Vec::new(),
                damage_step_completed: false,
                blocked_attackers: Default::default(),
                blockers: Default::default(),
            });
            facts.attackers_declared = true;
            next.predecessor_v5.core.priority = PriorityState::HeldBy {
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

/// Ends the current step and performs turn-based actions until a step in
/// which a player must act (CR 500.2, 500.3).
fn advance(
    admission: &ExecutableProfileAdmissionV1,
    next: &mut EngineStatePartsV3,
    facts: &mut Facts,
) -> Result<NextDecision, Error> {
    loop {
        let from = next.predecessor_v5.core.position;
        if from
            == (TurnPosition::Combat {
                step: CombatStep::EndOfCombat,
            })
        {
            // CR 511.3: creatures stop being attacking at end of combat.
            next.predecessor_v5.combat = None;
            facts.combat_ended = true;
        }
        let mut to = crate::temporal_successor(from);
        // CR 508.8: with no attackers, skip declare blockers and damage.
        if to
            == (TurnPosition::Combat {
                step: CombatStep::DeclareBlockers,
            })
            && next
                .predecessor_v5
                .combat
                .as_ref()
                .is_some_and(|combat| combat.attackers.is_empty())
        {
            to = TurnPosition::Combat {
                step: CombatStep::EndOfCombat,
            };
            facts.combat_skipped = true;
        }
        next.predecessor_v5.core.position = to;
        let active = next.predecessor_v5.core.active_player;
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
                // CR 103.8a: the starting player skips the draw of turn 1.
                if next.predecessor_v5.core.turn_number >= 2 {
                    if library_top(next, active).is_none() {
                        // CR 121.4, 704.5b: drawing from an empty library
                        // loses the game when state-based actions are next
                        // checked, before anyone receives priority (CR 117.5).
                        admits(admission, "rules/state-based-actions-empty-library")?;
                        next.predecessor_v5
                            .core
                            .players
                            .get_mut(&active)
                            .ok_or(Error::InvalidResult)?
                            .has_lost = true;
                        return Ok(NextDecision::GameOver { loser: active });
                    }
                    draw(next, active, facts)?;
                }
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
                    .predecessor_v5
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

fn open_priority(next: &mut EngineStatePartsV3) -> NextDecision {
    let active = next.predecessor_v5.core.active_player;
    next.predecessor_v5.core.priority = PriorityState::HeldBy {
        player: active,
        consecutive_passes: 0,
    };
    NextDecision::Priority(active)
}

fn begin_turn(next: &mut EngineStatePartsV3, facts: &mut Facts) -> Result<(), Error> {
    let new_active = other_player(next)?;
    let core = &mut next.predecessor_v5.core;
    core.active_player = new_active;
    core.turn_number = core
        .turn_number
        .checked_add(1)
        .ok_or(Error::IdentityExhausted)?;
    let turn_number = core.turn_number;
    let engine: EngineState = next.predecessor_v5.clone().into();
    let snapshots =
        crate::snapshots::object_snapshots(&engine).map_err(|_| Error::InvalidResult)?;
    let affected =
        crate::turn_structure::derive_ordinary_untap_affected_objects(&snapshots, new_active);
    for object in &affected {
        next.predecessor_v5
            .zones
            .objects
            .get_mut(object)
            .ok_or(Error::InvalidResult)?
            .tapped = false;
    }
    facts.untapped = Some(affected);
    let history = &mut next.card_rules_state.turn_history;
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

fn library_top(state: &EngineStatePartsV3, owner: PlayerId) -> Option<GameObjectId> {
    let library = ZoneLocation {
        zone: ZoneKind::Library,
        player: Some(owner),
        position: ZonePosition::Top { offset: 0 },
        visibility: VisibilityPartition::FaceDown,
        partition: None,
    };
    let key: ZoneKey = library.key();
    state
        .predecessor_v5
        .zones
        .ordered_zones
        .get(&key)
        .and_then(|objects| objects.first())
        .copied()
}

/// CR 504.1: the active player draws the top card of their library. The
/// caller handles an empty library.
fn draw(next: &mut EngineStatePartsV3, owner: PlayerId, facts: &mut Facts) -> Result<(), Error> {
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
    )
}

/// Moves one card through the shared zone-incarnation authority (new
/// incarnation, knowledge and identity updates) and carries its face over.
fn move_card(
    next: &mut EngineStatePartsV3,
    object: GameObjectId,
    kind: crate::zone_incarnation::SelectedZoneTransitionKind,
    claimed_to: ZoneLocation,
    facts: &mut Facts,
) -> Result<(), Error> {
    let claimed_from = next
        .predecessor_v5
        .zones
        .locations
        .get(&object)
        .cloned()
        .ok_or(Error::InvalidResult)?;
    let mut engine: EngineState = next.predecessor_v5.clone().into();
    let mut events = Vec::new();
    crate::zone_incarnation::apply_selected_zone_transition_in_workspace(
        &mut engine,
        &crate::zone_incarnation::SelectedZoneTransitionRequest {
            object,
            kind,
            claimed_from,
            claimed_to,
        },
        // Placeholder origin: `finish` numbers every event of the transition.
        RuleEventId(1),
        &mut events,
    )
    .map_err(|_| Error::TurnProgressUnsupported)?;
    let mut parts = engine.parts();
    parts.execution = Default::default();
    // The new incarnation shows the same face as the card it came from.
    for event in &events {
        if let AuthoritativeRuleEventKind::ZoneTransition { transition } = &event.event {
            let face = next
                .card_rules_state
                .faces
                .faces
                .remove(&transition.old_object)
                .ok_or(Error::InvalidResult)?;
            next.card_rules_state
                .faces
                .faces
                .insert(transition.new_object, face);
        }
    }
    next.predecessor_v5 = parts;
    facts.zone_events.extend(events);
    Ok(())
}

/// Emits one net event per changed aspect, installs the next decision and
/// builds the validated V3 product.
fn finish(
    admission: &ExecutableProfileAdmissionV1,
    before: &EngineStatePartsV3,
    answered: &AuthoritativeDecisionRequestV4,
    mut next: EngineStatePartsV3,
    facts: Facts,
    next_decision: NextDecision,
) -> Result<BasicLandTransitionProductV4, Error> {
    enum Pending {
        Kind(Box<AuthoritativeRuleEventKindV3>),
        Occurrence {
            lifecycle: PerspectiveLifecycleAuditV1,
            source: usize,
        },
    }
    let legacy = |event: AuthoritativeRuleEventKind| {
        Pending::Kind(Box::new(AuthoritativeRuleEventKindV3::Existing {
            event: Box::new(event),
        }))
    };
    let old = &before.predecessor_v5.core;
    let new = next.predecessor_v5.core.clone();
    let mut pending = vec![legacy(AuthoritativeRuleEventKind::DecisionCleared {
        decision: answered.decision_id,
    })];
    if old.priority != new.priority {
        pending.push(legacy(AuthoritativeRuleEventKind::PriorityChanged {
            from: old.priority,
            to: new.priority,
        }));
    }
    if old.position != new.position {
        pending.push(legacy(AuthoritativeRuleEventKind::TurnPositionChanged {
            from: old.position,
            to: new.position,
        }));
    }
    if old.active_player != new.active_player {
        pending.push(legacy(AuthoritativeRuleEventKind::ActivePlayerChanged {
            from: old.active_player,
            to: new.active_player,
        }));
    }
    if old.turn_number != new.turn_number {
        pending.push(legacy(AuthoritativeRuleEventKind::TurnNumberChanged {
            from: old.turn_number,
            to: new.turn_number,
        }));
    }
    if let Some(affected_objects) = facts.untapped {
        pending.push(legacy(AuthoritativeRuleEventKind::UntapCompleted {
            affected_objects,
        }));
    }
    if facts.attackers_declared {
        let combat = next
            .predecessor_v5
            .combat
            .as_ref()
            .ok_or(Error::InvalidResult)?;
        pending.push(legacy(AuthoritativeRuleEventKind::AttackersDeclared {
            defending_player: combat.defending_player,
            attackers: combat.attackers.clone(),
        }));
    }
    if facts.combat_skipped {
        pending.push(legacy(AuthoritativeRuleEventKind::EmptyCombatStepsSkipped));
    }
    if facts.combat_ended {
        pending.push(legacy(AuthoritativeRuleEventKind::CombatEnded));
    }
    let mut transition_index = None;
    for event in facts.zone_events {
        match event.event {
            AuthoritativeRuleEventKind::PerspectiveOccurrence { lifecycle, .. } => {
                pending.push(Pending::Occurrence {
                    lifecycle,
                    source: transition_index.ok_or(Error::InvalidResult)?,
                });
            }
            other => {
                transition_index = Some(pending.len());
                pending.push(legacy(other));
            }
        }
    }
    // CR 500.4: mana empties from each player's pool at the end of each
    // step. Pools are public: every player observes each change.
    if old.position != new.position {
        let pools = next.card_rules_state.mana.pools.clone();
        for (player, pool) in pools {
            if pool == Default::default() {
                continue;
            }
            next.card_rules_state
                .mana
                .pools
                .insert(player, Default::default());
            let source = pending.len();
            pending.push(Pending::Kind(Box::new(
                AuthoritativeRuleEventKindV3::ManaPoolChanged {
                    player,
                    before: pool,
                    after: Default::default(),
                    cause: ManaPoolChangeCauseV1::Emptied,
                },
            )));
            let perspectives: Vec<PlayerId> =
                next.predecessor_v5.core.players.keys().copied().collect();
            for perspective in perspectives {
                let lifecycle = PerspectiveLifecycleAuditV1 {
                    perspective,
                    sequence: next
                        .predecessor_v5
                        .knowledge
                        .players
                        .get(&perspective)
                        .ok_or(Error::InvalidResult)?
                        .next_visible_sequence,
                    mutation: Default::default(),
                };
                let mut engine: EngineState = next.predecessor_v5.clone().into();
                mtgml_state::apply_perspective_lifecycle(&mut engine, &lifecycle)
                    .map_err(|_| Error::InvalidResult)?;
                let mut parts = engine.parts();
                parts.execution = Default::default();
                next.predecessor_v5 = parts;
                pending.push(Pending::Occurrence { lifecycle, source });
            }
        }
    }
    let running = EpisodeStatus::Running;
    let (status, request) = match next_decision {
        NextDecision::Priority(actor) => (
            running.clone(),
            Some(
                crate::install_basic_land_request_v4(admission, &mut next, actor, &running)
                    .map_err(|_| Error::InvalidResult)?,
            ),
        ),
        NextDecision::Attackers => (running, Some(install_attacker_request(&mut next)?)),
        NextDecision::Discard => (running, Some(install_discard_request(&mut next)?)),
        // CR 104.2a: in a two-player game the other player wins.
        NextDecision::GameOver { loser } => {
            pending.push(legacy(
                AuthoritativeRuleEventKind::StateBasedActionsApplied {
                    actions: vec![SbaSelectedActionV1::PlayerLoses { player: loser }],
                },
            ));
            let players = next
                .predecessor_v5
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
        pending.push(legacy(AuthoritativeRuleEventKind::DecisionCreated {
            decision: request.decision_id,
        }));
    }

    let revision = next.predecessor_v5.revision;
    let first = before.predecessor_v5.allocators.next_rule_event_id;
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
                AuthoritativeRuleEventKindV3::PerspectiveObservationOccurrence {
                    lifecycle: Box::new(lifecycle),
                    source_event_id: RuleEventId(first.0 + source as u64),
                }
            }
        };
        events.push(AuthoritativeRuleEventV3 {
            event_id,
            state_revision: revision,
            event,
        });
    }
    next.predecessor_v5.allocators.next_rule_event_id = RuleEventId(
        first
            .0
            .checked_add(events.len() as u64)
            .ok_or(Error::IdentityExhausted)?,
    );
    let mut operations: Vec<SemanticDeltaOperationV3> = events
        .iter()
        .flat_map(|event| event.event.semantic_operations())
        .collect();
    operations.push(SemanticDeltaOperationV3::PendingRequestChanged {
        from: Some(Box::new(answered.clone())),
        to: request.clone().map(Box::new),
    });
    next.validate_structure()
        .map_err(|_| Error::InvalidResult)?;
    let delta = StateDeltaV3::between_structural_only(before, &next, operations)
        .map_err(|_| Error::Delta)?;
    crate::events_v3::validate_events_for_built_delta_v3(before, &next, &events, &delta)
        .map_err(|_| Error::InvalidResult)?;
    Ok(BasicLandTransitionProductV4 {
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
fn discard_candidates(state: &EngineStatePartsV3) -> Result<Vec<AuthoritativeCandidateV4>, Error> {
    let parts = &state.predecessor_v5;
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
        .map(|(index, (opaque, object))| AuthoritativeCandidateV4 {
            candidate_id: mtgml_model::CandidateIdV1(index as u32),
            visible_intent: CandidateIntentV4::SelectObject { object: opaque },
            trusted_binding: EngineCandidateBindingV4::SelectObject { object },
        })
        .collect())
}

/// CR 514.1: the active player chooses the card to discard.
fn install_discard_request(
    next: &mut EngineStatePartsV3,
) -> Result<AuthoritativeDecisionRequestV4, Error> {
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
    state: &EngineStatePartsV3,
    request: &AuthoritativeDecisionRequestV4,
    status: &EpisodeStatus,
) -> Result<(), BasicLandCandidateError> {
    state
        .validate_structure()
        .map_err(|_| BasicLandCandidateError::InvalidState)?;
    validate_slice(admission, state).map_err(|_| BasicLandCandidateError::InvalidState)?;
    // Only an admission with the rule that creates this request accepts it.
    admits(admission, "rules/cleanup-reset")
        .map_err(|_| BasicLandCandidateError::PendingCandidateSetMismatch)?;
    let parts = &state.predecessor_v5;
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

/// Identity, sequence and visibility fields every actor-only request shares.
fn actor_only_request_matches(
    state: &EngineStatePartsV3,
    request: &AuthoritativeDecisionRequestV4,
) -> bool {
    let parts = &state.predecessor_v5;
    let Some(knowledge) = parts.knowledge.players.get(&request.actor) else {
        return false;
    };
    let Some(identity) = parts.perspective_identities.players.get(&request.actor) else {
        return false;
    };
    request.actor == parts.core.active_player
        && request.decision_id.0.checked_add(1) == Some(parts.allocators.next_decision_id.0)
        && request.player_decision_id.0.checked_add(1) == Some(identity.next_player_decision_id.0)
        && request.state_revision == parts.revision
        && request.view_sequence == knowledge.next_visible_sequence
        && request.visibility == DecisionVisibility::ActingPlayerOnly
        && request.parent_player_decision_id.is_none()
        && request.continuation_id.is_none()
        && request.project_player_request().is_ok()
}

/// CR 508.1: the active player declares attackers. The land-only slice has
/// no creatures, so the request offers no candidates.
fn install_attacker_request(
    next: &mut EngineStatePartsV3,
) -> Result<AuthoritativeDecisionRequestV4, Error> {
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
    next: &mut EngineStatePartsV3,
    purpose: DecisionPurposeV4,
    decision_domain_v2: DecisionDomainV2,
    candidates: Vec<AuthoritativeCandidateV4>,
) -> Result<AuthoritativeDecisionRequestV4, Error> {
    let parts = &mut next.predecessor_v5;
    let actor = parts.core.active_player;
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
    let request = AuthoritativeDecisionRequestV4 {
        decision_id,
        player_decision_id,
        state_revision: parts.revision,
        view_sequence,
        actor,
        visibility: DecisionVisibility::ActingPlayerOnly,
        decision_domain_v2,
        purpose,
        parent_player_decision_id: None,
        continuation_id: None,
        candidates,
    };
    // A zero-candidate attacker declaration must be representable too.
    request
        .project_player_request()
        .map_err(|_| Error::TurnProgressUnsupported)?;
    next.execution_v4.pending_decision = Some(request.clone());
    Ok(request)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mtgml_card_ir::ExecutableProfileAdmissionV1;
    use mtgml_decision::{
        AuthoritativeDecisionRequestV4, DecisionAnswerV2, DecisionDomainV2, DecisionPurposeV4,
        DecisionResponseV3, EngineCandidateBindingV4, DECISION_RESPONSE_V3_SCHEMA,
    };
    use mtgml_model::{EpisodeStatus, GameObjectId, PhysicalCardId, PlayerId, ZoneKind};
    use mtgml_state::{
        BeginningStep, CombatStep, EndingStep, EngineStatePartsV3, GameObject, PriorityState,
        TurnPosition, VisibilityPartition, ZoneLocation, ZonePosition,
    };

    pub(super) const P1: PlayerId = PlayerId(1);
    pub(super) const P2: PlayerId = PlayerId(2);

    /// The synthetic reset puts a face-down (morph-like) card on top of P2's
    /// library; the draw profile admits only ordinary face-up cards.
    pub(super) fn make_synthetic_library_card_ordinary(state: &mut EngineStatePartsV3) {
        state
            .predecessor_v5
            .zones
            .objects
            .get_mut(&GameObjectId(2))
            .unwrap()
            .face_down = false;
    }

    pub(super) fn add_library_cards(state: &mut EngineStatePartsV3, owner: PlayerId, count: u64) {
        let definition = crate::basic_land::s1_b_state_with_two_lands_fixture()
            .predecessor_v5
            .zones
            .objects
            .values()
            .next()
            .unwrap()
            .card_definition;
        for _ in 0..count {
            let id = state.predecessor_v5.allocators.next_object_id;
            state.predecessor_v5.allocators.next_object_id = GameObjectId(id.0 + 1);
            let base = ZoneLocation {
                zone: ZoneKind::Library,
                player: Some(owner),
                position: ZonePosition::Top { offset: 0 },
                visibility: VisibilityPartition::FaceDown,
                partition: None,
            };
            let order = state
                .predecessor_v5
                .zones
                .ordered_zones
                .entry(base.key())
                .or_default();
            let location = ZoneLocation {
                position: ZonePosition::Top {
                    offset: order.len() as u32,
                },
                ..base
            };
            order.push(id);
            state.predecessor_v5.zones.objects.insert(
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
            state.predecessor_v5.zones.locations.insert(id, location);
            state.card_rules_state.faces.faces.insert(id, 0);
        }
    }

    /// Adds cards to `owner`'s hand that only the owner tracks, with the
    /// owner's knowledge record, as a real game's opening hand has.
    pub(super) fn add_hand_cards(state: &mut EngineStatePartsV3, owner: PlayerId, count: u64) {
        let definition = state.predecessor_v5.zones.objects[&GameObjectId(2)].card_definition;
        for _ in 0..count {
            let parts = &mut state.predecessor_v5;
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
            state.card_rules_state.faces.faces.insert(id, 0);
        }
    }

    /// Two players, P1 active with priority in precombat main, Mountain and
    /// Plains in P1's hand, one untapped Mountain with its mana ability on
    /// P1's battlefield and `library` face-down cards in each library.
    fn game_with(
        admission: ExecutableProfileAdmissionV1,
        library: u64,
    ) -> (ExecutableProfileAdmissionV1, EngineStatePartsV3) {
        game_with_hands(admission, library, 0, 0)
    }

    /// As `game_with`, with extra owner-tracked cards in each hand.
    fn game_with_hands(
        admission: ExecutableProfileAdmissionV1,
        library: u64,
        p1_extra: u64,
        p2_extra: u64,
    ) -> (ExecutableProfileAdmissionV1, EngineStatePartsV3) {
        let v2 = crate::basic_land::s1_b_state_with_two_lands_fixture();
        let mut state =
            EngineStatePartsV3::new(v2.predecessor_v5, Default::default(), v2.card_rules_state)
                .unwrap();
        make_synthetic_library_card_ordinary(&mut state);
        add_library_cards(&mut state, P1, library);
        add_library_cards(&mut state, P2, library);
        add_hand_cards(&mut state, P1, p1_extra);
        add_hand_cards(&mut state, P2, p2_extra);
        crate::install_basic_land_request_v4(&admission, &mut state, P1, &EpisodeStatus::Running)
            .unwrap();
        (admission, state)
    }

    fn game(library: u64) -> (ExecutableProfileAdmissionV1, EngineStatePartsV3) {
        game_with(crate::basic_land::basic_land_admission_fixture(), library)
    }

    pub(super) fn pending(state: &EngineStatePartsV3) -> &AuthoritativeDecisionRequestV4 {
        state.execution_v4.pending_decision.as_ref().unwrap()
    }

    fn candidate(
        request: &AuthoritativeDecisionRequestV4,
        pick: impl Fn(&EngineCandidateBindingV4) -> bool,
    ) -> Option<mtgml_model::CandidateIdV1> {
        request
            .candidates
            .iter()
            .find(|candidate| pick(&candidate.trusted_binding))
            .map(|candidate| candidate.candidate_id)
    }

    pub(super) fn submit(
        admission: &ExecutableProfileAdmissionV1,
        state: &EngineStatePartsV3,
        answer: DecisionAnswerV2,
    ) -> Result<crate::BasicLandTransitionProductV4, crate::BasicLandTransitionError> {
        let request = pending(state);
        let response = DecisionResponseV3 {
            schema_version: DECISION_RESPONSE_V3_SCHEMA.to_owned(),
            player_decision_id: request.player_decision_id,
            view_sequence: request.view_sequence,
            answer,
        };
        execute_magic_response_v4(
            admission,
            state,
            request.actor,
            &response,
            &EpisodeStatus::Running,
        )
    }

    /// Checks a product is a complete, valid V3 step and returns its state.
    pub(super) fn apply(
        before: &EngineStatePartsV3,
        product: &crate::BasicLandTransitionProductV4,
    ) -> EngineStatePartsV3 {
        assert!(product.accepted);
        assert_eq!(
            product.next_state.predecessor_v5.revision.0,
            before.predecessor_v5.revision.0 + 1
        );
        crate::events_v3::validate_events_for_built_delta_v3(
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
            product.next_state.execution_v4.pending_decision,
            product.next_decision
        );
        product.next_state.clone()
    }

    pub(super) fn pass_answer(request: &AuthoritativeDecisionRequestV4) -> DecisionAnswerV2 {
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
                    matches!(binding, EngineCandidateBindingV4::PassPriority)
                })
                .unwrap(),
            }
        }
    }

    pub(super) fn pass(
        admission: &ExecutableProfileAdmissionV1,
        state: &EngineStatePartsV3,
    ) -> (EngineStatePartsV3, crate::BasicLandTransitionProductV4) {
        let product = submit(admission, state, pass_answer(pending(state))).unwrap();
        (apply(state, &product), product)
    }

    fn take_action(
        admission: &ExecutableProfileAdmissionV1,
        state: &EngineStatePartsV3,
        pick: impl Fn(&EngineCandidateBindingV4) -> bool,
    ) -> EngineStatePartsV3 {
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
        state: &EngineStatePartsV3,
    ) -> EngineStatePartsV3 {
        take_action(admission, state, |binding| {
            matches!(binding, EngineCandidateBindingV4::PlayLand { .. })
        })
    }

    fn tap_first_mana_source(
        admission: &ExecutableProfileAdmissionV1,
        state: &EngineStatePartsV3,
    ) -> EngineStatePartsV3 {
        take_action(admission, state, |binding| {
            matches!(binding, EngineCandidateBindingV4::ActivateAbility { .. })
        })
    }

    /// Passes (and declares no attackers) until `until` holds.
    pub(super) fn pass_until(
        admission: &ExecutableProfileAdmissionV1,
        mut state: EngineStatePartsV3,
        until: impl Fn(&EngineStatePartsV3) -> bool,
    ) -> EngineStatePartsV3 {
        for _ in 0..200 {
            if until(&state) {
                return state;
            }
            state = pass(admission, &state).0;
        }
        panic!("condition not reached within 200 responses");
    }

    pub(super) fn at(position: TurnPosition, turn: u64) -> impl Fn(&EngineStatePartsV3) -> bool {
        move |state| {
            state.predecessor_v5.core.position == position
                && state.predecessor_v5.core.turn_number == turn
                && state
                    .execution_v4
                    .pending_decision
                    .as_ref()
                    .is_some_and(|request| {
                        request.actor == state.predecessor_v5.core.active_player
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

    pub(super) fn zone_count(state: &EngineStatePartsV3, owner: PlayerId, zone: ZoneKind) -> usize {
        state
            .predecessor_v5
            .zones
            .locations
            .values()
            .filter(|location| location.zone == zone && location.player == Some(owner))
            .count()
    }

    fn battlefield_tapped(state: &EngineStatePartsV3, controller: PlayerId) -> Vec<bool> {
        state
            .predecessor_v5
            .zones
            .objects
            .values()
            .filter(|object| {
                object.controller == controller
                    && state.predecessor_v5.zones.locations[&object.id].zone
                        == ZoneKind::Battlefield
            })
            .map(|object| object.tapped)
            .collect()
    }

    fn mana_source(state: &EngineStatePartsV3) -> GameObjectId {
        state
            .card_rules_state
            .abilities
            .by_instance
            .values()
            .next()
            .unwrap()
            .source
    }

    fn has_play_land(state: &EngineStatePartsV3) -> bool {
        candidate(pending(state), |binding| {
            matches!(binding, EngineCandidateBindingV4::PlayLand { .. })
        })
        .is_some()
    }

    #[test]
    fn first_pass_only_transfers_priority() {
        let (admission, state) = game(3);
        let after = pass(&admission, &state).0;

        assert_eq!(
            after.predecessor_v5.core.position,
            TurnPosition::PrecombatMain
        );
        let request = pending(&after);
        assert_eq!(request.actor, P2);
        assert_eq!(request.purpose, DecisionPurposeV4::PriorityAction);
        assert!(!has_play_land(&after));
    }

    #[test]
    fn built_delta_check_rejects_another_after_state() {
        let (admission, state) = game(3);
        let product = submit(&admission, &state, pass_answer(pending(&state))).unwrap();
        crate::events_v3::validate_events_for_built_delta_v3(
            &state,
            &product.next_state,
            &product.events,
            &product.delta,
        )
        .unwrap();
        let mut other = product.next_state.clone();
        other.predecessor_v5.core.turn_number += 1;
        assert!(crate::events_v3::validate_events_for_built_delta_v3(
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

        assert_eq!(after.predecessor_v5.core.position, BEGIN_COMBAT);
        assert_eq!(pending(&after).actor, P1);
        assert_eq!(pending(&after).purpose, DecisionPurposeV4::PriorityAction);
    }

    #[test]
    fn mana_pools_empty_when_the_step_changes() {
        let (admission, state) = game(3);
        let state = tap_first_mana_source(&admission, &state);
        assert_ne!(state.card_rules_state.mana.pools[&P1], Default::default());
        let state = pass(&admission, &state).0;
        let (after, product) = pass(&admission, &state);

        assert_eq!(after.predecessor_v5.core.position, BEGIN_COMBAT);
        assert_eq!(after.card_rules_state.mana.pools[&P1], Default::default());
        assert!(product.events.iter().any(|event| matches!(
            event.event,
            crate::AuthoritativeRuleEventKindV3::ManaPoolChanged {
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
        assert_eq!(state.predecessor_v5.core.priority, held(P1));
        let state = pass(&admission, &state).0;
        assert_eq!(
            state.predecessor_v5.core.position,
            TurnPosition::PrecombatMain
        );
        assert_eq!(pending(&state).actor, P2);

        let state = play_first_land(&admission, &state);
        assert_eq!(state.predecessor_v5.core.priority, held(P2));
        let state = pass(&admission, &state).0;
        assert_eq!(pending(&state).actor, P1);
        let after = pass(&admission, &state).0;
        assert_eq!(after.predecessor_v5.core.position, BEGIN_COMBAT);
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
    fn empty_attack_declaration_ends_combat_without_damage() {
        let (admission, state) = game(3);
        let state = pass_until(&admission, state, |state| {
            state
                .execution_v4
                .pending_decision
                .as_ref()
                .is_some_and(|request| request.purpose == DecisionPurposeV4::AttackerDeclaration)
        });
        let after = pass(&admission, &state).0;
        // Priority in declare attackers with an empty combat, then
        // end of combat (blockers and damage skipped), then postcombat main.
        assert!(after
            .predecessor_v5
            .combat
            .as_ref()
            .is_some_and(|combat| combat.attackers.is_empty()));
        assert_eq!(
            after.predecessor_v5.core.priority,
            PriorityState::HeldBy {
                player: P1,
                consecutive_passes: 0
            }
        );
        let state = pass(&admission, &after).0;
        let end_of_combat = pass(&admission, &state).0;
        assert_eq!(
            end_of_combat.predecessor_v5.core.position,
            TurnPosition::Combat {
                step: CombatStep::EndOfCombat
            }
        );
        let state = pass(&admission, &end_of_combat).0;
        let postcombat = pass(&admission, &state).0;
        assert_eq!(
            postcombat.predecessor_v5.core.position,
            TurnPosition::PostcombatMain
        );
        assert!(postcombat.predecessor_v5.combat.is_none());
    }

    #[test]
    fn end_step_passes_run_cleanup_and_open_next_upkeep() {
        let (admission, state) = game(3);
        let mountain = mana_source(&state);
        let state = tap_first_mana_source(&admission, &state);
        let state = pass_until(&admission, state, at(END_STEP, 1));
        let state = pass(&admission, &state).0;
        let after = pass(&admission, &state).0;

        let core = &after.predecessor_v5.core;
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
            after.predecessor_v5.zones.objects[&mountain].tapped,
            "only the active player's permanents untap"
        );
        assert_eq!(after.card_rules_state.turn_history.turn_number, 2);
    }

    #[test]
    fn upkeep_passes_draw_one_card_for_the_active_player() {
        let (admission, state) = game(3);
        let state = pass_until(&admission, state, at(UPKEEP, 2));
        let hand_before = zone_count(&state, P2, ZoneKind::Hand);
        let library_before = zone_count(&state, P2, ZoneKind::Library);
        let state = pass(&admission, &state).0;
        let after = pass(&admission, &state).0;

        assert_eq!(after.predecessor_v5.core.position, DRAW);
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
        assert_eq!(state.card_rules_state.turn_history.turn_number, 3);
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
        let core = &after.predecessor_v5.core;
        assert!(core.players[&P1].has_lost && !core.players[&P2].has_lost);
        assert_eq!(core.position, DRAW);
        assert_eq!(core.priority, PriorityState::None);
        assert_eq!(product.next_decision, None);
        assert!(product.events.iter().any(|event| matches!(
            &event.event,
            crate::AuthoritativeRuleEventKindV3::Existing { event }
                if **event == AuthoritativeRuleEventKind::StateBasedActionsApplied {
                    actions: vec![mtgml_state::SbaSelectedActionV1::PlayerLoses { player: P1 }],
                }
        )));
        assert_eq!(
            zone_count(&after, P1, ZoneKind::Hand),
            zone_count(&state, P1, ZoneKind::Hand)
        );
        validate_magic_pending_request_v4(&admission, &after, &product.status).unwrap();
    }

    #[test]
    fn unsupported_state_fails_closed() {
        let (admission, mut state) = game(3);
        let land = *state
            .card_rules_state
            .abilities
            .by_instance
            .values()
            .next()
            .map(|authority| &authority.source)
            .unwrap();
        state.predecessor_v5.foundation_sources.insert(
            land,
            mtgml_state::FoundationCreatureSource {
                source_kind: mtgml_state::FoundationSourceKind::Creature,
                base_characteristics: mtgml_state::BaseCharacteristics::Simple {
                    power: 1,
                    toughness: 1,
                },
                marked_damage: 0,
                control_history: mtgml_state::ControlHistory::BeforeTurnStart { turn_number: 1 },
            },
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
        assert_eq!(state.predecessor_v5.core.active_player, P1);
    }

    fn discard_request(
        admission: &ExecutableProfileAdmissionV1,
        state: EngineStatePartsV3,
    ) -> EngineStatePartsV3 {
        let state = pass_until(admission, state, at(END_STEP, 1));
        let state = pass(admission, &state).0;
        pass(admission, &state).0
    }

    fn discard(
        admission: &ExecutableProfileAdmissionV1,
        state: &EngineStatePartsV3,
        candidate_ids: Vec<mtgml_model::CandidateIdV1>,
    ) -> Result<crate::BasicLandTransitionProductV4, crate::BasicLandTransitionError> {
        submit(
            admission,
            state,
            DecisionAnswerV2::SelectMany { candidate_ids },
        )
    }

    fn last_candidate(state: &EngineStatePartsV3) -> mtgml_model::CandidateIdV1 {
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
            state.predecessor_v5.core.position,
            TurnPosition::Ending {
                step: EndingStep::Cleanup
            }
        );
        assert_eq!(state.predecessor_v5.core.turn_number, 1);
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
            assert!(validate_magic_pending_request_v4(&admission, &state, &status).is_err());
            assert_eq!(
                submit(&admission, &state, pass_answer(pending(&state))),
                Err(crate::BasicLandTransitionError::TurnProgressUnsupported)
            );
        }
        // The active player may hold eight after their draw, the other seven.
        let (_, state) = game_with_hands(admission.clone(), 3, 6, 7);
        assert_eq!(zone_count(&state, P1, ZoneKind::Hand), 8);
        assert_eq!(zone_count(&state, P2, ZoneKind::Hand), 7);
        validate_magic_pending_request_v4(&admission, &state, &status).unwrap();
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
            validate_magic_pending_request_v4(&admission, &state, &status).unwrap();
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
                EngineCandidateBindingV4::SelectObject { object } => object,
                _ => unreachable!(),
            })
            .unwrap();
        let product = discard(&admission, &state, vec![last_candidate(&state)]).unwrap();
        let after = apply(&state, &product);

        assert!(!after.predecessor_v5.zones.objects.contains_key(&chosen));
        assert_eq!(zone_count(&after, P1, ZoneKind::Graveyard), 1);
        assert_eq!(zone_count(&after, P1, ZoneKind::Hand), 7);
        assert_eq!(after.predecessor_v5.core.turn_number, 2);
        assert_eq!(after.predecessor_v5.core.active_player, P2);
        assert_eq!(after.predecessor_v5.core.position, UPKEEP);
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
        assert_eq!(after.predecessor_v5.core.turn_number, 4);
    }
}
