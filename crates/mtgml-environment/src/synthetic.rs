use std::collections::BTreeMap;
use std::collections::BTreeSet;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use mtgml_decision::{DecisionResponseV2, PlayerDecisionRequestV2};
use mtgml_model::{
    EpisodeStatus, InformationStateDigestV2, ObservationDigest, PlayerId, StateRevision,
};
use mtgml_observation::{
    InformationStateDigestInputV2, ObservationEnvelope, ObservedEventEnvelopeV2,
    PlayerInformationStateV2, PlayerKnowledgeCauseV1, PlayerKnowledgeChannelV1,
    PlayerKnowledgeInvalidationReasonV1, PlayerKnowledgeProvenanceV1, PlayerKnownLocationFactV1,
    PlayerKnownLocationV1, PlayerKnownObjectV1, PlayerStepSubmissionV1, PlayerStepV2,
    PlayerSubmissionCodeV1, INFORMATION_STATE_SCHEMA_V2, OBSERVATION_SCHEMA, PLAYER_STEP_SCHEMA_V2,
};
use mtgml_random::RootSeed256;
use mtgml_replay::{
    AuthoritativeReplayV3, DeckIdentityV1, InitialEnvironmentIdentityV3, KernelIdentityV1,
    RandomnessIdentityV2, ReplayManifestV3, ReplayRecorderV3, ReplaySchemaVersionsV1, ReplayStepV3,
    REPLAY_MANIFEST_SCHEMA_V3,
};
use mtgml_rules::{
    validate_transition_contract, RulesKernel, SyntheticM1RulesKernel, TransitionResult,
};
use mtgml_state::{
    construct_synthetic_engine_state, EngineState, KnowledgeAcquisitionCause,
    KnowledgeAcquisitionReason, KnowledgeHistoryChannel, KnowledgeInvalidationReason,
    SyntheticResetInputs,
};

use crate::checkpoint::{
    CheckpointCodecIdentity, EnvironmentCheckpointV3, EnvironmentLimitCounters,
};
use crate::controller::EnvironmentBackend;
use crate::endpoint::PlayerEndpointError;
use crate::errors::{ControllerError, EnvironmentCommitError};

const SYNTHETIC_M2_OBSERVATION_CODEC: &str = "synthetic-m2-observation.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntheticM1EnvironmentConfig {
    pub codec: CheckpointCodecIdentity,
    pub replay: SyntheticM1ReplayConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntheticM1ReplayConfig {
    pub engine_build: String,
    pub kernel: KernelIdentityV1,
    pub rules_snapshot: String,
    pub format_policy_snapshot: String,
    pub oracle_snapshot: String,
    pub card_bundle: String,
    pub randomness_contract_id: String,
    pub schemas: ReplaySchemaVersionsV1,
    pub decks: Vec<DeckIdentityV1>,
}

pub struct SyntheticM1EnvironmentBackend {
    state: EngineState,
    status: EpisodeStatus,
    limit_counters: EnvironmentLimitCounters,
    codec: CheckpointCodecIdentity,
    config: SyntheticM1EnvironmentConfig,
    replay: ReplayRecorderV3,
    kernel: SyntheticM1RulesKernel,
}

impl SyntheticM1EnvironmentBackend {
    pub fn new(
        players: [PlayerId; 2],
        root_seed: RootSeed256,
        config: SyntheticM1EnvironmentConfig,
    ) -> Result<Self, ControllerError> {
        let state = construct_synthetic_engine_state(SyntheticResetInputs { players, root_seed })?;
        let status = EpisodeStatus::Running;
        let limit_counters = EnvironmentLimitCounters::default();
        let checkpoint = EnvironmentCheckpointV3::new(
            state.clone(),
            status.clone(),
            limit_counters.clone(),
            config.codec.clone(),
        )?;
        let replay = ReplayRecorderV3::new(build_manifest(&config, &checkpoint)?)?;
        Ok(Self {
            state,
            status,
            limit_counters,
            codec: config.codec.clone(),
            config,
            replay,
            kernel: SyntheticM1RulesKernel,
        })
    }

