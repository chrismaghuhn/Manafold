#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EngineStateViolation {
    #[error("active or priority player is absent")]
    MissingTurnPlayer,
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
    #[error("pending decision is invalid for this state")]
    PendingDecisionMismatch,
    #[error("continuation reference is missing")]
    MissingContinuation,
    #[error("execution record keys do not match their embedded identities")]
    ExecutionMismatch,
    #[error("perspective identities are not bijective or reference missing objects")]
    PerspectiveIdentityMismatch,
    #[error("knowledge state references an absent player/object or has invalid provenance")]
    KnowledgeMismatch,
    #[error("format state references absent players or undesignated commanders")]
    FormatMismatch,
    #[error("random state is invalid")]
    RandomState,
    #[error("M2 state shape is invalid: {0}")]
    M2Shape(#[from] crate::m2_shape::M2ShapeViolation),
}

mod allocators_execution;
mod decision;
mod format;
mod information;
mod random;
mod zones;

use std::collections::BTreeSet;

use mtgml_decision::{EngineCandidateBinding, VisibleCandidateV2};
use thiserror::Error;

use crate::engine::EngineState;
use crate::m2_shape::validate_m2_shape;

use self::allocators_execution::validate_allocators_and_execution;
use self::decision::validate_pending_authoritative_request;
use self::format::validate_commander_format_references;
use self::information::{
    validate_perspective_identity_relationships, validate_retained_knowledge_against_live_state,
};
use self::random::validate_authoritative_random_state;
use self::zones::validate_zone_structure;

#[allow(dead_code)]
fn _binding_type_marker(_: &EngineCandidateBinding, _: &VisibleCandidateV2) {}

/// Ordered coordinator over the extracted CURRENT validation segments.
///
/// The error precedence of this function is FROZEN; each delegated segment
/// occupies exactly the position its inline block occupied before Issue #62.
pub fn validate_engine_state(state: &EngineState) -> Result<(), EngineStateViolation> {
    validate_zone_structure(state)?;
    validate_allocators_and_execution(state)?;

    let players: BTreeSet<_> = state.core.players.keys().copied().collect();
    validate_m2_shape(
        state.revision,
        &players,
        state.execution.pending_decision.as_ref(),
        &state.execution.continuations,
        &state.knowledge,
        &state.perspective_identities,
    )?;

    validate_retained_knowledge_against_live_state(state, &players)?;
    validate_pending_authoritative_request(state)?;
    validate_perspective_identity_relationships(state)?;
    validate_commander_format_references(state)?;
    validate_authoritative_random_state(state)?;
    Ok(())
}
