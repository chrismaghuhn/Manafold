//! Shared player-safe projections used by every environment backend.
//!
//! This module is deliberately backend-neutral: it accepts an authoritative
//! state snapshot and a bound perspective, then constructs only the public
//! observation, information, decision, and step products.

#![cfg_attr(not(test), allow(unused_imports))]

use base64::{engine::general_purpose::STANDARD, Engine as _};
use mtgml_decision::PlayerDecisionRequestV2;
use mtgml_model::{
    EpisodeStatus, ExecutionIdentityV1, ExecutionProgramV1, PlayerId, RulesAuthorityV1,
    RulesContractManifestV1, SemanticContractManifestV1,
};
use mtgml_observation::{
    AttachmentObservationV1, CounterObservationV1, FaceObservationV1,
    InformationStateDigestInputV2, InformationStateDigestInputV3, MagicBasicLandObservationV1,
    MagicSharedExecutionObservationV1, ManaPoolObservationV1, ObservationEnvelope,
    ObservationEnvelopeV2, ObservedEventEnvelopeV2, ObservedFaceV1, PlayerInformationStateV2,
    PlayerInformationStateV3, PlayerKnowledgeCauseV1, PlayerKnowledgeChannelV1,
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
    BeginningStep, CombatStep, CounterKindV1, EffectExpiry, EndingStep, EngineState,
    EngineStatePartsV2, EngineStatePartsV3, KnowledgeAcquisitionCause, KnowledgeAcquisitionReason,
    KnowledgeHistoryChannel, KnowledgeInvalidationReason, PriorityState, StackItemPayload,
    TargetRef, TemporaryKeyword, TemporaryOperation, TurnPosition,
};

/// Builds the successor public-state observation from successor state and
/// already verified content authority. The admitted successor runtime selects
/// this codec only through its exact executable profile.
pub fn project_magic_basic_land_observation_v1(
    parts: &EngineStatePartsV2,
    perspective: PlayerId,
    execution_identity: &ExecutionIdentityV1,
    semantic_manifest: &SemanticContractManifestV1,
    rules_manifest: &RulesContractManifestV1,
    catalog: &mtgml_card_ir::VerifiedContentCatalogV1,
) -> Result<ObservationEnvelope, PlayerEndpointError> {
    parts
        .validate()
        .map_err(|_| PlayerEndpointError::ServiceUnavailable)?;
    let engine = parts.materialize();
    if !engine.core.players.contains_key(&perspective) {
        return Err(PlayerEndpointError::ServiceUnavailable);
    }
    rules_manifest
        .validate()
        .map_err(|_| PlayerEndpointError::ServiceUnavailable)?;
    let rules_id =
        mtgml_persistence::semantic_contract_digest::calculate_rules_contract_id_v1(rules_manifest)
            .map_err(|_| PlayerEndpointError::ServiceUnavailable)?;
    let semantic_id =
        mtgml_persistence::semantic_contract_digest::calculate_semantic_contract_id_v1(
            semantic_manifest,
        )
        .map_err(|_| PlayerEndpointError::ServiceUnavailable)?;
    let Some(expected_content_id) = semantic_manifest.content_contract_id.as_ref() else {
        return Err(PlayerEndpointError::ServiceUnavailable);
    };
    if execution_identity.program_kind != ExecutionProgramV1::MagicRules
        || !matches!(
            &rules_manifest.rules_authority,
            RulesAuthorityV1::ComprehensiveRules { .. }
        )
        || rules_id != semantic_manifest.rules_contract_id
        || semantic_id != execution_identity.semantic_contract_id
        || expected_content_id != catalog.content_contract_id()
        || parts.card_rules_state.faces.faces.len() != engine.zones.objects.len()
    {
        return Err(PlayerEndpointError::ServiceUnavailable);
    }
    let content_id = catalog.content_contract_id();
    let mut public_faces = std::collections::BTreeMap::new();
    for object in engine.zones.objects.values() {
        let definition = catalog
            .get(content_id, object.card_definition)
            .map_err(|_| PlayerEndpointError::ServiceUnavailable)?;
        let face_key = parts
            .card_rules_state
            .faces
            .faces
            .get(&object.id)
            .ok_or(PlayerEndpointError::ServiceUnavailable)?;
        let face_index = definition
            .faces
            .iter()
            .position(|face| face.face_key.0 == *face_key)
            .ok_or(PlayerEndpointError::ServiceUnavailable)?;
        let orientation = match face_index {
            0 => ObservedFaceV1::Front,
            1 => ObservedFaceV1::Back,
            _ => return Err(PlayerEndpointError::ServiceUnavailable),
        };
        if !object.face_down {
            public_faces.insert(object.id, orientation);
        }
    }
    for authority in parts.card_rules_state.abilities.by_instance.values() {
        let source = engine
            .zones
            .objects
            .get(&authority.source)
            .ok_or(PlayerEndpointError::ServiceUnavailable)?;
        let definition = catalog
            .get(content_id, source.card_definition)
            .map_err(|_| PlayerEndpointError::ServiceUnavailable)?;
        let face_key = parts
            .card_rules_state
            .faces
            .faces
            .get(&authority.source)
            .ok_or(PlayerEndpointError::ServiceUnavailable)?;
        if !definition.ability_identities.iter().any(|identity| {
            identity.face_key.0 == *face_key && identity.ability_key.0 == authority.ability_key
        }) {
            return Err(PlayerEndpointError::ServiceUnavailable);
        }
    }
    project_magic_basic_land_observation_from_verified_faces(parts, perspective, &public_faces)
}

