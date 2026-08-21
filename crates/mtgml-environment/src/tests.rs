use super::*;
use mtgml_decision::{
    CandidateAssignment, DecisionResponse, PlayerDecisionRequest, DECISION_RESPONSE_SCHEMA,
};
use mtgml_model::{
    AbilityInstanceId, CheckpointDigestV2, ContentDigest, ContinuationId, DecisionId,
    EffectInstanceId, EpisodeStatus, FullStateDigestV2, GameObjectId, InformationStateDigest,
    ObservationDigest, OpaqueAbilityId, OpaqueObjectId, PlayerId, RuleEventId, StackObjectId,
    StateRevision, TerminalReason, TriggerInstanceId, TruncationReason,
};
use mtgml_observation::{
    InformationStateEnvelope, ObservationEnvelope, PlayerStep, INFORMATION_STATE_SCHEMA,
    OBSERVATION_SCHEMA,
};
use mtgml_random::{
    CanonicalRandomStreamEntryV1, RandomStateV1, RandomStreamCursorV1, RandomStreamKeyV1,
    RandomStreamKindV1, RootSeed256,
};
use mtgml_replay::{
    AuthoritativeReplayV2, DeckIdentityV1, KernelIdentityV1, ReplayRecorderV2,
    ReplaySchemaVersionsV1, ReplayStepV2,
};
use mtgml_rules::AuthoritativeRuleEventKind;
use mtgml_state::{
    CoreRulesState, EngineState, ExecutionState, FormatState, IdentityAllocatorState,
    KnowledgeState, PerspectiveIdentityMap, PerspectiveIdentityState, PlayerKnowledgeState,
    PlayerState, ZoneState,
};
use std::collections::BTreeMap;

