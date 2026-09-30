//! Ownership: existing inline `mod tests` block moved verbatim from the
//! former monolithic `lib.rs`; module name and test identities unchanged.

use super::*;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use mtgml_model::{ObservationDigest, PlayerId, StateRevision, VisibleSequence};

fn observation(payload: &[u8], digest_payload: &[u8]) -> ObservationEnvelope {
    ObservationEnvelope {
        schema_version: OBSERVATION_SCHEMA.into(),
        perspective: PlayerId(1),
        state_revision: StateRevision(0),
        payload_codec: "synthetic-m2-observation.v1".into(),
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
    let input = InformationStateDigestInputV2 {
        schema_version: "information-state-digest-input.v2".into(),
        perspective: PlayerId(1),
        state_revision: StateRevision(0),
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
    assert_eq!(input.schema_version, "information-state-digest-input.v2");
}

#[test]
fn observation_digest_binding_accepts_matching_payload_and_rejects_mismatch() {
    assert_eq!(observation(b"{}", b"{}").validate(), Ok(()));
    assert_eq!(
        observation(b"{}", br#"{"x":1}"#).validate(),
        Err(ObservationValidationError::DigestMismatch)
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
