use crate::canonical_json::encode_canonical;
use crate::contract::WireContract;
use crate::error::WireError;
use mtgml_model::InformationStateDigest;
use mtgml_observation::{
    InformationStateDigestInput, MagicBasicLandObservationV1, MagicSharedExecutionObservationV1,
    ObservationEnvelope, ObservedEventEnvelopeV4, PlayerInformationState, PlayerStepV4,
};

impl WireContract for ObservationEnvelope {
    fn validate_wire(&self) -> Result<(), WireError> {
        self.validate()
            .map_err(|error| WireError::new("semantic.observation", error.to_string()))
    }
}

impl WireContract for MagicBasicLandObservationV1 {
    fn validate_wire(&self) -> Result<(), WireError> {
        self.validate().map_err(|error| {
            WireError::new(
                "semantic.magic_basic_land_observation_v1",
                error.to_string(),
            )
        })
    }
}

impl WireContract for MagicSharedExecutionObservationV1 {
    fn validate_wire(&self) -> Result<(), WireError> {
        self.validate()
            .map_err(|error| WireError::new("semantic.observation", error.to_string()))
    }
}

impl WireContract for InformationStateDigestInput {
    fn validate_wire(&self) -> Result<(), WireError> {
        self.validate()
            .map_err(|error| WireError::new("semantic.information_state", error.to_string()))
    }
}

impl WireContract for PlayerInformationState {
    fn validate_wire(&self) -> Result<(), WireError> {
        self.validate()
            .map_err(|error| WireError::new("semantic.information_state", error.to_string()))?;
        verify_information_state_digest_v3(self)
    }
}

pub fn compute_information_state_digest(
    input: &InformationStateDigestInput,
) -> Result<(Vec<u8>, InformationStateDigest), WireError> {
    let payload = encode_canonical(input)?;
    let digest = InformationStateDigest::from_canonical_bytes(&payload);
    Ok((payload, digest))
}

fn verify_information_state_digest_v3(state: &PlayerInformationState) -> Result<(), WireError> {
    let (_, expected) = compute_information_state_digest(&state.digest_input())?;
    if expected == state.digest {
        Ok(())
    } else {
        Err(WireError::new(
            "semantic.information_state",
            "information-state digest does not match its semantic payload",
        ))
    }
}

impl WireContract for ObservedEventEnvelopeV4 {
    fn validate_wire(&self) -> Result<(), WireError> {
        self.validate()
            .map_err(|error| WireError::new("semantic.observed_event", error.to_string()))
    }
}

impl WireContract for PlayerStepV4 {
    fn validate_wire(&self) -> Result<(), WireError> {
        self.validate()
            .map_err(|error| WireError::new("semantic.player_step", error.to_string()))?;
        verify_information_state_digest_v3(&self.information_state)
    }
}