fn synthetic_config(players: [PlayerId; 2]) -> SyntheticM1EnvironmentConfig {
    SyntheticM1EnvironmentConfig {
        codec: CheckpointCodecIdentity {
            codec_id: "synthetic-m1-memory".into(),
            semantic_version: "2".into(),
        },
        replay: SyntheticM1ReplayConfig {
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
            randomness_contract_id: "mtgml.rng.v1".into(),
            schemas: ReplaySchemaVersionsV1 {
                observation: OBSERVATION_SCHEMA.into(),
                information_state: INFORMATION_STATE_SCHEMA.into(),
                decision: "player-decision-request.v1".into(),
                decision_response: DECISION_RESPONSE_SCHEMA.into(),
                observed_event: "observed-event-envelope.v1".into(),
                player_step: "player-step.v1".into(),
                replay_step: "replay-step.v2".into(),
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

fn synthetic_seed() -> RootSeed256 {
    RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap()
}

fn synthetic_response() -> DecisionResponse {
    DecisionResponse {
        schema_version: DECISION_RESPONSE_SCHEMA.into(),
        decision_id: DecisionId(1),
        state_revision: StateRevision(0),
        assignments: vec![CandidateAssignment {
            candidate_id: "select_public_object".into(),
            ordinal: None,
        }],
    }
}

fn synthetic_backend() -> SyntheticM1EnvironmentBackend {
    let players = [PlayerId(1), PlayerId(2)];
    SyntheticM1EnvironmentBackend::new(players, synthetic_seed(), synthetic_config(players))
        .unwrap()
}

fn checkpoint_state() -> EngineState {
    let p1 = PlayerId(1);
    let p2 = PlayerId(2);
    EngineState {
        revision: StateRevision(0),
        core: CoreRulesState {
            players: BTreeMap::from([
                (
                    p1,
                    PlayerState {
                        life: 40,
                        has_lost: false,
                    },
                ),
                (
                    p2,
                    PlayerState {
                        life: 40,
                        has_lost: false,
                    },
                ),
            ]),
            active_player: p1,
            priority_player: p1,
            turn_number: 1,
        },
        zones: ZoneState::default(),
        allocators: IdentityAllocatorState {
            next_object_id: GameObjectId(1),
            next_ability_id: AbilityInstanceId(1),
            next_stack_object_id: StackObjectId(1),
            next_effect_id: EffectInstanceId(1),
            next_trigger_id: TriggerInstanceId(1),
            next_decision_id: DecisionId(1),
            next_continuation_id: ContinuationId(1),
            next_rule_event_id: RuleEventId(1),
            next_opaque_object_id: BTreeMap::from([
                (p1, OpaqueObjectId(1)),
                (p2, OpaqueObjectId(1)),
            ]),
            next_opaque_ability_id: BTreeMap::from([
                (p1, OpaqueAbilityId(1)),
                (p2, OpaqueAbilityId(1)),
            ]),
        },
        execution: ExecutionState::default(),
        random: RandomStateV1::default(),
        knowledge: KnowledgeState {
            players: BTreeMap::from([
                (p1, PlayerKnowledgeState::default()),
                (p2, PlayerKnowledgeState::default()),
            ]),
        },
        perspective_identities: PerspectiveIdentityState {
            players: BTreeMap::from([
                (p1, PerspectiveIdentityMap::default()),
                (p2, PerspectiveIdentityMap::default()),
            ]),
        },
        format: FormatState::None,
    }
}

#[derive(Clone)]
struct FakeBackend {
    players: Vec<PlayerId>,
    observations: std::collections::BTreeMap<PlayerId, ObservationEnvelope>,
}

impl FakeBackend {
    fn new() -> Self {
        let players = vec![PlayerId(1), PlayerId(2)];
        let observations = players
            .iter()
            .copied()
            .map(|player| {
                (
                    player,
                    ObservationEnvelope {
                        schema_version: OBSERVATION_SCHEMA.into(),
                        perspective: player,
                        state_revision: StateRevision(0),
                        payload_codec: "test.v1".into(),
                        payload_base64: "e30=".into(),
                        digest: ObservationDigest::from_canonical_bytes(b"{}"),
                    },
                )
            })
            .collect();
        Self {
            players,
            observations,
        }
    }
}

impl EnvironmentBackend for FakeBackend {
    fn players(&self) -> Vec<PlayerId> {
        self.players.clone()
    }
    fn checkpoint(&self) -> Result<EnvironmentCheckpointV2, ControllerError> {
        Err(ControllerError::Backend("not needed in handle test".into()))
    }
    fn restore(&mut self, _checkpoint: EnvironmentCheckpointV2) -> Result<(), ControllerError> {
        Ok(())
    }
    fn fork_boxed(&self) -> Result<Box<dyn EnvironmentBackend>, ControllerError> {
        Ok(Box::new(self.clone()))
    }
    fn export_replay(&self) -> Result<AuthoritativeReplayV2, ControllerError> {
        Err(ControllerError::Backend("not needed in handle test".into()))
    }
    fn player_observation(
        &self,
        perspective: PlayerId,
    ) -> Result<ObservationEnvelope, PlayerApiError> {
        self.observations
            .get(&perspective)
            .cloned()
            .ok_or(PlayerApiError::Unavailable)
    }

    fn player_information_state(
        &self,
        perspective: PlayerId,
    ) -> Result<InformationStateEnvelope, PlayerApiError> {
        let observation = self.player_observation(perspective)?;
        Ok(InformationStateEnvelope {
            schema_version: INFORMATION_STATE_SCHEMA.into(),
            perspective,
            state_revision: observation.state_revision,
            current_observation: observation,
            public_history_length: 0,
            private_history_length: 0,
            digest: InformationStateDigest::from_canonical_bytes(b"info"),
        })
    }
    fn player_visible_decision(
        &self,
        _perspective: PlayerId,
    ) -> Result<Option<PlayerDecisionRequest>, PlayerApiError> {
        Ok(None)
    }

    fn submit_player_response(
        &mut self,
        perspective: PlayerId,
        _response: DecisionResponse,
    ) -> Result<PlayerStep, PlayerApiError> {
        Ok(PlayerStep {
            schema_version: "player-step.v1".into(),
            information_state: self.player_information_state(perspective)?,
            observed_events: vec![],
            next_decision: None,
            status: EpisodeStatus::Running,
        })
    }
}

struct CounterCorruptingBackend {
    inner: Box<dyn EnvironmentBackend>,
    corrupt_after_execute: bool,
}

impl CounterCorruptingBackend {
    fn new() -> Self {
        Self {
            inner: Box::new(synthetic_backend()),
            corrupt_after_execute: false,
        }
    }
}

impl EnvironmentBackend for CounterCorruptingBackend {
    fn players(&self) -> Vec<PlayerId> {
        self.inner.players()
    }

    fn checkpoint(&self) -> Result<EnvironmentCheckpointV2, ControllerError> {
        let checkpoint = self.inner.checkpoint()?;
        if !self.corrupt_after_execute {
            return Ok(checkpoint);
        }
        let mut counters = checkpoint.limit_counters.clone();
        counters.decisions_submitted = counters.decisions_submitted.checked_add(1).unwrap();
        EnvironmentCheckpointV2::new(
            checkpoint.state,
            checkpoint.status,
            counters,
            checkpoint.codec,
        )
        .map_err(ControllerError::CheckpointValidation)
    }

    fn restore(&mut self, checkpoint: EnvironmentCheckpointV2) -> Result<(), ControllerError> {
        self.inner.restore(checkpoint)?;
        self.corrupt_after_execute = false;
        Ok(())
    }

    fn fork_boxed(&self) -> Result<Box<dyn EnvironmentBackend>, ControllerError> {
        Ok(Box::new(Self {
            inner: self.inner.fork_boxed()?,
            corrupt_after_execute: false,
        }))
    }

    fn export_replay(&self) -> Result<AuthoritativeReplayV2, ControllerError> {
        self.inner.export_replay()
    }

    fn execute_trusted_response(
        &mut self,
        actor: PlayerId,
        response: DecisionResponse,
    ) -> Result<mtgml_rules::TransitionResult, ControllerError> {
        let transition = self.inner.execute_trusted_response(actor, response)?;
        if transition.accepted {
            self.corrupt_after_execute = true;
        }
        Ok(transition)
    }

    fn player_observation(
        &self,
        perspective: PlayerId,
    ) -> Result<ObservationEnvelope, PlayerApiError> {
        self.inner.player_observation(perspective)
    }

    fn player_information_state(
        &self,
        perspective: PlayerId,
    ) -> Result<InformationStateEnvelope, PlayerApiError> {
        self.inner.player_information_state(perspective)
    }

    fn player_visible_decision(
        &self,
        perspective: PlayerId,
    ) -> Result<Option<PlayerDecisionRequest>, PlayerApiError> {
        self.inner.player_visible_decision(perspective)
    }

    fn submit_player_response(
        &mut self,
        perspective: PlayerId,
        response: DecisionResponse,
    ) -> Result<PlayerStep, PlayerApiError> {
        self.inner.submit_player_response(perspective, response)
    }
}

#[test]
fn frozen_checkpoint_v2_digest_is_stable() {
    let checkpoint = EnvironmentCheckpointV2::new(
        checkpoint_state(),
        EpisodeStatus::Running,
        EnvironmentLimitCounters::default(),
        CheckpointCodecIdentity {
            codec_id: "in-memory-reference".into(),
            semantic_version: "1".into(),
        },
    )
    .unwrap();
    checkpoint.validate().unwrap();
    let digest = checkpoint.checkpoint_digest.clone();
    let canonical = serde_json::to_vec(&crate::checkpoint::CheckpointDigestInputV2 {
        schema_version: &checkpoint.schema_version,
        domain: CheckpointDigestV2::DOMAIN,
        state_digest: &checkpoint.state_digest,
        status: &checkpoint.status,
        limit_counters: &checkpoint.limit_counters,
        codec: &checkpoint.codec,
    })
    .unwrap();
    let expected_canonical = br#"{"schema_version":"environment-checkpoint.v2","domain":"mtgml.checkpoint-digest.v2","state_digest":"3ce7c015bba9669f2b7cabf0efc423b31b4507e213de56c10e1242e9e001334e","status":{"kind":"running"},"limit_counters":{"decisions_submitted":0,"accepted_transitions":0,"rule_events_emitted":0,"resource_units_consumed":0,"wall_clock_elapsed_millis":0},"codec":{"codec_id":"in-memory-reference","semantic_version":"1"}}"#;
    assert_eq!(
        canonical, expected_canonical,
        "frozen migration evidence: exact checkpoint V2 canonical bytes must not change"
    );
    let recomputed = CheckpointDigestV2::from_canonical_bytes(&canonical);
    assert_eq!(
        digest, recomputed,
        "frozen migration evidence: CheckpointDigestV2 from canonical bytes must match"
    );
    let expected_digest = CheckpointDigestV2::parse(
        "c823b1b894ba40f137a1b31a4330671f670d5da962233e0eeb1289eb13cce356",
    )
    .unwrap();
    assert_eq!(
        digest, expected_digest,
        "frozen migration evidence: exact checkpoint V2 digest must not change"
    );
}

#[test]
fn checkpoint_closes_state_status_and_limit_counters() {
    let checkpoint = EnvironmentCheckpointV2::new(
        checkpoint_state(),
        EpisodeStatus::Truncated {
            reason: TruncationReason::ExternalStop,
            players: vec![],
        },
        EnvironmentLimitCounters {
            decisions_submitted: 3,
            accepted_transitions: 2,
            rule_events_emitted: 7,
            resource_units_consumed: 11,
            wall_clock_elapsed_millis: 13,
        },
        CheckpointCodecIdentity {
            codec_id: "in-memory-reference".into(),
            semantic_version: "1".into(),
        },
    )
    .unwrap();
    checkpoint.validate().unwrap();
    assert!(matches!(
        &checkpoint.status,
        EpisodeStatus::Truncated {
            reason: TruncationReason::ExternalStop,
            ..
        }
    ));
    assert_eq!(checkpoint.limit_counters.accepted_transitions, 2);
}

#[test]
fn checkpoint_rejects_impossible_limit_counters() {
    let error = EnvironmentCheckpointV2::new(
        checkpoint_state(),
        EpisodeStatus::Running,
        EnvironmentLimitCounters {
            decisions_submitted: 1,
            accepted_transitions: 2,
            ..EnvironmentLimitCounters::default()
        },
        CheckpointCodecIdentity {
            codec_id: "in-memory-reference".into(),
            semantic_version: "1".into(),
        },
    )
    .unwrap_err();
    assert_eq!(error, CheckpointValidationError::LimitCounters);
}

#[test]
fn checkpoint_digest_covers_status_and_limit_counters() {
    let mut checkpoint = EnvironmentCheckpointV2::new(
        checkpoint_state(),
        EpisodeStatus::Running,
        EnvironmentLimitCounters::default(),
        CheckpointCodecIdentity {
            codec_id: "in-memory-reference".into(),
            semantic_version: "1".into(),
        },
    )
    .unwrap();
    checkpoint.limit_counters.resource_units_consumed = 1;
    assert_eq!(
        checkpoint.validate(),
        Err(CheckpointValidationError::CheckpointDigest)
    );
}

#[test]
fn checkpoint_v2_digest_is_stable_and_depends_on_rng() {
    let state_a = checkpoint_state();
    let mut state_b = checkpoint_state();
    let state_c = checkpoint_state();

    state_b
        .random
        .add_stream(
            RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1),
            RandomStreamCursorV1::default(),
        )
        .unwrap();

    let cp_a = EnvironmentCheckpointV2::new(
        state_a,
        EpisodeStatus::Running,
        EnvironmentLimitCounters::default(),
        CheckpointCodecIdentity {
            codec_id: "in-memory-reference".into(),
            semantic_version: "1".into(),
        },
    )
    .unwrap();

    let cp_b = EnvironmentCheckpointV2::new(
        state_b,
        EpisodeStatus::Running,
        EnvironmentLimitCounters::default(),
        CheckpointCodecIdentity {
            codec_id: "in-memory-reference".into(),
            semantic_version: "1".into(),
        },
    )
    .unwrap();

    cp_a.validate().unwrap();
    cp_b.validate().unwrap();

    let digest_a = cp_a.checkpoint_digest;
    let digest_b = cp_b.checkpoint_digest;

    assert_ne!(
        digest_a, digest_b,
        "checkpoint digest must depend on RNG state"
    );
    assert_eq!(digest_a, digest_a, "checkpoint digest must be stable");

    let cp_c = EnvironmentCheckpointV2::new(
        state_c,
        EpisodeStatus::Running,
        EnvironmentLimitCounters::default(),
        CheckpointCodecIdentity {
            codec_id: "in-memory-reference".into(),
            semantic_version: "1".into(),
        },
    )
    .unwrap();
    cp_c.validate().unwrap();
    assert_eq!(
        digest_a, cp_c.checkpoint_digest,
        "identical checkpoints must share a digest"
    );
}

#[test]
fn checkpoint_captures_m1_5_continuation_state() {
    let stream = RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1);
    let mut state = checkpoint_state();
    state.allocators.next_effect_id = EffectInstanceId(2);
    state.random = RandomStateV1::from_entries(
        RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap(),
        vec![CanonicalRandomStreamEntryV1 {
            key: stream,
            next_raw_u64: 1,
        }],
    )
    .unwrap();

    let checkpoint = EnvironmentCheckpointV2::new(
        state,
        EpisodeStatus::Running,
        EnvironmentLimitCounters::default(),
        CheckpointCodecIdentity {
            codec_id: "in-memory-reference".into(),
            semantic_version: "1".into(),
        },
    )
    .unwrap();
    checkpoint.validate().unwrap();

    let encoded = serde_json::to_vec(&checkpoint).unwrap();
    let decoded: EnvironmentCheckpointV2 = serde_json::from_slice(&encoded).unwrap();

    assert_eq!(decoded, checkpoint);
    assert_eq!(decoded.state.allocators.next_effect_id, EffectInstanceId(2));
    assert_eq!(
        decoded
            .state
            .random
            .lookup_stream(&stream)
            .unwrap()
            .next_raw_u64,
        1
    );
    assert_eq!(decoded.state_digest, decoded.state.digest().unwrap());
    decoded.validate().unwrap();
}

#[test]
fn checkpoint_v2_digest_changes_with_status() {
    let state = checkpoint_state();
    let cp_running = EnvironmentCheckpointV2::new(
        state.clone(),
        EpisodeStatus::Running,
        EnvironmentLimitCounters::default(),
        CheckpointCodecIdentity {
            codec_id: "in-memory-reference".into(),
            semantic_version: "1".into(),
        },
    )
    .unwrap();

    let cp_terminal = EnvironmentCheckpointV2::new(
        state,
        EpisodeStatus::Terminal {
            reason: TerminalReason::Concession,
            players: vec![],
        },
        EnvironmentLimitCounters::default(),
        CheckpointCodecIdentity {
            codec_id: "in-memory-reference".into(),
            semantic_version: "1".into(),
        },
    )
    .unwrap();

    cp_running.validate().unwrap();
    cp_terminal.validate().unwrap();

    assert_ne!(cp_running.checkpoint_digest, cp_terminal.checkpoint_digest);
}

#[test]
fn checkpoint_v2_digest_changes_with_limit_counters() {
    let state = checkpoint_state();
    let cp_a = EnvironmentCheckpointV2::new(
        state.clone(),
        EpisodeStatus::Running,
        EnvironmentLimitCounters::default(),
        CheckpointCodecIdentity {
            codec_id: "in-memory-reference".into(),
            semantic_version: "1".into(),
        },
    )
    .unwrap();

    let limits = EnvironmentLimitCounters {
        resource_units_consumed: 42,
        ..Default::default()
    };

    let cp_b = EnvironmentCheckpointV2::new(
        state,
        EpisodeStatus::Running,
        limits,
        CheckpointCodecIdentity {
            codec_id: "in-memory-reference".into(),
            semantic_version: "1".into(),
        },
    )
    .unwrap();

    cp_a.validate().unwrap();
    cp_b.validate().unwrap();

    assert_ne!(cp_a.checkpoint_digest, cp_b.checkpoint_digest);
}

#[test]
fn checkpoint_v2_digest_changes_with_codec() {
    let state = checkpoint_state();
    let cp_a = EnvironmentCheckpointV2::new(
        state.clone(),
        EpisodeStatus::Running,
        EnvironmentLimitCounters::default(),
        CheckpointCodecIdentity {
            codec_id: "codec-a".into(),
            semantic_version: "1".into(),
        },
    )
    .unwrap();

    let cp_b = EnvironmentCheckpointV2::new(
        state,
        EpisodeStatus::Running,
        EnvironmentLimitCounters::default(),
        CheckpointCodecIdentity {
            codec_id: "codec-b".into(),
            semantic_version: "1".into(),
        },
    )
    .unwrap();

    cp_a.validate().unwrap();
    cp_b.validate().unwrap();

    assert_ne!(cp_a.checkpoint_digest, cp_b.checkpoint_digest);
}

#[test]
fn two_player_endpoints_can_remain_alive_simultaneously() {
    let controller = TrustedEnvironmentController::new(FakeBackend::new());
    let player_one = controller.bind_player(PlayerId(1)).unwrap();
    let player_two = controller.bind_player(PlayerId(2)).unwrap();
    assert_eq!(player_one.observation().unwrap().perspective, PlayerId(1));
    assert_eq!(player_two.observation().unwrap().perspective, PlayerId(2));
    assert_eq!(player_one.observation().unwrap().perspective, PlayerId(1));
}

#[test]
fn player_api_errors_do_not_render_trusted_or_hidden_values() {
    let errors = [
        PlayerApiError::NoVisibleDecision,
        PlayerApiError::StaleResponse,
        PlayerApiError::InvalidSelection,
        PlayerApiError::EpisodeComplete,
        PlayerApiError::Unavailable,
    ];
    let forbidden = [
        "KernelExecutionError",
        "before state",
        "GameObjectId",
        "DecisionId",
        "OpaqueObjectId",
        "binding",
        "root seed",
        "next_raw_u64",
        "allocator",
        "knowledge",
    ];

    for error in errors {
        let rendered = error.to_string();
        for value in forbidden {
            assert!(
                !rendered.contains(value),
                "{error:?} exposed forbidden internal text {value:?}: {rendered:?}"
            );
        }
    }
}

#[test]
fn trusted_execution_api_can_export_and_verify_an_empty_replay_segment() {
    let controller = TrustedEnvironmentController::new(synthetic_backend());
    let checkpoint = controller.checkpoint().unwrap();
    let replay = controller.export_replay().unwrap();

    let report = controller
        .execute_replay_from_checkpoint(checkpoint.clone(), replay)
        .unwrap();

    assert!(report.traces.is_empty());
    assert_eq!(report.final_checkpoint, checkpoint);
}

#[test]
fn synthetic_backend_player_submission_surface_fails_closed_before_m1_7() {
    let controller = TrustedEnvironmentController::new(synthetic_backend());
    let endpoint = controller.bind_player(PlayerId(1)).unwrap();

    assert_eq!(endpoint.observation(), Err(PlayerApiError::Unavailable));
    assert_eq!(
        endpoint.information_state(),
        Err(PlayerApiError::Unavailable)
    );
    assert_eq!(
        endpoint.visible_decision(),
        Err(PlayerApiError::Unavailable)
    );
    assert_eq!(
        endpoint.submit(synthetic_response()),
        Err(PlayerApiError::Unavailable)
    );
}

#[test]
fn synthetic_backend_checkpoint_captures_the_complete_initial_product() {
    let controller = TrustedEnvironmentController::new(synthetic_backend());
    let checkpoint = controller.checkpoint().unwrap();
    let stream = RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1);

    assert_eq!(checkpoint.state.revision, StateRevision(0));
    assert!(checkpoint.state.execution.pending_decision.is_some());
    assert_eq!(
        checkpoint
            .state
            .random
            .lookup_stream(&stream)
            .unwrap()
            .next_raw_u64,
        0
    );
    assert_eq!(
        checkpoint.state.allocators.next_effect_id,
        EffectInstanceId(1)
    );
    assert_eq!(
        checkpoint.state.allocators.next_rule_event_id,
        RuleEventId(1)
    );
    assert_eq!(
        checkpoint.limit_counters,
        EnvironmentLimitCounters::default()
    );
    checkpoint.validate().unwrap();
}

#[test]
fn synthetic_backend_restore_rejects_state_tampering_without_mutation() {
    let controller = TrustedEnvironmentController::new(synthetic_backend());
    let before = controller.checkpoint().unwrap();
    let mut tampered = before.clone();
    tampered
        .state
        .core
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .life = 39;

    assert!(controller.restore(tampered).is_err());
    assert_eq!(controller.checkpoint().unwrap(), before);
}

#[test]
fn synthetic_backend_restore_rejects_counter_tampering_without_mutation() {
    let controller = TrustedEnvironmentController::new(synthetic_backend());
    let before = controller.checkpoint().unwrap();
    let mut tampered = before.clone();
    tampered.limit_counters.resource_units_consumed = 1;

    assert!(controller.restore(tampered).is_err());
    assert_eq!(controller.checkpoint().unwrap(), before);
}

#[test]
fn synthetic_backend_restore_rejects_unsupported_codec_without_mutation() {
    let controller = TrustedEnvironmentController::new(synthetic_backend());
    let before = controller.checkpoint().unwrap();
    let unsupported = EnvironmentCheckpointV2::new(
        before.state.clone(),
        before.status.clone(),
        before.limit_counters.clone(),
        CheckpointCodecIdentity {
            codec_id: "other-codec".into(),
            semantic_version: "2".into(),
        },
    )
    .unwrap();

    assert!(matches!(
        controller.restore(unsupported),
        Err(ControllerError::UnsupportedCheckpointCodec)
    ));
    assert_eq!(controller.checkpoint().unwrap(), before);
}

#[test]
fn direct_backend_restore_also_validates_before_mutation() {
    let mut backend = synthetic_backend();
    let before = backend.checkpoint().unwrap();
    let mut tampered = before.clone();
    tampered.limit_counters.accepted_transitions = 1;

    assert!(EnvironmentBackend::restore(&mut backend, tampered).is_err());
    assert_eq!(backend.checkpoint().unwrap(), before);
}

#[test]
fn synthetic_backend_from_checkpoint_rebases_to_an_empty_segment() {
    let source = synthetic_backend();
    let checkpoint = source.checkpoint().unwrap();
    let players = [PlayerId(1), PlayerId(2)];
    let restored = SyntheticM1EnvironmentBackend::from_checkpoint(
        checkpoint.clone(),
        synthetic_config(players),
    )
    .unwrap();
    let controller = TrustedEnvironmentController::new(restored);

    assert_eq!(controller.checkpoint().unwrap(), checkpoint);
    let replay = controller.export_replay().unwrap();
    assert!(replay.steps.is_empty());
    assert_eq!(
        replay.manifest.initial_state_revision,
        checkpoint.state.revision
    );
    assert_eq!(
        replay.manifest.initial_state_digest,
        checkpoint.state_digest
    );
    assert_eq!(replay.final_state_revision, checkpoint.state.revision);
    assert_eq!(replay.final_state_digest, checkpoint.state_digest);
}

#[test]
fn synthetic_backend_restore_rejects_non_two_player_replay_identity() {
    let backend = synthetic_backend();
    let checkpoint = backend.checkpoint().unwrap();
    let mut config = synthetic_config([PlayerId(1), PlayerId(2)]);
    config.replay.decks.pop();

    assert!(matches!(
        SyntheticM1EnvironmentBackend::from_checkpoint(checkpoint, config),
        Err(ControllerError::ReplayIdentityMismatch)
    ));
}

#[test]
fn accepted_environment_transaction_commits_the_exact_m1_product() {
    let controller = TrustedEnvironmentController::new(synthetic_backend());
    let before = controller.checkpoint().unwrap();

    let transition = controller
        .execute_trusted_response(PlayerId(1), synthetic_response())
        .unwrap();

    assert!(transition.accepted);
    assert_eq!(transition.next_state.revision, StateRevision(1));
    assert_eq!(transition.next_state.core.players[&PlayerId(1)].life, 38);
    assert_eq!(transition.next_state.core.players[&PlayerId(2)].life, 40);
    assert!(transition.next_state.execution.pending_decision.is_none());
    assert_eq!(transition.events.len(), 4);
    assert!(matches!(
        transition.events[2].event,
        AuthoritativeRuleEventKind::RandomValueSampled {
            bound: 10,
            value: 1,
            raw_words_consumed: 1,
            cursor_before: 0,
            cursor_after: 1,
            ..
        }
    ));
    assert_eq!(
        transition
            .next_state
            .random
            .lookup_stream(&RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1))
            .unwrap()
            .next_raw_u64,
        1
    );
    assert_eq!(
        transition.next_state.allocators.next_effect_id,
        EffectInstanceId(2)
    );
    assert_eq!(
        transition.next_state.allocators.next_rule_event_id,
        RuleEventId(5)
    );
    assert_eq!(
        transition.delta.apply(&before.state).unwrap(),
        transition.next_state
    );

    let after = controller.checkpoint().unwrap();
    assert_eq!(after.state, transition.next_state);
    assert_eq!(
        after.limit_counters,
        EnvironmentLimitCounters {
            decisions_submitted: 1,
            accepted_transitions: 1,
            rule_events_emitted: 4,
            resource_units_consumed: 0,
            wall_clock_elapsed_millis: 0,
        }
    );
    assert_eq!(after.state_digest, after.state.digest().unwrap());

    let replay = controller.export_replay().unwrap();
    assert_eq!(replay.steps.len(), 1);
    assert_eq!(replay.steps[0].step_index, 0);
    assert_eq!(replay.steps[0].state_revision_before, StateRevision(0));
    assert_eq!(replay.steps[0].response, synthetic_response());
    assert!(replay.steps[0].accepted);
    assert_eq!(replay.steps[0].state_revision_after, StateRevision(1));
    assert_eq!(replay.steps[0].state_digest_after, after.state_digest);
    assert_eq!(replay.final_state_revision, StateRevision(1));
    assert_eq!(replay.final_state_digest, after.state_digest);
    replay.validate().unwrap();

    let canonical = mtgml_wire::encode_canonical(&replay).unwrap();
    let decoded: AuthoritativeReplayV2 = mtgml_wire::decode_canonical(&canonical).unwrap();
    assert_eq!(decoded, replay);
}

