use forge_card_script_parser::lowering::{FORGE_SOURCE_REVISION, TYPE_VOCABULARY_SNAPSHOT};
use mtgml_card_ir::{
    content_validation_only, decode_content_manifest_v1, decode_provenance_catalog_v1,
    encode_content_manifest_v1, CardSemanticBindingV1, ContentAuthorizationV1,
    RequiredCapabilityLifecycleV1,
};
use mtgml_model::CardDefinitionId;
use mtgml_persistence::content_contract_digest::calculate_content_contract_id_v1;

const MANIFEST: &[u8] = include_bytes!("../candidates/r1w1/content-contract.v1.cbor");
const PROVENANCE: &[u8] = include_bytes!("../candidates/r1w1/provenance.v1.cbor");
const METADATA: &[u8] = include_bytes!("../candidates/r1w1/lowering-metadata.v1.json");

#[test]
fn selected_r1_w1_candidates_use_existing_card_ir_validation_and_pinned_sources() {
    let manifest = decode_content_manifest_v1(MANIFEST).expect("canonical typed manifest");
    assert_eq!(manifest.definitions.len(), 26);
    assert_eq!(encode_content_manifest_v1(&manifest).unwrap(), MANIFEST);

    let content_id = calculate_content_contract_id_v1(MANIFEST).unwrap();
    let provenance = decode_provenance_catalog_v1(PROVENANCE).unwrap();
    assert_eq!(provenance.records.len(), 26);
    assert!(provenance.records.iter().all(|record| {
        record.content_contract_id == content_id
            && record
                .source_provenance
                .source_snapshot_id
                .contains(FORGE_SOURCE_REVISION)
    }));

    let roots = (1..=26).map(CardDefinitionId).collect::<Vec<_>>();
    let validation = content_validation_only(
        MANIFEST,
        &content_id,
        PROVENANCE,
        &roots,
        RequiredCapabilityLifecycleV1::Proposed,
    )
    .expect("validation-only content preflight");
    assert_eq!(
        validation.authorization,
        ContentAuthorizationV1::ValidationOnly
    );
    assert_eq!(validation.reachable_definitions.len(), 26);
    assert!(validation.direct_requirement_roots.is_empty());

    for definition in &manifest.definitions {
        assert_eq!(
            definition.semantic_binding,
            CardSemanticBindingV1::UnprofiledV1
        );
        assert!(definition.ability_identities.is_empty());
        assert!(definition.definition_references.is_empty());
        assert!(definition.explicit_additional_requirements.is_empty());
        for (face_index, face) in definition.faces.iter().enumerate() {
            assert_eq!(face.face_key.0, face_index as u32);
        }
    }

    let ojer = manifest
        .definitions
        .iter()
        .find(|definition| {
            definition.faces[0].base_characteristics.name == "Ojer Axonil, Deepest Might"
        })
        .unwrap();
    assert_eq!(ojer.faces.len(), 2);
    assert_eq!(ojer.faces[1].base_characteristics.name, "Temple of Power");
    assert_eq!(ojer.faces[1].base_characteristics.mana_cost, None);

    let skyward = manifest
        .definitions
        .iter()
        .find(|definition| definition.faces[0].base_characteristics.name == "Skyward Spider")
        .unwrap();
    assert_eq!(
        skyward.faces[0].base_characteristics.mana_cost,
        Some(vec![
            mtgml_card_ir::PrintedManaSymbolV1::Hybrid(
                mtgml_card_ir::ManaColorV1::White,
                mtgml_card_ir::ManaColorV1::Blue,
            ),
            mtgml_card_ir::PrintedManaSymbolV1::Hybrid(
                mtgml_card_ir::ManaColorV1::White,
                mtgml_card_ir::ManaColorV1::Blue,
            ),
        ])
    );

    let metadata: serde_json::Value = serde_json::from_slice(METADATA).unwrap();
    assert_eq!(metadata["forge_source_revision"], FORGE_SOURCE_REVISION);
    assert_eq!(
        metadata["type_vocabulary_snapshot"],
        TYPE_VOCABULARY_SNAPSHOT
    );
    assert_eq!(metadata["authorization"], "VALIDATION_ONLY");
    assert_eq!(metadata["card_support_claim"], "NONE");
    assert_eq!(metadata["unlowered_construct_total"], 96);
    assert_eq!(metadata["unlowered_construct_counts"]["A"], 12);
    assert_eq!(metadata["unlowered_construct_counts"]["T"], 14);
    assert_eq!(metadata["unlowered_construct_counts"]["R"], 3);
    assert_eq!(metadata["unlowered_construct_counts"]["S"], 8);
    assert_eq!(metadata["unlowered_construct_counts"]["K"], 21);
    assert_eq!(metadata["unlowered_construct_counts"]["SVar"], 38);
    let candidates = metadata["candidates"].as_array().unwrap();
    assert!(candidates
        .iter()
        .all(|candidate| candidate["unlowered_constructs"]
            .as_array()
            .unwrap()
            .iter()
            .all(|construct| construct["semantic_lowering"] == "NOT_IMPLEMENTED")));
    let origin = candidates
        .iter()
        .find(|candidate| candidate["card_name"] == "Origin of Spider-Man")
        .unwrap();
    assert_eq!(origin["issue_alias"], "A Most Helpful Weaver");
    let skyward = candidates
        .iter()
        .find(|candidate| candidate["card_name"] == "Skyward Spider")
        .unwrap();
    assert_eq!(skyward["issue_alias"], "Wonderweave Aerialist");
}
