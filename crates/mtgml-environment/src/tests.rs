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
    let rejected = p1.submit(response(0, 1)).unwrap();
    assert_eq!(
        rejected.submission,
        mtgml_observation::PlayerStepSubmissionV1::Rejected {
            code: mtgml_observation::PlayerSubmissionCodeV1::StaleDecision,
        }
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

    // Player 2 does not own the visible decision: non-disclosing
    // unavailable_decision without any oracle difference.
    let foreign = p2.submit(response(0, 0)).unwrap();
    assert_eq!(
        foreign.submission,
        mtgml_observation::PlayerStepSubmissionV1::Rejected {
            code: mtgml_observation::PlayerSubmissionCodeV1::UnavailableDecision,
        }
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

    // Typed rejections are Ok steps carrying only the closed code.
    let stale = p1.submit(response(0, 9)).unwrap();
    assert_eq!(
        stale.submission,
        mtgml_observation::PlayerStepSubmissionV1::Rejected {
            code: mtgml_observation::PlayerSubmissionCodeV1::StaleDecision,
        }
    );

    // Trusted controller errors stay inside orchestration; assert no
    // trusted detail leaks through their rendering either.
    let bind_failure = match controller.bind_player(PlayerId(9)) {
        Ok(_) => "bound".to_string(),
        Err(error) => format!("{error}"),
    };
    for vocabulary in ["seed", "digest", "checkpoint", "gameobject", "decisionid"] {
        assert!(
            !bind_failure.to_lowercase().contains(vocabulary),
            "leaked {vocabulary}"
        );
    }

    // Serialized typed rejections must not carry trusted detail either.
    let bytes = mtgml_wire::encode_canonical(&stale).unwrap();
    let rendered = String::from_utf8(bytes).unwrap().to_lowercase();
    for vocabulary in ["continuation", "binding", "gameobject", "decisionid"] {
        assert!(
            !rendered.contains(vocabulary),
            "leaked {vocabulary} in public step"
        );
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
        .unwrap();
    assert_eq!(
        wrong_cardinality.submission,
        mtgml_observation::PlayerStepSubmissionV1::Rejected {
            code: mtgml_observation::PlayerSubmissionCodeV1::InvalidCardinality,
        }
    );
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

    // Resubmitting the earlier stage response is stale_decision as a typed
    // rejected step mirroring the unchanged product; nothing else mutates.
    let rejected_step = p1.submit(stage0_response).unwrap();
    rejected_step.validate().unwrap();
    assert_eq!(
        rejected_step.submission,
        mtgml_observation::PlayerStepSubmissionV1::Rejected {
            code: mtgml_observation::PlayerSubmissionCodeV1::StaleDecision,
        }
    );
    assert_eq!(
        rejected_step.information_state.state_revision,
        StateRevision(2)
    );
    assert_eq!(
        rejected_step
            .next_decision
            .as_ref()
            .unwrap()
            .player_decision_id,
        PlayerDecisionIdV1(3)
    );
    assert_eq!(controller.checkpoint().unwrap(), advanced);
    assert_eq!(controller.export_replay().unwrap(), advanced_replay);
}

#[test]
fn order_permutations_bind_distinct_replay_identity() {
    let run = |order: &[u32]| -> (AuthoritativeReplayV3, EnvironmentCheckpointV3) {
        let controller = TrustedEnvironmentController::new(backend());
        let p1 = controller.bind_player(PlayerId(1)).unwrap();
        let _ = submit_answer(&p1, order_entry_answer());
        let _ = submit_answer(&p1, number_answer(2));
        let _ = submit_answer(&p1, members_answer(&[0, 1]));
        let _ = submit_answer(&p1, order_answer(order));
        (
            controller.export_replay().unwrap(),
            controller.checkpoint().unwrap(),
        )
    };

    let (forward_replay, forward_checkpoint) = run(&[0, 1]);
    let (reverse_replay, reverse_checkpoint) = run(&[1, 0]);

    // The semantic order lives in the recorded authoritative response.
    fn last(replay: &AuthoritativeReplayV3) -> &DecisionResponseV2 {
        &replay.steps.last().unwrap().response
    }
    assert_eq!(
        last(&forward_replay).answer,
        mtgml_decision::DecisionAnswerV2::Order {
            candidate_ids: vec![CandidateIdV1(0), CandidateIdV1(1)]
        }
    );
    assert_eq!(
        last(&reverse_replay).answer,
        mtgml_decision::DecisionAnswerV2::Order {
            candidate_ids: vec![CandidateIdV1(1), CandidateIdV1(0)]
        }
    );
    // Neither order is repaired into the other: the recorded steps differ.
    assert_ne!(forward_replay, reverse_replay);
    assert_ne!(forward_replay.steps[3], reverse_replay.steps[3]);
    // The final environment identity is legitimately identical because this
    // synthetic domain persists no order; completion erases the sequence.
    assert_eq!(forward_checkpoint.state, reverse_checkpoint.state);
}

#[test]
fn from_checkpoint_rejects_states_the_kernel_cannot_execute() {
    use mtgml_decision::{DecisionDomainV2, DecisionVisibility};
    use mtgml_state::PendingDecisionRecordV2;

    let codec = CheckpointCodecIdentity {
        codec_id: "synthetic-m2-memory".into(),
        semantic_version: "3".into(),
    };
    let base = mtgml_state::construct_synthetic_engine_state(mtgml_state::SyntheticResetInputs {
        players: [PlayerId(1), PlayerId(2)],
        root_seed: seed(),
    })
    .unwrap();

    // A structurally valid generic EngineState with a standalone
    // ChooseNumber pending request and no continuation.
    let mut standalone_number = base.clone();
    standalone_number.execution.pending_decision = Some(PendingDecisionRecordV2 {
        request: mtgml_decision::AuthoritativeDecisionRequestV2 {
            decision_id: mtgml_model::DecisionId(1),
            player_decision_id: PlayerDecisionIdV1(1),
            state_revision: StateRevision(0),
            actor: PlayerId(1),
            visibility: DecisionVisibility::Public,
            decision: DecisionDomainV2::ChooseNumber {
                minimum: 0,
                maximum: 3,
            },
            candidates: Vec::new(),
            continuation_id: None,
        },
    });
    mtgml_state::validate_engine_state(&standalone_number).unwrap();
    let checkpoint = EnvironmentCheckpointV3::new(
        standalone_number.clone(),
        EpisodeStatus::Running,
        EnvironmentLimitCounters::default(),
        codec.clone(),
    )
    .expect("generic validation passes");
    assert!(matches!(
        SyntheticM1EnvironmentBackend::from_checkpoint(
            checkpoint.clone(),
            config([PlayerId(1), PlayerId(2)])
        ),
        Err(ControllerError::UnsupportedSyntheticState)
    ));
    let controller = TrustedEnvironmentController::new(backend());
    assert!(matches!(
        controller.restore(checkpoint),
        Err(ControllerError::UnsupportedSyntheticState)
    ));

    // A root ChooseOne whose kernel preconditions are violated (life not at
    // the entry value) is equally unsupported.
    let mut mismatched_entry = base;
    mismatched_entry
        .core
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .life = 39;
    let checkpoint = EnvironmentCheckpointV3::new(
        mismatched_entry,
        EpisodeStatus::Running,
        EnvironmentLimitCounters::default(),
        codec.clone(),
    )
    .unwrap();
    assert!(matches!(
        SyntheticM1EnvironmentBackend::from_checkpoint(
            checkpoint.clone(),
            config([PlayerId(1), PlayerId(2)])
        ),
        Err(ControllerError::UnsupportedSyntheticState)
    ));

    // The genuine program remains restorable.
    let genuine_checkpoint = environment_at_members_stage().checkpoint().unwrap();
    assert!(SyntheticM1EnvironmentBackend::from_checkpoint(
        genuine_checkpoint,
        config([PlayerId(1), PlayerId(2)])
    )
    .is_ok());
}

#[test]
fn unsupported_standalone_decisions_are_internal_kernel_failures() {
    use mtgml_decision::{DecisionDomainV2, DecisionVisibility};
    use mtgml_state::PendingDecisionRecordV2;

    let mut state =
        mtgml_state::construct_synthetic_engine_state(mtgml_state::SyntheticResetInputs {
            players: [PlayerId(1), PlayerId(2)],
            root_seed: seed(),
        })
        .unwrap();
    state.execution.pending_decision = Some(PendingDecisionRecordV2 {
        request: mtgml_decision::AuthoritativeDecisionRequestV2 {
            decision_id: mtgml_model::DecisionId(1),
            player_decision_id: PlayerDecisionIdV1(1),
            state_revision: StateRevision(0),
            actor: PlayerId(1),
            visibility: DecisionVisibility::Public,
            decision: DecisionDomainV2::ChooseNumber {
                minimum: 0,
                maximum: 3,
            },
            candidates: Vec::new(),
            continuation_id: None,
        },
    });
    // Soundness boundary: the engine never turns its own unsupported offer
    // into a player rejection; it is an internal failure before execution.
    let mut kernel = mtgml_rules::SyntheticM1RulesKernel;
    assert!(matches!(
        mtgml_rules::RulesKernel::apply(
            &mut kernel,
            &state,
            PlayerId(1),
            &mtgml_decision::DecisionResponseV2 {
                schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
                player_decision_id: PlayerDecisionIdV1(1),
                state_revision: StateRevision(0),
                answer: number_answer(1),
            }
        ),
        Err(mtgml_rules::KernelExecutionError::UnsupportedStagePath)
    ));
}

fn public_fingerprint(controller: &TrustedEnvironmentController) -> Vec<u8> {
    let checkpoint = controller.checkpoint().unwrap();
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&checkpoint.state_digest.raw_bytes());
    bytes.extend_from_slice(&checkpoint.checkpoint_digest.raw_bytes());
    bytes.extend(serde_json::to_vec(&controller.export_replay().unwrap()).unwrap());
    bytes
}

#[test]
fn projection_reads_are_pure_and_order_independent() {
    let controller = TrustedEnvironmentController::new(backend());
    let p1 = controller.bind_player(PlayerId(1)).unwrap();

    let observation_bytes = mtgml_wire::encode_canonical(&p1.observation().unwrap()).unwrap();
    let information_bytes = mtgml_wire::encode_canonical(&p1.information_state().unwrap()).unwrap();
    let decision_bytes =
        mtgml_wire::encode_canonical(&p1.visible_decision().unwrap().unwrap()).unwrap();

    let before = public_fingerprint(&controller);

    // Out-of-order repeated reads.
    for _ in 0..3 {
        assert_eq!(
            mtgml_wire::encode_canonical(&p1.visible_decision().unwrap().unwrap()).unwrap(),
            decision_bytes
        );
        assert_eq!(
            mtgml_wire::encode_canonical(&p1.observation().unwrap()).unwrap(),
            observation_bytes
        );
        assert_eq!(
            mtgml_wire::encode_canonical(&p1.information_state().unwrap()).unwrap(),
            information_bytes
        );
        assert_eq!(public_fingerprint(&controller), before);
    }
}

#[test]
fn projection_perspective_and_revision_coherence_matrix() {
    let controller = TrustedEnvironmentController::new(backend());
    let p1 = controller.bind_player(PlayerId(1)).unwrap();
    let p2 = controller.bind_player(PlayerId(2)).unwrap();

    let observation = p1.observation().unwrap();
    let information = p1.information_state().unwrap();
    let decision = p1.visible_decision().unwrap().unwrap();

    assert_eq!(observation.perspective, PlayerId(1));
    assert_eq!(information.perspective, PlayerId(1));
    assert_eq!(decision.actor, PlayerId(1));
    assert_eq!(observation.state_revision, information.state_revision);
    assert_eq!(decision.state_revision, information.state_revision);
    // One canonical current observation.
    assert_eq!(
        mtgml_wire::encode_canonical(&observation).unwrap(),
        mtgml_wire::encode_canonical(&information.current_observation).unwrap()
    );

    // The other perspective sees the same revision but its own surface.
    let info2 = p2.information_state().unwrap();
    assert_eq!(info2.perspective, PlayerId(2));
    assert_eq!(info2.state_revision, information.state_revision);
}

#[test]
fn episode_status_does_not_change_the_information_digest() {
    use mtgml_model::EpisodeStatus;

    // Drive the synthetic chain to completion: no pending decision remains,
    // so both Running and Terminal statuses are valid environment contexts
    // over the identical authoritative state.
    let controller = TrustedEnvironmentController::new(backend());
    let p1 = controller.bind_player(PlayerId(1)).unwrap();
    let _ = submit_answer(&p1, order_entry_answer());
    let _ = submit_answer(&p1, number_answer(2));
    let _ = submit_answer(&p1, members_answer(&[0, 1]));
    let _ = submit_answer(&p1, order_answer(&[1, 0]));
    let final_state = controller.checkpoint().unwrap().state;

    let codec = CheckpointCodecIdentity {
        codec_id: "synthetic-m2-memory".into(),
        semantic_version: "3".into(),
    };
    let running = EnvironmentCheckpointV3::new(
        final_state.clone(),
        EpisodeStatus::Running,
        EnvironmentLimitCounters::default(),
        codec.clone(),
    )
    .unwrap();
    let terminal = EnvironmentCheckpointV3::new(
        final_state.clone(),
        EpisodeStatus::Terminal {
            reason: TerminalReason::Concession,
            players: vec![mtgml_model::PlayerOutcome {
                player: PlayerId(1),
                result: mtgml_model::PlayerResult::Loss,
            }],
        },
        EnvironmentLimitCounters::default(),
        codec.clone(),
    )
    .unwrap();

    let running_env = TrustedEnvironmentController::new(
        SyntheticM1EnvironmentBackend::from_checkpoint(running, config([PlayerId(1), PlayerId(2)]))
            .unwrap(),
    );
    let terminal_env = TrustedEnvironmentController::new(
        SyntheticM1EnvironmentBackend::from_checkpoint(
            terminal,
            config([PlayerId(1), PlayerId(2)]),
        )
        .unwrap(),
    );
    let running_p1 = running_env.bind_player(PlayerId(1)).unwrap();
    let terminal_p1 = terminal_env.bind_player(PlayerId(1)).unwrap();

    // Identical information products including digest identity.
    assert_eq!(
        running_p1.information_state().unwrap(),
        terminal_p1.information_state().unwrap()
    );
    // Episode status itself differs on the step surface.
    assert_ne!(
        running_env.checkpoint().unwrap(),
        terminal_env.checkpoint().unwrap()
    );
}

#[test]
fn malformed_wire_bytes_never_reach_the_semantic_endpoint() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use crate::endpoint::{PlayerEndpoint, PlayerEndpointError};

    /// Seam probe: counts every semantic `submit` crossing the A/B split.
    struct CountingEndpoint<'a> {
        inner: &'a dyn PlayerEndpoint,
        submit_calls: AtomicUsize,
    }
    impl PlayerEndpoint for CountingEndpoint<'_> {
        fn perspective(&self) -> PlayerId {
            self.inner.perspective()
        }
        fn observation(
            &self,
        ) -> Result<mtgml_observation::ObservationEnvelope, PlayerEndpointError> {
            self.inner.observation()
        }
        fn information_state(
            &self,
        ) -> Result<mtgml_observation::PlayerInformationStateV2, PlayerEndpointError> {
            self.inner.information_state()
        }
        fn visible_decision(
            &self,
        ) -> Result<Option<mtgml_decision::PlayerDecisionRequestV2>, PlayerEndpointError> {
            self.inner.visible_decision()
        }
        fn submit(
            &self,
            response: DecisionResponseV2,
        ) -> Result<PlayerStepV2, PlayerEndpointError> {
            self.submit_calls.fetch_add(1, Ordering::SeqCst);
            self.inner.submit(response)
        }
    }

    let controller = TrustedEnvironmentController::new(backend());
    let handle = controller.bind_player(PlayerId(1)).unwrap();
    let endpoint = CountingEndpoint {
        inner: &handle,
        submit_calls: AtomicUsize::new(0),
    };
    let before = public_fingerprint(&controller);
    let submit_count = |endpoint: &CountingEndpoint| endpoint.submit_calls.load(Ordering::SeqCst);

    let malformed = b"{not json";
    let boundary_error = crate::submit_response_bytes(&endpoint, malformed).unwrap_err();
    assert_eq!(boundary_error.code(), "malformed_response");
    assert_eq!(submit_count(&endpoint), 0, "malformed bytes reached submit");

    // Noncanonical: valid JSON, wrong key order.
    let canonical = mtgml_wire::encode_canonical(&response(0, 0)).unwrap();
    let mut noncanonical = Vec::with_capacity(canonical.len() + 1);
    noncanonical.push(b' ');
    noncanonical.extend_from_slice(&canonical);
    let boundary_error = crate::submit_response_bytes(&endpoint, &noncanonical).unwrap_err();
    assert_eq!(boundary_error.code(), "malformed_response");
    assert_eq!(
        submit_count(&endpoint),
        0,
        "noncanonical bytes reached submit"
    );

    // Wrong schema version.
    let wrong_schema = String::from_utf8(canonical.clone())
        .unwrap()
        .replace("decision-response.v2", "decision-response.v1")
        .into_bytes();
    let boundary_error = crate::submit_response_bytes(&endpoint, &wrong_schema).unwrap_err();
    assert_eq!(boundary_error.code(), "malformed_response");
    assert_eq!(
        submit_count(&endpoint),
        0,
        "wrong-schema bytes reached submit"
    );

    // Positive seam control: canonical bytes carrying a semantically
    // invalid answer do reach Layer B exactly once and return a typed
    // rejected PlayerStep there.
    let mut stale = response(0, 0);
    stale.state_revision = StateRevision(9);
    let stale_bytes = mtgml_wire::encode_canonical(&stale).unwrap();
    let step = crate::submit_response_bytes(&endpoint, &stale_bytes).unwrap();
    assert_eq!(
        step.submission,
        mtgml_observation::PlayerStepSubmissionV1::Rejected {
            code: mtgml_observation::PlayerSubmissionCodeV1::StaleDecision,
        }
    );
    assert_eq!(submit_count(&endpoint), 1);

    // Zero mutation across both layers.
    assert_eq!(public_fingerprint(&controller), before);
}

