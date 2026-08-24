//! Multi-endpoint information isolation and query-purity evidence (M2.G G.3).
//!
//! Two-player synthetic environment only: no networking, sessions, auth, or
//! multiplayer semantics exist behind these endpoints. Every "reads do not
//! mutate" claim below is backed by a complete four-group fingerprint
//! comparison over real boundary reads, never by comments alone.

#[cfg(test)]
mod tests {
    use crate::isolation::fingerprint::{
        assert_fingerprint_policies, capture_complete, capture_snapshot,
        capture_transition_product, FingerprintComparison,
    };
    use crate::isolation::paired::test_support::accepted_entry_submission;
    use crate::isolation::paired::{
        base_pair_state, spawn_environment, synthetic_environment_config,
    };
    use crate::isolation::HarnessError;
    use mtgml_decision::{DecisionAnswerV2, DecisionResponseV2, DECISION_RESPONSE_V2_SCHEMA};
    use mtgml_environment::{PlayerEndpoint, PlayerEndpointHandle};
    use mtgml_model::{CandidateIdV1, PlayerDecisionIdV1, PlayerId, StateRevision};
    use mtgml_observation::PlayerStepSubmissionV1;
    use mtgml_replay::AuthoritativeReplayV3;
    use mtgml_state::{validate_engine_state, EngineState};

    const P1: PlayerId = PlayerId(1);
    const P2: PlayerId = PlayerId(2);

    const SEED_HEX_A: &str = "3333333333333333333333333333333333333333333333333333333333333333";
    const SEED_HEX_B: &str = "4444444444444444444444444444444444444444444444444444444444444444";

    /// The closed wire spelling mirrored from the typed
    /// `PlayerSubmissionCodeV1::UnavailableDecision` outcome.
    const UNAVAILABLE_DECISION_CODE: &str = "unavailable_decision";

    fn config() -> mtgml_environment::SyntheticM1EnvironmentConfig {
        synthetic_environment_config([P1, P2])
    }

    /// The established base pair state minus its authoritative pending
    /// request: a legal running instant where NO decision exists at all.
    /// Validation gates the shape exactly like every other harness state;
    /// runtime acceptance is exercised by `spawn_environment`.
    fn base_pair_without_pending_request(seed_hex: &str) -> Result<EngineState, HarnessError> {
        let mut state = base_pair_state(seed_hex)?;
        state.execution.pending_decision = None;
        validate_engine_state(&state).map_err(HarnessError::StateValidation)?;
        Ok(state)
    }

    /// A syntactically plausible response built purely from closed defaults;
    /// it references nothing foreign (no live id, candidate, or revision).
    fn plausible_response() -> DecisionResponseV2 {
        DecisionResponseV2 {
            schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
            player_decision_id: PlayerDecisionIdV1(1),
            state_revision: StateRevision(0),
            answer: DecisionAnswerV2::SelectOne {
                candidate_id: CandidateIdV1(0),
            },
        }
    }

    #[derive(Debug, Clone, Copy)]
    enum Read {
        Observation,
        Information,
        Decision,
    }

    /// The declared read orders of the purity matrix; each letter is ONE real
    /// endpoint read, interleaved across the two bound endpoints.
    const READ_SEQUENCES: &[&[Read]] = &[
        &[Read::Observation, Read::Information, Read::Decision],
        &[Read::Decision, Read::Observation, Read::Information],
        &[
            Read::Information,
            Read::Information,
            Read::Observation,
            Read::Decision,
            Read::Observation,
        ],
        &[Read::Decision, Read::Decision, Read::Decision],
    ];

    fn perform_read(
        endpoints: &[PlayerEndpointHandle; 2],
        index: usize,
        read: Read,
    ) -> Result<(), HarnessError> {
        let endpoint = &endpoints[index % endpoints.len()];
        match read {
            Read::Observation => {
                endpoint
                    .observation()
                    .map_err(|_| HarnessError::EndpointService)?;
            }
            Read::Information => {
                endpoint
                    .information_state()
                    .map_err(|_| HarnessError::EndpointService)?;
            }
            Read::Decision => {
                endpoint
                    .visible_decision()
                    .map_err(|_| HarnessError::EndpointService)?;
            }
        }
        Ok(())
    }

