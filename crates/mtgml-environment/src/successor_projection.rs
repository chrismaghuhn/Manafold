//! Perspective projection for authoritative successor transition products.

use std::collections::{BTreeMap, BTreeSet};

use mtgml_model::{OpaqueObjectId, PlayerId, VisibleSequence};
use mtgml_observation::{
    ManaPoolAfterV1, ObservedEventEnvelopeV4, ObservedEventKindV4, ObservedFaceV1,
    PlayerStepSubmissionV1, PlayerSubmissionCodeV1,
};
use mtgml_rules::AuthoritativeRuleEventKind;
use mtgml_state::{EngineState, PerspectiveIdentityRecordV2};

use crate::errors::ControllerError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SuccessorProjectionError {
    #[error("successor event references an unknown perspective")]
    UnknownPerspective,
    #[error("successor event sequence does not continue the perspective cursor")]
    CursorMismatch,
    #[error("authorized object has no perspective opaque identity")]
    MissingOpaqueIdentity,
    #[error("after-state visible sequence differs from projected events")]
    FinalCursorMismatch,
    #[error("battlefield-entry event face/tapped facts disagree with authoritative state")]
    BattlefieldEntryFactsMismatch,
    #[error("successor event has no safe public projection")]
    UnsupportedObservedEvent,
    #[error("public observation occurrence does not bind a prior authoritative event")]
    ObservationOccurrenceMismatch,
    #[error("event-sequenced stack order does not match the authoritative after-state")]
    FinalStackOrderMismatch,
}

pub struct SuccessorProjectionAuthority<'a> {
    pub execution_identity: &'a mtgml_model::ExecutionIdentityV1,
    pub semantic_manifest: &'a mtgml_model::SemanticContractManifestV1,
    pub rules_manifest: &'a mtgml_model::RulesContractManifestV1,
    pub catalog: &'a mtgml_card_ir::VerifiedContentCatalogV1,
    pub basic_land_admission: Option<&'a mtgml_card_ir::ExecutableProfileAdmissionV1>,
}

pub struct SuccessorTransitionV4Projection<'a> {
    pub before: &'a EngineState,
    pub after: &'a EngineState,
    pub before_status: &'a mtgml_model::EpisodeStatus,
    pub events: &'a [mtgml_rules::AuthoritativeRuleEvent],
    pub delta: Option<&'a mtgml_state::StateDelta>,
    pub accepted: bool,
    pub status: &'a mtgml_model::EpisodeStatus,
    pub next_request: Option<&'a mtgml_decision::AuthoritativeDecisionRequest>,
    pub actor: PlayerId,
    pub rejected_code: PlayerSubmissionCodeV1,
}

struct V4PublicSourceEventContext<'a> {
    before: &'a EngineState,
    after: &'a EngineState,
    perspective: PlayerId,
    before_identity: &'a PerspectiveIdentityRecordV2,
    after_identity: &'a PerspectiveIdentityRecordV2,
    knowledge: &'a mtgml_state::PlayerKnowledgeStateV2,
    stack_order_before: &'a [mtgml_model::StackObjectId],
}

fn resolve(
    authorized: bool,
    object: mtgml_model::GameObjectId,
    identity: &PerspectiveIdentityRecordV2,
) -> Result<Option<OpaqueObjectId>, SuccessorProjectionError> {
    if !authorized {
        return Ok(None);
    }
    identity
        .object_to_opaque
        .get(&object)
        .copied()
        .map(Some)
        .ok_or(SuccessorProjectionError::MissingOpaqueIdentity)
}

/// Projects Rules-authored successor observation occurrences into the
/// revision-free V4 player event product. StateRevision never appears in the
/// result; only the perspective's already-authoritative visible sequence is
/// used for public chronology.
pub fn project_successor_events_v4(
    before: &EngineState,
    after: &EngineState,
    events: &[mtgml_rules::AuthoritativeRuleEvent],
) -> Result<BTreeMap<PlayerId, Vec<ObservedEventEnvelopeV4>>, SuccessorProjectionError> {
    project_successor_events_v4_inner(before, after, events, None, false)
}

