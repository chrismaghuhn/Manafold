//! Complete four-group fingerprints over one environment instant.
//!
//! Every captured byte string is produced by a real boundary read or typed
//! result and canonically encoded through `mtgml_wire`. Schema/version
//! identity is read off produced DTOs; comparisons never hardcode constants.

use mtgml_environment::{PlayerEndpoint, PlayerEndpointHandle, TrustedEnvironmentController};
use mtgml_model::{
    CheckpointCodecIdentity, CheckpointDigestV3, EnvironmentLimitCounters, EpisodeStatus,
    FullStateDigestV3, InformationStateDigestV2, PlayerId, StateRevision, VisibleSequence,
};
use mtgml_observation::{
    PlayerServiceErrorCodeV1, PlayerStepSubmissionV1, PlayerStepV2, PlayerSubmissionCodeV1,
};
use mtgml_state::EngineState;
use mtgml_wire::{compute_information_state_digest_v2, encode_canonical};

use super::HarnessError;

/// Schema/version strings read off actually produced player-boundary DTOs.
///
/// A field is `None` exactly when the captured calls did not produce that
/// DTO; it is never filled from a hardcoded constant.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlayerProtocolIdentitySurface {
    pub observation_schema: Option<String>,
    pub information_state_schema: Option<String>,
    pub observed_event_schema: Option<String>,
    pub player_step_schema: Option<String>,
    pub player_decision_request_schema: Option<String>,
    pub decision_response_schema: Option<String>,
}

/// Schema/version strings read off actually produced trusted-environment
/// artifacts (checkpoint and exported replay).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustedEnvironmentIdentitySurface {
    pub checkpoint_schema: String,
    pub checkpoint_codec_id: String,
    pub checkpoint_codec_semantic_version: String,
    pub replay_manifest_schema: String,
    pub replay_step_schema: String,
    pub replay_file_schema: String,
}

/// One perspective's visible product, captured only through real
/// `PlayerEndpointHandle` reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerVisibleSnapshot {
    pub perspective: PlayerId,
    /// Canonical bytes of the `ObservationEnvelope` returned by
    /// `observation()`.
    pub current_observation_bytes: Vec<u8>,
    /// Canonical bytes of the `PlayerInformationStateV2` returned by
    /// `information_state()`.
    pub information_state_bytes: Vec<u8>,
    pub information_digest: InformationStateDigestV2,
    /// Canonical bytes over the `Some` value of `visible_decision()`.
    pub visible_decision_bytes: Option<Vec<u8>>,
    pub current_visible_sequence: VisibleSequence,
    pub protocol: PlayerProtocolIdentitySurface,
}

/// The visible product of one submission, built from a real typed submit
/// result or from the closed endpoint failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionVisibleProduct {
    pub observed_event_bytes: Vec<Vec<u8>>,
    pub player_step_bytes: Vec<u8>,
    /// `"accepted"` or the closed `PlayerSubmissionCodeV1` wire string.
    pub semantic_submission_code: Option<String>,
    /// Closed layer-A wire code (`"malformed_response"`); filled only by
    /// byte-level submission callers.
    pub wire_error_code: Option<String>,
    /// Closed service-failure code (`"service_unavailable"`).
    pub endpoint_error_code: Option<String>,
    pub protocol: PlayerProtocolIdentitySurface,
}

/// Semantic-state group. The engine-state probe is kept alongside the
/// digest as a belt-and-braces structural equality witness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticStateFingerprint {
    pub revision: StateRevision,
    pub full_state_digest: FullStateDigestV3,
    pub engine_state_equal_probe: EngineState,
}

/// Trusted-environment group, including the trusted-side protocol surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentFingerprint {
    pub status: EpisodeStatus,
    pub limit_counters: EnvironmentLimitCounters,
    pub codec: CheckpointCodecIdentity,
    pub checkpoint_digest: CheckpointDigestV3,
    pub surface: TrustedEnvironmentIdentitySurface,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerVisibleFingerprint {
    pub p1_snapshot: PlayerVisibleSnapshot,
    pub p2_snapshot: PlayerVisibleSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayRecorderFingerprint {
    /// Canonical bytes of `export_replay()`.
    pub exported_replay_bytes: Vec<u8>,
}

/// The complete fingerprint of one M2 environment instant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompleteM2Fingerprint {
    pub semantic: SemanticStateFingerprint,
    pub environment: EnvironmentFingerprint,
    pub player: PlayerVisibleFingerprint,
    pub replay_recorder: ReplayRecorderFingerprint,
}