    /// Both endpoints coexist permanently. Binding happens ONLY through the
    /// controller at bind time; handles expose their perspective through a
    /// read-only accessor with no setter anywhere in the public API, so
    /// rebinding an existing handle is impossible by construction.
    #[test]
    fn case_coexist_binding_permanent() -> Result<(), HarnessError> {
        let state = base_pair_state(SEED_HEX_A)?;
        let (controller, endpoints) = spawn_environment(state, &config())?;
        assert_eq!(endpoints[0].perspective(), P1);
        assert_eq!(endpoints[1].perspective(), P2);
        assert_ne!(endpoints[0].perspective(), endpoints[1].perspective());
        // Both bound endpoints read concurrently from the same instant.
        let p1 = capture_snapshot(&endpoints[0])?;
        let p2 = capture_snapshot(&endpoints[1])?;
        assert_eq!(p1.perspective, P1);
        assert_eq!(p2.perspective, P2);
        // Binding remains creatable only through the controller; a fresh
        // bind yields an independent handle with the same permanent
        // perspective, never a moved or re-targeted one.
        let rebound = controller
            .bind_player(P1)
            .map_err(|_| HarnessError::BindFailed)?;
        assert_eq!(rebound.perspective(), P1);
        Ok(())
    }

    /// Public agreement / private divergence mid-episode: both perspectives
    /// observe the same state revision, identical protocol identity surface,
    /// and identical observation payload codec, while each snapshot carries
    /// exactly its own perspective's private projection (distinct bytes by
    /// construction). Captured at a running decision-free instant so both
    /// perspectives produce the same DTO set; ownership asymmetry of visible
    /// decisions is covered by `case_wrong_perspective_closed_surface`.
    #[test]
    fn case_public_private_projection_agreement() -> Result<(), HarnessError> {
        let state = base_pair_without_pending_request(SEED_HEX_A)?;
        let (_controller, endpoints) = spawn_environment(state, &config())?;
        let p1 = capture_snapshot(&endpoints[0])?;
        let p2 = capture_snapshot(&endpoints[1])?;
        // Same authoritative instant across both perspectives.
        let information_p1 = endpoints[0]
            .information_state()
            .map_err(|_| HarnessError::EndpointService)?;
        let information_p2 = endpoints[1]
            .information_state()
            .map_err(|_| HarnessError::EndpointService)?;
        assert_eq!(
            information_p1.state_revision, information_p2.state_revision,
            "both perspectives must observe one shared state revision"
        );
        // Identical protocol identity surface across perspectives.
        assert_eq!(p1.protocol, p2.protocol);
        // Each snapshot is exactly its own perspective's projection.
        assert_eq!(information_p1.perspective, P1);
        assert_eq!(information_p2.perspective, P2);
        // Private knowledge differs by construction: the projections are
        // distinct even though the trusted instant is shared.
        assert_ne!(p1.information_state_bytes, p2.information_state_bytes);
        // Observation payload codec identity agrees across perspectives.
        let observation_p1 = endpoints[0]
            .observation()
            .map_err(|_| HarnessError::EndpointService)?;
        let observation_p2 = endpoints[1]
            .observation()
            .map_err(|_| HarnessError::EndpointService)?;
        assert_eq!(observation_p1.payload_codec, observation_p2.payload_codec);
        assert_eq!(observation_p1.schema_version, observation_p2.schema_version);
        Ok(())
    }

    /// While P1 owns the pending request, a plausible P2 submission receives
    /// the closed `unavailable_decision` surface that is byte-indistinguishable
    /// from the same submission against an environment holding NO request:
    /// same closed code string and byte-equal serialized rejection shape and
    /// length. The wrong-perspective submit leaves the COMPLETE fingerprint
    /// (including the replay recorder) untouched.
    #[test]
    fn case_wrong_perspective_closed_surface() -> Result<(), HarnessError> {
        let state_a = base_pair_state(SEED_HEX_B)?;
        let (controller_a, endpoints_a) = spawn_environment(state_a.clone(), &config())?;
        // P1 owns the request; P2 sees none of it.
        assert!(endpoints_a[0]
            .visible_decision()
            .map_err(|_| HarnessError::EndpointService)?
            .is_some());
        assert!(endpoints_a[1]
            .visible_decision()
            .map_err(|_| HarnessError::EndpointService)?
            .is_none());
        let complete_before = capture_complete(&controller_a, &endpoints_a)?;

        let step_wrong = endpoints_a[1]
            .submit(plausible_response())
            .map_err(|_| HarnessError::EndpointService)?;
        let product_wrong = capture_transition_product(Ok(step_wrong))?;
        assert_eq!(
            product_wrong.semantic_submission_code.as_deref(),
            Some(UNAVAILABLE_DECISION_CODE)
        );

        // Twin environment where NO request exists at all.
        let state_b = base_pair_without_pending_request(SEED_HEX_B)?;
        let (_controller_b, endpoints_b) = spawn_environment(state_b, &config())?;
        let step_none = endpoints_b[1]
            .submit(plausible_response())
            .map_err(|_| HarnessError::EndpointService)?;
        let product_none = capture_transition_product(Ok(step_none))?;
        assert_eq!(
            product_none.semantic_submission_code.as_deref(),
            Some(UNAVAILABLE_DECISION_CODE)
        );
        // Indistinguishability: byte-equal serialized rejection shape AND
        // equal serialized length.
        assert_eq!(
            product_none.player_step_bytes, product_wrong.player_step_bytes,
            "wrong-actor and no-request rejections must be indistinguishable"
        );
        assert_eq!(
            product_none.player_step_bytes.len(),
            product_wrong.player_step_bytes.len()
        );

        // The wrong-perspective submit mutated nothing anywhere.
        let complete_after = capture_complete(&controller_a, &endpoints_a)?;
        assert_fingerprint_policies(
            &complete_before,
            &complete_after,
            FingerprintComparison::All,
        )?;
        Ok(())
    }

