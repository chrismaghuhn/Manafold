use std::collections::BTreeMap;

use mtgml_model::{EffectInstanceId, TriggerInstanceId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectRecord {
    pub id: EffectInstanceId,
    pub label: String,
}

/// The execution records of the state: the pending request, continuations,
/// temporary effects and waiting triggers.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExecutionState {
    pub pending_decision: Option<mtgml_decision::AuthoritativeDecisionRequest>,
    pub continuations: BTreeMap<mtgml_model::ContinuationId, crate::ContinuationRecord>,
    pub effects: BTreeMap<EffectInstanceId, crate::TemporaryEffectRecord>,
    pub waiting_triggers: BTreeMap<TriggerInstanceId, crate::PendingTriggerRecord>,
    /// Retained as a closed empty compatibility slot. Shared G0 has no
    /// delayed-effect witness and does not admit non-empty records here.
    pub delayed_effects: BTreeMap<EffectInstanceId, EffectRecord>,
}
