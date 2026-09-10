use super::{authority, cbor, checkpoint_digest, envelope, PersistenceDecodeErrorV1};
use authority::{
    canonical_identity_input, AcceptanceEvidenceRefV1, AcceptanceSubjectKind,
    AcceptanceSubjectKindV4, AcceptanceSubjectPayloadV1, AcceptanceV1, AuthorityIdentityKind,
    EvidenceLocatorV1, ParticipantRoleBridgeEntryV1, ParticipantRoleBridgeV1,
    ReviewAcceptanceEventInputV1, ReviewAcceptanceEventLeafV1, ReviewAuthoritySourceBindingV4,
    ReviewEventRefV1, ReviewEventRefV4, ReviewMode, ReviewerRoleBindingV1, ReviewerRosterRefV1,
    SourceBindingDigestV1,
};
use mtgml_model::{CheckpointCodecIdentity, EnvironmentLimitCounters, EpisodeStatus};

fn json_value_to_cbor(value: &serde_json::Value) -> cbor::Value {
    match value {
        serde_json::Value::Null => cbor::Value::Null,
        serde_json::Value::Bool(value) => cbor::Value::Bool(*value),
        serde_json::Value::Number(value) => {
            if let Some(value) = value.as_u64() {
                cbor::Value::Unsigned(value)
            } else if let Some(value) = value.as_i64() {
                cbor::Value::Signed(value)
            } else {
                panic!("matrix number is outside the supported integer range")
            }
        }
        serde_json::Value::String(value) => cbor::Value::Text(value.clone()),
        serde_json::Value::Array(values) => {
            cbor::Value::Array(values.iter().map(json_value_to_cbor).collect())
        }
        serde_json::Value::Object(_) => panic!("matrix values must not be objects"),
    }
}

#[test]
fn authority_relation_identity_matches_cross_language_known_answer() {
    let identity = authority::AuthorityIdentityV1::compute(
        AuthorityIdentityKind::RelationTheorem,
        cbor::Value::Array(vec![
            cbor::Value::Text("manafold.m2.5.c.relation-proof-input.v1".to_owned()),
            cbor::Value::Text("model".to_owned()),
            cbor::Value::Text("positive_interaction".to_owned()),
            cbor::Value::Text("unary".to_owned()),
            cbor::Value::Text("declared_card_trigger".to_owned()),
            cbor::Value::Text("none".to_owned()),
            cbor::Value::Text("not_applicable".to_owned()),
            cbor::Value::Array(vec![cbor::Value::Array(vec![
                cbor::Value::Unsigned(0),
                cbor::Value::Text("ordered_participant".to_owned()),
                cbor::Value::Text("card".to_owned()),
                cbor::Value::Text("subject-ref".to_owned()),
            ])]),
            cbor::Value::Array(vec![]),
            cbor::Value::Array(vec![
                cbor::Value::Text("positive_interaction".to_owned()),
                cbor::Value::Array(vec![
                    cbor::Value::Array(vec![cbor::Value::Array(vec![
                        cbor::Value::Unsigned(0),
                        cbor::Value::Unsigned(0),
                        cbor::Value::Text("reads".to_owned()),
                        cbor::Value::Array(vec![]),
                        cbor::Value::Null,
                        cbor::Value::Null,
                        cbor::Value::Array(vec![]),
                    ])]),
                    cbor::Value::Array(vec![]),
                    cbor::Value::Null,
                ]),
            ]),
            cbor::Value::Array(vec![]),
            cbor::Value::Array(vec![]),
        ]),
    )
    .unwrap();

    assert_eq!(
        identity.as_text(),
        "rp.v1/06dd852fa6a19b5e86d819955ee17cbfaef25d2efa563e1ee67db1368093fddc"
    );
    assert_eq!(
        identity.semantic_domain(),
        "manafold.m2.5.c.relation-proof.v1"
    );
    assert!(canonical_identity_input(
        AuthorityIdentityKind::RelationTheorem,
        cbor::Value::Array(vec![cbor::Value::Text("wrong-schema".to_owned())]),
    )
    .is_err());
    assert_eq!(
        identity.input_schema_id(),
        "manafold.m2.5.c.relation-proof-input.v1"
    );
    let invalid_relation = cbor::Value::Array(vec![
        cbor::Value::Text("manafold.m2.5.c.relation-proof-input.v1".to_owned()),
        cbor::Value::Text("model".to_owned()),
        cbor::Value::Text("positive_interaction".to_owned()),
        cbor::Value::Text("unary".to_owned()),
        cbor::Value::Text("reviewed_relation".to_owned()),
        cbor::Value::Text("directional".to_owned()),
        cbor::Value::Text("same_subject".to_owned()),
        cbor::Value::Array(vec![]),
        cbor::Value::Array(vec![]),
        cbor::Value::Array(vec![]),
        cbor::Value::Array(vec![]),
        cbor::Value::Array(vec![]),
    ]);
    assert!(authority::AuthorityIdentityV1::compute(
        AuthorityIdentityKind::RelationTheorem,
        invalid_relation,
    )
    .is_err());
    assert!(authority::AuthorityIdentityV1::compute(
        AuthorityIdentityKind::AcceptanceSubject,
        cbor::Value::Array(vec![
            cbor::Value::Text("manafold.m2.5.c.acceptance-subject-payload-input.v1".to_owned()),
            cbor::Value::Text("relation_theorem_record".to_owned()),
            cbor::Value::Array(vec![]),
        ]),
    )
    .is_err());
}

#[test]
fn authority_source_binding_has_fixed_cbor_preimage() {
    let binding = SourceBindingDigestV1::new(
        "declared_model",
        "sources/m2_5/closures/C/declared_interaction_model.v2.json",
        Some("manafold.m2.5.c.declared-interaction-model.v2"),
        [0u8; 32],
    )
    .unwrap();

    assert_eq!(
        binding.to_cbor(),
        cbor::Value::Array(vec![
            cbor::Value::Text("declared_model".to_owned()),
            cbor::Value::Text(
                "sources/m2_5/closures/C/declared_interaction_model.v2.json".to_owned()
            ),
            cbor::Value::Text("manafold.m2.5.c.declared-interaction-model.v2".to_owned()),
            cbor::Value::Bytes(vec![0u8; 32]),
        ])
    );
    assert!(SourceBindingDigestV1::new(
        "declared_model",
        "derived/Pair_Interaction_Census_REV3.csv",
        None,
        [0u8; 32],
    )
    .is_err());
    assert!(AcceptanceEvidenceRefV1::new(
        "docs/review/authority.md",
        [0u8; 32],
        EvidenceLocatorV1::JsonPointer("/review~2".to_owned()),
    )
    .is_err());
}

#[test]
fn participant_role_bridge_matches_shared_golden_matrix() {
    let matrix: serde_json::Value = serde_json::from_str(include_str!(
        "../../../conformance/fixtures/authority/participant_role_bridge_golden_matrix.v1.json"
    ))
    .unwrap();
    for case in matrix["valid"].as_array().unwrap() {
        let entries = case["entries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| {
                ParticipantRoleBridgeEntryV1::new(
                    entry["position"].as_u64().unwrap() as u32,
                    entry["participant_kind"].as_str().unwrap(),
                    entry["semantic_ref"].as_str().unwrap(),
                    entry["historical_source_role"].as_str().unwrap(),
                    entry["reviewed_role"].as_str().unwrap(),
                )
                .unwrap()
            })
            .collect();
        let bridge = ParticipantRoleBridgeV1::new(entries).unwrap();
        assert_eq!(bridge.to_cbor(), json_value_to_cbor(&case["cbor"]));
        assert_eq!(bridge.to_wire(), case["wire"]);
    }
}

#[test]
fn participant_role_bridge_negative_matrix_fails_closed() {
    let matrix: serde_json::Value = serde_json::from_str(include_str!(
        "../../../conformance/fixtures/authority/participant_role_bridge_negative_matrix.v1.json"
    ))
    .unwrap();
    for case in matrix["negative"].as_array().unwrap() {
        assert!(
            ParticipantRoleBridgeV1::from_cbor(&json_value_to_cbor(&case["cbor"])).is_err(),
            "negative bridge case was accepted: {}",
            case["name"].as_str().unwrap()
        );
    }
}

#[test]
fn v4_acceptance_contract_has_closed_subject_and_projection_surfaces() {
    assert_eq!(
        AuthorityIdentityKind::AcceptanceSubjectV4.prefix(),
        "asp.v4/"
    );
    assert_eq!(
        AuthorityIdentityKind::ReviewAcceptanceEventV4.prefix(),
        "ae.v4/"
    );
    assert_eq!(
        AcceptanceSubjectKindV4::RelationApplicationV2Record.as_str(),
        "relation_application_v2_record"
    );
    let binding = SourceBindingDigestV1::new(
        "rev3_source",
        "derived/Pair_Interaction_Census_REV3.csv",
        None,
        [0u8; 32],
    )
    .unwrap();
    let projected = authority::v1_dependency_source_binding_to_v4(&binding).unwrap();
    assert_eq!(projected.artifact_role, "rev3_candidate_census");
    assert_eq!(projected.path, binding.path);
    let _ = ReviewAuthoritySourceBindingV4::new(
        "declared_model",
        "sources/m2_5/closures/C/declared_interaction_model.v2.json",
        Some("manafold.m2.5.c.declared-interaction-model.v2"),
        [0u8; 32],
    )
    .unwrap();
    let _ = ReviewEventRefV4::new(
        format!(
            "sources/m2_5/authorities/review_acceptance_events/v4/{}.json",
            "0".repeat(64)
        ),
        [0u8; 32],
        format!("ae.v4/{}", "0".repeat(64)),
    )
    .unwrap();
}

