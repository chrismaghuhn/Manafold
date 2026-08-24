use mtgml_decision::{
    CandidateAssignment, DecisionAnswerV2, DecisionResponse, DecisionResponseV2,
    DECISION_RESPONSE_SCHEMA, DECISION_RESPONSE_V2_SCHEMA,
};
use mtgml_model::{
    CheckpointCodecIdentity, ContentDigest, DecisionId, EnvironmentLimitCounters, EpisodeStatus,
    FullStateDigest, FullStateDigestV2, FullStateDigestV3, PlayerDecisionIdV1, PlayerId,
    StateRevision,
};

use crate::recorder::ReplayRecorderV2;
use crate::{
    AuthoritativeReplayV1, AuthoritativeReplayV3, DeckIdentityV1, InitialEnvironmentIdentityV3,
    KernelIdentityV1, RandomnessIdentityV1, RandomnessIdentityV2, ReplayManifestV1,
    ReplayManifestV2, ReplayManifestV3, ReplayRecorderV3, ReplaySchemaVersionsV1, ReplayStepV1,
    ReplayStepV2, ReplayStepV3, ReplayValidationError, REPLAY_FILE_SCHEMA, REPLAY_FILE_SCHEMA_V2,
    REPLAY_MANIFEST_SCHEMA, REPLAY_MANIFEST_SCHEMA_V2, REPLAY_MANIFEST_SCHEMA_V3,
    REPLAY_STEP_SCHEMA_V3,
};

fn digest(text: char) -> FullStateDigest {
    FullStateDigest::parse(text.to_string().repeat(64)).unwrap()
}

fn manifest() -> ReplayManifestV1 {
    ReplayManifestV1 {
        schema_version: REPLAY_MANIFEST_SCHEMA.into(),
        engine_build: "build".into(),
        kernel: KernelIdentityV1 {
            implementation_id: "reference".into(),
            semantic_version: "0.2.2".into(),
            build_profile: "test".into(),
        },
        rules_snapshot: "rules".into(),
        format_policy_snapshot: "format".into(),
        oracle_snapshot: "oracle".into(),
        card_bundle: "bundle".into(),
        schemas: ReplaySchemaVersionsV1 {
            observation: "observation-envelope.v1".into(),
            information_state: "information-state-envelope.v1".into(),
            decision: "player-decision-request.v1".into(),
            decision_response: "decision-response.v1".into(),
            observed_event: "observed-event-envelope.v1".into(),
            player_step: "player-step.v1".into(),
            replay_step: "replay-step.v1".into(),
        },
        randomness: RandomnessIdentityV1 {
            algorithm_id: "counter".into(),
            derivation_version: "v1".into(),
            root_seed_hex: "00".repeat(32),
        },
        decks: vec![DeckIdentityV1 {
            player: PlayerId(1),
            deck_id: "deck".into(),
            digest: ContentDigest::parse("11".repeat(32)).unwrap(),
        }],
        initial_state_revision: StateRevision(0),
        initial_state_digest: digest('0'),
    }
}

fn digest_v2(text: char) -> FullStateDigestV2 {
    FullStateDigestV2::parse(text.to_string().repeat(64)).unwrap()
}

fn manifest_v2() -> ReplayManifestV2 {
    ReplayManifestV2 {
        schema_version: REPLAY_MANIFEST_SCHEMA_V2.into(),
        engine_build: "synthetic-build".into(),
        kernel: KernelIdentityV1 {
            implementation_id: "synthetic-m1".into(),
            semantic_version: "0.2.2".into(),
            build_profile: "test".into(),
        },
        rules_snapshot: "synthetic-rules".into(),
        format_policy_snapshot: "synthetic-format".into(),
        oracle_snapshot: "synthetic-oracle".into(),
        card_bundle: "synthetic-bundle".into(),
        schemas: ReplaySchemaVersionsV1 {
            observation: "observation-envelope.v1".into(),
            information_state: "information-state-envelope.v1".into(),
            decision: "player-decision-request.v1".into(),
            decision_response: DECISION_RESPONSE_SCHEMA.into(),
            observed_event: "observed-event-envelope.v1".into(),
            player_step: "player-step.v1".into(),
            replay_step: "replay-step.v2".into(),
        },
        randomness: RandomnessIdentityV2 {
            contract_id: "mtgml.rng.v1".into(),
            root_seed_hex: "00".repeat(32),
        },
        decks: vec![DeckIdentityV1 {
            player: PlayerId(1),
            deck_id: "synthetic-deck-1".into(),
            digest: ContentDigest::parse("11".repeat(32)).unwrap(),
        }],
        initial_state_revision: StateRevision(0),
        initial_state_digest: digest_v2('0'),
    }
}

