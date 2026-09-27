//! Exact consistency validation for the bounded successor transition product.

use mtgml_card_ir::ExecutableProfileAdmissionV1;
use mtgml_decision::AuthoritativeDecisionRequestV3;
use mtgml_model::StateRevision;
use mtgml_state::{
    apply_perspective_lifecycle, AbilityAuthorityV1, EngineStatePartsV2, ManaPoolV1,
    SemanticDeltaOperation, SemanticDeltaOperationV2,
};

use crate::{AuthoritativeRuleEventKindV2, TransitionResult, TransitionViolation};

/// Proves that a complete successor state is exactly accounted for by its
/// closed semantic operations, and that its event sequence is the matching
/// public/evidence projection. This is called at the environment commit seam.
macro_rules! invalid {
    () => {{
        return Err(TransitionViolation::SuccessorProduct);
    }};
}

pub fn validate_successor_transition_contract(
    admission: &ExecutableProfileAdmissionV1,
    before: &EngineStatePartsV2,
    product: &TransitionResult,
) -> Result<(), TransitionViolation> {
    before
        .validate()
        .map_err(|_| TransitionViolation::SuccessorProduct)?;
    product
        .next_state
        .validate()
        .map_err(|_| TransitionViolation::SuccessorProduct)?;
    let applied = product
        .delta
        .apply(before)
        .map_err(|_| TransitionViolation::DeltaReapplication)?;
    if applied != product.next_state {
        return Err(TransitionViolation::DeltaReapplication);
    }

    if !product.accepted {
        if product.next_state != *before
            || !product.delta.operations.is_empty()
            || !product.events.is_empty()
            || product.status != mtgml_model::EpisodeStatus::Running
                && product.next_decision.is_some()
            || product.next_decision != before.execution_v3.pending_decision
        {
            return Err(TransitionViolation::RejectedMutation);
        }
        return Ok(());
    }

    let before_request = before
        .execution_v3
        .pending_decision
        .as_ref()
        .ok_or(TransitionViolation::SuccessorProduct)?;
    before_request
        .validate()
        .map_err(|_| TransitionViolation::SuccessorProduct)?;
    let mut before_candidate_state = before.clone();
    before_candidate_state.execution_v3.pending_decision = None;
    let before_candidates = crate::derive_basic_land_candidates(
        admission,
        &before_candidate_state,
        before_request.actor,
        &product.status,
    )
    .map_err(|_| TransitionViolation::SuccessorProduct)?;
    if before_request.state_revision != before.predecessor_v5.revision
        || before_request.visibility != mtgml_decision::DecisionVisibility::Public
        || before_request.decision != mtgml_decision::DecisionDomainV2::ChooseOne
        || before_request.continuation_id.is_some()
        || before_request.candidates != before_candidates
    {
        return Err(TransitionViolation::SuccessorProduct);
    }

    let expected_revision = StateRevision(
        before
            .predecessor_v5
            .revision
            .0
            .checked_add(1)
            .ok_or(TransitionViolation::RevisionDidNotAdvance)?,
    );
    if product.next_state.predecessor_v5.revision != expected_revision
        || !matches!(product.status, mtgml_model::EpisodeStatus::Running)
        || product.delta.before_revision != before.predecessor_v5.revision
        || product.delta.after_revision != expected_revision
        || product.next_decision != product.next_state.execution_v3.pending_decision
        || product
            .next_decision
            .as_ref()
            .is_some_and(|request| request.state_revision != expected_revision)
    {
        invalid!();
    }

    for (index, event) in product.events.iter().enumerate() {
        let expected_id = before
            .predecessor_v5
            .allocators
            .next_rule_event_id
            .0
            .checked_add(index as u64)
            .ok_or(TransitionViolation::EventIdentity)?;
        if event.event_id.0 != expected_id || event.state_revision != expected_revision {
            return Err(TransitionViolation::EventIdentity);
        }
    }

    let mut reconstructed = before.clone();
    let mut created: Option<&AuthoritativeDecisionRequestV3> = None;
    let mut cleared = None;
    let mut tapped_for_mana = std::collections::BTreeSet::new();
    let mut position_changed = false;
    let mut expected_event_kinds = Vec::new();

    // Zone identity is the foundation for the occurrence lifecycle rows in
    // this bounded producer. Stage its authoritative event payloads first;
    // the operation list records the semantic family ordering, not a series
    // of independently publishable intermediate states.
    for operation in &product.delta.operations {
        let SemanticDeltaOperationV2::ObjectEntered {
            old_object,
            new_object,
            ..
        } = operation
        else {
            continue;
        };
        let transition = product
            .events
            .iter()
            .find_map(|event| match &event.event {
                AuthoritativeRuleEventKindV2::ZoneTransition(transition)
                    if transition.old_object == old_object.unwrap_or(*new_object)
                        && transition.new_object == *new_object =>
                {
                    Some(transition.as_ref())
                }
                _ => None,
            })
            .ok_or(TransitionViolation::EventDeltaMismatch)?;
        apply_zone_transition(&mut reconstructed, transition)?;
    }

    for operation in &product.delta.operations {
        match operation {
            SemanticDeltaOperationV2::Existing { operation } => match operation.as_ref() {
                SemanticDeltaOperation::PriorityChanged { from, to } => {
                    if reconstructed.predecessor_v5.core.priority != *from
                        || !before_request.candidates.iter().any(|candidate| {
                            matches!(
                                candidate.trusted_binding,
                                mtgml_decision::EngineCandidateBindingV3::PassPriority
                            )
                        })
                    {
                        invalid!();
                    }
                    reconstructed.predecessor_v5.core.priority = *to;
                    expected_event_kinds.push(AuthoritativeRuleEventKindV2::PriorityChanged {
                        from: *from,
                        to: *to,
                    });
                }
                SemanticDeltaOperation::TurnPositionChanged { from, to } => {
                    if reconstructed.predecessor_v5.core.position != *from
                        || crate::temporal_successor(*from) != *to
                        || position_changed
                    {
                        invalid!();
                    }
                    reconstructed.predecessor_v5.core.position = *to;
                    position_changed = true;
                    expected_event_kinds.push(AuthoritativeRuleEventKindV2::TurnPositionChanged {
                        from: *from,
                        to: *to,
                    });
                }
                SemanticDeltaOperation::DecisionCleared { decision } => {
                    if before
                        .execution_v3
                        .pending_decision
                        .as_ref()
                        .map(|request| request.decision_id)
                        != Some(*decision)
                        || cleared.replace(*decision).is_some()
                    {
                        invalid!();
                    }
                    reconstructed.execution_v3.pending_decision = None;
                    expected_event_kinds.push(AuthoritativeRuleEventKindV2::DecisionCleared {
                        decision: *decision,
                    });
                }
                SemanticDeltaOperation::DecisionCreated { decision } => {
                    let request = product
                        .next_decision
                        .as_ref()
                        .filter(|request| request.decision_id == *decision)
                        .ok_or(TransitionViolation::SuccessorProduct)?;
                    if created.replace(request).is_some() {
                        invalid!();
                    }
                    expected_event_kinds.push(AuthoritativeRuleEventKindV2::DecisionCreated {
                        decision: *decision,
                    });
                }
                SemanticDeltaOperation::ObjectTapped { object, from, to } => {
                    let game_object = reconstructed
                        .predecessor_v5
                        .zones
                        .objects
                        .get_mut(object)
                        .ok_or(TransitionViolation::SuccessorProduct)?;
                    if game_object.tapped != *from || from == to {
                        invalid!();
                    }
                    game_object.tapped = *to;
                    expected_event_kinds.push(AuthoritativeRuleEventKindV2::ObjectTapped {
                        object: *object,
                        from: *from,
                        to: *to,
                    });
                }
                SemanticDeltaOperation::PerspectiveLifecycle { lifecycle } => {
                    let mut engine = reconstructed.materialize();
                    apply_perspective_lifecycle(&mut engine, lifecycle)
                        .map_err(|_| TransitionViolation::SuccessorProduct)?;
                    let execution = reconstructed.execution_v3.clone();
                    reconstructed = EngineStatePartsV2::from_state(
                        &engine,
                        reconstructed.card_rules_state.clone(),
                    );
                    reconstructed.execution_v3 = execution;
                    expected_event_kinds.push(
                        product
                            .events
                            .iter()
                            .find_map(|event| match &event.event {
                                AuthoritativeRuleEventKindV2::PerspectiveOccurrence {
                                    lifecycle: event_lifecycle,
                                    ..
                                } if event_lifecycle.as_ref() == lifecycle => {
                                    Some(event.event.clone())
                                }
                                _ => None,
                            })
                            .ok_or(TransitionViolation::EventDeltaMismatch)?,
                    );
                }
                _ => return Err(TransitionViolation::SuccessorProduct),
            },
            SemanticDeltaOperationV2::LandPlayCountChanged { player, from, to } => {
                let history = reconstructed
                    .card_rules_state
                    .turn_history
                    .players
                    .get_mut(player)
                    .ok_or(TransitionViolation::SuccessorProduct)?;
                if history.land_plays_used != *from || *from != 0 || *to != 1 {
                    invalid!();
                }
                history.land_plays_used = *to;
            }
            SemanticDeltaOperationV2::ManaAdded {
                player,
                color,
                restriction,
                amount,
                source,
                ability_key,
            } => {
                let source_object = reconstructed
                    .predecessor_v5
                    .zones
                    .objects
                    .get(source)
                    .ok_or(TransitionViolation::SuccessorProduct)?;
                let source_location = reconstructed
                    .predecessor_v5
                    .zones
                    .locations
                    .get(source)
                    .ok_or(TransitionViolation::SuccessorProduct)?;
                let definition = admission
                    .verified_catalog()
                    .get(
                        admission.content_contract_id(),
                        source_object.card_definition,
                    )
                    .map_err(|_| TransitionViolation::SuccessorProduct)?;
                let expected_color = match &definition.semantic_binding {
                    mtgml_card_ir::CardSemanticBindingV1::ProfiledV1 { body, .. } => {
                        match body.subtype {
                            mtgml_card_ir::BasicLandSubtypeV1::Mountain => {
                                mtgml_state::ManaColorV1::Red
                            }
                            mtgml_card_ir::BasicLandSubtypeV1::Plains => {
                                mtgml_state::ManaColorV1::White
                            }
                        }
                    }
                    mtgml_card_ir::CardSemanticBindingV1::UnprofiledV1 => {
                        return Err(TransitionViolation::SuccessorProduct);
                    }
                };
                if *amount != 1
                    || *restriction != mtgml_state::ManaRestrictionV1::Unrestricted
                    || *color != expected_color
                    || *player != source_object.controller
                    || source_location.zone != mtgml_model::ZoneKind::Battlefield
                    || !source_object.tapped
                    || !tapped_for_mana.contains(source)
                    || *ability_key != 0
                    || !before_request.candidates.iter().any(|candidate| {
                        matches!(
                            candidate.trusted_binding,
                            mtgml_decision::EngineCandidateBindingV3::ActivateAbility { ability }
                                if before
                                    .card_rules_state
                                    .abilities
                                    .by_instance
                                    .get(&ability)
                                    .is_some_and(|authority| {
                                        authority.source == *source
                                            && authority.ability_key == *ability_key
                                    })
                        )
                    })
                {
                    invalid!();
                }
                if !reconstructed
                    .card_rules_state
                    .abilities
                    .by_instance
                    .values()
                    .any(|authority| {
                        authority.source == *source && authority.ability_key == *ability_key
                    })
                {
                    invalid!();
                }
                let pool_before = *reconstructed
                    .card_rules_state
                    .mana
                    .pools
                    .get(player)
                    .ok_or(TransitionViolation::SuccessorProduct)?;
                reconstructed
                    .card_rules_state
                    .mana
                    .add(*player, *color, *restriction, *amount)
                    .map_err(|_| TransitionViolation::SuccessorProduct)?;
                let pool_after = reconstructed.card_rules_state.mana.pools[player];
                expected_event_kinds.push(AuthoritativeRuleEventKindV2::ManaPoolChanged {
                    player: *player,
                    previous: pool_before,
                    pool_after,
                    color: Some(*color),
                    amount: *amount,
                });
            }
            SemanticDeltaOperationV2::ManaPoolEmptied {
                player,
                previous_pool,
            } => {
                let pool = reconstructed
                    .card_rules_state
                    .mana
                    .pools
                    .get_mut(player)
                    .ok_or(TransitionViolation::SuccessorProduct)?;
                if pool != previous_pool || *pool == ManaPoolV1::default() {
                    invalid!();
                }
                *pool = ManaPoolV1::default();
                expected_event_kinds.push(AuthoritativeRuleEventKindV2::ManaPoolChanged {
                    player: *player,
                    previous: *previous_pool,
                    pool_after: ManaPoolV1::default(),
                    color: None,
                    amount: 0,
                });
            }
            SemanticDeltaOperationV2::ObjectTapped { object, from, to } => {
                let game_object = reconstructed
                    .predecessor_v5
                    .zones
                    .objects
                    .get_mut(object)
                    .ok_or(TransitionViolation::SuccessorProduct)?;
                if game_object.tapped != *from || from == to {
                    invalid!();
                }
                game_object.tapped = *to;
                if !from && *to && !tapped_for_mana.insert(*object) {
                    invalid!();
                }
                expected_event_kinds.push(AuthoritativeRuleEventKindV2::ObjectTapped {
                    object: *object,
                    from: *from,
                    to: *to,
                });
            }
            SemanticDeltaOperationV2::ObjectEntered {
                old_object,
                new_object,
                from_zone,
                to_zone,
                tapped,
                face,
            } => {
                if *from_zone != mtgml_model::ZoneKind::Hand
                    || *to_zone != mtgml_model::ZoneKind::Battlefield
                    || *tapped
                    || *face != 0
                    || old_object.is_none()
                    || !before_request.candidates.iter().any(|candidate| {
                        matches!(
                            candidate.trusted_binding,
                            mtgml_decision::EngineCandidateBindingV3::PlayLand { object }
                                if Some(object) == *old_object
                        )
                    })
                {
                    invalid!();
                }
                let transition = product
                    .events
                    .iter()
                    .find_map(|event| match &event.event {
                        AuthoritativeRuleEventKindV2::ZoneTransition(transition)
                            if transition.old_object == old_object.unwrap_or(*new_object)
                                && transition.new_object == *new_object =>
                        {
                            Some(transition.as_ref())
                        }
                        _ => None,
                    })
                    .ok_or(TransitionViolation::EventDeltaMismatch)?;
                if transition.from.zone != *from_zone
                    || transition.to.zone != *to_zone
                    || transition.new_snapshot.tapped != *tapped
                    || transition.to.zone
                        != reconstructed
                            .predecessor_v5
                            .zones
                            .locations
                            .get(new_object)
                            .map(|location| location.zone)
                            .unwrap_or(*to_zone)
                {
                    invalid!();
                }
                reconstructed
                    .card_rules_state
                    .faces
                    .faces
                    .insert(*new_object, *face);
                expected_event_kinds.push(AuthoritativeRuleEventKindV2::ZoneTransition(Box::new(
                    transition.clone(),
                )));
                let move_event = product
                    .events
                    .iter()
                    .find_map(|event| match &event.event {
                        AuthoritativeRuleEventKindV2::ObjectMoved {
                            old_object: old,
                            new_object: new,
                            from,
                            to,
                            entering_face,
                            tapped: event_tapped,
                        } if old == &old_object.unwrap_or(*new_object)
                            && new == new_object
                            && from == from_zone
                            && to == to_zone
                            && event_tapped == tapped
                            && matches!(
                                (*entering_face, *face),
                                (Some(crate::BasicLandFaceV1::Front), 0)
                                    | (Some(crate::BasicLandFaceV1::Back), 1)
                            ) =>
                        {
                            Some(event.event.clone())
                        }
                        _ => None,
                    })
                    .ok_or(TransitionViolation::EventDeltaMismatch)?;
                expected_event_kinds.push(move_event);
            }
            SemanticDeltaOperationV2::AbilityAuthorityAdded {
                instance,
                source,
                ability_key,
            } => {
                if *instance != reconstructed.predecessor_v5.allocators.next_ability_id
                    || reconstructed
                        .card_rules_state
                        .abilities
                        .by_instance
                        .insert(
                            *instance,
                            AbilityAuthorityV1 {
                                source: *source,
                                ability_key: *ability_key,
                            },
                        )
                        .is_some()
                {
                    invalid!();
                }
                reconstructed.predecessor_v5.allocators.next_ability_id =
                    mtgml_model::AbilityInstanceId(
                        instance
                            .0
                            .checked_add(1)
                            .ok_or(TransitionViolation::SuccessorProduct)?,
                    );
            }
            SemanticDeltaOperationV2::AbilityAuthorityRemoved {
                instance,
                source,
                ability_key,
            } => {
                let removed = reconstructed
                    .card_rules_state
                    .abilities
                    .by_instance
                    .remove(instance)
                    .ok_or(TransitionViolation::SuccessorProduct)?;
                if removed.source != *source || removed.ability_key != *ability_key {
                    invalid!();
                }
            }
            SemanticDeltaOperationV2::AbilityIdentityChanged {
                perspective,
                instance,
                from,
                to,
            } => {
                let identity = reconstructed
                    .predecessor_v5
                    .perspective_identities
                    .players
                    .get_mut(perspective)
                    .ok_or(TransitionViolation::SuccessorProduct)?;
                let current = identity.ability_to_opaque.get(instance).copied();
                if current != *from {
                    invalid!();
                }
                if let Some(opaque) = from {
                    identity.ability_to_opaque.remove(instance);
                    identity.opaque_to_ability.remove(opaque);
                    identity.retired_ability_ids.insert(*opaque);
                }
                if let Some(opaque) = to {
                    if opaque.0 != identity.next_opaque_ability_id.0
                        || identity
                            .opaque_to_ability
                            .insert(*opaque, *instance)
                            .is_some()
                        || identity
                            .ability_to_opaque
                            .insert(*instance, *opaque)
                            .is_some()
                    {
                        invalid!();
                    }
                    identity.next_opaque_ability_id = mtgml_model::OpaqueAbilityId(
                        opaque
                            .0
                            .checked_add(1)
                            .ok_or(TransitionViolation::SuccessorProduct)?,
                    );
                }
            }
            SemanticDeltaOperationV2::CounterChanged { .. }
            | SemanticDeltaOperationV2::AttachmentChanged { .. }
            | SemanticDeltaOperationV2::ObjectFaceChanged { .. } => invalid!(),
        }
    }

    if position_changed
        && reconstructed
            .card_rules_state
            .mana
            .pools
            .values()
            .any(|pool| *pool != ManaPoolV1::default())
    {
        invalid!();
    }

    if cleared.is_none()
        || created.map(|request| request.decision_id)
            != product
                .next_decision
                .as_ref()
                .map(|request| request.decision_id)
    {
        return Err(TransitionViolation::DecisionEvent);
    }
    let next_request = product
        .next_decision
        .as_ref()
        .ok_or(TransitionViolation::NextDecisionMismatch)?;
    reconstructed.predecessor_v5.revision = expected_revision;
    prune_departed_state(&mut reconstructed);
    next_request
        .validate()
        .map_err(|_| TransitionViolation::SuccessorProduct)?;
    if next_request.decision_id != reconstructed.predecessor_v5.allocators.next_decision_id
        || next_request.player_decision_id
            != reconstructed
                .predecessor_v5
                .perspective_identities
                .players
                .get(&next_request.actor)
                .ok_or(TransitionViolation::SuccessorProduct)?
                .next_player_decision_id
    {
        invalid!();
    }
    let expected_candidates = crate::derive_basic_land_candidates(
        admission,
        &reconstructed,
        next_request.actor,
        &product.status,
    )
    .map_err(|_| TransitionViolation::SuccessorProduct)?;
    if next_request.state_revision != expected_revision
        || next_request.visibility != mtgml_decision::DecisionVisibility::Public
        || next_request.decision != mtgml_decision::DecisionDomainV2::ChooseOne
        || next_request.continuation_id.is_some()
        || next_request.candidates != expected_candidates
    {
        invalid!();
    }
    reconstructed.predecessor_v5.allocators.next_decision_id = mtgml_model::DecisionId(
        next_request
            .decision_id
            .0
            .checked_add(1)
            .ok_or(TransitionViolation::SuccessorProduct)?,
    );
    let actor_identity = reconstructed
        .predecessor_v5
        .perspective_identities
        .players
        .get_mut(&next_request.actor)
        .ok_or(TransitionViolation::SuccessorProduct)?;
    actor_identity.next_player_decision_id = mtgml_model::PlayerDecisionIdV1(
        next_request
            .player_decision_id
            .0
            .checked_add(1)
            .ok_or(TransitionViolation::SuccessorProduct)?,
    );
    reconstructed.execution_v3.pending_decision = Some(next_request.clone());
    reconstructed.predecessor_v5.allocators.next_rule_event_id = mtgml_model::RuleEventId(
        before
            .predecessor_v5
            .allocators
            .next_rule_event_id
            .0
            .checked_add(product.events.len() as u64)
            .ok_or(TransitionViolation::EventIdentity)?,
    );

    prune_departed_state(&mut reconstructed);

    if reconstructed != product.next_state {
        return Err(TransitionViolation::UnexplainedMutation);
    }
    if expected_event_kinds.len() != product.events.len()
        || expected_event_kinds
            .iter()
            .zip(&product.events)
            .any(|(expected, actual)| expected != &actual.event)
    {
        return Err(TransitionViolation::EventDeltaMismatch);
    }
    Ok(())
}

