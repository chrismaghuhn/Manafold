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
fn basic_land_observation_v1_shows_creatures_and_attackers() {
    let example: serde_json::Value = serde_json::from_str(include_str!(
        "../../../schemas/examples/magic-basic-land-observation-v1.json"
    ))
    .unwrap();
    let observation: MagicBasicLandObservationV1 = serde_json::from_value(example.clone()).unwrap();
    observation.validate().unwrap();
    assert_eq!(
        observation.creatures,
        vec![
            CreatureObservationV1 {
                object: mtgml_model::OpaqueObjectId(3),
                controller: PlayerId(1),
                power: 3,
                toughness: 3,
                controlled_since_turn: 1,
            },
            CreatureObservationV1 {
                object: mtgml_model::OpaqueObjectId(7),
                controller: PlayerId(0),
                power: 2,
                toughness: 1,
                controlled_since_turn: 1,
            },
        ]
    );
    assert_eq!(observation.attacking, vec![mtgml_model::OpaqueObjectId(7)]);
    // The arrival turn travels as a decimal string, as `turn_number` does.
    assert_eq!(serde_json::to_value(&observation).unwrap(), example);

    let broken = |edit: &dyn Fn(&mut serde_json::Value)| {
        let mut value = example.clone();
        edit(&mut value);
        serde_json::from_value::<MagicBasicLandObservationV1>(value)
            .map_err(|_| ())
            .and_then(|observation| observation.validate().map_err(|_| ()))
    };
    // Creatures are listed once each, in ascending order.
    assert!(broken(&|value| value["creatures"].as_array_mut().unwrap().reverse()).is_err());
    assert!(broken(&|value| {
        let second = value["creatures"][1].clone();
        value["creatures"][0] = second;
    })
    .is_err());
    // Attackers are listed once each, in ascending order, and are creatures.
    assert!(broken(&|value| value["attacking"] = serde_json::json!(["3", "7"])).is_ok());
    assert!(broken(&|value| value["attacking"] = serde_json::json!(["7", "3"])).is_err());
    assert!(broken(&|value| value["attacking"] = serde_json::json!(["7", "7"])).is_err());
    assert!(broken(&|value| value["attacking"] = serde_json::json!(["8"])).is_err());
    assert!(broken(&|value| value["attacking"] = serde_json::json!(["3", "8"])).is_err());
    // Numbers are numbers, not strings; every field and list is required.
    assert!(broken(&|value| value["creatures"][0]["power"] = serde_json::json!("3")).is_err());
    assert!(broken(&|value| value["creatures"][0]["toughness"] = serde_json::json!("3")).is_err());
    // The arrival turn is a canonical decimal string of a u64: a JSON number,
    // a sign, leading zeros, other characters and an out-of-range value are
    // refused.
    for turn in ["0", "1", "18446744073709551615"] {
        assert!(
            broken(&|value| {
                value["creatures"][0]["controlled_since_turn"] = serde_json::json!(turn);
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
            broken(&|value| value["creatures"][0]["controlled_since_turn"] = turn.clone()).is_err(),
            "{turn}"
        );
    }
    assert!(broken(&|value| value["creatures"][0]["extra"] = serde_json::json!(0)).is_err());
    assert!(broken(&|value| {
        value["creatures"][0]
            .as_object_mut()
            .unwrap()
            .remove("controller");
    })
    .is_err());
    assert!(broken(&|value| {
        value.as_object_mut().unwrap().remove("creatures");
    })
    .is_err());
    assert!(broken(&|value| {
        value.as_object_mut().unwrap().remove("attacking");
    })
    .is_err());
}
