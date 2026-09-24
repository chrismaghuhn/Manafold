// S3.A1 RED witnesses enter the real private Magic rules kernel through the
// fixed S3.A conformance candidate. The frozen production S1 contract remains
// covered separately and retains its own Basic Priority boundary.

fn sba_upkeep_state() -> EngineState {
    let mut state = synthetic_state();
    state.execution.pending_decision = None;
    state.execution.continuations.clear();
    let battlefield: Vec<_> = state
        .zones
        .locations
        .iter()
        .filter_map(|(object, location)| {
            (location.zone == mtgml_model::ZoneKind::Battlefield).then_some(*object)
        })
        .collect();
    for object in battlefield {
        state.foundation_sources.insert(
            object,
            mtgml_state::FoundationCreatureSource {
                source_kind: mtgml_state::FoundationSourceKind::Creature,
                base_characteristics: mtgml_state::BaseCharacteristics::Simple {
                    power: 2,
                    toughness: 2,
                },
                marked_damage: 0,
                control_history: mtgml_state::ControlHistory::BeforeTurnStart {
                    turn_number: 1,
                },
            },
        );
    }
    state.core.position = mtgml_state::TurnPosition::Beginning {
        step: mtgml_state::BeginningStep::Upkeep,
    };
    mtgml_state::validate_engine_state(&state)
        .expect("authored S3.A rules fixture must be structurally valid");
    state
}

fn two_same_owner_deaths_at_upkeep() -> EngineState {
    use mtgml_model::{CardDefinitionId, OpaqueObjectId, PhysicalCardId, ZoneKind};
    use mtgml_state::{
        BaseCharacteristics, ControlHistory, FoundationCreatureSource, FoundationSourceKind,
        KnowledgeAcquisitionReason, KnowledgeRecordV2, KnownLocationFactV2, VisibilityPartition,
        ZoneLocation, ZonePosition,
    };

    let mut state = sba_upkeep_state();
    let second = GameObjectId(2);
    let location = ZoneLocation {
        zone: ZoneKind::Battlefield,
        player: None,
        position: ZonePosition::Unordered,
        visibility: VisibilityPartition::Public,
        partition: None,
    };
    state.zones.ordered_zones.clear();
    state.zones.locations.insert(second, location.clone());
    let object = state.zones.objects.get_mut(&second).unwrap();
    object.owner = PlayerId(1);
    object.controller = PlayerId(1);
    object.face_down = false;

    for id in [GameObjectId(1), second] {
        state.foundation_sources.insert(
            id,
            FoundationCreatureSource {
                source_kind: FoundationSourceKind::Creature,
                base_characteristics: BaseCharacteristics::Simple {
                    power: 2,
                    toughness: 0,
                },
                marked_damage: 0,
                control_history: ControlHistory::BeforeTurnStart { turn_number: 1 },
            },
        );
    }

    let p1_opaque = OpaqueObjectId(2);
    {
        let identity = state
            .perspective_identities
            .players
            .get_mut(&PlayerId(1))
            .unwrap();
        identity.opaque_to_object.insert(p1_opaque, second);
        identity.object_to_opaque.insert(second, p1_opaque);
        identity.next_opaque_object_id = OpaqueObjectId(3);
    }
    state
        .knowledge
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .active
        .insert(
            p1_opaque,
            KnowledgeRecordV2 {
                opaque_object: p1_opaque,
                physical_card: Some(PhysicalCardId(2)),
                card_definition: Some(CardDefinitionId(2)),
                known_location: Some(KnownLocationFactV2 {
                    location: location.clone(),
                    provenance: KnowledgeAcquisitionReason::InitialConfiguration,
                }),
                acquisition: KnowledgeAcquisitionReason::InitialConfiguration,
                historical_locations: Vec::new(),
            },
        );
    state
        .knowledge
        .players
        .get_mut(&PlayerId(2))
        .unwrap()
        .active
        .get_mut(&OpaqueObjectId(2))
        .unwrap()
        .known_location = Some(KnownLocationFactV2 {
        location,
        provenance: KnowledgeAcquisitionReason::InitialConfiguration,
    });
    mtgml_state::validate_engine_state(&state)
        .expect("authored same-owner death fixture must be structurally valid");
    state
}

