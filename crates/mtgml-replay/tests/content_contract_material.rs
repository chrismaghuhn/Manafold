//! The content contract child that Replay V8 embeds in its semantic contract.

use mtgml_replay::ReplayManifestV8;
use serde_json::Value;

const MANIFEST: &[u8] = include_bytes!("../../../schemas/examples/replay-manifest-v8.json");
const CONTENT_ID: &str = "80d26c187739664e880948e767e44ed9791aa7c25e7ef703e6d63385311fb346";

fn baseline() -> Value {
    let value: Value = serde_json::from_slice(MANIFEST).unwrap();
    serde_json::from_value::<ReplayManifestV8>(value.clone())
        .unwrap()
        .validate()
        .unwrap();
    value
}

#[test]
fn content_child_rejects_noncanonical_transport_and_identity_mismatch() {
    let baseline = baseline();
    let mut cases = Vec::new();

    let mut bad_alphabet = baseline.clone();
    bad_alphabet["semantic_contract"]["content_contract"]["manifest_canonical_cbor_base64"] =
        Value::String("!!!!".into());
    cases.push(bad_alphabet);

    let mut missing_padding = baseline.clone();
    let encoded = missing_padding["semantic_contract"]["content_contract"]
        ["manifest_canonical_cbor_base64"]
        .as_str()
        .unwrap()
        .trim_end_matches('=')
        .to_owned();
    missing_padding["semantic_contract"]["content_contract"]["manifest_canonical_cbor_base64"] =
        Value::String(encoded);
    cases.push(missing_padding);

    let mut wrong_child_id = baseline.clone();
    wrong_child_id["semantic_contract"]["content_contract"]["content_contract_id"] =
        Value::String("00".repeat(32));
    cases.push(wrong_child_id);

    let mut parent_child_mismatch = baseline;
    parent_child_mismatch["semantic_contract"]["manifest"]["content_contract_id"] = Value::Null;
    cases.push(parent_child_mismatch);

    for candidate in cases {
        if let Ok(manifest) = serde_json::from_value::<ReplayManifestV8>(candidate) {
            assert!(manifest.validate().is_err());
        }
    }
}

#[test]
fn content_child_semantic_negative_vectors_are_rejected() {
    let cases: Value = serde_json::from_slice(include_bytes!(
        "../../../schemas/negative/m4-phase2-replay-child-semantic-negatives.json"
    ))
    .unwrap();
    for case in cases["cases"].as_array().unwrap() {
        let mut manifest = baseline();
        manifest["semantic_contract"]["manifest"]["content_contract_id"] =
            case["parent_content_contract_id"].clone();
        manifest["semantic_contract"]["content_contract"] = case["content_contract"].clone();
        if let Ok(value) = serde_json::from_value::<ReplayManifestV8>(manifest) {
            assert!(value.validate().is_err(), "case {}", case["case"]);
        }
    }
}

#[test]
fn content_child_rejects_duplicate_and_unknown_json_fields() {
    let child_id = format!("\"content_contract_id\": \"{CONTENT_ID}\",");
    let text = std::str::from_utf8(MANIFEST).unwrap();
    assert!(text.contains(&child_id));
    let duplicate = text.replacen(&child_id, &format!("{child_id}{child_id}"), 1);
    assert!(serde_json::from_str::<ReplayManifestV8>(&duplicate).is_err());

    let mut value = baseline();
    value["semantic_contract"]["content_contract"]["unrecognized"] = Value::Bool(true);
    assert!(serde_json::from_value::<ReplayManifestV8>(value).is_err());
}

#[test]
fn content_child_serializer_rejects_invalid_typed_identity() {
    let mut manifest: ReplayManifestV8 = serde_json::from_slice(MANIFEST).unwrap();
    manifest
        .semantic_contract
        .content_contract
        .as_mut()
        .unwrap()
        .content_contract_id = mtgml_model::ContentContractIdV1::parse("00".repeat(32)).unwrap();
    assert!(serde_json::to_vec(&manifest).is_err());
}
