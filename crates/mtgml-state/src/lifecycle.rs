//! Authoritative perspective-lifecycle audit payloads (M2.E, Issue #52).
//!
//! This module is the single authority for the state-changing semantics of one
//! perspective-visible occurrence: whose knowledge/identity state mutates, at
//! which perspective-local visible sequence, and with which typed identity and
//! knowledge mutation. Rules wrap these payloads in authoritative events;
//! `SemanticDeltaOperation` mirrors them verbatim; the semantic validation
//! cursor replays them; fixture support applies them through the normal
//! accepted-product path. Observation/redaction policy deliberately never
//! appears here: it does not mutate authoritative state.

use mtgml_model::{CardDefinitionId, GameObjectId, OpaqueObjectId, PlayerId, VisibleSequence};
use serde::{Deserialize, Serialize};

use crate::engine::EngineState;
use crate::knowledge::{KnowledgeAcquisitionReason, KnowledgeInvalidationReason};
use crate::m2_shape::{
    KnownLocationFactV2, KnowledgeInvalidationV2, KnowledgeRecordV2, PerspectiveIdentityRecordV2,
    PlayerKnowledgeStateV2, RetiredKnowledgeRecordV2,
};
use crate::zones::ZoneLocation;

/// Complete state-changing meaning of one perspective-visible occurrence.
///
/// `perspective` decides whose state mutates; `sequence` is the one consumed
/// perspective-local `VisibleSequence`; every observed provenance created or
/// updated by `mutation` must reference exactly this sequence. The cursor
/// advance is implicit and mandatory: an occurrence always consumes its value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PerspectiveLifecycleAuditV1 {
    pub perspective: PlayerId,
    pub sequence: VisibleSequence,
    pub mutation: PerspectiveLifecycleMutationV1,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PerspectiveLifecycleMutationV1 {
    #[serde(default)]
    pub identity: IdentityMutationV1,
    #[serde(default)]
    pub knowledge: Option<KnowledgeMutationV1>,
}

/// Typed perspective-identity mutation. `Retire` removes both mapping
/// directions, inserts into `retired_object_ids` and never touches the
/// allocator: retired IDs are never reused and never re-allocated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum IdentityMutationV1 {
    None,
    Allocate {
        opaque: OpaqueObjectId,
        object: GameObjectId,
    },
    Remap {
        opaque: OpaqueObjectId,
        from_object: GameObjectId,
        to_object: GameObjectId,
    },
    Retire {
        opaque: OpaqueObjectId,
        object: GameObjectId,
    },
}