pub fn project_successor_events_v4_for_basic_land_profile(
    admission: &mtgml_card_ir::ExecutableProfileAdmissionV1,
    before: &EngineState,
    before_status: &mtgml_model::EpisodeStatus,
    after: &EngineState,
    after_status: &mtgml_model::EpisodeStatus,
    events: &[mtgml_rules::AuthoritativeRuleEvent],
    delta: Option<&mtgml_state::StateDelta>,
) -> Result<BTreeMap<PlayerId, Vec<ObservedEventEnvelopeV4>>, SuccessorProjectionError> {
    mtgml_rules::validate_magic_pending_request(admission, before, before_status)
        .map_err(|_| SuccessorProjectionError::ObservationOccurrenceMismatch)?;
    mtgml_rules::validate_magic_pending_request(admission, after, after_status)
        .map_err(|_| SuccessorProjectionError::ObservationOccurrenceMismatch)?;
    project_successor_events_v4_inner(before, after, events, delta, true)
}

fn project_successor_events_v4_inner(
    before: &EngineState,
    after: &EngineState,
    events: &[mtgml_rules::AuthoritativeRuleEvent],
    delta: Option<&mtgml_state::StateDelta>,
    rules_domain_validated: bool,
) -> Result<BTreeMap<PlayerId, Vec<ObservedEventEnvelopeV4>>, SuccessorProjectionError> {
    let state_validation = if rules_domain_validated {
        before.validate_structure().and(after.validate_structure())
    } else {
        before.validate().and(after.validate())
    };
    state_validation.map_err(|_| SuccessorProjectionError::ObservationOccurrenceMismatch)?;
    mtgml_rules::validate_rule_event_cursor(
        before.allocators.next_rule_event_id,
        after.allocators.next_rule_event_id,
        after.revision,
        events,
    )
    .map_err(|_| SuccessorProjectionError::ObservationOccurrenceMismatch)?;
    let before_engine = before;
    let after_engine = after;
    let mut projected: BTreeMap<_, Vec<_>> = before_engine
        .knowledge
        .players
        .keys()
        .copied()
        .map(|player| (player, Vec::new()))
        .collect();
    let player_set: BTreeSet<_> = projected.keys().copied().collect();
    let mut observed_source_audiences: BTreeMap<mtgml_model::RuleEventId, BTreeSet<PlayerId>> =
        BTreeMap::new();
    let mut cursors: BTreeMap<_, _> = before_engine
        .knowledge
        .players
        .iter()
        .map(|(player, state)| (*player, state.next_visible_sequence.0))
        .collect();
    // Keep event-time player authority separate from the fully materialized
    // after-state. A fact learned by a later occurrence in this same atomic
    // transition must not authorize an earlier event projection.
    let mut identities = before_engine.perspective_identities.players.clone();
    let mut knowledge_states = before_engine.knowledge.players.clone();
    let mut working_stack_order = before.zones.stack_order.clone();
    let mut stack_order_before_event = BTreeMap::new();
    for event in events {
        match &event.event {
            AuthoritativeRuleEventKind::StackItemAdded { stack_object, .. } => {
                stack_order_before_event.insert(event.event_id, working_stack_order.clone());
                if working_stack_order.contains(stack_object) {
                    return Err(SuccessorProjectionError::ObservationOccurrenceMismatch);
                }
                working_stack_order.push(*stack_object);
            }
            AuthoritativeRuleEventKind::TriggerPlaced { stack_object, .. } => {
                stack_order_before_event.insert(event.event_id, working_stack_order.clone());
                if working_stack_order.contains(stack_object) {
                    return Err(SuccessorProjectionError::ObservationOccurrenceMismatch);
                }
                working_stack_order.push(*stack_object);
            }
            AuthoritativeRuleEventKind::StackItemRemoved { stack_object, .. } => {
                stack_order_before_event.insert(event.event_id, working_stack_order.clone());
                let Some(index) = working_stack_order
                    .iter()
                    .position(|candidate| candidate == stack_object)
                else {
                    return Err(SuccessorProjectionError::ObservationOccurrenceMismatch);
                };
                working_stack_order.remove(index);
            }
            _ => {}
        }
    }
    if working_stack_order != after.zones.stack_order {
        return Err(SuccessorProjectionError::FinalStackOrderMismatch);
    }

    for (index, event) in events.iter().enumerate() {
        let AuthoritativeRuleEventKind::PerspectiveObservationOccurrence {
            lifecycle,
            source_event_id,
        } = &event.event
        else {
            continue;
        };
        let (lifecycle, source_event_id) = (lifecycle.as_ref(), *source_event_id);
        let cursor = cursors
            .get_mut(&lifecycle.perspective)
            .ok_or(SuccessorProjectionError::UnknownPerspective)?;
        if *cursor != lifecycle.sequence.0 {
            return Err(SuccessorProjectionError::CursorMismatch);
        }
        *cursor = cursor
            .checked_add(1)
            .ok_or(SuccessorProjectionError::CursorMismatch)?;

        let before_identity = identities
            .get(&lifecycle.perspective)
            .ok_or(SuccessorProjectionError::UnknownPerspective)?
            .clone();
        let after_identity = identities
            .get_mut(&lifecycle.perspective)
            .ok_or(SuccessorProjectionError::UnknownPerspective)?;
        let after_knowledge = knowledge_states
            .get_mut(&lifecycle.perspective)
            .ok_or(SuccessorProjectionError::UnknownPerspective)?;
        // Apply each trusted lifecycle occurrence to the projection cursor.
        // This validates its sequence and advances both the identity allocator
        // and knowledge record exactly where the occurrence appears.
        mtgml_state::apply_lifecycle_to_player(
            after_knowledge,
            after_identity,
            &|object| {
                before_engine.zones.objects.contains_key(&object)
                    || after_engine.zones.objects.contains_key(&object)
            },
            lifecycle,
        )
        .map_err(|_| SuccessorProjectionError::ObservationOccurrenceMismatch)?;
        let after_knowledge = after_knowledge.clone();

        let observation = {
            let source_index = events
                .iter()
                .position(|candidate| candidate.event_id == source_event_id)
                .filter(|source_index| *source_index < index)
                .ok_or(SuccessorProjectionError::ObservationOccurrenceMismatch)?;
            let source = &events[source_index];
            let stack_order = stack_order_before_event
                .get(&source_event_id)
                .cloned()
                .unwrap_or_else(|| working_stack_order.clone());
            Some(project_v4_public_source_event(
                source,
                V4PublicSourceEventContext {
                    before,
                    after,
                    perspective: lifecycle.perspective,
                    before_identity: &before_identity,
                    after_identity,
                    knowledge: &after_knowledge,
                    stack_order_before: &stack_order,
                },
            )?)
        };
        if let Some(event_kind) = observation {
            let envelope = ObservedEventEnvelopeV4 {
                schema_version: mtgml_observation::OBSERVED_EVENT_SCHEMA_V4.into(),
                sequence: VisibleSequence(lifecycle.sequence.0),
                event: event_kind,
            };
            envelope
                .validate()
                .map_err(|_| SuccessorProjectionError::ObservationOccurrenceMismatch)?;
            projected
                .get_mut(&lifecycle.perspective)
                .ok_or(SuccessorProjectionError::UnknownPerspective)?
                .push(envelope);
            if !observed_source_audiences
                .entry(source_event_id)
                .or_default()
                .insert(lifecycle.perspective)
            {
                return Err(SuccessorProjectionError::ObservationOccurrenceMismatch);
            }
        }
    }
    if let Some(delta) = delta {
        apply_projection_identity_delta(&mut identities, delta)?;
    }
    if identities != after_engine.perspective_identities.players {
        return Err(SuccessorProjectionError::FinalCursorMismatch);
    }
    if knowledge_states != after_engine.knowledge.players {
        return Err(SuccessorProjectionError::FinalCursorMismatch);
    }
    for (player, next_sequence) in cursors {
        let actual = after_engine
            .knowledge
            .players
            .get(&player)
            .ok_or(SuccessorProjectionError::UnknownPerspective)?
            .next_visible_sequence
            .0;
        if actual != next_sequence {
            return Err(SuccessorProjectionError::FinalCursorMismatch);
        }
    }
    for event in events {
        if requires_all_player_audience(event, before, after)?
            && observed_source_audiences.get(&event.event_id) != Some(&player_set)
        {
            return Err(SuccessorProjectionError::ObservationOccurrenceMismatch);
        }
    }
    Ok(projected)
}

