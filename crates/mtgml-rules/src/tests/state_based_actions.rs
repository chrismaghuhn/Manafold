// S3.A1 RED witnesses enter the real private Magic rules kernel. The state
// fixtures are ordinary valid inputs; the current S1 kernel is expected to
// stop at its typed Basic Priority boundary because no S3.A producer exists.

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
