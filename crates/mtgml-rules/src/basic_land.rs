//! The admitted, typed basic-land profile: its priority candidates, its
//! pending request, and the land-play and mana-ability transitions.
//!
//! This module consumes the same immutable admission token used to construct
//! the Magic kernel. It never dispatches on card names. Candidate intent
//! ordering and request-local IDs are owned by CandidateOrderingV3.

use mtgml_card_ir::{
    CardSemanticBindingV1, ExecutableProfileAdmissionV1, BASIC_LAND_PROFILE_ID_V1,
};
use mtgml_decision::{
    AuthoritativeCandidateV4, AuthoritativeDecisionRequestV4, CandidateIntentV4,
    CandidateOrderingV3, DecisionAnswerV2, DecisionDomainV2, DecisionPurposeV4, DecisionResponseV3,
    DecisionVisibility, EngineCandidateBindingV4,
};
use mtgml_model::{
    DecisionId, EpisodeStatus, ExecutionProgramV1, PlayerDecisionIdV1, PlayerId, RuleEventId,
    ZoneKind,
};
use mtgml_state::{
    apply_perspective_lifecycle, IdentityMutationV1, KnowledgeAcquisitionCause,
    KnowledgeAcquisitionReason, KnowledgeHistoryChannel, KnowledgeMutationV1, KnownLocationFactV2,
    PerspectiveLifecycleAuditV1, PerspectiveLifecycleMutationV1, SemanticDeltaOperationV3,
    VisibilityPartition, ZoneLocation, ZonePosition,
};
use mtgml_state::{EngineStatePartsV3, StateDeltaV3, TurnPosition};

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
/// selected candidate and a matching response.
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
    #[error("the turn rules cannot progress from this state")]
    TurnProgressUnsupported,
}

/// One rule event of a basic-land transition before it is numbered. An
/// observation occurrence names its public source event by list index.
enum DraftEvent {
    Kind(Box<crate::AuthoritativeRuleEventKindV3>),
    Occurrence {
        lifecycle: PerspectiveLifecycleAuditV1,
        source: usize,
    },
}

/// The next state, the delta operations in their final order, and the rule
/// events in their final order, not yet numbered.
struct BasicLandDraft {
    next_state: EngineStatePartsV3,
    operations: Vec<SemanticDeltaOperationV3>,
    events: Vec<DraftEvent>,
}

fn apply_lifecycle(
    state: &mut EngineStatePartsV3,
    audit: &PerspectiveLifecycleAuditV1,
) -> Result<(), BasicLandTransitionError> {
    let mut engine: mtgml_state::EngineState = state.predecessor_v5.clone().into();
    apply_perspective_lifecycle(&mut engine, audit)
        .map_err(|_| BasicLandTransitionError::InvalidResult)?;
    state.predecessor_v5 = engine.parts();
    Ok(())
}

/// Every perspective observes the public event at `source`: each visible
/// cursor advances, in `knowledge.players` order.
fn push_visible_occurrences(
    state: &mut EngineStatePartsV3,
    events: &mut Vec<DraftEvent>,
    operations: &mut Vec<SemanticDeltaOperationV3>,
    source: usize,
) -> Result<(), BasicLandTransitionError> {
    let perspectives: Vec<_> = state
        .predecessor_v5
        .knowledge
        .players
        .keys()
        .copied()
        .collect();
    for perspective in perspectives {
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
        apply_lifecycle(state, &audit)?;
        operations.push(SemanticDeltaOperationV3::PerspectiveLifecycle {
            lifecycle: audit.clone(),
        });
        events.push(DraftEvent::Occurrence {
            lifecycle: audit,
            source,
        });
    }
    Ok(())
}

