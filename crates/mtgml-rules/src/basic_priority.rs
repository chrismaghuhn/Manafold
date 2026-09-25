//! Closed two-player, pass-only Basic Priority semantics.

use mtgml_decision::{
    AuthoritativeDecisionRequestV2, CandidateIntent, CandidateOrderingV1, DecisionDomainV2,
    DecisionVisibility, EngineCandidateBinding,
};
use mtgml_model::{DecisionId, PlayerId, RuleEventId, StateRevision};
use mtgml_state::{
    validate_engine_state, EngineState, PendingDecisionRecordV2, PriorityState, TurnPosition,
};

use crate::decision_stage::{advance_player_allocator, fresh_stage_identity};
use crate::errors::KernelExecutionError;
use crate::events::{AuthoritativeRuleEvent, AuthoritativeRuleEventKind};
use crate::product::build_accepted_product;
use crate::transition::TransitionResult;
use crate::validation::TransitionViolation;

pub(crate) fn is_priority_bearing_position(position: TurnPosition) -> bool {
    matches!(
        position,
        TurnPosition::Beginning {
            step: mtgml_state::BeginningStep::Upkeep
        } | TurnPosition::PrecombatMain
            | TurnPosition::Beginning {
                step: mtgml_state::BeginningStep::Draw
            }
            | TurnPosition::PostcombatMain
            | TurnPosition::Ending {
                step: mtgml_state::EndingStep::EndStep
            }
    )
}

fn declared_combat_state_supported(state: &EngineState) -> bool {
    let Some(combat) = state.combat.as_ref() else {
        return false;
    };
    let Some(defending_player) = state
        .core
        .players
        .keys()
        .copied()
        .find(|player| *player != state.core.active_player)
    else {
        return false;
    };
    combat.defending_player == defending_player
        && combat.blocked_attackers.is_empty()
        && combat.attackers.windows(2).all(|pair| pair[0] < pair[1])
        && combat.blockers.len() == combat.attackers.len()
        && combat.attackers.iter().all(|attacker| {
            if combat.blockers.get(attacker) != Some(&None) {
                return false;
            }
            let Some(object) = state.zones.objects.get(attacker) else {
                return false;
            };
            let Some(location) = state.zones.locations.get(attacker) else {
                return false;
            };
            let Some(source) = state.foundation_sources.get(attacker) else {
                return false;
            };
            let controlled_in_time = match source.control_history {
                mtgml_state::ControlHistory::BeforeTurnStart { turn_number } => {
                    turn_number <= state.core.turn_number
                }
                mtgml_state::ControlHistory::DuringTurn { turn_number, .. } => {
                    turn_number < state.core.turn_number
                }
            };
            location.zone == mtgml_model::ZoneKind::Battlefield
                && object.controller == state.core.active_player
                && object.tapped
                && !object.face_down
                && source.source_kind == mtgml_state::FoundationSourceKind::Creature
                && matches!(
                    source.base_characteristics,
                    mtgml_state::BaseCharacteristics::Simple { .. }
                )
                && controlled_in_time
        })
}

/// Proves the currently representable pass-only state from authoritative
/// state. State-based-action semantics remain owned by their rules authority.
pub(crate) fn validate_pass_only_state(
    state: &EngineState,
    require_held_decision: bool,
) -> Result<(), KernelExecutionError> {
    validate_pass_only_state_with_combat(state, require_held_decision, false)
}

pub(crate) fn validate_pass_only_state_with_combat(
    state: &EngineState,
    require_held_decision: bool,
    combat_enabled: bool,
) -> Result<(), KernelExecutionError> {
    validate_pass_only_state_with_combat_and_blockers(
        state,
        require_held_decision,
        combat_enabled,
        false,
        false,
    )
}

pub(crate) fn validate_pass_only_state_with_blockers(
    state: &EngineState,
    require_held_decision: bool,
) -> Result<(), KernelExecutionError> {
    validate_pass_only_state_with_combat_and_blockers(
        state,
        require_held_decision,
        true,
        true,
        false,
    )
}

