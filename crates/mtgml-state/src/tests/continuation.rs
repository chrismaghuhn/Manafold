// Ownership fragment: synthetic assembly continuation program evidence. Included lexically by tests.rs so
// every identity remains tests::<name>.

#[test]
fn pending_decision_must_reference_an_existing_continuation() {
    let mut state = synthetic_state();
    let pending = state.execution.pending_decision.as_mut().unwrap();
    pending.request.continuation_id = Some(ContinuationId(42));
    assert!(matches!(
        validate_engine_state(&state),
        Err(EngineStateViolation::M2Shape(
            M2ShapeViolation::ContinuationReference
        ))
    ));
}

#[test]
fn continuation_stage_must_match_its_payload() {
    let mut state = empty_shell();
    state.execution.continuations.insert(
        ContinuationId(1),
        ContinuationRecordV2 {
            id: ContinuationId(1),
            actor: PlayerId(1),
            created_at_revision: StateRevision(0),
            stage_index: 2,
            payload: ContinuationPayloadV2::SyntheticM2Assembly {
                stage: AssemblyStageV2::ChooseCount,
                selected_count: None,
                selected_piece_keys: Vec::new(),
                ordered_piece_keys: Vec::new(),
            },
        },
    );
    state.allocators.next_continuation_id = ContinuationId(2);
    assert!(matches!(
        validate_engine_state(&state),
        Err(EngineStateViolation::M2Shape(
            M2ShapeViolation::ContinuationStage
        ))
    ));
}

#[test]
fn continuation_revision_must_not_be_future_dated() {
    let mut state = empty_shell();
    state.execution.continuations.insert(
        ContinuationId(1),
        ContinuationRecordV2 {
            id: ContinuationId(1),
            actor: PlayerId(1),
            created_at_revision: StateRevision(3),
            stage_index: 0,
            payload: ContinuationPayloadV2::SyntheticM2Assembly {
                stage: AssemblyStageV2::ChooseCount,
                selected_count: None,
                selected_piece_keys: Vec::new(),
                ordered_piece_keys: Vec::new(),
            },
        },
    );
    state.allocators.next_continuation_id = ContinuationId(2);
    assert!(matches!(
        validate_engine_state(&state),
        Err(EngineStateViolation::M2Shape(
            M2ShapeViolation::ContinuationRevision
        ))
    ));
}

#[test]
fn continuation_actor_must_own_the_referenced_request() {
    let mut state = empty_shell();
    state.execution.continuations.insert(
        ContinuationId(1),
        ContinuationRecordV2 {
            id: ContinuationId(1),
            actor: PlayerId(1),
            created_at_revision: StateRevision(0),
            stage_index: 0,
            payload: ContinuationPayloadV2::SyntheticM2Assembly {
                stage: AssemblyStageV2::ChooseCount,
                selected_count: None,
                selected_piece_keys: Vec::new(),
                ordered_piece_keys: Vec::new(),
            },
        },
    );
    state.allocators.next_continuation_id = ContinuationId(2);
    state.allocators.next_decision_id = DecisionId(2);
    // A valid pending request owned by a different player.
    state.execution.pending_decision = Some(PendingDecisionRecordV2 {
        request: mtgml_decision::AuthoritativeDecisionRequestV2 {
            decision_id: DecisionId(1),
            player_decision_id: mtgml_model::PlayerDecisionIdV1(1),
            state_revision: StateRevision(0),
            actor: PlayerId(2),
            visibility: mtgml_decision::DecisionVisibility::Public,
            decision: mtgml_decision::DecisionDomainV2::ChooseOne,
            candidates: vec![mtgml_decision::AuthoritativeCandidateV2 {
                candidate_id: mtgml_model::CandidateIdV1(0),
                visible_intent: mtgml_decision::CandidateIntent::PassPriority,
                trusted_binding: mtgml_decision::EngineCandidateBinding::PassPriority,
            }],
            continuation_id: Some(ContinuationId(1)),
        },
    });
    assert_eq!(
        validate_engine_state(&state),
        Err(EngineStateViolation::M2Shape(
            M2ShapeViolation::ContinuationActor
        ))
    );
}

