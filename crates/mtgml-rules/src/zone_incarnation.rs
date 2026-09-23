//! Private rules-owned entry seam for the selected zone-incarnation capability.
//!
//! This module owns the single future production implementation point. The
//! conformance facade below only translates its closed test vocabulary and
//! delegates here; it contains no transition behavior.

use mtgml_model::GameObjectId;
use mtgml_state::EngineState;

use crate::{KernelExecutionError, TransitionResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SelectedZoneTransitionKind {
    BattlefieldToOwnerGraveyard,
    LibraryTopToOwnerHand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SelectedZoneTransitionRequest {
    pub object: GameObjectId,
    pub kind: SelectedZoneTransitionKind,
}

/// The one future implementation point for both selected transition families.
///
/// FIX-01 deliberately remains fail-closed: it does not inspect or mutate
/// state, allocate identities, consume randomness, or emit products.
pub(crate) fn execute_selected_zone_transition(
    _state: &EngineState,
    _request: &SelectedZoneTransitionRequest,
) -> Result<TransitionResult, KernelExecutionError> {
    Err(KernelExecutionError::ZoneIncarnationUnavailable)
}

/// Closed family vocabulary available only when the conformance testkit feature
/// is explicitly enabled. It is not a runtime request or environment API.
#[cfg(feature = "m3-conformance-testkit")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConformanceZoneTransitionKind {
    BattlefieldToOwnerGraveyard,
    LibraryTopToOwnerHand,
}

/// Narrow conformance bridge to the private production-owned seam.
#[cfg(feature = "m3-conformance-testkit")]
pub fn execute_selected_zone_transition_for_conformance(
    state: &EngineState,
    object: GameObjectId,
    kind: ConformanceZoneTransitionKind,
) -> Result<TransitionResult, KernelExecutionError> {
    let kind = match kind {
        ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard => {
            SelectedZoneTransitionKind::BattlefieldToOwnerGraveyard
        }
        ConformanceZoneTransitionKind::LibraryTopToOwnerHand => {
            SelectedZoneTransitionKind::LibraryTopToOwnerHand
        }
    };
    execute_selected_zone_transition(&state, &SelectedZoneTransitionRequest { object, kind })
}