fn requires_all_player_audience(
    event: &mtgml_rules::AuthoritativeRuleEvent,
    before: &EngineState,
    after: &EngineState,
) -> Result<bool, SuccessorProjectionError> {
    use AuthoritativeRuleEventKind as Event;
    match &event.event {
        Event::StackItemAdded { .. }
        | Event::TriggerPlaced { .. }
        | Event::StackItemRemoved { .. }
        | Event::ManaPoolChanged { .. }
        | Event::StartingPlayerChosen { .. }
        | Event::MulliganDeclared { .. } => Ok(true),
        Event::CounterChanged { object, .. } => {
            Ok(public_face_up_battlefield_object(*object, before, after))
        }
        Event::TemporaryEffectCreated { effect } | Event::TemporaryEffectExpired { effect } => {
            if effect.affected_objects.is_empty() {
                return Err(SuccessorProjectionError::ObservationOccurrenceMismatch);
            }
            Ok(effect
                .affected_objects
                .iter()
                .all(|object| public_face_up_battlefield_object(*object, before, after)))
        }
        _ => Ok(false),
    }
}

pub(crate) fn public_face_up_battlefield_object(
    object: mtgml_model::GameObjectId,
    before: &EngineState,
    after: &EngineState,
) -> bool {
    let Some(object_state) = after
        .zones
        .objects
        .get(&object)
        .or_else(|| before.zones.objects.get(&object))
    else {
        return false;
    };
    let Some(location) = after
        .zones
        .locations
        .get(&object)
        .or_else(|| before.zones.locations.get(&object))
    else {
        return false;
    };
    location.zone == mtgml_model::ZoneKind::Battlefield
        && location.visibility == mtgml_state::VisibilityPartition::Public
        && !object_state.face_down
}

