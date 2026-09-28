use std::collections::BTreeMap;

use mtgml_decision::{AuthoritativeDecisionRequestV3, DecisionAnswerV2, DecisionResponseV2};
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

    /// Resolves a validated response only through the current authoritative
    /// request. The returned bindings are trusted inputs for the RulesKernel;
    /// this method performs no rules action or state mutation.
    pub fn selected_bindings(
        &self,
        actor: PlayerId,
        revision: mtgml_model::StateRevision,
        response: &DecisionResponseV2,
    ) -> Result<Vec<&mtgml_decision::EngineCandidateBindingV3>, SuccessorDecisionError> {
        let request = self
            .pending_decision
            .as_ref()
            .ok_or(SuccessorDecisionError::NoPendingRequest)?;
        if request.actor != actor || request.state_revision != revision {
            return Err(SuccessorDecisionError::RequestMismatch);
        }
        request
            .project_player_request()
            .and_then(|visible| visible.validate_response(response))
            .map_err(|_| SuccessorDecisionError::InvalidResponse)?;
        let selected: Vec<_> = match &response.answer {
            DecisionAnswerV2::SelectOne { candidate_id } => vec![*candidate_id],
            DecisionAnswerV2::SelectMany { candidate_ids }
            | DecisionAnswerV2::Order { candidate_ids } => candidate_ids.clone(),
            DecisionAnswerV2::ChooseNumber { .. } => Vec::new(),
        };
        selected
            .into_iter()
            .map(|id| {
                request
                    .candidates
                    .iter()
                    .find(|candidate| candidate.candidate_id == id)
                    .map(|candidate| &candidate.trusted_binding)
                    .ok_or(SuccessorDecisionError::InvalidResponse)
            })
            .collect()
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SuccessorDecisionError {
    #[error("successor state has no pending authoritative request")]
    NoPendingRequest,
    #[error("response actor or revision does not match the pending request")]
    RequestMismatch,
    #[error("response is invalid for the pending V3 request")]
    InvalidResponse,
}

#[cfg(test)]
mod tests {
    use super::*;
    use mtgml_decision::{
        AuthoritativeCandidateV3, CandidateIntentV3, DecisionDomainV2, DecisionVisibility,
        EngineCandidateBindingV3, DECISION_RESPONSE_V2_SCHEMA,
    };
    use mtgml_model::{
        CandidateIdV1, DecisionId, GameObjectId, OpaqueObjectId, PlayerDecisionIdV1, StateRevision,
    };

    fn execution() -> ExecutionStateV3 {
        ExecutionStateV3 {
            pending_decision: Some(AuthoritativeDecisionRequestV3 {
                decision_id: DecisionId(4),
                player_decision_id: PlayerDecisionIdV1(5),
                state_revision: StateRevision(6),
                actor: PlayerId(1),
                visibility: DecisionVisibility::Public,
                decision: DecisionDomainV2::ChooseOne,
                candidates: vec![AuthoritativeCandidateV3 {
                    candidate_id: CandidateIdV1(0),
                    visible_intent: CandidateIntentV3::PlayLand {
                        object: OpaqueObjectId(2),
                    },
                    trusted_binding: EngineCandidateBindingV3::PlayLand {
                        object: GameObjectId(9),
                    },
                }],
                continuation_id: None,
            }),
            ..ExecutionStateV3::default()
        }
    }

    fn response(candidate_id: u32, revision: u64) -> DecisionResponseV2 {
        DecisionResponseV2 {
            schema_version: DECISION_RESPONSE_V2_SCHEMA.to_owned(),
            player_decision_id: PlayerDecisionIdV1(5),
            state_revision: StateRevision(revision),
            answer: DecisionAnswerV2::SelectOne {
                candidate_id: CandidateIdV1(candidate_id),
            },
        }
    }

    #[test]
    fn response_resolves_only_the_exact_pending_v3_trusted_binding() {
        let state = execution();
        assert_eq!(
            state.selected_bindings(PlayerId(1), StateRevision(6), &response(0, 6)),
            Ok(vec![&EngineCandidateBindingV3::PlayLand {
                object: GameObjectId(9)
            }])
        );
        assert_eq!(
            state.selected_bindings(PlayerId(2), StateRevision(6), &response(0, 6)),
            Err(SuccessorDecisionError::RequestMismatch)
        );
        assert_eq!(
            state.selected_bindings(PlayerId(1), StateRevision(7), &response(0, 6)),
            Err(SuccessorDecisionError::RequestMismatch)
        );
        assert_eq!(
            state.selected_bindings(PlayerId(1), StateRevision(6), &response(1, 6)),
            Err(SuccessorDecisionError::InvalidResponse)
        );
    }
}