#[test]
fn rejected_environment_submission_preserves_complete_outer_nonmutation() {
    let controller = TrustedEnvironmentController::new(synthetic_backend());
    let before_checkpoint = controller.checkpoint().unwrap();
    let before_replay = controller.export_replay().unwrap();
    let before_bytes = mtgml_wire::encode_canonical(&before_replay).unwrap();
    let mut rejected = synthetic_response();
    rejected.assignments[0].candidate_id = "unknown_candidate".into();

    let transition = controller
        .execute_trusted_response(PlayerId(1), rejected)
        .unwrap();

    assert!(!transition.accepted);
    let after_checkpoint = controller.checkpoint().unwrap();
    let after_replay = controller.export_replay().unwrap();
    assert_eq!(after_checkpoint, before_checkpoint);
    assert_eq!(after_replay, before_replay);
    assert_eq!(
        mtgml_wire::encode_canonical(&after_replay).unwrap(),
        before_bytes
    );
    assert_eq!(after_replay.steps.len(), 0);
}

#[test]
fn counter_overflow_fails_before_environment_commit() {
    let source = synthetic_backend();
    let initial = source.checkpoint().unwrap();
    let overflow_checkpoint = EnvironmentCheckpointV2::new(
        initial.state,
        initial.status,
        EnvironmentLimitCounters {
            decisions_submitted: u64::MAX,
            ..EnvironmentLimitCounters::default()
        },
        initial.codec,
    )
    .unwrap();
    let players = [PlayerId(1), PlayerId(2)];
    let backend = SyntheticM1EnvironmentBackend::from_checkpoint(
        overflow_checkpoint,
        synthetic_config(players),
    )
    .unwrap();
    let controller = TrustedEnvironmentController::new(backend);
    let before_checkpoint = controller.checkpoint().unwrap();
    let before_replay = controller.export_replay().unwrap();

    assert!(matches!(
        controller.execute_trusted_response(PlayerId(1), synthetic_response()),
        Err(ControllerError::CounterOverflow {
            counter: "decisions_submitted"
        })
    ));
    assert_eq!(controller.checkpoint().unwrap(), before_checkpoint);
    assert_eq!(controller.export_replay().unwrap(), before_replay);
}

