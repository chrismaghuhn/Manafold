// Ownership fragment: canonical digest known-answer/mutation evidence. Included lexically by tests.rs so
// every identity remains tests::<name>.

fn json_to_cbor(value: &serde_json::Value) -> Value {
    match value {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(value) => Value::Bool(*value),
        serde_json::Value::Number(value) => {
            if let Some(value) = value.as_u64() {
                Value::Unsigned(value)
            } else {
                Value::Signed(value.as_i64().expect("fixture integer fits i64"))
            }
        }
        serde_json::Value::String(value) => Value::Text(value.clone()),
        serde_json::Value::Array(values) => Value::Array(values.iter().map(json_to_cbor).collect()),
        serde_json::Value::Object(_) => Value::Text("object-not-permitted".to_owned()),
    }
}

/// Decodes a V6 input payload the way FullStateDigestV7 validates its V6
/// components: canonical CBOR, typed decode, and byte-identical re-encoding.
fn decode_v6_payload(payload: &[u8]) -> Result<crate::FullStateDigestInputV6, ()> {
    let value = mtgml_persistence::cbor::decode_canonical(payload).map_err(|_| ())?;
    let input = crate::FullStateDigestInputV6::from_canonical_value(&value).map_err(|_| ())?;
    if input.canonical_payload().map_err(|_| ())? != payload {
        return Err(());
    }
    Ok(input)
}

fn phase2_v6_fixture() -> (serde_json::Value, crate::FullStateDigestInputV6) {
    let vector: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../persistence/golden/full-state-digest-v6-kat.v1.json"
    ))
    .unwrap();
    let family = json_to_cbor(&vector["card_rules_authoritative_state"]);
    let card_state = crate::CardRulesAuthoritativeStateV1::from_value(&family).unwrap();
    let v5 = decode_hex(
        include_str!("../../tests/fixtures/magic-sba-graveyard-order-v5-input.hex").trim(),
    );
    let input = crate::FullStateDigestInputV6::from_phase2_v5_payload(&v5, card_state).unwrap();
    (vector, input)
}


#[test]
fn full_state_digest_v6_rejects_predecessor_and_noncanonical_fixtures() {
    let (_, input) = phase2_v6_fixture();
    let payload = input.canonical_payload().unwrap();
    let predecessor = decode_hex(
        include_str!("../../tests/fixtures/magic-sba-graveyard-order-v5-input.hex").trim(),
    );
    assert!(decode_v6_payload(&predecessor).is_err());

    for path in [
        "../../persistence/negative/m4-v6-indefinite-array.cbor",
        "../../persistence/negative/m4-v6-map-where-array-required.cbor",
        "../../persistence/negative/m4-v6-noncanonical-integer.cbor",
        "../../persistence/negative/m4-v6-trailing-value.cbor",
    ] {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
        let invalid = std::fs::read(path).unwrap();
        assert!(decode_v6_payload(&invalid).is_err());
    }
    decode_v6_payload(&payload).unwrap();
}

#[test]
fn full_state_digest_v6_rejects_unknown_legacy_component_variants() {
    let (_, input) = phase2_v6_fixture();
    let baseline = input.canonical_payload().unwrap();
    type Mutation = (&'static str, Box<dyn Fn(&mut Value)>);
    let mutations: [Mutation; 5] = [
        (
            "unknown turn position",
            Box::new(|value| {
                let Value::Array(top) = value else { unreachable!() };
                let Value::Array(core) = &mut top[3] else { unreachable!() };
                let Value::Array(position) = &mut core[3] else { unreachable!() };
                position[0] = Value::Text("totally_unknown_turn_state".into());
            }),
        ),
        (
            "unknown zone kind",
            Box::new(|value| {
                let Value::Array(top) = value else { unreachable!() };
                let Value::Array(zones) = &mut top[4] else { unreachable!() };
                let Value::Array(locations) = &mut zones[1] else { unreachable!() };
                let Value::Array(first) = &mut locations[0] else { unreachable!() };
                let Value::Array(location) = &mut first[1] else { unreachable!() };
                location[0] = Value::Text("unknown_zone".into());
            }),
        ),
        (
            "malformed random stream key",
            Box::new(|value| {
                let Value::Array(top) = value else { unreachable!() };
                let Value::Array(random) = &mut top[7] else { unreachable!() };
                random[2] = Value::Array(vec![Value::Array(vec![
                    Value::Bytes(vec![0xff]),
                    Value::Unsigned(0),
                ])]);
            }),
        ),
        (
            "unknown knowledge provenance",
            Box::new(|value| {
                let Value::Array(top) = value else { unreachable!() };
                let Value::Array(players) = &mut top[8] else { unreachable!() };
                let Value::Array(first_player) = &mut players[0] else { unreachable!() };
                let Value::Array(active) = &mut first_player[2] else { unreachable!() };
                let Value::Array(first_active) = &mut active[0] else { unreachable!() };
                first_active[5] = Value::Array(vec![
                    Value::Text("unknown_provenance".into()),
                    Value::Null,
                ]);
            }),
        ),
        (
            "malformed perspective identity row",
            Box::new(|value| {
                let Value::Array(top) = value else { unreachable!() };
                let Value::Array(players) = &mut top[9] else { unreachable!() };
                let Value::Array(first_player) = &mut players[0] else { unreachable!() };
                let Value::Array(object_mappings) = &mut first_player[1] else { unreachable!() };
                let Value::Array(first_mapping) = &mut object_mappings[0] else { unreachable!() };
                first_mapping.pop();
            }),
        ),
    ];
    for (name, mutate) in mutations {
        let mut value = mtgml_persistence::cbor::decode_canonical(&baseline).unwrap();
        mutate(&mut value);
        let payload = mtgml_persistence::cbor::encode_canonical(&value).unwrap();
        assert!(
            decode_v6_payload(&payload).is_err(),
            "accepted malformed legacy component: {name}"
        );
    }
}

#[test]
fn full_state_digest_v6_rejects_unordered_commander_predecessor_maps() {
    let (_, input) = phase2_v6_fixture();
    let bytes = input.canonical_payload().unwrap();
    let base = mtgml_persistence::cbor::decode_canonical(&bytes).unwrap();
    let make_commander = |designations: Vec<Value>, cast_counts: Vec<Value>, damage: Vec<Value>| {
        Value::Array(vec![
            Value::Text("commander".into()),
            Value::Array(vec![
                Value::Array(designations),
                Value::Array(cast_counts),
                Value::Array(damage),
            ]),
        ])
    };
    let designation = |player, object| {
        Value::Array(vec![
            Value::Unsigned(player),
            Value::Array(vec![Value::Unsigned(object)]),
        ])
    };
    let cast_count = |card| Value::Array(vec![Value::Unsigned(card), Value::Unsigned(1)]);
    let damage_row = |card, player| {
        Value::Array(vec![
            Value::Unsigned(card),
            Value::Array(vec![Value::Array(vec![
                Value::Unsigned(player),
                Value::Unsigned(1),
            ])]),
        ])
    };
    let cases = [
        (
            "swapped designations",
            make_commander(
                vec![designation(2, 2), designation(1, 1)],
                vec![],
                vec![],
            ),
        ),
        (
            "duplicate designation key",
            make_commander(
                vec![designation(1, 1), designation(1, 2)],
                vec![],
                vec![],
            ),
        ),
        (
            "swapped cast-count keys",
            make_commander(
                vec![],
                vec![cast_count(20), cast_count(10)],
                vec![],
            ),
        ),
        (
            "duplicate cast-count key",
            make_commander(vec![], vec![cast_count(10), cast_count(10)], vec![]),
        ),
        (
            "swapped damage-source keys",
            make_commander(vec![], vec![], vec![damage_row(20, 1), damage_row(10, 1)]),
        ),
        (
            "duplicate damage-source key",
            make_commander(vec![], vec![], vec![damage_row(10, 1), damage_row(10, 2)]),
        ),
    ];
    let mut valid_value = base.clone();
    let Value::Array(valid_top) = &mut valid_value else {
        unreachable!();
    };
    valid_top[12] = make_commander(
        vec![designation(1, 1), designation(2, 2)],
        vec![cast_count(10), cast_count(20)],
        vec![damage_row(10, 1), damage_row(20, 2)],
    );
    assert!(crate::FullStateDigestInputV6::from_canonical_value(&valid_value).is_ok());

    for (name, format) in cases {
        let mut value = base.clone();
        let Value::Array(top) = &mut value else {
            unreachable!();
        };
        top[12] = format;
        assert!(
            crate::FullStateDigestInputV6::from_canonical_value(&value).is_err(),
            "accepted noncanonical Commander predecessor map: {name}"
        );
    }
}

#[test]
fn full_state_digest_v6_consumes_phase2_state_shape_negatives() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../schemas/negative/m4-phase2-v6-state-shapes.json"
    ))
    .unwrap();
    let cases = fixture["cases"].as_array().unwrap();
    assert!(cases.len() >= 14);
    for case in cases {
        let family = json_to_cbor(&case["family"]);
        assert!(
            crate::CardRulesAuthoritativeStateV1::from_value(&family).is_err(),
            "accepted malformed state case: {}",
            case["case"].as_str().unwrap()
        );
    }
}