pub(crate) fn validate_pass_only_state_with_combat_damage(
    state: &EngineState,
    require_held_decision: bool,
) -> Result<(), KernelExecutionError> {
    validate_pass_only_state_with_combat_and_blockers(
        state,
        require_held_decision,
        true,
        true,
        true,
    )
}

fn validate_pass_only_state_with_combat_and_blockers(
    state: &EngineState,
    require_held_decision: bool,
    combat_enabled: bool,
    blockers_enabled: bool,
    damage_enabled: bool,
) -> Result<(), KernelExecutionError> {
    validate_engine_state(state).map_err(KernelExecutionError::BeforeState)?;
    let combat_position = matches!(
        state.core.position,
        TurnPosition::Combat {
            step: mtgml_state::CombatStep::BeginningOfCombat
                | mtgml_state::CombatStep::DeclareAttackers
                | mtgml_state::CombatStep::EndOfCombat
        }
    ) || (blockers_enabled
        && state.core.position
            == (TurnPosition::Combat {
                step: mtgml_state::CombatStep::DeclareBlockers,
            }))
        || (damage_enabled
            && state.core.position
                == (TurnPosition::Combat {
                    step: mtgml_state::CombatStep::CombatDamage,
                }));
    let combat_state_supported = match state.core.position {
        TurnPosition::Combat {
            step: mtgml_state::CombatStep::BeginningOfCombat,
        } => state.combat.is_none(),
        TurnPosition::Combat {
            step: mtgml_state::CombatStep::DeclareAttackers,
        } => state.combat.is_none() || declared_combat_state_supported(state),
        TurnPosition::Combat {
            step: mtgml_state::CombatStep::EndOfCombat,
        } => {
            if damage_enabled {
                crate::combat_damage::validate_combat_damage_state(state).is_ok()
            } else {
                declared_combat_state_supported(state)
                    && state.combat.as_ref().is_some_and(|combat| {
                        combat.attackers.is_empty() && combat.blockers.is_empty()
                    })
            }
        }
        TurnPosition::Combat {
            step: mtgml_state::CombatStep::DeclareBlockers,
        } => {
            blockers_enabled
                && crate::magic::MagicRulesKernel::validate_declared_blocker_combat_runtime_support(
                    state,
                )
                .is_ok()
        }
        TurnPosition::Combat {
            step: mtgml_state::CombatStep::CombatDamage,
        } => damage_enabled && crate::combat_damage::validate_combat_damage_state(state).is_ok(),
        _ => state.combat.is_none(),
    };
    if !(is_priority_bearing_position(state.core.position) || (combat_enabled && combat_position))
        || !state.execution.continuations.is_empty()
        || !combat_state_supported
        || state.core.players.values().any(|player| player.has_lost)
    {
        return Err(KernelExecutionError::UnsupportedStagePath);
    }

    let mut stable = state.clone();
    stable.core.priority = PriorityState::None;
    stable.execution.pending_decision = None;
    crate::state_based_actions::validate_state_based_actions_support_profile(&stable)
        .map_err(|_| KernelExecutionError::UnsupportedStagePath)?;
    let plan = crate::state_based_actions::derive_bounded_sba_round_plan(&stable)
        .map_err(|_| KernelExecutionError::UnsupportedStagePath)?;
    if !plan.selected_sba_actions.is_empty() || !plan.apnap_owners.is_empty() {
        return Err(KernelExecutionError::UnsupportedStagePath);
    }

    match (
        state.core.priority,
        state.execution.pending_decision.as_ref(),
    ) {
        (PriorityState::None, None) if !require_held_decision => Ok(()),
        (
            PriorityState::HeldBy {
                player,
                consecutive_passes,
            },
            Some(pending),
        ) if require_held_decision
            && ((player == state.core.active_player && consecutive_passes == 0)
                || (player != state.core.active_player && consecutive_passes == 1))
            && pending.request.actor == player
            && pending.request.state_revision == state.revision
            && pending.request.visibility == DecisionVisibility::ActingPlayerOnly
            && pending.request.continuation_id.is_none()
            && pending.request.decision == DecisionDomainV2::ChooseOne
            && pending.request.candidates.len() == 1
            && pending.request.candidates[0].candidate_id.0 == 0
            && pending.request.candidates[0].visible_intent == CandidateIntent::PassPriority
            && pending.request.candidates[0].trusted_binding
                == EngineCandidateBinding::PassPriority
            && pending.request.project_player_request().is_ok() =>
        {
            Ok(())
        }
        _ => Err(KernelExecutionError::UnsupportedStagePath),
    }
}

