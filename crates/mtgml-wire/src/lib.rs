//! Canonical JSON writer/reader for every public v1 wire contract.
//!
//! JSON Schema validates shape. This crate additionally enforces closed Rust
//! variants, integer ranges, canonical bytes, and cross-field semantics.

use mtgml_decision::{
    DecisionResponse, DecisionResponseV2, PlayerDecisionRequest, PlayerDecisionRequestV2,
};
use mtgml_model::EpisodeStatus;
use mtgml_observation::{
    InformationStateDigestInputV2, InformationStateEnvelope, ObservationEnvelope,
    ObservedEventEnvelope, ObservedEventEnvelopeV2, PlayerInformationStateV2, PlayerStep,
    PlayerStepV2,
};
use mtgml_replay::{
    AuthoritativeReplayV1, AuthoritativeReplayV2, AuthoritativeReplayV3, ReplayManifestV1,
    ReplayManifestV2, ReplayManifestV3,
};
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

impl WireContract for PlayerDecisionRequestV2 {
    fn validate_wire(&self) -> Result<(), WireError> {
        self.validate()
            .map_err(|error| WireError::new("semantic.decision", error.to_string()))
    }
}

impl WireContract for DecisionResponseV2 {
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
    // Preserved order for every pre-existing wire consumer: closed semantic
    // validation precedes the canonical byte comparison.
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

/// Player-submission entry decode for `DecisionResponseV2`.
///
/// Layer A of the accepted boundary: malformed/noncanonical/wrong-schema
/// bytes are rejected here with the closed wire code, while response-local
/// semantics (variant/membership/uniqueness/canonical/bounds) deliberately
/// remain the typed endpoint's responsibility.
pub mod decision_response_v2 {
    use super::*;

    pub fn decode_submission(bytes: &[u8]) -> Result<DecisionResponseV2, WireError> {
        let response = decode_canonical_shape::<DecisionResponseV2>(bytes)?;
        if response.schema_version != mtgml_decision::DECISION_RESPONSE_V2_SCHEMA {
            return Err(WireError::new(
                "decode.unknown_schema",
                "unsupported decision-response schema version",
            ));
        }
        Ok(response)
    }
}

/// Canonical shape decoding only: JSON parse, closed serde shape/types, and
/// canonical byte comparison. Deliberately skips `WireContract` semantic
/// validation so request-relative checks can run later at their owning
/// layer. Not public API; the public submission entry composes this.
fn decode_canonical_shape<T>(bytes: &[u8]) -> Result<T, WireError>
where
    T: DeserializeOwned + Serialize,
{
    let value: T = serde_json::from_slice(bytes)
        .map_err(|error| WireError::new("decode.invalid_json", error.to_string()))?;
    let canonical = encode_shape(&value)?;
    if canonical != bytes {
        return Err(WireError::new(
            "decode.non_canonical_json",
            "wire bytes are valid JSON but not the canonical representation",
        ));
    }
    Ok(value)
}

/// Shape-only canonical encoding (no semantic validation), used exclusively
/// by `decode_canonical_shape` for the byte-equality check.
fn encode_shape<T: Serialize>(value: &T) -> Result<Vec<u8>, WireError> {
    let value = serde_json::to_value(value)
        .map_err(|error| WireError::new("encode.serialization", error.to_string()))?;
    serde_json::to_vec(&canonicalize(value))
        .map_err(|error| WireError::new("encode.serialization", error.to_string()))
}

/// Versioned public wire code family for player-boundary decode failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerWireErrorCodeV1 {
    MalformedResponse,
}