#[test]
fn full_state_digest_v6_rejects_empty_counter_object_rows() {
    let (vector, _) = phase2_v6_fixture();
    let mut family = json_to_cbor(&vector["card_rules_authoritative_state"]);
    let Value::Array(fields) = &mut family else {
        unreachable!();
    };
    fields[3] = Value::Array(vec![Value::Array(vec![
        Value::Unsigned(1),
        Value::Array(Vec::new()),
    ])]);
    assert!(crate::CardRulesAuthoritativeStateV1::from_value(&family).is_err());
}

#[test]
fn full_state_digest_v6_typed_producer_rejects_empty_counter_maps() {
    let mut state = crate::CardRulesAuthoritativeStateV1::default();
    state.counters.counters.insert(
        mtgml_model::GameObjectId(1),
        std::collections::BTreeMap::new(),
    );
    assert!(state.validate().is_err());
    assert!(state.canonical_value().is_err());
    assert!(crate::canonical_state_bytes_v6_with_execution_v3(
        &synthetic_state(),
        &crate::ExecutionStateV3::default(),
        state
    )
    .is_err());
}


#[test]
fn full_state_digest_v6_typed_collection_insertion_order_is_irrelevant() {
    let (_, input) = phase2_v6_fixture();
    let baseline = input.canonical_payload().unwrap();
    let mut reordered = input;
    let state = &mut reordered.card_rules_state;
    state.mana.pools = state.mana.pools.iter().rev().map(|(k, v)| (*k, *v)).collect();
    state.turn_history.players = state
        .turn_history
        .players
        .iter()
        .rev()
        .map(|(k, v)| (*k, *v))
        .collect();
    state.turn_history.target_occurrences = state
        .turn_history
        .target_occurrences
        .iter()
        .rev()
        .copied()
        .collect();
    state.turn_history.once_ability_used = state
        .turn_history
        .once_ability_used
        .iter()
        .rev()
        .copied()
        .collect();
    state.counters.counters = state
        .counters
        .counters
        .iter()
        .rev()
        .map(|(object, counters)| {
            (
                *object,
                counters.iter().rev().map(|(kind, count)| (*kind, *count)).collect(),
            )
        })
        .collect();
    state.attachments.by_source = state
        .attachments
        .by_source
        .iter()
        .rev()
        .map(|(source, edge)| (*source, *edge))
        .collect();
    state.faces.faces = state.faces.faces.iter().rev().map(|(k, v)| (*k, *v)).collect();
    state.abilities.by_instance = state
        .abilities
        .by_instance
        .iter()
        .rev()
        .map(|(k, v)| (*k, *v))
        .collect();
    assert_eq!(baseline, reordered.canonical_payload().unwrap());
}


#[test]
fn execution_v3_persists_play_land_without_adding_current_decision_runtime() {
    let (_, input) = phase2_v6_fixture();
    let mut execution = input.execution_v3.canonical_value().clone();
    let Value::Array(execution_fields) = &mut execution else {
        panic!("validated execution V3 is an array");
    };
    execution_fields[1] = Value::Array(vec![]);
    let Value::Array(request_fields) = &mut execution_fields[0] else {
        panic!("fixture has a pending decision request");
    };
    // This fixture is repurposed as a standalone PlayLand persistence shape;
    // do not leave the base fixture's assembly continuation attached to a
    // request from an unrelated program stage.
    request_fields[7] = Value::Null;
    let Value::Array(candidates) = &mut request_fields[6] else {
        panic!("candidate list is an array");
    };
    let Value::Array(first_candidate) = &mut candidates[0] else {
        panic!("candidate is a positional array");
    };
    let Value::Array(visible) = &mut first_candidate[1] else {
        panic!("visible intent is a positional array");
    };
    visible[0] = Value::Text("play_land".to_owned());
    let Value::Array(binding) = &mut first_candidate[2] else {
        panic!("trusted binding is a positional array");
    };
    binding[0] = Value::Text("play_land".to_owned());
    let mut input = input;
    input.execution_v3 = crate::PersistedExecutionV3::from_value(execution).unwrap();
    let payload = input.canonical_payload().unwrap();
    decode_v6_payload(&payload).unwrap();

    let mut out_of_order = input.execution_v3.canonical_value().clone();
    let Value::Array(execution_fields) = &mut out_of_order else {
        unreachable!();
    };
    let Value::Array(request_fields) = &mut execution_fields[0] else {
        unreachable!();
    };
    let Value::Array(candidates) = &mut request_fields[6] else {
        unreachable!();
    };
    let Value::Array(first_candidate) = &mut candidates[0] else {
        unreachable!();
    };
    let Value::Array(visible) = &mut first_candidate[1] else {
        unreachable!();
    };
    visible[0] = Value::Text("select_object".to_owned());
    let Value::Array(binding) = &mut first_candidate[2] else {
        unreachable!();
    };
    binding[0] = Value::Text("select_object".to_owned());
    let Value::Array(second_candidate) = &mut candidates[1] else {
        unreachable!();
    };
    let Value::Array(visible) = &mut second_candidate[1] else {
        unreachable!();
    };
    visible[0] = Value::Text("play_land".to_owned());
    let Value::Array(binding) = &mut second_candidate[2] else {
        unreachable!();
    };
    binding[0] = Value::Text("play_land".to_owned());
    assert!(crate::PersistedExecutionV3::from_value(out_of_order).is_err());
}

