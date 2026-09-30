//! The player information state: the current observation, the next visible
//! sequence and the retained knowledge of one perspective, bound by one digest.

use mtgml_model::{InformationStateDigest, OpaqueObjectId, PlayerId, VisibleSequence};
use serde::{Deserialize, Serialize};

use crate::error::ObservationValidationError;
use crate::knowledge::{provenance_sequence, PlayerKnownObjectV1};
use crate::observation::ObservationEnvelope;

pub const INFORMATION_STATE_SCHEMA_V3: &str = "information-state-envelope.v3";
pub const INFORMATION_STATE_DIGEST_INPUT_SCHEMA_V3: &str = "information-state-digest-input.v3";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InformationStateDigestInput {
    pub schema_version: String,
    pub perspective: PlayerId,
    pub current_observation: ObservationEnvelope,
    pub next_visible_sequence: VisibleSequence,
    pub retained_knowledge: Vec<PlayerKnownObjectV1>,
}

impl InformationStateDigestInput {
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
pub struct PlayerInformationState {
    pub schema_version: String,
    pub perspective: PlayerId,
    pub current_observation: ObservationEnvelope,
    pub next_visible_sequence: VisibleSequence,
    pub retained_knowledge: Vec<PlayerKnownObjectV1>,
    pub digest: InformationStateDigest,
}

impl PlayerInformationState {
    pub fn digest_input(&self) -> InformationStateDigestInput {
        InformationStateDigestInput {
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
        let observation: ObservationEnvelope = serde_json::from_str(OBSERVATION).unwrap();
        observation.validate().unwrap();
        assert_eq!(
            serde_json::to_value(&observation).unwrap(),
            serde_json::from_str::<Value>(OBSERVATION).unwrap()
        );

        let information: PlayerInformationState = serde_json::from_str(INFORMATION_STATE).unwrap();
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
        let mut information: PlayerInformationState =
            serde_json::from_str(INFORMATION_STATE).unwrap();
        information.next_visible_sequence = VisibleSequence(6);
        assert_eq!(
            information.validate(),
            Err(ObservationValidationError::VisibleSequence)
        );
    }
}