    /// Reads are pure: arbitrary interleaved read orders across both bound
    /// endpoints leave every perspective's final snapshot byte-identical to a
    /// fresh single-capture baseline and leave the complete fingerprint —
    /// including the replay recorder and every visible-sequence cursor —
    /// unchanged. Reads append no replay segment and move nothing.
    #[test]
    fn purity_read_order_matrix() -> Result<(), HarnessError> {
        // Fresh single-capture baseline per perspective.
        let baseline_state = base_pair_state(SEED_HEX_A)?;
        let (baseline_controller, baseline_endpoints) =
            spawn_environment(baseline_state, &config())?;
        let baseline = capture_complete(&baseline_controller, &baseline_endpoints)?;

        for sequence in READ_SEQUENCES {
            let state = base_pair_state(SEED_HEX_A)?;
            let (controller, endpoints) = spawn_environment(state, &config())?;
            for (index, read) in sequence.iter().enumerate() {
                perform_read(&endpoints, index, *read)?;
            }
            let after = capture_complete(&controller, &endpoints)?;
            // Byte-identical snapshots per perspective ...
            assert_eq!(
                after.player.p1_snapshot, baseline.player.p1_snapshot,
                "sequence {sequence:?}: P1 snapshot drifted under reads"
            );
            assert_eq!(
                after.player.p2_snapshot, baseline.player.p2_snapshot,
                "sequence {sequence:?}: P2 snapshot drifted under reads"
            );
            // ... and a completely unchanged environment.
            assert_fingerprint_policies(&baseline, &after, FingerprintComparison::All)?;
        }
        Ok(())
    }

    /// Restore through the ORIGINAL controller keeps the old handles bound
    /// (they share the backend Arc) and returns every perspective's visible
    /// product to the checkpoint-time bytes; the replay recorder restarts as
    /// a fresh segment anchored at exactly the restored checkpoint identity.
    /// A freshly bound handle after restore observes byte-equal projections.
    #[test]
    fn restore_with_live_handles_and_rebinding() -> Result<(), HarnessError> {
        let state = base_pair_state(SEED_HEX_A)?;
        let (controller, endpoints) = spawn_environment(state, &config())?;
        let cp0 = controller
            .checkpoint()
            .map_err(|_| HarnessError::ControllerService)?;
        let fp_cp0 = capture_complete(&controller, &endpoints)?;

        // One accepted transition advances the live environment past cp0.
        let step = accepted_entry_submission(&endpoints[0])?;
        assert_eq!(
            step.submission,
            PlayerStepSubmissionV1::Accepted,
            "the transition must be accepted before restore"
        );
        let fp_advanced = capture_complete(&controller, &endpoints)?;
        assert_ne!(
            fp_advanced.semantic.revision, fp_cp0.semantic.revision,
            "the accepted transition must advance the revision"
        );

        // Restore in place; the original handles stay bound.
        controller
            .restore(cp0.clone())
            .map_err(|_| HarnessError::ControllerService)?;
        let fp_restored = capture_complete(&controller, &endpoints)?;
        assert_fingerprint_policies(
            &fp_cp0,
            &fp_restored,
            FingerprintComparison::ExcludeReplayRecorder,
        )?;

        // Segment anchor: restoring seeds a FRESH replay segment whose
        // initial identity IS the restored checkpoint identity, with no steps.
        let exported: AuthoritativeReplayV3 = controller
            .export_replay()
            .map_err(|_| HarnessError::ControllerService)?;
        assert!(
            exported.steps.is_empty(),
            "a restored recorder starts an empty segment"
        );
        let anchor = &exported.manifest.initial_identity;
        assert_eq!(anchor.state_revision, cp0.state.revision);
        assert_eq!(anchor.full_state_digest, cp0.state_digest);
        assert_eq!(anchor.episode_status, cp0.status);
        assert_eq!(anchor.environment_limit_counters, cp0.limit_counters);
        assert_eq!(anchor.checkpoint_codec_identity, cp0.codec);
        assert_eq!(anchor.checkpoint_digest, cp0.checkpoint_digest);

        // Old handles observe byte-equal snapshots to the cp0-time captures.
        assert_eq!(capture_snapshot(&endpoints[0])?, fp_cp0.player.p1_snapshot);
        assert_eq!(capture_snapshot(&endpoints[1])?, fp_cp0.player.p2_snapshot);

        // Fresh binding after restore observes byte-equal projections too.
        let fresh_p1 = controller
            .bind_player(P1)
            .map_err(|_| HarnessError::BindFailed)?;
        let fresh_p2 = controller
            .bind_player(P2)
            .map_err(|_| HarnessError::BindFailed)?;
        assert_eq!(
            capture_snapshot(&fresh_p1)?,
            fp_cp0.player.p1_snapshot,
            "freshly rebound handle must see the restored projection"
        );
        assert_eq!(
            capture_snapshot(&fresh_p2)?,
            fp_cp0.player.p2_snapshot,
            "freshly rebound handle must see the restored projection"
        );
        Ok(())
    }