impl PlayerWireErrorCodeV1 {
    pub fn code(self) -> &'static str {
        match self {
            Self::MalformedResponse => "malformed_response",
        }
    }
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
        "player-decision-request.v2" => {
            decode_canonical::<PlayerDecisionRequestV2>(bytes).map(drop)
        }
        "decision-response.v2" => decode_canonical::<DecisionResponseV2>(bytes).map(drop),
        "observation-envelope.v1" => decode_canonical::<ObservationEnvelope>(bytes).map(drop),
        "information-state-envelope.v1" => {
            decode_canonical::<InformationStateEnvelope>(bytes).map(drop)
        }
        "observed-event-envelope.v1" => decode_canonical::<ObservedEventEnvelope>(bytes).map(drop),
        "player-step.v1" => decode_canonical::<PlayerStep>(bytes).map(drop),
        "information-state-envelope.v2" => {
            decode_canonical::<PlayerInformationStateV2>(bytes).map(drop)
        }
        "observed-event-envelope.v2" => {
            decode_canonical::<ObservedEventEnvelopeV2>(bytes).map(drop)
        }
        "player-step.v2" => decode_canonical::<PlayerStepV2>(bytes).map(drop),
        "episode-status.v1" => decode_canonical::<EpisodeStatus>(bytes).map(drop),
        "replay-manifest.v1" => decode_canonical::<ReplayManifestV1>(bytes).map(drop),
        "authoritative-replay.v1" => decode_canonical::<AuthoritativeReplayV1>(bytes).map(drop),
        "replay-manifest.v2" => decode_canonical::<ReplayManifestV2>(bytes).map(drop),
        "authoritative-replay.v2" => decode_canonical::<AuthoritativeReplayV2>(bytes).map(drop),
        "replay-manifest.v3" => decode_canonical::<ReplayManifestV3>(bytes).map(drop),
        "authoritative-replay.v3" => decode_canonical::<AuthoritativeReplayV3>(bytes).map(drop),
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
    use mtgml_model::{
        InformationStateDigestV2, ObservationDigest, PlayerId, StateRevision, VisibleSequence,
    };
    use mtgml_observation::{
        InformationStateDigestInputV2, ObservationEnvelope, OBSERVATION_SCHEMA,
    };

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

    #[test]
    fn information_state_digest_v2_known_answer() {
        let input = InformationStateDigestInputV2 {
            schema_version: "information-state-digest-input.v2".into(),
            perspective: PlayerId(1),
            state_revision: StateRevision(0),
            current_observation: ObservationEnvelope {
                schema_version: OBSERVATION_SCHEMA.into(),
                perspective: PlayerId(1),
                state_revision: StateRevision(0),
                payload_codec: "synthetic-m2-observation.v1".into(),
                payload_base64: "e30=".into(),
                digest: ObservationDigest::from_canonical_bytes(b"{}"),
            },
            next_visible_sequence: VisibleSequence(0),
            retained_knowledge: vec![],
        };
        let (bytes, digest) = compute_information_state_digest_v2(&input).unwrap();
        assert_eq!(
            String::from_utf8(bytes).unwrap(),
            r#"{"current_observation":{"digest":"90845308617867fd703c6c4f37ede7908da24420053821f89190ad36236dfca3","payload_base64":"e30=","payload_codec":"synthetic-m2-observation.v1","perspective":"1","schema_version":"observation-envelope.v1","state_revision":"0"},"next_visible_sequence":"0","perspective":"1","retained_knowledge":[],"schema_version":"information-state-digest-input.v2","state_revision":"0"}"#
        );
        assert_eq!(
            digest,
            InformationStateDigestV2::parse(
                "a329332227a8e6f4ca95e4e798e5fad3996f344ec924070b71080f44291e2f33",
            )
            .unwrap()
        );
    }
}

/// Constructive producer nodes for the gate-owned player DTOs (M2.H H.1-iii).
///
/// Each test builds one gate-owned type with an explicit struct literal
/// naming EVERY top-level field — never `..Default::default()` or a `::new()`
/// shortcut — semantically equal to one checked-in golden fixture, then
/// asserts byte equality through [`encode_canonical`]. Adding, removing, or
/// retyping a DTO field breaks this module's compilation until the drift is
/// consciously re-reviewed against the shared fixture bytes.
#[cfg(test)]
mod constructive_producer_tests {
    use mtgml_decision::{
        CandidateIntent, DecisionAnswerV2, DecisionDomainV2, DecisionResponseV2,
        DecisionVisibility, PlayerDecisionRequestV2, VisibleCandidateV2,
        DECISION_RESPONSE_V2_SCHEMA, PLAYER_DECISION_REQUEST_V2_SCHEMA,
    };
    use mtgml_model::{
        CandidateIdV1, CardDefinitionId, EpisodeStatus, InformationStateDigestV2,
        ObservationDigest, OpaqueObjectId, PlayerDecisionIdV1, PlayerId, PlayerOutcome,
        PlayerResult, StateRevision, TerminalReason, VisibleSequence, ZoneKind,
    };
    use mtgml_observation::{
        ObservationEnvelope, PlayerInformationStateV2, PlayerKnowledgeCauseV1,
        PlayerKnowledgeChannelV1, PlayerKnowledgeInvalidationReasonV1,
        PlayerKnowledgeInvalidationV1, PlayerKnowledgeProvenanceV1, PlayerKnownLocationFactV1,
        PlayerKnownLocationV1, PlayerKnownObjectV1, PlayerStepSubmissionV1, PlayerStepV2,
        INFORMATION_STATE_SCHEMA_V2, OBSERVATION_SCHEMA, PLAYER_STEP_SCHEMA_V2,
    };

