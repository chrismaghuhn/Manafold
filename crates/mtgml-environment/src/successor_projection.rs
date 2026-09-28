//! Perspective projection for authoritative successor transition products.

use std::collections::{BTreeMap, BTreeSet};

use mtgml_model::{OpaqueObjectId, PlayerId, VisibleSequence};
use mtgml_observation::{
    ManaPoolAfterV1, ObservedEventEnvelopeV3, ObservedEventEnvelopeV4, ObservedEventKindV3,
    ObservedEventKindV4, ObservedFaceV1, PlayerStepSubmissionV1, PlayerStepV3,
    PlayerSubmissionCodeV1, OBSERVED_EVENT_SCHEMA_V3, PLAYER_STEP_SCHEMA_V3,
};
use mtgml_rules::{
    AuthoritativeRuleEventKindV2, AuthoritativeRuleEventKindV3, SuccessorObservationPolicyV1,
    TransitionResult,
};
use mtgml_state::{EngineStatePartsV2, EngineStatePartsV3, PerspectiveIdentityRecordV2};

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
    pub before: &'a EngineStatePartsV3,
    pub after: &'a EngineStatePartsV3,
    pub events: &'a [mtgml_rules::AuthoritativeRuleEventV3],
    pub delta: Option<&'a mtgml_state::StateDeltaV3>,
    pub accepted: bool,
    pub status: &'a mtgml_model::EpisodeStatus,
    pub next_request: Option<&'a mtgml_decision::AuthoritativeDecisionRequestV4>,
    pub actor: PlayerId,
    pub rejected_code: PlayerSubmissionCodeV1,
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