/// Whether a perspective sees the new incarnation of a moved card: always in
/// a public destination (so a missing opaque identity fails closed), and in
/// a hidden destination only when the perspective tracks it, as an owner who
/// sees a card disappear into its own library does not.
fn reveals_new_incarnation(
    new_object: mtgml_model::GameObjectId,
    to: &mtgml_state::ZoneLocation,
    after_identity: &mtgml_state::PerspectiveIdentityRecordV2,
) -> bool {
    to.visibility == mtgml_state::VisibilityPartition::Public
        || after_identity.object_to_opaque.contains_key(&new_object)
}

fn project_v4_public_source_event(
    source_event: &mtgml_rules::AuthoritativeRuleEvent,
    context: V4PublicSourceEventContext<'_>,
) -> Result<ObservedEventKindV4, SuccessorProjectionError> {
    let V4PublicSourceEventContext {
        before,
        after,
        perspective,
        before_identity,
        after_identity,
        knowledge,
        stack_order_before,
    } = context;
    use AuthoritativeRuleEventKind as Event;
    match &source_event.event {
        Event::ZoneTransition { transition } => {
            let policy = mtgml_rules::PerspectiveObservationPolicyV1::MovedInSight {
                from_zone: transition.from.zone,
                to_zone: transition.to.zone,
                old_object: transition.old_object,
                new_object: transition.new_object,
                // The old incarnation is visible exactly when this
                // perspective had an opaque identity for it before the move.
                reveals_old: before_identity
                    .object_to_opaque
                    .contains_key(&transition.old_object),
                reveals_new: reveals_new_incarnation(
                    transition.new_object,
                    &transition.to,
                    after_identity,
                ),
            };
            project_v4_legacy_observation_policy(
                after,
                &policy,
                before_identity,
                after_identity,
                knowledge,
            )?
            .ok_or(SuccessorProjectionError::ObservationOccurrenceMismatch)
        }
        Event::StartingPlayerChosen {
            chooser,
            starting_player,
        } => Ok(ObservedEventKindV4::StartingPlayerChosen {
            chooser: *chooser,
            starting_player: *starting_player,
        }),
        Event::MulliganDeclared { player, mulligan } => Ok(ObservedEventKindV4::MulliganDeclared {
            player: *player,
            mulligan: *mulligan,
        }),
        Event::ObjectTapped { object, to, .. } => Ok(ObservedEventKindV4::ObjectTapped {
            object: crate::player_projection::public_opaque_object(
                after,
                after_identity,
                knowledge,
                *object,
            )
            .or_else(|| {
                crate::player_projection::public_opaque_object(
                    before,
                    before_identity,
                    knowledge,
                    *object,
                )
            })
            .ok_or(SuccessorProjectionError::MissingOpaqueIdentity)?,
            tapped: *to,
        }),
        Event::StackItemAdded {
            stack_object,
            payload,
        }
        | Event::TriggerPlaced {
            stack_object,
            payload,
            ..
        } => {
            let controller = after
                .zones
                .stack_records
                .get(stack_object)
                .map(|record| record.controller)
                .ok_or(SuccessorProjectionError::ObservationOccurrenceMismatch)?;
            let item = crate::player_projection::project_public_stack_item_v1(
                after,
                after_identity,
                knowledge,
                stack_order_before,
                controller,
                payload,
            )
            .map_err(|_| SuccessorProjectionError::MissingOpaqueIdentity)?;
            Ok(ObservedEventKindV4::StackItemAdded {
                stack_position_from_top: 0,
                item,
            })
        }
        Event::StackItemRemoved {
            stack_object,
            payload,
            result,
        } => {
            let controller = before
                .zones
                .stack_records
                .get(stack_object)
                .map(|record| record.controller)
                .ok_or(SuccessorProjectionError::ObservationOccurrenceMismatch)?;
            let position = stack_order_before
                .iter()
                .rev()
                .position(|candidate| candidate == stack_object)
                .and_then(|position| u32::try_from(position).ok())
                .ok_or(SuccessorProjectionError::ObservationOccurrenceMismatch)?;
            let item = crate::player_projection::project_public_stack_item_v1(
                before,
                before_identity,
                knowledge,
                stack_order_before,
                controller,
                payload,
            )
            .map_err(|_| SuccessorProjectionError::MissingOpaqueIdentity)?;
            Ok(ObservedEventKindV4::StackItemRemoved {
                stack_position_from_top: position,
                item,
                cause: match result {
                    mtgml_state::StackItemEndKindV1::Resolved => {
                        mtgml_observation::StackItemRemovalCauseV1::Resolved
                    }
                    mtgml_state::StackItemEndKindV1::Countered => {
                        mtgml_observation::StackItemRemovalCauseV1::Countered
                    }
                },
            })
        }
        Event::CounterChanged {
            object,
            kind,
            before: from,
            after: to,
        } => Ok(ObservedEventKindV4::CountersChanged {
            object: crate::player_projection::public_opaque_object(
                after,
                after_identity,
                knowledge,
                *object,
            )
            .or_else(|| {
                crate::player_projection::public_opaque_object(
                    before,
                    before_identity,
                    knowledge,
                    *object,
                )
            })
            .ok_or(SuccessorProjectionError::MissingOpaqueIdentity)?,
            counter_kind: match kind {
                mtgml_state::CounterKindV1::PlusOnePlusOne => {
                    mtgml_observation::ObservedCounterKindV3::PlusOnePlusOne
                }
                mtgml_state::CounterKindV1::MinusOneMinusOne => {
                    mtgml_observation::ObservedCounterKindV3::MinusOneMinusOne
                }
                mtgml_state::CounterKindV1::Lore => mtgml_observation::ObservedCounterKindV3::Lore,
            },
            from: *from,
            to: *to,
        }),
        Event::ManaPoolChanged {
            player,
            after: pool,
            cause,
            ..
        } => Ok(ObservedEventKindV4::ManaPoolChanged {
            player: *player,
            pool_after: ManaPoolAfterV1 {
                unrestricted: pool.unrestricted,
                creature_spell_only: pool.creature_spell_only,
            },
            cause: match cause {
                mtgml_state::ManaPoolChangeCauseV1::Produced => {
                    mtgml_observation::ManaPoolChangeCauseV2::Produced
                }
                mtgml_state::ManaPoolChangeCauseV1::Emptied => {
                    mtgml_observation::ManaPoolChangeCauseV2::Emptied
                }
                mtgml_state::ManaPoolChangeCauseV1::Spent => {
                    mtgml_observation::ManaPoolChangeCauseV2::Spent
                }
            },
        }),
        Event::TemporaryEffectCreated { effect } => {
            Ok(ObservedEventKindV4::TemporaryEffectCreated {
                effect: crate::player_projection::project_public_temporary_effect_v1(
                    after,
                    after_identity,
                    knowledge,
                    effect,
                )
                .map_err(|_| SuccessorProjectionError::MissingOpaqueIdentity)?,
            })
        }
        Event::TemporaryEffectExpired { effect } => {
            Ok(ObservedEventKindV4::TemporaryEffectExpired {
                effect: crate::player_projection::project_public_temporary_effect_v1(
                    before,
                    before_identity,
                    knowledge,
                    effect,
                )
                .map_err(|_| SuccessorProjectionError::MissingOpaqueIdentity)?,
            })
        }
        _ => {
            let _ = perspective;
            Err(SuccessorProjectionError::UnsupportedObservedEvent)
        }
    }
}

