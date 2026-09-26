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
fn full_state_digest_v6_matches_phase2_frozen_kat_and_verifies() {
    let (vector, input) = phase2_v6_fixture();
    let payload = input.canonical_payload().unwrap();
    assert_eq!(hex(&payload), vector["canonical_payload_hex"].as_str().unwrap());
    let digest = crate::digest_v6::calculate_full_state_digest_v6_payload(&payload).unwrap();
    assert_eq!(digest.to_string(), vector["expected_digest"].as_str().unwrap());
    crate::verify_full_state_digest_v6(&payload, &digest).unwrap();

    let wrong_digest = mtgml_model::FullStateDigestV6::from_digest_bytes([0xa5; 32]);
    assert!(crate::verify_full_state_digest_v6(&payload, &wrong_digest).is_err());
}

#[test]
fn full_state_digest_v6_rejects_predecessor_and_noncanonical_fixtures() {
    let (vector, input) = phase2_v6_fixture();
    let payload = input.canonical_payload().unwrap();
    let digest = mtgml_model::FullStateDigestV6::parse(
        vector["expected_digest"].as_str().unwrap().to_owned(),
    )
    .unwrap();
    let predecessor = decode_hex(
        include_str!("../../tests/fixtures/magic-sba-graveyard-order-v5-input.hex").trim(),
    );
    assert!(crate::verify_full_state_digest_v6(&predecessor, &digest).is_err());

    for path in [
        "../../persistence/negative/m4-v6-indefinite-array.cbor",
        "../../persistence/negative/m4-v6-map-where-array-required.cbor",
        "../../persistence/negative/m4-v6-noncanonical-integer.cbor",
        "../../persistence/negative/m4-v6-trailing-value.cbor",
    ] {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
        let invalid = std::fs::read(path).unwrap();
        assert!(crate::verify_full_state_digest_v6(&invalid, &digest).is_err());
    }
    crate::verify_full_state_digest_v6(&payload, &digest).unwrap();
}

#[test]
fn full_state_digest_v6_rejects_unknown_legacy_component_variants() {
    let (_, input) = phase2_v6_fixture();
    let baseline = input.canonical_payload().unwrap();
    let digest = crate::digest_v6::calculate_full_state_digest_v6_payload(&baseline).unwrap();
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
            crate::verify_full_state_digest_v6(&payload, &digest).is_err(),
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
    assert!(crate::canonical_state_bytes_v6(&synthetic_state(), state).is_err());
}