    pub fn from_checkpoint(
        checkpoint: EnvironmentCheckpointV3,
        config: SyntheticM1EnvironmentConfig,
    ) -> Result<Self, ControllerError> {
        checkpoint.validate()?;
        if checkpoint.codec != config.codec {
            return Err(ControllerError::UnsupportedCheckpointCodec);
        }
        // A structurally valid generic EngineState may still express
        // decisions this synthetic kernel cannot execute; such checkpoints
        // are rejected before any player projection can expose them.
        mtgml_rules::validate_synthetic_runtime_state(&checkpoint.state)
            .map_err(|_| ControllerError::UnsupportedSyntheticState)?;
        let replay = ReplayRecorderV3::new(build_manifest(&config, &checkpoint)?)?;
        Ok(Self {
            state: checkpoint.state,
            status: checkpoint.status,
            limit_counters: checkpoint.limit_counters,
            codec: checkpoint.codec,
            config,
            replay,
            kernel: SyntheticM1RulesKernel,
        })
    }

    fn current_checkpoint(&self) -> Result<EnvironmentCheckpointV3, ControllerError> {
        Ok(EnvironmentCheckpointV3::new(
            self.state.clone(),
            self.status.clone(),
            self.limit_counters.clone(),
            self.codec.clone(),
        )?)
    }

    fn checked_add_counter(
        value: u64,
        increment: u64,
        counter: &'static str,
    ) -> Result<u64, ControllerError> {
        value
            .checked_add(increment)
            .ok_or(ControllerError::CounterOverflow { counter })
    }

    fn candidate_counters(
        before: &EnvironmentLimitCounters,
        event_count: usize,
    ) -> Result<EnvironmentLimitCounters, ControllerError> {
        let event_count =
            u64::try_from(event_count).map_err(|_| ControllerError::CounterOverflow {
                counter: "rule_events_emitted",
            })?;
        Ok(EnvironmentLimitCounters {
            decisions_submitted: Self::checked_add_counter(
                before.decisions_submitted,
                1,
                "decisions_submitted",
            )?,
            accepted_transitions: Self::checked_add_counter(
                before.accepted_transitions,
                1,
                "accepted_transitions",
            )?,
            rule_events_emitted: Self::checked_add_counter(
                before.rule_events_emitted,
                event_count,
                "rule_events_emitted",
            )?,
            resource_units_consumed: before.resource_units_consumed,
            wall_clock_elapsed_millis: before.wall_clock_elapsed_millis,
        })
    }

    pub(crate) fn execute_response<F>(
        &mut self,
        actor: PlayerId,
        response: DecisionResponseV2,
        before_commit: F,
    ) -> Result<TransitionResult, ControllerError>
    where
        F: FnOnce(
            &EnvironmentCheckpointV3,
            &TransitionResult,
            &BTreeMap<PlayerId, Vec<ObservedEventEnvelopeV2>>,
        ) -> Result<(), ControllerError>,
    {
        let before = self.current_checkpoint()?;
        let transition = self.kernel.apply(&before.state, actor, &response)?;
        validate_transition_contract(&before.state, &transition)?;

        if !transition.accepted {
            let after = self.current_checkpoint()?;
            if after != before {
                return Err(EnvironmentCommitError::RejectedMutation.into());
            }
            return Ok(transition);
        }

        let candidate_counters =
            Self::candidate_counters(&before.limit_counters, transition.events.len())?;
        let candidate = EnvironmentCheckpointV3::new(
            transition.next_state.clone(),
            transition.status.clone(),
            candidate_counters,
            before.codec.clone(),
        )?;
        if candidate.state != transition.next_state || candidate.status != transition.status {
            return Err(EnvironmentCommitError::CandidateMismatch.into());
        }

        let step_index = u64::try_from(self.replay.step_count()).map_err(|_| {
            ControllerError::CounterOverflow {
                counter: "replay_step_index",
            }
        })?;
        let step = ReplayStepV3 {
            step_index,
            actor,
            checkpoint_digest_before: before.checkpoint_digest.clone(),
            state_revision_before: before.state.revision,
            response,
            accepted: true,
            state_revision_after: candidate.state.revision,
            full_state_digest_after: candidate.state_digest.clone(),
            episode_status_after: candidate.status.clone(),
            environment_limit_counters_after: candidate.limit_counters.clone(),
            checkpoint_digest_after: candidate.checkpoint_digest.clone(),
        };
        let mut candidate_replay = self.replay.clone();
        candidate_replay.append(step)?;
        candidate_replay.export()?;
        // ADR-0040: every required per-perspective projection is validated
        // against the candidate product BEFORE the atomic commit.
        let occurrence_envelopes = crate::lifecycle_projection::project_occurrence_envelopes(
            &before.state,
            &transition.next_state,
            &transition.events,
        )
        .map_err(|_| {
            ControllerError::EnvironmentCommit(EnvironmentCommitError::PlayerProjectionInvalid)
        })?;
        before_commit(&candidate, &transition, &occurrence_envelopes)?;

        self.state = candidate.state;
        self.status = candidate.status;
        self.limit_counters = candidate.limit_counters;
        self.replay = candidate_replay;
        Ok(transition)
    }

