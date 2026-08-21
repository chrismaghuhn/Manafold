use super::*;
use mtgml_decision::{DecisionResponse, PlayerDecisionRequest};
use mtgml_model::{
    AbilityInstanceId, CheckpointDigestV2, ContinuationId, DecisionId, EffectInstanceId,
    EpisodeStatus, GameObjectId, InformationStateDigest, ObservationDigest, OpaqueAbilityId,
    OpaqueObjectId, PlayerId, RuleEventId, StackObjectId, StateRevision, TerminalReason,
    TriggerInstanceId, TruncationReason,
};
use mtgml_observation::{
    InformationStateEnvelope, ObservationEnvelope, PlayerStep, INFORMATION_STATE_SCHEMA,
    OBSERVATION_SCHEMA,
};
use mtgml_random::{
    CanonicalRandomStreamEntryV1, RandomStateV1, RandomStreamCursorV1, RandomStreamKeyV1,
    RandomStreamKindV1, RootSeed256,
};
use mtgml_replay::AuthoritativeReplayV2;
use mtgml_state::{
    CoreRulesState, EngineState, ExecutionState, FormatState, IdentityAllocatorState,
    KnowledgeState, PerspectiveIdentityMap, PerspectiveIdentityState, PlayerKnowledgeState,
    PlayerState, ZoneState,
};
use std::collections::BTreeMap;

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