#[test]
fn authority_acceptance_event_identity_matches_cross_language_known_answer() {
    let subject = AcceptanceSubjectPayloadV1::new(
        AcceptanceSubjectKind::RelationTheoremRecord,
        cbor::Value::Array(vec![
            cbor::Value::Bytes(vec![0u8; 32]),
            cbor::Value::Array(vec![cbor::Value::Array(vec![
                cbor::Value::Text("model".to_owned()),
                cbor::Value::Text("sources/model.json".to_owned()),
                cbor::Value::Array(vec![
                    cbor::Value::Text("whole_artifact".to_owned()),
                    cbor::Value::Null,
                ]),
                cbor::Value::Bytes(vec![0u8; 32]),
            ])]),
            cbor::Value::Text("fixture rationale".to_owned()),
        ]),
    )
    .unwrap();
    assert!(subject.identity().unwrap().as_text().starts_with("asp.v1/"));

    let roster_ref = ReviewerRosterRefV1::new(
        format!(
            "sources/m2_5/authorities/reviewer_rosters/v1/{}.json",
            "00".repeat(32)
        ),
        authority::REVIEWER_ROSTER_SCHEMA_V1,
        [0u8; 32],
    )
    .unwrap();
    let reviewer = ReviewerRoleBindingV1::new(
        "alice",
        vec![
            "architecture_maintainer".to_owned(),
            "rules_authority_maintainer".to_owned(),
        ],
    )
    .unwrap();
    let source = SourceBindingDigestV1::new(
        "declared_model",
        "sources/m2_5/closures/C/declared_interaction_model.v2.json",
        Some("manafold.m2.5.c.declared-interaction-model.v2"),
        [0u8; 32],
    )
    .unwrap();
    let roster_source = SourceBindingDigestV1::new(
        "reviewer_roster_leaf",
        roster_ref.path.clone(),
        Some(authority::REVIEWER_ROSTER_SCHEMA_V1),
        [0u8; 32],
    )
    .unwrap();
    let review_evidence = AcceptanceEvidenceRefV1::new(
        "docs/review/authority.md",
        [0u8; 32],
        EvidenceLocatorV1::WholeArtifact,
    )
    .unwrap();
    let event = ReviewAcceptanceEventInputV1::new(
        AcceptanceSubjectKind::RelationTheoremRecord,
        [0u8; 32],
        roster_ref,
        vec![reviewer],
        ReviewMode::SoloSeparateSelfReview,
        vec![source, roster_source],
        vec![review_evidence],
    )
    .unwrap();

    let leaf = ReviewAcceptanceEventLeafV1::from_input(event.clone()).unwrap();
    let wire = leaf.to_wire().unwrap();
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../conformance/fixtures/authority/review_acceptance_event.v1.json"
    ))
    .unwrap();
    assert_eq!(wire, fixture);
    assert_eq!(
        wire["event_id"],
        serde_json::json!("ae.v1/605cc0fcb6020f5066896ddc238bc7594e39a7bf731c33a72d88ab7a7acc8013")
    );
    assert_eq!(leaf.to_cbor().unwrap(), event.semantic_input().unwrap());
}

#[test]
fn authority_acceptance_binding_has_fixed_cbor_preimage() {
    let event_ref = ReviewEventRefV1::new(
        format!(
            "sources/m2_5/authorities/review_acceptance_events/v1/{}.json",
            "00".repeat(32)
        ),
        [0u8; 32],
        format!("ae.v1/{}", "00".repeat(32)),
    )
    .unwrap();
    let acceptance = AcceptanceV1 {
        review_event_ref: event_ref.clone(),
    };
    assert_eq!(
        acceptance.to_cbor(),
        cbor::Value::Array(vec![
            cbor::Value::Text("human_accepted".to_owned()),
            event_ref.to_cbor(),
        ])
    );
}

#[test]
fn all_authority_identity_kinds_match_the_shared_golden_matrix() {
    let matrix: serde_json::Value = serde_json::from_str(include_str!(
        "../../../conformance/fixtures/authority/identity_golden_matrix.v1.json"
    ))
    .unwrap();
    let identities = matrix["identities"].as_array().unwrap();
    assert_eq!(identities.len(), 19);

    for entry in identities {
        let kind = authority_kind(entry["kind"].as_str().unwrap());
        let payload_bytes = decode_hex(entry["payload_cbor_hex"].as_str().unwrap());
        let payload = cbor::decode_canonical(&payload_bytes).unwrap();
        assert_eq!(
            payload_array_len(&payload),
            entry["arity"].as_u64().unwrap() as usize
        );
        let identity = authority::AuthorityIdentityV1::compute(kind, payload).unwrap();
        assert_eq!(identity.as_text(), entry["identity"].as_str().unwrap());
        assert_eq!(
            identity.semantic_domain(),
            entry["semantic_domain"].as_str().unwrap()
        );
        assert_eq!(
            identity.input_schema_id(),
            entry["input_schema_id"].as_str().unwrap()
        );
        assert_eq!(identity.kind().prefix(), entry["prefix"].as_str().unwrap());
    }
}

#[test]
fn relation_application_v2_identity_vectors_match_python_contract() {
    let matrix: serde_json::Value = serde_json::from_str(include_str!(
        "../../../conformance/fixtures/authority/relation_application_v2_identity_golden_matrix.v1.json"
    ))
    .unwrap();
    assert_eq!(
        matrix["schema_version"],
        serde_json::json!("relation-application-v2-identity-golden-matrix.v1")
    );
    for entry in matrix["identities"].as_array().unwrap() {
        let kind = authority_kind(entry["kind"].as_str().unwrap());
        let payload =
            cbor::decode_canonical(&decode_hex(entry["payload_cbor_hex"].as_str().unwrap()))
                .unwrap();
        let identity = authority::AuthorityIdentityV1::compute(kind, payload).unwrap();
        assert_eq!(identity.as_text(), entry["identity"].as_str().unwrap());
        assert_eq!(
            identity.digest_bytes(),
            decode_hex(entry["digest_hex"].as_str().unwrap()).as_slice()
        );
        assert_eq!(
            cbor::encode_canonical(&identity.to_cbor()).unwrap(),
            decode_hex(entry["identity_cbor_hex"].as_str().unwrap())
        );
    }
}

#[test]
fn relation_application_v2_supersession_identity_vectors_match_python_contract() {
    let matrix: serde_json::Value = serde_json::from_str(include_str!(
        "../../../conformance/fixtures/authority/relation_application_v2_supersession_identity_golden_matrix.v1.json"
    ))
    .unwrap();
    for entry in matrix["identities"].as_array().unwrap() {
        let kind = authority_kind(entry["kind"].as_str().unwrap());
        let payload =
            cbor::decode_canonical(&decode_hex(entry["payload_cbor_hex"].as_str().unwrap()))
                .unwrap();
        let identity = authority::AuthorityIdentityV1::compute(kind, payload).unwrap();
        assert_eq!(identity.as_text(), entry["identity"].as_str().unwrap());
        assert_eq!(
            cbor::encode_canonical(&identity.to_cbor()).unwrap(),
            decode_hex(entry["identity_cbor_hex"].as_str().unwrap())
        );
    }
}

#[test]
fn relation_application_v2_member_proof_wire_goldens_match_python_contract() {
    let matrix: serde_json::Value = serde_json::from_str(include_str!(
        "../../../conformance/fixtures/authority/relation_application_v2_wire_golden.v1.json"
    ))
    .unwrap();
    let evidence = || {
        cbor::Value::Array(vec![
            cbor::Value::Text("model".to_owned()),
            cbor::Value::Text("sources/model.json".to_owned()),
            cbor::Value::Array(vec![
                cbor::Value::Text("whole_artifact".to_owned()),
                cbor::Value::Null,
            ]),
            cbor::Value::Bytes(vec![0x65; 32]),
        ])
    };
    let positive_interaction = cbor::Value::Array(vec![
        cbor::Value::Text("positive_interaction".to_owned()),
        cbor::Value::Array(vec![
            cbor::Value::Array(vec![cbor::Value::Unsigned(0)]),
            cbor::Value::Null,
        ]),
    ]);
    let channels = [
        "participant_boundary",
        "event_or_effect_causality",
        "target_or_choice",
        "zone_or_object_identity",
        "control_or_ownership",
        "replacement_or_layer",
        "trigger_or_lki",
        "information_or_visibility",
        "ordering_or_temporal",
        "decision_actor",
        "format_and_declared_scope",
    ];
    let coverages = channels
        .iter()
        .map(|channel| {
            cbor::Value::Array(vec![
                cbor::Value::Text((*channel).to_owned()),
                cbor::Value::Text("separated".to_owned()),
                cbor::Value::Array(vec![cbor::Value::Array(vec![
                    cbor::Value::Text("b2_boundary".to_owned()),
                    cbor::Value::Array(vec![
                        cbor::Value::Text("family".to_owned()),
                        cbor::Value::Text("active".to_owned()),
                        cbor::Value::Text("primary".to_owned()),
                        cbor::Value::Text("definition".to_owned()),
                    ]),
                ])]),
                cbor::Value::Array(vec![evidence()]),
                cbor::Value::Array(vec![]),
                cbor::Value::Text("covered".to_owned()),
            ])
        })
        .collect::<Vec<_>>();
    let positive_separation = cbor::Value::Array(vec![
        cbor::Value::Text("positive_separation".to_owned()),
        cbor::Value::Array(vec![cbor::Value::Array(coverages)]),
    ]);
    let model_bound_scope = cbor::Value::Array(vec![
        cbor::Value::Text("model_bound_scope".to_owned()),
        cbor::Value::Array(vec![cbor::Value::Array(vec![
            cbor::Value::Text("declared-interaction-model.v2".to_owned()),
            cbor::Value::Text("2".to_owned()),
            cbor::Value::Array(vec![
                cbor::Value::Text(
                    "sources/m2_5/closures/C/declared_interaction_model.v2.json".to_owned(),
                ),
                cbor::Value::Text("manafold.m2.5.c.declared-interaction-model.v2".to_owned()),
                cbor::Value::Bytes(vec![0x6d; 32]),
                cbor::Value::Array(vec![
                    cbor::Value::Text("coverage_scope".to_owned()),
                    cbor::Value::Null,
                ]),
            ]),
            cbor::Value::Text("undeclared_relation_shape".to_owned()),
            cbor::Value::Array(vec![
                cbor::Value::Text("cross_deck".to_owned()),
                cbor::Value::Text("directional_binary".to_owned()),
                cbor::Value::Text("binary".to_owned()),
                cbor::Value::Text("directed".to_owned()),
                cbor::Value::Unsigned(2),
            ]),
            cbor::Value::Array(vec![evidence()]),
        ])]),
    ]);
    for (kind, proof) in [
        ("positive_interaction", positive_interaction),
        ("positive_separation", positive_separation),
        ("model_bound_scope", model_bound_scope),
    ] {
        assert_eq!(
            authority::member_proof_attestation_to_wire(&proof),
            matrix["proof_wire_goldens"][kind]
        );
    }
}

