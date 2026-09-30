use crate::contract::WireContract;
use crate::error::WireError;
use mtgml_model::EpisodeStatus;
use mtgml_replay::{AuthoritativeReplayV8, ReplayManifestV8, ReplayStepV8};

impl WireContract for EpisodeStatus {
    fn validate_wire(&self) -> Result<(), WireError> {
        self.validate()
            .map_err(|error| WireError::new("semantic.episode_status", error.to_string()))
    }
}

impl WireContract for ReplayManifestV8 {
    fn validate_wire(&self) -> Result<(), WireError> {
        self.validate().map_err(|error| {
            let code = if error == mtgml_replay::ReplayValidationError::SchemaVersion {
                "decode.invalid_json"
            } else {
                "semantic.replay_manifest"
            };
            WireError::new(code, error.to_string())
        })
    }
}

impl WireContract for ReplayStepV8 {
    fn validate_wire(&self) -> Result<(), WireError> {
        self.validate()
            .map_err(|error| WireError::new("semantic.replay", error.to_string()))
    }
}

impl WireContract for AuthoritativeReplayV8 {
    fn validate_wire(&self) -> Result<(), WireError> {
        self.validate().map_err(|error| {
            let code = if error == mtgml_replay::ReplayValidationError::SchemaVersion {
                "decode.invalid_json"
            } else {
                "semantic.replay"
            };
            WireError::new(code, error.to_string())
        })
    }
}
