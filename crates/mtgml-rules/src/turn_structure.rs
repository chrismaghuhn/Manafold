use mtgml_model::{GameObjectId, PlayerId, ZoneKind};
use mtgml_state::{BeginningStep, CombatStep, EndingStep, ObjectSnapshot, TurnPosition};
use std::collections::BTreeMap;
use thiserror::Error;

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
    #[error("turn number would overflow")]
    TurnNumberOverflow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsupportedRulesBoundary {
    BasicPriority,
    DrawCard,
    Combat,
    CleanupReset,
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

/// Pinned Comprehensive Rules CR 402.2 ordinary maximum hand size.
///
/// "Each player has a maximum hand size, which is normally seven cards."
/// Within the S1 admitted profile (no format, no effects/triggers/delayed
/// effects), no modifier that could change this value can be represented,
/// so 7 is the fixed operative constant. Hand sizes are computed from
/// authoritative Hand-zone facts; the maximum hand size is a derived rule
/// constant, not a field of `EngineState`.
pub(crate) const ORDINARY_MAXIMUM_HAND_SIZE: usize = 7;

#[cfg(test)]
mod tests {
    use super::*;

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
}
