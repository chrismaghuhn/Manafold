//! Detached public stack and temporary-effect projection for the G0 payload.

use mtgml_decision::{CostFactsV1, SafeTargetDescriptorV1};
use mtgml_model::{parse_canonical_u64, OpaqueAbilityId, OpaqueObjectId, PlayerId};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

use crate::{
    AttachmentObservationV1, CounterObservationV1, FaceObservationV1, MagicBasicLandObservationV1,
    MagicPendingSbaOrdering, ManaPoolObservationV1, ObservationValidationError, SyntheticPriority,
    SyntheticTurnPosition, MAGIC_BASIC_LAND_OBSERVATION_SCHEMA_V1,
};

pub const MAGIC_SHARED_EXECUTION_OBSERVATION_SCHEMA_V1: &str =
    "magic-shared-execution-observation.v1";

fn deserialize_required_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::deserialize(deserializer)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicModeV1 {
    pub mode_slot: u32,
    pub selected_mode: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PublicStackItemV1 {
    Spell {
        controller: PlayerId,
        card_object: OpaqueObjectId,
        modes: Vec<PublicModeV1>,
        targets: Vec<SafeTargetDescriptorV1>,
        cost_facts: CostFactsV1,
    },
    ActivatedAbility {
        controller: PlayerId,
        #[serde(deserialize_with = "deserialize_required_option")]
        source_object: Option<OpaqueObjectId>,
        #[serde(deserialize_with = "deserialize_required_option")]
        source_ability: Option<OpaqueAbilityId>,
        modes: Vec<PublicModeV1>,
        targets: Vec<SafeTargetDescriptorV1>,
        cost_facts: CostFactsV1,
    },
    TriggeredAbility {
        controller: PlayerId,
        #[serde(deserialize_with = "deserialize_required_option")]
        source_object: Option<OpaqueObjectId>,
        #[serde(deserialize_with = "deserialize_required_option")]
        source_ability: Option<OpaqueAbilityId>,
        targets: Vec<SafeTargetDescriptorV1>,
    },
}

impl PublicStackItemV1 {
    pub(crate) fn validate_public(&self) -> Result<(), ObservationValidationError> {
        match self {
            Self::Spell {
                modes, cost_facts, ..
            } => {
                validate_public_cost_facts(cost_facts)?;
                if modes
                    .windows(2)
                    .any(|pair| pair[0].mode_slot >= pair[1].mode_slot)
                {
                    return Err(ObservationValidationError::ObservationPayload);
                }
                Ok(())
            }
            Self::ActivatedAbility {
                source_object,
                source_ability,
                modes,
                cost_facts,
                ..
            } => {
                validate_public_source(*source_object, *source_ability)?;
                validate_public_cost_facts(cost_facts)?;
                if modes
                    .windows(2)
                    .any(|pair| pair[0].mode_slot >= pair[1].mode_slot)
                {
                    return Err(ObservationValidationError::ObservationPayload);
                }
                Ok(())
            }
            Self::TriggeredAbility {
                source_object,
                source_ability,
                ..
            } => validate_public_source(*source_object, *source_ability),
        }
    }
}

fn validate_public_cost_facts(facts: &CostFactsV1) -> Result<(), ObservationValidationError> {
    if facts
        .paid_additional_cost_ids
        .windows(2)
        .any(|pair| pair[0] >= pair[1])
    {
        return Err(ObservationValidationError::ObservationPayload);
    }
    Ok(())
}

fn validate_public_source(
    source_object: Option<OpaqueObjectId>,
    source_ability: Option<OpaqueAbilityId>,
) -> Result<(), ObservationValidationError> {
    if source_ability.is_some() && source_object.is_none() {
        return Err(ObservationValidationError::ObservationPayload);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PublicEffectKeywordV1 {
    Haste,
    DoubleStrike,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PublicTemporaryOperationV1 {
    PowerToughnessDelta { power: i32, toughness: i32 },
    GrantKeyword { keyword: PublicEffectKeywordV1 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PublicEffectExpiryV1 {
    UntilEndOfTurn { turn_number: String },
}

impl PublicEffectExpiryV1 {
    fn turn_number(&self) -> Result<u64, ObservationValidationError> {
        match self {
            Self::UntilEndOfTurn { turn_number } => parse_canonical_u64(turn_number)
                .map_err(|_| ObservationValidationError::ObservationPayload),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicTemporaryEffectV1 {
    pub affected_objects: Vec<OpaqueObjectId>,
    pub operation: PublicTemporaryOperationV1,
    pub expiry: PublicEffectExpiryV1,
}

impl PublicTemporaryEffectV1 {
    pub(crate) fn validate_public(&self) -> Result<(), ObservationValidationError> {
        if self.affected_objects.is_empty()
            || self
                .affected_objects
                .windows(2)
                .any(|pair| pair[0] >= pair[1])
        {
            return Err(ObservationValidationError::ObservationPayload);
        }
        self.expiry.turn_number()?;
        Ok(())
    }

    pub fn compare_canonical(&self, other: &Self) -> Result<Ordering, ObservationValidationError> {
        let self_turn = self.expiry.turn_number()?;
        let other_turn = other.expiry.turn_number()?;
        let order = self
            .affected_objects
            .cmp(&other.affected_objects)
            .then_with(|| {
                temporary_operation_rank(self.operation)
                    .cmp(&temporary_operation_rank(other.operation))
            })
            .then_with(|| compare_temporary_operation(self.operation, other.operation))
            .then_with(|| self_turn.cmp(&other_turn));
        Ok(order)
    }
}

fn temporary_operation_rank(operation: PublicTemporaryOperationV1) -> u8 {
    match operation {
        PublicTemporaryOperationV1::PowerToughnessDelta { .. } => 0,
        PublicTemporaryOperationV1::GrantKeyword { .. } => 1,
    }
}

fn compare_temporary_operation(
    left: PublicTemporaryOperationV1,
    right: PublicTemporaryOperationV1,
) -> Ordering {
    match (left, right) {
        (
            PublicTemporaryOperationV1::PowerToughnessDelta {
                power: left_power,
                toughness: left_toughness,
            },
            PublicTemporaryOperationV1::PowerToughnessDelta {
                power: right_power,
                toughness: right_toughness,
            },
        ) => left_power
            .cmp(&right_power)
            .then_with(|| left_toughness.cmp(&right_toughness)),
        (
            PublicTemporaryOperationV1::GrantKeyword { keyword: left },
            PublicTemporaryOperationV1::GrantKeyword { keyword: right },
        ) => keyword_rank(left).cmp(&keyword_rank(right)),
        _ => Ordering::Equal,
    }
}

fn keyword_rank(keyword: PublicEffectKeywordV1) -> u8 {
    match keyword {
        PublicEffectKeywordV1::Haste => 0,
        PublicEffectKeywordV1::DoubleStrike => 1,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MagicSharedExecutionObservationV1 {
    pub schema_version: String,
    pub active_player: PlayerId,
    pub turn_number: String,
    pub turn_position: SyntheticTurnPosition,
    pub priority: SyntheticPriority,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub pending_sba_ordering: Option<MagicPendingSbaOrdering>,
    pub mana_pools: Vec<ManaPoolObservationV1>,
    pub counters: Vec<CounterObservationV1>,
    pub attachments: Vec<AttachmentObservationV1>,
    pub faces: Vec<FaceObservationV1>,
    pub stack: Vec<PublicStackItemV1>,
    pub temporary_effects: Vec<PublicTemporaryEffectV1>,
}

impl MagicSharedExecutionObservationV1 {
    pub fn validate(&self) -> Result<(), ObservationValidationError> {
        if self.schema_version != MAGIC_SHARED_EXECUTION_OBSERVATION_SCHEMA_V1 {
            return Err(ObservationValidationError::SchemaOrCodec);
        }
        let base = MagicBasicLandObservationV1 {
            schema_version: MAGIC_BASIC_LAND_OBSERVATION_SCHEMA_V1.into(),
            active_player: self.active_player,
            turn_number: self.turn_number.clone(),
            turn_position: self.turn_position,
            priority: self.priority,
            pending_sba_ordering: self.pending_sba_ordering.clone(),
            mana_pools: self.mana_pools.clone(),
            counters: self.counters.clone(),
            attachments: self.attachments.clone(),
            faces: self.faces.clone(),
        };
        base.validate()?;
        for item in &self.stack {
            item.validate_public()?;
        }
        for effect in &self.temporary_effects {
            effect.validate_public()?;
        }
        for pair in self.temporary_effects.windows(2) {
            if pair[0].compare_canonical(&pair[1])? == Ordering::Greater {
                return Err(ObservationValidationError::ObservationPayload);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const FIXTURE: &str =
        include_str!("../../../schemas/examples/magic-shared-execution-observation-v1.json");

    #[test]
    fn rust_dto_matches_all_shared_stack_and_temporary_effect_variants() {
        let view: MagicSharedExecutionObservationV1 = serde_json::from_str(FIXTURE).unwrap();
        view.validate().unwrap();
        let expected: Value = serde_json::from_str(FIXTURE).unwrap();
        assert_eq!(serde_json::to_value(&view).unwrap(), expected);
        assert_eq!(view.stack.len(), 3);
        assert_eq!(view.temporary_effects.len(), 3);
    }

    #[test]
    fn rejects_unbound_ability_and_noncanonical_effect_view_order() {
        let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
        value["stack"][1]["source_object"] = Value::Null;
        let view: MagicSharedExecutionObservationV1 = serde_json::from_value(value).unwrap();
        assert_eq!(
            view.validate(),
            Err(ObservationValidationError::ObservationPayload)
        );

        let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
        value["temporary_effects"].as_array_mut().unwrap().reverse();
        let view: MagicSharedExecutionObservationV1 = serde_json::from_value(value).unwrap();
        assert_eq!(
            view.validate(),
            Err(ObservationValidationError::ObservationPayload)
        );
    }

    #[test]
    fn rejects_duplicate_mode_cost_and_effect_identity_values() {
        let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
        value["stack"][0]["modes"] = serde_json::json!([
            {"mode_slot": 0, "selected_mode": 1},
            {"mode_slot": 0, "selected_mode": 2}
        ]);
        let view: MagicSharedExecutionObservationV1 = serde_json::from_value(value).unwrap();
        assert_eq!(
            view.validate(),
            Err(ObservationValidationError::ObservationPayload)
        );

        let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
        value["stack"][1]["modes"] = serde_json::json!([
            {"mode_slot": 0, "selected_mode": 1},
            {"mode_slot": 0, "selected_mode": 2}
        ]);
        let view: MagicSharedExecutionObservationV1 = serde_json::from_value(value).unwrap();
        assert_eq!(
            view.validate(),
            Err(ObservationValidationError::ObservationPayload)
        );

        let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
        value["stack"][0]["cost_facts"]["paid_additional_cost_ids"] = serde_json::json!([2, 2]);
        let view: MagicSharedExecutionObservationV1 = serde_json::from_value(value).unwrap();
        assert_eq!(
            view.validate(),
            Err(ObservationValidationError::ObservationPayload)
        );

        let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
        value["temporary_effects"][0]["affected_objects"] = serde_json::json!(["2", "2"]);
        let view: MagicSharedExecutionObservationV1 = serde_json::from_value(value).unwrap();
        assert_eq!(
            view.validate(),
            Err(ObservationValidationError::ObservationPayload)
        );

        let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
        value["temporary_effects"][0]["operation"]["power"] = serde_json::json!(2147483648_i64);
        assert!(serde_json::from_value::<MagicSharedExecutionObservationV1>(value).is_err());
    }

    #[test]
    fn activated_ability_modes_are_preserved_in_the_public_stack_view() {
        let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
        value["stack"][1]["modes"] = serde_json::json!([
            {"mode_slot": 0, "selected_mode": 1}
        ]);
        let view: MagicSharedExecutionObservationV1 = serde_json::from_value(value).unwrap();
        view.validate().unwrap();
        assert!(matches!(
            &view.stack[1],
            PublicStackItemV1::ActivatedAbility { modes, .. }
                if modes == &[PublicModeV1 { mode_slot: 0, selected_mode: 1 }]
        ));
    }
}
