// S3.A1 RED witnesses enter the real private Magic rules kernel through the
// fixed S3.A conformance candidate. The frozen production S1 contract remains
// covered separately and retains its own Basic Priority boundary.

fn sba_upkeep_state() -> EngineState {
    let mut state = synthetic_state();
    state.execution.pending_decision = None;
    state.execution.continuations.clear();
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
fn magic_rules_state_based_actions_round_is_missing_before_priority() {
    let mut state = sba_upkeep_state();
    state.core.players.get_mut(&PlayerId(1)).unwrap().life = 0;
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
