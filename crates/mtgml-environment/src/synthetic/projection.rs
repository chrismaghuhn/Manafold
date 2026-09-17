//! Ownership: player-safe read-only projection. PerspectiveIdentityState
//! stays the live mapping authority; knowledge stays retained memory; no
//! second mapping authority is introduced here.

use base64::{engine::general_purpose::STANDARD, Engine as _};
use mtgml_decision::PlayerDecisionRequestV2;
use mtgml_model::{EpisodeStatus, InformationStateDigestV2, ObservationDigest, PlayerId};
use mtgml_observation::{
    InformationStateDigestInputV2, ObservationEnvelope, ObservedEventEnvelopeV2,
    PlayerInformationStateV2, PlayerKnowledgeCauseV1, PlayerKnowledgeChannelV1,
    PlayerKnowledgeInvalidationReasonV1, PlayerKnowledgeProvenanceV1, PlayerKnownLocationFactV1,
    PlayerKnownLocationV1, PlayerKnownObjectV1, PlayerStepSubmissionV1, PlayerStepV2,
    SyntheticM3BeginningStep, SyntheticM3CombatStep, SyntheticM3EndingStep, SyntheticM3Observation,
    SyntheticM3Priority, SyntheticM3TurnPosition, INFORMATION_STATE_SCHEMA_V2, OBSERVATION_SCHEMA,
    PLAYER_STEP_SCHEMA_V2, SYNTHETIC_M3_OBSERVATION_SCHEMA,
};
use mtgml_state::{
    BeginningStep, CombatStep, EndingStep, EngineState, KnowledgeAcquisitionCause,
    KnowledgeAcquisitionReason, KnowledgeHistoryChannel, KnowledgeInvalidationReason,
    PriorityState, TurnPosition,
};

use super::SyntheticM1EnvironmentBackend;
use crate::endpoint::PlayerEndpointError;

const SYNTHETIC_M3_OBSERVATION_CODEC: &str = SYNTHETIC_M3_OBSERVATION_SCHEMA;

impl SyntheticM1EnvironmentBackend {
    pub(super) fn require_player(&self, perspective: PlayerId) -> Result<(), PlayerEndpointError> {
        self.state
            .core
            .players
            .contains_key(&perspective)
            .then_some(())
            .ok_or(PlayerEndpointError::ServiceUnavailable)
    }

    pub(crate) fn synthetic_observation(
        state: &EngineState,
        perspective: PlayerId,
    ) -> Result<ObservationEnvelope, PlayerEndpointError> {
        let payload_value = SyntheticM3Observation {
            schema_version: SYNTHETIC_M3_OBSERVATION_SCHEMA.into(),
            active_player: state.core.active_player,
            turn_number: state.core.turn_number.to_string(),
            turn_position: public_turn_position(state.core.position),
            priority: public_priority(state.core.priority),
        };
        let payload = mtgml_wire::encode_canonical(&payload_value)
            .map_err(|_| PlayerEndpointError::ServiceUnavailable)?;
        let observation = ObservationEnvelope {
            schema_version: OBSERVATION_SCHEMA.into(),
            perspective,
            state_revision: state.revision,
            payload_codec: SYNTHETIC_M3_OBSERVATION_CODEC.into(),
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

    pub(crate) fn player_information_state_from_state(
        state: &EngineState,
        perspective: PlayerId,
    ) -> Result<PlayerInformationStateV2, PlayerEndpointError> {
        if !state.core.players.contains_key(&perspective) {
            return Err(PlayerEndpointError::ServiceUnavailable);
        }
        let current_observation = Self::synthetic_observation(state, perspective)?;
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

fn public_turn_position(position: TurnPosition) -> SyntheticM3TurnPosition {
    match position {
        TurnPosition::Beginning { step } => SyntheticM3TurnPosition::Beginning {
            step: match step {
                BeginningStep::Untap => SyntheticM3BeginningStep::Untap,
                BeginningStep::Upkeep => SyntheticM3BeginningStep::Upkeep,
                BeginningStep::Draw => SyntheticM3BeginningStep::Draw,
            },
        },
        TurnPosition::PrecombatMain => SyntheticM3TurnPosition::PrecombatMain,
        TurnPosition::Combat { step } => SyntheticM3TurnPosition::Combat {
            step: match step {
                CombatStep::BeginningOfCombat => SyntheticM3CombatStep::BeginningOfCombat,
                CombatStep::DeclareAttackers => SyntheticM3CombatStep::DeclareAttackers,
                CombatStep::DeclareBlockers => SyntheticM3CombatStep::DeclareBlockers,
                CombatStep::CombatDamage => SyntheticM3CombatStep::CombatDamage,
                CombatStep::EndOfCombat => SyntheticM3CombatStep::EndOfCombat,
            },
        },
        TurnPosition::PostcombatMain => SyntheticM3TurnPosition::PostcombatMain,
        TurnPosition::Ending { step } => SyntheticM3TurnPosition::Ending {
            step: match step {
                EndingStep::EndStep => SyntheticM3EndingStep::EndStep,
                EndingStep::Cleanup => SyntheticM3EndingStep::Cleanup,
            },
        },
    }
}

fn public_priority(priority: PriorityState) -> SyntheticM3Priority {
    match priority {
        PriorityState::None => SyntheticM3Priority::None,
        PriorityState::HeldBy { player, .. } => SyntheticM3Priority::HeldBy { player },
    }
}

impl SyntheticM1EnvironmentBackend {
    /// Pure visible-decision projection over an explicit state: the exact
    /// logic the endpoint serves, reusable for pre-commit candidate
    /// validation without a committed backend.
    pub(crate) fn visible_decision_from_state(
        state: &EngineState,
        perspective: PlayerId,
    ) -> Result<Option<PlayerDecisionRequestV2>, PlayerEndpointError> {
        if !state.core.players.contains_key(&perspective) {
            return Err(PlayerEndpointError::ServiceUnavailable);
        }
        let Some(pending) = state.execution.pending_decision.as_ref() else {
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

    pub(super) fn player_step_from_state(
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
