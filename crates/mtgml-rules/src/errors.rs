use mtgml_random::RandomValidationError;
use mtgml_state::{EngineStateViolation, IdentityAllocationError, StateDigestError};
use thiserror::Error;

use crate::turn_structure::{TurnStructureError, UnsupportedRulesBoundary};
use crate::TransitionViolation;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ZoneIncarnationError {
    #[error("selected source object is not live")]
    ObjectNotLive,
    #[error("claimed source location does not match the authoritative location")]
    ClaimedSourceLocationMismatch,
    #[error("source location is outside the selected transition family")]
    UnadmittedSourceFamily,
    #[error("claimed destination does not match the selected family and owner")]
    DestinationMismatch,
    #[error("selected transition requires a physical card identity")]
    PhysicalCardRequired,
    #[error("selected source profile is not admitted")]
    UnsupportedSourceProfile,
    #[error("library source is not the exact ordered top member")]
    LibrarySourceNotTop,
    #[error("graveyard order offset would overflow")]
    GraveyardOffsetOverflow,
    #[error("allocated game-object identity collides with a live object")]
    ObjectIdCollision,
    #[error("selected object is referenced by combat state")]
    CombatReference,
    #[error("selected object is referenced by a stack source record")]
    StackSourceReference,
    #[error("selected object is referenced by a pending trusted decision")]
    PendingDecisionReference,
    #[error("non-owner has a live identity mapping for a hidden Library source")]
    NonOwnerTracksHiddenSource,
    #[error("perspective identity/knowledge state cannot represent the selected move")]
    PerspectiveKnowledgeMismatch,
}

#[derive(Debug, Error)]
pub enum KernelExecutionError {
    #[error("before state is invalid: {0}")]
    BeforeState(EngineStateViolation),
    #[error("revision would overflow")]
    RevisionOverflow,
    #[error("rule event identity would overflow")]
    RuleEventIdOverflow,
    #[error("visible sequence would overflow")]
    VisibleSequenceOverflow,
    #[error("state delta construction failed: {0}")]
    Delta(StateDigestError),
    #[error("after state is invalid: {0}")]
    AfterState(EngineStateViolation),
    #[error("transition contract failed: {0}")]
    TransitionContract(TransitionViolation),
    #[error("deterministic RNG service failed: {0}")]
    Random(#[from] RandomValidationError),
    #[error("identity allocator failed: {0}")]
    IdentityAllocation(#[from] IdentityAllocationError),
    #[error("perspective lifecycle failed: {0}")]
    PerspectiveLifecycle(#[from] mtgml_state::LifecycleApplicationError),
    #[error("{0} identity space is exhausted")]
    Exhaustion(&'static str),
    #[error("engine-offered decision stage path is unsupported in the active rules profile")]
    UnsupportedStagePath,
    #[error("zone-incarnation request rejected: {0}")]
    ZoneIncarnation(ZoneIncarnationError),
    #[error("turn structure validation failed: {0}")]
    TurnStructure(TurnStructureError),
    #[error("player response is not accepted on this no-choice Magic path")]
    UnsupportedPlayerResponse,
    #[error("unsupported rules boundary: {0:?}")]
    UnsupportedRulesBoundary(UnsupportedRulesBoundary),
}