#[test]
fn typed_successor_execution_state_encodes_the_frozen_v6_play_land_shape() {
    use mtgml_decision::{
        AuthoritativeCandidateV3, AuthoritativeDecisionRequestV3, CandidateIntentV3,
        DecisionDomainV2, DecisionVisibility, EngineCandidateBindingV3,
    };
    use mtgml_model::{
        CandidateIdV1, DecisionId, GameObjectId, OpaqueObjectId, PlayerDecisionIdV1, PlayerId,
        StateRevision,
    };

    let (_, input) = phase2_v6_fixture();
    let request = AuthoritativeDecisionRequestV3 {
        decision_id: DecisionId(5),
        player_decision_id: PlayerDecisionIdV1(6),
        state_revision: StateRevision(7),
        actor: PlayerId(1),
        visibility: DecisionVisibility::Public,
        decision: DecisionDomainV2::ChooseOne,
        candidates: vec![AuthoritativeCandidateV3 {
            candidate_id: CandidateIdV1(0),
            visible_intent: CandidateIntentV3::PlayLand {
                object: OpaqueObjectId(8),
            },
            trusted_binding: EngineCandidateBindingV3::PlayLand {
                object: GameObjectId(9),
            },
        }],
        continuation_id: None,
    };
    let successor = crate::ExecutionStateV3 {
        pending_decision: Some(request),
        ..Default::default()
    };
    let actual = crate::PersistedExecutionV3::from_successor(&successor)
        .unwrap()
        .canonical_value()
        .clone();

    let mut expected = input.execution_v3.canonical_value().clone();
    let Value::Array(fields) = &mut expected else { unreachable!() };
    fields[1] = Value::Array(vec![]);
    let Value::Array(request) = &mut fields[0] else { unreachable!() };
    request[0] = Value::Unsigned(5);
    request[1] = Value::Unsigned(6);
    request[2] = Value::Unsigned(7);
    request[3] = Value::Unsigned(1);
    request[4] = Value::Text("public".into());
    request[5] = Value::Array(vec![Value::Text("choose_one".into()), Value::Null]);
    request[6] = Value::Array(vec![Value::Array(vec![
        Value::Unsigned(0),
        Value::Array(vec![Value::Text("play_land".into()), Value::Unsigned(8)]),
        Value::Array(vec![Value::Text("play_land".into()), Value::Unsigned(9)]),
    ])]);
    request[7] = Value::Null;
    assert_eq!(actual, expected);
}

#[test]
fn full_state_digest_v6_uses_typed_v3_execution_and_rejects_a_v2_duplicate() {
    use mtgml_decision::{
        AuthoritativeCandidateV3, AuthoritativeDecisionRequestV3, CandidateIntentV3,
        DecisionDomainV2, DecisionVisibility, EngineCandidateBindingV3,
    };
    use mtgml_model::{
        CandidateIdV1, DecisionId, GameObjectId, OpaqueObjectId, PlayerDecisionIdV1, PlayerId,
        StateRevision,
    };

    let mut state = synthetic_state();
    state.execution.pending_decision = None;
    state.execution.continuations.clear();
    let execution = crate::ExecutionStateV3 {
        pending_decision: Some(AuthoritativeDecisionRequestV3 {
            decision_id: DecisionId(1),
            player_decision_id: PlayerDecisionIdV1(2),
            state_revision: state.revision,
            actor: PlayerId(1),
            visibility: DecisionVisibility::Public,
            decision: DecisionDomainV2::ChooseOne,
            candidates: vec![AuthoritativeCandidateV3 {
                candidate_id: CandidateIdV1(0),
                visible_intent: CandidateIntentV3::PlayLand {
                    object: OpaqueObjectId(4),
                },
                trusted_binding: EngineCandidateBindingV3::PlayLand {
                    object: GameObjectId(5),
                },
            }],
            continuation_id: None,
        }),
        ..Default::default()
    };
    let card_state = crate::CardRulesAuthoritativeStateV1::default();
    let payload = crate::canonical_state_bytes_v6_with_execution_v3(
        &state,
        &execution,
        card_state.clone(),
    )
    .unwrap();
    let value = mtgml_persistence::cbor::decode_canonical(&payload).unwrap();
    let Value::Array(fields) = value else { unreachable!() };
    assert_eq!(fields[6], execution.canonical_value().unwrap());
    decode_v6_payload(&payload).unwrap();

    state.execution.pending_decision = Some(crate::PendingDecisionRecordV2 {
        request: mtgml_decision::AuthoritativeDecisionRequestV2 {
            decision_id: DecisionId(1),
            player_decision_id: PlayerDecisionIdV1(2),
            state_revision: StateRevision(state.revision.0),
            actor: PlayerId(1),
            visibility: DecisionVisibility::Public,
            decision: DecisionDomainV2::ChooseOne,
            candidates: vec![],
            continuation_id: None,
        },
    });
    assert!(crate::canonical_state_bytes_v6_with_execution_v3(
        &state,
        &execution,
        crate::CardRulesAuthoritativeStateV1::default(),
    )
    .is_err());
}

