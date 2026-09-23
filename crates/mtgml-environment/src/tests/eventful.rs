//! Test-only eventful situation generator for FND-026D.
//!
//! This module creates one M2 lifecycle situation and routes it through the
//! normal environment transaction, replay, and projection code. It is not a
//! runtime rules path and does not define a second projector or engine.

use mtgml_decision::DecisionResponseV2;
use mtgml_model::{
    CardDefinitionId, EpisodeStatus, GameObjectId, OpaqueObjectId, PhysicalCardId, PlayerId,
    RuleEventId, VisibleSequence, ZoneKind,
};
use mtgml_random::RootSeed256;
use mtgml_replay::ReplayRecorderV6;
use mtgml_rules::fixture_support::{FixtureTransition, PlannedOccurrence};
use mtgml_rules::{AuthoritativeRuleEvent, AuthoritativeRuleEventKind, TransitionResult};
use mtgml_state::{
    validate_engine_state, EngineState, GameObject, IdentityMutationV1, KnowledgeAcquisitionCause,
    KnowledgeAcquisitionReason, KnowledgeHistoryChannel, KnowledgeMutationV1,
    PerspectiveLifecycleAuditV1, PerspectiveLifecycleMutationV1, StateDelta, VisibilityPartition,
    ZoneLocation, ZonePosition,
};

use super::{synthetic_identity, SyntheticM1EnvironmentBackend, SyntheticM1EnvironmentConfig};
use crate::checkpoint::EnvironmentCheckpointV6;
use crate::errors::ControllerError;

fn battlefield() -> ZoneLocation {
    ZoneLocation {
        zone: ZoneKind::Battlefield,
        player: None,
        position: ZonePosition::Unordered,
        visibility: VisibilityPartition::Public,
        partition: None,
    }
}

fn add_eventful_objects(state: &mut EngineState) {
    let exile = ZoneLocation {
        zone: ZoneKind::Exile,
        player: None,
        position: ZonePosition::Unordered,
        visibility: VisibilityPartition::Public,
        partition: None,
    };
    for index in 3..=4u64 {
        let object = GameObjectId(index);
        state.zones.objects.insert(
            object,
            GameObject {
                id: object,
                physical_card: Some(PhysicalCardId(index)),
                card_definition: CardDefinitionId(index),
                owner: PlayerId(1),
                controller: PlayerId(1),
                tapped: false,
                face_down: false,
            },
        );
        state.zones.locations.insert(object, exile.clone());
    }
    state.allocators.next_object_id = GameObjectId(5);
}

pub(super) fn backend(
    players: [PlayerId; 2],
    root_seed: RootSeed256,
    config: SyntheticM1EnvironmentConfig,
) -> Result<SyntheticM1EnvironmentBackend, ControllerError> {
    let mut backend = SyntheticM1EnvironmentBackend::new(players, root_seed, config)?;
    add_eventful_objects(&mut backend.state);
    validate_engine_state(&backend.state)
        .map_err(mtgml_state::SyntheticStateConstructionError::Validation)?;
    let checkpoint = EnvironmentCheckpointV6::new(
        backend.state.clone(),
        backend.status.clone(),
        backend.limit_counters.clone(),
        backend.codec.clone(),
        synthetic_identity(),
    )?;
    backend.replay =
        ReplayRecorderV6::new(super::replay::build_manifest(&backend.config, &checkpoint)?)?;
    backend.eventful_fixture = true;
    Ok(backend)
}

fn rejected(before: &EngineState) -> Result<TransitionResult, mtgml_rules::KernelExecutionError> {
    let delta = StateDelta::between(before, before, Vec::new())
        .map_err(mtgml_rules::KernelExecutionError::Delta)?;
    Ok(TransitionResult {
        accepted: false,
        next_decision: before
            .execution
            .pending_decision
            .as_ref()
            .map(|record| record.request.clone()),
        status: EpisodeStatus::Running,
        next_state: before.clone(),
        delta,
        events: Vec::new(),
    })
}

pub(super) fn apply(
    before: &EngineState,
    actor: PlayerId,
    response: &DecisionResponseV2,
) -> Result<TransitionResult, mtgml_rules::KernelExecutionError> {
    let Some(pending) = before.execution.pending_decision.as_ref() else {
        return rejected(before);
    };
    let Ok(visible) = pending.request.project_player_request() else {
        return rejected(before);
    };
    if pending.request.actor != actor || response.validate_for(&visible).is_err() {
        return rejected(before);
    }

    let decision = pending.request.decision_id;
    let mut fixture_before = before.clone();
    fixture_before.execution.pending_decision = None;
    fixture_before.allocators.next_rule_event_id = RuleEventId(
        before
            .allocators
            .next_rule_event_id
            .0
            .checked_add(1)
            .ok_or(mtgml_rules::KernelExecutionError::RuleEventIdOverflow)?,
    );

    let mut fixture = FixtureTransition::start(&fixture_before)?;
    let revealed = fixture.move_object_incarnation(GameObjectId(3), battlefield())?;
    fixture.apply_occurrence(PlannedOccurrence {
        lifecycle: PerspectiveLifecycleAuditV1 {
            perspective: PlayerId(1),
            sequence: VisibleSequence(1),
            mutation: PerspectiveLifecycleMutationV1 {
                identity: IdentityMutationV1::Allocate {
                    opaque: OpaqueObjectId(2),
                    object: revealed,
                },
                knowledge: Some(KnowledgeMutationV1::Acquire {
                    opaque: OpaqueObjectId(2),
                    definition: Some(CardDefinitionId(3)),
                    location: Some(battlefield()),
                    acquisition: KnowledgeAcquisitionReason::Observed {
                        channel: KnowledgeHistoryChannel::Public,
                        sequence: VisibleSequence(1),
                        cause: KnowledgeAcquisitionCause::ExplicitReveal,
                    },
                }),
            },
        },
        observation: mtgml_rules::PerspectiveObservationPolicyV1::Appeared {
            from_zone: ZoneKind::Exile,
            to_zone: ZoneKind::Battlefield,
            new_object: revealed,
        },
    })?;
    fixture.apply_occurrence(PlannedOccurrence {
        lifecycle: PerspectiveLifecycleAuditV1 {
            perspective: PlayerId(2),
            sequence: VisibleSequence(1),
            mutation: PerspectiveLifecycleMutationV1::default(),
        },
        observation: mtgml_rules::PerspectiveObservationPolicyV1::AnnouncedOutcome {
            code: "p2-public".into(),
        },
    })?;
    let generated = fixture.finish()?;

    let clear = AuthoritativeRuleEvent {
        event_id: before.allocators.next_rule_event_id,
        state_revision: generated.next_state.revision,
        event: AuthoritativeRuleEventKind::DecisionCleared { decision },
    };
    let mut events = Vec::with_capacity(generated.events.len() + 1);
    events.push(clear);
    events.extend(generated.events);
    let audit = events
        .iter()
        .map(|event| event.event.semantic_delta())
        .collect();
    let delta = StateDelta::between(before, &generated.next_state, audit)
        .map_err(mtgml_rules::KernelExecutionError::Delta)?;
    let result = TransitionResult {
        accepted: true,
        next_decision: None,
        status: EpisodeStatus::Running,
        next_state: generated.next_state,
        delta,
        events,
    };
    validate_engine_state(&result.next_state)
        .map_err(mtgml_rules::KernelExecutionError::AfterState)?;
    mtgml_rules::validate_transition_contract(before, &result)
        .map_err(mtgml_rules::KernelExecutionError::TransitionContract)?;
    Ok(result)
}
