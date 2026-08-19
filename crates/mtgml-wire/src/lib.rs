//! Canonical JSON writer/reader for every public v1 wire contract.
//!
//! JSON Schema validates shape. This crate additionally enforces closed Rust
//! variants, integer ranges, canonical bytes, and cross-field semantics.

use mtgml_decision::{DecisionResponse, PlayerDecisionRequest};
use mtgml_model::EpisodeStatus;
use mtgml_observation::{
    InformationStateEnvelope, ObservationEnvelope, ObservedEventEnvelope, PlayerStep,
};
use mtgml_replay::{AuthoritativeReplayV1, AuthoritativeReplayV2, ReplayManifestV1, ReplayManifestV2};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{Map, Value};
use std::{fmt, fs, path::Path};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireError {
    pub code: &'static str,
    pub message: String,
}

impl WireError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl fmt::Display for WireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for WireError {}

pub trait WireContract {
    fn validate_wire(&self) -> Result<(), WireError>;
}

impl WireContract for PlayerDecisionRequest {
    fn validate_wire(&self) -> Result<(), WireError> {
        self.validate()
            .map_err(|error| WireError::new("semantic.decision", error.to_string()))
    }
}

impl WireContract for DecisionResponse {
    fn validate_wire(&self) -> Result<(), WireError> {
        self.validate()
            .map_err(|error| WireError::new("semantic.decision_response", error.to_string()))
    }
}

impl WireContract for ObservationEnvelope {
    fn validate_wire(&self) -> Result<(), WireError> {
        self.validate()
            .map_err(|error| WireError::new("semantic.observation", error.to_string()))
    }
}

