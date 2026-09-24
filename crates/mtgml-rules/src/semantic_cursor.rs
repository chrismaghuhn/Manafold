use mtgml_model::{ContinuationId, DecisionId, GameObjectId, PlayerId};
use mtgml_random::{RandomStreamCursorV1, RandomStreamKeyV1, RootSeed256};
use mtgml_state::{
    BeginningStep, CombatState, ContinuationPayloadV2, ContinuationRecordV2, EngineState,
    FoundationCreatureSource, KnowledgeStateV2, ObjectSnapshot, PerspectiveIdentityStateV2,
    PriorityState, SbaGraveyardOwnerOrderV1, SbaSelectedActionV1, TurnPosition,
};
use std::collections::{BTreeMap, BTreeSet};

use crate::turn_structure::derive_ordinary_untap_affected_objects;
use crate::turn_structure::temporal_successor;
use crate::validation::TransitionViolation;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SemanticValidationCursor {
    life: BTreeMap<PlayerId, i64>,
    objects: BTreeMap<GameObjectId, ObjectSnapshot>,
    position: TurnPosition,
    priority: PriorityState,
    combat: Option<CombatState>,
    foundation_sources: BTreeMap<GameObjectId, FoundationCreatureSource>,
    pending_decision: Option<DecisionId>,
    sba_continuation: Option<ContinuationRecordV2>,
    root_seed: RootSeed256,
    random_counters: BTreeMap<RandomStreamKeyV1, u64>,
    active_player: PlayerId,
    turn_number: u64,
    lifecycle_knowledge: KnowledgeStateV2,
    lifecycle_identities: PerspectiveIdentityStateV2,
}

impl SemanticValidationCursor {
    pub(crate) fn from_state(state: &EngineState) -> Result<Self, TransitionViolation> {
        Ok(Self {
            life: state
                .core
                .players
                .iter()
                .map(|(player, player_state)| (*player, player_state.life))
                .collect(),
            objects: crate::snapshots::object_snapshots(state)?,
            position: state.core.position,
            priority: state.core.priority,
            combat: state.combat.clone(),
            foundation_sources: state.foundation_sources.clone(),
            pending_decision: state
                .execution
                .pending_decision
                .as_ref()
                .map(|record| record.request.decision_id),
            sba_continuation: state
                .execution
                .continuations
                .values()
                .find(|record| {
                    matches!(
                        &record.payload,
                        ContinuationPayloadV2::MagicSbaGraveyardOrderV1 { .. }
                    )
                })
                .cloned(),
            root_seed: state.random.root_seed,
            random_counters: state
                .random
                .streams
                .iter()
                .map(|(stream, value)| (*stream, value.next_raw_u64))
                .collect(),
            lifecycle_knowledge: state.knowledge.clone(),
            lifecycle_identities: state.perspective_identities.clone(),
            active_player: state.core.active_player,
            turn_number: state.core.turn_number,
        })
    }

