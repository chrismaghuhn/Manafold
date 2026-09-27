use mtgml_card_ir::{decode_content_manifest_v1, encode_content_manifest_v1};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct VectorFile {
    schema_version: String,
    fixture_purpose: String,
    cases: Vec<VectorCase>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct VectorCase {
    case: String,
    expected_valid: bool,
    canonical_cbor_hex: String,
}

fn decode_hex(value: &str) -> Vec<u8> {
    assert_eq!(value.len() % 2, 0);
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let digit = |byte: u8| match byte {
                b'0'..=b'9' => byte - b'0',
                b'a'..=b'f' => byte - b'a' + 10,
                _ => panic!("noncanonical hex fixture"),
            };
            digit(pair[0]) * 16 + digit(pair[1])
        })
        .collect()
}

#[test]
fn card_ir_manifest_decoder_matches_shared_python_replay_vectors() {
    let fixture: VectorFile = serde_json::from_slice(include_bytes!(
        "fixtures/content_contract_manifest_parity.v1.json"
    ))
    .unwrap();
    assert_eq!(
        fixture.schema_version,
        "content-contract-manifest-parity-v1"
    );
    assert!(fixture.fixture_purpose.contains("acceptance"));
    for vector in fixture.cases {
        let bytes = decode_hex(&vector.canonical_cbor_hex);
        let decoded = decode_content_manifest_v1(&bytes);
        assert_eq!(
            decoded.is_ok(),
            vector.expected_valid,
            "Rust acceptance drift for {}",
            vector.case
        );
        if let Ok(manifest) = decoded {
            assert_eq!(encode_content_manifest_v1(&manifest).unwrap(), bytes);
        }
    }
}