    /// A fork is a structurally separate controller: `fork()` wraps a FRESH
    /// backend Arc (`controller.rs`), source and fork share no mutable cell,
    /// and no API hands either backend to the other — crossing can happen
    /// only through caller-held variables. Driving ONE accepted transition
    /// through the fork's own endpoints therefore leaves the source's
    /// complete fingerprint and snapshots byte-unchanged while the fork's
    /// exported replay diverges.
    #[test]
    fn fork_controller_isolation() -> Result<(), HarnessError> {
        let state = base_pair_state(SEED_HEX_A)?;
        let (controller, endpoints) = spawn_environment(state, &config())?;
        let fp_source_before = capture_complete(&controller, &endpoints)?;

        let fork = controller
            .fork()
            .map_err(|_| HarnessError::ControllerService)?;
        let fork_endpoints = [
            fork.bind_player(P1).map_err(|_| HarnessError::BindFailed)?,
            fork.bind_player(P2).map_err(|_| HarnessError::BindFailed)?,
        ];
        let step = accepted_entry_submission(&fork_endpoints[0])?;
        assert_eq!(
            step.submission,
            PlayerStepSubmissionV1::Accepted,
            "the fork transition must be accepted"
        );

        // The source is byte-identical everywhere, recorder included.
        let fp_source_after = capture_complete(&controller, &endpoints)?;
        assert_fingerprint_policies(
            &fp_source_before,
            &fp_source_after,
            FingerprintComparison::All,
        )?;

        // The fork advanced while the source did not.
        let fork_after = capture_complete(&fork, &fork_endpoints)?;
        assert_ne!(
            fork_after.semantic.revision, fp_source_after.semantic.revision,
            "the fork must hold its own advanced instant"
        );
        // Their replay histories differ: the fork recorded the transition.
        let source_replay_bytes = fp_source_after
            .replay_recorder
            .exported_replay_bytes
            .clone();
        assert_ne!(
            fork_after.replay_recorder.exported_replay_bytes, source_replay_bytes,
            "the fork's export_replay must differ from the source's"
        );
        Ok(())
    }

    /// Two independently spawned environments from the same seed and config
    /// are deterministic twins: the identical accepted input produces
    /// byte-identical transition products, successor player-visible
    /// fingerprints, environment fingerprints, and complete fingerprints.
    #[test]
    fn accepted_determinism_twins() -> Result<(), HarnessError> {
        let first = base_pair_state(SEED_HEX_B)?;
        let second = base_pair_state(SEED_HEX_B)?;
        let (controller_one, endpoints_one) = spawn_environment(first, &config())?;
        let (controller_two, endpoints_two) = spawn_environment(second, &config())?;

        let step_one = accepted_entry_submission(&endpoints_one[0])?;
        let step_two = accepted_entry_submission(&endpoints_two[0])?;
        assert_eq!(
            step_one.submission,
            PlayerStepSubmissionV1::Accepted,
            "the twin inputs must be accepted"
        );
        let product_one = capture_transition_product(Ok(step_one))?;
        let product_two = capture_transition_product(Ok(step_two))?;
        assert_eq!(
            product_one, product_two,
            "byte-identical TransitionVisibleProduct required"
        );

        let successor_one = capture_complete(&controller_one, &endpoints_one)?;
        let successor_two = capture_complete(&controller_two, &endpoints_two)?;
        assert_eq!(
            successor_one.player, successor_two.player,
            "successor PlayerVisibleFingerprints must match"
        );
        assert_eq!(
            successor_one.environment, successor_two.environment,
            "EnvironmentFingerprints must match"
        );
        assert_fingerprint_policies(&successor_one, &successor_two, FingerprintComparison::All)?;
        Ok(())
    }
}
