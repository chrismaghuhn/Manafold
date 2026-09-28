//! Canonical restricted-CBOR input for the detached G0 FullStateDigestV7.
//!
//! V6 component bytes are reused only for unchanged state families. The new
//! `zones_v2` and `execution_v4` children are encoded from their typed owners.

use mtgml_card_ir::CardSemanticProfileId;
use mtgml_decision::{
    AuthoritativeCandidateV4, AuthoritativeDecisionRequestV4, CandidateIntentV4,
    CostFactsV1 as DecisionCostFactsV1, DecisionDomainV2, DecisionPurposeV4, DecisionVisibility,
    EngineCandidateBindingV4, SafeDamageRecipientV1, SafeTargetDescriptorV1,
    SafeTriggerDescriptorV1, SafeTriggerSubjectV1, SafeZoneKindV1,
};
use mtgml_persistence::cbor::{self, Value};

use crate::{
    AbilitySourceContext, ActionCostFacts, AssemblyStageV2, AttackerFact, CastContinuation,
    CastContinuationStage, ContinuationPayloadV3, ContinuationRecordV3, CostFacts, CostRoute,
    DamageKind, DamageRecipient, EffectExpiry, EffectTimestamp, EngineStatePartsV2,
    EngineStatePartsV3, ExecutionStateV3, ExecutionStateV4, LifeChangeCause, ManaCost,
    ManaPaymentStage, ManaPaymentStaging, ManaSourceActivation, ManaSourceActivationCost,
    ModeBinding, NonManaActivationContinuation, NonManaActivationStage, PendingTriggerRecord,
    ReservedNonManaCost, SelectedCostOperand, SourceContext, StackItemPayload,
    StackResolutionContinuation, StackResolutionStage, TargetBinding, TargetRef,
    TemporaryEffectRecord, TemporaryKeyword, TemporaryOperation, TriggerActorRequestRoot,
    TriggerEventSnapshot, TriggerPlacementContinuation, TriggerTargetTiming,
    FULL_STATE_DIGEST_DOMAIN_V7, FULL_STATE_DIGEST_INPUT_SCHEMA_V7,
};

fn u(value: u64) -> Value {
    Value::Unsigned(value)
}

fn i(value: i64) -> Value {
    Value::Signed(value)
}

fn text(value: impl Into<String>) -> Value {
    Value::Text(value.into())
}

fn array(values: impl IntoIterator<Item = Value>) -> Value {
    Value::Array(values.into_iter().collect())
}