#[test]
fn authority_contract_negative_matrix_rejects_every_case() {
    let matrix: serde_json::Value = serde_json::from_str(include_str!(
        "../../../conformance/fixtures/authority/identity_contract_negative_matrix.v1.json"
    ))
    .unwrap();
    let cases = matrix["cases"].as_array().unwrap();
    assert!(cases.len() >= 10);

    for case in cases {
        assert_eq!(case["expected"], serde_json::json!("reject"));
        let kind = authority_kind(case["kind"].as_str().unwrap());
        let payload =
            cbor::decode_canonical(&decode_hex(case["payload_cbor_hex"].as_str().unwrap()))
                .unwrap();
        assert!(
            authority::AuthorityIdentityV1::compute(kind, payload).is_err(),
            "negative authority case was accepted: {}",
            case["case_id"].as_str().unwrap()
        );
    }
}

#[test]
fn authority_contract_matrix_positive_controls_are_accepted() {
    let matrix: serde_json::Value = serde_json::from_str(include_str!(
        "../../../conformance/fixtures/authority/identity_contract_negative_matrix.v1.json"
    ))
    .unwrap();
    let controls = matrix["positive_controls"].as_array().unwrap();
    assert!(!controls.is_empty());

    for control in controls {
        assert_eq!(control["expected"], serde_json::json!("accept"));
        let kind = authority_kind(control["kind"].as_str().unwrap());
        let payload =
            cbor::decode_canonical(&decode_hex(control["payload_cbor_hex"].as_str().unwrap()))
                .unwrap();
        assert!(
            authority::AuthorityIdentityV1::compute(kind, payload).is_ok(),
            "positive authority control was rejected: {}",
            control["control_id"].as_str().unwrap()
        );
    }
}

#[test]
fn context_application_v2_semantic_golden_matrix_matches_python_contract() {
    fn json_to_cbor(value: &serde_json::Value) -> cbor::Value {
        match value {
            serde_json::Value::Null => cbor::Value::Null,
            serde_json::Value::Bool(value) => cbor::Value::Bool(*value),
            serde_json::Value::Number(value) => {
                if let Some(value) = value.as_u64() {
                    cbor::Value::Unsigned(value)
                } else if let Some(value) = value.as_i64() {
                    cbor::Value::Signed(value)
                } else {
                    panic!("matrix number is outside the supported integer range")
                }
            }
            serde_json::Value::String(value) => cbor::Value::Text(value.clone()),
            serde_json::Value::Array(values) => {
                cbor::Value::Array(values.iter().map(json_to_cbor).collect())
            }
            serde_json::Value::Object(_) => panic!("semantic matrix values must not be objects"),
        }
    }

    fn strings(case: &serde_json::Value, field: &str) -> Vec<String> {
        case[field]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap().to_owned())
            .collect()
    }

    fn relation(value: &str) -> authority::ContextBridgeRelationV2 {
        match value {
            "exact_match" => authority::ContextBridgeRelationV2::ExactMatch,
            "reviewed_divergence" => authority::ContextBridgeRelationV2::ReviewedDivergence,
            other => panic!("unknown relation {other}"),
        }
    }

    fn preconditions(
        case: &serde_json::Value,
        field: &str,
        value_field: &str,
    ) -> Vec<authority::ContextPreconditionValueV1> {
        case[field]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| authority::ContextPreconditionValueV1 {
                precondition_id: value["precondition_id"].as_str().unwrap().to_owned(),
                value: json_to_cbor(&value[value_field]),
            })
            .collect()
    }

    let matrix: serde_json::Value = serde_json::from_str(include_str!(
        "../../../conformance/fixtures/authority/context_application_v2_semantic_golden_matrix.v1.json"
    ))
    .unwrap();
    assert_eq!(
        matrix["schema"],
        serde_json::json!("manafold.m2.5.c.context-application-v2-semantic-golden-matrix.v1")
    );

    for case in matrix["cases"].as_array().unwrap() {
        let input = authority::ContextApplicationV2SemanticInput {
            theorem_subject_shape: json_to_cbor(&case["theorem_subject_shape"]),
            member_context_binding: json_to_cbor(&case["member_context_binding"]),
            historical_source_values: strings(case, "historical_source_values"),
            bridge_source_values: strings(case, "bridge_source_values"),
            theorem_context_values: strings(case, "theorem_context_values"),
            bridge_reviewed_values: strings(case, "bridge_reviewed_values"),
            bridge_relations: case["bridge_relations"]
                .as_array()
                .unwrap()
                .iter()
                .map(|value| relation(value.as_str().unwrap()))
                .collect(),
            theorem_temporal_values: strings(case, "theorem_temporal_values"),
            bridge_temporal_values: strings(case, "bridge_temporal_values"),
            theorem_preconditions: preconditions(case, "theorem_preconditions", "payload"),
            member_preconditions: preconditions(case, "member_preconditions", "observed_value"),
        };
        if case["case_id"] == serde_json::json!("exact_match") {
            let subject = match &input.theorem_subject_shape {
                cbor::Value::Array(values) => values,
                other => panic!("expected subject array, got {other:?}"),
            };
            let participants = match &subject[2] {
                cbor::Value::Array(values) => values,
                other => panic!("expected participant array, got {other:?}"),
            };
            let first_position = match &participants[0] {
                cbor::Value::Array(values) => &values[0],
                other => panic!("expected first participant array, got {other:?}"),
            };
            let second_position = match &participants[1] {
                cbor::Value::Array(values) => &values[0],
                other => panic!("expected second participant array, got {other:?}"),
            };
            assert_eq!(first_position, &cbor::Value::Unsigned(0));
            assert_eq!(second_position, &cbor::Value::Unsigned(1));
        }
        let actual = authority::validate_context_application_v2_semantics(&input);
        let expected = &case["expected"];
        if expected["valid"].as_bool().unwrap() {
            assert!(
                actual.is_ok(),
                "case {} failed: {actual:?}",
                case["case_id"]
            );
        } else {
            let error = actual.unwrap_err();
            assert_eq!(
                Some(error.code),
                expected["error_code"].as_str(),
                "case {}",
                case["case_id"]
            );
        }
    }
}

#[test]
fn context_application_v2_identity_vectors_match_shared_matrix() {
    let matrix: serde_json::Value = serde_json::from_str(include_str!(
        "../../../conformance/fixtures/authority/context_application_v2_identity_golden_matrix.v1.json"
    ))
    .unwrap();
    let identities = matrix["identities"].as_array().unwrap();
    assert_eq!(identities.len(), 7);
    for entry in identities {
        let kind = match entry["kind"].as_str().unwrap() {
            "context_application_v2" => AuthorityIdentityKind::ContextApplicationV2,
            "context_application_record_v2" => AuthorityIdentityKind::ContextApplicationRecordV2,
            "context_supersession_v2" => AuthorityIdentityKind::ContextSupersessionV2,
            "context_supersession_record_v2" => AuthorityIdentityKind::ContextSupersessionRecordV2,
            "acceptance_subject_v3" => AuthorityIdentityKind::AcceptanceSubjectV3,
            "review_acceptance_event_v3" => AuthorityIdentityKind::ReviewAcceptanceEventV3,
            other => panic!("unknown context application identity kind: {other}"),
        };
        let payload =
            cbor::decode_canonical(&decode_hex(entry["payload_cbor_hex"].as_str().unwrap()))
                .unwrap();
        let identity = authority::AuthorityIdentityV1::compute(kind, payload).unwrap();
        assert_eq!(identity.as_text(), entry["identity"].as_str().unwrap());
        assert_eq!(
            hex(&identity.digest_bytes()),
            entry["digest_hex"].as_str().unwrap()
        );
        assert_eq!(
            hex(&cbor::encode_canonical(&identity.to_cbor()).unwrap()),
            entry["identity_cbor_hex"].as_str().unwrap()
        );
    }
}

#[test]
fn context_application_v3_identity_vectors_match_python_contract() {
    let matrix: serde_json::Value = serde_json::from_str(include_str!(
        "../../../conformance/fixtures/authority/context_application_v3_identity_golden_matrix.v1.json"
    ))
    .unwrap();
    assert_eq!(
        matrix["schema_version"],
        serde_json::json!("context-application-v3-identity-golden-matrix.v1")
    );
    for entry in matrix["identities"].as_array().unwrap() {
        let kind = match entry["kind"].as_str().unwrap() {
            "context_application_v3" => AuthorityIdentityKind::ContextApplicationV3,
            "context_application_record_v3" => AuthorityIdentityKind::ContextApplicationRecordV3,
            other => panic!("unknown context application V3 identity kind: {other}"),
        };
        let payload =
            cbor::decode_canonical(&decode_hex(entry["payload_cbor_hex"].as_str().unwrap()))
                .unwrap();
        let identity = authority::AuthorityIdentityV1::compute(kind, payload).unwrap();
        assert_eq!(identity.as_text(), entry["identity"].as_str().unwrap());
        assert_eq!(
            cbor::encode_canonical(&identity.to_cbor()).unwrap(),
            decode_hex(entry["identity_cbor_hex"].as_str().unwrap())
        );
    }
}