fn response_v2(revision: u64) -> DecisionResponse {
    DecisionResponse {
        schema_version: DECISION_RESPONSE_SCHEMA.into(),
        decision_id: DecisionId(1),
        state_revision: StateRevision(revision),
        assignments: vec![CandidateAssignment {
            candidate_id: "select_public_object".into(),
            ordinal: None,
        }],
    }
}

fn accepted_step_v2() -> ReplayStepV2 {
    ReplayStepV2 {
        step_index: 0,
        state_revision_before: StateRevision(0),
        response: response_v2(0),
        accepted: true,
        state_revision_after: StateRevision(1),
        state_digest_after: digest_v2('1'),
    }
}

#[test]
fn replay_recorder_starts_empty_segment_at_manifest_identity() {
    let manifest = manifest_v2();
    let recorder = ReplayRecorderV2::new(manifest.clone()).unwrap();

    assert_eq!(recorder.step_count(), 0);
    assert_eq!(recorder.manifest(), &manifest);
    let replay = recorder.export().unwrap();

    assert_eq!(replay.schema_version, REPLAY_FILE_SCHEMA_V2);
    assert_eq!(replay.manifest, manifest);
    assert!(replay.steps.is_empty());
    assert_eq!(
        replay.final_state_revision,
        StateRevision(0),
        "an empty segment remains at its checkpoint revision"
    );
    assert_eq!(replay.final_state_digest, digest_v2('0'));
    replay.validate().unwrap();
}

#[test]
fn replay_recorder_appends_exact_accepted_step() {
    let mut recorder = ReplayRecorderV2::new(manifest_v2()).unwrap();
    recorder.append(accepted_step_v2()).unwrap();

    let replay = recorder.export().unwrap();
    assert_eq!(recorder.step_count(), 1);
    assert_eq!(replay.steps, vec![accepted_step_v2()]);
    assert_eq!(replay.final_state_revision, StateRevision(1));
    assert_eq!(replay.final_state_digest, digest_v2('1'));
    replay.validate().unwrap();
}

#[test]
fn replay_recorder_rejects_invalid_append_without_mutation() {
    let mut recorder = ReplayRecorderV2::new(manifest_v2()).unwrap();
    let before = recorder.export().unwrap();
    let mut invalid = accepted_step_v2();
    invalid.step_index = 1;

    assert_eq!(
        recorder.append(invalid),
        Err(ReplayValidationError::RevisionDiscontinuity)
    );
    assert_eq!(recorder.export().unwrap(), before);
}

#[test]
fn replay_recorder_keeps_rejected_diagnostic_identity() {
    let mut recorder = ReplayRecorderV2::new(manifest_v2()).unwrap();
    let mut step = accepted_step_v2();
    step.accepted = false;
    step.state_revision_after = StateRevision(0);
    step.state_digest_after = digest_v2('0');

    recorder.append(step.clone()).unwrap();
    let replay = recorder.export().unwrap();

    assert_eq!(replay.steps, vec![step]);
    assert_eq!(replay.final_state_revision, StateRevision(0));
    assert_eq!(replay.final_state_digest, digest_v2('0'));
    replay.validate().unwrap();
}

#[test]
fn replay_schema_version_fields_must_all_be_non_empty() {
    let mut invalid = manifest();
    invalid.schemas.observed_event.clear();
    assert_eq!(
        invalid.validate(),
        Err(ReplayValidationError::EmptyIdentity)
    );
}

#[test]
fn rejected_replay_step_must_preserve_the_full_state_digest() {
    let mut replay = AuthoritativeReplayV1 {
        schema_version: REPLAY_FILE_SCHEMA.into(),
        manifest: manifest(),
        steps: vec![ReplayStepV1 {
            step_index: 0,
            state_revision_before: StateRevision(0),
            response: DecisionResponse {
                schema_version: "decision-response.v1".into(),
                decision_id: mtgml_model::DecisionId(1),
                state_revision: StateRevision(0),
                assignments: vec![],
            },
            accepted: false,
            state_revision_after: StateRevision(0),
            state_digest_after: digest('1'),
        }],
        final_state_revision: StateRevision(0),
        final_state_digest: digest('1'),
    };
    assert_eq!(
        replay.validate(),
        Err(ReplayValidationError::RejectedMutation)
    );
    replay.steps[0].state_digest_after = digest('0');
    replay.final_state_digest = digest('0');
    replay.validate().unwrap();
}