pub(crate) fn open_priority_window(
    state: &EngineState,
) -> Result<TransitionResult, KernelExecutionError> {
    open_priority_window_with_combat(state, false)
}

pub(crate) fn open_combat_priority_window(
    state: &EngineState,
) -> Result<TransitionResult, KernelExecutionError> {
    open_priority_window_with_combat(state, true)
}

pub(crate) fn open_combat_blocker_priority_window(
    state: &EngineState,
) -> Result<TransitionResult, KernelExecutionError> {
    open_priority_window_with_combat_and_blockers(state, true, true, false)
}

pub(crate) fn open_combat_damage_priority_window(
    state: &EngineState,
) -> Result<TransitionResult, KernelExecutionError> {
    open_priority_window_with_combat_and_blockers(state, true, true, true)
}

fn open_priority_window_with_combat(
    state: &EngineState,
    combat_enabled: bool,
) -> Result<TransitionResult, KernelExecutionError> {
    open_priority_window_with_combat_and_blockers(state, combat_enabled, false, false)
}

fn open_priority_window_with_combat_and_blockers(
    state: &EngineState,
    combat_enabled: bool,
    blockers_enabled: bool,
    damage_enabled: bool,
) -> Result<TransitionResult, KernelExecutionError> {
    validate_pass_only_state_with_combat_and_blockers(
        state,
        false,
        combat_enabled,
        blockers_enabled,
        damage_enabled,
    )?;
    let actor = state.core.active_player;
    let identity = fresh_stage_identity(state, actor)?;
    let request = make_pass_request(
        actor,
        identity.revision,
        identity.decision_id,
        identity.player_decision_id,
    )?;
    let next_decision = DecisionId(
        identity
            .decision_id
            .0
            .checked_add(1)
            .ok_or(KernelExecutionError::Exhaustion("decision"))?,
    );
    let event = bound_event(
        state,
        0,
        identity.revision,
        AuthoritativeRuleEventKind::PriorityChanged {
            from: PriorityState::None,
            to: PriorityState::HeldBy {
                player: actor,
                consecutive_passes: 0,
            },
        },
    )?;
    let created = bound_event(
        state,
        1,
        identity.revision,
        AuthoritativeRuleEventKind::DecisionCreated {
            decision: identity.decision_id,
        },
    )?;
    let mut next = state.clone();
    next.revision = identity.revision;
    build_accepted_product(state, next, vec![event, created], |workspace| {
        workspace.core.priority = PriorityState::HeldBy {
            player: actor,
            consecutive_passes: 0,
        };
        workspace.execution.pending_decision = Some(PendingDecisionRecordV2 { request });
        workspace.allocators.next_decision_id = next_decision;
        advance_player_allocator(workspace, actor, identity.player_decision_id)?;
        Ok(())
    })
}