#[test]
fn candidate_4_identity_surface_matches_python_fixture() {
    let role_fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../conformance/fixtures/authority/candidate_4_role_bridge_golden.v1.json"
    ))
    .unwrap();
    assert_eq!(
        role_fixture["candidate_id"],
        serde_json::json!(
            "CROSS_DECK|P3|cap.mass_destruction|cap.death_trigger|DIRECTIONAL_BINARY"
        )
    );
    assert_eq!(role_fixture["rev3_row_ordinal"], serde_json::json!(6463));
    assert_eq!(
        role_fixture["source_instance_id"],
        serde_json::json!("si.v1/Q1JPU1NfREVDS3xQM3xjYXAubWFzc19kZXN0cnVjdGlvbnxjYXAuZGVhdGhfdHJpZ2dlcnxESVJFQ1RJT05BTF9CSU5BUlk/0")
    );
    let bridge = cbor::decode_canonical(&decode_hex(
        role_fixture["bridge_cbor_hex"].as_str().unwrap(),
    ))
    .unwrap();
    assert_eq!(
        hex(&cbor::encode_canonical(&bridge).unwrap()),
        role_fixture["bridge_cbor_hex"].as_str().unwrap()
    );

    let rpa_fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../conformance/fixtures/authority/candidate_4_rpa_v2_golden.v1.json"
    ))
    .unwrap();
    let context_fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../conformance/fixtures/authority/candidate_4_context_v3_golden.v1.json"
    ))
    .unwrap();
    for (fixture, entries) in [
        (
            rpa_fixture,
            [
                ("rpa_v2", AuthorityIdentityKind::RelationApplicationV2),
                (
                    "rpar_v2",
                    AuthorityIdentityKind::RelationApplicationRecordV2,
                ),
            ],
        ),
        (
            context_fixture,
            [
                ("cpa_v3", AuthorityIdentityKind::ContextApplicationV3),
                ("cpar_v3", AuthorityIdentityKind::ContextApplicationRecordV3),
            ],
        ),
    ] {
        for (field, kind) in entries {
            let text = fixture[field].as_str().unwrap();
            let digest = decode_hex(&text[text.find('/').unwrap() + 1..]);
            let digest: [u8; 32] = digest.try_into().unwrap();
            let identity = authority::AuthorityIdentityV1::from_digest_bytes(kind, digest);
            assert_eq!(identity.as_text(), text);
        }
    }
}

#[test]
fn context_v3_supersession_identity_vectors_match_python_contract() {
    let matrix: serde_json::Value = serde_json::from_str(include_str!(
        "../../../conformance/fixtures/authority/context_application_v3_supersession_identity_golden_matrix.v1.json"
    ))
    .unwrap();
    for entry in matrix["identities"].as_array().unwrap() {
        let kind = match entry["kind"].as_str().unwrap() {
            "context_supersession_v3" => AuthorityIdentityKind::ContextSupersessionV3,
            "context_supersession_record_v3" => AuthorityIdentityKind::ContextSupersessionRecordV3,
            other => panic!("unknown context supersession identity kind: {other}"),
        };
        let payload =
            cbor::decode_canonical(&decode_hex(entry["payload_cbor_hex"].as_str().unwrap()))
                .unwrap();
        let identity = authority::AuthorityIdentityV1::compute(kind, payload).unwrap();
        assert_eq!(identity.as_text(), entry["identity"].as_str().unwrap());
        assert_eq!(
            cbor::encode_canonical(&identity.to_cbor()).unwrap(),
            decode_hex(entry["identity_cbor_hex"].as_str().unwrap())
        );
    }
}

#[test]
fn context_v3_supersession_identities_are_versioned_and_closed() {
    let evidence = cbor::Value::Array(vec![
        cbor::Value::Text("model".to_owned()),
        cbor::Value::Text("sources/model.json".to_owned()),
        cbor::Value::Array(vec![
            cbor::Value::Text("whole_artifact".to_owned()),
            cbor::Value::Null,
        ]),
        cbor::Value::Bytes(vec![b'e'; 32]),
    ]);
    let cps_payload = cbor::Value::Array(vec![
        cbor::Value::Text(
            "manafold.m2.5.c.context-application-v3-supersession-input.v3".to_owned(),
        ),
        cbor::Value::Bytes(vec![b'a'; 32]),
        cbor::Value::Null,
        cbor::Value::Text("context_application_v3_record".to_owned()),
        cbor::Value::Null,
        cbor::Value::Text("authority_revocation".to_owned()),
        cbor::Value::Array(vec![evidence]),
    ]);
    let cps = authority::AuthorityIdentityV1::compute(
        AuthorityIdentityKind::ContextSupersessionV3,
        cps_payload.clone(),
    )
    .unwrap();
    assert_eq!(cps.as_text().len(), 71);
    assert_eq!(cps.kind(), AuthorityIdentityKind::ContextSupersessionV3);
    let event_ref = ReviewEventRefV4::new(
        format!(
            "sources/m2_5/authorities/review_acceptance_events/v4/{}.json",
            "b".repeat(64)
        ),
        [b'b'; 32],
        format!("ae.v4/{}", "b".repeat(64)),
    )
    .unwrap();
    let cpsr = authority::AuthorityIdentityV1::compute(
        AuthorityIdentityKind::ContextSupersessionRecordV3,
        cbor::Value::Array(vec![
            cbor::Value::Text(
                "manafold.m2.5.c.context-application-supersession-record-input.v3".to_owned(),
            ),
            cbor::Value::Bytes(cps.digest_bytes().to_vec()),
            event_ref.to_cbor(),
        ]),
    )
    .unwrap();
    assert_eq!(
        cpsr.kind(),
        AuthorityIdentityKind::ContextSupersessionRecordV3
    );
    assert_eq!(cpsr.as_text().len(), 72);
}

#[test]
fn context_v3_source_bindings_reject_duplicate_role_path_even_with_different_digest() {
    let base = authority::ContextAuthoritySourceBindingV3::new(
        "base_authority_v1",
        "sources/m2_5/authorities/interaction_review_authority.v1.json",
        Some("manafold.m2.5.c.interaction-review-authority.v1"),
        [b'a'; 32],
    )
    .unwrap();
    let candidate = authority::ContextAuthoritySourceBindingV3::new(
        "candidate_universe",
        "sources/m2_5/closures/C/interaction_candidate_universe.v2.json",
        Some("manafold.m2.5.c.interaction-candidate-universe.v2"),
        [b'b'; 32],
    )
    .unwrap();
    let relation = authority::ContextAuthoritySourceBindingV3::new(
        "relation_authority_v2",
        "sources/m2_5/authorities/relation_application_authority/v2/relation_application_authority.v2.json",
        Some("manafold.m2.5.c.relation-application-authority.v2"),
        [b'c'; 32],
    )
    .unwrap();
    let model_a = authority::ContextAuthoritySourceBindingV3::new(
        "declared_model",
        "sources/m2_5/closures/C/declared_interaction_model.v2.json",
        Some("manafold.m2.5.c.declared-interaction-model.v2"),
        [b'm'; 32],
    )
    .unwrap();
    let model_b = authority::ContextAuthoritySourceBindingV3::new(
        "declared_model",
        "sources/m2_5/closures/C/declared_interaction_model.v2.json",
        Some("manafold.m2.5.c.declared-interaction-model.v2"),
        [b'z'; 32],
    )
    .unwrap();
    let mut source_bindings = vec![
        base.clone(),
        candidate.clone(),
        relation.clone(),
        model_a,
        model_b,
    ];
    source_bindings.sort_by_key(|item| cbor::encode_canonical(&item.to_cbor()).unwrap());
    assert!(authority::ContextApplicationAuthorityV3::new(
        base,
        candidate,
        source_bindings,
        relation,
        vec![],
        None,
        vec![],
        vec![],
        vec![],
        vec![],
    )
    .is_err());
}

#[test]
fn context_v3_supersession_rejects_v2_endpoint_kinds() {
    let supersession_id = authority::AuthorityIdentityV1::compute(
        AuthorityIdentityKind::ContextSupersessionV3,
        cbor::Value::Array(vec![
            cbor::Value::Text(
                "manafold.m2.5.c.context-application-v3-supersession-input.v3".to_owned(),
            ),
            cbor::Value::Bytes(vec![b'a'; 32]),
            cbor::Value::Null,
            cbor::Value::Text("context_application_v3_record".to_owned()),
            cbor::Value::Null,
            cbor::Value::Text("authority_revocation".to_owned()),
            cbor::Value::Array(vec![cbor::Value::Array(vec![
                cbor::Value::Text("model".to_owned()),
                cbor::Value::Text("sources/model.json".to_owned()),
                cbor::Value::Array(vec![
                    cbor::Value::Text("whole_artifact".to_owned()),
                    cbor::Value::Null,
                ]),
                cbor::Value::Bytes(vec![b'e'; 32]),
            ])]),
        ]),
    )
    .unwrap();
    let v2_endpoint = authority::AuthorityIdentityV1::from_digest_bytes(
        AuthorityIdentityKind::ContextApplicationRecordV2,
        [b'v'; 32],
    );
    let event_ref = ReviewEventRefV4::new(
        format!(
            "sources/m2_5/authorities/review_acceptance_events/v4/{}.json",
            "b".repeat(64)
        ),
        [b'b'; 32],
        format!("ae.v4/{}", "b".repeat(64)),
    )
    .unwrap();
    let result = authority::ContextApplicationV3SupersessionRecord::from_parts(
        supersession_id,
        v2_endpoint,
        None,
        authority::SupersessionReason::AuthorityRevocation,
        vec![],
        event_ref,
    );
    assert!(matches!(
        result,
        Err(PersistenceDecodeErrorV1::SchemaIdentityMismatch)
    ));
}

