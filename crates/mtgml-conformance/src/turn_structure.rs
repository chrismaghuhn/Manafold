use mtgml_environment::{
    magic_turn_structure_0_1_0_rules_manifest, ReferenceEnvironmentConfig,
    ReferenceEnvironmentReplayConfig, REFERENCE_SCENARIO_ID,
};
use mtgml_model::{CheckpointCodecIdentity, RulesAuthorityV1};
use mtgml_model::{
    EnvironmentLimitCounters, EpisodeStatus, ExecutionIdentityV1, ExecutionProgramV1,
};
use mtgml_observation::{
    INFORMATION_STATE_SCHEMA_V2, OBSERVATION_SCHEMA, OBSERVED_EVENT_SCHEMA_V2,
    PLAYER_STEP_SCHEMA_V2,
};
use mtgml_replay::{KernelIdentityV1, ReplaySchemaVersionsV5};

use crate::facade::{
    ConformanceCase, ConformanceCaseStep, ConformanceForcedProgressStepRef,
    ForcedProgressExpectation, LimitCounterDeltas,
};

fn reference_config(state: mtgml_state::EngineState) -> ReferenceEnvironmentConfig {
    let rules_snapshot = match magic_turn_structure_0_1_0_rules_manifest().rules_authority {
        RulesAuthorityV1::ComprehensiveRules { snapshot_id } => snapshot_id,
        RulesAuthorityV1::SyntheticLegacy => panic!("S1 needs ComprehensiveRules"),
    };
    ReferenceEnvironmentConfig {
        state,
        status: EpisodeStatus::Running,
        limit_counters: EnvironmentLimitCounters::default(),
        codec: CheckpointCodecIdentity {
            codec_id: "in-memory-reference".into(),
            semantic_version: "5".into(),
        },
        execution_identity: ExecutionIdentityV1 {
            program_kind: ExecutionProgramV1::MagicRules,
            semantic_contract_id:
                mtgml_environment::magic_turn_structure_0_1_0_semantic_contract_id(),
        },
        replay: ReferenceEnvironmentReplayConfig {
            scenario_id: REFERENCE_SCENARIO_ID.into(),
            engine_build: "reference-test".into(),
            kernel: KernelIdentityV1 {
                implementation_id: "magic-reference".into(),
                semantic_version: "0.2.2".into(),
                build_profile: "test".into(),
            },
            rules_snapshot,
            format_policy_snapshot: "format:none".into(),
            oracle_snapshot: "oracle:none".into(),
            schemas: ReplaySchemaVersionsV5 {
                observation: OBSERVATION_SCHEMA.into(),
                observation_payload_codec: "synthetic-m3-observation.v1".into(),
                information_state: INFORMATION_STATE_SCHEMA_V2.into(),
                decision: "player-decision-request.v2".into(),
                decision_response: "decision-response.v2".into(),
                observed_event: OBSERVED_EVENT_SCHEMA_V2.into(),
                player_step: PLAYER_STEP_SCHEMA_V2.into(),
                replay_step: "replay-step.v5".into(),
            },
        },
    }
}

#[test]
fn reference_backend_is_available_to_the_real_conformance_path() {
    let mut state =
        mtgml_state::construct_synthetic_engine_state(mtgml_state::SyntheticResetInputs {
            players: [mtgml_model::PlayerId(1), mtgml_model::PlayerId(2)],
            root_seed: mtgml_random::RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap(),
            setup: mtgml_state::SyntheticV4Setup {
                position: mtgml_state::TurnPosition::Beginning {
                    step: mtgml_state::BeginningStep::Untap,
                },
                priority: mtgml_state::PriorityState::None,
                combat: None,
                foundation_sources: std::collections::BTreeMap::new(),
            },
        })
        .unwrap();
    state.execution.pending_decision = None;

    let backend = mtgml_environment::ReferenceEnvironmentBackend::new(reference_config(state))
        .expect("the reference backend must be available to conformance");
    let controller = mtgml_environment::TrustedEnvironmentController::new(backend);
    controller
        .execute_forced_progress()
        .expect("conformance must reach the trusted forced-progress path");
}

fn reference_controller_and_endpoints(
    state: mtgml_state::EngineState,
) -> (
    mtgml_environment::TrustedEnvironmentController,
    [mtgml_environment::PlayerEndpointHandle; 2],
) {
    let controller = mtgml_environment::TrustedEnvironmentController::new(
        mtgml_environment::ReferenceEnvironmentBackend::new(reference_config(state)).unwrap(),
    );
    let endpoints = [
        controller.bind_player(mtgml_model::PlayerId(1)).unwrap(),
        controller.bind_player(mtgml_model::PlayerId(2)).unwrap(),
    ];
    (controller, endpoints)
}

