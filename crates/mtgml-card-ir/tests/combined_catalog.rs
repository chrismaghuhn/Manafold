//! The committed catalog of two basic lands and three vanilla creatures.
//!
//! The files are read at run time so that `writes_the_combined_catalog` can
//! create them; every other test checks the committed bytes.

use mtgml_card_ir::{
    decode_content_manifest_v1, decode_provenance_catalog_v1, encode_content_manifest_v1,
    encode_provenance_catalog_v1, BaseCharacteristicsV1, CardDefinitionEnvelopeV1,
    CardProfileBodyV1, CardSemanticBindingV1, CardSemanticProfileId, ContentContractManifestV1,
    DefinitionProvenanceRecordV1, FaceDefinitionV1, FaceKey, PrintedManaSymbolV1,
    ProvenanceCatalogV1, SourceProvenanceV1, TypeLineV1, CARD_DEFINITION_ENVELOPE_V1,
    CONTENT_CONTRACT_MANIFEST_V1, VANILLA_CREATURE_PROFILE_ID_V1,
};
use mtgml_model::CardDefinitionId;
use mtgml_persistence::content_contract_digest::calculate_content_contract_id_v1;
use std::path::{Path, PathBuf};

const SNAPSHOT: &str = "oracle-cards-20260925210158";
const CODEC: &str = "scryfall.oracle-card-jsonl-record.v1";

fn repository_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn combined_path(file: &str) -> PathBuf {
    repository_path("cards/definitions/basic-land-and-vanilla-creature-v1").join(file)
}

fn read(path: &Path) -> Vec<u8> {
    std::fs::read(path).unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
}

