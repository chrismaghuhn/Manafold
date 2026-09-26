use mtgml_model::{
    ExecutionProgramV1, RulesAuthorityV1, RulesContractManifestV1, SemanticContractManifestV1,
};
use mtgml_persistence::{checkpoint_digest, semantic_contract_digest};
use mtgml_replay::{
    AuthoritativeReplayV7, ReplayManifestV7, ReplayRecorderV7, ReplayValidationError,
};
use serde_json::Value;

const MANIFEST: &[u8] = include_bytes!("../../../schemas/examples/replay-manifest.v7.json");
const REPLAY: &[u8] =
    include_bytes!("../../../schemas/examples/authoritative-replay-v7-rejected-step.json");

#[test]
fn valid_v7_manifest_verifies_content_child_and_contract_identity_chain() {
    let manifest: ReplayManifestV7 = serde_json::from_slice(MANIFEST).expect("typed V7 manifest");
    manifest
        .validate()
        .expect("valid manifest and content child");

    let wire = serde_json::to_vec(&manifest).expect("V7 manifest wire");
    let round_trip: ReplayManifestV7 = serde_json::from_slice(&wire).expect("round trip");
    assert_eq!(round_trip, manifest);
}

#[test]
fn authoritative_replay_v7_validates_rejected_step_and_final_identity() {
    let replay: AuthoritativeReplayV7 = serde_json::from_slice(REPLAY).expect("typed replay");
    replay.validate().expect("structurally valid rejected step");
}

#[test]
fn content_child_rejects_noncanonical_transport_and_identity_mismatch() {
    let baseline: Value = serde_json::from_slice(MANIFEST).unwrap();
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
        let parsed = serde_json::from_value::<ReplayManifestV7>(candidate);
        if let Ok(manifest) = parsed {
            assert!(manifest.validate().is_err());
        }
    }
}

#[test]
fn phase_two_content_child_negative_vectors_are_consumed() {
    let cases: Value = serde_json::from_slice(include_bytes!(
        "../../../schemas/negative/m4-phase2-replay-child-semantic-negatives.json"
    ))
    .unwrap();
    for case in cases["cases"].as_array().unwrap() {
        let mut manifest: Value = serde_json::from_slice(MANIFEST).unwrap();
        manifest["semantic_contract"]["manifest"]["content_contract_id"] =
            case["parent_content_contract_id"].clone();
        manifest["semantic_contract"]["content_contract"] = case["content_contract"].clone();
        let parsed = serde_json::from_value::<ReplayManifestV7>(manifest);
        if let Ok(value) = parsed {
            assert!(value.validate().is_err(), "case {}", case["case"]);
        }
    }
}

#[test]
fn content_child_rejects_duplicate_and_unknown_json_fields() {
    let duplicate = include_str!("../../../schemas/examples/replay-manifest.v7.json")
        .replace(
            "\"content_contract_id\": \"80d26c187739664e880948e767e44ed9791aa7c25e7ef703e6d63385311fb346\",",
            "\"content_contract_id\": \"80d26c187739664e880948e767e44ed9791aa7c25e7ef703e6d63385311fb346\",\"content_contract_id\": \"80d26c187739664e880948e767e44ed9791aa7c25e7ef703e6d63385311fb346\",",
        );
    assert!(serde_json::from_str::<ReplayManifestV7>(&duplicate).is_err());

    let mut value: Value = serde_json::from_slice(MANIFEST).unwrap();
    value["semantic_contract"]["content_contract"]["unrecognized"] = Value::Bool(true);
    assert!(serde_json::from_value::<ReplayManifestV7>(value).is_err());
}

#[test]
fn content_child_serializer_rejects_invalid_typed_identity() {
    let mut manifest: ReplayManifestV7 = serde_json::from_slice(MANIFEST).unwrap();
    manifest
        .semantic_contract
        .content_contract
        .as_mut()
        .unwrap()
        .content_contract_id = mtgml_model::ContentContractIdV1::parse("00".repeat(32)).unwrap();
    assert!(serde_json::to_vec(&manifest).is_err());
}

#[test]
fn replay_v7_rejects_wrong_schema_identity_and_step_revision_link() {
    let mut manifest: Value = serde_json::from_slice(MANIFEST).unwrap();
    manifest["schemas"]["decision"] = Value::String("player-decision-request.v2".into());
    let parsed: ReplayManifestV7 = serde_json::from_value(manifest).unwrap();
    assert!(parsed.validate().is_err());

    let mut replay: Value = serde_json::from_slice(REPLAY).unwrap();
    replay["steps"][0]["state_revision_before"] = Value::String("1".into());
    let parsed: AuthoritativeReplayV7 = serde_json::from_value(replay).unwrap();
    assert!(parsed.validate().is_err());

    let mut replay: Value = serde_json::from_slice(REPLAY).unwrap();
    replay["steps"][0]["actor"] = Value::String("9".into());
    let parsed: AuthoritativeReplayV7 = serde_json::from_value(replay).unwrap();
    assert!(parsed.validate().is_err());
}

