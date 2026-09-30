use crate::canonical_json::decode_canonical_shape;
use crate::contract::WireContract;
use crate::error::WireError;
use mtgml_decision::{DecisionResponseV3, PlayerDecisionRequestV4};

impl WireContract for PlayerDecisionRequestV4 {
    fn validate_wire(&self) -> Result<(), WireError> {
        self.validate()
            .map_err(|error| WireError::new("semantic.decision", error.to_string()))
    }
}

impl WireContract for DecisionResponseV3 {
    fn validate_wire(&self) -> Result<(), WireError> {
        self.validate()
            .map_err(|error| WireError::new("semantic.decision_response", error.to_string()))
    }
}

/// Player-submission entry decode for the revision-free `DecisionResponseV3`.
/// Candidate membership and the perspective-local cursor are checked later by
/// the authoritative pending V4 request.
pub mod decision_response_v3 {
    use super::*;

    pub fn decode_submission(bytes: &[u8]) -> Result<DecisionResponseV3, WireError> {
        let response = decode_canonical_shape::<DecisionResponseV3>(bytes)?;
        if response.schema_version != mtgml_decision::DECISION_RESPONSE_V3_SCHEMA {
            return Err(WireError::new(
                "decode.unknown_schema",
                "unsupported decision-response schema version",
            ));
        }
        Ok(response)
    }
}

#[cfg(test)]
mod successor_response_tests {
    use super::decision_response_v3::decode_submission;
    use mtgml_decision::DecisionResponseV3;

    #[test]
    fn decision_response_v3_accepts_its_fixture_and_rejects_global_revision() {
        let response: DecisionResponseV3 = serde_json::from_str(include_str!(
            "../../../schemas/examples/decision-response-v3.json"
        ))
        .unwrap();
        let canonical = crate::encode_canonical(&response).unwrap();
        assert_eq!(decode_submission(&canonical).unwrap(), response);
        let negative_canonical = br#"{"answer":{"candidate_id":2,"kind":"select_one"},"player_decision_id":"18446744073709551615","schema_version":"decision-response.v3","state_revision":3,"view_sequence":"0"}"#;
        assert!(decode_submission(negative_canonical).is_err());
    }
}
