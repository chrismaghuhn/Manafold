use mtgml_model::{DecisionId, GameObjectId, PlayerId};
use mtgml_random::{RandomStreamCursorV1, RandomStreamKeyV1, RootSeed256};
use mtgml_state::{EngineState, KnowledgeStateV2, ObjectSnapshot, PerspectiveIdentityStateV2};
use std::collections::BTreeMap;

use crate::validation::TransitionViolation;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SemanticValidationCursor {
    life: BTreeMap<PlayerId, i64>,
    objects: BTreeMap<GameObjectId, ObjectSnapshot>,
    pending_decision: Option<DecisionId>,
    root_seed: RootSeed256,
    random_counters: BTreeMap<RandomStreamKeyV1, u64>,
    /// M2.E lifecycle shadows: knowledge state and perspective identities are
    /// replayed through the authoritative occurrence payloads so sequential
    /// event parity covers every knowledge/identity/cursor mutation.
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
            pending_decision: state
                .execution
                .pending_decision
                .as_ref()
                .map(|record| record.request.decision_id),
            root_seed: state.random.root_seed,
            random_counters: state
                .random
                .streams
                .iter()
                .map(|(stream, value)| (*stream, value.next_raw_u64))
                .collect(),
            lifecycle_knowledge: state.knowledge.clone(),
            lifecycle_identities: state.perspective_identities.clone(),
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
                if *current != *from {
                    return Err(TransitionViolation::LifeChange);
                }
                *current = *to;
            }
            AuthoritativeRuleEventKind::ObjectTapped { object, from, to } => {
                let current = self
                    .objects
                    .get_mut(object)
                    .ok_or(TransitionViolation::TapChange)?;
                if current.tapped != *from {
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

    pub(crate) fn validate_final_state(
        &self,
        after: &EngineState,
    ) -> Result<(), TransitionViolation> {
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
