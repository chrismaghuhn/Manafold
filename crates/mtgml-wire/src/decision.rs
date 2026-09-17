use crate::canonical_json::decode_canonical_shape;
use crate::contract::WireContract;
use crate::error::WireError;
use mtgml_decision::{
    DecisionResponse, DecisionResponseV2, PlayerDecisionRequest, PlayerDecisionRequestV2,
};

impl WireContract for PlayerDecisionRequest {
    fn validate_wire(&self) -> Result<(), WireError> {
        self.validate()
            .map_err(|error| WireError::new("semantic.decision", error.to_string()))
    }
}

impl WireContract for DecisionResponse {
    fn validate_wire(&self) -> Result<(), WireError> {
        self.validate()
            .map_err(|error| WireError::new("semantic.decision_response", error.to_string()))
    }
}

impl WireContract for PlayerDecisionRequestV2 {
    fn validate_wire(&self) -> Result<(), WireError> {
        self.validate()
            .map_err(|error| WireError::new("semantic.decision", error.to_string()))
    }
}

impl WireContract for DecisionResponseV2 {
    fn validate_wire(&self) -> Result<(), WireError> {
        self.validate()
            .map_err(|error| WireError::new("semantic.decision_response", error.to_string()))
    }
}

/// Player-submission entry decode for `DecisionResponseV2`.
///
/// Layer A of the accepted boundary: malformed/noncanonical/wrong-schema
/// bytes are rejected here with the closed wire code, while response-local
/// semantics (variant/membership/uniqueness/canonical/bounds) deliberately
/// remain the typed endpoint's responsibility.
pub mod decision_response_v2 {
    use super::*;

    pub fn decode_submission(bytes: &[u8]) -> Result<DecisionResponseV2, WireError> {
        let response = decode_canonical_shape::<DecisionResponseV2>(bytes)?;
        if response.schema_version != mtgml_decision::DECISION_RESPONSE_V2_SCHEMA {
            return Err(WireError::new(
                "decode.unknown_schema",
                "unsupported decision-response schema version",
            ));
        }
        Ok(response)
    }
}