fn v3_identity(
    revision: u64,
    digest_byte: u8,
    counters: EnvironmentLimitCounters,
) -> InitialEnvironmentIdentityV3 {
    let full_state_digest = FullStateDigestV3::from_digest_bytes([digest_byte; 32]);
    let codec = CheckpointCodecIdentity {
        codec_id: "in-memory-reference".into(),
        semantic_version: "3".into(),
    };
    let checkpoint_digest = mtgml_persistence::checkpoint_digest::calculate_checkpoint_digest_v3(
        &full_state_digest.as_digest_reference(),
        &EpisodeStatus::Running,
        &counters,
        &codec,
    )
    .unwrap();
    InitialEnvironmentIdentityV3 {
        state_revision: StateRevision(revision),
        full_state_digest,
        episode_status: EpisodeStatus::Running,
        environment_limit_counters: counters,
        checkpoint_codec_identity: codec,
        checkpoint_digest,
    }
}

fn manifest_v3() -> ReplayManifestV3 {
    ReplayManifestV3 {
        schema_version: REPLAY_MANIFEST_SCHEMA_V3.into(),
        engine_build: "synthetic-build".into(),
        kernel: KernelIdentityV1 {
            implementation_id: "synthetic-m2".into(),
            semantic_version: "0.2.2".into(),
            build_profile: "test".into(),
        },
        rules_snapshot: "rules".into(),
        format_policy_snapshot: "format".into(),
        oracle_snapshot: "oracle".into(),
        card_bundle: "bundle".into(),
        schemas: ReplaySchemaVersionsV1 {
            observation: "observation-envelope.v1".into(),
            information_state: "information-state-envelope.v2".into(),
            decision: "player-decision-request.v2".into(),
            decision_response: DECISION_RESPONSE_V2_SCHEMA.into(),
            observed_event: "observed-event-envelope.v2".into(),
            player_step: "player-step.v2".into(),
            replay_step: REPLAY_STEP_SCHEMA_V3.into(),
        },
        randomness: RandomnessIdentityV2 {
            contract_id: "mtgml.rng.v1".into(),
            root_seed_hex: "00".repeat(32),
        },
        decks: vec![DeckIdentityV1 {
            player: PlayerId(1),
            deck_id: "deck".into(),
            digest: ContentDigest::parse("11".repeat(32)).unwrap(),
        }],
        initial_identity: v3_identity(0, 0, EnvironmentLimitCounters::default()),
    }
}

fn response_v3() -> DecisionResponseV2 {
    DecisionResponseV2 {
        schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
        player_decision_id: PlayerDecisionIdV1(1),
        state_revision: StateRevision(0),
        answer: DecisionAnswerV2::ChooseNumber { value: 0 },
    }
}

#[test]
fn replay_v3_empty_accepted_rejected_identity_matrix() {
    let manifest = manifest_v3();
    manifest.validate().unwrap();

    let recorder = ReplayRecorderV3::new(manifest.clone()).unwrap();
    let empty: AuthoritativeReplayV3 = recorder.export().unwrap();
    assert!(empty.steps.is_empty());
    assert_eq!(empty.final_identity, manifest.initial_identity);

    let mut rejected_recorder = ReplayRecorderV3::new(manifest.clone()).unwrap();
    let rejected = ReplayStepV3 {
        step_index: 0,
        actor: PlayerId(1),
        checkpoint_digest_before: manifest.initial_identity.checkpoint_digest.clone(),
        state_revision_before: StateRevision(0),
        response: response_v3(),
        accepted: false,
        state_revision_after: StateRevision(0),
        full_state_digest_after: manifest.initial_identity.full_state_digest.clone(),
        episode_status_after: EpisodeStatus::Running,
        environment_limit_counters_after: EnvironmentLimitCounters::default(),
        checkpoint_digest_after: manifest.initial_identity.checkpoint_digest.clone(),
    };
    rejected_recorder.append(rejected).unwrap();
    let rejected_replay = rejected_recorder.export().unwrap();
    rejected_replay.validate().unwrap();
    assert_eq!(rejected_replay.final_identity, manifest.initial_identity);

    let counters = EnvironmentLimitCounters {
        decisions_submitted: 1,
        accepted_transitions: 1,
        ..EnvironmentLimitCounters::default()
    };
    let after = v3_identity(1, 1, counters.clone());
    let mut accepted_recorder = ReplayRecorderV3::new(manifest.clone()).unwrap();
    accepted_recorder
        .append(ReplayStepV3 {
            step_index: 0,
            actor: PlayerId(1),
            checkpoint_digest_before: manifest.initial_identity.checkpoint_digest.clone(),
            state_revision_before: StateRevision(0),
            response: response_v3(),
            accepted: true,
            state_revision_after: after.state_revision,
            full_state_digest_after: after.full_state_digest.clone(),
            episode_status_after: after.episode_status.clone(),
            environment_limit_counters_after: after.environment_limit_counters.clone(),
            checkpoint_digest_after: after.checkpoint_digest.clone(),
        })
        .unwrap();
    let accepted_replay = accepted_recorder.export().unwrap();
    accepted_replay.validate().unwrap();
    assert_eq!(accepted_replay.final_identity, after);
}

