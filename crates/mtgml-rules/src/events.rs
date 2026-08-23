use mtgml_model::{DecisionId, GameObjectId, PlayerId, RuleEventId, StateRevision, ZoneKind};
use mtgml_random::RandomStreamKeyV1;
use mtgml_state::{
    IdentityMutationV1, KnowledgeAcquisitionCause, KnowledgeAcquisitionReason,
    KnowledgeHistoryChannel, KnowledgeMutationV1, PerspectiveLifecycleAuditV1,
    SemanticDeltaOperation, ZoneTransition,
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
    #[error("occurrence carries no state-changing mutation")]
    EmptyOccurrence,
    #[error("observation does not bind to any physical zone transition")]
    ObjectBindingMismatch,
}

pub fn validate_occurrence_pairing(
    lifecycle: &PerspectiveLifecycleAuditV1,
    observation: &PerspectiveObservationPolicyV1,
    transitions: &[ZoneTransition],
) -> Result<(), OccurrencePairingError> {
    use PerspectiveObservationPolicyV1 as Policy;
    let mutation = &lifecycle.mutation;

    // A completely empty occurrence must never consume a visible sequence.
    if matches!(mutation.identity, IdentityMutationV1::None) && mutation.knowledge.is_none() {
        return Err(OccurrencePairingError::EmptyOccurrence);
    }

    // Sight policies must describe exactly one physical zone transition of
    // this product (state/event/projection binding).
    let bind_transition = |old_object: GameObjectId,
                           new_object: GameObjectId,
                           from_zone: ZoneKind,
                           to_zone: ZoneKind|
     -> Result<&ZoneTransition, OccurrencePairingError> {
        transitions
            .iter()
            .find(|transition| {
                transition.old_object == old_object
                    && transition.new_object == new_object
                    && transition.from.zone == from_zone
                    && transition.to.zone == to_zone
            })
            .ok_or(OccurrencePairingError::ObjectBindingMismatch)
    };

    match observation {
        Policy::MovedInSight {
            from_zone,
            to_zone,
            old_object,
            new_object,
            reveals_old,
            reveals_new,
        } => {
            if !matches!(
                mutation.identity,
                IdentityMutationV1::None
                    | IdentityMutationV1::Remap { .. }
                    | IdentityMutationV1::Retire { .. }
            ) {
                return Err(OccurrencePairingError::IdentityMismatch);
            }
            if !matches!(
                mutation.knowledge,
                None | Some(KnowledgeMutationV1::UpdateLocation { .. })
                    | Some(KnowledgeMutationV1::CurrentToHistory { .. })
                    | Some(KnowledgeMutationV1::Invalidate { .. })
            ) {
                return Err(OccurrencePairingError::KnowledgeMismatch);
            }
            if !*reveals_old && !*reveals_new {
                return Err(OccurrencePairingError::NothingRevealed);
            }
            if matches!(mutation.identity, IdentityMutationV1::Retire { .. })
                && !matches!(
                    mutation.knowledge,
                    Some(KnowledgeMutationV1::Invalidate { .. })
                )
            {
                return Err(OccurrencePairingError::KnowledgeMismatch);
            }
            let transition = bind_transition(*old_object, *new_object, *from_zone, *to_zone)?;
            match &mutation.identity {
                IdentityMutationV1::Remap {
                    from_object,
                    to_object,
                    ..
                } => {
                    if *from_object != transition.old_object || *to_object != transition.new_object
                    {
                        return Err(OccurrencePairingError::IdentityMismatch);
                    }
                }
                IdentityMutationV1::Retire { object, .. } => {
                    if *object != transition.old_object {
                        return Err(OccurrencePairingError::IdentityMismatch);
                    }
                }
                IdentityMutationV1::None => {}
                IdentityMutationV1::Allocate { .. } => {
                    return Err(OccurrencePairingError::IdentityMismatch)
                }
            }
        }
        Policy::Appeared {
            from_zone,
            to_zone,
            new_object,
        } => {
            if !matches!(mutation.identity, IdentityMutationV1::Allocate { .. }) {
                return Err(OccurrencePairingError::IdentityMismatch);
            }
            match &mutation.knowledge {
                Some(KnowledgeMutationV1::Acquire { acquisition, .. }) => {
                    if !matches!(
                        acquisition,
                        KnowledgeAcquisitionReason::Observed {
                            channel: KnowledgeHistoryChannel::Public,
                            cause: KnowledgeAcquisitionCause::ExplicitReveal
                                | KnowledgeAcquisitionCause::PublicEvent,
                            ..
                        }
                    ) {
                        return Err(OccurrencePairingError::KnowledgeMismatch);
                    }
                }
                _ => return Err(OccurrencePairingError::KnowledgeMismatch),
            }
            let transition = transitions
                .iter()
                .find(|transition| {
                    transition.new_object == *new_object
                        && transition.from.zone == *from_zone
                        && transition.to.zone == *to_zone
                })
                .ok_or(OccurrencePairingError::ObjectBindingMismatch)?;
            if let IdentityMutationV1::Allocate { object, .. } = &mutation.identity {
                if *object != transition.new_object {
                    return Err(OccurrencePairingError::IdentityMismatch);
                }
            }
        }
        Policy::NoEnvelope => {
            // Envelope-less occurrences carry arbitrary authorized lifecycle
            // mutations (private looks, own-private identity, explicit
            // forget, hidden randomization/shuffle retirement); emptiness is
            // rejected above.
        }
        Policy::SawRandomOutcome {
            label,
            exclusive_upper_bound,
            value,
        } => {
            if label.is_empty() || *exclusive_upper_bound == 0 || value >= exclusive_upper_bound {
                return Err(OccurrencePairingError::KnowledgeMismatch);
            }
            if !matches!(mutation.identity, IdentityMutationV1::None)
                || mutation.knowledge.is_some()
            {
                return Err(OccurrencePairingError::IdentityMismatch);
            }
        }
        Policy::AnnouncedOutcome { code } => {
            if code.is_empty() {
                return Err(OccurrencePairingError::KnowledgeMismatch);
            }
            if !matches!(mutation.identity, IdentityMutationV1::None)
                || mutation.knowledge.is_some()
            {
                return Err(OccurrencePairingError::IdentityMismatch);
            }
        }
    }
    Ok(())
}
