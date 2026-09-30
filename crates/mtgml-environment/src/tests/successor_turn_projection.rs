//! Player projection of native turn progression products: turn-structure
//! changes and draws.

use mtgml_card_ir::ExecutableProfileAdmissionV1;
use mtgml_decision::{
    DecisionAnswerV2, DecisionPurposeV4, DecisionResponseV3, EngineCandidateBindingV4,
    DECISION_RESPONSE_V3_SCHEMA,
};
use mtgml_model::{
    CardDefinitionId, EpisodeStatus, GameObjectId, PhysicalCardId, PlayerId, ZoneKind,
};
use mtgml_observation::{MagicSharedExecutionObservationV1, ObservedEventKindV4, PlayerStepV4};
use mtgml_state::{
    BeginningStep, EndingStep, EngineState, GameObject, TurnPosition, VisibilityPartition,
    ZoneLocation, ZonePosition,
};
use std::collections::BTreeMap;

const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);

fn add_library_cards(state: &mut EngineState, owner: PlayerId, count: u64) {
    let definition = state.zones.objects[&GameObjectId(2)].card_definition;
    for _ in 0..count {
        let id = state.allocators.next_object_id;
        state.allocators.next_object_id = GameObjectId(id.0 + 1);
        let base = ZoneLocation {
            zone: ZoneKind::Library,
            player: Some(owner),
            position: ZonePosition::Top { offset: 0 },
            visibility: VisibilityPartition::FaceDown,
            partition: None,
        };
        let order = state.zones.ordered_zones.entry(base.key()).or_default();
        let location = ZoneLocation {
            position: ZonePosition::Top {
                offset: order.len() as u32,
            },
            ..base
        };
        order.push(id);
        state.zones.objects.insert(
            id,
            GameObject {
                id,
                physical_card: Some(PhysicalCardId(1_000 + id.0)),
                card_definition: definition,
                owner,
                controller: owner,
                tapped: false,
                face_down: false,
            },
        );
        state.zones.locations.insert(id, location);
        state.card_rules.faces.faces.insert(id, 0);
    }
}

/// P1 active with priority in precombat main; P2's library top is the
/// synthetic card 2 (made an ordinary card), followed by three more cards in
/// each library. `second_p2_card` overrides the definition of P2's second
/// library card, which P2 draws on turn 4.
fn game(second_p2_card: Option<CardDefinitionId>) -> (ExecutableProfileAdmissionV1, EngineState) {
    let admission = crate::basic_land_runtime_v8::fixtures::game_admission();
    let mut state = crate::basic_land_runtime_v8::fixtures::state_with_two_lands();
    let top = state.zones.objects.get_mut(&GameObjectId(2)).unwrap();
    top.face_down = false;
    add_library_cards(&mut state, P1, 3);
    add_library_cards(&mut state, P2, 3);
    if let Some(definition) = second_p2_card {
        let library = ZoneLocation {
            zone: ZoneKind::Library,
            player: Some(P2),
            position: ZonePosition::Top { offset: 0 },
            visibility: VisibilityPartition::FaceDown,
            partition: None,
        };
        let second = state.zones.ordered_zones[&library.key()][1];
        state
            .zones
            .objects
            .get_mut(&second)
            .unwrap()
            .card_definition = definition;
    }
    mtgml_rules::install_basic_land_request_v4(&admission, &mut state, P1, &EpisodeStatus::Running)
        .unwrap();
    (admission, state)
}