#[test]
fn projection_bytes_survive_checkpoint_restore_and_equal_forks() {
    let source = environment_at_members_stage();
    let p1 = source.bind_player(PlayerId(1)).unwrap();

    let observation_bytes = mtgml_wire::encode_canonical(&p1.observation().unwrap()).unwrap();
    let information_bytes = mtgml_wire::encode_canonical(&p1.information_state().unwrap()).unwrap();
    let decision_bytes =
        mtgml_wire::encode_canonical(&p1.visible_decision().unwrap().unwrap()).unwrap();
    let checkpoint_before = source.checkpoint().unwrap();

    let restored = TrustedEnvironmentController::new(backend());
    restored.restore(checkpoint_before.clone()).unwrap();
    let restored_p1 = restored.bind_player(PlayerId(1)).unwrap();
    assert_eq!(
        mtgml_wire::encode_canonical(&restored_p1.observation().unwrap()).unwrap(),
        observation_bytes
    );
    assert_eq!(
        mtgml_wire::encode_canonical(&restored_p1.information_state().unwrap()).unwrap(),
        information_bytes
    );
    assert_eq!(
        mtgml_wire::encode_canonical(&restored_p1.visible_decision().unwrap().unwrap()).unwrap(),
        decision_bytes
    );

    let fork = source.fork().unwrap();
    let fork_p1 = fork.bind_player(PlayerId(1)).unwrap();
    assert_eq!(
        mtgml_wire::encode_canonical(&fork_p1.observation().unwrap()).unwrap(),
        observation_bytes
    );
    assert_eq!(
        mtgml_wire::encode_canonical(&fork_p1.information_state().unwrap()).unwrap(),
        information_bytes
    );
    assert_eq!(
        mtgml_wire::encode_canonical(&fork_p1.visible_decision().unwrap().unwrap()).unwrap(),
        decision_bytes
    );

    // Projection calls do not mutate the checkpoint fingerprint.
    assert_eq!(source.checkpoint().unwrap(), checkpoint_before);
}