fn accepted_v3_step(
    index: u64,
    before: &InitialEnvironmentIdentityV3,
    after: InitialEnvironmentIdentityV3,
) -> ReplayStepV3 {
    ReplayStepV3 {
        step_index: index,
        actor: PlayerId(1),
        checkpoint_digest_before: before.checkpoint_digest.clone(),
        state_revision_before: before.state_revision,
        response: DecisionResponseV2 {
            state_revision: before.state_revision,
            ..response_v3()
        },
        accepted: true,
        state_revision_after: after.state_revision,
        full_state_digest_after: after.full_state_digest.clone(),
        episode_status_after: after.episode_status.clone(),
        environment_limit_counters_after: after.environment_limit_counters.clone(),
        checkpoint_digest_after: after.checkpoint_digest.clone(),
    }
}

fn regressed_final(manifest: &ReplayManifestV3) -> InitialEnvironmentIdentityV3 {
    let mut final_identity = manifest.initial_identity.clone();
    final_identity.state_revision = StateRevision(1);
    final_identity.full_state_digest = FullStateDigestV3::from_digest_bytes([1; 32]);
    final_identity.environment_limit_counters = EnvironmentLimitCounters {
        decisions_submitted: 1,
        accepted_transitions: 1,
        ..EnvironmentLimitCounters::default()
    };
    let checkpoint_digest = mtgml_persistence::checkpoint_digest::calculate_checkpoint_digest_v3(
        &final_identity.full_state_digest.as_digest_reference(),
        &final_identity.episode_status,
        &final_identity.environment_limit_counters,
        &final_identity.checkpoint_codec_identity,
    )
    .unwrap();
    final_identity.checkpoint_digest = checkpoint_digest;
    final_identity
}

#[test]
fn replay_v3_rejects_corrupt_accepted_progression() {
    let manifest = manifest_v3();
    let initial = manifest.initial_identity.clone();

    // Accepted steps must advance the revision exactly contiguously.
    let jumped_counters = EnvironmentLimitCounters {
        decisions_submitted: 1,
        accepted_transitions: 1,
        ..EnvironmentLimitCounters::default()
    };
    let jumped = v3_identity(2, 1, jumped_counters);
    let mut replay = AuthoritativeReplayV3 {
        schema_version: crate::REPLAY_FILE_SCHEMA_V3.into(),
        manifest: manifest.clone(),
        steps: vec![accepted_v3_step(0, &initial, jumped)],
        final_identity: v3_identity(
            2,
            1,
            EnvironmentLimitCounters {
                decisions_submitted: 1,
                accepted_transitions: 1,
                ..EnvironmentLimitCounters::default()
            },
        ),
    };
    assert_eq!(
        replay.validate(),
        Err(ReplayValidationError::RevisionDiscontinuity)
    );

    // Counter progression is deterministic per accepted step.
    for (name, counters) in [
        (
            "decisions_submitted",
            EnvironmentLimitCounters {
                decisions_submitted: 2,
                accepted_transitions: 1,
                ..EnvironmentLimitCounters::default()
            },
        ),
        (
            "accepted_transitions",
            EnvironmentLimitCounters {
                decisions_submitted: 1,
                accepted_transitions: 2,
                ..EnvironmentLimitCounters::default()
            },
        ),
    ] {
        let after = v3_identity(1, 1, counters);
        replay.steps = vec![accepted_v3_step(0, &initial, after.clone())];
        replay.final_identity = after;
        assert_eq!(
            replay.validate(),
            Err(ReplayValidationError::CounterProgression),
            "counter corruption case {name} must be rejected"
        );
    }

    // Trace counters never move backwards.
    let started_counters = EnvironmentLimitCounters {
        rule_events_emitted: 5,
        resource_units_consumed: 7,
        wall_clock_elapsed_millis: 9,
        ..EnvironmentLimitCounters::default()
    };
    let mut regressing_manifest = manifest_v3();
    regressing_manifest.initial_identity = v3_identity(0, 0, started_counters);
    let regressed = v3_identity(
        1,
        1,
        EnvironmentLimitCounters {
            decisions_submitted: 1,
            accepted_transitions: 1,
            ..EnvironmentLimitCounters::default()
        },
    );
    let regressing_replay = AuthoritativeReplayV3 {
        schema_version: crate::REPLAY_FILE_SCHEMA_V3.into(),
        manifest: regressing_manifest.clone(),
        steps: vec![accepted_v3_step(
            0,
            &regressing_manifest.initial_identity,
            regressed,
        )],
        final_identity: regressed_final(&regressing_manifest),
    };
    assert_eq!(
        regressing_replay.validate(),
        Err(ReplayValidationError::CounterProgression)
    );

    // The exact deterministic progression remains valid.
    let after = v3_identity(
        1,
        1,
        EnvironmentLimitCounters {
            decisions_submitted: 1,
            accepted_transitions: 1,
            rule_events_emitted: 4,
            ..EnvironmentLimitCounters::default()
        },
    );
    replay.steps = vec![accepted_v3_step(0, &initial, after.clone())];
    replay.final_identity = after;
    replay.validate().unwrap();

    // A rejected step must preserve the complete checkpoint identity.
    let mut step = accepted_v3_step(0, &initial, {
        let counters = EnvironmentLimitCounters {
            decisions_submitted: 1,
            accepted_transitions: 1,
            ..EnvironmentLimitCounters::default()
        };
        v3_identity(1, 1, counters)
    });
    step.accepted = false;
    replay.steps = vec![step];
    assert_eq!(
        replay.validate(),
        Err(ReplayValidationError::RejectedMutation)
    );
}