#[test]
fn assembly_payload_stage_invariants_are_enforced() {
    // ChooseCount must carry no partial values.
    assert!(
        validate_engine_state(&state_with_continuation(assembly_continuation(
            AssemblyStageV2::ChooseCount,
            None,
            vec![],
            vec![],
        )))
        .is_ok()
    );
    assert!(
        validate_engine_state(&state_with_continuation(assembly_continuation(
            AssemblyStageV2::ChooseCount,
            Some(2),
            vec![],
            vec![],
        )))
        .is_err()
    );

    // ChooseMembers carries only the decided count.
    assert!(
        validate_engine_state(&state_with_continuation(assembly_continuation(
            AssemblyStageV2::ChooseMembers,
            Some(2),
            vec![],
            vec![],
        )))
        .is_ok()
    );
    assert!(
        validate_engine_state(&state_with_continuation(assembly_continuation(
            AssemblyStageV2::ChooseMembers,
            None,
            vec![],
            vec![],
        )))
        .is_err()
    );
    assert!(
        validate_engine_state(&state_with_continuation(assembly_continuation(
            AssemblyStageV2::ChooseMembers,
            Some(2),
            vec![0, 1],
            vec![],
        )))
        .is_err()
    );

    // OrderMembers carries exactly the canonical member set and no order.
    assert!(
        validate_engine_state(&state_with_continuation(assembly_continuation(
            AssemblyStageV2::OrderMembers,
            Some(2),
            vec![0, 1],
            vec![],
        )))
        .is_ok()
    );
    // Member-set size disagrees with the decided count.
    assert!(
        validate_engine_state(&state_with_continuation(assembly_continuation(
            AssemblyStageV2::OrderMembers,
            Some(3),
            vec![0, 1],
            vec![],
        )))
        .is_err()
    );
    // Noncanonical set representation.
    assert!(
        validate_engine_state(&state_with_continuation(assembly_continuation(
            AssemblyStageV2::OrderMembers,
            Some(2),
            vec![1, 0],
            vec![],
        )))
        .is_err()
    );
    // A persisted order is never a valid partial value.
    assert!(
        validate_engine_state(&state_with_continuation(assembly_continuation(
            AssemblyStageV2::OrderMembers,
            Some(2),
            vec![0, 1],
            vec![1, 0],
        )))
        .is_err()
    );
}

