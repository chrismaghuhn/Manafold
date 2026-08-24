//! M2.G G.6 Node B: information-rich EMPTY-replay identity parity.
//!
//! A wire-shaped empty `AuthoritativeReplayV3` is constructed around an
//! information-rich checkpoint identity (mirroring the golden
//! `authoritative-replay-empty.v3.json` shape under the synthetic M2 codec),
//! validated, executed from that checkpoint, and proven to preserve the
//! complete identity while endpoints rebuilt from the final checkpoint
//! reproduce the pre-capture P1/P2 snapshot bytes byte-for-byte. Every M2.G
//! gate remains `NOT_RUN`; nothing here is a gate verdict.

#[cfg(test)]
mod tests {
    use crate::isolation::checkpoint_parity::support::{config, information_rich_spawned, P1, P2};
    use crate::isolation::fingerprint::{
        assert_fingerprint_policies, capture_complete, capture_snapshot, FingerprintComparison,
    };
    use crate::isolation::HarnessError;
    use mtgml_environment::{
        ControllerError, EnvironmentCheckpointV3, SyntheticM1EnvironmentBackend,
        TrustedEnvironmentController,
    };
    use mtgml_replay::{
        AuthoritativeReplayV3, InitialEnvironmentIdentityV3, RandomnessIdentityV2,
        ReplayManifestV3, REPLAY_FILE_SCHEMA_V3, REPLAY_MANIFEST_SCHEMA_V3,
    };
    use mtgml_wire::{decode_canonical, encode_canonical};

    fn controller_service(_: ControllerError) -> HarnessError {
        HarnessError::ControllerService
    }

    /// The checkpoint identity triple shared by the manifest anchor and the
    /// empty segment's final identity.
    fn identity_of(checkpoint: &EnvironmentCheckpointV3) -> InitialEnvironmentIdentityV3 {
        InitialEnvironmentIdentityV3 {
            state_revision: checkpoint.state.revision,
            full_state_digest: checkpoint.state_digest.clone(),
            episode_status: checkpoint.status.clone(),
            environment_limit_counters: checkpoint.limit_counters.clone(),
            checkpoint_codec_identity: checkpoint.codec.clone(),
            checkpoint_digest: checkpoint.checkpoint_digest.clone(),
        }
    }

    #[test]
    fn final_identity_and_snapshot_parity() -> Result<(), HarnessError> {
        let (controller, endpoints) = information_rich_spawned()?;
        let pre = capture_complete(&controller, &endpoints)?;
        let pre_p1 = capture_snapshot(&endpoints[0])?;
        let pre_p2 = capture_snapshot(&endpoints[1])?;

        let checkpoint = controller.checkpoint().map_err(controller_service)?;
        let initial = identity_of(&checkpoint);
        initial
            .validate()
            .map_err(|_| HarnessError::CheckpointInvalid)?;
        assert_eq!(
            initial.checkpoint_codec_identity.codec_id, "synthetic-m2-memory",
            "the synthetic M2 codec must anchor the constructed replay"
        );
        assert_eq!(initial.checkpoint_codec_identity.semantic_version, "3");

        // Empty V3 replay mirroring the wire golden
        // authoritative-replay-empty.v3.json shape: manifest anchored at the
        // checkpoint identity, zero steps, final identity equal to initial.
        let harness_config = config();
        let manifest = ReplayManifestV3 {
            schema_version: REPLAY_MANIFEST_SCHEMA_V3.into(),
            engine_build: harness_config.replay.engine_build.clone(),
            kernel: harness_config.replay.kernel.clone(),
            rules_snapshot: harness_config.replay.rules_snapshot.clone(),
            format_policy_snapshot: harness_config.replay.format_policy_snapshot.clone(),
            oracle_snapshot: harness_config.replay.oracle_snapshot.clone(),
            card_bundle: harness_config.replay.card_bundle.clone(),
            schemas: harness_config.replay.schemas.clone(),
            randomness: RandomnessIdentityV2 {
                contract_id: harness_config.replay.randomness_contract_id.clone(),
                root_seed_hex: checkpoint.state.random.root_seed.to_lower_hex(),
            },
            decks: harness_config.replay.decks.clone(),
            initial_identity: initial.clone(),
        };
        manifest
            .validate()
            .map_err(|_| HarnessError::CheckpointInvalid)?;
        let replay = AuthoritativeReplayV3 {
            schema_version: REPLAY_FILE_SCHEMA_V3.into(),
            manifest,
            steps: Vec::new(),
            final_identity: initial.clone(),
        };
        replay
            .validate()
            .map_err(|_| HarnessError::CheckpointInvalid)?;

        // The constructed artifact survives the canonical wire round-trip.
        let bytes = encode_canonical(&replay).map_err(|_| HarnessError::WireEncoding)?;
        let decoded: AuthoritativeReplayV3 =
            decode_canonical(&bytes).map_err(|_| HarnessError::WireEncoding)?;
        assert_eq!(decoded, replay);

        // Execution from the anchoring checkpoint preserves the identity.
        let report = controller
            .execute_replay_from_checkpoint(checkpoint.clone(), replay)
            .map_err(controller_service)?;
        assert!(report.traces.is_empty());
        assert_eq!(identity_of(&report.final_checkpoint), initial);

        // The live controller (and its recorder) did not move.
        assert_eq!(
            controller.checkpoint().map_err(controller_service)?,
            checkpoint
        );
        let post = capture_complete(&controller, &endpoints)?;
        assert_fingerprint_policies(&pre, &post, FingerprintComparison::All)?;

        // Endpoints rebuilt from the FINAL checkpoint reproduce the
        // pre-capture snapshot bytes for both perspectives.
        let rebuilt_backend = SyntheticM1EnvironmentBackend::from_checkpoint(
            report.final_checkpoint.clone(),
            config(),
        )
        .map_err(|_| HarnessError::SyntheticBackendRejected)?;
        let rebuilt = TrustedEnvironmentController::new(rebuilt_backend);
        let rebuilt_endpoints = [
            rebuilt
                .bind_player(P1)
                .map_err(|_| HarnessError::BindFailed)?,
            rebuilt
                .bind_player(P2)
                .map_err(|_| HarnessError::BindFailed)?,
        ];
        assert_eq!(capture_snapshot(&rebuilt_endpoints[0])?, pre_p1);
        assert_eq!(capture_snapshot(&rebuilt_endpoints[1])?, pre_p2);
        Ok(())
    }
}
