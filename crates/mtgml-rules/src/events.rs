use mtgml_model::{
    ContinuationId, DecisionId, GameObjectId, PlayerId, RuleEventId, StateRevision, ZoneKind,
};
use mtgml_random::RandomStreamKeyV1;
use mtgml_state::{
    PerspectiveLifecycleAuditV1, SemanticDeltaOperation, TurnPosition, ZoneTransition,
};
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
    CombatDamageDealt {
        assignments: Vec<mtgml_state::DamageAssignmentV1>,
    },
    CombatDamageStepCompleted,
    MarkedDamageChanged {
        creature: GameObjectId,
        from: u64,
        to: u64,
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
    SbaGraveyardOrderChosen {
        continuation: ContinuationId,
        owner: PlayerId,
        top_to_bottom: Vec<GameObjectId>,
    },
    StateBasedActionsApplied {
        actions: Vec<mtgml_state::SbaSelectedActionV1>,
    },
    PriorityChanged {
        from: mtgml_state::PriorityState,
        to: mtgml_state::PriorityState,
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
    TurnPositionChanged {
        from: TurnPosition,
        to: TurnPosition,
    },
    AttackersDeclared {
        defending_player: PlayerId,
        attackers: Vec<GameObjectId>,
    },
    BlockersDeclared {
        assignments: Vec<mtgml_state::CombatBlockerAssignmentV1>,
    },
    CombatEnded,
    EmptyCombatStepsSkipped,
    UntapCompleted {
        affected_objects: Vec<GameObjectId>,
    },
    ActivePlayerChanged {
        from: PlayerId,
        to: PlayerId,
    },
    TurnNumberChanged {
        from: u64,
        to: u64,
    },
    /// One complete perspective-visible occurrence (M2.E). The state-owned
    /// `lifecycle` payload is the single authority for perspective, consumed
    /// visible sequence, and the typed identity/knowledge mutation; the
    /// rules-owned `observation` policy carries only perception/authorization
    /// data and is deliberately excluded from the authoritative audit.
    PerspectiveOccurrence {
        lifecycle: PerspectiveLifecycleAuditV1,
        observation: PerspectiveObservationPolicyV1,
    },
}

impl AuthoritativeRuleEventKind {
    pub fn semantic_delta(&self) -> SemanticDeltaOperation {
        match self {
            Self::PerspectiveOccurrence { lifecycle, .. } => {
                SemanticDeltaOperation::PerspectiveLifecycle {
                    lifecycle: lifecycle.clone(),
                }
            }
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
            Self::CombatDamageDealt { assignments } => SemanticDeltaOperation::CombatDamageDealt {
                assignments: assignments.clone(),
            },
            Self::CombatDamageStepCompleted => SemanticDeltaOperation::CombatDamageStepCompleted,
            Self::MarkedDamageChanged { creature, from, to } => {
                SemanticDeltaOperation::MarkedDamageChanged {
                    creature: *creature,
                    from: *from,
                    to: *to,
                }
            }
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
            Self::SbaGraveyardOrderChosen {
                continuation,
                owner,
                top_to_bottom,
            } => SemanticDeltaOperation::SbaGraveyardOrderChosen {
                continuation: *continuation,
                owner: *owner,
                top_to_bottom: top_to_bottom.clone(),
            },
            Self::StateBasedActionsApplied { actions } => {
                SemanticDeltaOperation::StateBasedActionsApplied {
                    actions: actions.clone(),
                }
            }
            Self::PriorityChanged { from, to } => SemanticDeltaOperation::PriorityChanged {
                from: *from,
                to: *to,
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
            Self::TurnPositionChanged { from, to } => SemanticDeltaOperation::TurnPositionChanged {
                from: *from,
                to: *to,
            },
            Self::AttackersDeclared {
                defending_player,
                attackers,
            } => SemanticDeltaOperation::AttackersDeclared {
                defending_player: *defending_player,
                attackers: attackers.clone(),
            },
            Self::BlockersDeclared { assignments } => SemanticDeltaOperation::BlockersDeclared {
                assignments: assignments.clone(),
            },
            Self::CombatEnded => SemanticDeltaOperation::CombatEnded,
            Self::EmptyCombatStepsSkipped => SemanticDeltaOperation::EmptyCombatStepsSkipped,
            Self::UntapCompleted { affected_objects } => SemanticDeltaOperation::UntapCompleted {
                affected_objects: affected_objects.clone(),
            },
            Self::ActivePlayerChanged { from, to } => SemanticDeltaOperation::ActivePlayerChanged {
                from: *from,
                to: *to,
            },
            Self::TurnNumberChanged { from, to } => SemanticDeltaOperation::TurnNumberChanged {
                from: *from,
                to: *to,
            },
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

/// Rules-owned trusted perception/authorization policy of one perspective
/// occurrence. Trusted references are authoritative `GameObjectId`s; the
/// public opaque substitution happens exclusively in observation projection.
/// This type never enters the authoritative audit because it does not mutate
/// authoritative state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PerspectiveObservationPolicyV1 {
    /// A tracked movement perceived by the perspective. Field flags decide
    /// which incarnations are authorized for opaque substitution. Revealing
    /// only the old incarnation models a tracked disappearance; revealing
    /// only the new one models an appearance of an already-tracked identity.
    MovedInSight {
        from_zone: ZoneKind,
        to_zone: ZoneKind,
        old_object: GameObjectId,
        new_object: GameObjectId,
        reveals_old: bool,
        reveals_new: bool,
    },
    /// A previously unknown incarnation becomes visible to the perspective.
    Appeared {
        from_zone: ZoneKind,
        to_zone: ZoneKind,
        new_object: GameObjectId,
    },
    /// Knowledge-only occurrence: no observed envelope is projected.
    NoEnvelope,
    /// A public object's tapped state is authorized to change for this
    /// perspective. Trusted `GameObjectId` stays here; opaque
    /// substitution happens exclusively in observation projection.
    ObjectTapped {
        object: GameObjectId,
        tapped: bool,
    },
    SawRandomOutcome {
        label: String,
        exclusive_upper_bound: u64,
        value: u64,
    },
    AnnouncedOutcome {
        code: String,
    },
}
