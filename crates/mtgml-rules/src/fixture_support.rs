//! M2.E lifecycle fixture support (test/conformance only).
//!
//! This module exists exclusively behind the non-default
//! `synthetic-conformance-fixtures` feature and is consumed only by
//! `mtgml-conformance` and rule tests. It is NOT a runtime action channel:
//! no EnvironmentBackend, controller, or replay path may call it, so every
//! authoritative state change it produces remains attributable to the normal
//! accepted-product semantics that Replay V3 reexecutes.

use mtgml_model::{GameObjectId, RuleEventId, StateRevision};
use mtgml_random::{RandomStreamKeyV1, RandomStreamKindV1};
use mtgml_state::{
    apply_perspective_lifecycle, EngineState, ObjectSnapshot, ZoneKey, ZoneLocation, ZonePosition,
    ZoneTransition,
};

use crate::errors::KernelExecutionError;
use crate::events::{
    AuthoritativeRuleEvent, AuthoritativeRuleEventKind, PerspectiveObservationPolicyV1,
};
use crate::product::build_accepted_product;
use crate::transition::TransitionResult;

/// One planned perspective-visible occurrence: the state-owned lifecycle
/// audit plus its rules-owned observation policy.
#[derive(Debug, Clone)]
pub struct PlannedOccurrence {
    pub lifecycle: mtgml_state::PerspectiveLifecycleAuditV1,
    pub observation: PerspectiveObservationPolicyV1,
}

/// Builder for one accepted synthetic lifecycle transition. The workspace is
/// mutated through production primitives only; the product is validated by
/// the exact shared epilogue (`validate_transition_contract` included).
pub struct FixtureTransition {
    before: EngineState,
    workspace: EngineState,
    events: Vec<AuthoritativeRuleEvent>,
    offset: u64,
}

impl FixtureTransition {
    pub fn start(before: &EngineState) -> Result<Self, KernelExecutionError> {
        let mut workspace = before.clone();
        workspace.revision = StateRevision(
            workspace
                .revision
                .0
                .checked_add(1)
                .ok_or(KernelExecutionError::RevisionOverflow)?,
        );
        Ok(Self {
            before: before.clone(),
            workspace,
            events: Vec::new(),
            offset: 0,
        })
    }

    fn bind(
        &self,
        kind: AuthoritativeRuleEventKind,
    ) -> Result<AuthoritativeRuleEvent, KernelExecutionError> {
        Ok(AuthoritativeRuleEvent {
            event_id: RuleEventId(
                self.before
                    .allocators
                    .next_rule_event_id
                    .0
                    .checked_add(self.offset)
                    .ok_or(KernelExecutionError::RuleEventIdOverflow)?,
            ),
            state_revision: self.workspace.revision,
            event: kind,
        })
    }

    fn transaction<T>(
        &mut self,
        action: impl FnOnce(&mut Self) -> Result<T, KernelExecutionError>,
    ) -> Result<T, KernelExecutionError> {
        let workspace = self.workspace.clone();
        let events = self.events.clone();
        let offset = self.offset;
        match action(self) {
            Ok(value) => Ok(value),
            Err(error) => {
                self.workspace = workspace;
                self.events = events;
                self.offset = offset;
                Err(error)
            }
        }
    }

    /// Authoritative zone movement creating a fresh incarnation (the frozen
    /// semantic of every synthetic zone transition). Source and target zones
    /// must be unordered; ordered-position bookkeeping stays outside the
    /// M2.E fixture family on purpose.
    pub fn move_object_incarnation(
        &mut self,
        object: GameObjectId,
        to: ZoneLocation,
    ) -> Result<GameObjectId, KernelExecutionError> {
        self.transaction(|transition| transition.move_object_incarnation_inner(object, to))
    }

