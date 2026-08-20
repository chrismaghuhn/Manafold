use std::collections::BTreeMap;

use mtgml_decision::{EngineCandidateBinding, PlayerDecisionRequest};
use mtgml_model::{ContinuationId, EffectInstanceId, PlayerId, TriggerInstanceId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PendingDecisionRecord {
    pub request: PlayerDecisionRequest,
    pub candidate_bindings: BTreeMap<String, EngineCandidateBinding>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continuation: Option<ContinuationId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContinuationRecord {
    pub id: ContinuationId,
    pub label: String,
}

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
    pub pending_decision: Option<PendingDecisionRecord>,
    pub continuations: BTreeMap<ContinuationId, ContinuationRecord>,
    pub effects: BTreeMap<EffectInstanceId, EffectRecord>,
    pub waiting_triggers: BTreeMap<TriggerInstanceId, TriggerRecord>,
    pub delayed_effects: BTreeMap<EffectInstanceId, EffectRecord>,
}