fn project_magic_basic_land_observation_from_verified_faces(
    parts: &EngineStatePartsV2,
    perspective: PlayerId,
    public_faces: &std::collections::BTreeMap<mtgml_model::GameObjectId, ObservedFaceV1>,
) -> Result<ObservationEnvelope, PlayerEndpointError> {
    parts
        .validate()
        .map_err(|_| PlayerEndpointError::ServiceUnavailable)?;
    let state = parts.materialize();
    let identity = state
        .perspective_identities
        .players
        .get(&perspective)
        .ok_or(PlayerEndpointError::ServiceUnavailable)?;
    let public_battlefield = |object: mtgml_model::GameObjectId| {
        state.zones.locations.get(&object).is_some_and(|location| {
            location.zone == mtgml_model::ZoneKind::Battlefield
                && matches!(
                    location.visibility,
                    mtgml_state::VisibilityPartition::Public
                        | mtgml_state::VisibilityPartition::FaceDown
                )
        })
    };
    let opaque = |object: mtgml_model::GameObjectId| {
        identity
            .object_to_opaque
            .get(&object)
            .copied()
            .ok_or(PlayerEndpointError::ServiceUnavailable)
    };
    let mut value = MagicBasicLandObservationV1 {
        schema_version: mtgml_observation::MAGIC_BASIC_LAND_OBSERVATION_SCHEMA_V1.into(),
        active_player: state.core.active_player,
        turn_number: state.core.turn_number.to_string(),
        turn_position: public_turn_position(state.core.position),
        priority: public_priority(state.core.priority),
        pending_sba_ordering: project_sba_ordering(&state, perspective)?,
        mana_pools: parts
            .card_rules_state
            .mana
            .pools
            .iter()
            .map(|(player, pool)| ManaPoolObservationV1 {
                player: *player,
                unrestricted: pool.unrestricted,
                creature_spell_only: pool.creature_spell_only,
            })
            .collect(),
        counters: Vec::new(),
        attachments: Vec::new(),
        faces: Vec::new(),
    };
    for (object, counters) in &parts.card_rules_state.counters.counters {
        if !public_battlefield(*object) {
            continue;
        }
        for (kind, count) in counters {
            value.counters.push(CounterObservationV1 {
                object: opaque(*object)?,
                counter_kind: match kind {
                    CounterKindV1::PlusOnePlusOne => {
                        mtgml_observation::PublicCounterKindV1::PlusOnePlusOne
                    }
                    CounterKindV1::MinusOneMinusOne => {
                        mtgml_observation::PublicCounterKindV1::MinusOneMinusOne
                    }
                    CounterKindV1::Lore => mtgml_observation::PublicCounterKindV1::Lore,
                },
                count: *count,
            });
        }
    }
    for (source, edge) in &parts.card_rules_state.attachments.by_source {
        if public_battlefield(*source) && public_battlefield(edge.target) {
            value.attachments.push(AttachmentObservationV1 {
                source: opaque(*source)?,
                target: opaque(edge.target)?,
            });
        }
    }
    for (object, face) in public_faces {
        if public_battlefield(*object)
            && state
                .zones
                .objects
                .get(object)
                .is_some_and(|value| !value.face_down)
        {
            value.faces.push(FaceObservationV1 {
                object: opaque(*object)?,
                face: match face {
                    ObservedFaceV1::Front => mtgml_observation::PublicFaceV1::Front,
                    ObservedFaceV1::Back => mtgml_observation::PublicFaceV1::Back,
                },
            });
        }
    }
    value.mana_pools.sort_by_key(|entry| entry.player);
    value
        .counters
        .sort_by_key(|entry| (entry.object, entry.counter_kind));
    value.attachments.sort_by_key(|entry| entry.source);
    value.faces.sort_by_key(|entry| entry.object);
    value
        .validate()
        .map_err(|_| PlayerEndpointError::ServiceUnavailable)?;
    let payload = mtgml_wire::encode_canonical(&value)
        .map_err(|_| PlayerEndpointError::ServiceUnavailable)?;
    let observation = ObservationEnvelope {
        schema_version: OBSERVATION_SCHEMA.into(),
        perspective,
        state_revision: state.revision,
        payload_codec: mtgml_observation::MAGIC_BASIC_LAND_OBSERVATION_SCHEMA_V1.into(),
        payload_base64: STANDARD.encode(&payload),
        digest: mtgml_model::ObservationDigest::from_canonical_bytes(&payload),
    };
    observation
        .validate()
        .map_err(|_| PlayerEndpointError::ServiceUnavailable)?;
    Ok(observation)
}