#[test]
fn context_application_v2_rust_dtos_emit_the_shared_member_payload() {
    let evidence = authority::EvidenceRefV1::new(
        "model",
        "a",
        authority::EvidenceLocatorV1::WholeArtifact,
        [0; 32],
    )
    .unwrap();
    let context = [
        "zone",
        "visibility",
        "timing",
        "temporal_order",
        "source_affected_relation",
        "control_ownership_relation",
        "replacement_layer_relation",
        "trigger_lki_relation",
        "information_relation",
        "decision_actor_relation",
    ]
    .into_iter()
    .map(|slot| {
        authority::ContextSlotBridgeAttestationV2::new(
            slot,
            "not_applicable",
            "not_applicable",
            authority::ContextBridgeRelationV2::ExactMatch,
            vec![evidence.clone()],
            "x",
        )
        .unwrap()
    })
    .collect();
    let temporal = [
        "trigger_order",
        "dependency_order",
        "duration",
        "replacement_order",
    ]
    .into_iter()
    .map(|slot| {
        authority::TemporalSlotAttestationV2::new(
            slot,
            "not_applicable",
            vec![evidence.clone()],
            "x",
        )
        .unwrap()
    })
    .collect();
    let bridge = authority::ContextMemberBridgeAttestationV2::new(context, temporal).unwrap();
    let invalid_candidate_identity = authority::DigestReferenceV1 {
        envelope_version: envelope::DIGEST_ENVELOPE_ID.to_owned(),
        algorithm_id: envelope::SHA256_ID.to_owned(),
        semantic_domain: "d".to_owned(),
        payload_codec_id: envelope::CANONICAL_CBOR_ID.to_owned(),
        input_schema_id: "i".to_owned(),
        digest_bytes: [0; 32],
    };
    assert!(authority::ContextApplicationMemberV2::new(
        "c",
        invalid_candidate_identity,
        "s",
        cbor::Value::Array(vec![
            cbor::Value::Text("u".to_owned()),
            cbor::Value::Text("manafold.m2.5.c.interaction-candidate-universe.v2".to_owned()),
            cbor::Value::Bytes(vec![0; 32]),
        ]),
        cbor::Value::Array(vec![
            cbor::Value::Text("binary".to_owned()),
            cbor::Value::Text("symmetric".to_owned()),
            cbor::Value::Array(vec![cbor::Value::Array(vec![
                cbor::Value::Unsigned(0),
                cbor::Value::Text("ordered_participant".to_owned()),
                cbor::Value::Text("card".to_owned()),
                cbor::Value::Text("draw".to_owned()),
            ])]),
            cbor::Value::Text("same_host".to_owned()),
        ]),
        cbor::Value::Array(Vec::new()),
        vec![evidence.clone()],
        bridge.clone(),
    )
    .is_err());
    let member = authority::ContextApplicationMemberV2::new(
        "c",
        authority::DigestReferenceV1 {
            envelope_version: envelope::DIGEST_ENVELOPE_ID.to_owned(),
            algorithm_id: envelope::SHA256_ID.to_owned(),
            semantic_domain: "manafold.m2.5.c.candidate-identity.v1".to_owned(),
            payload_codec_id: envelope::CANONICAL_CBOR_ID.to_owned(),
            input_schema_id: "manafold.m2.5.c.candidate-identity-input.v1".to_owned(),
            digest_bytes: [0; 32],
        },
        "s",
        cbor::Value::Array(vec![
            cbor::Value::Text("u".to_owned()),
            cbor::Value::Text("manafold.m2.5.c.interaction-candidate-universe.v2".to_owned()),
            cbor::Value::Bytes(vec![0; 32]),
        ]),
        cbor::Value::Array(vec![
            cbor::Value::Text("binary".to_owned()),
            cbor::Value::Text("symmetric".to_owned()),
            cbor::Value::Array(vec![cbor::Value::Array(vec![
                cbor::Value::Unsigned(0),
                cbor::Value::Text("ordered_participant".to_owned()),
                cbor::Value::Text("card".to_owned()),
                cbor::Value::Text("draw".to_owned()),
            ])]),
            cbor::Value::Text("same_host".to_owned()),
        ]),
        cbor::Value::Array(Vec::new()),
        vec![evidence],
        bridge,
    )
    .unwrap();
    let input = authority::ContextApplicationV2InputV1::new([0; 32], vec![member]).unwrap();
    let matrix: serde_json::Value = serde_json::from_str(include_str!(
        "../../../conformance/fixtures/authority/context_application_v2_identity_golden_matrix.v1.json"
    ))
    .unwrap();
    let entry = matrix["identities"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["kind"] == serde_json::json!("context_application_v2"))
        .unwrap();
    assert_eq!(
        hex(&cbor::encode_canonical(&input.to_cbor()).unwrap()),
        entry["payload_cbor_hex"].as_str().unwrap()
    );
    assert_eq!(
        input.identity().unwrap().as_text(),
        entry["identity"].as_str().unwrap()
    );
}

#[test]
fn context_application_v2_preimage_dtos_cover_all_remaining_families() {
    let matrix: serde_json::Value = serde_json::from_str(include_str!(
        "../../../conformance/fixtures/authority/context_application_v2_identity_golden_matrix.v1.json"
    ))
    .unwrap();
    let entry = |kind: &str| {
        matrix["identities"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["kind"] == serde_json::json!(kind))
            .unwrap()
    };
    let event_ref = authority::ReviewEventRefV3::new(
        "sources/m2_5/authorities/review_acceptance_events/v3/".to_owned()
            + &"00".repeat(32)
            + ".json",
        [0x22; 32],
        "ae.v3/".to_owned() + &"00".repeat(32),
    )
    .unwrap();

    let cpa_digest: [u8; 32] = decode_hex(
        entry("context_application_v2")["digest_hex"]
            .as_str()
            .unwrap(),
    )
    .try_into()
    .unwrap();
    let cpar_input = authority::ContextApplicationV2RecordInputV1 {
        context_application_id_bytes: cpa_digest,
        review_event_ref_v3: event_ref.clone(),
    };
    assert_eq!(
        hex(&cbor::encode_canonical(&cpar_input.semantic_input()).unwrap()),
        entry("context_application_record_v2")["payload_cbor_hex"]
            .as_str()
            .unwrap()
    );

    let evidence = authority::EvidenceRefV1::new(
        "model",
        "a",
        authority::EvidenceLocatorV1::WholeArtifact,
        [0; 32],
    )
    .unwrap();
    let supersession = authority::ContextApplicationV2SupersessionInputV2::new(
        [0; 32],
        Some([1; 32]),
        Some("context_application_v2_record".to_owned()),
        authority::SupersessionReason::SemanticCorrection,
        vec![evidence],
    )
    .unwrap();
    assert_eq!(
        hex(&cbor::encode_canonical(&supersession.semantic_input()).unwrap()),
        entry("context_supersession_v2")["payload_cbor_hex"]
            .as_str()
            .unwrap()
    );
    let cpsr_input = authority::ContextApplicationV2SupersessionRecordInputV1 {
        supersession_id_bytes: supersession.identity().unwrap().digest_bytes(),
        review_event_ref_v3: event_ref.clone(),
    };
    assert_eq!(
        hex(&cbor::encode_canonical(&cpsr_input.semantic_input()).unwrap()),
        entry("context_supersession_record_v2")["payload_cbor_hex"]
            .as_str()
            .unwrap()
    );

    let asp_fields = cbor::decode_canonical(&decode_hex(
        entry("acceptance_subject_v3")["payload_cbor_hex"]
            .as_str()
            .unwrap(),
    ))
    .unwrap();
    let asp_payload = match asp_fields {
        cbor::Value::Array(fields) => fields[2].clone(),
        _ => panic!("acceptance subject vector is not an array"),
    };
    let subject = authority::AcceptanceSubjectPayloadV3::new(
        authority::AcceptanceSubjectKindV3::ContextApplicationV2Record,
        asp_payload,
    )
    .unwrap();
    assert_eq!(
        hex(&cbor::encode_canonical(&subject.semantic_input()).unwrap()),
        entry("acceptance_subject_v3")["payload_cbor_hex"]
            .as_str()
            .unwrap()
    );

    let roster_ref = authority::ReviewerRosterRefV1::new(
        "sources/m2_5/authorities/reviewer_rosters/v1/".to_owned() + &"00".repeat(32) + ".json",
        authority::REVIEWER_ROSTER_SCHEMA_V1.to_owned(),
        [0; 32],
    )
    .unwrap();
    let base = authority::ContextAuthoritySourceBindingV2::new(
        "base_authority_v1",
        "sources/m2_5/authorities/interaction_review_authority.v1.json",
        Some("manafold.m2.5.c.interaction-review-authority.v1"),
        [0; 32],
    )
    .unwrap();
    let roster = authority::ContextAuthoritySourceBindingV2::new(
        "reviewer_roster_leaf",
        roster_ref.path.clone(),
        Some(authority::REVIEWER_ROSTER_SCHEMA_V1),
        [0; 32],
    )
    .unwrap();
    let event = authority::ReviewAcceptanceEventInputV3::new(
        authority::AcceptanceSubjectKindV3::ContextApplicationV2Record,
        subject.identity().unwrap().as_digest_reference(),
        roster_ref,
        vec![authority::ReviewerRoleBindingV1::new(
            "alice",
            vec![
                "architecture_maintainer".to_owned(),
                "rules_authority_maintainer".to_owned(),
            ],
        )
        .unwrap()],
        authority::ReviewMode::MultiReviewer,
        vec![base, roster],
        vec![authority::AcceptanceEvidenceRefV1::new(
            "a",
            [0; 32],
            authority::EvidenceLocatorV1::WholeArtifact,
        )
        .unwrap()],
    )
    .unwrap();
    assert_eq!(
        hex(&cbor::encode_canonical(&event.semantic_input()).unwrap()),
        entry("review_acceptance_event_v3")["payload_cbor_hex"]
            .as_str()
            .unwrap()
    );
    let host = authority::ApplicationHostBindingV2::new(
        "context_application",
        authority::AuthorityIdentityV1::from_digest_bytes(
            AuthorityIdentityKind::ContextApplicationV2,
            cpa_digest,
        ),
        vec!["hbc.v1/".to_owned() + &"00".repeat(32)],
    )
    .unwrap();
    let host_fields = match host.to_cbor() {
        cbor::Value::Array(fields) => fields,
        _ => panic!("host binding vector is not an array"),
    };
    assert_eq!(
        host_fields[0],
        cbor::Value::Text("context_application".to_owned())
    );
}

