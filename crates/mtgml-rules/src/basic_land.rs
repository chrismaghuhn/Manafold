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
    pub next_state: EngineStatePartsV2,
    pub delta: StateDeltaV2,
    pub events: Vec<AuthoritativeRuleEventV2>,
    pub next_decision: Option<AuthoritativeDecisionRequestV3>,
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
    PerspectiveOccurrence(Box<PerspectiveLifecycleAuditV1>),
    ZoneTransition(Box<mtgml_state::ZoneTransition>),
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
    let old_revision = state.predecessor_v5.revision;
    let next_revision = mtgml_model::StateRevision(
        old_revision
            .0
            .checked_add(1)
            .ok_or(BasicLandTransitionError::IdentityExhausted)?,
    );
    let pending = state
        .execution_v3
        .pending_decision
        .as_ref()
        .ok_or(BasicLandTransitionError::InvalidSelection)?;
    let mut next = state.clone();
    next.predecessor_v5.revision = next_revision;
    next.execution_v3.pending_decision = None;
    let mut operations = vec![SemanticDeltaOperationV2::Existing {
        operation: Box::new(mtgml_state::SemanticDeltaOperation::DecisionCleared {
            decision: pending.decision_id,
        }),
    }];
    let mut events = Vec::new();
    push_successor_event(
        &mut next,
        &mut events,
        AuthoritativeRuleEventKindV2::DecisionCleared {
            decision: pending.decision_id,
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
                } if player == actor && actor == core.active_player => {
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
                    let next_request =
                        install_basic_land_request(admission, &mut next, other, status)
                            .map_err(|_| BasicLandTransitionError::InvalidResult)?;
                    operations.push(SemanticDeltaOperationV2::Existing {
                        operation: Box::new(mtgml_state::SemanticDeltaOperation::DecisionCreated {
                            decision: next_request.decision_id,
                        }),
                    });
                    push_successor_event(
                        &mut next,
                        &mut events,
                        AuthoritativeRuleEventKindV2::DecisionCreated {
                            decision: next_request.decision_id,
                        },
                    )?;
                }
                mtgml_state::PriorityState::HeldBy {
                    player,
                    consecutive_passes: 1,
                } if player == actor && actor != core.active_player => {
                    let old_position = core.position;
                    let new_position = crate::turn_structure::temporal_successor(old_position);
                    let to_priority = mtgml_state::PriorityState::None;
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
                        }
                    }
                }
                _ => return Err(BasicLandTransitionError::InvalidSelection),
            }
            // The second consecutive pass ends the current step. Forced
            // progress is an explicit subsequent kernel transition.
            let next_request = next.execution_v3.pending_decision.clone();
            next.validate()
                .map_err(|_| BasicLandTransitionError::InvalidResult)?;
            let delta = StateDeltaV2::between(state, &next, operations)
                .map_err(|_| BasicLandTransitionError::Delta)?;
            return Ok(BasicLandTransitionProductV1 {
                next_state: next,
                delta,
                events,
                next_decision: next_request,
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
            if profile_id.as_str() != BASIC_LAND_PROFILE_ID_V1 {
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
                    AuthoritativeRuleEventKindV2::PerspectiveOccurrence(Box::new(audit)),
                )?;
            }
            let next_event_id = next.predecessor_v5.allocators.next_rule_event_id;
            next = EngineStatePartsV2::from_state(&materialized, next.card_rules_state);
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
            let CardSemanticBindingV1::ProfiledV1 { profile_id, body } =
                &definition.semantic_binding
            else {
                return Err(BasicLandTransitionError::InvalidAbility);
            };
            if profile_id.as_str() != BASIC_LAND_PROFILE_ID_V1 {
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
                AuthoritativeRuleEventKindV2::ObjectTapped {
                    object: authority.source,
                    from: from_tapped,
                    to: true,
                },
            )?;
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
        }
    }

    // Candidate requests are state, not backend cache. Both bounded actions
    // leave priority with the actor, so the next exact request is installed
    // before the transaction can be committed.
    let next_request = install_basic_land_request(admission, &mut next, actor, status)
        .map_err(|_| BasicLandTransitionError::InvalidResult)?;
    operations.push(SemanticDeltaOperationV2::Existing {
        operation: Box::new(mtgml_state::SemanticDeltaOperation::DecisionCreated {
            decision: next_request.decision_id,
        }),
    });
    push_successor_event(
        &mut next,
        &mut events,
        AuthoritativeRuleEventKindV2::DecisionCreated {
            decision: next_request.decision_id,
        },
    )?;
    next.validate()
        .map_err(|_| BasicLandTransitionError::InvalidResult)?;
    let delta = StateDeltaV2::between(state, &next, operations)
        .map_err(|_| BasicLandTransitionError::Delta)?;
    Ok(BasicLandTransitionProductV1 {
        next_state: next,
        delta,
        events,
        next_decision: Some(next_request),
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
            && state.card_rules_state.faces.faces.get(object_id) == Some(&0)
        {
            public_object.ok_or(BasicLandCandidateError::InvalidState)?;
            for (ability_id, authority) in &state.card_rules_state.abilities.by_instance {
                if authority.source != *object_id || authority.ability_key != 0 {
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

    fn admission() -> ExecutableProfileAdmissionV1 {
        let id = calculate_content_contract_id_v1(MANIFEST).unwrap();
        let closure = [
            "rules/basic-land-mana",
            "rules/basic-priority",
            "rules/land-play",
            "rules/mana-pool",
            "rules/state-based-actions-combat",
            "rules/turn-structure",
            "rules/zone-incarnation",
        ]
        .into_iter()
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

    fn state_with_two_lands() -> EngineStatePartsV2 {
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
            mtgml_state::PriorityState::None
        );
        assert!(second.next_decision.is_none());
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
}
