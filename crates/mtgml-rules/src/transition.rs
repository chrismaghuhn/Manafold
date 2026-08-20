use mtgml_decision::PlayerDecisionRequest;
use mtgml_model::EpisodeStatus;
use mtgml_state::{EngineState, StateDelta};

use crate::events::AuthoritativeRuleEvent;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionResult {
    pub accepted: bool,
    pub next_state: EngineState,
    pub delta: StateDelta,
    pub events: Vec<AuthoritativeRuleEvent>,
    pub next_decision: Option<PlayerDecisionRequest>,
    pub status: EpisodeStatus,
}

pub trait RulesKernel: Send {
    fn apply(
        &mut self,
        state: &EngineState,
        response: &mtgml_decision::DecisionResponse,
    ) -> TransitionResult;
}
