//! Content-contract digest framing over the canonical manifest payload.
//!
//! The Card IR owner validates and canonically encodes the closed schema.
//! This module owns the persistence domain, existing envelope, and digest.

use crate::{cbor, envelope, PersistenceDecodeErrorV1};
use mtgml_model::ContentContractIdV1;

pub const CONTENT_CONTRACT_DOMAIN: &str = "mtgml.content-contract.v1";
pub const CONTENT_CONTRACT_INPUT_SCHEMA: &str = "content-contract-manifest.v1";

pub fn calculate_content_contract_id_v1(
    canonical_payload: &[u8],
) -> Result<ContentContractIdV1, PersistenceDecodeErrorV1> {
    let value = cbor::decode_canonical(canonical_payload)?;
    let ValueParts {
        schema,
        domain,
        definitions,
    } = split_manifest(value)?;
    if schema != CONTENT_CONTRACT_INPUT_SCHEMA || domain != CONTENT_CONTRACT_DOMAIN {
        return Err(PersistenceDecodeErrorV1::SchemaIdentityMismatch);
    }
    for definition in definitions {
        let fields = expect_array(definition, 7)?;
        let binding = &fields[4];
        let binding = expect_array(binding.clone(), 2)?;
        match (&binding[0], &binding[1]) {
            (cbor::Value::Text(variant), cbor::Value::Null) if variant == "unprofiled" => {}
            (cbor::Value::Text(variant), profile) if variant == "profiled" => {
                validate_profiled_identity_shape(profile.clone())?
            }
            _ => return Err(PersistenceDecodeErrorV1::UnknownVariant),
        }
    }
    let envelope = envelope::encode_envelope(
        CONTENT_CONTRACT_DOMAIN,
        CONTENT_CONTRACT_INPUT_SCHEMA,
        canonical_payload,
    )?;
    let bytes = envelope::hash_envelope(&envelope);
    let mut text = String::with_capacity(64);
    for byte in bytes {
        use std::fmt::Write as _;
        write!(&mut text, "{byte:02x}").expect("writing to String cannot fail");
    }
    ContentContractIdV1::parse(text).map_err(|_| PersistenceDecodeErrorV1::SemanticValidation)
}

fn validate_profiled_identity_shape(value: cbor::Value) -> Result<(), PersistenceDecodeErrorV1> {
    let profile = expect_array(value, 2)?;
    let profile_id = expect_text(&profile[0])?;
    if !matches!(profile_id, "basic-land@1.0.0" | "vanilla-creature@1.0.0") {
        return Err(PersistenceDecodeErrorV1::UnknownVariant);
    }
    let body = expect_array(profile[1].clone(), 2)?;
    let body_label = expect_text(&body[0])?;
    let body_is_valid = if profile_id == "basic-land@1.0.0" {
        body_label == "basic-land-profile.v1"
            && matches!(
                &body[1],
                cbor::Value::Text(subtype) if subtype == "mountain" || subtype == "plains"
            )
    } else {
        body_label == "vanilla-creature-profile.v1" && matches!(&body[1], cbor::Value::Null)
    };
    if !body_is_valid {
        return Err(PersistenceDecodeErrorV1::SemanticValidation);
    }
    Ok(())
}

fn expect_text(value: &cbor::Value) -> Result<&str, PersistenceDecodeErrorV1> {
    match value {
        cbor::Value::Text(text) => Ok(text),
        _ => Err(PersistenceDecodeErrorV1::SemanticValidation),
    }
}

struct ValueParts {
    schema: String,
    domain: String,
    definitions: Vec<cbor::Value>,
}

fn split_manifest(value: cbor::Value) -> Result<ValueParts, PersistenceDecodeErrorV1> {
    let mut fields = expect_array(value, 3)?;
    let definitions = match fields.pop().expect("checked array length") {
        cbor::Value::Array(values) => values,
        _ => return Err(PersistenceDecodeErrorV1::SemanticValidation),
    };
    let domain = match fields.pop().expect("checked array length") {
        cbor::Value::Text(text) => text,
        _ => return Err(PersistenceDecodeErrorV1::SchemaIdentityMismatch),
    };
    let schema = match fields.pop().expect("checked array length") {
        cbor::Value::Text(text) => text,
        _ => return Err(PersistenceDecodeErrorV1::SchemaIdentityMismatch),
    };
    Ok(ValueParts {
        schema,
        domain,
        definitions,
    })
}