    use crate::encode_canonical;

    const ZERO_DIGEST: &str = "0000000000000000000000000000000000000000000000000000000000000000";
    const GOLDEN_INFORMATION_STATE_DIGEST_V2: &str =
        "256b504fe8fc2b9cb41395986c74586ea5617cf192a8939f05e7373f25dd41ca";

    fn golden_fixture(name: &str) -> Vec<u8> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        std::fs::read(root.join("wire/golden").join(name)).expect("golden fixture is readable")
    }

    fn observed_provenance(
        channel: PlayerKnowledgeChannelV1,
        sequence: u64,
        cause: PlayerKnowledgeCauseV1,
    ) -> PlayerKnowledgeProvenanceV1 {
        PlayerKnowledgeProvenanceV1::Observed {
            channel,
            sequence: VisibleSequence(sequence),
            cause,
        }
    }

    fn constructed_information_state_v2() -> PlayerInformationStateV2 {
        PlayerInformationStateV2 {
            schema_version: INFORMATION_STATE_SCHEMA_V2.to_owned(),
            perspective: PlayerId(1),
            state_revision: StateRevision(0),
            current_observation: ObservationEnvelope {
                schema_version: OBSERVATION_SCHEMA.to_owned(),
                perspective: PlayerId(1),
                state_revision: StateRevision(0),
                payload_codec: "synthetic-m2-observation.v1".to_owned(),
                payload_base64: "e30=".to_owned(),
                digest: ObservationDigest::parse(ZERO_DIGEST).expect("zero digest"),
            },
            next_visible_sequence: VisibleSequence(5),
            retained_knowledge: vec![
                PlayerKnownObjectV1::Active {
                    opaque_object_id: OpaqueObjectId(3),
                    known_definition: Some(CardDefinitionId(42)),
                    current_known_location_fact: Some(PlayerKnownLocationFactV1 {
                        location: PlayerKnownLocationV1 {
                            zone: ZoneKind::Exile,
                            player: Some(PlayerId(2)),
                        },
                        provenance: observed_provenance(
                            PlayerKnowledgeChannelV1::Public,
                            4,
                            PlayerKnowledgeCauseV1::ExplicitReveal,
                        ),
                    }),
                    historical_locations: vec![PlayerKnownLocationFactV1 {
                        location: PlayerKnownLocationV1 {
                            zone: ZoneKind::Hand,
                            player: None,
                        },
                        provenance: observed_provenance(
                            PlayerKnowledgeChannelV1::Private,
                            3,
                            PlayerKnowledgeCauseV1::OwnPrivateIdentity,
                        ),
                    }],
                    acquisition: observed_provenance(
                        PlayerKnowledgeChannelV1::Private,
                        1,
                        PlayerKnowledgeCauseV1::PrivateLook,
                    ),
                },
                PlayerKnownObjectV1::Retired {
                    opaque_object_id: OpaqueObjectId(7),
                    known_definition: None,
                    last_known_location_fact: Some(PlayerKnownLocationFactV1 {
                        location: PlayerKnownLocationV1 {
                            zone: ZoneKind::Battlefield,
                            player: None,
                        },
                        provenance: PlayerKnowledgeProvenanceV1::InitialConfiguration,
                    }),
                    historical_locations: Vec::new(),
                    acquisition: observed_provenance(
                        PlayerKnowledgeChannelV1::Public,
                        2,
                        PlayerKnowledgeCauseV1::PublicEvent,
                    ),
                    invalidation: PlayerKnowledgeInvalidationV1 {
                        provenance: observed_provenance(
                            PlayerKnowledgeChannelV1::Public,
                            4,
                            PlayerKnowledgeCauseV1::ExplicitReveal,
                        ),
                        reason: PlayerKnowledgeInvalidationReasonV1::Shuffle,
                    },
                },
            ],
            digest: InformationStateDigestV2::parse(GOLDEN_INFORMATION_STATE_DIGEST_V2)
                .expect("golden information-state digest"),
        }
    }

    #[test]
    fn information_state_envelope_v2_constructs_the_golden_bytes() {
        assert_eq!(
            encode_canonical(&constructed_information_state_v2()).unwrap(),
            golden_fixture("information-state-envelope.v2.json")
        );
    }

    fn constructed_choose_one_request_v2() -> PlayerDecisionRequestV2 {
        PlayerDecisionRequestV2 {
            schema_version: PLAYER_DECISION_REQUEST_V2_SCHEMA.to_owned(),
            player_decision_id: PlayerDecisionIdV1(1),
            state_revision: StateRevision(0),
            actor: PlayerId(1),
            visibility: DecisionVisibility::Public,
            decision: DecisionDomainV2::ChooseOne,
            candidates: vec![
                VisibleCandidateV2 {
                    candidate_id: CandidateIdV1(0),
                    intent: CandidateIntent::ChooseBoolean { value: false },
                },
                VisibleCandidateV2 {
                    candidate_id: CandidateIdV1(1),
                    intent: CandidateIntent::ChooseBoolean { value: true },
                },
            ],
        }
    }

    #[test]
    fn player_decision_request_v2_constructs_the_golden_bytes() {
        assert_eq!(
            encode_canonical(&constructed_choose_one_request_v2()).unwrap(),
            golden_fixture("player-decision-request.v2.json")
        );
    }

    #[test]
    fn observation_envelope_v1_constructs_the_golden_bytes() {
        let value = ObservationEnvelope {
            schema_version: OBSERVATION_SCHEMA.to_owned(),
            perspective: PlayerId(1),
            state_revision: StateRevision(0),
            payload_codec: "synthetic-json.v1".to_owned(),
            payload_base64: "e30=".to_owned(),
            digest: ObservationDigest::parse(ZERO_DIGEST).expect("zero digest"),
        };
        assert_eq!(
            encode_canonical(&value).unwrap(),
            golden_fixture("observation-envelope.v1.json")
        );
    }

    #[test]
    fn decision_response_v2_select_one_constructs_the_golden_bytes() {
        let value = DecisionResponseV2 {
            schema_version: DECISION_RESPONSE_V2_SCHEMA.to_owned(),
            player_decision_id: PlayerDecisionIdV1(1),
            state_revision: StateRevision(0),
            answer: DecisionAnswerV2::SelectOne {
                candidate_id: CandidateIdV1(1),
            },
        };
        assert_eq!(
            encode_canonical(&value).unwrap(),
            golden_fixture("decision-response.v2-select-one.json")
        );
    }

    #[test]
    fn player_step_v2_constructs_the_golden_bytes() {
        let value = PlayerStepV2 {
            schema_version: PLAYER_STEP_SCHEMA_V2.to_owned(),
            information_state: constructed_information_state_v2(),
            observed_events: Vec::new(),
            next_decision: Some(constructed_choose_one_request_v2()),
            status: EpisodeStatus::Running,
            submission: PlayerStepSubmissionV1::Accepted,
        };
        assert_eq!(
            encode_canonical(&value).unwrap(),
            golden_fixture("player-step.v2.json")
        );
    }

    #[test]
    fn episode_status_terminal_concession_constructs_the_golden_bytes() {
        let value = EpisodeStatus::Terminal {
            reason: TerminalReason::Concession,
            players: vec![
                PlayerOutcome {
                    player: PlayerId(1),
                    result: PlayerResult::Win,
                },
                PlayerOutcome {
                    player: PlayerId(2),
                    result: PlayerResult::Loss,
                },
            ],
        };
        assert_eq!(
            encode_canonical(&value).unwrap(),
            golden_fixture("episode-status-terminal-concession.v1.json")
        );
    }
}
