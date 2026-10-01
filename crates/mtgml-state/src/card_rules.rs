//! Typed card-rules authoritative state (mana, turn history, counters,
//! attachments, faces, ability authority, permanents) and its FullStateDigest
//! record encoding.

use std::collections::{BTreeMap, BTreeSet};

use mtgml_model::{AbilityInstanceId, GameObjectId, PlayerId, StateRevision};
use mtgml_persistence::cbor::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u64)]
pub enum ManaColorV1 {
    White = 0,
    Blue = 1,
    Black = 2,
    Red = 3,
    Green = 4,
    Colorless = 5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ManaRestrictionV1 {
    Unrestricted,
    CreatureSpellOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ManaPoolV1 {
    pub unrestricted: [u32; 6],
    pub creature_spell_only: [u32; 6],
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ManaStateV1 {
    pub pools: BTreeMap<PlayerId, ManaPoolV1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlayerTurnHistoryV1 {
    pub land_plays_used: u8,
    pub spells_cast_total: u32,
    pub noncreature_spells_cast: u32,
    pub lost_life_this_turn: bool,
    pub red_noncombat_damage_dealt: u32,
    pub permanent_card_to_graveyard: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TurnHistoryStateV1 {
    pub turn_number: u64,
    pub players: BTreeMap<PlayerId, PlayerTurnHistoryV1>,
    pub target_occurrences: BTreeSet<(GameObjectId, PlayerId)>,
    pub once_ability_used: BTreeSet<(GameObjectId, u32)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u64)]
pub enum CounterKindV1 {
    PlusOnePlusOne = 0,
    MinusOneMinusOne = 1,
    Lore = 2,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CounterStateV1 {
    pub counters: BTreeMap<GameObjectId, BTreeMap<CounterKindV1, u32>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct AttachmentTimestampV1 {
    pub revision: StateRevision,
    pub operation_ordinal: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttachmentV1 {
    pub target: GameObjectId,
    pub timestamp: AttachmentTimestampV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AttachmentStateV1 {
    pub by_source: BTreeMap<GameObjectId, AttachmentV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FaceStateV1 {
    pub faces: BTreeMap<GameObjectId, u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AbilityAuthorityV1 {
    pub source: GameObjectId,
    pub ability_key: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AbilityAuthorityStateV1 {
    pub by_instance: BTreeMap<AbilityInstanceId, AbilityAuthorityV1>,
}

/// What the rules record about one permanent beyond its object: the turn
/// since which its controller has controlled it (CR 302.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PermanentState {
    pub controlled_since_turn: u64,
}

/// One entry per permanent on the battlefield, keyed by its object.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PermanentsState {
    pub permanents: BTreeMap<GameObjectId, PermanentState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CardRulesAuthoritativeStateV1 {
    pub mana: ManaStateV1,
    pub turn_history: TurnHistoryStateV1,
    pub counters: CounterStateV1,
    pub attachments: AttachmentStateV1,
    pub faces: FaceStateV1,
    pub abilities: AbilityAuthorityStateV1,
    pub permanents: PermanentsState,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CardRulesStateError {
    #[error("invalid card-rules state structure")]
    InvalidStructure,
}

fn array(values: impl IntoIterator<Item = Value>) -> Value {
    Value::Array(values.into_iter().collect())
}

fn unsigned(value: u64) -> Value {
    Value::Unsigned(value)
}

impl ManaStateV1 {
    pub fn validate(&self) -> Result<(), CardRulesStateError> {
        Ok(())
    }

    fn to_value(&self) -> Value {
        array(self.pools.iter().map(|(player, pool)| {
            array([
                unsigned(player.0),
                array(pool.unrestricted.map(|count| unsigned(u64::from(count)))),
                array(
                    pool.creature_spell_only
                        .map(|count| unsigned(u64::from(count))),
                ),
            ])
        }))
    }
}

impl TurnHistoryStateV1 {
    pub fn validate(&self) -> Result<(), CardRulesStateError> {
        if self.players.values().any(|history| {
            history.land_plays_used > 1
                || history.noncreature_spells_cast > history.spells_cast_total
        }) {
            return Err(CardRulesStateError::InvalidStructure);
        }
        Ok(())
    }

    fn to_value(&self) -> Value {
        array([
            unsigned(self.turn_number),
            array(self.players.iter().map(|(player, history)| {
                array([
                    unsigned(player.0),
                    unsigned(u64::from(history.land_plays_used)),
                    unsigned(u64::from(history.spells_cast_total)),
                    unsigned(u64::from(history.noncreature_spells_cast)),
                    Value::Bool(history.lost_life_this_turn),
                    unsigned(u64::from(history.red_noncombat_damage_dealt)),
                    Value::Bool(history.permanent_card_to_graveyard),
                ])
            })),
            array(
                self.target_occurrences
                    .iter()
                    .map(|(object, player)| array([unsigned(object.0), unsigned(player.0)])),
            ),
            array(
                self.once_ability_used
                    .iter()
                    .map(|(object, key)| array([unsigned(object.0), unsigned(u64::from(*key))])),
            ),
        ])
    }
}

impl CounterStateV1 {
    fn to_value(&self) -> Value {
        array(self.counters.iter().map(|(object, counters)| {
            array([
                unsigned(object.0),
                array(counters.iter().map(|(kind, count)| {
                    array([unsigned(*kind as u64), unsigned(u64::from(*count))])
                })),
            ])
        }))
    }
}

impl AttachmentStateV1 {
    fn validate(&self) -> Result<(), CardRulesStateError> {
        let mut timestamps = BTreeSet::new();
        if self
            .by_source
            .values()
            .any(|edge| !timestamps.insert(edge.timestamp))
        {
            return Err(CardRulesStateError::InvalidStructure);
        }
        Ok(())
    }

    fn to_value(&self) -> Value {
        array(self.by_source.iter().map(|(source, edge)| {
            array([
                unsigned(source.0),
                unsigned(edge.target.0),
                unsigned(edge.timestamp.revision.0),
                unsigned(u64::from(edge.timestamp.operation_ordinal)),
            ])
        }))
    }
}

impl FaceStateV1 {
    fn to_value(&self) -> Value {
        array(
            self.faces
                .iter()
                .map(|(object, face)| array([unsigned(object.0), unsigned(u64::from(*face))])),
        )
    }
}

impl PermanentsState {
    fn to_value(&self) -> Value {
        array(self.permanents.iter().map(|(object, permanent)| {
            array([
                unsigned(object.0),
                unsigned(permanent.controlled_since_turn),
            ])
        }))
    }
}

impl AbilityAuthorityStateV1 {
    fn validate(&self) -> Result<(), CardRulesStateError> {
        let mut semantic_keys = BTreeSet::new();
        for authority in self.by_instance.values() {
            if !semantic_keys.insert((authority.source, authority.ability_key)) {
                return Err(CardRulesStateError::InvalidStructure);
            }
        }
        Ok(())
    }

    fn to_value(&self) -> Value {
        array(self.by_instance.iter().map(|(instance, authority)| {
            array([
                unsigned(instance.0),
                unsigned(authority.source.0),
                unsigned(u64::from(authority.ability_key)),
            ])
        }))
    }
}

impl CardRulesAuthoritativeStateV1 {
    pub fn validate(&self) -> Result<(), CardRulesStateError> {
        self.mana.validate()?;
        self.turn_history.validate()?;
        self.abilities.validate()?;
        self.attachments.validate()?;
        if self
            .counters
            .counters
            .values()
            .any(|values| values.is_empty() || values.values().any(|n| *n == 0))
        {
            return Err(CardRulesStateError::InvalidStructure);
        }
        Ok(())
    }

    pub fn canonical_value(&self) -> Result<Value, CardRulesStateError> {
        self.validate()?;
        // Exhaustive: a new family cannot be left out of the digest.
        let Self {
            mana,
            turn_history,
            counters,
            attachments,
            faces,
            abilities,
            permanents,
        } = self;
        Ok(array([
            Value::Text("card-rules-authoritative-state.v1".to_owned()),
            mana.to_value(),
            turn_history.to_value(),
            counters.to_value(),
            attachments.to_value(),
            faces.to_value(),
            abilities.to_value(),
            permanents.to_value(),
        ]))
    }
}
