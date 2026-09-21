//! Task 2 contract tests: canonical rules/semantic contract digest machinery
//! (spec §7–§9, §19.1/§19.2).
//!
//! The KAT vectors live in the shared fixture
//! `persistence/golden/semantic-contract-kat.v1.json` — the single source of
//! truth read byte-identically by the Rust and Python suites. This suite also
//! proves the plan-mandated negative evidence: schema/domain identity defects
//! reject at the envelope boundary, and malformed child digest lengths reject
//! at the typed-ID boundary.

use std::collections::BTreeMap;

use mtgml_model::{
    CapabilityRequirementV1, ContentContractIdV1, FormatContractIdV1, RulesAuthorityV1,
    RulesContractIdV1, RulesContractManifestV1, SemanticContractIdV1, SemanticContractManifestV1,
};
use mtgml_persistence::cbor;
use mtgml_persistence::envelope;
use mtgml_persistence::semantic_contract_digest::{
    calculate_rules_contract_id_v1, calculate_semantic_contract_id_v1, RULES_CONTRACT_DOMAIN,
    RULES_CONTRACT_INPUT_SCHEMA,
};

fn synthetic_rules_manifest() -> RulesContractManifestV1 {
    RulesContractManifestV1 {
        rules_authority: RulesAuthorityV1::SyntheticLegacy,
        capability_closure: None,
    }
}

fn minimal_comprehensive_manifest() -> RulesContractManifestV1 {
    RulesContractManifestV1 {
        rules_authority: RulesAuthorityV1::ComprehensiveRules {
            snapshot_id: "CR-2026-09-19".to_owned(),
        },
        capability_closure: Some(vec![CapabilityRequirementV1 {
            key: "rules/synthetic-transition".to_owned(),
            version: "1.0.0".to_owned(),
        }]),
    }
}

fn kat_fixture_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../persistence/golden/semantic-contract-kat.v1.json")
}

fn kat_vectors() -> Vec<serde_json::Value> {
    let raw = std::fs::read(kat_fixture_path()).unwrap();
    let document: serde_json::Value = serde_json::from_slice(&raw).unwrap();
    document["vectors"].as_array().cloned().unwrap()
}

fn rules_manifest_from(vector: &serde_json::Value) -> RulesContractManifestV1 {
    let authority = &vector["rules_authority"];
    let rules_authority = if authority["variant"] == "synthetic_legacy" {
        RulesAuthorityV1::SyntheticLegacy
    } else {
        RulesAuthorityV1::ComprehensiveRules {
            snapshot_id: authority["snapshot_id"].as_str().unwrap().to_owned(),
        }
    };
    let capability_closure = vector["capability_closure"].as_array().map(|entries| {
        entries
            .iter()
            .map(|entry| CapabilityRequirementV1 {
                key: entry["key"].as_str().unwrap().to_owned(),
                version: entry["version"].as_str().unwrap().to_owned(),
            })
            .collect()
    });
    RulesContractManifestV1 {
        rules_authority,
        capability_closure,
    }
}

#[test]
fn shared_kat_fixture_vectors_reproduce_frozen_ids() {
    let vectors = kat_vectors();
    assert!(vectors.len() >= 5, "shared KAT fixture regressed");

    let mut rules_ids: BTreeMap<String, RulesContractIdV1> = BTreeMap::new();
    for vector in &vectors {
        let case = vector["case"].as_str().unwrap();
        match vector["manifest_kind"].as_str().unwrap() {
            "rules" => {
                let id = calculate_rules_contract_id_v1(&rules_manifest_from(vector)).unwrap();
                assert_eq!(
                    id.as_str(),
                    vector["rules_contract_id"].as_str().unwrap(),
                    "KAT case {case} drifted"
                );
                rules_ids.insert(case.to_owned(), id);
            }
            "semantic" => {}
            other => panic!("unknown manifest_kind {other}"),
        }
    }

    for vector in &vectors {
        let case = vector["case"].as_str().unwrap();
        if vector["manifest_kind"].as_str().unwrap() != "semantic" {
            continue;
        }
        let rules_contract_id = match vector["rules_contract_id_ref"].as_str() {
            Some(reference) => rules_ids[reference].clone(),
            None => {
                RulesContractIdV1::parse(vector["rules_contract_id"].as_str().unwrap()).unwrap()
            }
        };
        let format_contract_id = vector["format_contract_id"]
            .as_str()
            .map(|hex| FormatContractIdV1::parse(hex).unwrap());
        let content_contract_id = vector["content_contract_id"]
            .as_str()
            .map(|hex| ContentContractIdV1::parse(hex).unwrap());
        let id = calculate_semantic_contract_id_v1(&SemanticContractManifestV1 {
            rules_contract_id,
            format_contract_id,
            content_contract_id,
        })
        .unwrap();
        assert_eq!(
            id.as_str(),
            vector["semantic_contract_id"].as_str().unwrap(),
            "KAT case {case} drifted"
        );
    }
}

