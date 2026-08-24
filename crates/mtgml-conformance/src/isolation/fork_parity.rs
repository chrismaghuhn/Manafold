//! Fork information-parity evidence for M2.G G.5.
//!
//! A fork is a structurally separate controller over a fresh backend: this
//! slice proves that the fork holds exactly the source instant across the
//! semantic, environment, and player groups with a fresh empty recorder
//! segment anchored at the shared fork-time identity; that identical inputs
//! through both sides' real player endpoints produce byte-equal steps and
//! successor fingerprints; that divergence arises only from divergent input
//! streams while an input-frozen source stays byte-identical to its own
//! pre-fork capture under the complete `All` policy; and that accepted,
//! rejected, and restore mutations on the fork never leak into the source
//! (whose four fingerprint groups span knowledge, identity maps, retired
//! sets, visible sequences, continuations, RNG cursors, counters, and the
//! recorder). Both scenario classes are exercised. Every M2.G gate remains
//! `NOT_RUN`; nothing here is a gate verdict.

#[cfg(test)]
mod tests {
    use crate::isolation::checkpoint_parity::support::{
        assert_segment_anchor, choose_count_answer, decision_rich_spawned,
        information_rich_spawned, stale_members_answer, visible_request, ParityScenario,
        DIVERGENT_COUNT_VALUE, EQUAL_COUNT_VALUE, P1, P2,
    };
    use crate::isolation::fingerprint::{
        assert_fingerprint_policies, capture_complete, capture_transition_product,
        FingerprintComparison,
    };
    use crate::isolation::HarnessError;
    use mtgml_decision::{DecisionResponseV2, DECISION_RESPONSE_V2_SCHEMA};
    use mtgml_environment::{
        ControllerError, PlayerEndpoint, PlayerEndpointHandle, TrustedEnvironmentController,
    };
    use mtgml_model::{CandidateIdV1, PlayerDecisionIdV1, StateRevision};
    use mtgml_observation::{PlayerStepSubmissionV1, PlayerSubmissionCodeV1};
    use mtgml_replay::AuthoritativeReplayV3;
    use mtgml_wire::encode_canonical;

    type ForkPair = (
        TrustedEnvironmentController,
        [PlayerEndpointHandle; 2],
        TrustedEnvironmentController,
        [PlayerEndpointHandle; 2],
    );

    /// Shared builder: forks ONE runtime-accepted scenario instant into two
    /// structurally separate controllers with independent bound endpoints.
    fn fork_pair(scenario: ParityScenario) -> Result<ForkPair, HarnessError> {
        let (source_controller, source_handles) = match scenario {
            ParityScenario::DecisionRich => decision_rich_spawned()?,
            ParityScenario::InformationRich => information_rich_spawned()?,
        };
        let fork_controller = source_controller
            .fork()
            .map_err(|_| HarnessError::ControllerService)?;
        let fork_handles = [
            fork_controller
                .bind_player(P1)
                .map_err(|_| HarnessError::BindFailed)?,
            fork_controller
                .bind_player(P2)
                .map_err(|_| HarnessError::BindFailed)?,
        ];
        Ok((
            source_controller,
            source_handles,
            fork_controller,
            fork_handles,
        ))
    }

    fn controller_service(_: ControllerError) -> HarnessError {
        HarnessError::ControllerService
    }

    /// A closed-default plausible response referencing nothing foreign; the
    /// identical legal input class at requestless information-rich instants.
    fn plausible_response() -> DecisionResponseV2 {
        DecisionResponseV2 {
            schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
            player_decision_id: PlayerDecisionIdV1(1),
            state_revision: StateRevision(0),
            answer: mtgml_decision::DecisionAnswerV2::SelectOne {
                candidate_id: CandidateIdV1(0),
            },
        }
    }

