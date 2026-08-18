//! Capability-separated environment APIs.
//!
//! `TrustedEnvironmentController` owns checkpoint/fork/replay capabilities.
//! `PlayerEndpointHandle` is permanently perspective-bound and exposes only
//! projected information. Multiple player handles may coexist.

use mtgml_decision::{DecisionResponse, PlayerDecisionRequest};
use mtgml_model::{CheckpointDigest, EpisodeStatus, FullStateDigest, PlayerId};
use mtgml_observation::{InformationStateEnvelope, ObservationEnvelope, PlayerStep};
use mtgml_replay::AuthoritativeReplayV1;
use mtgml_state::{validate_engine_state, EngineState};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex, MutexGuard};
use thiserror::Error;

pub const ENVIRONMENT_CHECKPOINT_SCHEMA: &str = "environment-checkpoint.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentLimitCounters {
    pub decisions_submitted: u64,
    pub accepted_transitions: u64,
    pub rule_events_emitted: u64,
    pub resource_units_consumed: u64,
    pub wall_clock_elapsed_millis: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckpointCodecIdentity {
    pub codec_id: String,
    pub semantic_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentCheckpointV1 {
    pub schema_version: String,
    pub state: EngineState,
    pub state_digest: FullStateDigest,
    pub status: EpisodeStatus,
    pub limit_counters: EnvironmentLimitCounters,
    pub codec: CheckpointCodecIdentity,
    pub checkpoint_digest: CheckpointDigest,
}

#[derive(Serialize)]
struct CheckpointDigestInputV1<'a> {
    schema_version: &'a str,
    domain: &'static str,
    state_digest: &'a FullStateDigest,
    status: &'a EpisodeStatus,
    limit_counters: &'a EnvironmentLimitCounters,
    codec: &'a CheckpointCodecIdentity,
}

impl EnvironmentCheckpointV1 {
    pub fn new(
        state: EngineState,
        status: EpisodeStatus,
        limit_counters: EnvironmentLimitCounters,
        codec: CheckpointCodecIdentity,
    ) -> Result<Self, CheckpointValidationError> {
        let state_digest = state
            .digest()
            .map_err(|_| CheckpointValidationError::StateDigest)?;
        let schema_version = ENVIRONMENT_CHECKPOINT_SCHEMA.to_owned();
        let checkpoint_digest = Self::calculate_digest(
            &schema_version,
            &state_digest,
            &status,
            &limit_counters,
            &codec,
        )?;
        let checkpoint = Self {
            schema_version,
            state,
            state_digest,
            status,
            limit_counters,
            codec,
            checkpoint_digest,
        };
        checkpoint.validate()?;
        Ok(checkpoint)
    }

    fn calculate_digest(
        schema_version: &str,
        state_digest: &FullStateDigest,
        status: &EpisodeStatus,
        limit_counters: &EnvironmentLimitCounters,
        codec: &CheckpointCodecIdentity,
    ) -> Result<CheckpointDigest, CheckpointValidationError> {
        let input = CheckpointDigestInputV1 {
            schema_version,
            domain: CheckpointDigest::DOMAIN,
            state_digest,
            status,
            limit_counters,
            codec,
        };
        let bytes =
            serde_json::to_vec(&input).map_err(|_| CheckpointValidationError::CheckpointDigest)?;
        Ok(CheckpointDigest::from_canonical_bytes(&bytes))
    }

