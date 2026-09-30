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

use mtgml_model::{GameObjectId, PlayerId, StateRevision};
use thiserror::Error;

use crate::zones::GameObject;

use self::continuation::{
    validate_magic_sba_graveyard_order, validate_synthetic_assembly,
    MagicSbaGraveyardOrderValidation,
};
pub use self::continuation::{
    AssemblyStageV2, SbaGraveyardOwnerOrderV1, SbaObjectCauseV1, SbaSelectedActionV1,
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
    #[error("continuation revision is stale or future-dated")]
    ContinuationRevision,
    #[error("Magic SBA Graveyard-order continuation is structurally inconsistent")]
    MagicContinuation,
}

/// Inclusive numeric interval of the synthetic assembly ChooseCount stage.
///
/// This is the single authority for the frozen synthetic-program bound; the rules
/// kernel consumes these values instead of restating them.
pub const SYNTHETIC_COUNT_MAX: u32 = 3;

pub(crate) fn validate_successor_synthetic_assembly(
    stage: AssemblyStageV2,
    selected_count: Option<u32>,
    selected_piece_keys: &[u32],
    ordered_piece_keys: &[u32],
) -> Result<(), EngineStateShapeViolation> {
    validate_synthetic_assembly(
        stage,
        selected_count,
        selected_piece_keys,
        ordered_piece_keys,
    )
}

pub(crate) struct SuccessorMagicSbaGraveyardOrderValidation<'a> {
    pub round_start_revision: StateRevision,
    pub continuation_created_at_revision: StateRevision,
    pub selected_sba_actions: &'a [SbaSelectedActionV1],
    pub apnap_owners: &'a [PlayerId],
    pub next_owner_index: u32,
    pub completed_owner_orders: &'a [SbaGraveyardOwnerOrderV1],
    pub current_revision: StateRevision,
    pub players: &'a BTreeSet<PlayerId>,
    pub objects: &'a BTreeMap<GameObjectId, GameObject>,
}

pub(crate) fn validate_successor_magic_sba_graveyard_order(
    validation: SuccessorMagicSbaGraveyardOrderValidation<'_>,
) -> Result<(), EngineStateShapeViolation> {
    let SuccessorMagicSbaGraveyardOrderValidation {
        round_start_revision,
        continuation_created_at_revision,
        selected_sba_actions,
        apnap_owners,
        next_owner_index,
        completed_owner_orders,
        current_revision,
        players,
        objects,
    } = validation;
    validate_magic_sba_graveyard_order(MagicSbaGraveyardOrderValidation {
        round_start_revision,
        continuation_created_at_revision,
        selected_sba_actions,
        apnap_owners,
        next_owner_index,
        completed_owner_orders,
        current_revision,
        players,
        objects,
    })
    .map(|_| ())
}

pub fn validate_engine_state_shape(
    players: &BTreeSet<PlayerId>,
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
    Ok(())
}
