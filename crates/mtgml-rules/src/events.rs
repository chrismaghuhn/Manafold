use mtgml_model::{DecisionId, GameObjectId, PlayerId, RuleEventId, StateRevision};
use mtgml_random::RandomStreamKeyV1;
use mtgml_state::{SemanticDeltaOperation, ZoneTransition};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AuthoritativeRuleEventKind {
    ZoneTransition {
        transition: Box<ZoneTransition>,
    },
    ObjectCeasedToExist {
        object: GameObjectId,
    },
    LifeChanged {
        player: PlayerId,
        from: i64,
        to: i64,
    },
    ObjectTapped {
        object: GameObjectId,
        from: bool,
        to: bool,
    },
    DecisionCreated {
        decision: DecisionId,
    },
    DecisionCleared {
        decision: DecisionId,
    },
    RandomValueSampled {
        stream: RandomStreamKeyV1,
        bound: u64,
        value: u64,
        raw_words_consumed: u64,
        cursor_before: u64,
        cursor_after: u64,
    },
    PublicOutcome {
        code: String,
    },
}

impl AuthoritativeRuleEventKind {
    pub fn semantic_delta(&self) -> SemanticDeltaOperation {
        match self {
            Self::ZoneTransition { transition } => SemanticDeltaOperation::ZoneTransition {
                transition: transition.clone(),
            },
            Self::ObjectCeasedToExist { object } => {
                SemanticDeltaOperation::ObjectCeasedToExist { object: *object }
            }
            Self::LifeChanged { player, from, to } => SemanticDeltaOperation::LifeChanged {
                player: *player,
                from: *from,
                to: *to,
            },
            Self::ObjectTapped { object, from, to } => SemanticDeltaOperation::ObjectTapped {
                object: *object,
                from: *from,
                to: *to,
            },
            Self::DecisionCreated { decision } => SemanticDeltaOperation::DecisionCreated {
                decision: *decision,
            },
            Self::DecisionCleared { decision } => SemanticDeltaOperation::DecisionCleared {
                decision: *decision,
            },
            Self::RandomValueSampled {
                stream,
                bound,
                value,
                raw_words_consumed,
                cursor_before,
                cursor_after,
            } => SemanticDeltaOperation::RandomValueSampled {
                stream: *stream,
                bound: *bound,
                value: *value,
                raw_words_consumed: *raw_words_consumed,
                cursor_before: *cursor_before,
                cursor_after: *cursor_after,
            },
            Self::PublicOutcome { code } => {
                SemanticDeltaOperation::PublicOutcome { code: code.clone() }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoritativeRuleEvent {
    pub event_id: RuleEventId,
    pub state_revision: StateRevision,
    pub event: AuthoritativeRuleEventKind,
}