#[test]
fn context_application_v2_v3_subject_and_reviewer_order_contracts_are_closed() {
    let evidence = authority::EvidenceRefV1::new(
        "model",
        "a",
        authority::EvidenceLocatorV1::WholeArtifact,
        [0; 32],
    )
    .unwrap();
    let revocation_supersession = authority::ContextApplicationV2SupersessionInputV2::new(
        [0; 32],
        None,
        None,
        authority::SupersessionReason::AuthorityRevocation,
        vec![evidence.clone()],
    )
    .unwrap();
    let revocation_supersession_id = revocation_supersession.identity().unwrap().digest_bytes();
    let revocation_subject = authority::AcceptanceSubjectPayloadV3::new(
        authority::AcceptanceSubjectKindV3::ContextApplicationV2SupersessionRecord,
        cbor::Value::Array(vec![
            cbor::Value::Text("context_application_v2_supersession_record".to_owned()),
            cbor::Value::Bytes(revocation_supersession_id.to_vec()),
            cbor::Value::Bytes(vec![0; 32]),
            cbor::Value::Null,
            cbor::Value::Text("context_application_v2_record".to_owned()),
            cbor::Value::Null,
            cbor::Value::Text("authority_revocation".to_owned()),
            cbor::Value::Array(vec![evidence.to_cbor()]),
        ]),
    )
    .unwrap();
    let matrix: serde_json::Value = serde_json::from_str(include_str!(
        "../../../conformance/fixtures/authority/context_application_v2_identity_golden_matrix.v1.json"
    ))
    .unwrap();
    let revocation_entry = matrix["identities"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| {
            entry["kind"] == serde_json::json!("acceptance_subject_v3")
                && entry["subject_kind"]
                    == serde_json::json!("context_application_v2_supersession_record")
        })
        .unwrap();
    let revocation_identity = revocation_subject.identity().unwrap();
    assert_eq!(
        revocation_identity.as_text(),
        revocation_entry["identity"].as_str().unwrap()
    );

    let invalid_subject = authority::AcceptanceSubjectPayloadV3::new(
        authority::AcceptanceSubjectKindV3::ContextApplicationV2SupersessionRecord,
        cbor::Value::Array(vec![
            cbor::Value::Text("context_application_v2_supersession_record".to_owned()),
            cbor::Value::Bytes(vec![0; 32]),
            cbor::Value::Bytes(vec![0; 32]),
            cbor::Value::Null,
            cbor::Value::Text("context_application_v2_record".to_owned()),
            cbor::Value::Null,
            cbor::Value::Text("semantic_correction".to_owned()),
            cbor::Value::Array(vec![authority::EvidenceRefV1::new(
                "model",
                "a",
                authority::EvidenceLocatorV1::WholeArtifact,
                [0; 32],
            )
            .unwrap()
            .to_cbor()]),
        ]),
    )
    .unwrap();
    assert!(invalid_subject.identity().is_err());

    let roster_ref = authority::ReviewerRosterRefV1::new(
        "sources/m2_5/authorities/reviewer_rosters/v1/".to_owned() + &"00".repeat(32) + ".json",
        authority::REVIEWER_ROSTER_SCHEMA_V1.to_owned(),
        [0; 32],
    )
    .unwrap();
    let base = authority::ContextAuthoritySourceBindingV2::new(
        "base_authority_v1",
        "sources/m2_5/authorities/interaction_review_authority.v1.json",
        Some("manafold.m2.5.c.interaction-review-authority.v1"),
        [0; 32],
    )
    .unwrap();
    let roster = authority::ContextAuthoritySourceBindingV2::new(
        "reviewer_roster_leaf",
        roster_ref.path.clone(),
        Some(authority::REVIEWER_ROSTER_SCHEMA_V1),
        [0; 32],
    )
    .unwrap();
    let mut source_bindings = vec![base, roster];
    source_bindings.sort_by_key(|binding| {
        cbor::encode_canonical(&binding.to_cbor()).expect("source binding is encodable")
    });
    let roles = vec![
        "architecture_maintainer".to_owned(),
        "rules_authority_maintainer".to_owned(),
    ];
    let event = authority::ReviewAcceptanceEventInputV3::new(
        authority::AcceptanceSubjectKindV3::ContextApplicationV2SupersessionRecord,
        revocation_subject.identity().unwrap().as_digest_reference(),
        roster_ref,
        vec![
            authority::ReviewerRoleBindingV1::new("b", roles.clone()).unwrap(),
            authority::ReviewerRoleBindingV1::new("aa", roles).unwrap(),
        ],
        authority::ReviewMode::MultiReviewer,
        source_bindings,
        vec![authority::AcceptanceEvidenceRefV1::new(
            "a",
            [0; 32],
            authority::EvidenceLocatorV1::WholeArtifact,
        )
        .unwrap()],
    );
    assert!(event.is_ok());
}

#[test]
fn required_relation_channels_use_declared_vocabulary_order() {
    let matrix: serde_json::Value = serde_json::from_str(include_str!(
        "../../../conformance/fixtures/authority/identity_golden_matrix.v1.json"
    ))
    .unwrap();
    let entry = matrix["identities"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["kind"] == serde_json::json!("relation_theorem"))
        .unwrap();
    let mut payload =
        cbor::decode_canonical(&decode_hex(entry["payload_cbor_hex"].as_str().unwrap())).unwrap();
    let fields = match &mut payload {
        cbor::Value::Array(fields) => fields,
        _ => panic!("relation theorem matrix payload is not an array"),
    };
    let proof_payload = match &mut fields[9] {
        cbor::Value::Array(values) => values,
        _ => panic!("relation proof payload is not an array"),
    };
    let positive_fields = match &mut proof_payload[1] {
        cbor::Value::Array(values) => values,
        _ => panic!("positive relation payload is not an array"),
    };
    positive_fields[1] = cbor::Value::Array(vec![
        cbor::Value::Text("participant_boundary".to_owned()),
        cbor::Value::Text("event_or_effect_causality".to_owned()),
    ]);
    assert!(authority::AuthorityIdentityV1::compute(
        AuthorityIdentityKind::RelationTheorem,
        payload
    )
    .is_ok());
}

#[test]
fn context_application_v2_closure_golden_matrix_matches_rust_algebra() {
    fn source_binding(value: &serde_json::Value) -> authority::ContextAuthoritySourceBindingV2 {
        let role = value["artifact_role"].as_str().unwrap();
        let path = value["path"].as_str().unwrap();
        let schema = value["schema"].as_str();
        let digest: [u8; 32] = decode_hex(value["raw_sha256"].as_str().unwrap())
            .try_into()
            .unwrap();
        authority::ContextAuthoritySourceBindingV2::new(role, path, schema, digest).unwrap()
    }

    fn source_bindings(
        value: &serde_json::Value,
    ) -> Vec<authority::ContextAuthoritySourceBindingV2> {
        value
            .as_array()
            .unwrap()
            .iter()
            .map(source_binding)
            .collect()
    }

    let matrix: serde_json::Value = serde_json::from_str(include_str!(
        "../../../conformance/fixtures/authority/context_application_v2_closure_golden_matrix.v1.json"
    ))
    .unwrap();
    assert_eq!(
        matrix["schema"],
        serde_json::json!("manafold.m2.5.c.context-application-v2-closure-golden-matrix.v1")
    );

    for case in matrix["event_cases"].as_array().unwrap() {
        let fixed = source_bindings(&case["fixed_bindings"]);
        let direct = source_bindings(&case["direct_bindings"]);
        let available = source_bindings(&case["available_bindings"]);
        let hosts = source_bindings(&case["host_bindings"]);
        let roles: Vec<&str> = case["b2_evidence_roles"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap())
            .collect();
        let actual = authority::reconstruct_event_source_closure_v2(
            &fixed,
            &direct,
            &available,
            &roles,
            case["b1_citation"].as_bool().unwrap(),
            &hosts,
        )
        .unwrap();
        let expected = source_bindings(&case["expected_source_bindings"]);
        assert_eq!(actual, expected, "event case {}", case["case_id"]);
        authority::require_exact_context_source_set_v2(&actual, &expected).unwrap();
    }

    for case in matrix["container_cases"].as_array().unwrap() {
        let static_bindings = source_bindings(&case["static_bindings"]);
        let event_leaf_bindings = source_bindings(&case["event_leaf_bindings"]);
        let event_closures: Vec<Vec<authority::ContextAuthoritySourceBindingV2>> = case
            ["event_closures"]
            .as_array()
            .unwrap()
            .iter()
            .map(source_bindings)
            .collect();
        let hosts = source_bindings(&case["host_bindings"]);
        let actual = authority::reconstruct_container_source_closure_v2(
            &static_bindings,
            &event_leaf_bindings,
            &event_closures,
            &hosts,
        )
        .unwrap();
        let expected = source_bindings(&case["expected_source_bindings"]);
        assert_eq!(actual, expected, "container case {}", case["case_id"]);
    }

    let matrix_case = &matrix["event_cases"][0];
    let fixed = source_bindings(&matrix_case["fixed_bindings"]);
    let expected = source_bindings(&matrix_case["expected_source_bindings"]);
    let mut noncanonical = expected.clone();
    noncanonical.swap(0, 1);
    assert!(authority::require_exact_context_source_set_v2(&noncanonical, &expected).is_err());
    assert!(authority::require_exact_context_source_set_v2(
        &[fixed[0].clone(), fixed[0].clone()],
        &expected,
    )
    .is_err());
}

fn payload_array_len(value: &cbor::Value) -> usize {
    match value {
        cbor::Value::Array(values) => values.len(),
        other => panic!("identity matrix payload is not an array: {other:?}"),
    }
}

