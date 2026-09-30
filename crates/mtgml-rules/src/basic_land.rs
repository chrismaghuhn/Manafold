//! Candidate derivation for the admitted, typed basic-land profile.
//!
//! This module consumes the same immutable admission token used to construct
//! the Magic kernel. It never dispatches on card names and it never mutates
//! state. Candidate intent ordering and request-local IDs are owned by
//! CandidateOrderingV2.

use mtgml_card_ir::{
    CardSemanticBindingV1, ExecutableProfileAdmissionV1, BASIC_LAND_PROFILE_ID_V1,
};
use mtgml_decision::{
    AuthoritativeCandidateV3, AuthoritativeDecisionRequestV3, CandidateIntentV3,
    CandidateOrderingV2, DecisionDomainV2, DecisionVisibility, EngineCandidateBindingV3,
};
use mtgml_model::{EpisodeStatus, ExecutionProgramV1, PlayerId, ZoneKind};
use mtgml_state::{
    apply_perspective_lifecycle, IdentityMutationV1, KnowledgeAcquisitionCause,
    KnowledgeAcquisitionReason, KnowledgeHistoryChannel, KnowledgeMutationV1, KnownLocationFactV2,
    PerspectiveLifecycleAuditV1, PerspectiveLifecycleMutationV1, SemanticDeltaOperationV2,
    StateDeltaV2, VisibilityPartition, ZoneLocation, ZonePosition,
};
use mtgml_state::{EngineStatePartsV2, TurnPosition};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum BasicLandCandidateError {
    #[error("the execution identity is not the admitted Magic profile")]
    WrongExecutionIdentity,
    #[error("the successor state is structurally invalid")]
    InvalidState,
    #[error("a visible live object is missing its admitted content definition")]
    InvalidDefinition,
    #[error("candidate ordering or capacity validation failed")]
    CandidateOrdering,
    #[error("identity allocator exhausted while creating the pending request")]
    IdentityExhausted,
    #[error("the current state has no legal successor decision candidates")]
    NoCandidates,
    #[error("selected candidate is not a closed MagicRules action")]
    UnsupportedSelectedAction,
    #[error("pending request does not equal the complete current candidate set")]
    PendingCandidateSetMismatch,
}

/// The only internal profile action requests admitted by this bounded
/// executable profile. This value can only be derived from a validated
/// selected V3 candidate and a matching V2 response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MagicActionRequestV1 {
    PlayLand {
        actor: PlayerId,
        object: mtgml_model::GameObjectId,
    },
    ActivateManaAbility {
        actor: PlayerId,
        ability: mtgml_model::AbilityInstanceId,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectedSuccessorDecisionV1 {
    PassPriority,
    MagicAction(MagicActionRequestV1),
}

/// Complete successor product for one bounded basic-land action. It is kept
/// in the rules crate so callers cannot manufacture a replacement state or
/// semantic delta independently.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasicLandTransitionProductV1 {
    pub accepted: bool,
    pub next_state: EngineStatePartsV2,
    pub delta: StateDeltaV2,
    pub events: Vec<AuthoritativeRuleEventV2>,
    pub next_decision: Option<AuthoritativeDecisionRequestV3>,
    pub status: EpisodeStatus,
}

/// Shared mutation draft used by the V7 historical adapter and the current
/// V8 bridge. It carries no V2 delta or digest product; each API boundary
/// emits only its own accepted contract family.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BasicLandTransitionDraftV1 {
    pub next_state: EngineStatePartsV2,
    pub operations: Vec<SemanticDeltaOperationV2>,
    pub events: Vec<AuthoritativeRuleEventV2>,
    pub next_decision_actor: PlayerId,
    pub status: EpisodeStatus,
}

/// Authoritative public facts emitted by this bounded transition slice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoritativeRuleEventV2 {
    pub event_id: mtgml_model::RuleEventId,
    pub state_revision: mtgml_model::StateRevision,
    pub event: AuthoritativeRuleEventKindV2,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthoritativeRuleEventKindV2 {
    DecisionCleared {
        decision: mtgml_model::DecisionId,
    },
    DecisionCreated {
        decision: mtgml_model::DecisionId,
    },
    PriorityChanged {
        from: mtgml_state::PriorityState,
        to: mtgml_state::PriorityState,
    },
    TurnPositionChanged {
        from: TurnPosition,
        to: TurnPosition,
    },
    ObjectMoved {
        old_object: mtgml_model::GameObjectId,
        new_object: mtgml_model::GameObjectId,
        from: ZoneKind,
        to: ZoneKind,
        entering_face: Option<BasicLandFaceV1>,
        tapped: bool,
    },
    LandPlayed {
        old_object: mtgml_model::GameObjectId,
        new_object: mtgml_model::GameObjectId,
        actor: PlayerId,
        land_plays_used: u8,
    },
    ObjectTapped {
        object: mtgml_model::GameObjectId,
        from: bool,
        to: bool,
    },
    ManaPoolChanged {
        player: PlayerId,
        previous: mtgml_state::ManaPoolV1,
        pool_after: mtgml_state::ManaPoolV1,
        color: Option<mtgml_state::ManaColorV1>,
        amount: u32,
    },
    PerspectiveOccurrence {
        lifecycle: Box<PerspectiveLifecycleAuditV1>,
        observation: SuccessorObservationPolicyV1,
    },
    ZoneTransition(Box<mtgml_state::ZoneTransition>),
}

