//! Ownership: existing inline `mod tests` block moved verbatim from the
//! former monolithic `lib.rs`; module name and test identities unchanged.

use super::*;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use mtgml_model::{ObservationDigest, PlayerId, VisibleSequence};

fn observation(payload: &[u8], digest_payload: &[u8]) -> ObservationEnvelope {
    ObservationEnvelope {
        schema_version: OBSERVATION_SCHEMA_V2.into(),
        perspective: PlayerId(1),
        view_sequence: VisibleSequence(0),
        payload_codec: MAGIC_SHARED_EXECUTION_OBSERVATION_SCHEMA_V1.into(),
        payload_base64: STANDARD.encode(payload),
        digest: ObservationDigest::from_canonical_bytes(digest_payload),
    }
}

#[test]
fn basic_land_observation_v1_accepts_closed_public_state_facts_only() {
    let observation: MagicBasicLandObservationV1 = serde_json::from_str(include_str!(
        "../../../schemas/examples/magic-basic-land-observation-v1-ordered.json"
    ))
    .unwrap();
    observation.validate().unwrap();

    let with_candidates =
        include_str!("../../../schemas/examples/magic-basic-land-observation-v1.json").replace(
            "\"schema_version\":",
            "\"candidates\":[],\n  \"schema_version\":",
        );
    assert!(serde_json::from_str::<MagicBasicLandObservationV1>(&with_candidates).is_err());
}

#[test]
fn basic_land_observation_v1_examples_validate() {
    for fixture in [
        include_str!("../../../schemas/examples/magic-basic-land-observation-v1.json"),
        include_str!("../../../schemas/examples/magic-basic-land-observation-v1-ordered.json"),
    ] {
        let observation: MagicBasicLandObservationV1 = serde_json::from_str(fixture).unwrap();
        observation.validate().unwrap();
    }
}

#[test]
fn basic_land_observation_v1_rejects_duplicated_candidate_authority() {
    let mut value: serde_json::Value = serde_json::from_str(include_str!(
        "../../../schemas/examples/magic-basic-land-observation-v1.json"
    ))
    .unwrap();
    value["candidates"] = serde_json::json!([]);
    assert!(serde_json::from_value::<MagicBasicLandObservationV1>(value).is_err());
}

#[test]
fn information_state_input_excludes_trusted_fields() {
    let observation = observation(b"{}", b"{}");
    let input = InformationStateDigestInput {
        schema_version: INFORMATION_STATE_DIGEST_INPUT_SCHEMA_V3.into(),
        perspective: PlayerId(1),
        current_observation: observation,
        next_visible_sequence: VisibleSequence(0),
        retained_knowledge: vec![],
    };
    let json = serde_json::to_string(&input).unwrap();
    for forbidden in [
        "EpisodeStatus",
        "environment_limit_counters",
        "checkpoint_digest",
        "root_seed",
        "GameObjectId",
        "physical_card",
    ] {
        assert!(
            !json.contains(forbidden),
            "unexpected trusted field {forbidden}"
        );
    }
    let object = serde_json::to_value(&input).unwrap();
    assert!(object.get("digest").is_none());
    assert!(object.get("state_revision").is_none());
    assert_eq!(input.schema_version, "information-state-digest-input.v3");
}

