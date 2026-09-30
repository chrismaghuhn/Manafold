//! The authoritative engine state: one flat value. Every component is a
//! direct field; nothing is nested in a predecessor layer.

use mtgml_model::StateRevision;
use mtgml_random::RandomStateV1;

use crate::core::{CombatState, CoreRulesState};
use crate::engine_state_shape::{KnowledgeStateV2, PerspectiveIdentityStateV2};
use crate::execution::ExecutionStateV4;
use crate::format::FormatState;
use crate::identity::IdentityAllocatorState;
use crate::zones::ZoneState;
use crate::CardRulesAuthoritativeStateV1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineState {
    pub revision: StateRevision,
    pub core: CoreRulesState,
    pub combat: Option<CombatState>,
    pub zones: ZoneState,
    pub allocators: IdentityAllocatorState,
    pub execution: ExecutionStateV4,
    pub random: RandomStateV1,
    pub knowledge: KnowledgeStateV2,
    pub perspective_identities: PerspectiveIdentityStateV2,
    pub format: FormatState,
    pub card_rules: CardRulesAuthoritativeStateV1,
}