#[test]
fn checkpoint_restore_repeats_exact_transition_and_replay_segment() {
    let controller = TrustedEnvironmentController::new(synthetic_backend());
    let mut rejected = synthetic_response();
    rejected.assignments[0].candidate_id = "unknown_candidate".into();
    let rejected_result = controller
        .execute_trusted_response(PlayerId(1), rejected)
        .unwrap();
    assert!(!rejected_result.accepted);

    let c0 = controller.checkpoint().unwrap();
    assert_eq!(c0.state.revision, StateRevision(0));
    assert_eq!(
        c0.state
            .random
            .lookup_stream(&RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1))
            .unwrap()
            .next_raw_u64,
        0
    );
    assert_eq!(c0.state.allocators.next_effect_id, EffectInstanceId(1));
    assert_eq!(c0.state.allocators.next_rule_event_id, RuleEventId(1));

    let transition_c = controller
        .execute_trusted_response(PlayerId(1), synthetic_response())
        .unwrap();
    let c1 = controller.checkpoint().unwrap();
    let replay_1 = controller.export_replay().unwrap();
    let replay_bytes_1 = mtgml_wire::encode_canonical(&replay_1).unwrap();

    controller.restore(c0.clone()).unwrap();
    let transition_f = controller
        .execute_trusted_response(PlayerId(1), synthetic_response())
        .unwrap();
    let c1b = controller.checkpoint().unwrap();
    let replay_1b = controller.export_replay().unwrap();
    let replay_bytes_1b = mtgml_wire::encode_canonical(&replay_1b).unwrap();

    assert_eq!(transition_c, transition_f);
    assert_eq!(c1, c1b);
    assert_eq!(c1.state_digest, c1b.state_digest);
    assert_eq!(c1.checkpoint_digest, c1b.checkpoint_digest);
    assert_eq!(replay_1, replay_1b);
    assert_eq!(replay_bytes_1, replay_bytes_1b);
    assert_eq!(
        c1.state
            .random
            .lookup_stream(&RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1))
            .unwrap()
            .next_raw_u64,
        1
    );
    assert_eq!(c1.state.allocators.next_effect_id, EffectInstanceId(2));
    assert_eq!(c1.state.allocators.next_rule_event_id, RuleEventId(5));
    assert_eq!(
        c1.limit_counters,
        EnvironmentLimitCounters {
            decisions_submitted: 1,
            accepted_transitions: 1,
            rule_events_emitted: 4,
            resource_units_consumed: 0,
            wall_clock_elapsed_millis: 0,
        }
    );
}

