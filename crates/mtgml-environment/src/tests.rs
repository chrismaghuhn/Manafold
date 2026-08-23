use super::*;
use mtgml_decision::{DecisionAnswerV2, DecisionResponseV2, DECISION_RESPONSE_V2_SCHEMA};
use mtgml_model::{
    CandidateIdV1, CheckpointDigestV3, ContentDigest, ContinuationId, EpisodeStatus,
    FullStateDigestV3, PlayerDecisionIdV1, PlayerId, StateRevision, TerminalReason,
    TruncationReason,
};
use mtgml_observation::{
    PlayerStepV2, INFORMATION_STATE_SCHEMA_V2, OBSERVATION_SCHEMA, OBSERVED_EVENT_SCHEMA_V2,
    PLAYER_STEP_SCHEMA_V2,
};
use mtgml_random::RootSeed256;
use mtgml_replay::{
    AuthoritativeReplayV3, DeckIdentityV1, KernelIdentityV1, ReplaySchemaVersionsV1,
};

fn config(players: [PlayerId; 2]) -> SyntheticM1EnvironmentConfig {
    SyntheticM1EnvironmentConfig {
        codec: CheckpointCodecIdentity {
            codec_id: "synthetic-m2-memory".into(),
            semantic_version: "3".into(),
        },
        replay: SyntheticM1ReplayConfig {
            engine_build: "synthetic-build".into(),
            kernel: KernelIdentityV1 {
                implementation_id: "synthetic-m2".into(),
                semantic_version: "0.2.2".into(),
                build_profile: "test".into(),
            },
            rules_snapshot: "synthetic-rules".into(),
            format_policy_snapshot: "synthetic-format".into(),
            oracle_snapshot: "synthetic-oracle".into(),
            card_bundle: "synthetic-bundle".into(),
            randomness_contract_id: "mtgml.rng.v1".into(),
            schemas: ReplaySchemaVersionsV1 {
                observation: OBSERVATION_SCHEMA.into(),
                information_state: INFORMATION_STATE_SCHEMA_V2.into(),
                decision: "player-decision-request.v2".into(),
                decision_response: DECISION_RESPONSE_V2_SCHEMA.into(),
                observed_event: OBSERVED_EVENT_SCHEMA_V2.into(),
                player_step: PLAYER_STEP_SCHEMA_V2.into(),
                replay_step: "replay-step.v3".into(),
            },
            decks: players
                .into_iter()
                .enumerate()
                .map(|(index, player)| DeckIdentityV1 {
                    player,
                    deck_id: format!("synthetic-deck-{}", index + 1),
                    digest: ContentDigest::from_canonical_bytes(
                        format!("synthetic-deck-{}", index + 1).as_bytes(),
                    ),
                })
                .collect(),
        },
    }
}

fn seed() -> RootSeed256 {
    RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap()
}

fn response(candidate_id: u32, revision: u64) -> DecisionResponseV2 {
    DecisionResponseV2 {
        schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
        player_decision_id: PlayerDecisionIdV1(1),
        state_revision: StateRevision(revision),
        answer: DecisionAnswerV2::SelectOne {
            candidate_id: CandidateIdV1(candidate_id),
        },
    }
}

fn backend() -> SyntheticM1EnvironmentBackend {
    let players = [PlayerId(1), PlayerId(2)];
    SyntheticM1EnvironmentBackend::new(players, seed(), config(players)).unwrap()
}

#[test]
fn checkpoint_v3_validation_and_restore_nonmutation_matrix() {
    let checkpoint = backend().checkpoint().unwrap();
    checkpoint.validate().unwrap();
    assert_eq!(checkpoint.schema_version, ENVIRONMENT_CHECKPOINT_SCHEMA);
    assert_eq!(checkpoint.state_digest, checkpoint.state.digest().unwrap());
    assert_eq!(checkpoint.state_digest.raw_bytes().len(), 32);
    assert!(!checkpoint.codec.codec_id.is_empty());
    assert!(!checkpoint.codec.semantic_version.is_empty());

    // Corrupting any authoritative checkpoint field must be rejected.
    let corrupt_state_digest = |mutate: fn(&mut EnvironmentCheckpointV3)| {
        let mut corrupted = backend().checkpoint().unwrap();
        mutate(&mut corrupted);
        corrupted.validate().is_err()
    };
    assert!(corrupt_state_digest(|c| {
        c.state_digest = FullStateDigestV3::from_digest_bytes([0xff; 32]);
    }));
    assert!(corrupt_state_digest(|c| {
        c.checkpoint_digest = CheckpointDigestV3::from_digest_bytes([0xee; 32]);
    }));
    assert!(corrupt_state_digest(|c| {
        c.status = EpisodeStatus::Terminal {
            reason: TerminalReason::Concession,
            players: vec![],
        };
    }));
    assert!(corrupt_state_digest(|c| {
        c.limit_counters.accepted_transitions += 1;
    }));
    assert!(corrupt_state_digest(|c| {
        c.limit_counters.decisions_submitted += 1;
    }));
    assert!(corrupt_state_digest(|c| {
        c.codec.codec_id = "unsupported-codec".into();
    }));
    assert!(corrupt_state_digest(|c| {
        c.codec.semantic_version.clear();
    }));
    assert!(corrupt_state_digest(|c| {
        c.schema_version = "environment-checkpoint.v2".into();
    }));

    // Restoring a corrupted checkpoint must leave the backend untouched.
    let backend_controller = TrustedEnvironmentController::new(backend());
    let before_checkpoint = backend_controller.checkpoint().unwrap();
    let before_replay = backend_controller.export_replay().unwrap();
    let mut corrupted = before_checkpoint.clone();
    corrupted.limit_counters.decisions_submitted += 1;
    assert!(backend_controller.restore(corrupted).is_err());
    let mut corrupted = before_checkpoint.clone();
    corrupted.status = EpisodeStatus::Truncated {
        reason: TruncationReason::ExternalStop,
        players: vec![],
    };
    // The digest no longer matches, so restore must fail closed.
    assert!(backend_controller.restore(corrupted).is_err());
    let mut corrupted = before_checkpoint.clone();
    corrupted.codec = CheckpointCodecIdentity {
        codec_id: "other".into(),
        semantic_version: "9".into(),
    };
    assert!(backend_controller.restore(corrupted).is_err());
    assert_eq!(
        backend_controller.checkpoint().unwrap(),
        before_checkpoint,
        "failed restores must not mutate the backend"
    );
    assert_eq!(backend_controller.export_replay().unwrap(), before_replay);

    // A valid restore reproduces exact identity and rebases replay.
    let source = TrustedEnvironmentController::new(backend());
    source
        .execute_trusted_response(PlayerId(1), response(0, 0))
        .unwrap();
    let advanced = source.checkpoint().unwrap();
    let target = TrustedEnvironmentController::new(backend());
    target.restore(advanced.clone()).unwrap();
    assert_eq!(target.checkpoint().unwrap(), advanced);
    let rebased = target.export_replay().unwrap();
    assert!(rebased.steps.is_empty());
    assert_eq!(
        rebased.manifest.initial_identity.checkpoint_digest,
        advanced.checkpoint_digest
    );
}