fn two_same_owner_order_stage0() -> EngineState {
    use mtgml_decision::{
        AuthoritativeCandidateV2, AuthoritativeDecisionRequestV2, CandidateIntent,
        DecisionDomainV2, DecisionVisibility, EngineCandidateBinding,
    };
    use mtgml_model::{CandidateIdV1, ContinuationId, DecisionId, PlayerDecisionIdV1};
    use mtgml_state::{
        ContinuationPayloadV2, ContinuationRecordV2, PendingDecisionRecordV2, SbaObjectCauseV1,
        SbaSelectedActionV1,
    };

    let mut state = two_same_owner_deaths_at_upkeep();
    state.revision = StateRevision(1);
    let continuation = ContinuationId(1);
    let actions = vec![
        SbaSelectedActionV1::ObjectToOwnerGraveyard {
            object: GameObjectId(1),
            causes: vec![SbaObjectCauseV1::ZeroToughness],
        },
        SbaSelectedActionV1::ObjectToOwnerGraveyard {
            object: GameObjectId(2),
            causes: vec![SbaObjectCauseV1::ZeroToughness],
        },
    ];
    let payload = ContinuationPayloadV2::MagicSbaGraveyardOrderV1 {
        round_start_revision: StateRevision(0),
        selected_sba_actions: actions,
        apnap_owners: vec![PlayerId(1)],
        next_owner_index: 0,
        completed_owner_orders: Vec::new(),
    };
    state.execution.continuations.insert(
        continuation,
        ContinuationRecordV2 {
            id: continuation,
            actor: PlayerId(1),
            created_at_revision: StateRevision(1),
            stage_index: 0,
            payload,
        },
    );
    let candidates = [GameObjectId(1), GameObjectId(2)]
        .into_iter()
        .enumerate()
        .map(|(index, object)| AuthoritativeCandidateV2 {
            candidate_id: CandidateIdV1(index as u32),
            visible_intent: CandidateIntent::SelectObject {
                object: state.perspective_identities.players[&PlayerId(1)].object_to_opaque
                    [&object],
            },
            trusted_binding: EngineCandidateBinding::SelectObject { object },
        })
        .collect();
    state.execution.pending_decision = Some(PendingDecisionRecordV2 {
        request: AuthoritativeDecisionRequestV2 {
            decision_id: DecisionId(2),
            player_decision_id: PlayerDecisionIdV1(2),
            state_revision: StateRevision(1),
            actor: PlayerId(1),
            visibility: DecisionVisibility::ActingPlayerOnly,
            decision: DecisionDomainV2::Order {
                minimum: 2,
                maximum: 2,
            },
            candidates,
            continuation_id: Some(continuation),
        },
    });
    state.allocators.next_decision_id = DecisionId(3);
    state.allocators.next_continuation_id = ContinuationId(2);
    state
        .perspective_identities
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .next_player_decision_id = PlayerDecisionIdV1(3);
    mtgml_state::validate_engine_state(&state)
        .expect("authored Rules semantic plan fixture must be structural");
    state
}

#[test]
fn task6_sba_rules_validator_accepts_an_exact_stage_zero_plan() {
    let state = two_same_owner_order_stage0();
    assert_eq!(
        crate::state_based_actions::validate_sba_order_continuation(&state),
        Ok(())
    );
}

#[test]
fn task6_sba_rules_validator_rejects_stale_applicable_causes() {
    use mtgml_state::{
        ContinuationPayloadV2, SbaObjectCauseV1, SbaSelectedActionV1,
    };

    let mut state = two_same_owner_order_stage0();
    let continuation = state
        .execution
        .continuations
        .get_mut(&mtgml_model::ContinuationId(1))
        .unwrap();
    let ContinuationPayloadV2::MagicSbaGraveyardOrderV1 {
        selected_sba_actions,
        ..
    } = &mut continuation.payload
    else {
        unreachable!()
    };
    selected_sba_actions[0] = SbaSelectedActionV1::ObjectToOwnerGraveyard {
        object: GameObjectId(1),
        causes: vec![SbaObjectCauseV1::LethalDamage],
    };
    mtgml_state::validate_engine_state(&state)
        .expect("stale cause remains structurally well-formed for Rules validation");
    assert_eq!(
        crate::state_based_actions::validate_sba_order_continuation(&state),
        Err(crate::state_based_actions::SbaContinuationValidationError::SelectedActionSetMismatch)
    );
}

