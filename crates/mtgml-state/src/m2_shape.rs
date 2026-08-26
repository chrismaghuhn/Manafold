//! Private, detached M2 state shapes prepared before the current-runtime cut.
//!
//! Nothing in this module is reachable through the current `EngineState` until
//! the coordinated Tasks 7-11 cut. The types deliberately do not adapt or
//! reinterpret any historical V1/V2 value.

mod continuation;
mod knowledge;
mod perspective_identity;

use std::collections::{BTreeMap, BTreeSet};

use mtgml_model::{ContinuationId, PlayerId, StateRevision};
use thiserror::Error;

use self::continuation::{validate_program_coherence, validate_synthetic_assembly};
pub use self::continuation::{
    AssemblyStageV2, ContinuationPayloadV2, ContinuationRecordV2, PendingDecisionRecordV2,
};
use self::knowledge::validate_knowledge;
pub use self::knowledge::{
    KnowledgeInvalidationV2, KnowledgeRecordV2, KnowledgeStateV2, KnownLocationFactV2,
    PlayerKnowledgeStateV2, RetiredKnowledgeRecordV2,
};
use self::perspective_identity::validate_identity;
pub use self::perspective_identity::{PerspectiveIdentityRecordV2, PerspectiveIdentityStateV2};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum M2ShapeViolation {
    #[error("M2 state does not cover exactly the declared players")]
    PlayerCoverage,
    #[error("M2 perspective-local allocator is missing or behind")]
    Allocator,
    #[error("M2 opaque mapping is not bijective")]
    IdentityMapping,
    #[error("M2 opaque identity is active and retired simultaneously")]
    RetiredIdentity,
    #[error("M2 retained knowledge shape is invalid")]
    Knowledge,
    #[error("M2 visible sequence is not strictly monotonic")]
    VisibleSequence,
    #[error("M2 pending decision is invalid")]
    PendingDecision,
    #[error("M2 pending decision references a missing continuation")]
    ContinuationReference,
    #[error("M2 continuation stage is owned by a different actor than its request")]
    ContinuationActor,
    #[error("M2 continuation revision is stale or future-dated")]
    ContinuationRevision,
    #[error("M2 continuation stage is invalid")]
    ContinuationStage,
}

/// Inclusive numeric interval of the synthetic assembly ChooseCount stage.
///
/// This is the single authority for the frozen M2.C program bound; the rules
/// kernel consumes these values instead of restating them.
pub const SYNTHETIC_COUNT_MIN: u32 = 0;
pub const SYNTHETIC_COUNT_MAX: u32 = 3;

pub fn validate_m2_shape(
    current_revision: StateRevision,
    players: &BTreeSet<PlayerId>,
    pending: Option<&PendingDecisionRecordV2>,
    continuations: &BTreeMap<ContinuationId, ContinuationRecordV2>,
    knowledge: &KnowledgeStateV2,
    identities: &PerspectiveIdentityStateV2,
) -> Result<(), M2ShapeViolation> {
    if knowledge.players.keys().copied().collect::<BTreeSet<_>>() != *players
        || identities.players.keys().copied().collect::<BTreeSet<_>>() != *players
    {
        return Err(M2ShapeViolation::PlayerCoverage);
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
            return Err(M2ShapeViolation::IdentityMapping);
        }
        let player_knowledge = knowledge
            .players
            .get(player)
            .ok_or(M2ShapeViolation::PlayerCoverage)?;
        validate_knowledge(player_knowledge)?;
    }

    for continuation in continuations.values() {
        if !players.contains(&continuation.actor) {
            return Err(M2ShapeViolation::PlayerCoverage);
        }
        if continuation.created_at_revision > current_revision {
            return Err(M2ShapeViolation::ContinuationRevision);
        }
        if continuation.stage_index != continuation.payload.stage_index() {
            return Err(M2ShapeViolation::ContinuationStage);
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
        }
    }

    if let Some(pending) = pending {
        pending
            .request
            .validate()
            .map_err(|_| M2ShapeViolation::PendingDecision)?;
        if pending.request.state_revision != current_revision
            || !players.contains(&pending.request.actor)
        {
            return Err(M2ShapeViolation::PendingDecision);
        }
        if let Some(continuation_id) = pending.request.continuation_id {
            let continuation = continuations
                .get(&continuation_id)
                .ok_or(M2ShapeViolation::ContinuationReference)?;
            // DECISION_PROTOCOL.md: the endpoint bound to the actor projects
            // the request; ADR 0039 serializes the owning actor as
            // continuation state. M2 has no accepted stage-transfer
            // semantics, so a referenced continuation must belong to the
            // pending request's actor.
            if continuation.actor != pending.request.actor {
                return Err(M2ShapeViolation::ContinuationActor);
            }
            if continuation.created_at_revision > pending.request.state_revision {
                return Err(M2ShapeViolation::ContinuationRevision);
            }
            if continuation.stage_index != continuation.payload.stage_index() {
                return Err(M2ShapeViolation::ContinuationStage);
            }
        }
    }
    validate_program_coherence(pending, continuations)?;
    Ok(())
}