fn optional(value: Option<Value>) -> Value {
    value.unwrap_or(Value::Null)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FullStateDigestInputV7(Value);

impl FullStateDigestInputV7 {
    pub fn from_successor(state: &EngineStatePartsV3) -> Result<Self, crate::StateDigestError> {
        state
            .validate()
            .map_err(|_| crate::StateDigestError::StateInvariant)?;
        Ok(Self(state_value_validated(state)?))
    }

    pub fn from_successor_with_profile_domain_context(
        state: &EngineStatePartsV3,
        context: &mtgml_decision::ProfileDecisionDomainContextV1,
        active_execution_identity: &mtgml_model::ExecutionIdentityV1,
        active_rules_contract_id: &mtgml_model::RulesContractIdV1,
    ) -> Result<Self, crate::StateDigestError> {
        state
            .validate_with_profile_domain_context(
                context,
                active_execution_identity,
                active_rules_contract_id,
            )
            .map_err(|_| crate::StateDigestError::StateInvariant)?;
        Ok(Self(state_value_validated(state)?))
    }

    pub fn canonical_value(&self) -> &Value {
        &self.0
    }

    pub fn canonical_payload(&self) -> Result<Vec<u8>, crate::StateDigestError> {
        cbor::encode_canonical(&self.0).map_err(crate::StateDigestError::Persistence)
    }

    pub fn from_canonical_payload(payload: &[u8]) -> Result<Self, crate::StateDigestError> {
        let value =
            cbor::decode_canonical(payload).map_err(crate::StateDigestError::Persistence)?;
        validate_v7_envelope_shape(&value)?;
        Ok(Self(value))
    }
}

fn validate_v7_envelope_shape(value: &Value) -> Result<(), crate::StateDigestError> {
    let Value::Array(fields) = value else {
        return Err(crate::StateDigestError::StateInvariant);
    };
    if fields.len() != 14
        || fields[0] != text(FULL_STATE_DIGEST_INPUT_SCHEMA_V7)
        || fields[1] != text(FULL_STATE_DIGEST_DOMAIN_V7)
        || !matches!(fields[2], Value::Unsigned(_))
        || !matches!(&fields[4], Value::Array(zones) if zones.len() == 6 && zones[0] == text("zones_v2"))
        || !matches!(&fields[6], Value::Array(execution) if execution.len() == 6 && execution[0] == text("execution_v4"))
    {
        return Err(crate::StateDigestError::StateInvariant);
    }
    validate_unchanged_v6_components(fields)?;
    validate_zones_v2_value(&fields[4])?;
    validate_execution_v4_value(&fields[6])?;
    Ok(())
}

fn validate_unchanged_v6_components(fields: &[Value]) -> Result<(), crate::StateDigestError> {
    let Value::Array(zones) = &fields[4] else {
        return Err(crate::StateDigestError::StateInvariant);
    };
    let Value::Array(stack_records) = &zones[4] else {
        return Err(crate::StateDigestError::StateInvariant);
    };
    let legacy_records = stack_records
        .iter()
        .map(|record| {
            let fields = array_fields(record, 3)?;
            Ok(array([
                fields[0].clone(),
                fields[1].clone(),
                Value::Null,
                Value::Null,
            ]))
        })
        .collect::<Result<Vec<_>, crate::StateDigestError>>()?;
    let mut legacy = fields.to_vec();
    legacy[0] = text(crate::FULL_STATE_DIGEST_INPUT_SCHEMA_V6);
    legacy[1] = text(crate::FULL_STATE_DIGEST_DOMAIN_V6);
    legacy[4] = array([
        zones[1].clone(),
        zones[2].clone(),
        zones[3].clone(),
        array(legacy_records),
        zones[5].clone(),
    ]);
    legacy[6] = array([Value::Null, array([]), array([]), array([]), array([])]);
    crate::FullStateDigestInputV6::from_canonical_value(&Value::Array(legacy))
        .map_err(|_| crate::StateDigestError::StateInvariant)?;
    Ok(())
}

fn validate_zones_v2_value(value: &Value) -> Result<(), crate::StateDigestError> {
    let fields = array_fields(value, 6)?;
    expect_text(&fields[0], "zones_v2")?;
    let Value::Array(records) = &fields[4] else {
        return Err(crate::StateDigestError::StateInvariant);
    };
    let mut ids = std::collections::BTreeSet::new();
    let mut previous = None;
    for record in records {
        let record = array_fields(record, 3)?;
        let id = uint(&record[0])?;
        if id == 0 || previous.is_some_and(|last| last >= id) {
            return Err(crate::StateDigestError::StateInvariant);
        }
        previous = Some(id);
        uint(&record[1])?;
        validate_stack_payload_value(&record[2])?;
        ids.insert(id);
    }
    let Value::Array(order) = &fields[5] else {
        return Err(crate::StateDigestError::StateInvariant);
    };
    let mut ordered_ids = std::collections::BTreeSet::new();
    for item in order {
        let id = uint(item)?;
        if !ordered_ids.insert(id) {
            return Err(crate::StateDigestError::StateInvariant);
        }
    }
    if ids != ordered_ids {
        return Err(crate::StateDigestError::StateInvariant);
    }
    Ok(())
}

fn validate_stack_payload_value(value: &Value) -> Result<(), crate::StateDigestError> {
    let Value::Array(fields) = value else {
        return Err(crate::StateDigestError::StateInvariant);
    };
    let Some(Value::Text(tag)) = fields.first() else {
        return Err(crate::StateDigestError::StateInvariant);
    };
    match tag.as_str() {
        "spell" if fields.len() == 8 => {
            uint(&fields[1])?;
            uint(&fields[2])?;
            uint32(&fields[3])?;
            nonempty_text(&fields[4])?;
            validate_mode_bindings(&fields[5])?;
            validate_target_bindings(&fields[6])?;
            validate_cost_facts(&fields[7])?;
        }
        "activated_ability" if fields.len() == 5 => {
            validate_ability_source_context(&fields[1])?;
            validate_mode_bindings(&fields[2])?;
            validate_target_bindings(&fields[3])?;
            validate_cost_facts(&fields[4])?;
        }
        "triggered_ability" if fields.len() == 5 => {
            if uint(&fields[1])? == 0 {
                return Err(crate::StateDigestError::StateInvariant);
            }
            validate_ability_source_context(&fields[2])?;
            validate_trigger_event_value(&fields[3])?;
            validate_target_bindings(&fields[4])?;
        }
        _ => return Err(crate::StateDigestError::StateInvariant),
    }
    Ok(())
}

fn validate_execution_v4_value(value: &Value) -> Result<(), crate::StateDigestError> {
    let fields = array_fields(value, 6)?;
    expect_text(&fields[0], "execution_v4")?;
    if !matches!(fields[1], Value::Null) {
        validate_decision_request(&fields[1])?;
    }
    let Value::Array(continuations) = &fields[2] else {
        return Err(crate::StateDigestError::StateInvariant);
    };
    let mut previous = None;
    for record in continuations {
        let record = array_fields(record, 3)?;
        let id = uint(&record[0])?;
        if id == 0 || previous.is_some_and(|last| last >= id) {
            return Err(crate::StateDigestError::StateInvariant);
        }
        previous = Some(id);
        uint(&record[1])?;
        validate_continuation_payload(&record[2])?;
    }
    let Value::Array(effects) = &fields[3] else {
        return Err(crate::StateDigestError::StateInvariant);
    };
    previous = None;
    for effect in effects {
        let effect = array_fields(effect, 5)?;
        let id = uint(&effect[0])?;
        if id == 0 || previous.is_some_and(|last| last >= id) {
            return Err(crate::StateDigestError::StateInvariant);
        }
        previous = Some(id);
        validate_u64_array(&effect[1])?;
        validate_temporary_operation(&effect[2])?;
        let expiry = array_fields(&effect[3], 2)?;
        expect_text(&expiry[0], "until_end_of_turn")?;
        uint(&expiry[1])?;
        if !matches!(effect[4], Value::Null) {
            let timestamp = array_fields(&effect[4], 2)?;
            uint(&timestamp[0])?;
            uint32(&timestamp[1])?;
        }
    }
    let Value::Array(triggers) = &fields[4] else {
        return Err(crate::StateDigestError::StateInvariant);
    };
    previous = None;
    for trigger in triggers {
        let trigger = array_fields(trigger, 5)?;
        let id = uint(&trigger[0])?;
        if id == 0 || previous.is_some_and(|last| last >= id) {
            return Err(crate::StateDigestError::StateInvariant);
        }
        previous = Some(id);
        uint(&trigger[1])?;
        validate_ability_source_context(&trigger[2])?;
        validate_trigger_event_value(&trigger[3])?;
        closed_tag(
            &trigger[4],
            &["no_targets", "captured_from_event", "choose_on_placement"],
        )?;
    }
    if fields[5] != array([]) {
        return Err(crate::StateDigestError::StateInvariant);
    }
    Ok(())
}

fn validate_continuation_payload(value: &Value) -> Result<(), crate::StateDigestError> {
    let Value::Array(fields) = value else {
        return Err(crate::StateDigestError::StateInvariant);
    };
    let Some(Value::Text(tag)) = fields.first() else {
        return Err(crate::StateDigestError::StateInvariant);
    };
    match tag.as_str() {
        "synthetic_assembly" if fields.len() == 6 => {
            uint(&fields[1])?;
            closed_tag(
                &fields[2],
                &["choose_count", "choose_members", "order_members"],
            )?;
            if !matches!(fields[3], Value::Null) {
                uint32(&fields[3])?;
            }
            validate_u32_array(&fields[4])?;
            validate_u32_array(&fields[5])?;
        }
        "magic_sba_graveyard_order_v1" if fields.len() == 6 => {
            uint(&fields[1])?;
            validate_sba_actions(&fields[2])?;
            validate_u64_array(&fields[3])?;
            uint32(&fields[4])?;
            let Value::Array(completed) = &fields[5] else {
                return Err(crate::StateDigestError::StateInvariant);
            };
            for order in completed {
                let order = array_fields(order, 2)?;
                uint(&order[0])?;
                validate_u64_array(&order[1])?;
            }
        }
        "cast" if fields.len() == 13 => validate_cast_continuation(fields)?,
        "nonmana_activation" if fields.len() == 11 => validate_activation_continuation(fields)?,
        "trigger_placement" if fields.len() == 7 => validate_trigger_placement(fields)?,
        "stack_resolution" if fields.len() == 5 => validate_stack_resolution(value)?,
        _ => return Err(crate::StateDigestError::StateInvariant),
    }
    Ok(())
}

fn validate_cast_continuation(fields: &[Value]) -> Result<(), crate::StateDigestError> {
    for index in [1, 2, 3, 4] {
        uint(&fields[index])?;
    }
    nonempty_text(&fields[5])?;
    closed_tag(
        &fields[6],
        &[
            "selecting_cost_route",
            "selecting_modes",
            "selecting_targets",
            "selecting_additional_costs",
            "selecting_cost_operands",
            "paying_mana",
        ],
    )?;
    validate_cost_route(&fields[7])?;
    validate_mode_bindings(&fields[8])?;
    validate_target_bindings(&fields[9])?;
    validate_u32_array(&fields[10])?;
    validate_action_cost_facts(&fields[11])?;
    validate_optional_mana_staging(&fields[12])
}

fn validate_activation_continuation(fields: &[Value]) -> Result<(), crate::StateDigestError> {
    for index in [1, 2, 3, 4] {
        uint(&fields[index])?;
    }
    nonempty_text(&fields[5])?;
    closed_tag(
        &fields[6],
        &[
            "selecting_modes",
            "selecting_targets",
            "selecting_cost_operands",
            "paying_mana",
        ],
    )?;
    validate_mode_bindings(&fields[7])?;
    validate_target_bindings(&fields[8])?;
    validate_action_cost_facts(&fields[9])?;
    validate_optional_mana_staging(&fields[10])
}

fn validate_trigger_placement(fields: &[Value]) -> Result<(), crate::StateDigestError> {
    validate_u64_array(&fields[1])?;
    uint32(&fields[2])?;
    validate_u64_array(&fields[3])?;
    let Value::Array(orders) = &fields[4] else {
        return Err(crate::StateDigestError::StateInvariant);
    };
    for order in orders {
        let order = array_fields(order, 2)?;
        uint(&order[0])?;
        validate_u64_array(&order[1])?;
    }
    let Value::Array(targets) = &fields[5] else {
        return Err(crate::StateDigestError::StateInvariant);
    };
    for target in targets {
        let target = array_fields(target, 2)?;
        uint(&target[0])?;
        validate_target_binding(&target[1])?;
    }
    let Value::Array(roots) = &fields[6] else {
        return Err(crate::StateDigestError::StateInvariant);
    };
    for root in roots {
        let root = array_fields(root, 2)?;
        uint(&root[0])?;
        uint(&root[1])?;
    }
    Ok(())
}

fn validate_optional_mana_staging(value: &Value) -> Result<(), crate::StateDigestError> {
    if matches!(value, Value::Null) {
        return Ok(());
    }
    let fields = array_fields(value, 3)?;
    expect_text(&fields[0], "mana_payment_staging")?;
    closed_tag(
        &fields[1],
        &["selecting_sources", "awaiting_final_allocation"],
    )?;
    let Value::Array(activations) = &fields[2] else {
        return Err(crate::StateDigestError::StateInvariant);
    };
    for activation in activations {
        let activation = array_fields(activation, 6)?;
        for index in [0, 1, 2] {
            uint(&activation[index])?;
        }
        nonempty_text(&activation[3])?;
        let cost = array_fields(&activation[4], 1)?;
        expect_text(&cost[0], "tap_source")?;
        validate_fixed_u32_array(&activation[5], 12)?;
    }
    Ok(())
}

fn validate_action_cost_facts(value: &Value) -> Result<(), crate::StateDigestError> {
    let fields = array_fields(value, 3)?;
    if !matches!(fields[0], Value::Null) {
        let mana = array_fields(&fields[0], 3)?;
        validate_fixed_u32_array(&mana[0], 5)?;
        uint32(&mana[1])?;
        uint32(&mana[2])?;
    }
    let Value::Array(reserved) = &fields[1] else {
        return Err(crate::StateDigestError::StateInvariant);
    };
    for cost in reserved {
        let cost = array_fields(cost, 1)?;
        closed_tag(&cost[0], &["tap_source", "sacrifice_source"])?;
    }
    let Value::Array(operands) = &fields[2] else {
        return Err(crate::StateDigestError::StateInvariant);
    };
    for operand in operands {
        let operand = array_fields(operand, 4)?;
        expect_text(&operand[0], "put_counters")?;
        uint(&operand[1])?;
        expect_text(&operand[2], "minus_one_minus_one")?;
        uint32(&operand[3])?;
    }
    Ok(())
}

fn validate_decision_request(value: &Value) -> Result<(), crate::StateDigestError> {
    let fields = array_fields(value, 11)?;
    for index in [0, 1, 2, 3, 4] {
        uint(&fields[index])?;
    }
    closed_tag(&fields[5], &["public", "acting_player_only", "mixed"])?;
    validate_decision_domain(&fields[6])?;
    validate_decision_purpose(&fields[7])?;
    optional_uint(&fields[8])?;
    optional_uint(&fields[9])?;
    let Value::Array(candidates) = &fields[10] else {
        return Err(crate::StateDigestError::StateInvariant);
    };
    for (index, candidate) in candidates.iter().enumerate() {
        let candidate = array_fields(candidate, 3)?;
        if uint32(&candidate[0])? != index as u32 {
            return Err(crate::StateDigestError::StateInvariant);
        }
        let visible_tag = tag(&candidate[1])?;
        let trusted_tag = tag(&candidate[2])?;
        if visible_tag != trusted_tag {
            return Err(crate::StateDigestError::StateInvariant);
        }
        validate_candidate_intent(&candidate[1])?;
        validate_candidate_binding(&candidate[2])?;
    }
    Ok(())
}

fn validate_decision_domain(value: &Value) -> Result<(), crate::StateDigestError> {
    let Value::Array(fields) = value else {
        return Err(crate::StateDigestError::StateInvariant);
    };
    let Some(Value::Text(tag)) = fields.first() else {
        return Err(crate::StateDigestError::StateInvariant);
    };
    match tag.as_str() {
        "choose_one" if fields.len() == 1 => Ok(()),
        "choose_many" | "order" if fields.len() == 3 => {
            uint32(&fields[1])?;
            uint32(&fields[2])?;
            Ok(())
        }
        "choose_number" if fields.len() == 3 => {
            signed(&fields[1])?;
            signed(&fields[2])?;
            Ok(())
        }
        _ => Err(crate::StateDigestError::StateInvariant),
    }
}

fn validate_decision_purpose(value: &Value) -> Result<(), crate::StateDigestError> {
    let Value::Array(fields) = value else {
        return Err(crate::StateDigestError::StateInvariant);
    };
    let Some(Value::Text(tag)) = fields.first() else {
        return Err(crate::StateDigestError::StateInvariant);
    };
    match tag.as_str() {
        "priority_action"
        | "attacker_declaration"
        | "sba_graveyard_order"
        | "cast_cost_route"
        | "mana_production_choice"
        | "mana_payment"
        | "ability_action"
        | "trigger_order"
            if fields.len() == 1 =>
        {
            Ok(())
        }
        "mode_selection" | "target_selection" | "trigger_target" if fields.len() == 2 => {
            uint32(&fields[1])?;
            Ok(())
        }
        "optional_cost_payment" if fields.len() == 2 => {
            uint32(&fields[1])?;
            Ok(())
        }
        "synthetic_assembly" if fields.len() == 2 => closed_tag(
            &fields[1],
            &["entry", "choose_count", "choose_members", "order_members"],
        ),
        "cost_operand_selection" if fields.len() == 5 => {
            uint32(&fields[1])?;
            expect_text(&fields[2], "put_counters")?;
            closed_tag(
                &fields[3],
                &["plus_one_plus_one", "minus_one_minus_one", "lore"],
            )?;
            uint32(&fields[4])?;
            Ok(())
        }
        _ => Err(crate::StateDigestError::StateInvariant),
    }
}

fn validate_candidate_intent(value: &Value) -> Result<(), crate::StateDigestError> {
    let Value::Array(fields) = value else {
        return Err(crate::StateDigestError::StateInvariant);
    };
    let Some(Value::Text(tag)) = fields.first() else {
        return Err(crate::StateDigestError::StateInvariant);
    };
    match tag.as_str() {
        "pass_priority" | "confirm" | "finalize_mana_production" if fields.len() == 2 => {
            if fields[1] != Value::Null {
                return Err(crate::StateDigestError::StateInvariant);
            }
        }
        "play_land" | "cast_spell" | "activate_ability" | "select_object" | "select_player"
            if fields.len() == 2 =>
        {
            uint(&fields[1])?;
        }
        "select_mode" if fields.len() == 2 => {
            uint32(&fields[1])?;
        }
        "choose_boolean" if fields.len() == 2 => {
            if !matches!(fields[1], Value::Bool(_)) {
                return Err(crate::StateDigestError::StateInvariant);
            }
        }
        "declare_number" if fields.len() == 2 => {
            signed(&fields[1])?;
        }
        "select_cost_route" if fields.len() == 4 => {
            let route_class = match &fields[1] {
                Value::Text(route_class) => route_class.as_str(),
                _ => return Err(crate::StateDigestError::StateInvariant),
            };
            let symbols = array_fields(&fields[2], 3)?;
            validate_fixed_u32_array(&symbols[0], 5)?;
            uint32(&symbols[1])?;
            uint32(&symbols[2])?;
            match (route_class, &fields[3]) {
                ("normal", Value::Null) => {}
                ("alternative", Value::Unsigned(_)) => {
                    uint32(&fields[3])?;
                }
                _ => return Err(crate::StateDigestError::StateInvariant),
            }
        }
        "select_mana_source" if fields.len() == 4 => {
            uint(&fields[1])?;
            uint(&fields[2])?;
            validate_fixed_u32_array(&fields[3], 12)?;
        }
        "select_mana_payment" if fields.len() == 2 => {
            validate_fixed_u32_array(&fields[1], 12)?;
        }
        "select_trigger" if fields.len() == 2 => validate_safe_trigger_descriptor(&fields[1])?,
        _ => return Err(crate::StateDigestError::StateInvariant),
    }
    Ok(())
}

fn validate_candidate_binding(value: &Value) -> Result<(), crate::StateDigestError> {
    let Value::Array(fields) = value else {
        return Err(crate::StateDigestError::StateInvariant);
    };
    let Some(Value::Text(tag)) = fields.first() else {
        return Err(crate::StateDigestError::StateInvariant);
    };
    match tag.as_str() {
        "pass_priority" | "confirm" if fields.len() == 2 && fields[1] == Value::Null => Ok(()),
        "play_land" | "cast_spell" | "activate_ability" | "select_object" | "select_player"
            if fields.len() == 2 =>
        {
            uint(&fields[1])?;
            Ok(())
        }
        "select_mode" if fields.len() == 2 => {
            uint32(&fields[1])?;
            Ok(())
        }
        "declare_number" if fields.len() == 2 => {
            signed(&fields[1])?;
            Ok(())
        }
        "choose_boolean" if fields.len() == 2 => {
            if matches!(fields[1], Value::Bool(_)) {
                Ok(())
            } else {
                Err(crate::StateDigestError::StateInvariant)
            }
        }
        "select_cost_route" if fields.len() == 2 => validate_cost_route(&fields[1]),
        "select_mana_source" if fields.len() == 7 => {
            for index in [1, 2] {
                uint(&fields[index])?;
            }
            uint32(&fields[3])?;
            nonempty_text(&fields[4])?;
            let cost = array_fields(&fields[5], 1)?;
            expect_text(&cost[0], "tap_source")?;
            validate_fixed_u32_array(&fields[6], 12)
        }
        "finalize_mana_production" if fields.len() == 2 => {
            uint(&fields[1])?;
            Ok(())
        }
        "select_mana_payment" if fields.len() == 2 => validate_fixed_u32_array(&fields[1], 12),
        "select_trigger" if fields.len() == 2 => {
            uint(&fields[1])?;
            Ok(())
        }
        _ => Err(crate::StateDigestError::StateInvariant),
    }
}

fn validate_safe_trigger_descriptor(value: &Value) -> Result<(), crate::StateDigestError> {
    let fields = array_fields(value, 4)?;
    optional_uint(&fields[0])?;
    optional_uint(&fields[1])?;
    let event_kind = match &fields[2] {
        Value::Text(event_kind) => event_kind.as_str(),
        _ => return Err(crate::StateDigestError::StateInvariant),
    };
    closed_tag(
        &fields[2],
        &[
            "spell_cast",
            "ability_activated",
            "target_became",
            "object_entered",
            "object_left_or_died",
            "beginning_of_combat",
            "attack_declared",
            "card_drawn",
            "counter_changed",
            "damage_applied",
            "life_changed",
        ],
    )?;
    if tag(&fields[3])? != event_kind {
        return Err(crate::StateDigestError::StateInvariant);
    }
    validate_safe_trigger_subject(&fields[3])
}

fn validate_safe_trigger_subject(value: &Value) -> Result<(), crate::StateDigestError> {
    let Value::Array(fields) = value else {
        return Err(crate::StateDigestError::StateInvariant);
    };
    let Some(Value::Text(tag)) = fields.first() else {
        return Err(crate::StateDigestError::StateInvariant);
    };
    match tag.as_str() {
        "spell_cast" if fields.len() == 5 => {
            uint(&fields[1])?;
            optional_uint(&fields[2])?;
            if !matches!(fields[3], Value::Bool(_)) {
                return Err(crate::StateDigestError::StateInvariant);
            }
            validate_cost_facts(&fields[4])
        }
        "ability_activated" if fields.len() == 6 => {
            uint(&fields[1])?;
            uint(&fields[2])?;
            uint(&fields[3])?;
            validate_cost_facts(&fields[4])?;
            let Value::Array(targets) = &fields[5] else {
                return Err(crate::StateDigestError::StateInvariant);
            };
            for target in targets {
                validate_safe_target(target)?;
            }
            Ok(())
        }
        "target_became" if fields.len() == 3 => {
            uint(&fields[1])?;
            validate_safe_target(&fields[2])
        }
        "object_entered" if fields.len() == 2 => optional_uint(&fields[1]),
        "object_left_or_died" if fields.len() == 3 => {
            optional_uint(&fields[1])?;
            safe_zone_tag(&fields[2])
        }
        "beginning_of_combat" if fields.len() == 3 => {
            uint(&fields[1])?;
            uint(&fields[2])?;
            Ok(())
        }
        "attack_declared" if fields.len() == 3 => {
            uint(&fields[1])?;
            let Value::Array(attackers) = &fields[2] else {
                return Err(crate::StateDigestError::StateInvariant);
            };
            let mut previous = None;
            for attacker in attackers {
                let attacker = array_fields(attacker, 2)?;
                let key = (uint(&attacker[0])?, uint(&attacker[1])?);
                if previous.is_some_and(|old| old >= key) {
                    return Err(crate::StateDigestError::StateInvariant);
                }
                previous = Some(key);
            }
            Ok(())
        }
        "card_drawn" if fields.len() == 2 => uint(&fields[1]).map(|_| ()),
        "counter_changed" if fields.len() == 5 => {
            optional_uint(&fields[1])?;
            closed_tag(
                &fields[2],
                &["plus_one_plus_one", "minus_one_minus_one", "lore"],
            )?;
            uint32(&fields[3])?;
            uint32(&fields[4])?;
            Ok(())
        }
        "damage_applied" if fields.len() == 5 => {
            optional_uint(&fields[1])?;
            validate_safe_damage_recipient(&fields[2])?;
            uint32(&fields[3])?;
            closed_tag(&fields[4], &["combat", "noncombat"])
        }
        "life_changed" if fields.len() == 5 => {
            uint(&fields[1])?;
            signed(&fields[2])?;
            signed(&fields[3])?;
            closed_tag(&fields[4], &["damage", "non_damage"])
        }
        _ => Err(crate::StateDigestError::StateInvariant),
    }
}

fn validate_safe_target(value: &Value) -> Result<(), crate::StateDigestError> {
    let fields = array_fields(value, 2)?;
    match tag(value)? {
        "object" | "player" => uint(&fields[1]).map(|_| ()),
        "stack_item" => uint32(&fields[1]).map(|_| ()),
        _ => Err(crate::StateDigestError::StateInvariant),
    }
}

fn validate_safe_damage_recipient(value: &Value) -> Result<(), crate::StateDigestError> {
    let fields = array_fields(value, 2)?;
    match tag(value)? {
        "object" | "player" => uint(&fields[1]).map(|_| ()),
        _ => Err(crate::StateDigestError::StateInvariant),
    }
}

fn safe_zone_tag(value: &Value) -> Result<(), crate::StateDigestError> {
    closed_tag(
        value,
        &[
            "library",
            "hand",
            "battlefield",
            "graveyard",
            "exile",
            "stack",
            "command",
            "ante",
            "outside",
        ],
    )
}

fn validate_stack_resolution(value: &Value) -> Result<(), crate::StateDigestError> {
    let fields = array_fields(value, 5)?;
    expect_text(&fields[0], "stack_resolution")?;
    uint(&fields[1])?;
    closed_tag(&fields[2], &["awaiting_optional_payment", "paying_mana"])?;
    if !matches!(fields[3], Value::Null) {
        validate_action_cost_facts(&fields[3])?;
    }
    validate_optional_mana_staging(&fields[4])
}

fn validate_trigger_event_value(value: &Value) -> Result<(), crate::StateDigestError> {
    let Value::Array(fields) = value else {
        return Err(crate::StateDigestError::StateInvariant);
    };
    let Some(Value::Text(tag)) = fields.first() else {
        return Err(crate::StateDigestError::StateInvariant);
    };
    match tag.as_str() {
        "spell_cast" if fields.len() == 6 => {
            uint(&fields[1])?;
            uint(&fields[2])?;
            validate_source_context(&fields[3])?;
            if !matches!(fields[4], Value::Bool(_)) {
                return Err(crate::StateDigestError::StateInvariant);
            }
            validate_cost_facts(&fields[5])
        }
        "ability_activated" if fields.len() == 6 => {
            uint(&fields[1])?;
            uint(&fields[2])?;
            validate_ability_source_context(&fields[3])?;
            validate_target_bindings(&fields[4])?;
            validate_cost_facts(&fields[5])
        }
        "target_became" if fields.len() == 4 => {
            uint(&fields[1])?;
            uint(&fields[2])?;
            validate_target_ref(&fields[3])
        }
        "object_entered" if fields.len() == 2 => validate_object_snapshot(&fields[1]),
        "object_left_or_died" if fields.len() == 3 => {
            validate_object_snapshot(&fields[1])?;
            validate_zone_location(&fields[2])
        }
        "beginning_of_combat" if fields.len() == 3 => {
            uint(&fields[1])?;
            uint(&fields[2])?;
            Ok(())
        }
        "attack_declared" if fields.len() == 3 => {
            uint(&fields[1])?;
            let Value::Array(attackers) = &fields[2] else {
                return Err(crate::StateDigestError::StateInvariant);
            };
            for attacker in attackers {
                let attacker = array_fields(attacker, 2)?;
                uint(&attacker[0])?;
                uint(&attacker[1])?;
            }
            Ok(())
        }
        "card_drawn" if fields.len() == 2 => uint(&fields[1]).map(|_| ()),
        "counter_changed" if fields.len() == 5 => {
            uint(&fields[1])?;
            closed_tag(
                &fields[2],
                &["plus_one_plus_one", "minus_one_minus_one", "lore"],
            )?;
            uint32(&fields[3])?;
            uint32(&fields[4])?;
            Ok(())
        }
        "damage_applied" if fields.len() == 5 => {
            if !matches!(fields[1], Value::Null) {
                validate_source_context(&fields[1])?;
            }
            validate_damage_recipient(&fields[2])?;
            uint32(&fields[3])?;
            closed_tag(&fields[4], &["combat", "noncombat"])
        }
        "life_changed" if fields.len() == 5 => {
            uint(&fields[1])?;
            signed(&fields[2])?;
            signed(&fields[3])?;
            closed_tag(&fields[4], &["damage", "non_damage"])
        }
        _ => Err(crate::StateDigestError::StateInvariant),
    }
}

fn validate_damage_recipient(value: &Value) -> Result<(), crate::StateDigestError> {
    let fields = array_fields(value, 2)?;
    match tag(value)? {
        "object" | "player" => uint(&fields[1]).map(|_| ()),
        _ => Err(crate::StateDigestError::StateInvariant),
    }
}

fn validate_ability_source_context(value: &Value) -> Result<(), crate::StateDigestError> {
    let fields = array_fields(value, 3)?;
    validate_source_context(&fields[0])?;
    uint(&fields[1])?;
    uint32(&fields[2])?;
    Ok(())
}

fn validate_source_context(value: &Value) -> Result<(), crate::StateDigestError> {
    let fields = array_fields(value, 3)?;
    validate_object_snapshot(&fields[0])?;
    uint32(&fields[1])?;
    nonempty_text(&fields[2])?;
    Ok(())
}

fn validate_object_snapshot(value: &Value) -> Result<(), crate::StateDigestError> {
    let fields = array_fields(value, 8)?;
    uint(&fields[0])?;
    optional_uint(&fields[1])?;
    for index in [2, 3, 4] {
        uint(&fields[index])?;
    }
    if !matches!(fields[5], Value::Bool(_)) || !matches!(fields[6], Value::Bool(_)) {
        return Err(crate::StateDigestError::StateInvariant);
    }
    validate_zone_location(&fields[7])
}

fn validate_zone_location(value: &Value) -> Result<(), crate::StateDigestError> {
    let fields = array_fields(value, 5)?;
    safe_zone_tag(&fields[0])?;
    optional_uint(&fields[1])?;
    let position = array_fields(&fields[2], 2)?;
    match tag(&fields[2])? {
        "unordered" if position[1] == Value::Null => {}
        "top" | "bottom" | "index" => {
            uint32(&position[1])?;
        }
        _ => return Err(crate::StateDigestError::StateInvariant),
    }
    closed_tag(
        &fields[3],
        &["public", "owner_only", "face_down", "private_group"],
    )?;
    if !matches!(fields[4], Value::Null | Value::Text(_)) {
        return Err(crate::StateDigestError::StateInvariant);
    }
    Ok(())
}

fn validate_mode_bindings(value: &Value) -> Result<(), crate::StateDigestError> {
    let Value::Array(bindings) = value else {
        return Err(crate::StateDigestError::StateInvariant);
    };
    let mut previous = None;
    for binding in bindings {
        let binding = array_fields(binding, 2)?;
        let slot = uint32(&binding[0])?;
        if previous.is_some_and(|prior| prior >= slot) {
            return Err(crate::StateDigestError::StateInvariant);
        }
        previous = Some(slot);
        uint32(&binding[1])?;
    }
    Ok(())
}

fn validate_target_bindings(value: &Value) -> Result<(), crate::StateDigestError> {
    let Value::Array(bindings) = value else {
        return Err(crate::StateDigestError::StateInvariant);
    };
    let mut previous = None;
    for binding in bindings {
        validate_target_binding(binding)?;
        let slot = uint32(&array_fields(binding, 2)?[0])?;
        if previous.is_some_and(|prior| prior >= slot) {
            return Err(crate::StateDigestError::StateInvariant);
        }
        previous = Some(slot);
    }
    Ok(())
}

fn validate_target_binding(value: &Value) -> Result<(), crate::StateDigestError> {
    let fields = array_fields(value, 2)?;
    uint32(&fields[0])?;
    validate_target_ref(&fields[1])
}

fn validate_target_ref(value: &Value) -> Result<(), crate::StateDigestError> {
    let fields = array_fields(value, 2)?;
    match tag(value)? {
        "object" | "player" | "stack_item" => uint(&fields[1]).map(|_| ()),
        _ => Err(crate::StateDigestError::StateInvariant),
    }
}

fn validate_cost_facts(value: &Value) -> Result<(), crate::StateDigestError> {
    let fields = array_fields(value, 2)?;
    validate_cost_route(&fields[0])?;
    let Value::Array(ids) = &fields[1] else {
        return Err(crate::StateDigestError::StateInvariant);
    };
    let mut previous = None;
    for id in ids {
        let id = uint32(id)?;
        if previous.is_some_and(|prior| prior >= id) {
            return Err(crate::StateDigestError::StateInvariant);
        }
        previous = Some(id);
    }
    Ok(())
}

fn validate_cost_route(value: &Value) -> Result<(), crate::StateDigestError> {
    if matches!(value, Value::Null) {
        return Ok(());
    }
    let Value::Array(fields) = value else {
        return Err(crate::StateDigestError::StateInvariant);
    };
    match (tag(value)?, fields.len()) {
        ("normal", 1) => Ok(()),
        ("alternative", 2) => uint32(&fields[1]).map(|_| ()),
        _ => Err(crate::StateDigestError::StateInvariant),
    }
}

fn validate_temporary_operation(value: &Value) -> Result<(), crate::StateDigestError> {
    let Value::Array(fields) = value else {
        return Err(crate::StateDigestError::StateInvariant);
    };
    match (tag(value)?, fields.len()) {
        ("power_toughness_delta", 3) => {
            signed32(&fields[1])?;
            signed32(&fields[2])?;
            Ok(())
        }
        ("grant_keyword", 2) => closed_tag(&fields[1], &["haste", "double_strike"]),
        _ => Err(crate::StateDigestError::StateInvariant),
    }
}

fn validate_sba_actions(value: &Value) -> Result<(), crate::StateDigestError> {
    let Value::Array(actions) = value else {
        return Err(crate::StateDigestError::StateInvariant);
    };
    for action in actions {
        match tag(action)? {
            "player_loses" => {
                let fields = array_fields(action, 2)?;
                uint(&fields[1])?;
            }
            "object_to_owner_graveyard" => {
                let fields = array_fields(action, 3)?;
                uint(&fields[1])?;
                let Value::Array(causes) = &fields[2] else {
                    return Err(crate::StateDigestError::StateInvariant);
                };
                for cause in causes {
                    closed_tag(cause, &["zero_toughness", "lethal_damage"])?;
                }
            }
            _ => return Err(crate::StateDigestError::StateInvariant),
        }
    }
    Ok(())
}

fn validate_fixed_u32_array(value: &Value, len: usize) -> Result<(), crate::StateDigestError> {
    let Value::Array(values) = value else {
        return Err(crate::StateDigestError::StateInvariant);
    };
    if values.len() != len {
        return Err(crate::StateDigestError::StateInvariant);
    }
    for value in values {
        uint32(value)?;
    }
    Ok(())
}

fn validate_u32_array(value: &Value) -> Result<(), crate::StateDigestError> {
    let Value::Array(values) = value else {
        return Err(crate::StateDigestError::StateInvariant);
    };
    for value in values {
        uint32(value)?;
    }
    Ok(())
}

fn validate_u64_array(value: &Value) -> Result<(), crate::StateDigestError> {
    let Value::Array(values) = value else {
        return Err(crate::StateDigestError::StateInvariant);
    };
    for value in values {
        uint(value)?;
    }
    Ok(())
}

fn array_fields(value: &Value, len: usize) -> Result<&[Value], crate::StateDigestError> {
    match value {
        Value::Array(fields) if fields.len() == len => Ok(fields),
        _ => Err(crate::StateDigestError::StateInvariant),
    }
}

fn tag(value: &Value) -> Result<&str, crate::StateDigestError> {
    match value {
        Value::Array(fields) => match fields.first() {
            Some(Value::Text(tag)) => Ok(tag),
            _ => Err(crate::StateDigestError::StateInvariant),
        },
        _ => Err(crate::StateDigestError::StateInvariant),
    }
}

fn expect_text(value: &Value, expected: &str) -> Result<(), crate::StateDigestError> {
    if value == &text(expected) {
        Ok(())
    } else {
        Err(crate::StateDigestError::StateInvariant)
    }
}

fn nonempty_text(value: &Value) -> Result<(), crate::StateDigestError> {
    match value {
        Value::Text(value) if CardSemanticProfileId::parse(value.clone()).is_ok() => Ok(()),
        _ => Err(crate::StateDigestError::StateInvariant),
    }
}

fn closed_tag(value: &Value, allowed: &[&str]) -> Result<(), crate::StateDigestError> {
    match value {
        Value::Text(value) if allowed.contains(&value.as_str()) => Ok(()),
        _ => Err(crate::StateDigestError::StateInvariant),
    }
}

fn uint(value: &Value) -> Result<u64, crate::StateDigestError> {
    match value {
        Value::Unsigned(value) => Ok(*value),
        _ => Err(crate::StateDigestError::StateInvariant),
    }
}

fn uint32(value: &Value) -> Result<u32, crate::StateDigestError> {
    u32::try_from(uint(value)?).map_err(|_| crate::StateDigestError::StateInvariant)
}

fn signed(value: &Value) -> Result<i64, crate::StateDigestError> {
    match value {
        Value::Signed(value) => Ok(*value),
        Value::Unsigned(value) => {
            i64::try_from(*value).map_err(|_| crate::StateDigestError::StateInvariant)
        }
        _ => Err(crate::StateDigestError::StateInvariant),
    }
}

fn signed32(value: &Value) -> Result<i32, crate::StateDigestError> {
    i32::try_from(signed(value)?).map_err(|_| crate::StateDigestError::StateInvariant)
}

fn optional_uint(value: &Value) -> Result<(), crate::StateDigestError> {
    if matches!(value, Value::Null) {
        Ok(())
    } else {
        uint(value).map(|_| ())
    }
}

fn state_value_validated(state: &EngineStatePartsV3) -> Result<Value, crate::StateDigestError> {
    // Strip successor-only stack payloads solely from this temporary V6
    // component projection; they are encoded by zones_v2 below.
    let mut predecessor = state.predecessor_v5.clone();
    for record in predecessor.zones.stack_records.values_mut() {
        record.payload = None;
    }
    let v2 = EngineStatePartsV2 {
        predecessor_v5: predecessor,
        execution_v3: ExecutionStateV3::default(),
        card_rules_state: state.card_rules_state.clone(),
    };
    let common_bytes = crate::canonical_state_bytes_v6_with_execution_v3(
        &v2.materialize(),
        &v2.execution_v3,
        v2.card_rules_state.clone(),
    )?;
    let common =
        cbor::decode_canonical(&common_bytes).map_err(crate::StateDigestError::Persistence)?;
    let Value::Array(mut fields) = common else {
        return Err(crate::StateDigestError::StateInvariant);
    };
    if fields.len() != 14 {
        return Err(crate::StateDigestError::StateInvariant);
    }
    let Value::Array(old_zones) = fields[4].clone() else {
        return Err(crate::StateDigestError::StateInvariant);
    };
    if old_zones.len() != 5 {
        return Err(crate::StateDigestError::StateInvariant);
    }
    fields[0] = text(FULL_STATE_DIGEST_INPUT_SCHEMA_V7);
    fields[1] = text(FULL_STATE_DIGEST_DOMAIN_V7);
    fields[4] = zones_v2_value(state, &old_zones)?;
    fields[6] = execution_v4_value(&state.execution_v4)?;
    let value = Value::Array(fields);
    validate_v7_envelope_shape(&value)?;
    Ok(value)
}

fn zones_v2_value(
    state: &EngineStatePartsV3,
    old: &[Value],
) -> Result<Value, crate::StateDigestError> {
    Ok(array([
        text("zones_v2"),
        old[0].clone(),
        old[1].clone(),
        old[2].clone(),
        array(
            state
                .predecessor_v5
                .zones
                .stack_records
                .values()
                .map(|record| {
                    let payload = record
                        .payload
                        .as_ref()
                        .ok_or(crate::StateDigestError::StateInvariant)?;
                    Ok(array([
                        u(record.id.0),
                        u(record.controller.0),
                        stack_payload_value(payload),
                    ]))
                })
                .collect::<Result<Vec<_>, crate::StateDigestError>>()?,
        ),
        array(
            state
                .predecessor_v5
                .zones
                .stack_order
                .iter()
                .map(|id| u(id.0)),
        ),
    ]))
}

fn execution_v4_value(state: &ExecutionStateV4) -> Result<Value, crate::StateDigestError> {
    Ok(array([
        text("execution_v4"),
        optional(state.pending_decision.as_ref().map(decision_request_value)),
        array(
            state
                .continuations
                .values()
                .map(continuation_value)
                .collect::<Result<Vec<_>, _>>()?,
        ),
        array(state.effects.values().map(temporary_effect_value)),
        array(state.waiting_triggers.values().map(pending_trigger_value)),
        array([]),
    ]))
}

fn stack_payload_value(value: &StackItemPayload) -> Value {
    match value {
        StackItemPayload::Spell {
            stack_card_object,
            card_definition_id,
            face_key,
            semantic_profile_id,
            modes,
            targets,
            cost_facts,
        } => array([
            text("spell"),
            u(stack_card_object.0),
            u(card_definition_id.0),
            u(u64::from(face_key.0)),
            text(semantic_profile_id.as_str()),
            array(modes.iter().map(mode_value)),
            array(targets.iter().map(target_binding_value)),
            cost_facts_value(cost_facts),
        ]),
        StackItemPayload::ActivatedAbility {
            source_context,
            modes,
            targets,
            cost_facts,
        } => array([
            text("activated_ability"),
            ability_source_context_value(source_context),
            array(modes.iter().map(mode_value)),
            array(targets.iter().map(target_binding_value)),
            cost_facts_value(cost_facts),
        ]),
        StackItemPayload::TriggeredAbility {
            originating_trigger,
            source_context,
            captured_trigger_context,
            targets,
        } => array([
            text("triggered_ability"),
            u(originating_trigger.0),
            ability_source_context_value(source_context),
            trigger_event_value(captured_trigger_context),
            array(targets.iter().map(target_binding_value)),
        ]),
    }
}

fn source_context_value(value: &SourceContext) -> Value {
    array([
        object_snapshot_value(&value.snapshot),
        u(u64::from(value.face_key.0)),
        text(value.semantic_profile_id.as_str()),
    ])
}

fn ability_source_context_value(value: &AbilitySourceContext) -> Value {
    array([
        source_context_value(&value.source),
        u(value.ability_instance_id.0),
        u(u64::from(value.ability_key.0)),
    ])
}

fn object_snapshot_value(value: &crate::ObjectSnapshot) -> Value {
    array([
        u(value.object.0),
        optional(value.physical_card.map(|id| u(id.0))),
        u(value.card_definition.0),
        u(value.owner.0),
        u(value.controller.0),
        Value::Bool(value.tapped),
        Value::Bool(value.face_down),
        crate::digest_v5::zone_location_value(&value.location),
    ])
}

fn target_ref_value(value: TargetRef) -> Value {
    match value {
        TargetRef::Object(id) => array([text("object"), u(id.0)]),
        TargetRef::Player(id) => array([text("player"), u(id.0)]),
        TargetRef::StackItem(id) => array([text("stack_item"), u(id.0)]),
    }
}

fn target_binding_value(value: &TargetBinding) -> Value {
    array([
        u(u64::from(value.target_slot)),
        target_ref_value(value.target),
    ])
}

fn mode_value(value: &ModeBinding) -> Value {
    array([
        u(u64::from(value.mode_slot)),
        u(u64::from(value.selected_mode)),
    ])
}

fn route_value(value: Option<CostRoute>) -> Value {
    match value {
        None => Value::Null,
        Some(CostRoute::Normal) => array([text("normal")]),
        Some(CostRoute::Alternative {
            profile_local_route_id,
        }) => array([text("alternative"), u(u64::from(profile_local_route_id))]),
    }
}

fn cost_facts_value(value: &CostFacts) -> Value {
    array([
        route_value(value.selected_route),
        array(
            value
                .paid_additional_cost_ids
                .iter()
                .map(|id| u(u64::from(*id))),
        ),
    ])
}

fn action_cost_facts_value(value: &ActionCostFacts) -> Value {
    array([
        optional(value.mana_cost.map(mana_cost_value)),
        array(value.reserved_nonmana_costs.iter().map(reserved_cost_value)),
        array(value.selected_cost_operands.iter().map(cost_operand_value)),
    ])
}

fn mana_cost_value(value: ManaCost) -> Value {
    array([
        array(
            value
                .colored_wubrg_counts
                .iter()
                .map(|amount| u(u64::from(*amount))),
        ),
        u(u64::from(value.colorless_count)),
        u(u64::from(value.generic_count)),
    ])
}

fn reserved_cost_value(value: &ReservedNonManaCost) -> Value {
    match value {
        ReservedNonManaCost::TapSource => array([text("tap_source")]),
        ReservedNonManaCost::SacrificeSource => array([text("sacrifice_source")]),
    }
}

fn cost_operand_value(value: &SelectedCostOperand) -> Value {
    match value {
        SelectedCostOperand::PutCounters {
            object,
            counter_kind,
            count,
        } => array([
            text("put_counters"),
            u(object.0),
            text(match counter_kind {
                crate::CounterKindV1::PlusOnePlusOne => "plus_one_plus_one",
                crate::CounterKindV1::MinusOneMinusOne => "minus_one_minus_one",
                crate::CounterKindV1::Lore => "lore",
            }),
            u(u64::from(*count)),
        ]),
    }
}

fn mana_staging_value(value: &ManaPaymentStaging) -> Value {
    array([
        text("mana_payment_staging"),
        text(match value.stage {
            ManaPaymentStage::SelectingSources => "selecting_sources",
            ManaPaymentStage::AwaitingFinalAllocation => "awaiting_final_allocation",
        }),
        array(value.mana_source_activations.iter().map(mana_source_value)),
    ])
}

fn mana_source_value(value: &ManaSourceActivation) -> Value {
    array([
        u(value.source_object.0),
        u(value.source_ability_instance.0),
        u(u64::from(value.ability_key.0)),
        text(value.semantic_profile_id.as_str()),
        match value.activation_cost_receipt {
            ManaSourceActivationCost::TapSource => array([text("tap_source")]),
        },
        array(
            value
                .produced_buckets
                .iter()
                .map(|amount| u(u64::from(*amount))),
        ),
    ])
}

fn cast_continuation_value(value: &CastContinuation) -> Value {
    array([
        text("cast"),
        u(value.actor.0),
        u(value.spell_object.0),
        u(value.card_definition_id.0),
        u(u64::from(value.face_key.0)),
        text(value.semantic_profile_id.as_str()),
        text(match value.stage {
            CastContinuationStage::SelectingCostRoute => "selecting_cost_route",
            CastContinuationStage::SelectingModes => "selecting_modes",
            CastContinuationStage::SelectingTargets => "selecting_targets",
            CastContinuationStage::SelectingAdditionalCosts => "selecting_additional_costs",
            CastContinuationStage::SelectingCostOperands => "selecting_cost_operands",
            CastContinuationStage::PayingMana => "paying_mana",
        }),
        route_value(value.selected_route),
        array(value.modes.iter().map(mode_value)),
        array(value.targets.iter().map(target_binding_value)),
        array(value.paid_cost_choices.iter().map(|id| u(u64::from(*id)))),
        action_cost_facts_value(&value.action_cost_facts),
        optional(value.mana_payment_staging.as_ref().map(mana_staging_value)),
    ])
}

fn activation_continuation_value(value: &NonManaActivationContinuation) -> Value {
    array([
        text("nonmana_activation"),
        u(value.actor.0),
        u(value.source_object.0),
        u(value.source_ability_instance.0),
        u(u64::from(value.ability_key.0)),
        text(value.semantic_profile_id.as_str()),
        text(match value.stage {
            NonManaActivationStage::SelectingModes => "selecting_modes",
            NonManaActivationStage::SelectingTargets => "selecting_targets",
            NonManaActivationStage::SelectingCostOperands => "selecting_cost_operands",
            NonManaActivationStage::PayingMana => "paying_mana",
        }),
        array(value.modes.iter().map(mode_value)),
        array(value.targets.iter().map(target_binding_value)),
        action_cost_facts_value(&value.action_cost_facts),
        optional(value.mana_payment_staging.as_ref().map(mana_staging_value)),
    ])
}

fn stack_resolution_value(value: &StackResolutionContinuation) -> Value {
    array([
        text("stack_resolution"),
        u(value.resolving_stack_object.0),
        text(match value.stage {
            StackResolutionStage::AwaitingOptionalPayment => "awaiting_optional_payment",
            StackResolutionStage::PayingMana => "paying_mana",
        }),
        optional(
            value
                .action_cost_facts
                .as_ref()
                .map(action_cost_facts_value),
        ),
        optional(value.mana_payment_staging.as_ref().map(mana_staging_value)),
    ])
}

fn continuation_value(value: &ContinuationRecordV3) -> Result<Value, crate::StateDigestError> {
    let payload = match &value.payload {
        ContinuationPayloadV3::SyntheticAssembly {
            actor,
            stage,
            selected_count,
            selected_piece_keys,
            ordered_piece_keys,
        } => array([
            text("synthetic_assembly"),
            u(actor.0),
            text(match stage {
                AssemblyStageV2::ChooseCount => "choose_count",
                AssemblyStageV2::ChooseMembers => "choose_members",
                AssemblyStageV2::OrderMembers => "order_members",
            }),
            optional(selected_count.map(|count| u(u64::from(count)))),
            array(selected_piece_keys.iter().map(|key| u(u64::from(*key)))),
            array(ordered_piece_keys.iter().map(|key| u(u64::from(*key)))),
        ]),
        ContinuationPayloadV3::MagicSbaGraveyardOrderV1 {
            round_start_revision,
            selected_sba_actions,
            apnap_owners,
            next_owner_index,
            completed_owner_orders,
        } => array([
            text("magic_sba_graveyard_order_v1"),
            u(round_start_revision.0),
            array(selected_sba_actions.iter().map(|action| match action {
                crate::SbaSelectedActionV1::PlayerLoses { player } => {
                    array([text("player_loses"), u(player.0)])
                }
                crate::SbaSelectedActionV1::ObjectToOwnerGraveyard { object, causes } => array([
                    text("object_to_owner_graveyard"),
                    u(object.0),
                    array(causes.iter().map(|cause| {
                        text(match cause {
                            crate::SbaObjectCauseV1::ZeroToughness => "zero_toughness",
                            crate::SbaObjectCauseV1::LethalDamage => "lethal_damage",
                        })
                    })),
                ]),
            })),
            array(apnap_owners.iter().map(|player| u(player.0))),
            u(u64::from(*next_owner_index)),
            array(completed_owner_orders.iter().map(|order| {
                array([
                    u(order.owner.0),
                    array(order.top_to_bottom.iter().map(|id| u(id.0))),
                ])
            })),
        ]),
        ContinuationPayloadV3::Cast(value) => cast_continuation_value(value),
        ContinuationPayloadV3::NonManaActivation(value) => activation_continuation_value(value),
        ContinuationPayloadV3::TriggerPlacement(value) => trigger_placement_value(value),
        ContinuationPayloadV3::StackResolution(value) => stack_resolution_value(value),
    };
    Ok(array([
        u(value.id.0),
        u(value.created_at_revision.0),
        payload,
    ]))
}

fn trigger_placement_value(value: &TriggerPlacementContinuation) -> Value {
    array([
        text("trigger_placement"),
        array(value.apnap_actors.iter().map(|player| u(player.0))),
        u(u64::from(value.current_actor_index)),
        array(value.pending_trigger_ids.iter().map(|id| u(id.0))),
        array(value.completed_orders.iter().map(|order| {
            array([
                u(order.actor.0),
                array(order.ordered_trigger_ids.iter().map(|id| u(id.0))),
            ])
        })),
        array(
            value.selected_trigger_targets.iter().map(|target| {
                array([u(target.trigger_id.0), target_binding_value(&target.target)])
            }),
        ),
        array(
            value
                .actor_request_roots
                .iter()
                .map(trigger_actor_root_value),
        ),
    ])
}

fn trigger_actor_root_value(value: &TriggerActorRequestRoot) -> Value {
    array([u(value.actor.0), u(value.first_decision_id.0)])
}

fn pending_trigger_value(value: &PendingTriggerRecord) -> Value {
    array([
        u(value.id.0),
        u(value.controller.0),
        ability_source_context_value(&value.source_context),
        trigger_event_value(&value.trigger_context),
        text(match value.target_timing {
            TriggerTargetTiming::NoTargets => "no_targets",
            TriggerTargetTiming::CapturedFromEvent => "captured_from_event",
            TriggerTargetTiming::ChooseOnPlacement => "choose_on_placement",
        }),
    ])
}

fn temporary_effect_value(value: &TemporaryEffectRecord) -> Value {
    array([
        u(value.id.0),
        array(value.affected_objects.iter().map(|id| u(id.0))),
        match value.operation {
            TemporaryOperation::PowerToughnessDelta { power, toughness } => array([
                text("power_toughness_delta"),
                i(i64::from(power)),
                i(i64::from(toughness)),
            ]),
            TemporaryOperation::GrantKeyword { keyword } => array([
                text("grant_keyword"),
                text(match keyword {
                    TemporaryKeyword::Haste => "haste",
                    TemporaryKeyword::DoubleStrike => "double_strike",
                }),
            ]),
        },
        match value.expiry {
            EffectExpiry::UntilEndOfTurn { turn_number } => {
                array([text("until_end_of_turn"), u(turn_number)])
            }
        },
        optional(value.timestamp.map(effect_timestamp_value)),
    ])
}

fn effect_timestamp_value(value: EffectTimestamp) -> Value {
    array([
        u(value.creation_revision.0),
        u(u64::from(value.operation_ordinal)),
    ])
}

fn trigger_event_value(value: &TriggerEventSnapshot) -> Value {
    match value {
        TriggerEventSnapshot::SpellCast {
            actor,
            stack_item,
            spell,
            is_creature_spell,
            cost_facts,
        } => array([
            text("spell_cast"),
            u(actor.0),
            u(stack_item.0),
            source_context_value(spell),
            Value::Bool(*is_creature_spell),
            cost_facts_value(cost_facts),
        ]),
        TriggerEventSnapshot::AbilityActivated {
            actor,
            stack_item,
            source,
            targets,
            cost_facts,
        } => array([
            text("ability_activated"),
            u(actor.0),
            u(stack_item.0),
            ability_source_context_value(source),
            array(targets.iter().map(target_binding_value)),
            cost_facts_value(cost_facts),
        ]),
        TriggerEventSnapshot::TargetBecame {
            actor,
            source_stack_item,
            target,
        } => array([
            text("target_became"),
            u(actor.0),
            u(source_stack_item.0),
            target_ref_value(*target),
        ]),
        TriggerEventSnapshot::ObjectEntered { object } => {
            array([text("object_entered"), object_snapshot_value(object)])
        }
        TriggerEventSnapshot::ObjectLeftOrDied {
            last_known,
            destination,
        } => array([
            text("object_left_or_died"),
            object_snapshot_value(last_known),
            crate::digest_v5::zone_location_value(destination),
        ]),
        TriggerEventSnapshot::BeginningOfCombat {
            active_player,
            turn_number,
        } => array([
            text("beginning_of_combat"),
            u(active_player.0),
            u(*turn_number),
        ]),
        TriggerEventSnapshot::AttackDeclared {
            controller,
            attackers,
        } => array([
            text("attack_declared"),
            u(controller.0),
            array(attackers.iter().map(attacker_fact_value)),
        ]),
        TriggerEventSnapshot::CardDrawn { player } => array([text("card_drawn"), u(player.0)]),
        TriggerEventSnapshot::CounterChanged {
            object,
            kind,
            before,
            after,
        } => array([
            text("counter_changed"),
            u(object.0),
            text(match kind {
                crate::CounterKindV1::PlusOnePlusOne => "plus_one_plus_one",
                crate::CounterKindV1::MinusOneMinusOne => "minus_one_minus_one",
                crate::CounterKindV1::Lore => "lore",
            }),
            u(u64::from(*before)),
            u(u64::from(*after)),
        ]),
        TriggerEventSnapshot::DamageApplied {
            source,
            recipient,
            amount,
            damage_kind,
        } => array([
            text("damage_applied"),
            optional(source.as_ref().map(source_context_value)),
            damage_recipient_value(*recipient),
            u(u64::from(*amount)),
            text(match damage_kind {
                DamageKind::Combat => "combat",
                DamageKind::Noncombat => "noncombat",
            }),
        ]),
        TriggerEventSnapshot::LifeChanged {
            player,
            before,
            after,
            cause,
        } => array([
            text("life_changed"),
            u(player.0),
            i(*before),
            i(*after),
            text(match cause {
                LifeChangeCause::Damage => "damage",
                LifeChangeCause::NonDamage => "non_damage",
            }),
        ]),
    }
}

fn attacker_fact_value(value: &AttackerFact) -> Value {
    array([u(value.object.0), u(value.defending_player.0)])
}

fn damage_recipient_value(value: DamageRecipient) -> Value {
    match value {
        DamageRecipient::Object(id) => array([text("object"), u(id.0)]),
        DamageRecipient::Player(id) => array([text("player"), u(id.0)]),
    }
}

fn decision_request_value(value: &AuthoritativeDecisionRequestV4) -> Value {
    array([
        u(value.decision_id.0),
        u(value.player_decision_id.0),
        u(value.state_revision.0),
        u(value.view_sequence.0),
        u(value.actor.0),
        text(match value.visibility {
            DecisionVisibility::Public => "public",
            DecisionVisibility::ActingPlayerOnly => "acting_player_only",
            DecisionVisibility::Mixed => "mixed",
        }),
        decision_domain_value(&value.decision_domain_v2),
        decision_purpose_value(&value.purpose),
        optional(value.parent_player_decision_id.map(|id| u(id.0))),
        optional(value.continuation_id.map(|id| u(id.0))),
        array(value.candidates.iter().map(candidate_value)),
    ])
}

fn decision_domain_value(value: &DecisionDomainV2) -> Value {
    match value {
        DecisionDomainV2::ChooseOne => array([text("choose_one")]),
        DecisionDomainV2::ChooseMany { minimum, maximum } => array([
            text("choose_many"),
            u(u64::from(*minimum)),
            u(u64::from(*maximum)),
        ]),
        DecisionDomainV2::ChooseNumber { minimum, maximum } => {
            array([text("choose_number"), i(*minimum), i(*maximum)])
        }
        DecisionDomainV2::Order { minimum, maximum } => array([
            text("order"),
            u(u64::from(*minimum)),
            u(u64::from(*maximum)),
        ]),
    }
}

fn decision_purpose_value(value: &DecisionPurposeV4) -> Value {
    match value {
        DecisionPurposeV4::PriorityAction => array([text("priority_action")]),
        DecisionPurposeV4::AttackerDeclaration => array([text("attacker_declaration")]),
        DecisionPurposeV4::SbaGraveyardOrder => array([text("sba_graveyard_order")]),
        DecisionPurposeV4::CastCostRoute => array([text("cast_cost_route")]),
        DecisionPurposeV4::ModeSelection { mode_slot } => {
            array([text("mode_selection"), u(u64::from(*mode_slot))])
        }
        DecisionPurposeV4::TargetSelection { target_slot } => {
            array([text("target_selection"), u(u64::from(*target_slot))])
        }
        DecisionPurposeV4::CostOperandSelection {
            cost_slot,
            operation,
            counter_kind,
            count,
        } => array([
            text("cost_operand_selection"),
            u(u64::from(*cost_slot)),
            text(match operation {
                mtgml_decision::CostOperandOperationV1::PutCounters => "put_counters",
            }),
            text(match counter_kind {
                mtgml_decision::CounterKindV1::PlusOnePlusOne => "plus_one_plus_one",
                mtgml_decision::CounterKindV1::MinusOneMinusOne => "minus_one_minus_one",
                mtgml_decision::CounterKindV1::Lore => "lore",
            }),
            u(u64::from(*count)),
        ]),
        DecisionPurposeV4::ManaProductionChoice => array([text("mana_production_choice")]),
        DecisionPurposeV4::ManaPayment => array([text("mana_payment")]),
        DecisionPurposeV4::OptionalCostPayment {
            profile_local_cost_id,
        } => array([
            text("optional_cost_payment"),
            u(u64::from(*profile_local_cost_id)),
        ]),
        DecisionPurposeV4::AbilityAction => array([text("ability_action")]),
        DecisionPurposeV4::TriggerOrder => array([text("trigger_order")]),
        DecisionPurposeV4::TriggerTarget { target_slot } => {
            array([text("trigger_target"), u(u64::from(*target_slot))])
        }
        DecisionPurposeV4::SyntheticAssembly { stage } => array([
            text("synthetic_assembly"),
            text(match stage {
                mtgml_decision::SyntheticAssemblyStageV1::Entry => "entry",
                mtgml_decision::SyntheticAssemblyStageV1::ChooseCount => "choose_count",
                mtgml_decision::SyntheticAssemblyStageV1::ChooseMembers => "choose_members",
                mtgml_decision::SyntheticAssemblyStageV1::OrderMembers => "order_members",
            }),
        ]),
    }
}

fn candidate_value(value: &AuthoritativeCandidateV4) -> Value {
    array([
        u(u64::from(value.candidate_id.0)),
        candidate_intent_value(&value.visible_intent),
        candidate_binding_value(&value.trusted_binding),
    ])
}

fn candidate_intent_value(value: &CandidateIntentV4) -> Value {
    match value {
        CandidateIntentV4::PassPriority => array([text("pass_priority"), Value::Null]),
        CandidateIntentV4::PlayLand { object } => array([text("play_land"), u(object.0)]),
        CandidateIntentV4::CastSpell { object } => array([text("cast_spell"), u(object.0)]),
        CandidateIntentV4::ActivateAbility { ability } => {
            array([text("activate_ability"), u(ability.0)])
        }
        CandidateIntentV4::SelectObject { object } => array([text("select_object"), u(object.0)]),
        CandidateIntentV4::SelectPlayer { player } => array([text("select_player"), u(player.0)]),
        CandidateIntentV4::SelectMode { mode_index } => {
            array([text("select_mode"), u(u64::from(*mode_index))])
        }
        CandidateIntentV4::ChooseBoolean { value } => {
            array([text("choose_boolean"), Value::Bool(*value)])
        }
        CandidateIntentV4::DeclareNumber { value } => array([text("declare_number"), i(*value)]),
        CandidateIntentV4::Confirm => array([text("confirm"), Value::Null]),
        CandidateIntentV4::SelectCostRoute { descriptor } => array([
            text("select_cost_route"),
            text(match descriptor.route_class {
                mtgml_decision::CostRouteClassV1::Normal => "normal",
                mtgml_decision::CostRouteClassV1::Alternative => "alternative",
            }),
            printed_symbols_value(descriptor.printed_mana_symbols),
            optional(
                descriptor
                    .profile_local_option_ordinal
                    .map(|id| u(u64::from(id))),
            ),
        ]),
        CandidateIntentV4::SelectManaSource {
            source,
            ability,
            produced_buckets,
        } => array([
            text("select_mana_source"),
            u(source.0),
            u(ability.0),
            array(produced_buckets.iter().map(|amount| u(u64::from(*amount)))),
        ]),
        CandidateIntentV4::FinalizeManaProduction => {
            array([text("finalize_mana_production"), Value::Null])
        }
        CandidateIntentV4::SelectManaPayment { spent_buckets } => array([
            text("select_mana_payment"),
            array(spent_buckets.iter().map(|amount| u(u64::from(*amount)))),
        ]),
        CandidateIntentV4::SelectTrigger { trigger } => array([
            text("select_trigger"),
            safe_trigger_descriptor_value(trigger),
        ]),
    }
}

fn candidate_binding_value(value: &EngineCandidateBindingV4) -> Value {
    match value {
        EngineCandidateBindingV4::PassPriority => array([text("pass_priority"), Value::Null]),
        EngineCandidateBindingV4::PlayLand { object } => array([text("play_land"), u(object.0)]),
        EngineCandidateBindingV4::CastSpell { object } => array([text("cast_spell"), u(object.0)]),
        EngineCandidateBindingV4::ActivateAbility { ability } => {
            array([text("activate_ability"), u(ability.0)])
        }
        EngineCandidateBindingV4::SelectObject { object } => {
            array([text("select_object"), u(object.0)])
        }
        EngineCandidateBindingV4::SelectPlayer { player } => {
            array([text("select_player"), u(player.0)])
        }
        EngineCandidateBindingV4::SelectMode { mode_index } => {
            array([text("select_mode"), u(u64::from(*mode_index))])
        }
        EngineCandidateBindingV4::ChooseBoolean { value } => {
            array([text("choose_boolean"), Value::Bool(*value)])
        }
        EngineCandidateBindingV4::DeclareNumber { value } => {
            array([text("declare_number"), i(*value)])
        }
        EngineCandidateBindingV4::Confirm => array([text("confirm"), Value::Null]),
        EngineCandidateBindingV4::SelectCostRoute { route } => array([
            text("select_cost_route"),
            match route {
                mtgml_decision::CostRouteV1::Normal => array([text("normal")]),
                mtgml_decision::CostRouteV1::Alternative { route_id } => {
                    array([text("alternative"), u(u64::from(*route_id))])
                }
            },
        ]),
        EngineCandidateBindingV4::SelectManaSource {
            source,
            ability,
            ability_key,
            semantic_profile_id,
            activation_cost,
            produced_buckets,
        } => array([
            text("select_mana_source"),
            u(source.0),
            u(ability.0),
            u(u64::from(ability_key.0)),
            text(semantic_profile_id.as_str()),
            match activation_cost {
                mtgml_decision::ManaSourceActivationCostV1::TapSource => {
                    array([text("tap_source")])
                }
            },
            array(produced_buckets.iter().map(|amount| u(u64::from(*amount)))),
        ]),
        EngineCandidateBindingV4::FinalizeManaProduction { continuation } => {
            array([text("finalize_mana_production"), u(continuation.0)])
        }
        EngineCandidateBindingV4::SelectManaPayment { spent_buckets } => array([
            text("select_mana_payment"),
            array(spent_buckets.iter().map(|amount| u(u64::from(*amount)))),
        ]),
        EngineCandidateBindingV4::SelectTrigger { trigger } => {
            array([text("select_trigger"), u(trigger.0)])
        }
    }
}

fn printed_symbols_value(value: mtgml_decision::PrintedManaSymbolsV1) -> Value {
    array([
        array(
            value
                .colored_wubrg_counts
                .iter()
                .map(|amount| u(u64::from(*amount))),
        ),
        u(u64::from(value.colorless_count)),
        u(u64::from(value.generic_count)),
    ])
}

fn safe_trigger_descriptor_value(value: &SafeTriggerDescriptorV1) -> Value {
    array([
        optional(value.source_object.map(|id| u(id.0))),
        optional(value.source_ability.map(|id| u(id.0))),
        text(match value.event_kind {
            mtgml_decision::TriggerEventKindV1::SpellCast => "spell_cast",
            mtgml_decision::TriggerEventKindV1::AbilityActivated => "ability_activated",
            mtgml_decision::TriggerEventKindV1::TargetBecame => "target_became",
            mtgml_decision::TriggerEventKindV1::ObjectEntered => "object_entered",
            mtgml_decision::TriggerEventKindV1::ObjectLeftOrDied => "object_left_or_died",
            mtgml_decision::TriggerEventKindV1::BeginningOfCombat => "beginning_of_combat",
            mtgml_decision::TriggerEventKindV1::AttackDeclared => "attack_declared",
            mtgml_decision::TriggerEventKindV1::CardDrawn => "card_drawn",
            mtgml_decision::TriggerEventKindV1::CounterChanged => "counter_changed",
            mtgml_decision::TriggerEventKindV1::DamageApplied => "damage_applied",
            mtgml_decision::TriggerEventKindV1::LifeChanged => "life_changed",
        }),
        safe_trigger_subject_value(&value.subject),
    ])
}

fn safe_trigger_subject_value(value: &SafeTriggerSubjectV1) -> Value {
    match value {
        SafeTriggerSubjectV1::SpellCast {
            actor,
            spell_source_object,
            creature_spell,
            cost_facts,
        } => array([
            text("spell_cast"),
            u(actor.0),
            optional(spell_source_object.map(|id| u(id.0))),
            Value::Bool(*creature_spell),
            decision_cost_facts_value(cost_facts),
        ]),
        SafeTriggerSubjectV1::AbilityActivated {
            actor,
            activated_source_object,
            activated_source_ability,
            cost_facts,
            targets,
        } => array([
            text("ability_activated"),
            u(actor.0),
            u(activated_source_object.0),
            u(activated_source_ability.0),
            decision_cost_facts_value(cost_facts),
            array(targets.iter().map(safe_target_value)),
        ]),
        SafeTriggerSubjectV1::TargetBecame { actor, target } => {
            array([text("target_became"), u(actor.0), safe_target_value(target)])
        }
        SafeTriggerSubjectV1::ObjectEntered { object } => {
            array([text("object_entered"), optional(object.map(|id| u(id.0)))])
        }
        SafeTriggerSubjectV1::ObjectLeftOrDied {
            last_known_object,
            destination,
        } => array([
            text("object_left_or_died"),
            optional(last_known_object.map(|id| u(id.0))),
            text(safe_zone_name(*destination)),
        ]),
        SafeTriggerSubjectV1::BeginningOfCombat {
            active_player,
            turn_number,
        } => array([
            text("beginning_of_combat"),
            u(active_player.0),
            u(*turn_number),
        ]),
        SafeTriggerSubjectV1::AttackDeclared {
            controller,
            attackers,
        } => {
            array([
                text("attack_declared"),
                u(controller.0),
                array(attackers.iter().map(|attacker| {
                    array([u(attacker.attacker.0), u(attacker.defending_player.0)])
                })),
            ])
        }
        SafeTriggerSubjectV1::CardDrawn { player } => array([text("card_drawn"), u(player.0)]),
        SafeTriggerSubjectV1::CounterChanged {
            object,
            counter_kind,
            before,
            after,
        } => array([
            text("counter_changed"),
            optional(object.map(|id| u(id.0))),
            text(match counter_kind {
                mtgml_decision::CounterKindV1::PlusOnePlusOne => "plus_one_plus_one",
                mtgml_decision::CounterKindV1::MinusOneMinusOne => "minus_one_minus_one",
                mtgml_decision::CounterKindV1::Lore => "lore",
            }),
            u(u64::from(*before)),
            u(u64::from(*after)),
        ]),
        SafeTriggerSubjectV1::DamageApplied {
            source_object,
            recipient,
            amount,
            damage_kind,
        } => array([
            text("damage_applied"),
            optional(source_object.map(|id| u(id.0))),
            match recipient {
                SafeDamageRecipientV1::Object { object } => array([text("object"), u(object.0)]),
                SafeDamageRecipientV1::Player { player } => array([text("player"), u(player.0)]),
            },
            u(u64::from(*amount)),
            text(match damage_kind {
                mtgml_decision::DamageKindV1::Combat => "combat",
                mtgml_decision::DamageKindV1::Noncombat => "noncombat",
            }),
        ]),
        SafeTriggerSubjectV1::LifeChanged {
            player,
            before,
            after,
            cause,
        } => array([
            text("life_changed"),
            u(player.0),
            i(*before),
            i(*after),
            text(match cause {
                mtgml_decision::LifeChangeCauseV1::Damage => "damage",
                mtgml_decision::LifeChangeCauseV1::NonDamage => "non_damage",
            }),
        ]),
    }
}

fn safe_target_value(value: &SafeTargetDescriptorV1) -> Value {
    match value {
        SafeTargetDescriptorV1::Object { object } => array([text("object"), u(object.0)]),
        SafeTargetDescriptorV1::Player { player } => array([text("player"), u(player.0)]),
        SafeTargetDescriptorV1::StackItem {
            stack_position_from_top,
        } => array([text("stack_item"), u(u64::from(*stack_position_from_top))]),
    }
}

fn decision_cost_facts_value(value: &DecisionCostFactsV1) -> Value {
    array([
        match &value.selected_route {
            None => Value::Null,
            Some(mtgml_decision::CostRouteV1::Normal) => array([text("normal")]),
            Some(mtgml_decision::CostRouteV1::Alternative { route_id }) => {
                array([text("alternative"), u(u64::from(*route_id))])
            }
        },
        array(
            value
                .paid_additional_cost_ids
                .iter()
                .map(|id| u(u64::from(*id))),
        ),
    ])
}

fn safe_zone_name(value: SafeZoneKindV1) -> &'static str {
    match value {
        SafeZoneKindV1::Library => "library",
        SafeZoneKindV1::Hand => "hand",
        SafeZoneKindV1::Battlefield => "battlefield",
        SafeZoneKindV1::Graveyard => "graveyard",
        SafeZoneKindV1::Exile => "exile",
        SafeZoneKindV1::Stack => "stack",
        SafeZoneKindV1::Command => "command",
        SafeZoneKindV1::Ante => "ante",
        SafeZoneKindV1::Outside => "outside",
    }
}