pub(crate) fn validate_priority_transition(
    before: &EngineState,
    result: &TransitionResult,
) -> Result<(), TransitionViolation> {
    if crate::contract::is_bounded_cleanup_to_upkeep_composition(before, result) {
        return Ok(());
    }
    if crate::contract::is_endstep_cleanup_upkeep_composition(before, result) {
        return Ok(());
    }
    if crate::contract::is_second_pass_priority_progress_composition(before, result) {
        return Ok(());
    }
    if validate_second_pass_combat_composition(before, result)? {
        return Ok(());
    }
    let after = &result.next_state;
    let priority_events: Vec<_> = result
        .events
        .iter()
        .filter_map(|event| match &event.event {
            AuthoritativeRuleEventKind::PriorityChanged { from, to } => Some((*from, *to)),
            _ => None,
        })
        .collect();
    if before.core.priority == after.core.priority {
        if !priority_events.is_empty() {
            return Err(TransitionViolation::Priority);
        }
        return Ok(());
    }
    if priority_events.len() != 1
        || priority_events[0] != (before.core.priority, after.core.priority)
    {
        return Err(TransitionViolation::Priority);
    }
    match (before.core.priority, after.core.priority) {
        (
            PriorityState::None,
            PriorityState::HeldBy {
                player,
                consecutive_passes: 0,
            },
        ) => {
            if player != before.core.active_player
                || after.core.position != before.core.position
                || after.execution.pending_decision.is_none()
                || !matches!(
                    result
                        .events
                        .get(result.events.len().saturating_sub(2))
                        .map(|e| &e.event),
                    Some(AuthoritativeRuleEventKind::PriorityChanged { .. })
                )
            {
                return Err(TransitionViolation::Priority);
            }
            validate_priority_endpoint(after).map_err(|_| TransitionViolation::Priority)?;
        }
        (
            PriorityState::HeldBy {
                player: from_player,
                consecutive_passes: 0,
            },
            PriorityState::HeldBy {
                player: to_player,
                consecutive_passes: 1,
            },
        ) => {
            if from_player != before.core.active_player
                || to_player == from_player
                || result.events.len() != 3
                || !matches!(
                    result.events[0].event,
                    AuthoritativeRuleEventKind::DecisionCleared { .. }
                )
                || !matches!(
                    result.events[1].event,
                    AuthoritativeRuleEventKind::PriorityChanged { .. }
                )
                || !matches!(
                    result.events[2].event,
                    AuthoritativeRuleEventKind::DecisionCreated { .. }
                )
                || after.core.position != before.core.position
            {
                return Err(TransitionViolation::Priority);
            }
            validate_priority_endpoint(after).map_err(|_| TransitionViolation::Priority)?;
        }
        (
            PriorityState::HeldBy {
                player: from_player,
                consecutive_passes: 1,
            },
            PriorityState::None,
        ) => {
            let cleanup_composition =
                crate::contract::is_second_pass_cleanup_composition(before, result);
            let empty_attack_skip = before.core.position
                == (TurnPosition::Combat {
                    step: mtgml_state::CombatStep::DeclareAttackers,
                })
                && before
                    .combat
                    .as_ref()
                    .is_some_and(|combat| combat.attackers.is_empty());
            let closes_combat = before.core.position
                == (TurnPosition::Combat {
                    step: mtgml_state::CombatStep::EndOfCombat,
                })
                && before.combat.is_some()
                && after.core.position == TurnPosition::PostcombatMain
                && after.combat.is_none();
            let expected_position = if cleanup_composition {
                TurnPosition::Beginning {
                    step: mtgml_state::BeginningStep::Untap,
                }
            } else if empty_attack_skip {
                TurnPosition::Combat {
                    step: mtgml_state::CombatStep::EndOfCombat,
                }
            } else if closes_combat {
                TurnPosition::PostcombatMain
            } else {
                crate::turn_structure::temporal_successor(before.core.position)
            };
            if from_player == before.core.active_player
                || result.events.len()
                    != if cleanup_composition {
                        6
                    } else if closes_combat {
                        4
                    } else {
                        3
                    }
                || !matches!(
                    result.events[0].event,
                    AuthoritativeRuleEventKind::DecisionCleared { .. }
                )
                || !matches!(
                    result.events[1].event,
                    AuthoritativeRuleEventKind::PriorityChanged { .. }
                )
                || if empty_attack_skip {
                    !matches!(
                        result.events[2].event,
                        AuthoritativeRuleEventKind::EmptyCombatStepsSkipped
                    )
                } else if closes_combat {
                    !matches!(
                        result.events[2].event,
                        AuthoritativeRuleEventKind::CombatEnded
                    ) || !matches!(
                        result.events[3].event,
                        AuthoritativeRuleEventKind::TurnPositionChanged { from, to }
                            if from == before.core.position && to == TurnPosition::PostcombatMain
                    )
                } else {
                    !matches!(
                        result.events[2].event,
                        AuthoritativeRuleEventKind::TurnPositionChanged { from, to }
                            if from == before.core.position
                                && to == crate::turn_structure::temporal_successor(from)
                    )
                }
                || after.core.position != expected_position
                || after.execution.pending_decision.is_some()
            {
                return Err(TransitionViolation::Priority);
            }
        }
        _ => return Err(TransitionViolation::Priority),
    }
    Ok(())
}