use crate::endpoint::PlayerEndpointError;
#[cfg(any(test, feature = "historical-conformance-runtime"))]
use crate::errors::{ControllerError, EnvironmentCommitError};
#[cfg(any(test, feature = "historical-conformance-runtime"))]
use crate::semantic_catalog_generated::{
    magic_bounded_turn_0_1_0_semantic_contract_id,
    magic_combat_attackers_0_1_0_semantic_contract_id,
    magic_combat_blockers_0_1_0_semantic_contract_id,
    magic_combat_damage_0_1_0_semantic_contract_id,
    magic_s3_a_ordered_sba_0_1_0_semantic_contract_id,
    magic_s3_b_basic_priority_0_1_0_semantic_contract_id,
    magic_s3_c_draw_interaction_0_1_0_semantic_contract_id,
    magic_turn_structure_0_1_0_semantic_contract_id, synthetic_legacy_default_semantic_contract_id,
};

#[cfg(any(test, feature = "historical-conformance-runtime"))]
const SYNTHETIC_OBSERVATION_CODEC: &str = SYNTHETIC_OBSERVATION_SCHEMA_V1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg(any(test, feature = "historical-conformance-runtime"))]
pub(crate) enum ObservationProjectionProfile {
    Synthetic,
    Magic,
    MagicCombat,
    MagicCombatBlockers,
    MagicCombatDamage,
}

#[cfg(any(test, feature = "historical-conformance-runtime"))]
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
    } else if identity.semantic_contract_id == magic_combat_damage_0_1_0_semantic_contract_id()
        || identity.semantic_contract_id == magic_bounded_turn_0_1_0_semantic_contract_id()
    {
        Ok(ObservationProjectionProfile::MagicCombatDamage)
    } else {
        Err(ControllerError::SemanticContractUnsupported)
    }
}

#[cfg(any(test, feature = "historical-conformance-runtime"))]
pub(crate) fn project_observation(
    state: &EngineState,
    perspective: PlayerId,
) -> Result<ObservationEnvelope, PlayerEndpointError> {
    project_observation_with_profile(state, perspective, ObservationProjectionProfile::Synthetic)
}

#[cfg(any(test, feature = "historical-conformance-runtime"))]
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

#[cfg(any(test, feature = "historical-conformance-runtime"))]
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

#[cfg(any(test, feature = "historical-conformance-runtime"))]
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

#[cfg(any(test, feature = "historical-conformance-runtime"))]
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

#[cfg(any(test, feature = "historical-conformance-runtime"))]
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

#[cfg(any(test, feature = "historical-conformance-runtime"))]
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

#[cfg(any(test, feature = "historical-conformance-runtime"))]
pub(crate) fn project_information_state_with_profile(
    state: &EngineState,
    perspective: PlayerId,
    profile: ObservationProjectionProfile,
) -> Result<PlayerInformationStateV2, PlayerEndpointError> {
    if !state.core.players.contains_key(&perspective) {
        return Err(PlayerEndpointError::ServiceUnavailable);
    }
    let current_observation = project_observation_with_profile(state, perspective, profile)?;
    project_information_state_from_observation(state, perspective, current_observation)
}

pub fn project_successor_information_state(
    parts: &EngineStatePartsV2,
    perspective: PlayerId,
    execution_identity: &ExecutionIdentityV1,
    semantic_manifest: &SemanticContractManifestV1,
    rules_manifest: &RulesContractManifestV1,
    catalog: &mtgml_card_ir::VerifiedContentCatalogV1,
) -> Result<PlayerInformationStateV2, PlayerEndpointError> {
    let state = parts.materialize();
    let observation = project_magic_basic_land_observation_v1(
        parts,
        perspective,
        execution_identity,
        semantic_manifest,
        rules_manifest,
        catalog,
    )?;
    project_information_state_from_observation(&state, perspective, observation)
}

