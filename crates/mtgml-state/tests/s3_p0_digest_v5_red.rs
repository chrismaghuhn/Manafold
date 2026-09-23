//! S3.P0 Task 1 RED contract for the typed V5 state digest.
//!
//! The continuation fixture's semantic payload includes the immutable
//! round-start revision, complete selected SBA action set and causes, APNAP
//! owners, next owner index, and every completed owner's exact permutation.
//! Its canonical payload tag is `magic_sba_graveyard_order_v1`.

use mtgml_decision::{
    AuthoritativeCandidateV2, CandidateIntent, DecisionDomainV2, DecisionVisibility,
    EngineCandidateBinding,
};
use mtgml_model::{
    CardDefinitionId, ContinuationId, DecisionId, FullStateDigestV5, GameObjectId, OpaqueObjectId,
    PhysicalCardId, PlayerDecisionIdV1, PlayerId, StateRevision, ZoneKind,
};
use mtgml_random::RootSeed256;
use mtgml_state::{
    construct_synthetic_engine_state, ContinuationPayloadV2, ContinuationRecordV2, EngineState,
    GameObject, KnowledgeAcquisitionReason, KnowledgeRecordV2, KnownLocationFactV2,
    PendingDecisionRecordV2, SyntheticResetInputs, SyntheticV4Setup, VisibilityPartition,
    ZoneLocation, ZonePosition, FULL_STATE_DIGEST_INPUT_SCHEMA_V5,
};

fn synthetic_state() -> mtgml_state::EngineState {
    construct_synthetic_engine_state(SyntheticResetInputs {
        players: [PlayerId(1), PlayerId(2)],
        root_seed: RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap(),
        setup: SyntheticV4Setup::m2_compatibility(),
    })
    .unwrap()
}

fn public_location() -> ZoneLocation {
    ZoneLocation {
        zone: ZoneKind::Battlefield,
        player: None,
        position: ZonePosition::Unordered,
        visibility: VisibilityPartition::Public,
        partition: None,
    }
}

fn add_public_object_identity(state: &mut EngineState, player: PlayerId, object: GameObjectId) {
    let opaque = match (player, object) {
        (PlayerId(1), GameObjectId(2)) => OpaqueObjectId(2),
        (PlayerId(1), GameObjectId(3)) => OpaqueObjectId(3),
        (PlayerId(1), GameObjectId(4)) => OpaqueObjectId(4),
        (PlayerId(2), GameObjectId(3)) => OpaqueObjectId(3),
        (PlayerId(2), GameObjectId(4)) => OpaqueObjectId(4),
        _ => panic!("fixture identity pair is not declared"),
    };
    let identity = state
        .perspective_identities
        .players
        .get_mut(&player)
        .unwrap();
    identity.opaque_to_object.insert(opaque, object);
    identity.object_to_opaque.insert(object, opaque);
    identity.next_opaque_object_id = OpaqueObjectId(5);

    let live = state.zones.objects.get(&object).unwrap();
    state
        .knowledge
        .players
        .get_mut(&player)
        .unwrap()
        .active
        .insert(
            opaque,
            KnowledgeRecordV2 {
                opaque_object: opaque,
                physical_card: live.physical_card,
                card_definition: Some(live.card_definition),
                known_location: Some(KnownLocationFactV2 {
                    location: state.zones.locations[&object].clone(),
                    provenance: KnowledgeAcquisitionReason::InitialConfiguration,
                }),
                historical_locations: Vec::new(),
                acquisition: KnowledgeAcquisitionReason::InitialConfiguration,
            },
        );
}