    pub(crate) fn apply(
        &mut self,
        event: &crate::events::AuthoritativeRuleEventKind,
    ) -> Result<(), TransitionViolation> {
        use crate::events::AuthoritativeRuleEventKind;
        match event {
            AuthoritativeRuleEventKind::ZoneTransition { transition } => {
                let current = self
                    .objects
                    .get(&transition.old_object)
                    .ok_or(TransitionViolation::ZoneTransition)?;
                if current != &transition.last_known
                    || transition.old_object == transition.new_object
                    || self.objects.contains_key(&transition.new_object)
                    || transition.last_known.object != transition.old_object
                    || transition.last_known.location != transition.from
                    || transition.new_snapshot.object != transition.new_object
                    || transition.new_snapshot.location != transition.to
                    || transition.last_known.physical_card != transition.physical_card
                    || transition.new_snapshot.physical_card != transition.physical_card
                {
                    return Err(TransitionViolation::ZoneTransition);
                }

                let owner = transition.last_known.owner;
                let selected_locations = match (transition.from.zone, transition.to.zone) {
                    (mtgml_model::ZoneKind::Battlefield, mtgml_model::ZoneKind::Graveyard) => {
                        Some((
                            mtgml_state::ZoneLocation {
                                zone: mtgml_model::ZoneKind::Battlefield,
                                player: None,
                                position: mtgml_state::ZonePosition::Unordered,
                                visibility: mtgml_state::VisibilityPartition::Public,
                                partition: None,
                            },
                            mtgml_state::ZoneLocation {
                                zone: mtgml_model::ZoneKind::Graveyard,
                                player: Some(owner),
                                position: mtgml_state::ZonePosition::Top { offset: 0 },
                                visibility: mtgml_state::VisibilityPartition::Public,
                                partition: None,
                            },
                        ))
                    }
                    (mtgml_model::ZoneKind::Library, mtgml_model::ZoneKind::Hand) => Some((
                        mtgml_state::ZoneLocation {
                            zone: mtgml_model::ZoneKind::Library,
                            player: Some(owner),
                            position: mtgml_state::ZonePosition::Top { offset: 0 },
                            visibility: mtgml_state::VisibilityPartition::FaceDown,
                            partition: None,
                        },
                        mtgml_state::ZoneLocation {
                            zone: mtgml_model::ZoneKind::Hand,
                            player: Some(owner),
                            position: mtgml_state::ZonePosition::Unordered,
                            visibility: mtgml_state::VisibilityPartition::OwnerOnly,
                            partition: None,
                        },
                    )),
                    _ => None,
                };
                if let Some((expected_from, expected_to)) = selected_locations {
                    if transition.from != expected_from
                        || transition.to != expected_to
                        || transition.physical_card.is_none()
                        || transition.last_known.card_definition
                            != transition.new_snapshot.card_definition
                        || transition.last_known.owner != transition.new_snapshot.owner
                        || transition.new_snapshot.controller != owner
                        || transition.new_snapshot.tapped
                        || transition.new_snapshot.face_down
                    {
                        return Err(TransitionViolation::ZoneTransition);
                    }
                    if matches!(transition.from.zone, mtgml_model::ZoneKind::Battlefield)
                        && transition.last_known.face_down
                    {
                        return Err(TransitionViolation::ZoneTransition);
                    }
                }

                // A selected ordered-zone move also changes the redundant
                // `Top` witnesses of the other objects in that zone. Keep
                // the cursor's object projection compositional for these two
                // frozen families; EngineState validation cross-checks the
                // final vectors against these exact locations.
                let battlefield = mtgml_state::ZoneLocation {
                    zone: mtgml_model::ZoneKind::Battlefield,
                    player: None,
                    position: mtgml_state::ZonePosition::Unordered,
                    visibility: mtgml_state::VisibilityPartition::Public,
                    partition: None,
                };
                let graveyard_top = mtgml_state::ZoneLocation {
                    zone: mtgml_model::ZoneKind::Graveyard,
                    player: Some(transition.last_known.owner),
                    position: mtgml_state::ZonePosition::Top { offset: 0 },
                    visibility: mtgml_state::VisibilityPartition::Public,
                    partition: None,
                };
                let selected_battlefield_graveyard =
                    transition.from == battlefield && transition.to == graveyard_top;
                let library_top = mtgml_state::ZoneLocation {
                    zone: mtgml_model::ZoneKind::Library,
                    player: Some(transition.last_known.owner),
                    position: mtgml_state::ZonePosition::Top { offset: 0 },
                    visibility: mtgml_state::VisibilityPartition::FaceDown,
                    partition: None,
                };
                let hand = mtgml_state::ZoneLocation {
                    zone: mtgml_model::ZoneKind::Hand,
                    player: Some(transition.last_known.owner),
                    position: mtgml_state::ZonePosition::Unordered,
                    visibility: mtgml_state::VisibilityPartition::OwnerOnly,
                    partition: None,
                };
                let selected_library_hand = transition.from == library_top && transition.to == hand;

                if selected_battlefield_graveyard {
                    if self.objects.values().any(|snapshot| {
                        snapshot.location.key() == transition.to.key() && snapshot.face_down
                    }) {
                        return Err(TransitionViolation::ZoneTransition);
                    }
                    self.foundation_sources.remove(&transition.old_object);
                    let destination_key = transition.to.key();
                    for (object, snapshot) in &mut self.objects {
                        if *object == transition.old_object
                            || snapshot.location.key() != destination_key
                        {
                            continue;
                        }
                        let mtgml_state::ZonePosition::Top { offset } = snapshot.location.position
                        else {
                            return Err(TransitionViolation::ZoneTransition);
                        };
                        snapshot.location.position = mtgml_state::ZonePosition::Top {
                            offset: offset
                                .checked_add(1)
                                .ok_or(TransitionViolation::ZoneTransition)?,
                        };
                    }
                } else if selected_library_hand {
                    let source_key = transition.from.key();
                    for (object, snapshot) in &mut self.objects {
                        if *object == transition.old_object || snapshot.location.key() != source_key
                        {
                            continue;
                        }
                        let mtgml_state::ZonePosition::Top { offset } = snapshot.location.position
                        else {
                            return Err(TransitionViolation::ZoneTransition);
                        };
                        snapshot.location.position = mtgml_state::ZonePosition::Top {
                            offset: offset
                                .checked_sub(1)
                                .ok_or(TransitionViolation::ZoneTransition)?,
                        };
                    }
                }
                self.objects.remove(&transition.old_object);
                self.objects
                    .insert(transition.new_object, transition.new_snapshot.clone());
            }
            AuthoritativeRuleEventKind::ObjectCeasedToExist { object } => {
                if self.objects.remove(object).is_none() {
                    return Err(TransitionViolation::ObjectCessation);
                }
            }
            AuthoritativeRuleEventKind::LifeChanged { player, from, to } => {
                let current = self
                    .life
                    .get_mut(player)
                    .ok_or(TransitionViolation::LifeChange)?;
                if from == to || *current != *from {
                    return Err(TransitionViolation::LifeChange);
                }
                *current = *to;
            }
            AuthoritativeRuleEventKind::ObjectTapped { object, from, to } => {
                let current = self
                    .objects
                    .get_mut(object)
                    .ok_or(TransitionViolation::TapChange)?;
                if from == to || current.tapped != *from {
                    return Err(TransitionViolation::TapChange);
                }
                current.tapped = *to;
            }
            AuthoritativeRuleEventKind::DecisionCreated { decision } => {
                if self.pending_decision.replace(*decision).is_some() {
                    return Err(TransitionViolation::DecisionEvent);
                }
            }
            AuthoritativeRuleEventKind::DecisionCleared { decision } => {
                if self.pending_decision != Some(*decision) {
                    return Err(TransitionViolation::DecisionEvent);
                }
                self.pending_decision = None;
            }
            AuthoritativeRuleEventKind::SbaGraveyardOrderChosen {
                continuation,
                owner,
                top_to_bottom,
            } => self.apply_sba_order_chosen(*continuation, *owner, top_to_bottom)?,
            AuthoritativeRuleEventKind::RandomValueSampled {
                stream,
                bound,
                value,
                raw_words_consumed,
                cursor_before,
                cursor_after,
            } => {
                let current = self
                    .random_counters
                    .get(stream)
                    .copied()
                    .ok_or(TransitionViolation::Randomness)?;
                if current != *cursor_before {
                    return Err(TransitionViolation::Randomness);
                }
                let current_cursor = RandomStreamCursorV1 {
                    next_raw_u64: current,
                };
                let (expected_value, expected_consumed, expected_cursor) =
                    mtgml_random::sampling::uniform_below_u64(
                        &self.root_seed,
                        stream,
                        &current_cursor,
                        *bound,
                    )
                    .map_err(|_| TransitionViolation::Randomness)?;
                if expected_value != *value
                    || expected_consumed != *raw_words_consumed
                    || expected_cursor.next_raw_u64 != *cursor_after
                {
                    return Err(TransitionViolation::Randomness);
                }
                self.random_counters
                    .insert(*stream, expected_cursor.next_raw_u64);
            }
            AuthoritativeRuleEventKind::PublicOutcome { code } => {
                if code.is_empty() {
                    return Err(TransitionViolation::PublicOutcome);
                }
            }
            AuthoritativeRuleEventKind::TurnPositionChanged { from, to } => {
                if self.position != *from || temporal_successor(*from) != *to {
                    return Err(TransitionViolation::TurnStructure);
                }
                self.position = *to;
            }
            AuthoritativeRuleEventKind::UntapCompleted { affected_objects } => {
                if !matches!(
                    self.position,
                    TurnPosition::Beginning {
                        step: BeginningStep::Untap,
                    }
                ) {
                    return Err(TransitionViolation::TurnStructure);
                }
                let expected =
                    derive_ordinary_untap_affected_objects(&self.objects, self.active_player);
                if *affected_objects != expected {
                    return Err(TransitionViolation::TurnStructure);
                }
                for object_id in affected_objects {
                    if let Some(object) = self.objects.get_mut(object_id) {
                        object.tapped = false;
                    }
                }
            }
            AuthoritativeRuleEventKind::ActivePlayerChanged { from, to } => {
                // Task 7: ActivePlayerChanged represents the exact
                // two-player S1 unique-other relation. derived from
                // the cursor's authoritative player set.
                if self.life.len() != 2 || self.active_player != *from || from == to {
                    return Err(TransitionViolation::TurnStructure);
                }
                let unique_other = self
                    .life
                    .keys()
                    .find(|&&p| p != *from)
                    .ok_or(TransitionViolation::TurnStructure)?;
                if *to != *unique_other {
                    return Err(TransitionViolation::TurnStructure);
                }
                self.active_player = *to;
            }
            AuthoritativeRuleEventKind::TurnNumberChanged { from, to } => {
                if self.turn_number != *from || from.checked_add(1) != Some(*to) {
                    return Err(TransitionViolation::TurnStructure);
                }
                self.turn_number = *to;
            }
            AuthoritativeRuleEventKind::PerspectiveOccurrence { lifecycle, .. } => {
                let Self {
                    objects,
                    lifecycle_knowledge,
                    lifecycle_identities,
                    ..
                } = self;
                let knowledge = lifecycle_knowledge
                    .players
                    .get_mut(&lifecycle.perspective)
                    .ok_or(TransitionViolation::OccurrencePairing)?;
                let identity = lifecycle_identities
                    .players
                    .get_mut(&lifecycle.perspective)
                    .ok_or(TransitionViolation::OccurrencePairing)?;
                mtgml_state::apply_lifecycle_to_player(
                    knowledge,
                    identity,
                    &|object| objects.contains_key(&object),
                    lifecycle,
                )
                .map_err(|_| TransitionViolation::OccurrencePairing)?;
            }
        }
        Ok(())
    }