#[test]
fn synthetic_endpoint_returns_v2_surface() {
    let controller = TrustedEnvironmentController::new(backend());
    let p1 = controller.bind_player(PlayerId(1)).unwrap();
    let p2 = controller.bind_player(PlayerId(2)).unwrap();

    let visible = p1.visible_decision().unwrap().unwrap();
    visible.validate().unwrap();
    assert_eq!(visible.schema_version, "player-decision-request.v2");
    assert!(p2.visible_decision().unwrap().is_none());

    let information = p1.information_state().unwrap();
    information.validate().unwrap();
    let (_, digest) =
        mtgml_wire::compute_information_state_digest_v2(&information.digest_input()).unwrap();
    assert_eq!(information.digest, digest);
}

#[test]
fn accepted_endpoint_submission_commits_v3_state_delta_and_replay() {
    let controller = TrustedEnvironmentController::new(backend());
    let before = controller.checkpoint().unwrap();
    let p1 = controller.bind_player(PlayerId(1)).unwrap();

    let step = p1.submit(response(0, 0)).unwrap();
    step.validate().unwrap();
    assert_eq!(step.schema_version, PLAYER_STEP_SCHEMA_V2);
    assert_eq!(step.information_state.state_revision, StateRevision(1));
    // The entry acceptance creates the continuation and exposes stage 0.
    assert!(step.next_decision.is_some());
    assert_eq!(
        step.next_decision.as_ref().unwrap().decision,
        mtgml_decision::DecisionDomainV2::ChooseNumber {
            minimum: 0,
            maximum: 3
        }
    );

    let after = controller.checkpoint().unwrap();
    assert_eq!(after.state.revision, StateRevision(1));
    assert_eq!(after.state.core.players[&PlayerId(1)].life, 38);
    assert_eq!(after.limit_counters.decisions_submitted, 1);
    assert_eq!(after.limit_counters.accepted_transitions, 1);
    assert_eq!(after.state_digest, after.state.digest().unwrap());

    let replay = controller.export_replay().unwrap();
    replay.validate().unwrap();
    assert_eq!(replay.steps.len(), 1);
    assert_eq!(
        replay.steps[0].checkpoint_digest_before,
        before.checkpoint_digest
    );
    assert_eq!(replay.steps[0].full_state_digest_after, after.state_digest);
    assert_eq!(replay.final_identity.full_state_digest, after.state_digest);
    let bytes = mtgml_wire::encode_canonical(&replay).unwrap();
    let decoded: AuthoritativeReplayV3 = mtgml_wire::decode_canonical(&bytes).unwrap();
    assert_eq!(decoded, replay);

    // Drive the remaining stages through the bound endpoint.
    let stage0 = p1.visible_decision().unwrap().unwrap();
    let count_step = p1
        .submit(mtgml_decision::DecisionResponseV2 {
            schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
            player_decision_id: stage0.player_decision_id,
            state_revision: stage0.state_revision,
            answer: mtgml_decision::DecisionAnswerV2::ChooseNumber { value: 2 },
        })
        .unwrap();
    count_step.validate().unwrap();
    let stage1 = p1.visible_decision().unwrap().unwrap();
    assert_eq!(
        stage1.decision,
        mtgml_decision::DecisionDomainV2::ChooseMany {
            minimum: 2,
            maximum: 2
        }
    );
    let members_step = p1
        .submit(mtgml_decision::DecisionResponseV2 {
            schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
            player_decision_id: stage1.player_decision_id,
            state_revision: stage1.state_revision,
            answer: mtgml_decision::DecisionAnswerV2::SelectMany {
                candidate_ids: vec![CandidateIdV1(0), CandidateIdV1(1)],
            },
        })
        .unwrap();
    members_step.validate().unwrap();
    let stage2 = p1.visible_decision().unwrap().unwrap();
    let order_step = p1
        .submit(mtgml_decision::DecisionResponseV2 {
            schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
            player_decision_id: stage2.player_decision_id,
            state_revision: stage2.state_revision,
            answer: mtgml_decision::DecisionAnswerV2::Order {
                candidate_ids: vec![CandidateIdV1(1), CandidateIdV1(0)],
            },
        })
        .unwrap();
    order_step.validate().unwrap();
    // Completion removes the continuation and clears pending decisions.
    assert!(p1.visible_decision().unwrap().is_none());
    let completed = controller.checkpoint().unwrap();
    assert!(completed.state.execution.continuations.is_empty());
    assert_eq!(controller.export_replay().unwrap().steps.len(), 4);
}

