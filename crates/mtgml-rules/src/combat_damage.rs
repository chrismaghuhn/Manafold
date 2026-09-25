use std::collections::{BTreeMap, BTreeSet};

use mtgml_model::{GameObjectId, PlayerId, ZoneKind};
use mtgml_state::{
    BaseCharacteristics, DamageAssignmentV1, DamageRecipientV1, EngineState, FoundationSourceKind,
    TurnPosition,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(crate) enum CombatDamageError {
    #[error("unsupported combat damage state")]
    Unsupported,
    #[error("combat damage arithmetic exhausted")]
    Exhaustion,
}

fn source_power(
    state: &EngineState,
    object: GameObjectId,
) -> Result<Option<u64>, CombatDamageError> {
    let game_object = state
        .zones
        .objects
        .get(&object)
        .ok_or(CombatDamageError::Unsupported)?;
    let location = state
        .zones
        .locations
        .get(&object)
        .ok_or(CombatDamageError::Unsupported)?;
    let source = state
        .foundation_sources
        .get(&object)
        .ok_or(CombatDamageError::Unsupported)?;
    if location.zone != ZoneKind::Battlefield
        || game_object.face_down
        || source.source_kind != FoundationSourceKind::Creature
    {
        return Err(CombatDamageError::Unsupported);
    }
    let BaseCharacteristics::Simple { power, .. } = source.base_characteristics;
    if power < 0 {
        return Err(CombatDamageError::Unsupported);
    }
    if power == 0 {
        Ok(None)
    } else {
        Ok(Some(
            u64::try_from(power).map_err(|_| CombatDamageError::Exhaustion)?,
        ))
    }
}

fn push_assignment(
    assignments: &mut Vec<DamageAssignmentV1>,
    source: GameObjectId,
    recipient: DamageRecipientV1,
    state: &EngineState,
) -> Result<(), CombatDamageError> {
    if let Some(amount) = source_power(state, source)? {
        assignments.push(DamageAssignmentV1 {
            source,
            recipient,
            amount,
        });
    }
    Ok(())
}

pub(crate) fn derive_assignments(
    state: &EngineState,
) -> Result<Vec<DamageAssignmentV1>, CombatDamageError> {
    validate_combat_damage_state(state)?;
    let combat = state
        .combat
        .as_ref()
        .ok_or(CombatDamageError::Unsupported)?;
    if state.core.position
        != (TurnPosition::Combat {
            step: mtgml_state::CombatStep::CombatDamage,
        })
        || combat.damage_step_completed
    {
        return Err(CombatDamageError::Unsupported);
    }
    let mut assignments = Vec::new();
    for attacker in &combat.attackers {
        let blocker = combat
            .blockers
            .get(attacker)
            .ok_or(CombatDamageError::Unsupported)?;
        let blocked = combat.blocked_attackers.contains(attacker);
        if !blocked && blocker.is_some() {
            return Err(CombatDamageError::Unsupported);
        }
        if blocked {
            if let Some(blocker) = blocker {
                push_assignment(
                    &mut assignments,
                    *attacker,
                    DamageRecipientV1::Creature { object: *blocker },
                    state,
                )?;
                push_assignment(
                    &mut assignments,
                    *blocker,
                    DamageRecipientV1::Creature { object: *attacker },
                    state,
                )?;
            }
        } else {
            push_assignment(
                &mut assignments,
                *attacker,
                DamageRecipientV1::Player {
                    player: combat.defending_player,
                },
                state,
            )?;
        }
    }
    Ok(assignments)
}

pub(crate) fn validate_combat_damage_state(state: &EngineState) -> Result<(), CombatDamageError> {
    if !matches!(
        state.core.position,
        TurnPosition::Combat {
            step: mtgml_state::CombatStep::CombatDamage | mtgml_state::CombatStep::EndOfCombat
        }
    ) || state.core.players.len() != 2
        || !matches!(state.format, mtgml_state::FormatState::None)
    {
        return Err(CombatDamageError::Unsupported);
    }
    let combat = state
        .combat
        .as_ref()
        .ok_or(CombatDamageError::Unsupported)?;
    let defender = state
        .core
        .players
        .keys()
        .copied()
        .find(|player| *player != state.core.active_player)
        .ok_or(CombatDamageError::Unsupported)?;
    let attackers: BTreeSet<_> = combat.attackers.iter().copied().collect();
    if combat.defending_player != defender
        || combat.attackers.len() > 8
        || combat.attackers.windows(2).any(|pair| pair[0] >= pair[1])
        || attackers.len() != combat.attackers.len()
        || combat.blockers.keys().copied().collect::<BTreeSet<_>>() != attackers
        || !combat.blocked_attackers.is_subset(&attackers)
    {
        return Err(CombatDamageError::Unsupported);
    }
    let mut blockers = BTreeSet::new();
    for attacker in &combat.attackers {
        let blocker = combat
            .blockers
            .get(attacker)
            .ok_or(CombatDamageError::Unsupported)?;
        if blocker.is_some() && !combat.blocked_attackers.contains(attacker) {
            return Err(CombatDamageError::Unsupported);
        }
        let object = state
            .zones
            .objects
            .get(attacker)
            .ok_or(CombatDamageError::Unsupported)?;
        if object.controller != state.core.active_player || !object.tapped {
            return Err(CombatDamageError::Unsupported);
        }
        source_power(state, *attacker)?;
        if let Some(blocker) = blocker {
            if !blockers.insert(*blocker) {
                return Err(CombatDamageError::Unsupported);
            }
            let object = state
                .zones
                .objects
                .get(blocker)
                .ok_or(CombatDamageError::Unsupported)?;
            if object.controller != defender || object.tapped {
                return Err(CombatDamageError::Unsupported);
            }
            source_power(state, *blocker)?;
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CombatDamageMutationPlan {
    pub(crate) player_life: Vec<(PlayerId, i64, i64)>,
    pub(crate) creature_marks: Vec<(GameObjectId, u64, u64)>,
}

pub(crate) fn derive_mutations(
    state: &EngineState,
    assignments: &[DamageAssignmentV1],
) -> Result<CombatDamageMutationPlan, CombatDamageError> {
    let mut player_totals = BTreeMap::<PlayerId, u64>::new();
    let mut creature_totals = BTreeMap::<GameObjectId, u64>::new();
    let mut creature_order = Vec::new();
    for assignment in assignments {
        match assignment.recipient {
            DamageRecipientV1::Player { player } => {
                let total = player_totals.entry(player).or_default();
                *total = total
                    .checked_add(assignment.amount)
                    .ok_or(CombatDamageError::Exhaustion)?;
            }
            DamageRecipientV1::Creature { object } => {
                if !creature_totals.contains_key(&object) {
                    creature_order.push(object);
                }
                let total = creature_totals.entry(object).or_default();
                *total = total
                    .checked_add(assignment.amount)
                    .ok_or(CombatDamageError::Exhaustion)?;
            }
        }
    }
    let mut player_life = Vec::new();
    for (player, amount) in player_totals {
        let before = state
            .core
            .players
            .get(&player)
            .ok_or(CombatDamageError::Unsupported)?
            .life;
        let amount = i64::try_from(amount).map_err(|_| CombatDamageError::Exhaustion)?;
        let after = before
            .checked_sub(amount)
            .ok_or(CombatDamageError::Exhaustion)?;
        if after != before {
            player_life.push((player, before, after));
        }
    }
    let mut creature_marks = Vec::new();
    for object in creature_order {
        let source = state
            .foundation_sources
            .get(&object)
            .ok_or(CombatDamageError::Unsupported)?;
        let total = creature_totals
            .get(&object)
            .copied()
            .ok_or(CombatDamageError::Unsupported)?;
        let after = source
            .marked_damage
            .checked_add(total)
            .ok_or(CombatDamageError::Exhaustion)?;
        if after != source.marked_damage {
            creature_marks.push((object, source.marked_damage, after));
        }
    }
    Ok(CombatDamageMutationPlan {
        player_life,
        creature_marks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use mtgml_model::{CardDefinitionId, PhysicalCardId};
    use mtgml_random::RootSeed256;
    use mtgml_state::{
        ControlHistory, FoundationCreatureSource, SyntheticResetInputs, SyntheticV4Setup,
    };

    fn state() -> EngineState {
        let mut state = mtgml_state::construct_synthetic_engine_state(SyntheticResetInputs {
            players: [PlayerId(1), PlayerId(2)],
            root_seed: RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap(),
            setup: SyntheticV4Setup::synthetic_compatibility(),
        })
        .unwrap();
        let object = *state.zones.objects.keys().next().unwrap();
        let game_object = state.zones.objects.get_mut(&object).unwrap();
        game_object.physical_card = Some(PhysicalCardId(777));
        game_object.card_definition = CardDefinitionId(777);
        state.foundation_sources.insert(
            object,
            FoundationCreatureSource {
                source_kind: FoundationSourceKind::Creature,
                base_characteristics: BaseCharacteristics::Simple {
                    power: 1,
                    toughness: i64::MAX,
                },
                marked_damage: u64::MAX,
                control_history: ControlHistory::BeforeTurnStart { turn_number: 1 },
            },
        );
        state
    }

    #[test]
    fn damage_life_and_mark_arithmetic_exhaustion_mutates_nothing() {
        let mut state = state();
        state.core.players.get_mut(&PlayerId(2)).unwrap().life = 1;
        let before = state.clone();
        assert_eq!(
            derive_mutations(
                &state,
                &[DamageAssignmentV1 {
                    source: GameObjectId(1),
                    recipient: DamageRecipientV1::Player {
                        player: PlayerId(2)
                    },
                    amount: u64::MAX,
                }],
            ),
            Err(CombatDamageError::Exhaustion)
        );
        assert_eq!(state, before);

        state.core.players.get_mut(&PlayerId(2)).unwrap().life = i64::MIN;
        let before = state.clone();
        assert_eq!(
            derive_mutations(
                &state,
                &[DamageAssignmentV1 {
                    source: GameObjectId(1),
                    recipient: DamageRecipientV1::Player {
                        player: PlayerId(2)
                    },
                    amount: 1,
                }],
            ),
            Err(CombatDamageError::Exhaustion)
        );
        assert_eq!(state, before);

        let object = *state.foundation_sources.keys().next().unwrap();
        let before = state.clone();
        assert_eq!(
            derive_mutations(
                &state,
                &[DamageAssignmentV1 {
                    source: GameObjectId(2),
                    recipient: DamageRecipientV1::Creature { object },
                    amount: 1,
                }],
            ),
            Err(CombatDamageError::Exhaustion)
        );
        assert_eq!(state, before);
    }
}