fn prune_departed_state(reconstructed: &mut EngineStatePartsV2) {
    let live: std::collections::BTreeSet<_> = reconstructed
        .predecessor_v5
        .zones
        .objects
        .keys()
        .copied()
        .collect();
    reconstructed
        .card_rules_state
        .counters
        .prune_departed_objects(&live);
    reconstructed
        .card_rules_state
        .attachments
        .prune_departed_objects(&live);
    reconstructed
        .card_rules_state
        .faces
        .prune_departed_objects(&live);
    reconstructed
        .card_rules_state
        .abilities
        .prune_departed_objects(&live);
    reconstructed
        .card_rules_state
        .turn_history
        .target_occurrences
        .retain(|(target, _)| live.contains(target));
    reconstructed
        .card_rules_state
        .turn_history
        .once_ability_used
        .retain(|(source, _)| live.contains(source));
}

fn apply_zone_transition(
    state: &mut EngineStatePartsV2,
    transition: &mtgml_state::ZoneTransition,
) -> Result<(), TransitionViolation> {
    let old = state
        .predecessor_v5
        .zones
        .objects
        .get(&transition.old_object)
        .ok_or(TransitionViolation::SuccessorProduct)?;
    let old_location = state
        .predecessor_v5
        .zones
        .locations
        .get(&transition.old_object)
        .ok_or(TransitionViolation::SuccessorProduct)?;
    if old_location != &transition.from
        || transition.physical_card != old.physical_card
        || old.id != transition.last_known.object
        || old.physical_card != transition.last_known.physical_card
        || old.card_definition != transition.last_known.card_definition
        || old.owner != transition.last_known.owner
        || old.controller != transition.last_known.controller
        || old.tapped != transition.last_known.tapped
        || old.face_down != transition.last_known.face_down
        || transition.new_object != transition.new_snapshot.object
        || transition.new_snapshot.location != transition.to
    {
        invalid!();
    }
    let old_location = state
        .predecessor_v5
        .zones
        .locations
        .remove(&transition.old_object)
        .ok_or(TransitionViolation::SuccessorProduct)?;
    state
        .predecessor_v5
        .zones
        .objects
        .remove(&transition.old_object);
    remove_ordered_member(
        &mut state.predecessor_v5.zones,
        &old_location,
        transition.old_object,
    )?;
    let snapshot = &transition.new_snapshot;
    if state
        .predecessor_v5
        .zones
        .objects
        .contains_key(&snapshot.object)
    {
        invalid!();
    }
    state.predecessor_v5.zones.objects.insert(
        snapshot.object,
        mtgml_state::GameObject {
            id: snapshot.object,
            physical_card: snapshot.physical_card,
            card_definition: snapshot.card_definition,
            owner: snapshot.owner,
            controller: snapshot.controller,
            tapped: snapshot.tapped,
            face_down: snapshot.face_down,
        },
    );
    state
        .predecessor_v5
        .zones
        .locations
        .insert(snapshot.object, snapshot.location.clone());
    if matches!(
        snapshot.location.position,
        mtgml_state::ZonePosition::Top { .. }
    ) {
        insert_ordered_member(
            &mut state.predecessor_v5.zones.ordered_zones,
            &snapshot.location,
            snapshot.object,
        )?;
    }
    state.predecessor_v5.allocators.next_object_id = mtgml_model::GameObjectId(
        snapshot
            .object
            .0
            .checked_add(1)
            .ok_or(TransitionViolation::ObjectAllocatorProgression)?,
    );
    Ok(())
}