    pub fn validate(&self) -> Result<(), CheckpointValidationError> {
        if self.schema_version != ENVIRONMENT_CHECKPOINT_SCHEMA
            || self.codec.codec_id.is_empty()
            || self.codec.semantic_version.is_empty()
        {
            return Err(CheckpointValidationError::Identity);
        }
        validate_engine_state(&self.state)
            .map_err(|_| CheckpointValidationError::StateInvariant)?;
        let digest = self
            .state
            .digest()
            .map_err(|_| CheckpointValidationError::StateDigest)?;
        if digest != self.state_digest {
            return Err(CheckpointValidationError::StateDigest);
        }
        let checkpoint_digest = Self::calculate_digest(
            &self.schema_version,
            &self.state_digest,
            &self.status,
            &self.limit_counters,
            &self.codec,
        )?;
        if checkpoint_digest != self.checkpoint_digest {
            return Err(CheckpointValidationError::CheckpointDigest);
        }
        self.status
            .validate()
            .map_err(|_| CheckpointValidationError::EpisodeStatus)?;
        if self.limit_counters.accepted_transitions > self.limit_counters.decisions_submitted {
            return Err(CheckpointValidationError::LimitCounters);
        }
        if !matches!(&self.status, EpisodeStatus::Running)
            && self.state.execution.pending_decision.is_some()
        {
            return Err(CheckpointValidationError::CompletedWithDecision);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum CheckpointValidationError {
    #[error("unsupported or empty checkpoint identity")]
    Identity,
    #[error("checkpoint EngineState violates cross-component invariants")]
    StateInvariant,
    #[error("checkpoint full-state digest does not match its state")]
    StateDigest,
    #[error("checkpoint digest does not match status, limits, codec, and state identity")]
    CheckpointDigest,
    #[error("checkpoint episode status is invalid")]
    EpisodeStatus,
    #[error("checkpoint limit counters are inconsistent")]
    LimitCounters,
    #[error("completed checkpoint retains a pending player decision")]
    CompletedWithDecision,
}

pub trait EnvironmentBackend: Send {
    fn players(&self) -> Vec<PlayerId>;
    fn checkpoint(&self) -> Result<EnvironmentCheckpointV1, ControllerError>;
    fn restore(&mut self, checkpoint: EnvironmentCheckpointV1) -> Result<(), ControllerError>;
    fn fork_boxed(&self) -> Result<Box<dyn EnvironmentBackend>, ControllerError>;
    fn export_replay(&self) -> Result<AuthoritativeReplayV1, ControllerError>;

    fn player_observation(
        &self,
        perspective: PlayerId,
    ) -> Result<ObservationEnvelope, PlayerApiError>;
    fn player_information_state(
        &self,
        perspective: PlayerId,
    ) -> Result<InformationStateEnvelope, PlayerApiError>;
    fn player_visible_decision(
        &self,
        perspective: PlayerId,
    ) -> Result<Option<PlayerDecisionRequest>, PlayerApiError>;
    fn submit_player_response(
        &mut self,
        perspective: PlayerId,
        response: DecisionResponse,
    ) -> Result<PlayerStep, PlayerApiError>;
}

type SharedBackend = Arc<Mutex<Box<dyn EnvironmentBackend>>>;

#[derive(Clone)]
pub struct TrustedEnvironmentController {
    inner: SharedBackend,
}

impl TrustedEnvironmentController {
    pub fn new(backend: impl EnvironmentBackend + 'static) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Box::new(backend))),
        }
    }

    pub fn bind_player(&self, player: PlayerId) -> Result<PlayerEndpointHandle, ControllerError> {
        if !self.lock()?.players().contains(&player) {
            return Err(ControllerError::UnknownPlayer);
        }
        Ok(PlayerEndpointHandle {
            perspective: player,
            inner: Arc::clone(&self.inner),
        })
    }

    pub fn checkpoint(&self) -> Result<EnvironmentCheckpointV1, ControllerError> {
        self.lock()?.checkpoint()
    }

    pub fn restore(&self, checkpoint: EnvironmentCheckpointV1) -> Result<(), ControllerError> {
        checkpoint
            .validate()
            .map_err(|error| ControllerError::InvalidCheckpoint(error.to_string()))?;
        self.lock()?.restore(checkpoint)
    }

    pub fn fork(&self) -> Result<Self, ControllerError> {
        let backend = self.lock()?.fork_boxed()?;
        Ok(Self {
            inner: Arc::new(Mutex::new(backend)),
        })
    }

    pub fn export_replay(&self) -> Result<AuthoritativeReplayV1, ControllerError> {
        self.lock()?.export_replay()
    }

    fn lock(&self) -> Result<MutexGuard<'_, Box<dyn EnvironmentBackend>>, ControllerError> {
        self.inner.lock().map_err(|_| ControllerError::Poisoned)
    }
}

#[derive(Clone)]
pub struct PlayerEndpointHandle {
    perspective: PlayerId,
    inner: SharedBackend,
}

pub trait PlayerEndpoint: Send + Sync {
    fn perspective(&self) -> PlayerId;
    fn observation(&self) -> Result<ObservationEnvelope, PlayerApiError>;
    fn information_state(&self) -> Result<InformationStateEnvelope, PlayerApiError>;
    fn visible_decision(&self) -> Result<Option<PlayerDecisionRequest>, PlayerApiError>;
    fn submit(&self, response: DecisionResponse) -> Result<PlayerStep, PlayerApiError>;
}

impl PlayerEndpointHandle {
    fn lock(&self) -> Result<MutexGuard<'_, Box<dyn EnvironmentBackend>>, PlayerApiError> {
        self.inner.lock().map_err(|_| PlayerApiError::Unavailable)
    }
}