#[test]
fn distinct_manifests_produce_distinct_rule_ids() {
    let synthetic = calculate_rules_contract_id_v1(&synthetic_rules_manifest()).unwrap();
    let comprehensive = calculate_rules_contract_id_v1(&minimal_comprehensive_manifest()).unwrap();
    assert_ne!(synthetic.as_str(), comprehensive.as_str());

    let other_snapshot = RulesContractManifestV1 {
        rules_authority: RulesAuthorityV1::ComprehensiveRules {
            snapshot_id: "CR-OTHER".to_owned(),
        },
        capability_closure: Some(vec![CapabilityRequirementV1 {
            key: "rules/synthetic-transition".to_owned(),
            version: "1.0.0".to_owned(),
        }]),
    };
    assert_ne!(
        comprehensive.as_str(),
        calculate_rules_contract_id_v1(&other_snapshot)
            .unwrap()
            .as_str()
    );
}

#[test]
fn invalid_manifests_fail_closed_before_hashing() {
    let unsorted = RulesContractManifestV1 {
        rules_authority: RulesAuthorityV1::ComprehensiveRules {
            snapshot_id: "CR-2026-09-19".to_owned(),
        },
        capability_closure: Some(vec![
            CapabilityRequirementV1 {
                key: "rules/synthetic-transition".to_owned(),
                version: "1.0.0".to_owned(),
            },
            CapabilityRequirementV1 {
                key: "mechanic/lifelink".to_owned(),
                version: "1.0.0".to_owned(),
            },
        ]),
    };
    assert!(calculate_rules_contract_id_v1(&unsorted).is_err());

    let duplicate = RulesContractManifestV1 {
        rules_authority: RulesAuthorityV1::ComprehensiveRules {
            snapshot_id: "CR-2026-09-19".to_owned(),
        },
        capability_closure: Some(vec![
            CapabilityRequirementV1 {
                key: "rules/synthetic-transition".to_owned(),
                version: "1.0.0".to_owned(),
            },
            CapabilityRequirementV1 {
                key: "rules/synthetic-transition".to_owned(),
                version: "2.0.0".to_owned(),
            },
        ]),
    };
    assert!(calculate_rules_contract_id_v1(&duplicate).is_err());

    let synthetic_with_closure = RulesContractManifestV1 {
        rules_authority: RulesAuthorityV1::SyntheticLegacy,
        capability_closure: Some(vec![CapabilityRequirementV1 {
            key: "rules/synthetic-transition".to_owned(),
            version: "1.0.0".to_owned(),
        }]),
    };
    assert!(calculate_rules_contract_id_v1(&synthetic_with_closure).is_err());
}

#[test]
fn rule_ids_and_semantic_ids_are_distinct_typed_domains() {
    let rules = calculate_rules_contract_id_v1(&synthetic_rules_manifest()).unwrap();
    let semantic = calculate_semantic_contract_id_v1(&SemanticContractManifestV1 {
        rules_contract_id: rules.clone(),
        format_contract_id: None,
        content_contract_id: None,
    })
    .unwrap();
    assert_eq!(RulesContractIdV1::DOMAIN, "mtgml.rules-contract.v1");
    assert_eq!(SemanticContractIdV1::DOMAIN, "mtgml.semantic-contract.v1");
    assert_ne!(rules.as_str(), semantic.as_str());
}

#[test]
fn malformed_child_digest_length_rejects_at_the_typed_boundary() {
    // A child contract ID that is not exactly 64 lowercase hex characters is
    // unrepresentable as a typed value; parse and serde both fail closed.
    for bad in [
        "5a".repeat(31),
        "5a".repeat(33),
        "5A".repeat(32),
        String::new(),
    ] {
        assert!(
            RulesContractIdV1::parse(bad.clone()).is_err(),
            "malformed rules child id must reject: {bad}"
        );
        assert!(
            FormatContractIdV1::parse(bad.clone()).is_err(),
            "malformed format child id must reject: {bad}"
        );
        assert!(
            ContentContractIdV1::parse(bad.clone()).is_err(),
            "malformed content child id must reject: {bad}"
        );
    }
    let value = serde_json::json!({
        "rules_contract_id": "5a".repeat(31),
        "format_contract_id": null,
        "content_contract_id": null
    });
    assert!(serde_json::from_value::<SemanticContractManifestV1>(value).is_err());
}