#[test]
fn execution_v3_rejects_unowned_predecessor_variants_and_impossible_domains() {
    let (_, input) = phase2_v6_fixture();
    let base = input.execution_v3.canonical_value().clone();

    let mut unknown_continuation = base.clone();
    let Value::Array(fields) = &mut unknown_continuation else {
        unreachable!();
    };
    let Value::Array(continuations) = &mut fields[1] else {
        unreachable!();
    };
    let Value::Array(continuation) = &mut continuations[0] else {
        unreachable!();
    };
    let Value::Array(payload) = &mut continuation[4] else {
        unreachable!();
    };
    payload[0] = Value::Text("totally_unknown_continuation".into());
    assert!(crate::PersistedExecutionV3::from_value(unknown_continuation).is_err());

    for unsupported_index in 2..5 {
        let mut unsupported = base.clone();
        let Value::Array(fields) = &mut unsupported else {
            unreachable!();
        };
        fields[unsupported_index] = Value::Array(vec![Value::Array(vec![Value::Text(
            "unowned_payload".into(),
        )])]);
        assert!(crate::PersistedExecutionV3::from_value(unsupported).is_err());
    }

    let mut empty_choose_one = base.clone();
    let Value::Array(fields) = &mut empty_choose_one else {
        unreachable!();
    };
    let Value::Array(request) = &mut fields[0] else {
        unreachable!();
    };
    request[5] = Value::Array(vec![Value::Text("choose_one".into()), Value::Null]);
    request[6] = Value::Array(Vec::new());
    assert!(crate::PersistedExecutionV3::from_value(empty_choose_one).is_err());

    let mut impossible_many = base.clone();
    let Value::Array(fields) = &mut impossible_many else {
        unreachable!();
    };
    let Value::Array(request) = &mut fields[0] else {
        unreachable!();
    };
    request[5] = Value::Array(vec![
        Value::Text("choose_many".into()),
        Value::Array(vec![Value::Unsigned(1), Value::Unsigned(2)]),
    ]);
    request[6] = Value::Array(Vec::new());
    assert!(crate::PersistedExecutionV3::from_value(impossible_many).is_err());

    let mut impossible_order = input.execution_v3.canonical_value().clone();
    let Value::Array(fields) = &mut impossible_order else {
        unreachable!();
    };
    let Value::Array(request) = &mut fields[0] else {
        unreachable!();
    };
    request[5] = Value::Array(vec![
        Value::Text("order".into()),
        Value::Array(vec![Value::Unsigned(1), Value::Unsigned(2)]),
    ]);
    request[6] = Value::Array(Vec::new());
    assert!(crate::PersistedExecutionV3::from_value(impossible_order).is_err());

    let mut inverted_number = base.clone();
    let Value::Array(fields) = &mut inverted_number else {
        unreachable!();
    };
    let Value::Array(request) = &mut fields[0] else {
        unreachable!();
    };
    request[5] = Value::Array(vec![
        Value::Text("choose_number".into()),
        Value::Array(vec![Value::Signed(2), Value::Signed(1)]),
    ]);
    request[6] = Value::Array(Vec::new());
    assert!(crate::PersistedExecutionV3::from_value(inverted_number).is_err());

    let mut unsigned_number = base;
    let Value::Array(fields) = &mut unsigned_number else {
        unreachable!();
    };
    let Value::Array(request) = &mut fields[0] else {
        unreachable!();
    };
    request[5] = Value::Array(vec![
        Value::Text("choose_number".into()),
        Value::Array(vec![Value::Unsigned(0), Value::Unsigned(1_u64 << 63)]),
    ]);
    request[6] = Value::Array(Vec::new());
    assert!(crate::PersistedExecutionV3::from_value(unsigned_number).is_err());
}

#[test]
fn execution_v3_preserves_closed_continuation_stage_and_link_invariants() {
    let (_, input) = phase2_v6_fixture();
    let base = input.execution_v3.canonical_value().clone();

    let mut malformed_assembly = base.clone();
    let Value::Array(fields) = &mut malformed_assembly else {
        unreachable!();
    };
    let Value::Array(continuations) = &mut fields[1] else {
        unreachable!();
    };
    let Value::Array(continuation) = &mut continuations[0] else {
        unreachable!();
    };
    let Value::Array(payload) = &mut continuation[4] else {
        unreachable!();
    };
    match payload[0] {
        Value::Text(ref tag) if tag == "synthetic_m2_assembly" => {
            let Value::Array(assembly) = &mut payload[1] else {
                unreachable!();
            };
            let Value::Array(stage) = &mut assembly[0] else {
                unreachable!();
            };
            stage[0] = Value::Text("choose_members".into());
            // choose_members requires a previously selected count.
            assembly[1] = Value::Null;
        }
        Value::Text(ref tag) if tag == "magic_sba_graveyard_order_v1" => {
            let Value::Array(sba) = &mut payload[1] else {
                unreachable!();
            };
            // An ordering continuation must retain the selected SBA action set.
            sba[1] = Value::Array(Vec::new());
        }
        _ => panic!("unexpected frozen continuation payload"),
    }
    assert!(crate::PersistedExecutionV3::from_value(malformed_assembly).is_err());

    let mut unlinked = base;
    let Value::Array(fields) = &mut unlinked else {
        unreachable!();
    };
    let Value::Array(request) = &mut fields[0] else {
        unreachable!();
    };
    request[7] = Value::Null;
    assert!(crate::PersistedExecutionV3::from_value(unlinked).is_err());
}