fn pass(
    admission: &ExecutableProfileAdmissionV1,
    state: &EngineState,
) -> mtgml_rules::BasicLandTransitionProductV4 {
    let request = state.execution.pending_decision.as_ref().unwrap();
    let answer = if request.purpose == DecisionPurposeV4::AttackerDeclaration {
        DecisionAnswerV2::SelectMany {
            candidate_ids: Vec::new(),
        }
    } else {
        DecisionAnswerV2::SelectOne {
            candidate_id: request
                .candidates
                .iter()
                .find(|candidate| {
                    matches!(
                        candidate.trusted_binding,
                        EngineCandidateBindingV4::PassPriority
                    )
                })
                .unwrap()
                .candidate_id,
        }
    };
    let response = DecisionResponseV3 {
        schema_version: DECISION_RESPONSE_V3_SCHEMA.to_owned(),
        player_decision_id: request.player_decision_id,
        view_sequence: request.view_sequence,
        answer,
    };
    mtgml_rules::execute_magic_response_v4(
        admission,
        state,
        request.actor,
        &response,
        &EpisodeStatus::Running,
    )
    .unwrap()
}

/// Passes until the next response would enter `position` on `turn`; returns
/// that state and the product of the response that enters it.
fn product_entering(
    admission: &ExecutableProfileAdmissionV1,
    mut state: EngineState,
    position: TurnPosition,
    turn: u64,
) -> (EngineState, mtgml_rules::BasicLandTransitionProductV4) {
    for _ in 0..200 {
        let product = pass(admission, &state);
        let core = &product.next_state.core;
        if core.position == position && core.turn_number == turn {
            return (state, product);
        }
        state = product.next_state;
    }
    panic!("position not reached");
}

fn project(
    admission: &ExecutableProfileAdmissionV1,
    before: &EngineState,
    product: &mtgml_rules::BasicLandTransitionProductV4,
) -> BTreeMap<PlayerId, PlayerStepV4> {
    let running = EpisodeStatus::Running;
    crate::successor_projection::project_successor_player_steps_v4(
        crate::successor_projection::SuccessorTransitionV4Projection {
            before,
            after: &product.next_state,
            before_status: &running,
            events: &product.events,
            delta: Some(&product.delta),
            accepted: true,
            status: &product.status,
            next_request: product.next_decision.as_ref(),
            actor: before.execution.pending_decision.as_ref().unwrap().actor,
            rejected_code: mtgml_observation::PlayerSubmissionCodeV1::InvalidAnswer,
        },
        crate::successor_projection::SuccessorProjectionAuthority {
            execution_identity: admission.execution_identity(),
            semantic_manifest: admission.semantic_contract_manifest(),
            rules_manifest: admission.rules_contract_manifest(),
            catalog: admission.verified_catalog(),
            basic_land_admission: Some(admission),
        },
    )
    .unwrap()
}

fn observation(step: &PlayerStepV4) -> MagicSharedExecutionObservationV1 {
    let payload = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        &step.information_state.current_observation.payload_base64,
    )
    .unwrap();
    mtgml_wire::decode_canonical(&payload).unwrap()
}

#[test]
fn turn_structure_events_produce_no_observed_envelope() {
    let (admission, state) = game(None);
    let (before, product) = product_entering(
        &admission,
        state,
        TurnPosition::Beginning {
            step: BeginningStep::Upkeep,
        },
        2,
    );
    assert_eq!(
        before.core.position,
        TurnPosition::Ending {
            step: EndingStep::EndStep
        }
    );
    let steps = project(&admission, &before, &product);

    for player in [P1, P2] {
        let step = &steps[&player];
        assert!(step.observed_events.is_empty(), "{player:?}");
        let observation = observation(step);
        assert_eq!(observation.active_player, P2);
        assert_eq!(observation.turn_number, "2");
    }
    assert!(steps[&P2].next_decision.is_some());
    assert!(steps[&P1].next_decision.is_none());
}

#[test]
fn drawing_player_sees_the_drawn_card() {
    let (admission, state) = game(None);
    let (before, product) = product_entering(
        &admission,
        state,
        TurnPosition::Beginning {
            step: BeginningStep::Draw,
        },
        2,
    );
    let steps = project(&admission, &before, &product);

    assert!(steps[&P2].observed_events.iter().any(|envelope| matches!(
        envelope.event,
        ObservedEventKindV4::ObjectMoved {
            new_object: Some(_),
            from: ZoneKind::Library,
            to: ZoneKind::Hand,
            ..
        }
    )));
}