    fn apply_sba_order_chosen(
        &mut self,
        continuation_id: ContinuationId,
        owner: PlayerId,
        top_to_bottom: &[GameObjectId],
    ) -> Result<(), TransitionViolation> {
        let record = self
            .sba_continuation
            .as_ref()
            .filter(|record| record.id == continuation_id)
            .ok_or(TransitionViolation::SbaOrder)?;
        let ContinuationPayloadV2::MagicSbaGraveyardOrderV1 {
            selected_sba_actions,
            apnap_owners,
            next_owner_index,
            completed_owner_orders,
            ..
        } = &record.payload
        else {
            return Err(TransitionViolation::SbaOrder);
        };

        let index =
            usize::try_from(*next_owner_index).map_err(|_| TransitionViolation::SbaOrder)?;
        if record.actor != owner
            || apnap_owners.get(index) != Some(&owner)
            || completed_owner_orders.len() != index
            || top_to_bottom.len() < 2
        {
            return Err(TransitionViolation::SbaOrder);
        }

        let mut selected_for_owner = BTreeSet::new();
        for action in selected_sba_actions {
            if let SbaSelectedActionV1::ObjectToOwnerGraveyard { object, .. } = action {
                let snapshot = self
                    .objects
                    .get(object)
                    .ok_or(TransitionViolation::SbaOrder)?;
                if snapshot.owner == owner {
                    selected_for_owner.insert(*object);
                }
            }
        }
        let chosen: BTreeSet<_> = top_to_bottom.iter().copied().collect();
        if selected_for_owner.len() != top_to_bottom.len()
            || chosen.len() != top_to_bottom.len()
            || chosen != selected_for_owner
        {
            return Err(TransitionViolation::SbaOrder);
        }

        let next_index = next_owner_index
            .checked_add(1)
            .ok_or(TransitionViolation::SbaOrder)?;
        let next_actor = apnap_owners
            .get(usize::try_from(next_index).map_err(|_| TransitionViolation::SbaOrder)?)
            .copied()
            // Task 7 accepts only intermediate order stages. The final choice
            // is owned by Task 9's atomic choice-plus-SBA transition.
            .ok_or(TransitionViolation::SbaOrder)?;
        let record = self
            .sba_continuation
            .as_mut()
            .filter(|record| record.id == continuation_id)
            .ok_or(TransitionViolation::SbaOrder)?;
        let ContinuationPayloadV2::MagicSbaGraveyardOrderV1 {
            next_owner_index,
            completed_owner_orders,
            ..
        } = &mut record.payload
        else {
            return Err(TransitionViolation::SbaOrder);
        };
        completed_owner_orders.push(SbaGraveyardOwnerOrderV1 {
            owner,
            top_to_bottom: top_to_bottom.to_vec(),
        });
        *next_owner_index = next_index;
        record.actor = next_actor;
        record.stage_index =
            u16::try_from(next_index).map_err(|_| TransitionViolation::SbaOrder)?;
        Ok(())
    }