fn authority_kind(value: &str) -> AuthorityIdentityKind {
    match value {
        "relation_theorem" => AuthorityIdentityKind::RelationTheorem,
        "relation_theorem_record" => AuthorityIdentityKind::RelationTheoremRecord,
        "relation_application" => AuthorityIdentityKind::RelationApplication,
        "relation_application_record" => AuthorityIdentityKind::RelationApplicationRecord,
        "relation_application_v2" => AuthorityIdentityKind::RelationApplicationV2,
        "relation_application_record_v2" => AuthorityIdentityKind::RelationApplicationRecordV2,
        "relation_application_v2_supersession" => AuthorityIdentityKind::RelationSupersessionV2,
        "relation_application_v2_supersession_record" => {
            AuthorityIdentityKind::RelationSupersessionRecordV2
        }
        "relation_supersession" => AuthorityIdentityKind::RelationSupersession,
        "domain_theorem" => AuthorityIdentityKind::DomainTheorem,
        "domain_theorem_record" => AuthorityIdentityKind::DomainTheoremRecord,
        "domain_application" => AuthorityIdentityKind::DomainApplication,
        "domain_application_record" => AuthorityIdentityKind::DomainApplicationRecord,
        "domain_supersession" => AuthorityIdentityKind::DomainSupersession,
        "context_theorem" => AuthorityIdentityKind::ContextTheorem,
        "context_theorem_record" => AuthorityIdentityKind::ContextTheoremRecord,
        "context_application" => AuthorityIdentityKind::ContextApplication,
        "context_application_record" => AuthorityIdentityKind::ContextApplicationRecord,
        "context_supersession" => AuthorityIdentityKind::ContextSupersession,
        "acceptance_subject" => AuthorityIdentityKind::AcceptanceSubject,
        "review_acceptance_event" => AuthorityIdentityKind::ReviewAcceptanceEvent,
        "acceptance_subject_v4" => AuthorityIdentityKind::AcceptanceSubjectV4,
        "review_acceptance_event_v4" => AuthorityIdentityKind::ReviewAcceptanceEventV4,
        other => panic!("unknown authority identity kind: {other}"),
    }
}

fn decode_hex(value: &str) -> Vec<u8> {
    assert_eq!(value.len() % 2, 0);
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let high = (pair[0] as char).to_digit(16).unwrap();
            let low = (pair[1] as char).to_digit(16).unwrap();
            ((high << 4) | low) as u8
        })
        .collect()
}

#[test]
fn canonical_cbor_v1_complete_profile_matrix() {
    // Every accepted primitive at its width boundaries.
    let accepted: Vec<(cbor::Value, Vec<u8>)> = [
        (cbor::Value::Null, vec![0xf6]),
        (cbor::Value::Bool(false), vec![0xf4]),
        (cbor::Value::Bool(true), vec![0xf5]),
        (cbor::Value::Unsigned(0), vec![0x00]),
        (cbor::Value::Unsigned(23), vec![0x17]),
        (cbor::Value::Unsigned(24), vec![0x18, 0x18]),
        (
            cbor::Value::Unsigned(u64::MAX),
            vec![0x1b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
        ),
        (cbor::Value::Signed(-1), vec![0x20]),
        (
            cbor::Value::Signed(i64::MIN),
            vec![0x3b, 0x7f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
        ),
        (cbor::Value::Bytes(vec![]), vec![0x40]),
        (
            cbor::Value::Bytes(vec![0xab; 25]),
            std::iter::once(0x58)
                .chain(std::iter::once(25))
                .chain(std::iter::repeat_n(0xab, 25))
                .collect(),
        ),
        (cbor::Value::Text(String::new()), vec![0x60]),
        (
            cbor::Value::Text("\u{e9}\u{20ac}".to_owned()),
            vec![0x65, 0xc3, 0xa9, 0xe2, 0x82, 0xac],
        ),
        (cbor::Value::Array(vec![]), vec![0x80]),
    ]
    .into_iter()
    .collect();
    for (value, expected) in &accepted {
        let encoded = cbor::encode_canonical(value).unwrap();
        assert_eq!(&encoded, expected, "canonical bytes drifted for {value:?}");
        assert_eq!(cbor::decode_canonical(&encoded).unwrap(), *value);
    }

    // Every forbidden form with its exact ADR-0040 category.
    let forbidden: Vec<(Vec<u8>, PersistenceDecodeErrorV1)> = vec![
        (vec![0xa0], PersistenceDecodeErrorV1::DisallowedCborForm),
        (vec![0xc0], PersistenceDecodeErrorV1::DisallowedCborForm),
        (
            vec![0xfb, 0x3f, 0xf0, 0, 0, 0, 0, 0, 0],
            PersistenceDecodeErrorV1::DisallowedCborForm,
        ),
        (
            vec![0xf9, 0x3c, 0x00],
            PersistenceDecodeErrorV1::DisallowedCborForm,
        ),
        (
            vec![0x9f, 0x01, 0xff],
            PersistenceDecodeErrorV1::DisallowedCborForm,
        ),
        (
            vec![0x7f, 0x62, 0x68, 0x69, 0xff],
            PersistenceDecodeErrorV1::DisallowedCborForm,
        ),
        (vec![0xf7], PersistenceDecodeErrorV1::DisallowedCborForm),
        (
            vec![0xf8, 0x20],
            PersistenceDecodeErrorV1::DisallowedCborForm,
        ),
        (vec![0x1c], PersistenceDecodeErrorV1::DisallowedCborForm),
        (vec![0x1d], PersistenceDecodeErrorV1::DisallowedCborForm),
        (vec![0x1e], PersistenceDecodeErrorV1::DisallowedCborForm),
        (vec![0x1f], PersistenceDecodeErrorV1::DisallowedCborForm),
        // Non-shortest integer encodings.
        (
            vec![0x18, 0x17],
            PersistenceDecodeErrorV1::NoncanonicalPrimitive,
        ),
        (
            vec![0x19, 0x00, 0xff],
            PersistenceDecodeErrorV1::NoncanonicalPrimitive,
        ),
        (
            vec![0x1a, 0x00, 0x00, 0xff, 0xff],
            PersistenceDecodeErrorV1::NoncanonicalPrimitive,
        ),
        (
            vec![0x1b, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff],
            PersistenceDecodeErrorV1::NoncanonicalPrimitive,
        ),
        (
            vec![0x38, 0x00],
            PersistenceDecodeErrorV1::NoncanonicalPrimitive,
        ),
        // Multi-defect precedence: disallowed form precedes noncanonical
        // primitive (rank 9 < rank 10).
        (
            vec![0xb8, 0x00],
            PersistenceDecodeErrorV1::DisallowedCborForm,
        ),
        // Malformed UTF-8.
        (vec![0x61, 0xff], PersistenceDecodeErrorV1::InvalidUtf8),
        // Trailing top-level data.
        (vec![0x01, 0x00], PersistenceDecodeErrorV1::TrailingData),
        // Truncated inputs report the framing category.
        (vec![], PersistenceDecodeErrorV1::EnvelopeLength),
        (vec![0x19, 0x01], PersistenceDecodeErrorV1::EnvelopeLength),
        (
            vec![0x45, 0x61, 0x62],
            PersistenceDecodeErrorV1::EnvelopeLength,
        ),
        // Signed range bound.
        (
            [0x3b]
                .into_iter()
                .chain((i64::MAX as u64 + 1).to_be_bytes())
                .collect(),
            PersistenceDecodeErrorV1::ValueOutOfRange,
        ),
        // Declared over-limit lengths that are ALSO truncated report the
        // earlier framing category (rank 3 < rank 4/5).
        (
            [0x7a]
                .into_iter()
                .chain((1024u32 * 1024 + 1).to_be_bytes())
                .collect(),
            PersistenceDecodeErrorV1::EnvelopeLength,
        ),
        (
            [0x5a]
                .into_iter()
                .chain(((64u32 * 1024 * 1024) + 1).to_be_bytes())
                .collect(),
            PersistenceDecodeErrorV1::EnvelopeLength,
        ),
        (
            [0x9a]
                .into_iter()
                .chain(((1024u32 * 1024) + 1).to_be_bytes())
                .collect(),
            PersistenceDecodeErrorV1::ArrayTooLarge,
        ),
    ];
    for (bytes, expected) in forbidden {
        assert_eq!(
            cbor::decode_canonical(&bytes).unwrap_err(),
            expected,
            "input {:02x?}",
            bytes
        );
    }

    // Exactly 64 nested arrays decode; 65 exceed the declared depth budget.
    let boundary = |depth: usize| -> Vec<u8> {
        std::iter::repeat_n(0x81u8, depth)
            .chain(std::iter::once(0x00))
            .collect()
    };
    assert!(cbor::decode_canonical(&boundary(64)).is_ok());
    assert_eq!(
        cbor::decode_canonical(&boundary(65)).unwrap_err(),
        PersistenceDecodeErrorV1::DepthExceeded
    );

    // The item counter bounds the total decoded data items: five sibling
    // arrays at the maximum element count exceed MAX_ITEMS while every
    // individual array stays within its own declared limit.
    let mut item_bomb = vec![0x85];
    for _ in 0..5 {
        item_bomb.extend_from_slice(&[0x9a, 0x00, 0x10, 0x00, 0x00]);
        item_bomb.extend(std::iter::repeat_n(0xf6, 1024 * 1024));
    }
    assert!(item_bomb.len() <= cbor::MAX_PAYLOAD_BYTES);
    assert!(matches!(
        cbor::decode_canonical(&item_bomb),
        Err(PersistenceDecodeErrorV1::ItemLimitExceeded)
    ));

    // With the declared bytes fully present the resource categories fire:
    // a real 1 MiB + 1 byte text exceeds MAX_TEXT_BYTES.
    let full_text_size = 1024usize * 1024 + 1;
    let mut full_text = vec![0x7a];
    full_text.extend((full_text_size as u32).to_be_bytes());
    full_text.extend(std::iter::repeat_n(b'a', full_text_size));
    assert_eq!(
        cbor::decode_canonical(&full_text).unwrap_err(),
        PersistenceDecodeErrorV1::StringTooLarge
    );
}

/// An envelope whose payload frame declares above the bound AND fully
/// contains those bytes proves `payload_too_large` fires once truncation is
/// excluded (rank 4 after rank 3).
#[test]
fn payload_too_large_requires_full_bytes_present() {
    let payload_limit = cbor::MAX_PAYLOAD_BYTES;
    let declared = payload_limit + 1;
    let mut envelope = Vec::with_capacity(declared + 256);
    envelope.extend_from_slice(envelope::DIGEST_ENVELOPE_ID.as_bytes());
    envelope.push(0);
    for field in [
        &envelope::SHA256_ID.as_bytes().to_vec(),
        &b"mtgml.test-domain.v1".to_vec(),
        &envelope::CANONICAL_CBOR_ID.as_bytes().to_vec(),
        &b"test-input.v1".to_vec(),
    ] {
        envelope.extend((field.len() as u64).to_be_bytes());
        envelope.extend_from_slice(field);
    }
    envelope.extend((declared as u64).to_be_bytes());
    envelope.resize(envelope.len() + declared, 0);
    assert_eq!(
        envelope::decode_envelope(&envelope).unwrap_err(),
        PersistenceDecodeErrorV1::PayloadTooLarge
    );

    // The same oversized payload plus a single trailing byte is a framing
    // defect first (rank 3 < rank 4): the payload frame must end the
    // envelope exactly.
    envelope.push(0);
    assert_eq!(
        envelope::decode_envelope(&envelope).unwrap_err(),
        PersistenceDecodeErrorV1::EnvelopeLength
    );
}

#[test]
fn digest_envelope_v1_known_answer_matrix() {
    let payload = cbor::encode_canonical(&cbor::Value::Array(vec![
        cbor::Value::Text("input.v1".to_owned()),
        cbor::Value::Unsigned(7),
    ]))
    .unwrap();
    let envelope =
        envelope::encode_envelope("mtgml.test-domain.v1", "test-input.v1", &payload).unwrap();
    let (reference, decoded_payload) = envelope::decode_envelope(&envelope).unwrap();
    assert_eq!(decoded_payload, payload);
    assert_eq!(reference.semantic_domain, "mtgml.test-domain.v1");
    assert_eq!(reference.input_schema_id, "test-input.v1");
    assert_eq!(reference.digest_bytes, envelope::hash_envelope(&envelope));
    assert_eq!(
        hex(&reference.digest_bytes),
        "b1188a072cbe39da6a521f51a3d5790fe1f0e4c46c25b5e90f62bf5ee4a7f6ad"
    );
    assert_eq!(reference.envelope_version, envelope::DIGEST_ENVELOPE_ID);

    let prefix_len = envelope::DIGEST_ENVELOPE_ID.len() + 1;
    let fields = parse_frames(&envelope);
    assert_eq!(fields.len(), 5);
    let assemble = |fields: &[&[u8]]| -> Vec<u8> {
        let mut output = envelope[..prefix_len].to_vec();
        for field in fields {
            output.extend(mtgml_frame(field));
        }
        output
    };

    // Identity defects (ADR-0040 rank 2).
    let identity_cases: Vec<(Vec<u8>, &str)> = vec![
        (b"not-an-envelope".to_vec(), "wrong prefix"),
        (envelope[..10].to_vec(), "truncated below prefix"),
        (
            assemble(&[
                &vec![b'A'; 256],
                &fields[1],
                &fields[2],
                &fields[3],
                &fields[4],
            ]),
            "identifier above the 255-byte bound",
        ),
        (
            assemble(&[b"sha-512", &fields[1], &fields[2], &fields[3], &fields[4]]),
            "unsupported algorithm",
        ),
        (
            assemble(&[
                &fields[0],
                &fields[1],
                b"mtgml.canonical-json.v1",
                &fields[3],
                &fields[4],
            ]),
            "unsupported payload codec",
        ),
        (
            assemble(&[&fields[0], &[0x80; 20], &fields[2], &fields[3], &fields[4]]),
            "non-ASCII identifier",
        ),
        // Cross-frame precedence: an early identity defect must beat any
        // later payload or framing defect (rank 2 < 3 < 4).
        (
            [
                assemble(&[b"sha-512", &fields[1], &fields[2], &fields[3]]),
                ((u64::from(cbor::MAX_PAYLOAD_BYTES as u32)) + 1)
                    .to_be_bytes()
                    .to_vec(),
            ]
            .concat(),
            "wrong algorithm + over-limit payload declaration",
        ),
        (
            [
                envelope[..prefix_len].to_vec(),
                mtgml_frame(b"sha-256"),
                mtgml_frame(&fields[1]),
                mtgml_frame(b"mtgml.canonical-json.v1"),
                mtgml_frame(&fields[3]),
                128_u64.to_be_bytes().to_vec(),
            ]
            .concat(),
            "wrong codec + truncated payload frame",
        ),
        (
            [
                envelope[..prefix_len].to_vec(),
                mtgml_frame(b"sha-256"),
                mtgml_frame(&[0x80, 0x80, 0x80, 0x80, b't', b'e', b's', b't']),
                vec![0x00, 0x00, 0x00],
            ]
            .concat(),
            "non-ASCII domain + truncated tail",
        ),
    ];
    for (input, why) in identity_cases {
        assert_eq!(
            envelope::decode_envelope(&input).unwrap_err(),
            PersistenceDecodeErrorV1::EnvelopeIdentity,
            "{why}"
        );
    }

    // Length/framing defects (rank 3) only surface on identity-valid input:
    // any truncated input that cannot carry the full prefix is an identity
    // defect first.
    let truncated_frame = [envelope[..prefix_len].to_vec(), vec![0; 4]].concat();
    assert_eq!(
        envelope::decode_envelope(&truncated_frame).unwrap_err(),
        PersistenceDecodeErrorV1::EnvelopeLength
    );
    let trailing = [envelope.as_slice(), &[0]].concat();
    assert_eq!(
        envelope::decode_envelope(&trailing).unwrap_err(),
        PersistenceDecodeErrorV1::EnvelopeLength
    );

    // A payload frame that both declares above the bound and lacks its
    // bytes reports the earlier framing defect (rank 3 < rank 4). The
    // bytes-present counterpart is covered by
    // payload_too_large_requires_full_bytes_present.
    let mut over_limit_payload_decl = assemble(&[&fields[0], &fields[1], &fields[2], &fields[3]]);
    over_limit_payload_decl.extend((u64::from(cbor::MAX_PAYLOAD_BYTES as u32) + 1).to_be_bytes());
    assert_eq!(
        envelope::decode_envelope(&over_limit_payload_decl).unwrap_err(),
        PersistenceDecodeErrorV1::EnvelopeLength
    );
}

fn parse_frames(envelope: &[u8]) -> Vec<Vec<u8>> {
    let mut offset = envelope::DIGEST_ENVELOPE_ID.len() + 1;
    let mut fields = Vec::new();
    while offset < envelope.len() {
        let length = usize::try_from(u64::from_be_bytes(
            envelope[offset..offset + 8].try_into().unwrap(),
        ))
        .unwrap();
        fields.push(envelope[offset + 8..offset + 8 + length].to_vec());
        offset += 8 + length;
    }
    fields
}

fn mtgml_frame(value: &[u8]) -> Vec<u8> {
    let mut output = (value.len() as u64).to_be_bytes().to_vec();
    output.extend_from_slice(value);
    output
}

/// The shared mechanical negative corpus is Rust-authoritative evidence:
/// every committed fixture must produce its manifest-declared category from
/// the Rust decoder. Python parity runs against the same corpus.
#[test]
fn persisted_negative_fixture_manifest_matches_rust_categories() {
    use std::collections::BTreeMap;

    #[derive(Debug, serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Manifest {
        #[serde(rename = "schema_version")]
        _schema_version: String,
        fixtures: Vec<Fixture>,
    }
    #[derive(Debug, serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Fixture {
        contract: String,
        expected_error_code: String,
        path: String,
    }

    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let raw = std::fs::read(root.join("persistence/negative/manifest.json")).unwrap();
    let manifest: Manifest = serde_json::from_slice(&raw).unwrap();
    let mut seen: BTreeMap<String, String> = BTreeMap::new();
    assert!(manifest.fixtures.len() >= 20, "corpus regressed");
    for fixture in &manifest.fixtures {
        let bytes = std::fs::read(root.join("persistence/negative").join(&fixture.path)).unwrap();
        let actual = match fixture.contract.as_str() {
            "canonical-cbor.v1" => cbor::decode_canonical(&bytes).map(|_| ()).unwrap_err(),
            "digest-envelope.v1" => envelope::decode_envelope(&bytes).map(|_| ()).unwrap_err(),
            other => panic!("unknown fixture contract {other}"),
        };
        assert_eq!(
            actual.as_str(),
            fixture.expected_error_code,
            "fixture {} drifted",
            fixture.path
        );
        assert!(seen
            .insert(fixture.path.clone(), fixture.contract.clone())
            .is_none());
    }
}