/// Explicit comparison policy between two complete fingerprints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FingerprintComparison {
    /// Every group must be equal, including the replay recorder.
    All,
    /// Semantic, environment, and player groups must be equal; the caller
    /// asserts the recorder segment anchor separately.
    ExcludeReplayRecorder,
}

/// Captures one perspective's snapshot through the real endpoint reads.
///
/// The persisted information-state digest is recomputed independently via
/// the public digest computation and asserted against the DTO value before
/// the snapshot is returned.
pub fn capture_snapshot(
    endpoint: &PlayerEndpointHandle,
) -> Result<PlayerVisibleSnapshot, HarnessError> {
    let perspective = endpoint.perspective();
    let observation = endpoint
        .observation()
        .map_err(|_| HarnessError::EndpointService)?;
    let current_observation_bytes =
        encode_canonical(&observation).map_err(|_| HarnessError::WireEncoding)?;
    let information_state = endpoint
        .information_state()
        .map_err(|_| HarnessError::EndpointService)?;
    let information_state_bytes =
        encode_canonical(&information_state).map_err(|_| HarnessError::WireEncoding)?;
    let (_, recomputed) = compute_information_state_digest_v2(&information_state.digest_input())
        .map_err(|_| HarnessError::WireEncoding)?;
    if recomputed != information_state.digest {
        return Err(HarnessError::InformationDigestMismatch);
    }
    let visible_decision = endpoint
        .visible_decision()
        .map_err(|_| HarnessError::EndpointService)?;
    let visible_decision_bytes = visible_decision
        .as_ref()
        .map(|decision| encode_canonical(decision).map_err(|_| HarnessError::WireEncoding))
        .transpose()?;
    let protocol = PlayerProtocolIdentitySurface {
        observation_schema: Some(observation.schema_version),
        information_state_schema: Some(information_state.schema_version),
        observed_event_schema: None,
        player_step_schema: None,
        player_decision_request_schema: visible_decision.map(|decision| decision.schema_version),
        decision_response_schema: None,
    };
    Ok(PlayerVisibleSnapshot {
        perspective,
        current_observation_bytes,
        information_state_bytes,
        information_digest: information_state.digest,
        visible_decision_bytes,
        current_visible_sequence: information_state.next_visible_sequence,
        protocol,
    })
}

/// Maps a typed submit outcome into its visible product.
///
/// Accepted steps and typed rejections (carried inside
/// `PlayerStepSubmissionV1`) both derive from the returned step; only the
/// closed service failure produces an empty product with the endpoint error
/// code.
pub fn capture_transition_product(
    submit_result: Result<PlayerStepV2, mtgml_environment::PlayerEndpointError>,
) -> Result<TransitionVisibleProduct, HarnessError> {
    match submit_result {
        Ok(step) => {
            let mut observed_event_bytes = Vec::with_capacity(step.observed_events.len());
            for event in &step.observed_events {
                observed_event_bytes
                    .push(encode_canonical(event).map_err(|_| HarnessError::WireEncoding)?);
            }
            let player_step_bytes =
                encode_canonical(&step).map_err(|_| HarnessError::WireEncoding)?;
            let semantic_submission_code = match step.submission {
                PlayerStepSubmissionV1::Accepted => Some("accepted".to_owned()),
                PlayerStepSubmissionV1::Rejected { code } => {
                    Some(submission_code_string(code).to_owned())
                }
            };
            let protocol = PlayerProtocolIdentitySurface {
                observation_schema: None,
                information_state_schema: Some(step.information_state.schema_version.clone()),
                observed_event_schema: step
                    .observed_events
                    .first()
                    .map(|event| event.schema_version.clone()),
                player_step_schema: Some(step.schema_version.clone()),
                player_decision_request_schema: step
                    .next_decision
                    .as_ref()
                    .map(|decision| decision.schema_version.clone()),
                decision_response_schema: None,
            };
            Ok(TransitionVisibleProduct {
                observed_event_bytes,
                player_step_bytes,
                semantic_submission_code,
                wire_error_code: None,
                endpoint_error_code: None,
                protocol,
            })
        }
        Err(mtgml_environment::PlayerEndpointError::ServiceUnavailable) => {
            Ok(TransitionVisibleProduct {
                observed_event_bytes: Vec::new(),
                player_step_bytes: Vec::new(),
                semantic_submission_code: None,
                wire_error_code: None,
                endpoint_error_code: Some(
                    PlayerServiceErrorCodeV1::ServiceUnavailable
                        .code()
                        .to_owned(),
                ),
                protocol: PlayerProtocolIdentitySurface::default(),
            })
        }
    }
}

