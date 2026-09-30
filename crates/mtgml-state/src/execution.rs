use std::collections::BTreeMap;

use mtgml_decision::AuthoritativeDecisionRequestV3;
use mtgml_model::{EffectInstanceId, PlayerId, TriggerInstanceId};
use serde::{Deserialize, Serialize};

use crate::engine_state_shape::{ContinuationRecordV2, PendingDecisionRecordV2};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectRecord {
    pub id: EffectInstanceId,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TriggerRecord {
    pub id: TriggerInstanceId,
    pub controller: PlayerId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ExecutionState {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_decision: Option<PendingDecisionRecordV2>,
    pub continuations: BTreeMap<mtgml_model::ContinuationId, ContinuationRecordV2>,
    pub effects: BTreeMap<EffectInstanceId, EffectRecord>,
    pub waiting_triggers: BTreeMap<TriggerInstanceId, TriggerRecord>,
    pub delayed_effects: BTreeMap<EffectInstanceId, EffectRecord>,
}

/// Typed successor execution records. There is one V3 pending request and no
/// V2 duplicate; this is the producer for the frozen PersistedExecutionV3
/// digest value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ExecutionStateV3 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_decision: Option<AuthoritativeDecisionRequestV3>,
    pub continuations: BTreeMap<mtgml_model::ContinuationId, ContinuationRecordV2>,
    pub effects: BTreeMap<EffectInstanceId, EffectRecord>,
    pub waiting_triggers: BTreeMap<TriggerInstanceId, TriggerRecord>,
    pub delayed_effects: BTreeMap<EffectInstanceId, EffectRecord>,
}

impl ExecutionStateV3 {
    pub fn canonical_value(
        &self,
    ) -> Result<mtgml_persistence::cbor::Value, crate::StateDigestError> {
        crate::digest_v5::successor_execution_value_v3(self)
    }
}

/// Detached G0 execution owner. It is not connected to current producers;
/// G0j is the only writer-activation boundary. Legacy V3 remains unchanged.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExecutionStateV4 {
    pub pending_decision: Option<mtgml_decision::AuthoritativeDecisionRequestV4>,
    pub continuations: BTreeMap<mtgml_model::ContinuationId, crate::ContinuationRecordV3>,
    pub effects: BTreeMap<EffectInstanceId, crate::TemporaryEffectRecord>,
    pub waiting_triggers: BTreeMap<TriggerInstanceId, crate::PendingTriggerRecord>,
    /// Retained as a closed empty compatibility slot. Shared G0 has no
    /// delayed-effect witness and does not admit non-empty records here.
    pub delayed_effects: BTreeMap<EffectInstanceId, EffectRecord>,
}