#[test]
fn execution_v3_binds_assembly_continuation_to_its_exact_decision_stage() {
    fn request_value(
        stage: &str,
        selected_count: Option<u64>,
        selected_keys: &[u64],
        domain: &str,
        minimum: u64,
        maximum: u64,
        mode_candidates: &[u64],
    ) -> Value {
        let (_, input) = phase2_v6_fixture();
        let mut execution = input.execution_v3.canonical_value().clone();
        let Value::Array(fields) = &mut execution else {
            unreachable!();
        };
        let Value::Array(request) = &mut fields[0] else {
            unreachable!();
        };
        let request_id = 999_u64;
        let mode_candidate = |id: usize, mode: u64| {
            let visible = Value::Array(vec![
                Value::Text("select_mode".into()),
                Value::Unsigned(mode),
            ]);
            Value::Array(vec![
                Value::Unsigned(id as u64),
                visible.clone(),
                visible,
            ])
        };
        let candidates = if domain == "choose_one" {
            vec![Value::Array(vec![
                Value::Unsigned(0),
                Value::Array(vec![Value::Text("pass_priority".into()), Value::Null]),
                Value::Array(vec![Value::Text("pass_priority".into()), Value::Null]),
            ])]
        } else {
            mode_candidates
                .iter()
                .enumerate()
                .map(|(index, mode)| mode_candidate(index, *mode))
                .collect()
        };
        request[5] = if domain == "choose_one" {
            Value::Array(vec![Value::Text(domain.into()), Value::Null])
        } else {
            Value::Array(vec![
                Value::Text(domain.into()),
                Value::Array(vec![Value::Unsigned(minimum), Value::Unsigned(maximum)]),
            ])
        };
        request[6] = Value::Array(candidates);
        request[7] = Value::Unsigned(request_id);
        let stage_index = match stage {
            "choose_count" => 0,
            "choose_members" => 1,
            "order_members" => 2,
            _ => unreachable!(),
        };
        let assembly = Value::Array(vec![
            Value::Array(vec![Value::Text(stage.into()), Value::Null]),
            selected_count.map_or(Value::Null, Value::Unsigned),
            Value::Array(selected_keys.iter().copied().map(Value::Unsigned).collect()),
            Value::Array(vec![]),
        ]);
        fields[1] = Value::Array(vec![Value::Array(vec![
            Value::Unsigned(request_id),
            request[3].clone(),
            request[2].clone(),
            Value::Unsigned(stage_index),
            Value::Array(vec![Value::Text("synthetic_m2_assembly".into()), assembly]),
        ])]);
        execution
    }

    let valid_count = request_value("choose_count", None, &[], "choose_number", 0, 3, &[]);
    assert!(crate::PersistedExecutionV3::from_value(valid_count).is_ok());
    let wrong_count_domain =
        request_value("choose_count", None, &[], "choose_one", 0, 0, &[]);
    assert!(crate::PersistedExecutionV3::from_value(wrong_count_domain).is_err());
    let wrong_count_range =
        request_value("choose_count", None, &[], "choose_number", 1, 3, &[]);
    assert!(crate::PersistedExecutionV3::from_value(wrong_count_range).is_err());

    let valid_members = request_value("choose_members", Some(2), &[], "choose_many", 2, 2, &[0, 1]);
    assert!(crate::PersistedExecutionV3::from_value(valid_members).is_ok());
    let wrong_members_domain = request_value("choose_members", Some(2), &[], "order", 2, 2, &[0, 1]);
    assert!(crate::PersistedExecutionV3::from_value(wrong_members_domain).is_err());
    let wrong_members_surface =
        request_value("choose_members", Some(2), &[], "choose_many", 2, 2, &[0, 2]);
    assert!(crate::PersistedExecutionV3::from_value(wrong_members_surface).is_err());

    let valid_order = request_value("order_members", Some(2), &[2, 5], "order", 2, 2, &[2, 5]);
    assert!(crate::PersistedExecutionV3::from_value(valid_order).is_ok());
    let wrong_order_range =
        request_value("order_members", Some(2), &[2, 5], "order", 1, 2, &[2, 5]);
    assert!(crate::PersistedExecutionV3::from_value(wrong_order_range).is_err());
}



