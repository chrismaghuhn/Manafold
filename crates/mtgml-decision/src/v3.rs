//! The player's decision response: the answer to one visible request.

use crate::error::DecisionValidationError;
use crate::v2::DecisionAnswerV2;
use mtgml_model::{PlayerDecisionIdV1, VisibleSequence};
use serde::{Deserialize, Serialize};

pub const DECISION_RESPONSE_V3_SCHEMA: &str = "decision-response.v3";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionResponseV3 {
    pub schema_version: String,
    pub player_decision_id: PlayerDecisionIdV1,
    pub view_sequence: VisibleSequence,
    pub answer: DecisionAnswerV2,
}

impl DecisionResponseV3 {
    pub fn validate(&self) -> Result<(), DecisionValidationError> {
        if self.schema_version != DECISION_RESPONSE_V3_SCHEMA {
            return Err(DecisionValidationError::SchemaVersion);
        }
        self.answer.validate_shape()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mtgml_model::{CandidateIdV1, PlayerDecisionIdV1, VisibleSequence};

    #[test]
    fn response_v3_uses_the_shared_decision_answer_and_visible_cursor() {
        let response = DecisionResponseV3 {
            schema_version: DECISION_RESPONSE_V3_SCHEMA.to_owned(),
            player_decision_id: PlayerDecisionIdV1(u64::MAX),
            view_sequence: VisibleSequence(0),
            answer: DecisionAnswerV2::SelectOne {
                candidate_id: CandidateIdV1(2),
            },
        };
        response.validate().unwrap();
        let bytes = serde_json::to_vec(&response).unwrap();
        assert_eq!(
            bytes,
            br#"{"schema_version":"decision-response.v3","player_decision_id":"18446744073709551615","view_sequence":"0","answer":{"kind":"select_one","candidate_id":2}}"#
        );
        let decoded: DecisionResponseV3 = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(decoded, response);
        assert!(serde_json::from_slice::<DecisionResponseV3>(
            br#"{"schema_version":"decision-response.v3","player_decision_id":"1","view_sequence":"2","state_revision":"3","answer":{"kind":"select_one","candidate_id":0}}"#
        )
        .is_err());
    }

    #[test]
    fn response_v3_rejects_noncanonical_select_many() {
        let response = DecisionResponseV3 {
            schema_version: DECISION_RESPONSE_V3_SCHEMA.to_owned(),
            player_decision_id: PlayerDecisionIdV1(1),
            view_sequence: VisibleSequence(2),
            answer: DecisionAnswerV2::SelectMany {
                candidate_ids: vec![CandidateIdV1(2), CandidateIdV1(1)],
            },
        };
        assert_eq!(
            response.validate(),
            Err(DecisionValidationError::NoncanonicalAnswer)
        );
    }
}
