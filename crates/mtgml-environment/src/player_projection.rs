//! Shared player-safe projections used by every environment backend.
//!
//! This module is deliberately backend-neutral: it accepts an authoritative
//! state snapshot and a bound perspective, then constructs only the public
//! observation, information, decision, and step products.

use base64::{engine::general_purpose::STANDARD, Engine as _};
use mtgml_model::{
    ExecutionIdentityV1, PlayerId, RulesContractManifestV1, SemanticContractManifestV1,
};
use mtgml_observation::{
    AssignedDamageObservationV1, AttachmentObservationV1, BlockObservationV1, CounterObservationV1,
    DeclaredBlockObservationV1, FaceObservationV1, InformationStateDigestInput,
    MagicBasicLandObservationV1, MagicSharedExecutionObservationV1, ManaPoolObservationV1,
    ObservationEnvelope, ObservedFaceV1, PermanentObservationV1, PlayerInformationState,
    PlayerKnowledgeCauseV1, PlayerKnowledgeChannelV1, PlayerKnowledgeInvalidationReasonV1,
    PlayerKnowledgeProvenanceV1, PlayerKnownLocationFactV1, PlayerKnownLocationV1,
    PlayerKnownObjectV1, SyntheticBeginningStep, SyntheticCombatStep, SyntheticEndingStep,
    SyntheticPriority, SyntheticTurnPosition,
};
use mtgml_observation::{MagicCompletedOrder, MagicPendingSbaOrdering};
use mtgml_state::{
    BeginningStep, CombatStep, CounterKindV1, EffectExpiry, EndingStep, EngineState,
    KnowledgeAcquisitionCause, KnowledgeAcquisitionReason, KnowledgeHistoryChannel,
    KnowledgeInvalidationReason, PriorityState, StackItemPayload, TargetRef, TemporaryKeyword,
    TemporaryOperation, TurnPosition,
};