/// Rules-owned audience and public facts for one successor observation
/// occurrence. Object identifiers remain trusted here and are replaced by the
/// perspective-specific opaque mapping in the environment projector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuccessorObservationPolicyV1 {
    MovedInSight {
        from: ZoneKind,
        to: ZoneKind,
        old_object: mtgml_model::GameObjectId,
        new_object: mtgml_model::GameObjectId,
        reveals_old: bool,
        entering_face: BasicLandFaceV1,
        tapped: bool,
    },
    ObjectTapped {
        object: mtgml_model::GameObjectId,
        tapped: bool,
    },
    ManaPoolChanged {
        player: PlayerId,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BasicLandFaceV1 {
    Front,
    Back,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum BasicLandTransitionError {
    #[error("the selected action is not valid in the current successor state")]
    InvalidSelection,
    #[error("the successor state cannot allocate another identity")]
    IdentityExhausted,
    #[error("the selected land is not an admitted live basic land")]
    InvalidLand,
    #[error("the selected ability is not the admitted intrinsic mana ability")]
    InvalidAbility,
    #[error("the transition would violate a state invariant")]
    InvalidResult,
    #[error("the transition delta could not be constructed")]
    Delta,
    #[error("the next turn boundary requires semantics outside this admitted profile")]
    UnsupportedPriorityBoundary,
    #[error("the turn rules cannot progress from this state")]
    TurnProgressUnsupported,
}

/// Resolve the second-pass boundary only when this profile can immediately
/// expose a valid ordinary priority window. Other boundaries require a
/// turn-based action or another rules-owned forced-progress producer (draw,
/// combat declaration, cleanup, and similar work). They fail closed instead
/// of manufacturing a priority decision at the temporal successor.
fn priority_window_after_second_pass(
    position: TurnPosition,
) -> Result<TurnPosition, BasicLandTransitionError> {
    match position {
        TurnPosition::PrecombatMain => Ok(TurnPosition::Combat {
            step: mtgml_state::CombatStep::BeginningOfCombat,
        }),
        _ => Err(BasicLandTransitionError::UnsupportedPriorityBoundary),
    }
}

fn push_successor_event(
    state: &mut EngineStatePartsV2,
    events: &mut Vec<AuthoritativeRuleEventV2>,
    event: AuthoritativeRuleEventKindV2,
) -> Result<(), BasicLandTransitionError> {
    let event_id = state.predecessor_v5.allocators.next_rule_event_id;
    state.predecessor_v5.allocators.next_rule_event_id = mtgml_model::RuleEventId(
        event_id
            .0
            .checked_add(1)
            .ok_or(BasicLandTransitionError::IdentityExhausted)?,
    );
    events.push(AuthoritativeRuleEventV2 {
        event_id,
        state_revision: state.predecessor_v5.revision,
        event,
    });
    Ok(())
}

fn push_visible_occurrence(
    state: &mut EngineStatePartsV2,
    events: &mut Vec<AuthoritativeRuleEventV2>,
    operations: &mut Vec<SemanticDeltaOperationV2>,
    perspective: PlayerId,
    observation: SuccessorObservationPolicyV1,
) -> Result<(), BasicLandTransitionError> {
    let current = state
        .predecessor_v5
        .knowledge
        .players
        .get(&perspective)
        .ok_or(BasicLandTransitionError::InvalidResult)?;
    let audit = PerspectiveLifecycleAuditV1 {
        perspective,
        sequence: current.next_visible_sequence,
        mutation: PerspectiveLifecycleMutationV1::default(),
    };
    let mut engine = state.materialize();
    apply_perspective_lifecycle(&mut engine, &audit)
        .map_err(|_| BasicLandTransitionError::InvalidResult)?;
    operations.push(SemanticDeltaOperationV2::Existing {
        operation: Box::new(mtgml_state::SemanticDeltaOperation::PerspectiveLifecycle {
            lifecycle: audit.clone(),
        }),
    });
    let card_rules_state = state.card_rules_state.clone();
    let execution_v3 = state.execution_v3.clone();
    let next_event_id = state.predecessor_v5.allocators.next_rule_event_id;
    *state = EngineStatePartsV2::from_state(&engine, card_rules_state);
    state.execution_v3 = execution_v3;
    state.predecessor_v5.allocators.next_rule_event_id = next_event_id;
    push_successor_event(
        state,
        events,
        AuthoritativeRuleEventKindV2::PerspectiveOccurrence {
            lifecycle: Box::new(audit),
            observation,
        },
    )
}

/// Executes exactly the response selected by the digest-bound V3 request,
/// then constructs one validated successor state/delta product. The input is
/// never changed; every mutation is staged on a clone and published only
/// after validation and digest construction succeed.
pub fn execute_basic_land_response(
    admission: &ExecutableProfileAdmissionV1,
    state: &EngineStatePartsV2,
    actor: PlayerId,
    response: &mtgml_decision::DecisionResponseV2,
    status: &EpisodeStatus,
) -> Result<BasicLandTransitionProductV1, BasicLandTransitionError> {
    let decision = selected_successor_decision(admission, state, actor, response, status)
        .map_err(|_| BasicLandTransitionError::InvalidSelection)?;
    let pending_decision = state
        .execution_v3
        .pending_decision
        .as_ref()
        .ok_or(BasicLandTransitionError::InvalidSelection)?
        .decision_id;
    let mut draft = execute_basic_land_decision_draft(
        admission,
        state,
        actor,
        pending_decision,
        decision,
        status,
    )?;
    let next_decision = install_basic_land_request(
        admission,
        &mut draft.next_state,
        draft.next_decision_actor,
        status,
    )
    .map_err(|_| BasicLandTransitionError::InvalidResult)?;
    draft.operations.push(SemanticDeltaOperationV2::Existing {
        operation: Box::new(mtgml_state::SemanticDeltaOperation::DecisionCreated {
            decision: next_decision.decision_id,
        }),
    });
    push_successor_event(
        &mut draft.next_state,
        &mut draft.events,
        AuthoritativeRuleEventKindV2::DecisionCreated {
            decision: next_decision.decision_id,
        },
    )?;
    draft
        .next_state
        .validate()
        .map_err(|_| BasicLandTransitionError::InvalidResult)?;
    let delta = StateDeltaV2::between(state, &draft.next_state, draft.operations)
        .map_err(|_| BasicLandTransitionError::Delta)?;
    Ok(BasicLandTransitionProductV1 {
        accepted: true,
        next_state: draft.next_state,
        delta,
        events: draft.events,
        next_decision: Some(next_decision),
        status: draft.status,
    })
}

pub(crate) fn execute_basic_land_decision_draft(
    admission: &ExecutableProfileAdmissionV1,
    state: &EngineStatePartsV2,
    actor: PlayerId,
    pending_decision_id: mtgml_model::DecisionId,
    decision: SelectedSuccessorDecisionV1,
    status: &EpisodeStatus,
) -> Result<BasicLandTransitionDraftV1, BasicLandTransitionError> {
    state
        .validate()
        .map_err(|_| BasicLandTransitionError::InvalidResult)?;
    let old_revision = state.predecessor_v5.revision;
    let next_revision = mtgml_model::StateRevision(
        old_revision
            .0
            .checked_add(1)
            .ok_or(BasicLandTransitionError::IdentityExhausted)?,
    );
    let mut next = state.clone();
    next.predecessor_v5.revision = next_revision;
    next.execution_v3.pending_decision = None;
    let mut next_decision_actor = actor;
    let mut operations = vec![SemanticDeltaOperationV2::Existing {
        operation: Box::new(mtgml_state::SemanticDeltaOperation::DecisionCleared {
            decision: pending_decision_id,
        }),
    }];
    let mut events = Vec::new();
    push_successor_event(
        &mut next,
        &mut events,
        AuthoritativeRuleEventKindV2::DecisionCleared {
            decision: pending_decision_id,
        },
    )?;

    match decision {
        SelectedSuccessorDecisionV1::PassPriority => {
            let core = &mut next.predecessor_v5.core;
            let other = core
                .players
                .keys()
                .copied()
                .find(|player| *player != actor)
                .ok_or(BasicLandTransitionError::InvalidSelection)?;
            let from_priority = core.priority;
            match from_priority {
                mtgml_state::PriorityState::HeldBy {
                    player,
                    consecutive_passes: 0,
                } if player == actor => {
                    let to_priority = mtgml_state::PriorityState::HeldBy {
                        player: other,
                        consecutive_passes: 1,
                    };
                    core.priority = to_priority;
                    operations.push(SemanticDeltaOperationV2::Existing {
                        operation: Box::new(mtgml_state::SemanticDeltaOperation::PriorityChanged {
                            from: from_priority,
                            to: to_priority,
                        }),
                    });
                    push_successor_event(
                        &mut next,
                        &mut events,
                        AuthoritativeRuleEventKindV2::PriorityChanged {
                            from: from_priority,
                            to: to_priority,
                        },
                    )?;
                    next_decision_actor = other;
                }
                mtgml_state::PriorityState::HeldBy {
                    player,
                    consecutive_passes: 1,
                } if player == actor => {
                    let next_actor = core.active_player;
                    let old_position = core.position;
                    let new_position = priority_window_after_second_pass(old_position)?;
                    let to_priority = mtgml_state::PriorityState::HeldBy {
                        player: next_actor,
                        consecutive_passes: 0,
                    };
                    core.priority = to_priority;
                    core.position = new_position;
                    operations.push(SemanticDeltaOperationV2::Existing {
                        operation: Box::new(mtgml_state::SemanticDeltaOperation::PriorityChanged {
                            from: from_priority,
                            to: to_priority,
                        }),
                    });
                    operations.push(SemanticDeltaOperationV2::Existing {
                        operation: Box::new(
                            mtgml_state::SemanticDeltaOperation::TurnPositionChanged {
                                from: old_position,
                                to: new_position,
                            },
                        ),
                    });
                    push_successor_event(
                        &mut next,
                        &mut events,
                        AuthoritativeRuleEventKindV2::PriorityChanged {
                            from: from_priority,
                            to: to_priority,
                        },
                    )?;
                    push_successor_event(
                        &mut next,
                        &mut events,
                        AuthoritativeRuleEventKindV2::TurnPositionChanged {
                            from: old_position,
                            to: new_position,
                        },
                    )?;
                    for player in next
                        .card_rules_state
                        .mana
                        .pools
                        .keys()
                        .copied()
                        .collect::<Vec<_>>()
                    {
                        let previous_pool = next
                            .card_rules_state
                            .mana
                            .empty_pool(player)
                            .map_err(|_| BasicLandTransitionError::InvalidResult)?;
                        if previous_pool != mtgml_state::ManaPoolV1::default() {
                            operations.push(SemanticDeltaOperationV2::ManaPoolEmptied {
                                player,
                                previous_pool,
                            });
                            push_successor_event(
                                &mut next,
                                &mut events,
                                AuthoritativeRuleEventKindV2::ManaPoolChanged {
                                    player,
                                    previous: previous_pool,
                                    pool_after: mtgml_state::ManaPoolV1::default(),
                                    color: None,
                                    amount: 0,
                                },
                            )?;
                            for perspective in next
                                .predecessor_v5
                                .knowledge
                                .players
                                .keys()
                                .copied()
                                .collect::<Vec<_>>()
                            {
                                push_visible_occurrence(
                                    &mut next,
                                    &mut events,
                                    &mut operations,
                                    perspective,
                                    SuccessorObservationPolicyV1::ManaPoolChanged { player },
                                )?;
                            }
                        }
                    }
                    next_decision_actor = next_actor;
                }
                _ => return Err(BasicLandTransitionError::InvalidSelection),
            }
            // A second consecutive pass closes this priority window and
            // exposes the next rules-owned window in the same response
            // transaction. The active player receives priority at the next
            // temporal position; callers never observe Running without a
            // pending decision.
            next.validate()
                .map_err(|_| BasicLandTransitionError::InvalidResult)?;
            return Ok(BasicLandTransitionDraftV1 {
                next_state: next,
                operations,
                events,
                next_decision_actor,
                status: status.clone(),
            });
        }
        SelectedSuccessorDecisionV1::MagicAction(MagicActionRequestV1::PlayLand {
            actor,
            object,
        }) => {
            let old = next
                .predecessor_v5
                .zones
                .objects
                .get(&object)
                .cloned()
                .ok_or(BasicLandTransitionError::InvalidLand)?;
            let from = next
                .predecessor_v5
                .zones
                .locations
                .get(&object)
                .cloned()
                .ok_or(BasicLandTransitionError::InvalidLand)?;
            if old.owner != actor || from.zone != ZoneKind::Hand || from.player != Some(actor) {
                return Err(BasicLandTransitionError::InvalidLand);
            }
            let definition = admission
                .verified_catalog()
                .get(admission.content_contract_id(), old.card_definition)
                .map_err(|_| BasicLandTransitionError::InvalidLand)?;
            let CardSemanticBindingV1::ProfiledV1 { profile_id, .. } = &definition.semantic_binding
            else {
                return Err(BasicLandTransitionError::InvalidLand);
            };
            if profile_id.as_str() != BASIC_LAND_PROFILE_ID_V1
                || next.card_rules_state.faces.faces.get(&object) != Some(&0)
            {
                return Err(BasicLandTransitionError::InvalidLand);
            }
            let new_object = next
                .predecessor_v5
                .allocators
                .allocate_object_id()
                .map_err(|_| BasicLandTransitionError::IdentityExhausted)?;
            if next.predecessor_v5.zones.objects.contains_key(&new_object) {
                return Err(BasicLandTransitionError::InvalidResult);
            }
            let to = ZoneLocation {
                zone: ZoneKind::Battlefield,
                player: None,
                position: ZonePosition::Unordered,
                visibility: VisibilityPartition::Public,
                partition: None,
            };
            let old_snapshot = mtgml_state::ObjectSnapshot {
                object,
                physical_card: old.physical_card,
                card_definition: old.card_definition,
                owner: old.owner,
                controller: old.controller,
                tapped: old.tapped,
                face_down: old.face_down,
                location: from.clone(),
            };
            next.predecessor_v5.zones.objects.remove(&object);
            next.predecessor_v5.zones.locations.remove(&object);
            if matches!(from.position, ZonePosition::Top { .. }) {
                let key = from.key();
                let ordered = next
                    .predecessor_v5
                    .zones
                    .ordered_zones
                    .get_mut(&key)
                    .ok_or(BasicLandTransitionError::InvalidLand)?;
                let index = ordered
                    .iter()
                    .position(|member| *member == object)
                    .ok_or(BasicLandTransitionError::InvalidLand)?;
                ordered.remove(index);
                if ordered.is_empty() {
                    next.predecessor_v5.zones.ordered_zones.remove(&key);
                } else {
                    for (offset, member) in ordered.iter().enumerate() {
                        let location = next
                            .predecessor_v5
                            .zones
                            .locations
                            .get_mut(member)
                            .ok_or(BasicLandTransitionError::InvalidLand)?;
                        location.position = ZonePosition::Top {
                            offset: u32::try_from(offset)
                                .map_err(|_| BasicLandTransitionError::InvalidResult)?,
                        };
                    }
                }
            } else if from.position != ZonePosition::Unordered {
                return Err(BasicLandTransitionError::InvalidLand);
            }
            let new = mtgml_state::GameObject {
                id: new_object,
                physical_card: old.physical_card,
                card_definition: old.card_definition,
                owner: old.owner,
                controller: actor,
                tapped: false,
                face_down: false,
            };
            next.predecessor_v5
                .zones
                .objects
                .insert(new_object, new.clone());
            next.predecessor_v5
                .zones
                .locations
                .insert(new_object, to.clone());
            let new_snapshot = mtgml_state::ObjectSnapshot {
                object: new_object,
                physical_card: new.physical_card,
                card_definition: new.card_definition,
                owner: new.owner,
                controller: new.controller,
                tapped: false,
                face_down: false,
                location: to.clone(),
            };
            let live: std::collections::BTreeSet<_> =
                next.predecessor_v5.zones.objects.keys().copied().collect();
            next.card_rules_state.counters.prune_departed_objects(&live);
            next.card_rules_state
                .attachments
                .prune_departed_objects(&live);
            next.card_rules_state.faces.prune_departed_objects(&live);
            next.card_rules_state
                .abilities
                .prune_departed_objects(&live);
            // Opaque ability identities for the departed incarnation are
            // permanently retired before the new incarnation gets its own
            // allocator-assigned authority and perspective IDs.
            let departed_abilities: std::collections::BTreeSet<_> = state
                .card_rules_state
                .abilities
                .by_instance
                .iter()
                .filter_map(|(instance, authority)| {
                    (authority.source == object).then_some(*instance)
                })
                .collect();
            for (instance, authority) in &state.card_rules_state.abilities.by_instance {
                if authority.source == object {
                    operations.push(SemanticDeltaOperationV2::AbilityAuthorityRemoved {
                        instance: *instance,
                        source: authority.source,
                        ability_key: authority.ability_key,
                    });
                }
            }
            next.card_rules_state
                .turn_history
                .target_occurrences
                .retain(|(target, _)| live.contains(target));
            next.card_rules_state
                .turn_history
                .once_ability_used
                .retain(|(source, _)| live.contains(source));

            let mut materialized = next.materialize();
            for perspective in materialized
                .core
                .players
                .keys()
                .copied()
                .collect::<Vec<_>>()
            {
                let identity = materialized
                    .perspective_identities
                    .players
                    .get(&perspective)
                    .ok_or(BasicLandTransitionError::InvalidResult)?;
                let old_opaque = identity.object_to_opaque.get(&object).copied();
                let knowledge = materialized
                    .knowledge
                    .players
                    .get(&perspective)
                    .ok_or(BasicLandTransitionError::InvalidResult)?;
                let sequence = knowledge.next_visible_sequence;
                let provenance = KnowledgeAcquisitionReason::Observed {
                    channel: KnowledgeHistoryChannel::Public,
                    sequence,
                    cause: KnowledgeAcquisitionCause::PublicEvent,
                };
                let (identity_mutation, knowledge_mutation) = if let Some(opaque) = old_opaque {
                    let knowledge_mutation = if knowledge.active.contains_key(&opaque) {
                        KnowledgeMutationV1::UpdateLocation {
                            opaque,
                            fact: KnownLocationFactV2 {
                                location: to.clone(),
                                provenance,
                            },
                        }
                    } else {
                        KnowledgeMutationV1::Acquire {
                            opaque,
                            definition: Some(old.card_definition),
                            location: Some(to.clone()),
                            acquisition: provenance,
                        }
                    };
                    (
                        IdentityMutationV1::Remap {
                            opaque,
                            from_object: object,
                            to_object: new_object,
                        },
                        Some(knowledge_mutation),
                    )
                } else {
                    let opaque = identity.next_opaque_object_id;
                    (
                        IdentityMutationV1::Allocate {
                            opaque,
                            object: new_object,
                        },
                        Some(KnowledgeMutationV1::Acquire {
                            opaque,
                            definition: Some(old.card_definition),
                            location: Some(to.clone()),
                            acquisition: provenance,
                        }),
                    )
                };
                let audit = PerspectiveLifecycleAuditV1 {
                    perspective,
                    sequence,
                    mutation: PerspectiveLifecycleMutationV1 {
                        identity: identity_mutation,
                        knowledge: knowledge_mutation,
                    },
                };
                apply_perspective_lifecycle(&mut materialized, &audit)
                    .map_err(|_| BasicLandTransitionError::InvalidResult)?;
                operations.push(SemanticDeltaOperationV2::Existing {
                    operation: Box::new(
                        mtgml_state::SemanticDeltaOperation::PerspectiveLifecycle {
                            lifecycle: audit.clone(),
                        },
                    ),
                });
                push_successor_event(
                    &mut next,
                    &mut events,
                    AuthoritativeRuleEventKindV2::PerspectiveOccurrence {
                        lifecycle: Box::new(audit),
                        observation: SuccessorObservationPolicyV1::MovedInSight {
                            from: from.zone,
                            to: to.zone,
                            old_object: object,
                            new_object,
                            reveals_old: old_opaque.is_some(),
                            entering_face: BasicLandFaceV1::Front,
                            tapped: false,
                        },
                    },
                )?;
            }
            let next_event_id = next.predecessor_v5.allocators.next_rule_event_id;
            let successor_execution = next.execution_v3.clone();
            next = EngineStatePartsV2::from_state(&materialized, next.card_rules_state);
            next.execution_v3 = successor_execution;
            next.predecessor_v5.allocators.next_rule_event_id = next_event_id;
            for (perspective, identity) in next
                .predecessor_v5
                .perspective_identities
                .players
                .iter_mut()
            {
                for instance in &departed_abilities {
                    if let Some(opaque) = identity.ability_to_opaque.remove(instance) {
                        identity.opaque_to_ability.remove(&opaque);
                        identity.retired_ability_ids.insert(opaque);
                        operations.push(SemanticDeltaOperationV2::AbilityIdentityChanged {
                            perspective: *perspective,
                            instance: *instance,
                            from: Some(opaque),
                            to: None,
                        });
                    }
                }
            }
            next.card_rules_state
                .faces
                .set(new_object, 0, &live)
                .map_err(|_| BasicLandTransitionError::InvalidResult)?;
            let mut next_ability_id = next.predecessor_v5.allocators.next_ability_id;
            let abilities = next
                .card_rules_state
                .abilities
                .allocate_for_source(&mut next_ability_id, new_object, [0], &live)
                .map_err(|_| BasicLandTransitionError::IdentityExhausted)?;
            next.predecessor_v5.allocators.next_ability_id = next_ability_id;
            let ability = *abilities
                .first()
                .ok_or(BasicLandTransitionError::InvalidResult)?;
            operations.push(SemanticDeltaOperationV2::AbilityAuthorityAdded {
                instance: ability,
                source: new_object,
                ability_key: 0,
            });
            for (perspective, identity) in next
                .predecessor_v5
                .perspective_identities
                .players
                .iter_mut()
            {
                let opaque = identity.next_opaque_ability_id;
                identity.next_opaque_ability_id = mtgml_model::OpaqueAbilityId(
                    opaque
                        .0
                        .checked_add(1)
                        .ok_or(BasicLandTransitionError::IdentityExhausted)?,
                );
                if identity.opaque_to_ability.insert(opaque, ability).is_some()
                    || identity.ability_to_opaque.insert(ability, opaque).is_some()
                {
                    return Err(BasicLandTransitionError::InvalidResult);
                }
                operations.push(SemanticDeltaOperationV2::AbilityIdentityChanged {
                    perspective: *perspective,
                    instance: ability,
                    from: None,
                    to: Some(opaque),
                });
            }
            next.card_rules_state
                .turn_history
                .record_land_play(actor)
                .map_err(|_| BasicLandTransitionError::InvalidLand)?;
            let transition = mtgml_state::ZoneTransition {
                old_object: object,
                new_object,
                physical_card: old.physical_card,
                from: from.clone(),
                to: to.clone(),
                last_known: old_snapshot,
                new_snapshot,
            };
            operations.push(SemanticDeltaOperationV2::ObjectEntered {
                old_object: Some(object),
                new_object,
                from_zone: from.zone,
                to_zone: to.zone,
                tapped: false,
                face: 0,
            });
            operations.push(SemanticDeltaOperationV2::LandPlayCountChanged {
                player: actor,
                from: 0,
                to: 1,
            });
            push_successor_event(
                &mut next,
                &mut events,
                AuthoritativeRuleEventKindV2::ZoneTransition(Box::new(transition)),
            )?;
            push_successor_event(
                &mut next,
                &mut events,
                AuthoritativeRuleEventKindV2::LandPlayed {
                    old_object: object,
                    new_object,
                    actor,
                    land_plays_used: 1,
                },
            )?;
            push_successor_event(
                &mut next,
                &mut events,
                AuthoritativeRuleEventKindV2::ObjectMoved {
                    old_object: object,
                    new_object,
                    from: from.zone,
                    to: to.zone,
                    entering_face: Some(BasicLandFaceV1::Front),
                    tapped: false,
                },
            )?;
        }
        SelectedSuccessorDecisionV1::MagicAction(MagicActionRequestV1::ActivateManaAbility {
            actor,
            ability,
        }) => {
            let authority = next
                .card_rules_state
                .abilities
                .by_instance
                .get(&ability)
                .copied()
                .ok_or(BasicLandTransitionError::InvalidAbility)?;
            let object = next
                .predecessor_v5
                .zones
                .objects
                .get_mut(&authority.source)
                .ok_or(BasicLandTransitionError::InvalidAbility)?;
            let location = next
                .predecessor_v5
                .zones
                .locations
                .get(&authority.source)
                .ok_or(BasicLandTransitionError::InvalidAbility)?;
            if authority.ability_key != 0
                || object.controller != actor
                || object.tapped
                || location.zone != ZoneKind::Battlefield
                || next.card_rules_state.faces.faces.get(&authority.source) != Some(&0)
            {
                return Err(BasicLandTransitionError::InvalidAbility);
            }
            let definition = admission
                .verified_catalog()
                .get(admission.content_contract_id(), object.card_definition)
                .map_err(|_| BasicLandTransitionError::InvalidAbility)?;
            let current_face = next
                .card_rules_state
                .faces
                .faces
                .get(&authority.source)
                .copied()
                .ok_or(BasicLandTransitionError::InvalidAbility)?;
            let CardSemanticBindingV1::ProfiledV1 { profile_id, body } =
                &definition.semantic_binding
            else {
                return Err(BasicLandTransitionError::InvalidAbility);
            };
            if profile_id.as_str() != BASIC_LAND_PROFILE_ID_V1
                || !definition.ability_identities.iter().any(|identity| {
                    identity.face_key.0 == current_face
                        && identity.ability_key.0 == authority.ability_key
                })
            {
                return Err(BasicLandTransitionError::InvalidAbility);
            }
            let color = match body.subtype {
                mtgml_card_ir::BasicLandSubtypeV1::Mountain => mtgml_state::ManaColorV1::Red,
                mtgml_card_ir::BasicLandSubtypeV1::Plains => mtgml_state::ManaColorV1::White,
            };
            let from_tapped = object.tapped;
            object.tapped = true;
            let pool_before = *next
                .card_rules_state
                .mana
                .pools
                .get(&actor)
                .ok_or(BasicLandTransitionError::InvalidResult)?;
            next.card_rules_state
                .mana
                .add(
                    actor,
                    color,
                    mtgml_state::ManaRestrictionV1::Unrestricted,
                    1,
                )
                .map_err(|_| BasicLandTransitionError::InvalidResult)?;
            let pool_after = *next
                .card_rules_state
                .mana
                .pools
                .get(&actor)
                .ok_or(BasicLandTransitionError::InvalidResult)?;
            operations.push(SemanticDeltaOperationV2::ObjectTapped {
                object: authority.source,
                from: from_tapped,
                to: true,
            });
            push_successor_event(
                &mut next,
                &mut events,
                AuthoritativeRuleEventKindV2::ObjectTapped {
                    object: authority.source,
                    from: from_tapped,
                    to: true,
                },
            )?;
            for perspective in next
                .predecessor_v5
                .knowledge
                .players
                .keys()
                .copied()
                .collect::<Vec<_>>()
            {
                push_visible_occurrence(
                    &mut next,
                    &mut events,
                    &mut operations,
                    perspective,
                    SuccessorObservationPolicyV1::ObjectTapped {
                        object: authority.source,
                        tapped: true,
                    },
                )?;
            }
            operations.push(SemanticDeltaOperationV2::ManaAdded {
                player: actor,
                color,
                restriction: mtgml_state::ManaRestrictionV1::Unrestricted,
                amount: 1,
                source: authority.source,
                ability_key: authority.ability_key,
            });
            push_successor_event(
                &mut next,
                &mut events,
                AuthoritativeRuleEventKindV2::ManaPoolChanged {
                    player: actor,
                    previous: pool_before,
                    pool_after,
                    color: Some(color),
                    amount: 1,
                },
            )?;
            for perspective in next
                .predecessor_v5
                .knowledge
                .players
                .keys()
                .copied()
                .collect::<Vec<_>>()
            {
                push_visible_occurrence(
                    &mut next,
                    &mut events,
                    &mut operations,
                    perspective,
                    SuccessorObservationPolicyV1::ManaPoolChanged { player: actor },
                )?;
            }
        }
    }

    // CR 117.3c, 117.4: the acting player receives priority again, and the
    // action ends any succession of passes.
    let from_priority = next.predecessor_v5.core.priority;
    if let mtgml_state::PriorityState::HeldBy {
        player,
        consecutive_passes,
    } = from_priority
    {
        if consecutive_passes != 0 {
            let to_priority = mtgml_state::PriorityState::HeldBy {
                player,
                consecutive_passes: 0,
            };
            next.predecessor_v5.core.priority = to_priority;
            operations.push(SemanticDeltaOperationV2::Existing {
                operation: Box::new(mtgml_state::SemanticDeltaOperation::PriorityChanged {
                    from: from_priority,
                    to: to_priority,
                }),
            });
            push_successor_event(
                &mut next,
                &mut events,
                AuthoritativeRuleEventKindV2::PriorityChanged {
                    from: from_priority,
                    to: to_priority,
                },
            )?;
        }
    }

    next.validate()
        .map_err(|_| BasicLandTransitionError::InvalidResult)?;
    Ok(BasicLandTransitionDraftV1 {
        next_state: next,
        operations,
        events,
        next_decision_actor,
        status: status.clone(),
    })
}

/// Derive the complete bounded PlayLand and intrinsic basic-land mana
/// ability candidate surface for `actor`. This performs no state mutation,
/// response selection, or execution. Hidden or unmapped identities fail
/// closed rather than receiving placeholder public IDs.
pub fn derive_basic_land_candidates(
    admission: &ExecutableProfileAdmissionV1,
    state: &EngineStatePartsV2,
    actor: PlayerId,
    status: &EpisodeStatus,
) -> Result<Vec<AuthoritativeCandidateV3>, BasicLandCandidateError> {
    state
        .validate()
        .map_err(|_| BasicLandCandidateError::InvalidState)?;
    state
        .card_rules_state
        .counters
        .validate_decision_boundary()
        .map_err(|_| BasicLandCandidateError::InvalidState)?;
    if admission.execution_identity().program_kind != ExecutionProgramV1::MagicRules
        || admission.content_contract_id() != admission.verified_catalog().content_contract_id()
    {
        return Err(BasicLandCandidateError::WrongExecutionIdentity);
    }
    if !matches!(status, EpisodeStatus::Running) || state.execution_v3.pending_decision.is_some() {
        return Err(BasicLandCandidateError::InvalidState);
    }
    let core = &state.predecessor_v5.core;
    if !core.players.contains_key(&actor) {
        return Err(BasicLandCandidateError::InvalidState);
    }
    let has_priority = matches!(core.priority, mtgml_state::PriorityState::HeldBy { player, .. } if player == actor);
    let main_phase = matches!(
        core.position,
        TurnPosition::PrecombatMain | TurnPosition::PostcombatMain
    );
    let active = core.active_player == actor;
    let stack_empty = state.predecessor_v5.zones.stack_order.is_empty();
    let player_history = state
        .card_rules_state
        .turn_history
        .players
        .get(&actor)
        .ok_or(BasicLandCandidateError::InvalidState)?;
    let land_allowed = admission
        .resolved_capabilities()
        .iter()
        .any(|requirement| requirement.key == "rules/land-play");
    let mana_allowed = admission
        .resolved_capabilities()
        .iter()
        .any(|requirement| requirement.key == "rules/basic-land-mana");
    let priority_allowed = admission
        .resolved_capabilities()
        .iter()
        .any(|requirement| requirement.key == "rules/basic-priority");
    let identity = state
        .predecessor_v5
        .perspective_identities
        .players
        .get(&actor)
        .ok_or(BasicLandCandidateError::InvalidState)?;

    let mut raw = Vec::new();
    if has_priority && priority_allowed {
        raw.push((
            CandidateIntentV3::PassPriority,
            EngineCandidateBindingV3::PassPriority,
        ));
    }
    for (object_id, object) in &state.predecessor_v5.zones.objects {
        let location = state
            .predecessor_v5
            .zones
            .locations
            .get(object_id)
            .ok_or(BasicLandCandidateError::InvalidState)?;
        let definition = admission
            .verified_catalog()
            .get(admission.content_contract_id(), object.card_definition)
            .map_err(|_| BasicLandCandidateError::InvalidDefinition)?;
        let face_key = *state
            .card_rules_state
            .faces
            .faces
            .get(object_id)
            .ok_or(BasicLandCandidateError::InvalidDefinition)?;
        if !definition
            .faces
            .iter()
            .any(|face| face.face_key.0 == face_key)
            || state
                .card_rules_state
                .abilities
                .by_instance
                .values()
                .filter(|authority| authority.source == *object_id)
                .any(|authority| {
                    !definition.ability_identities.iter().any(|identity| {
                        identity.face_key.0 == face_key
                            && identity.ability_key.0 == authority.ability_key
                    })
                })
        {
            return Err(BasicLandCandidateError::InvalidDefinition);
        }
        let CardSemanticBindingV1::ProfiledV1 { profile_id, body } = &definition.semantic_binding
        else {
            continue;
        };
        if profile_id.as_str() != BASIC_LAND_PROFILE_ID_V1 {
            continue;
        }
        let public_object = identity.object_to_opaque.get(object_id).copied();
        if location.zone == ZoneKind::Hand
            && location.player == Some(actor)
            && active
            && has_priority
            && main_phase
            && stack_empty
            && player_history.land_plays_used == 0
            && land_allowed
            && face_key == 0
        {
            let opaque = public_object.ok_or(BasicLandCandidateError::InvalidState)?;
            raw.push((
                CandidateIntentV3::PlayLand { object: opaque },
                EngineCandidateBindingV3::PlayLand { object: *object_id },
            ));
        }
        if location.zone == ZoneKind::Battlefield
            && object.controller == actor
            && !object.tapped
            && has_priority
            && mana_allowed
            && face_key == 0
        {
            public_object.ok_or(BasicLandCandidateError::InvalidState)?;
            for (ability_id, authority) in &state.card_rules_state.abilities.by_instance {
                if authority.source != *object_id || authority.ability_key != 0 {
                    continue;
                }
                if !definition.ability_identities.iter().any(|ability| {
                    ability.face_key.0 == face_key && ability.ability_key.0 == authority.ability_key
                }) {
                    continue;
                }
                let opaque_ability = identity
                    .ability_to_opaque
                    .get(ability_id)
                    .copied()
                    .ok_or(BasicLandCandidateError::InvalidState)?;
                // The typed profile identity, not the card name, selects the
                // corresponding public opaque ability binding. The current
                // request vocabulary exposes the ability identity itself;
                // subtype determines the mana produced during resolution.
                let _subtype = body.subtype;
                raw.push((
                    CandidateIntentV3::ActivateAbility {
                        ability: opaque_ability,
                    },
                    EngineCandidateBindingV3::ActivateAbility {
                        ability: *ability_id,
                    },
                ));
            }
        }
    }
    let candidates = CandidateOrderingV2::assign_dense(raw)
        .map_err(|_| BasicLandCandidateError::CandidateOrdering)?;
    let live: std::collections::BTreeSet<_> =
        state.predecessor_v5.zones.objects.keys().copied().collect();
    if candidates
        .iter()
        .any(|candidate| match &candidate.trusted_binding {
            EngineCandidateBindingV3::PassPriority => false,
            EngineCandidateBindingV3::PlayLand { object } => !live.contains(object),
            EngineCandidateBindingV3::ActivateAbility { ability } => !state
                .card_rules_state
                .abilities
                .by_instance
                .contains_key(ability),
            _ => true,
        })
    {
        return Err(BasicLandCandidateError::InvalidState);
    }
    Ok(candidates)
}

/// Validate the stored authoritative request against the complete rules-owned
/// candidate surface without replacing or reconstructing that request.
/// Executable admission calls this before exposing a restored decision.
pub fn validate_basic_land_pending_request(
    admission: &ExecutableProfileAdmissionV1,
    state: &EngineStatePartsV2,
    status: &EpisodeStatus,
) -> Result<(), BasicLandCandidateError> {
    state
        .validate()
        .map_err(|_| BasicLandCandidateError::InvalidState)?;
    match status {
        EpisodeStatus::Running => {
            let request = state
                .execution_v3
                .pending_decision
                .as_ref()
                .ok_or(BasicLandCandidateError::PendingCandidateSetMismatch)?;
            request
                .validate()
                .map_err(|_| BasicLandCandidateError::PendingCandidateSetMismatch)?;
            let mut candidate_state = state.clone();
            candidate_state.execution_v3.pending_decision = None;
            let expected =
                derive_basic_land_candidates(admission, &candidate_state, request.actor, status)?;
            if request.state_revision != state.predecessor_v5.revision
                || request.visibility != DecisionVisibility::Public
                || request.decision != DecisionDomainV2::ChooseOne
                || request.continuation_id.is_some()
                || request.candidates != expected
            {
                return Err(BasicLandCandidateError::PendingCandidateSetMismatch);
            }
            Ok(())
        }
        EpisodeStatus::Terminal { .. } | EpisodeStatus::Truncated { .. } => {
            if state.execution_v3.pending_decision.is_some() {
                return Err(BasicLandCandidateError::PendingCandidateSetMismatch);
            }
            Ok(())
        }
    }
}

/// Installs the next authoritative V3 request into an already revisioned
/// transition workspace. The caller must advance the state revision and emit
/// the matching trusted decision-created event in the same RulesKernel
/// product. Allocation and installation are atomic within this helper.
pub fn install_basic_land_request(
    admission: &ExecutableProfileAdmissionV1,
    state: &mut EngineStatePartsV2,
    actor: PlayerId,
    status: &EpisodeStatus,
) -> Result<AuthoritativeDecisionRequestV3, BasicLandCandidateError> {
    let mut candidate_state = state.clone();
    let candidates = derive_basic_land_candidates(admission, &candidate_state, actor, status)?;
    if candidates.is_empty() {
        return Err(BasicLandCandidateError::NoCandidates);
    }
    let identity = candidate_state
        .predecessor_v5
        .perspective_identities
        .players
        .get_mut(&actor)
        .ok_or(BasicLandCandidateError::InvalidState)?;
    let decision_id = candidate_state.predecessor_v5.allocators.next_decision_id;
    let next_decision_id = decision_id
        .0
        .checked_add(1)
        .ok_or(BasicLandCandidateError::IdentityExhausted)?;
    let player_decision_id = identity.next_player_decision_id;
    let next_player_decision_id = player_decision_id
        .0
        .checked_add(1)
        .ok_or(BasicLandCandidateError::IdentityExhausted)?;
    let request = AuthoritativeDecisionRequestV3 {
        decision_id,
        player_decision_id,
        state_revision: candidate_state.predecessor_v5.revision,
        actor,
        visibility: DecisionVisibility::Public,
        decision: DecisionDomainV2::ChooseOne,
        candidates,
        continuation_id: None,
    };
    request
        .validate()
        .map_err(|_| BasicLandCandidateError::CandidateOrdering)?;
    candidate_state.predecessor_v5.allocators.next_decision_id =
        mtgml_model::DecisionId(next_decision_id);
    let final_identity = candidate_state
        .predecessor_v5
        .perspective_identities
        .players
        .get_mut(&actor)
        .ok_or(BasicLandCandidateError::InvalidState)?;
    final_identity.next_player_decision_id =
        mtgml_model::PlayerDecisionIdV1(next_player_decision_id);
    candidate_state.execution_v3.pending_decision = Some(request.clone());
    candidate_state
        .validate()
        .map_err(|_| BasicLandCandidateError::InvalidState)?;
    *state = candidate_state;
    Ok(request)
}

/// Validates the exact pending V3 request and V2 answer, recomputes its full
/// candidate set from the current state, and forms the closed internal
/// action request. No caller may directly author a `MagicActionRequestV1`.
pub fn selected_magic_action_request(
    admission: &ExecutableProfileAdmissionV1,
    state: &EngineStatePartsV2,
    actor: PlayerId,
    response: &mtgml_decision::DecisionResponseV2,
    status: &EpisodeStatus,
) -> Result<MagicActionRequestV1, BasicLandCandidateError> {
    match selected_successor_decision(admission, state, actor, response, status)? {
        SelectedSuccessorDecisionV1::MagicAction(action) => Ok(action),
        SelectedSuccessorDecisionV1::PassPriority => {
            Err(BasicLandCandidateError::UnsupportedSelectedAction)
        }
    }
}

pub fn selected_successor_decision(
    admission: &ExecutableProfileAdmissionV1,
    state: &EngineStatePartsV2,
    actor: PlayerId,
    response: &mtgml_decision::DecisionResponseV2,
    status: &EpisodeStatus,
) -> Result<SelectedSuccessorDecisionV1, BasicLandCandidateError> {
    state
        .validate()
        .map_err(|_| BasicLandCandidateError::InvalidState)?;
    let request = state
        .execution_v3
        .pending_decision
        .as_ref()
        .ok_or(BasicLandCandidateError::InvalidState)?;
    let mut candidate_state = state.clone();
    candidate_state.execution_v3.pending_decision = None;
    let expected = derive_basic_land_candidates(admission, &candidate_state, actor, status)?;
    if request.candidates != expected {
        return Err(BasicLandCandidateError::PendingCandidateSetMismatch);
    }
    let selected = state
        .execution_v3
        .selected_bindings(actor, state.predecessor_v5.revision, response)
        .map_err(|_| BasicLandCandidateError::InvalidState)?;
    if selected.len() != 1 {
        return Err(BasicLandCandidateError::UnsupportedSelectedAction);
    }
    match selected[0] {
        EngineCandidateBindingV3::PlayLand { object } => Ok(
            SelectedSuccessorDecisionV1::MagicAction(MagicActionRequestV1::PlayLand {
                actor,
                object: *object,
            }),
        ),
        EngineCandidateBindingV3::ActivateAbility { ability } => Ok(
            SelectedSuccessorDecisionV1::MagicAction(MagicActionRequestV1::ActivateManaAbility {
                actor,
                ability: *ability,
            }),
        ),
        EngineCandidateBindingV3::PassPriority => Ok(SelectedSuccessorDecisionV1::PassPriority),
        _ => Err(BasicLandCandidateError::UnsupportedSelectedAction),
    }
}

#[cfg(test)]
pub(crate) fn s1_b_state_with_two_lands_fixture() -> EngineStatePartsV2 {
    tests::state_with_two_lands()
}

#[cfg(test)]
pub(crate) fn basic_land_admission_fixture() -> mtgml_card_ir::ExecutableProfileAdmissionV1 {
    tests::admission()
}

#[cfg(test)]
pub(crate) fn content_only_admission_fixture() -> mtgml_card_ir::ExecutableProfileAdmissionV1 {
    tests::admission_with_closure(&[
        "rules/basic-land-mana",
        "rules/basic-priority",
        "rules/land-play",
        "rules/mana-pool",
        "rules/state-based-actions-combat",
        "rules/turn-structure",
        "rules/zone-incarnation",
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use mtgml_card_ir::{
        admit_executable_profile_v1, decode_content_manifest_v1, ExecutableProfileAdmissionV1,
    };
    use mtgml_decision::CandidateIntentV3;
    use mtgml_model::{
        CardDefinitionId, ExecutionIdentityV1, PlayerId, RulesAuthorityV1, RulesContractManifestV1,
        SemanticContractManifestV1,
    };
    use mtgml_persistence::{
        content_contract_digest::calculate_content_contract_id_v1,
        semantic_contract_digest::{
            calculate_rules_contract_id_v1, calculate_semantic_contract_id_v1,
        },
    };
    use mtgml_state::{
        AbilityAuthorityStateV1, AbilityAuthorityV1, CardRulesAuthoritativeStateV1, FaceStateV1,
        ManaStateV1, PlayerTurnHistoryV1, TurnHistoryStateV1,
    };
    use std::collections::BTreeMap;

    const MANIFEST: &[u8] =
        include_bytes!("../../../cards/definitions/basic-land-v1/content-contract.v1.cbor");
    const PROVENANCE: &[u8] =
        include_bytes!("../../../cards/definitions/basic-land-v1/provenance.v1.cbor");
    const RULES_SNAPSHOT: &str = "wotc-cr-2026-09-25-txt-20260925-sha256-8d860e451f20f38865b725b42d82feb714c725373dd8f3b32b8652b3eeb070ca";

    pub(super) fn admission() -> ExecutableProfileAdmissionV1 {
        admission_with_closure(&[
            "rules/basic-land-mana",
            "rules/basic-priority",
            "rules/cleanup-reset",
            "rules/combat-phase",
            "rules/declare-attackers",
            "rules/draw-card",
            "rules/land-play",
            "rules/mana-pool",
            "rules/state-based-actions-combat",
            "rules/state-based-actions-empty-library",
            "rules/turn-structure",
            "rules/zone-incarnation",
        ])
    }

    pub(super) fn admission_with_closure(keys: &[&str]) -> ExecutableProfileAdmissionV1 {
        let id = calculate_content_contract_id_v1(MANIFEST).unwrap();
        let closure = keys
            .iter()
            .copied()
            .map(|key| mtgml_model::CapabilityRequirementV1 {
                key: key.to_owned(),
                version: "0.1.0".to_owned(),
            })
            .collect();
        let rules = RulesContractManifestV1 {
            rules_authority: RulesAuthorityV1::ComprehensiveRules {
                snapshot_id: RULES_SNAPSHOT.to_owned(),
            },
            capability_closure: Some(closure),
        };
        let semantic = SemanticContractManifestV1 {
            rules_contract_id: calculate_rules_contract_id_v1(&rules).unwrap(),
            format_contract_id: None,
            content_contract_id: Some(id.clone()),
        };
        let semantic_id = calculate_semantic_contract_id_v1(&semantic).unwrap();
        let execution = ExecutionIdentityV1 {
            program_kind: ExecutionProgramV1::MagicRules,
            semantic_contract_id: semantic_id,
        };
        admit_executable_profile_v1(MANIFEST, &id, PROVENANCE, &rules, &semantic, &execution)
            .unwrap()
    }

    pub(super) fn state_with_two_lands() -> EngineStatePartsV2 {
        let admission = admission();
        let manifest = decode_content_manifest_v1(MANIFEST).unwrap();
        let mountain = manifest
            .definitions
            .iter()
            .find(|definition| {
                matches!(definition.semantic_binding, CardSemanticBindingV1::ProfiledV1 { body, .. } if body.subtype == mtgml_card_ir::BasicLandSubtypeV1::Mountain)
            })
            .unwrap()
            .card_definition_id;
        let plains = manifest
            .definitions
            .iter()
            .find(|definition| {
                matches!(definition.semantic_binding, CardSemanticBindingV1::ProfiledV1 { body, .. } if body.subtype == mtgml_card_ir::BasicLandSubtypeV1::Plains)
            })
            .unwrap()
            .card_definition_id;
        let actor = PlayerId(1);
        let mut setup = mtgml_state::SyntheticV4Setup::synthetic_compatibility();
        setup.position = TurnPosition::PrecombatMain;
        setup.priority = mtgml_state::PriorityState::HeldBy {
            player: actor,
            consecutive_passes: 0,
        };
        let mut engine =
            mtgml_state::construct_synthetic_engine_state(mtgml_state::SyntheticResetInputs {
                players: [PlayerId(1), PlayerId(2)],
                root_seed: mtgml_random::RootSeed256([0x28; 32]),
                setup,
            })
            .unwrap();
        let mountain_id = add_object(&mut engine, mountain, actor, ZoneKind::Hand, 10);
        let plains_id = add_object(&mut engine, plains, actor, ZoneKind::Hand, 11);
        let battlefield_id = add_object(&mut engine, mountain, actor, ZoneKind::Battlefield, 12);
        mtgml_state::validate_engine_state(&engine).unwrap();
        for (player, identity) in &mut engine.perspective_identities.players {
            let opaque = mtgml_model::OpaqueAbilityId(player.0);
            identity
                .ability_to_opaque
                .insert(mtgml_model::AbilityInstanceId(1), opaque);
            identity
                .opaque_to_ability
                .insert(opaque, mtgml_model::AbilityInstanceId(1));
            identity.next_opaque_ability_id.0 = identity.next_opaque_ability_id.0.max(player.0 + 1);
        }
        engine.allocators.next_ability_id = mtgml_model::AbilityInstanceId(2);
        let cards = CardRulesAuthoritativeStateV1 {
            mana: ManaStateV1 {
                pools: engine
                    .core
                    .players
                    .keys()
                    .map(|player| (*player, Default::default()))
                    .collect(),
            },
            turn_history: TurnHistoryStateV1 {
                turn_number: engine.core.turn_number,
                players: engine
                    .core
                    .players
                    .keys()
                    .map(|player| (*player, PlayerTurnHistoryV1::default()))
                    .collect(),
                ..TurnHistoryStateV1::default()
            },
            faces: FaceStateV1 {
                faces: engine
                    .zones
                    .objects
                    .keys()
                    .map(|object| (*object, 0))
                    .collect(),
            },
            abilities: AbilityAuthorityStateV1 {
                by_instance: BTreeMap::from([(
                    mtgml_model::AbilityInstanceId(1),
                    AbilityAuthorityV1 {
                        source: battlefield_id,
                        ability_key: 0,
                    },
                )]),
            },
            ..CardRulesAuthoritativeStateV1::default()
        };
        let mut state = EngineStatePartsV2::from_state(&engine, cards);
        state.predecessor_v5.allocators.next_ability_id = mtgml_model::AbilityInstanceId(2);
        state.execution_v3.pending_decision = None;
        // The variable prevents an accidental fixture regression that omits
        // one of the two distinct legal land candidates.
        assert_ne!(mountain_id, plains_id);
        assert_eq!(
            admission.content_contract_id(),
            admission.verified_catalog().content_contract_id()
        );
        state
    }

    fn add_object(
        state: &mut mtgml_state::EngineState,
        definition: CardDefinitionId,
        owner: PlayerId,
        zone: ZoneKind,
        opaque_id: u64,
    ) -> mtgml_model::GameObjectId {
        use mtgml_state::{GameObject, VisibilityPartition, ZoneKey, ZoneLocation, ZonePosition};
        let id = state.allocators.allocate_object_id().unwrap();
        state.zones.objects.insert(
            id,
            GameObject {
                id,
                physical_card: None,
                card_definition: definition,
                owner,
                controller: owner,
                tapped: false,
                face_down: false,
            },
        );
        let key = ZoneKey {
            zone,
            player: Some(owner),
            visibility: if zone == ZoneKind::Battlefield {
                VisibilityPartition::Public
            } else {
                VisibilityPartition::OwnerOnly
            },
            partition: None,
        };
        let index = state.zones.ordered_zones.get(&key).map_or(0, Vec::len) as u32;
        let location = ZoneLocation {
            zone,
            player: Some(owner),
            position: ZonePosition::Top { offset: index },
            visibility: key.visibility,
            partition: None,
        };
        state.zones.locations.insert(id, location);
        state.zones.ordered_zones.entry(key).or_default().push(id);
        for (player, identity) in &mut state.perspective_identities.players {
            let assigned = opaque_id + player.0 * 100;
            let opaque = mtgml_model::OpaqueObjectId(assigned);
            identity.object_to_opaque.insert(id, opaque);
            identity.opaque_to_object.insert(opaque, id);
            identity.next_opaque_object_id.0 = identity.next_opaque_object_id.0.max(assigned + 1);
        }
        id
    }

    #[test]
    fn legal_candidate_set_is_complete_and_canonically_ordered() {
        let admission = admission();
        let state = state_with_two_lands();
        state.validate().unwrap();
        let candidates =
            derive_basic_land_candidates(&admission, &state, PlayerId(1), &EpisodeStatus::Running)
                .unwrap();
        let intents = candidates
            .iter()
            .map(|candidate| candidate.visible_intent.clone())
            .collect::<Vec<_>>();
        assert_eq!(intents.len(), 4);
        assert!(matches!(intents[0], CandidateIntentV3::PassPriority));
        assert!(matches!(intents[1], CandidateIntentV3::PlayLand { .. }));
        assert!(matches!(intents[2], CandidateIntentV3::PlayLand { .. }));
        assert!(matches!(
            intents[3],
            CandidateIntentV3::ActivateAbility { .. }
        ));
        let visible = intents
            .iter()
            .enumerate()
            .map(|(index, intent)| mtgml_decision::VisibleCandidateV3 {
                candidate_id: mtgml_model::CandidateIdV1(index as u32),
                intent: intent.clone(),
            })
            .collect::<Vec<_>>();
        CandidateOrderingV2::validate_public(&visible).unwrap();
    }

    #[test]
    fn v4_priority_domain_reuses_exact_m42_candidate_order_and_bindings() {
        let admission = admission();
        let v2 = state_with_two_lands();
        let mut state = mtgml_state::EngineStatePartsV3::new(
            v2.predecessor_v5,
            Default::default(),
            v2.card_rules_state,
        )
        .unwrap();
        let candidates = crate::derive_basic_land_candidates_v4(
            &admission,
            &state,
            PlayerId(1),
            &EpisodeStatus::Running,
        )
        .unwrap();
        assert_eq!(candidates.len(), 4);
        assert!(matches!(
            candidates[0].visible_intent,
            mtgml_decision::CandidateIntentV4::PassPriority
        ));
        assert!(matches!(
            candidates[1].visible_intent,
            mtgml_decision::CandidateIntentV4::PlayLand { .. }
        ));
        assert!(matches!(
            candidates[2].visible_intent,
            mtgml_decision::CandidateIntentV4::PlayLand { .. }
        ));
        assert!(matches!(
            candidates[3].visible_intent,
            mtgml_decision::CandidateIntentV4::ActivateAbility { .. }
        ));

        let request = crate::install_basic_land_request_v4(
            &admission,
            &mut state,
            PlayerId(1),
            &EpisodeStatus::Running,
        )
        .unwrap();
        assert!(state.validate().is_err());
        state.validate_structure().unwrap();
        crate::validate_basic_land_pending_request_v4(&admission, &state, &EpisodeStatus::Running)
            .unwrap();

        let response = mtgml_decision::DecisionResponseV3 {
            schema_version: mtgml_decision::DECISION_RESPONSE_V3_SCHEMA.to_owned(),
            player_decision_id: request.player_decision_id,
            view_sequence: request.view_sequence,
            answer: mtgml_decision::DecisionAnswerV2::SelectOne {
                candidate_id: mtgml_model::CandidateIdV1(1),
            },
        };
        assert!(matches!(
            crate::selected_basic_land_action_v4(
                &admission,
                &state,
                PlayerId(1),
                &response,
                &EpisodeStatus::Running,
            ),
            Ok(SelectedSuccessorDecisionV1::MagicAction(
                MagicActionRequestV1::PlayLand { .. }
            ))
        ));
        let transition = crate::execute_basic_land_response_v4(
            &admission,
            &state,
            PlayerId(1),
            &response,
            &EpisodeStatus::Running,
        )
        .unwrap();
        assert!(transition.accepted);
        transition.next_state.validate_structure().unwrap();
        assert_eq!(
            transition.delta.apply_structural_only(&state).unwrap(),
            transition.next_state
        );
        let stale = mtgml_decision::DecisionResponseV3 {
            view_sequence: mtgml_model::VisibleSequence(request.view_sequence.0 + 1),
            ..response
        };
        assert!(crate::selected_basic_land_action_v4(
            &admission,
            &state,
            PlayerId(1),
            &stale,
            &EpisodeStatus::Running,
        )
        .is_err());
        let before_rejected = state.clone();
        assert!(crate::execute_basic_land_response_v4(
            &admission,
            &state,
            PlayerId(1),
            &stale,
            &EpisodeStatus::Running,
        )
        .is_err());
        assert_eq!(state, before_rejected);
    }

    #[test]
    fn v4_mana_ability_transition_preserves_atomic_tap_and_pool_outcome() {
        let admission = admission();
        let v2 = state_with_two_lands();
        let mut state = mtgml_state::EngineStatePartsV3::new(
            v2.predecessor_v5,
            Default::default(),
            v2.card_rules_state,
        )
        .unwrap();
        let request = crate::install_basic_land_request_v4(
            &admission,
            &mut state,
            PlayerId(1),
            &EpisodeStatus::Running,
        )
        .unwrap();
        let response = mtgml_decision::DecisionResponseV3 {
            schema_version: mtgml_decision::DECISION_RESPONSE_V3_SCHEMA.to_owned(),
            player_decision_id: request.player_decision_id,
            view_sequence: request.view_sequence,
            answer: mtgml_decision::DecisionAnswerV2::SelectOne {
                candidate_id: mtgml_model::CandidateIdV1(3),
            },
        };
        let transition = crate::execute_basic_land_response_v4(
            &admission,
            &state,
            PlayerId(1),
            &response,
            &EpisodeStatus::Running,
        )
        .unwrap();
        assert!(transition.accepted);
        let source = match request.candidates[3].trusted_binding {
            mtgml_decision::EngineCandidateBindingV4::ActivateAbility { ability } => {
                state.card_rules_state.abilities.by_instance[&ability].source
            }
            _ => panic!("candidate 3 is the intrinsic mana ability"),
        };
        assert!(transition.next_state.predecessor_v5.zones.objects[&source].tapped);
        assert_eq!(
            transition.next_state.card_rules_state.mana.pools[&PlayerId(1)].unrestricted[3],
            1
        );
        assert_eq!(
            transition.delta.apply_structural_only(&state).unwrap(),
            transition.next_state
        );
    }

    #[test]
    fn v4_pass_priority_preserves_actor_transfer_and_next_request() {
        let admission = admission();
        let v2 = state_with_two_lands();
        let mut state = mtgml_state::EngineStatePartsV3::new(
            v2.predecessor_v5,
            Default::default(),
            v2.card_rules_state,
        )
        .unwrap();
        let request = crate::install_basic_land_request_v4(
            &admission,
            &mut state,
            PlayerId(1),
            &EpisodeStatus::Running,
        )
        .unwrap();
        let response = mtgml_decision::DecisionResponseV3 {
            schema_version: mtgml_decision::DECISION_RESPONSE_V3_SCHEMA.to_owned(),
            player_decision_id: request.player_decision_id,
            view_sequence: request.view_sequence,
            answer: mtgml_decision::DecisionAnswerV2::SelectOne {
                candidate_id: mtgml_model::CandidateIdV1(0),
            },
        };
        let transition = crate::execute_basic_land_response_v4(
            &admission,
            &state,
            PlayerId(1),
            &response,
            &EpisodeStatus::Running,
        )
        .unwrap();
        assert!(transition.accepted);
        assert!(matches!(
            transition.next_state.predecessor_v5.core.priority,
            mtgml_state::PriorityState::HeldBy {
                player: PlayerId(2),
                consecutive_passes: 1,
            }
        ));
        assert_eq!(
            transition.next_decision.as_ref().unwrap().actor,
            PlayerId(2)
        );
        transition.next_state.validate_structure().unwrap();
    }

    #[test]
    fn v4_basic_land_admission_rejects_structural_but_unsupported_cast_action() {
        let admission = admission();
        let v2 = state_with_two_lands();
        let mut state = mtgml_state::EngineStatePartsV3::new(
            v2.predecessor_v5,
            Default::default(),
            v2.card_rules_state,
        )
        .unwrap();
        let request = crate::install_basic_land_request_v4(
            &admission,
            &mut state,
            PlayerId(1),
            &EpisodeStatus::Running,
        )
        .unwrap();
        let land_object = match request.candidates[1].trusted_binding {
            mtgml_decision::EngineCandidateBindingV4::PlayLand { object } => object,
            _ => panic!("candidate 1 is a legal PlayLand"),
        };
        let opaque_object = state.predecessor_v5.perspective_identities.players[&PlayerId(1)]
            .object_to_opaque[&land_object];
        let mut forged = request;
        forged.candidates.insert(
            3,
            mtgml_decision::AuthoritativeCandidateV4 {
                candidate_id: mtgml_model::CandidateIdV1(3),
                visible_intent: mtgml_decision::CandidateIntentV4::CastSpell {
                    object: opaque_object,
                },
                trusted_binding: mtgml_decision::EngineCandidateBindingV4::CastSpell {
                    object: land_object,
                },
            },
        );
        for (index, candidate) in forged.candidates.iter_mut().enumerate() {
            candidate.candidate_id = mtgml_model::CandidateIdV1(index as u32);
        }
        forged.project_player_request().unwrap();
        state.execution_v4.pending_decision = Some(forged);
        state.validate_structure().unwrap();
        assert!(crate::validate_basic_land_pending_request_v4(
            &admission,
            &state,
            &EpisodeStatus::Running,
        )
        .is_err());
    }

    #[test]
    fn installed_request_is_digest_bound_v3_authority_with_dense_candidates() {
        let admission = admission();
        let mut state = state_with_two_lands();
        let request = install_basic_land_request(
            &admission,
            &mut state,
            PlayerId(1),
            &EpisodeStatus::Running,
        )
        .unwrap();
        assert_eq!(state.execution_v3.pending_decision.as_ref(), Some(&request));
        assert_eq!(request.state_revision, state.predecessor_v5.revision);
        assert_eq!(request.candidates[0].candidate_id.0, 0);
        assert!(matches!(
            request.candidates[0].visible_intent,
            CandidateIntentV3::PassPriority
        ));
        assert_eq!(
            request.project_player_request().unwrap().candidates.len(),
            request.candidates.len()
        );
        state.full_state_digest_v6().unwrap();
    }

    #[test]
    fn failed_request_installation_preserves_complete_successor_state() {
        let admission = admission();
        let mut state = state_with_two_lands();
        state.predecessor_v5.allocators.next_decision_id = mtgml_model::DecisionId(u64::MAX);
        let before = state.clone();
        assert_eq!(
            install_basic_land_request(
                &admission,
                &mut state,
                PlayerId(1),
                &EpisodeStatus::Running,
            ),
            Err(BasicLandCandidateError::IdentityExhausted)
        );
        assert_eq!(state, before);
    }

    #[test]
    fn program_kernel_successor_entry_requires_the_verified_profile_admission() {
        let kernel = crate::ProgramKernelV1::for_executable_profile(admission()).unwrap();
        let mut state = state_with_two_lands();
        let request = kernel
            .install_successor_request(&mut state, PlayerId(1), &EpisodeStatus::Running)
            .unwrap();
        assert_eq!(
            request,
            state.execution_v3.pending_decision.clone().unwrap()
        );
        assert_eq!(request.candidates.len(), 4);
        assert!(
            kernel
                .successor_candidates(&state, PlayerId(1), &EpisodeStatus::Running)
                .is_err(),
            "a second active pending request cannot be synthesized"
        );
    }

    #[test]
    fn program_kernel_rules_trait_commits_v3_successor_product() {
        let mut kernel = crate::ProgramKernelV1::for_executable_profile(admission()).unwrap();
        let mut state = state_with_two_lands();
        let request = kernel
            .install_successor_request(&mut state, PlayerId(1), &EpisodeStatus::Running)
            .unwrap();

        let legacy_projection = state.materialize();
        let before_legacy_call = legacy_projection.clone();
        assert!(matches!(
            kernel.apply_predecessor(
                &legacy_projection,
                PlayerId(1),
                &select_response(&request, 1)
            ),
            Err(crate::KernelExecutionError::UnsupportedPlayerResponse)
        ));
        assert_eq!(legacy_projection, before_legacy_call);

        let selected = request
            .candidates
            .iter()
            .find(|candidate| {
                matches!(candidate.visible_intent, CandidateIntentV3::PlayLand { .. })
            })
            .unwrap();
        let response = select_response(&request, selected.candidate_id.0);

        let product = crate::RulesKernel::apply(
            &mut kernel,
            &state,
            PlayerId(1),
            &response,
            &EpisodeStatus::Running,
        )
        .unwrap();

        assert!(product.accepted);
        assert_eq!(product.delta.apply(&state).unwrap(), product.next_state);
        assert!(product.next_state.execution_v3.pending_decision.is_some());
        assert!(product
            .next_state
            .predecessor_v5
            .execution
            .pending_decision
            .is_none());
        assert_eq!(
            product.next_decision,
            product.next_state.execution_v3.pending_decision
        );
    }

    #[test]
    fn selected_magic_action_exists_only_for_exact_pending_candidate_and_response() {
        let admission = admission();
        let mut state = state_with_two_lands();
        let request = install_basic_land_request(
            &admission,
            &mut state,
            PlayerId(1),
            &EpisodeStatus::Running,
        )
        .unwrap();
        let response = mtgml_decision::DecisionResponseV2 {
            schema_version: mtgml_decision::DECISION_RESPONSE_V2_SCHEMA.to_owned(),
            player_decision_id: request.player_decision_id,
            state_revision: request.state_revision,
            answer: mtgml_decision::DecisionAnswerV2::SelectOne {
                candidate_id: mtgml_model::CandidateIdV1(1),
            },
        };
        let before = state.clone();
        assert!(matches!(
            selected_magic_action_request(
                &admission,
                &state,
                PlayerId(1),
                &response,
                &EpisodeStatus::Running,
            ),
            Ok(MagicActionRequestV1::PlayLand {
                actor: PlayerId(1),
                ..
            })
        ));
        assert_eq!(state, before);

        let mut stale = response.clone();
        stale.state_revision.0 += 1;
        assert_eq!(
            selected_magic_action_request(
                &admission,
                &state,
                PlayerId(1),
                &stale,
                &EpisodeStatus::Running,
            ),
            Err(BasicLandCandidateError::InvalidState)
        );
        assert_eq!(state, before);
    }

    fn select_response(
        request: &AuthoritativeDecisionRequestV3,
        candidate: u32,
    ) -> mtgml_decision::DecisionResponseV2 {
        mtgml_decision::DecisionResponseV2 {
            schema_version: mtgml_decision::DECISION_RESPONSE_V2_SCHEMA.to_owned(),
            player_decision_id: request.player_decision_id,
            state_revision: request.state_revision,
            answer: mtgml_decision::DecisionAnswerV2::SelectOne {
                candidate_id: mtgml_model::CandidateIdV1(candidate),
            },
        }
    }

    #[test]
    fn selected_play_land_executes_as_one_digest_bound_incarnation_transition() {
        let admission = admission();
        let mut state = state_with_two_lands();
        let old_hand_object =
            state
                .predecessor_v5
                .zones
                .locations
                .iter()
                .find_map(|(object, location)| {
                    (location.zone == ZoneKind::Hand
                    && admission
                        .verified_catalog()
                        .get(
                            admission.content_contract_id(),
                            state.predecessor_v5.zones.objects[object].card_definition,
                        )
                        .is_ok_and(|definition| matches!(
                            definition.semantic_binding,
                            CardSemanticBindingV1::ProfiledV1 { body, .. }
                                if body.subtype == mtgml_card_ir::BasicLandSubtypeV1::Mountain
                        )))
                    .then_some(*object)
                })
                .unwrap();
        let request = install_basic_land_request(
            &admission,
            &mut state,
            PlayerId(1),
            &EpisodeStatus::Running,
        )
        .unwrap();
        let response = select_response(&request, 1);
        let before = state.clone();
        let product = execute_basic_land_response(
            &admission,
            &state,
            PlayerId(1),
            &response,
            &EpisodeStatus::Running,
        )
        .unwrap();
        let successor_product = crate::TransitionResult {
            accepted: product.accepted,
            next_state: product.next_state.clone(),
            delta: product.delta.clone(),
            events: product.events.clone(),
            next_decision: product.next_decision.clone(),
            status: product.status.clone(),
        };
        crate::validate_successor_transition_contract(&admission, &before, &successor_product)
            .unwrap();
        let mut reordered_events = successor_product.clone();
        let first_payload = reordered_events.events[0].event.clone();
        reordered_events.events[0].event = reordered_events.events[1].event.clone();
        reordered_events.events[1].event = first_payload;
        assert_eq!(
            reordered_events.events[0].event_id, successor_product.events[0].event_id,
            "ordering RED keeps sequential event identities in place"
        );
        assert_eq!(
            reordered_events.events[1].event_id, successor_product.events[1].event_id,
            "ordering RED swaps payloads only"
        );
        assert!(matches!(
            crate::validate_successor_transition_contract(&admission, &before, &reordered_events),
            Err(crate::TransitionViolation::EventDeltaMismatch)
        ));
        let mut incomplete_candidates = successor_product.clone();
        let mut incomplete_state = incomplete_candidates.next_state.clone();
        incomplete_state
            .execution_v3
            .pending_decision
            .as_mut()
            .unwrap()
            .candidates
            .pop();
        incomplete_candidates.next_decision =
            incomplete_state.execution_v3.pending_decision.clone();
        incomplete_candidates.delta = mtgml_state::StateDeltaV2::between(
            &before,
            &incomplete_state,
            incomplete_candidates.delta.operations.clone(),
        )
        .unwrap();
        incomplete_candidates.next_state = incomplete_state;
        assert!(crate::validate_successor_transition_contract(
            &admission,
            &before,
            &incomplete_candidates,
        )
        .is_err());
        for operation in [
            SemanticDeltaOperationV2::ObjectEntered {
                old_object: None,
                new_object: mtgml_model::GameObjectId(0),
                from_zone: ZoneKind::Hand,
                to_zone: ZoneKind::Battlefield,
                tapped: false,
                face: 0,
            },
            SemanticDeltaOperationV2::LandPlayCountChanged {
                player: PlayerId(1),
                from: 0,
                to: 1,
            },
            SemanticDeltaOperationV2::AbilityAuthorityAdded {
                instance: mtgml_model::AbilityInstanceId(0),
                source: mtgml_model::GameObjectId(0),
                ability_key: 0,
            },
        ] {
            let mut tampered = successor_product.clone();
            let index = tampered
                .delta
                .operations
                .iter()
                .position(|candidate| {
                    std::mem::discriminant(candidate) == std::mem::discriminant(&operation)
                })
                .unwrap();
            tampered.delta.operations.remove(index);
            assert!(
                crate::validate_successor_transition_contract(&admission, &before, &tampered)
                    .is_err()
            );
        }
        let transitioned_object = *successor_product
            .next_state
            .predecessor_v5
            .zones
            .objects
            .keys()
            .next()
            .unwrap();
        for unsupported in [
            SemanticDeltaOperationV2::CounterChanged {
                object: transitioned_object,
                kind: mtgml_state::CounterKindV1::PlusOnePlusOne,
                from: 0,
                to: 1,
                cause: before.predecessor_v5.allocators.next_rule_event_id,
            },
            SemanticDeltaOperationV2::AttachmentChanged {
                source: transitioned_object,
                from_target: None,
                to_target: Some(transitioned_object),
                timestamp_revision: product.next_state.predecessor_v5.revision,
                operation_ordinal: 0,
            },
            SemanticDeltaOperationV2::ObjectFaceChanged {
                object: transitioned_object,
                from_face: 0,
                to_face: 1,
            },
        ] {
            let mut tampered = successor_product.clone();
            tampered.delta.operations.push(unsupported);
            tampered.delta = mtgml_state::StateDeltaV2::between(
                &before,
                &tampered.next_state,
                tampered.delta.operations.clone(),
            )
            .unwrap();
            assert!(
                crate::validate_successor_transition_contract(&admission, &before, &tampered)
                    .is_err()
            );
        }
        let mut missing_move_event = successor_product.clone();
        let index = missing_move_event
            .events
            .iter()
            .position(|event| {
                matches!(
                    event.event,
                    AuthoritativeRuleEventKindV2::ObjectMoved { .. }
                )
            })
            .unwrap();
        missing_move_event.events.remove(index);
        assert!(crate::validate_successor_transition_contract(
            &admission,
            &before,
            &missing_move_event
        )
        .is_err());
        let mut missing_land_played = successor_product.clone();
        let index = missing_land_played
            .events
            .iter()
            .position(|event| {
                matches!(event.event, AuthoritativeRuleEventKindV2::LandPlayed { .. })
            })
            .unwrap();
        missing_land_played.events.remove(index);
        assert!(crate::validate_successor_transition_contract(
            &admission,
            &before,
            &missing_land_played
        )
        .is_err());
        let mut wrong_land_played = successor_product.clone();
        let land_played = wrong_land_played
            .events
            .iter_mut()
            .find_map(|event| match &mut event.event {
                AuthoritativeRuleEventKindV2::LandPlayed {
                    land_plays_used, ..
                } => Some(land_plays_used),
                _ => None,
            })
            .unwrap();
        *land_played = 0;
        assert!(crate::validate_successor_transition_contract(
            &admission,
            &before,
            &wrong_land_played
        )
        .is_err());
        let new_object = product
            .next_state
            .predecessor_v5
            .zones
            .locations
            .keys()
            .find(|object| !before.predecessor_v5.zones.objects.contains_key(object))
            .copied()
            .unwrap();
        assert!(!product
            .next_state
            .predecessor_v5
            .zones
            .objects
            .contains_key(&old_hand_object));
        assert_eq!(
            product.next_state.predecessor_v5.zones.locations[&new_object].zone,
            ZoneKind::Battlefield
        );
        assert_eq!(
            product.next_state.card_rules_state.turn_history.players[&PlayerId(1)].land_plays_used,
            1
        );
        assert!(product.events.iter().any(|event| matches!(
            event,
            AuthoritativeRuleEventV2 { event: AuthoritativeRuleEventKindV2::ObjectMoved {
                old_object,
                new_object: moved,
                from: ZoneKind::Hand,
                to: ZoneKind::Battlefield,
                entering_face: Some(BasicLandFaceV1::Front),
                tapped: false,
            }, .. } if *old_object == old_hand_object && *moved == new_object
        )));
        assert!(product.events.iter().any(|event| matches!(
            event,
            AuthoritativeRuleEventV2 {
                event: AuthoritativeRuleEventKindV2::LandPlayed {
                    old_object,
                    new_object: played,
                    actor: PlayerId(1),
                    land_plays_used: 1,
                },
                ..
            } if *old_object == old_hand_object && *played == new_object
        )));
        assert_eq!(product.delta.apply(&before).unwrap(), product.next_state);
        assert_eq!(
            product.next_state.predecessor_v5.zones.stack_order,
            before.predecessor_v5.zones.stack_order
        );
        assert_eq!(
            product.next_state.card_rules_state.mana,
            before.card_rules_state.mana
        );
        assert_eq!(
            product
                .events
                .iter()
                .map(|event| event.event_id.0)
                .collect::<Vec<_>>(),
            (before.predecessor_v5.allocators.next_rule_event_id.0
                ..product
                    .next_state
                    .predecessor_v5
                    .allocators
                    .next_rule_event_id
                    .0)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn selected_mana_ability_taps_and_adds_subtype_mana_atomically_without_stack() {
        let admission = admission();
        let mut state = state_with_two_lands();
        let request = install_basic_land_request(
            &admission,
            &mut state,
            PlayerId(1),
            &EpisodeStatus::Running,
        )
        .unwrap();
        let response = select_response(&request, 3);
        let source = match selected_magic_action_request(
            &admission,
            &state,
            PlayerId(1),
            &response,
            &EpisodeStatus::Running,
        )
        .unwrap()
        {
            MagicActionRequestV1::ActivateManaAbility { ability, .. } => {
                state.card_rules_state.abilities.by_instance[&ability].source
            }
            _ => panic!("candidate 3 is the source land's mana ability"),
        };
        let before = state.clone();
        let product = execute_basic_land_response(
            &admission,
            &state,
            PlayerId(1),
            &response,
            &EpisodeStatus::Running,
        )
        .unwrap();
        let mut successor_product = crate::TransitionResult {
            accepted: product.accepted,
            next_state: product.next_state.clone(),
            delta: product.delta.clone(),
            events: product.events.clone(),
            next_decision: product.next_decision.clone(),
            status: product.status.clone(),
        };
        crate::validate_successor_transition_contract(&admission, &before, &successor_product)
            .unwrap();
        let mana_added = successor_product
            .delta
            .operations
            .iter()
            .position(|operation| matches!(operation, SemanticDeltaOperationV2::ManaAdded { .. }))
            .unwrap();
        successor_product.delta.operations.remove(mana_added);
        assert!(crate::validate_successor_transition_contract(
            &admission,
            &before,
            &successor_product
        )
        .is_err());
        let mut missing_tap = crate::TransitionResult {
            accepted: product.accepted,
            next_state: product.next_state.clone(),
            delta: product.delta.clone(),
            events: product.events.clone(),
            next_decision: product.next_decision.clone(),
            status: product.status.clone(),
        };
        let tap_operation = missing_tap
            .delta
            .operations
            .iter()
            .position(|operation| {
                matches!(operation, SemanticDeltaOperationV2::ObjectTapped { .. })
            })
            .unwrap();
        missing_tap.delta.operations.remove(tap_operation);
        assert!(
            crate::validate_successor_transition_contract(&admission, &before, &missing_tap)
                .is_err()
        );

        let mut wrong_mana_event = crate::TransitionResult {
            accepted: product.accepted,
            next_state: product.next_state.clone(),
            delta: product.delta.clone(),
            events: product.events.clone(),
            next_decision: product.next_decision.clone(),
            status: product.status.clone(),
        };
        let mana_event = wrong_mana_event
            .events
            .iter_mut()
            .find(|event| {
                matches!(
                    event.event,
                    AuthoritativeRuleEventKindV2::ManaPoolChanged { .. }
                )
            })
            .unwrap();
        if let AuthoritativeRuleEventKindV2::ManaPoolChanged { amount, .. } = &mut mana_event.event
        {
            *amount += 1;
        }
        assert!(crate::validate_successor_transition_contract(
            &admission,
            &before,
            &wrong_mana_event
        )
        .is_err());

        let mut successor_product = crate::TransitionResult {
            accepted: product.accepted,
            next_state: product.next_state.clone(),
            delta: product.delta.clone(),
            events: product.events.clone(),
            next_decision: product.next_decision.clone(),
            status: product.status.clone(),
        };
        let tapped_event = successor_product
            .events
            .iter()
            .position(|event| {
                matches!(
                    event.event,
                    AuthoritativeRuleEventKindV2::ObjectTapped { .. }
                )
            })
            .unwrap();
        successor_product.events.remove(tapped_event);
        assert!(crate::validate_successor_transition_contract(
            &admission,
            &before,
            &successor_product
        )
        .is_err());
        assert!(product.next_state.predecessor_v5.zones.objects[&source].tapped);
        assert_eq!(
            product.next_state.card_rules_state.mana.pools[&PlayerId(1)].unrestricted[3],
            1
        );
        assert_eq!(
            product.next_state.predecessor_v5.zones.stack_order,
            before.predecessor_v5.zones.stack_order
        );
        assert_eq!(product.delta.apply(&before).unwrap(), product.next_state);
        assert!(product.events.iter().any(|event| matches!(
            event,
            AuthoritativeRuleEventV2 {
                event: AuthoritativeRuleEventKindV2::ManaPoolChanged {
                    player: PlayerId(1),
                    color: Some(mtgml_state::ManaColorV1::Red),
                    amount: 1,
                    ..
                },
                ..
            }
        )));
    }

    #[test]
    fn mana_ability_overflow_rejects_without_mutating_the_complete_state() {
        let admission = admission();
        let mut state = state_with_two_lands();
        state
            .card_rules_state
            .mana
            .pools
            .get_mut(&PlayerId(1))
            .unwrap()
            .unrestricted[3] = u32::MAX;
        let request = install_basic_land_request(
            &admission,
            &mut state,
            PlayerId(1),
            &EpisodeStatus::Running,
        )
        .unwrap();
        let response = select_response(&request, 3);
        let before = state.clone();
        assert_eq!(
            execute_basic_land_response(
                &admission,
                &state,
                PlayerId(1),
                &response,
                &EpisodeStatus::Running,
            ),
            Err(BasicLandTransitionError::InvalidResult)
        );
        assert_eq!(state, before);
    }

    #[test]
    fn two_priority_passes_advance_one_step_and_empty_every_mana_pool() {
        let admission = admission();
        let mut state = state_with_two_lands();
        state
            .card_rules_state
            .mana
            .add(
                PlayerId(1),
                mtgml_state::ManaColorV1::Red,
                mtgml_state::ManaRestrictionV1::Unrestricted,
                2,
            )
            .unwrap();
        let request = install_basic_land_request(
            &admission,
            &mut state,
            PlayerId(1),
            &EpisodeStatus::Running,
        )
        .unwrap();
        let first = execute_basic_land_response(
            &admission,
            &state,
            PlayerId(1),
            &select_response(&request, 0),
            &EpisodeStatus::Running,
        )
        .unwrap();
        assert_eq!(
            first.next_state.predecessor_v5.core.priority,
            mtgml_state::PriorityState::HeldBy {
                player: PlayerId(2),
                consecutive_passes: 1,
            }
        );
        assert_eq!(first.next_decision.as_ref().unwrap().actor, PlayerId(2));
        assert_eq!(
            first.next_state.card_rules_state.mana.pools[&PlayerId(1)].unrestricted[3],
            2
        );
        let second_request = first.next_decision.as_ref().unwrap();
        let second = execute_basic_land_response(
            &admission,
            &first.next_state,
            PlayerId(2),
            &select_response(second_request, 0),
            &EpisodeStatus::Running,
        )
        .unwrap();
        assert_eq!(
            second.next_state.predecessor_v5.core.position,
            TurnPosition::Combat {
                step: mtgml_state::CombatStep::BeginningOfCombat,
            }
        );
        assert_eq!(
            second.next_state.predecessor_v5.core.priority,
            mtgml_state::PriorityState::HeldBy {
                player: PlayerId(1),
                consecutive_passes: 0,
            }
        );
        assert_eq!(second.next_decision.as_ref().unwrap().actor, PlayerId(1));
        assert_eq!(
            second.next_state.card_rules_state.mana.pools[&PlayerId(1)],
            mtgml_state::ManaPoolV1::default()
        );
        assert!(second.events.iter().any(|event| matches!(
            event.event,
            AuthoritativeRuleEventKindV2::ManaPoolChanged {
                player: PlayerId(1),
                amount: 0,
                ..
            }
        )));
    }

    #[test]
    fn unsupported_second_pass_boundaries_fail_closed() {
        use mtgml_state::{BeginningStep, CombatStep, EndingStep};

        assert_eq!(
            priority_window_after_second_pass(TurnPosition::PrecombatMain),
            Ok(TurnPosition::Combat {
                step: CombatStep::BeginningOfCombat,
            })
        );
        for position in [
            TurnPosition::Beginning {
                step: BeginningStep::Upkeep,
            },
            TurnPosition::Combat {
                step: CombatStep::BeginningOfCombat,
            },
            TurnPosition::Ending {
                step: EndingStep::EndStep,
            },
            TurnPosition::Ending {
                step: EndingStep::Cleanup,
            },
        ] {
            assert_eq!(
                priority_window_after_second_pass(position),
                Err(BasicLandTransitionError::UnsupportedPriorityBoundary),
                "must not open ordinary priority after unsupported boundary {position:?}"
            );
        }
    }

    #[test]
    fn illegal_play_land_windows_produce_no_land_candidates() {
        let admission = admission();
        let mut state = state_with_two_lands();
        state.predecessor_v5.core.position = TurnPosition::Beginning {
            step: mtgml_state::BeginningStep::Upkeep,
        };
        let candidates =
            derive_basic_land_candidates(&admission, &state, PlayerId(1), &EpisodeStatus::Running)
                .unwrap();
        assert!(candidates.iter().all(|candidate| !matches!(
            candidate.visible_intent,
            CandidateIntentV3::PlayLand { .. }
        )));
    }

    #[test]
    fn land_candidates_require_active_player_priority_and_remaining_entitlement() {
        let admission = admission();
        let mut state = state_with_two_lands();
        state.predecessor_v5.core.priority = mtgml_state::PriorityState::HeldBy {
            player: PlayerId(2),
            consecutive_passes: 0,
        };
        let candidates =
            derive_basic_land_candidates(&admission, &state, PlayerId(1), &EpisodeStatus::Running)
                .unwrap();
        assert!(candidates.is_empty());

        state.predecessor_v5.core.priority = mtgml_state::PriorityState::HeldBy {
            player: PlayerId(1),
            consecutive_passes: 0,
        };
        state
            .card_rules_state
            .turn_history
            .players
            .get_mut(&PlayerId(1))
            .unwrap()
            .land_plays_used = 1;
        let candidates =
            derive_basic_land_candidates(&admission, &state, PlayerId(1), &EpisodeStatus::Running)
                .unwrap();
        assert!(candidates.iter().all(|candidate| !matches!(
            candidate.visible_intent,
            CandidateIntentV3::PlayLand { .. }
        )));
        assert!(candidates.iter().any(|candidate| matches!(
            candidate.visible_intent,
            CandidateIntentV3::ActivateAbility { .. }
        )));
        assert!(candidates
            .iter()
            .any(|candidate| matches!(candidate.visible_intent, CandidateIntentV3::PassPriority)));
    }

    fn successor_state_with_two_lands() -> mtgml_state::EngineStatePartsV3 {
        let state = state_with_two_lands();
        mtgml_state::EngineStatePartsV3::new(
            state.predecessor_v5,
            Default::default(),
            state.card_rules_state,
        )
        .unwrap()
    }

    #[test]
    fn s1_a_resolves_exact_admitted_live_object_without_mutation() {
        let admission = admission();
        let state = successor_state_with_two_lands();
        let object_id = *state.predecessor_v5.zones.objects.keys().next().unwrap();
        let before = state.clone();
        let authority = crate::S1QueryAuthority::for_object(&admission, &state, object_id).unwrap();
        let queried = authority.queried_object();

        assert_eq!(queried.object, object_id);
        let object = state.predecessor_v5.zones.objects.get(&object_id).unwrap();
        let location = state
            .predecessor_v5
            .zones
            .locations
            .get(&object_id)
            .unwrap();
        assert_eq!(queried.card_definition, object.card_definition);
        assert_eq!(queried.owner, object.owner);
        assert_eq!(queried.controller, object.controller);
        assert_eq!(queried.zone, location.zone);
        assert_eq!(queried.face_key.0, 0);
        assert_eq!(
            authority.execution_identity(),
            admission.execution_identity()
        );
        assert_eq!(state, before);
    }

    #[test]
    fn s1_query_for_objects_matches_one_query_per_object() {
        let admission = admission();
        let state = successor_state_with_two_lands();
        let objects: Vec<_> = state
            .predecessor_v5
            .zones
            .objects
            .iter()
            .filter(|(_, object)| !object.face_down)
            .map(|(id, _)| *id)
            .collect();
        assert!(objects.len() >= 2);
        let batch = crate::S1QueryAuthority::for_objects(&admission, &state, &objects).unwrap();
        let single: Vec<_> = objects
            .iter()
            .map(|object| {
                crate::S1QueryAuthority::for_object(&admission, &state, *object)
                    .unwrap()
                    .queried_object()
            })
            .collect();
        assert_eq!(
            batch.iter().map(|a| a.queried_object()).collect::<Vec<_>>(),
            single
        );
        assert!(
            crate::S1QueryAuthority::for_objects(&admission, &state, &[])
                .unwrap()
                .is_empty()
        );

        let mut missing_location = state.clone();
        missing_location
            .predecessor_v5
            .zones
            .locations
            .remove(&objects[1]);
        for order in [[objects[0], objects[1]], [objects[1], objects[0]]] {
            assert_eq!(
                crate::S1QueryAuthority::for_objects(&admission, &missing_location, &order)
                    .unwrap_err(),
                crate::S1QueryAuthority::for_object(&admission, &missing_location, order[0])
                    .unwrap_err(),
            );
        }
    }

    #[test]
    fn s1_b_admitted_mountain_and_plains_have_empty_colors_and_no_pt() {
        let admission = admission();
        let state = successor_state_with_two_lands();
        let before = state.clone();
        let mut seen_mountain = false;
        let mut seen_plains = false;

        for (object_id, object) in &state.predecessor_v5.zones.objects {
            // The shared synthetic setup also contains a face-down object.
            // S1-B production positives cover only the admitted face-up land
            // objects; face-down rejection remains an S1-A negative case.
            if object.face_down {
                continue;
            }
            let definition = admission
                .verified_catalog()
                .get(admission.content_contract_id(), object.card_definition)
                .unwrap();
            let subtype = match &definition.semantic_binding {
                CardSemanticBindingV1::ProfiledV1 { body, .. } => body.subtype,
                CardSemanticBindingV1::UnprofiledV1 => panic!("expected admitted Basic Land"),
            };
            let authority =
                crate::S1QueryAuthority::for_object(&admission, &state, *object_id).unwrap();
            let result = authority.derive_base_characteristics();
            assert_eq!(result, authority.derive_base_characteristics());

            assert!(result.colors.is_empty());
            assert_eq!(result.base_power_toughness, None);
            assert_eq!(result.queried.card_definition, object.card_definition);
            assert_eq!(result.queried.face_key.0, 0);
            match subtype {
                mtgml_card_ir::BasicLandSubtypeV1::Mountain => {
                    assert_eq!(result.supertypes, ["Basic"]);
                    assert_eq!(result.card_types, ["Land"]);
                    assert_eq!(result.subtypes, ["Mountain"]);
                    seen_mountain = true;
                }
                mtgml_card_ir::BasicLandSubtypeV1::Plains => {
                    assert_eq!(result.supertypes, ["Basic"]);
                    assert_eq!(result.card_types, ["Land"]);
                    assert_eq!(result.subtypes, ["Plains"]);
                    seen_plains = true;
                }
            }
        }
        assert!(seen_mountain && seen_plains);
        assert_eq!(state, before);
    }

    #[test]
    fn s1_a_rejects_unknown_definition_without_fallback_or_mutation() {
        let admission = admission();
        let mut state = successor_state_with_two_lands();
        let object_id = *state.predecessor_v5.zones.objects.keys().next().unwrap();
        state
            .predecessor_v5
            .zones
            .objects
            .get_mut(&object_id)
            .unwrap()
            .card_definition = CardDefinitionId(999_999);
        for (player, identity) in &state.predecessor_v5.perspective_identities.players {
            if let Some(opaque) = identity.object_to_opaque.get(&object_id) {
                if let Some(record) = state
                    .predecessor_v5
                    .knowledge
                    .players
                    .get_mut(player)
                    .and_then(|knowledge| knowledge.active.get_mut(opaque))
                {
                    record.card_definition = Some(CardDefinitionId(999_999));
                }
            }
        }
        let before = state.clone();

        let error = crate::S1QueryAuthority::for_object(&admission, &state, object_id).unwrap_err();
        assert!(
            matches!(
                error,
                crate::S1QueryError::MissingCardDefinition(CardDefinitionId(999_999))
            ),
            "unexpected query error: {error:?}"
        );
        assert_eq!(state, before);
    }

    #[test]
    fn s1_a_distinguishes_missing_location_face_and_unknown_face() {
        let admission = admission();
        let state = successor_state_with_two_lands();
        let object_id = *state.predecessor_v5.zones.objects.keys().next().unwrap();

        let mut missing_location = state.clone();
        missing_location
            .predecessor_v5
            .zones
            .locations
            .remove(&object_id);
        assert!(matches!(
            crate::S1QueryAuthority::for_object(&admission, &missing_location, object_id),
            Err(crate::S1QueryError::MissingZoneLocation(id)) if id == object_id
        ));

        let mut mismatched_object_id = state.clone();
        mismatched_object_id
            .predecessor_v5
            .zones
            .objects
            .get_mut(&object_id)
            .unwrap()
            .id = mtgml_model::GameObjectId(object_id.0 + 1);
        assert!(matches!(
            crate::S1QueryAuthority::for_object(&admission, &mismatched_object_id, object_id),
            Err(crate::S1QueryError::InconsistentState(_))
        ));

        let mut missing_face = state.clone();
        missing_face.card_rules_state.faces.faces.remove(&object_id);
        assert!(matches!(
            crate::S1QueryAuthority::for_object(&admission, &missing_face, object_id),
            Err(crate::S1QueryError::FaceStateMissing(id)) if id == object_id
        ));

        let mut unknown_face = state;
        unknown_face
            .card_rules_state
            .faces
            .faces
            .insert(object_id, u32::MAX);
        let definition = unknown_face.predecessor_v5.zones.objects[&object_id].card_definition;
        assert!(matches!(
            crate::S1QueryAuthority::for_object(&admission, &unknown_face, object_id),
            Err(crate::S1QueryError::UnknownFace { definition: found, face_key })
                if found == definition && face_key.0 == u32::MAX
        ));
    }

    #[test]
    fn s1_a_distinguishes_unknown_stale_and_face_down_objects() {
        let admission = admission();
        let state = successor_state_with_two_lands();
        assert!(matches!(
            crate::S1QueryAuthority::for_object(
                &admission,
                &state,
                mtgml_model::GameObjectId(u64::MAX)
            ),
            Err(crate::S1QueryError::UnknownObject(
                mtgml_model::GameObjectId(u64::MAX)
            ))
        ));

        // The existing bounded PlayLand transition consumes the old incarnation
        // and allocates a new one, providing authoritative stale-ID evidence.
        let mut pending_state = state;
        let request = crate::install_basic_land_request_v4(
            &admission,
            &mut pending_state,
            PlayerId(1),
            &EpisodeStatus::Running,
        )
        .unwrap();
        let old_object = match request.candidates[1].trusted_binding {
            mtgml_decision::EngineCandidateBindingV4::PlayLand { object } => object,
            ref other => panic!("expected a PlayLand binding, got {other:?}"),
        };
        let response = mtgml_decision::DecisionResponseV3 {
            schema_version: mtgml_decision::DECISION_RESPONSE_V3_SCHEMA.to_owned(),
            player_decision_id: request.player_decision_id,
            view_sequence: request.view_sequence,
            answer: mtgml_decision::DecisionAnswerV2::SelectOne {
                candidate_id: mtgml_model::CandidateIdV1(1),
            },
        };
        let transition = crate::execute_basic_land_response_v4(
            &admission,
            &pending_state,
            PlayerId(1),
            &response,
            &EpisodeStatus::Running,
        )
        .unwrap();
        assert!(matches!(
            crate::S1QueryAuthority::for_object(&admission, &transition.next_state, old_object),
            Err(crate::S1QueryError::StaleObjectIncarnation(id)) if id == old_object
        ));

        let mut face_down = successor_state_with_two_lands();
        let object_id = *face_down
            .predecessor_v5
            .zones
            .objects
            .keys()
            .next()
            .unwrap();
        face_down
            .predecessor_v5
            .zones
            .objects
            .get_mut(&object_id)
            .unwrap()
            .face_down = true;
        assert!(matches!(
            crate::S1QueryAuthority::for_object(&admission, &face_down, object_id),
            Err(crate::S1QueryError::FaceDownCharacteristicsUnsupported(id)) if id == object_id
        ));
    }
}