/// Builds a detached V2 observation envelope and V3 player information state
/// from one structurally valid G0 state. This is not selected by the current
/// endpoint until the G0j activation cut.
pub fn project_successor_information_state_v3(
    parts: &EngineStatePartsV3,
    perspective: PlayerId,
    execution_identity: &ExecutionIdentityV1,
    semantic_manifest: &SemanticContractManifestV1,
    rules_manifest: &RulesContractManifestV1,
    catalog: &mtgml_card_ir::VerifiedContentCatalogV1,
) -> Result<PlayerInformationStateV3, PlayerEndpointError> {
    project_successor_information_state_v3_inner(
        parts,
        perspective,
        execution_identity,
        semantic_manifest,
        rules_manifest,
        catalog,
        false,
    )
}

pub(crate) fn project_successor_information_state_v3_structural_only(
    parts: &EngineStatePartsV3,
    perspective: PlayerId,
    execution_identity: &ExecutionIdentityV1,
    semantic_manifest: &SemanticContractManifestV1,
    rules_manifest: &RulesContractManifestV1,
    catalog: &mtgml_card_ir::VerifiedContentCatalogV1,
) -> Result<PlayerInformationStateV3, PlayerEndpointError> {
    project_successor_information_state_v3_inner(
        parts,
        perspective,
        execution_identity,
        semantic_manifest,
        rules_manifest,
        catalog,
        true,
    )
}

fn project_successor_information_state_v3_inner(
    parts: &EngineStatePartsV3,
    perspective: PlayerId,
    execution_identity: &ExecutionIdentityV1,
    semantic_manifest: &SemanticContractManifestV1,
    rules_manifest: &RulesContractManifestV1,
    catalog: &mtgml_card_ir::VerifiedContentCatalogV1,
    rules_domain_validated: bool,
) -> Result<PlayerInformationStateV3, PlayerEndpointError> {
    let state_validation = if rules_domain_validated {
        parts.validate_structure()
    } else {
        parts.validate()
    };
    state_validation.map_err(|_| PlayerEndpointError::ServiceUnavailable)?;

    let mut predecessor = parts.predecessor_v5.clone();
    for record in predecessor.zones.stack_records.values_mut() {
        record.payload = None;
    }
    let projection_state = EngineStatePartsV2 {
        predecessor_v5: predecessor,
        execution_v3: Default::default(),
        card_rules_state: parts.card_rules_state.clone(),
    };
    let basic_envelope = project_magic_basic_land_observation_v1(
        &projection_state,
        perspective,
        execution_identity,
        semantic_manifest,
        rules_manifest,
        catalog,
    )?;
    let base_payload = STANDARD
        .decode(&basic_envelope.payload_base64)
        .map_err(|_| PlayerEndpointError::ServiceUnavailable)?;
    let basic = mtgml_wire::decode_canonical::<MagicBasicLandObservationV1>(&base_payload)
        .map_err(|_| PlayerEndpointError::ServiceUnavailable)?;
    let shared = project_shared_execution_observation(parts, perspective, basic)?;
    shared
        .validate()
        .map_err(|_| PlayerEndpointError::ServiceUnavailable)?;
    let payload = mtgml_wire::encode_canonical(&shared)
        .map_err(|_| PlayerEndpointError::ServiceUnavailable)?;
    let knowledge = parts
        .predecessor_v5
        .knowledge
        .players
        .get(&perspective)
        .ok_or(PlayerEndpointError::ServiceUnavailable)?;
    let current_observation = ObservationEnvelopeV2 {
        schema_version: mtgml_observation::OBSERVATION_SCHEMA_V2.into(),
        perspective,
        view_sequence: knowledge.next_visible_sequence,
        payload_codec: mtgml_observation::MAGIC_SHARED_EXECUTION_OBSERVATION_SCHEMA_V1.into(),
        payload_base64: STANDARD.encode(&payload),
        digest: mtgml_model::ObservationDigest::from_canonical_bytes(&payload),
    };
    current_observation
        .validate()
        .map_err(|_| PlayerEndpointError::ServiceUnavailable)?;

    let state = projection_state.materialize();
    let basic_information =
        project_information_state_from_observation(&state, perspective, basic_envelope)?;
    let mut information = PlayerInformationStateV3 {
        schema_version: mtgml_observation::INFORMATION_STATE_SCHEMA_V3.into(),
        perspective,
        current_observation,
        next_visible_sequence: knowledge.next_visible_sequence,
        retained_knowledge: basic_information.retained_knowledge,
        digest: mtgml_model::InformationStateDigestV3::from_canonical_bytes(
            b"g0-information-state-placeholder",
        ),
    };
    let input: InformationStateDigestInputV3 = information.digest_input();
    let (_, digest) = mtgml_wire::compute_information_state_digest_v3(&input)
        .map_err(|_| PlayerEndpointError::ServiceUnavailable)?;
    information.digest = digest;
    information
        .validate()
        .map_err(|_| PlayerEndpointError::ServiceUnavailable)?;
    Ok(information)
}