    fn move_object_incarnation_inner(
        &mut self,
        object: GameObjectId,
        to: ZoneLocation,
    ) -> Result<GameObjectId, KernelExecutionError> {
        if to.position != ZonePosition::Unordered {
            return Err(KernelExecutionError::UnsupportedStagePath);
        }
        let snapshots = crate::snapshots::object_snapshots(&self.workspace)
            .map_err(|_| KernelExecutionError::UnsupportedStagePath)?;
        let old_snapshot =
            snapshots
                .get(&object)
                .cloned()
                .ok_or(KernelExecutionError::AfterState(
                    mtgml_state::EngineStateViolation::ObjectLocationMismatch,
                ))?;
        let from = old_snapshot.location.clone();
        if from == to {
            return Err(KernelExecutionError::UnsupportedStagePath);
        }
        let new_object = self.workspace.allocators.next_object_id;
        self.workspace.allocators.next_object_id = GameObjectId(
            new_object
                .0
                .checked_add(1)
                .ok_or(KernelExecutionError::Exhaustion("object"))?,
        );

        let mut moved = self.workspace.zones.objects.remove(&object).ok_or(
            KernelExecutionError::AfterState(
                mtgml_state::EngineStateViolation::ObjectLocationMismatch,
            ),
        )?;
        moved.id = new_object;
        self.workspace.zones.objects.insert(new_object, moved);
        self.workspace.zones.locations.remove(&object);
        self.workspace
            .zones
            .locations
            .insert(new_object, to.clone());
        if from.position != ZonePosition::Unordered {
            let key: ZoneKey = from.key();
            let remove_key = if let Some(entries) = self.workspace.zones.ordered_zones.get_mut(&key)
            {
                entries.retain(|entry| *entry != object);
                entries.is_empty()
            } else {
                false
            };
            if remove_key {
                self.workspace.zones.ordered_zones.remove(&key);
            }
        }

        let last_known = old_snapshot.clone();
        let new_snapshot = ObjectSnapshot {
            object: new_object,
            physical_card: old_snapshot.physical_card,
            card_definition: old_snapshot.card_definition,
            owner: old_snapshot.owner,
            controller: old_snapshot.controller,
            tapped: old_snapshot.tapped,
            face_down: old_snapshot.face_down,
            location: to.clone(),
        };
        let transition = ZoneTransition {
            old_object: object,
            new_object,
            physical_card: old_snapshot.physical_card,
            from,
            to,
            last_known,
            new_snapshot,
        };
        let event = self.bind(AuthoritativeRuleEventKind::ZoneTransition {
            transition: Box::new(transition),
        });
        self.events.push(event?);
        self.offset = self
            .offset
            .checked_add(1)
            .ok_or(KernelExecutionError::RuleEventIdOverflow)?;
        Ok(new_object)
    }

    /// Applies one perspective-visible occurrence through the authoritative
    /// state primitive and records it as a first-class occurrence event.
    /// The closed pairing matrix is enforced by the contract validation that
    /// [`Self::finish`] runs.
    pub fn apply_occurrence(
        &mut self,
        planned: PlannedOccurrence,
    ) -> Result<(), KernelExecutionError> {
        self.transaction(|transition| transition.apply_occurrence_inner(planned))
    }

    fn apply_occurrence_inner(
        &mut self,
        planned: PlannedOccurrence,
    ) -> Result<(), KernelExecutionError> {
        apply_perspective_lifecycle(&mut self.workspace, &planned.lifecycle).map_err(|_| {
            KernelExecutionError::AfterState(
                mtgml_state::EngineStateViolation::PerspectiveIdentityMismatch,
            )
        })?;
        let event = self.bind(AuthoritativeRuleEventKind::PerspectiveOccurrence {
            lifecycle: planned.lifecycle,
            observation: planned.observation,
        });
        self.events.push(event?);
        self.offset = self
            .offset
            .checked_add(1)
            .ok_or(KernelExecutionError::RuleEventIdOverflow)?;
        Ok(())
    }

    /// Records one authoritative hidden RNG sample (audited event, no
    /// perspective occurrence): the trusted counterpart of a hidden
    /// randomization step inside the fixture program.
    pub fn record_hidden_random_sample(&mut self, bound: u64) -> Result<(), KernelExecutionError> {
        self.transaction(|transition| transition.record_hidden_random_sample_inner(bound))
    }