    pub(crate) fn validate_final_state(
        &self,
        after: &EngineState,
    ) -> Result<(), TransitionViolation> {
        if self.position != after.core.position
            || self.active_player != after.core.active_player
            || self.turn_number != after.core.turn_number
            || self.priority != after.core.priority
            || self.combat != after.combat
            || self.foundation_sources != after.foundation_sources
        {
            return Err(TransitionViolation::UnexplainedMutation);
        }
        if let Some(expected) = &self.sba_continuation {
            if after.execution.continuations.get(&expected.id) != Some(expected) {
                return Err(TransitionViolation::SbaOrder);
            }
        }
        let after_life: BTreeMap<_, _> = after
            .core
            .players
            .iter()
            .map(|(player, state)| (*player, state.life))
            .collect();
        if self.life != after_life {
            return Err(TransitionViolation::LifeChange);
        }
        if self.objects != crate::snapshots::object_snapshots(after)? {
            return Err(TransitionViolation::ObjectTraceIncomplete);
        }
        let after_pending = after
            .execution
            .pending_decision
            .as_ref()
            .map(|record| record.request.decision_id);
        if self.pending_decision != after_pending {
            return Err(TransitionViolation::DecisionEvent);
        }
        if self.root_seed != after.random.root_seed {
            return Err(TransitionViolation::Randomness);
        }
        let after_counters: BTreeMap<_, _> = after
            .random
            .streams
            .iter()
            .map(|(stream, value)| (*stream, value.next_raw_u64))
            .collect();
        if self.random_counters != after_counters {
            return Err(TransitionViolation::Randomness);
        }
        if self.lifecycle_knowledge != after.knowledge
            || !lifecycle_identities_match(
                &self.lifecycle_identities,
                &after.perspective_identities,
            )
        {
            return Err(TransitionViolation::OccurrencePairing);
        }
        Ok(())
    }
}

/// Occurrence parity owns every identity component except
/// `next_player_decision_id`, whose allocation is governed by the decision
/// protocol (fresh stage identities) rather than by lifecycle occurrences.
fn lifecycle_identities_match(
    replayed: &PerspectiveIdentityStateV2,
    after: &PerspectiveIdentityStateV2,
) -> bool {
    replayed.players.keys().count() == after.players.keys().count()
        && replayed
            .players
            .iter()
            .all(|(player, record)| match after.players.get(player) {
                Some(other) => {
                    record.opaque_to_object == other.opaque_to_object
                        && record.opaque_to_ability == other.opaque_to_ability
                        && record.object_to_opaque == other.object_to_opaque
                        && record.ability_to_opaque == other.ability_to_opaque
                        && record.next_opaque_object_id == other.next_opaque_object_id
                        && record.next_opaque_ability_id == other.next_opaque_ability_id
                        && record.retired_object_ids == other.retired_object_ids
                        && record.retired_ability_ids == other.retired_ability_ids
                }
                None => false,
            })
}
