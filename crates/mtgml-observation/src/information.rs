//! Ownership: information-state envelopes (V1) and the V2 player information
//! state including its existing validation/digest-input construction.

use mtgml_model::{
    InformationStateDigest, InformationStateDigestV2, PlayerId, StateRevision, VisibleSequence,
};
use serde::{Deserialize, Serialize};

use crate::error::ObservationValidationError;
use crate::knowledge::{provenance_sequence, PlayerKnownObjectV1};
use crate::observation::ObservationEnvelope;
use crate::{INFORMATION_STATE_SCHEMA, INFORMATION_STATE_SCHEMA_V2};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InformationStateEnvelope {
    pub schema_version: String,
    pub perspective: PlayerId,
    pub state_revision: StateRevision,
    pub current_observation: ObservationEnvelope,
    pub public_history_length: u64,
    pub private_history_length: u64,
    pub digest: InformationStateDigest,
}

impl InformationStateEnvelope {
    pub fn validate(&self) -> Result<(), ObservationValidationError> {
        if self.schema_version != INFORMATION_STATE_SCHEMA {
            return Err(ObservationValidationError::SchemaOrCodec);
        }
        self.current_observation.validate()?;
        if self.current_observation.perspective != self.perspective
            || self.current_observation.state_revision != self.state_revision
        {
            return Err(ObservationValidationError::InformationStateMismatch);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InformationStateDigestInputV2 {
    pub schema_version: String,
    pub perspective: PlayerId,
    pub state_revision: StateRevision,
    pub current_observation: ObservationEnvelope,
    pub next_visible_sequence: VisibleSequence,
    pub retained_knowledge: Vec<PlayerKnownObjectV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerInformationStateV2 {
    pub schema_version: String,
    pub perspective: PlayerId,
    pub state_revision: StateRevision,
    pub current_observation: ObservationEnvelope,
    pub next_visible_sequence: VisibleSequence,
    pub retained_knowledge: Vec<PlayerKnownObjectV1>,
    pub digest: InformationStateDigestV2,
}

impl PlayerInformationStateV2 {
    pub fn digest_input(&self) -> InformationStateDigestInputV2 {
        InformationStateDigestInputV2 {
            schema_version: "information-state-digest-input.v2".into(),
            perspective: self.perspective,
            state_revision: self.state_revision,
            current_observation: self.current_observation.clone(),
            next_visible_sequence: self.next_visible_sequence,
            retained_knowledge: self.retained_knowledge.clone(),
        }
    }

    pub fn validate(&self) -> Result<(), ObservationValidationError> {
        if self.schema_version != INFORMATION_STATE_SCHEMA_V2 {
            return Err(ObservationValidationError::SchemaOrCodec);
        }
        self.current_observation.validate()?;
        if self.current_observation.perspective != self.perspective
            || self.current_observation.state_revision != self.state_revision
        {
            return Err(ObservationValidationError::PerspectiveRevision);
        }
        let mut previous = None;
        for record in &self.retained_knowledge {
            let current = record.opaque_object_id();
            if current.0 == 0 || previous.is_some_and(|previous| previous >= current) {
                return Err(ObservationValidationError::RetainedKnowledge);
            }
            previous = Some(current);
            if !record.provenance_is_valid(self.next_visible_sequence) {
                return Err(ObservationValidationError::VisibleSequence);
            }
            // Only actually observed facts carry visible sequences; the
            // strictly-increasing rule compares observed sequences.
            let observed: Vec<_> = record
                .historical_locations()
                .iter()
                .filter_map(|fact| provenance_sequence(&fact.provenance))
                .collect();
            if observed.windows(2).any(|window| window[0] >= window[1]) {
                return Err(ObservationValidationError::VisibleSequence);
            }
        }
        Ok(())
    }
}