/// Typed knowledge mutation. Every observed provenance carried by one of
/// these variants is causally bound to the enclosing audit's sequence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum KnowledgeMutationV1 {
    /// First authorized retention of an opaque identity for this perspective.
    Acquire {
        opaque: OpaqueObjectId,
        definition: Option<CardDefinitionId>,
        location: Option<ZoneLocation>,
        acquisition: KnowledgeAcquisitionReason,
    },
    /// Current known location updates: the previous current fact becomes the
    /// newest historical fact; the new fact carries its own provenance.
    UpdateLocation {
        opaque: OpaqueObjectId,
        fact: KnownLocationFactV2,
    },
    /// Destination becomes unknown while distinguishability persists: the
    /// current fact moves to history and the record stays active. An observed
    /// incarnation change may authorize a definition refresh in the same
    /// occurrence.
    CurrentToHistory {
        opaque: OpaqueObjectId,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        observed_definition: Option<CardDefinitionId>,
    },
    /// Active -> retired with exactly one typed invalidation. The previous
    /// current fact becomes the retired record's last-known location.
    Invalidate {
        opaque: OpaqueObjectId,
        reason: KnowledgeInvalidationReason,
        invalidation_provenance: KnowledgeAcquisitionReason,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum LifecycleApplicationError {
    #[error("lifecycle perspective is unknown")]
    UnknownPlayer,
    #[error("lifecycle sequence does not match the perspective cursor")]
    CursorMismatch,
    #[error("observed provenance must bind exactly the occurrence sequence")]
    ProvenanceSequence,
    #[error("lifecycle-created provenance must be observed")]
    UnsequencedProvenance,
    #[error("opaque identity is already known to this perspective")]
    DuplicateOpaque,
    #[error("opaque identity is retired and can never be reused")]
    OpaqueRetired,
    #[error("allocation does not consume the perspective allocator")]
    AllocationMismatch,
    #[error("allocator cursor overflow")]
    AllocatorOverflow,
    #[error("visible sequence cursor overflow")]
    CursorOverflow,
    #[error("live mapping does not match the declared identity mutation")]
    MappingMismatch,
    #[error("authoritative object is missing from the zones")]
    ObjectMissing,
    #[error("reverse identity index disagrees with the mutation")]
    ReverseMappingMismatch,
    #[error("knowledge record required for the mutation is missing")]
    UnknownKnowledge,
}

fn ensure_bound_provenance(
    provenance: &KnowledgeAcquisitionReason,
    sequence: VisibleSequence,
) -> Result<(), LifecycleApplicationError> {
    match provenance.observed_sequence() {
        None => Err(LifecycleApplicationError::UnsequencedProvenance),
        Some(observed) if observed == sequence => Ok(()),
        Some(_) => Err(LifecycleApplicationError::ProvenanceSequence),
    }
}

impl Default for IdentityMutationV1 {
    fn default() -> Self {
        Self::None
    }
}

/// Applies the complete state-changing meaning of one perspective-visible
/// occurrence to the perspective's knowledge and identity state.
///
/// The cursor advance is mandatory and exact: `next_visible_sequence` must
/// equal `audit.sequence` before the call and advances by exactly one. Every
/// observed provenance created here is validated to bind exactly the
/// occurrence sequence; provenance never allocates sequences independently.
pub fn apply_lifecycle_to_player(
    knowledge_slot: &mut PlayerKnowledgeStateV2,
    identity_slot: &mut PerspectiveIdentityRecordV2,
    object_exists: &dyn Fn(GameObjectId) -> bool,
    audit: &PerspectiveLifecycleAuditV1,
) -> Result<(), LifecycleApplicationError> {
    // The audit is atomic: any error leaves both components untouched.
    let mut knowledge = knowledge_slot.clone();
    let mut identity = identity_slot.clone();
    if knowledge.next_visible_sequence != audit.sequence {
        return Err(LifecycleApplicationError::CursorMismatch);
    }
    match &audit.mutation.identity {
        IdentityMutationV1::None => {}
        IdentityMutationV1::Allocate { opaque, object } => {
            if opaque.0 != identity.next_opaque_object_id.0 {
                return Err(LifecycleApplicationError::AllocationMismatch);
            }
            if identity.opaque_to_object.contains_key(opaque)
                || identity.object_to_opaque.contains_key(object)
            {
                return Err(LifecycleApplicationError::DuplicateOpaque);
            }
            if identity.retired_object_ids.contains(opaque) {
                return Err(LifecycleApplicationError::OpaqueRetired);
            }
            if !object_exists(*object) {
                return Err(LifecycleApplicationError::ObjectMissing);
            }
            identity.opaque_to_object.insert(*opaque, *object);
            identity.object_to_opaque.insert(*object, *opaque);
            identity.next_opaque_object_id = OpaqueObjectId(
                identity
                    .next_opaque_object_id
                    .0
                    .checked_add(1)
                    .ok_or(LifecycleApplicationError::AllocatorOverflow)?,
            );
        }
        IdentityMutationV1::Remap {
            opaque,
            from_object,
            to_object,
        } => {
            if from_object == to_object {
                return Err(LifecycleApplicationError::MappingMismatch);
            }
            let current = identity
                .opaque_to_object
                .get(opaque)
                .copied()
                .ok_or(LifecycleApplicationError::MappingMismatch)?;
            if current != *from_object
                || identity.object_to_opaque.get(from_object) != Some(opaque)
                || !object_exists(*to_object)
                || identity.object_to_opaque.contains_key(to_object)
            {
                return Err(LifecycleApplicationError::ReverseMappingMismatch);
            }
            identity.opaque_to_object.insert(*opaque, *to_object);
            identity.object_to_opaque.remove(from_object);
            identity.object_to_opaque.insert(*to_object, *opaque);
        }
        IdentityMutationV1::Retire { opaque, object } => {
            if identity.opaque_to_object.get(opaque) != Some(object)
                || identity.object_to_opaque.get(object) != Some(opaque)
            {
                return Err(LifecycleApplicationError::MappingMismatch);
            }
            identity.opaque_to_object.remove(opaque);
            identity.object_to_opaque.remove(object);
            identity.retired_object_ids.insert(*opaque);
        }
    }
    match &audit.mutation.knowledge {
        None => {}
        Some(KnowledgeMutationV1::Acquire {
            opaque,
            definition,
            location,
            acquisition,
        }) => {
            if knowledge.retired.contains_key(opaque) {
                return Err(LifecycleApplicationError::OpaqueRetired);
            }
            if knowledge.active.contains_key(opaque) {
                return Err(LifecycleApplicationError::DuplicateOpaque);
            }
            ensure_bound_provenance(acquisition, audit.sequence)?;
            let known_location = location
                .clone()
                .map(|location| KnownLocationFactV2 {
                    location,
                    provenance: *acquisition,
                });
            knowledge.active.insert(
                *opaque,
                KnowledgeRecordV2 {
                    opaque_object: *opaque,
                    physical_card: None,
                    card_definition: *definition,
                    known_location,
                    historical_locations: Vec::new(),
                    acquisition: *acquisition,
                },
            );
        }
        Some(KnowledgeMutationV1::UpdateLocation { opaque, fact }) => {
            let record = knowledge
                .active
                .get_mut(opaque)
                .ok_or(LifecycleApplicationError::UnknownKnowledge)?;
            ensure_bound_provenance(&fact.provenance, audit.sequence)?;
            if let Some(current) = record.known_location.take() {
                record.historical_locations.push(current);
            }
            record.known_location = Some(fact.clone());
        }
        Some(KnowledgeMutationV1::CurrentToHistory {
            opaque,
            observed_definition,
        }) => {
            let record = knowledge
                .active
                .get_mut(opaque)
                .ok_or(LifecycleApplicationError::UnknownKnowledge)?;
            if let Some(current) = record.known_location.take() {
                record.historical_locations.push(current);
            }
            if let Some(definition) = *observed_definition {
                record.card_definition = Some(definition);
            }
        }
        Some(KnowledgeMutationV1::Invalidate {
            opaque,
            reason,
            invalidation_provenance,
        }) => {
            ensure_bound_provenance(invalidation_provenance, audit.sequence)?;
            let record = knowledge
                .active
                .remove(opaque)
                .ok_or(LifecycleApplicationError::UnknownKnowledge)?;
            knowledge.retired.insert(
                *opaque,
                RetiredKnowledgeRecordV2 {
                    opaque_object: record.opaque_object,
                    physical_card: record.physical_card,
                    card_definition: record.card_definition,
                    last_known_location: record.known_location,
                    historical_locations: record.historical_locations,
                    acquisition: record.acquisition,
                    invalidation: KnowledgeInvalidationV2 {
                        provenance: *invalidation_provenance,
                        reason: *reason,
                    },
                },
            );
        }
    }
    knowledge.next_visible_sequence = VisibleSequence(
        knowledge
            .next_visible_sequence
            .0
            .checked_add(1)
            .ok_or(LifecycleApplicationError::CursorOverflow)?,
    );
    *knowledge_slot = knowledge;
    *identity_slot = identity;
    Ok(())
}

/// Engine-level application: resolves the perspective and the authoritative
/// zone existence predicate, then delegates to [`apply_lifecycle_to_player`].
pub fn apply_perspective_lifecycle(
    state: &mut EngineState,
    audit: &PerspectiveLifecycleAuditV1,
) -> Result<(), LifecycleApplicationError> {
    if !state.core.players.contains_key(&audit.perspective) {
        return Err(LifecycleApplicationError::UnknownPlayer);
    }
    let zones_objects = &state.zones.objects;
    let knowledge = state
        .knowledge
        .players
        .get_mut(&audit.perspective)
        .ok_or(LifecycleApplicationError::UnknownPlayer)?;
    let identity = state
        .perspective_identities
        .players
        .get_mut(&audit.perspective)
        .ok_or(LifecycleApplicationError::UnknownPlayer)?;
    apply_lifecycle_to_player(knowledge, identity, &|object| {
        zones_objects.contains_key(&object)
    }, audit)
}
