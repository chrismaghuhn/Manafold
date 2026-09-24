//! Shared player-safe projections used by every environment backend.
//!
//! This module is deliberately backend-neutral: it accepts an authoritative
//! state snapshot and a bound perspective, then constructs only the public
//! observation, information, decision, and step products.

use base64::{engine::general_purpose::STANDARD, Engine as _};
use mtgml_decision::PlayerDecisionRequestV2;
use mtgml_model::{EpisodeStatus, PlayerId};
use mtgml_observation::{
    InformationStateDigestInputV2, ObservationEnvelope, ObservedEventEnvelopeV2,
    PlayerInformationStateV2, PlayerKnowledgeCauseV1, PlayerKnowledgeChannelV1,
    PlayerKnowledgeInvalidationReasonV1, PlayerKnowledgeProvenanceV1, PlayerKnownLocationFactV1,
    PlayerKnownLocationV1, PlayerKnownObjectV1, PlayerStepSubmissionV1, PlayerStepV2,
    SyntheticM3BeginningStep, SyntheticM3CombatStep, SyntheticM3EndingStep, SyntheticM3Observation,
    SyntheticM3Priority, SyntheticM3TurnPosition, INFORMATION_STATE_SCHEMA_V2, OBSERVATION_SCHEMA,
    PLAYER_STEP_SCHEMA_V2, SYNTHETIC_M3_OBSERVATION_SCHEMA,
};
#[cfg(test)]
use mtgml_observation::{
    MagicM3CompletedOrder, MagicM3Observation, MagicM3PendingSbaOrdering,
    MAGIC_M3_OBSERVATION_SCHEMA,
};
#[cfg(test)]
use mtgml_state::ContinuationPayloadV2;
use mtgml_state::{
    BeginningStep, CombatStep, EndingStep, EngineState, KnowledgeAcquisitionCause,
    KnowledgeAcquisitionReason, KnowledgeHistoryChannel, KnowledgeInvalidationReason,
    PriorityState, TurnPosition,
};

use crate::endpoint::PlayerEndpointError;
use crate::errors::{ControllerError, EnvironmentCommitError};

const SYNTHETIC_M3_OBSERVATION_CODEC: &str = SYNTHETIC_M3_OBSERVATION_SCHEMA;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ObservationProjectionProfile {
    SyntheticM3,
    #[cfg(test)]
    MagicM3,
}

pub(crate) fn project_observation(
    state: &EngineState,
    perspective: PlayerId,
) -> Result<ObservationEnvelope, PlayerEndpointError> {
    project_observation_with_profile(
        state,
        perspective,
        ObservationProjectionProfile::SyntheticM3,
    )
}

pub(crate) fn project_observation_with_profile(
    state: &EngineState,
    perspective: PlayerId,
    profile: ObservationProjectionProfile,
) -> Result<ObservationEnvelope, PlayerEndpointError> {
    if !state.core.players.contains_key(&perspective) {
        return Err(PlayerEndpointError::ServiceUnavailable);
    }
    let (codec, payload) = match profile {
        ObservationProjectionProfile::SyntheticM3 => {
            let value = SyntheticM3Observation {
                schema_version: SYNTHETIC_M3_OBSERVATION_SCHEMA.into(),
                active_player: state.core.active_player,
                turn_number: state.core.turn_number.to_string(),
                turn_position: public_turn_position(state.core.position),
                priority: public_priority(state.core.priority),
            };
            (
                SYNTHETIC_M3_OBSERVATION_CODEC,
                mtgml_wire::encode_canonical(&value),
            )
        }
        #[cfg(test)]
        ObservationProjectionProfile::MagicM3 => {
            let value = MagicM3Observation {
                schema_version: MAGIC_M3_OBSERVATION_SCHEMA.into(),
                active_player: state.core.active_player,
                turn_number: state.core.turn_number.to_string(),
                turn_position: public_turn_position(state.core.position),
                priority: public_priority(state.core.priority),
                pending_sba_ordering: project_sba_ordering(state, perspective)?,
            };
            value
                .validate()
                .map_err(|_| PlayerEndpointError::ServiceUnavailable)?;
            (
                MAGIC_M3_OBSERVATION_SCHEMA,
                mtgml_wire::encode_canonical(&value),
            )
        }
    };
    let payload = payload.map_err(|_| PlayerEndpointError::ServiceUnavailable)?;
    let observation = ObservationEnvelope {
        schema_version: OBSERVATION_SCHEMA.into(),
        perspective,
        state_revision: state.revision,
        payload_codec: codec.into(),
        payload_base64: STANDARD.encode(&payload),
        digest: mtgml_model::ObservationDigest::from_canonical_bytes(&payload),
    };
    observation
        .validate()
        .map_err(|_| PlayerEndpointError::ServiceUnavailable)?;
    Ok(observation)
}

