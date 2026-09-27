use mtgml_decision::{AuthoritativeDecisionRequestV2, DecisionResponseV2};
use mtgml_model::{EpisodeStatus, PlayerId};
use mtgml_state::{EngineState, StateDelta};

use crate::errors::KernelExecutionError;
use crate::events::AuthoritativeRuleEvent;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PredecessorTransitionResult {
    pub accepted: bool,
    pub next_state: EngineState,
    pub delta: StateDelta,
    pub events: Vec<AuthoritativeRuleEvent>,
    pub next_decision: Option<AuthoritativeDecisionRequestV2>,
    pub status: EpisodeStatus,
}

/// The same rules transition authority's complete successor product. This is
/// the environment-facing product on the accepted successor integration
/// branch; the legacy-shaped result remains only for predecessor fixtures
/// while callers migrate in place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionResult {
    pub accepted: bool,
    pub next_state: mtgml_state::EngineStatePartsV2,
    pub delta: mtgml_state::StateDeltaV2,
    pub events: Vec<crate::AuthoritativeRuleEventV2>,
    pub next_decision: Option<mtgml_decision::AuthoritativeDecisionRequestV3>,
    pub status: EpisodeStatus,
}

pub trait RulesKernel: Send {
    fn apply(
        &mut self,
        state: &mtgml_state::EngineStatePartsV2,
        trusted_actor: PlayerId,
        response: &DecisionResponseV2,
        status: &EpisodeStatus,
    ) -> Result<TransitionResult, KernelExecutionError>;
}