/// Exhaustively mirrors the serde snake_case wire names of
/// `PlayerSubmissionCodeV1`; a new variant fails compilation here instead of
/// silently comparing under an invented code.
fn submission_code_string(code: PlayerSubmissionCodeV1) -> &'static str {
    match code {
        PlayerSubmissionCodeV1::StaleDecision => "stale_decision",
        PlayerSubmissionCodeV1::UnavailableDecision => "unavailable_decision",
        PlayerSubmissionCodeV1::InvalidAnswer => "invalid_answer",
        PlayerSubmissionCodeV1::InvalidCandidate => "invalid_candidate",
        PlayerSubmissionCodeV1::DuplicateAssignment => "duplicate_assignment",
        PlayerSubmissionCodeV1::InvalidCardinality => "invalid_cardinality",
        PlayerSubmissionCodeV1::InvalidNumber => "invalid_number",
        PlayerSubmissionCodeV1::InvalidOrder => "invalid_order",
        PlayerSubmissionCodeV1::EpisodeClosed => "episode_closed",
    }
}

/// Captures the complete four-group fingerprint of the controller's current
/// instant plus both bound endpoints' snapshots.
pub fn capture_complete(
    controller: &TrustedEnvironmentController,
    endpoints: &[PlayerEndpointHandle; 2],
) -> Result<CompleteM2Fingerprint, HarnessError> {
    let checkpoint = controller
        .checkpoint()
        .map_err(|_| HarnessError::ControllerService)?;
    let replay = controller
        .export_replay()
        .map_err(|_| HarnessError::ControllerService)?;
    let exported_replay_bytes =
        encode_canonical(&replay).map_err(|_| HarnessError::WireEncoding)?;
    let p1_snapshot = capture_snapshot(&endpoints[0])?;
    let p2_snapshot = capture_snapshot(&endpoints[1])?;
    let surface = TrustedEnvironmentIdentitySurface {
        checkpoint_schema: checkpoint.schema_version.clone(),
        checkpoint_codec_id: checkpoint.codec.codec_id.clone(),
        checkpoint_codec_semantic_version: checkpoint.codec.semantic_version.clone(),
        replay_manifest_schema: replay.manifest.schema_version.clone(),
        replay_step_schema: replay.manifest.schemas.replay_step.clone(),
        replay_file_schema: replay.schema_version.clone(),
    };
    Ok(CompleteM2Fingerprint {
        semantic: SemanticStateFingerprint {
            revision: checkpoint.state.revision,
            full_state_digest: checkpoint.state_digest.clone(),
            engine_state_equal_probe: checkpoint.state.clone(),
        },
        environment: EnvironmentFingerprint {
            status: checkpoint.status,
            limit_counters: checkpoint.limit_counters,
            codec: checkpoint.codec,
            checkpoint_digest: checkpoint.checkpoint_digest,
            surface,
        },
        player: PlayerVisibleFingerprint {
            p1_snapshot,
            p2_snapshot,
        },
        replay_recorder: ReplayRecorderFingerprint {
            exported_replay_bytes,
        },
    })
}

