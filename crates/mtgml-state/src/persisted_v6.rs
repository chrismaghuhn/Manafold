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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManaPoolV1 {
    pub unrestricted: [u32; 6],
    pub creature_spell_only: [u32; 6],
}

impl Default for ManaPoolV1 {
    fn default() -> Self {
        Self {
            unrestricted: [0; 6],
            creature_spell_only: [0; 6],
        }
    }
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
            .any(|values| values.values().any(|n| *n == 0))
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
        validate_legacy_components(
            &self.core_v1,
            &self.zones_v1,
            &self.allocators_v3,
            &self.random_v1,
            &self.knowledge_v2,
            &self.perspective_identities_v2,
            &self.combat,
            &self.foundation_sources,
            &self.format_v1,
        )?;
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
        validate_legacy_components(
            &fields[3],
            &fields[4],
            &fields[5],
            &fields[7],
            &fields[8],
            &fields[9],
            &fields[10],
            &fields[11],
            &fields[12],
        )?;
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

fn validate_legacy_components(
    core: &Value,
    zones: &Value,
    allocators: &Value,
    random: &Value,
    knowledge: &Value,
    perspective_identities: &Value,
    combat: &Value,
    foundation_sources: &Value,
    format: &Value,
) -> Result<(), PersistedV6Error> {
    parse_array(core, 5)?;
    parse_array(zones, 5)?;
    let allocator_values = parse_array(allocators, 8)?;
    for value in allocator_values {
        parse_u64(value)?;
    }
    parse_array(random, 3)?;
    parse_array(knowledge, 2)?;
    parse_array(perspective_identities, 2)?;
    match combat {
        Value::Null => {}
        Value::Array(values) if values.len() == 3 || values.len() == 5 => {}
        _ => return Err(PersistedV6Error::InvalidStructure),
    }
    if !matches!(foundation_sources, Value::Array(_)) {
        return Err(PersistedV6Error::InvalidStructure);
    }
    parse_array(format, 2)?;
    Ok(())
}

pub fn validate_execution_v3(value: &Value) -> Result<(), PersistedV6Error> {
    let fields = parse_array(value, 5)?;
    if !matches!(fields[0], Value::Null) {
        validate_pending_request(&fields[0])?;
    }
    for field in &fields[1..] {
        if !matches!(field, Value::Array(_)) {
            return Err(PersistedV6Error::InvalidStructure);
        }
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
    validate_decision_domain(&request[5])?;
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
    if matches!(&request[5], Value::Array(domain) if domain.first() == Some(&Value::Text("choose_number".to_owned())))
        && !candidates.is_empty()
    {
        return Err(PersistedV6Error::InvalidStructure);
    }
    match &request[7] {
        Value::Null => Ok(()),
        value => parse_u64(value).map(|_| ()),
    }
}

fn validate_decision_domain(value: &Value) -> Result<(), PersistedV6Error> {
    let fields = parse_array(value, 2)?;
    match &fields[0] {
        Value::Text(tag) if tag == "choose_one" || tag == "confirm" => {
            if !matches!(fields[1], Value::Null) {
                return Err(PersistedV6Error::InvalidStructure);
            }
        }
        Value::Text(tag) if tag == "choose_many" || tag == "order" => {
            let range = parse_array(&fields[1], 2)?;
            let minimum = parse_u32(&range[0])?;
            let maximum = parse_u32(&range[1])?;
            if minimum > maximum {
                return Err(PersistedV6Error::InvalidStructure);
            }
        }
        Value::Text(tag) if tag == "choose_number" => {
            let range = parse_array(&fields[1], 2)?;
            for endpoint in range {
                if !matches!(endpoint, Value::Signed(_) | Value::Unsigned(_)) {
                    return Err(PersistedV6Error::InvalidStructure);
                }
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
        "play_land" | "cast_spell" | "select_object" | "select_player" => {
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
        "activate_ability" => {
            let key = i128::from(parse_u64(&intent[1])?);
            parse_u64(&binding[1])?;
            (3, key)
        }
        "select_mode" => {
            let key = i128::from(parse_u32(&intent[1])?);
            parse_u32(&binding[1])?;
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
        Value::Unsigned(value) => Ok(i128::from(*value)),
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
