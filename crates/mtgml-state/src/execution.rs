use std::collections::BTreeMap;

use mtgml_model::{EffectInstanceId, PlayerId, TriggerInstanceId};
use serde::{Deserialize, Serialize};

use crate::m2_shape::{ContinuationRecordV2, PendingDecisionRecordV2};

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
