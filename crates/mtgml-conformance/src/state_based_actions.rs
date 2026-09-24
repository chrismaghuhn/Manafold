//! S3.A1 RED witnesses for one simultaneous bounded SBA round.
//!
//! These cases build structurally valid authoritative snapshots and submit
//! them to the real Magic rules kernel through its fixed conformance candidate. No expected SBA action is
//! applied by the fixture; each RED should currently stop at the S1
//! unsupported forced-progress boundary because S3.A has no producer yet.
//! The literal scope follows Foundation V2 and the pinned Rules snapshot
//! `wotc-cr-2026-08-07-txt-20260819-sha256-4381ad1b39ab2c05f7d03633a20f711ed37277074d3266dcba5f38cbb527423f`:
//! CR 704.5a, 704.5f, 704.5g, 404.3, and 101.4.
//!
//! Behavioral S3.A cases use the fixed `m3-conformance-testkit` candidate
//! constructor, never the admitted S1 SemanticContractId. The explicit
//! legacy regression below separately binds that frozen S1 identity.

use std::collections::BTreeMap;

use mtgml_decision::{
    AuthoritativeCandidateV2, AuthoritativeDecisionRequestV2, CandidateIntent, DecisionAnswerV2,
    DecisionDomainV2, DecisionResponseV2, DecisionVisibility, EngineCandidateBinding,
    DECISION_RESPONSE_V2_SCHEMA,
};
use mtgml_model::{
    CandidateIdV1, DecisionId, EpisodeStatus, ExecutionProgramV1, GameObjectId, OpaqueObjectId,
    PlayerDecisionIdV1, PlayerId, PlayerOutcome, PlayerResult, StateRevision, TerminalReason,
    ZoneKind,
};
use mtgml_random::RootSeed256;
use mtgml_rules::{AuthoritativeRuleEventKind, ProgramKernelV1, TransitionResult};
use mtgml_state::{
    construct_synthetic_engine_state, validate_engine_state, BaseCharacteristics,
    ContinuationPayloadV2, ContinuationRecordV2, ControlHistory, EngineState,
    FoundationCreatureSource, FoundationSourceKind, GameObject, KnowledgeAcquisitionReason,
    KnowledgeRecordV2, KnownLocationFactV2, PendingDecisionRecordV2, SbaGraveyardOwnerOrderV1,
    SbaObjectCauseV1, SbaSelectedActionV1, SyntheticResetInputs, SyntheticV4Setup,
    VisibilityPartition, ZoneLocation, ZonePosition,
};

const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);

#[derive(Debug, Clone, Copy)]
struct CreatureSpec {
    owner: PlayerId,
    toughness: i64,
    marked_damage: u64,
}

fn battlefield() -> ZoneLocation {
    ZoneLocation {
        zone: ZoneKind::Battlefield,
        player: None,
        position: ZonePosition::Unordered,
        visibility: VisibilityPartition::Public,
        partition: None,
    }
}