#[test]
fn request_existence_is_not_an_error_oracle() {
    let controller = TrustedEnvironmentController::new(backend());
    let p1 = controller.bind_player(PlayerId(1)).unwrap();
    let p2 = controller.bind_player(PlayerId(2)).unwrap();

    // Case A: no request exists at all. The continuation chain was fully
    // completed, so pending_decision is None while the episode stays
    // Running; P2 submits into a genuinely requestless state.
    let _ = submit_answer(&p1, order_entry_answer());
    let _ = submit_answer(&p1, number_answer(2));
    let _ = submit_answer(&p1, members_answer(&[0, 1]));
    let _ = submit_answer(&p1, order_answer(&[1, 0]));
    let no_request = p2.submit(response(0, 0)).unwrap();
    assert_eq!(
        no_request.submission,
        mtgml_observation::PlayerStepSubmissionV1::Rejected {
            code: mtgml_observation::PlayerSubmissionCodeV1::UnavailableDecision,
        }
    );

    // Case B: a decision exists but belongs to P1 (paired foreign-request
    // state) — same non-disclosing surface for P2.
    let other = TrustedEnvironmentController::new(backend());
    let other_p2 = other.bind_player(PlayerId(2)).unwrap();
    let foreign = other_p2.submit(response(0, 0)).unwrap();
    assert_eq!(
        foreign.submission,
        mtgml_observation::PlayerStepSubmissionV1::Rejected {
            code: mtgml_observation::PlayerSubmissionCodeV1::UnavailableDecision,
        }
    );

    // The public rejection surface must not distinguish "no request
    // exists" from "the request belongs to another perspective" by
    // anything other than the legitimately changed environment product.
    let foreign_information = mtgml_wire::encode_canonical(&foreign.information_state).unwrap();
    let no_request_information =
        mtgml_wire::encode_canonical(&no_request.information_state).unwrap();
    assert_ne!(
        foreign_information, no_request_information,
        "states differ legitimately; the CODE must not"
    );
    let code_of = |submission: &mtgml_observation::PlayerStepSubmissionV1| match submission {
        mtgml_observation::PlayerStepSubmissionV1::Rejected { code } => *code,
        other => panic!("expected rejected submission, got {other:?}"),
    };
    assert_eq!(
        code_of(&foreign.submission),
        code_of(&no_request.submission),
        "request existence leaked as a distinct code"
    );
}