#[test]
fn m3_p0_full_state_digest_v5_mutation_matrix() {
    type Mutation = (&'static str, fn(&mut EngineState));
    let mutations: Vec<Mutation> = vec![
        ("revision_and_pending_revision", |state| {
            state.revision = StateRevision(1);
            if let Some(pending) = state.execution.pending_decision.as_mut() {
                pending.request.state_revision = StateRevision(1);
            }
        }),
        ("core_life", |state| {
            state.core.players.get_mut(&PlayerId(1)).unwrap().life = 39;
        }),
        ("core_has_lost", |state| {
            state.core.players.get_mut(&PlayerId(2)).unwrap().has_lost = true;
        }),
        ("core_active_player", |state| {
            state.core.active_player = PlayerId(2);
        }),
        ("core_priority", |state| {
            state.core.priority = PriorityState::HeldBy {
                player: PlayerId(2),
                consecutive_passes: 0,
            };
        }),
        ("core_turn_number", |state| {
            state.core.turn_number += 1;
        }),
        ("core_position", |state| {
            state.core.position = TurnPosition::Beginning {
                step: BeginningStep::Upkeep,
            };
        }),
        ("core_priority_pass_count", |state| {
            state.core.priority = PriorityState::HeldBy {
                player: PlayerId(1),
                consecutive_passes: 1,
            };
        }),
        ("combat_presence", |state| {
            state.combat = Some(CombatState {
                defending_player: PlayerId(2),
                attackers: vec![GameObjectId(1)],
                damage_step_completed: false,
                blocked_attackers: BTreeSet::new(),
                blockers: BTreeMap::from([(GameObjectId(1), None)]),
            });
        }),
        ("foundation_source_presence", |state| {
            state.foundation_sources.insert(
                GameObjectId(1),
                FoundationCreatureSource {
                    source_kind: FoundationSourceKind::Creature,
                    base_characteristics: BaseCharacteristics::Simple {
                        power: 3,
                        toughness: 3,
                    },
                    marked_damage: 0,
                    control_history: ControlHistory::BeforeTurnStart { turn_number: 1 },
                },
            );
        }),
        ("zone_object_tapped", |state| {
            state
                .zones
                .objects
                .get_mut(&GameObjectId(1))
                .unwrap()
                .tapped = true;
        }),
        ("zone_object_face_down", |state| {
            state
                .zones
                .objects
                .get_mut(&GameObjectId(1))
                .unwrap()
                .face_down = true;
        }),
        ("zone_object_controller", |state| {
            state
                .zones
                .objects
                .get_mut(&GameObjectId(1))
                .unwrap()
                .controller = PlayerId(2);
        }),
        ("zone_object_owner", |state| {
            state.zones.objects.get_mut(&GameObjectId(1)).unwrap().owner = PlayerId(2);
        }),
        ("zone_object_physical_card", |state| {
            state
                .zones
                .objects
                .get_mut(&GameObjectId(1))
                .unwrap()
                .physical_card = None;
            for knowledge in state.knowledge.players.values_mut() {
                if let Some(record) = knowledge.active.get_mut(&OpaqueObjectId(1)) {
                    record.physical_card = None;
                }
            }
        }),
        ("zone_object_card_definition", |state| {
            state
                .zones
                .objects
                .get_mut(&GameObjectId(1))
                .unwrap()
                .card_definition = CardDefinitionId(9);
            for knowledge in state.knowledge.players.values_mut() {
                if let Some(record) = knowledge.active.get_mut(&OpaqueObjectId(1)) {
                    record.card_definition = Some(CardDefinitionId(9));
                }
            }
        }),
        ("zone_location_zone", |state| {
            let graveyard = ZoneLocation {
                zone: ZoneKind::Graveyard,
                ..public_location()
            };
            state
                .zones
                .locations
                .insert(GameObjectId(1), graveyard.clone());
            for knowledge in state.knowledge.players.values_mut() {
                if let Some(record) = knowledge.active.get_mut(&OpaqueObjectId(1)) {
                    if let Some(current) = record.known_location.as_mut() {
                        current.location = graveyard.clone();
                    }
                }
            }
        }),
        ("zone_stack_records", |state| {
            state.zones.stack_records.insert(
                StackObjectId(1),
                StackRecord {
                    id: StackObjectId(1),
                    controller: PlayerId(1),
                    source_object: None,
                    source_ability: None,
                payload: None,
                },
            );
            state.zones.stack_order.push(StackObjectId(1));
            state.allocators.next_stack_object_id = StackObjectId(2);
        }),
        ("allocator_next_object_id", |state| {
            state.allocators.next_object_id = GameObjectId(4);
        }),
        ("allocator_next_ability_id", |state| {
            state.allocators.next_ability_id = AbilityInstanceId(2);
        }),
        ("allocator_next_stack_object_id", |state| {
            state.allocators.next_stack_object_id = StackObjectId(2);
        }),
        ("allocator_next_effect_id", |state| {
            state.allocators.next_effect_id = mtgml_model::EffectInstanceId(2);
        }),
        ("allocator_next_trigger_id", |state| {
            state.allocators.next_trigger_id = TriggerInstanceId(2);
        }),
        ("allocator_next_decision_id", |state| {
            state.allocators.next_decision_id = DecisionId(3);
        }),
        ("allocator_next_continuation_id", |state| {
            state.allocators.next_continuation_id = ContinuationId(2);
        }),
        ("allocator_next_rule_event_id", |state| {
            state.allocators.next_rule_event_id = mtgml_model::RuleEventId(2);
        }),
        ("pending_decision_visibility", |state| {
            let pending = state.execution.pending_decision.as_mut().unwrap();
            pending.request.visibility = mtgml_decision::DecisionVisibility::ActingPlayerOnly;
        }),
        ("pending_decision_trusted_id", |state| {
            let pending = state.execution.pending_decision.as_mut().unwrap();
            pending.request.decision_id = DecisionId(2);
            state.allocators.next_decision_id = DecisionId(3);
        }),
        ("pending_decision_actor", |state| {
            let pending = state.execution.pending_decision.as_mut().unwrap();
            pending.request.actor = PlayerId(2);
        }),
        ("pending_candidate_binding", |state| {
            let identity = state
                .perspective_identities
                .players
                .get_mut(&PlayerId(1))
                .unwrap();
            identity
                .opaque_to_object
                .insert(OpaqueObjectId(2), GameObjectId(2));
            identity
                .object_to_opaque
                .insert(GameObjectId(2), OpaqueObjectId(2));
            identity.next_opaque_object_id = OpaqueObjectId(3);
            let pending = state.execution.pending_decision.as_mut().unwrap();
            let candidate = &mut pending.request.candidates[0];
            candidate.visible_intent = mtgml_decision::CandidateIntent::SelectObject {
                object: OpaqueObjectId(2),
            };
            candidate.trusted_binding = mtgml_decision::EngineCandidateBinding::SelectObject {
                object: GameObjectId(2),
            };
        }),
        ("execution_continuation", |state| {
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
            let pending = state.execution.pending_decision.as_mut().unwrap();
            pending.request.decision = mtgml_decision::DecisionDomainV2::ChooseNumber {
                minimum: 0,
                maximum: 3,
            };
            pending.request.candidates.clear();
            pending.request.continuation_id = Some(ContinuationId(1));
        }),
        ("pending_continuation_reference", |state| {
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
            let pending = state.execution.pending_decision.as_mut().unwrap();
            pending.request.decision = mtgml_decision::DecisionDomainV2::ChooseNumber {
                minimum: 0,
                maximum: 3,
            };
            pending.request.candidates.clear();
            pending.request.continuation_id = Some(ContinuationId(1));
        }),
        ("random_root_seed", |state| {
            let seed = state.random.root_seed.as_bytes();
            let mut hex = String::with_capacity(64);
            for byte in seed {
                std::fmt::Write::write_fmt(&mut hex, format_args!("{byte:02x}")).unwrap();
            }
            let last = hex.pop().unwrap();
            hex.push(if last == '1' { '2' } else { '1' });
            state.random.root_seed = RootSeed256::from_lower_hex(&hex).unwrap();
        }),
        ("random_stream_cursor", |state| {
            let key = RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1);
            let next = state.random.lookup_stream(&key).unwrap().next_raw_u64 + 1;
            state
                .random
                .set_cursor(&key, RandomStreamCursorV1 { next_raw_u64: next })
                .unwrap();
        }),
        ("random_additional_stream", |state| {
            state
                .random
                .streams
                .entry(RandomStreamKeyV1::player_scoped(
                    RandomStreamKindV1::SyntheticM1,
                    1,
                ))
                .or_default();
        }),
        ("knowledge_acquisition_provenance", |state| {
            let knowledge = state.knowledge.players.get_mut(&PlayerId(1)).unwrap();
            let record = knowledge.active.get_mut(&OpaqueObjectId(1)).unwrap();
            let provenance = observed(
                KnowledgeHistoryChannel::Public,
                0,
                KnowledgeAcquisitionCause::PublicEvent,
            );
            record.acquisition = provenance;
            record.known_location.as_mut().unwrap().provenance = provenance;
        }),
        ("knowledge_provenance_cause_only", |state| {
            let knowledge = state.knowledge.players.get_mut(&PlayerId(1)).unwrap();
            let record = knowledge.active.get_mut(&OpaqueObjectId(1)).unwrap();
            let provenance = observed(
                KnowledgeHistoryChannel::Public,
                0,
                KnowledgeAcquisitionCause::ExplicitReveal,
            );
            record.acquisition = provenance;
            record.known_location.as_mut().unwrap().provenance = provenance;
        }),
        ("knowledge_known_location", |state| {
            let graveyard = ZoneLocation {
                zone: ZoneKind::Graveyard,
                ..public_location()
            };
            state
                .zones
                .locations
                .insert(GameObjectId(1), graveyard.clone());
            for knowledge in state.knowledge.players.values_mut() {
                if let Some(record) = knowledge.active.get_mut(&OpaqueObjectId(1)) {
                    if let Some(current) = record.known_location.as_mut() {
                        current.location = graveyard.clone();
                    }
                }
            }
        }),
        ("knowledge_private_acquisition", |state| {
            let knowledge = state.knowledge.players.get_mut(&PlayerId(2)).unwrap();
            let record = knowledge.active.get_mut(&OpaqueObjectId(2)).unwrap();
            let provenance = observed(
                KnowledgeHistoryChannel::Private,
                0,
                KnowledgeAcquisitionCause::PrivateLook,
            );
            record.acquisition = provenance;
            record.known_location.as_mut().unwrap().provenance = provenance;
        }),
        ("knowledge_historical_location", |state| {
            let knowledge = state.knowledge.players.get_mut(&PlayerId(1)).unwrap();
            let record = knowledge.active.get_mut(&OpaqueObjectId(1)).unwrap();
            record.known_location = None;
            record.historical_locations.push(fact(
                    public_location(),
                    observed(
                        KnowledgeHistoryChannel::Public,
                        0,
                        KnowledgeAcquisitionCause::PublicEvent,
                    ),
                ));
        }),
        ("knowledge_retired_record", |state| {
            let identity = state
                .perspective_identities
                .players
                .get_mut(&PlayerId(1))
                .unwrap();
            identity.next_opaque_object_id = OpaqueObjectId(6);
            identity.retired_object_ids.insert(OpaqueObjectId(5));
            let knowledge = state.knowledge.players.get_mut(&PlayerId(1)).unwrap();
            knowledge
                .retired
                .insert(OpaqueObjectId(5), retired_record(OpaqueObjectId(5)));
        }),
        ("knowledge_next_visible_sequence", |state| {
            let knowledge = state.knowledge.players.get_mut(&PlayerId(1)).unwrap();
            knowledge.next_visible_sequence = VisibleSequence(2);
        }),
        ("identity_object_mapping", |state| {
            let identity = state
                .perspective_identities
                .players
                .get_mut(&PlayerId(1))
                .unwrap();
            identity
                .opaque_to_object
                .insert(OpaqueObjectId(2), GameObjectId(2));
            identity
                .object_to_opaque
                .insert(GameObjectId(2), OpaqueObjectId(2));
            identity.next_opaque_object_id = OpaqueObjectId(3);
        }),
        ("identity_ability_mapping", |state| {
            let identity = state
                .perspective_identities
                .players
                .get_mut(&PlayerId(1))
                .unwrap();
            identity
                .opaque_to_ability
                .insert(OpaqueAbilityId(1), AbilityInstanceId(1));
            identity
                .ability_to_opaque
                .insert(AbilityInstanceId(1), OpaqueAbilityId(1));
            identity.next_opaque_ability_id = OpaqueAbilityId(2);
            state.allocators.next_ability_id = AbilityInstanceId(2);
        }),
        ("identity_next_player_decision_id", |state| {
            let identity = state
                .perspective_identities
                .players
                .get_mut(&PlayerId(1))
                .unwrap();
            identity.next_player_decision_id = mtgml_model::PlayerDecisionIdV1(3);
        }),
        ("format_commander", |state| {
            state.format = FormatState::Commander {
                state: CommanderState {
                    designations: BTreeMap::from([(PlayerId(1), vec![PhysicalCardId(1)])]),
                    cast_counts: BTreeMap::new(),
                    damage: BTreeMap::new(),
                },
            };
        }),
    ];

    let baseline = synthetic_state();
    let baseline_digest = baseline.digest().unwrap();
    assert!(!mutations.is_empty());
    for (name, mutate) in mutations {
        let mut changed = synthetic_state();
        mutate(&mut changed);
        validate_engine_state(&changed)
            .unwrap_or_else(|error| panic!("mutation {name} must stay valid: {error}"));
        let changed_digest = changed.digest().unwrap();
        assert_ne!(
            baseline_digest, changed_digest,
            "mutation {name} must change the current V5 digest"
        );
    }
}


