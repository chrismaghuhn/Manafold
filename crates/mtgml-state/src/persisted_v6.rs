//! Detached, typed persistence state for the proposed FullStateDigestV6.
//!
//! This module is not connected to `EngineState`, checkpoint admission, or any
//! current digest alias. V5 continues to own the current runtime identity.

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CardRulesAuthoritativeStateV1 {
    pub mana: ManaStateV1,
    pub turn_history: TurnHistoryStateV1,
    pub counters: CounterStateV1,
    pub attachments: AttachmentStateV1,
    pub faces: FaceStateV1,
    pub abilities: AbilityAuthorityStateV1,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PersistedV6Error {
    #[error("invalid FullStateDigestV6 persisted structure")]
    InvalidStructure,
    #[error("noncanonical FullStateDigestV6 persisted input")]
    NonCanonical,
    #[error("canonical CBOR error: {0}")]
    Cbor(#[from] mtgml_persistence::PersistenceDecodeErrorV1),
}

fn array(values: impl IntoIterator<Item = Value>) -> Value {
    Value::Array(values.into_iter().collect())
}

fn unsigned(value: u64) -> Value {
    Value::Unsigned(value)
}

fn parse_array(value: &Value, len: usize) -> Result<&[Value], PersistedV6Error> {
    match value {
        Value::Array(values) if values.len() == len => Ok(values),
        _ => Err(PersistedV6Error::InvalidStructure),
    }
}

fn parse_u64(value: &Value) -> Result<u64, PersistedV6Error> {
    match value {
        Value::Unsigned(value) => Ok(*value),
        _ => Err(PersistedV6Error::InvalidStructure),
    }
}

fn parse_u32(value: &Value) -> Result<u32, PersistedV6Error> {
    u32::try_from(parse_u64(value)?).map_err(|_| PersistedV6Error::InvalidStructure)
}

fn parse_list(value: &Value) -> Result<&[Value], PersistedV6Error> {
    match value {
        Value::Array(values) => Ok(values),
        _ => Err(PersistedV6Error::InvalidStructure),
    }
}

fn parse_text(value: &Value, expected: &str) -> Result<(), PersistedV6Error> {
    if value == &Value::Text(expected.to_owned()) {
        Ok(())
    } else {
        Err(PersistedV6Error::InvalidStructure)
    }
}

impl ManaStateV1 {
    pub fn validate(&self) -> Result<(), PersistedV6Error> {
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

    fn from_value(value: &Value) -> Result<Self, PersistedV6Error> {
        let Value::Array(rows) = value else {
            return Err(PersistedV6Error::InvalidStructure);
        };
        let mut pools = BTreeMap::new();
        let mut previous = None;
        for row in rows {
            let row = parse_array(row, 3)?;
            let player = PlayerId(parse_u64(&row[0])?);
            if previous.is_some_and(|id| id >= player) {
                return Err(PersistedV6Error::InvalidStructure);
            }
            previous = Some(player);
            let parse_bucket = |bucket: &Value| -> Result<[u32; 6], PersistedV6Error> {
                let values = parse_array(bucket, 6)?;
                let mut result = [0; 6];
                for (index, value) in values.iter().enumerate() {
                    result[index] = parse_u32(value)?;
                }
                Ok(result)
            };
            let pool = ManaPoolV1 {
                unrestricted: parse_bucket(&row[1])?,
                creature_spell_only: parse_bucket(&row[2])?,
            };
            pools.insert(player, pool);
        }
        Ok(Self { pools })
    }
}

impl TurnHistoryStateV1 {
    pub fn validate(&self) -> Result<(), PersistedV6Error> {
        if self
            .players
            .values()
            .any(|history| history.land_plays_used > 1)
        {
            return Err(PersistedV6Error::InvalidStructure);
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

    fn from_value(value: &Value) -> Result<Self, PersistedV6Error> {
        let fields = parse_array(value, 4)?;
        let turn_number = parse_u64(&fields[0])?;
        let Value::Array(player_rows) = &fields[1] else {
            return Err(PersistedV6Error::InvalidStructure);
        };
        let mut players = BTreeMap::new();
        let mut previous_player = None;
        for row in player_rows {
            let row = parse_array(row, 7)?;
            let player = PlayerId(parse_u64(&row[0])?);
            if previous_player.is_some_and(|id| id >= player) {
                return Err(PersistedV6Error::InvalidStructure);
            }
            previous_player = Some(player);
            let land_plays_used = u8::try_from(parse_u64(&row[1])?)
                .map_err(|_| PersistedV6Error::InvalidStructure)?;
            if land_plays_used > 1 {
                return Err(PersistedV6Error::InvalidStructure);
            }
            let Value::Bool(lost_life_this_turn) = row[4] else {
                return Err(PersistedV6Error::InvalidStructure);
            };
            let Value::Bool(permanent_card_to_graveyard) = row[6] else {
                return Err(PersistedV6Error::InvalidStructure);
            };
            players.insert(
                player,
                PlayerTurnHistoryV1 {
                    land_plays_used,
                    spells_cast_total: parse_u32(&row[2])?,
                    noncreature_spells_cast: parse_u32(&row[3])?,
                    lost_life_this_turn,
                    red_noncombat_damage_dealt: parse_u32(&row[5])?,
                    permanent_card_to_graveyard,
                },
            );
        }
        let mut target_occurrences = BTreeSet::new();
        let mut previous_target = None;
        for row in parse_list(&fields[2])? {
            let row = parse_array(row, 2)?;
            let pair = (
                GameObjectId(parse_u64(&row[0])?),
                PlayerId(parse_u64(&row[1])?),
            );
            if previous_target.is_some_and(|previous| previous >= pair) {
                return Err(PersistedV6Error::InvalidStructure);
            }
            previous_target = Some(pair);
            target_occurrences.insert(pair);
        }
        let mut once_ability_used = BTreeSet::new();
        let mut previous_ability = None;
        for row in parse_list(&fields[3])? {
            let row = parse_array(row, 2)?;
            let pair = (GameObjectId(parse_u64(&row[0])?), parse_u32(&row[1])?);
            if previous_ability.is_some_and(|previous| previous >= pair) {
                return Err(PersistedV6Error::InvalidStructure);
            }
            previous_ability = Some(pair);
            once_ability_used.insert(pair);
        }
        Ok(Self {
            turn_number,
            players,
            target_occurrences,
            once_ability_used,
        })
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

    fn from_value(value: &Value) -> Result<Self, PersistedV6Error> {
        let Value::Array(rows) = value else {
            return Err(PersistedV6Error::InvalidStructure);
        };
        let mut counters = BTreeMap::new();
        let mut previous_object = None;
        for row in rows {
            let row = parse_array(row, 2)?;
            let object = GameObjectId(parse_u64(&row[0])?);
            if previous_object.is_some_and(|previous| previous >= object) {
                return Err(PersistedV6Error::InvalidStructure);
            }
            previous_object = Some(object);
            let Value::Array(entries) = &row[1] else {
                return Err(PersistedV6Error::InvalidStructure);
            };
            if entries.is_empty() {
                return Err(PersistedV6Error::InvalidStructure);
            }
            let mut object_counters = BTreeMap::new();
            let mut previous_kind = None;
            for entry in entries {
                let entry = parse_array(entry, 2)?;
                let kind = match parse_u64(&entry[0])? {
                    0 => CounterKindV1::PlusOnePlusOne,
                    1 => CounterKindV1::MinusOneMinusOne,
                    2 => CounterKindV1::Lore,
                    _ => return Err(PersistedV6Error::InvalidStructure),
                };
                if previous_kind.is_some_and(|previous| previous >= kind) {
                    return Err(PersistedV6Error::InvalidStructure);
                }
                previous_kind = Some(kind);
                let count = parse_u32(&entry[1])?;
                if count == 0 {
                    return Err(PersistedV6Error::InvalidStructure);
                }
                object_counters.insert(kind, count);
            }
            counters.insert(object, object_counters);
        }
        Ok(Self { counters })
    }
}

impl AttachmentStateV1 {
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

    fn from_value(value: &Value) -> Result<Self, PersistedV6Error> {
        let Value::Array(rows) = value else {
            return Err(PersistedV6Error::InvalidStructure);
        };
        let mut by_source = BTreeMap::new();
        let mut previous_source = None;
        for row in rows {
            let row = parse_array(row, 4)?;
            let source = GameObjectId(parse_u64(&row[0])?);
            if previous_source.is_some_and(|previous| previous >= source) {
                return Err(PersistedV6Error::InvalidStructure);
            }
            previous_source = Some(source);
            by_source.insert(
                source,
                AttachmentV1 {
                    target: GameObjectId(parse_u64(&row[1])?),
                    timestamp: AttachmentTimestampV1 {
                        revision: StateRevision(parse_u64(&row[2])?),
                        operation_ordinal: parse_u32(&row[3])?,
                    },
                },
            );
        }
        Ok(Self { by_source })
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

    fn from_value(value: &Value) -> Result<Self, PersistedV6Error> {
        let Value::Array(rows) = value else {
            return Err(PersistedV6Error::InvalidStructure);
        };
        let mut faces = BTreeMap::new();
        let mut previous_object = None;
        for row in rows {
            let row = parse_array(row, 2)?;
            let object = GameObjectId(parse_u64(&row[0])?);
            if previous_object.is_some_and(|previous| previous >= object) {
                return Err(PersistedV6Error::InvalidStructure);
            }
            previous_object = Some(object);
            faces.insert(object, parse_u32(&row[1])?);
        }
        Ok(Self { faces })
    }
}

impl AbilityAuthorityStateV1 {
    fn validate(&self) -> Result<(), PersistedV6Error> {
        let mut semantic_keys = BTreeSet::new();
        for authority in self.by_instance.values() {
            if !semantic_keys.insert((authority.source, authority.ability_key)) {
                return Err(PersistedV6Error::InvalidStructure);
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

    fn from_value(value: &Value) -> Result<Self, PersistedV6Error> {
        let Value::Array(rows) = value else {
            return Err(PersistedV6Error::InvalidStructure);
        };
        let mut by_instance = BTreeMap::new();
        let mut semantic_keys = BTreeSet::new();
        let mut previous_instance = None;
        for row in rows {
            let row = parse_array(row, 3)?;
            let instance = AbilityInstanceId(parse_u64(&row[0])?);
            if previous_instance.is_some_and(|previous| previous >= instance) {
                return Err(PersistedV6Error::InvalidStructure);
            }
            previous_instance = Some(instance);
            let source = GameObjectId(parse_u64(&row[1])?);
            let ability_key = parse_u32(&row[2])?;
            if !semantic_keys.insert((source, ability_key)) {
                return Err(PersistedV6Error::InvalidStructure);
            }
            by_instance.insert(
                instance,
                AbilityAuthorityV1 {
                    source,
                    ability_key,
                },
            );
        }
        Ok(Self { by_instance })
    }
}

impl CardRulesAuthoritativeStateV1 {
    pub fn validate(&self) -> Result<(), PersistedV6Error> {
        self.mana.validate()?;
        self.turn_history.validate()?;
        self.abilities.validate()?;
        if self
            .counters
            .counters
            .values()
            .any(|values| values.is_empty() || values.values().any(|n| *n == 0))
        {
            return Err(PersistedV6Error::InvalidStructure);
        }
        Ok(())
    }

    pub fn canonical_value(&self) -> Result<Value, PersistedV6Error> {
        self.validate()?;
        Ok(array([
            Value::Text("card-rules-authoritative-state.v1".to_owned()),
            self.mana.to_value(),
            self.turn_history.to_value(),
            self.counters.to_value(),
            self.attachments.to_value(),
            self.faces.to_value(),
            self.abilities.to_value(),
        ]))
    }

    pub fn from_value(value: &Value) -> Result<Self, PersistedV6Error> {
        let fields = parse_array(value, 7)?;
        parse_text(&fields[0], "card-rules-authoritative-state.v1")?;
        let state = Self {
            mana: ManaStateV1::from_value(&fields[1])?,
            turn_history: TurnHistoryStateV1::from_value(&fields[2])?,
            counters: CounterStateV1::from_value(&fields[3])?,
            attachments: AttachmentStateV1::from_value(&fields[4])?,
            faces: FaceStateV1::from_value(&fields[5])?,
            abilities: AbilityAuthorityStateV1::from_value(&fields[6])?,
        };
        state.validate()?;
        Ok(state)
    }
}

/// Closed persisted execution representation used in the V6 digest input.
/// The private `Value` is admitted only after the V3 fixed-shape validator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedExecutionV3(Value);

impl PersistedExecutionV3 {
    pub fn from_value(value: Value) -> Result<Self, PersistedV6Error> {
        validate_execution_v3(&value)?;
        Ok(Self(value))
    }

    pub fn canonical_value(&self) -> &Value {
        &self.0
    }
}

/// V5 input values are retained mechanically in their exact positional form;
/// only the top-level identity and typed state-family tail are successor data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FullStateDigestInputV6 {
    pub(crate) revision: u64,
    pub(crate) core_v1: Value,
    pub(crate) zones_v1: Value,
    pub(crate) allocators_v3: Value,
    pub(crate) execution_v3: PersistedExecutionV3,
    pub(crate) random_v1: Value,
    pub(crate) knowledge_v2: Value,
    pub(crate) perspective_identities_v2: Value,
    pub(crate) combat: Value,
    pub(crate) foundation_sources: Value,
    pub(crate) format_v1: Value,
    pub(crate) card_rules_state: CardRulesAuthoritativeStateV1,
}

impl FullStateDigestInputV6 {
    pub fn canonical_value(&self) -> Result<Value, PersistedV6Error> {
        validate_legacy_components([
            &self.core_v1,
            &self.zones_v1,
            &self.allocators_v3,
            &self.random_v1,
            &self.knowledge_v2,
            &self.perspective_identities_v2,
            &self.combat,
            &self.foundation_sources,
            &self.format_v1,
        ])?;
        Ok(array([
            Value::Text("full-state-digest-input.v6".to_owned()),
            Value::Text("mtgml.full-state-digest.v6".to_owned()),
            unsigned(self.revision),
            self.core_v1.clone(),
            self.zones_v1.clone(),
            self.allocators_v3.clone(),
            self.execution_v3.canonical_value().clone(),
            self.random_v1.clone(),
            self.knowledge_v2.clone(),
            self.perspective_identities_v2.clone(),
            self.combat.clone(),
            self.foundation_sources.clone(),
            self.format_v1.clone(),
            self.card_rules_state.canonical_value()?,
        ]))
    }

    pub fn canonical_payload(&self) -> Result<Vec<u8>, PersistedV6Error> {
        mtgml_persistence::cbor::encode_canonical(&self.canonical_value()?).map_err(Into::into)
    }

    pub fn from_canonical_value(value: &Value) -> Result<Self, PersistedV6Error> {
        let fields = parse_array(value, 14)?;
        parse_text(&fields[0], "full-state-digest-input.v6")?;
        parse_text(&fields[1], "mtgml.full-state-digest.v6")?;
        validate_legacy_components([
            &fields[3],
            &fields[4],
            &fields[5],
            &fields[7],
            &fields[8],
            &fields[9],
            &fields[10],
            &fields[11],
            &fields[12],
        ])?;
        let input = Self {
            revision: parse_u64(&fields[2])?,
            core_v1: fields[3].clone(),
            zones_v1: fields[4].clone(),
            allocators_v3: fields[5].clone(),
            execution_v3: PersistedExecutionV3::from_value(fields[6].clone())?,
            random_v1: fields[7].clone(),
            knowledge_v2: fields[8].clone(),
            perspective_identities_v2: fields[9].clone(),
            combat: fields[10].clone(),
            foundation_sources: fields[11].clone(),
            format_v1: fields[12].clone(),
            card_rules_state: CardRulesAuthoritativeStateV1::from_value(&fields[13])?,
        };
        Ok(input)
    }

    #[cfg(test)]
    pub(crate) fn from_phase2_v5_payload(
        payload: &[u8],
        card_rules_state: CardRulesAuthoritativeStateV1,
    ) -> Result<Self, PersistedV6Error> {
        let value = mtgml_persistence::cbor::decode_canonical(payload)?;
        let fields = parse_array(&value, 13)?;
        parse_text(&fields[0], "full-state-digest-input.v5")?;
        parse_text(&fields[1], mtgml_model::FullStateDigestV5::DOMAIN)?;
        let revision = parse_u64(&fields[2])?;
        let input = Self {
            revision,
            core_v1: fields[3].clone(),
            zones_v1: fields[4].clone(),
            allocators_v3: fields[5].clone(),
            execution_v3: PersistedExecutionV3::from_value(fields[6].clone())?,
            random_v1: fields[7].clone(),
            knowledge_v2: fields[8].clone(),
            perspective_identities_v2: fields[9].clone(),
            combat: fields[10].clone(),
            foundation_sources: fields[11].clone(),
            format_v1: fields[12].clone(),
            card_rules_state,
        };
        Ok(input)
    }
}

fn validate_legacy_components(components: [&Value; 9]) -> Result<(), PersistedV6Error> {
    let [core, zones, allocators, random, knowledge, perspective_identities, combat, foundation_sources, format] =
        components;
    validate_core_v5(core)?;
    validate_zones_v5(zones)?;
    for value in parse_array(allocators, 8)? {
        parse_u64(value)?;
    }
    validate_random_v5(random)?;
    validate_knowledge_v5(knowledge)?;
    validate_perspective_v5(perspective_identities)?;
    validate_combat_v5(combat)?;
    validate_foundation_sources_v5(foundation_sources)?;
    validate_format_v5(format)?;
    Ok(())
}

fn parse_bool(value: &Value) -> Result<(), PersistedV6Error> {
    if matches!(value, Value::Bool(_)) {
        Ok(())
    } else {
        Err(PersistedV6Error::InvalidStructure)
    }
}

fn parse_i64(value: &Value) -> Result<(), PersistedV6Error> {
    parse_i64_value(value).map(|_| ())
}

fn parse_i64_value(value: &Value) -> Result<i64, PersistedV6Error> {
    match value {
        Value::Signed(value) => Ok(*value),
        Value::Unsigned(value) => {
            i64::try_from(*value).map_err(|_| PersistedV6Error::InvalidStructure)
        }
        _ => Err(PersistedV6Error::InvalidStructure),
    }
}

fn validate_turn_position_v5(value: &Value) -> Result<(), PersistedV6Error> {
    let fields = parse_array(value, 2)?;
    let Value::Text(tag) = &fields[0] else {
        return Err(PersistedV6Error::InvalidStructure);
    };
    match tag.as_str() {
        "beginning" if matches!(&fields[1], Value::Text(step) if matches!(step.as_str(), "untap" | "upkeep" | "draw")) => {
            Ok(())
        }
        "combat" if matches!(&fields[1], Value::Text(step) if matches!(step.as_str(), "beginning_of_combat" | "declare_attackers" | "declare_blockers" | concat!("combat_", "damage") | "end_of_combat")) => {
            Ok(())
        }
        "ending" if matches!(&fields[1], Value::Text(step) if matches!(step.as_str(), "end_step" | "cleanup")) => {
            Ok(())
        }
        "precombat_main" | "postcombat_main" if matches!(fields[1], Value::Null) => Ok(()),
        _ => Err(PersistedV6Error::InvalidStructure),
    }
}

fn validate_core_v5(value: &Value) -> Result<(), PersistedV6Error> {
    let fields = parse_array(value, 5)?;
    let mut previous = None;
    for player in parse_list(&fields[0])? {
        let player = parse_array(player, 3)?;
        let id = parse_u64(&player[0])?;
        if previous.is_some_and(|last| last >= id) {
            return Err(PersistedV6Error::InvalidStructure);
        }
        previous = Some(id);
        parse_i64(&player[1])?;
        parse_bool(&player[2])?;
    }
    if previous.is_none() {
        return Err(PersistedV6Error::InvalidStructure);
    }
    parse_u64(&fields[1])?;
    parse_u64(&fields[2])?;
    validate_turn_position_v5(&fields[3])?;
    let priority = parse_array(&fields[4], 2)?;
    match &priority[0] {
        Value::Text(tag) if tag == "none" && matches!(priority[1], Value::Null) => Ok(()),
        Value::Text(tag) if tag == "held_by" => {
            let held = parse_array(&priority[1], 2)?;
            parse_u64(&held[0])?;
            if parse_u64(&held[1])? > 1 {
                return Err(PersistedV6Error::InvalidStructure);
            }
            Ok(())
        }
        _ => Err(PersistedV6Error::InvalidStructure),
    }
}

fn validate_zone_kind_v5(value: &Value) -> Result<(), PersistedV6Error> {
    if matches!(value, Value::Text(tag) if matches!(tag.as_str(), "library" | "hand" | "battlefield" | "graveyard" | "exile" | "stack" | "command" | "ante" | "outside"))
    {
        Ok(())
    } else {
        Err(PersistedV6Error::InvalidStructure)
    }
}

fn validate_visibility_v5(value: &Value) -> Result<(), PersistedV6Error> {
    if matches!(value, Value::Text(tag) if matches!(tag.as_str(), "public" | "owner_only" | "face_down" | "private_group"))
    {
        Ok(())
    } else {
        Err(PersistedV6Error::InvalidStructure)
    }
}

fn validate_optional_u64(value: &Value) -> Result<(), PersistedV6Error> {
    if matches!(value, Value::Null) {
        Ok(())
    } else {
        parse_u64(value).map(|_| ())
    }
}

fn validate_zone_key_v5(value: &Value) -> Result<(), PersistedV6Error> {
    let fields = parse_array(value, 4)?;
    validate_zone_kind_v5(&fields[0])?;
    validate_optional_u64(&fields[1])?;
    validate_visibility_v5(&fields[2])?;
    if !matches!(fields[3], Value::Null | Value::Text(_)) {
        return Err(PersistedV6Error::InvalidStructure);
    }
    Ok(())
}

fn validate_zone_position_v5(value: &Value) -> Result<(), PersistedV6Error> {
    let fields = parse_array(value, 2)?;
    match &fields[0] {
        Value::Text(tag) if tag == "unordered" && matches!(fields[1], Value::Null) => Ok(()),
        Value::Text(tag) if matches!(tag.as_str(), "top" | "bottom" | "index") => {
            parse_u32(&fields[1]).map(|_| ())
        }
        _ => Err(PersistedV6Error::InvalidStructure),
    }
}

fn validate_zone_location_v5(value: &Value) -> Result<(), PersistedV6Error> {
    let fields = parse_array(value, 5)?;
    validate_zone_kind_v5(&fields[0])?;
    validate_optional_u64(&fields[1])?;
    validate_zone_position_v5(&fields[2])?;
    validate_visibility_v5(&fields[3])?;
    if !matches!(fields[4], Value::Null | Value::Text(_)) {
        return Err(PersistedV6Error::InvalidStructure);
    }
    Ok(())
}

fn validate_zones_v5(value: &Value) -> Result<(), PersistedV6Error> {
    let fields = parse_array(value, 5)?;
    let mut previous_object = None;
    for row in parse_list(&fields[0])? {
        let row = parse_array(row, 7)?;
        let id = parse_u64(&row[0])?;
        if previous_object.is_some_and(|last| last >= id) {
            return Err(PersistedV6Error::InvalidStructure);
        }
        previous_object = Some(id);
        validate_optional_u64(&row[1])?;
        for index in [2, 3, 4] {
            parse_u64(&row[index])?;
        }
        parse_bool(&row[5])?;
        parse_bool(&row[6])?;
    }
    let mut previous_location = None;
    for row in parse_list(&fields[1])? {
        let row = parse_array(row, 2)?;
        let id = parse_u64(&row[0])?;
        if previous_location.is_some_and(|last| last >= id) {
            return Err(PersistedV6Error::InvalidStructure);
        }
        previous_location = Some(id);
        validate_zone_location_v5(&row[1])?;
    }
    let mut previous_key_bytes: Option<Vec<u8>> = None;
    for row in parse_list(&fields[2])? {
        let row = parse_array(row, 2)?;
        validate_zone_key_v5(&row[0])?;
        let key_bytes = mtgml_persistence::cbor::encode_canonical(&row[0])?;
        if previous_key_bytes
            .as_ref()
            .is_some_and(|last| last >= &key_bytes)
        {
            return Err(PersistedV6Error::InvalidStructure);
        }
        previous_key_bytes = Some(key_bytes);
        validate_u64_array(&row[1])?;
    }
    let mut previous_stack = None;
    for row in parse_list(&fields[3])? {
        let row = parse_array(row, 4)?;
        let id = parse_u64(&row[0])?;
        if previous_stack.is_some_and(|last| last >= id) {
            return Err(PersistedV6Error::InvalidStructure);
        }
        previous_stack = Some(id);
        parse_u64(&row[1])?;
        validate_optional_u64(&row[2])?;
        validate_optional_u64(&row[3])?;
    }
    validate_u64_array(&fields[4])
}

fn validate_random_v5(value: &Value) -> Result<(), PersistedV6Error> {
    let fields = parse_array(value, 3)?;
    if fields[0] != Value::Text("mtgml.rng.v1".into())
        || !matches!(&fields[1], Value::Bytes(seed) if seed.len() == 32)
    {
        return Err(PersistedV6Error::InvalidStructure);
    }
    let mut previous_key: Option<&[u8]> = None;
    for row in parse_list(&fields[2])? {
        let row = parse_array(row, 2)?;
        let Value::Bytes(key) = &row[0] else {
            return Err(PersistedV6Error::InvalidStructure);
        };
        mtgml_random::RandomStreamKeyV1::from_canonical_bytes(key)
            .map_err(|_| PersistedV6Error::InvalidStructure)?;
        if previous_key.is_some_and(|last| last >= key.as_slice()) {
            return Err(PersistedV6Error::InvalidStructure);
        }
        previous_key = Some(key);
        parse_u64(&row[1])?;
    }
    Ok(())
}

fn validate_provenance_v5(
    value: &Value,
    next_visible_sequence: u64,
) -> Result<(), PersistedV6Error> {
    let fields = parse_array(value, 2)?;
    let Value::Text(tag) = &fields[0] else {
        return Err(PersistedV6Error::InvalidStructure);
    };
    match tag.as_str() {
        "initial_configuration" if matches!(fields[1], Value::Null) => Ok(()),
        "observed" => {
            let observed = parse_array(&fields[1], 3)?;
            let Value::Text(channel) = &observed[0] else {
                return Err(PersistedV6Error::InvalidStructure);
            };
            if parse_u64(&observed[1])? >= next_visible_sequence {
                return Err(PersistedV6Error::InvalidStructure);
            }
            let Value::Text(cause) = &observed[2] else {
                return Err(PersistedV6Error::InvalidStructure);
            };
            if !matches!(
                (channel.as_str(), cause.as_str()),
                ("public", "public_event" | "explicit_reveal")
                    | ("private", "private_look" | "own_private_identity")
            ) {
                return Err(PersistedV6Error::InvalidStructure);
            }
            Ok(())
        }
        _ => Err(PersistedV6Error::InvalidStructure),
    }
}

fn validate_location_fact_v5(
    value: &Value,
    next_visible_sequence: u64,
) -> Result<(), PersistedV6Error> {
    let fields = parse_array(value, 2)?;
    validate_zone_location_v5(&fields[0])?;
    validate_provenance_v5(&fields[1], next_visible_sequence)
}

fn validate_history_facts_v5(
    value: &Value,
    next_visible_sequence: u64,
) -> Result<(), PersistedV6Error> {
    for fact in parse_list(value)? {
        validate_location_fact_v5(fact, next_visible_sequence)?;
    }
    Ok(())
}

fn validate_knowledge_v5(value: &Value) -> Result<(), PersistedV6Error> {
    let mut previous_player = None;
    for row in parse_list(value)? {
        let row = parse_array(row, 4)?;
        let player = parse_u64(&row[0])?;
        if previous_player.is_some_and(|last| last >= player) {
            return Err(PersistedV6Error::InvalidStructure);
        }
        previous_player = Some(player);
        let next_visible_sequence = parse_u64(&row[1])?;
        let mut previous_active = None;
        for active in parse_list(&row[2])? {
            let active = parse_array(active, 6)?;
            let opaque = parse_u64(&active[0])?;
            if previous_active.is_some_and(|last| last >= opaque) {
                return Err(PersistedV6Error::InvalidStructure);
            }
            previous_active = Some(opaque);
            validate_optional_u64(&active[1])?;
            validate_optional_u64(&active[2])?;
            if !matches!(active[3], Value::Null) {
                validate_location_fact_v5(&active[3], next_visible_sequence)?;
            }
            validate_history_facts_v5(&active[4], next_visible_sequence)?;
            validate_provenance_v5(&active[5], next_visible_sequence)?;
        }
        let mut previous_retired = None;
        for retired in parse_list(&row[3])? {
            let retired = parse_array(retired, 7)?;
            let opaque = parse_u64(&retired[0])?;
            if previous_retired.is_some_and(|last| last >= opaque) {
                return Err(PersistedV6Error::InvalidStructure);
            }
            previous_retired = Some(opaque);
            validate_optional_u64(&retired[1])?;
            validate_optional_u64(&retired[2])?;
            if !matches!(retired[3], Value::Null) {
                validate_location_fact_v5(&retired[3], next_visible_sequence)?;
            }
            validate_history_facts_v5(&retired[4], next_visible_sequence)?;
            validate_provenance_v5(&retired[5], next_visible_sequence)?;
            let invalidation = parse_array(&retired[6], 2)?;
            if matches!(&invalidation[0], Value::Array(values) if values.first() == Some(&Value::Text("initial_configuration".into())))
            {
                return Err(PersistedV6Error::InvalidStructure);
            }
            validate_provenance_v5(&invalidation[0], next_visible_sequence)?;
            if !matches!(&invalidation[1], Value::Text(reason) if matches!(reason.as_str(), "hidden_transition" | "randomization" | "shuffle" | "explicit_forget"))
            {
                return Err(PersistedV6Error::InvalidStructure);
            }
        }
    }
    Ok(())
}

fn validate_pair_rows(value: &Value) -> Result<(), PersistedV6Error> {
    let mut previous = None;
    for row in parse_list(value)? {
        let row = parse_array(row, 2)?;
        let pair = (parse_u64(&row[0])?, parse_u64(&row[1])?);
        if previous.is_some_and(|last| last >= pair) {
            return Err(PersistedV6Error::InvalidStructure);
        }
        previous = Some(pair);
    }
    Ok(())
}

fn validate_id_set(value: &Value) -> Result<(), PersistedV6Error> {
    let mut previous = None;
    for item in parse_list(value)? {
        let id = parse_u64(item)?;
        if previous.is_some_and(|last| last >= id) {
            return Err(PersistedV6Error::InvalidStructure);
        }
        previous = Some(id);
    }
    Ok(())
}

fn validate_perspective_v5(value: &Value) -> Result<(), PersistedV6Error> {
    let mut previous_player = None;
    for row in parse_list(value)? {
        let row = parse_array(row, 8)?;
        let player = parse_u64(&row[0])?;
        if previous_player.is_some_and(|last| last >= player) {
            return Err(PersistedV6Error::InvalidStructure);
        }
        previous_player = Some(player);
        validate_pair_rows(&row[1])?;
        validate_pair_rows(&row[2])?;
        parse_u64(&row[3])?;
        parse_u64(&row[4])?;
        parse_u64(&row[5])?;
        validate_id_set(&row[6])?;
        validate_id_set(&row[7])?;
    }
    Ok(())
}

fn validate_combat_v5(value: &Value) -> Result<(), PersistedV6Error> {
    if matches!(value, Value::Null) {
        return Ok(());
    }
    let fields = match value {
        Value::Array(fields) if fields.len() == 3 || fields.len() == 5 => fields,
        _ => return Err(PersistedV6Error::InvalidStructure),
    };
    parse_u64(&fields[0])?;
    validate_id_set(&fields[1])?;
    let mut previous_attacker = None;
    for row in parse_list(&fields[2])? {
        let row = parse_array(row, 2)?;
        let attacker = parse_u64(&row[0])?;
        if previous_attacker.is_some_and(|last| last >= attacker) {
            return Err(PersistedV6Error::InvalidStructure);
        }
        previous_attacker = Some(attacker);
        validate_optional_u64(&row[1])?;
    }
    if fields.len() == 5 {
        validate_id_set(&fields[3])?;
        parse_bool(&fields[4])?;
    }
    Ok(())
}

fn validate_foundation_sources_v5(value: &Value) -> Result<(), PersistedV6Error> {
    let mut previous_object = None;
    for row in parse_list(value)? {
        let row = parse_array(row, 5)?;
        let object = parse_u64(&row[0])?;
        if previous_object.is_some_and(|last| last >= object) {
            return Err(PersistedV6Error::InvalidStructure);
        }
        previous_object = Some(object);
        if row[1] != Value::Text("creature".into()) {
            return Err(PersistedV6Error::InvalidStructure);
        }
        let characteristics = parse_array(&row[2], 2)?;
        if characteristics[0] != Value::Text("simple".into()) {
            return Err(PersistedV6Error::InvalidStructure);
        }
        let power_toughness = parse_array(&characteristics[1], 2)?;
        parse_i64(&power_toughness[0])?;
        parse_i64(&power_toughness[1])?;
        parse_u64(&row[3])?;
        let history = parse_array(&row[4], 2)?;
        match &history[0] {
            Value::Text(tag) if tag == "before_turn_start" => {
                parse_u64(&history[1])?;
            }
            Value::Text(tag) if tag == "during_turn" => {
                let during = parse_array(&history[1], 2)?;
                parse_u64(&during[0])?;
                validate_turn_position_v5(&during[1])?;
            }
            _ => return Err(PersistedV6Error::InvalidStructure),
        }
    }
    Ok(())
}

fn validate_format_v5(value: &Value) -> Result<(), PersistedV6Error> {
    let fields = parse_array(value, 2)?;
    match &fields[0] {
        Value::Text(tag) if tag == "none" && matches!(fields[1], Value::Null) => Ok(()),
        Value::Text(tag) if tag == "commander" => {
            let commander = parse_array(&fields[1], 3)?;
            let mut previous_player = None;
            for row in parse_list(&commander[0])? {
                let row = parse_array(row, 2)?;
                let player = parse_u64(&row[0])?;
                if previous_player.is_some_and(|previous| previous >= player) {
                    return Err(PersistedV6Error::InvalidStructure);
                }
                previous_player = Some(player);
                validate_id_set(&row[1])?;
            }
            let mut previous_card = None;
            for row in parse_list(&commander[1])? {
                let row = parse_array(row, 2)?;
                let card = parse_u64(&row[0])?;
                if previous_card.is_some_and(|previous| previous >= card) {
                    return Err(PersistedV6Error::InvalidStructure);
                }
                previous_card = Some(card);
                parse_u32(&row[1])?;
            }
            let mut previous_card = None;
            for row in parse_list(&commander[2])? {
                let row = parse_array(row, 2)?;
                let card = parse_u64(&row[0])?;
                if previous_card.is_some_and(|previous| previous >= card) {
                    return Err(PersistedV6Error::InvalidStructure);
                }
                previous_card = Some(card);
                let mut previous_player = None;
                for damage in parse_list(&row[1])? {
                    let damage = parse_array(damage, 2)?;
                    let player = parse_u64(&damage[0])?;
                    if previous_player.is_some_and(|last| last >= player) {
                        return Err(PersistedV6Error::InvalidStructure);
                    }
                    previous_player = Some(player);
                    parse_u32(&damage[1])?;
                }
            }
            Ok(())
        }
        _ => Err(PersistedV6Error::InvalidStructure),
    }
}

pub fn validate_execution_v3(value: &Value) -> Result<(), PersistedV6Error> {
    let fields = parse_array(value, 5)?;
    let pending = if matches!(fields[0], Value::Null) {
        None
    } else {
        validate_pending_request(&fields[0])?;
        Some(parse_array(&fields[0], 8)?)
    };
    let continuations = validate_continuations(&fields[1])?;
    validate_execution_continuation_link(pending, continuations)?;
    // The accepted V5 producer explicitly fails closed while these successor
    // payload families have no typed semantic authority. V6 carries the same
    // predecessor meaning and must not authorize arbitrary future records.
    for field in &fields[2..] {
        if !matches!(field, Value::Array(entries) if entries.is_empty()) {
            return Err(PersistedV6Error::InvalidStructure);
        }
    }
    Ok(())
}

fn validate_continuations(value: &Value) -> Result<Vec<&[Value]>, PersistedV6Error> {
    let Value::Array(records) = value else {
        return Err(PersistedV6Error::InvalidStructure);
    };
    if records.len() > 1 {
        return Err(PersistedV6Error::InvalidStructure);
    }
    let mut previous_id = None;
    let mut validated = Vec::with_capacity(records.len());
    for record in records {
        let record = parse_array(record, 5)?;
        let id = parse_u64(&record[0])?;
        if previous_id.is_some_and(|previous| previous >= id) {
            return Err(PersistedV6Error::InvalidStructure);
        }
        previous_id = Some(id);
        parse_u64(&record[1])?;
        parse_u64(&record[2])?;
        parse_u32(&record[3])?;
        let payload = parse_array(&record[4], 2)?;
        let Value::Text(tag) = &payload[0] else {
            return Err(PersistedV6Error::InvalidStructure);
        };
        match tag.as_str() {
            "synthetic_m2_assembly" => validate_assembly_payload(&payload[1])?,
            "magic_sba_graveyard_order_v1" => validate_sba_order_payload(&payload[1])?,
            _ => return Err(PersistedV6Error::InvalidStructure),
        }
        validated.push(record);
    }
    Ok(validated)
}

fn validate_execution_continuation_link(
    pending: Option<&[Value]>,
    continuations: Vec<&[Value]>,
) -> Result<(), PersistedV6Error> {
    let Some(record) = continuations.first().copied() else {
        if pending.is_some_and(|request| !matches!(request[7], Value::Null)) {
            return Err(PersistedV6Error::InvalidStructure);
        }
        return Ok(());
    };
    let Some(request) = pending else {
        return Err(PersistedV6Error::InvalidStructure);
    };
    let continuation_id = parse_u64(&record[0])?;
    if parse_u64(&request[7])? != continuation_id
        || parse_u64(&record[1])? != parse_u64(&request[3])?
        || parse_u64(&record[2])? > parse_u64(&request[2])?
    {
        return Err(PersistedV6Error::InvalidStructure);
    }
    let payload = parse_array(&record[4], 2)?;
    let expected_stage = match &payload[0] {
        Value::Text(tag) if tag == "synthetic_m2_assembly" => {
            let assembly = parse_array(&payload[1], 4)?;
            let stage = parse_array(&assembly[0], 2)?;
            let stage_index = match &stage[0] {
                Value::Text(stage) if stage == "choose_count" => 0,
                Value::Text(stage) if stage == "choose_members" => 1,
                Value::Text(stage) if stage == "order_members" => 2,
                _ => return Err(PersistedV6Error::InvalidStructure),
            };
            validate_assembly_request(assembly, request)?;
            stage_index
        }
        Value::Text(tag) if tag == "magic_sba_graveyard_order_v1" => {
            let sba = parse_array(&payload[1], 5)?;
            let next = parse_u32(&sba[3])?;
            let expected_created = parse_u64(&sba[0])?
                .checked_add(1)
                .ok_or(PersistedV6Error::InvalidStructure)?;
            let expected_revision = expected_created
                .checked_add(u64::from(next))
                .ok_or(PersistedV6Error::InvalidStructure)?;
            if parse_u64(&record[2])? != expected_created
                || parse_u64(&request[2])? != expected_revision
            {
                return Err(PersistedV6Error::InvalidStructure);
            }
            let owners = parse_list(&sba[2])?;
            let current_owner = owners
                .get(usize::try_from(next).map_err(|_| PersistedV6Error::InvalidStructure)?)
                .ok_or(PersistedV6Error::InvalidStructure)?;
            if parse_u64(current_owner)? != parse_u64(&request[3])? {
                return Err(PersistedV6Error::InvalidStructure);
            }
            validate_sba_order_request(request)?;
            u32::from(u16::try_from(next).unwrap_or(u16::MAX))
        }
        _ => return Err(PersistedV6Error::InvalidStructure),
    };
    if parse_u32(&record[3])? != expected_stage {
        return Err(PersistedV6Error::InvalidStructure);
    }
    Ok(())
}

fn validate_assembly_request(
    assembly: &[Value],
    request: &[Value],
) -> Result<(), PersistedV6Error> {
    let stage = parse_array(&assembly[0], 2)?;
    let Value::Text(stage_tag) = &stage[0] else {
        return Err(PersistedV6Error::InvalidStructure);
    };
    let Value::Array(candidates) = &request[6] else {
        return Err(PersistedV6Error::InvalidStructure);
    };
    let domain = parse_array(&request[5], 2)?;
    match stage_tag.as_str() {
        "choose_count" => {
            let Value::Text(domain_tag) = &domain[0] else {
                return Err(PersistedV6Error::InvalidStructure);
            };
            if domain_tag != "choose_number" || !candidates.is_empty() {
                return Err(PersistedV6Error::InvalidStructure);
            }
            let range = parse_array(&domain[1], 2)?;
            if parse_i64_value(&range[0])? != i64::from(crate::SYNTHETIC_COUNT_MIN)
                || parse_i64_value(&range[1])? != i64::from(crate::SYNTHETIC_COUNT_MAX)
            {
                return Err(PersistedV6Error::InvalidStructure);
            }
        }
        "choose_members" | "order_members" => {
            let count = parse_u32(&assembly[1])?;
            let expected_domain = if stage_tag == "choose_members" {
                "choose_many"
            } else {
                "order"
            };
            if !matches!(&domain[0], Value::Text(tag) if tag == expected_domain) {
                return Err(PersistedV6Error::InvalidStructure);
            }
            let range = parse_array(&domain[1], 2)?;
            if parse_u32(&range[0])? != count || parse_u32(&range[1])? != count {
                return Err(PersistedV6Error::InvalidStructure);
            }
            let expected_modes: Vec<u32> = if stage_tag == "choose_members" {
                (0..count).collect()
            } else {
                parse_u32_array(&assembly[2])?
            };
            if candidates.len() != expected_modes.len() {
                return Err(PersistedV6Error::InvalidStructure);
            }
            for (index, (candidate, expected_mode)) in
                candidates.iter().zip(expected_modes).enumerate()
            {
                let candidate = parse_array(candidate, 3)?;
                if parse_u32(&candidate[0])? != index as u32
                    || !is_mode_binding(&candidate[1], expected_mode)?
                    || !is_mode_binding(&candidate[2], expected_mode)?
                {
                    return Err(PersistedV6Error::InvalidStructure);
                }
            }
        }
        _ => return Err(PersistedV6Error::InvalidStructure),
    }
    Ok(())
}

fn parse_u32_array(value: &Value) -> Result<Vec<u32>, PersistedV6Error> {
    parse_list(value)?.iter().map(parse_u32).collect()
}

fn is_mode_binding(value: &Value, expected_mode: u32) -> Result<bool, PersistedV6Error> {
    let fields = parse_array(value, 2)?;
    Ok(
        matches!(&fields[0], Value::Text(tag) if tag == "select_mode")
            && parse_u32(&fields[1])? == expected_mode,
    )
}

fn validate_sba_order_request(request: &[Value]) -> Result<(), PersistedV6Error> {
    let Value::Array(candidates) = &request[6] else {
        return Err(PersistedV6Error::InvalidStructure);
    };
    let domain = parse_array(&request[5], 2)?;
    if !matches!(&domain[0], Value::Text(tag) if tag == "order") {
        return Err(PersistedV6Error::InvalidStructure);
    }
    let range = parse_array(&domain[1], 2)?;
    let count = u32::try_from(candidates.len()).map_err(|_| PersistedV6Error::InvalidStructure)?;
    if parse_u32(&range[0])? != count || parse_u32(&range[1])? != count {
        return Err(PersistedV6Error::InvalidStructure);
    }
    for candidate in candidates {
        let candidate = parse_array(candidate, 3)?;
        for field in [&candidate[1], &candidate[2]] {
            let fields = parse_array(field, 2)?;
            if !matches!(&fields[0], Value::Text(tag) if tag == "select_object") {
                return Err(PersistedV6Error::InvalidStructure);
            }
            parse_u64(&fields[1])?;
        }
    }
    Ok(())
}

fn validate_assembly_payload(value: &Value) -> Result<(), PersistedV6Error> {
    let fields = parse_array(value, 4)?;
    let stage = parse_array(&fields[0], 2)?;
    let Value::Text(stage_tag) = &stage[0] else {
        return Err(PersistedV6Error::InvalidStructure);
    };
    if !matches!(stage[1], Value::Null) {
        return Err(PersistedV6Error::InvalidStructure);
    }
    let selected_count = match &fields[1] {
        Value::Null => None,
        value => Some(parse_u32(value)?),
    };
    validate_u32_array(&fields[2])?;
    validate_u32_array(&fields[3])?;
    let Value::Array(selected_piece_keys) = &fields[2] else {
        unreachable!("validate_u32_array checked the array")
    };
    let Value::Array(ordered_piece_keys) = &fields[3] else {
        unreachable!("validate_u32_array checked the array")
    };
    match stage_tag.as_str() {
        "choose_count"
            if selected_count.is_none()
                && selected_piece_keys.is_empty()
                && ordered_piece_keys.is_empty() =>
        {
            Ok(())
        }
        "choose_members"
            if selected_count.is_some_and(|count| count <= 3)
                && selected_piece_keys.is_empty()
                && ordered_piece_keys.is_empty() =>
        {
            Ok(())
        }
        "order_members"
            if selected_count.is_some_and(|count| {
                count <= 3 && usize::try_from(count).ok() == Some(selected_piece_keys.len())
            }) && ordered_piece_keys.is_empty()
                && selected_piece_keys
                    .windows(2)
                    .all(|pair| parse_u32(&pair[0]).ok() < parse_u32(&pair[1]).ok()) =>
        {
            Ok(())
        }
        _ => Err(PersistedV6Error::InvalidStructure),
    }
}

fn validate_sba_order_payload(value: &Value) -> Result<(), PersistedV6Error> {
    let fields = parse_array(value, 5)?;
    parse_u64(&fields[0])?;
    let Value::Array(actions) = &fields[1] else {
        return Err(PersistedV6Error::InvalidStructure);
    };
    let mut previous_action_key: Option<(u8, u64)> = None;
    for action in actions {
        let action = parse_array(action, 2)?;
        let Value::Text(tag) = &action[0] else {
            return Err(PersistedV6Error::InvalidStructure);
        };
        match tag.as_str() {
            "player_loses" => {
                let player = parse_u64(&action[1])?;
                let key = (0, player);
                if previous_action_key.is_some_and(|previous| previous >= key) {
                    return Err(PersistedV6Error::InvalidStructure);
                }
                previous_action_key = Some(key);
            }
            "object_to_owner_graveyard" => {
                let payload = parse_array(&action[1], 2)?;
                let object = parse_u64(&payload[0])?;
                let key = (1, object);
                if previous_action_key.is_some_and(|previous| previous >= key) {
                    return Err(PersistedV6Error::InvalidStructure);
                }
                previous_action_key = Some(key);
                let Value::Array(causes) = &payload[1] else {
                    return Err(PersistedV6Error::InvalidStructure);
                };
                if causes.is_empty() {
                    return Err(PersistedV6Error::InvalidStructure);
                }
                let mut previous_cause = None;
                for cause in causes {
                    let Value::Text(cause) = cause else {
                        return Err(PersistedV6Error::InvalidStructure);
                    };
                    let rank = match cause.as_str() {
                        "zero_toughness" => 0,
                        "lethal_damage" => 1,
                        _ => return Err(PersistedV6Error::InvalidStructure),
                    };
                    if previous_cause.is_some_and(|previous| previous >= rank) {
                        return Err(PersistedV6Error::InvalidStructure);
                    }
                    previous_cause = Some(rank);
                }
            }
            _ => return Err(PersistedV6Error::InvalidStructure),
        }
    }
    if actions.is_empty() {
        return Err(PersistedV6Error::InvalidStructure);
    }
    let Value::Array(owners) = &fields[2] else {
        return Err(PersistedV6Error::InvalidStructure);
    };
    let mut seen_owners = std::collections::BTreeSet::new();
    for owner in owners {
        let owner = parse_u64(owner)?;
        if !seen_owners.insert(owner) {
            // The canonical APNAP order is not numeric order; only uniqueness
            // is checked here, while the state-level validator proves order.
            return Err(PersistedV6Error::InvalidStructure);
        }
    }
    let next_owner_index = parse_u32(&fields[3])?;
    let Value::Array(completed) = &fields[4] else {
        return Err(PersistedV6Error::InvalidStructure);
    };
    if owners.is_empty()
        || usize::try_from(next_owner_index).ok() != Some(completed.len())
        || usize::try_from(next_owner_index)
            .ok()
            .is_none_or(|index| index >= owners.len())
    {
        return Err(PersistedV6Error::InvalidStructure);
    }
    for (index, order) in completed.iter().enumerate() {
        let order = parse_array(order, 2)?;
        if parse_u64(&order[0])? != parse_u64(&owners[index])? {
            return Err(PersistedV6Error::InvalidStructure);
        }
        validate_u64_array(&order[1])?;
        let Value::Array(objects) = &order[1] else {
            unreachable!("validate_u64_array checked the array")
        };
        let mut seen = std::collections::BTreeSet::new();
        if objects.iter().any(|object| {
            parse_u64(object)
                .ok()
                .is_none_or(|object| !seen.insert(object))
        }) {
            return Err(PersistedV6Error::InvalidStructure);
        }
    }
    Ok(())
}

fn validate_u32_array(value: &Value) -> Result<(), PersistedV6Error> {
    for item in parse_list(value)? {
        parse_u32(item)?;
    }
    Ok(())
}

fn validate_u64_array(value: &Value) -> Result<(), PersistedV6Error> {
    for item in parse_list(value)? {
        parse_u64(item)?;
    }
    Ok(())
}

fn validate_pending_request(value: &Value) -> Result<(), PersistedV6Error> {
    let request = parse_array(value, 8)?;
    for field in &request[..4] {
        parse_u64(field)?;
    }
    if !matches!(&request[4], Value::Text(tag) if matches!(tag.as_str(), "public" | "acting_player_only" | "mixed"))
    {
        return Err(PersistedV6Error::InvalidStructure);
    }
    let Value::Array(candidates) = &request[6] else {
        return Err(PersistedV6Error::InvalidStructure);
    };
    let mut previous_order_key = None;
    for (expected_id, candidate) in candidates.iter().enumerate() {
        let candidate = parse_array(candidate, 3)?;
        if parse_u32(&candidate[0])? != expected_id as u32 {
            return Err(PersistedV6Error::InvalidStructure);
        }
        let order_key = validate_candidate(&candidate[1], &candidate[2])?;
        if previous_order_key.is_some_and(|previous| previous >= order_key) {
            return Err(PersistedV6Error::InvalidStructure);
        }
        previous_order_key = Some(order_key);
    }
    match &request[7] {
        Value::Null => {}
        value => {
            parse_u64(value)?;
        }
    }
    validate_decision_domain(&request[5], candidates.len())
}

fn validate_decision_domain(value: &Value, candidate_count: usize) -> Result<(), PersistedV6Error> {
    let fields = parse_array(value, 2)?;
    match &fields[0] {
        Value::Text(tag) if tag == "choose_one" => {
            if !matches!(fields[1], Value::Null) {
                return Err(PersistedV6Error::InvalidStructure);
            }
            if candidate_count == 0 {
                return Err(PersistedV6Error::InvalidStructure);
            }
        }
        Value::Text(tag) if tag == "choose_many" || tag == "order" => {
            let range = parse_array(&fields[1], 2)?;
            let minimum = parse_u32(&range[0])?;
            let maximum = parse_u32(&range[1])?;
            if minimum > maximum || usize::try_from(minimum).unwrap_or(usize::MAX) > candidate_count
            {
                return Err(PersistedV6Error::InvalidStructure);
            }
        }
        Value::Text(tag) if tag == "choose_number" => {
            let range = parse_array(&fields[1], 2)?;
            let minimum = parse_i64_value(&range[0])?;
            let maximum = parse_i64_value(&range[1])?;
            if minimum > maximum || candidate_count != 0 {
                return Err(PersistedV6Error::InvalidStructure);
            }
        }
        _ => return Err(PersistedV6Error::InvalidStructure),
    }
    Ok(())
}

fn validate_candidate(intent: &Value, binding: &Value) -> Result<(u8, i128), PersistedV6Error> {
    let intent = parse_array(intent, 2)?;
    let binding = parse_array(binding, 2)?;
    let Value::Text(tag) = &intent[0] else {
        return Err(PersistedV6Error::InvalidStructure);
    };
    if binding[0] != intent[0] {
        return Err(PersistedV6Error::InvalidStructure);
    }
    let rank = match tag.as_str() {
        "pass_priority" => {
            require_null(&intent[1])?;
            require_null(&binding[1])?;
            (0, 0)
        }
        "play_land" | "cast_spell" | "select_object" => {
            let key = i128::from(parse_u64(&intent[1])?);
            parse_u64(&binding[1])?;
            let rank = match tag.as_str() {
                "play_land" => 1,
                "cast_spell" => 2,
                "select_object" => 4,
                _ => 5,
            };
            (rank, key)
        }
        "select_player" => {
            let key = parse_u64(&intent[1])?;
            if parse_u64(&binding[1])? != key {
                return Err(PersistedV6Error::InvalidStructure);
            }
            (5, i128::from(key))
        }
        "activate_ability" => {
            let key = i128::from(parse_u64(&intent[1])?);
            parse_u64(&binding[1])?;
            (3, key)
        }
        "select_mode" => {
            let key = i128::from(parse_u32(&intent[1])?);
            if parse_u32(&binding[1])? != key as u32 {
                return Err(PersistedV6Error::InvalidStructure);
            }
            (6, key)
        }
        "choose_boolean" => {
            if !matches!(intent[1], Value::Bool(_)) || intent[1] != binding[1] {
                return Err(PersistedV6Error::InvalidStructure);
            }
            (
                7,
                if matches!(intent[1], Value::Bool(true)) {
                    1
                } else {
                    0
                },
            )
        }
        "declare_number" => {
            let key = parse_integer(&intent[1])?;
            if intent[1] != binding[1] {
                return Err(PersistedV6Error::InvalidStructure);
            }
            (8, key)
        }
        "confirm" => {
            require_null(&intent[1])?;
            require_null(&binding[1])?;
            (9, 0)
        }
        _ => return Err(PersistedV6Error::InvalidStructure),
    };
    Ok(rank)
}

fn parse_integer(value: &Value) -> Result<i128, PersistedV6Error> {
    match value {
        Value::Unsigned(value) => i64::try_from(*value)
            .map(i128::from)
            .map_err(|_| PersistedV6Error::InvalidStructure),
        Value::Signed(value) => Ok(i128::from(*value)),
        _ => Err(PersistedV6Error::InvalidStructure),
    }
}

fn require_null(value: &Value) -> Result<(), PersistedV6Error> {
    if matches!(value, Value::Null) {
        Ok(())
    } else {
        Err(PersistedV6Error::InvalidStructure)
    }
}