    fn require_player(&self, perspective: PlayerId) -> Result<(), PlayerEndpointError> {
        self.state
            .core
            .players
            .contains_key(&perspective)
            .then_some(())
            .ok_or(PlayerEndpointError::ServiceUnavailable)
    }

    fn synthetic_observation(
        perspective: PlayerId,
        revision: StateRevision,
    ) -> Result<ObservationEnvelope, PlayerEndpointError> {
        let payload = format!(
            "{SYNTHETIC_M2_OBSERVATION_CODEC}|perspective={}|state-revision={}",
            perspective.0, revision.0
        )
        .into_bytes();
        let observation = ObservationEnvelope {
            schema_version: OBSERVATION_SCHEMA.into(),
            perspective,
            state_revision: revision,
            payload_codec: SYNTHETIC_M2_OBSERVATION_CODEC.into(),
            payload_base64: STANDARD.encode(&payload),
            digest: ObservationDigest::from_canonical_bytes(&payload),
        };
        observation
            .validate()
            .map_err(|_| PlayerEndpointError::ServiceUnavailable)?;
        Ok(observation)
    }

    fn public_location(location: &mtgml_state::ZoneLocation) -> PlayerKnownLocationV1 {
        PlayerKnownLocationV1 {
            zone: location.zone,
            player: location.player,
        }
    }

    fn public_fact(fact: &mtgml_state::KnownLocationFactV2) -> PlayerKnownLocationFactV1 {
        PlayerKnownLocationFactV1 {
            location: Self::public_location(&fact.location),
            provenance: Self::public_provenance(&fact.provenance),
        }
    }

    fn public_provenance(reason: &KnowledgeAcquisitionReason) -> PlayerKnowledgeProvenanceV1 {
        match reason {
            KnowledgeAcquisitionReason::InitialConfiguration => {
                PlayerKnowledgeProvenanceV1::InitialConfiguration
            }
            KnowledgeAcquisitionReason::Observed {
                channel,
                sequence,
                cause,
            } => PlayerKnowledgeProvenanceV1::Observed {
                channel: match channel {
                    KnowledgeHistoryChannel::Public => PlayerKnowledgeChannelV1::Public,
                    KnowledgeHistoryChannel::Private => PlayerKnowledgeChannelV1::Private,
                },
                sequence: *sequence,
                cause: match cause {
                    KnowledgeAcquisitionCause::PublicEvent => PlayerKnowledgeCauseV1::PublicEvent,
                    KnowledgeAcquisitionCause::PrivateLook => PlayerKnowledgeCauseV1::PrivateLook,
                    KnowledgeAcquisitionCause::ExplicitReveal => {
                        PlayerKnowledgeCauseV1::ExplicitReveal
                    }
                    KnowledgeAcquisitionCause::OwnPrivateIdentity => {
                        PlayerKnowledgeCauseV1::OwnPrivateIdentity
                    }
                },
            },
        }
    }

    fn public_invalidation_reason(
        reason: &KnowledgeInvalidationReason,
    ) -> PlayerKnowledgeInvalidationReasonV1 {
        match reason {
            KnowledgeInvalidationReason::Shuffle => PlayerKnowledgeInvalidationReasonV1::Shuffle,
            KnowledgeInvalidationReason::Randomization => {
                PlayerKnowledgeInvalidationReasonV1::Randomization
            }
            KnowledgeInvalidationReason::HiddenTransition => {
                PlayerKnowledgeInvalidationReasonV1::HiddenTransition
            }
            KnowledgeInvalidationReason::ExplicitForget => {
                PlayerKnowledgeInvalidationReasonV1::ExplicitForget
            }
        }
    }

    fn public_history(
        records: &[mtgml_state::KnownLocationFactV2],
    ) -> Vec<PlayerKnownLocationFactV1> {
        records.iter().map(Self::public_fact).collect()
    }