fn project_v4_legacy_observation_policy(
    after: &EngineState,
    policy: &mtgml_rules::PerspectiveObservationPolicyV1,
    before_identity: &PerspectiveIdentityRecordV2,
    after_identity: &PerspectiveIdentityRecordV2,
    knowledge: &mtgml_state::PlayerKnowledgeStateV2,
) -> Result<Option<ObservedEventKindV4>, SuccessorProjectionError> {
    use mtgml_rules::PerspectiveObservationPolicyV1 as Policy;
    let public_face = |object: mtgml_model::GameObjectId| {
        let Some(object_state) = after.zones.objects.get(&object) else {
            return Err(SuccessorProjectionError::BattlefieldEntryFactsMismatch);
        };
        if object_state.face_down {
            return Ok(None);
        }
        match after.card_rules.faces.faces.get(&object).copied() {
            Some(0) => Ok(Some(ObservedFaceV1::Front)),
            Some(1) => Ok(Some(ObservedFaceV1::Back)),
            _ => Err(SuccessorProjectionError::BattlefieldEntryFactsMismatch),
        }
    };
    match policy {
        Policy::MovedInSight {
            from_zone,
            to_zone,
            old_object,
            new_object,
            reveals_old,
            reveals_new,
        } => Ok(Some(ObservedEventKindV4::ObjectMoved {
            old_object: resolve(*reveals_old, *old_object, before_identity)?,
            new_object: resolve(*reveals_new, *new_object, after_identity)?,
            from: *from_zone,
            to: *to_zone,
            entering_face: if *to_zone == mtgml_model::ZoneKind::Battlefield && *reveals_new {
                public_face(*new_object)?
            } else {
                None
            },
            tapped: if *to_zone == mtgml_model::ZoneKind::Battlefield && *reveals_new {
                Some(
                    after
                        .zones
                        .objects
                        .get(new_object)
                        .ok_or(SuccessorProjectionError::BattlefieldEntryFactsMismatch)?
                        .tapped,
                )
            } else {
                None
            },
        })),
        Policy::Appeared {
            from_zone,
            to_zone,
            new_object,
        } => Ok(Some(ObservedEventKindV4::ObjectMoved {
            old_object: None,
            new_object: resolve(true, *new_object, after_identity)?,
            from: *from_zone,
            to: *to_zone,
            entering_face: if *to_zone == mtgml_model::ZoneKind::Battlefield {
                public_face(*new_object)?
            } else {
                None
            },
            tapped: if *to_zone == mtgml_model::ZoneKind::Battlefield {
                Some(
                    after
                        .zones
                        .objects
                        .get(new_object)
                        .ok_or(SuccessorProjectionError::BattlefieldEntryFactsMismatch)?
                        .tapped,
                )
            } else {
                None
            },
        })),
        Policy::NoEnvelope => Ok(None),
        Policy::ObjectTapped { object, tapped } => Ok(Some(ObservedEventKindV4::ObjectTapped {
            object: crate::player_projection::public_opaque_object(
                after,
                after_identity,
                knowledge,
                *object,
            )
            .or_else(|| {
                crate::player_projection::public_opaque_object(
                    after,
                    before_identity,
                    knowledge,
                    *object,
                )
            })
            .ok_or(SuccessorProjectionError::MissingOpaqueIdentity)?,
            tapped: *tapped,
        })),
        Policy::SawRandomOutcome {
            label,
            exclusive_upper_bound,
            value,
        } => Ok(Some(ObservedEventKindV4::RandomOutcomeVisible {
            label: label.clone(),
            exclusive_upper_bound: *exclusive_upper_bound,
            value: *value,
        })),
        Policy::AnnouncedOutcome { code } => Ok(Some(ObservedEventKindV4::PublicOutcome {
            code: code.clone(),
        })),
    }
}

