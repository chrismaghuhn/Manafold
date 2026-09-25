use crate::canonical_json::encode_canonical;
use crate::contract::WireContract;
use crate::error::WireError;
use mtgml_observation::{
    InformationStateDigestInputV2, InformationStateEnvelope, MagicObservation, MagicObservationV2,
    MagicObservationV3, MagicObservationV4, ObservationEnvelope, ObservedEventEnvelope,
    ObservedEventEnvelopeV2, PlayerInformationStateV2, PlayerStep, PlayerStepV2,
    SyntheticObservation,
};

impl WireContract for ObservationEnvelope {
    fn validate_wire(&self) -> Result<(), WireError> {
        self.validate()
            .map_err(|error| WireError::new("semantic.observation", error.to_string()))
    }
}

impl WireContract for SyntheticObservation {
    fn validate_wire(&self) -> Result<(), WireError> {
        self.validate()
            .map_err(|error| WireError::new("semantic.synthetic_m3_observation", error.to_string()))
    }
}

impl WireContract for MagicObservation {
    fn validate_wire(&self) -> Result<(), WireError> {
        self.validate()
            .map_err(|error| WireError::new("semantic.magic_m3_observation", error.to_string()))
    }
}

impl WireContract for MagicObservationV2 {
    fn validate_wire(&self) -> Result<(), WireError> {
        self.validate()
            .map_err(|error| WireError::new("semantic.magic_combat_observation", error.to_string()))
    }
}

impl WireContract for MagicObservationV3 {
    fn validate_wire(&self) -> Result<(), WireError> {
        self.validate()
            .map_err(|error| WireError::new("semantic.magic_combat_observation", error.to_string()))
    }
}

impl WireContract for MagicObservationV4 {
    fn validate_wire(&self) -> Result<(), WireError> {
        self.validate().map_err(|error| {
            WireError::new("semantic.magic_combat_observation_v4", error.to_string())
        })
    }
}

impl WireContract for InformationStateEnvelope {
    fn validate_wire(&self) -> Result<(), WireError> {
        self.validate()
            .map_err(|error| WireError::new("semantic.information_state", error.to_string()))
    }
}

impl WireContract for InformationStateDigestInputV2 {
    fn validate_wire(&self) -> Result<(), WireError> {
        if self.schema_version != "information-state-digest-input.v2" {
            return Err(WireError::new(
                "semantic.information_state",
                "unsupported information-state digest input schema",
            ));
        }
        self.current_observation
            .validate()
            .map_err(|error| WireError::new("semantic.information_state", error.to_string()))?;
        let public = PlayerInformationStateV2 {
            schema_version: "information-state-envelope.v2".into(),
            perspective: self.perspective,
            state_revision: self.state_revision,
            current_observation: self.current_observation.clone(),
            next_visible_sequence: self.next_visible_sequence,
            retained_knowledge: self.retained_knowledge.clone(),
            digest: mtgml_model::InformationStateDigestV2::from_canonical_bytes(b"wire-validation"),
        };
        public
            .validate()
            .map_err(|error| WireError::new("semantic.information_state", error.to_string()))
    }
}

impl WireContract for PlayerInformationStateV2 {
    fn validate_wire(&self) -> Result<(), WireError> {
        self.validate()
            .map_err(|error| WireError::new("semantic.information_state", error.to_string()))?;
        verify_information_state_digest_v2(self)
    }
}

/// The persisted `InformationStateDigestV2` is the canonical identity of the
/// player-safe semantic payload; forged digest values must never validate.
fn verify_information_state_digest_v2(state: &PlayerInformationStateV2) -> Result<(), WireError> {
    let (_, expected) = compute_information_state_digest_v2(&state.digest_input())?;
    if expected == state.digest {
        Ok(())
    } else {
        Err(WireError::new(
            "semantic.information_state",
            "information-state digest does not match its semantic payload",
        ))
    }
}

impl WireContract for ObservedEventEnvelopeV2 {
    fn validate_wire(&self) -> Result<(), WireError> {
        self.validate()
            .map_err(|error| WireError::new("semantic.observed_event", error.to_string()))
    }
}

impl WireContract for PlayerStepV2 {
    fn validate_wire(&self) -> Result<(), WireError> {
        self.validate()
            .map_err(|error| WireError::new("semantic.player_step", error.to_string()))?;
        verify_information_state_digest_v2(&self.information_state)
    }
}

impl WireContract for ObservedEventEnvelope {
    fn validate_wire(&self) -> Result<(), WireError> {
        self.validate()
            .map_err(|error| WireError::new("semantic.observed_event", error.to_string()))
    }
}

impl WireContract for PlayerStep {
    fn validate_wire(&self) -> Result<(), WireError> {
        self.validate()
            .map_err(|error| WireError::new("semantic.player_step", error.to_string()))
    }
}

pub fn compute_information_state_digest_v2(
    input: &InformationStateDigestInputV2,
) -> Result<(Vec<u8>, mtgml_model::InformationStateDigestV2), WireError> {
    let bytes = encode_canonical(input)?;
    let digest = mtgml_model::InformationStateDigestV2::from_canonical_bytes(&bytes);
    Ok((bytes, digest))
}