#[test]
fn accepted_state_restore_preserves_identity_and_rebases_empty_replay() {
    let source = TrustedEnvironmentController::new(synthetic_backend());
    source
        .execute_trusted_response(PlayerId(1), synthetic_response())
        .unwrap();
    let c1 = source.checkpoint().unwrap();

    let restored = TrustedEnvironmentController::new(
        SyntheticM1EnvironmentBackend::from_checkpoint(
            c1.clone(),
            synthetic_config([PlayerId(1), PlayerId(2)]),
        )
        .unwrap(),
    );
    let restored_checkpoint = restored.checkpoint().unwrap();
    let restored_replay = restored.export_replay().unwrap();

    assert_eq!(restored_checkpoint, c1);
    assert!(restored_replay.steps.is_empty());
    assert_eq!(
        restored_replay.manifest.initial_state_revision,
        StateRevision(1)
    );
    assert_eq!(
        restored_replay.manifest.initial_state_digest,
        c1.state_digest
    );
    assert_eq!(restored_replay.final_state_revision, StateRevision(1));
    assert_eq!(restored_replay.final_state_digest, c1.state_digest);
    assert_eq!(
        restored_checkpoint
            .state
            .random
            .lookup_stream(&RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1))
            .unwrap()
            .next_raw_u64,
        1
    );
    assert_eq!(
        restored_checkpoint.state.allocators.next_effect_id,
        EffectInstanceId(2)
    );
    assert_eq!(
        restored_checkpoint.state.allocators.next_rule_event_id,
        RuleEventId(5)
    );
}