fn project_shared_execution_observation(
    parts: &EngineStatePartsV3,
    perspective: PlayerId,
    basic: MagicBasicLandObservationV1,
) -> Result<MagicSharedExecutionObservationV1, PlayerEndpointError> {
    let identity = parts
        .predecessor_v5
        .perspective_identities
        .players
        .get(&perspective)
        .ok_or(PlayerEndpointError::ServiceUnavailable)?;
    let mut stack = Vec::with_capacity(parts.predecessor_v5.zones.stack_records.len());
    for stack_id in parts.predecessor_v5.zones.stack_order.iter().rev() {
        let record = parts
            .predecessor_v5
            .zones
            .stack_records
            .get(stack_id)
            .ok_or(PlayerEndpointError::ServiceUnavailable)?;
        let payload = record
            .payload
            .as_ref()
            .ok_or(PlayerEndpointError::ServiceUnavailable)?;
        stack.push(project_public_stack_item_v1(
            identity,
            parts
                .predecessor_v5
                .knowledge
                .players
                .get(&perspective)
                .ok_or(PlayerEndpointError::ServiceUnavailable)?,
            &parts.predecessor_v5.zones.stack_order,
            record.controller,
            payload,
        )?);
    }

    let mut temporary_effects = parts
        .execution_v4
        .effects
        .values()
        .map(|effect| project_public_temporary_effect_v1(parts, perspective, effect))
        .collect::<Result<Vec<_>, _>>()?;
    for index in 1..temporary_effects.len() {
        let mut position = index;
        while position > 0
            && temporary_effects[position - 1]
                .compare_canonical(&temporary_effects[position])
                .map_err(|_| PlayerEndpointError::ServiceUnavailable)?
                == std::cmp::Ordering::Greater
        {
            temporary_effects.swap(position - 1, position);
            position -= 1;
        }
    }

    Ok(MagicSharedExecutionObservationV1 {
        schema_version: mtgml_observation::MAGIC_SHARED_EXECUTION_OBSERVATION_SCHEMA_V1.into(),
        active_player: basic.active_player,
        turn_number: basic.turn_number,
        turn_position: basic.turn_position,
        priority: basic.priority,
        pending_sba_ordering: project_sba_ordering_v4(parts, perspective)?,
        mana_pools: basic.mana_pools,
        counters: basic.counters,
        attachments: basic.attachments,
        faces: basic.faces,
        stack,
        temporary_effects,
    })
}

