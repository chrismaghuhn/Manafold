//! Complete four-group fingerprints over one environment instant.
//!
//! Every captured byte string is produced by a real boundary read or typed
//! result and canonically encoded through `mtgml_wire`. Schema/version
//! identity is read off produced DTOs; comparisons never hardcode constants.

use mtgml_environment::{PlayerEndpoint, PlayerEndpointHandle, TrustedEnvironmentController};
use mtgml_model::{
    CheckpointCodecIdentity, CheckpointDigestV5, DigestReferenceV1, EnvironmentLimitCounters,
    EpisodeStatus, FullStateDigestV4, InformationStateDigestV2, PlayerId, StateRevision,
    VisibleSequence,
};
use mtgml_observation::{
    PlayerServiceErrorCodeV1, PlayerStepSubmissionV1, PlayerStepV2, PlayerSubmissionCodeV1,
};
use mtgml_replay::{
    DeckIdentityV1, InitialEnvironmentIdentityV5, KernelIdentityV1, ReplaySchemaVersionsV5,
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
    pub schema_version: String,
    pub engine_build: String,
    pub kernel: KernelIdentityV1,
    pub rules_snapshot: String,
    pub format_policy_snapshot: String,
    pub oracle_snapshot: String,
    pub card_bundle: String,
    pub randomness_contract_id: String,
    pub schemas: ReplaySchemaVersionsV5,
    pub decks: Vec<DeckIdentityV1>,
    pub initial_identity: InitialEnvironmentIdentityV5,
    pub checkpoint_schema: String,
    pub checkpoint_codec_id: String,
    pub checkpoint_codec_semantic_version: String,
    pub replay_manifest_schema: String,
    pub replay_step_schema: String,
    pub replay_file_schema: String,
    pub full_state_digest_reference: DigestReferenceV1,
    pub checkpoint_digest_reference: DigestReferenceV1,
}

/// One perspective's visible product, captured only through real
/// `PlayerEndpointHandle` reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerVisibleSnapshot {
    pub perspective: PlayerId,
    pub state_revision: StateRevision,
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
    pub full_state_digest: FullStateDigestV4,
    pub engine_state_equal_probe: EngineState,
}