#[test]
fn phase9_complete_closure_is_required_for_basic_land_v7_codec() {
    let mut manifest: ReplayManifestV7 = serde_json::from_slice(MANIFEST).unwrap();
    manifest
        .semantic_contract
        .rules_manifest
        .capability_closure
        .as_mut()
        .unwrap()
        .retain(|entry| entry.key != "rules/state-based-actions-combat");
    let rules_id = semantic_contract_digest::calculate_rules_contract_id_v1(
        &manifest.semantic_contract.rules_manifest,
    )
    .unwrap();
    manifest.semantic_contract.manifest.rules_contract_id = rules_id;
    let semantic_id = semantic_contract_digest::calculate_semantic_contract_id_v1(
        &manifest.semantic_contract.manifest,
    )
    .unwrap();
    manifest.semantic_contract.semantic_contract_id = semantic_id.clone();
    manifest.execution_identity.semantic_contract_id = semantic_id;
    manifest.initial_identity.execution_identity = manifest.execution_identity.clone();
    recompute_checkpoint_digest(&mut manifest);

    assert_eq!(
        manifest.validate(),
        Err(ReplayValidationError::ReplayStepIdentity)
    );
}

#[test]
fn replay_v7_rejects_both_adr_0055_program_rules_cross_pairs() {
    let baseline: ReplayManifestV7 = serde_json::from_slice(MANIFEST).unwrap();

    let mut magic_with_synthetic = baseline.clone();
    magic_with_synthetic.semantic_contract.rules_manifest = RulesContractManifestV1 {
        rules_authority: RulesAuthorityV1::SyntheticLegacy,
        capability_closure: None,
    };
    let rules_id = semantic_contract_digest::calculate_rules_contract_id_v1(
        &magic_with_synthetic.semantic_contract.rules_manifest,
    )
    .unwrap();
    magic_with_synthetic.semantic_contract.manifest = SemanticContractManifestV1 {
        rules_contract_id: rules_id,
        format_contract_id: None,
        content_contract_id: None,
    };
    magic_with_synthetic.semantic_contract.content_contract = None;
    magic_with_synthetic.schemas.observation_payload_codec =
        "synthetic-m3-observation.v1".to_owned();
    magic_with_synthetic.semantic_contract.semantic_contract_id =
        semantic_contract_digest::calculate_semantic_contract_id_v1(
            &magic_with_synthetic.semantic_contract.manifest,
        )
        .unwrap();
    magic_with_synthetic.execution_identity.semantic_contract_id = magic_with_synthetic
        .semantic_contract
        .semantic_contract_id
        .clone();
    magic_with_synthetic.initial_identity.execution_identity =
        magic_with_synthetic.execution_identity.clone();
    recompute_checkpoint_digest(&mut magic_with_synthetic);
    assert_eq!(
        magic_with_synthetic.validate(),
        Err(ReplayValidationError::SemanticContractMismatch)
    );

    let mut synthetic_with_magic = baseline;
    synthetic_with_magic.execution_identity.program_kind = ExecutionProgramV1::SyntheticRulesCompat;
    synthetic_with_magic.initial_identity.execution_identity =
        synthetic_with_magic.execution_identity.clone();
    recompute_checkpoint_digest(&mut synthetic_with_magic);
    assert_eq!(
        synthetic_with_magic.validate(),
        Err(ReplayValidationError::SemanticContractMismatch)
    );
}

fn recompute_checkpoint_digest(manifest: &mut ReplayManifestV7) {
    manifest.initial_identity.checkpoint_digest =
        checkpoint_digest::calculate_checkpoint_digest_v7(
            &manifest
                .initial_identity
                .full_state_digest
                .as_digest_reference(),
            &manifest.initial_identity.episode_status,
            &manifest.initial_identity.environment_limit_counters,
            &manifest.initial_identity.checkpoint_codec_identity,
            &manifest.initial_identity.execution_identity,
        )
        .unwrap();
}

#[test]
fn structural_step_linkage_rejects_wrong_before_checkpoint_and_final_identity() {
    let baseline: Value = serde_json::from_slice(REPLAY).unwrap();

    let mut wrong_before = baseline.clone();
    wrong_before["steps"][0]["checkpoint_digest_before"] = Value::String("00".repeat(32));
    assert!(
        serde_json::from_value::<AuthoritativeReplayV7>(wrong_before)
            .unwrap()
            .validate()
            .is_err()
    );

    let mut wrong_final = baseline;
    wrong_final["final_identity"]["full_state_digest"] = Value::String("11".repeat(32));
    assert!(serde_json::from_value::<AuthoritativeReplayV7>(wrong_final)
        .unwrap()
        .validate()
        .is_err());
}

#[test]
fn detached_recorder_builder_accepts_supplied_structural_step_without_execution_claim() {
    let replay: AuthoritativeReplayV7 = serde_json::from_slice(REPLAY).expect("typed replay");
    let mut recorder = ReplayRecorderV7::new(replay.manifest.clone()).expect("valid manifest");
    recorder
        .append(replay.steps[0].clone())
        .expect("valid supplied step");
    assert_eq!(recorder.export().unwrap(), replay);
}