#[test]
fn visible_decision_exposes_no_trusted_identities_or_internals() {
    let controller = TrustedEnvironmentController::new(backend());
    let p1 = controller.bind_player(PlayerId(1)).unwrap();
    let request = p1.visible_decision().unwrap().unwrap();
    let bytes = mtgml_wire::encode_canonical(&request).unwrap();
    let rendered = String::from_utf8(bytes.clone()).unwrap();

    // Structural forbidden-key checks over the serialized graph.
    // `player_decision_id` is a legitimate public key; the forbidden keys
    // are matched as exact quoted JSON keys.
    for forbidden_key in [
        "\"decision_id\"",
        "\"continuation_id\"",
        "\"game_object_id\"",
        "\"physical_card_id\"",
        "\"ability_instance_id\"",
        "\"rule_event_id\"",
        "\"trusted_binding\"",
        "\"root_seed\"",
        "\"checkpoint_digest\"",
        "\"full_state_digest\"",
        "\"stream_key\"",
        "\"next_raw_u64\"",
    ] {
        assert!(
            !rendered.contains(forbidden_key),
            "visible decision leaked forbidden key {forbidden_key}"
        );
    }

    // Paired states: unrelated trusted/global values must not move the
    // public bytes.
    let mut variant =
        mtgml_state::construct_synthetic_engine_state(mtgml_state::SyntheticResetInputs {
            players: [PlayerId(1), PlayerId(2)],
            root_seed: seed(),
        })
        .unwrap();
    variant.allocators.next_effect_id = mtgml_model::EffectInstanceId(500);
    variant.allocators.next_trigger_id = mtgml_model::TriggerInstanceId(900);
    let checkpoint = EnvironmentCheckpointV3::new(
        variant,
        EpisodeStatus::Running,
        EnvironmentLimitCounters::default(),
        CheckpointCodecIdentity {
            codec_id: "synthetic-m2-memory".into(),
            semantic_version: "3".into(),
        },
    )
    .unwrap();
    let other_controller = TrustedEnvironmentController::new(
        SyntheticM1EnvironmentBackend::from_checkpoint(
            checkpoint,
            config([PlayerId(1), PlayerId(2)]),
        )
        .unwrap(),
    );
    let other_p1 = other_controller.bind_player(PlayerId(1)).unwrap();
    let other_request = other_p1.visible_decision().unwrap().unwrap();
    assert_eq!(
        mtgml_wire::encode_canonical(&request).unwrap(),
        mtgml_wire::encode_canonical(&other_request).unwrap(),
        "unrelated trusted/global history changed the visible decision bytes"
    );
}