#[test]
fn rejected_submission_preserves_outer_environment_identity() {
    let controller = TrustedEnvironmentController::new(backend());
    let before_checkpoint = controller.checkpoint().unwrap();
    let before_replay = controller.export_replay().unwrap();
    let rejected = controller
        .execute_trusted_response(PlayerId(1), response(1, 0))
        .unwrap();

    assert!(!rejected.accepted);
    assert_eq!(controller.checkpoint().unwrap(), before_checkpoint);
    assert_eq!(controller.export_replay().unwrap(), before_replay);
}

#[test]
fn checkpoint_restore_and_fork_are_exact_and_rebase_replay() {
    let source = TrustedEnvironmentController::new(backend());
    let c0 = source.checkpoint().unwrap();
    let fork_a = source.fork().unwrap();
    let fork_b = source.fork().unwrap();
    let transition_a = fork_a
        .execute_trusted_response(PlayerId(1), response(0, 0))
        .unwrap();
    let transition_b = fork_b
        .execute_trusted_response(PlayerId(1), response(0, 0))
        .unwrap();

    assert_eq!(transition_a, transition_b);
    assert_eq!(fork_a.checkpoint().unwrap(), fork_b.checkpoint().unwrap());
    assert_eq!(
        fork_a.export_replay().unwrap(),
        fork_b.export_replay().unwrap()
    );
    assert_eq!(source.checkpoint().unwrap(), c0);

    source.restore(fork_a.checkpoint().unwrap()).unwrap();
    let rebased = source.export_replay().unwrap();
    assert!(rebased.steps.is_empty());
    assert_eq!(
        rebased.manifest.initial_identity.state_revision,
        StateRevision(1)
    );
    assert_eq!(rebased.final_identity.state_revision, StateRevision(1));
}

#[test]
fn semantic_replay_reproduces_the_authoritative_transition() {
    let controller = TrustedEnvironmentController::new(backend());
    let c0 = controller.checkpoint().unwrap();
    let live = controller
        .execute_trusted_response(PlayerId(1), response(0, 0))
        .unwrap();
    let after = controller.checkpoint().unwrap();
    let replay = controller.export_replay().unwrap();

    let report = controller
        .execute_replay_from_checkpoint(c0, replay)
        .unwrap();
    assert_eq!(report.traces.len(), 1);
    assert_eq!(report.traces[0].transition, live);
    assert_eq!(report.traces[0].after, after);
    assert_eq!(report.final_checkpoint, after);
}

#[test]
fn stale_endpoint_response_is_rejected_without_mutation() {
    let controller = TrustedEnvironmentController::new(backend());
    let p1 = controller.bind_player(PlayerId(1)).unwrap();
    let before = controller.checkpoint().unwrap();
    assert_eq!(
        p1.submit(response(0, 1)).unwrap_err(),
        PlayerApiError::StaleResponse
    );
    assert_eq!(controller.checkpoint().unwrap(), before);
}

#[test]
fn checkpoint_identity_tampering_is_rejected() {
    let mut checkpoint = backend().checkpoint().unwrap();
    checkpoint.state_digest = FullStateDigestV3::from_digest_bytes([0xff; 32]);
    assert_eq!(
        checkpoint.validate().unwrap_err(),
        CheckpointValidationError::StateDigest
    );
}