    fn record_hidden_random_sample_inner(
        &mut self,
        bound: u64,
    ) -> Result<(), KernelExecutionError> {
        let key = RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1);
        let cursor_before = self.workspace.random.lookup_stream(&key)?.next_raw_u64;
        let (value, consumed) = self.workspace.uniform_below_u64(&key, bound)?;
        let cursor_after = self.workspace.random.lookup_stream(&key)?.next_raw_u64;
        let event = self.bind(AuthoritativeRuleEventKind::RandomValueSampled {
            stream: key,
            bound,
            value,
            raw_words_consumed: consumed,
            cursor_before,
            cursor_after,
        })?;
        self.events.push(event);
        self.offset = self
            .offset
            .checked_add(1)
            .ok_or(KernelExecutionError::RuleEventIdOverflow)?;
        Ok(())
    }

    /// Validates and returns the complete accepted product.
    pub fn finish(self) -> Result<TransitionResult, KernelExecutionError> {
        let Self {
            before,
            workspace,
            events,
            ..
        } = self;
        build_accepted_product(&before, workspace, events, |_| Ok(()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mtgml_model::{PlayerId, VisibleSequence, ZoneKind};
    use mtgml_random::RootSeed256;
    use mtgml_state::{
        construct_synthetic_engine_state, PerspectiveLifecycleAuditV1,
        PerspectiveLifecycleMutationV1, SyntheticResetInputs, SyntheticV4Setup,
        VisibilityPartition,
    };

    const P1: PlayerId = PlayerId(1);
    const P2: PlayerId = PlayerId(2);

    fn state() -> EngineState {
        construct_synthetic_engine_state(SyntheticResetInputs {
            players: [P1, P2],
            root_seed: RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap(),
            setup: SyntheticV4Setup::synthetic_compatibility(),
        })
        .unwrap()
    }

    fn location(zone: ZoneKind) -> ZoneLocation {
        ZoneLocation {
            zone,
            player: None,
            position: ZonePosition::Unordered,
            visibility: VisibilityPartition::Public,
            partition: None,
        }
    }

    fn occurrence(sequence: u64) -> PlannedOccurrence {
        PlannedOccurrence {
            lifecycle: PerspectiveLifecycleAuditV1 {
                perspective: P1,
                sequence: VisibleSequence(sequence),
                mutation: PerspectiveLifecycleMutationV1::default(),
            },
            observation: PerspectiveObservationPolicyV1::NoEnvelope,
        }
    }

    #[test]
    fn move_object_rolls_back_when_event_binding_fails() {
        let mut before = state();
        before.allocators.next_rule_event_id = RuleEventId(u64::MAX);
        let mut transition = FixtureTransition::start(&before).unwrap();
        let moved = transition
            .move_object_incarnation(GameObjectId(1), location(ZoneKind::Exile))
            .unwrap();
        let workspace_before = transition.workspace.clone();
        let events_before = transition.events.clone();
        let offset_before = transition.offset;

        assert!(matches!(
            transition.move_object_incarnation(moved, location(ZoneKind::Battlefield)),
            Err(KernelExecutionError::RuleEventIdOverflow)
        ));
        assert_eq!(transition.workspace, workspace_before);
        assert_eq!(transition.events, events_before);
        assert_eq!(transition.offset, offset_before);
    }

    #[test]
    fn occurrence_rolls_back_when_event_binding_fails() {
        let mut before = state();
        before.allocators.next_rule_event_id = RuleEventId(u64::MAX);
        let mut transition = FixtureTransition::start(&before).unwrap();
        transition.apply_occurrence(occurrence(1)).unwrap();
        let workspace_before = transition.workspace.clone();
        let events_before = transition.events.clone();
        let offset_before = transition.offset;

        assert!(matches!(
            transition.apply_occurrence(occurrence(2)),
            Err(KernelExecutionError::RuleEventIdOverflow)
        ));
        assert_eq!(transition.workspace, workspace_before);
        assert_eq!(transition.events, events_before);
        assert_eq!(transition.offset, offset_before);
    }

    #[test]
    fn random_sample_rolls_back_cursor_when_event_binding_fails() {
        let mut before = state();
        before.allocators.next_rule_event_id = RuleEventId(u64::MAX);
        let mut transition = FixtureTransition::start(&before).unwrap();
        transition.record_hidden_random_sample(1000).unwrap();
        let workspace_before = transition.workspace.clone();
        let events_before = transition.events.clone();
        let offset_before = transition.offset;

        assert!(matches!(
            transition.record_hidden_random_sample(1000),
            Err(KernelExecutionError::RuleEventIdOverflow)
        ));
        assert_eq!(transition.workspace, workspace_before);
        assert_eq!(transition.events, events_before);
        assert_eq!(transition.offset, offset_before);
    }

    #[test]
    fn random_sample_requires_the_declared_global_stream() {
        let mut before = state();
        let global = RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1);
        let cursor = before.random.lookup_stream(&global).unwrap();
        before.random.streams.remove(&global);
        before.random.streams.insert(
            RandomStreamKeyV1::player_scoped(RandomStreamKindV1::SyntheticM1, P1.0),
            cursor,
        );
        let mut transition = FixtureTransition::start(&before).unwrap();
        let workspace_before = transition.workspace.clone();

        assert!(matches!(
            transition.record_hidden_random_sample(1000),
            Err(KernelExecutionError::Random(_))
        ));
        assert_eq!(transition.workspace, workspace_before);
    }
}