#[test]
fn observation_digest_binding_accepts_matching_payload_and_rejects_mismatch() {
    assert_eq!(observation(b"{}", b"{}").validate(), Ok(()));
    assert_eq!(
        observation(b"{}", br#"{"x":1}"#).validate(),
        Err(ObservationValidationError::DigestMismatch)
    );
    let mut invalid_base64 = observation(b"{}", b"{}");
    invalid_base64.payload_base64 = "***".into();
    assert_eq!(
        invalid_base64.validate(),
        Err(ObservationValidationError::Base64)
    );
}

#[test]
fn observation_digest_known_value_binds_exact_payload_bytes() {
    let digest = ObservationDigest::from_canonical_bytes(b"{}");
    assert_eq!(
        digest.as_str(),
        "90845308617867fd703c6c4f37ede7908da24420053821f89190ad36236dfca3"
    );
    assert_ne!(digest, ObservationDigest::from_canonical_bytes(b"e30="));
}

#[test]
fn basic_land_observation_v1_shows_life_card_counts_and_tapped_permanents() {
    let example: serde_json::Value = serde_json::from_str(include_str!(
        "../../../schemas/examples/magic-basic-land-observation-v1.json"
    ))
    .unwrap();
    let observation: MagicBasicLandObservationV1 = serde_json::from_value(example.clone()).unwrap();
    observation.validate().unwrap();
    assert_eq!(
        observation.players,
        vec![
            PlayerObservationV1 {
                player: PlayerId(0),
                life: 20,
                hand_count: 6,
                library_count: 33,
            },
            PlayerObservationV1 {
                player: PlayerId(1),
                life: -2,
                hand_count: 7,
                library_count: 33,
            },
        ]
    );
    assert_eq!(observation.tapped, vec![mtgml_model::OpaqueObjectId(7)]);

    let broken = |edit: &dyn Fn(&mut serde_json::Value)| {
        let mut value = example.clone();
        edit(&mut value);
        serde_json::from_value::<MagicBasicLandObservationV1>(value)
            .map_err(|_| ())
            .and_then(|observation| observation.validate().map_err(|_| ()))
    };
    // Players are listed once each, in ascending order, and include the
    // active player.
    assert!(broken(&|value| value["players"].as_array_mut().unwrap().reverse()).is_err());
    assert!(broken(&|value| {
        let first = value["players"][0].clone();
        value["players"][1] = first;
    })
    .is_err());
    assert!(broken(&|value| value["active_player"] = serde_json::json!("2")).is_err());
    // Tapped permanents are listed once each, in ascending order.
    assert!(broken(&|value| value["tapped"] = serde_json::json!(["7", "7"])).is_err());
    assert!(broken(&|value| value["tapped"] = serde_json::json!(["8", "7"])).is_err());
    // Counts are numbers, not strings; both fields are required.
    assert!(broken(&|value| value["players"][0]["hand_count"] = serde_json::json!("6")).is_err());
    assert!(broken(&|value| {
        value.as_object_mut().unwrap().remove("tapped");
    })
    .is_err());
}

#[test]
fn basic_land_observation_v1_shows_every_permanent_with_its_controller_and_the_attackers() {
    let example: serde_json::Value = serde_json::from_str(include_str!(
        "../../../schemas/examples/magic-basic-land-observation-v1.json"
    ))
    .unwrap();
    let observation: MagicBasicLandObservationV1 = serde_json::from_value(example.clone()).unwrap();
    observation.validate().unwrap();
    let row = |object,
               controller,
               controlled_since_turn,
               power_toughness: Option<(i64, i64)>,
               marked_damage| PermanentObservationV1 {
        object: mtgml_model::OpaqueObjectId(object),
        controller: PlayerId(controller),
        controlled_since_turn,
        power: power_toughness.map(|(power, _)| power),
        toughness: power_toughness.map(|(_, toughness)| toughness),
        marked_damage,
    };
    // Two creatures and a non-creature, each with its controller.
    assert_eq!(
        observation.permanents,
        vec![
            row(6, 1, 2, Some((3, 3)), 2),
            row(7, 0, 1, Some((2, 2)), 1),
            row(8, 0, 3, None, 0),
        ]
    );
    assert_eq!(observation.attacking, vec![mtgml_model::OpaqueObjectId(7)]);
    // The arrival turn travels as a decimal string, as `turn_number` does,
    // and a non-creature's power and toughness as null.
    assert_eq!(serde_json::to_value(&observation).unwrap(), example);
    assert!(example["permanents"][2]["power"].is_null());

    let broken = |edit: &dyn Fn(&mut serde_json::Value)| {
        let mut value = example.clone();
        edit(&mut value);
        serde_json::from_value::<MagicBasicLandObservationV1>(value)
            .map_err(|_| ())
            .and_then(|observation| observation.validate().map_err(|_| ()))
    };
    // Permanents are listed once each, in ascending order.
    assert!(broken(&|value| value["permanents"].as_array_mut().unwrap().reverse()).is_err());
    assert!(broken(&|value| {
        let second = value["permanents"][1].clone();
        value["permanents"][0] = second;
    })
    .is_err());
    // Power and toughness are both present or both null.
    assert!(
        broken(&|value| value["permanents"][1]["toughness"] = serde_json::Value::Null).is_err()
    );
    assert!(broken(&|value| value["permanents"][2]["power"] = serde_json::json!(1)).is_err());
    assert!(broken(&|value| {
        value["permanents"][1]
            .as_object_mut()
            .unwrap()
            .remove("power");
    })
    .is_err());
    // Attackers are listed once each, in ascending order, and are the
    // creatures among the permanents: a non-creature or an unlisted object is
    // not one.
    assert!(broken(&|value| {
        // Nobody attacks, so nothing is blocked and nothing blocks.
        value["attacking"] = serde_json::json!([]);
        value["blocked"] = serde_json::json!([]);
        value["blocking"] = serde_json::json!([]);
    })
    .is_ok());
    assert!(broken(&|value| value["attacking"] = serde_json::json!(["6", "7"])).is_ok());
    assert!(broken(&|value| value["attacking"] = serde_json::json!(["7", "6"])).is_err());
    assert!(broken(&|value| value["attacking"] = serde_json::json!(["7", "7"])).is_err());
    assert!(broken(&|value| value["attacking"] = serde_json::json!(["8"])).is_err());
    assert!(broken(&|value| value["attacking"] = serde_json::json!(["7", "8"])).is_err());
    assert!(broken(&|value| value["attacking"] = serde_json::json!(["9"])).is_err());
    // Numbers are numbers, not strings; every field and list is required.
    assert!(broken(&|value| value["permanents"][0]["power"] = serde_json::json!("3")).is_err());
    assert!(broken(&|value| value["permanents"][0]["toughness"] = serde_json::json!("3")).is_err());
    // The arrival turn is a canonical decimal string of a u64: a JSON number,
    // a sign, leading zeros, other characters and an out-of-range value are
    // refused.
    for turn in ["0", "1", "18446744073709551615"] {
        assert!(
            broken(&|value| {
                value["permanents"][0]["controlled_since_turn"] = serde_json::json!(turn);
            })
            .is_ok(),
            "{turn}"
        );
    }
    for turn in [
        serde_json::json!(1),
        serde_json::json!(-1),
        serde_json::json!(1.5),
        serde_json::json!(null),
        serde_json::json!("01"),
        serde_json::json!("+1"),
        serde_json::json!("-1"),
        serde_json::json!("1a"),
        serde_json::json!(" 1"),
        serde_json::json!(""),
        serde_json::json!("18446744073709551616"),
    ] {
        assert!(
            broken(&|value| value["permanents"][0]["controlled_since_turn"] = turn.clone())
                .is_err(),
            "{turn}"
        );
    }
    assert!(broken(&|value| value["permanents"][0]["extra"] = serde_json::json!(0)).is_err());
    assert!(broken(&|value| {
        value["permanents"][0]
            .as_object_mut()
            .unwrap()
            .remove("controller");
    })
    .is_err());
    assert!(broken(&|value| {
        value.as_object_mut().unwrap().remove("permanents");
    })
    .is_err());
    assert!(broken(&|value| {
        value.as_object_mut().unwrap().remove("attacking");
    })
    .is_err());
    // The creature-only rows of the first design are gone.
    assert!(broken(&|value| value["creatures"] = serde_json::json!([])).is_err());
}

/// `example` with `edit` applied, decoded and validated as the basic-land
/// observation.
fn edited_basic(
    example: &serde_json::Value,
    edit: &dyn Fn(&mut serde_json::Value),
) -> Result<MagicBasicLandObservationV1, ()> {
    let mut value = example.clone();
    edit(&mut value);
    serde_json::from_value::<MagicBasicLandObservationV1>(value)
        .map_err(|_| ())
        .and_then(|observation| observation.validate().map(|()| observation).map_err(|_| ()))
}

#[test]
fn basic_land_observation_v1_shows_blocks_and_marked_damage() {
    // CR 509.1g, 509.1h, 120.3e: the blocked attackers, each blocking creature
    // with the attacker it blocks (none when that attacker left combat), and
    // the damage marked on every permanent.
    let example: serde_json::Value = serde_json::from_str(include_str!(
        "../../../schemas/examples/magic-basic-land-observation-v1.json"
    ))
    .unwrap();
    let observation = edited_basic(&example, &|_| {}).unwrap();
    let opaque = mtgml_model::OpaqueObjectId;
    assert_eq!(observation.attacking, vec![opaque(7)]);
    assert_eq!(observation.blocked, vec![opaque(7)]);
    assert_eq!(
        observation.blocking,
        vec![BlockObservationV1 {
            blocker: opaque(6),
            attacker: Some(opaque(7)),
        }]
    );
    // The example has no block declaration or division in progress: the keys
    // are there, and null.
    assert_eq!(observation.pending_blocks, None);
    assert_eq!(observation.pending_damage_assignment, None);
    assert!(example["pending_blocks"].is_null());
    assert!(example["pending_damage_assignment"].is_null());
    // Marked damage travels as a decimal string, as `turn_number` does, and is
    // "0" for a permanent that is not a creature.
    let marked: Vec<_> = observation
        .permanents
        .iter()
        .map(|permanent| permanent.marked_damage)
        .collect();
    assert_eq!(marked, vec![2, 1, 0]);
    assert_eq!(example["permanents"][0]["marked_damage"], "2");
    assert_eq!(example["permanents"][2]["marked_damage"], "0");
    assert_eq!(serde_json::to_value(&observation).unwrap(), example);

    // A blocker whose attacker left combat blocks nothing, and is still a
    // blocking creature.
    let ordered: serde_json::Value = serde_json::from_str(include_str!(
        "../../../schemas/examples/magic-basic-land-observation-v1-ordered.json"
    ))
    .unwrap();
    let observation = edited_basic(&ordered, &|_| {}).unwrap();
    assert_eq!(observation.blocked, Vec::new());
    assert_eq!(
        observation.blocking,
        vec![BlockObservationV1 {
            blocker: opaque(5),
            attacker: None,
        }]
    );
    assert!(ordered["blocking"][0]["attacker"].is_null());
    assert_eq!(serde_json::to_value(&observation).unwrap(), ordered);

    let broken = |edit: &dyn Fn(&mut serde_json::Value)| edited_basic(&example, edit).is_err();
    // Blocked attackers are listed once each, in ascending order, and are
    // attacking.
    assert!(!broken(&|value| value["blocked"] = serde_json::json!(["7"])));
    assert!(broken(&|value| value["blocked"] = serde_json::json!(["6"])));
    assert!(broken(
        &|value| value["blocked"] = serde_json::json!(["7", "7"])
    ));
    assert!(broken(&|value| {
        value["attacking"] = serde_json::json!(["6", "7"]);
        value["blocked"] = serde_json::json!(["7", "6"]);
    }));
    // A blocking creature's attacker is attacking and blocked; a blocked
    // attacker may have no blocker left (CR 509.1h).
    assert!(!broken(&|value| value["blocking"] = serde_json::json!([])));
    assert!(broken(
        &|value| value["blocking"][0]["attacker"] = serde_json::json!("6")
    ));
    assert!(broken(
        &|value| value["blocking"][0]["attacker"] = serde_json::json!("9")
    ));
    assert!(broken(&|value| value["blocked"] = serde_json::json!([])));
    // Blocks are listed once per blocker, in ascending order of blocker, and
    // a blocker is a creature among the permanents.
    let two_blockers = |first: &str, second: &str| {
        serde_json::json!([
            {"blocker": first, "attacker": "7"},
            {"blocker": second, "attacker": "7"},
        ])
    };
    assert!(!broken(&|value| value["blocking"] = two_blockers("6", "7")));
    assert!(broken(&|value| value["blocking"] = two_blockers("7", "6")));
    assert!(broken(&|value| value["blocking"] = two_blockers("6", "6")));
    assert!(broken(
        &|value| value["blocking"][0]["blocker"] = serde_json::json!("8")
    ));
    assert!(broken(
        &|value| value["blocking"][0]["blocker"] = serde_json::json!("9")
    ));
    // Marked damage exists on creatures only (CR 120.3e).
    assert!(!broken(
        &|value| value["permanents"][1]["marked_damage"] = serde_json::json!("0")
    ));
    assert!(broken(
        &|value| value["permanents"][2]["marked_damage"] = serde_json::json!("1")
    ));
    // It is a canonical decimal string of a u64.
    for damage in ["0", "3", "18446744073709551615"] {
        assert!(
            !broken(&|value| value["permanents"][0]["marked_damage"] = serde_json::json!(damage)),
            "{damage}"
        );
    }
    for damage in [
        serde_json::json!(2),
        serde_json::json!(-1),
        serde_json::json!(null),
        serde_json::json!("02"),
        serde_json::json!("+2"),
        serde_json::json!("-2"),
        serde_json::json!("2a"),
        serde_json::json!(""),
        serde_json::json!("18446744073709551616"),
    ] {
        assert!(
            broken(&|value| value["permanents"][0]["marked_damage"] = damage.clone()),
            "{damage}"
        );
    }
    // Every key is required, and a block row has no other.
    for key in [
        "blocked",
        "blocking",
        "pending_blocks",
        "pending_damage_assignment",
    ] {
        assert!(
            broken(&|value| {
                value.as_object_mut().unwrap().remove(key);
            }),
            "{key}"
        );
    }
    assert!(broken(&|value| {
        value["permanents"][0]
            .as_object_mut()
            .unwrap()
            .remove("marked_damage");
    }));
    assert!(broken(&|value| {
        value["blocking"][0]
            .as_object_mut()
            .unwrap()
            .remove("attacker");
    }));
    assert!(broken(
        &|value| value["blocking"][0]["extra"] = serde_json::json!(0)
    ));
}

#[test]
fn the_pending_answers_of_a_block_or_a_division_have_a_shape_and_are_null_by_default() {
    // The keys exist for the player's own half-made answers: a block row
    // names the creature and what it blocks (null: it blocks nothing), a
    // damage row the attacker, the creature it damages and the amount.
    let example: serde_json::Value = serde_json::from_str(include_str!(
        "../../../schemas/examples/magic-basic-land-observation-v1.json"
    ))
    .unwrap();
    let observation = edited_basic(&example, &|value| {
        value["pending_blocks"] = serde_json::json!([
            {"blocker": "6", "attacker": "7"},
            {"blocker": "7", "attacker": null},
        ]);
        value["pending_damage_assignment"] = serde_json::json!([
            {"attacker": "7", "blocker": "6", "amount": "2"},
        ]);
    })
    .unwrap();
    let opaque = mtgml_model::OpaqueObjectId;
    assert_eq!(
        observation.pending_blocks,
        Some(vec![
            DeclaredBlockObservationV1 {
                blocker: opaque(6),
                attacker: Some(opaque(7)),
            },
            DeclaredBlockObservationV1 {
                blocker: opaque(7),
                attacker: None,
            },
        ])
    );
    assert_eq!(
        observation.pending_damage_assignment,
        Some(vec![AssignedDamageObservationV1 {
            attacker: opaque(7),
            blocker: opaque(6),
            amount: 2,
        }])
    );
    // The amount is a canonical decimal string; a row has no missing field.
    for amount in [serde_json::json!(2), serde_json::json!("02")] {
        assert!(edited_basic(&example, &|value| {
            value["pending_damage_assignment"] =
                serde_json::json!([{"attacker": "7", "blocker": "6", "amount": amount}]);
        })
        .is_err());
    }
    assert!(edited_basic(&example, &|value| {
        value["pending_blocks"] = serde_json::json!([{"blocker": "6"}]);
    })
    .is_err());
}

#[test]
fn the_pending_answers_are_listed_in_ascending_order() {
    // A block row is for one creature, ascending by blocker; a damage row is
    // for one blocker of an attacker, ascending by attacker and then blocker.
    // Both lists may be empty: the declaration or division has begun and
    // nothing is answered yet, which is not null.
    let example: serde_json::Value = serde_json::from_str(include_str!(
        "../../../schemas/examples/magic-basic-land-observation-v1.json"
    ))
    .unwrap();
    let blocks = |rows: &[(&str, Option<&str>)]| {
        edited_basic(&example, &|value| {
            value["pending_blocks"] = rows
                .iter()
                .map(|(blocker, attacker)| {
                    serde_json::json!({"blocker": blocker, "attacker": attacker})
                })
                .collect();
        })
        .is_ok()
    };
    assert!(blocks(&[]));
    assert!(blocks(&[("6", Some("7"))]));
    assert!(blocks(&[("6", Some("7")), ("7", None)]));
    assert!(!blocks(&[("7", None), ("6", Some("7"))]));
    assert!(!blocks(&[("6", Some("7")), ("6", None)]));
    assert!(!blocks(&[("6", Some("7")), ("6", Some("7"))]));
    // Order is by the number, not the text.
    assert!(blocks(&[("9", None), ("10", None)]));
    assert!(!blocks(&[("10", None), ("9", None)]));

    let damage = |rows: &[(&str, &str, &str)]| {
        edited_basic(&example, &|value| {
            value["pending_damage_assignment"] = rows
                .iter()
                .map(|(attacker, blocker, amount)| {
                    serde_json::json!({"attacker": attacker, "blocker": blocker, "amount": amount})
                })
                .collect();
        })
        .is_ok()
    };
    assert!(damage(&[]));
    assert!(damage(&[("7", "6", "2")]));
    assert!(damage(&[("7", "6", "2"), ("7", "7", "0")]));
    assert!(damage(&[("7", "7", "0"), ("8", "6", "1")]));
    assert!(!damage(&[("7", "7", "0"), ("7", "6", "2")]));
    assert!(!damage(&[("8", "6", "1"), ("7", "7", "0")]));
    assert!(!damage(&[("7", "6", "2"), ("7", "6", "1")]));
    assert!(damage(&[("9", "6", "1"), ("10", "6", "1")]));
    assert!(!damage(&[("10", "6", "1"), ("9", "6", "1")]));
}