pub(crate) fn project_public_stack_item_v1(
    identity: &mtgml_state::PerspectiveIdentityRecordV2,
    knowledge: &mtgml_state::PlayerKnowledgeStateV2,
    stack_order: &[mtgml_model::StackObjectId],
    controller: PlayerId,
    payload: &StackItemPayload,
) -> Result<mtgml_observation::PublicStackItemV1, PlayerEndpointError> {
    let object_opaque = |object: mtgml_model::GameObjectId| {
        public_opaque_object(identity, knowledge, object)
            .ok_or(PlayerEndpointError::ServiceUnavailable)
    };
    let target_views = |bindings: &[mtgml_state::TargetBinding]| {
        bindings
            .iter()
            .map(|binding| match binding.target {
                TargetRef::Object(object) => Ok(mtgml_decision::SafeTargetDescriptorV1::Object {
                    object: object_opaque(object)?,
                }),
                TargetRef::Player(player) => {
                    Ok(mtgml_decision::SafeTargetDescriptorV1::Player { player })
                }
                TargetRef::StackItem(stack_object) => {
                    let position = stack_order
                        .iter()
                        .rev()
                        .position(|candidate| *candidate == stack_object)
                        .and_then(|position| u32::try_from(position).ok())
                        .ok_or(PlayerEndpointError::ServiceUnavailable)?;
                    Ok(mtgml_decision::SafeTargetDescriptorV1::StackItem {
                        stack_position_from_top: position,
                    })
                }
            })
            .collect::<Result<Vec<_>, PlayerEndpointError>>()
    };
    let mode_views = |bindings: &[mtgml_state::ModeBinding]| {
        bindings
            .iter()
            .map(|binding| mtgml_observation::PublicModeV1 {
                mode_slot: binding.mode_slot,
                selected_mode: binding.selected_mode,
            })
            .collect::<Vec<_>>()
    };
    let cost_view = |facts: &mtgml_state::CostFacts| mtgml_decision::CostFactsV1 {
        selected_route: facts.selected_route.map(|route| match route {
            mtgml_state::CostRoute::Normal => mtgml_decision::CostRouteV1::Normal,
            mtgml_state::CostRoute::Alternative {
                profile_local_route_id,
            } => mtgml_decision::CostRouteV1::Alternative {
                route_id: profile_local_route_id,
            },
        }),
        paid_additional_cost_ids: facts.paid_additional_cost_ids.clone(),
    };
    let source_views = |source: &mtgml_state::AbilitySourceContext| {
        let source_object =
            public_known_card_object(identity, knowledge, source.source.snapshot.object);
        let source_ability = source_object.and_then(|_| {
            identity
                .ability_to_opaque
                .get(&source.ability_instance_id)
                .copied()
        });
        (source_object, source_ability)
    };

    match payload {
        StackItemPayload::Spell {
            stack_card_object,
            modes,
            targets,
            cost_facts,
            ..
        } => Ok(mtgml_observation::PublicStackItemV1::Spell {
            controller,
            card_object: {
                let opaque = object_opaque(*stack_card_object)?;
                if knowledge
                    .active
                    .get(&opaque)
                    .is_none_or(|record| record.card_definition.is_none())
                {
                    return Err(PlayerEndpointError::ServiceUnavailable);
                }
                opaque
            },
            modes: mode_views(modes),
            targets: target_views(targets)?,
            cost_facts: cost_view(cost_facts),
        }),
        StackItemPayload::ActivatedAbility {
            source_context,
            modes,
            targets,
            cost_facts,
        } => {
            let (source_object, source_ability) = source_views(source_context);
            Ok(mtgml_observation::PublicStackItemV1::ActivatedAbility {
                controller,
                source_object,
                source_ability,
                modes: mode_views(modes),
                targets: target_views(targets)?,
                cost_facts: cost_view(cost_facts),
            })
        }
        StackItemPayload::TriggeredAbility {
            source_context,
            targets,
            ..
        } => {
            let (source_object, source_ability) = source_views(source_context);
            Ok(mtgml_observation::PublicStackItemV1::TriggeredAbility {
                controller,
                source_object,
                source_ability,
                targets: target_views(targets)?,
            })
        }
    }
}

pub(crate) fn project_public_temporary_effect_v1(
    state: &mtgml_state::EngineStatePartsV3,
    perspective: PlayerId,
    effect: &mtgml_state::TemporaryEffectRecord,
) -> Result<mtgml_observation::PublicTemporaryEffectV1, PlayerEndpointError> {
    let identity = state
        .predecessor_v5
        .perspective_identities
        .players
        .get(&perspective)
        .ok_or(PlayerEndpointError::ServiceUnavailable)?;
    let knowledge = state
        .predecessor_v5
        .knowledge
        .players
        .get(&perspective)
        .ok_or(PlayerEndpointError::ServiceUnavailable)?;
    let mut affected_objects = effect
        .affected_objects
        .iter()
        .map(|object| {
            if !crate::successor_projection::public_face_up_battlefield_object(
                *object, state, state,
            ) {
                return Err(PlayerEndpointError::ServiceUnavailable);
            }
            public_known_card_object(identity, knowledge, *object)
                .ok_or(PlayerEndpointError::ServiceUnavailable)
        })
        .collect::<Result<Vec<_>, _>>()?;
    affected_objects.sort();
    if affected_objects.is_empty() || affected_objects.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(PlayerEndpointError::ServiceUnavailable);
    }
    let operation = match effect.operation {
        TemporaryOperation::PowerToughnessDelta { power, toughness } => {
            mtgml_observation::PublicTemporaryOperationV1::PowerToughnessDelta { power, toughness }
        }
        TemporaryOperation::GrantKeyword { keyword } => {
            mtgml_observation::PublicTemporaryOperationV1::GrantKeyword {
                keyword: match keyword {
                    TemporaryKeyword::Haste => mtgml_observation::PublicEffectKeywordV1::Haste,
                    TemporaryKeyword::DoubleStrike => {
                        mtgml_observation::PublicEffectKeywordV1::DoubleStrike
                    }
                },
            }
        }
    };
    let expiry = match effect.expiry {
        EffectExpiry::UntilEndOfTurn { turn_number } => {
            mtgml_observation::PublicEffectExpiryV1::UntilEndOfTurn {
                turn_number: turn_number.to_string(),
            }
        }
    };
    Ok(mtgml_observation::PublicTemporaryEffectV1 {
        affected_objects,
        operation,
        expiry,
    })
}

