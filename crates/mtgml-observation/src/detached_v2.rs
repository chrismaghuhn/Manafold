//! Detached G0 observation and information-state successor DTOs.
//!
//! These values have no environment producer yet. G0g owns public projection;
//! this module owns only the closed versioned value shapes and local checks.

use base64::{engine::general_purpose::STANDARD, Engine as _};
use mtgml_model::{
    InformationStateDigestV3, ObservationDigest, OpaqueObjectId, PlayerId, VisibleSequence,
};
use serde::{Deserialize, Serialize};

use crate::error::ObservationValidationError;
use crate::knowledge::{provenance_sequence, PlayerKnownObjectV1};

pub const OBSERVATION_SCHEMA_V2: &str = "observation-envelope.v2";
pub const INFORMATION_STATE_SCHEMA_V3: &str = "information-state-envelope.v3";
pub const INFORMATION_STATE_DIGEST_INPUT_SCHEMA_V3: &str = "information-state-digest-input.v3";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationEnvelopeV2 {
    pub schema_version: String,
    pub perspective: PlayerId,
    pub view_sequence: VisibleSequence,
    pub payload_codec: String,
    pub payload_base64: String,
    pub digest: ObservationDigest,
}

impl ObservationEnvelopeV2 {
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InformationStateDigestInputV3 {
    pub schema_version: String,
    pub perspective: PlayerId,
    pub current_observation: ObservationEnvelopeV2,
    pub next_visible_sequence: VisibleSequence,
    pub retained_knowledge: Vec<PlayerKnownObjectV1>,
}

impl InformationStateDigestInputV3 {
    pub fn validate(&self) -> Result<(), ObservationValidationError> {
        if self.schema_version != INFORMATION_STATE_DIGEST_INPUT_SCHEMA_V3 {
            return Err(ObservationValidationError::SchemaOrCodec);
        }
        self.current_observation.validate()?;
        if self.current_observation.perspective != self.perspective {
            return Err(ObservationValidationError::PerspectiveRevision);
        }
        if self.current_observation.view_sequence != self.next_visible_sequence {
            return Err(ObservationValidationError::VisibleSequence);
        }
        validate_retained_knowledge(&self.retained_knowledge, self.next_visible_sequence)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerInformationStateV3 {
    pub schema_version: String,
    pub perspective: PlayerId,
    pub current_observation: ObservationEnvelopeV2,
    pub next_visible_sequence: VisibleSequence,
    pub retained_knowledge: Vec<PlayerKnownObjectV1>,
    pub digest: InformationStateDigestV3,
}

impl PlayerInformationStateV3 {
    pub fn digest_input(&self) -> InformationStateDigestInputV3 {
        InformationStateDigestInputV3 {
            schema_version: INFORMATION_STATE_DIGEST_INPUT_SCHEMA_V3.into(),
            perspective: self.perspective,
            current_observation: self.current_observation.clone(),
            next_visible_sequence: self.next_visible_sequence,
            retained_knowledge: self.retained_knowledge.clone(),
        }
    }

    pub fn validate(&self) -> Result<(), ObservationValidationError> {
        if self.schema_version != INFORMATION_STATE_SCHEMA_V3 {
            return Err(ObservationValidationError::SchemaOrCodec);
        }
        self.digest_input().validate()
    }
}

fn validate_retained_knowledge(
    retained_knowledge: &[PlayerKnownObjectV1],
    next_visible_sequence: VisibleSequence,
) -> Result<(), ObservationValidationError> {
    let mut previous: Option<OpaqueObjectId> = None;
    for record in retained_knowledge {
        let current = record.opaque_object_id();
        if current.0 == 0 || previous.is_some_and(|before| before >= current) {
            return Err(ObservationValidationError::RetainedKnowledge);
        }
        previous = Some(current);
        if !record.provenance_is_valid(next_visible_sequence) {
            return Err(ObservationValidationError::VisibleSequence);
        }
        let observed: Vec<_> = record
            .historical_locations()
            .iter()
            .filter_map(|fact| provenance_sequence(&fact.provenance))
            .collect();
        if observed.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(ObservationValidationError::VisibleSequence);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const OBSERVATION: &str =
        include_str!("../../../schemas/examples/observation-envelope-v2.json");
    const INFORMATION_STATE: &str =
        include_str!("../../../schemas/examples/information-state-envelope-v3.json");

    #[test]
    fn rust_dtos_match_successor_observation_and_information_fixtures() {
        let observation: ObservationEnvelopeV2 = serde_json::from_str(OBSERVATION).unwrap();
        observation.validate().unwrap();
        assert_eq!(
            serde_json::to_value(&observation).unwrap(),
            serde_json::from_str::<Value>(OBSERVATION).unwrap()
        );

        let information: PlayerInformationStateV3 =
            serde_json::from_str(INFORMATION_STATE).unwrap();
        information.validate().unwrap();
        assert_eq!(
            serde_json::to_value(&information).unwrap(),
            serde_json::from_str::<Value>(INFORMATION_STATE).unwrap()
        );
        assert_eq!(
            information.digest_input().schema_version,
            INFORMATION_STATE_DIGEST_INPUT_SCHEMA_V3
        );
    }

    #[test]
    fn information_state_requires_observation_view_cursor_equality() {
        let mut information: PlayerInformationStateV3 =
            serde_json::from_str(INFORMATION_STATE).unwrap();
        information.next_visible_sequence = VisibleSequence(6);
        assert_eq!(
            information.validate(),
            Err(ObservationValidationError::VisibleSequence)
        );
    }
}