#[cfg(test)]
mod g0_validation_tests {
    use super::*;

    #[test]
    fn v7_request_decoder_accepts_every_typed_counter_kind() {
        for kind in ["plus_one_plus_one", "minus_one_minus_one", "lore"] {
            let request = array([
                u(1),
                u(1),
                u(1),
                u(0),
                u(1),
                text("acting_player_only"),
                array([text("choose_one")]),
                array([
                    text("cost_operand_selection"),
                    u(0),
                    text("put_counters"),
                    text(kind),
                    u(2),
                ]),
                Value::Null,
                Value::Null,
                array([]),
            ]);
            assert!(validate_decision_request(&request).is_ok(), "{kind}");
        }
    }

    #[test]
    fn v7_stack_payload_rejects_zero_originating_trigger_identity() {
        let payload = array([
            text("triggered_ability"),
            u(0),
            Value::Null,
            Value::Null,
            array([]),
        ]);
        assert!(validate_stack_payload_value(&payload).is_err());
    }

    #[test]
    fn v7_mana_source_binding_uses_u32_ability_key() {
        let binding = |ability_key| {
            array([
                text("select_mana_source"),
                u(1),
                u(2),
                u(ability_key),
                text("test/mana-source@1.0.0"),
                array([text("tap_source")]),
                array((0..12).map(|_| u(0))),
            ])
        };
        assert!(validate_candidate_binding(&binding(u64::from(u32::MAX))).is_ok());
        assert!(validate_candidate_binding(&binding(u64::from(u32::MAX) + 1)).is_err());
    }