fn expect_array(
    value: cbor::Value,
    length: usize,
) -> Result<Vec<cbor::Value>, PersistenceDecodeErrorV1> {
    match value {
        cbor::Value::Array(values) if values.len() == length => Ok(values),
        cbor::Value::Array(_) => Err(PersistenceDecodeErrorV1::WrongRecordLength),
        _ => Err(PersistenceDecodeErrorV1::SemanticValidation),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_id_matches_minimal_manifest_known_answer() {
        let payload =
            include_bytes!("../../../persistence/golden/content-contract-minimal-v1.cbor");
        let id = calculate_content_contract_id_v1(payload).unwrap();
        assert_eq!(
            id.as_str(),
            "105a083f417293333532c3ffed1d96c13f74664c3bbaf64e8fad995918fb5a4a"
        );
    }

    #[test]
    fn unknown_profiled_manifest_is_rejected_before_content_id_minting() {
        let mut value = cbor::decode_canonical(include_bytes!(
            "../../../persistence/golden/content-contract-minimal-v1.cbor"
        ))
        .unwrap();
        let cbor::Value::Array(root) = &mut value else {
            unreachable!()
        };
        let cbor::Value::Array(definitions) = &mut root[2] else {
            unreachable!()
        };
        let cbor::Value::Array(definition) = &mut definitions[0] else {
            unreachable!()
        };
        definition[4] = cbor::Value::Array(vec![
            cbor::Value::Text("profiled".to_owned()),
            cbor::Value::Array(vec![
                cbor::Value::Text("test/profile@1.0.0".to_owned()),
                cbor::Value::Array(vec![]),
            ]),
        ]);
        let bytes = cbor::encode_canonical(&value).unwrap();
        assert_eq!(
            calculate_content_contract_id_v1(&bytes),
            Err(PersistenceDecodeErrorV1::UnknownVariant)
        );
    }

    #[test]
    fn basic_land_profile_content_id_matches_frozen_v1_known_answer() {
        let fixture = basic_land_fixture();
        let payload = decode_hex(fixture["canonical_payload_hex"].as_str().unwrap());
        let id = calculate_content_contract_id_v1(&payload).unwrap();
        assert_eq!(
            id.as_str(),
            fixture["content_contract_id"].as_str().unwrap()
        );
    }

    #[test]
    fn combined_catalog_content_id_matches_frozen_known_answer() {
        let fixture: serde_json::Value = serde_json::from_slice(include_bytes!(
            "../../../persistence/golden/content-contract-basic-land-and-vanilla-creature-v1-kat.v1.json"
        ))
        .unwrap();
        let payload = decode_hex(fixture["canonical_payload_hex"].as_str().unwrap());
        let id = calculate_content_contract_id_v1(&payload).unwrap();
        assert_eq!(
            id.as_str(),
            fixture["content_contract_id"].as_str().unwrap()
        );
    }

    #[test]
    fn unsupported_or_malformed_profile_content_is_rejected_before_hashing() {
        let fixture = basic_land_fixture();
        let original = decode_hex(fixture["canonical_payload_hex"].as_str().unwrap());

        for mutation in ["profile_id", "body_label", "subtype"] {
            let mut value = cbor::decode_canonical(&original).unwrap();
            let cbor::Value::Array(root) = &mut value else {
                unreachable!()
            };
            let cbor::Value::Array(definitions) = &mut root[2] else {
                unreachable!()
            };
            let cbor::Value::Array(definition) = &mut definitions[0] else {
                unreachable!()
            };
            let cbor::Value::Array(binding) = &mut definition[4] else {
                unreachable!()
            };
            let cbor::Value::Array(profile) = &mut binding[1] else {
                unreachable!()
            };
            let cbor::Value::Array(body) = &mut profile[1] else {
                unreachable!()
            };
            match mutation {
                "profile_id" => profile[0] = cbor::Value::Text("future@1.0.0".to_owned()),
                "body_label" => body[0] = cbor::Value::Text("future-body.v1".to_owned()),
                "subtype" => body[1] = cbor::Value::Text("island".to_owned()),
                _ => unreachable!(),
            }
            let malformed = cbor::encode_canonical(&value).unwrap();
            assert!(calculate_content_contract_id_v1(&malformed).is_err());
        }
    }

    fn basic_land_fixture() -> serde_json::Value {
        serde_json::from_slice(include_bytes!(
            "../../../persistence/golden/content-contract-basic-land-v1-kat.v1.json"
        ))
        .unwrap()
    }

    fn decode_hex(value: &str) -> Vec<u8> {
        value
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| {
                let digit = |value: u8| match value {
                    b'0'..=b'9' => value - b'0',
                    b'a'..=b'f' => value - b'a' + 10,
                    _ => panic!("fixture hex must be lowercase"),
                };
                digit(pair[0]) * 16 + digit(pair[1])
            })
            .collect()
    }
}
