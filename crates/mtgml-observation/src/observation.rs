//! Ownership: `ObservationEnvelope` and its existing validation.

use base64::{engine::general_purpose::STANDARD, Engine as _};
use mtgml_model::{ObservationDigest, PlayerId, StateRevision};
use serde::{Deserialize, Serialize};

use crate::error::ObservationValidationError;
use crate::OBSERVATION_SCHEMA;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationEnvelope {
    pub schema_version: String,
    pub perspective: PlayerId,
    pub state_revision: StateRevision,
    pub payload_codec: String,
    pub payload_base64: String,
    pub digest: ObservationDigest,
}

impl ObservationEnvelope {
    pub fn validate(&self) -> Result<(), ObservationValidationError> {
        if self.schema_version != OBSERVATION_SCHEMA || self.payload_codec.is_empty() {
            return Err(ObservationValidationError::SchemaOrCodec);
        }
        let decoded = STANDARD
            .decode(&self.payload_base64)
            .map_err(|_| ObservationValidationError::Base64)?;
        if STANDARD.encode(decoded) != self.payload_base64 {
            return Err(ObservationValidationError::Base64);
        }
        Ok(())
    }
}