fn state_with_foundation_source() -> EngineState {
    let mut state = synthetic_state();
    state.foundation_sources.insert(
        GameObjectId(1),
        FoundationCreatureSource {
            source_kind: FoundationSourceKind::Creature,
            base_characteristics: BaseCharacteristics::Simple {
                power: 3,
                toughness: 3,
            },
            marked_damage: 0,
            control_history: ControlHistory::BeforeTurnStart { turn_number: 1 },
        },
    );
    validate_engine_state(&state).unwrap();
    state
}

#[test]
fn v5_digest_binds_foundation_source_inner_values() {
    let baseline = state_with_foundation_source();
    let baseline_digest = baseline.digest().unwrap();
    let mutations: [fn(&mut EngineState); 3] = [
        |state| {
            state
                .foundation_sources
                .get_mut(&GameObjectId(1))
                .unwrap()
                .marked_damage = 1;
        },
        |state| {
            state
                .foundation_sources
                .get_mut(&GameObjectId(1))
                .unwrap()
                .base_characteristics = BaseCharacteristics::Simple {
                power: 4,
                toughness: 3,
            };
        },
        |state| {
            state
                .foundation_sources
                .get_mut(&GameObjectId(1))
                .unwrap()
                .control_history = ControlHistory::DuringTurn {
                turn_number: state.core.turn_number,
                boundary: TurnPosition::Beginning {
                    step: BeginningStep::Untap,
                },
            };
        },
    ];

    for mutate in mutations {
        let mut changed = baseline.clone();
        mutate(&mut changed);
        validate_engine_state(&changed).unwrap();
        assert_ne!(baseline_digest, changed.digest().unwrap());
    }
}

#[test]
fn v5_digest_binds_combat_inner_values() {
    let mut baseline = synthetic_state();
    baseline.combat = Some(CombatState {
        defending_player: PlayerId(2),
        attackers: vec![GameObjectId(1), GameObjectId(2)],
        damage_step_completed: false,
        blocked_attackers: BTreeSet::new(),
        blockers: BTreeMap::from([
            (GameObjectId(1), None),
            (GameObjectId(2), None),
        ]),
    });
    baseline.core.position = TurnPosition::Combat {
        step: crate::CombatStep::CombatDamage,
    };
    validate_engine_state(&baseline).unwrap();
    let baseline_digest = baseline.digest().unwrap();

    let mut changed = baseline.clone();
    changed.combat.as_mut().unwrap().defending_player = PlayerId(1);
    validate_engine_state(&changed).unwrap();
    assert_ne!(baseline_digest, changed.digest().unwrap());

    let mut changed = baseline.clone();
    changed.combat.as_mut().unwrap().blocked_attackers.insert(GameObjectId(1));
    validate_engine_state(&changed).unwrap();
    assert_ne!(baseline_digest, changed.digest().unwrap());

    let mut changed = baseline.clone();
    changed.combat.as_mut().unwrap().damage_step_completed = true;
    validate_engine_state(&changed).unwrap();
    assert_ne!(baseline_digest, changed.digest().unwrap());
}