fn validate_priority_endpoint(state: &EngineState) -> Result<(), KernelExecutionError> {
    if state.core.position
        == (TurnPosition::Combat {
            step: mtgml_state::CombatStep::DeclareBlockers,
        })
    {
        validate_pass_only_state_with_blockers(state, true)
    } else if matches!(
        state.core.position,
        TurnPosition::Combat {
            step: mtgml_state::CombatStep::CombatDamage | mtgml_state::CombatStep::EndOfCombat
        }
    ) {
        validate_pass_only_state_with_combat_damage(state, true)
    } else if matches!(state.core.position, TurnPosition::Combat { .. }) {
        validate_pass_only_state_with_combat(state, true, true)
    } else {
        validate_pass_only_state(state, true)
    }
}

pub(crate) fn validate_second_pass_combat_composition(
    before: &EngineState,
    result: &TransitionResult,
) -> Result<bool, TransitionViolation> {
    let after = &result.next_state;
    let Some(pending) = before.execution.pending_decision.as_ref() else {
        return Ok(false);
    };
    if !matches!(
        before.core.priority,
        PriorityState::HeldBy {
            player,
            consecutive_passes: 1
        } if player != before.core.active_player && pending.request.actor == player
    ) {
        return Ok(false);
    }
    let closes_blockers_into_damage = before.core.position
        == (TurnPosition::Combat {
            step: mtgml_state::CombatStep::DeclareBlockers,
        })
        && after.core.position
            == (TurnPosition::Combat {
                step: mtgml_state::CombatStep::CombatDamage,
            })
        && before
            .combat
            .as_ref()
            .is_some_and(|combat| !combat.attackers.is_empty());
    if closes_blockers_into_damage {
        let response_only = result.events.len() == 3
            && before.revision.0.checked_add(1) == Some(after.revision.0)
            && after.core.priority == PriorityState::None
            && after.execution.pending_decision.is_none()
            && matches!(result.status, mtgml_model::EpisodeStatus::Running)
            && matches!(
                &result.events[0].event,
                AuthoritativeRuleEventKind::DecisionCleared { decision }
                    if *decision == pending.request.decision_id
            )
            && matches!(
                result.events[1].event,
                AuthoritativeRuleEventKind::PriorityChanged { from, to }
                    if from == before.core.priority && to == PriorityState::None
            )
            && matches!(
                result.events[2].event,
                AuthoritativeRuleEventKind::TurnPositionChanged { from, to }
                    if from == before.core.position && to == after.core.position
            );
        if response_only {
            return Ok(false);
        }
        let terminal = matches!(result.status, mtgml_model::EpisodeStatus::Terminal { .. });
        if result.events.len() < 5
            || before.revision.0.checked_add(2) != Some(after.revision.0)
            || (!matches!(result.status, mtgml_model::EpisodeStatus::Running)
                && (!terminal
                    || after.execution.pending_decision.is_some()
                    || after.core.priority != PriorityState::None))
            || !matches!(
                &result.events[0].event,
                AuthoritativeRuleEventKind::DecisionCleared { decision }
                    if *decision == pending.request.decision_id
            )
            || !matches!(
                result.events[1].event,
                AuthoritativeRuleEventKind::PriorityChanged { from, to }
                    if from == before.core.priority && to == PriorityState::None
            )
            || !matches!(
                result.events[2].event,
                AuthoritativeRuleEventKind::TurnPositionChanged { from, to }
                    if from == before.core.position && to == after.core.position
            )
        {
            return Err(TransitionViolation::Priority);
        }
        if matches!(result.status, mtgml_model::EpisodeStatus::Running) {
            let order_pending = after
                .execution
                .pending_decision
                .as_ref()
                .is_some_and(|pending| {
                    matches!(pending.request.decision, DecisionDomainV2::Order { .. })
                });
            if order_pending {
                if after.core.priority != PriorityState::None {
                    return Err(TransitionViolation::Priority);
                }
            } else {
                validate_pass_only_state_with_combat_damage(after, true)
                    .map_err(|_| TransitionViolation::Priority)?;
            }
        }
        return Ok(true);
    }
    let entering_beginning_of_combat = before.core.position == TurnPosition::PrecombatMain
        && after.core.position
            == (TurnPosition::Combat {
                step: mtgml_state::CombatStep::BeginningOfCombat,
            });
    let entering_attacker_decision = before.core.position
        == (TurnPosition::Combat {
            step: mtgml_state::CombatStep::BeginningOfCombat,
        })
        && after.core.position
            == (TurnPosition::Combat {
                step: mtgml_state::CombatStep::DeclareAttackers,
            });
    let entering_end_of_combat = before.core.position
        == (TurnPosition::Combat {
            step: mtgml_state::CombatStep::DeclareAttackers,
        })
        && after.core.position
            == (TurnPosition::Combat {
                step: mtgml_state::CombatStep::EndOfCombat,
            })
        && before
            .combat
            .as_ref()
            .is_some_and(|combat| combat.attackers.is_empty())
        && after.combat == before.combat;
    let entering_end_of_combat_after_damage = before.core.position
        == (TurnPosition::Combat {
            step: mtgml_state::CombatStep::CombatDamage,
        })
        && after.core.position
            == (TurnPosition::Combat {
                step: mtgml_state::CombatStep::EndOfCombat,
            })
        && before.combat.is_some();
    let entering_postcombat_main = before.core.position
        == (TurnPosition::Combat {
            step: mtgml_state::CombatStep::EndOfCombat,
        })
        && after.core.position == TurnPosition::PostcombatMain
        && before.combat.as_ref().is_some_and(|combat| {
            combat.attackers.is_empty()
                || crate::combat_damage::validate_combat_damage_state(before).is_ok()
        })
        && after.combat.is_none();
    let entering_blockers_step = before.core.position
        == (TurnPosition::Combat {
            step: mtgml_state::CombatStep::DeclareAttackers,
        })
        && after.core.position
            == (TurnPosition::Combat {
                step: mtgml_state::CombatStep::DeclareBlockers,
            })
        && before
            .combat
            .as_ref()
            .is_some_and(|combat| !combat.attackers.is_empty())
        && after.combat == before.combat;
    let entering_blocker_decision = entering_blockers_step
        && after.core.priority == PriorityState::None
        && after
            .execution
            .pending_decision
            .as_ref()
            .is_some_and(|pending| {
                pending.request.actor
                    == before
                        .combat
                        .as_ref()
                        .map(|combat| combat.defending_player)
                        .unwrap_or(before.core.active_player)
                    && pending.request.decision == DecisionDomainV2::ChooseOne
            });
    let entering_empty_blocker_priority = entering_blockers_step
        && matches!(
            after.core.priority,
            PriorityState::HeldBy {
                player,
                consecutive_passes: 0
            } if player == after.core.active_player
        )
        && after
            .execution
            .pending_decision
            .as_ref()
            .is_some_and(|pending| {
                pending.request.actor == after.core.active_player
                    && pending.request.decision == DecisionDomainV2::ChooseOne
                    && pending.request.candidates.len() == 1
                    && pending.request.candidates[0].visible_intent == CandidateIntent::PassPriority
            });
    if !entering_beginning_of_combat
        && !entering_attacker_decision
        && !entering_blockers_step
        && !entering_blocker_decision
        && !entering_empty_blocker_priority
        && !entering_end_of_combat
        && !entering_end_of_combat_after_damage
        && !entering_postcombat_main
    {
        return Ok(false);
    }
    if entering_blockers_step
        && result.events.len() == 3
        && after.core.priority == PriorityState::None
        && after.execution.pending_decision.is_none()
        && before.revision.0.checked_add(1) == Some(after.revision.0)
    {
        return Ok(false);
    }
    if result.events.len() == if entering_postcombat_main { 4 } else { 3 }
        && after.core.priority == PriorityState::None
        && after.execution.pending_decision.is_none()
        && before.revision.0.checked_add(1) == Some(after.revision.0)
    {
        return Ok(false);
    }
    let expected_count = if entering_empty_blocker_priority {
        6
    } else if entering_blocker_decision {
        4
    } else if entering_beginning_of_combat
        || entering_end_of_combat
        || entering_end_of_combat_after_damage
    {
        5
    } else if entering_postcombat_main {
        6
    } else {
        4
    };
    if result.events.len() != expected_count
        || before.revision.0.checked_add(2) != Some(after.revision.0)
        || result.status != mtgml_model::EpisodeStatus::Running
        || !matches!(
            &result.events[0].event,
            AuthoritativeRuleEventKind::DecisionCleared { decision }
                if *decision == pending.request.decision_id
        )
        || !matches!(
            result.events[1].event,
            AuthoritativeRuleEventKind::PriorityChanged { from, to }
                if from == before.core.priority && to == PriorityState::None
        )
    {
        return Err(TransitionViolation::Priority);
    }
    if entering_blockers_step {
        if !matches!(
            result.events[2].event,
            AuthoritativeRuleEventKind::TurnPositionChanged { from, to }
                if from == before.core.position && to == TurnPosition::Combat { step: mtgml_state::CombatStep::DeclareBlockers }
        ) {
            return Err(TransitionViolation::Priority);
        }
        if entering_blocker_decision {
            let defending_player = before
                .combat
                .as_ref()
                .map(|combat| combat.defending_player)
                .ok_or(TransitionViolation::Combat)?;
            if !matches!(
                result.events[3].event,
                AuthoritativeRuleEventKind::DecisionCreated { decision }
                    if after.execution.pending_decision.as_ref().is_some_and(|request| {
                        request.request.decision_id == decision
                            && request.request.actor == defending_player
                            && request.request.decision == DecisionDomainV2::ChooseOne
                    })
            ) {
                return Err(TransitionViolation::Priority);
            }
            crate::magic::MagicRulesKernel::validate_combat_blockers_runtime_state(
                after,
                &result.status,
            )
            .map_err(|_| TransitionViolation::Priority)?;
        } else {
            let attackers = before
                .combat
                .as_ref()
                .map(|combat| &combat.attackers)
                .ok_or(TransitionViolation::Combat)?;
            if !matches!(
                result.events[3].event,
                AuthoritativeRuleEventKind::BlockersDeclared { ref assignments }
                    if assignments.len() == attackers.len()
                        && assignments.iter().zip(attackers).all(|(assignment, attacker)| {
                            assignment.attacker == *attacker && assignment.blocker.is_none()
                        })
            ) || !matches!(
                result.events[4].event,
                AuthoritativeRuleEventKind::PriorityChanged {
                    from: PriorityState::None,
                    to: PriorityState::HeldBy { player, consecutive_passes: 0 }
                } if player == after.core.active_player
            ) || !matches!(
                result.events[5].event,
                AuthoritativeRuleEventKind::DecisionCreated { decision }
                    if after.execution.pending_decision.as_ref().is_some_and(|request| {
                        request.request.decision_id == decision
                            && request.request.actor == after.core.active_player
                            && request.request.candidates.first().is_some_and(|candidate| {
                                candidate.visible_intent == CandidateIntent::PassPriority
                            })
                    })
            ) {
                return Err(TransitionViolation::Priority);
            }
            validate_pass_only_state_with_blockers(after, true)
                .map_err(|_| TransitionViolation::Priority)?;
        }
    } else if entering_beginning_of_combat
        || entering_end_of_combat
        || entering_end_of_combat_after_damage
    {
        let expected_progression = if entering_end_of_combat {
            AuthoritativeRuleEventKind::EmptyCombatStepsSkipped
        } else {
            AuthoritativeRuleEventKind::TurnPositionChanged {
                from: before.core.position,
                to: crate::turn_structure::temporal_successor(before.core.position),
            }
        };
        if result.events[2].event != expected_progression {
            return Err(TransitionViolation::Priority);
        }
        if !matches!(
            result.events[3].event,
            AuthoritativeRuleEventKind::PriorityChanged {
                from: PriorityState::None,
                to: PriorityState::HeldBy {
                    player,
                    consecutive_passes: 0,
                }
            } if player == before.core.active_player
        ) || !matches!(
            result.events[4].event,
            AuthoritativeRuleEventKind::DecisionCreated { decision }
                if after.execution.pending_decision.as_ref().is_some_and(|request| request.request.decision_id == decision)
        ) {
            return Err(TransitionViolation::Priority);
        }
        if entering_end_of_combat_after_damage {
            validate_pass_only_state_with_combat_damage(after, true)
        } else {
            validate_pass_only_state_with_combat(after, true, true)
        }
        .map_err(|_| TransitionViolation::Priority)?;
    } else if entering_postcombat_main {
        if !matches!(
            result.events[2].event,
            AuthoritativeRuleEventKind::CombatEnded
        ) || !matches!(
            result.events[3].event,
            AuthoritativeRuleEventKind::TurnPositionChanged { from, to }
                if from == before.core.position && to == TurnPosition::PostcombatMain
        ) || !matches!(
            result.events[4].event,
            AuthoritativeRuleEventKind::PriorityChanged {
                from: PriorityState::None,
                to: PriorityState::HeldBy {
                    player,
                    consecutive_passes: 0,
                }
            } if player == after.core.active_player
        ) || !matches!(
            result.events[5].event,
            AuthoritativeRuleEventKind::DecisionCreated { decision }
                if after.execution.pending_decision.as_ref().is_some_and(|request| request.request.decision_id == decision)
        ) {
            return Err(TransitionViolation::Priority);
        }
        validate_pass_only_state(after, true).map_err(|_| TransitionViolation::Priority)?;
    } else {
        if after.core.priority != PriorityState::None
            || !matches!(
                result.events[2].event,
                AuthoritativeRuleEventKind::TurnPositionChanged { from, to }
                    if from == before.core.position && to == crate::turn_structure::temporal_successor(from)
            )
            || !matches!(
                result.events[3].event,
                AuthoritativeRuleEventKind::DecisionCreated { decision }
                    if after.execution.pending_decision.as_ref().is_some_and(|request| request.request.decision_id == decision)
            )
        {
            return Err(TransitionViolation::Priority);
        }
        crate::magic::MagicRulesKernel::validate_combat_runtime_state(after, &result.status)
            .map_err(|_| TransitionViolation::Priority)?;
    }
    Ok(true)
}