fn reference_state(position: mtgml_state::TurnPosition) -> mtgml_state::EngineState {
    let mut state =
        mtgml_state::construct_synthetic_engine_state(mtgml_state::SyntheticResetInputs {
            players: [mtgml_model::PlayerId(1), mtgml_model::PlayerId(2)],
            root_seed: mtgml_random::RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap(),
            setup: mtgml_state::SyntheticV4Setup {
                position,
                priority: mtgml_state::PriorityState::None,
                combat: None,
                foundation_sources: std::collections::BTreeMap::new(),
            },
        })
        .unwrap();
    state.execution.pending_decision = None;
    state
}

fn authored_forced_expectation(
    before: &mtgml_state::EngineState,
    after: mtgml_state::EngineState,
    events: Vec<mtgml_rules::AuthoritativeRuleEvent>,
) -> ForcedProgressExpectation {
    let audit = events
        .iter()
        .map(|event| event.event.semantic_delta())
        .collect();
    let delta = mtgml_state::StateDelta::between(before, &after, audit).unwrap();
    let event_count = events.len() as u64;
    ForcedProgressExpectation {
        expected_state_digest: after.digest().unwrap(),
        expected_authoritative_events: events,
        expected_semantic_delta: delta,
        expected_next_decision: None,
        expected_status: mtgml_model::EpisodeStatus::Running,
        expected_information_states: std::collections::BTreeMap::new(),
        expected_visible_decisions: std::collections::BTreeMap::from([
            (mtgml_model::PlayerId(1), None),
            (mtgml_model::PlayerId(2), None),
        ]),
        expected_limit_counter_deltas: LimitCounterDeltas {
            decisions_submitted: 0,
            accepted_transitions: 0,
            rule_events_emitted: event_count,
        },
    }
}

fn authored_untap_events(
    affected: &[mtgml_model::GameObjectId],
) -> Vec<mtgml_rules::AuthoritativeRuleEvent> {
    use mtgml_model::{RuleEventId, StateRevision, VisibleSequence};
    use mtgml_rules::{
        AuthoritativeRuleEvent, AuthoritativeRuleEventKind, PerspectiveObservationPolicyV1,
    };
    use mtgml_state::{PerspectiveLifecycleAuditV1, PerspectiveLifecycleMutationV1};

    let mut events = vec![AuthoritativeRuleEvent {
        event_id: RuleEventId(1),
        state_revision: StateRevision(1),
        event: AuthoritativeRuleEventKind::UntapCompleted {
            affected_objects: affected.to_vec(),
        },
    }];
    let mut next_event = 2;
    for player in [mtgml_model::PlayerId(1), mtgml_model::PlayerId(2)] {
        for object in affected {
            events.push(AuthoritativeRuleEvent {
                event_id: RuleEventId(next_event),
                state_revision: StateRevision(1),
                event: AuthoritativeRuleEventKind::PerspectiveOccurrence {
                    lifecycle: PerspectiveLifecycleAuditV1 {
                        perspective: player,
                        sequence: VisibleSequence(1),
                        mutation: PerspectiveLifecycleMutationV1::default(),
                    },
                    observation: PerspectiveObservationPolicyV1::ObjectTapped {
                        object: *object,
                        tapped: false,
                    },
                },
            });
            next_event += 1;
        }
    }
    events.push(AuthoritativeRuleEvent {
        event_id: RuleEventId(next_event),
        state_revision: StateRevision(1),
        event: AuthoritativeRuleEventKind::TurnPositionChanged {
            from: mtgml_state::TurnPosition::Beginning {
                step: mtgml_state::BeginningStep::Untap,
            },
            to: mtgml_state::TurnPosition::Beginning {
                step: mtgml_state::BeginningStep::Upkeep,
            },
        },
    });
    events
}