fn remove_ordered_member(
    zones: &mut mtgml_state::ZoneState,
    location: &mtgml_state::ZoneLocation,
    object: mtgml_model::GameObjectId,
) -> Result<(), TransitionViolation> {
    if !matches!(location.position, mtgml_state::ZonePosition::Top { .. }) {
        return Ok(());
    }
    let key = location.key();
    if let Some(members) = zones.ordered_zones.get_mut(&key) {
        members.retain(|member| *member != object);
        for (offset, member) in members.iter().enumerate() {
            if let Some(member_location) = zones.locations.get_mut(member) {
                member_location.position = mtgml_state::ZonePosition::Top {
                    offset: u32::try_from(offset)
                        .map_err(|_| TransitionViolation::ZoneOrderProgression)?,
                };
            }
        }
        if members.is_empty() {
            zones.ordered_zones.remove(&key);
        }
    }
    Ok(())
}

fn insert_ordered_member(
    ordered: &mut std::collections::BTreeMap<mtgml_state::ZoneKey, Vec<mtgml_model::GameObjectId>>,
    location: &mtgml_state::ZoneLocation,
    object: mtgml_model::GameObjectId,
) -> Result<(), TransitionViolation> {
    let mtgml_state::ZonePosition::Top { offset } = location.position else {
        return Ok(());
    };
    let members = ordered.entry(location.key()).or_default();
    let index = usize::try_from(offset).map_err(|_| TransitionViolation::ZoneOrderProgression)?;
    if index > members.len() {
        return Err(TransitionViolation::ZoneOrderProgression);
    }
    members.insert(index, object);
    Ok(())
}