    fn player_information_state_from_state(
        state: &EngineState,
        perspective: PlayerId,
    ) -> Result<PlayerInformationStateV2, PlayerEndpointError> {
        if !state.core.players.contains_key(&perspective) {
            return Err(PlayerEndpointError::ServiceUnavailable);
        }
        let current_observation = Self::synthetic_observation(perspective, state.revision)?;
        let knowledge = state
            .knowledge
            .players
            .get(&perspective)
            .ok_or(PlayerEndpointError::ServiceUnavailable)?;
        // Canonical retained-knowledge order is ascending numeric OpaqueObjectId
        // across active and retired records (INFORMATION_MODEL.md).
        let mut retained_knowledge =
            Vec::with_capacity(knowledge.active.len() + knowledge.retired.len());
        for record in knowledge.active.values() {
            retained_knowledge.push((
                record.opaque_object,
                PlayerKnownObjectV1::Active {
                    opaque_object_id: record.opaque_object,
                    known_definition: record.card_definition,
                    current_known_location_fact: record
                        .known_location
                        .as_ref()
                        .map(Self::public_fact),
                    historical_locations: Self::public_history(&record.historical_locations),
                    acquisition: Self::public_provenance(&record.acquisition),
                },
            ));
        }
        for record in knowledge.retired.values() {
            retained_knowledge.push((
                record.opaque_object,
                PlayerKnownObjectV1::Retired {
                    opaque_object_id: record.opaque_object,
                    known_definition: record.card_definition,
                    last_known_location_fact: record
                        .last_known_location
                        .as_ref()
                        .map(Self::public_fact),
                    historical_locations: Self::public_history(&record.historical_locations),
                    acquisition: Self::public_provenance(&record.acquisition),
                    invalidation: mtgml_observation::PlayerKnowledgeInvalidationV1 {
                        provenance: Self::public_provenance(&record.invalidation.provenance),
                        reason: Self::public_invalidation_reason(&record.invalidation.reason),
                    },
                },
            ));
        }
        retained_knowledge.sort_by_key(|(opaque, _)| *opaque);
        let retained_knowledge: Vec<_> = retained_knowledge
            .into_iter()
            .map(|(_, object)| object)
            .collect();

        let mut information_state = PlayerInformationStateV2 {
            schema_version: INFORMATION_STATE_SCHEMA_V2.into(),
            perspective,
            state_revision: state.revision,
            current_observation,
            next_visible_sequence: knowledge.next_visible_sequence,
            retained_knowledge,
            digest: InformationStateDigestV2::from_canonical_bytes(
                b"m2-information-state-placeholder",
            ),
        };
        let input: InformationStateDigestInputV2 = information_state.digest_input();
        let (_, digest) = mtgml_wire::compute_information_state_digest_v2(&input)
            .map_err(|_| PlayerEndpointError::ServiceUnavailable)?;
        information_state.digest = digest;
        information_state
            .validate()
            .map_err(|_| PlayerEndpointError::ServiceUnavailable)?;
        Ok(information_state)
    }
}

impl EnvironmentBackend for SyntheticM1EnvironmentBackend {
    fn players(&self) -> Vec<PlayerId> {
        self.state.core.players.keys().copied().collect()
    }

    fn checkpoint(&self) -> Result<EnvironmentCheckpointV3, ControllerError> {
        self.current_checkpoint()
    }

    fn restore(&mut self, checkpoint: EnvironmentCheckpointV3) -> Result<(), ControllerError> {
        let candidate = Self::from_checkpoint(checkpoint, self.config.clone())?;
        self.state = candidate.state;
        self.status = candidate.status;
        self.limit_counters = candidate.limit_counters;
        self.codec = candidate.codec;
        self.replay = candidate.replay;
        self.kernel = SyntheticM1RulesKernel;
        Ok(())
    }

    fn fork_boxed(&self) -> Result<Box<dyn EnvironmentBackend>, ControllerError> {
        let checkpoint = self.current_checkpoint()?;
        Ok(Box::new(Self::from_checkpoint(
            checkpoint,
            self.config.clone(),
        )?))
    }

    fn export_replay(&self) -> Result<AuthoritativeReplayV3, ControllerError> {
        Ok(self.replay.export()?)
    }

    fn execute_trusted_response(
        &mut self,
        actor: PlayerId,
        response: DecisionResponseV2,
    ) -> Result<TransitionResult, ControllerError> {
        self.execute_response(
            actor,
            response,
            |_candidate, _transition, _envelopes| Ok(()),
        )
    }

    fn player_observation(
        &self,
        perspective: PlayerId,
    ) -> Result<ObservationEnvelope, PlayerEndpointError> {
        self.require_player(perspective)?;
        Self::synthetic_observation(perspective, self.state.revision)
    }