fn apply_projection_identity_delta(
    identities: &mut BTreeMap<PlayerId, PerspectiveIdentityRecordV2>,
    delta: &mtgml_state::StateDelta,
) -> Result<(), SuccessorProjectionError> {
    use mtgml_state::SemanticDeltaOperation as V3;

    for operation in &delta.operations {
        match operation {
            V3::AbilityIdentityChanged {
                perspective,
                instance,
                from,
                to,
            } => {
                let identity = identities
                    .get_mut(perspective)
                    .ok_or(SuccessorProjectionError::UnknownPerspective)?;
                if identity.ability_to_opaque.get(instance).copied() != *from {
                    return Err(SuccessorProjectionError::ObservationOccurrenceMismatch);
                }
                if let Some(opaque) = from {
                    identity.ability_to_opaque.remove(instance);
                    identity.opaque_to_ability.remove(opaque);
                    identity.retired_ability_ids.insert(*opaque);
                }
                if let Some(opaque) = to {
                    if *opaque != identity.next_opaque_ability_id
                        || identity
                            .opaque_to_ability
                            .insert(*opaque, *instance)
                            .is_some()
                        || identity
                            .ability_to_opaque
                            .insert(*instance, *opaque)
                            .is_some()
                    {
                        return Err(SuccessorProjectionError::ObservationOccurrenceMismatch);
                    }
                    identity.next_opaque_ability_id = mtgml_model::OpaqueAbilityId(
                        opaque
                            .0
                            .checked_add(1)
                            .ok_or(SuccessorProjectionError::CursorMismatch)?,
                    );
                }
            }
            V3::PendingRequestChanged {
                to: Some(request), ..
            } => {
                let identity = identities
                    .get_mut(&request.actor)
                    .ok_or(SuccessorProjectionError::UnknownPerspective)?;
                if identity.next_player_decision_id != request.player_decision_id {
                    return Err(SuccessorProjectionError::ObservationOccurrenceMismatch);
                }
                identity.next_player_decision_id = mtgml_model::PlayerDecisionIdV1(
                    request
                        .player_decision_id
                        .0
                        .checked_add(1)
                        .ok_or(SuccessorProjectionError::CursorMismatch)?,
                );
            }
            _ => {}
        }
    }
    Ok(())
}