#[test]
fn task6_sba_rules_validator_rejects_a_newly_applicable_player_loss() {
    let mut state = two_same_owner_order_stage0();
    state.core.players.get_mut(&PlayerId(1)).unwrap().life = 0;
    mtgml_state::validate_engine_state(&state)
        .expect("missing semantic loss action remains structurally valid");
    assert_eq!(
        crate::state_based_actions::validate_sba_order_continuation(&state),
        Err(crate::state_based_actions::SbaContinuationValidationError::SelectedActionSetMismatch)
    );
}

#[test]
fn task6_sba_profile_rejects_a_non_none_format_state() {
    let mut state = two_same_owner_order_stage0();
    state.format = mtgml_state::FormatState::Commander {
        state: mtgml_state::CommanderState {
            designations: std::collections::BTreeMap::from([(
                PlayerId(1),
                vec![mtgml_model::PhysicalCardId(1)],
            )]),
            cast_counts: std::collections::BTreeMap::new(),
            damage: std::collections::BTreeMap::new(),
        },
    };
    mtgml_state::validate_engine_state(&state).unwrap();
    assert_eq!(
        crate::state_based_actions::validate_sba_order_continuation(&state),
        Err(crate::state_based_actions::SbaContinuationValidationError::UnsupportedSbaProfile)
    );
}

#[test]
fn task6_sba_profile_rejects_already_applied_player_loss() {
    let mut state = two_same_owner_order_stage0();
    state.core.players.get_mut(&PlayerId(1)).unwrap().has_lost = true;
    mtgml_state::validate_engine_state(&state).unwrap();
    assert_eq!(
        crate::state_based_actions::validate_sba_order_continuation(&state),
        Err(crate::state_based_actions::SbaContinuationValidationError::UnsupportedSbaProfile)
    );
}

#[test]
fn task6_sba_profile_rejects_untap_but_accepts_selected_cleanup_check() {
    let mut untap = two_same_owner_order_stage0();
    untap.core.position = mtgml_state::TurnPosition::Beginning {
        step: mtgml_state::BeginningStep::Untap,
    };
    mtgml_state::validate_engine_state(&untap).unwrap();
    assert_eq!(
        crate::state_based_actions::validate_sba_order_continuation(&untap),
        Err(crate::state_based_actions::SbaContinuationValidationError::UnsupportedSbaProfile)
    );

    let mut cleanup = two_same_owner_order_stage0();
    cleanup.core.position = mtgml_state::TurnPosition::Ending {
        step: mtgml_state::EndingStep::Cleanup,
    };
    mtgml_state::validate_engine_state(&cleanup).unwrap();
    assert_eq!(
        crate::state_based_actions::validate_sba_order_continuation(&cleanup),
        Ok(())
    );
}

#[test]
fn task6_sba_profile_allows_bounded_combat_damage_boundary() {
    let mut state = two_same_owner_order_stage0();
    state.core.position = mtgml_state::TurnPosition::Combat {
        step: mtgml_state::CombatStep::CombatDamage,
    };
    state.combat = Some(mtgml_state::CombatState {
        defending_player: PlayerId(2),
        attackers: vec![GameObjectId(1)],
        blockers: std::collections::BTreeMap::from([(GameObjectId(1), None)]),
    });
    mtgml_state::validate_engine_state(&state).unwrap();
    assert_eq!(
        crate::state_based_actions::validate_sba_order_continuation(&state),
        Ok(())
    );
}