fn magic_order_state(completed_top_to_bottom: [GameObjectId; 2]) -> EngineState {
    let mut state = synthetic_state();
    let public = public_location();

    // The two existing reset objects plus these two inert objects give each
    // APNAP owner a two-member public Order candidate set. This is structural
    // continuation fixture data only; S3.A later proves rule applicability.
    state.zones.objects.get_mut(&GameObjectId(2)).unwrap().owner = PlayerId(2);
    state
        .zones
        .objects
        .get_mut(&GameObjectId(2))
        .unwrap()
        .controller = PlayerId(2);
    state
        .zones
        .objects
        .get_mut(&GameObjectId(2))
        .unwrap()
        .face_down = false;
    state
        .zones
        .locations
        .insert(GameObjectId(2), public.clone());
    for members in state.zones.ordered_zones.values_mut() {
        members.retain(|object| *object != GameObjectId(2));
    }
    state
        .zones
        .ordered_zones
        .retain(|_, members| !members.is_empty());

    for (id, owner) in [
        (GameObjectId(3), PlayerId(1)),
        (GameObjectId(4), PlayerId(2)),
    ] {
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
        state.zones.locations.insert(id, public.clone());
    }
    state.allocators.next_object_id = GameObjectId(5);

    // Publicly visible incarnations have a perspective-local opaque identity
    // and matching initial-configuration knowledge for each player.
    for player in [PlayerId(1), PlayerId(2)] {
        for object in [GameObjectId(2), GameObjectId(3), GameObjectId(4)] {
            if player == PlayerId(2) && object == GameObjectId(2) {
                let fact = state
                    .knowledge
                    .players
                    .get_mut(&player)
                    .unwrap()
                    .active
                    .get_mut(&OpaqueObjectId(2))
                    .unwrap();
                fact.known_location = Some(KnownLocationFactV2 {
                    location: public.clone(),
                    provenance: KnowledgeAcquisitionReason::InitialConfiguration,
                });
            } else {
                add_public_object_identity(&mut state, player, object);
            }
        }
    }

    let payload_fixture = serde_json::json!({
        "kind": "magic_sba_graveyard_order_v1",
        "round_start_revision": "0",
        "selected_sba_actions": [
            {"object": "1", "causes": ["lethal_damage"]},
            {"object": "2", "causes": ["lethal_damage"]},
            {"object": "3", "causes": ["zero_toughness"]},
            {"object": "4", "causes": ["zero_toughness"]}
        ],
        "apnap_owners": ["1", "2"],
        "next_owner_index": 1,
        "completed_owner_orders": [
            {"owner": "1", "top_to_bottom": [
                completed_top_to_bottom[0].0.to_string(),
                completed_top_to_bottom[1].0.to_string()
            ]}
        ]
    });
    let payload: ContinuationPayloadV2 = serde_json::from_value(payload_fixture).unwrap();
    let continuation = ContinuationRecordV2 {
        id: ContinuationId(1),
        actor: PlayerId(2),
        created_at_revision: StateRevision(0),
        stage_index: payload.stage_index(),
        payload,
    };
    state.allocators.next_continuation_id = ContinuationId(2);
    state
        .execution
        .continuations
        .insert(continuation.id, continuation);

    state.allocators.next_decision_id = DecisionId(2);
    state
        .perspective_identities
        .players
        .get_mut(&PlayerId(2))
        .unwrap()
        .next_player_decision_id = PlayerDecisionIdV1(2);
    state.execution.pending_decision = Some(PendingDecisionRecordV2 {
        request: mtgml_decision::AuthoritativeDecisionRequestV2 {
            decision_id: DecisionId(1),
            player_decision_id: PlayerDecisionIdV1(1),
            state_revision: StateRevision(0),
            actor: PlayerId(2),
            visibility: DecisionVisibility::ActingPlayerOnly,
            decision: DecisionDomainV2::Order {
                minimum: 2,
                maximum: 2,
            },
            candidates: [GameObjectId(2), GameObjectId(4)]
                .into_iter()
                .enumerate()
                .map(|(index, object)| AuthoritativeCandidateV2 {
                    candidate_id: mtgml_model::CandidateIdV1(index as u32),
                    visible_intent: CandidateIntent::SelectObject {
                        object: state.perspective_identities.players[&PlayerId(2)].object_to_opaque
                            [&object],
                    },
                    trusted_binding: EngineCandidateBinding::SelectObject { object },
                })
                .collect(),
            continuation_id: Some(ContinuationId(1)),
        },
    });
    state
}

fn canonical_texts(value: &mtgml_persistence::cbor::Value, output: &mut Vec<String>) {
    use mtgml_persistence::cbor::Value;
    match value {
        Value::Text(text) => output.push(text.clone()),
        Value::Array(items) => {
            for item in items {
                canonical_texts(item, output);
            }
        }
        _ => {}
    }
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(DIGITS[usize::from(byte >> 4)] as char);
        encoded.push(DIGITS[usize::from(byte & 0x0f)] as char);
    }
    encoded
}

#[test]
fn engine_state_digest_uses_the_typed_v5_identity_and_schema() {
    let digest: FullStateDigestV5 = synthetic_state().digest().unwrap();
    assert_eq!(
        digest.as_digest_reference().semantic_domain,
        FullStateDigestV5::DOMAIN
    );
    assert_eq!(
        FULL_STATE_DIGEST_INPUT_SCHEMA_V5,
        "full-state-digest-input.v5"
    );
}

#[test]
fn magic_continuation_is_valid_v5_state_and_changes_digest_when_its_order_changes() {
    let state_a = magic_order_state([GameObjectId(1), GameObjectId(3)]);
    let state_b = magic_order_state([GameObjectId(3), GameObjectId(1)]);
    mtgml_state::validate_engine_state(&state_a).unwrap();
    mtgml_state::validate_engine_state(&state_b).unwrap();
    assert!(mtgml_state::calculate_full_state_digest_v4_historical(&state_a).is_err());

    let digest_a: FullStateDigestV5 = state_a.digest().unwrap();
    let digest_b: FullStateDigestV5 = state_b.digest().unwrap();
    assert_eq!(
        digest_a.to_string(),
        "718d1d675154da69c638f19e33b24c6244ba2892c982bc5c2e9c2d22f98eb344"
    );
    assert_ne!(digest_a, digest_b);

    let canonical_bytes = state_a.canonical_digest_bytes().unwrap();
    assert_eq!(
        hex(&canonical_bytes),
        include_str!("fixtures/magic-sba-graveyard-order-v5-input.hex").trim()
    );
    let canonical = mtgml_persistence::cbor::decode_canonical(&canonical_bytes).unwrap();
    let mut texts = Vec::new();
    canonical_texts(&canonical, &mut texts);
    assert!(
        texts
            .iter()
            .any(|text| text == "magic_sba_graveyard_order_v1"),
        "FullStateDigestV5 canonical state input must bind the Magic continuation tag"
    );
}