/// Builds detached V4 player products from a validated G0 successor
/// transition. The caller must run State/Delta/event and Rules admission
/// before passing profile-independent state to this projection boundary.
pub fn project_successor_player_steps_v4(
    transition: SuccessorTransitionV4Projection<'_>,
    authority: SuccessorProjectionAuthority<'_>,
) -> Result<BTreeMap<PlayerId, mtgml_observation::PlayerStepV4>, ControllerError> {
    if let Some(admission) = authority.basic_land_admission {
        mtgml_rules::validate_magic_pending_request(
            admission,
            transition.before,
            transition.before_status,
        )
        .map_err(|error| ControllerError::Backend(error.to_string()))?;
        mtgml_rules::validate_magic_pending_request(admission, transition.after, transition.status)
            .map_err(|error| ControllerError::Backend(error.to_string()))?;
    } else {
        transition
            .before
            .validate()
            .and(transition.after.validate())
            .map_err(|error| ControllerError::Backend(error.to_string()))?;
    }
    if transition.next_request != transition.after.execution.pending_decision.as_ref() {
        return Err(ControllerError::Backend(
            "projected V4 request differs from the authoritative pending request".into(),
        ));
    }
    let events = if let Some(admission) = authority.basic_land_admission {
        project_successor_events_v4_for_basic_land_profile(
            admission,
            transition.before,
            transition.before_status,
            transition.after,
            transition.status,
            transition.events,
            transition.delta,
        )
    } else {
        project_successor_events_v4(transition.before, transition.after, transition.events)
    }
    .map_err(|error| ControllerError::Backend(error.to_string()))?;
    let players: BTreeSet<_> = transition.after.core.players.keys().copied().collect();
    let mut steps = BTreeMap::new();
    for perspective in players {
        let information_state = if authority.basic_land_admission.is_some() {
            crate::player_projection::project_successor_information_state_structural_only(
                transition.after,
                perspective,
                authority.execution_identity,
                authority.semantic_manifest,
                authority.rules_manifest,
                authority.catalog,
            )
        } else {
            crate::project_successor_information_state(
                transition.after,
                perspective,
                authority.execution_identity,
                authority.semantic_manifest,
                authority.rules_manifest,
                authority.catalog,
            )
        }
        .map_err(|error| ControllerError::Backend(error.to_string()))?;
        let next_decision = transition
            .next_request
            .filter(|request| request.actor == perspective)
            .map(|request| request.project_player_request())
            .transpose()
            .map_err(|error| ControllerError::Backend(error.to_string()))?;
        let submission = if transition.accepted || perspective != transition.actor {
            PlayerStepSubmissionV1::Accepted
        } else {
            PlayerStepSubmissionV1::Rejected {
                code: transition.rejected_code,
            }
        };
        let step = mtgml_observation::PlayerStepV4 {
            schema_version: mtgml_observation::PLAYER_STEP_SCHEMA_V4.to_owned(),
            information_state,
            observed_events: events
                .get(&perspective)
                .cloned()
                .ok_or_else(|| ControllerError::Backend("missing perspective events".into()))?,
            next_decision,
            status: transition.status.clone(),
            submission,
        };
        step.validate()
            .map_err(|error| ControllerError::Backend(error.to_string()))?;
        mtgml_wire::encode_canonical(&step)
            .map_err(|error| ControllerError::Backend(error.to_string()))?;
        steps.insert(perspective, step);
    }
    Ok(steps)
}

