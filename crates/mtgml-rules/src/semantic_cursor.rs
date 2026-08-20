use mtgml_model::{DecisionId, GameObjectId, PlayerId};
use mtgml_random::RandomStreamKeyV1;
use mtgml_state::{EngineState, ObjectSnapshot};
use std::collections::BTreeMap;

use crate::validation::TransitionViolation;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SemanticValidationCursor {
    life: BTreeMap<PlayerId, i64>,
    objects: BTreeMap<GameObjectId, ObjectSnapshot>,
    pending_decision: Option<DecisionId>,
    random_counters: BTreeMap<RandomStreamKeyV1, u64>,
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
            random_counters: state
                .random
                .streams
                .iter()
                .map(|(stream, value)| (*stream, value.next_raw_u64))
                .collect(),
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
            AuthoritativeRuleEventKind::PublicOutcome { code } => {
                if code.is_empty() {
                    return Err(TransitionViolation::PublicOutcome);
                }
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
        let after_counters: BTreeMap<_, _> = after
            .random
            .streams
            .iter()
            .map(|(stream, value)| (*stream, value.next_raw_u64))
            .collect();
        if self.random_counters != after_counters {
            return Err(TransitionViolation::Randomness);
        }
        Ok(())
    }
}