#[test]
fn checkpoint_digest_v3_known_answer() {
    let full_state = mtgml_model::DigestReferenceV1 {
        envelope_version: envelope::DIGEST_ENVELOPE_ID.to_owned(),
        algorithm_id: envelope::SHA256_ID.to_owned(),
        semantic_domain: "mtgml.full-state-digest.v3".to_owned(),
        payload_codec_id: envelope::CANONICAL_CBOR_ID.to_owned(),
        input_schema_id: "full-state-digest-input.v3".to_owned(),
        digest_bytes: [7; 32],
    };
    let counters = EnvironmentLimitCounters::default();
    let codec = CheckpointCodecIdentity {
        codec_id: envelope::CANONICAL_CBOR_ID.to_owned(),
        semantic_version: "v3".to_owned(),
    };
    let reference = checkpoint_digest::calculate_checkpoint_digest_v3(
        &full_state,
        &EpisodeStatus::Running,
        &counters,
        &codec,
    )
    .unwrap();
    assert_eq!(
        mtgml_model::CheckpointDigestV3::DOMAIN,
        "mtgml.checkpoint-digest.v3"
    );
    assert_eq!(reference.raw_bytes().len(), 32);
    assert_eq!(
        hex(&reference.raw_bytes()),
        "b0cf94e1f49fb58feb6ebc07d88b2a7e226be78c1ca92ee7b9772d4f51290f6c"
    );
}

#[test]
fn error_categories_are_closed_and_stable() {
    assert_eq!(
        PersistenceDecodeErrorV1::TrailingData.as_str(),
        "trailing_data"
    );
    assert_eq!(
        PersistenceDecodeErrorV1::UnsupportedHistoricalVersion.as_str(),
        "unsupported_historical_version"
    );
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    bytes.iter().fold(
        String::with_capacity(bytes.len() * 2),
        |mut encoded, byte| {
            write!(encoded, "{byte:02x}").expect("writing hexadecimal bytes to String cannot fail");
            encoded
        },
    )
}