#[test]
fn task6_sba_profile_checks_effect_trigger_and_delayed_effect_axes_directly() {
    use mtgml_model::{EffectInstanceId, TriggerInstanceId};
    use mtgml_state::{EffectRecord, TriggerRecord};

    let mut effects = two_same_owner_order_stage0();
    effects.execution.effects.insert(
        EffectInstanceId(1),
        EffectRecord {
            id: EffectInstanceId(1),
            label: "unsupported".into(),
        },
    );
    assert_eq!(
        crate::state_based_actions::validate_s3_a_support_profile(&effects),
        Err(crate::state_based_actions::SbaContinuationValidationError::UnsupportedSbaProfile)
    );

    let mut triggers = two_same_owner_order_stage0();
    triggers.execution.waiting_triggers.insert(
        TriggerInstanceId(1),
        TriggerRecord {
            id: TriggerInstanceId(1),
            controller: PlayerId(1),
        },
    );
    assert_eq!(
        crate::state_based_actions::validate_s3_a_support_profile(&triggers),
        Err(crate::state_based_actions::SbaContinuationValidationError::UnsupportedSbaProfile)
    );

    let mut delayed = two_same_owner_order_stage0();
    delayed.execution.delayed_effects.insert(
        EffectInstanceId(1),
        EffectRecord {
            id: EffectInstanceId(1),
            label: "unsupported".into(),
        },
    );
    assert_eq!(
        crate::state_based_actions::validate_s3_a_support_profile(&delayed),
        Err(crate::state_based_actions::SbaContinuationValidationError::UnsupportedSbaProfile)
    );
}

#[test]
fn task6_sba_profile_rejects_both_stack_representations() {
    use mtgml_model::StackObjectId;
    use mtgml_state::StackRecord;

    let mut stack = two_same_owner_order_stage0();
    stack.zones.stack_records.insert(
        StackObjectId(1),
        StackRecord {
            id: StackObjectId(1),
            controller: PlayerId(1),
            source_object: None,
            source_ability: None,
        },
    );
    stack.zones.stack_order.push(StackObjectId(1));
    stack.allocators.next_stack_object_id = StackObjectId(2);
    mtgml_state::validate_engine_state(&stack).unwrap();
    assert_eq!(
        crate::state_based_actions::validate_sba_order_continuation(&stack),
        Err(crate::state_based_actions::SbaContinuationValidationError::UnsupportedSbaProfile)
    );

    let mut stack_zone = two_same_owner_order_stage0();
    stack_zone
        .zones
        .locations
        .get_mut(&GameObjectId(1))
        .unwrap()
        .zone = mtgml_model::ZoneKind::Stack;
    let stack_location = stack_zone.zones.locations[&GameObjectId(1)].clone();
    for player in [PlayerId(1), PlayerId(2)] {
        let identity = &stack_zone.perspective_identities.players[&player];
        let opaque = identity.object_to_opaque[&GameObjectId(1)];
        stack_zone
            .knowledge
            .players
            .get_mut(&player)
            .unwrap()
            .active
            .get_mut(&opaque)
            .unwrap()
            .known_location = Some(mtgml_state::KnownLocationFactV2 {
            location: stack_location.clone(),
            provenance: mtgml_state::KnowledgeAcquisitionReason::InitialConfiguration,
        });
    }
    mtgml_state::validate_engine_state(&stack_zone).unwrap();
    assert_eq!(
        crate::state_based_actions::validate_sba_order_continuation(&stack_zone),
        Err(crate::state_based_actions::SbaContinuationValidationError::UnsupportedSbaProfile)
    );
}

#[test]
fn task6_sba_profile_rejects_non_physical_objects_and_live_ability_mappings() {
    let mut token = two_same_owner_order_stage0();
    token
        .zones
        .objects
        .get_mut(&GameObjectId(1))
        .unwrap()
        .physical_card = None;
    assert_eq!(
        crate::state_based_actions::validate_s3_a_support_profile(&token),
        Err(crate::state_based_actions::SbaContinuationValidationError::UnsupportedSbaProfile)
    );

    let mut ability = two_same_owner_order_stage0();
    let identity = ability
        .perspective_identities
        .players
        .get_mut(&PlayerId(1))
        .unwrap();
    identity
        .opaque_to_ability
        .insert(mtgml_model::OpaqueAbilityId(1), mtgml_model::AbilityInstanceId(1));
    identity
        .ability_to_opaque
        .insert(mtgml_model::AbilityInstanceId(1), mtgml_model::OpaqueAbilityId(1));
    assert_eq!(
        crate::state_based_actions::validate_s3_a_support_profile(&ability),
        Err(crate::state_based_actions::SbaContinuationValidationError::UnsupportedSbaProfile)
    );
}