#[test]
fn continuation_pending_program_coherence_matrix() {
    use crate::m2_shape::SYNTHETIC_COUNT_MAX;
    use mtgml_decision::AuthoritativeCandidateV2;
    use mtgml_model::CandidateIdV1;

    // Coherent base: ChooseCount stage of the supported program.
    let coherent = state_with_continuation(assembly_continuation(
        AssemblyStageV2::ChooseCount,
        None,
        vec![],
        vec![],
    ));
    validate_engine_state(&coherent).unwrap();

    let set_pending_domain = |state: &mut EngineState, domain, pieces: Vec<u32>| {
        let pending = state.execution.pending_decision.as_mut().unwrap();
        pending.request.decision = domain;
        pending.request.candidates = pieces
            .iter()
            .enumerate()
            .map(|(index, piece)| AuthoritativeCandidateV2 {
                candidate_id: CandidateIdV1(index as u32),
                visible_intent: mtgml_decision::CandidateIntent::SelectMode { mode_index: *piece },
                trusted_binding: mtgml_decision::EngineCandidateBinding::SelectMode {
                    mode_index: *piece,
                },
            })
            .collect();
    };

    // BLOCKER regression: the engine may offer exactly the supported
    // program interval - nothing wider, nothing shifted.
    for bounds in [(0_i64, 99_i64), (1, 3), (-1, 3), (0, 2)] {
        let mut state = coherent.clone();
        set_pending_domain(
            &mut state,
            mtgml_decision::DecisionDomainV2::ChooseNumber {
                minimum: bounds.0,
                maximum: bounds.1,
            },
            vec![],
        );
        assert!(
            validate_engine_state(&state).is_err(),
            "ChooseCount {bounds:?} must be invalid"
        );
    }

    // Wrong pending decision family at the ChooseCount stage.
    let mut state = coherent.clone();
    set_pending_domain(
        &mut state,
        mtgml_decision::DecisionDomainV2::ChooseMany {
            minimum: 0,
            maximum: 3,
        },
        vec![0, 1, 2],
    );
    assert!(validate_engine_state(&state).is_err());

    // selected_count disagrees with the pending ChooseMany bounds.
    let members_state = |count: u32| {
        let mut state = state_with_continuation(assembly_continuation(
            AssemblyStageV2::ChooseMembers,
            Some(count),
            vec![],
            vec![],
        ));
        set_pending_domain(
            &mut state,
            mtgml_decision::DecisionDomainV2::ChooseMany {
                minimum: count,
                maximum: count,
            },
            (0..count).collect(),
        );
        state
    };
    assert!(validate_engine_state(&members_state(2)).is_ok());
    let mut mismatched = members_state(3);
    set_pending_domain(
        &mut mismatched,
        mtgml_decision::DecisionDomainV2::ChooseMany {
            minimum: 1,
            maximum: 2,
        },
        vec![0, 1],
    );
    assert!(validate_engine_state(&mismatched).is_err());

    // A decided count outside the supported interval is unreachable and
    // invalid even with fully matching request bounds.
    let mut over_limit = members_state(SYNTHETIC_COUNT_MAX + 1);
    set_pending_domain(
        &mut over_limit,
        mtgml_decision::DecisionDomainV2::ChooseMany {
            minimum: SYNTHETIC_COUNT_MAX + 1,
            maximum: SYNTHETIC_COUNT_MAX + 1,
        },
        (0..=SYNTHETIC_COUNT_MAX).collect(),
    );
    assert!(validate_engine_state(&over_limit).is_err());

    // The only reachable OrderMembers member set is the full prefix 0..C:
    // ChooseMembers offers exactly those C candidates and requires exactly C
    // selections. An unreachable history such as [0,2] is invalid authority.
    let order_stage = |pieces: &[u32]| {
        let mut state = state_with_continuation(assembly_continuation(
            AssemblyStageV2::OrderMembers,
            Some(pieces.len() as u32),
            pieces.to_vec(),
            vec![],
        ));
        set_pending_domain(
            &mut state,
            mtgml_decision::DecisionDomainV2::Order {
                minimum: pieces.len() as u32,
                maximum: pieces.len() as u32,
            },
            pieces.to_vec(),
        );
        state
    };
    assert!(validate_engine_state(&order_stage(&[0, 1])).is_ok());
    assert!(validate_engine_state(&order_stage(&[0])).is_ok());
    assert!(validate_engine_state(&order_stage(&[0, 1, 2])).is_ok());

    // Unreachable partial history.
    assert!(validate_engine_state(&order_stage(&[0, 2])).is_err());
    // Wrong candidate surface with matching cardinality stays invalid.
    let mut wrong_surface = order_stage(&[0, 1]);
    set_pending_domain(
        &mut wrong_surface,
        mtgml_decision::DecisionDomainV2::Order {
            minimum: 2,
            maximum: 2,
        },
        vec![0, 1],
    );
    wrong_surface
        .execution
        .continuations
        .get_mut(&ContinuationId(1))
        .unwrap()
        .payload = ContinuationPayloadV2::SyntheticM2Assembly {
        stage: AssemblyStageV2::OrderMembers,
        selected_count: Some(2),
        selected_piece_keys: vec![0, 1],
        ordered_piece_keys: Vec::new(),
    };
    // Rebuild a mismatched surface on the pending side only.
    if let Some(pending) = wrong_surface.execution.pending_decision.as_mut() {
        pending.request.candidates = vec![mtgml_decision::AuthoritativeCandidateV2 {
            candidate_id: CandidateIdV1(0),
            visible_intent: mtgml_decision::CandidateIntent::SelectMode { mode_index: 0 },
            trusted_binding: mtgml_decision::EngineCandidateBinding::SelectMode { mode_index: 0 },
        }];
    }
    assert!(validate_engine_state(&wrong_surface).is_err());

    // An active continuation without a resuming pending request can never
    // become checkpointable state.
    let orphaned = {
        let mut state = coherent.clone();
        state.execution.pending_decision = None;
        state
    };
    assert!(validate_engine_state(&orphaned).is_err());
}