#[test]
fn full_state_digest_v6_matches_all_valid_phase2_mutation_vectors() {
    let (vector, baseline_input) = phase2_v6_fixture();
    let base = &vector["card_rules_authoritative_state"];
    let mutations = vector["mutation_digests"].as_object().unwrap();
    let expected_invalid = [
        "mana.player",
        "history.target_object",
        "counter.object",
        "face.object",
    ];
    let mut covered = std::collections::BTreeSet::new();
    for (name, expected) in mutations {
        let mut family = base.clone();
        match name.as_str() {
            "mana.player" => family[1][0][0] = serde_json::json!(3),
            "mana.restriction" => {
                family[1][0][1][0] = serde_json::json!(2);
                family[1][0][2][0] = serde_json::json!(0);
            }
            "mana.u32_boundary" => family[1][1][2][5] = serde_json::json!(u32::MAX - 1),
            "mana.explicit_zero" => family[1][1][1][0] = serde_json::json!(1),
            name if name.starts_with("mana.unrestricted.") => {
                let color = ["white", "blue", "black", "red", "green", "colorless"]
                    .iter()
                    .position(|color| name.ends_with(color))
                    .unwrap();
                family[1][0][1][color] = serde_json::json!(2);
            }
            name if name.starts_with("mana.creature_spell_only.") => {
                let color = ["white", "blue", "black", "red", "green", "colorless"]
                    .iter()
                    .position(|color| name.ends_with(color))
                    .unwrap();
                family[1][0][2][color] = serde_json::json!(2);
            }
            "history.turn_number" => family[2][0] = serde_json::json!(2),
            "history.land_plays_used" => family[2][1][0][1] = serde_json::json!(0),
            "history.spells_cast_total" => family[2][1][0][2] = serde_json::json!(3),
            "history.noncreature_spells_cast" => family[2][1][0][3] = serde_json::json!(2),
            "history.lost_life" => family[2][1][0][4] = serde_json::json!(false),
            "history.red_noncombat_damage" => family[2][1][0][5] = serde_json::json!(4),
            "history.permanent_to_graveyard" => family[2][1][0][6] = serde_json::json!(false),
            "history.target_object" => family[2][2][0][0] = serde_json::json!(3),
            "history.target_controller" => family[2][2][0][1] = serde_json::json!(1),
            "history.once_ability_object" => family[2][3][0][0] = serde_json::json!(2),
            "history.once_ability_key" => family[2][3][0][1] = serde_json::json!(1),
            "counter.object" => family[3][0][0] = serde_json::json!(2),
            "counter.kind" => family[3][0][1][0][0] = serde_json::json!(1),
            "counter.count" => family[3][0][1][0][1] = serde_json::json!(3),
            "attachment.source" => family[4][0][0] = serde_json::json!(4),
            "attachment.target" => family[4][0][1] = serde_json::json!(2),
            "attachment.timestamp_revision" => family[4][0][2] = serde_json::json!(3),
            "attachment.timestamp_operation" => family[4][0][3] = serde_json::json!(1),
            "face.object" => family[5][0][0] = serde_json::json!(2),
            "face.key" => family[5][0][1] = serde_json::json!(1),
            "ability.instance" => family[6][0][0] = serde_json::json!(2),
            "ability.source" => family[6][0][1] = serde_json::json!(2),
            "ability.key" => family[6][0][2] = serde_json::json!(1),
            unknown => panic!("unhandled Phase-2 mutation KAT: {unknown}"),
        }
        covered.insert(name.as_str());
        let raw_state = json_to_cbor(&family);
        let mut raw_input = mtgml_persistence::cbor::decode_canonical(
            &baseline_input.canonical_payload().unwrap(),
        )
        .unwrap();
        let Value::Array(raw_fields) = &mut raw_input else {
            unreachable!();
        };
        raw_fields[13] = raw_state.clone();
        let raw_payload = mtgml_persistence::cbor::encode_canonical(&raw_input).unwrap();
        let raw_digest = crate::digest_v6::calculate_full_state_digest_v6_payload(&raw_payload)
            .unwrap();
        assert_eq!(raw_digest.to_string(), expected.as_str().unwrap(), "raw KAT {name}");

        let typed = crate::CardRulesAuthoritativeStateV1::from_value(&raw_state);
        if expected_invalid.contains(&name.as_str()) {
            assert!(typed.is_err(), "invalid mutation unexpectedly became valid: {name}");
            continue;
        }
        let mut input = baseline_input.clone();
        input.card_rules_state = typed.unwrap();
        let payload = input.canonical_payload().unwrap();
        assert_eq!(payload, raw_payload, "typed encoder bytes for mutation {name}");
        let digest = crate::digest_v6::calculate_full_state_digest_v6_payload(&payload).unwrap();
        assert_eq!(
            digest.to_string(),
            expected.as_str().unwrap(),
            "mutation vector {name}"
        );
    }
    assert_eq!(covered.len(), mutations.len());
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
fn full_state_digest_v6_is_explicit_and_does_not_change_current_v5_digest() {
    let state = synthetic_state();
    let v5_before = state.digest().unwrap();
    let card_rules = crate::CardRulesAuthoritativeStateV1::default();
    let payload = crate::canonical_state_bytes_v6(&state, card_rules.clone()).unwrap();
    let v6 = crate::calculate_full_state_digest_v6(&state, card_rules).unwrap();
    crate::verify_full_state_digest_v6(&payload, &v6).unwrap();
    assert_eq!(state.digest().unwrap(), v5_before);
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
    let digest = crate::digest_v6::calculate_full_state_digest_v6_payload(&payload).unwrap();
    crate::verify_full_state_digest_v6(&payload, &digest).unwrap();

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

/// Frozen historical V4 known answer for the canonical synthetic reset state.
/// V4 bytes are evaluated only through the detached historical verifier.
#[test]
fn full_state_digest_v4_known_answer() {
    let state = synthetic_state();
    let payload = crate::canonical_state_bytes_v4_historical(&state).unwrap();
    const EXPECTED_PAYLOAD_HEX: &str = "8d781a66756c6c2d73746174652d6469676573742d696e7075742e7634781a6d74676d6c2e66756c6c2d73746174652d6469676573742e763400858283011828f483021828f401018269626567696e6e696e6765756e74617082646e6f6e65f68582870101010101f4f4870202020202f4f5828201856b626174746c656669656c64f68269756e6f726465726564f6667075626c6963f6820285676c696272617279028263746f700069666163655f646f776ef6818284676c6962726172790269666163655f646f776ef681028080880301010101020101858801010001667075626c6963826a63686f6f73655f6f6e65f6818300826d73656c6563745f6f626a65637401826d73656c6563745f6f626a65637401f680808080836c6d74676d6c2e726e672e763158201111111111111111111111111111111111111111111111111111111111111111818244010001000082840101818601010182856b626174746c656669656c64f68269756e6f726465726564f6667075626c6963f68275696e697469616c5f636f6e66696775726174696f6ef6808275696e697469616c5f636f6e66696775726174696f6ef680840201828601010182856b626174746c656669656c64f68269756e6f726465726564f6667075626c6963f68275696e697469616c5f636f6e66696775726174696f6ef6808275696e697469616c5f636f6e66696775726174696f6ef6860202028285676c696272617279028263746f700069666163655f646f776ef68275696e697469616c5f636f6e66696775726174696f6ef6808275696e697469616c5f636f6e66696775726174696f6ef68082880181820101800201028080880282820101820202800301028080f68082646e6f6e65f6";
    const EXPECTED_DIGEST_HEX: &str =
        "24fe3ab44864b6e3e7e75e55a62fba7fed6c94be3198ae5e93c1c196c1527227";
    assert_eq!(hex(&payload), EXPECTED_PAYLOAD_HEX);
    let digest = crate::calculate_full_state_digest_v4_historical(&state).unwrap();
    assert_eq!(digest.to_string(), EXPECTED_DIGEST_HEX);
    assert_eq!(digest.raw_bytes().len(), 32);
    assert_eq!(
        digest,
        crate::calculate_full_state_digest_v4_historical(&state).unwrap()
    );

    // The payload is exactly the thirteen declared top-level fields, and each
    // knowledge entry is the fixed four-element per-player record.
    let decoded = mtgml_persistence::cbor::decode_canonical(&payload).unwrap();
    let Value::Array(fields) = &decoded else {
        panic!("V4 payload must be an array");
    };
    assert_eq!(fields.len(), 13);
    assert_eq!(fields[0], Value::Text("full-state-digest-input.v4".into()));
    assert_eq!(fields[1], Value::Text("mtgml.full-state-digest.v4".into()));
    let Value::Array(knowledge_players) = &fields[8] else {
        panic!("knowledge_v2 must be an array");
    };
    for player_entry in knowledge_players {
        let Value::Array(entry) = player_entry else {
            panic!("knowledge_v2 entries must be arrays");
        };
        assert_eq!(entry.len(), 4, "knowledge_v2 per-player layout changed");
    }
}

#[test]
fn full_state_digest_v3_historical_known_answer_is_detached() {
    const HISTORICAL_V3_PAYLOAD_HEX: &str = concat!(
        "8b781a66756c6c2d73746174652d6469676573742d696e7075742e7633781a6d74676d6c2e66756c6c2d73746174652d",
        "6469676573742e763300848283011828f483021828f40101018582870101010101f4f4870202020202f4f5828201856b",
        "626174746c656669656c64f68269756e6f726465726564f6667075626c6963f6820285676c696272617279028263746f",
        "700069666163655f646f776ef6818284676c6962726172790269666163655f646f776ef6810280808803010101010201",
        "01858801010001667075626c6963826a63686f6f73655f6f6e65f6818300826d73656c6563745f6f626a65637401826d",
        "73656c6563745f6f626a65637401f680808080836c6d74676d6c2e726e672e7631582011111111111111111111111111",
        "11111111111111111111111111111111111111818244010001000082840101818601010182856b626174746c65666965",
        "6c64f68269756e6f726465726564f6667075626c6963f68275696e697469616c5f636f6e66696775726174696f6ef680",
        "8275696e697469616c5f636f6e66696775726174696f6ef680840201828601010182856b626174746c656669656c64f6",
        "8269756e6f726465726564f6667075626c6963f68275696e697469616c5f636f6e66696775726174696f6ef680827569",
        "6e697469616c5f636f6e66696775726174696f6ef6860202028285676c696272617279028263746f700069666163655f",
        "646f776ef68275696e697469616c5f636f6e66696775726174696f6ef6808275696e697469616c5f636f6e6669677572",
        "6174696f6ef6808288018182010180020102808088028282010182020280030102808082646e6f6e65f6",
    );
    let payload = decode_hex(HISTORICAL_V3_PAYLOAD_HEX);
    let envelope = mtgml_persistence::envelope::encode_envelope(
        "mtgml.full-state-digest.v3",
        "full-state-digest-input.v3",
        &payload,
    )
    .unwrap();
    let digest = mtgml_model::FullStateDigestV3::from_digest_bytes(
        mtgml_persistence::envelope::hash_envelope(&envelope),
    );

    assert_eq!(
        digest.to_string(),
        "680120895f69a0cea14399e53a80cc6bf3b10f167d7f9b21c5e2d38ebddf164a"
    );
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

#[test]
fn m3_p0_full_state_digest_v4_mutation_matrix() {
    let baseline = synthetic_state();
    let baseline_digest = crate::calculate_full_state_digest_v4_historical(&baseline).unwrap();
    let mutations: [fn(&mut EngineState); 3] = [
        |state| state.core.players.get_mut(&PlayerId(1)).unwrap().life += 1,
        |state| {
            state.core.position = TurnPosition::Beginning {
                step: BeginningStep::Upkeep,
            }
        },
        |state| state.zones.objects.get_mut(&GameObjectId(1)).unwrap().tapped = true,
    ];
    for mutate in mutations {
        let mut changed = synthetic_state();
        mutate(&mut changed);
        validate_engine_state(&changed).unwrap();
        assert_ne!(
            baseline_digest,
            crate::calculate_full_state_digest_v4_historical(&changed).unwrap()
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
fn state_delta_uses_full_state_digest_v5() {
    let before = synthetic_state();
    let mut after = before.clone();
    after.core.players.get_mut(&PlayerId(1)).unwrap().life = 39;
    let graveyard = ZoneLocation {
        zone: ZoneKind::Graveyard,
        ..public_location()
    };
    after
        .zones
        .locations
        .insert(GameObjectId(1), graveyard.clone());
    for knowledge in after.knowledge.players.values_mut() {
        if let Some(record) = knowledge.active.get_mut(&OpaqueObjectId(1)) {
            if let Some(current) = record.known_location.as_mut() {
                current.location = graveyard.clone();
            }
        }
    }
    let knowledge = after.knowledge.players.get_mut(&PlayerId(1)).unwrap();
    knowledge.next_visible_sequence = VisibleSequence(2);
    after
        .random
        .set_cursor(
            &RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1),
            RandomStreamCursorV1 { next_raw_u64: 1 },
        )
        .unwrap();
    after.allocators.next_object_id = GameObjectId(4);

    let delta = StateDelta::between(&before, &after, vec![]).unwrap();
    assert_eq!(delta.before_digest, before.digest().unwrap());
    assert_eq!(delta.after_digest, after.digest().unwrap());
    let reapplied = delta.apply(&before).unwrap();
    assert_eq!(reapplied, after);
    assert_eq!(reapplied.digest().unwrap(), delta.after_digest);

    let unrelated = empty_shell();
    assert!(matches!(
        delta.apply(&unrelated),
        Err(DeltaApplicationError::BeforeMismatch)
    ));
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