impl WireContract for InformationStateEnvelope {
    fn validate_wire(&self) -> Result<(), WireError> {
        self.validate()
            .map_err(|error| WireError::new("semantic.information_state", error.to_string()))
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

pub fn encode_canonical<T>(value: &T) -> Result<Vec<u8>, WireError>
where
    T: Serialize + WireContract,
{
    value.validate_wire()?;
    let value = serde_json::to_value(value)
        .map_err(|error| WireError::new("encode.serialization", error.to_string()))?;
    serde_json::to_vec(&canonicalize(value))
        .map_err(|error| WireError::new("encode.serialization", error.to_string()))
}

pub fn decode_canonical<T>(bytes: &[u8]) -> Result<T, WireError>
where
    T: DeserializeOwned + Serialize + WireContract,
{
    let value: T = serde_json::from_slice(bytes)
        .map_err(|error| WireError::new("decode.invalid_json", error.to_string()))?;
    value.validate_wire()?;
    let canonical = encode_canonical(&value)?;
    if canonical != bytes {
        return Err(WireError::new(
            "decode.non_canonical_json",
            "wire bytes are valid JSON but not the canonical representation",
        ));
    }
    Ok(value)
}

fn canonicalize(value: Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.into_iter().map(canonicalize).collect()),
        Value::Object(object) => {
            let mut pairs: Vec<_> = object.into_iter().collect();
            pairs.sort_by(|left, right| left.0.cmp(&right.0));
            let mut sorted = Map::new();
            for (key, value) in pairs {
                sorted.insert(key, canonicalize(value));
            }
            Value::Object(sorted)
        }
        scalar => scalar,
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureManifest {
    fixtures: Vec<FixtureCase>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureCase {
    path: String,
    contract: String,
    #[serde(default)]
    expected_error_code: Option<String>,
    #[serde(default)]
    expected_reject_layer: Option<String>,
}

const SUPPORTED_REJECT_LAYER: &str = "rust-python-semantic-or-decode";

pub fn verify_golden_fixture_directory(root: &Path) -> Result<(), FixtureVerificationError> {
    let manifest: FixtureManifest = serde_json::from_slice(
        &fs::read(root.join("manifest.json")).map_err(FixtureVerificationError::Io)?,
    )
    .map_err(FixtureVerificationError::Manifest)?;
    for fixture in manifest.fixtures {
        if fixture.expected_error_code.is_some() || fixture.expected_reject_layer.is_some() {
            return Err(FixtureVerificationError::UnexpectedExpectation(
                fixture.path,
            ));
        }
        let bytes = fs::read(root.join(&fixture.path)).map_err(FixtureVerificationError::Io)?;
        decode_named(&fixture.contract, &bytes).map_err(|error| {
            FixtureVerificationError::UnexpectedRejection {
                path: fixture.path,
                error,
            }
        })?;
    }
    Ok(())
}

pub fn verify_negative_fixture_directory(root: &Path) -> Result<(), FixtureVerificationError> {
    let manifest: FixtureManifest = serde_json::from_slice(
        &fs::read(root.join("manifest.json")).map_err(FixtureVerificationError::Io)?,
    )
    .map_err(FixtureVerificationError::Manifest)?;
    for fixture in manifest.fixtures {
        let expected = fixture
            .expected_error_code
            .ok_or_else(|| FixtureVerificationError::MissingExpectation(fixture.path.clone()))?;
        let reject_layer = fixture
            .expected_reject_layer
            .ok_or_else(|| FixtureVerificationError::MissingRejectLayer(fixture.path.clone()))?;
        if reject_layer != SUPPORTED_REJECT_LAYER {
            return Err(FixtureVerificationError::UnsupportedRejectLayer {
                path: fixture.path,
                layer: reject_layer,
            });
        }
        let bytes = fs::read(root.join(&fixture.path)).map_err(FixtureVerificationError::Io)?;
        match decode_named(&fixture.contract, &bytes) {
            Ok(()) => return Err(FixtureVerificationError::UnexpectedAcceptance(fixture.path)),
            Err(error) if error.code == expected.as_str() => {}
            Err(error) => {
                return Err(FixtureVerificationError::WrongError {
                    path: fixture.path,
                    expected,
                    actual: error.code.to_owned(),
                })
            }
        }
    }
    Ok(())
}

fn decode_named(contract: &str, bytes: &[u8]) -> Result<(), WireError> {
    match contract {
        "player-decision-request.v1" => decode_canonical::<PlayerDecisionRequest>(bytes).map(drop),
        "decision-response.v1" => decode_canonical::<DecisionResponse>(bytes).map(drop),
        "observation-envelope.v1" => decode_canonical::<ObservationEnvelope>(bytes).map(drop),
        "information-state-envelope.v1" => {
            decode_canonical::<InformationStateEnvelope>(bytes).map(drop)
        }
        "observed-event-envelope.v1" => decode_canonical::<ObservedEventEnvelope>(bytes).map(drop),
        "player-step.v1" => decode_canonical::<PlayerStep>(bytes).map(drop),
        "episode-status.v1" => decode_canonical::<EpisodeStatus>(bytes).map(drop),
        "replay-manifest.v1" => decode_canonical::<ReplayManifestV1>(bytes).map(drop),
        "authoritative-replay.v1" => decode_canonical::<AuthoritativeReplayV1>(bytes).map(drop),
        "replay-manifest.v2" => decode_canonical::<ReplayManifestV2>(bytes).map(drop),
        "authoritative-replay.v2" => decode_canonical::<AuthoritativeReplayV2>(bytes).map(drop),
        _ => Err(WireError::new(
            "fixture.unknown_contract",
            format!("unknown fixture contract {contract}"),
        )),
    }
}

#[derive(Debug, Error)]
pub enum FixtureVerificationError {
    #[error("fixture I/O failed: {0}")]
    Io(std::io::Error),
    #[error("fixture manifest is invalid: {0}")]
    Manifest(serde_json::Error),
    #[error("golden fixture {path} was rejected: {error}")]
    UnexpectedRejection { path: String, error: WireError },
    #[error("negative fixture has no expected reject layer: {0}")]
    MissingRejectLayer(String),
    #[error("negative fixture {path} uses unsupported reject layer {layer}")]
    UnsupportedRejectLayer { path: String, layer: String },
    #[error("negative fixture was accepted: {0}")]
    UnexpectedAcceptance(String),
    #[error("fixture {path} expected {expected} but received {actual}")]
    WrongError {
        path: String,
        expected: String,
        actual: String,
    },
    #[error("negative fixture lacks an expected error code: {0}")]
    MissingExpectation(String),
    #[error("golden fixture unexpectedly declares an error: {0}")]
    UnexpectedExpectation(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repository_root() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    #[test]
    fn all_golden_wire_fixtures_roundtrip_canonically() {
        verify_golden_fixture_directory(&repository_root().join("wire/golden")).unwrap();
    }

    #[test]
    fn every_shared_negative_fixture_is_rejected_with_the_expected_code() {
        verify_negative_fixture_directory(&repository_root().join("wire/negative")).unwrap();
    }
}
