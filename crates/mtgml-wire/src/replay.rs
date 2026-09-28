use crate::contract::WireContract;
use crate::error::WireError;
use mtgml_model::EpisodeStatus;
use mtgml_replay::{
    AuthoritativeReplayV1, AuthoritativeReplayV2, AuthoritativeReplayV3, AuthoritativeReplayV4,
    AuthoritativeReplayV5, AuthoritativeReplayV6, AuthoritativeReplayV7, AuthoritativeReplayV8,
    ReplayManifestV1, ReplayManifestV2, ReplayManifestV3, ReplayManifestV4, ReplayManifestV5,
    ReplayManifestV6, ReplayManifestV7, ReplayManifestV8, ReplayStepV8,
};

impl WireContract for EpisodeStatus {
    fn validate_wire(&self) -> Result<(), WireError> {
        self.validate()
            .map_err(|error| WireError::new("semantic.episode_status", error.to_string()))
    }
}

impl WireContract for ReplayManifestV1 {
    fn validate_wire(&self) -> Result<(), WireError> {
        self.validate()
            .map_err(|error| WireError::new("semantic.replay_manifest", error.to_string()))
    }
}

impl WireContract for AuthoritativeReplayV1 {
    fn validate_wire(&self) -> Result<(), WireError> {
        self.validate()
            .map_err(|error| WireError::new("semantic.replay", error.to_string()))
    }
}

impl WireContract for ReplayManifestV2 {
    fn validate_wire(&self) -> Result<(), WireError> {
        self.validate()
            .map_err(|error| WireError::new("semantic.replay_manifest", error.to_string()))
    }
}

impl WireContract for AuthoritativeReplayV2 {
    fn validate_wire(&self) -> Result<(), WireError> {
        self.validate()
            .map_err(|error| WireError::new("semantic.replay", error.to_string()))
    }
}

impl WireContract for ReplayManifestV3 {
    fn validate_wire(&self) -> Result<(), WireError> {
        self.validate()
            .map_err(|error| WireError::new("semantic.replay_manifest", error.to_string()))
    }
}

impl WireContract for AuthoritativeReplayV3 {
    fn validate_wire(&self) -> Result<(), WireError> {
        self.validate()
            .map_err(|error| WireError::new("semantic.replay", error.to_string()))
    }
}

impl WireContract for ReplayManifestV4 {
    fn validate_wire(&self) -> Result<(), WireError> {
        self.validate()
            .map_err(|error| WireError::new("semantic.replay_manifest", error.to_string()))
    }
}

impl WireContract for AuthoritativeReplayV4 {
    fn validate_wire(&self) -> Result<(), WireError> {
        self.validate()
            .map_err(|error| WireError::new("semantic.replay", error.to_string()))
    }
}

impl WireContract for ReplayManifestV5 {
    fn validate_wire(&self) -> Result<(), WireError> {
        self.validate()
            .map_err(|error| WireError::new("semantic.replay_manifest", error.to_string()))
    }
}

impl WireContract for AuthoritativeReplayV5 {
    fn validate_wire(&self) -> Result<(), WireError> {
        self.validate()
            .map_err(|error| WireError::new("semantic.replay", error.to_string()))
    }
}

impl WireContract for ReplayManifestV6 {
    fn validate_wire(&self) -> Result<(), WireError> {
        self.validate()
            .map_err(|error| WireError::new("semantic.replay_manifest", error.to_string()))
    }
}

impl WireContract for AuthoritativeReplayV6 {
    fn validate_wire(&self) -> Result<(), WireError> {
        self.validate()
            .map_err(|error| WireError::new("semantic.replay", error.to_string()))
    }
}

impl WireContract for ReplayManifestV7 {
    fn validate_wire(&self) -> Result<(), WireError> {
        self.validate()
            .map_err(|error| WireError::new("semantic.replay_manifest", error.to_string()))
    }
}

impl WireContract for AuthoritativeReplayV7 {
    fn validate_wire(&self) -> Result<(), WireError> {
        self.validate()
            .map_err(|error| WireError::new("semantic.replay", error.to_string()))
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