#[test]
fn knowledge_history_is_digested_without_a_player_level_aggregate() {
    let mut state = synthetic_state();
    let knowledge = state.knowledge.players.get_mut(&PlayerId(2)).unwrap();
    let record = knowledge.active.get_mut(&OpaqueObjectId(2)).unwrap();
    let location = record.known_location.clone().unwrap().location;
    record.known_location = None;
    record.historical_locations.push(fact(
        location,
        observed(
            KnowledgeHistoryChannel::Private,
            0,
            KnowledgeAcquisitionCause::PrivateLook,
        ),
    ));
    validate_engine_state(&state).unwrap();
    let with_history = state.digest().unwrap();
    let mut stripped = state.clone();
    stripped
        .knowledge
        .players
        .get_mut(&PlayerId(2))
        .unwrap()
        .active
        .get_mut(&OpaqueObjectId(2))
        .unwrap()
        .historical_locations
        .clear();
    validate_engine_state(&stripped).unwrap();
    assert_ne!(with_history, stripped.digest().unwrap());
}


#[test]
fn v5_digest_payload_is_nonempty_canonical_cbor() {
    let state = synthetic_state();
    let payload = state.canonical_digest_bytes().unwrap();
    assert!(!payload.is_empty());
    assert_eq!(payload[0] & 0xe0, 0x80, "root must be a CBOR array");
}

#[test]
fn historical_private_look_provenance_is_bound_into_the_digest() {
    let mut state = synthetic_state();
    let knowledge = state.knowledge.players.get_mut(&PlayerId(2)).unwrap();
    let record = knowledge.active.get_mut(&OpaqueObjectId(2)).unwrap();
    let location = record.known_location.clone().unwrap().location;
    record.known_location = None;
    record.historical_locations.push(fact(
        location,
        observed(
            KnowledgeHistoryChannel::Private,
            0,
            KnowledgeAcquisitionCause::PrivateLook,
        ),
    ));
    validate_engine_state(&state).unwrap();
    assert!(digest_payload_texts(&state).contains(&"private_look".to_string()));

    // Changing only the retained cause changes the V4 digest.
    let baseline_digest = state.digest().unwrap();
    let mut changed = state.clone();
    let knowledge = changed.knowledge.players.get_mut(&PlayerId(2)).unwrap();
    let record = knowledge.active.get_mut(&OpaqueObjectId(2)).unwrap();
    record.historical_locations[0].provenance = observed(
        KnowledgeHistoryChannel::Private,
        0,
        KnowledgeAcquisitionCause::OwnPrivateIdentity,
    );
    validate_engine_state(&changed).unwrap();
    assert_ne!(baseline_digest, changed.digest().unwrap());
}

#[test]
fn explicit_reveal_is_not_collapsed_to_public_event() {
    let mut state = synthetic_state();
    let knowledge = state.knowledge.players.get_mut(&PlayerId(1)).unwrap();
    let record = knowledge.active.get_mut(&OpaqueObjectId(1)).unwrap();
    let location = record.known_location.clone().unwrap().location;
    record.known_location = None;
    record.historical_locations.push(fact(
        location,
        observed(
            KnowledgeHistoryChannel::Public,
            0,
            KnowledgeAcquisitionCause::ExplicitReveal,
        ),
    ));
    validate_engine_state(&state).unwrap();
    let texts = digest_payload_texts(&state);
    assert!(texts.contains(&"explicit_reveal".to_string()));
}

#[test]
fn own_private_identity_is_not_collapsed_to_private_look() {
    let mut state = synthetic_state();
    let knowledge = state.knowledge.players.get_mut(&PlayerId(2)).unwrap();
    let record = knowledge.active.get_mut(&OpaqueObjectId(2)).unwrap();
    let location = record.known_location.clone().unwrap().location;
    record.known_location = None;
    record.historical_locations.push(fact(
        location,
        observed(
            KnowledgeHistoryChannel::Private,
            0,
            KnowledgeAcquisitionCause::OwnPrivateIdentity,
        ),
    ));
    validate_engine_state(&state).unwrap();
    assert!(digest_payload_texts(&state).contains(&"own_private_identity".to_string()));
}

#[test]
fn invalidation_provenance_is_preserved_exactly() {
    let mut state = synthetic_state();
    let identity = state
        .perspective_identities
        .players
        .get_mut(&PlayerId(1))
        .unwrap();
    identity.next_opaque_object_id = OpaqueObjectId(6);
    identity.retired_object_ids.insert(OpaqueObjectId(5));
    let knowledge = state.knowledge.players.get_mut(&PlayerId(1)).unwrap();
    knowledge
        .retired
        .insert(OpaqueObjectId(5), retired_record(OpaqueObjectId(5)));
    validate_engine_state(&state).unwrap();

    let texts = digest_payload_texts(&state);
    assert!(texts.contains(&"explicit_reveal".to_string()));
    assert!(texts.contains(&"shuffle".to_string()));

    // Mutating only the invalidation provenance changes the digest.
    let baseline_digest = state.digest().unwrap();
    let mut changed = state.clone();
    let knowledge = changed.knowledge.players.get_mut(&PlayerId(1)).unwrap();
    let record = knowledge.retired.get_mut(&OpaqueObjectId(5)).unwrap();
    record.invalidation.provenance = observed(
        KnowledgeHistoryChannel::Public,
        0,
        KnowledgeAcquisitionCause::PublicEvent,
    );
    validate_engine_state(&changed).unwrap();
    assert_ne!(baseline_digest, changed.digest().unwrap());
}

#[test]
fn historical_monotonicity_ignores_unsequenced_provenance() {
    let build = |provenances: Vec<KnowledgeAcquisitionReason>| {
        let mut state = synthetic_state();
        let knowledge = state.knowledge.players.get_mut(&PlayerId(1)).unwrap();
        let record = knowledge.active.get_mut(&OpaqueObjectId(1)).unwrap();
        let location = record.known_location.clone().unwrap().location;
        record.known_location = None;
        record.historical_locations = provenances
            .into_iter()
            .map(|provenance| fact(location.clone(), provenance))
            .collect();
        state
    };

    // An unsequenced initial fact followed by an observed fact is valid.
    let valid = build(vec![
        KnowledgeAcquisitionReason::InitialConfiguration,
        observed(
            KnowledgeHistoryChannel::Public,
            0,
            KnowledgeAcquisitionCause::PublicEvent,
        ),
    ]);
    validate_engine_state(&valid).unwrap();

    // An initial location after observed history is invalid.
    let invalid = build(vec![
        observed(
            KnowledgeHistoryChannel::Public,
            0,
            KnowledgeAcquisitionCause::PublicEvent,
        ),
        KnowledgeAcquisitionReason::InitialConfiguration,
    ]);
    assert_eq!(
        validate_engine_state(&invalid),
        Err(EngineStateViolation::EngineStateShape(
            EngineStateShapeViolation::Knowledge
        ))
    );
}