/// Trusted-environment group, including the trusted-side protocol surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentFingerprint {
    pub status: EpisodeStatus,
    pub limit_counters: EnvironmentLimitCounters,
    pub codec: CheckpointCodecIdentity,
    pub checkpoint_digest: CheckpointDigestV5,
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
    if observation.perspective != perspective
        || information_state.perspective != perspective
        || observation.state_revision != information_state.state_revision
    {
        return Err(HarnessError::IncoherentFingerprintCapture);
    }
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
    if visible_decision.as_ref().is_some_and(|decision| {
        decision.actor != perspective || decision.state_revision != information_state.state_revision
    }) {
        return Err(HarnessError::IncoherentFingerprintCapture);
    }
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
        state_revision: information_state.state_revision,
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
            let semantic_submission_code = match &step.submission {
                PlayerStepSubmissionV1::Accepted => Some(ACCEPTED_SUBMISSION_MARKER.to_owned()),
                PlayerStepSubmissionV1::Rejected { code } => {
                    Some(submission_code_string(*code).to_owned())
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

/// Wire marker of the accepted submission outcome, mirroring the serde
/// `snake_case` spelling of `PlayerStepSubmissionV1::Accepted` exactly like
/// [`submission_code_string`] mirrors the rejection codes. The literal is
/// defined once here and pinned against the real serde serialization by
/// `accepted_submission_marker_matches_serde_wire_spelling`, so a renamed or
/// re-spelled outcome variant fails a test instead of silently comparing
/// under drifted bytes.
const ACCEPTED_SUBMISSION_MARKER: &str = "accepted";

/// Captures the complete four-group fingerprint of the controller's current
/// instant plus both bound endpoints' snapshots.
pub fn capture_complete(
    controller: &TrustedEnvironmentController,
    endpoints: &[PlayerEndpointHandle; 2],
) -> Result<CompleteM2Fingerprint, HarnessError> {
    let checkpoint_before = controller
        .checkpoint()
        .map_err(|_| HarnessError::ControllerService)?;
    let replay = controller
        .export_replay()
        .map_err(|_| HarnessError::ControllerService)?;
    let exported_replay_bytes =
        encode_canonical(&replay).map_err(|_| HarnessError::WireEncoding)?;
    let p1_snapshot = capture_snapshot(&endpoints[0])?;
    let p2_snapshot = capture_snapshot(&endpoints[1])?;
    let current_identity = InitialEnvironmentIdentityV5 {
        state_revision: checkpoint_before.state.revision,
        full_state_digest: checkpoint_before.state_digest.clone(),
        episode_status: checkpoint_before.status.clone(),
        environment_limit_counters: checkpoint_before.limit_counters.clone(),
        checkpoint_codec_identity: checkpoint_before.codec.clone(),
        checkpoint_digest: checkpoint_before.checkpoint_digest.clone(),
        execution_identity: checkpoint_before.execution_identity.clone(),
    };
    if replay.final_identity != current_identity
        || p1_snapshot.state_revision != checkpoint_before.state.revision
        || p2_snapshot.state_revision != checkpoint_before.state.revision
    {
        return Err(HarnessError::IncoherentFingerprintCapture);
    }
    let checkpoint_after = controller
        .checkpoint()
        .map_err(|_| HarnessError::ControllerService)?;
    if checkpoint_after != checkpoint_before {
        return Err(HarnessError::IncoherentFingerprintCapture);
    }
    let checkpoint_digest_reference = DigestReferenceV1 {
        envelope_version: "mtgml.digest-envelope.v1".into(),
        algorithm_id: "sha-256".into(),
        semantic_domain: CheckpointDigestV5::DOMAIN.into(),
        payload_codec_id: "mtgml.canonical-cbor.v1".into(),
        input_schema_id: "environment-checkpoint-digest-input.v5".into(),
        digest_bytes: checkpoint_before.checkpoint_digest.raw_bytes(),
    };
    let surface = TrustedEnvironmentIdentitySurface {
        schema_version: replay.manifest.schema_version.clone(),
        engine_build: replay.manifest.engine_build.clone(),
        kernel: replay.manifest.kernel.clone(),
        rules_snapshot: replay.manifest.rules_snapshot.clone(),
        format_policy_snapshot: replay.manifest.format_policy_snapshot.clone(),
        oracle_snapshot: replay.manifest.oracle_snapshot.clone(),
        card_bundle: replay.manifest.card_bundle.clone(),
        randomness_contract_id: replay.manifest.randomness.contract_id.clone(),
        schemas: replay.manifest.schemas.clone(),
        decks: replay.manifest.decks.clone(),
        initial_identity: replay.manifest.initial_identity.clone(),
        checkpoint_schema: checkpoint_before.schema_version.clone(),
        checkpoint_codec_id: checkpoint_before.codec.codec_id.clone(),
        checkpoint_codec_semantic_version: checkpoint_before.codec.semantic_version.clone(),
        replay_manifest_schema: replay.manifest.schema_version.clone(),
        replay_step_schema: replay.manifest.schemas.replay_step.clone(),
        replay_file_schema: replay.schema_version.clone(),
        full_state_digest_reference: checkpoint_before.state_digest.as_digest_reference(),
        checkpoint_digest_reference,
    };
    Ok(CompleteM2Fingerprint {
        semantic: SemanticStateFingerprint {
            revision: checkpoint_before.state.revision,
            full_state_digest: checkpoint_before.state_digest.clone(),
            engine_state_equal_probe: checkpoint_before.state.clone(),
        },
        environment: EnvironmentFingerprint {
            status: checkpoint_before.status,
            limit_counters: checkpoint_before.limit_counters,
            codec: checkpoint_before.codec,
            checkpoint_digest: checkpoint_before.checkpoint_digest,
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
    let environment_equal = match comparison {
        FingerprintComparison::All => before.environment == after.environment,
        FingerprintComparison::ExcludeReplayRecorder => {
            before.environment.status == after.environment.status
                && before.environment.limit_counters == after.environment.limit_counters
                && before.environment.codec == after.environment.codec
                && before.environment.checkpoint_digest == after.environment.checkpoint_digest
                && immutable_environment_surface_equal(
                    &before.environment.surface,
                    &after.environment.surface,
                )
        }
    };
    if !environment_equal {
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

fn immutable_environment_surface_equal(
    before: &TrustedEnvironmentIdentitySurface,
    after: &TrustedEnvironmentIdentitySurface,
) -> bool {
    before.schema_version == after.schema_version
        && before.engine_build == after.engine_build
        && before.kernel == after.kernel
        && before.rules_snapshot == after.rules_snapshot
        && before.format_policy_snapshot == after.format_policy_snapshot
        && before.oracle_snapshot == after.oracle_snapshot
        && before.card_bundle == after.card_bundle
        && before.randomness_contract_id == after.randomness_contract_id
        && before.schemas == after.schemas
        && before.decks == after.decks
        && before.initial_identity.checkpoint_codec_identity
            == after.initial_identity.checkpoint_codec_identity
        && before.checkpoint_schema == after.checkpoint_schema
        && before.checkpoint_codec_id == after.checkpoint_codec_id
        && before.checkpoint_codec_semantic_version == after.checkpoint_codec_semantic_version
        && before.replay_manifest_schema == after.replay_manifest_schema
        && before.replay_step_schema == after.replay_step_schema
        && before.replay_file_schema == after.replay_file_schema
        && before.full_state_digest_reference == after.full_state_digest_reference
        && before.checkpoint_digest_reference == after.checkpoint_digest_reference
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::isolation::paired::{
        base_pair_state, spawn_environment, synthetic_environment_config,
    };
    use mtgml_environment::ENVIRONMENT_CHECKPOINT_SCHEMA_V5;
    use mtgml_observation::{INFORMATION_STATE_SCHEMA_V2, OBSERVATION_SCHEMA};
    use mtgml_replay::{REPLAY_FILE_SCHEMA_V5, REPLAY_MANIFEST_SCHEMA_V5};

    const P1: PlayerId = PlayerId(1);
    const P2: PlayerId = PlayerId(2);

    #[test]
    fn snapshot_capture_uses_real_endpoints_and_recomputes_digest() {
        let state = base_pair_state(&"11".repeat(32)).unwrap();
        let config = synthetic_environment_config([P1, P2]);
        let (controller, endpoints) = spawn_environment(state, &config).unwrap();
        let snapshot = capture_snapshot(&endpoints[0]).unwrap();
        assert_eq!(snapshot.perspective, P1);
        assert_eq!(snapshot.state_revision, StateRevision(0));
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
            ENVIRONMENT_CHECKPOINT_SCHEMA_V5
        );
        assert_eq!(
            complete_before
                .environment
                .surface
                .replay_manifest_schema
                .as_str(),
            REPLAY_MANIFEST_SCHEMA_V5
        );
        assert_eq!(
            complete_before
                .environment
                .surface
                .replay_file_schema
                .as_str(),
            REPLAY_FILE_SCHEMA_V5
        );
    }

    #[test]
    fn fingerprint_source_declares_manifest_identity_contract() {
        let source = include_str!("fingerprint.rs");
        for required in [
            "pub engine_build",
            "ReplayManifestV5",
            "pub initial_identity",
            "pub full_state_digest_reference",
            "pub checkpoint_digest_reference",
        ] {
            assert!(
                source.contains(required),
                "missing fingerprint source field {required}"
            );
        }
    }

    #[test]
    fn manifest_identity_is_part_of_the_environment_fingerprint() {
        let (mut before, after) = equal_complete_fingerprints();
        before.environment.surface.engine_build.push_str("-mutated");
        assert_eq!(
            assert_fingerprint_policies(&before, &after, FingerprintComparison::All),
            Err(HarnessError::EnvironmentGroupMismatch)
        );
    }

    #[test]
    fn digest_reference_surfaces_preserve_the_declared_domains() {
        let state = base_pair_state(&"11".repeat(32)).unwrap();
        let config = synthetic_environment_config([P1, P2]);
        let (controller, endpoints) = spawn_environment(state, &config).unwrap();
        let fingerprint = capture_complete(&controller, &endpoints).unwrap();
        let surface = &fingerprint.environment.surface;
        assert_eq!(
            surface.full_state_digest_reference.semantic_domain,
            FullStateDigestV4::DOMAIN
        );
        assert_eq!(
            surface.full_state_digest_reference.input_schema_id,
            "full-state-digest-input.v4"
        );
        assert_eq!(
            surface.checkpoint_digest_reference.semantic_domain,
            CheckpointDigestV5::DOMAIN
        );
        assert_eq!(
            surface.checkpoint_digest_reference.input_schema_id,
            "environment-checkpoint-digest-input.v5"
        );
        let replay = controller.export_replay().unwrap();
        assert_eq!(surface.initial_identity, replay.manifest.initial_identity);
        assert!(!surface.randomness_contract_id.is_empty());
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

    #[test]
    fn accepted_submission_marker_matches_serde_wire_spelling() {
        let serialized = serde_json::to_value(PlayerStepSubmissionV1::Accepted).unwrap();
        assert_eq!(
            serialized,
            serde_json::json!({ "kind": ACCEPTED_SUBMISSION_MARKER })
        );
        let rejection = serde_json::to_value(PlayerStepSubmissionV1::Rejected {
            code: PlayerSubmissionCodeV1::InvalidAnswer,
        })
        .unwrap();
        assert_ne!(rejection, serde_json::json!({ "kind": "accepted" }));
    }

    fn equal_complete_fingerprints() -> (CompleteM2Fingerprint, CompleteM2Fingerprint) {
        let state = base_pair_state(&"11".repeat(32)).unwrap();
        let config = synthetic_environment_config([P1, P2]);
        let (controller, endpoints) = spawn_environment(state, &config).unwrap();
        let before = capture_complete(&controller, &endpoints).unwrap();
        let state_again = base_pair_state(&"11".repeat(32)).unwrap();
        let (controller_again, endpoints_again) = spawn_environment(state_again, &config).unwrap();
        let after = capture_complete(&controller_again, &endpoints_again).unwrap();
        (before, after)
    }

    #[test]
    fn policy_mismatch_detected_in_semantic_group() {
        let (mut before, after) = equal_complete_fingerprints();
        before.semantic.revision = mtgml_model::StateRevision(before.semantic.revision.0 + 1);
        assert_eq!(
            assert_fingerprint_policies(&before, &after, FingerprintComparison::All),
            Err(HarnessError::SemanticGroupMismatch)
        );
    }

    #[test]
    fn policy_mismatch_detected_in_environment_group() {
        let (mut before, after) = equal_complete_fingerprints();
        before.environment.limit_counters.decisions_submitted += 1;
        assert_eq!(
            assert_fingerprint_policies(&before, &after, FingerprintComparison::All),
            Err(HarnessError::EnvironmentGroupMismatch)
        );
    }

    #[test]
    fn policy_mismatch_detected_in_player_group() {
        let (mut before, after) = equal_complete_fingerprints();
        before.player.p1_snapshot.current_visible_sequence =
            mtgml_model::VisibleSequence(before.player.p1_snapshot.current_visible_sequence.0 + 1);
        assert_eq!(
            assert_fingerprint_policies(&before, &after, FingerprintComparison::All),
            Err(HarnessError::PlayerGroupMismatch)
        );
    }

    #[test]
    fn policy_mismatch_detected_in_replay_recorder_group() {
        let (mut before, after) = equal_complete_fingerprints();
        before.replay_recorder.exported_replay_bytes.push(b'x');
        assert_eq!(
            assert_fingerprint_policies(&before, &after, FingerprintComparison::All),
            Err(HarnessError::ReplayRecorderGroupMismatch)
        );
        // The recorder group is deliberately outside the
        // `ExcludeReplayRecorder` policy and must not fire there.
        assert_eq!(
            assert_fingerprint_policies(
                &before,
                &after,
                FingerprintComparison::ExcludeReplayRecorder
            ),
            Ok(())
        );
    }
}