fn public_opaque_object(
    identity: &mtgml_state::PerspectiveIdentityRecordV2,
    knowledge: &mtgml_state::PlayerKnowledgeStateV2,
    object: mtgml_model::GameObjectId,
) -> Option<mtgml_model::OpaqueObjectId> {
    let opaque = identity.object_to_opaque.get(&object).copied()?;
    let record = knowledge.active.get(&opaque)?;
    if record.opaque_object != opaque
        || !record.known_location.as_ref().is_some_and(|fact| {
            fact.location.visibility == mtgml_state::VisibilityPartition::Public
                && public_knowledge_provenance(&fact.provenance)
        })
    {
        return None;
    }
    Some(opaque)
}

fn public_known_card_object(
    identity: &mtgml_state::PerspectiveIdentityRecordV2,
    knowledge: &mtgml_state::PlayerKnowledgeStateV2,
    object: mtgml_model::GameObjectId,
) -> Option<mtgml_model::OpaqueObjectId> {
    let opaque = public_opaque_object(identity, knowledge, object)?;
    knowledge
        .active
        .get(&opaque)
        .is_some_and(|record| record.card_definition.is_some())
        .then_some(opaque)
}

fn public_knowledge_provenance(reason: &mtgml_state::KnowledgeAcquisitionReason) -> bool {
    use mtgml_state::{KnowledgeAcquisitionCause as Cause, KnowledgeAcquisitionReason as Reason};
    match reason {
        Reason::InitialConfiguration => true,
        Reason::Observed {
            channel: mtgml_state::KnowledgeHistoryChannel::Public,
            cause: Cause::PublicEvent | Cause::ExplicitReveal,
            ..
        } => true,
        Reason::Observed { .. } => false,
    }
}

