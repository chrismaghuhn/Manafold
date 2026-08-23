use mtgml_model::{DecisionId, GameObjectId, PlayerId, RuleEventId, StateRevision, ZoneKind};
use mtgml_random::RandomStreamKeyV1;
use mtgml_state::{KnowledgeAcquisitionReason, PerspectiveLifecycleAuditV1, SemanticDeltaOperation, ZoneTransition};
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

/// Rules-owned trusted perception/authorization policy of one perspective
/// occurrence. Trusted references are authoritative `GameObjectId`s; the
/// public opaque substitution happens exclusively in observation projection.
/// This type never enters the authoritative audit because it does not mutate
/// authoritative state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PerspectiveObservationPolicyV1 {
    /// A tracked movement perceived by the perspective. Field flags decide
    /// which incarnations are authorized for opaque substitution.
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
    /// A known object leaves sight while staying distinguishable.
    VanishedTracked {
        from_zone: ZoneKind,
        to_zone: ZoneKind,
        old_object: GameObjectId,
    },
    /// Knowledge-only occurrence: no observed envelope is projected.
    NoEnvelope,
    SawRandomOutcome {
        label: String,
        exclusive_upper_bound: u64,
        value: u64,
    },
    AnnouncedOutcome {
        code: String,
    },
}

/// Closed pairing matrix between the state-owned lifecycle mutation and the
/// rules-owned observation policy. Malformed pairings are authoritative
/// invariant failures, never silently normalized.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum OccurrencePairingError {
    #[error("moved-in-sight occurrence must reveal at least one incarnation")]
    NothingRevealed,
    #[error("observation policy does not match the identity mutation")]
    IdentityMismatch,
    #[error("observation policy does not match the knowledge mutation")]
    KnowledgeMismatch,
}

pub fn validate_occurrence_pairing(
    lifecycle: &PerspectiveLifecycleAuditV1,
    observation: &PerspectiveObservationPolicyV1,
) -> Result<(), OccurrencePairingError> {
    use PerspectiveObservationPolicyV1 as Policy;
    let mutation = &lifecycle.mutation;
    match observation {
        Policy::MovedInSight {
            reveals_old,
            reveals_new,
            ..
        } => {
            if !matches!(
                mutation.identity,
                mtgml_state::IdentityMutationV1::None | mtgml_state::IdentityMutationV1::Remap { .. }
            ) {
                return Err(OccurrencePairingError::IdentityMismatch);
            }
            if !matches!(
                mutation.knowledge,
                None | Some(mtgml_state::KnowledgeMutationV1::UpdateLocation { .. })
                    | Some(mtgml_state::KnowledgeMutationV1::CurrentToHistory { .. })
            ) {
                return Err(OccurrencePairingError::KnowledgeMismatch);
            }
            if !*reveals_old && !*reveals_new {
                return Err(OccurrencePairingError::NothingRevealed);
            }
        }
        Policy::Appeared { .. } => {
            if !matches!(
                mutation.identity,
                mtgml_state::IdentityMutationV1::Allocate { .. }
            ) {
                return Err(OccurrencePairingError::IdentityMismatch);
            }
            match &mutation.knowledge {
                Some(mtgml_state::KnowledgeMutationV1::Acquire { acquisition, .. }) => {
                    if !matches!(
                        acquisition,
                        KnowledgeAcquisitionReason::Observed {
                            channel: mtgml_state::KnowledgeHistoryChannel::Public,
                            cause: mtgml_state::KnowledgeAcquisitionCause::ExplicitReveal
                                | mtgml_state::KnowledgeAcquisitionCause::PublicEvent,
                            ..
                        }
                    ) {
                        return Err(OccurrencePairingError::KnowledgeMismatch);
                    }
                }
                _ => return Err(OccurrencePairingError::KnowledgeMismatch),
            }
        }
        Policy::VanishedTracked { .. } => {
            if !matches!(
                mutation.identity,
                mtgml_state::IdentityMutationV1::Retire { .. }
            ) {
                return Err(OccurrencePairingError::IdentityMismatch);
            }
            if !matches!(
                mutation.knowledge,
                None | Some(mtgml_state::KnowledgeMutationV1::CurrentToHistory { .. })
            ) {
                return Err(OccurrencePairingError::KnowledgeMismatch);
            }
        }
        Policy::NoEnvelope => {
            if !matches!(mutation.identity, mtgml_state::IdentityMutationV1::None) {
                return Err(OccurrencePairingError::IdentityMismatch);
            }
            if mutation.knowledge.is_none() {
                return Err(OccurrencePairingError::KnowledgeMismatch);
            }
        }
        Policy::SawRandomOutcome { .. } | Policy::AnnouncedOutcome { .. } => {
            if !matches!(mutation.identity, mtgml_state::IdentityMutationV1::None)
                || mutation.knowledge.is_some()
            {
                return Err(OccurrencePairingError::IdentityMismatch);
            }
        }
    }
    Ok(())
}