    #[test]
    fn v7_activation_continuation_rejects_unknown_stage_and_short_mana_buckets() {
        let source = array([
            u(1),
            u(1),
            u(1),
            text("test/mana-source@1.0.0"),
            array([text("tap_source")]),
            array((0..12).map(|_| u(0))),
        ]);
        let continuation = || {
            array([
                text("nonmana_activation"),
                u(1),
                u(1),
                u(1),
                u(1),
                text("test/ability@1.0.0"),
                text("paying_mana"),
                array([]),
                array([]),
                array([Value::Null, array([]), array([])]),
                array([
                    text("mana_payment_staging"),
                    text("selecting_sources"),
                    array([source.clone()]),
                ]),
            ])
        };

        let valid = continuation();
        assert!(validate_continuation_payload(&valid).is_ok());

        let mut unknown_stage = continuation();
        let Value::Array(fields) = &mut unknown_stage else {
            unreachable!()
        };
        fields[6] = text("unknown_stage");
        assert!(validate_continuation_payload(&unknown_stage).is_err());

        let mut short_buckets = continuation();
        let Value::Array(fields) = &mut short_buckets else {
            unreachable!()
        };
        let Value::Array(staging) = &mut fields[10] else {
            unreachable!()
        };
        let Value::Array(sources) = &mut staging[2] else {
            unreachable!()
        };
        let Value::Array(source) = &mut sources[0] else {
            unreachable!()
        };
        let Value::Array(buckets) = &mut source[5] else {
            unreachable!()
        };
        buckets.pop();
        assert!(validate_continuation_payload(&short_buckets).is_err());
    }
}