    fn player_information_state(
        &self,
        perspective: PlayerId,
    ) -> Result<PlayerInformationStateV2, PlayerEndpointError> {
        self.require_player(perspective)?;
        Self::player_information_state_from_state(&self.state, perspective)
    }

    fn player_visible_decision(
        &self,
        perspective: PlayerId,
    ) -> Result<Option<PlayerDecisionRequestV2>, PlayerEndpointError> {
        self.require_player(perspective)?;
        let Some(pending) = self.state.execution.pending_decision.as_ref() else {
            return Ok(None);
        };
        if pending.request.actor != perspective {
            return Ok(None);
        }
        pending
            .request
            .project_player_request()
            .map(Some)
            .map_err(|_| PlayerEndpointError::ServiceUnavailable)
    }

    /// Ordered typed-submission pipeline (DECISION_PROTOCOL validation
    /// order). Layer-B rejections return `Ok` with a mirrored unchanged
    /// product carrying only the closed rejected outcome; anything else maps
    /// to the closed service failure.
    fn submit_player_response(
        &mut self,
        perspective: PlayerId,
        response: DecisionResponseV2,
    ) -> Result<PlayerStepV2, PlayerEndpointError> {
        use mtgml_observation::PlayerStepSubmissionV1 as Submission;

        self.require_player(perspective)?;
        // 2. episode availability.
        if !matches!(&self.status, EpisodeStatus::Running) {
            let step = Self::player_step_from_state(
                &self.state,
                perspective,
                self.status.clone(),
                Submission::Rejected {
                    code: PlayerSubmissionCodeV1::EpisodeClosed,
                },
            )?;
            step.validate()
                .map_err(|_| PlayerEndpointError::ServiceUnavailable)?;
            return Ok(step);
        }
        // 3. request availability for this perspective (non-disclosing).
        let Some(pending) = self.state.execution.pending_decision.as_ref() else {
            let step = Self::player_step_from_state(
                &self.state,
                perspective,
                self.status.clone(),
                Submission::Rejected {
                    code: PlayerSubmissionCodeV1::UnavailableDecision,
                },
            )?;
            step.validate()
                .map_err(|_| PlayerEndpointError::ServiceUnavailable)?;
            return Ok(step);
        };
        if pending.request.actor != perspective {
            let step = Self::player_step_from_state(
                &self.state,
                perspective,
                self.status.clone(),
                Submission::Rejected {
                    code: PlayerSubmissionCodeV1::UnavailableDecision,
                },
            )?;
            step.validate()
                .map_err(|_| PlayerEndpointError::ServiceUnavailable)?;
            return Ok(step);
        }
        // 4. visible request projection.
        let visible_request = pending
            .request
            .project_player_request()
            .map_err(|_| PlayerEndpointError::ServiceUnavailable)?;

        // 5.-11. identity/revision/variant/membership/uniqueness/canonical/
        // cardinality/numeric classification via the decision authority.
        let code = match response.validate_for(&visible_request) {
            Ok(()) => None,
            Err(mtgml_decision::DecisionValidationError::DecisionIdentityMismatch)
            | Err(mtgml_decision::DecisionValidationError::StateRevisionMismatch) => {
                Some(PlayerSubmissionCodeV1::StaleDecision)
            }
            Err(mtgml_decision::DecisionValidationError::AnswerDomainMismatch) => {
                Some(PlayerSubmissionCodeV1::InvalidAnswer)
            }
            Err(mtgml_decision::DecisionValidationError::UnknownCandidate) => {
                Some(PlayerSubmissionCodeV1::InvalidCandidate)
            }
            Err(mtgml_decision::DecisionValidationError::DuplicateAnswerCandidate) => {
                Some(PlayerSubmissionCodeV1::DuplicateAssignment)
            }
            Err(mtgml_decision::DecisionValidationError::NoncanonicalAnswer) => {
                Some(PlayerSubmissionCodeV1::InvalidOrder)
            }
            Err(mtgml_decision::DecisionValidationError::AnswerCardinality) => {
                Some(PlayerSubmissionCodeV1::InvalidCardinality)
            }
            Err(mtgml_decision::DecisionValidationError::NumericOutOfBounds) => {
                Some(PlayerSubmissionCodeV1::InvalidNumber)
            }
            Err(_) => Some(PlayerSubmissionCodeV1::InvalidAnswer),
        };
        if let Some(code) = code {
            // Layer B: mirror the unchanged committed product.
            let step = Self::player_step_from_state(
                &self.state,
                perspective,
                self.status.clone(),
                Submission::Rejected { code },
            )?;
            step.validate()
                .map_err(|_| PlayerEndpointError::ServiceUnavailable)?;
            return Ok(step);
        }

        // 12./13. trusted binding/context and kernel execution. A kernel that
        // still reports `accepted=false` after a fully accepted public
        // submission is an internal soundness failure, not player illegality.
        let mut projected_step = None;
        let transition = self
            .execute_response(
                perspective,
                response,
                |candidate, transition, occurrence_envelopes| {
                    let step = Self::player_step_from_state(
                        &candidate.state,
                        perspective,
                        transition.status.clone(),
                        Submission::Accepted,
                    )
                    .map_err(|_| {
                        ControllerError::EnvironmentCommit(
                            EnvironmentCommitError::PlayerProjectionInvalid,
                        )
                    })?;
                    let mut step = step;
                    // Attach the per-perspective observed batch validated above.
                    if let Some(envelopes) = occurrence_envelopes.get(&perspective) {
                        step.observed_events = envelopes.clone();
                    }
                    projected_step = Some(step);
                    Ok(())
                },
            )
            .map_err(|_| PlayerEndpointError::ServiceUnavailable)?;
        if !transition.accepted {
            return Err(PlayerEndpointError::ServiceUnavailable);
        }
        projected_step.ok_or(PlayerEndpointError::ServiceUnavailable)
    }
}