    /// Pre-divergence equality plus an equal-input phase: the identical
    /// accepted ChooseCount answer through source-P1 and fork-P1 yields
    /// byte-equal steps and successor fingerprints, each side recording its
    /// one step atop a fresh segment anchored at the shared fork-time
    /// identity.
    #[test]
    fn fork_decision_rich() -> Result<(), HarnessError> {
        let (source, source_h, fork, fork_h) = fork_pair(ParityScenario::DecisionRich)?;
        let source_pre = capture_complete(&source, &source_h)?;
        let origin_cp = source.checkpoint().map_err(controller_service)?;

        // Pre-divergence: the fork holds exactly the source instant; its
        // recorder restarted as a fresh empty segment anchored at the
        // fork-time identity.
        let fork_initial = capture_complete(&fork, &fork_h)?;
        assert_fingerprint_policies(
            &source_pre,
            &fork_initial,
            FingerprintComparison::ExcludeReplayRecorder,
        )?;
        let fork_exported: AuthoritativeReplayV3 =
            fork.export_replay().map_err(controller_service)?;
        assert!(fork_exported.steps.is_empty());
        assert_segment_anchor(&fork_exported.manifest.initial_identity, &origin_cp);

        // Equal-input phase: ONE identical next input on both sides.
        let source_request = visible_request(&source_h[0])?;
        let fork_request = visible_request(&fork_h[0])?;
        assert_eq!(
            encode_canonical(&source_request).map_err(|_| HarnessError::WireEncoding)?,
            encode_canonical(&fork_request).map_err(|_| HarnessError::WireEncoding)?,
            "twins must expose identical pending requests pre-divergence"
        );
        let response = choose_count_answer(&source_request, EQUAL_COUNT_VALUE)?;
        let step_source = source_h[0]
            .submit(response.clone())
            .map_err(|_| HarnessError::EndpointService)?;
        let step_fork = fork_h[0]
            .submit(response)
            .map_err(|_| HarnessError::EndpointService)?;
        assert_eq!(
            step_source.submission, step_fork.submission,
            "the identical input must classify identically"
        );
        assert_eq!(step_source.submission, PlayerStepSubmissionV1::Accepted);
        let product_source = capture_transition_product(Ok(step_source))?;
        let product_fork = capture_transition_product(Ok(step_fork))?;
        assert_eq!(product_source, product_fork, "byte-equal steps required");

        let successor_source = capture_complete(&source, &source_h)?;
        let successor_fork = capture_complete(&fork, &fork_h)?;
        assert_fingerprint_policies(
            &successor_source,
            &successor_fork,
            FingerprintComparison::ExcludeReplayRecorder,
        )?;
        // Fresh-anchor check on the fork's fresh-seeded segment: exactly one
        // appended step atop the segment anchored at the shared fork-time
        // identity. The source keeps its spawn-seeded segment, so it holds
        // its pre-fork entry step plus the identical-input count step.
        let fork_exported_after: AuthoritativeReplayV3 =
            fork.export_replay().map_err(controller_service)?;
        assert_eq!(fork_exported_after.steps.len(), 1, "fork segment");
        assert_segment_anchor(&fork_exported_after.manifest.initial_identity, &origin_cp);
        let source_exported_after: AuthoritativeReplayV3 =
            source.export_replay().map_err(controller_service)?;
        assert_eq!(source_exported_after.steps.len(), 2, "source segment");
        Ok(())
    }

    /// Divergence is attributable ONLY to input streams: with zero further
    /// inputs the fork stays fingerprint-equal to the frozen source; once
    /// the fork consumes a different valid answer, revision and counters
    /// diverge between the two environments forked from one instant while
    /// the source remains byte-identical to its own pre-fork capture under
    /// the complete `All` policy.
    #[test]
    fn divergence_only_from_inputs() -> Result<(), HarnessError> {
        let (source, source_h, fork, fork_h) = fork_pair(ParityScenario::DecisionRich)?;
        let source_pre = capture_complete(&source, &source_h)?;

        // Zero-input phase: forking alone changes nothing observable.
        let fork_initial = capture_complete(&fork, &fork_h)?;
        assert_fingerprint_policies(
            &source_pre,
            &fork_initial,
            FingerprintComparison::ExcludeReplayRecorder,
        )?;

        // The fork consumes a DIFFERENT valid answer than the canonical
        // count; the source receives no further input at all.
        let fork_request = visible_request(&fork_h[0])?;
        let step_fork = fork_h[0]
            .submit(choose_count_answer(&fork_request, DIVERGENT_COUNT_VALUE)?)
            .map_err(|_| HarnessError::EndpointService)?;
        assert_eq!(step_fork.submission, PlayerStepSubmissionV1::Accepted);
        let fork_after = capture_complete(&fork, &fork_h)?;

        assert_ne!(
            fork_after.semantic.revision, source_pre.semantic.revision,
            "revisions must diverge once only the fork consumed an input"
        );
        assert_ne!(
            fork_after.environment.limit_counters, source_pre.environment.limit_counters,
            "counters must diverge once only the fork consumed an input"
        );
        assert_ne!(fork_after.semantic, source_pre.semantic);

        // The SOURCE is byte-frozen versus its own pre-fork capture,
        // recorder included.
        let source_after = capture_complete(&source, &source_h)?;
        assert_fingerprint_policies(&source_pre, &source_after, FingerprintComparison::All)?;
        Ok(())
    }

    /// After EACH fork-side mutation — one accepted transition, one rejected
    /// submission, and a restore of an older fork-side checkpoint — the
    /// SOURCE's COMPLETE fingerprint equals its pre-fork capture under the
    /// `All` policy: knowledge, identity maps, retired sets, visible
    /// sequences, continuations, RNG cursors, counters, and the replay
    /// recorder are all covered by the four groups.
    #[test]
    fn cross_mutation_isolation_matrix() -> Result<(), HarnessError> {
        let (source, source_h, fork, fork_h) = fork_pair(ParityScenario::DecisionRich)?;
        let source_pre = capture_complete(&source, &source_h)?;
        let fork_initial = capture_complete(&fork, &fork_h)?;
        let older_fork_cp = fork.checkpoint().map_err(controller_service)?;

        let source_unchanged = |context: &'static str| -> Result<(), HarnessError> {
            let current = capture_complete(&source, &source_h)?;
            assert_fingerprint_policies(&source_pre, &current, FingerprintComparison::All)
                .unwrap_or_else(|error| panic!("source mutated {context}: {error:?}"));
            Ok(())
        };