#[cfg(test)]
fn project_sba_ordering(
    state: &EngineState,
    perspective: PlayerId,
) -> Result<Option<MagicM3PendingSbaOrdering>, PlayerEndpointError> {
    let mut matching = state
        .execution
        .continuations
        .values()
        .filter(|continuation| {
            matches!(
                continuation.payload,
                ContinuationPayloadV2::MagicSbaGraveyardOrderV1 { .. }
            )
        });
    let Some(continuation) = matching.next() else {
        return Ok(None);
    };
    if matching.next().is_some() {
        return Err(PlayerEndpointError::ServiceUnavailable);
    }
    let ContinuationPayloadV2::MagicSbaGraveyardOrderV1 {
        apnap_owners,
        next_owner_index,
        completed_owner_orders,
        ..
    } = &continuation.payload
    else {
        unreachable!()
    };
    let next_order_owner = apnap_owners
        .get(
            usize::try_from(*next_owner_index)
                .map_err(|_| PlayerEndpointError::ServiceUnavailable)?,
        )
        .copied()
        .ok_or(PlayerEndpointError::ServiceUnavailable)?;
    let identity = state
        .perspective_identities
        .players
        .get(&perspective)
        .ok_or(PlayerEndpointError::ServiceUnavailable)?;
    let completed_orders = completed_owner_orders
        .iter()
        .map(|order| {
            let ordered_objects = order
                .top_to_bottom
                .iter()
                .map(|object| {
                    identity
                        .object_to_opaque
                        .get(object)
                        .copied()
                        .ok_or(PlayerEndpointError::ServiceUnavailable)
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(MagicM3CompletedOrder {
                owner: order.owner,
                ordered_objects,
            })
        })
        .collect::<Result<Vec<_>, PlayerEndpointError>>()?;
    Ok(Some(MagicM3PendingSbaOrdering {
        completed_orders,
        next_order_owner,
    }))
}

fn public_location(location: &mtgml_state::ZoneLocation) -> PlayerKnownLocationV1 {
    PlayerKnownLocationV1 {
        zone: location.zone,
        player: location.player,
    }
}

fn public_fact(fact: &mtgml_state::KnownLocationFactV2) -> PlayerKnownLocationFactV1 {
    PlayerKnownLocationFactV1 {
        location: public_location(&fact.location),
        provenance: public_provenance(&fact.provenance),
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
                KnowledgeAcquisitionCause::ExplicitReveal => PlayerKnowledgeCauseV1::ExplicitReveal,
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

fn public_history(records: &[mtgml_state::KnownLocationFactV2]) -> Vec<PlayerKnownLocationFactV1> {
    records.iter().map(public_fact).collect()
}

pub(crate) fn project_information_state(
    state: &EngineState,
    perspective: PlayerId,
) -> Result<PlayerInformationStateV2, PlayerEndpointError> {
    project_information_state_with_profile(
        state,
        perspective,
        ObservationProjectionProfile::SyntheticM3,
    )
}

pub(crate) fn project_information_state_with_profile(
    state: &EngineState,
    perspective: PlayerId,
    profile: ObservationProjectionProfile,
) -> Result<PlayerInformationStateV2, PlayerEndpointError> {
    if !state.core.players.contains_key(&perspective) {
        return Err(PlayerEndpointError::ServiceUnavailable);
    }
    let current_observation = project_observation_with_profile(state, perspective, profile)?;
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
                current_known_location_fact: record.known_location.as_ref().map(public_fact),
                historical_locations: public_history(&record.historical_locations),
                acquisition: public_provenance(&record.acquisition),
            },
        ));
    }
    for record in knowledge.retired.values() {
        retained_knowledge.push((
            record.opaque_object,
            PlayerKnownObjectV1::Retired {
                opaque_object_id: record.opaque_object,
                known_definition: record.card_definition,
                last_known_location_fact: record.last_known_location.as_ref().map(public_fact),
                historical_locations: public_history(&record.historical_locations),
                acquisition: public_provenance(&record.acquisition),
                invalidation: mtgml_observation::PlayerKnowledgeInvalidationV1 {
                    provenance: public_provenance(&record.invalidation.provenance),
                    reason: public_invalidation_reason(&record.invalidation.reason),
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
        digest: mtgml_model::InformationStateDigestV2::from_canonical_bytes(
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

pub(crate) fn project_visible_decision(
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

pub(crate) fn project_player_step(
    state: &EngineState,
    perspective: PlayerId,
    status: EpisodeStatus,
    submission: PlayerStepSubmissionV1,
) -> Result<PlayerStepV2, PlayerEndpointError> {
    project_player_step_with_profile(
        state,
        perspective,
        status,
        submission,
        ObservationProjectionProfile::SyntheticM3,
    )
}

pub(crate) fn project_player_step_with_profile(
    state: &EngineState,
    perspective: PlayerId,
    status: EpisodeStatus,
    submission: PlayerStepSubmissionV1,
    profile: ObservationProjectionProfile,
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
        information_state: project_information_state_with_profile(state, perspective, profile)?,
        observed_events: Vec::<ObservedEventEnvelopeV2>::new(),
        next_decision,
        status,
        submission,
    };
    step.validate()
        .map_err(|_| PlayerEndpointError::ServiceUnavailable)?;
    Ok(step)
}

pub(crate) fn validate_candidate_projections(state: &EngineState) -> Result<(), ControllerError> {
    for perspective in state.core.players.keys().copied() {
        project_observation(state, perspective).map_err(|_| {
            ControllerError::EnvironmentCommit(EnvironmentCommitError::PlayerProjectionInvalid)
        })?;
        project_information_state(state, perspective).map_err(|_| {
            ControllerError::EnvironmentCommit(EnvironmentCommitError::PlayerProjectionInvalid)
        })?;
        project_visible_decision(state, perspective).map_err(|_| {
            ControllerError::EnvironmentCommit(EnvironmentCommitError::PlayerProjectionInvalid)
        })?;
    }
    Ok(())
}