/// Applies the selected action to the candidate state `state`: validated,
/// without a pending request, with an empty execution.
fn draft_basic_land_action(
    admission: &ExecutableProfileAdmissionV1,
    state: &EngineStatePartsV3,
    pending_decision_id: mtgml_model::DecisionId,
    decision: SelectedSuccessorDecisionV1,
) -> Result<BasicLandDraft, BasicLandTransitionError> {
    let old_revision = state.predecessor_v5.revision;
    let next_revision = mtgml_model::StateRevision(
        old_revision
            .0
            .checked_add(1)
            .ok_or(BasicLandTransitionError::IdentityExhausted)?,
    );
    let mut next = state.clone();
    next.predecessor_v5.revision = next_revision;
    next.execution_v4.pending_decision = None;
    let mut operations = vec![SemanticDeltaOperationV3::DecisionCleared {
        decision: pending_decision_id,
    }];
    let mut events = vec![DraftEvent::Kind(Box::new(
        crate::AuthoritativeRuleEventKindV3::DecisionCleared {
            decision: pending_decision_id,
        },
    ))];

    match decision {
        // Passing priority is resolved by the turn progression, never here.
        SelectedSuccessorDecisionV1::PassPriority => {
            return Err(BasicLandTransitionError::InvalidSelection);
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
                    operations.push(SemanticDeltaOperationV3::AbilityAuthorityRemoved {
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

            let mut materialized: mtgml_state::EngineState = next.predecessor_v5.clone().into();
            let mut occurrences = Vec::new();
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
                operations.push(SemanticDeltaOperationV3::PerspectiveLifecycle {
                    lifecycle: audit.clone(),
                });
                // The occurrence follows the zone transition it observes.
                occurrences.push(audit);
            }
            next.predecessor_v5 = materialized.parts();
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
                        operations.push(SemanticDeltaOperationV3::AbilityIdentityChanged {
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
            operations.push(SemanticDeltaOperationV3::AbilityAuthorityAdded {
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
                operations.push(SemanticDeltaOperationV3::AbilityIdentityChanged {
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
            operations.push(SemanticDeltaOperationV3::ObjectEntered {
                old_object: Some(object),
                new_object,
                from_zone: from.zone,
                to_zone: to.zone,
                tapped: false,
                face: 0,
            });
            operations.push(SemanticDeltaOperationV3::LandPlayCountChanged {
                player: actor,
                from: 0,
                to: 1,
            });
            let source = events.len();
            events.push(DraftEvent::Kind(Box::new(
                crate::AuthoritativeRuleEventKindV3::ZoneTransition {
                    transition: Box::new(transition),
                },
            )));
            events.extend(
                occurrences
                    .into_iter()
                    .map(|lifecycle| DraftEvent::Occurrence { lifecycle, source }),
            );
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
            operations.push(SemanticDeltaOperationV3::ObjectTapped {
                object: authority.source,
                from: from_tapped,
                to: true,
            });
            let tapped_event = events.len();
            events.push(DraftEvent::Kind(Box::new(
                crate::AuthoritativeRuleEventKindV3::ObjectTapped {
                    object: authority.source,
                    from: from_tapped,
                    to: true,
                },
            )));
            push_visible_occurrences(&mut next, &mut events, &mut operations, tapped_event)?;
            operations.push(SemanticDeltaOperationV3::ManaPoolChanged {
                player: actor,
                from: pool_before,
                to: pool_after,
                cause: mtgml_state::ManaPoolChangeCauseV1::Produced,
            });
            let mana_event = events.len();
            events.push(DraftEvent::Kind(Box::new(
                crate::AuthoritativeRuleEventKindV3::ManaPoolChanged {
                    player: actor,
                    before: pool_before,
                    after: pool_after,
                    cause: mtgml_state::ManaPoolChangeCauseV1::Produced,
                },
            )));
            push_visible_occurrences(&mut next, &mut events, &mut operations, mana_event)?;
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
            operations.push(SemanticDeltaOperationV3::PriorityChanged {
                from: from_priority,
                to: to_priority,
            });
            events.push(DraftEvent::Kind(Box::new(
                crate::AuthoritativeRuleEventKindV3::PriorityChanged {
                    from: from_priority,
                    to: to_priority,
                },
            )));
        }
    }

    next.validate_structure()
        .map_err(|_| BasicLandTransitionError::InvalidResult)?;
    Ok(BasicLandDraft {
        next_state: next,
        operations,
        events,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasicLandTransitionProductV4 {
    pub accepted: bool,
    pub next_state: EngineStatePartsV3,
    pub delta: StateDeltaV3,
    pub events: Vec<crate::AuthoritativeRuleEventV3>,
    pub next_decision: Option<AuthoritativeDecisionRequestV4>,
    pub status: EpisodeStatus,
}

/// The state candidates are derived from: the current state without its
/// pending request. The bounded profile owns no stack, continuation, effect or
/// trigger state.
fn candidate_state(
    state: &EngineStatePartsV3,
) -> Result<EngineStatePartsV3, BasicLandCandidateError> {
    let mut state = state.clone();
    state.execution_v4.pending_decision = None;
    state
        .validate_structure()
        .map_err(|_| BasicLandCandidateError::InvalidState)?;
    if !state.execution_v4.continuations.is_empty()
        || !state.execution_v4.effects.is_empty()
        || !state.execution_v4.waiting_triggers.is_empty()
        || !state.execution_v4.delayed_effects.is_empty()
        || state
            .predecessor_v5
            .zones
            .stack_records
            .values()
            .any(|record| record.payload.is_some())
    {
        return Err(BasicLandCandidateError::InvalidState);
    }
    Ok(state)
}

/// Derive the complete bounded PlayLand and intrinsic basic-land mana
/// ability candidate surface for `actor`. This performs no state mutation,
/// response selection, or execution. Hidden or unmapped identities fail
/// closed rather than receiving placeholder public IDs.
pub fn derive_basic_land_candidates_v4(
    admission: &ExecutableProfileAdmissionV1,
    state: &EngineStatePartsV3,
    actor: PlayerId,
    status: &EpisodeStatus,
) -> Result<Vec<AuthoritativeCandidateV4>, BasicLandCandidateError> {
    let state = &candidate_state(state)?;
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
    if !matches!(status, EpisodeStatus::Running) {
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
            CandidateIntentV4::PassPriority,
            EngineCandidateBindingV4::PassPriority,
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
                CandidateIntentV4::PlayLand { object: opaque },
                EngineCandidateBindingV4::PlayLand { object: *object_id },
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
                    CandidateIntentV4::ActivateAbility {
                        ability: opaque_ability,
                    },
                    EngineCandidateBindingV4::ActivateAbility {
                        ability: *ability_id,
                    },
                ));
            }
        }
    }
    let candidates = CandidateOrderingV3::assign_dense(raw)
        .map_err(|_| BasicLandCandidateError::CandidateOrdering)?;
    let live: std::collections::BTreeSet<_> =
        state.predecessor_v5.zones.objects.keys().copied().collect();
    if candidates
        .iter()
        .any(|candidate| match &candidate.trusted_binding {
            EngineCandidateBindingV4::PassPriority => false,
            EngineCandidateBindingV4::PlayLand { object } => !live.contains(object),
            EngineCandidateBindingV4::ActivateAbility { ability } => !state
                .card_rules_state
                .abilities
                .by_instance
                .contains_key(ability),
            _ => true,
        })
    {
        return Err(BasicLandCandidateError::InvalidState);
    }
    let mut objects = Vec::new();
    for candidate in &candidates {
        match &candidate.trusted_binding {
            EngineCandidateBindingV4::PlayLand { object } => objects.push(*object),
            EngineCandidateBindingV4::ActivateAbility { ability } => objects.push(
                state
                    .card_rules_state
                    .abilities
                    .by_instance
                    .get(ability)
                    .ok_or(BasicLandCandidateError::InvalidState)?
                    .source,
            ),
            _ => {}
        }
    }
    let authorities = crate::S1QueryAuthority::for_objects(admission, state, &objects)
        .map_err(map_s1_query_error)?;
    for (authority, object) in authorities.iter().zip(&objects) {
        if authority.queried_object().object != *object
            || authority.execution_identity().program_kind != ExecutionProgramV1::MagicRules
        {
            return Err(BasicLandCandidateError::WrongExecutionIdentity);
        }
    }
    Ok(candidates)
}

fn map_s1_query_error(error: crate::S1QueryError) -> BasicLandCandidateError {
    match error {
        crate::S1QueryError::MissingCardDefinition(_)
        | crate::S1QueryError::ContentContractMismatch
        | crate::S1QueryError::ProfileNotAdmitted
        | crate::S1QueryError::UnknownFace { .. }
        | crate::S1QueryError::FaceStateMissing(_) => BasicLandCandidateError::InvalidDefinition,
        crate::S1QueryError::UnknownObject(_)
        | crate::S1QueryError::StaleObjectIncarnation(_)
        | crate::S1QueryError::MissingZoneLocation(_)
        | crate::S1QueryError::FaceDownCharacteristicsUnsupported(_)
        | crate::S1QueryError::UnsupportedCharacteristic(_)
        | crate::S1QueryError::UnsupportedContributor(_)
        | crate::S1QueryError::InvalidAttachmentReference(_)
        | crate::S1QueryError::InvalidCounterState(_)
        | crate::S1QueryError::InconsistentState(_)
        | crate::S1QueryError::ArithmeticOverflow => BasicLandCandidateError::InvalidState,
    }
}

pub fn validate_basic_land_pending_request_v4(
    admission: &ExecutableProfileAdmissionV1,
    state: &EngineStatePartsV3,
    status: &EpisodeStatus,
) -> Result<(), BasicLandCandidateError> {
    state
        .validate_structure()
        .map_err(|_| BasicLandCandidateError::InvalidState)?;
    let Some(request) = state.execution_v4.pending_decision.as_ref() else {
        // Closed episodes have no pending request, but they remain subject to
        // the same Basic-Land profile state boundary. Do not let terminal or
        // truncated checkpoints bypass rejection of stack/effect/trigger
        // state by taking the no-request fast path.
        candidate_state(state)?;
        return match status {
            EpisodeStatus::Terminal { .. } | EpisodeStatus::Truncated { .. } => Ok(()),
            EpisodeStatus::Running => Err(BasicLandCandidateError::PendingCandidateSetMismatch),
        };
    };
    request
        .project_player_request()
        .map_err(|_| BasicLandCandidateError::PendingCandidateSetMismatch)?;
    let expected = derive_basic_land_candidates_v4(admission, state, request.actor, status)?;
    let view_sequence = state
        .predecessor_v5
        .knowledge
        .players
        .get(&request.actor)
        .ok_or(BasicLandCandidateError::InvalidState)?
        .next_visible_sequence;
    let identity = state
        .predecessor_v5
        .perspective_identities
        .players
        .get(&request.actor)
        .ok_or(BasicLandCandidateError::InvalidState)?;
    if request.decision_id.0.checked_add(1)
        != Some(state.predecessor_v5.allocators.next_decision_id.0)
        || request.player_decision_id.0.checked_add(1) != Some(identity.next_player_decision_id.0)
        || request.state_revision != state.predecessor_v5.revision
        || request.view_sequence != view_sequence
        || request.visibility != DecisionVisibility::Public
        || request.decision_domain_v2 != DecisionDomainV2::ChooseOne
        || request.purpose != DecisionPurposeV4::PriorityAction
        || request.parent_player_decision_id.is_some()
        || request.continuation_id.is_some()
        || request.candidates != expected
        || !matches!(status, EpisodeStatus::Running)
    {
        return Err(BasicLandCandidateError::PendingCandidateSetMismatch);
    }
    Ok(())
}

pub fn install_basic_land_request_v4(
    admission: &ExecutableProfileAdmissionV1,
    state: &mut EngineStatePartsV3,
    actor: PlayerId,
    status: &EpisodeStatus,
) -> Result<AuthoritativeDecisionRequestV4, BasicLandCandidateError> {
    if state.execution_v4.pending_decision.is_some() {
        return Err(BasicLandCandidateError::InvalidState);
    }
    let candidates = derive_basic_land_candidates_v4(admission, state, actor, status)?;
    if candidates.is_empty() {
        return Err(BasicLandCandidateError::NoCandidates);
    }
    let view_sequence = state
        .predecessor_v5
        .knowledge
        .players
        .get(&actor)
        .ok_or(BasicLandCandidateError::InvalidState)?
        .next_visible_sequence;
    let identity = state
        .predecessor_v5
        .perspective_identities
        .players
        .get(&actor)
        .ok_or(BasicLandCandidateError::InvalidState)?;
    let decision_id = state.predecessor_v5.allocators.next_decision_id;
    let player_decision_id = identity.next_player_decision_id;
    let next_decision_id = DecisionId(
        decision_id
            .0
            .checked_add(1)
            .ok_or(BasicLandCandidateError::IdentityExhausted)?,
    );
    let next_player_decision_id = PlayerDecisionIdV1(
        player_decision_id
            .0
            .checked_add(1)
            .ok_or(BasicLandCandidateError::IdentityExhausted)?,
    );
    let request = AuthoritativeDecisionRequestV4 {
        decision_id,
        player_decision_id,
        state_revision: state.predecessor_v5.revision,
        view_sequence,
        actor,
        visibility: DecisionVisibility::Public,
        decision_domain_v2: DecisionDomainV2::ChooseOne,
        purpose: DecisionPurposeV4::PriorityAction,
        parent_player_decision_id: None,
        continuation_id: None,
        candidates,
    };
    request
        .project_player_request()
        .map_err(|_| BasicLandCandidateError::CandidateOrdering)?;

    let mut candidate_state = state.clone();
    candidate_state.predecessor_v5.allocators.next_decision_id = next_decision_id;
    candidate_state
        .predecessor_v5
        .perspective_identities
        .players
        .get_mut(&actor)
        .ok_or(BasicLandCandidateError::InvalidState)?
        .next_player_decision_id = next_player_decision_id;
    candidate_state.execution_v4.pending_decision = Some(request.clone());
    candidate_state
        .validate_structure()
        .map_err(|_| BasicLandCandidateError::InvalidState)?;
    *state = candidate_state;
    Ok(request)
}

pub fn selected_basic_land_action_v4(
    admission: &ExecutableProfileAdmissionV1,
    state: &EngineStatePartsV3,
    actor: PlayerId,
    response: &DecisionResponseV3,
    status: &EpisodeStatus,
) -> Result<SelectedSuccessorDecisionV1, BasicLandCandidateError> {
    validate_basic_land_pending_request_v4(admission, state, status)?;
    let request = state
        .execution_v4
        .pending_decision
        .as_ref()
        .ok_or(BasicLandCandidateError::InvalidState)?;
    if request.actor != actor {
        return Err(BasicLandCandidateError::InvalidState);
    }
    request
        .validate_response(response)
        .map_err(|_| BasicLandCandidateError::InvalidState)?;
    let selected_ids = match &response.answer {
        DecisionAnswerV2::SelectOne { candidate_id } => vec![*candidate_id],
        DecisionAnswerV2::SelectMany { candidate_ids }
        | DecisionAnswerV2::Order { candidate_ids } => candidate_ids.clone(),
        DecisionAnswerV2::ChooseNumber { .. } => Vec::new(),
    };
    let selected = selected_ids
        .iter()
        .map(|candidate_id| {
            request
                .candidates
                .iter()
                .find(|candidate| candidate.candidate_id == *candidate_id)
                .map(|candidate| &candidate.trusted_binding)
                .ok_or(BasicLandCandidateError::InvalidState)
        })
        .collect::<Result<Vec<_>, _>>()?;
    if selected.len() != 1 {
        return Err(BasicLandCandidateError::UnsupportedSelectedAction);
    }
    match selected[0] {
        EngineCandidateBindingV4::PassPriority => Ok(SelectedSuccessorDecisionV1::PassPriority),
        EngineCandidateBindingV4::PlayLand { object } => Ok(
            SelectedSuccessorDecisionV1::MagicAction(MagicActionRequestV1::PlayLand {
                actor,
                object: *object,
            }),
        ),
        EngineCandidateBindingV4::ActivateAbility { ability } => Ok(
            SelectedSuccessorDecisionV1::MagicAction(MagicActionRequestV1::ActivateManaAbility {
                actor,
                ability: *ability,
            }),
        ),
        _ => Err(BasicLandCandidateError::UnsupportedSelectedAction),
    }
}

pub fn execute_basic_land_response_v4(
    admission: &ExecutableProfileAdmissionV1,
    state: &EngineStatePartsV3,
    actor: PlayerId,
    response: &DecisionResponseV3,
    status: &EpisodeStatus,
) -> Result<BasicLandTransitionProductV4, BasicLandTransitionError> {
    let request = state
        .execution_v4
        .pending_decision
        .as_ref()
        .ok_or(BasicLandTransitionError::InvalidSelection)?;
    validate_basic_land_pending_request_v4(admission, state, status)
        .map_err(|_| BasicLandTransitionError::InvalidSelection)?;
    request
        .validate_response(response)
        .map_err(|_| BasicLandTransitionError::InvalidSelection)?;

    let before = candidate_state(state).map_err(|_| BasicLandTransitionError::InvalidResult)?;
    let selected = selected_basic_land_action_v4(admission, state, actor, response, status)
        .map_err(|_| BasicLandTransitionError::InvalidSelection)?;
    let BasicLandDraft {
        mut next_state,
        mut operations,
        events: draft_events,
    } = draft_basic_land_action(admission, &before, request.decision_id, selected)?;

    // Number the events from the state's cursor, in their final order.
    let first = state.predecessor_v5.allocators.next_rule_event_id;
    let event_id = |index: usize| {
        u64::try_from(index)
            .ok()
            .and_then(|index| first.0.checked_add(index))
            .map(RuleEventId)
            .ok_or(BasicLandTransitionError::IdentityExhausted)
    };
    let revision = next_state.predecessor_v5.revision;
    let mut events = Vec::with_capacity(draft_events.len() + 1);
    for (index, draft) in draft_events.into_iter().enumerate() {
        let event = match draft {
            DraftEvent::Kind(kind) => *kind,
            DraftEvent::Occurrence { lifecycle, source } => {
                crate::AuthoritativeRuleEventKindV3::PerspectiveObservationOccurrence {
                    lifecycle: Box::new(lifecycle),
                    source_event_id: event_id(source)?,
                }
            }
        };
        events.push(crate::AuthoritativeRuleEventV3 {
            event_id: event_id(index)?,
            state_revision: revision,
            event,
        });
    }
    next_state.predecessor_v5.allocators.next_rule_event_id = event_id(events.len())?;

    let next_request = install_basic_land_request_v4(admission, &mut next_state, actor, status)
        .map_err(|_| BasicLandTransitionError::InvalidResult)?;
    let decision_event_id = next_state.predecessor_v5.allocators.next_rule_event_id;
    next_state.predecessor_v5.allocators.next_rule_event_id = RuleEventId(
        decision_event_id
            .0
            .checked_add(1)
            .ok_or(BasicLandTransitionError::IdentityExhausted)?,
    );
    let decision_created = crate::AuthoritativeRuleEventV3 {
        event_id: decision_event_id,
        state_revision: revision,
        event: crate::AuthoritativeRuleEventKindV3::DecisionCreated {
            decision: next_request.decision_id,
        },
    };
    events.push(decision_created.clone());
    operations.push(SemanticDeltaOperationV3::PendingRequestChanged {
        from: Some(Box::new(request.clone())),
        to: Some(Box::new(next_request.clone())),
    });
    operations.extend(decision_created.event.semantic_operations());
    next_state
        .validate_structure()
        .map_err(|_| BasicLandTransitionError::InvalidResult)?;
    let delta = StateDeltaV3::between_structural_only(state, &next_state, operations)
        .map_err(|_| BasicLandTransitionError::Delta)?;
    crate::events_v3::validate_events_for_built_delta_v3(state, &next_state, &events, &delta)
        .map_err(|_| BasicLandTransitionError::InvalidResult)?;
    Ok(BasicLandTransitionProductV4 {
        accepted: true,
        next_state,
        delta,
        events,
        next_decision: Some(next_request),
        status: status.clone(),
    })
}

#[cfg(test)]
pub(crate) fn s1_b_state_with_two_lands_fixture() -> mtgml_state::EngineStatePartsV2 {
    tests::state_with_two_lands()
}

#[cfg(test)]
pub(crate) fn basic_land_admission_fixture() -> mtgml_card_ir::ExecutableProfileAdmissionV1 {
    tests::admission()
}

#[cfg(test)]
mod tests {
    use super::*;
    use mtgml_card_ir::{
        admit_executable_profile_v1, decode_content_manifest_v1, ExecutableProfileAdmissionV1,
    };
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
    use mtgml_state::EngineStatePartsV2;
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

    /// The current-format state around a fixture.
    fn current(state: EngineStatePartsV2) -> EngineStatePartsV3 {
        EngineStatePartsV3::new(
            state.predecessor_v5,
            Default::default(),
            state.card_rules_state,
        )
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
        let candidates = derive_basic_land_candidates_v4(
            &admission,
            &current(state),
            PlayerId(1),
            &EpisodeStatus::Running,
        )
        .unwrap();
        let intents = candidates
            .iter()
            .map(|candidate| candidate.visible_intent.clone())
            .collect::<Vec<_>>();
        assert_eq!(intents.len(), 4);
        assert!(matches!(intents[0], CandidateIntentV4::PassPriority));
        assert!(matches!(intents[1], CandidateIntentV4::PlayLand { .. }));
        assert!(matches!(intents[2], CandidateIntentV4::PlayLand { .. }));
        assert!(matches!(
            intents[3],
            CandidateIntentV4::ActivateAbility { .. }
        ));
        let visible = intents
            .iter()
            .enumerate()
            .map(|(index, intent)| mtgml_decision::VisibleCandidateV4 {
                candidate_id: mtgml_model::CandidateIdV1(index as u32),
                intent: intent.clone(),
            })
            .collect::<Vec<_>>();
        CandidateOrderingV3::validate_public(&visible).unwrap();
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
    fn illegal_play_land_windows_produce_no_land_candidates() {
        let admission = admission();
        let mut state = state_with_two_lands();
        state.predecessor_v5.core.position = TurnPosition::Beginning {
            step: mtgml_state::BeginningStep::Upkeep,
        };
        let candidates = derive_basic_land_candidates_v4(
            &admission,
            &current(state.clone()),
            PlayerId(1),
            &EpisodeStatus::Running,
        )
        .unwrap();
        assert!(candidates.iter().all(|candidate| !matches!(
            candidate.visible_intent,
            CandidateIntentV4::PlayLand { .. }
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
        let candidates = derive_basic_land_candidates_v4(
            &admission,
            &current(state.clone()),
            PlayerId(1),
            &EpisodeStatus::Running,
        )
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
        let candidates = derive_basic_land_candidates_v4(
            &admission,
            &current(state.clone()),
            PlayerId(1),
            &EpisodeStatus::Running,
        )
        .unwrap();
        assert!(candidates.iter().all(|candidate| !matches!(
            candidate.visible_intent,
            CandidateIntentV4::PlayLand { .. }
        )));
        assert!(candidates.iter().any(|candidate| matches!(
            candidate.visible_intent,
            CandidateIntentV4::ActivateAbility { .. }
        )));
        assert!(candidates
            .iter()
            .any(|candidate| matches!(candidate.visible_intent, CandidateIntentV4::PassPriority)));
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