/// Builds the basic-land public-state observation from a validated state and
/// already verified content authority. The caller validates the state; the
/// admitted runtime selects this codec only through its exact profile.
pub(crate) fn project_magic_basic_land_observation(
    parts: &EngineState,
    perspective: PlayerId,
    execution_identity: &ExecutionIdentityV1,
    semantic_manifest: &SemanticContractManifestV1,
    rules_manifest: &RulesContractManifestV1,
    catalog: &mtgml_card_ir::VerifiedContentCatalogV1,
) -> Result<MagicBasicLandObservationV1, PlayerEndpointError> {
    let engine = parts;
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
    if rules_id != semantic_manifest.rules_contract_id
        || semantic_id != execution_identity.semantic_contract_id
        || expected_content_id != catalog.content_contract_id()
        || parts.card_rules.faces.faces.len() != engine.zones.objects.len()
    {
        return Err(PlayerEndpointError::ServiceUnavailable);
    }
    let content_id = catalog.content_contract_id();
    let mut public_faces = std::collections::BTreeMap::new();
    // Every permanent on the battlefield, with its printed power and
    // toughness if it is a creature.
    let mut battlefield = std::collections::BTreeMap::new();
    for object in engine.zones.objects.values() {
        let definition = catalog
            .get(content_id, object.card_definition)
            .map_err(|_| PlayerEndpointError::ServiceUnavailable)?;
        let face_key = parts
            .card_rules
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
        let on_battlefield = engine
            .zones
            .locations
            .get(&object.id)
            .is_some_and(|location| location.zone == mtgml_model::ZoneKind::Battlefield);
        if !on_battlefield {
            continue;
        }
        // A face-down permanent is a creature whatever its card is (CR 708.2),
        // and its card is hidden: not supported.
        if object.face_down {
            return Err(PlayerEndpointError::ServiceUnavailable);
        }
        let base = &definition.faces[face_index].base_characteristics;
        let printed = if base
            .type_line
            .card_types
            .iter()
            .any(|card_type| card_type == "Creature")
        {
            // Every creature has a power and a toughness (CR 208.1).
            let (power, toughness) = base
                .power_toughness
                .ok_or(PlayerEndpointError::ServiceUnavailable)?;
            Some((i64::from(power), i64::from(toughness)))
        } else {
            None
        };
        battlefield.insert(object.id, printed);
    }
    // A creature shows its printed power and toughness, which are its current
    // ones only while nothing changes them. The projection applies neither
    // effects nor counters: it fails closed rather than show a wrong number.
    let creatures: Vec<_> = battlefield
        .iter()
        .filter(|(_, printed)| printed.is_some())
        .map(|(object, _)| *object)
        .collect();
    if !creatures.is_empty() && !engine.execution.effects.is_empty() {
        return Err(PlayerEndpointError::ServiceUnavailable);
    }
    for object in &creatures {
        let changes_power_toughness =
            parts
                .card_rules
                .counters
                .counters
                .get(object)
                .is_some_and(|counters| {
                    counters.iter().any(|(kind, count)| {
                        *count > 0
                            && matches!(
                                kind,
                                CounterKindV1::PlusOnePlusOne | CounterKindV1::MinusOneMinusOne
                            )
                    })
                });
        if changes_power_toughness {
            return Err(PlayerEndpointError::ServiceUnavailable);
        }
    }
    for authority in parts.card_rules.abilities.by_instance.values() {
        let source = engine
            .zones
            .objects
            .get(&authority.source)
            .ok_or(PlayerEndpointError::ServiceUnavailable)?;
        let definition = catalog
            .get(content_id, source.card_definition)
            .map_err(|_| PlayerEndpointError::ServiceUnavailable)?;
        let face_key = parts
            .card_rules
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
    project_magic_basic_land_observation_from_verified_faces(
        parts,
        perspective,
        &public_faces,
        &battlefield,
    )
}

fn project_magic_basic_land_observation_from_verified_faces(
    parts: &EngineState,
    perspective: PlayerId,
    public_faces: &std::collections::BTreeMap<mtgml_model::GameObjectId, ObservedFaceV1>,
    battlefield: &std::collections::BTreeMap<mtgml_model::GameObjectId, Option<(i64, i64)>>,
) -> Result<MagicBasicLandObservationV1, PlayerEndpointError> {
    let state = parts;
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
    // CR 402.3, 401.3: hand and library sizes are public.
    let cards_in = |zone: mtgml_model::ZoneKind, player: PlayerId| {
        u32::try_from(
            state
                .zones
                .locations
                .values()
                .filter(|location| location.zone == zone && location.player == Some(player))
                .count(),
        )
        .map_err(|_| PlayerEndpointError::ServiceUnavailable)
    };
    let players = state
        .core
        .players
        .iter()
        .map(|(player, entry)| {
            Ok(mtgml_observation::PlayerObservationV1 {
                player: *player,
                life: entry.life,
                hand_count: cards_in(mtgml_model::ZoneKind::Hand, *player)?,
                library_count: cards_in(mtgml_model::ZoneKind::Library, *player)?,
            })
        })
        .collect::<Result<Vec<_>, PlayerEndpointError>>()?;
    let mut value = MagicBasicLandObservationV1 {
        schema_version: mtgml_observation::MAGIC_BASIC_LAND_OBSERVATION_SCHEMA_V1.into(),
        active_player: state.core.active_player,
        turn_number: state.core.turn_number.to_string(),
        turn_position: public_turn_position(state.core.position),
        priority: public_priority(state.core.priority),
        players,
        // The shared observation carries the pending ordering from the
        // current execution; the basic value never owns it.
        pending_sba_ordering: None,
        // A half-made answer is told to the player who is making it, and to
        // nobody else.
        pending_blocks: project_pending_blocks(state, perspective, opaque)?,
        pending_damage_assignment: project_pending_damage_assignment(state, perspective, opaque)?,
        mana_pools: parts
            .card_rules
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
        tapped: Vec::new(),
        permanents: Vec::new(),
        attacking: Vec::new(),
        blocked: Vec::new(),
        blocking: Vec::new(),
    };
    for (object, counters) in &parts.card_rules.counters.counters {
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
    // CR 110.5: a permanent's tapped status is public.
    for (object, card) in &state.zones.objects {
        if card.tapped && public_battlefield(*object) {
            value.tapped.push(opaque(*object)?);
        }
    }
    // CR 302.6, 208.1: every permanent's controller and the turn since which
    // they have controlled it are public, and so are a creature's power and
    // toughness. The raw turn is shown; whether a creature can attack is for
    // the viewer to derive.
    for (object, printed) in battlefield {
        if !public_battlefield(*object) {
            continue;
        }
        let controller = state
            .zones
            .objects
            .get(object)
            .ok_or(PlayerEndpointError::ServiceUnavailable)?
            .controller;
        let permanent = state
            .card_rules
            .permanents
            .permanents
            .get(object)
            .ok_or(PlayerEndpointError::ServiceUnavailable)?;
        value.permanents.push(PermanentObservationV1 {
            object: opaque(*object)?,
            controller,
            controlled_since_turn: permanent.controlled_since_turn,
            power: printed.map(|(power, _)| power),
            toughness: printed.map(|(_, toughness)| toughness),
            // CR 120.3e: the damage marked on the permanent, which is public.
            marked_damage: permanent.marked_damage,
        });
    }
    // CR 508.1k: the attacking creatures are public. One that is not a
    // creature among the permanents fails validation below.
    if let Some(combat) = &state.combat {
        for attacker in &combat.attackers {
            value.attacking.push(opaque(*attacker)?);
        }
        // CR 509.1h: an attacker that has a blocker, or had one, is blocked.
        for attacker in &combat.blocked_attackers {
            value.blocked.push(opaque(*attacker)?);
        }
        // CR 509.1g: a blocking creature, with the attacker it blocks; that
        // attacker is null once it has left combat (CR 506.4).
        for (blocker, attacker) in &combat.blockers {
            value.blocking.push(BlockObservationV1 {
                blocker: opaque(*blocker)?,
                attacker: attacker.map(opaque).transpose()?,
            });
        }
    }
    for (source, edge) in &parts.card_rules.attachments.by_source {
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
    value.tapped.sort();
    value.permanents.sort_by_key(|entry| entry.object);
    value.attacking.sort();
    value.blocked.sort();
    value.blocking.sort_by_key(|block| block.blocker);
    value
        .validate()
        .map_err(|_| PlayerEndpointError::ServiceUnavailable)?;
    Ok(value)
}

use crate::endpoint::PlayerEndpointError;

/// The blocks `perspective` has declared so far, while its block declaration
/// is in progress (CR 509.1a): one row for each creature answered, ascending by
/// the perspective's opaque id of the blocker. Nothing for any other player, and
/// nothing when no declaration is in progress. A declaration with no creature
/// answered yet is an empty list, which tells it from none.
fn project_pending_blocks(
    state: &EngineState,
    perspective: PlayerId,
    opaque: impl Fn(
        mtgml_model::GameObjectId,
    ) -> Result<mtgml_model::OpaqueObjectId, PlayerEndpointError>,
) -> Result<Option<Vec<DeclaredBlockObservationV1>>, PlayerEndpointError> {
    let mut declarations =
        state
            .execution
            .continuations
            .values()
            .filter_map(|record| match &record.payload {
                mtgml_state::ContinuationPayload::BlockDeclaration {
                    defender, declared, ..
                } => Some((defender, declared)),
                _ => None,
            });
    let Some((defender, declared)) = declarations.next() else {
        return Ok(None);
    };
    if declarations.next().is_some() {
        return Err(PlayerEndpointError::ServiceUnavailable);
    }
    if *defender != perspective {
        return Ok(None);
    }
    let mut rows = declared
        .iter()
        .map(|(blocker, attacker)| {
            Ok(DeclaredBlockObservationV1 {
                blocker: opaque(*blocker)?,
                attacker: attacker.map(&opaque).transpose()?,
            })
        })
        .collect::<Result<Vec<_>, PlayerEndpointError>>()?;
    rows.sort_by_key(|row| row.blocker);
    Ok(Some(rows))
}

/// The damage `perspective` has divided so far, while the division of its
/// attackers' combat damage is in progress (CR 510.1c): one row for each blocker
/// answered, with the attacker it blocks, ascending by the perspective's opaque
/// ids of the attacker and then the blocker. An amount the rules force is not an
/// answer and is not listed. Nothing for any other player, and nothing when no
/// division is in progress. A division with no blocker answered yet is an empty
/// list, which tells it from none.
fn project_pending_damage_assignment(
    state: &EngineState,
    perspective: PlayerId,
    opaque: impl Fn(
        mtgml_model::GameObjectId,
    ) -> Result<mtgml_model::OpaqueObjectId, PlayerEndpointError>,
) -> Result<Option<Vec<AssignedDamageObservationV1>>, PlayerEndpointError> {
    let mut divisions =
        state
            .execution
            .continuations
            .values()
            .filter_map(|record| match &record.payload {
                mtgml_state::ContinuationPayload::CombatDamageAssignment {
                    player,
                    assigned,
                    ..
                } => Some((player, assigned)),
                _ => None,
            });
    let Some((player, assigned)) = divisions.next() else {
        return Ok(None);
    };
    if divisions.next().is_some() {
        return Err(PlayerEndpointError::ServiceUnavailable);
    }
    if *player != perspective {
        return Ok(None);
    }
    let combat = state
        .combat
        .as_ref()
        .ok_or(PlayerEndpointError::ServiceUnavailable)?;
    let mut rows = assigned
        .iter()
        .map(|(blocker, amount)| {
            // Each blocker blocks one attacker (CR 509.1a).
            let attacker = combat
                .blockers
                .get(blocker)
                .copied()
                .flatten()
                .ok_or(PlayerEndpointError::ServiceUnavailable)?;
            Ok(AssignedDamageObservationV1 {
                attacker: opaque(attacker)?,
                blocker: opaque(*blocker)?,
                amount: *amount,
            })
        })
        .collect::<Result<Vec<_>, PlayerEndpointError>>()?;
    rows.sort_by_key(|row| (row.attacker, row.blocker));
    Ok(Some(rows))
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

/// Builds a detached V2 observation envelope and V3 player information state
/// from one structurally valid G0 state. This is not selected by the current
/// endpoint until the G0j activation cut.
pub fn project_successor_information_state(
    parts: &EngineState,
    perspective: PlayerId,
    execution_identity: &ExecutionIdentityV1,
    semantic_manifest: &SemanticContractManifestV1,
    rules_manifest: &RulesContractManifestV1,
    catalog: &mtgml_card_ir::VerifiedContentCatalogV1,
) -> Result<PlayerInformationState, PlayerEndpointError> {
    project_successor_information_state_inner(
        parts,
        perspective,
        execution_identity,
        semantic_manifest,
        rules_manifest,
        catalog,
        false,
    )
}

pub(crate) fn project_successor_information_state_structural_only(
    parts: &EngineState,
    perspective: PlayerId,
    execution_identity: &ExecutionIdentityV1,
    semantic_manifest: &SemanticContractManifestV1,
    rules_manifest: &RulesContractManifestV1,
    catalog: &mtgml_card_ir::VerifiedContentCatalogV1,
) -> Result<PlayerInformationState, PlayerEndpointError> {
    project_successor_information_state_inner(
        parts,
        perspective,
        execution_identity,
        semantic_manifest,
        rules_manifest,
        catalog,
        true,
    )
}

fn project_successor_information_state_inner(
    parts: &EngineState,
    perspective: PlayerId,
    execution_identity: &ExecutionIdentityV1,
    semantic_manifest: &SemanticContractManifestV1,
    rules_manifest: &RulesContractManifestV1,
    catalog: &mtgml_card_ir::VerifiedContentCatalogV1,
    rules_domain_validated: bool,
) -> Result<PlayerInformationState, PlayerEndpointError> {
    let state_validation = if rules_domain_validated {
        parts.validate_structure()
    } else {
        parts.validate()
    };
    state_validation.map_err(|_| PlayerEndpointError::ServiceUnavailable)?;

    let basic = project_magic_basic_land_observation(
        parts,
        perspective,
        execution_identity,
        semantic_manifest,
        rules_manifest,
        catalog,
    )?;
    let shared = project_shared_execution_observation(parts, perspective, basic)?;
    shared
        .validate()
        .map_err(|_| PlayerEndpointError::ServiceUnavailable)?;
    let payload = mtgml_wire::encode_canonical(&shared)
        .map_err(|_| PlayerEndpointError::ServiceUnavailable)?;
    let knowledge = parts
        .knowledge
        .players
        .get(&perspective)
        .ok_or(PlayerEndpointError::ServiceUnavailable)?;
    let current_observation = ObservationEnvelope {
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

    let mut information = PlayerInformationState {
        schema_version: mtgml_observation::INFORMATION_STATE_SCHEMA_V3.into(),
        perspective,
        current_observation,
        next_visible_sequence: knowledge.next_visible_sequence,
        retained_knowledge: project_retained_knowledge(knowledge),
        digest: mtgml_model::InformationStateDigest::from_canonical_bytes(
            b"g0-information-state-placeholder",
        ),
    };
    let input: InformationStateDigestInput = information.digest_input();
    let (_, digest) = mtgml_wire::compute_information_state_digest(&input)
        .map_err(|_| PlayerEndpointError::ServiceUnavailable)?;
    information.digest = digest;
    information
        .validate()
        .map_err(|_| PlayerEndpointError::ServiceUnavailable)?;
    Ok(information)
}

fn project_shared_execution_observation(
    parts: &EngineState,
    perspective: PlayerId,
    basic: MagicBasicLandObservationV1,
) -> Result<MagicSharedExecutionObservationV1, PlayerEndpointError> {
    let identity = parts
        .perspective_identities
        .players
        .get(&perspective)
        .ok_or(PlayerEndpointError::ServiceUnavailable)?;
    let knowledge = parts
        .knowledge
        .players
        .get(&perspective)
        .ok_or(PlayerEndpointError::ServiceUnavailable)?;
    let mut stack = Vec::with_capacity(parts.zones.stack_records.len());
    for stack_id in parts.zones.stack_order.iter().rev() {
        let record = parts
            .zones
            .stack_records
            .get(stack_id)
            .ok_or(PlayerEndpointError::ServiceUnavailable)?;
        let payload = record
            .payload
            .as_ref()
            .ok_or(PlayerEndpointError::ServiceUnavailable)?;
        stack.push(project_public_stack_item_v1(
            parts,
            identity,
            knowledge,
            &parts.zones.stack_order,
            record.controller,
            payload,
        )?);
    }

    let mut temporary_effects = parts
        .execution
        .effects
        .values()
        .map(|effect| project_public_temporary_effect_v1(parts, identity, knowledge, effect))
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
        players: basic.players,
        pending_sba_ordering: project_sba_ordering_v4(parts, perspective)?,
        pending_blocks: basic.pending_blocks,
        pending_damage_assignment: basic.pending_damage_assignment,
        mana_pools: basic.mana_pools,
        counters: basic.counters,
        attachments: basic.attachments,
        faces: basic.faces,
        tapped: basic.tapped,
        permanents: basic.permanents,
        attacking: basic.attacking,
        blocked: basic.blocked,
        blocking: basic.blocking,
        stack,
        temporary_effects,
    })
}

pub(crate) fn project_public_stack_item_v1(
    state: &mtgml_state::EngineState,
    identity: &mtgml_state::PerspectiveIdentityRecordV2,
    knowledge: &mtgml_state::PlayerKnowledgeStateV2,
    stack_order: &[mtgml_model::StackObjectId],
    controller: PlayerId,
    payload: &StackItemPayload,
) -> Result<mtgml_observation::PublicStackItemV1, PlayerEndpointError> {
    let object_opaque = |object: mtgml_model::GameObjectId| {
        public_opaque_object(state, identity, knowledge, object)
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
        let source_must_be_public = source.source.snapshot.location.visibility
            == mtgml_state::VisibilityPartition::Public
            && !source.source.snapshot.face_down;
        if let Some(authority) = state
            .card_rules
            .abilities
            .by_instance
            .get(&source.ability_instance_id)
        {
            if authority.source != source.source.snapshot.object
                || authority.ability_key != source.ability_key.0
            {
                return Err(PlayerEndpointError::ServiceUnavailable);
            }
        } else if state
            .zones
            .objects
            .contains_key(&source.source.snapshot.object)
        {
            return Err(PlayerEndpointError::ServiceUnavailable);
        }
        let source_object =
            public_known_card_object(identity, knowledge, source.source.snapshot.object);
        if source_must_be_public && source_object.is_none() {
            return Err(PlayerEndpointError::ServiceUnavailable);
        }
        let source_ability = source_object.and_then(|_| {
            identity
                .ability_to_opaque
                .get(&source.ability_instance_id)
                .copied()
        });
        if source_object.is_some() && source_ability.is_none() {
            return Err(PlayerEndpointError::ServiceUnavailable);
        }
        Ok((source_object, source_ability))
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
            let (source_object, source_ability) = source_views(source_context)?;
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
            let (source_object, source_ability) = source_views(source_context)?;
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
    state: &mtgml_state::EngineState,
    identity: &mtgml_state::PerspectiveIdentityRecordV2,
    knowledge: &mtgml_state::PlayerKnowledgeStateV2,
    effect: &mtgml_state::TemporaryEffectRecord,
) -> Result<mtgml_observation::PublicTemporaryEffectV1, PlayerEndpointError> {
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

pub(crate) fn public_opaque_object(
    state: &mtgml_state::EngineState,
    identity: &mtgml_state::PerspectiveIdentityRecordV2,
    knowledge: &mtgml_state::PlayerKnowledgeStateV2,
    object: mtgml_model::GameObjectId,
) -> Option<mtgml_model::OpaqueObjectId> {
    let opaque = identity.object_to_opaque.get(&object).copied()?;
    let public_current_object = state
        .zones
        .objects
        .get(&object)
        .zip(state.zones.locations.get(&object))
        .is_some_and(|(record, location)| {
            location.visibility == mtgml_state::VisibilityPartition::Public && !record.face_down
        });
    let knowledge_authorized = knowledge.active.get(&opaque).is_some_and(|record| {
        record.opaque_object == opaque
            && record.known_location.as_ref().is_some_and(|fact| {
                (fact.location.visibility == mtgml_state::VisibilityPartition::Public
                    || matches!(
                        fact.provenance,
                        mtgml_state::KnowledgeAcquisitionReason::Observed {
                            channel: mtgml_state::KnowledgeHistoryChannel::Public,
                            cause: mtgml_state::KnowledgeAcquisitionCause::ExplicitReveal,
                            ..
                        }
                    ))
                    && public_knowledge_provenance(&fact.provenance)
            })
    });
    // A trusted opaque mapping is not visibility authority. Public current
    // state or public-provenance knowledge independently authorizes the ID.
    if !public_current_object && !knowledge_authorized {
        return None;
    }
    Some(opaque)
}

fn public_known_card_object(
    identity: &mtgml_state::PerspectiveIdentityRecordV2,
    knowledge: &mtgml_state::PlayerKnowledgeStateV2,
    object: mtgml_model::GameObjectId,
) -> Option<mtgml_model::OpaqueObjectId> {
    let opaque = identity.object_to_opaque.get(&object).copied()?;
    knowledge.active.get(&opaque).and_then(|record| {
        (record.opaque_object == opaque && record.card_definition.is_some()).then_some(opaque)
    })
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
    parts: &EngineState,
    perspective: PlayerId,
) -> Result<Option<MagicPendingSbaOrdering>, PlayerEndpointError> {
    let mut matching =
        parts
            .execution
            .continuations
            .values()
            .filter_map(|record| match &record.payload {
                mtgml_state::ContinuationPayload::MagicSbaGraveyardOrderV1 {
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

/// The perspective's retained knowledge in canonical order: ascending numeric
/// OpaqueObjectId across active and retired records (INFORMATION_MODEL.md).
fn project_retained_knowledge(
    knowledge: &mtgml_state::PlayerKnowledgeStateV2,
) -> Vec<PlayerKnownObjectV1> {
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
    retained_knowledge
        .into_iter()
        .map(|(_, object)| object)
        .collect()
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