#[test]
fn observation_equals_information_state_current_observation_bytes() {
    let controller = TrustedEnvironmentController::new(backend());
    let p1 = controller.bind_player(PlayerId(1)).unwrap();
    let observation = p1.observation().unwrap();
    let information = p1.information_state().unwrap();
    assert_eq!(
        mtgml_wire::encode_canonical(&observation).unwrap(),
        mtgml_wire::encode_canonical(&information.current_observation).unwrap()
    );
}

#[test]
fn typed_rejection_codes_matrix() {
    let controller = TrustedEnvironmentController::new(backend());
    let p1 = controller.bind_player(PlayerId(1)).unwrap();

    let expected_code = |step: &PlayerStepV2| -> mtgml_observation::PlayerSubmissionCodeV1 {
        match &step.submission {
            mtgml_observation::PlayerStepSubmissionV1::Rejected { code } => *code,
            other => panic!("expected rejected submission, got {other:?}"),
        }
    };

    // Accept entry to reach stage 0 (ChooseCount).
    let _ = submit_answer(&p1, order_entry_answer());

    // stale_decision via revision mismatch.
    let visible = p1.visible_decision().unwrap().unwrap();
    let stale = p1
        .submit(mtgml_decision::DecisionResponseV2 {
            schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
            player_decision_id: visible.player_decision_id,
            state_revision: StateRevision(9),
            answer: number_answer(1),
        })
        .unwrap();
    assert_eq!(
        expected_code(&stale),
        mtgml_observation::PlayerSubmissionCodeV1::StaleDecision,
        "stale"
    );

    // invalid_answer via Order variant against ChooseNumber domain.
    let wrong_variant = p1
        .submit(mtgml_decision::DecisionResponseV2 {
            schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
            player_decision_id: visible.player_decision_id,
            state_revision: visible.state_revision,
            answer: mtgml_decision::DecisionAnswerV2::Order {
                candidate_ids: vec![],
            },
        })
        .unwrap();
    assert_eq!(
        expected_code(&wrong_variant),
        mtgml_observation::PlayerSubmissionCodeV1::InvalidAnswer,
        "invalid_answer"
    );

    // Advance to ChooseMembers{2,2}.
    let _ = submit_answer(&p1, number_answer(2));

    // invalid_candidate.
    let request = p1.visible_decision().unwrap().unwrap();
    let unknown = p1
        .submit(mtgml_decision::DecisionResponseV2 {
            schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
            player_decision_id: request.player_decision_id,
            state_revision: request.state_revision,
            answer: members_answer(&[7]),
        })
        .unwrap();
    assert_eq!(
        expected_code(&unknown),
        mtgml_observation::PlayerSubmissionCodeV1::InvalidCandidate,
        "invalid_candidate"
    );

    // duplicate_assignment.
    let dup = p1
        .submit(mtgml_decision::DecisionResponseV2 {
            schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
            player_decision_id: request.player_decision_id,
            state_revision: request.state_revision,
            answer: members_answer(&[0, 0]),
        })
        .unwrap();
    assert_eq!(
        expected_code(&dup),
        mtgml_observation::PlayerSubmissionCodeV1::DuplicateAssignment,
        "duplicate_assignment"
    );

    // invalid_cardinality.
    let card = p1
        .submit(mtgml_decision::DecisionResponseV2 {
            schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
            player_decision_id: request.player_decision_id,
            state_revision: request.state_revision,
            answer: members_answer(&[0]),
        })
        .unwrap();
    assert_eq!(
        expected_code(&card),
        mtgml_observation::PlayerSubmissionCodeV1::InvalidCardinality,
        "invalid_cardinality"
    );

    // invalid_order (noncanonical representation).
    let order = p1
        .submit(mtgml_decision::DecisionResponseV2 {
            schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
            player_decision_id: request.player_decision_id,
            state_revision: request.state_revision,
            answer: members_answer(&[1, 0]),
        })
        .unwrap();
    assert_eq!(
        expected_code(&order),
        mtgml_observation::PlayerSubmissionCodeV1::InvalidOrder,
        "invalid_order"
    );

    // Complete the continuation (clears pending_decision).
    let _ = submit_answer(&p1, members_answer(&[0, 1]));
    let _ = submit_answer(&p1, order_answer(&[1, 0]));

    // Build a truncated checkpoint to drive episode_closed.
    let completed_state = controller.checkpoint().unwrap().state;
    let truncated_checkpoint = EnvironmentCheckpointV3::new(
        completed_state,
        EpisodeStatus::Truncated {
            reason: TruncationReason::ExternalStop,
            players: vec![],
        },
        EnvironmentLimitCounters::default(),
        CheckpointCodecIdentity {
            codec_id: "synthetic-m2-memory".into(),
            semantic_version: "3".into(),
        },
    )
    .unwrap();
    let truncated_env = TrustedEnvironmentController::new(
        SyntheticM1EnvironmentBackend::from_checkpoint(
            truncated_checkpoint,
            config([PlayerId(1), PlayerId(2)]),
        )
        .unwrap(),
    );
    let truncated_p1 = truncated_env.bind_player(PlayerId(1)).unwrap();
    let closed = truncated_p1.submit(response(0, 99)).unwrap();
    assert_eq!(
        expected_code(&closed),
        mtgml_observation::PlayerSubmissionCodeV1::EpisodeClosed,
        "episode_closed"
    );
}