#[test]
fn magic_rules_state_based_actions_round_is_missing_before_priority() {
    let mut state = sba_upkeep_state();
    state.core.players.get_mut(&PlayerId(1)).unwrap().life = 0;
    let plan = crate::state_based_actions::derive_bounded_sba_round_plan(&state)
        .expect("bounded zero-life state must derive its exact SBA action");
    assert_eq!(
        plan.selected_sba_actions,
        vec![mtgml_state::SbaSelectedActionV1::PlayerLoses { player: PlayerId(1) }]
    );
    assert!(plan.apnap_owners.is_empty());
    let before = state.clone();
    let mut kernel = crate::magic::MagicRulesKernel::s3_a_conformance_candidate();
    let result = kernel.advance_forced_progress(&state);
    assert_eq!(state, before, "the RED probe must not mutate the input fixture");
    let transition = result.unwrap_or_else(|error| {
        panic!("S3.A RED: Magic forced progress has no SBA/loss producer: {error:?}")
    });
    assert!(transition.accepted);
    assert!(transition.next_state.core.players[&PlayerId(1)].has_lost);
    assert_eq!(
        transition.status,
        mtgml_model::EpisodeStatus::Terminal {
            reason: mtgml_model::TerminalReason::RulesLoss,
            players: vec![
                mtgml_model::PlayerOutcome {
                    player: PlayerId(1),
                    result: mtgml_model::PlayerResult::Loss,
                },
                mtgml_model::PlayerOutcome {
                    player: PlayerId(2),
                    result: mtgml_model::PlayerResult::Win,
                },
            ],
        }
    );
}

#[test]
fn magic_rules_state_based_actions_same_owner_deaths_require_order_before_mutation() {
    let state = two_same_owner_deaths_at_upkeep();
    let before = state.clone();
    let mut kernel = crate::magic::MagicRulesKernel::s3_a_conformance_candidate();
    let result = kernel.advance_forced_progress(&state);
    assert_eq!(state, before, "forced progress must not mutate its input");
    let transition = result.unwrap_or_else(|error| {
        panic!("S3.A RED: Magic kernel has no same-owner Graveyard Order stage: {error:?}")
    });
    assert!(transition.accepted);
    assert_eq!(transition.next_state.zones, before.zones);
    assert_eq!(transition.next_state.core.players, before.core.players);
    assert!(matches!(
        transition.next_decision.as_ref().map(|request| &request.decision),
        Some(mtgml_decision::DecisionDomainV2::Order {
            minimum: 2,
            maximum: 2
        })
    ));
}

#[test]
fn production_s1_contract_state_based_actions_boundary_remains_frozen() {
    let state = sba_upkeep_state();
    let before = state.clone();
    let mut kernel = crate::ProgramKernelV1::for_admitted_execution(
        mtgml_model::ExecutionProgramV1::MagicRules,
        crate::semantic_execution_generated::magic_turn_structure_0_1_0_semantic_contract_id(),
    )
    .expect("the exact production S1 contract remains admitted");
    let error = kernel
        .advance_forced_progress(&state)
        .expect_err("S1 must not gain the S3.A SBA producer");
    assert!(matches!(
        error,
        crate::KernelExecutionError::UnsupportedRulesBoundary(
            crate::UnsupportedRulesBoundary::BasicPriority
        )
    ));
    assert_eq!(state, before);
}