#[cfg(test)]
mod reveal_tests {
    use super::reveals_new_incarnation;
    use mtgml_model::{GameObjectId, OpaqueObjectId, PlayerId, ZoneKind};
    use mtgml_state::{
        PerspectiveIdentityRecordV2, VisibilityPartition, ZoneLocation, ZonePosition,
    };

    fn location(zone: ZoneKind, visibility: VisibilityPartition) -> ZoneLocation {
        ZoneLocation {
            zone,
            player: Some(PlayerId(1)),
            position: ZonePosition::Top { offset: 0 },
            visibility,
            partition: None,
        }
    }

    #[test]
    fn a_public_destination_always_reveals_and_a_hidden_one_only_when_tracked() {
        let untracked = PerspectiveIdentityRecordV2::default();
        let mut tracked = PerspectiveIdentityRecordV2::default();
        tracked
            .object_to_opaque
            .insert(GameObjectId(2), OpaqueObjectId(1));
        let graveyard = location(ZoneKind::Graveyard, VisibilityPartition::Public);
        let library = location(ZoneKind::Library, VisibilityPartition::FaceDown);
        // A missing identity in a public zone must surface, not vanish.
        assert!(reveals_new_incarnation(
            GameObjectId(2),
            &graveyard,
            &untracked
        ));
        assert!(!reveals_new_incarnation(
            GameObjectId(2),
            &library,
            &untracked
        ));
        assert!(reveals_new_incarnation(GameObjectId(2), &library, &tracked));
    }
}
