use mtgml_model::{GameObjectId, PlayerId, ZoneKind};
use mtgml_state::{
    BeginningStep, CombatStep, EndingStep, EngineState, FormatState, ObjectSnapshot, PriorityState,
    TurnPosition,
};
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnStructureSupportProfile {
    active_player: PlayerId,
    other_player: PlayerId,
    turn_number: u64,
    position: TurnPosition,
}

impl TurnStructureSupportProfile {
    pub fn active_player(&self) -> PlayerId {
        self.active_player
    }

    pub fn other_player(&self) -> PlayerId {
        self.other_player
    }

    pub fn turn_number(&self) -> u64 {
        self.turn_number
    }

    pub fn position(&self) -> TurnPosition {
        self.position
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum TurnStructureError {
    #[error("exactly two players required")]
    UnsupportedPlayerCount,
    #[error("active player is undeclared")]
    ActivePlayerUndeclared,
    #[error("other player derivation failed")]
    OtherPlayerDerivation,
    #[error("turn number is zero")]
    ZeroTurnNumber,
    #[error("pending decision is present")]
    PendingDecision,
    #[error("continuation state is present")]
    ContinuationState,
    #[error("stack state is present")]
    StackState,
    #[error("effect state is present")]
    EffectState,
    #[error("waiting trigger state is present")]
    TriggerState,
    #[error("delayed effect state is present")]
    DelayedEffect,
    #[error("format state is not None")]
    FormatState,
    #[error("combat state is active")]
    CombatState,
    #[error("priority is held")]
    PriorityHeld,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsupportedRulesBoundary {
    BasicPriority,
    DrawCard,
    Combat,
    CleanupReset,
}

pub fn validate_turn_structure_support(
    state: &EngineState,
) -> Result<TurnStructureSupportProfile, TurnStructureError> {
    if state.core.players.len() != 2 {
        return Err(TurnStructureError::UnsupportedPlayerCount);
    }

    let active_player = state.core.active_player;
    if !state.core.players.contains_key(&active_player) {
        return Err(TurnStructureError::ActivePlayerUndeclared);
    }

    let other_players: Vec<PlayerId> = state
        .core
        .players
        .keys()
        .copied()
        .filter(|p| *p != active_player)
        .collect();
    if other_players.len() != 1 {
        return Err(TurnStructureError::OtherPlayerDerivation);
    }
    let other_player = other_players[0];

    if state.core.turn_number == 0 {
        return Err(TurnStructureError::ZeroTurnNumber);
    }

    if state.execution.pending_decision.is_some() {
        return Err(TurnStructureError::PendingDecision);
    }

    if !state.execution.continuations.is_empty() {
        return Err(TurnStructureError::ContinuationState);
    }

    if !state.execution.effects.is_empty() {
        return Err(TurnStructureError::EffectState);
    }

    if !state.execution.waiting_triggers.is_empty() {
        return Err(TurnStructureError::TriggerState);
    }

    if !state.execution.delayed_effects.is_empty() {
        return Err(TurnStructureError::DelayedEffect);
    }

    if !state.zones.stack_records.is_empty() {
        return Err(TurnStructureError::StackState);
    }

    if !state.zones.stack_order.is_empty() {
        return Err(TurnStructureError::StackState);
    }

    if state
        .zones
        .locations
        .values()
        .any(|location| location.zone == ZoneKind::Stack)
    {
        return Err(TurnStructureError::StackState);
    }

    if state.format != FormatState::None {
        return Err(TurnStructureError::FormatState);
    }

    if state.combat.is_some() {
        return Err(TurnStructureError::CombatState);
    }

    if state.core.priority != PriorityState::None {
        return Err(TurnStructureError::PriorityHeld);
    }

    Ok(TurnStructureSupportProfile {
        active_player,
        other_player,
        turn_number: state.core.turn_number,
        position: state.core.position,
    })
}

pub fn temporal_successor(position: TurnPosition) -> TurnPosition {
    match position {
        TurnPosition::Beginning {
            step: BeginningStep::Untap,
        } => TurnPosition::Beginning {
            step: BeginningStep::Upkeep,
        },
        TurnPosition::Beginning {
            step: BeginningStep::Upkeep,
        } => TurnPosition::Beginning {
            step: BeginningStep::Draw,
        },
        TurnPosition::Beginning {
            step: BeginningStep::Draw,
        } => TurnPosition::PrecombatMain,
        TurnPosition::PrecombatMain => TurnPosition::Combat {
            step: CombatStep::BeginningOfCombat,
        },
        TurnPosition::Combat {
            step: CombatStep::BeginningOfCombat,
        } => TurnPosition::Combat {
            step: CombatStep::DeclareAttackers,
        },
        TurnPosition::Combat {
            step: CombatStep::DeclareAttackers,
        } => TurnPosition::Combat {
            step: CombatStep::DeclareBlockers,
        },
        TurnPosition::Combat {
            step: CombatStep::DeclareBlockers,
        } => TurnPosition::Combat {
            step: CombatStep::CombatDamage,
        },
        TurnPosition::Combat {
            step: CombatStep::CombatDamage,
        } => TurnPosition::Combat {
            step: CombatStep::EndOfCombat,
        },
        TurnPosition::Combat {
            step: CombatStep::EndOfCombat,
        } => TurnPosition::PostcombatMain,
        TurnPosition::PostcombatMain => TurnPosition::Ending {
            step: EndingStep::EndStep,
        },
        TurnPosition::Ending {
            step: EndingStep::EndStep,
        } => TurnPosition::Ending {
            step: EndingStep::Cleanup,
        },
        TurnPosition::Ending {
            step: EndingStep::Cleanup,
        } => TurnPosition::Beginning {
            step: BeginningStep::Untap,
        },
    }
}

pub fn unsupported_rules_boundary(position: TurnPosition) -> Option<UnsupportedRulesBoundary> {
    match position {
        TurnPosition::Beginning {
            step: BeginningStep::Untap,
        } => None,
        TurnPosition::Beginning {
            step: BeginningStep::Upkeep,
        } => Some(UnsupportedRulesBoundary::BasicPriority),
        TurnPosition::Beginning {
            step: BeginningStep::Draw,
        } => Some(UnsupportedRulesBoundary::DrawCard),
        TurnPosition::PrecombatMain => Some(UnsupportedRulesBoundary::BasicPriority),
        TurnPosition::Combat { .. } => Some(UnsupportedRulesBoundary::Combat),
        TurnPosition::PostcombatMain => Some(UnsupportedRulesBoundary::BasicPriority),
        TurnPosition::Ending {
            step: EndingStep::EndStep,
        } => Some(UnsupportedRulesBoundary::BasicPriority),
        TurnPosition::Ending {
            step: EndingStep::Cleanup,
        } => None,
    }
}

/// Crate-private ordinary-untap eligibility authority.
///
/// Derives the complete canonical set of `GameObjectId`s eligible for ordinary
/// untap from authoritative object snapshots. An object is affected exactly when
/// all of the following hold in the before-state:
///   - live object
///   - location.zone == ZoneKind::Battlefield
///   - controller == active_player
///   - tapped == true
///
/// The returned vector is in strict ascending `GameObjectId` order. This is
/// the single eligibility authority shared by `MagicRulesKernel` and
/// `SemanticValidationCursor` so the two can never disagree about which objects
/// ordinary Untap must affect.
pub(crate) fn derive_ordinary_untap_affected_objects(
    snapshots: &BTreeMap<GameObjectId, ObjectSnapshot>,
    active_player: PlayerId,
) -> Vec<GameObjectId> {
    let mut affected: Vec<GameObjectId> = snapshots
        .iter()
        .filter(|(_, snapshot)| {
            snapshot.location.zone == ZoneKind::Battlefield
                && snapshot.controller == active_player
                && snapshot.tapped
        })
        .map(|(id, _)| *id)
        .collect();
    affected.sort();
    affected
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validate_runtime_state;
    use mtgml_model::{
        ExecutionProgramV1, GameObjectId, OpaqueObjectId, PlayerId, StateRevision, ZoneKind,
    };
    use mtgml_random::RootSeed256;
    use mtgml_state::{
        construct_synthetic_engine_state, CombatState, SyntheticResetInputs, SyntheticV4Setup,
        VisibilityPartition, ZoneLocation, ZonePosition,
    };

    fn valid_s1_state() -> EngineState {
        let mut state = construct_synthetic_engine_state(SyntheticResetInputs {
            players: [PlayerId(7), PlayerId(42)],
            root_seed: RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap(),
            setup: SyntheticV4Setup {
                position: TurnPosition::Beginning {
                    step: BeginningStep::Untap,
                },
                priority: PriorityState::None,
                combat: None,
                foundation_sources: std::collections::BTreeMap::new(),
            },
        })
        .unwrap();
        state.execution.pending_decision = None;
        state
    }

    #[test]
    fn support_profile_accepts_valid_two_player_state() {
        let state = valid_s1_state();
        assert!(
            validate_runtime_state(ExecutionProgramV1::MagicRules, &state).is_ok(),
            "valid S1 state should be admitted"
        );
    }

    #[test]
    fn support_profile_rejects_one_player() {
        let mut state = valid_s1_state();
        state.core.players.remove(&PlayerId(42));
        assert!(matches!(
            validate_turn_structure_support(&state),
            Err(TurnStructureError::UnsupportedPlayerCount)
        ));
    }

    #[test]
    fn support_profile_rejects_three_players() {
        let mut state = valid_s1_state();
        state.core.players.insert(
            PlayerId(99),
            mtgml_state::PlayerState {
                life: 40,
                has_lost: false,
            },
        );
        assert!(matches!(
            validate_turn_structure_support(&state),
            Err(TurnStructureError::UnsupportedPlayerCount)
        ));
    }

    #[test]
    fn support_profile_rejects_undeclared_active_player() {
        let mut state = valid_s1_state();
        state.core.active_player = PlayerId(99);
        assert!(matches!(
            validate_turn_structure_support(&state),
            Err(TurnStructureError::ActivePlayerUndeclared)
        ));
    }

    #[test]
    fn support_profile_derives_unique_other_player() {
        let state = valid_s1_state();
        let profile = validate_turn_structure_support(&state).unwrap();
        assert_eq!(profile.active_player(), PlayerId(7));
        assert_eq!(profile.other_player(), PlayerId(42));
    }

    #[test]
    fn support_profile_rejects_zero_turn() {
        let mut state = valid_s1_state();
        state.core.turn_number = 0;
        assert!(matches!(
            validate_turn_structure_support(&state),
            Err(TurnStructureError::ZeroTurnNumber)
        ));
    }

    #[test]
    fn support_profile_accepts_u64_max_turn() {
        let mut state = valid_s1_state();
        state.core.turn_number = u64::MAX;
        assert!(validate_turn_structure_support(&state).is_ok());
    }

    #[test]
    fn support_profile_rejects_pending_decision() {
        let mut state = valid_s1_state();
        state.execution.pending_decision = Some(mtgml_state::PendingDecisionRecordV2 {
            request: mtgml_decision::AuthoritativeDecisionRequestV2 {
                decision_id: mtgml_model::DecisionId(1),
                player_decision_id: mtgml_model::PlayerDecisionIdV1(1),
                state_revision: StateRevision(0),
                actor: PlayerId(7),
                visibility: mtgml_decision::DecisionVisibility::Public,
                decision: mtgml_decision::DecisionDomainV2::ChooseOne,
                candidates: Vec::new(),
                continuation_id: None,
            },
        });
        assert!(matches!(
            validate_turn_structure_support(&state),
            Err(TurnStructureError::PendingDecision)
        ));
    }

    #[test]
    fn support_profile_rejects_continuation() {
        let mut state = valid_s1_state();
        state.execution.continuations.insert(
            mtgml_model::ContinuationId(1),
            mtgml_state::ContinuationRecordV2 {
                id: mtgml_model::ContinuationId(1),
                actor: PlayerId(7),
                created_at_revision: StateRevision(0),
                stage_index: mtgml_state::AssemblyStageV2::ChooseCount.stage_index(),
                payload: mtgml_state::ContinuationPayloadV2::SyntheticM2Assembly {
                    stage: mtgml_state::AssemblyStageV2::ChooseCount,
                    selected_count: None,
                    selected_piece_keys: Vec::new(),
                    ordered_piece_keys: Vec::new(),
                },
            },
        );
        assert!(matches!(
            validate_turn_structure_support(&state),
            Err(TurnStructureError::ContinuationState)
        ));
    }

    #[test]
    fn support_profile_rejects_stack_state() {
        let mut state = valid_s1_state();
        state.zones.stack_records.insert(
            mtgml_model::StackObjectId(1),
            mtgml_state::StackRecord {
                id: mtgml_model::StackObjectId(1),
                controller: PlayerId(7),
                source_object: None,
                source_ability: None,
            },
        );
        assert!(matches!(
            validate_turn_structure_support(&state),
            Err(TurnStructureError::StackState)
        ));
    }

    #[test]
    fn support_profile_rejects_live_stack_zone_object() {
        let mut state = valid_s1_state();
        assert!(
            mtgml_state::validate_engine_state(&state).is_ok(),
            "generic validator must accept the shape before S1 profile rejects it"
        );
        assert!(state.zones.stack_records.is_empty());
        assert!(state.zones.stack_order.is_empty());
        let stack_location = ZoneLocation {
            zone: ZoneKind::Stack,
            player: None,
            position: ZonePosition::Unordered,
            visibility: VisibilityPartition::Public,
            partition: None,
        };
        state
            .zones
            .locations
            .insert(GameObjectId(1), stack_location.clone());
        if let Some(player_knowledge) = state.knowledge.players.get_mut(&PlayerId(7)) {
            if let Some(record) = player_knowledge.active.get_mut(&OpaqueObjectId(1)) {
                record.known_location = Some(mtgml_state::KnownLocationFactV2 {
                    location: stack_location.clone(),
                    provenance: record.known_location.as_ref().unwrap().provenance,
                });
            }
        }
        if let Some(player_knowledge) = state.knowledge.players.get_mut(&PlayerId(42)) {
            if let Some(record) = player_knowledge.active.get_mut(&OpaqueObjectId(1)) {
                record.known_location = Some(mtgml_state::KnownLocationFactV2 {
                    location: stack_location.clone(),
                    provenance: record.known_location.as_ref().unwrap().provenance,
                });
            }
        }
        let generic_result = mtgml_state::validate_engine_state(&state);
        assert!(
            generic_result.is_ok(),
            "generic validator must still accept after knowledge alignment: {:?}",
            generic_result
        );
        assert!(matches!(
            validate_runtime_state(ExecutionProgramV1::MagicRules, &state),
            Err(crate::KernelExecutionError::TurnStructure(
                TurnStructureError::StackState
            ))
        ));
    }

    #[test]
    fn support_profile_rejects_effect() {
        let mut state = valid_s1_state();
        state.execution.effects.insert(
            mtgml_model::EffectInstanceId(1),
            mtgml_state::EffectRecord {
                id: mtgml_model::EffectInstanceId(1),
                label: "test".to_string(),
            },
        );
        assert!(matches!(
            validate_turn_structure_support(&state),
            Err(TurnStructureError::EffectState)
        ));
    }

    #[test]
    fn support_profile_rejects_waiting_trigger() {
        let mut state = valid_s1_state();
        state.execution.waiting_triggers.insert(
            mtgml_model::TriggerInstanceId(1),
            mtgml_state::TriggerRecord {
                id: mtgml_model::TriggerInstanceId(1),
                controller: PlayerId(7),
            },
        );
        assert!(matches!(
            validate_turn_structure_support(&state),
            Err(TurnStructureError::TriggerState)
        ));
    }

    #[test]
    fn support_profile_rejects_delayed_effect() {
        let mut state = valid_s1_state();
        state.execution.delayed_effects.insert(
            mtgml_model::EffectInstanceId(1),
            mtgml_state::EffectRecord {
                id: mtgml_model::EffectInstanceId(1),
                label: "test".to_string(),
            },
        );
        assert!(matches!(
            validate_turn_structure_support(&state),
            Err(TurnStructureError::DelayedEffect)
        ));
    }

    #[test]
    fn support_profile_rejects_format_state() {
        let mut state = valid_s1_state();
        state.format = mtgml_state::FormatState::Commander {
            state: mtgml_state::CommanderState {
                designations: std::collections::BTreeMap::new(),
                cast_counts: std::collections::BTreeMap::new(),
                damage: std::collections::BTreeMap::new(),
            },
        };
        assert!(matches!(
            validate_turn_structure_support(&state),
            Err(TurnStructureError::FormatState)
        ));
    }

    #[test]
    fn support_profile_rejects_active_combat() {
        let mut state = valid_s1_state();
        state.combat = Some(CombatState {
            defending_player: PlayerId(42),
            attackers: vec![GameObjectId(1)],
            blockers: std::collections::BTreeMap::new(),
        });
        assert!(matches!(
            validate_turn_structure_support(&state),
            Err(TurnStructureError::CombatState)
        ));
    }

    #[test]
    fn support_profile_rejects_held_priority() {
        let mut state = valid_s1_state();
        state.core.priority = PriorityState::HeldBy {
            player: PlayerId(7),
            consecutive_passes: 0,
        };
        assert!(matches!(
            validate_turn_structure_support(&state),
            Err(TurnStructureError::PriorityHeld)
        ));
    }

    #[test]
    fn temporal_successor_table_is_exact() {
        use BeginningStep::*;
        use CombatStep::*;
        use EndingStep::*;

        let cases: Vec<(TurnPosition, TurnPosition)> = vec![
            (
                TurnPosition::Beginning { step: Untap },
                TurnPosition::Beginning { step: Upkeep },
            ),
            (
                TurnPosition::Beginning { step: Upkeep },
                TurnPosition::Beginning { step: Draw },
            ),
            (
                TurnPosition::Beginning { step: Draw },
                TurnPosition::PrecombatMain,
            ),
            (
                TurnPosition::PrecombatMain,
                TurnPosition::Combat {
                    step: BeginningOfCombat,
                },
            ),
            (
                TurnPosition::Combat {
                    step: BeginningOfCombat,
                },
                TurnPosition::Combat {
                    step: DeclareAttackers,
                },
            ),
            (
                TurnPosition::Combat {
                    step: DeclareAttackers,
                },
                TurnPosition::Combat {
                    step: DeclareBlockers,
                },
            ),
            (
                TurnPosition::Combat {
                    step: DeclareBlockers,
                },
                TurnPosition::Combat { step: CombatDamage },
            ),
            (
                TurnPosition::Combat { step: CombatDamage },
                TurnPosition::Combat { step: EndOfCombat },
            ),
            (
                TurnPosition::Combat { step: EndOfCombat },
                TurnPosition::PostcombatMain,
            ),
            (
                TurnPosition::PostcombatMain,
                TurnPosition::Ending { step: EndStep },
            ),
            (
                TurnPosition::Ending { step: EndStep },
                TurnPosition::Ending { step: Cleanup },
            ),
            (
                TurnPosition::Ending { step: Cleanup },
                TurnPosition::Beginning { step: Untap },
            ),
        ];

        for (from, expected_to) in cases {
            assert_eq!(
                temporal_successor(from),
                expected_to,
                "successor of {from:?}"
            );
        }
    }

    #[test]
    fn temporal_successor_distinction_from_boundary() {
        assert_eq!(
            temporal_successor(TurnPosition::Beginning {
                step: BeginningStep::Upkeep
            }),
            TurnPosition::Beginning {
                step: BeginningStep::Draw
            }
        );
        assert_eq!(
            unsupported_rules_boundary(TurnPosition::Beginning {
                step: BeginningStep::Upkeep
            }),
            Some(UnsupportedRulesBoundary::BasicPriority)
        );
    }

    #[test]
    fn boundary_classification_untap() {
        assert_eq!(
            unsupported_rules_boundary(TurnPosition::Beginning {
                step: BeginningStep::Untap
            }),
            None
        );
    }

    #[test]
    fn boundary_classification_upkeep() {
        assert_eq!(
            unsupported_rules_boundary(TurnPosition::Beginning {
                step: BeginningStep::Upkeep
            }),
            Some(UnsupportedRulesBoundary::BasicPriority)
        );
    }

    #[test]
    fn boundary_classification_draw() {
        assert_eq!(
            unsupported_rules_boundary(TurnPosition::Beginning {
                step: BeginningStep::Draw
            }),
            Some(UnsupportedRulesBoundary::DrawCard)
        );
    }

    #[test]
    fn boundary_classification_precombat() {
        assert_eq!(
            unsupported_rules_boundary(TurnPosition::PrecombatMain),
            Some(UnsupportedRulesBoundary::BasicPriority)
        );
    }

    #[test]
    fn boundary_classification_all_combat() {
        for step in [
            CombatStep::BeginningOfCombat,
            CombatStep::DeclareAttackers,
            CombatStep::DeclareBlockers,
            CombatStep::CombatDamage,
            CombatStep::EndOfCombat,
        ] {
            assert_eq!(
                unsupported_rules_boundary(TurnPosition::Combat { step }),
                Some(UnsupportedRulesBoundary::Combat),
                "combat step {step:?}",
            );
        }
    }

    #[test]
    fn boundary_classification_postcombat() {
        assert_eq!(
            unsupported_rules_boundary(TurnPosition::PostcombatMain),
            Some(UnsupportedRulesBoundary::BasicPriority)
        );
    }

    #[test]
    fn boundary_classification_end_step() {
        assert_eq!(
            unsupported_rules_boundary(TurnPosition::Ending {
                step: EndingStep::EndStep
            }),
            Some(UnsupportedRulesBoundary::BasicPriority)
        );
    }

    #[test]
    fn boundary_classification_cleanup() {
        assert_eq!(
            unsupported_rules_boundary(TurnPosition::Ending {
                step: EndingStep::Cleanup
            }),
            None
        );
    }
}
