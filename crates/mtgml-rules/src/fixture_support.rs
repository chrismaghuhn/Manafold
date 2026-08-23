//! M2.E lifecycle fixture support (test/conformance only).
//!
//! This module exists exclusively behind the non-default
//! `m2-conformance-fixtures` feature and is consumed only by
//! `mtgml-conformance` and rule tests. It is NOT a runtime action channel:
//! no EnvironmentBackend, controller, or replay path may call it, so every
//! authoritative state change it produces remains attributable to the normal
//! accepted-product semantics that Replay V3 reexecutes.

use mtgml_model::{GameObjectId, RuleEventId, StateRevision};
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

    /// Authoritative zone movement creating a fresh incarnation (the frozen
    /// semantic of every synthetic zone transition). Source and target zones
    /// must be unordered; ordered-position bookkeeping stays outside the
    /// M2.E fixture family on purpose.
    pub fn move_object_incarnation(
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
            if let Some(entries) = self.workspace.zones.ordered_zones.get_mut(&key) {
                entries.retain(|entry| *entry != object);
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
        self.offset += 1;
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
        self.offset += 1;
        Ok(())
    }

    /// Records one authoritative hidden RNG sample (audited event, no
    /// perspective occurrence): the trusted counterpart of a hidden
    /// randomization step inside the fixture program.
    pub fn record_hidden_random_sample(&mut self, bound: u64) -> Result<(), KernelExecutionError> {
        let key = *self
            .workspace
            .random
            .streams
            .keys()
            .next()
            .ok_or(KernelExecutionError::UnsupportedStagePath)?;
        let cursor_before = self.workspace.random.streams[&key].next_raw_u64;
        let (value, consumed) = self
            .workspace
            .uniform_below_u64(&key, bound)
            .map_err(|_| KernelExecutionError::UnsupportedStagePath)?;
        let cursor_after = self.workspace.random.streams[&key].next_raw_u64;
        let event = self.bind(AuthoritativeRuleEventKind::RandomValueSampled {
            stream: key,
            bound,
            value,
            raw_words_consumed: consumed,
            cursor_before,
            cursor_after,
        })?;
        self.events.push(event);
        self.offset += 1;
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
