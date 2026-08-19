//! Authoritative events and exact, compositional transition validation.

use mtgml_decision::PlayerDecisionRequest;
use mtgml_model::{DecisionId, EpisodeStatus, GameObjectId, PlayerId, RuleEventId, StateRevision};
use mtgml_state::{
    validate_engine_state, EngineState, EngineStateViolation, ObjectSnapshot,
    SemanticDeltaOperation, StateDelta, ZoneTransition,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AuthoritativeRuleEventKind {
    ZoneTransition {
        transition: Box<ZoneTransition>,
    },
    ObjectCeasedToExist {
        object: GameObjectId,
    },
    LifeChanged {
        player: PlayerId,
        from: i64,
        to: i64,
    },
    ObjectTapped {
        object: GameObjectId,
        from: bool,
        to: bool,
    },
    DecisionCreated {
        decision: DecisionId,
    },
    DecisionCleared {
        decision: DecisionId,
    },
    RandomnessConsumed {
        stream: String,
        counter_before: u64,
        counter_after: u64,
        exclusive_upper_bound: u64,
        value: u64,
    },
    PublicOutcome {
        code: String,
    },
}

impl AuthoritativeRuleEventKind {
    pub fn semantic_delta(&self) -> SemanticDeltaOperation {
        match self {
            Self::ZoneTransition { transition } => SemanticDeltaOperation::ZoneTransition {
                transition: transition.clone(),
            },
            Self::ObjectCeasedToExist { object } => {
                SemanticDeltaOperation::ObjectCeasedToExist { object: *object }
            }
            Self::LifeChanged { player, from, to } => SemanticDeltaOperation::LifeChanged {
                player: *player,
                from: *from,
                to: *to,
            },
            Self::ObjectTapped { object, from, to } => SemanticDeltaOperation::ObjectTapped {
                object: *object,
                from: *from,
                to: *to,
            },
            Self::DecisionCreated { decision } => SemanticDeltaOperation::DecisionCreated {
                decision: *decision,
            },
            Self::DecisionCleared { decision } => SemanticDeltaOperation::DecisionCleared {
                decision: *decision,
            },
            Self::RandomnessConsumed {
                stream,
                counter_before,
                counter_after,
                exclusive_upper_bound,
                value,
            } => SemanticDeltaOperation::RandomStreamAdvanced {
                stream: stream.clone(),
                counter_before: *counter_before,
                counter_after: *counter_after,
                exclusive_upper_bound: *exclusive_upper_bound,
                value: *value,
            },
            Self::PublicOutcome { code } => {
                SemanticDeltaOperation::PublicOutcome { code: code.clone() }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoritativeRuleEvent {
    pub event_id: RuleEventId,
    pub state_revision: StateRevision,
    pub event: AuthoritativeRuleEventKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionResult {
    pub accepted: bool,
    pub next_state: EngineState,
    pub delta: StateDelta,
    pub events: Vec<AuthoritativeRuleEvent>,
    pub next_decision: Option<PlayerDecisionRequest>,
    pub status: EpisodeStatus,
}

pub trait RulesKernel: Send {
    fn apply(
        &mut self,
        state: &EngineState,
        response: &mtgml_decision::DecisionResponse,
    ) -> TransitionResult;
}

/// Validate an atomic transition while interpreting its semantic events in
/// sequence. The outer state still advances in one revision; the cursor exists
/// only so repeated changes to the same semantic value remain compositional.
pub fn validate_transition_contract(
    before: &EngineState,
    result: &TransitionResult,
) -> Result<(), TransitionViolation> {
    validate_engine_state(before).map_err(TransitionViolation::BeforeState)?;
    validate_engine_state(&result.next_state).map_err(TransitionViolation::AfterState)?;

    let reapplied = result
        .delta
        .apply(before)
        .map_err(|_| TransitionViolation::DeltaReapplication)?;
    if reapplied != result.next_state {
        return Err(TransitionViolation::DeltaReapplication);
    }

    if !result.accepted {
        if &result.next_state != before
            || !result.events.is_empty()
            || !result.delta.audit.is_empty()
            || result.delta.before_revision != result.delta.after_revision
            || result.delta.before_digest != result.delta.after_digest
            || !matches!(&result.status, EpisodeStatus::Running)
        {
            return Err(TransitionViolation::RejectedMutation);
        }
    } else if result.next_state.revision.0 <= before.revision.0 {
        return Err(TransitionViolation::RevisionDidNotAdvance);
    } else {
        let before_decision = before
            .execution
            .pending_decision
            .as_ref()
            .map(|record| record.request.decision_id);
        let after_decision = result
            .next_state
            .execution
            .pending_decision
            .as_ref()
            .map(|record| record.request.decision_id);
        if before_decision.is_some() && before_decision == after_decision {
            return Err(TransitionViolation::DecisionIdentityReused);
        }
    }

    let event_audit: Vec<_> = result
        .events
        .iter()
        .map(|event| event.event.semantic_delta())
        .collect();
    if event_audit != result.delta.audit {
        return Err(TransitionViolation::EventDeltaMismatch);
    }

    let mut seen = BTreeSet::new();
    let mut cursor = SemanticValidationCursor::from_state(before)?;
    for (offset, event) in result.events.iter().enumerate() {
        let offset = u64::try_from(offset).map_err(|_| TransitionViolation::EventIdentity)?;
        let expected = before
            .allocators
            .next_rule_event_id
            .0
            .checked_add(offset)
            .ok_or(TransitionViolation::EventIdentity)?;
        if event.event_id.0 != expected
            || event.state_revision != result.next_state.revision
            || !seen.insert(event.event_id)
        {
            return Err(TransitionViolation::EventIdentity);
        }
        cursor.apply(&event.event)?;
    }
    cursor.validate_final_state(&result.next_state)?;

    let event_count =
        u64::try_from(result.events.len()).map_err(|_| TransitionViolation::EventIdentity)?;
    let expected_next_event = before
        .allocators
        .next_rule_event_id
        .0
        .checked_add(event_count)
        .ok_or(TransitionViolation::EventIdentity)?;
    if result.next_state.allocators.next_rule_event_id.0 != expected_next_event {
        return Err(TransitionViolation::EventIdentity);
    }

    let pending = result
        .next_state
        .execution
        .pending_decision
        .as_ref()
        .map(|record| &record.request);
    if pending != result.next_decision.as_ref() {
        return Err(TransitionViolation::NextDecisionMismatch);
    }
    if let Some(decision) = &result.next_decision {
        if decision.state_revision != result.next_state.revision {
            return Err(TransitionViolation::NextDecisionMismatch);
        }
    }
    if !matches!(&result.status, EpisodeStatus::Running) && result.next_decision.is_some() {
        return Err(TransitionViolation::TerminalDecision);
    }
    result
        .status
        .validate()
        .map_err(|_| TransitionViolation::EpisodeStatus)?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SemanticValidationCursor {
    life: BTreeMap<PlayerId, i64>,
    objects: BTreeMap<GameObjectId, ObjectSnapshot>,
    pending_decision: Option<DecisionId>,
    random_counters: BTreeMap<String, u64>,
}

impl SemanticValidationCursor {
    fn from_state(state: &EngineState) -> Result<Self, TransitionViolation> {
        Ok(Self {
            life: state
                .core
                .players
                .iter()
                .map(|(player, player_state)| (*player, player_state.life))
                .collect(),
            objects: object_snapshots(state)?,
            pending_decision: state
                .execution
                .pending_decision
                .as_ref()
                .map(|record| record.request.decision_id),
            random_counters: state
                .random
                .streams
                .iter()
                .map(|(stream, value)| (stream.clone(), value.counter))
                .collect(),
        })
    }

    fn apply(&mut self, event: &AuthoritativeRuleEventKind) -> Result<(), TransitionViolation> {
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
            AuthoritativeRuleEventKind::RandomnessConsumed {
                stream,
                counter_before,
                counter_after,
                exclusive_upper_bound,
                value,
            } => {
                let counter = self
                    .random_counters
                    .get_mut(stream)
                    .ok_or(TransitionViolation::Randomness)?;
                if *exclusive_upper_bound == 0
                    || *value >= *exclusive_upper_bound
                    || (*counter_before).checked_add(1) != Some(*counter_after)
                    || *counter != *counter_before
                {
                    return Err(TransitionViolation::Randomness);
                }
                *counter = *counter_after;
            }
            AuthoritativeRuleEventKind::PublicOutcome { code } => {
                if code.is_empty() {
                    return Err(TransitionViolation::PublicOutcome);
                }
            }
        }
        Ok(())
    }

    fn validate_final_state(&self, after: &EngineState) -> Result<(), TransitionViolation> {
        let after_life: BTreeMap<_, _> = after
            .core
            .players
            .iter()
            .map(|(player, state)| (*player, state.life))
            .collect();
        if self.life != after_life {
            return Err(TransitionViolation::LifeChange);
        }
        if self.objects != object_snapshots(after)? {
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
            .map(|(stream, value)| (stream.clone(), value.counter))
            .collect();
        if self.random_counters != after_counters {
            return Err(TransitionViolation::Randomness);
        }
        Ok(())
    }
}

fn object_snapshots(
    state: &EngineState,
) -> Result<BTreeMap<GameObjectId, ObjectSnapshot>, TransitionViolation> {
    state
        .zones
        .objects
        .iter()
        .map(|(id, object)| {
            let location = state
                .zones
                .locations
                .get(id)
                .ok_or(TransitionViolation::ObjectTraceIncomplete)?;
            Ok((
                *id,
                ObjectSnapshot {
                    object: *id,
                    physical_card: object.physical_card,
                    card_definition: object.card_definition,
                    owner: object.owner,
                    controller: object.controller,
                    tapped: object.tapped,
                    face_down: object.face_down,
                    location: location.clone(),
                },
            ))
        })
        .collect()
}

#[derive(Debug, Error)]
pub enum TransitionViolation {
    #[error("before state is invalid: {0}")]
    BeforeState(EngineStateViolation),
    #[error("after state is invalid: {0}")]
    AfterState(EngineStateViolation),
    #[error("state delta does not exactly reconstruct next state")]
    DeltaReapplication,
    #[error("a rejected response changed authoritative state, RNG, IDs, or events")]
    RejectedMutation,
    #[error("accepted transition did not advance the revision")]
    RevisionDidNotAdvance,
    #[error("semantic event trace and semantic delta audit differ")]
    EventDeltaMismatch,
    #[error("event IDs are not contiguous or revision-bound")]
    EventIdentity,
    #[error("next decision differs from checkpointed execution state")]
    NextDecisionMismatch,
    #[error("terminal or truncated state still exposes a decision")]
    TerminalDecision,
    #[error("episode status is invalid")]
    EpisodeStatus,
    #[error("zone transition identity, snapshots, or last-known information is invalid")]
    ZoneTransition,
    #[error("object cessation event does not match the semantic cursor")]
    ObjectCessation,
    #[error("object trace does not compose to the final state")]
    ObjectTraceIncomplete,
    #[error("life event sequence does not compose to the final state")]
    LifeChange,
    #[error("tap event sequence does not compose to the final state")]
    TapChange,
    #[error("decision event sequence does not compose to the final state")]
    DecisionEvent,
    #[error("an accepted response reused the consumed decision identity")]
    DecisionIdentityReused,
    #[error("randomness event sequence does not match checkpointed stream state")]
    Randomness,
    #[error("public outcome code is empty")]
    PublicOutcome,
}

#[cfg(test)]
mod tests {
    use super::*;
    use mtgml_decision::{DecisionKind, DecisionVisibility};
    use mtgml_model::{
        AbilityInstanceId, CardDefinitionId, ContinuationId, EffectInstanceId, OpaqueAbilityId,
        OpaqueObjectId, PhysicalCardId, StackObjectId, TriggerInstanceId, ZoneKind,
    };
    use mtgml_random::{RandomState, RandomStreamState};
    use mtgml_state::{
        CoreRulesState, ExecutionState, FormatState, GameObject, IdentityAllocatorState,
        KnowledgeState, PendingDecisionRecord, PerspectiveIdentityMap, PerspectiveIdentityState,
        PlayerKnowledgeState, PlayerState, VisibilityPartition, ZoneLocation, ZonePosition,
        ZoneState,
    };

    fn request(id: u64, revision: u64) -> PlayerDecisionRequest {
        PlayerDecisionRequest {
            schema_version: "player-decision-request.v1".into(),
            decision_id: DecisionId(id),
            state_revision: StateRevision(revision),
            actor: PlayerId(1),
            visibility: DecisionVisibility::Public,
            decision: DecisionKind::ChooseNumber {
                minimum: 0,
                maximum: 0,
            },
            candidates: vec![],
        }
    }

    fn state() -> EngineState {
        let p1 = PlayerId(1);
        let p2 = PlayerId(2);
        EngineState {
            revision: StateRevision(0),
            core: CoreRulesState {
                players: BTreeMap::from([
                    (
                        p1,
                        PlayerState {
                            life: 40,
                            has_lost: false,
                        },
                    ),
                    (
                        p2,
                        PlayerState {
                            life: 40,
                            has_lost: false,
                        },
                    ),
                ]),
                active_player: p1,
                priority_player: p1,
                turn_number: 1,
            },
            zones: ZoneState::default(),
            allocators: IdentityAllocatorState {
                next_object_id: GameObjectId(1),
                next_ability_id: AbilityInstanceId(1),
                next_stack_object_id: StackObjectId(1),
                next_effect_id: EffectInstanceId(1),
                next_trigger_id: TriggerInstanceId(1),
                next_decision_id: DecisionId(1),
                next_continuation_id: ContinuationId(1),
                next_rule_event_id: RuleEventId(1),
                next_opaque_object_id: BTreeMap::from([
                    (p1, OpaqueObjectId(1)),
                    (p2, OpaqueObjectId(1)),
                ]),
                next_opaque_ability_id: BTreeMap::from([
                    (p1, OpaqueAbilityId(1)),
                    (p2, OpaqueAbilityId(1)),
                ]),
            },
            execution: ExecutionState::default(),
            random: RandomState {
                algorithm_id: "test-counter".into(),
                derivation_version: "v1".into(),
                root_seed_hex: "00".repeat(32),
                streams: BTreeMap::from([("test".into(), RandomStreamState { counter: 0 })]),
            },
            knowledge: KnowledgeState {
                players: BTreeMap::from([
                    (p1, PlayerKnowledgeState::default()),
                    (p2, PlayerKnowledgeState::default()),
                ]),
            },
            perspective_identities: PerspectiveIdentityState {
                players: BTreeMap::from([
                    (p1, PerspectiveIdentityMap::default()),
                    (p2, PerspectiveIdentityMap::default()),
                ]),
            },
            format: FormatState::None,
        }
    }

    fn insert_object(
        state: &mut EngineState,
        object_id: u64,
        zone: ZoneKind,
        tapped: bool,
    ) -> ObjectSnapshot {
        let id = GameObjectId(object_id);
        let location = ZoneLocation {
            zone,
            player: Some(PlayerId(1)),
            position: ZonePosition::Unordered,
            visibility: if zone == ZoneKind::Battlefield || zone == ZoneKind::Graveyard {
                VisibilityPartition::Public
            } else {
                VisibilityPartition::OwnerOnly
            },
            partition: None,
        };
        let object = GameObject {
            id,
            physical_card: Some(PhysicalCardId(1)),
            card_definition: CardDefinitionId(1),
            owner: PlayerId(1),
            controller: PlayerId(1),
            tapped,
            face_down: false,
        };
        state.zones.objects.insert(id, object.clone());
        state.zones.locations.insert(id, location.clone());
        state
            .zones
            .ordered_zones
            .entry(location.key())
            .or_default()
            .push(id);
        state.allocators.next_object_id = GameObjectId(object_id + 1);
        ObjectSnapshot {
            object: id,
            physical_card: object.physical_card,
            card_definition: object.card_definition,
            owner: object.owner,
            controller: object.controller,
            tapped: object.tapped,
            face_down: object.face_down,
            location,
        }
    }

    fn remove_object(state: &mut EngineState, object: GameObjectId) {
        let location = state.zones.locations.remove(&object).unwrap();
        state.zones.objects.remove(&object).unwrap();
        let key = location.key();
        let objects = state.zones.ordered_zones.get_mut(&key).unwrap();
        objects.retain(|current| *current != object);
        if objects.is_empty() {
            state.zones.ordered_zones.remove(&key);
        }
    }

    fn result(
        before: &EngineState,
        after: EngineState,
        events: Vec<AuthoritativeRuleEvent>,
        next_decision: Option<PlayerDecisionRequest>,
    ) -> TransitionResult {
        let audit = events
            .iter()
            .map(|event| event.event.semantic_delta())
            .collect();
        TransitionResult {
            accepted: true,
            delta: StateDelta::between(before, &after, audit).unwrap(),
            next_state: after,
            events,
            next_decision,
            status: EpisodeStatus::Running,
        }
    }

    #[test]
    fn two_life_changes_in_one_atomic_transition_are_compositional() {
        let before = state();
        let mut after = before.clone();
        after.revision = StateRevision(1);
        after.core.players.get_mut(&PlayerId(1)).unwrap().life = 38;
        after.allocators.next_rule_event_id = RuleEventId(3);
        let events = vec![
            AuthoritativeRuleEvent {
                event_id: RuleEventId(1),
                state_revision: StateRevision(1),
                event: AuthoritativeRuleEventKind::LifeChanged {
                    player: PlayerId(1),
                    from: 40,
                    to: 39,
                },
            },
            AuthoritativeRuleEvent {
                event_id: RuleEventId(2),
                state_revision: StateRevision(1),
                event: AuthoritativeRuleEventKind::LifeChanged {
                    player: PlayerId(1),
                    from: 39,
                    to: 38,
                },
            },
        ];
        validate_transition_contract(&before, &result(&before, after, events, None)).unwrap();
    }

    #[test]
    fn decision_clear_then_create_composes_to_the_next_decision() {
        let mut before = state();
        before.execution.pending_decision = Some(PendingDecisionRecord {
            request: request(1, 0),
            candidate_bindings: BTreeMap::new(),
            continuation: None,
        });
        before.allocators.next_decision_id = DecisionId(2);

        let mut after = before.clone();
        after.revision = StateRevision(1);
        let next = request(2, 1);
        after.execution.pending_decision = Some(PendingDecisionRecord {
            request: next.clone(),
            candidate_bindings: BTreeMap::new(),
            continuation: None,
        });
        after.allocators.next_decision_id = DecisionId(3);
        after.allocators.next_rule_event_id = RuleEventId(3);
        let events = vec![
            AuthoritativeRuleEvent {
                event_id: RuleEventId(1),
                state_revision: StateRevision(1),
                event: AuthoritativeRuleEventKind::DecisionCleared {
                    decision: DecisionId(1),
                },
            },
            AuthoritativeRuleEvent {
                event_id: RuleEventId(2),
                state_revision: StateRevision(1),
                event: AuthoritativeRuleEventKind::DecisionCreated {
                    decision: DecisionId(2),
                },
            },
        ];
        validate_transition_contract(&before, &result(&before, after, events, Some(next))).unwrap();
    }

    #[test]
    fn two_rng_uses_of_one_stream_are_compositional() {
        let before = state();
        let mut after = before.clone();
        after.revision = StateRevision(1);
        after.random.streams.get_mut("test").unwrap().counter = 2;
        after.allocators.next_rule_event_id = RuleEventId(3);
        let events = vec![
            AuthoritativeRuleEvent {
                event_id: RuleEventId(1),
                state_revision: StateRevision(1),
                event: AuthoritativeRuleEventKind::RandomnessConsumed {
                    stream: "test".into(),
                    counter_before: 0,
                    counter_after: 1,
                    exclusive_upper_bound: 6,
                    value: 2,
                },
            },
            AuthoritativeRuleEvent {
                event_id: RuleEventId(2),
                state_revision: StateRevision(1),
                event: AuthoritativeRuleEventKind::RandomnessConsumed {
                    stream: "test".into(),
                    counter_before: 1,
                    counter_after: 2,
                    exclusive_upper_bound: 6,
                    value: 4,
                },
            },
        ];
        validate_transition_contract(&before, &result(&before, after, events, None)).unwrap();
    }

    #[test]
    fn repeated_tap_changes_in_one_atomic_transition_are_compositional() {
        let mut before = state();
        insert_object(&mut before, 1, ZoneKind::Battlefield, false);
        let mut after = before.clone();
        after.revision = StateRevision(1);
        after.allocators.next_rule_event_id = RuleEventId(3);
        let events = vec![
            AuthoritativeRuleEvent {
                event_id: RuleEventId(1),
                state_revision: StateRevision(1),
                event: AuthoritativeRuleEventKind::ObjectTapped {
                    object: GameObjectId(1),
                    from: false,
                    to: true,
                },
            },
            AuthoritativeRuleEvent {
                event_id: RuleEventId(2),
                state_revision: StateRevision(1),
                event: AuthoritativeRuleEventKind::ObjectTapped {
                    object: GameObjectId(1),
                    from: true,
                    to: false,
                },
            },
        ];
        validate_transition_contract(&before, &result(&before, after, events, None)).unwrap();
    }

    #[test]
    fn consecutive_zone_incarnations_in_one_transition_are_compositional() {
        let mut before = state();
        let first = insert_object(&mut before, 1, ZoneKind::Battlefield, false);

        let second_location = ZoneLocation {
            zone: ZoneKind::Exile,
            player: Some(PlayerId(1)),
            position: ZonePosition::Unordered,
            visibility: VisibilityPartition::Public,
            partition: Some("linked-test".into()),
        };
        let second = ObjectSnapshot {
            object: GameObjectId(2),
            location: second_location.clone(),
            ..first.clone()
        };
        let third_location = ZoneLocation {
            zone: ZoneKind::Graveyard,
            player: Some(PlayerId(1)),
            position: ZonePosition::Unordered,
            visibility: VisibilityPartition::Public,
            partition: None,
        };
        let third = ObjectSnapshot {
            object: GameObjectId(3),
            location: third_location.clone(),
            ..first.clone()
        };

        let mut after = before.clone();
        remove_object(&mut after, GameObjectId(1));
        insert_object(&mut after, 3, ZoneKind::Graveyard, false);
        after.revision = StateRevision(1);
        after.allocators.next_object_id = GameObjectId(4);
        after.allocators.next_rule_event_id = RuleEventId(3);

        let events = vec![
            AuthoritativeRuleEvent {
                event_id: RuleEventId(1),
                state_revision: StateRevision(1),
                event: AuthoritativeRuleEventKind::ZoneTransition {
                    transition: Box::new(ZoneTransition {
                        old_object: GameObjectId(1),
                        new_object: GameObjectId(2),
                        physical_card: Some(PhysicalCardId(1)),
                        from: first.location.clone(),
                        to: second_location,
                        last_known: first,
                        new_snapshot: second.clone(),
                    }),
                },
            },
            AuthoritativeRuleEvent {
                event_id: RuleEventId(2),
                state_revision: StateRevision(1),
                event: AuthoritativeRuleEventKind::ZoneTransition {
                    transition: Box::new(ZoneTransition {
                        old_object: GameObjectId(2),
                        new_object: GameObjectId(3),
                        physical_card: Some(PhysicalCardId(1)),
                        from: second.location.clone(),
                        to: third_location,
                        last_known: second,
                        new_snapshot: third,
                    }),
                },
            },
        ];
        validate_transition_contract(&before, &result(&before, after, events, None)).unwrap();
    }

    #[test]
    fn accepted_transition_cannot_reuse_the_consumed_decision_id() {
        let mut before = state();
        before.execution.pending_decision = Some(PendingDecisionRecord {
            request: request(1, 0),
            candidate_bindings: BTreeMap::new(),
            continuation: None,
        });
        before.allocators.next_decision_id = DecisionId(2);

        let mut after = before.clone();
        after.revision = StateRevision(1);
        after
            .execution
            .pending_decision
            .as_mut()
            .unwrap()
            .request
            .state_revision = StateRevision(1);
        let transition = result(&before, after, vec![], Some(request(1, 1)));
        assert!(matches!(
            validate_transition_contract(&before, &transition),
            Err(TransitionViolation::DecisionIdentityReused)
        ));
    }
}