#[test]
fn forks_from_a_checkpoint_begin_with_exact_identity_and_empty_segments() {
    let source = TrustedEnvironmentController::new(synthetic_backend());
    let c0 = source.checkpoint().unwrap();
    let fork_a = source.fork().unwrap();
    let fork_b = source.fork().unwrap();

    assert_eq!(fork_a.checkpoint().unwrap(), c0);
    assert_eq!(fork_b.checkpoint().unwrap(), c0);
    let replay_a = fork_a.export_replay().unwrap();
    let replay_b = fork_b.export_replay().unwrap();
    assert_eq!(replay_a, replay_b);
    assert!(replay_a.steps.is_empty());
    assert_eq!(
        mtgml_wire::encode_canonical(&replay_a).unwrap(),
        mtgml_wire::encode_canonical(&replay_b).unwrap()
    );
}

#[test]
fn forks_with_the_same_input_have_exact_continuation_parity() {
    let source = TrustedEnvironmentController::new(synthetic_backend());
    let fork_a = source.fork().unwrap();
    let fork_b = source.fork().unwrap();

    let transition_a = fork_a
        .execute_trusted_response(PlayerId(1), synthetic_response())
        .unwrap();
    let transition_b = fork_b
        .execute_trusted_response(PlayerId(1), synthetic_response())
        .unwrap();

    assert_eq!(transition_a, transition_b);
    assert_eq!(fork_a.checkpoint().unwrap(), fork_b.checkpoint().unwrap());
    assert_eq!(
        fork_a.export_replay().unwrap(),
        fork_b.export_replay().unwrap()
    );
    assert_eq!(
        mtgml_wire::encode_canonical(&fork_a.export_replay().unwrap()).unwrap(),
        mtgml_wire::encode_canonical(&fork_b.export_replay().unwrap()).unwrap()
    );
    assert_eq!(
        source.checkpoint().unwrap(),
        synthetic_backend().checkpoint().unwrap()
    );
}