fn project_sba_ordering_v4(
    parts: &EngineStatePartsV3,
    perspective: PlayerId,
) -> Result<Option<MagicPendingSbaOrdering>, PlayerEndpointError> {
    let mut matching = parts
        .execution_v4
        .continuations
        .values()
        .filter_map(|record| match &record.payload {
            mtgml_state::ContinuationPayloadV3::MagicSbaGraveyardOrderV1 {
                apnap_owners,
                next_owner_index,
                completed_owner_orders,
                ..
            } => Some((apnap_owners, next_owner_index, completed_owner_orders)),
            _ => None,
        });
    let Some((owners, next_owner_index, orders)) = matching.next() else {
        return Ok(None);
    };
    if matching.next().is_some() {
        return Err(PlayerEndpointError::ServiceUnavailable);
    }
    let next_order_owner = owners
        .get(
            usize::try_from(*next_owner_index)
                .map_err(|_| PlayerEndpointError::ServiceUnavailable)?,
        )
        .copied()
        .ok_or(PlayerEndpointError::ServiceUnavailable)?;
    let identity = parts
        .predecessor_v5
        .perspective_identities
        .players
        .get(&perspective)
        .ok_or(PlayerEndpointError::ServiceUnavailable)?;
    let completed_orders = orders
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

fn project_information_state_from_observation(
    state: &EngineState,
    perspective: PlayerId,
    current_observation: ObservationEnvelope,
) -> Result<PlayerInformationStateV2, PlayerEndpointError> {
    if !state.core.players.contains_key(&perspective) {
        return Err(PlayerEndpointError::ServiceUnavailable);
    }
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

#[cfg(any(test, feature = "historical-conformance-runtime"))]
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

#[cfg(any(test, feature = "historical-conformance-runtime"))]
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

#[cfg(any(test, feature = "historical-conformance-runtime"))]
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

#[cfg(any(test, feature = "historical-conformance-runtime"))]
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

#[cfg(test)]
mod shared_execution_visibility_tests {
    use super::*;
    use mtgml_card_ir::{AbilityKey, CardSemanticProfileId, FaceKey};
    use mtgml_model::{AbilityInstanceId, GameObjectId, OpaqueAbilityId, OpaqueObjectId};
    use mtgml_state::{
        AbilitySourceContext, CostFacts, KnowledgeAcquisitionReason, ModeBinding, ObjectSnapshot,
        PerspectiveIdentityRecordV2, PlayerKnowledgeStateV2, SourceContext, StackItemPayload,
        TargetBinding, TriggerEventSnapshot, VisibilityPartition, ZoneLocation, ZonePosition,
    };
    use std::collections::BTreeMap;

    fn public_location() -> ZoneLocation {
        ZoneLocation {
            zone: mtgml_model::ZoneKind::Battlefield,
            player: None,
            position: ZonePosition::Unordered,
            visibility: VisibilityPartition::Public,
            partition: None,
        }
    }

    fn source_context(object: GameObjectId) -> AbilitySourceContext {
        AbilitySourceContext {
            source: SourceContext {
                snapshot: ObjectSnapshot {
                    object,
                    physical_card: None,
                    card_definition: mtgml_model::CardDefinitionId(7),
                    owner: PlayerId(1),
                    controller: PlayerId(1),
                    tapped: false,
                    face_down: false,
                    location: public_location(),
                },
                face_key: FaceKey(0),
                semantic_profile_id: CardSemanticProfileId::parse("test-source@1.0.0").unwrap(),
            },
            ability_instance_id: AbilityInstanceId(7),
            ability_key: AbilityKey(0),
        }
    }

    fn identity_with_source_mapping() -> PerspectiveIdentityRecordV2 {
        PerspectiveIdentityRecordV2 {
            object_to_opaque: BTreeMap::from([(GameObjectId(7), OpaqueObjectId(7))]),
            opaque_to_object: BTreeMap::from([(OpaqueObjectId(7), GameObjectId(7))]),
            ability_to_opaque: BTreeMap::from([(AbilityInstanceId(7), OpaqueAbilityId(7))]),
            opaque_to_ability: BTreeMap::from([(OpaqueAbilityId(7), AbilityInstanceId(7))]),
            next_opaque_object_id: OpaqueObjectId(8),
            next_opaque_ability_id: OpaqueAbilityId(8),
            next_player_decision_id: mtgml_model::PlayerDecisionIdV1(1),
            ..Default::default()
        }
    }

    #[test]
    fn hidden_ability_and_trigger_sources_are_not_disclosed_from_mappings_alone() {
        let mapped = identity_with_source_mapping();
        let unmapped = PerspectiveIdentityRecordV2::default();
        let no_knowledge = PlayerKnowledgeStateV2::default();
        let source = source_context(GameObjectId(7));
        let payloads = [
            StackItemPayload::ActivatedAbility {
                source_context: source.clone(),
                modes: vec![ModeBinding {
                    mode_slot: 0,
                    selected_mode: 1,
                }],
                targets: Vec::<TargetBinding>::new(),
                cost_facts: CostFacts::default(),
            },
            StackItemPayload::TriggeredAbility {
                originating_trigger: mtgml_model::TriggerInstanceId(4),
                source_context: source,
                captured_trigger_context: Box::new(TriggerEventSnapshot::CardDrawn {
                    player: PlayerId(1),
                }),
                targets: Vec::new(),
            },
        ];

        for payload in payloads {
            let with_mapping =
                project_public_stack_item_v1(&mapped, &no_knowledge, &[], PlayerId(1), &payload)
                    .unwrap();
            let without_mapping =
                project_public_stack_item_v1(&unmapped, &no_knowledge, &[], PlayerId(1), &payload)
                    .unwrap();
            assert_eq!(with_mapping, without_mapping);
            match with_mapping {
                mtgml_observation::PublicStackItemV1::ActivatedAbility {
                    source_object,
                    source_ability,
                    ..
                }
                | mtgml_observation::PublicStackItemV1::TriggeredAbility {
                    source_object,
                    source_ability,
                    ..
                } => {
                    assert_eq!(source_object, None);
                    assert_eq!(source_ability, None);
                }
                _ => panic!("expected ability stack item"),
            }
        }

        let mut public_knowledge = PlayerKnowledgeStateV2::default();
        public_knowledge.active.insert(
            OpaqueObjectId(7),
            mtgml_state::KnowledgeRecordV2 {
                opaque_object: OpaqueObjectId(7),
                physical_card: None,
                card_definition: Some(mtgml_model::CardDefinitionId(7)),
                known_location: Some(mtgml_state::KnownLocationFactV2 {
                    location: public_location(),
                    provenance: KnowledgeAcquisitionReason::InitialConfiguration,
                }),
                historical_locations: Vec::new(),
                acquisition: KnowledgeAcquisitionReason::InitialConfiguration,
            },
        );
        let public_item = project_public_stack_item_v1(
            &mapped,
            &public_knowledge,
            &[],
            PlayerId(1),
            &StackItemPayload::ActivatedAbility {
                source_context: source_context(GameObjectId(7)),
                modes: Vec::new(),
                targets: Vec::new(),
                cost_facts: CostFacts::default(),
            },
        )
        .unwrap();
        assert!(matches!(
            public_item,
            mtgml_observation::PublicStackItemV1::ActivatedAbility {
                source_object: Some(OpaqueObjectId(7)),
                source_ability: Some(OpaqueAbilityId(7)),
                ..
            }
        ));
    }
}