impl PlayerEndpoint for PlayerEndpointHandle {
    fn perspective(&self) -> PlayerId {
        self.perspective
    }

    fn observation(&self) -> Result<ObservationEnvelope, PlayerApiError> {
        self.lock()?.player_observation(self.perspective)
    }

    fn information_state(&self) -> Result<InformationStateEnvelope, PlayerApiError> {
        self.lock()?.player_information_state(self.perspective)
    }

    fn visible_decision(&self) -> Result<Option<PlayerDecisionRequest>, PlayerApiError> {
        self.lock()?.player_visible_decision(self.perspective)
    }

    fn submit(&self, response: DecisionResponse) -> Result<PlayerStep, PlayerApiError> {
        self.lock()?
            .submit_player_response(self.perspective, response)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum PlayerApiError {
    #[error("no decision is available to this player")]
    NoVisibleDecision,
    #[error("the response is stale")]
    StaleResponse,
    #[error("the selection is invalid")]
    InvalidSelection,
    #[error("the episode is complete")]
    EpisodeComplete,
    #[error("the endpoint is unavailable")]
    Unavailable,
}

#[derive(Debug, Error)]
pub enum ControllerError {
    #[error("unknown player")]
    UnknownPlayer,
    #[error("controller lock is poisoned")]
    Poisoned,
    #[error("checkpoint is invalid: {0}")]
    InvalidCheckpoint(String),
    #[error("backend failure: {0}")]
    Backend(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use mtgml_model::{
        AbilityInstanceId, ContinuationId, DecisionId, EffectInstanceId, GameObjectId,
        InformationStateDigest, ObservationDigest, OpaqueAbilityId, OpaqueObjectId, RuleEventId,
        StackObjectId, StateRevision, TriggerInstanceId, TruncationReason,
    };
    use mtgml_observation::{INFORMATION_STATE_SCHEMA, OBSERVATION_SCHEMA};
    use mtgml_random::{RandomState, RandomStreamState};
    use mtgml_state::{
        CoreRulesState, ExecutionState, FormatState, IdentityAllocatorState, KnowledgeState,
        PerspectiveIdentityMap, PerspectiveIdentityState, PlayerKnowledgeState, PlayerState,
        ZoneState,
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
            random: RandomState {
                algorithm_id: "test-counter".into(),
                derivation_version: "v1".into(),
                root_seed_hex: "00".repeat(32),
                streams: BTreeMap::from([("shuffle".into(), RandomStreamState { counter: 0 })]),
            },
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
        fn checkpoint(&self) -> Result<EnvironmentCheckpointV1, ControllerError> {
            Err(ControllerError::Backend("not needed in handle test".into()))
        }
        fn restore(&mut self, _checkpoint: EnvironmentCheckpointV1) -> Result<(), ControllerError> {
            Ok(())
        }
        fn fork_boxed(&self) -> Result<Box<dyn EnvironmentBackend>, ControllerError> {
            Ok(Box::new(self.clone()))
        }
        fn export_replay(&self) -> Result<AuthoritativeReplayV1, ControllerError> {
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
    fn checkpoint_closes_state_status_and_limit_counters() {
        let checkpoint = EnvironmentCheckpointV1::new(
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
        let error = EnvironmentCheckpointV1::new(
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
        let mut checkpoint = EnvironmentCheckpointV1::new(
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
    fn two_player_endpoints_can_remain_alive_simultaneously() {
        let controller = TrustedEnvironmentController::new(FakeBackend::new());
        let player_one = controller.bind_player(PlayerId(1)).unwrap();
        let player_two = controller.bind_player(PlayerId(2)).unwrap();
        assert_eq!(player_one.observation().unwrap().perspective, PlayerId(1));
        assert_eq!(player_two.observation().unwrap().perspective, PlayerId(2));
        assert_eq!(player_one.observation().unwrap().perspective, PlayerId(1));
    }
}