#[test]
fn information_state_orders_active_and_retired_knowledge_jointly() {
    use mtgml_model::VisibleSequence;
    use mtgml_observation::PlayerKnownObjectV1;
    use mtgml_state::{
        KnowledgeAcquisitionCause, KnowledgeAcquisitionReason, KnowledgeHistoryChannel,
        KnowledgeInvalidationReason, KnowledgeInvalidationV2, KnownLocationFactV2,
        RetiredKnowledgeRecordV2,
    };
    let observed = |channel, sequence: u64, cause| KnowledgeAcquisitionReason::Observed {
        channel,
        sequence: VisibleSequence(sequence),
        cause,
    };

    let mut state =
        mtgml_state::construct_synthetic_engine_state(mtgml_state::SyntheticResetInputs {
            players: [PlayerId(1), PlayerId(2)],
            root_seed: seed(),
        })
        .unwrap();

    // Interleave a retired record between two active records for player 1.
    let identity = state
        .perspective_identities
        .players
        .get_mut(&PlayerId(1))
        .unwrap();
    identity
        .opaque_to_object
        .insert(mtgml_model::OpaqueObjectId(3), mtgml_model::GameObjectId(2));
    identity
        .object_to_opaque
        .insert(mtgml_model::GameObjectId(2), mtgml_model::OpaqueObjectId(3));
    identity.next_opaque_object_id = mtgml_model::OpaqueObjectId(4);
    identity
        .retired_object_ids
        .insert(mtgml_model::OpaqueObjectId(2));

    let knowledge = state.knowledge.players.get_mut(&PlayerId(1)).unwrap();
    let hidden_location = mtgml_state::ZoneLocation {
        zone: mtgml_model::ZoneKind::Library,
        player: Some(PlayerId(2)),
        position: mtgml_state::ZonePosition::Top { offset: 0 },
        visibility: mtgml_state::VisibilityPartition::FaceDown,
        partition: None,
    };
    knowledge.retired.insert(
        mtgml_model::OpaqueObjectId(2),
        RetiredKnowledgeRecordV2 {
            opaque_object: mtgml_model::OpaqueObjectId(2),
            physical_card: None,
            card_definition: None,
            last_known_location: None,
            historical_locations: Vec::new(),
            acquisition: KnowledgeAcquisitionReason::InitialConfiguration,
            invalidation: KnowledgeInvalidationV2 {
                provenance: observed(
                    KnowledgeHistoryChannel::Public,
                    0,
                    KnowledgeAcquisitionCause::ExplicitReveal,
                ),
                reason: KnowledgeInvalidationReason::Shuffle,
            },
        },
    );
    knowledge.active.remove(&mtgml_model::OpaqueObjectId(2));
    knowledge.active.insert(
        mtgml_model::OpaqueObjectId(3),
        mtgml_state::KnowledgeRecordV2 {
            opaque_object: mtgml_model::OpaqueObjectId(3),
            physical_card: None,
            card_definition: Some(mtgml_model::CardDefinitionId(2)),
            known_location: Some(KnownLocationFactV2 {
                location: hidden_location,
                provenance: observed(
                    KnowledgeHistoryChannel::Public,
                    0,
                    KnowledgeAcquisitionCause::PublicEvent,
                ),
            }),
            acquisition: observed(
                KnowledgeHistoryChannel::Public,
                0,
                KnowledgeAcquisitionCause::PublicEvent,
            ),
            historical_locations: Vec::new(),
        },
    );

    let checkpoint = EnvironmentCheckpointV3::new(
        state.clone(),
        EpisodeStatus::Running,
        EnvironmentLimitCounters::default(),
        CheckpointCodecIdentity {
            codec_id: "synthetic-m2-memory".into(),
            semantic_version: "3".into(),
        },
    )
    .unwrap();
    let controller = TrustedEnvironmentController::new(
        SyntheticM1EnvironmentBackend::from_checkpoint(
            checkpoint,
            config([PlayerId(1), PlayerId(2)]),
        )
        .unwrap(),
    );
    let endpoint = controller.bind_player(PlayerId(1)).unwrap();
    let information = endpoint.information_state().unwrap();
    information.validate().unwrap();
    let ids: Vec<u64> = information
        .retained_knowledge
        .iter()
        .map(|record| match record {
            PlayerKnownObjectV1::Active {
                opaque_object_id, ..
            }
            | PlayerKnownObjectV1::Retired {
                opaque_object_id, ..
            } => opaque_object_id.0,
        })
        .collect();
    assert_eq!(ids, vec![1, 2, 3]);
}

#[test]
fn checkpoint_restore_repeats_exact_transition_and_replay_segment() {
    let source = TrustedEnvironmentController::new(backend());
    let initial = source.checkpoint().unwrap();
    let live = source
        .execute_trusted_response(PlayerId(1), response(0, 0))
        .unwrap();
    let live_after = source.checkpoint().unwrap();
    let live_replay = source.export_replay().unwrap();

    let restored = TrustedEnvironmentController::new(backend());
    restored.restore(initial).unwrap();
    let repeated = restored
        .execute_trusted_response(PlayerId(1), response(0, 0))
        .unwrap();
    assert_eq!(repeated, live);
    assert_eq!(restored.checkpoint().unwrap(), live_after);
    assert_eq!(restored.export_replay().unwrap(), live_replay);
}

#[test]
fn forks_diverge_only_on_explicit_input() {
    let source = TrustedEnvironmentController::new(backend());
    let before_fork = source.checkpoint().unwrap();

    let accepted_fork = source.fork().unwrap();
    let rejected_fork = source.fork().unwrap();
    assert_eq!(accepted_fork.checkpoint().unwrap(), before_fork);

    // A rejected diagnostic input leaves the fork at the shared identity.
    rejected_fork
        .execute_trusted_response(PlayerId(1), response(1, 0))
        .unwrap();
    assert_eq!(rejected_fork.checkpoint().unwrap(), before_fork);

    // Only an explicitly accepted input diverges the fork.
    let transition = accepted_fork
        .execute_trusted_response(PlayerId(1), response(0, 0))
        .unwrap();
    assert!(transition.accepted);
    let diverged = accepted_fork.checkpoint().unwrap();
    assert_ne!(diverged, before_fork);
    assert_eq!(
        source.checkpoint().unwrap(),
        before_fork,
        "the source must not observe fork inputs"
    );
}