fn magic_order_stage_state(
    current_revision: u64,
    created_at_revision: u64,
    round_start_revision: u64,
    next_owner_index: u32,
) -> EngineState {
    use crate::{
        BaseCharacteristics, ControlHistory, FoundationCreatureSource, FoundationSourceKind,
        GameObject, KnowledgeAcquisitionReason, KnowledgeRecordV2, KnownLocationFactV2,
        SbaGraveyardOwnerOrderV1, SbaObjectCauseV1, SbaSelectedActionV1, VisibilityPartition,
        ZoneLocation, ZonePosition,
    };
    use mtgml_model::{
        CardDefinitionId, OpaqueObjectId, PhysicalCardId, PlayerDecisionIdV1, VisibleSequence,
        ZoneKind,
    };

    let mut state = synthetic_state();
    let p1 = PlayerId(1);
    let p2 = PlayerId(2);
    let location = ZoneLocation {
        zone: ZoneKind::Battlefield,
        player: None,
        position: ZonePosition::Unordered,
        visibility: VisibilityPartition::Public,
        partition: None,
    };

    // Move the existing hidden object into the public battlefield fixture.
    state
        .zones
        .locations
        .insert(GameObjectId(2), location.clone());
    state.zones.objects.get_mut(&GameObjectId(2)).unwrap().face_down = false;
    for ordered in state.zones.ordered_zones.values_mut() {
        ordered.retain(|object| *object != GameObjectId(2));
    }
    state
        .zones
        .ordered_zones
        .retain(|_, ordered| !ordered.is_empty());

    for (id, owner) in [(GameObjectId(3), p1), (GameObjectId(4), p2)] {
        state.zones.objects.insert(
            id,
            GameObject {
                id,
                physical_card: Some(PhysicalCardId(id.0)),
                card_definition: CardDefinitionId(id.0),
                owner,
                controller: owner,
                tapped: false,
                face_down: false,
            },
        );
        state.zones.locations.insert(id, location.clone());
    }
    state.allocators.next_object_id = GameObjectId(5);

    for id in [GameObjectId(1), GameObjectId(2), GameObjectId(3), GameObjectId(4)] {
        let (power, toughness, marked_damage) = match id {
            GameObjectId(1 | 2) => (2, 2, 2),
            GameObjectId(3 | 4) => (2, 0, 0),
            _ => unreachable!(),
        };
        state.foundation_sources.insert(
            id,
            FoundationCreatureSource {
                source_kind: FoundationSourceKind::Creature,
                base_characteristics: BaseCharacteristics::Simple { power, toughness },
                marked_damage,
                control_history: ControlHistory::BeforeTurnStart { turn_number: 1 },
            },
        );
    }

    // The existing P2 identity for object 2 remains valid after its public
    // location change; add the newly public identity for P1.
    let p1_object2 = OpaqueObjectId(2);
    {
        let identity = state.perspective_identities.players.get_mut(&p1).unwrap();
        identity.opaque_to_object.insert(p1_object2, GameObjectId(2));
        identity.object_to_opaque.insert(GameObjectId(2), p1_object2);
        identity.next_opaque_object_id = OpaqueObjectId(3);
    }
    state.knowledge.players.get_mut(&p1).unwrap().active.insert(
        p1_object2,
        KnowledgeRecordV2 {
            opaque_object: p1_object2,
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
        .get_mut(&p2)
        .unwrap()
        .active
        .get_mut(&OpaqueObjectId(2))
        .unwrap()
        .known_location = Some(KnownLocationFactV2 {
        location: location.clone(),
        provenance: KnowledgeAcquisitionReason::InitialConfiguration,
    });

    // Add public mappings/knowledge for objects 3 and 4 for both perspectives.
    for player in [p1, p2] {
        for id in [GameObjectId(3), GameObjectId(4)] {
            let identity = state.perspective_identities.players.get_mut(&player).unwrap();
            let opaque = identity.next_opaque_object_id;
            identity.next_opaque_object_id = OpaqueObjectId(opaque.0 + 1);
            identity.opaque_to_object.insert(opaque, id);
            identity.object_to_opaque.insert(id, opaque);
            state.knowledge.players.get_mut(&player).unwrap().active.insert(
                opaque,
                KnowledgeRecordV2 {
                    opaque_object: opaque,
                    physical_card: Some(PhysicalCardId(id.0)),
                    card_definition: Some(CardDefinitionId(id.0)),
                    known_location: Some(KnownLocationFactV2 {
                        location: location.clone(),
                        provenance: KnowledgeAcquisitionReason::InitialConfiguration,
                    }),
                    acquisition: KnowledgeAcquisitionReason::InitialConfiguration,
                    historical_locations: Vec::new(),
                },
            );
        }
    }
    state
        .knowledge
        .players
        .get_mut(&p1)
        .unwrap()
        .next_visible_sequence = VisibleSequence(1);
    state
        .knowledge
        .players
        .get_mut(&p2)
        .unwrap()
        .next_visible_sequence = VisibleSequence(1);

    state.core.position = TurnPosition::Beginning {
        step: BeginningStep::Upkeep,
    };
    state.core.priority = PriorityState::None;
    state.execution.pending_decision = None;
    state.execution.continuations.clear();
    state.revision = StateRevision(current_revision);

    let selected_sba_actions = vec![
        SbaSelectedActionV1::ObjectToOwnerGraveyard {
            object: GameObjectId(1),
            causes: vec![SbaObjectCauseV1::LethalDamage],
        },
        SbaSelectedActionV1::ObjectToOwnerGraveyard {
            object: GameObjectId(2),
            causes: vec![SbaObjectCauseV1::LethalDamage],
        },
        SbaSelectedActionV1::ObjectToOwnerGraveyard {
            object: GameObjectId(3),
            causes: vec![SbaObjectCauseV1::ZeroToughness],
        },
        SbaSelectedActionV1::ObjectToOwnerGraveyard {
            object: GameObjectId(4),
            causes: vec![SbaObjectCauseV1::ZeroToughness],
        },
    ];
    let apnap_owners = vec![p1, p2];
    let completed_owner_orders = if next_owner_index == 0 {
        Vec::new()
    } else {
        vec![SbaGraveyardOwnerOrderV1 {
            owner: p1,
            top_to_bottom: vec![GameObjectId(1), GameObjectId(3)],
        }]
    };
    let actor = apnap_owners[next_owner_index as usize];
    let mut candidates = selected_sba_actions
        .iter()
        .filter_map(|action| {
            let SbaSelectedActionV1::ObjectToOwnerGraveyard { object, .. } = action else {
                return None;
            };
            (state.zones.objects[object].owner == actor).then_some((actor, *object))
        })
        .collect::<Vec<_>>();
    candidates.sort_by_key(|(owner, object)| {
        state.perspective_identities.players[owner].object_to_opaque[object]
    });
    let request_decision = DecisionId(2 + u64::from(next_owner_index));
    let request_player_decision = PlayerDecisionIdV1(2);
    let continuation = ContinuationId(1);
    let payload = ContinuationPayloadV2::MagicSbaGraveyardOrderV1 {
        round_start_revision: StateRevision(round_start_revision),
        selected_sba_actions,
        apnap_owners,
        next_owner_index,
        completed_owner_orders,
    };
    state.execution.continuations.insert(
        continuation,
        ContinuationRecordV2 {
            id: continuation,
            actor,
            created_at_revision: StateRevision(created_at_revision),
            stage_index: payload.stage_index(),
            payload,
        },
    );
    state.allocators.next_continuation_id = ContinuationId(2);
    state.allocators.next_decision_id = DecisionId(request_decision.0 + 1);
    state.perspective_identities.players.get_mut(&actor).unwrap().next_player_decision_id =
        PlayerDecisionIdV1(request_player_decision.0 + 1);
    state.execution.pending_decision = Some(PendingDecisionRecordV2 {
        request: mtgml_decision::AuthoritativeDecisionRequestV2 {
            decision_id: request_decision,
            player_decision_id: request_player_decision,
            state_revision: StateRevision(current_revision),
            actor,
            visibility: mtgml_decision::DecisionVisibility::ActingPlayerOnly,
            decision: mtgml_decision::DecisionDomainV2::Order {
                minimum: 2,
                maximum: 2,
            },
            candidates: candidates
                .iter()
                .enumerate()
                .map(|(index, (owner, object))| {
                    let opaque = state.perspective_identities.players[owner].object_to_opaque[object];
                    mtgml_decision::AuthoritativeCandidateV2 {
                        candidate_id: mtgml_model::CandidateIdV1(index as u32),
                        visible_intent: mtgml_decision::CandidateIntent::SelectObject {
                            object: opaque,
                        },
                        trusted_binding: mtgml_decision::EngineCandidateBinding::SelectObject {
                            object: *object,
                        },
                    }
                })
                .collect(),
            continuation_id: Some(continuation),
        },
    });
    state
}

#[test]
fn magic_sba_order_initial_and_second_apnap_stages_have_exact_valid_revisions() {
    let initial = magic_order_stage_state(1, 1, 0, 0);
    validate_engine_state(&initial).unwrap();
    let second = magic_order_stage_state(2, 1, 0, 1);
    validate_engine_state(&second).unwrap();
}

#[test]
fn magic_sba_created_revision_must_follow_round_start_revision() {
    let state = magic_order_stage_state(1, 0, 0, 0);
    assert_eq!(
        validate_engine_state(&state),
        Err(EngineStateViolation::M2Shape(
            M2ShapeViolation::ContinuationRevision
        ))
    );
}

#[test]
fn magic_sba_current_revision_must_count_each_completed_owner_order() {
    let state = magic_order_stage_state(1, 1, 0, 1);
    assert_eq!(
        validate_engine_state(&state),
        Err(EngineStateViolation::M2Shape(
            M2ShapeViolation::ContinuationRevision
        ))
    );
}

#[test]
fn magic_sba_stage_revision_relation_uses_checked_arithmetic() {
    let state = magic_order_stage_state(u64::MAX, u64::MAX, u64::MAX, 0);
    assert_eq!(
        validate_engine_state(&state),
        Err(EngineStateViolation::M2Shape(
            M2ShapeViolation::ContinuationRevision
        ))
    );
}

// ---------------------------------------------------------------- M2.E lifecycle