#[test]
fn forks_diverge_only_on_explicit_accepted_or_rejected_input() {
    let source = TrustedEnvironmentController::new(synthetic_backend());
    let fork_accepted = source.fork().unwrap();
    let fork_rejected = source.fork().unwrap();
    let source_checkpoint = source.checkpoint().unwrap();

    let accepted = fork_accepted
        .execute_trusted_response(PlayerId(1), synthetic_response())
        .unwrap();
    let mut rejected_response = synthetic_response();
    rejected_response.assignments[0].candidate_id = "unknown_candidate".into();
    let rejected = fork_rejected
        .execute_trusted_response(PlayerId(1), rejected_response)
        .unwrap();

    assert!(accepted.accepted);
    assert!(!rejected.accepted);
    assert_eq!(
        fork_accepted.checkpoint().unwrap().state.revision,
        StateRevision(1)
    );
    assert_eq!(fork_rejected.checkpoint().unwrap(), source_checkpoint);
    assert_ne!(
        fork_accepted.checkpoint().unwrap(),
        fork_rejected.checkpoint().unwrap()
    );
    assert_eq!(source.checkpoint().unwrap(), source_checkpoint);
}

#[test]
fn semantic_replay_reproduces_the_live_accepted_transition_exactly() {
    let controller = TrustedEnvironmentController::new(synthetic_backend());
    let c0 = controller.checkpoint().unwrap();
    let live_transition = controller
        .execute_trusted_response(PlayerId(1), synthetic_response())
        .unwrap();
    let live_after = controller.checkpoint().unwrap();
    let replay = controller.export_replay().unwrap();

    let report = controller
        .execute_replay_from_checkpoint(c0, replay)
        .unwrap();

    assert_eq!(report.traces.len(), 1);
    assert_eq!(report.traces[0].step_index, 0);
    assert_eq!(report.traces[0].transition, live_transition);
    assert_eq!(report.traces[0].after, live_after);
    assert_eq!(report.final_checkpoint, live_after);
}

