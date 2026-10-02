//! Detached public G0 observed-event V4 contract.

use mtgml_model::{OpaqueObjectId, PlayerId, VisibleSequence, ZoneKind};
use serde::{Deserialize, Serialize};

use crate::{
    error::ObservationValidationError, PublicStackItemV1, PublicTemporaryEffectV1,
    OBSERVED_EVENT_SCHEMA_V4,
};

fn deserialize_required_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::deserialize(deserializer)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservedCounterKindV3 {
    PlusOnePlusOne,
    MinusOneMinusOne,
    Lore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManaPoolAfterV1 {
    pub unrestricted: [u32; 6],
    pub creature_spell_only: [u32; 6],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservedFaceV1 {
    Front,
    Back,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManaPoolChangeCauseV2 {
    Produced,
    Emptied,
    Spent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StackItemRemovalCauseV1 {
    Resolved,
    Countered,
}

/// One block of a declaration: the creature `blocker` blocks `attacker`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservedBlockV1 {
    pub blocker: OpaqueObjectId,
    pub attacker: OpaqueObjectId,
}

/// Who combat damage is dealt to: a player (CR 510.1b) or a creature
/// (CR 510.1c, 510.1d). Players order before creatures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ObservedDamageRecipientV1 {
    Player { player: PlayerId },
    Object { object: OpaqueObjectId },
}

/// Combat damage: `source` deals `amount` to `recipient`. The amount is
/// positive: a creature that would assign no damage assigns none (CR 510.1a).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservedDamageV1 {
    pub source: OpaqueObjectId,
    pub recipient: ObservedDamageRecipientV1,
    pub amount: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ObservedEventKindV4 {
    ObjectMoved {
        #[serde(deserialize_with = "deserialize_required_option")]
        old_object: Option<OpaqueObjectId>,
        #[serde(deserialize_with = "deserialize_required_option")]
        new_object: Option<OpaqueObjectId>,
        from: ZoneKind,
        to: ZoneKind,
        #[serde(deserialize_with = "deserialize_required_option")]
        entering_face: Option<ObservedFaceV1>,
        #[serde(deserialize_with = "deserialize_required_option")]
        tapped: Option<bool>,
    },
    ObjectCeasedToExist {
        object: OpaqueObjectId,
    },
    LifeChanged {
        player: PlayerId,
        from: i64,
        to: i64,
    },
    ObjectTapped {
        object: OpaqueObjectId,
        tapped: bool,
    },
    DecisionAvailable {
        actor: PlayerId,
    },
    RandomOutcomeVisible {
        label: String,
        exclusive_upper_bound: u64,
        value: u64,
    },
    PublicOutcome {
        code: String,
    },
    ManaPoolChanged {
        player: PlayerId,
        pool_after: ManaPoolAfterV1,
        cause: ManaPoolChangeCauseV2,
    },
    CountersChanged {
        object: OpaqueObjectId,
        counter_kind: ObservedCounterKindV3,
        from: u32,
        to: u32,
    },
    AttachmentChanged {
        source: OpaqueObjectId,
        #[serde(deserialize_with = "deserialize_required_option")]
        old_target: Option<OpaqueObjectId>,
        #[serde(deserialize_with = "deserialize_required_option")]
        new_target: Option<OpaqueObjectId>,
    },
    ObjectFaceChanged {
        object: OpaqueObjectId,
        face: ObservedFaceV1,
    },
    StackItemAdded {
        stack_position_from_top: u32,
        item: PublicStackItemV1,
    },
    StackItemRemoved {
        stack_position_from_top: u32,
        item: PublicStackItemV1,
        cause: StackItemRemovalCauseV1,
    },
    TemporaryEffectCreated {
        effect: PublicTemporaryEffectV1,
    },
    TemporaryEffectExpired {
        effect: PublicTemporaryEffectV1,
    },
    /// CR 103.1: the chooser picked who takes the first turn.
    StartingPlayerChosen {
        chooser: PlayerId,
        starting_player: PlayerId,
    },
    /// CR 103.5: a player kept their hand (`mulligan: false`) or took a
    /// mulligan.
    MulliganDeclared {
        player: PlayerId,
        mulligan: bool,
    },
    /// CR 508.1: the attacking player declared these creatures as attackers
    /// against the defending player (CR 506.2), possibly none. The creatures
    /// are listed by ascending opaque id.
    AttackersDeclared {
        attacking_player: PlayerId,
        defending_player: PlayerId,
        attackers: Vec<OpaqueObjectId>,
    },
    /// CR 509.1: the defending player declared these blocks, possibly none:
    /// the step's declaration happens whenever there are attackers. Each
    /// attacker with a blocker becomes blocked (CR 509.1h). The blocks are
    /// listed by ascending opaque id of the blocker.
    BlockersDeclared {
        defending_player: PlayerId,
        blocks: Vec<ObservedBlockV1>,
    },
    /// CR 510.2: all combat damage that was assigned is dealt at once, listed
    /// by ascending source and recipient. A creature dealt lethal damage dies
    /// afterwards, as the zone move that follows (CR 704.5g).
    CombatDamageDealt {
        assignments: Vec<ObservedDamageV1>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservedEventEnvelopeV4 {
    pub schema_version: String,
    pub sequence: VisibleSequence,
    pub event: ObservedEventKindV4,
}

impl ObservedEventEnvelopeV4 {
    pub fn validate(&self) -> Result<(), ObservationValidationError> {
        if self.schema_version != OBSERVED_EVENT_SCHEMA_V4 {
            return Err(ObservationValidationError::SchemaOrCodec);
        }
        match &self.event {
            ObservedEventKindV4::ObjectMoved {
                old_object,
                new_object,
                to,
                entering_face,
                tapped,
                ..
            } => {
                if old_object.is_none() && new_object.is_none() {
                    return Err(ObservationValidationError::ObjectMovedIdentity);
                }
                if (entering_face.is_some() || tapped.is_some())
                    && (*to != ZoneKind::Battlefield || new_object.is_none())
                {
                    return Err(ObservationValidationError::ObservationPayload);
                }
            }
            ObservedEventKindV4::RandomOutcomeVisible {
                label,
                exclusive_upper_bound,
                value,
            } => {
                if label.is_empty() {
                    return Err(ObservationValidationError::EmptyEventText);
                }
                if *exclusive_upper_bound == 0 || *value >= *exclusive_upper_bound {
                    return Err(ObservationValidationError::RandomOutcome);
                }
            }
            ObservedEventKindV4::PublicOutcome { code } if code.is_empty() => {
                return Err(ObservationValidationError::EmptyEventText);
            }
            ObservedEventKindV4::CountersChanged { from, to, .. } if from == to => {
                return Err(ObservationValidationError::ObservationPayload);
            }
            ObservedEventKindV4::AttachmentChanged {
                old_target,
                new_target,
                ..
            } if old_target == new_target => {
                return Err(ObservationValidationError::ObservationPayload);
            }
            ObservedEventKindV4::ManaPoolChanged {
                cause: ManaPoolChangeCauseV2::Emptied,
                pool_after,
                ..
            } if pool_after
                .unrestricted
                .iter()
                .chain(pool_after.creature_spell_only.iter())
                .any(|amount| *amount != 0) =>
            {
                return Err(ObservationValidationError::ObservationPayload);
            }
            ObservedEventKindV4::AttackersDeclared {
                attacking_player,
                defending_player,
                attackers,
            } if attacking_player == defending_player
                || attackers.windows(2).any(|pair| pair[0] >= pair[1]) =>
            {
                return Err(ObservationValidationError::ObservationPayload);
            }
            ObservedEventKindV4::BlockersDeclared { blocks, .. }
                if blocks
                    .windows(2)
                    .any(|pair| pair[0].blocker >= pair[1].blocker) =>
            {
                return Err(ObservationValidationError::ObservationPayload);
            }
            ObservedEventKindV4::CombatDamageDealt { assignments }
                if assignments.iter().any(|damage| damage.amount == 0)
                    || assignments.windows(2).any(|pair| {
                        (pair[0].source, pair[0].recipient) >= (pair[1].source, pair[1].recipient)
                    }) =>
            {
                return Err(ObservationValidationError::ObservationPayload);
            }
            ObservedEventKindV4::StackItemAdded { item, .. }
            | ObservedEventKindV4::StackItemRemoved { item, .. } => item.validate_public()?,
            ObservedEventKindV4::TemporaryEffectCreated { effect }
            | ObservedEventKindV4::TemporaryEffectExpired { effect } => effect.validate_public()?,
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_start_events_round_trip_as_public_wire_values() {
        for (event, json) in [
            (
                ObservedEventKindV4::StartingPlayerChosen {
                    chooser: PlayerId(2),
                    starting_player: PlayerId(1),
                },
                serde_json::json!({
                    "kind": "starting_player_chosen",
                    "chooser": "2",
                    "starting_player": "1",
                }),
            ),
            (
                ObservedEventKindV4::MulliganDeclared {
                    player: PlayerId(1),
                    mulligan: true,
                },
                serde_json::json!({"kind": "mulligan_declared", "player": "1", "mulligan": true}),
            ),
        ] {
            let envelope = ObservedEventEnvelopeV4 {
                schema_version: OBSERVED_EVENT_SCHEMA_V4.into(),
                sequence: VisibleSequence(3),
                event,
            };
            envelope.validate().unwrap();
            assert_eq!(serde_json::to_value(&envelope.event).unwrap(), json);
            assert_eq!(
                serde_json::from_value::<ObservedEventKindV4>(json).unwrap(),
                envelope.event
            );
        }
    }

    fn attackers_declared(
        attacking_player: u64,
        defending_player: u64,
        attackers: &[u64],
    ) -> ObservedEventEnvelopeV4 {
        ObservedEventEnvelopeV4 {
            schema_version: OBSERVED_EVENT_SCHEMA_V4.into(),
            sequence: VisibleSequence(4),
            event: ObservedEventKindV4::AttackersDeclared {
                attacking_player: PlayerId(attacking_player),
                defending_player: PlayerId(defending_player),
                attackers: attackers.iter().copied().map(OpaqueObjectId).collect(),
            },
        }
    }

    #[test]
    fn attackers_declared_round_trips_as_a_public_wire_value() {
        for (envelope, json) in [
            (
                attackers_declared(1, 2, &[4, 9]),
                serde_json::json!({
                    "kind": "attackers_declared",
                    "attacking_player": "1",
                    "defending_player": "2",
                    "attackers": ["4", "9"],
                }),
            ),
            // An empty declaration is public too.
            (
                attackers_declared(2, 1, &[]),
                serde_json::json!({
                    "kind": "attackers_declared",
                    "attacking_player": "2",
                    "defending_player": "1",
                    "attackers": [],
                }),
            ),
        ] {
            envelope.validate().unwrap();
            assert_eq!(serde_json::to_value(&envelope.event).unwrap(), json);
            assert_eq!(
                serde_json::from_value::<ObservedEventKindV4>(json).unwrap(),
                envelope.event
            );
        }
    }

    #[test]
    fn a_malformed_attackers_declaration_is_rejected() {
        for envelope in [
            // The attacker is not the defender.
            attackers_declared(1, 1, &[4]),
            // Ascending and distinct.
            attackers_declared(1, 2, &[9, 4]),
            attackers_declared(1, 2, &[4, 4]),
        ] {
            assert_eq!(
                envelope.validate(),
                Err(ObservationValidationError::ObservationPayload)
            );
        }
    }

    fn envelope(event: ObservedEventKindV4) -> ObservedEventEnvelopeV4 {
        ObservedEventEnvelopeV4 {
            schema_version: OBSERVED_EVENT_SCHEMA_V4.into(),
            sequence: VisibleSequence(6),
            event,
        }
    }

    fn block(blocker: u64, attacker: u64) -> ObservedBlockV1 {
        ObservedBlockV1 {
            blocker: OpaqueObjectId(blocker),
            attacker: OpaqueObjectId(attacker),
        }
    }

    fn blockers_declared(blocks: Vec<ObservedBlockV1>) -> ObservedEventEnvelopeV4 {
        envelope(ObservedEventKindV4::BlockersDeclared {
            defending_player: PlayerId(2),
            blocks,
        })
    }

    #[test]
    fn blockers_declared_round_trips_as_a_public_wire_value() {
        for (envelope, json) in [
            (
                blockers_declared(vec![block(5, 4), block(8, 4)]),
                serde_json::json!({
                    "kind": "blockers_declared",
                    "defending_player": "2",
                    "blocks": [
                        {"blocker": "5", "attacker": "4"},
                        {"blocker": "8", "attacker": "4"},
                    ],
                }),
            ),
            // The declaration with no block is public too.
            (
                blockers_declared(Vec::new()),
                serde_json::json!({
                    "kind": "blockers_declared",
                    "defending_player": "2",
                    "blocks": [],
                }),
            ),
        ] {
            envelope.validate().unwrap();
            assert_eq!(serde_json::to_value(&envelope.event).unwrap(), json);
            assert_eq!(
                serde_json::from_value::<ObservedEventKindV4>(json).unwrap(),
                envelope.event
            );
        }
    }

    #[test]
    fn a_malformed_block_declaration_is_rejected() {
        // Ascending by blocker, and one block for each.
        for blocks in [
            vec![block(8, 4), block(5, 4)],
            vec![block(5, 4), block(5, 9)],
        ] {
            assert_eq!(
                blockers_declared(blocks).validate(),
                Err(ObservationValidationError::ObservationPayload)
            );
        }
        // A block names both its creatures.
        let mut json = serde_json::json!({
            "kind": "blockers_declared",
            "defending_player": "2",
            "blocks": [{"blocker": "5"}],
        });
        assert!(serde_json::from_value::<ObservedEventKindV4>(json.clone()).is_err());
        json["blocks"] = serde_json::json!([{"blocker": "5", "attacker": null}]);
        assert!(serde_json::from_value::<ObservedEventKindV4>(json).is_err());
    }

    fn damage(source: u64, recipient: ObservedDamageRecipientV1, amount: u64) -> ObservedDamageV1 {
        ObservedDamageV1 {
            source: OpaqueObjectId(source),
            recipient,
            amount,
        }
    }

    fn to_object(object: u64) -> ObservedDamageRecipientV1 {
        ObservedDamageRecipientV1::Object {
            object: OpaqueObjectId(object),
        }
    }

    fn to_player(player: u64) -> ObservedDamageRecipientV1 {
        ObservedDamageRecipientV1::Player {
            player: PlayerId(player),
        }
    }

    fn combat_damage_dealt(assignments: Vec<ObservedDamageV1>) -> ObservedEventEnvelopeV4 {
        envelope(ObservedEventKindV4::CombatDamageDealt { assignments })
    }

    #[test]
    fn combat_damage_dealt_round_trips_as_a_public_wire_value() {
        for (envelope, json) in [
            (
                combat_damage_dealt(vec![
                    damage(4, to_object(5), 3),
                    damage(5, to_object(4), 2),
                    damage(9, to_player(2), 1),
                ]),
                serde_json::json!({
                    "kind": "combat_damage_dealt",
                    "assignments": [
                        {"source": "4", "recipient": {"kind": "object", "object": "5"}, "amount": 3},
                        {"source": "5", "recipient": {"kind": "object", "object": "4"}, "amount": 2},
                        {"source": "9", "recipient": {"kind": "player", "player": "2"}, "amount": 1},
                    ],
                }),
            ),
            // A creature's damage divided among its blockers: one assignment
            // for each.
            (
                combat_damage_dealt(vec![damage(4, to_object(5), 1), damage(4, to_object(8), 2)]),
                serde_json::json!({
                    "kind": "combat_damage_dealt",
                    "assignments": [
                        {"source": "4", "recipient": {"kind": "object", "object": "5"}, "amount": 1},
                        {"source": "4", "recipient": {"kind": "object", "object": "8"}, "amount": 2},
                    ],
                }),
            ),
        ] {
            envelope.validate().unwrap();
            assert_eq!(serde_json::to_value(&envelope.event).unwrap(), json);
            assert_eq!(
                serde_json::from_value::<ObservedEventKindV4>(json).unwrap(),
                envelope.event
            );
        }
    }

    #[test]
    fn a_malformed_combat_damage_is_rejected() {
        for assignments in [
            // Combat damage is not dealt in an amount of 0 (CR 510.1a).
            vec![damage(4, to_object(5), 0)],
            vec![damage(4, to_object(5), 3), damage(5, to_player(2), 0)],
            // Sorted by source, and by recipient among one source's, and one
            // assignment for each pair.
            vec![damage(5, to_object(4), 2), damage(4, to_object(5), 3)],
            vec![damage(4, to_object(8), 2), damage(4, to_object(5), 1)],
            vec![damage(4, to_object(5), 3), damage(4, to_object(5), 3)],
        ] {
            assert_eq!(
                combat_damage_dealt(assignments).validate(),
                Err(ObservationValidationError::ObservationPayload)
            );
        }
        // An amount is a number, a recipient is tagged, and neither has an
        // unknown member.
        for assignment in [
            serde_json::json!({"source": "4", "recipient": {"kind": "object", "object": "5"}, "amount": "3"}),
            serde_json::json!({"source": "4", "recipient": {"kind": "object", "object": "5"}, "amount": -3}),
            serde_json::json!({"source": "4", "recipient": {"kind": "creature", "object": "5"}, "amount": 3}),
            serde_json::json!({"source": "4", "recipient": {"kind": "object", "player": "5"}, "amount": 3}),
            serde_json::json!({"source": "4", "recipient": {"kind": "object", "object": "5", "extra": 0}, "amount": 3}),
            serde_json::json!({"source": "4", "recipient": {"object": "5"}, "amount": 3}),
            serde_json::json!({"source": "4", "recipient": {"kind": "object", "object": "5"}}),
            serde_json::json!({"source": "4", "recipient": {"kind": "object", "object": "5"}, "amount": 3, "extra": 0}),
        ] {
            let json = serde_json::json!({
                "kind": "combat_damage_dealt",
                "assignments": [assignment],
            });
            assert!(
                serde_json::from_value::<ObservedEventKindV4>(json).is_err(),
                "{assignment}"
            );
        }
    }
}