#[test]
fn opponent_step_after_draw_is_identical_for_different_top_cards() {
    // Mountain and Plains, taken from P1's hand in the fixture.
    let fixture = crate::basic_land_runtime_v8::fixtures::state_with_two_lands();
    let definitions: std::collections::BTreeSet<CardDefinitionId> = fixture
        .zones
        .objects
        .values()
        .filter(|object| fixture.zones.locations[&object.id].zone == ZoneKind::Hand)
        .map(|object| object.card_definition)
        .collect();
    assert_eq!(definitions.len(), 2);
    let mut opponent_steps = Vec::new();
    for top in definitions {
        let (admission, state) = game(Some(top));
        let (before, product) = product_entering(
            &admission,
            state,
            TurnPosition::Beginning {
                step: BeginningStep::Draw,
            },
            4,
        );
        opponent_steps.push(project(&admission, &before, &product).remove(&P1).unwrap());
    }
    assert_eq!(opponent_steps[0], opponent_steps[1]);
}

#[test]
fn attacker_declaration_request_projects_for_both_players() {
    let (admission, mut state) = game(None);
    let (before, product) = loop {
        let product = pass(&admission, &state);
        if product
            .next_decision
            .as_ref()
            .is_some_and(|request| request.purpose == DecisionPurposeV4::AttackerDeclaration)
        {
            break (state, product);
        }
        state = product.next_state;
    };
    let steps = project(&admission, &before, &product);

    assert!(steps[&P1].next_decision.is_some());
    assert!(steps[&P2].next_decision.is_none());
}

/// Adds cards with the given definitions to `owner`'s hand; only the owner
/// tracks them, with the owner's knowledge record.
fn add_hand_cards(state: &mut EngineState, owner: PlayerId, definitions: &[CardDefinitionId]) {
    for definition in definitions {
        let parts = &mut *state;
        let id = parts.allocators.next_object_id;
        parts.allocators.next_object_id = GameObjectId(id.0 + 1);
        let location = ZoneLocation {
            zone: ZoneKind::Hand,
            player: Some(owner),
            position: ZonePosition::Unordered,
            visibility: VisibilityPartition::OwnerOnly,
            partition: None,
        };
        let physical_card = Some(PhysicalCardId(2_000 + id.0));
        parts.zones.objects.insert(
            id,
            GameObject {
                id,
                physical_card,
                card_definition: *definition,
                owner,
                controller: owner,
                tapped: false,
                face_down: false,
            },
        );
        parts.zones.locations.insert(id, location.clone());
        let identity = parts
            .perspective_identities
            .players
            .get_mut(&owner)
            .unwrap();
        let opaque = identity.next_opaque_object_id;
        identity.next_opaque_object_id = mtgml_model::OpaqueObjectId(opaque.0 + 1);
        identity.object_to_opaque.insert(id, opaque);
        identity.opaque_to_object.insert(opaque, id);
        parts
            .knowledge
            .players
            .get_mut(&owner)
            .unwrap()
            .active
            .insert(
                opaque,
                mtgml_state::KnowledgeRecordV2 {
                    opaque_object: opaque,
                    physical_card,
                    card_definition: Some(*definition),
                    known_location: Some(mtgml_state::KnownLocationFactV2 {
                        location,
                        provenance: mtgml_state::KnowledgeAcquisitionReason::InitialConfiguration,
                    }),
                    acquisition: mtgml_state::KnowledgeAcquisitionReason::InitialConfiguration,
                    historical_locations: Vec::new(),
                },
            );
        state.card_rules.faces.faces.insert(id, 0);
    }
}