impl SyntheticM1EnvironmentBackend {
    fn player_step_from_state(
        state: &EngineState,
        perspective: PlayerId,
        status: EpisodeStatus,
        submission: PlayerStepSubmissionV1,
    ) -> Result<PlayerStepV2, PlayerEndpointError> {
        let next_decision = state
            .execution
            .pending_decision
            .as_ref()
            .filter(|pending| pending.request.actor == perspective)
            .map(|pending| pending.request.project_player_request())
            .transpose()
            .map_err(|_| PlayerEndpointError::ServiceUnavailable)?;
        let step = PlayerStepV2 {
            schema_version: PLAYER_STEP_SCHEMA_V2.into(),
            information_state: Self::player_information_state_from_state(state, perspective)?,
            observed_events: Vec::<ObservedEventEnvelopeV2>::new(),
            next_decision,
            status,
            submission,
        };
        step.validate()
            .map_err(|_| PlayerEndpointError::ServiceUnavailable)?;
        Ok(step)
    }
}

fn build_manifest(
    config: &SyntheticM1EnvironmentConfig,
    checkpoint: &EnvironmentCheckpointV3,
) -> Result<ReplayManifestV3, ControllerError> {
    let manifest = ReplayManifestV3 {
        schema_version: REPLAY_MANIFEST_SCHEMA_V3.into(),
        engine_build: config.replay.engine_build.clone(),
        kernel: config.replay.kernel.clone(),
        rules_snapshot: config.replay.rules_snapshot.clone(),
        format_policy_snapshot: config.replay.format_policy_snapshot.clone(),
        oracle_snapshot: config.replay.oracle_snapshot.clone(),
        card_bundle: config.replay.card_bundle.clone(),
        schemas: config.replay.schemas.clone(),
        randomness: RandomnessIdentityV2 {
            contract_id: config.replay.randomness_contract_id.clone(),
            root_seed_hex: checkpoint.state.random.root_seed.to_lower_hex(),
        },
        decks: config.replay.decks.clone(),
        initial_identity: InitialEnvironmentIdentityV3 {
            state_revision: checkpoint.state.revision,
            full_state_digest: checkpoint.state_digest.clone(),
            episode_status: checkpoint.status.clone(),
            environment_limit_counters: checkpoint.limit_counters.clone(),
            checkpoint_codec_identity: checkpoint.codec.clone(),
            checkpoint_digest: checkpoint.checkpoint_digest.clone(),
        },
    };
    manifest.validate()?;

    let state_players: BTreeSet<_> = checkpoint.state.core.players.keys().copied().collect();
    let manifest_players: BTreeSet<_> = manifest.decks.iter().map(|deck| deck.player).collect();
    if state_players.len() != 2
        || manifest.decks.len() != 2
        || state_players != manifest_players
        || state_players.len() != manifest.decks.len()
    {
        return Err(ControllerError::ReplayIdentityMismatch);
    }
    Ok(manifest)
}