        // (a) One accepted transition on the fork.
        let fork_request = visible_request(&fork_h[0])?;
        let step_a = fork_h[0]
            .submit(choose_count_answer(&fork_request, EQUAL_COUNT_VALUE)?)
            .map_err(|_| HarnessError::EndpointService)?;
        assert_eq!(step_a.submission, PlayerStepSubmissionV1::Accepted);
        source_unchanged("after the fork's accepted transition")?;

        // (b) One rejected submission on the fork: well-formed but carrying
        // a stale player-decision identity, classified without mutation.
        let stale_request = visible_request(&fork_h[0])?;
        let step_b = fork_h[0]
            .submit(stale_members_answer(&stale_request)?)
            .map_err(|_| HarnessError::EndpointService)?;
        assert_eq!(
            step_b.submission,
            PlayerStepSubmissionV1::Rejected {
                code: PlayerSubmissionCodeV1::StaleDecision
            }
        );
        source_unchanged("after the fork's rejected submission")?;

        // (c) A restore of the older fork-side checkpoint.
        fork.restore(older_fork_cp).map_err(controller_service)?;
        source_unchanged("after the fork-side restore")?;

        // The fork itself returned to its pre-mutation instant; its recorder
        // restarted fresh at the restored identity.
        let fork_restored = capture_complete(&fork, &fork_h)?;
        assert_fingerprint_policies(
            &fork_initial,
            &fork_restored,
            FingerprintComparison::ExcludeReplayRecorder,
        )?;
        Ok(())
    }

    /// The information-rich scenario under the same protocol: pre-divergence
    /// equality with a fresh anchored fork segment, then an identical input
    /// phase whose closed typed rejections still produce byte-equal steps
    /// and successor fingerprints over the rich retained-knowledge
    /// projections of both perspectives.
    #[test]
    fn fork_information_rich() -> Result<(), HarnessError> {
        let (source, source_h, fork, fork_h) = fork_pair(ParityScenario::InformationRich)?;
        let source_pre = capture_complete(&source, &source_h)?;
        let origin_cp = source.checkpoint().map_err(controller_service)?;

        // Non-vacuous richness at the visible boundary: both cursors moved
        // past their construction-time defaults and projections differ.
        assert!(source_pre.player.p1_snapshot.current_visible_sequence.0 > 1);
        assert!(source_pre.player.p2_snapshot.current_visible_sequence.0 > 1);
        assert_ne!(
            source_pre.player.p1_snapshot.information_state_bytes,
            source_pre.player.p2_snapshot.information_state_bytes
        );

        // Pre-divergence: exact instant parity outside the restarted recorder.
        let fork_initial = capture_complete(&fork, &fork_h)?;
        assert_fingerprint_policies(
            &source_pre,
            &fork_initial,
            FingerprintComparison::ExcludeReplayRecorder,
        )?;
        let fork_exported: AuthoritativeReplayV3 =
            fork.export_replay().map_err(controller_service)?;
        assert!(fork_exported.steps.is_empty());
        assert_segment_anchor(&fork_exported.manifest.initial_identity, &origin_cp);

        // Equal-input phase: ONE identical input on both sides. At these
        // requestless instants it classifies as the closed typed rejection;
        // rejections append no replay segment on either side.
        let response = plausible_response();
        let step_source = source_h[0]
            .submit(response.clone())
            .map_err(|_| HarnessError::EndpointService)?;
        let step_fork = fork_h[0]
            .submit(response)
            .map_err(|_| HarnessError::EndpointService)?;
        assert_eq!(
            step_source.submission, step_fork.submission,
            "the identical input must classify identically"
        );
        assert_eq!(
            step_source.submission,
            PlayerStepSubmissionV1::Rejected {
                code: PlayerSubmissionCodeV1::UnavailableDecision
            }
        );
        let product_source = capture_transition_product(Ok(step_source))?;
        let product_fork = capture_transition_product(Ok(step_fork))?;
        assert_eq!(product_source, product_fork, "byte-equal steps required");

        let successor_source = capture_complete(&source, &source_h)?;
        let successor_fork = capture_complete(&fork, &fork_h)?;
        assert_fingerprint_policies(
            &successor_source,
            &successor_fork,
            FingerprintComparison::ExcludeReplayRecorder,
        )?;
        for (label, controller) in [("source", &source), ("fork", &fork)] {
            let exported: AuthoritativeReplayV3 =
                controller.export_replay().map_err(controller_service)?;
            assert!(
                exported.steps.is_empty(),
                "{label} rejections must not append replay segments"
            );
            assert_segment_anchor(&exported.manifest.initial_identity, &origin_cp);
        }
        Ok(())
    }
}
