//! Shared player-safe projections used by every environment backend.
//!
//! This module is deliberately backend-neutral: it accepts an authoritative
//! state snapshot and a bound perspective, then constructs only the public
//! observation, information, decision, and step products.

use base64::{engine::general_purpose::STANDARD, Engine as _};
use mtgml_decision::PlayerDecisionRequestV2;
use mtgml_model::{EpisodeStatus, ExecutionIdentityV1, ExecutionProgramV1, PlayerId};
use mtgml_observation::{
    InformationStateDigestInputV2, ObservationEnvelope, ObservedEventEnvelopeV2,
    PlayerInformationStateV2, PlayerKnowledgeCauseV1, PlayerKnowledgeChannelV1,
    PlayerKnowledgeInvalidationReasonV1, PlayerKnowledgeProvenanceV1, PlayerKnownLocationFactV1,
    PlayerKnownLocationV1, PlayerKnownObjectV1, PlayerStepSubmissionV1, PlayerStepV2,
    SyntheticBeginningStep, SyntheticCombatStep, SyntheticEndingStep, SyntheticObservation,
    SyntheticPriority, SyntheticTurnPosition, INFORMATION_STATE_SCHEMA_V2, OBSERVATION_SCHEMA,
    PLAYER_STEP_SCHEMA_V2, SYNTHETIC_OBSERVATION_SCHEMA_V1,
};
use mtgml_observation::{
    MagicBlockedStatusV4, MagicCombatBlockerAssignmentV2, MagicCombatBlockerAssignmentV3,
    MagicCombatBlockerAssignmentV4, MagicCombatParticipationV2, MagicCombatParticipationV3,
    MagicCombatParticipationV4, MagicCompletedOrder, MagicMarkedDamageV4, MagicObservation,
    MagicObservationV2, MagicObservationV3, MagicObservationV4, MagicPendingSbaOrdering,
    MagicPlayerLifeV4, MAGIC_OBSERVATION_SCHEMA_V1, MAGIC_OBSERVATION_SCHEMA_V2,
    MAGIC_OBSERVATION_SCHEMA_V3, MAGIC_OBSERVATION_SCHEMA_V4,
};
use mtgml_state::ContinuationPayloadV2;
use mtgml_state::{
    BeginningStep, CombatStep, EndingStep, EngineState, KnowledgeAcquisitionCause,
    KnowledgeAcquisitionReason, KnowledgeHistoryChannel, KnowledgeInvalidationReason,
    PriorityState, TurnPosition,
};

use crate::endpoint::PlayerEndpointError;
use crate::errors::{ControllerError, EnvironmentCommitError};
use crate::semantic_catalog_generated::{
    magic_combat_attackers_0_1_0_semantic_contract_id,
    magic_combat_blockers_0_1_0_semantic_contract_id,
    magic_combat_damage_0_1_0_semantic_contract_id,
    magic_s3_a_ordered_sba_0_1_0_semantic_contract_id,
    magic_s3_b_basic_priority_0_1_0_semantic_contract_id,
    magic_s3_c_draw_interaction_0_1_0_semantic_contract_id,
    magic_turn_structure_0_1_0_semantic_contract_id, synthetic_legacy_default_semantic_contract_id,
};

const SYNTHETIC_OBSERVATION_CODEC: &str = SYNTHETIC_OBSERVATION_SCHEMA_V1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ObservationProjectionProfile {
    Synthetic,
    Magic,
    MagicCombat,
    MagicCombatBlockers,
    MagicCombatDamage,
}

pub(crate) fn profile_for_execution_identity(
    identity: &ExecutionIdentityV1,
) -> Result<ObservationProjectionProfile, ControllerError> {
    if identity.program_kind == ExecutionProgramV1::SyntheticRulesCompat {
        return if identity.semantic_contract_id == synthetic_legacy_default_semantic_contract_id() {
            Ok(ObservationProjectionProfile::Synthetic)
        } else {
            Err(ControllerError::ProgramAuthorityMismatch)
        };
    }
    if identity.program_kind != ExecutionProgramV1::MagicRules {
        return Err(ControllerError::ProgramAuthorityMismatch);
    }
    if identity.semantic_contract_id == magic_turn_structure_0_1_0_semantic_contract_id() {
        Ok(ObservationProjectionProfile::Synthetic)
    } else if identity.semantic_contract_id == magic_s3_a_ordered_sba_0_1_0_semantic_contract_id()
        || identity.semantic_contract_id == magic_s3_b_basic_priority_0_1_0_semantic_contract_id()
        || identity.semantic_contract_id == magic_s3_c_draw_interaction_0_1_0_semantic_contract_id()
    {
        Ok(ObservationProjectionProfile::Magic)
    } else if identity.semantic_contract_id == magic_combat_attackers_0_1_0_semantic_contract_id() {
        Ok(ObservationProjectionProfile::MagicCombat)
    } else if identity.semantic_contract_id == magic_combat_blockers_0_1_0_semantic_contract_id() {
        Ok(ObservationProjectionProfile::MagicCombatBlockers)
    } else if identity.semantic_contract_id == magic_combat_damage_0_1_0_semantic_contract_id() {
        Ok(ObservationProjectionProfile::MagicCombatDamage)
    } else {
        Err(ControllerError::SemanticContractUnsupported)
    }
}