#[test]
fn opponent_sees_discarded_card_but_not_kept_cards() {
    let fixture = crate::basic_land_runtime_v8::fixtures::state_with_two_lands();
    let mut definitions: Vec<CardDefinitionId> = fixture
        .zones
        .objects
        .values()
        .filter(|object| fixture.zones.locations[&object.id].zone == ZoneKind::Hand)
        .map(|object| object.card_definition)
        .collect();
    definitions.sort();
    definitions.dedup();
    let [mountain, plains] = definitions[..] else {
        panic!("fixture hand holds Mountain and Plains");
    };

    let mut opponent_steps = Vec::new();
    for kept in [mountain, plains] {
        let admission = crate::basic_land_runtime_v8::fixtures::game_admission();
        let mut state = crate::basic_land_runtime_v8::fixtures::state_with_two_lands();
        state
            .zones
            .objects
            .get_mut(&GameObjectId(2))
            .unwrap()
            .face_down = false;
        add_library_cards(&mut state, P1, 3);
        add_library_cards(&mut state, P2, 3);
        // Six more cards: the first is kept and differs between the two
        // games; the last one (highest opaque id) is discarded.
        add_hand_cards(
            &mut state,
            P1,
            &[kept, mountain, mountain, mountain, mountain, mountain],
        );
        mtgml_rules::install_basic_land_request_v4(
            &admission,
            &mut state,
            P1,
            &EpisodeStatus::Running,
        )
        .unwrap();
        let (_, entering_cleanup) = product_entering(
            &admission,
            state,
            TurnPosition::Ending {
                step: EndingStep::Cleanup,
            },
            1,
        );
        let before = entering_cleanup.next_state;
        let request = before.execution.pending_decision.as_ref().unwrap();
        assert_eq!(request.purpose, DecisionPurposeV4::HandSizeDiscard);
        let response = DecisionResponseV3 {
            schema_version: DECISION_RESPONSE_V3_SCHEMA.to_owned(),
            player_decision_id: request.player_decision_id,
            view_sequence: request.view_sequence,
            answer: DecisionAnswerV2::SelectMany {
                candidate_ids: vec![request.candidates.last().unwrap().candidate_id],
            },
        };
        let product = mtgml_rules::execute_magic_response_v4(
            &admission,
            &before,
            P1,
            &response,
            &EpisodeStatus::Running,
        )
        .unwrap();
        let mut steps = project(&admission, &before, &product);
        let opponent = steps.remove(&P2).unwrap();
        assert!(opponent.observed_events.iter().any(|envelope| matches!(
            envelope.event,
            ObservedEventKindV4::ObjectMoved {
                old_object: None,
                new_object: Some(_),
                from: ZoneKind::Hand,
                to: ZoneKind::Graveyard,
                ..
            }
        )));
        opponent_steps.push(opponent);
    }
    assert_eq!(opponent_steps[0], opponent_steps[1]);
}

#[test]
fn mana_emptying_is_observed_by_every_player() {
    let (admission, state) = game(None);
    let request = state.execution.pending_decision.as_ref().unwrap();
    let tap = request
        .candidates
        .iter()
        .find(|candidate| {
            matches!(
                candidate.trusted_binding,
                EngineCandidateBindingV4::ActivateAbility { .. }
            )
        })
        .unwrap()
        .candidate_id;
    let response = DecisionResponseV3 {
        schema_version: DECISION_RESPONSE_V3_SCHEMA.to_owned(),
        player_decision_id: request.player_decision_id,
        view_sequence: request.view_sequence,
        answer: DecisionAnswerV2::SelectOne { candidate_id: tap },
    };
    let tapped = mtgml_rules::execute_magic_response_v4(
        &admission,
        &state,
        P1,
        &response,
        &EpisodeStatus::Running,
    )
    .unwrap()
    .next_state;
    let passed = pass(&admission, &tapped).next_state;
    let product = pass(&admission, &passed);
    let steps = project(&admission, &passed, &product);

    for player in [P1, P2] {
        assert!(
            steps[&player]
                .observed_events
                .iter()
                .any(|envelope| matches!(
                    envelope.event,
                    ObservedEventKindV4::ManaPoolChanged { player: P1, .. }
                )),
            "{player:?} observes P1's pool emptying"
        );
    }
}