#[test]
fn internal_failures_surface_only_service_unavailable() {
    use mtgml_state::PendingDecisionRecordV2;

    // A structurally valid generic state with a standalone ChooseNumber
    // pending request is not executable by the synthetic kernel; restore
    // must reject it before any projection can expose it.
    let mut state =
        mtgml_state::construct_synthetic_engine_state(mtgml_state::SyntheticResetInputs {
            players: [PlayerId(1), PlayerId(2)],
            root_seed: seed(),
        })
        .unwrap();
    state.execution.pending_decision = Some(PendingDecisionRecordV2 {
        request: mtgml_decision::AuthoritativeDecisionRequestV2 {
            decision_id: mtgml_model::DecisionId(1),
            player_decision_id: PlayerDecisionIdV1(1),
            state_revision: StateRevision(0),
            actor: PlayerId(1),
            visibility: mtgml_decision::DecisionVisibility::Public,
            decision: mtgml_decision::DecisionDomainV2::ChooseNumber {
                minimum: 0,
                maximum: 3,
            },
            candidates: Vec::new(),
            continuation_id: None,
        },
    });
    let checkpoint = EnvironmentCheckpointV3::new(
        state,
        EpisodeStatus::Running,
        EnvironmentLimitCounters::default(),
        CheckpointCodecIdentity {
            codec_id: "synthetic-m2-memory".into(),
            semantic_version: "3".into(),
        },
    )
    .unwrap();

    let result = SyntheticM1EnvironmentBackend::from_checkpoint(
        checkpoint.clone(),
        config([PlayerId(1), PlayerId(2)]),
    );
    let error = match result {
        Err(error) => error,
        Ok(_) => panic!("unsupported state must be rejected"),
    };
    let rendered = format!("{error}");
    for vocabulary in ["seed", "digest", "gameobject", "decisionid", "continuation"] {
        assert!(
            !rendered.to_lowercase().contains(vocabulary),
            "leaked {vocabulary}"
        );
    }

    // The same closed surface must hold across the public player boundary:
    // an internal service defect driven through `PlayerEndpoint::submit`
    // (here: authoritative limit-counter exhaustion while committing an
    // otherwise fully accepted submission) may not disclose trusted detail
    // and must map to exactly `service_unavailable`.
    let players = [PlayerId(1), PlayerId(2)];
    let fresh = backend().checkpoint().unwrap();
    let exhausted_checkpoint = EnvironmentCheckpointV3::new(
        fresh.state,
        fresh.status.clone(),
        EnvironmentLimitCounters {
            decisions_submitted: u64::MAX,
            ..fresh.limit_counters.clone()
        },
        fresh.codec.clone(),
    )
    .unwrap();
    let exhausted_controller = TrustedEnvironmentController::new(
        SyntheticM1EnvironmentBackend::from_checkpoint(exhausted_checkpoint, config(players))
            .unwrap(),
    );
    let p1 = exhausted_controller.bind_player(PlayerId(1)).unwrap();
    let before = public_fingerprint(&exhausted_controller);
    let request = p1
        .visible_decision()
        .unwrap()
        .expect("entry decision visible");
    let bytes = mtgml_wire::encode_canonical(&DecisionResponseV2 {
        schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
        player_decision_id: request.player_decision_id,
        state_revision: request.state_revision,
        answer: order_entry_answer(),
    })
    .unwrap();
    let boundary_error = crate::submit_response_bytes(&p1, &bytes).unwrap_err();
    assert_eq!(boundary_error.code(), "service_unavailable");

    // The failed internal commit must not have mutated anything.
    assert_eq!(public_fingerprint(&exhausted_controller), before);
}