#[test]
fn semantic_replay_rejects_tampered_identity_without_live_mutation() {
    use mtgml_persistence::checkpoint_digest::calculate_checkpoint_digest_v3;
    use mtgml_replay::InitialEnvironmentIdentityV3 as Identity;

    fn recompute(identity: &Identity) -> Identity {
        let mut fixed = identity.clone();
        fixed.checkpoint_digest = calculate_checkpoint_digest_v3(
            &fixed.full_state_digest.as_digest_reference(),
            &fixed.episode_status,
            &fixed.environment_limit_counters,
            &fixed.checkpoint_codec_identity,
        )
        .unwrap();
        fixed
    }

    let controller = TrustedEnvironmentController::new(backend());
    let c0 = controller.checkpoint().unwrap();
    controller
        .execute_trusted_response(PlayerId(1), response(0, 0))
        .unwrap();
    let after = controller.checkpoint().unwrap();
    let live_replay = controller.export_replay().unwrap();
    let live_counters = after.limit_counters.clone();

    let run = |replay: AuthoritativeReplayV3| {
        let fresh = TrustedEnvironmentController::new(backend());
        fresh.execute_replay_from_checkpoint(c0.clone(), replay)
    };

    // Wrong actor is rejected before execution.
    let mut tampered = live_replay.clone();
    tampered.steps[0].actor = PlayerId(2);
    assert!(matches!(
        run(tampered),
        Err(ControllerError::ReplayExecution(
            ReplayExecutionError::ActorUnavailable { step_index: 0 }
        ))
    ));

    // A recorded counter divergence is rejected against the recomputed product.
    let mut tampered = live_replay.clone();
    tampered.steps[0]
        .environment_limit_counters_after
        .rule_events_emitted += 2;
    tampered.steps[0].checkpoint_digest_after = {
        let identity = Identity {
            state_revision: tampered.steps[0].state_revision_after,
            full_state_digest: tampered.steps[0].full_state_digest_after.clone(),
            episode_status: tampered.steps[0].episode_status_after.clone(),
            environment_limit_counters: tampered.steps[0].environment_limit_counters_after.clone(),
            checkpoint_codec_identity: tampered
                .manifest
                .initial_identity
                .checkpoint_codec_identity
                .clone(),
            checkpoint_digest: mtgml_model::CheckpointDigestV3::from_digest_bytes([0; 32]),
        };
        let identity = recompute(&identity);
        tampered.final_identity = identity.clone();
        identity.checkpoint_digest
    };
    // The recorded counter divergence surfaces as a full after-identity
    // mismatch against the deterministically re-executed checkpoint.
    assert!(matches!(
        run(tampered),
        Err(ControllerError::ReplayExecution(
            ReplayExecutionError::AfterDigestMismatch { step_index: 0 }
        ))
    ));

    // A wrong final full-state digest is rejected after execution.
    let mut tampered = live_replay.clone();
    tampered.steps[0].full_state_digest_after = FullStateDigestV3::from_digest_bytes([7; 32]);
    let identity = Identity {
        state_revision: tampered.steps[0].state_revision_after,
        full_state_digest: tampered.steps[0].full_state_digest_after.clone(),
        episode_status: tampered.steps[0].episode_status_after.clone(),
        environment_limit_counters: tampered.steps[0].environment_limit_counters_after.clone(),
        checkpoint_codec_identity: tampered
            .manifest
            .initial_identity
            .checkpoint_codec_identity
            .clone(),
        checkpoint_digest: mtgml_model::CheckpointDigestV3::from_digest_bytes([0; 32]),
    };
    let identity = recompute(&identity);
    tampered.steps[0].checkpoint_digest_after = identity.checkpoint_digest.clone();
    tampered.final_identity = identity;
    assert!(matches!(
        run(tampered),
        Err(ControllerError::ReplayExecution(
            ReplayExecutionError::AfterDigestMismatch { step_index: 0 }
        ))
    ));

    // A different root seed cannot masquerade as the same environment.
    let mut tampered = live_replay.clone();
    tampered.manifest.randomness.root_seed_hex = "22".repeat(32);
    assert!(matches!(
        run(tampered),
        Err(ControllerError::ReplayExecution(
            ReplayExecutionError::ManifestMismatch
        ))
    ));

    // The live backend remains untouched by every failed replay attempt.
    assert_eq!(controller.checkpoint().unwrap(), after);
    assert_eq!(controller.export_replay().unwrap(), live_replay);
    assert_eq!(
        controller.checkpoint().unwrap().limit_counters,
        live_counters
    );
}

#[test]
fn multi_player_endpoints_remain_bound_through_visibility_and_submission() {
    let controller = TrustedEnvironmentController::new(backend());
    let p1 = controller.bind_player(PlayerId(1)).unwrap();
    let p2 = controller.bind_player(PlayerId(2)).unwrap();
    let before = controller.checkpoint().unwrap();

    // Player 2 does not own the visible decision.
    assert_eq!(
        p2.submit(response(0, 0)).unwrap_err(),
        PlayerApiError::NoVisibleDecision
    );
    assert!(p2.visible_decision().unwrap().is_none());
    assert_eq!(controller.checkpoint().unwrap(), before);

    // Both endpoints remain alive and bound to their own perspectives.
    assert!(p1.visible_decision().unwrap().is_some());
    assert_eq!(p1.information_state().unwrap().perspective, PlayerId(1));
    assert_eq!(p2.information_state().unwrap().perspective, PlayerId(2));

    // After player 1 commits, both endpoints project the advanced state;
    // the created continuation keeps a visible decision alive for p1.
    let step = p1.submit(response(0, 0)).unwrap();
    assert_eq!(step.information_state.perspective, PlayerId(1));
    assert_eq!(step.information_state.state_revision, StateRevision(1));
    assert!(p1.visible_decision().unwrap().is_some());
    assert_eq!(
        p2.information_state().unwrap().state_revision,
        StateRevision(1)
    );
}

#[test]
fn non_default_player_ids_remain_bound_through_submission() {
    let players = [PlayerId(7), PlayerId(9)];
    let controller = TrustedEnvironmentController::new(
        SyntheticM1EnvironmentBackend::new(players, seed(), config(players)).unwrap(),
    );
    let p7 = controller.bind_player(PlayerId(7)).unwrap();
    let step = p7.submit(response(0, 0)).unwrap();
    assert_eq!(step.information_state.perspective, PlayerId(7));
    assert_eq!(
        controller.checkpoint().unwrap().state.revision,
        StateRevision(1)
    );
}

#[test]
fn unknown_player_binding_is_rejected_without_backend_details() {
    let controller = TrustedEnvironmentController::new(backend());
    assert!(matches!(
        controller.bind_player(PlayerId(9)),
        Err(ControllerError::UnknownPlayer)
    ));
}

#[test]
fn player_api_errors_do_not_render_trusted_values() {
    let controller = TrustedEnvironmentController::new(backend());
    let p1 = controller.bind_player(PlayerId(1)).unwrap();
    let errors = vec![
        format!("{}", p1.submit(response(0, 5)).unwrap_err()),
        format!("{}", p1.submit(response(1, 0)).unwrap_err()),
        format!(
            "{}",
            match controller.bind_player(PlayerId(9)) {
                Ok(_) => "bound".to_string(),
                Err(error) => format!("{error}"),
            }
        ),
        format!("{}", p2_error_text(&controller)),
    ];
    for message in errors {
        // Evaluate to a bare boolean: no trusted-looking literal may reach a
        // log/format sink here (cf. CodeQL cleartext-logging).
        let renders_trusted_vocabulary = [
            "seed",
            "digest",
            "checkpoint",
            "continuation",
            "GameObject",
            "DecisionId",
        ]
        .iter()
        .any(|vocabulary| message.contains(vocabulary));
        assert!(
            !renders_trusted_vocabulary,
            "player API error must not render trusted values"
        );
    }
}