/// Creates only ordinary public creature facts. It does not derive an SBA,
/// order, or transition product; all objects get authorized perspective-local
/// identities, deliberately reverse-mapped from trusted object order.
fn state_with(creatures: &[CreatureSpec], life: [i64; 2]) -> EngineState {
    let mut state = construct_synthetic_engine_state(SyntheticResetInputs {
        players: [P1, P2],
        root_seed: RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap(),
        setup: SyntheticV4Setup {
            position: mtgml_state::TurnPosition::Beginning {
                step: mtgml_state::BeginningStep::Upkeep,
            },
            priority: mtgml_state::PriorityState::None,
            combat: None,
            foundation_sources: BTreeMap::new(),
        },
    })
    .unwrap();
    state.execution.pending_decision = None;
    state.execution.continuations.clear();
    state.zones.objects.clear();
    state.zones.locations.clear();
    state.zones.ordered_zones.clear();
    state.foundation_sources.clear();

    for (player, life_total) in [(P1, life[0]), (P2, life[1])] {
        state.core.players.get_mut(&player).unwrap().life = life_total;
        let identity = state
            .perspective_identities
            .players
            .get_mut(&player)
            .unwrap();
        identity.opaque_to_object.clear();
        identity.object_to_opaque.clear();
        identity.next_opaque_object_id = OpaqueObjectId(1);
        identity.retired_object_ids.clear();
        let knowledge = state.knowledge.players.get_mut(&player).unwrap();
        knowledge.active.clear();
        knowledge.retired.clear();
        knowledge.next_visible_sequence = mtgml_model::VisibleSequence(1);
    }

    let location = battlefield();
    for (index, creature) in creatures.iter().enumerate() {
        let id_value = u64::try_from(index + 1).unwrap();
        let object = GameObjectId(id_value);
        state.zones.objects.insert(
            object,
            GameObject {
                id: object,
                physical_card: Some(mtgml_model::PhysicalCardId(id_value)),
                card_definition: mtgml_model::CardDefinitionId(id_value),
                owner: creature.owner,
                controller: creature.owner,
                tapped: false,
                face_down: false,
            },
        );
        state.zones.locations.insert(object, location.clone());
        state.foundation_sources.insert(
            object,
            FoundationCreatureSource {
                source_kind: FoundationSourceKind::Creature,
                base_characteristics: BaseCharacteristics::Simple {
                    power: 2,
                    toughness: creature.toughness,
                },
                marked_damage: creature.marked_damage,
                control_history: ControlHistory::BeforeTurnStart { turn_number: 1 },
            },
        );

        let opaque_value = u64::try_from(creatures.len() - index).unwrap();
        let opaque = OpaqueObjectId(opaque_value);
        for player in [P1, P2] {
            let identity = state
                .perspective_identities
                .players
                .get_mut(&player)
                .unwrap();
            identity.opaque_to_object.insert(opaque, object);
            identity.object_to_opaque.insert(object, opaque);
            let knowledge = state.knowledge.players.get_mut(&player).unwrap();
            knowledge.active.insert(
                opaque,
                KnowledgeRecordV2 {
                    opaque_object: opaque,
                    physical_card: Some(mtgml_model::PhysicalCardId(id_value)),
                    card_definition: Some(mtgml_model::CardDefinitionId(id_value)),
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

    let next_opaque = u64::try_from(creatures.len() + 1).unwrap();
    for player in [P1, P2] {
        state
            .perspective_identities
            .players
            .get_mut(&player)
            .unwrap()
            .next_opaque_object_id = OpaqueObjectId(next_opaque);
    }
    state.allocators.next_object_id = GameObjectId(next_opaque);
    validate_engine_state(&state).expect("authored SBA round-start state must be valid");
    state
}

fn magic_kernel() -> ProgramKernelV1 {
    ProgramKernelV1::for_s3_a_conformance_testkit()
}

#[test]
fn legacy_s1_semantic_contract_keeps_its_frozen_sba_and_response_boundaries() {
    use mtgml_rules::{KernelExecutionError, UnsupportedRulesBoundary};

    let s1_manifest = mtgml_environment::magic_turn_structure_0_1_0_rules_manifest();
    let closure = s1_manifest
        .capability_closure
        .expect("S1 production rules contract has an explicit closure");
    assert_eq!(closure.len(), 1);
    assert_eq!(closure[0].key, "rules/turn-structure");
    assert_eq!(closure[0].version, "0.1.0");

    let state = state_with(
        &[CreatureSpec {
            owner: P1,
            toughness: 0,
            marked_damage: 0,
        }],
        [0, 40],
    );
    let before = state.clone();
    let mut s1 = ProgramKernelV1::for_admitted_execution(
        ExecutionProgramV1::MagicRules,
        mtgml_environment::magic_turn_structure_0_1_0_semantic_contract_id(),
    )
    .expect("the historical production S1 contract remains admitted");
    assert!(matches!(
        s1.advance_forced_progress(&state),
        Err(KernelExecutionError::UnsupportedRulesBoundary(
            UnsupportedRulesBoundary::BasicPriority
        ))
    ));
    assert!(matches!(
        s1.apply(
            &state,
            P1,
            &order_response(DecisionAnswerV2::Order {
                candidate_ids: vec![]
            })
        ),
        Err(KernelExecutionError::UnsupportedPlayerResponse)
    ));
    assert_eq!(state, before);
}

fn advance_sba(state: &EngineState, witness: &str) -> TransitionResult {
    let mut kernel = magic_kernel();
    kernel.advance_forced_progress(state).unwrap_or_else(|error| {
        panic!(
            "S3.A RED ({witness}; Foundation V2; pinned CR 704.5a/704.5f/704.5g/404.3/101.4): the real Magic kernel has no selected SBA producer yet: {error:?}"
        )
    })
}

fn object_action(object: u64, causes: Vec<SbaObjectCauseV1>) -> SbaSelectedActionV1 {
    SbaSelectedActionV1::ObjectToOwnerGraveyard {
        object: GameObjectId(object),
        causes,
    }
}

fn assert_pending_order(
    before: &EngineState,
    transition: &TransitionResult,
    actor: PlayerId,
    owners: &[PlayerId],
    objects_for_actor: &[GameObjectId],
    expected_actions: &[SbaSelectedActionV1],
) -> Vec<SbaSelectedActionV1> {
    use mtgml_model::RuleEventId;
    use mtgml_rules::{AuthoritativeRuleEvent, AuthoritativeRuleEventKind};
    use mtgml_state::SemanticDeltaOperation;

    assert!(transition.accepted);
    assert_eq!(transition.status, EpisodeStatus::Running);
    assert_eq!(
        transition.next_state.revision,
        StateRevision(before.revision.0 + 1),
        "S3.A round staging is one atomic transition revision"
    );
    assert_eq!(
        transition.delta.apply(before).unwrap(),
        transition.next_state
    );
    assert!(transition
        .events
        .iter()
        .all(|event| event.state_revision == transition.next_state.revision));
    assert_eq!(
        transition.events,
        vec![AuthoritativeRuleEvent {
            event_id: RuleEventId(1),
            state_revision: StateRevision(1),
            event: AuthoritativeRuleEventKind::DecisionCreated {
                decision: DecisionId(2),
            },
        }]
    );
    assert_eq!(
        transition.delta.audit,
        vec![SemanticDeltaOperation::DecisionCreated {
            decision: DecisionId(2),
        }]
    );
    assert_eq!(transition.delta.before_revision, before.revision);
    assert_eq!(
        transition.delta.after_revision,
        transition.next_state.revision
    );

    // No selected action is applied before every owner has ordered.
    assert_eq!(transition.next_state.zones, before.zones);
    assert_eq!(transition.next_state.core.players, before.core.players);
    assert_eq!(
        transition.next_state.foundation_sources,
        before.foundation_sources
    );
    assert_eq!(transition.next_state.random, before.random);
    assert_eq!(transition.next_state.knowledge, before.knowledge);
    for perspective in [P1, P2] {
        let before_identity = &before.perspective_identities.players[&perspective];
        let after_identity = &transition.next_state.perspective_identities.players[&perspective];
        assert_eq!(
            after_identity.opaque_to_object,
            before_identity.opaque_to_object
        );
        assert_eq!(
            after_identity.object_to_opaque,
            before_identity.object_to_opaque
        );
        assert_eq!(
            after_identity.retired_object_ids,
            before_identity.retired_object_ids
        );
        if perspective != actor {
            assert_eq!(
                after_identity.next_player_decision_id,
                before_identity.next_player_decision_id
            );
        }
    }
    assert_eq!(
        transition.next_state.allocators.next_object_id,
        before.allocators.next_object_id
    );
    assert_eq!(
        transition.next_state.core.priority,
        mtgml_state::PriorityState::None
    );

    let request = transition
        .next_decision
        .as_ref()
        .expect("the current APNAP owner must receive an Order Decision");
    assert_eq!(request.actor, actor);
    assert_eq!(request.visibility, DecisionVisibility::ActingPlayerOnly);
    assert_eq!(
        request.decision,
        DecisionDomainV2::Order {
            minimum: u32::try_from(objects_for_actor.len()).unwrap(),
            maximum: u32::try_from(objects_for_actor.len()).unwrap(),
        }
    );
    assert!(request.continuation_id.is_some());
    assert_eq!(request.candidates.len(), objects_for_actor.len());
    let visible_request = request
        .project_player_request()
        .expect("the current owner must receive a valid perspective request");
    assert_eq!(visible_request.actor, actor);
    assert_eq!(visible_request.candidates.len(), objects_for_actor.len());

    let continuation = transition
        .next_state
        .execution
        .continuations
        .values()
        .next()
        .expect("the whole selected round must persist across ordering stages");
    assert_eq!(transition.next_state.execution.continuations.len(), 1);
    assert_eq!(continuation.actor, actor);
    assert_eq!(
        transition
            .next_state
            .execution
            .pending_decision
            .as_ref()
            .map(|pending| &pending.request),
        Some(request)
    );
    let ContinuationPayloadV2::MagicSbaGraveyardOrderV1 {
        selected_sba_actions,
        apnap_owners,
        next_owner_index,
        completed_owner_orders,
        ..
    } = &continuation.payload
    else {
        panic!("S3.A order stage must use the existing typed P0 continuation");
    };
    assert_eq!(apnap_owners, owners);
    assert_eq!(*next_owner_index, 0);
    assert!(completed_owner_orders.is_empty());

    let mut expected = objects_for_actor.to_vec();
    expected.sort_by_key(|object| {
        before.perspective_identities.players[&actor].object_to_opaque[object]
    });
    let actual_bindings = request
        .candidates
        .iter()
        .enumerate()
        .map(|(index, candidate)| {
            assert_eq!(candidate.candidate_id, CandidateIdV1(index as u32));
            let EngineCandidateBinding::SelectObject { object } = candidate.trusted_binding else {
                panic!("trusted Order binding must name the selected live object");
            };
            let CandidateIntent::SelectObject { object: opaque } = candidate.visible_intent else {
                panic!("player Order candidate must expose only its opaque object identity");
            };
            assert_eq!(
                opaque,
                before.perspective_identities.players[&actor].object_to_opaque[&object]
            );
            object
        })
        .collect::<Vec<_>>();
    assert_eq!(actual_bindings, expected);
    if objects_for_actor.len() > 1 {
        let mut trusted_id_order = objects_for_actor.to_vec();
        trusted_id_order.sort_unstable();
        assert_ne!(
            actual_bindings, trusted_id_order,
            "fixture must detect an illicit GameObjectId-selected order"
        );
    }
    assert_eq!(selected_sba_actions, expected_actions);
    selected_sba_actions.clone()
}

#[test]
fn simultaneous_round_preserves_all_actions_while_apnap_order_is_pending() {
    let before = state_with(
        &[
            CreatureSpec {
                owner: P1,
                toughness: 0,
                marked_damage: 0,
            },
            CreatureSpec {
                owner: P1,
                toughness: 0,
                marked_damage: 0,
            },
            CreatureSpec {
                owner: P2,
                toughness: 0,
                marked_damage: 0,
            },
            CreatureSpec {
                owner: P2,
                toughness: 0,
                marked_damage: 0,
            },
        ],
        [40, 40],
    );
    let transition = advance_sba(&before, "complete two-owner round / APNAP staging");
    let actions = assert_pending_order(
        &before,
        &transition,
        P1,
        &[P1, P2],
        &[GameObjectId(1), GameObjectId(2)],
        &[
            object_action(1, vec![SbaObjectCauseV1::ZeroToughness]),
            object_action(2, vec![SbaObjectCauseV1::ZeroToughness]),
            object_action(3, vec![SbaObjectCauseV1::ZeroToughness]),
            object_action(4, vec![SbaObjectCauseV1::ZeroToughness]),
        ],
    );
    assert_eq!(actions.len(), 4);
}

#[test]
fn apnap_initial_order_decision_uses_the_active_player_first() {
    let mut before = state_with(
        &[
            CreatureSpec {
                owner: P1,
                toughness: 0,
                marked_damage: 0,
            },
            CreatureSpec {
                owner: P1,
                toughness: 0,
                marked_damage: 0,
            },
            CreatureSpec {
                owner: P2,
                toughness: 0,
                marked_damage: 0,
            },
            CreatureSpec {
                owner: P2,
                toughness: 0,
                marked_damage: 0,
            },
        ],
        [40, 40],
    );
    before.core.active_player = P2;
    validate_engine_state(&before).unwrap();
    let transition = advance_sba(&before, "P2-active APNAP staging");
    assert_pending_order(
        &before,
        &transition,
        P2,
        &[P2, P1],
        &[GameObjectId(3), GameObjectId(4)],
        &[
            object_action(1, vec![SbaObjectCauseV1::ZeroToughness]),
            object_action(2, vec![SbaObjectCauseV1::ZeroToughness]),
            object_action(3, vec![SbaObjectCauseV1::ZeroToughness]),
            object_action(4, vec![SbaObjectCauseV1::ZeroToughness]),
        ],
    );
}

#[test]
fn same_owner_multiple_graveyard_moves_require_a_full_order() {
    let before = state_with(
        &[
            CreatureSpec {
                owner: P1,
                toughness: 0,
                marked_damage: 0,
            },
            CreatureSpec {
                owner: P1,
                toughness: 0,
                marked_damage: 0,
            },
        ],
        [40, 40],
    );
    let transition = advance_sba(&before, "same-owner Graveyard ordering");
    assert_pending_order(
        &before,
        &transition,
        P1,
        &[P1],
        &[GameObjectId(1), GameObjectId(2)],
        &[
            object_action(1, vec![SbaObjectCauseV1::ZeroToughness]),
            object_action(2, vec![SbaObjectCauseV1::ZeroToughness]),
        ],
    );
}

#[test]
fn owner_with_one_move_is_skipped_from_apnap_order_collection() {
    let before = state_with(
        &[
            CreatureSpec {
                owner: P1,
                toughness: 0,
                marked_damage: 0,
            },
            CreatureSpec {
                owner: P1,
                toughness: 0,
                marked_damage: 0,
            },
            CreatureSpec {
                owner: P2,
                toughness: 0,
                marked_damage: 0,
            },
        ],
        [40, 40],
    );
    let transition = advance_sba(&before, "skip owner with one Graveyard move");
    assert_pending_order(
        &before,
        &transition,
        P1,
        &[P1],
        &[GameObjectId(1), GameObjectId(2)],
        &[
            object_action(1, vec![SbaObjectCauseV1::ZeroToughness]),
            object_action(2, vec![SbaObjectCauseV1::ZeroToughness]),
            object_action(3, vec![SbaObjectCauseV1::ZeroToughness]),
        ],
    );
}

#[test]
fn zero_toughness_and_lethal_damage_are_one_canonical_object_action() {
    let before = state_with(
        &[
            CreatureSpec {
                owner: P1,
                toughness: 0,
                marked_damage: 2,
            },
            CreatureSpec {
                owner: P1,
                toughness: 0,
                marked_damage: 0,
            },
        ],
        [40, 40],
    );
    let transition = advance_sba(&before, "combined causes on one selected object");
    let actions = assert_pending_order(
        &before,
        &transition,
        P1,
        &[P1],
        &[GameObjectId(1), GameObjectId(2)],
        &[
            object_action(
                1,
                vec![
                    SbaObjectCauseV1::ZeroToughness,
                    SbaObjectCauseV1::LethalDamage,
                ],
            ),
            object_action(2, vec![SbaObjectCauseV1::ZeroToughness]),
        ],
    );
    assert_eq!(
        actions,
        vec![
            object_action(
                1,
                vec![
                    SbaObjectCauseV1::ZeroToughness,
                    SbaObjectCauseV1::LethalDamage
                ],
            ),
            object_action(2, vec![SbaObjectCauseV1::ZeroToughness]),
        ]
    );
}

#[test]
fn player_loss_and_creature_deaths_stay_in_the_same_unapplied_round() {
    let before = state_with(
        &[
            CreatureSpec {
                owner: P1,
                toughness: 0,
                marked_damage: 0,
            },
            CreatureSpec {
                owner: P1,
                toughness: 0,
                marked_damage: 0,
            },
        ],
        [0, 40],
    );
    let transition = advance_sba(&before, "player loss plus same-round creature deaths");
    let actions = assert_pending_order(
        &before,
        &transition,
        P1,
        &[P1],
        &[GameObjectId(1), GameObjectId(2)],
        &[
            SbaSelectedActionV1::PlayerLoses { player: P1 },
            object_action(1, vec![SbaObjectCauseV1::ZeroToughness]),
            object_action(2, vec![SbaObjectCauseV1::ZeroToughness]),
        ],
    );
    assert_eq!(actions[0], SbaSelectedActionV1::PlayerLoses { player: P1 });
}

#[test]
fn both_players_losing_in_one_check_produce_canonical_simultaneous_draw() {
    let before = state_with(&[], [0, 0]);
    let transition = advance_sba(&before, "both-player simultaneous outcome");
    assert!(transition.accepted);
    assert!(transition.next_state.core.players[&P1].has_lost);
    assert!(transition.next_state.core.players[&P2].has_lost);
    assert_eq!(
        transition.status,
        EpisodeStatus::Terminal {
            reason: TerminalReason::SimultaneousOutcome,
            players: vec![
                PlayerOutcome {
                    player: P1,
                    result: PlayerResult::Draw
                },
                PlayerOutcome {
                    player: P2,
                    result: PlayerResult::Draw
                },
            ],
        }
    );
    assert!(transition.next_decision.is_none());
}

#[test]
fn one_player_rules_loss_produces_sorted_win_loss_terminal_status() {
    let before = state_with(&[], [0, 40]);
    let transition = advance_sba(&before, "one-player rules loss");
    assert!(transition.accepted);
    assert_eq!(
        transition.status,
        EpisodeStatus::Terminal {
            reason: TerminalReason::RulesLoss,
            players: vec![
                PlayerOutcome {
                    player: P1,
                    result: PlayerResult::Loss
                },
                PlayerOutcome {
                    player: P2,
                    result: PlayerResult::Win
                },
            ],
        }
    );
    assert!(transition.next_decision.is_none());
}

fn state_with_order_stage(
    mut state: EngineState,
    selected_sba_actions: Vec<SbaSelectedActionV1>,
    apnap_owners: Vec<PlayerId>,
    actor: PlayerId,
    next_owner_index: u32,
    completed_owner_orders: Vec<SbaGraveyardOwnerOrderV1>,
    revisions: (u64, u64, u64),
) -> EngineState {
    let (current_revision, created_at_revision, round_start_revision) = revisions;
    state.revision = StateRevision(current_revision);
    let continuation = mtgml_model::ContinuationId(1);
    let mut bindings = selected_sba_actions
        .iter()
        .filter_map(|action| {
            let SbaSelectedActionV1::ObjectToOwnerGraveyard { object, .. } = action else {
                return None;
            };
            (state.zones.objects[object].owner == actor).then_some(*object)
        })
        .map(|object| {
            let opaque = state.perspective_identities.players[&actor].object_to_opaque[&object];
            (opaque, object)
        })
        .collect::<Vec<_>>();
    bindings.sort_by_key(|(opaque, _)| *opaque);
    let candidates = bindings
        .iter()
        .enumerate()
        .map(|(index, (opaque, object))| AuthoritativeCandidateV2 {
            candidate_id: CandidateIdV1(index as u32),
            visible_intent: CandidateIntent::SelectObject { object: *opaque },
            trusted_binding: EngineCandidateBinding::SelectObject { object: *object },
        })
        .collect();
    state.execution.continuations.insert(
        continuation,
        ContinuationRecordV2 {
            id: continuation,
            actor,
            created_at_revision: StateRevision(created_at_revision),
            stage_index: next_owner_index as u16,
            payload: ContinuationPayloadV2::MagicSbaGraveyardOrderV1 {
                round_start_revision: StateRevision(round_start_revision),
                selected_sba_actions,
                apnap_owners,
                next_owner_index,
                completed_owner_orders,
            },
        },
    );
    let decision_id = DecisionId(2 + u64::from(next_owner_index));
    state.execution.pending_decision = Some(PendingDecisionRecordV2 {
        request: AuthoritativeDecisionRequestV2 {
            decision_id,
            player_decision_id: PlayerDecisionIdV1(2),
            state_revision: StateRevision(current_revision),
            actor,
            visibility: DecisionVisibility::ActingPlayerOnly,
            decision: DecisionDomainV2::Order {
                minimum: 2,
                maximum: 2,
            },
            candidates,
            continuation_id: Some(continuation),
        },
    });
    state.allocators.next_decision_id = DecisionId(decision_id.0 + 1);
    state.allocators.next_continuation_id = mtgml_model::ContinuationId(2);
    state
        .perspective_identities
        .players
        .get_mut(&actor)
        .unwrap()
        .next_player_decision_id = PlayerDecisionIdV1(3);
    validate_engine_state(&state).expect("authored pending-Order state must be valid");
    state
}

fn state_with_pending_order() -> EngineState {
    let state = state_with(
        &[
            CreatureSpec {
                owner: P1,
                toughness: 0,
                marked_damage: 0,
            },
            CreatureSpec {
                owner: P1,
                toughness: 0,
                marked_damage: 0,
            },
        ],
        [40, 40],
    );
    state_with_order_stage(
        state,
        vec![
            object_action(1, vec![SbaObjectCauseV1::ZeroToughness]),
            object_action(2, vec![SbaObjectCauseV1::ZeroToughness]),
        ],
        vec![P1],
        P1,
        0,
        Vec::new(),
        (1, 1, 0),
    )
}

fn state_with_pending_two_owner_order() -> EngineState {
    let state = state_with(
        &[
            CreatureSpec {
                owner: P1,
                toughness: 0,
                marked_damage: 0,
            },
            CreatureSpec {
                owner: P1,
                toughness: 0,
                marked_damage: 0,
            },
            CreatureSpec {
                owner: P2,
                toughness: 0,
                marked_damage: 0,
            },
            CreatureSpec {
                owner: P2,
                toughness: 0,
                marked_damage: 0,
            },
        ],
        [40, 40],
    );
    state_with_order_stage(
        state,
        (1..=4)
            .map(|id| object_action(id, vec![SbaObjectCauseV1::ZeroToughness]))
            .collect(),
        vec![P1, P2],
        P1,
        0,
        Vec::new(),
        (1, 1, 0),
    )
}

fn state_with_second_owner_order() -> EngineState {
    let state = state_with(
        &[
            CreatureSpec {
                owner: P1,
                toughness: 0,
                marked_damage: 0,
            },
            CreatureSpec {
                owner: P1,
                toughness: 0,
                marked_damage: 0,
            },
            CreatureSpec {
                owner: P2,
                toughness: 0,
                marked_damage: 0,
            },
            CreatureSpec {
                owner: P2,
                toughness: 0,
                marked_damage: 0,
            },
        ],
        [40, 40],
    );
    state_with_order_stage(
        state,
        (1..=4)
            .map(|id| object_action(id, vec![SbaObjectCauseV1::ZeroToughness]))
            .collect(),
        vec![P1, P2],
        P2,
        1,
        vec![SbaGraveyardOwnerOrderV1 {
            owner: P1,
            top_to_bottom: vec![GameObjectId(1), GameObjectId(2)],
        }],
        (2, 1, 0),
    )
}

#[test]
fn task6_sba_semantic_validator_accepts_a_valid_first_owner_stage() {
    let state = state_with_pending_two_owner_order();
    validate_engine_state(&state).unwrap();
    assert_eq!(
        magic_kernel().validate_s3_a_conformance_continuation(&state),
        Ok(())
    );
}

#[test]
fn task6_sba_semantic_validator_accepts_a_valid_resumed_second_owner_stage() {
    let state = state_with_second_owner_order();
    validate_engine_state(&state).unwrap();
    assert_eq!(
        magic_kernel().validate_s3_a_conformance_continuation(&state),
        Ok(())
    );
}

#[test]
fn task6_sba_semantic_validator_rejects_a_stale_cause_set() {
    let mut state = state_with_pending_two_owner_order();
    let continuation = state.execution.continuations.values_mut().next().unwrap();
    let ContinuationPayloadV2::MagicSbaGraveyardOrderV1 {
        selected_sba_actions,
        ..
    } = &mut continuation.payload
    else {
        unreachable!()
    };
    selected_sba_actions[0] = object_action(1, vec![SbaObjectCauseV1::LethalDamage]);
    validate_engine_state(&state).expect("a stale semantic cause remains structurally well-formed");
    let before = state.clone();
    let digest_before = state.digest().unwrap();
    assert_eq!(
        magic_kernel().validate_s3_a_conformance_continuation(&state),
        Err(mtgml_rules::SbaContinuationValidationError::SelectedActionSetMismatch)
    );
    assert_eq!(state, before, "semantic validation must not mutate state");
    assert_eq!(state.digest().unwrap(), digest_before);
}

#[test]
fn task6_sba_semantic_validator_rejects_a_missing_new_loss_action() {
    let mut state = state_with_pending_two_owner_order();
    state.core.players.get_mut(&P1).unwrap().life = 0;
    validate_engine_state(&state)
        .expect("a newly applicable loss action is not a generic state-shape defect");
    assert_eq!(
        magic_kernel().validate_s3_a_conformance_continuation(&state),
        Err(mtgml_rules::SbaContinuationValidationError::SelectedActionSetMismatch)
    );
}

#[test]
fn task6_sba_semantic_validator_rederives_apnap_from_the_active_player() {
    let mut state = state_with_pending_two_owner_order();
    state.core.active_player = P2;
    validate_engine_state(&state)
        .expect("persisted owner membership and current actor remain structurally coherent");
    assert_eq!(
        magic_kernel().validate_s3_a_conformance_continuation(&state),
        Err(mtgml_rules::SbaContinuationValidationError::ApnapOwnersMismatch)
    );
}

#[test]
fn task6_sba_semantic_validator_fails_closed_when_priority_is_already_held() {
    let mut state = state_with_pending_two_owner_order();
    state.core.priority = mtgml_state::PriorityState::HeldBy {
        player: P1,
        consecutive_passes: 0,
    };
    validate_engine_state(&state).unwrap();
    assert_eq!(
        magic_kernel().validate_s3_a_conformance_continuation(&state),
        Err(mtgml_rules::SbaContinuationValidationError::UnsupportedSbaProfile)
    );
}

#[test]
fn task6_sba_semantic_validator_fails_closed_when_creature_facts_are_missing() {
    let mut state = state_with_pending_two_owner_order();
    state.foundation_sources.remove(&GameObjectId(1));
    validate_engine_state(&state)
        .expect("generic state structure does not derive Magic creature facts");
    assert_eq!(
        magic_kernel().validate_s3_a_conformance_continuation(&state),
        Err(mtgml_rules::SbaContinuationValidationError::UnsupportedSbaProfile)
    );
}

#[test]
fn task6_sba_conformance_profile_rejects_format_untap_and_partial_loss() {
    let mut format_state = state_with_pending_two_owner_order();
    format_state.format = mtgml_state::FormatState::Commander {
        state: mtgml_state::CommanderState {
            designations: std::collections::BTreeMap::from([(
                P1,
                vec![mtgml_model::PhysicalCardId(1)],
            )]),
            cast_counts: std::collections::BTreeMap::new(),
            damage: std::collections::BTreeMap::new(),
        },
    };
    validate_engine_state(&format_state).unwrap();
    assert_eq!(
        magic_kernel().validate_s3_a_conformance_continuation(&format_state),
        Err(mtgml_rules::SbaContinuationValidationError::UnsupportedSbaProfile)
    );

    let mut untap = state_with_pending_two_owner_order();
    untap.core.position = mtgml_state::TurnPosition::Beginning {
        step: mtgml_state::BeginningStep::Untap,
    };
    validate_engine_state(&untap).unwrap();
    assert_eq!(
        magic_kernel().validate_s3_a_conformance_continuation(&untap),
        Err(mtgml_rules::SbaContinuationValidationError::UnsupportedSbaProfile)
    );

    let mut partially_lost = state_with_pending_two_owner_order();
    partially_lost.core.players.get_mut(&P1).unwrap().has_lost = true;
    validate_engine_state(&partially_lost).unwrap();
    assert_eq!(
        magic_kernel().validate_s3_a_conformance_continuation(&partially_lost),
        Err(mtgml_rules::SbaContinuationValidationError::UnsupportedSbaProfile)
    );
}

#[test]
fn task6_sba_conformance_profile_rejects_effect_trigger_delay_and_stack_surfaces() {
    use mtgml_model::{EffectInstanceId, StackObjectId, TriggerInstanceId};
    use mtgml_state::{EffectRecord, StackRecord, TriggerRecord};

    let mut effects = state_with_pending_two_owner_order();
    effects.execution.effects.insert(
        EffectInstanceId(1),
        EffectRecord {
            id: EffectInstanceId(1),
            label: "unsupported".into(),
        },
    );
    assert_eq!(
        magic_kernel().validate_s3_a_conformance_continuation(&effects),
        Err(mtgml_rules::SbaContinuationValidationError::UnsupportedSbaProfile)
    );

    let mut triggers = state_with_pending_two_owner_order();
    triggers.execution.waiting_triggers.insert(
        TriggerInstanceId(1),
        TriggerRecord {
            id: TriggerInstanceId(1),
            controller: P1,
        },
    );
    assert_eq!(
        magic_kernel().validate_s3_a_conformance_continuation(&triggers),
        Err(mtgml_rules::SbaContinuationValidationError::UnsupportedSbaProfile)
    );

    let mut delayed = state_with_pending_two_owner_order();
    delayed.execution.delayed_effects.insert(
        EffectInstanceId(1),
        EffectRecord {
            id: EffectInstanceId(1),
            label: "unsupported".into(),
        },
    );
    assert_eq!(
        magic_kernel().validate_s3_a_conformance_continuation(&delayed),
        Err(mtgml_rules::SbaContinuationValidationError::UnsupportedSbaProfile)
    );

    let mut stack = state_with_pending_two_owner_order();
    stack.zones.stack_records.insert(
        StackObjectId(1),
        StackRecord {
            id: StackObjectId(1),
            controller: P1,
            source_object: None,
            source_ability: None,
        },
    );
    stack.zones.stack_order.push(StackObjectId(1));
    stack.allocators.next_stack_object_id = StackObjectId(2);
    validate_engine_state(&stack).unwrap();
    assert_eq!(
        magic_kernel().validate_s3_a_conformance_continuation(&stack),
        Err(mtgml_rules::SbaContinuationValidationError::UnsupportedSbaProfile)
    );
}

fn order_response(answer: DecisionAnswerV2) -> DecisionResponseV2 {
    DecisionResponseV2 {
        schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
        player_decision_id: PlayerDecisionIdV1(2),
        state_revision: StateRevision(1),
        answer,
    }
}

fn assert_order_rejection_without_mutation(answer: DecisionAnswerV2, witness: &str) {
    let state = state_with_pending_order();
    let before = state.clone();
    let digest = state.digest().unwrap();
    let request = &state.execution.pending_decision.as_ref().unwrap().request;
    let visible = request.project_player_request().unwrap();
    let response = order_response(answer);
    assert!(
        response.validate_for(&visible).is_err(),
        "{witness}: malformed fixture answer must be rejected by the closed Decision shape"
    );
    let mut kernel = magic_kernel();
    let result = kernel.apply(&state, P1, &response);
    assert_eq!(state, before, "{witness}: apply must not mutate its input");
    assert_eq!(state.digest().unwrap(), digest, "{witness}: digest changed");
    let transition = result.unwrap_or_else(|error| {
        panic!("S3.A RED ({witness}): Order response handler is absent: {error:?}")
    });
    assert!(
        !transition.accepted,
        "{witness}: malformed order was accepted"
    );
    assert_eq!(transition.next_state, before);
}

#[test]
fn incomplete_order_is_rejected_without_mutation() {
    use mtgml_decision::DecisionValidationError::AnswerCardinality;

    let state = state_with_pending_order();
    let response = order_response(DecisionAnswerV2::Order {
        candidate_ids: vec![CandidateIdV1(0)],
    });
    let visible = state
        .execution
        .pending_decision
        .as_ref()
        .unwrap()
        .request
        .project_player_request()
        .unwrap();
    assert_eq!(response.validate_for(&visible), Err(AnswerCardinality));
    assert_order_rejection_without_mutation(response.answer, "incomplete owner permutation");
}

#[test]
fn duplicate_order_member_is_rejected_without_mutation() {
    use mtgml_decision::DecisionValidationError::DuplicateAnswerCandidate;

    let state = state_with_pending_order();
    let response = order_response(DecisionAnswerV2::Order {
        candidate_ids: vec![CandidateIdV1(0), CandidateIdV1(0)],
    });
    let visible = state
        .execution
        .pending_decision
        .as_ref()
        .unwrap()
        .request
        .project_player_request()
        .unwrap();
    assert_eq!(
        response.validate_for(&visible),
        Err(DuplicateAnswerCandidate)
    );
    assert_order_rejection_without_mutation(response.answer, "duplicate owner permutation member");
}

#[test]
fn pending_order_rejection_preserves_the_entire_authoritative_world() {
    let state = state_with_pending_order();
    let before = state.clone();
    let digest = state.digest().unwrap();
    let mut kernel = magic_kernel();
    let result = kernel.apply(
        &state,
        P1,
        &order_response(DecisionAnswerV2::Order {
            candidate_ids: vec![CandidateIdV1(0)],
        }),
    );
    assert_eq!(state, before, "pending Order input must remain immutable");
    assert_eq!(state.digest().unwrap(), digest);
    match result {
        Ok(transition) => {
            assert!(!transition.accepted);
            assert_eq!(transition.next_state, before);
        }
        Err(error) => panic!(
            "S3.A RED: invalid pending-Order response has no semantic response handler: {error:?}"
        ),
    }
}

#[test]
fn first_owner_order_only_advances_the_same_round_to_the_next_apnap_owner() {
    let state = state_with_pending_two_owner_order();
    let before = state.clone();
    let first_request = state
        .execution
        .pending_decision
        .as_ref()
        .unwrap()
        .request
        .clone();
    let response = DecisionResponseV2 {
        schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
        player_decision_id: first_request.player_decision_id,
        state_revision: first_request.state_revision,
        // Candidate order is reverse GameObjectId order because the fixture's
        // P1 opaque identity mapping is deliberately reversed.
        answer: DecisionAnswerV2::Order {
            candidate_ids: vec![CandidateIdV1(1), CandidateIdV1(0)],
        },
    };
    assert!(response
        .validate_for(&first_request.project_player_request().unwrap())
        .is_ok());
    let mut kernel = magic_kernel();
    let result = kernel.apply(&state, P1, &response);
    assert_eq!(state, before, "kernel input must remain immutable");
    let transition = result.unwrap_or_else(|error| {
        panic!("S3.A RED: a valid first-owner order has no staged APNAP response path: {error:?}")
    });

    assert!(transition.accepted);
    assert_eq!(transition.next_state.revision, StateRevision(2));
    assert_eq!(
        transition.delta.apply(&before).unwrap(),
        transition.next_state
    );
    assert!(transition
        .events
        .iter()
        .all(|event| event.state_revision == StateRevision(2)));
    assert_eq!(transition.events.len(), 3);
    assert!(matches!(
        &transition.events[0].event,
        AuthoritativeRuleEventKind::DecisionCleared { decision }
            if *decision == first_request.decision_id
    ));
    assert!(matches!(
        &transition.events[1].event,
        AuthoritativeRuleEventKind::SbaGraveyardOrderChosen {
            continuation,
            owner,
            top_to_bottom,
        } if Some(*continuation) == first_request.continuation_id
            && *owner == P1
            && *top_to_bottom == vec![GameObjectId(1), GameObjectId(2)]
    ));
    assert!(matches!(
        &transition.events[2].event,
        AuthoritativeRuleEventKind::DecisionCreated { decision }
            if Some(*decision) == transition.next_decision.as_ref().map(|request| request.decision_id)
    ));
    assert_eq!(transition.next_state.zones, before.zones);
    assert_eq!(transition.next_state.core.players, before.core.players);
    assert_eq!(
        transition.next_state.foundation_sources,
        before.foundation_sources
    );
    assert_eq!(transition.next_state.random, before.random);
    assert_eq!(transition.next_state.knowledge, before.knowledge);
    assert_eq!(
        transition.next_state.core.priority,
        mtgml_state::PriorityState::None
    );

    let second_request = transition.next_decision.as_ref().unwrap();
    assert_eq!(second_request.actor, P2);
    assert_eq!(second_request.decision_id, DecisionId(3));
    assert_eq!(second_request.player_decision_id, PlayerDecisionIdV1(2));
    assert_eq!(
        transition.next_state.perspective_identities.players[&P1].next_player_decision_id,
        before.perspective_identities.players[&P1].next_player_decision_id
    );
    assert_eq!(
        transition.next_state.perspective_identities.players[&P2]
            .next_player_decision_id
            .0,
        before.perspective_identities.players[&P2]
            .next_player_decision_id
            .0
            + 1
    );
    assert_eq!(
        second_request.continuation_id,
        first_request.continuation_id
    );
    assert_eq!(
        second_request.decision,
        DecisionDomainV2::Order {
            minimum: 2,
            maximum: 2,
        }
    );

    let continuation = transition
        .next_state
        .execution
        .continuations
        .values()
        .next()
        .unwrap();
    assert_eq!(continuation.id, mtgml_model::ContinuationId(1));
    assert_eq!(continuation.actor, P2);
    assert_eq!(continuation.stage_index, 1);
    assert_eq!(
        continuation.payload,
        ContinuationPayloadV2::MagicSbaGraveyardOrderV1 {
            round_start_revision: StateRevision(0),
            selected_sba_actions: (1..=4)
                .map(|id| object_action(id, vec![SbaObjectCauseV1::ZeroToughness]))
                .collect(),
            apnap_owners: vec![P1, P2],
            next_owner_index: 1,
            completed_owner_orders: vec![SbaGraveyardOwnerOrderV1 {
                owner: P1,
                top_to_bottom: vec![GameObjectId(1), GameObjectId(2)],
            }],
        }
    );

    assert_eq!(
        transition.delta.audit,
        transition
            .events
            .iter()
            .map(|event| event.event.semantic_delta())
            .collect::<Vec<_>>()
    );

    let mut wrong_owner_event = transition.clone();
    let AuthoritativeRuleEventKind::SbaGraveyardOrderChosen { owner, .. } =
        &mut wrong_owner_event.events[1].event
    else {
        unreachable!()
    };
    *owner = P2;
    wrong_owner_event.delta.audit = wrong_owner_event
        .events
        .iter()
        .map(|event| event.event.semantic_delta())
        .collect();
    assert!(matches!(
        mtgml_rules::validate_transition_contract(&before, &wrong_owner_event),
        Err(mtgml_rules::TransitionViolation::SbaOrder)
    ));
}

#[test]
fn task7_one_owner_final_order_waits_for_task9_atomic_application() {
    let state = state_with_pending_order();
    let before = state.clone();
    let request = state
        .execution
        .pending_decision
        .as_ref()
        .unwrap()
        .request
        .clone();
    let response = DecisionResponseV2 {
        schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
        player_decision_id: request.player_decision_id,
        state_revision: request.state_revision,
        answer: DecisionAnswerV2::Order {
            candidate_ids: vec![CandidateIdV1(1), CandidateIdV1(0)],
        },
    };
    assert!(response
        .validate_for(&request.project_player_request().unwrap())
        .is_ok());
    let mut kernel = magic_kernel();
    let result = kernel.apply(&state, P1, &response);
    assert_eq!(state, before, "the final-order RED must preserve its input");
    assert!(
        matches!(
            result,
            Err(mtgml_rules::KernelExecutionError::UnsupportedStagePath)
        ),
        "the Task-7 order path should identify the deferred Task-9 boundary: {result:?}"
    );
}

#[test]
fn task7_second_owner_final_order_waits_for_task9_atomic_application() {
    let state = state_with_second_owner_order();
    let before = state.clone();
    let request = state
        .execution
        .pending_decision
        .as_ref()
        .unwrap()
        .request
        .clone();
    assert_eq!(request.actor, P2);
    let response = DecisionResponseV2 {
        schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
        player_decision_id: request.player_decision_id,
        state_revision: request.state_revision,
        answer: DecisionAnswerV2::Order {
            candidate_ids: vec![CandidateIdV1(1), CandidateIdV1(0)],
        },
    };
    assert!(response
        .validate_for(&request.project_player_request().unwrap())
        .is_ok());
    let mut kernel = magic_kernel();
    let result = kernel.apply(&state, P2, &response);
    assert_eq!(state, before, "the final-order RED must preserve its input");
    assert!(
        matches!(
            result,
            Err(mtgml_rules::KernelExecutionError::UnsupportedStagePath)
        ),
        "the Task-7 order path should identify the deferred Task-9 boundary: {result:?}"
    );
}

#[test]
fn task7_no_order_sba_batch_is_deferred_to_task9() {
    let state = state_with(
        &[CreatureSpec {
            owner: P1,
            toughness: 0,
            marked_damage: 0,
        }],
        [40, 40],
    );
    let before = state.clone();
    let mut kernel = magic_kernel();
    assert!(matches!(
        kernel.advance_forced_progress(&state),
        Err(mtgml_rules::KernelExecutionError::UnsupportedStagePath)
    ));
    assert_eq!(state, before, "Task 7 must not apply a no-order SBA batch");
}

fn current_order_response(
    state: &EngineState,
    candidate_ids: Vec<CandidateIdV1>,
) -> DecisionResponseV2 {
    let request = &state
        .execution
        .pending_decision
        .as_ref()
        .expect("Order fixture has a pending Decision")
        .request;
    DecisionResponseV2 {
        schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
        player_decision_id: request.player_decision_id,
        state_revision: request.state_revision,
        answer: DecisionAnswerV2::Order { candidate_ids },
    }
}

fn assert_rejected_order_without_mutation(
    state: &EngineState,
    trusted_actor: PlayerId,
    response: &DecisionResponseV2,
) {
    let before = state.clone();
    let digest = state.digest().unwrap();
    let mut kernel = magic_kernel();
    let result = kernel.apply(state, trusted_actor, response);
    assert_eq!(state, &before);
    assert_eq!(state.digest().unwrap(), digest);
    let transition = result.expect("player-caused Order error is a semantic rejection");
    assert!(!transition.accepted);
    assert_eq!(transition.next_state, before);
}

#[test]
fn task7_order_actor_identity_answer_family_and_stale_revision_reject_atomically() {
    let state = state_with_pending_two_owner_order();
    let valid = current_order_response(&state, vec![CandidateIdV1(1), CandidateIdV1(0)]);
    assert_rejected_order_without_mutation(&state, P2, &valid);

    let mut stale = valid.clone();
    stale.state_revision = StateRevision(0);
    assert_rejected_order_without_mutation(&state, P1, &stale);

    let mut wrong_player_decision = valid.clone();
    wrong_player_decision.player_decision_id = PlayerDecisionIdV1(999);
    assert_rejected_order_without_mutation(&state, P1, &wrong_player_decision);

    let mut wrong_answer_family = valid;
    wrong_answer_family.answer = DecisionAnswerV2::SelectOne {
        candidate_id: CandidateIdV1(0),
    };
    assert_rejected_order_without_mutation(&state, P1, &wrong_answer_family);

    let unknown_candidate =
        current_order_response(&state, vec![CandidateIdV1(99), CandidateIdV1(0)]);
    assert_rejected_order_without_mutation(&state, P1, &unknown_candidate);
}

#[test]
fn task7_order_revalidates_saved_plan_and_continuation_binding() {
    let state = state_with_pending_two_owner_order();
    let response = current_order_response(&state, vec![CandidateIdV1(1), CandidateIdV1(0)]);

    let mut stale_plan = state.clone();
    stale_plan
        .foundation_sources
        .get_mut(&GameObjectId(1))
        .unwrap()
        .base_characteristics = BaseCharacteristics::Simple {
        power: 2,
        toughness: 2,
    };
    validate_engine_state(&stale_plan).unwrap();
    let before = stale_plan.clone();
    let digest = stale_plan.digest().unwrap();
    let mut kernel = magic_kernel();
    assert!(matches!(
        kernel.apply(&stale_plan, P1, &response),
        Err(mtgml_rules::KernelExecutionError::UnsupportedStagePath)
    ));
    assert_eq!(stale_plan, before);
    assert_eq!(stale_plan.digest().unwrap(), digest);

    let mut wrong_continuation = state.clone();
    wrong_continuation
        .execution
        .pending_decision
        .as_mut()
        .unwrap()
        .request
        .continuation_id = Some(mtgml_model::ContinuationId(99));
    let before = wrong_continuation.clone();
    let mut kernel = magic_kernel();
    assert!(matches!(
        kernel.apply(&wrong_continuation, P1, &response),
        Err(mtgml_rules::KernelExecutionError::BeforeState(_))
    ));
    assert_eq!(wrong_continuation, before);

    let mut wrong_current_actor = state.clone();
    wrong_current_actor
        .execution
        .continuations
        .values_mut()
        .next()
        .unwrap()
        .actor = P2;
    let before = wrong_current_actor.clone();
    let mut kernel = magic_kernel();
    assert!(matches!(
        kernel.apply(&wrong_current_actor, P1, &response),
        Err(mtgml_rules::KernelExecutionError::BeforeState(_))
    ));
    assert_eq!(wrong_current_actor, before);
}

#[test]
fn task7_intermediate_stage_identity_exhaustion_rejects_without_mutation() {
    use mtgml_model::RuleEventId;

    let response = current_order_response(
        &state_with_pending_two_owner_order(),
        vec![CandidateIdV1(1), CandidateIdV1(0)],
    );
    let mut decision = state_with_pending_two_owner_order();
    decision.allocators.next_decision_id = DecisionId(u64::MAX);
    mtgml_state::validate_engine_state(&decision).unwrap();
    assert_order_exhaustion(&decision, &response, |error| {
        matches!(
            error,
            mtgml_rules::KernelExecutionError::Exhaustion("decision")
        )
    });

    let mut player_decision = state_with_pending_two_owner_order();
    player_decision
        .perspective_identities
        .players
        .get_mut(&P2)
        .unwrap()
        .next_player_decision_id = PlayerDecisionIdV1(u64::MAX);
    mtgml_state::validate_engine_state(&player_decision).unwrap();
    assert_order_exhaustion(&player_decision, &response, |error| {
        matches!(
            error,
            mtgml_rules::KernelExecutionError::Exhaustion("player_decision")
        )
    });

    let mut event = state_with_pending_two_owner_order();
    event.allocators.next_rule_event_id = RuleEventId(u64::MAX - 2);
    mtgml_state::validate_engine_state(&event).unwrap();
    assert_order_exhaustion(&event, &response, |error| {
        matches!(
            error,
            mtgml_rules::KernelExecutionError::RuleEventIdOverflow
        )
    });

    let mut revision = state_with_pending_two_owner_order();
    let continuation = revision
        .execution
        .continuations
        .values_mut()
        .next()
        .unwrap();
    continuation.created_at_revision = StateRevision(u64::MAX);
    let ContinuationPayloadV2::MagicSbaGraveyardOrderV1 {
        round_start_revision,
        ..
    } = &mut continuation.payload
    else {
        unreachable!()
    };
    *round_start_revision = StateRevision(u64::MAX - 1);
    revision.revision = StateRevision(u64::MAX);
    revision
        .execution
        .pending_decision
        .as_mut()
        .unwrap()
        .request
        .state_revision = StateRevision(u64::MAX);
    mtgml_state::validate_engine_state(&revision).unwrap();
    let revision_response =
        current_order_response(&revision, vec![CandidateIdV1(1), CandidateIdV1(0)]);
    assert_order_exhaustion(&revision, &revision_response, |error| {
        matches!(error, mtgml_rules::KernelExecutionError::RevisionOverflow)
    });
}

fn assert_order_exhaustion(
    state: &EngineState,
    response: &DecisionResponseV2,
    expected: impl FnOnce(&mtgml_rules::KernelExecutionError) -> bool,
) {
    let before = state.clone();
    let digest = state.digest().unwrap();
    let mut kernel = magic_kernel();
    let error = kernel
        .apply(state, P1, response)
        .expect_err("exhausted stage identity must fail before commit");
    assert!(expected(&error), "unexpected exhaustion surface: {error:?}");
    assert_eq!(state, &before);
    assert_eq!(state.digest().unwrap(), digest);
}

#[test]
fn task7_initial_stage_identity_exhaustion_fails_before_decision_creation() {
    use mtgml_model::RuleEventId;

    let state_with_order = || {
        state_with(
            &[
                CreatureSpec {
                    owner: P1,
                    toughness: 0,
                    marked_damage: 0,
                },
                CreatureSpec {
                    owner: P1,
                    toughness: 0,
                    marked_damage: 0,
                },
            ],
            [40, 40],
        )
    };
    let assert_failure =
        |state: &EngineState, expected: fn(&mtgml_rules::KernelExecutionError) -> bool| {
            let before = state.clone();
            let digest = state.digest().unwrap();
            let mut kernel = magic_kernel();
            let error = kernel
                .advance_forced_progress(state)
                .expect_err("initial Order stage identity exhaustion must fail closed");
            assert!(expected(&error), "unexpected exhaustion surface: {error:?}");
            assert_eq!(state, &before);
            assert_eq!(state.digest().unwrap(), digest);
        };

    let mut decision = state_with_order();
    decision.allocators.next_decision_id = DecisionId(u64::MAX);
    mtgml_state::validate_engine_state(&decision).unwrap();
    assert_failure(&decision, |error| {
        matches!(
            error,
            mtgml_rules::KernelExecutionError::Exhaustion("decision")
        )
    });

    let mut continuation = state_with_order();
    continuation.allocators.next_continuation_id = mtgml_model::ContinuationId(u64::MAX);
    mtgml_state::validate_engine_state(&continuation).unwrap();
    assert_failure(&continuation, |error| {
        matches!(
            error,
            mtgml_rules::KernelExecutionError::Exhaustion("continuation")
        )
    });

    let mut player_decision = state_with_order();
    player_decision
        .perspective_identities
        .players
        .get_mut(&P1)
        .unwrap()
        .next_player_decision_id = PlayerDecisionIdV1(u64::MAX);
    mtgml_state::validate_engine_state(&player_decision).unwrap();
    assert_failure(&player_decision, |error| {
        matches!(
            error,
            mtgml_rules::KernelExecutionError::Exhaustion("player_decision")
        )
    });

    let mut event = state_with_order();
    event.allocators.next_rule_event_id = RuleEventId(u64::MAX);
    mtgml_state::validate_engine_state(&event).unwrap();
    assert_failure(&event, |error| {
        matches!(
            error,
            mtgml_rules::KernelExecutionError::RuleEventIdOverflow
        )
    });

    let mut revision = state_with_order();
    revision.revision = StateRevision(u64::MAX);
    mtgml_state::validate_engine_state(&revision).unwrap();
    assert_failure(&revision, |error| {
        matches!(error, mtgml_rules::KernelExecutionError::RevisionOverflow)
    });
}

#[test]
fn task7_missing_actor_opaque_identity_fails_before_stage_creation() {
    let mut state = state_with(
        &[
            CreatureSpec {
                owner: P1,
                toughness: 0,
                marked_damage: 0,
            },
            CreatureSpec {
                owner: P1,
                toughness: 0,
                marked_damage: 0,
            },
        ],
        [40, 40],
    );
    let opaque = state
        .perspective_identities
        .players
        .get_mut(&P1)
        .unwrap()
        .object_to_opaque
        .remove(&GameObjectId(1))
        .unwrap();
    state
        .perspective_identities
        .players
        .get_mut(&P1)
        .unwrap()
        .opaque_to_object
        .remove(&opaque);
    state
        .knowledge
        .players
        .get_mut(&P1)
        .unwrap()
        .active
        .remove(&opaque);
    validate_engine_state(&state).unwrap();
    let before = state.clone();
    let mut kernel = magic_kernel();
    assert!(matches!(
        kernel.advance_forced_progress(&state),
        Err(mtgml_rules::KernelExecutionError::UnsupportedStagePath)
    ));
    assert_eq!(state, before);
}
