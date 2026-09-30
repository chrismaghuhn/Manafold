use crate::canonical_json::encode_canonical;
use crate::contract::WireContract;
use crate::error::WireError;
use mtgml_model::InformationStateDigestV3;
use mtgml_observation::{
    InformationStateDigestInputV3, MagicBasicLandObservationV1, MagicSharedExecutionObservationV1,
    ObservationEnvelopeV2, ObservedEventEnvelopeV4, PlayerInformationStateV3, PlayerStepV4,
};

impl WireContract for ObservationEnvelopeV2 {
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

impl WireContract for InformationStateDigestInputV3 {
    fn validate_wire(&self) -> Result<(), WireError> {
        self.validate()
            .map_err(|error| WireError::new("semantic.information_state", error.to_string()))
    }
}

impl WireContract for PlayerInformationStateV3 {
    fn validate_wire(&self) -> Result<(), WireError> {
        self.validate()
            .map_err(|error| WireError::new("semantic.information_state", error.to_string()))?;
        verify_information_state_digest_v3(self)
    }
}

pub fn compute_information_state_digest_v3(
    input: &InformationStateDigestInputV3,
) -> Result<(Vec<u8>, InformationStateDigestV3), WireError> {
    let payload = encode_canonical(input)?;
    let digest = InformationStateDigestV3::from_canonical_bytes(&payload);
    Ok((payload, digest))
}

fn verify_information_state_digest_v3(state: &PlayerInformationStateV3) -> Result<(), WireError> {
    let (_, expected) = compute_information_state_digest_v3(&state.digest_input())?;
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