fn p2_error_text(controller: &TrustedEnvironmentController) -> String {
    match controller.bind_player(PlayerId(2)) {
        Ok(endpoint) => format!("{}", endpoint.submit(response(0, 0)).unwrap_err()),
        Err(error) => format!("{error}"),
    }
}

fn rich_provenance_state() -> mtgml_state::EngineState {
    use mtgml_model::VisibleSequence;
    use mtgml_state::{
        KnowledgeAcquisitionCause, KnowledgeAcquisitionReason, KnowledgeHistoryChannel,
        KnowledgeInvalidationReason, KnowledgeInvalidationV2, KnownLocationFactV2,
        RetiredKnowledgeRecordV2,
    };
    let observed = |channel, sequence: u64, cause| KnowledgeAcquisitionReason::Observed {
        channel,
        sequence: VisibleSequence(sequence),
        cause,
    };
    let mut state =
        mtgml_state::construct_synthetic_engine_state(mtgml_state::SyntheticResetInputs {
            players: [PlayerId(1), PlayerId(2)],
            root_seed: seed(),
        })
        .unwrap();

    let identity = state
        .perspective_identities
        .players
        .get_mut(&PlayerId(1))
        .unwrap();
    identity
        .opaque_to_object
        .insert(mtgml_model::OpaqueObjectId(3), mtgml_model::GameObjectId(2));
    identity
        .object_to_opaque
        .insert(mtgml_model::GameObjectId(2), mtgml_model::OpaqueObjectId(3));
    identity.next_opaque_object_id = mtgml_model::OpaqueObjectId(4);
    identity
        .retired_object_ids
        .insert(mtgml_model::OpaqueObjectId(2));

    let knowledge = state.knowledge.players.get_mut(&PlayerId(1)).unwrap();
    let hidden_location = mtgml_state::ZoneLocation {
        zone: mtgml_model::ZoneKind::Library,
        player: Some(PlayerId(2)),
        position: mtgml_state::ZonePosition::Top { offset: 0 },
        visibility: mtgml_state::VisibilityPartition::FaceDown,
        partition: None,
    };

    // Retired record: private_look acquisition, own_private_identity history,
    // explicit_reveal invalidation.
    let mut retired = RetiredKnowledgeRecordV2 {
        opaque_object: mtgml_model::OpaqueObjectId(2),
        physical_card: None,
        card_definition: None,
        last_known_location: Some(KnownLocationFactV2 {
            location: hidden_location.clone(),
            provenance: observed(
                KnowledgeHistoryChannel::Private,
                0,
                KnowledgeAcquisitionCause::PrivateLook,
            ),
        }),
        historical_locations: vec![KnownLocationFactV2 {
            location: hidden_location.clone(),
            provenance: observed(
                KnowledgeHistoryChannel::Private,
                0,
                KnowledgeAcquisitionCause::OwnPrivateIdentity,
            ),
        }],
        acquisition: observed(
            KnowledgeHistoryChannel::Private,
            0,
            KnowledgeAcquisitionCause::PrivateLook,
        ),
        invalidation: KnowledgeInvalidationV2 {
            provenance: observed(
                KnowledgeHistoryChannel::Public,
                0,
                KnowledgeAcquisitionCause::ExplicitReveal,
            ),
            reason: KnowledgeInvalidationReason::Shuffle,
        },
    };
    retired.last_known_location = Some(KnownLocationFactV2 {
        location: hidden_location.clone(),
        provenance: observed(
            KnowledgeHistoryChannel::Private,
            0,
            KnowledgeAcquisitionCause::PrivateLook,
        ),
    });
    knowledge
        .retired
        .insert(mtgml_model::OpaqueObjectId(2), retired);
    knowledge.active.remove(&mtgml_model::OpaqueObjectId(2));

    // Active record with explicit_reveal current-fact provenance.
    knowledge.active.insert(
        mtgml_model::OpaqueObjectId(3),
        mtgml_state::KnowledgeRecordV2 {
            opaque_object: mtgml_model::OpaqueObjectId(3),
            physical_card: None,
            card_definition: Some(mtgml_model::CardDefinitionId(2)),
            known_location: Some(KnownLocationFactV2 {
                location: hidden_location,
                provenance: observed(
                    KnowledgeHistoryChannel::Public,
                    0,
                    KnowledgeAcquisitionCause::ExplicitReveal,
                ),
            }),
            acquisition: observed(
                KnowledgeHistoryChannel::Public,
                0,
                KnowledgeAcquisitionCause::ExplicitReveal,
            ),
            historical_locations: Vec::new(),
        },
    );
    state
}