#[test]
fn schema_domain_disagreement_is_detectable_and_bound_to_identity() {
    // DEFERRED REJECTION NOTE (accepted Option-A disposition):
    // Spec §8 and docs/STATE_HASHING.md:317 require that a *decoder* rejects
    // a payload whose leading schema/domain fields disagree with the envelope
    // identity. Task 2 ships only the writer/calculator paths
    // (calculate_rules_contract_id_v1 / calculate_semantic_contract_id_v1);
    // no rules/semantic contract decode surface exists yet, so this suite can
    // honestly prove only the detectability/content-binding preconditions:
    //   1. agreement is observable at the envelope decode boundary;
    //   2. disagreement is observable and yields a DIFFERENT identity;
    //   3. content cannot be reinterpreted across domains (identity change).
    // Ownership: OWNER = the first actual rules/semantic contract envelope
    // decode path. CURRENT_V5_PLAN_OWNER = NONE — the merged V5 plan contains
    // no task that introduces such a decoder (Task 8 semantic admission works
    // on already-constructed typed manifests, not on canonical-CBOR contract
    // envelopes). The obligation is tracked durably in the Task-2 section of
    // docs/superpowers/plans/2026-09-19-v5-execution-identity-implementation.md.
    // Evidence status: SCHEMA_DOMAIN_DISAGREEMENT_DETECTABLE = PASS,
    // SCHEMA_DOMAIN_DISAGREEMENT_REJECTED = DEFERRED / NOT_RUN (not claimed here).
    // Positive control: the canonical rules payload inside its canonical
    // envelope decodes with agreeing schema/domain identity.
    let payload_value = cbor::Value::Array(vec![
        cbor::Value::Text(RULES_CONTRACT_INPUT_SCHEMA.to_owned()),
        cbor::Value::Text(RULES_CONTRACT_DOMAIN.to_owned()),
        cbor::Value::Text("diagnostic-agreement-probe".to_owned()),
    ]);
    let payload = cbor::encode_canonical(&payload_value).unwrap();
    let correct_envelope =
        envelope::encode_envelope(RULES_CONTRACT_DOMAIN, RULES_CONTRACT_INPUT_SCHEMA, &payload)
            .unwrap();
    let (reference, decoded_payload) = envelope::decode_envelope(&correct_envelope).unwrap();
    assert_eq!(reference.semantic_domain, RULES_CONTRACT_DOMAIN);
    assert_eq!(reference.input_schema_id, RULES_CONTRACT_INPUT_SCHEMA);
    assert_eq!(decoded_payload, payload);

    // Content binding: the same payload under the WRONG (semantic) envelope
    // schema/domain labels is a different artifact with a different identity —
    // it can never be mistaken for the rules contract identity. This proves
    // DISAGREEMENT_IS_DETECTABLE and IDENTITY_CHANGES; it deliberately does
    // NOT claim DISAGREEMENT_IS_REJECTED (no decode surface exists in Task 2).
    let disagreeing_envelope = envelope::encode_envelope(
        mtgml_persistence::semantic_contract_digest::SEMANTIC_CONTRACT_DOMAIN,
        mtgml_persistence::semantic_contract_digest::SEMANTIC_CONTRACT_INPUT_SCHEMA,
        &payload,
    )
    .unwrap();
    let (wrong_reference, _) = envelope::decode_envelope(&disagreeing_envelope).unwrap();
    assert_ne!(wrong_reference.semantic_domain, RULES_CONTRACT_DOMAIN);
    assert_ne!(wrong_reference.input_schema_id, RULES_CONTRACT_INPUT_SCHEMA);
    assert_ne!(
        RulesContractIdV1::from_digest_bytes(envelope::hash_envelope(&disagreeing_envelope)),
        calculate_rules_contract_id_v1(&synthetic_rules_manifest()).unwrap()
    );

    // Envelope-level identity defect (control): a corrupted (empty) domain
    // frame rejects the whole envelope at the decode boundary. This is
    // malformed envelope framing, NOT the payload-vs-envelope disagreement
    // case deferred above.
    let mut corrupted = correct_envelope.clone();
    let domain_length_offset =
        envelope::DIGEST_ENVELOPE_ID.len() + 1 + 8 + envelope::SHA256_ID.len();
    corrupted[domain_length_offset..domain_length_offset + 8].copy_from_slice(&0u64.to_be_bytes());
    assert!(envelope::decode_envelope(&corrupted).is_err());
}
