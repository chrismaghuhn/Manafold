//! The player observation envelope: one perspective's canonical observation
//! payload at one visible sequence, bound by its digest.

use base64::{engine::general_purpose::STANDARD, Engine as _};
use mtgml_model::{ObservationDigest, PlayerId, VisibleSequence};
use serde::{Deserialize, Serialize};

use crate::error::ObservationValidationError;

pub const OBSERVATION_SCHEMA_V2: &str = "observation-envelope.v2";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationEnvelope {
    pub schema_version: String,
    pub perspective: PlayerId,
    pub view_sequence: VisibleSequence,
    pub payload_codec: String,
    pub payload_base64: String,
    pub digest: ObservationDigest,
}

impl ObservationEnvelope {
    pub fn validate(&self) -> Result<(), ObservationValidationError> {
        if self.schema_version != OBSERVATION_SCHEMA_V2 || self.payload_codec.is_empty() {
            return Err(ObservationValidationError::SchemaOrCodec);
        }
        let decoded = STANDARD
            .decode(&self.payload_base64)
            .map_err(|_| ObservationValidationError::Base64)?;
        if STANDARD.encode(&decoded) != self.payload_base64 {
            return Err(ObservationValidationError::Base64);
        }
        if ObservationDigest::from_canonical_bytes(&decoded) != self.digest {
            return Err(ObservationValidationError::DigestMismatch);
        }
        Ok(())
    }
}