fn projected_provenance(
    information: &mtgml_observation::PlayerInformationStateV2,
) -> Vec<(u64, String)> {
    use mtgml_observation::{PlayerKnowledgeProvenanceV1, PlayerKnownObjectV1};
    fn render(provenance: &PlayerKnowledgeProvenanceV1) -> String {
        match provenance {
            PlayerKnowledgeProvenanceV1::InitialConfiguration => "initial_configuration".into(),
            PlayerKnowledgeProvenanceV1::Observed {
                channel,
                sequence,
                cause,
            } => format!("observed/{channel:?}/{}/{cause:?}", sequence.0),
        }
    }
    let mut rendered = Vec::new();
    for record in &information.retained_knowledge {
        match record {
            PlayerKnownObjectV1::Active {
                opaque_object_id,
                current_known_location_fact,
                historical_locations,
                acquisition,
                ..
            } => {
                if let Some(current) = current_known_location_fact {
                    rendered.push((
                        opaque_object_id.0,
                        format!("current/{}", render(&current.provenance)),
                    ));
                }
                for historical in historical_locations {
                    rendered.push((
                        opaque_object_id.0,
                        format!("historical/{}", render(&historical.provenance)),
                    ));
                }
                rendered.push((
                    opaque_object_id.0,
                    format!("acquisition/{}", render(acquisition)),
                ));
            }
            PlayerKnownObjectV1::Retired {
                opaque_object_id,
                last_known_location_fact,
                historical_locations,
                acquisition,
                invalidation,
                ..
            } => {
                if let Some(last) = last_known_location_fact {
                    rendered.push((
                        opaque_object_id.0,
                        format!("last/{}", render(&last.provenance)),
                    ));
                }
                for historical in historical_locations {
                    rendered.push((
                        opaque_object_id.0,
                        format!("historical/{}", render(&historical.provenance)),
                    ));
                }
                rendered.push((
                    opaque_object_id.0,
                    format!("acquisition/{}", render(acquisition)),
                ));
                let reason_text = format!("{:?}", invalidation.reason);
                rendered.push((
                    opaque_object_id.0,
                    format!(
                        "invalidation/{}/{}",
                        render(&invalidation.provenance),
                        reason_text
                    ),
                ));
            }
        }
    }
    rendered.sort();
    rendered
}

#[test]
fn provenance_is_preserved_through_projection_restore_and_fork() {
    let codec = CheckpointCodecIdentity {
        codec_id: "synthetic-m2-memory".into(),
        semantic_version: "3".into(),
    };
    let state = rich_provenance_state();
    let checkpoint = EnvironmentCheckpointV3::new(
        state.clone(),
        EpisodeStatus::Running,
        EnvironmentLimitCounters::default(),
        codec.clone(),
    )
    .unwrap();

    let controller = TrustedEnvironmentController::new(
        SyntheticM1EnvironmentBackend::from_checkpoint(
            checkpoint.clone(),
            config([PlayerId(1), PlayerId(2)]),
        )
        .unwrap(),
    );
    let endpoint = controller.bind_player(PlayerId(1)).unwrap();
    let projected = endpoint.information_state().unwrap();
    projected.validate().unwrap();
    let expected = projected_provenance(&projected);

    // The projection must not invent causes: every projected provenance
    // equals its authoritative counterpart.
    assert!(
        expected.contains(&(
            2u64,
            "invalidation/observed/Public/0/ExplicitReveal/Shuffle".to_string()
        )),
        "invalidation provenance was not preserved: {expected:?}"
    );
    assert!(
        expected.contains(&(
            2u64,
            "historical/observed/Private/0/OwnPrivateIdentity".to_string()
        )),
        "own_private_identity history was collapsed: {expected:?}"
    );
    assert!(
        expected.contains(&(3u64, "current/observed/Public/0/ExplicitReveal".to_string())),
        "explicit_reveal current fact was collapsed: {expected:?}"
    );
    assert!(
        !expected
            .iter()
            .any(|(_, text)| text.contains("PublicEvent")),
        "projection invented a public_event cause: {expected:?}"
    );

    // Checkpoint -> restore preserves exact provenance.
    let restored = TrustedEnvironmentController::new(
        SyntheticM1EnvironmentBackend::from_checkpoint(
            checkpoint.clone(),
            config([PlayerId(1), PlayerId(2)]),
        )
        .unwrap(),
    );
    restored.restore(checkpoint.clone()).unwrap();
    let restored_endpoint = restored.bind_player(PlayerId(1)).unwrap();
    assert_eq!(
        projected_provenance(&restored_endpoint.information_state().unwrap()),
        expected
    );
    assert_eq!(restored.checkpoint().unwrap().state, state);

    // A fork preserves exact provenance.
    let fork = controller.fork().unwrap();
    let fork_endpoint = fork.bind_player(PlayerId(1)).unwrap();
    assert_eq!(
        projected_provenance(&fork_endpoint.information_state().unwrap()),
        expected
    );
    assert_eq!(fork.checkpoint().unwrap().state, state);
}

fn submit_answer(
    endpoint: &PlayerEndpointHandle,
    answer: mtgml_decision::DecisionAnswerV2,
) -> PlayerStepV2 {
    let request = endpoint
        .visible_decision()
        .unwrap()
        .expect("a stage decision is visible");
    let step = endpoint
        .submit(mtgml_decision::DecisionResponseV2 {
            schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
            player_decision_id: request.player_decision_id,
            state_revision: request.state_revision,
            answer,
        })
        .unwrap();
    step.validate().unwrap();
    step
}

fn number_answer(value: i64) -> mtgml_decision::DecisionAnswerV2 {
    mtgml_decision::DecisionAnswerV2::ChooseNumber { value }
}

fn members_answer(ids: &[u32]) -> mtgml_decision::DecisionAnswerV2 {
    mtgml_decision::DecisionAnswerV2::SelectMany {
        candidate_ids: ids.iter().copied().map(CandidateIdV1).collect(),
    }
}

fn order_answer(ids: &[u32]) -> mtgml_decision::DecisionAnswerV2 {
    mtgml_decision::DecisionAnswerV2::Order {
        candidate_ids: ids.iter().copied().map(CandidateIdV1).collect(),
    }
}

