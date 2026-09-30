use mtgml_state::IdentityAllocationError;
use thiserror::Error;

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
    #[error("non-owner has a live identity mapping for a hidden Library source")]
    NonOwnerTracksHiddenSource,
    #[error("perspective identity/knowledge state cannot represent the selected move")]
    PerspectiveKnowledgeMismatch,
}

#[derive(Debug, Error)]
pub enum KernelExecutionError {
    #[error("transition contract failed: {0}")]
    TransitionContract(TransitionViolation),
    #[error("identity allocator failed: {0}")]
    IdentityAllocation(#[from] IdentityAllocationError),
    #[error("perspective lifecycle failed: {0}")]
    PerspectiveLifecycle(#[from] mtgml_state::LifecycleApplicationError),
    #[error("zone-incarnation request rejected: {0}")]
    ZoneIncarnation(ZoneIncarnationError),
}