/// Asserts the declared comparison policy between two complete fingerprints.
///
/// `All` requires every group to be equal. `ExcludeReplayRecorder` requires
/// the semantic, environment, and player groups to be equal while leaving
/// the recorder segment anchor to a separate caller assertion.
pub fn assert_fingerprint_policies(
    before: &CompleteM2Fingerprint,
    after: &CompleteM2Fingerprint,
    comparison: FingerprintComparison,
) -> Result<(), HarnessError> {
    if before.semantic != after.semantic {
        return Err(HarnessError::SemanticGroupMismatch);
    }
    if before.environment != after.environment {
        return Err(HarnessError::EnvironmentGroupMismatch);
    }
    if before.player != after.player {
        return Err(HarnessError::PlayerGroupMismatch);
    }
    if comparison == FingerprintComparison::All && before.replay_recorder != after.replay_recorder {
        return Err(HarnessError::ReplayRecorderGroupMismatch);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::isolation::paired::{
        base_pair_state, spawn_environment, synthetic_environment_config,
    };
    use mtgml_environment::ENVIRONMENT_CHECKPOINT_SCHEMA;
    use mtgml_observation::{INFORMATION_STATE_SCHEMA_V2, OBSERVATION_SCHEMA};
    use mtgml_replay::{REPLAY_FILE_SCHEMA_V3, REPLAY_MANIFEST_SCHEMA_V3};

    const P1: PlayerId = PlayerId(1);
    const P2: PlayerId = PlayerId(2);

    #[test]
    fn snapshot_capture_uses_real_endpoints_and_recomputes_digest() {
        let state = base_pair_state(&"11".repeat(32)).unwrap();
        let config = synthetic_environment_config([P1, P2]);
        let (controller, endpoints) = spawn_environment(state, &config).unwrap();
        let snapshot = capture_snapshot(&endpoints[0]).unwrap();
        assert_eq!(snapshot.perspective, P1);
        assert_eq!(
            snapshot.protocol.observation_schema.as_deref(),
            Some(OBSERVATION_SCHEMA)
        );
        assert_eq!(
            snapshot.protocol.information_state_schema.as_deref(),
            Some(INFORMATION_STATE_SCHEMA_V2)
        );
        assert!(!snapshot.current_observation_bytes.is_empty());
        assert!(!snapshot.information_state_bytes.is_empty());
        // The recomputed digest was checked inside capture; the captured
        // digest also equals a fresh independent recomputation.
        let fresh_information_state = endpoints[0].information_state().unwrap();
        let (_, recomputed) =
            compute_information_state_digest_v2(&fresh_information_state.digest_input()).unwrap();
        assert_eq!(snapshot.information_digest, recomputed);

        let complete_before = capture_complete(&controller, &endpoints).unwrap();
        let state_again = base_pair_state(&"11".repeat(32)).unwrap();
        let (controller_again, endpoints_again) = spawn_environment(state_again, &config).unwrap();
        let complete_after = capture_complete(&controller_again, &endpoints_again).unwrap();
        assert_fingerprint_policies(
            &complete_before,
            &complete_after,
            FingerprintComparison::All,
        )
        .unwrap();
        assert_eq!(
            complete_before
                .environment
                .surface
                .checkpoint_schema
                .as_str(),
            ENVIRONMENT_CHECKPOINT_SCHEMA
        );
        assert_eq!(
            complete_before
                .environment
                .surface
                .replay_manifest_schema
                .as_str(),
            REPLAY_MANIFEST_SCHEMA_V3
        );
        assert_eq!(
            complete_before
                .environment
                .surface
                .replay_file_schema
                .as_str(),
            REPLAY_FILE_SCHEMA_V3
        );
    }

    #[test]
    fn transition_product_maps_accepted_typed_and_service_outcomes() {
        use crate::isolation::paired::test_support::rejected_submit_result;
        let product = capture_transition_product(rejected_submit_result()).unwrap();
        assert_eq!(
            product.semantic_submission_code.as_deref(),
            Some("invalid_answer")
        );
        assert!(product.endpoint_error_code.is_none());
        assert_eq!(
            product.protocol.player_step_schema.as_deref(),
            Some("player-step.v2")
        );
        assert_eq!(product.observed_event_bytes.len(), 0);

        let service = capture_transition_product(Err(
            mtgml_environment::PlayerEndpointError::ServiceUnavailable,
        ))
        .unwrap();
        assert_eq!(
            service.endpoint_error_code.as_deref(),
            Some("service_unavailable")
        );
        assert!(service.semantic_submission_code.is_none());
    }
}