/// Drives entry + ChooseCount(2) so the environment sits at the nonterminal
/// ChooseMembers stage of continuation C(1).
fn environment_at_members_stage() -> TrustedEnvironmentController {
    let controller = TrustedEnvironmentController::new(backend());
    let p1 = controller.bind_player(PlayerId(1)).unwrap();
    let _ = submit_answer(&p1, order_entry_answer());
    let _ = submit_answer(&p1, number_answer(2));
    controller
}

fn order_entry_answer() -> mtgml_decision::DecisionAnswerV2 {
    mtgml_decision::DecisionAnswerV2::SelectOne {
        candidate_id: CandidateIdV1(0),
    }
}

#[test]
fn continuation_checkpoint_restore_roundtrip_preserves_the_chain() {
    let source = environment_at_members_stage();
    let mid_checkpoint = source.checkpoint().unwrap();

    // Complete the chain on the source to capture the reference product.
    let source_p1 = source.bind_player(PlayerId(1)).unwrap();
    let reference_step = submit_answer(&source_p1, members_answer(&[0, 1]));
    let reference_after = source.checkpoint().unwrap();

    // Restore the mid-chain checkpoint into an equivalent environment and
    // submit the same next answer.
    let restored = TrustedEnvironmentController::new(backend());
    restored.restore(mid_checkpoint.clone()).unwrap();
    let restored_p1 = restored.bind_player(PlayerId(1)).unwrap();
    let replayed_step = submit_answer(&restored_p1, members_answer(&[0, 1]));

    assert_eq!(reference_step, replayed_step);
    assert_eq!(restored.checkpoint().unwrap(), reference_after);

    // The checkpoint itself contains the live continuation state.
    let state = &mid_checkpoint.state;
    assert_eq!(
        state
            .execution
            .continuations
            .keys()
            .copied()
            .collect::<Vec<_>>(),
        vec![ContinuationId(1)]
    );
}

#[test]
fn continuation_fork_equal_input_produces_equal_results() {
    let source = environment_at_members_stage();
    let fork_a = source.fork().unwrap();
    let fork_b = source.fork().unwrap();
    let a = fork_a.bind_player(PlayerId(1)).unwrap();
    let b = fork_b.bind_player(PlayerId(1)).unwrap();

    let step_a = submit_answer(&a, members_answer(&[0, 1]));
    let step_b = submit_answer(&b, members_answer(&[0, 1]));
    assert_eq!(step_a, step_b);
    assert_eq!(fork_a.checkpoint().unwrap(), fork_b.checkpoint().unwrap());

    // Divergence happens only through different valid inputs afterwards.
    let a2 = fork_a.bind_player(PlayerId(1)).unwrap();
    assert!(a2.visible_decision().unwrap().is_some());
}

#[test]
fn continuation_replay_full_chain_parity() {
    let controller = TrustedEnvironmentController::new(backend());
    let p1 = controller.bind_player(PlayerId(1)).unwrap();
    let c0 = controller.checkpoint().unwrap();

    let _ = submit_answer(&p1, order_entry_answer());
    let _ = submit_answer(&p1, number_answer(2));
    // One rejection during the active continuation must not enter the
    // accepted replay history.
    let before_rejection = controller.export_replay().unwrap();
    let before_checkpoint = controller.checkpoint().unwrap();
    let stale_request = p1.visible_decision().unwrap().unwrap();
    let wrong_cardinality = p1
        .submit(mtgml_decision::DecisionResponseV2 {
            schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
            player_decision_id: stale_request.player_decision_id,
            state_revision: stale_request.state_revision,
            answer: members_answer(&[0]),
        })
        .unwrap_err();
    assert_eq!(wrong_cardinality, PlayerApiError::InvalidSelection);
    assert_eq!(controller.export_replay().unwrap(), before_rejection);
    assert_eq!(controller.checkpoint().unwrap(), before_checkpoint);

    let _ = submit_answer(&p1, members_answer(&[0, 1]));
    let _ = submit_answer(&p1, order_answer(&[1, 0]));

    let live_after = controller.checkpoint().unwrap();
    let replay = controller.export_replay().unwrap();
    assert_eq!(replay.steps.len(), 4);
    for step in &replay.steps {
        assert!(step.accepted, "rejections are not replay steps");
    }

    // Replay reproduces the whole continuation progression deterministically.
    let report = controller
        .execute_replay_from_checkpoint(c0.clone(), replay.clone())
        .unwrap();
    assert_eq!(report.traces.len(), 4);
    assert_eq!(report.final_checkpoint, live_after);
    assert!(report
        .final_checkpoint
        .state
        .execution
        .continuations
        .is_empty());
}

#[test]
fn stale_stage_response_is_rejected_without_any_mutation() {
    let controller = TrustedEnvironmentController::new(backend());
    let p1 = controller.bind_player(PlayerId(1)).unwrap();

    // Complete the entry and capture the stage-0 (ChooseNumber) identity.
    let _ = submit_answer(&p1, order_entry_answer());
    let stage0_request = p1.visible_decision().unwrap().unwrap();
    let stage0_response = mtgml_decision::DecisionResponseV2 {
        schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
        player_decision_id: stage0_request.player_decision_id,
        state_revision: stage0_request.state_revision,
        answer: number_answer(1),
    };

    // Advance to stage 1 with a fresh visible identity.
    let _ = submit_answer(&p1, number_answer(2));
    let advanced = controller.checkpoint().unwrap();
    let advanced_replay = controller.export_replay().unwrap();

    // Resubmitting the earlier stage response is stale_decision: no mutation
    // of state, counters, continuation, or replay history.
    assert_eq!(
        p1.submit(stage0_response).unwrap_err(),
        PlayerApiError::StaleResponse
    );
    assert_eq!(controller.checkpoint().unwrap(), advanced);
    assert_eq!(controller.export_replay().unwrap(), advanced_replay);
}