fn decode_hex(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn creature(
    id: u64,
    name: &str,
    cost: Vec<PrintedManaSymbolV1>,
    subtype: &str,
    power: i32,
    toughness: i32,
) -> CardDefinitionEnvelopeV1 {
    CardDefinitionEnvelopeV1 {
        envelope_version: CARD_DEFINITION_ENVELOPE_V1.to_owned(),
        card_definition_id: CardDefinitionId(id),
        faces: vec![FaceDefinitionV1 {
            face_key: FaceKey(0),
            base_characteristics: BaseCharacteristicsV1 {
                name: name.to_owned(),
                mana_cost: Some(cost),
                color_indicator: vec![],
                type_line: TypeLineV1 {
                    supertypes: vec![],
                    card_types: vec!["Creature".to_owned()],
                    subtypes: vec![subtype.to_owned()],
                },
                power_toughness: Some((power, toughness)),
                loyalty: None,
                defense: None,
            },
        }],
        ability_identities: vec![],
        semantic_binding: CardSemanticBindingV1::ProfiledV1 {
            profile_id: CardSemanticProfileId::parse(VANILLA_CREATURE_PROFILE_ID_V1).unwrap(),
            body: CardProfileBodyV1::VanillaCreature,
        },
        definition_references: vec![],
        explicit_additional_requirements: vec![],
    }
}

/// Mountain (1) and Plains (2) exactly as the basic-land catalog has them,
/// then Savannah Lions (3), Gray Ogre (4) and Hill Giant (5).
fn combined_manifest() -> ContentContractManifestV1 {
    let lands = decode_content_manifest_v1(&read(&repository_path(
        "cards/definitions/basic-land-v1/content-contract.v1.cbor",
    )))
    .unwrap();
    let mut definitions = lands.definitions;
    definitions.extend([
        creature(
            3,
            "Savannah Lions",
            vec![PrintedManaSymbolV1::White],
            "Cat",
            2,
            1,
        ),
        creature(
            4,
            "Gray Ogre",
            vec![PrintedManaSymbolV1::Generic(2), PrintedManaSymbolV1::Red],
            "Ogre",
            2,
            2,
        ),
        creature(
            5,
            "Hill Giant",
            vec![PrintedManaSymbolV1::Generic(3), PrintedManaSymbolV1::Red],
            "Giant",
            3,
            3,
        ),
    ]);
    ContentContractManifestV1 {
        schema_version: CONTENT_CONTRACT_MANIFEST_V1.to_owned(),
        definitions,
    }
}

/// The Oracle record of each definition, written down independently of the
/// admission table so that the two can be compared.
const SOURCE_RECORDS: [(u64, &str, &str); 5] = [
    (
        1,
        "a3fb7228-e76b-4e96-a40e-20b5fed75685",
        "b57d8ce5dcbbb01e1c128aaa9ebeab8a26d5f297edb51eac6adce5c4af770033",
    ),
    (
        2,
        "bc71ebf6-2056-41f7-be35-b2e5c34afa99",
        "af82e883368b8211c1845af680e1b4dab52666b41969cc6987bffdde7ada86b7",
    ),
    (
        3,
        "60ba93eb-39e6-4af2-9c66-cd38f72daff2",
        "84fce7b698d07816432dc2674941ffdc9e0ffe586793ad669310f1bbd438c4cf",
    ),
    (
        4,
        "83c8a3a6-2e1a-4e26-8847-6d066f42d906",
        "f07e4de80207bf5701840eb63bc8f35070e2e07dca93721d4fabb03c16fc79fa",
    ),
    (
        5,
        "342199e0-15b6-4824-83da-25caef2592b3",
        "7278c29c3e7fe48fd5251930df1df553c5773e2199738a183d895ec3a0fc1952",
    ),
];

fn combined_provenance(content_bytes: &[u8]) -> Vec<u8> {
    let content_contract_id = calculate_content_contract_id_v1(content_bytes).unwrap();
    let records = SOURCE_RECORDS
        .iter()
        .map(|(id, oracle_id, digest)| DefinitionProvenanceRecordV1 {
            content_contract_id: content_contract_id.clone(),
            card_definition_id: CardDefinitionId(*id),
            source_provenance: SourceProvenanceV1 {
                source_snapshot_id: SNAPSHOT.to_owned(),
                source_record_id: (*oracle_id).to_owned(),
                source_record_codec_id: CODEC.to_owned(),
                source_record_digest: decode_hex(digest).try_into().unwrap(),
            },
        })
        .collect();
    encode_provenance_catalog_v1(&ProvenanceCatalogV1 {
        schema_version: "definition-provenance-catalog.v1".to_owned(),
        records,
    })
    .unwrap()
}

/// Run once, with `-- --ignored`, to create the committed bytes. Every other
/// test then checks them.
#[test]
#[ignore = "writes the committed catalog files"]
fn writes_the_combined_catalog() {
    let content = encode_content_manifest_v1(&combined_manifest()).unwrap();
    let provenance = combined_provenance(&content);
    std::fs::create_dir_all(combined_path("")).unwrap();
    std::fs::write(combined_path("content-contract.v1.cbor"), &content).unwrap();
    std::fs::write(combined_path("provenance.v1.cbor"), &provenance).unwrap();
}

#[test]
fn combined_catalog_bytes_are_canonical() {
    let committed = read(&combined_path("content-contract.v1.cbor"));
    let manifest = decode_content_manifest_v1(&committed).expect("the committed bytes decode");
    assert_eq!(
        encode_content_manifest_v1(&manifest).unwrap(),
        committed,
        "decoding and re-encoding must not change a byte"
    );
    assert_eq!(manifest, combined_manifest());

    let kat: serde_json::Value = serde_json::from_slice(&read(&repository_path(
        "persistence/golden/content-contract-basic-land-and-vanilla-creature-v1-kat.v1.json",
    )))
    .unwrap();
    assert_eq!(
        decode_hex(kat["canonical_payload_hex"].as_str().unwrap()),
        committed,
        "the known-answer file holds the same bytes"
    );
    assert_eq!(
        calculate_content_contract_id_v1(&committed)
            .unwrap()
            .as_str(),
        kat["content_contract_id"].as_str().unwrap()
    );
}

#[test]
fn the_lands_are_the_committed_basic_land_definitions() {
    let lands = decode_content_manifest_v1(&read(&repository_path(
        "cards/definitions/basic-land-v1/content-contract.v1.cbor",
    )))
    .unwrap();
    let combined =
        decode_content_manifest_v1(&read(&combined_path("content-contract.v1.cbor"))).unwrap();
    assert_eq!(lands.definitions.len(), 2);
    assert_eq!(&combined.definitions[..2], lands.definitions.as_slice());
}

#[test]
fn every_definition_has_its_pinned_oracle_record() {
    let content = read(&combined_path("content-contract.v1.cbor"));
    let content_contract_id = calculate_content_contract_id_v1(&content).unwrap();
    let provenance =
        decode_provenance_catalog_v1(&read(&combined_path("provenance.v1.cbor"))).unwrap();
    assert_eq!(
        provenance.records.len(),
        SOURCE_RECORDS.len(),
        "one record per definition"
    );
    for (record, (id, oracle_id, digest)) in provenance.records.iter().zip(SOURCE_RECORDS) {
        assert_eq!(record.content_contract_id, content_contract_id);
        assert_eq!(record.card_definition_id, CardDefinitionId(id));
        assert_eq!(record.source_provenance.source_snapshot_id, SNAPSHOT);
        assert_eq!(record.source_provenance.source_record_id, oracle_id);
        assert_eq!(record.source_provenance.source_record_codec_id, CODEC);
        assert_eq!(
            record.source_provenance.source_record_digest.to_vec(),
            decode_hex(digest)
        );
    }
}
