//! Typed state components used by current `EngineState` validation.
//!
//! This module provides the continuation, retained-knowledge, and
//! perspective-identity shapes embedded in `EngineState`, plus their structural
//! checks. `validation::validate_engine_state` composes these checks with the
//! other authoritative state validators. The types do not adapt or
//! reinterpret any historical V1/V2 value.

mod continuation;
mod knowledge;
mod perspective_identity;

use std::collections::{BTreeMap, BTreeSet};

use mtgml_model::{ContinuationId, GameObjectId, PlayerId, StateRevision};
use thiserror::Error;

use crate::zones::GameObject;

use self::continuation::{
    validate_magic_sba_graveyard_order, validate_program_coherence, validate_synthetic_assembly,
    MagicSbaGraveyardOrderValidation,
};
pub use self::continuation::{
    AssemblyStageV2, ContinuationPayloadV2, ContinuationRecordV2, PendingDecisionRecordV2,
    SbaGraveyardOwnerOrderV1, SbaObjectCauseV1, SbaSelectedActionV1,
};
use self::knowledge::validate_knowledge;
pub use self::knowledge::{
    KnowledgeInvalidationV2, KnowledgeRecordV2, KnowledgeStateV2, KnownLocationFactV2,
    PlayerKnowledgeStateV2, RetiredKnowledgeRecordV2,
};
use self::perspective_identity::validate_identity;
pub use self::perspective_identity::{PerspectiveIdentityRecordV2, PerspectiveIdentityStateV2};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum EngineStateShapeViolation {
    #[error("engine state does not cover exactly the declared players")]
    PlayerCoverage,
    #[error("perspective-local allocator is missing or behind")]
    Allocator,
    #[error("opaque mapping is not bijective")]
    IdentityMapping,
    #[error("opaque identity is active and retired simultaneously")]
    RetiredIdentity,
    #[error("retained knowledge shape is invalid")]
    Knowledge,
    #[error("visible sequence is not strictly monotonic")]
    VisibleSequence,
    #[error("pending decision is invalid")]
    PendingDecision,
    #[error("pending decision references a missing continuation")]
    ContinuationReference,
    #[error("continuation stage is owned by a different actor than its request")]
    ContinuationActor,
    #[error("continuation revision is stale or future-dated")]
    ContinuationRevision,
    #[error("continuation stage is invalid")]
    ContinuationStage,
    #[error("Magic SBA Graveyard-order continuation is structurally inconsistent")]
    MagicContinuation,
}

/// Inclusive numeric interval of the synthetic assembly ChooseCount stage.
///
/// This is the single authority for the frozen synthetic-program bound; the rules
/// kernel consumes these values instead of restating them.
pub const SYNTHETIC_COUNT_MIN: u32 = 0;
pub const SYNTHETIC_COUNT_MAX: u32 = 3;

pub fn validate_engine_state_shape(
    current_revision: StateRevision,
    players: &BTreeSet<PlayerId>,
    objects: &BTreeMap<GameObjectId, GameObject>,
    pending: Option<&PendingDecisionRecordV2>,
    continuations: &BTreeMap<ContinuationId, ContinuationRecordV2>,
    knowledge: &KnowledgeStateV2,
    identities: &PerspectiveIdentityStateV2,
) -> Result<(), EngineStateShapeViolation> {
    if knowledge.players.keys().copied().collect::<BTreeSet<_>>() != *players
        || identities.players.keys().copied().collect::<BTreeSet<_>>() != *players
    {
        return Err(EngineStateShapeViolation::PlayerCoverage);
    }

    for (player, identity) in &identities.players {
        validate_identity(identity)?;
        if identity
            .opaque_to_object
            .values()
            .any(|object| !identity.object_to_opaque.contains_key(object))
            || identity
                .opaque_to_ability
                .values()
                .any(|ability| !identity.ability_to_opaque.contains_key(ability))
        {
            return Err(EngineStateShapeViolation::IdentityMapping);
        }
        let player_knowledge = knowledge
            .players
            .get(player)
            .ok_or(EngineStateShapeViolation::PlayerCoverage)?;
        validate_knowledge(player_knowledge)?;
    }

    for continuation in continuations.values() {
        if !players.contains(&continuation.actor) {
            return Err(EngineStateShapeViolation::PlayerCoverage);
        }
        if continuation.created_at_revision > current_revision {
            return Err(EngineStateShapeViolation::ContinuationRevision);
        }
        if continuation.stage_index != continuation.payload.stage_index() {
            return Err(EngineStateShapeViolation::ContinuationStage);
        }
        match &continuation.payload {
            ContinuationPayloadV2::SyntheticM2Assembly {
                stage,
                selected_count,
                selected_piece_keys,
                ordered_piece_keys,
            } => {
                validate_synthetic_assembly(
                    *stage,
                    *selected_count,
                    selected_piece_keys,
                    ordered_piece_keys,
                )?;
            }
            ContinuationPayloadV2::MagicSbaGraveyardOrderV1 {
                round_start_revision,
                selected_sba_actions,
                apnap_owners,
                next_owner_index,
                completed_owner_orders,
            } => {
                validate_magic_sba_graveyard_order(MagicSbaGraveyardOrderValidation {
                    round_start_revision: *round_start_revision,
                    continuation_created_at_revision: continuation.created_at_revision,
                    selected_sba_actions,
                    apnap_owners,
                    next_owner_index: *next_owner_index,
                    completed_owner_orders,
                    current_revision,
                    players,
                    objects,
                })?;
            }
        }
    }

    if let Some(pending) = pending {
        pending
            .request
            .validate()
            .map_err(|_| EngineStateShapeViolation::PendingDecision)?;
        if pending.request.state_revision != current_revision
            || !players.contains(&pending.request.actor)
        {
            return Err(EngineStateShapeViolation::PendingDecision);
        }
        if let Some(continuation_id) = pending.request.continuation_id {
            let continuation = continuations
                .get(&continuation_id)
                .ok_or(EngineStateShapeViolation::ContinuationReference)?;
            // DECISION_PROTOCOL.md: the endpoint bound to the actor projects
            // the request; ADR 0039 serializes the owning actor as
            // continuation state. M2 has no accepted stage-transfer
            // semantics, so a referenced continuation must belong to the
            // pending request's actor.
            if continuation.actor != pending.request.actor {
                return Err(EngineStateShapeViolation::ContinuationActor);
            }
            if continuation.created_at_revision > pending.request.state_revision {
                return Err(EngineStateShapeViolation::ContinuationRevision);
            }
            if continuation.stage_index != continuation.payload.stage_index() {
                return Err(EngineStateShapeViolation::ContinuationStage);
            }
        }
    }
    validate_program_coherence(pending, continuations, players, objects, identities)?;
    Ok(())
}