#[test]
fn task10_turn_structure_conformance_ordinary_untap_and_empty_set() {
    let mut tapped = reference_state(mtgml_state::TurnPosition::Beginning {
        step: mtgml_state::BeginningStep::Untap,
    });
    tapped
        .zones
        .objects
        .get_mut(&mtgml_model::GameObjectId(1))
        .unwrap()
        .tapped = true;
    let before = tapped.clone();
    tapped.revision = mtgml_model::StateRevision(1);
    tapped.core.position = mtgml_state::TurnPosition::Beginning {
        step: mtgml_state::BeginningStep::Upkeep,
    };
    tapped
        .zones
        .objects
        .get_mut(&mtgml_model::GameObjectId(1))
        .unwrap()
        .tapped = false;
    tapped.knowledge.players.values_mut().for_each(|knowledge| {
        knowledge.next_visible_sequence = mtgml_model::VisibleSequence(2);
    });
    tapped.allocators.next_rule_event_id = mtgml_model::RuleEventId(5);
    let tapped_expectation = authored_forced_expectation(
        &before,
        tapped,
        authored_untap_events(&[mtgml_model::GameObjectId(1)]),
    );
    let (controller, endpoints) = reference_controller_and_endpoints(before);
    crate::facade::run_case(
        &ConformanceCase {
            name: "s1.untap.one-active-tapped",
            description:
                "the real MagicRules reference backend untaps the active battlefield object",
            steps: vec![ConformanceCaseStep::ForcedProgress(Box::new(
                ConformanceForcedProgressStepRef {
                    label: "ordinary-untap",
                    expectation: tapped_expectation,
                },
            ))],
        },
        &controller,
        &endpoints,
    )
    .unwrap();

    let empty_before = reference_state(mtgml_state::TurnPosition::Beginning {
        step: mtgml_state::BeginningStep::Untap,
    });
    let mut empty_after = empty_before.clone();
    empty_after.revision = mtgml_model::StateRevision(1);
    empty_after.core.position = mtgml_state::TurnPosition::Beginning {
        step: mtgml_state::BeginningStep::Upkeep,
    };
    empty_after.allocators.next_rule_event_id = mtgml_model::RuleEventId(3);
    let empty_events = authored_untap_events(&[]);
    let empty_expectation = authored_forced_expectation(&empty_before, empty_after, empty_events);
    let (empty_controller, empty_endpoints) = reference_controller_and_endpoints(empty_before);
    crate::facade::run_case(
        &ConformanceCase {
            name: "s1.untap.empty-affected-set",
            description: "an empty ordinary untap still emits the exact boundary events",
            steps: vec![ConformanceCaseStep::ForcedProgress(Box::new(
                ConformanceForcedProgressStepRef {
                    label: "empty-untap",
                    expectation: empty_expectation,
                },
            ))],
        },
        &empty_controller,
        &empty_endpoints,
    )
    .unwrap();
}

#[test]
fn task10_turn_structure_conformance_cleanup_and_deterministic_rerun() {
    let before = reference_state(mtgml_state::TurnPosition::Ending {
        step: mtgml_state::EndingStep::Cleanup,
    });
    let mut after = before.clone();
    after.revision = mtgml_model::StateRevision(1);
    after.core.turn_number += 1;
    after.core.active_player = mtgml_model::PlayerId(2);
    after.core.position = mtgml_state::TurnPosition::Beginning {
        step: mtgml_state::BeginningStep::Untap,
    };
    after.allocators.next_rule_event_id = mtgml_model::RuleEventId(4);
    let events = vec![
        mtgml_rules::AuthoritativeRuleEvent {
            event_id: mtgml_model::RuleEventId(1),
            state_revision: mtgml_model::StateRevision(1),
            event: mtgml_rules::AuthoritativeRuleEventKind::TurnNumberChanged {
                from: before.core.turn_number,
                to: before.core.turn_number + 1,
            },
        },
        mtgml_rules::AuthoritativeRuleEvent {
            event_id: mtgml_model::RuleEventId(2),
            state_revision: mtgml_model::StateRevision(1),
            event: mtgml_rules::AuthoritativeRuleEventKind::ActivePlayerChanged {
                from: mtgml_model::PlayerId(1),
                to: mtgml_model::PlayerId(2),
            },
        },
        mtgml_rules::AuthoritativeRuleEvent {
            event_id: mtgml_model::RuleEventId(3),
            state_revision: mtgml_model::StateRevision(1),
            event: mtgml_rules::AuthoritativeRuleEventKind::TurnPositionChanged {
                from: mtgml_state::TurnPosition::Ending {
                    step: mtgml_state::EndingStep::Cleanup,
                },
                to: mtgml_state::TurnPosition::Beginning {
                    step: mtgml_state::BeginningStep::Untap,
                },
            },
        },
    ];
    let expectation = authored_forced_expectation(&before, after, events);

    let run = || {
        let (controller, endpoints) = reference_controller_and_endpoints(before.clone());
        crate::facade::run_case(
            &ConformanceCase {
                name: "s1.cleanup.next-turn",
                description: "quiescent cleanup switches the active player and increments the turn",
                steps: vec![ConformanceCaseStep::ForcedProgress(Box::new(
                    ConformanceForcedProgressStepRef {
                        label: "cleanup-switch",
                        expectation: expectation.clone(),
                    },
                ))],
            },
            &controller,
            &endpoints,
        )
        .unwrap();
        (
            controller.checkpoint().unwrap(),
            controller.export_replay().unwrap(),
        )
    };
    let first = run();
    let second = run();
    assert_eq!(first, second);
}