#[test]
fn production_s3_a_identity_is_distinct_and_executes_the_reviewed_sba_scope() {
    let mut state = sba_upkeep_state();
    state.core.players.get_mut(&PlayerId(1)).unwrap().life = 0;
    let s3_id = crate::semantic_execution_generated::
        magic_s3_a_ordered_sba_0_1_0_semantic_contract_id();
    assert_ne!(
        s3_id,
        crate::semantic_execution_generated::magic_turn_structure_0_1_0_semantic_contract_id(),
        "S3.A must not reinterpret the historical S1 semantic identity"
    );
    assert!(crate::validate_runtime_state_for_contract(
        mtgml_model::ExecutionProgramV1::MagicRules,
        s3_id.clone(),
        &state,
        &mtgml_model::EpisodeStatus::Running,
    )
    .is_ok());
    let before = state.clone();
    let mut s3 = crate::ProgramKernelV1::for_admitted_execution(
        mtgml_model::ExecutionProgramV1::MagicRules,
        s3_id,
    )
    .expect("generated production S3.A identity admits its exact profile");
    let transition = s3
        .advance_forced_progress(&state)
        .expect("production S3.A executes its declared loss batch");
    assert_eq!(state, before);
    assert!(transition.accepted);
    assert!(transition.next_state.core.players[&PlayerId(1)].has_lost);
    assert!(transition.events.iter().any(|event| matches!(
        event.event,
        crate::AuthoritativeRuleEventKind::StateBasedActionsApplied { .. }
    )));

    let mut s1 = crate::ProgramKernelV1::for_admitted_execution(
        mtgml_model::ExecutionProgramV1::MagicRules,
        crate::semantic_execution_generated::magic_turn_structure_0_1_0_semantic_contract_id(),
    )
    .unwrap();
    assert!(matches!(
        s1.advance_forced_progress(&state),
        Err(crate::KernelExecutionError::UnsupportedRulesBoundary(
            crate::UnsupportedRulesBoundary::BasicPriority
        ))
    ));
}

fn fresh_no_order_combat_death_at(step: mtgml_state::CombatStep) -> EngineState {
    let mut state = two_same_owner_deaths_at_upkeep();
    let mtgml_state::BaseCharacteristics::Simple { power, .. } = state.foundation_sources
        [&GameObjectId(1)]
        .base_characteristics;
    state
        .foundation_sources
        .get_mut(&GameObjectId(1))
        .unwrap()
        .base_characteristics = mtgml_state::BaseCharacteristics::Simple {
        power,
        toughness: 2,
    };
    state.core.position = mtgml_state::TurnPosition::Combat { step };
    state.combat = Some(mtgml_state::CombatState {
        defending_player: PlayerId(2),
        attackers: vec![GameObjectId(1)],
        blockers: std::collections::BTreeMap::from([(
            GameObjectId(1),
            Some(GameObjectId(2)),
        )]),
    });
    mtgml_state::validate_engine_state(&state)
        .expect("fresh no-order combat-death fixture must be structurally valid");
    state
}

fn assert_fresh_no_order_combat_boundary_rejects(step: mtgml_state::CombatStep) {
    let state = fresh_no_order_combat_death_at(step);
    let result = crate::state_based_actions::derive_bounded_sba_round_plan(&state);
    if let Ok(plan) = &result {
        assert_eq!(plan.selected_sba_actions.len(), 1);
        assert!(plan.apnap_owners.is_empty(), "one selected card needs no order");
    }
    assert!(
        matches!(
            result,
            Err(crate::state_based_actions::SbaContinuationValidationError::UnsupportedSbaProfile)
        ),
        "fresh no-order combat participant at {step:?} must fail closed, got {result:?}"
    );
}

#[test]
fn task9b_no_order_beginning_of_combat_fail_closed() {
    assert_fresh_no_order_combat_boundary_rejects(
        mtgml_state::CombatStep::BeginningOfCombat,
    );
}

#[test]
fn task9b_no_order_declare_attackers_fail_closed() {
    assert_fresh_no_order_combat_boundary_rejects(
        mtgml_state::CombatStep::DeclareAttackers,
    );
}

#[test]
fn task9b_no_order_declare_blockers_fail_closed() {
    assert_fresh_no_order_combat_boundary_rejects(
        mtgml_state::CombatStep::DeclareBlockers,
    );
}

#[test]
fn task9b_no_order_end_of_combat_fail_closed() {
    assert_fresh_no_order_combat_boundary_rejects(mtgml_state::CombatStep::EndOfCombat);
}