pub(crate) fn project_observation(
    state: &EngineState,
    perspective: PlayerId,
) -> Result<ObservationEnvelope, PlayerEndpointError> {
    project_observation_with_profile(state, perspective, ObservationProjectionProfile::Synthetic)
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
        ObservationProjectionProfile::Synthetic => {
            let value = SyntheticObservation {
                schema_version: SYNTHETIC_OBSERVATION_SCHEMA_V1.into(),
                active_player: state.core.active_player,
                turn_number: state.core.turn_number.to_string(),
                turn_position: public_turn_position(state.core.position),
                priority: public_priority(state.core.priority),
            };
            (
                SYNTHETIC_OBSERVATION_CODEC,
                mtgml_wire::encode_canonical(&value),
            )
        }
        ObservationProjectionProfile::Magic => {
            let value = MagicObservation {
                schema_version: MAGIC_OBSERVATION_SCHEMA_V1.into(),
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
                MAGIC_OBSERVATION_SCHEMA_V1,
                mtgml_wire::encode_canonical(&value),
            )
        }
        ObservationProjectionProfile::MagicCombat => {
            let value = MagicObservationV2 {
                schema_version: MAGIC_OBSERVATION_SCHEMA_V2.into(),
                active_player: state.core.active_player,
                turn_number: state.core.turn_number.to_string(),
                turn_position: public_turn_position(state.core.position),
                priority: public_priority(state.core.priority),
                pending_sba_ordering: project_sba_ordering(state, perspective)?,
                combat: project_combat(state, perspective)?,
            };
            value
                .validate()
                .map_err(|_| PlayerEndpointError::ServiceUnavailable)?;
            (
                MAGIC_OBSERVATION_SCHEMA_V2,
                mtgml_wire::encode_canonical(&value),
            )
        }
        ObservationProjectionProfile::MagicCombatBlockers => {
            let value = MagicObservationV3 {
                schema_version: MAGIC_OBSERVATION_SCHEMA_V3.into(),
                active_player: state.core.active_player,
                turn_number: state.core.turn_number.to_string(),
                turn_position: public_turn_position(state.core.position),
                priority: public_priority(state.core.priority),
                pending_sba_ordering: project_sba_ordering(state, perspective)?,
                combat: project_combat_v3(state, perspective)?,
            };
            value
                .validate()
                .map_err(|_| PlayerEndpointError::ServiceUnavailable)?;
            (
                MAGIC_OBSERVATION_SCHEMA_V3,
                mtgml_wire::encode_canonical(&value),
            )
        }
        ObservationProjectionProfile::MagicCombatDamage => {
            let value = MagicObservationV4 {
                schema_version: MAGIC_OBSERVATION_SCHEMA_V4.into(),
                active_player: state.core.active_player,
                turn_number: state.core.turn_number.to_string(),
                turn_position: public_turn_position(state.core.position),
                priority: public_priority(state.core.priority),
                player_life: state
                    .core
                    .players
                    .iter()
                    .map(|(player, status)| MagicPlayerLifeV4 {
                        player: *player,
                        life: status.life,
                        has_lost: status.has_lost,
                    })
                    .collect(),
                marked_damage: project_marked_damage_v4(state, perspective)?,
                pending_sba_ordering: project_sba_ordering(state, perspective)?,
                combat: project_combat_v4(state, perspective)?,
            };
            value
                .validate()
                .map_err(|_| PlayerEndpointError::ServiceUnavailable)?;
            (
                MAGIC_OBSERVATION_SCHEMA_V4,
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

fn project_combat(
    state: &EngineState,
    perspective: PlayerId,
) -> Result<Option<MagicCombatParticipationV2>, PlayerEndpointError> {
    let Some(combat) = state.combat.as_ref() else {
        return Ok(None);
    };
    let identity = state
        .perspective_identities
        .players
        .get(&perspective)
        .ok_or(PlayerEndpointError::ServiceUnavailable)?;
    let mut attackers = combat
        .attackers
        .iter()
        .map(|object| {
            identity
                .object_to_opaque
                .get(object)
                .copied()
                .map(|opaque| (opaque, *object))
                .ok_or(PlayerEndpointError::ServiceUnavailable)
        })
        .collect::<Result<Vec<_>, _>>()?;
    attackers.sort_by_key(|(opaque, _)| *opaque);
    let blockers = attackers
        .iter()
        .map(|(opaque, object)| {
            let blocker = combat
                .blockers
                .get(object)
                .ok_or(PlayerEndpointError::ServiceUnavailable)?;
            let blocker = blocker
                .map(|blocker| {
                    identity
                        .object_to_opaque
                        .get(&blocker)
                        .copied()
                        .ok_or(PlayerEndpointError::ServiceUnavailable)
                })
                .transpose()?;
            Ok(MagicCombatBlockerAssignmentV2 {
                attacker: *opaque,
                blocker,
            })
        })
        .collect::<Result<Vec<_>, PlayerEndpointError>>()?;
    Ok(Some(MagicCombatParticipationV2 {
        defending_player: combat.defending_player,
        attackers: attackers.iter().map(|(opaque, _)| *opaque).collect(),
        blockers,
    }))
}

fn project_combat_v3(
    state: &EngineState,
    perspective: PlayerId,
) -> Result<Option<MagicCombatParticipationV3>, PlayerEndpointError> {
    let Some(combat) = state.combat.as_ref() else {
        return Ok(None);
    };
    let identity = state
        .perspective_identities
        .players
        .get(&perspective)
        .ok_or(PlayerEndpointError::ServiceUnavailable)?;
    let mut attackers = combat
        .attackers
        .iter()
        .map(|object| {
            identity
                .object_to_opaque
                .get(object)
                .copied()
                .map(|opaque| (opaque, *object))
                .ok_or(PlayerEndpointError::ServiceUnavailable)
        })
        .collect::<Result<Vec<_>, _>>()?;
    attackers.sort_by_key(|(opaque, _)| *opaque);
    let blockers = attackers
        .iter()
        .map(|(opaque, object)| {
            let blocker = combat
                .blockers
                .get(object)
                .ok_or(PlayerEndpointError::ServiceUnavailable)?;
            let blocker = blocker
                .map(|blocker| {
                    identity
                        .object_to_opaque
                        .get(&blocker)
                        .copied()
                        .ok_or(PlayerEndpointError::ServiceUnavailable)
                })
                .transpose()?;
            Ok(MagicCombatBlockerAssignmentV3 {
                attacker: *opaque,
                blocker,
            })
        })
        .collect::<Result<Vec<_>, PlayerEndpointError>>()?;
    Ok(Some(MagicCombatParticipationV3 {
        defending_player: combat.defending_player,
        attackers: attackers.iter().map(|(opaque, _)| *opaque).collect(),
        blockers,
    }))
}

fn project_combat_v4(
    state: &EngineState,
    perspective: PlayerId,
) -> Result<Option<MagicCombatParticipationV4>, PlayerEndpointError> {
    let Some(combat) = state.combat.as_ref() else {
        return Ok(None);
    };
    let identity = state
        .perspective_identities
        .players
        .get(&perspective)
        .ok_or(PlayerEndpointError::ServiceUnavailable)?;
    let mut attackers = combat
        .attackers
        .iter()
        .map(|object| {
            identity
                .object_to_opaque
                .get(object)
                .copied()
                .map(|opaque| (opaque, *object))
                .ok_or(PlayerEndpointError::ServiceUnavailable)
        })
        .collect::<Result<Vec<_>, _>>()?;
    attackers.sort_by_key(|(opaque, _)| *opaque);
    let blockers = attackers
        .iter()
        .map(|(opaque, object)| {
            let blocker = combat
                .blockers
                .get(object)
                .ok_or(PlayerEndpointError::ServiceUnavailable)?;
            let blocker = blocker
                .map(|blocker| {
                    identity
                        .object_to_opaque
                        .get(&blocker)
                        .copied()
                        .ok_or(PlayerEndpointError::ServiceUnavailable)
                })
                .transpose()?;
            Ok(MagicCombatBlockerAssignmentV4 {
                attacker: *opaque,
                status: if combat.blocked_attackers.contains(object) {
                    MagicBlockedStatusV4::Blocked
                } else {
                    MagicBlockedStatusV4::Unblocked
                },
                blocker,
            })
        })
        .collect::<Result<Vec<_>, PlayerEndpointError>>()?;
    Ok(Some(MagicCombatParticipationV4 {
        defending_player: combat.defending_player,
        attackers: attackers.iter().map(|(opaque, _)| *opaque).collect(),
        blockers,
    }))
}

fn project_marked_damage_v4(
    state: &EngineState,
    perspective: PlayerId,
) -> Result<Vec<MagicMarkedDamageV4>, PlayerEndpointError> {
    let identity = state
        .perspective_identities
        .players
        .get(&perspective)
        .ok_or(PlayerEndpointError::ServiceUnavailable)?;
    let mut marked = Vec::new();
    for (object, source) in &state.foundation_sources {
        if source.marked_damage == 0 {
            continue;
        }
        let snapshot = state
            .zones
            .objects
            .get(object)
            .ok_or(PlayerEndpointError::ServiceUnavailable)?;
        let location = state
            .zones
            .locations
            .get(object)
            .ok_or(PlayerEndpointError::ServiceUnavailable)?;
        if location.zone != mtgml_model::ZoneKind::Battlefield || snapshot.face_down {
            continue;
        }
        let opaque = identity
            .object_to_opaque
            .get(object)
            .copied()
            .ok_or(PlayerEndpointError::ServiceUnavailable)?;
        marked.push((
            opaque,
            MagicMarkedDamageV4 {
                creature: opaque,
                amount: source.marked_damage.to_string(),
            },
        ));
    }
    marked.sort_by_key(|(opaque, _)| *opaque);
    Ok(marked.into_iter().map(|(_, item)| item).collect())
}

fn project_sba_ordering(
    state: &EngineState,
    perspective: PlayerId,
) -> Result<Option<MagicPendingSbaOrdering>, PlayerEndpointError> {
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
            Ok(MagicCompletedOrder {
                owner: order.owner,
                ordered_objects,
            })
        })
        .collect::<Result<Vec<_>, PlayerEndpointError>>()?;
    Ok(Some(MagicPendingSbaOrdering {
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
        ObservationProjectionProfile::Synthetic,
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

fn public_turn_position(position: TurnPosition) -> SyntheticTurnPosition {
    match position {
        TurnPosition::Beginning { step } => SyntheticTurnPosition::Beginning {
            step: match step {
                BeginningStep::Untap => SyntheticBeginningStep::Untap,
                BeginningStep::Upkeep => SyntheticBeginningStep::Upkeep,
                BeginningStep::Draw => SyntheticBeginningStep::Draw,
            },
        },
        TurnPosition::PrecombatMain => SyntheticTurnPosition::PrecombatMain,
        TurnPosition::Combat { step } => SyntheticTurnPosition::Combat {
            step: match step {
                CombatStep::BeginningOfCombat => SyntheticCombatStep::BeginningOfCombat,
                CombatStep::DeclareAttackers => SyntheticCombatStep::DeclareAttackers,
                CombatStep::DeclareBlockers => SyntheticCombatStep::DeclareBlockers,
                CombatStep::CombatDamage => SyntheticCombatStep::CombatDamage,
                CombatStep::EndOfCombat => SyntheticCombatStep::EndOfCombat,
            },
        },
        TurnPosition::PostcombatMain => SyntheticTurnPosition::PostcombatMain,
        TurnPosition::Ending { step } => SyntheticTurnPosition::Ending {
            step: match step {
                EndingStep::EndStep => SyntheticEndingStep::EndStep,
                EndingStep::Cleanup => SyntheticEndingStep::Cleanup,
            },
        },
    }
}

fn public_priority(priority: PriorityState) -> SyntheticPriority {
    match priority {
        PriorityState::None => SyntheticPriority::None,
        PriorityState::HeldBy { player, .. } => SyntheticPriority::HeldBy { player },
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
        ObservationProjectionProfile::Synthetic,
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

pub(crate) fn validate_candidate_projections_with_profile(
    state: &EngineState,
    profile: ObservationProjectionProfile,
) -> Result<(), ControllerError> {
    for perspective in state.core.players.keys().copied() {
        project_observation_with_profile(state, perspective, profile).map_err(|_| {
            ControllerError::EnvironmentCommit(EnvironmentCommitError::PlayerProjectionInvalid)
        })?;
        project_information_state_with_profile(state, perspective, profile).map_err(|_| {
            ControllerError::EnvironmentCommit(EnvironmentCommitError::PlayerProjectionInvalid)
        })?;
        project_visible_decision(state, perspective).map_err(|_| {
            ControllerError::EnvironmentCommit(EnvironmentCommitError::PlayerProjectionInvalid)
        })?;
    }
    Ok(())
}