/// Projects the exact rules-owned audience records. This function does not
/// infer visibility from CardDefinition, allocation order, or object storage.
pub fn project_successor_events_v3(
    before: &EngineStatePartsV2,
    after: &EngineStatePartsV2,
    events: &[mtgml_rules::AuthoritativeRuleEventV2],
) -> Result<BTreeMap<PlayerId, Vec<ObservedEventEnvelopeV3>>, SuccessorProjectionError> {
    let before_engine = before.materialize();
    let after_engine = after.materialize();
    let mut projected: BTreeMap<_, Vec<_>> = before_engine
        .knowledge
        .players
        .keys()
        .copied()
        .map(|player| (player, Vec::new()))
        .collect();
    let mut cursors: BTreeMap<_, _> = before_engine
        .knowledge
        .players
        .iter()
        .map(|(player, state)| (*player, state.next_visible_sequence.0))
        .collect();
    let mut identities = before_engine.perspective_identities.players.clone();

    for event in events {
        let AuthoritativeRuleEventKindV2::PerspectiveOccurrence {
            lifecycle,
            observation,
        } = &event.event
        else {
            continue;
        };
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
        let mut after_identity = before_identity.clone();
        mtgml_state::advance_identity_record(&mut after_identity, &lifecycle.mutation.identity);
        let kind = match observation {
            SuccessorObservationPolicyV1::MovedInSight {
                from,
                to,
                old_object,
                new_object,
                reveals_old,
                entering_face,
                tapped,
            } => ObservedEventKindV3::ObjectMoved {
                old_object: resolve(*reveals_old, *old_object, &before_identity)?,
                new_object: resolve(true, *new_object, &after_identity)?,
                from: *from,
                to: *to,
                entering_face: if *to == mtgml_model::ZoneKind::Battlefield {
                    let object_state = after
                        .predecessor_v5
                        .zones
                        .objects
                        .get(new_object)
                        .ok_or(SuccessorProjectionError::BattlefieldEntryFactsMismatch)?;
                    let face = after
                        .card_rules_state
                        .faces
                        .faces
                        .get(new_object)
                        .copied()
                        .ok_or(SuccessorProjectionError::BattlefieldEntryFactsMismatch)?;
                    let (authoritative_face, observed_face) = match face {
                        0 => (mtgml_rules::BasicLandFaceV1::Front, ObservedFaceV1::Front),
                        1 => (mtgml_rules::BasicLandFaceV1::Back, ObservedFaceV1::Back),
                        _ => return Err(SuccessorProjectionError::BattlefieldEntryFactsMismatch),
                    };
                    if object_state.tapped != *tapped || authoritative_face != *entering_face {
                        return Err(SuccessorProjectionError::BattlefieldEntryFactsMismatch);
                    }
                    Some(observed_face)
                } else {
                    None
                },
                tapped: (*to == mtgml_model::ZoneKind::Battlefield).then_some(*tapped),
            },
            SuccessorObservationPolicyV1::ObjectTapped { object, tapped } => {
                ObservedEventKindV3::ObjectTapped {
                    object: resolve(true, *object, &before_identity)?
                        .ok_or(SuccessorProjectionError::MissingOpaqueIdentity)?,
                    tapped: *tapped,
                }
            }
            SuccessorObservationPolicyV1::ManaPoolChanged { player } => {
                let pool = after
                    .card_rules_state
                    .mana
                    .pools
                    .get(player)
                    .ok_or(SuccessorProjectionError::UnknownPerspective)?;
                ObservedEventKindV3::ManaPoolChanged {
                    player: *player,
                    pool_after: ManaPoolAfterV1 {
                        unrestricted: pool.unrestricted,
                        creature_spell_only: pool.creature_spell_only,
                    },
                    cause: if pool
                        .unrestricted
                        .iter()
                        .chain(pool.creature_spell_only.iter())
                        .all(|amount| *amount == 0)
                    {
                        mtgml_observation::ManaPoolChangeCauseV1::Emptied
                    } else {
                        mtgml_observation::ManaPoolChangeCauseV1::Produced
                    },
                }
            }
        };
        let envelope = ObservedEventEnvelopeV3 {
            schema_version: OBSERVED_EVENT_SCHEMA_V3.to_owned(),
            sequence: VisibleSequence(lifecycle.sequence.0),
            state_revision: event.state_revision,
            event: kind,
        };
        envelope
            .validate()
            .map_err(|_| SuccessorProjectionError::CursorMismatch)?;
        projected
            .get_mut(&lifecycle.perspective)
            .ok_or(SuccessorProjectionError::UnknownPerspective)?
            .push(envelope);
        identities.insert(lifecycle.perspective, after_identity);
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
    Ok(projected)
}

/// Projects Rules-authored successor observation occurrences into the
/// revision-free V4 player event product. StateRevision never appears in the
/// result; only the perspective's already-authoritative visible sequence is
/// used for public chronology.
pub fn project_successor_events_v4(
    before: &EngineStatePartsV3,
    after: &EngineStatePartsV3,
    events: &[mtgml_rules::AuthoritativeRuleEventV3],
) -> Result<BTreeMap<PlayerId, Vec<ObservedEventEnvelopeV4>>, SuccessorProjectionError> {
    project_successor_events_v4_inner(before, after, events, None, false)
}

pub fn project_successor_events_v4_for_basic_land_profile(
    admission: &mtgml_card_ir::ExecutableProfileAdmissionV1,
    status: &mtgml_model::EpisodeStatus,
    before: &EngineStatePartsV3,
    after: &EngineStatePartsV3,
    events: &[mtgml_rules::AuthoritativeRuleEventV3],
    delta: Option<&mtgml_state::StateDeltaV3>,
) -> Result<BTreeMap<PlayerId, Vec<ObservedEventEnvelopeV4>>, SuccessorProjectionError> {
    mtgml_rules::validate_basic_land_pending_request_v4(admission, before, status)
        .map_err(|_| SuccessorProjectionError::ObservationOccurrenceMismatch)?;
    mtgml_rules::validate_basic_land_pending_request_v4(admission, after, status)
        .map_err(|_| SuccessorProjectionError::ObservationOccurrenceMismatch)?;
    project_successor_events_v4_inner(before, after, events, delta, true)
}

fn project_successor_events_v4_inner(
    before: &EngineStatePartsV3,
    after: &EngineStatePartsV3,
    events: &[mtgml_rules::AuthoritativeRuleEventV3],
    delta: Option<&mtgml_state::StateDeltaV3>,
    rules_domain_validated: bool,
) -> Result<BTreeMap<PlayerId, Vec<ObservedEventEnvelopeV4>>, SuccessorProjectionError> {
    let state_validation = if rules_domain_validated {
        before.validate_structure().and(after.validate_structure())
    } else {
        before.validate().and(after.validate())
    };
    state_validation.map_err(|_| SuccessorProjectionError::ObservationOccurrenceMismatch)?;
    mtgml_rules::validate_rule_event_cursor_v3(
        before.predecessor_v5.allocators.next_rule_event_id,
        after.predecessor_v5.allocators.next_rule_event_id,
        after.predecessor_v5.revision,
        events,
    )
    .map_err(|_| SuccessorProjectionError::ObservationOccurrenceMismatch)?;
    let before_engine: mtgml_state::EngineState = before.predecessor_v5.clone().into();
    let after_engine: mtgml_state::EngineState = after.predecessor_v5.clone().into();
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
    let mut identities = before_engine.perspective_identities.players.clone();
    let mut working_stack_order = before.predecessor_v5.zones.stack_order.clone();
    let mut stack_order_before_event = BTreeMap::new();
    for event in events {
        match &event.event {
            AuthoritativeRuleEventKindV3::StackItemAdded { stack_object, .. } => {
                stack_order_before_event.insert(event.event_id, working_stack_order.clone());
                if working_stack_order.contains(stack_object) {
                    return Err(SuccessorProjectionError::ObservationOccurrenceMismatch);
                }
                working_stack_order.push(*stack_object);
            }
            AuthoritativeRuleEventKindV3::TriggerPlaced { stack_object, .. } => {
                stack_order_before_event.insert(event.event_id, working_stack_order.clone());
                if working_stack_order.contains(stack_object) {
                    return Err(SuccessorProjectionError::ObservationOccurrenceMismatch);
                }
                working_stack_order.push(*stack_object);
            }
            AuthoritativeRuleEventKindV3::StackItemRemoved { stack_object, .. } => {
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
    if working_stack_order != after.predecessor_v5.zones.stack_order {
        return Err(SuccessorProjectionError::FinalStackOrderMismatch);
    }

    for (index, event) in events.iter().enumerate() {
        let (lifecycle, direct_policy, source_event_id) = match &event.event {
            AuthoritativeRuleEventKindV3::Existing { event } => match event.as_ref() {
                mtgml_rules::AuthoritativeRuleEventKind::PerspectiveOccurrence {
                    lifecycle,
                    observation,
                } => (Some(lifecycle), Some(observation), None),
                _ => (None, None, None),
            },
            AuthoritativeRuleEventKindV3::PerspectiveObservationOccurrence {
                lifecycle,
                source_event_id,
            } => (Some(lifecycle.as_ref()), None, Some(*source_event_id)),
            _ => (None, None, None),
        };
        let Some(lifecycle) = lifecycle else {
            continue;
        };
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
        mtgml_state::advance_identity_record(after_identity, &lifecycle.mutation.identity);

        let observation = if let Some(source_event_id) = source_event_id {
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
                before,
                after,
                source,
                lifecycle.perspective,
                &before_identity,
                after_identity,
                &stack_order,
            )?)
        } else {
            project_v4_legacy_observation_policy(
                after,
                direct_policy.ok_or(SuccessorProjectionError::ObservationOccurrenceMismatch)?,
                &before_identity,
                after_identity,
            )?
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
            if let Some(source_event_id) = source_event_id {
                if !observed_source_audiences
                    .entry(source_event_id)
                    .or_default()
                    .insert(lifecycle.perspective)
                {
                    return Err(SuccessorProjectionError::ObservationOccurrenceMismatch);
                }
            }
        }
    }
    if let Some(delta) = delta {
        apply_projection_identity_delta(&mut identities, delta)?;
    }
    if identities != after_engine.perspective_identities.players {
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
    event: &mtgml_rules::AuthoritativeRuleEventV3,
    before: &EngineStatePartsV3,
    after: &EngineStatePartsV3,
) -> Result<bool, SuccessorProjectionError> {
    use AuthoritativeRuleEventKindV3 as Event;
    match &event.event {
        Event::StackItemAdded { .. }
        | Event::TriggerPlaced { .. }
        | Event::StackItemRemoved { .. }
        | Event::ManaPoolChanged { .. } => Ok(true),
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

fn public_face_up_battlefield_object(
    object: mtgml_model::GameObjectId,
    before: &EngineStatePartsV3,
    after: &EngineStatePartsV3,
) -> bool {
    let Some(object_state) = after
        .predecessor_v5
        .zones
        .objects
        .get(&object)
        .or_else(|| before.predecessor_v5.zones.objects.get(&object))
    else {
        return false;
    };
    let Some(location) = after
        .predecessor_v5
        .zones
        .locations
        .get(&object)
        .or_else(|| before.predecessor_v5.zones.locations.get(&object))
    else {
        return false;
    };
    location.zone == mtgml_model::ZoneKind::Battlefield
        && location.visibility == mtgml_state::VisibilityPartition::Public
        && !object_state.face_down
}

fn project_v4_public_source_event(
    before: &EngineStatePartsV3,
    after: &EngineStatePartsV3,
    source_event: &mtgml_rules::AuthoritativeRuleEventV3,
    perspective: PlayerId,
    before_identity: &PerspectiveIdentityRecordV2,
    after_identity: &PerspectiveIdentityRecordV2,
    stack_order_before: &[mtgml_model::StackObjectId],
) -> Result<ObservedEventKindV4, SuccessorProjectionError> {
    use AuthoritativeRuleEventKindV3 as Event;
    match &source_event.event {
        Event::Existing { event } => match event.as_ref() {
            mtgml_rules::AuthoritativeRuleEventKind::ZoneTransition { transition } => {
                let policy = mtgml_rules::PerspectiveObservationPolicyV1::MovedInSight {
                    from_zone: transition.from.zone,
                    to_zone: transition.to.zone,
                    old_object: transition.old_object,
                    new_object: transition.new_object,
                    reveals_old: true,
                    reveals_new: true,
                };
                project_v4_legacy_observation_policy(
                    after,
                    &policy,
                    before_identity,
                    after_identity,
                )?
                .ok_or(SuccessorProjectionError::ObservationOccurrenceMismatch)
            }
            mtgml_rules::AuthoritativeRuleEventKind::ObjectTapped { object, to, .. } => {
                Ok(ObservedEventKindV4::ObjectTapped {
                    object: after_identity
                        .object_to_opaque
                        .get(object)
                        .or_else(|| before_identity.object_to_opaque.get(object))
                        .copied()
                        .ok_or(SuccessorProjectionError::MissingOpaqueIdentity)?,
                    tapped: *to,
                })
            }
            _ => Err(SuccessorProjectionError::UnsupportedObservedEvent),
        },
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
                .predecessor_v5
                .zones
                .stack_records
                .get(stack_object)
                .map(|record| record.controller)
                .ok_or(SuccessorProjectionError::ObservationOccurrenceMismatch)?;
            let item = crate::player_projection::project_public_stack_item_v1(
                after_identity,
                after
                    .predecessor_v5
                    .knowledge
                    .players
                    .get(&perspective)
                    .ok_or(SuccessorProjectionError::UnknownPerspective)?,
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
                .predecessor_v5
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
                before_identity,
                before
                    .predecessor_v5
                    .knowledge
                    .players
                    .get(&perspective)
                    .ok_or(SuccessorProjectionError::UnknownPerspective)?,
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
            object: after_identity
                .object_to_opaque
                .get(object)
                .or_else(|| before_identity.object_to_opaque.get(object))
                .copied()
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
                    after_identity,
                    effect,
                )
                .map_err(|_| SuccessorProjectionError::MissingOpaqueIdentity)?,
            })
        }
        Event::TemporaryEffectExpired { effect } => {
            Ok(ObservedEventKindV4::TemporaryEffectExpired {
                effect: crate::player_projection::project_public_temporary_effect_v1(
                    before_identity,
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
    after: &EngineStatePartsV3,
    policy: &mtgml_rules::PerspectiveObservationPolicyV1,
    before_identity: &PerspectiveIdentityRecordV2,
    after_identity: &PerspectiveIdentityRecordV2,
) -> Result<Option<ObservedEventKindV4>, SuccessorProjectionError> {
    use mtgml_rules::PerspectiveObservationPolicyV1 as Policy;
    let public_face = |object: mtgml_model::GameObjectId| {
        let Some(object_state) = after.predecessor_v5.zones.objects.get(&object) else {
            return Err(SuccessorProjectionError::BattlefieldEntryFactsMismatch);
        };
        if object_state.face_down {
            return Ok(None);
        }
        match after.card_rules_state.faces.faces.get(&object).copied() {
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
                        .predecessor_v5
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
                        .predecessor_v5
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
            object: resolve(true, *object, after_identity)?
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

/// Composes the new public observation, perspective event stream, and supplied
/// authoritative V3 request into one validated PlayerStep for every player.
pub fn project_successor_player_steps(
    before: &EngineStatePartsV2,
    transition: &TransitionResult,
    authority: SuccessorProjectionAuthority<'_>,
    actor: PlayerId,
    rejected_code: PlayerSubmissionCodeV1,
) -> Result<BTreeMap<PlayerId, PlayerStepV3>, ControllerError> {
    let events = project_successor_events_v3(before, &transition.next_state, &transition.events)
        .map_err(|error| ControllerError::Backend(error.to_string()))?;
    let players: BTreeSet<_> = transition
        .next_state
        .predecessor_v5
        .core
        .players
        .keys()
        .copied()
        .collect();
    let mut steps = BTreeMap::new();
    for perspective in players {
        let information_state = crate::project_successor_information_state(
            &transition.next_state,
            perspective,
            authority.execution_identity,
            authority.semantic_manifest,
            authority.rules_manifest,
            authority.catalog,
        )
        .map_err(|error| ControllerError::Backend(error.to_string()))?;
        let next_decision = transition
            .next_decision
            .as_ref()
            .filter(|request| request.actor == perspective)
            .map(|request| request.project_player_request())
            .transpose()
            .map_err(|error| ControllerError::Backend(error.to_string()))?;
        let submission = if transition.accepted || perspective != actor {
            PlayerStepSubmissionV1::Accepted
        } else {
            PlayerStepSubmissionV1::Rejected {
                code: rejected_code,
            }
        };
        let step = PlayerStepV3 {
            schema_version: PLAYER_STEP_SCHEMA_V3.to_owned(),
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
            .map_err(|_| ControllerError::Backend("invalid successor PlayerStep".into()))?;
        steps.insert(perspective, step);
    }
    Ok(steps)
}

fn apply_projection_identity_delta(
    identities: &mut BTreeMap<PlayerId, PerspectiveIdentityRecordV2>,
    delta: &mtgml_state::StateDeltaV3,
) -> Result<(), SuccessorProjectionError> {
    use mtgml_state::{SemanticDeltaOperationV2 as V2, SemanticDeltaOperationV3 as V3};

    for operation in &delta.operations {
        match operation {
            V3::Existing { operation } => {
                let V2::AbilityIdentityChanged {
                    perspective,
                    instance,
                    from,
                    to,
                } = operation.as_ref()
                else {
                    continue;
                };
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
        mtgml_rules::validate_basic_land_pending_request_v4(
            admission,
            transition.before,
            transition.status,
        )
        .map_err(|error| ControllerError::Backend(error.to_string()))?;
        mtgml_rules::validate_basic_land_pending_request_v4(
            admission,
            transition.after,
            transition.status,
        )
        .map_err(|error| ControllerError::Backend(error.to_string()))?;
    } else {
        transition
            .before
            .validate()
            .and(transition.after.validate())
            .map_err(|error| ControllerError::Backend(error.to_string()))?;
    }
    if transition.next_request != transition.after.execution_v4.pending_decision.as_ref() {
        return Err(ControllerError::Backend(
            "projected V4 request differs from the authoritative pending request".into(),
        ));
    }
    let events = if let Some(admission) = authority.basic_land_admission {
        project_successor_events_v4_for_basic_land_profile(
            admission,
            transition.status,
            transition.before,
            transition.after,
            transition.events,
            transition.delta,
        )
    } else {
        project_successor_events_v4(transition.before, transition.after, transition.events)
    }
    .map_err(|error| ControllerError::Backend(error.to_string()))?;
    let players: BTreeSet<_> = transition
        .after
        .predecessor_v5
        .core
        .players
        .keys()
        .copied()
        .collect();
    let mut steps = BTreeMap::new();
    for perspective in players {
        let information_state = if authority.basic_land_admission.is_some() {
            crate::project_successor_information_state_v3_after_rules_domain_validation(
                transition.after,
                perspective,
                authority.execution_identity,
                authority.semantic_manifest,
                authority.rules_manifest,
                authority.catalog,
            )
        } else {
            crate::project_successor_information_state_v3(
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
