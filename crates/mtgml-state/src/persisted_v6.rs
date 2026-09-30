//! Typed card-rules authoritative state (mana, turn history, counters,
//! attachments, faces, ability authority) and its FullStateDigestV7 record
//! encoding, plus the V3-execution validator the V2 state parts still run.

use std::collections::{BTreeMap, BTreeSet};

use mtgml_model::{AbilityInstanceId, GameObjectId, PlayerId, StateRevision};
use mtgml_persistence::cbor::Value;

use crate::StateDigestError;

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
}

impl TurnHistoryStateV1 {
    pub fn validate(&self) -> Result<(), PersistedV6Error> {
        if self.players.values().any(|history| {
            history.land_plays_used > 1
                || history.noncreature_spells_cast > history.spells_cast_total
        }) {
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
    fn validate(&self) -> Result<(), PersistedV6Error> {
        let mut timestamps = BTreeSet::new();
        if self
            .by_source
            .values()
            .any(|edge| !timestamps.insert(edge.timestamp))
        {
            return Err(PersistedV6Error::InvalidStructure);
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
}

impl CardRulesAuthoritativeStateV1 {
    pub fn validate(&self) -> Result<(), PersistedV6Error> {
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

    /// Builds the persisted value from the typed successor execution owner.
    pub fn from_successor(state: &crate::ExecutionStateV3) -> Result<Self, StateDigestError> {
        let value = state.canonical_value()?;
        Self::from_value(value).map_err(|_| StateDigestError::StateInvariant)
    }
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