#[test]
fn semantic_replay_rejects_counter_divergence_at_first_step() {
    let controller = TrustedEnvironmentController::new(CounterCorruptingBackend::new());
    let c0 = controller.checkpoint().unwrap();
    controller
        .execute_trusted_response(PlayerId(1), synthetic_response())
        .unwrap();
    let replay = controller.export_replay().unwrap();

    assert!(matches!(
        controller.execute_replay_from_checkpoint(c0, replay),
        Err(ControllerError::ReplayExecution(
            ReplayExecutionError::CounterMismatch { step_index: 0 }
        ))
    ));
}

#[test]
fn semantic_replay_executes_rejected_diagnostic_without_live_recording() {
    let controller = TrustedEnvironmentController::new(synthetic_backend());
    let c0 = controller.checkpoint().unwrap();
    let empty = controller.export_replay().unwrap();
    let mut rejected_response = synthetic_response();
    rejected_response.assignments[0].candidate_id = "unknown_candidate".into();
    let mut recorder = ReplayRecorderV2::new(empty.manifest).unwrap();
    recorder
        .append(ReplayStepV2 {
            step_index: 0,
            state_revision_before: StateRevision(0),
            response: rejected_response,
            accepted: false,
            state_revision_after: StateRevision(0),
            state_digest_after: c0.state_digest.clone(),
        })
        .unwrap();
    let diagnostic = recorder.export().unwrap();
    let canonical = mtgml_wire::encode_canonical(&diagnostic).unwrap();
    let diagnostic: AuthoritativeReplayV2 = mtgml_wire::decode_canonical(&canonical).unwrap();

    let report = controller
        .execute_replay_from_checkpoint(c0.clone(), diagnostic)
        .unwrap();

    assert_eq!(report.traces.len(), 1);
    assert!(!report.traces[0].transition.accepted);
    assert_eq!(report.traces[0].before, c0);
    assert_eq!(report.traces[0].after, c0);
    assert_eq!(report.final_checkpoint, c0);
    assert!(controller.export_replay().unwrap().steps.is_empty());
}

#[test]
fn semantic_replay_rejects_wrong_initial_digest_before_execution() {
    let controller = TrustedEnvironmentController::new(synthetic_backend());
    let c0 = controller.checkpoint().unwrap();
    controller
        .execute_trusted_response(PlayerId(1), synthetic_response())
        .unwrap();
    let mut replay = controller.export_replay().unwrap();
    replay.manifest.initial_state_digest = FullStateDigestV2::parse("ff".repeat(32)).unwrap();
    let before = controller.checkpoint().unwrap();

    assert!(matches!(
        controller.execute_replay_from_checkpoint(c0, replay),
        Err(ControllerError::ReplayExecution(
            ReplayExecutionError::ManifestMismatch
        ))
    ));
    assert_eq!(controller.checkpoint().unwrap(), before);
}

#[test]
fn semantic_replay_rejects_wrong_root_seed_before_execution() {
    let controller = TrustedEnvironmentController::new(synthetic_backend());
    let c0 = controller.checkpoint().unwrap();
    controller
        .execute_trusted_response(PlayerId(1), synthetic_response())
        .unwrap();
    let mut replay = controller.export_replay().unwrap();
    replay.manifest.randomness.root_seed_hex = "22".repeat(32);
    let before = controller.checkpoint().unwrap();

    assert!(matches!(
        controller.execute_replay_from_checkpoint(c0, replay),
        Err(ControllerError::ReplayExecution(
            ReplayExecutionError::ManifestMismatch
        ))
    ));
    assert_eq!(controller.checkpoint().unwrap(), before);
}

#[test]
fn semantic_replay_rejects_tampered_accepted_after_digest_at_first_step() {
    let controller = TrustedEnvironmentController::new(synthetic_backend());
    let c0 = controller.checkpoint().unwrap();
    controller
        .execute_trusted_response(PlayerId(1), synthetic_response())
        .unwrap();
    let mut replay = controller.export_replay().unwrap();
    replay.steps[0].state_digest_after = FullStateDigestV2::parse("ff".repeat(32)).unwrap();
    replay.final_state_digest = replay.steps[0].state_digest_after.clone();

    assert!(matches!(
        controller.execute_replay_from_checkpoint(c0, replay),
        Err(ControllerError::ReplayExecution(
            ReplayExecutionError::AfterDigestMismatch { step_index: 0 }
        ))
    ));
}

#[test]
fn semantic_replay_rejects_tampered_accepted_flag_at_first_step() {
    let controller = TrustedEnvironmentController::new(synthetic_backend());
    let c0 = controller.checkpoint().unwrap();
    controller
        .execute_trusted_response(PlayerId(1), synthetic_response())
        .unwrap();
    let mut replay = controller.export_replay().unwrap();
    replay.steps[0].accepted = false;
    replay.steps[0].state_revision_after = StateRevision(0);
    replay.steps[0].state_digest_after = c0.state_digest.clone();
    replay.final_state_revision = StateRevision(0);
    replay.final_state_digest = c0.state_digest.clone();

    assert!(matches!(
        controller.execute_replay_from_checkpoint(c0, replay),
        Err(ControllerError::ReplayExecution(
            ReplayExecutionError::OutcomeMismatch { step_index: 0 }
        ))
    ));
}

#[test]
fn semantic_replay_rejects_stale_response_at_first_step() {
    let controller = TrustedEnvironmentController::new(synthetic_backend());
    let c0 = controller.checkpoint().unwrap();
    controller
        .execute_trusted_response(PlayerId(1), synthetic_response())
        .unwrap();
    let mut replay = controller.export_replay().unwrap();
    replay.steps[0].response.decision_id = DecisionId(999);

    assert!(matches!(
        controller.execute_replay_from_checkpoint(c0, replay),
        Err(ControllerError::ReplayExecution(
            ReplayExecutionError::OutcomeMismatch { step_index: 0 }
        ))
    ));
}