pub(crate) fn make_pass_request(
    actor: PlayerId,
    revision: StateRevision,
    decision_id: DecisionId,
    player_decision_id: mtgml_model::PlayerDecisionIdV1,
) -> Result<AuthoritativeDecisionRequestV2, KernelExecutionError> {
    let candidates = CandidateOrderingV1::assign_dense(vec![(
        CandidateIntent::PassPriority,
        EngineCandidateBinding::PassPriority,
    )])
    .map_err(|_| KernelExecutionError::UnsupportedStagePath)?;
    Ok(AuthoritativeDecisionRequestV2 {
        decision_id,
        player_decision_id,
        state_revision: revision,
        actor,
        visibility: DecisionVisibility::ActingPlayerOnly,
        decision: DecisionDomainV2::ChooseOne,
        candidates,
        continuation_id: None,
    })
}

pub(crate) fn bound_event(
    state: &EngineState,
    offset: u64,
    revision: StateRevision,
    event: AuthoritativeRuleEventKind,
) -> Result<AuthoritativeRuleEvent, KernelExecutionError> {
    Ok(AuthoritativeRuleEvent {
        event_id: RuleEventId(
            state
                .allocators
                .next_rule_event_id
                .0
                .checked_add(offset)
                .ok_or(KernelExecutionError::RuleEventIdOverflow)?,
        ),
        state_revision: revision,
        event,
    })
}