#[test]
fn diagnostic_step_preserves_complete_identity() {
    // Direct-construction counterpart of the recorder-append path covered by
    // `replay_v3_empty_accepted_rejected_identity_matrix`: one hand-built
    // accepted:false step validates exactly because EVERY after-field
    // preserves the preceding identity, so the final identity remains the
    // initial one. Execution-side preservation is pinned in the environment
    // crate (`diagnostic_rejected_step_executes_with_intact_identity_chain`).
    let manifest = manifest_v3();
    let initial = manifest.initial_identity.clone();
    let step = ReplayStepV3 {
        step_index: 0,
        actor: PlayerId(1),
        checkpoint_digest_before: initial.checkpoint_digest.clone(),
        state_revision_before: initial.state_revision,
        response: DecisionResponseV2 {
            state_revision: initial.state_revision,
            ..response_v3()
        },
        accepted: false,
        state_revision_after: initial.state_revision,
        full_state_digest_after: initial.full_state_digest.clone(),
        episode_status_after: initial.episode_status.clone(),
        environment_limit_counters_after: initial.environment_limit_counters.clone(),
        checkpoint_digest_after: initial.checkpoint_digest.clone(),
    };
    let replay = AuthoritativeReplayV3 {
        schema_version: crate::REPLAY_FILE_SCHEMA_V3.into(),
        manifest,
        steps: vec![step],
        final_identity: initial,
    };
    replay.validate().unwrap();
}

#[test]
fn inactive_counter_forward_progression_passes_structural_monotonicity() {
    // Tamper-matrix coverage citations: overcounts of decisions_submitted /
    // accepted_transitions and backward drift of rule_events_emitted /
    // resource_units_consumed / wall_clock_elapsed_millis are already
    // rejected by `replay_v3_rejects_corrupt_accepted_progression`, and
    // rejected-step mutation by `replay_v3_empty_accepted_rejected_identity_matrix`.
    // The uncovered class is FORWARD inflation of the host-independent
    // counters: the structural contract pins monotonicity only, while exact
    // carry-forward is enforced by the environment executor, pinned there as
    // a fail-closed AfterDigestMismatch execution rejection
    // (`recorded_inactive_counter_progression_fails_closed_without_live_mutation`).
    let manifest = manifest_v3();
    let initial = manifest.initial_identity.clone();
    let inflated = v3_identity(
        1,
        1,
        EnvironmentLimitCounters {
            decisions_submitted: 1,
            accepted_transitions: 1,
            rule_events_emitted: 4,
            resource_units_consumed: 7,
            wall_clock_elapsed_millis: 1000,
        },
    );
    let replay = AuthoritativeReplayV3 {
        schema_version: crate::REPLAY_FILE_SCHEMA_V3.into(),
        manifest,
        steps: vec![accepted_v3_step(0, &initial, inflated.clone())],
        final_identity: inflated,
    };
    replay.validate().unwrap();
}
