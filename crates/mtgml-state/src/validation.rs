#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EngineStateViolation {
    #[error("active player is absent")]
    MissingTurnPlayer,
    #[error("priority state is structurally invalid")]
    PriorityState,
    #[error("combat state is structurally invalid")]
    CombatState,
    #[error("object map key does not equal object identity")]
    ObjectKeyMismatch,
    #[error("object owner/controller or zone player is absent")]
    ObjectPlayerMismatch,
    #[error("objects and locations are not bijective")]
    ObjectLocationMismatch,
    #[error("a physical card identifies more than one live game-object incarnation")]
    DuplicatePhysicalCard,
    #[error("ordered zones contain a missing, duplicated, or wrongly located object")]
    OrderedZoneMismatch,
    #[error("stack records and stack order are not bijective")]
    StackMismatch,
    #[error("an identity allocator does not exceed every allocated identity")]
    AllocatorBehind,
    #[error("perspective identities are not bijective or reference missing objects")]
    PerspectiveIdentityMismatch,
    #[error("knowledge state references an absent player/object or has invalid provenance")]
    KnowledgeMismatch,
    #[error("format state references absent players or undesignated commanders")]
    FormatMismatch,
    #[error("random state is invalid")]
    RandomState,
    #[error("engine state shape is invalid: {0}")]
    EngineStateShape(#[from] crate::engine_state_shape::EngineStateShapeViolation),
}

mod allocators_execution;
mod core;
mod format;
mod information;
mod random;
mod zones;

use std::collections::BTreeSet;

use thiserror::Error;

use crate::engine::EngineState;
use crate::engine_state_shape::validate_engine_state_shape;

use self::allocators_execution::validate_allocators;
use self::core::validate_core_structure;
use self::format::validate_commander_format_references;
use self::information::{
    validate_perspective_identity_relationships, validate_retained_knowledge_against_live_state,
};
use self::random::validate_authoritative_random_state;
use self::zones::validate_zone_structure;

/// Ordered coordinator over the extracted CURRENT validation segments.
///
/// The error precedence of this function is FROZEN; each delegated segment
/// occupies exactly the position its inline block occupied before Issue #62.
pub fn validate_engine_state(state: &EngineState) -> Result<(), EngineStateViolation> {
    validate_zone_structure(state)?;
    validate_core_structure(state)?;
    validate_allocators(state)?;

    let players: BTreeSet<_> = state.core.players.keys().copied().collect();
    validate_engine_state_shape(&players, &state.knowledge, &state.perspective_identities)?;

    validate_retained_knowledge_against_live_state(state, &players)?;
    validate_perspective_identity_relationships(state)?;
    validate_commander_format_references(state)?;
    validate_authoritative_random_state(state)?;
    Ok(())
}
