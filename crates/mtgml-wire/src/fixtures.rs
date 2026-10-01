use crate::canonical_json::decode_canonical;
use crate::error::WireError;
use mtgml_decision::{DecisionResponseV3, PlayerDecisionRequestV4};
use mtgml_model::EpisodeStatus;
use mtgml_observation::{
    MagicBasicLandObservationV1, MagicSharedExecutionObservationV1, ObservationEnvelope,
    ObservedEventEnvelopeV4, PlayerInformationState, PlayerStepV4,
};
use mtgml_replay::{AuthoritativeReplayV8, ReplayManifestV8, ReplayStepV8};
use serde::Deserialize;
use std::{fs, path::Path};
use thiserror::Error;

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
        "player-decision-request.v4" => {
            decode_canonical::<PlayerDecisionRequestV4>(bytes).map(drop)
        }
        "decision-response.v3" => decode_canonical::<DecisionResponseV3>(bytes).map(drop),
        "observation-envelope.v2" => decode_canonical::<ObservationEnvelope>(bytes).map(drop),
        "information-state-envelope.v3" => {
            decode_canonical::<PlayerInformationState>(bytes).map(drop)
        }
        "observed-event-envelope.v4" => {
            decode_canonical::<ObservedEventEnvelopeV4>(bytes).map(drop)
        }
        "player-step.v4" => decode_canonical::<PlayerStepV4>(bytes).map(drop),
        "episode-status.v1" => decode_canonical::<EpisodeStatus>(bytes).map(drop),
        "replay-manifest.v8" => decode_canonical::<ReplayManifestV8>(bytes).map(drop),
        "replay-step.v8" => decode_canonical::<ReplayStepV8>(bytes).map(drop),
        "authoritative-replay.v8" => decode_canonical::<AuthoritativeReplayV8>(bytes).map(drop),
        "magic-basic-land-observation.v1" => {
            decode_canonical::<MagicBasicLandObservationV1>(bytes).map(drop)
        }
        "magic-shared-execution-observation.v1" => {
            decode_canonical::<MagicSharedExecutionObservationV1>(bytes).map(drop)
        }
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
