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
            (cbor::Value::Text(variant), _) if variant == "profiled" => {
                return Err(PersistenceDecodeErrorV1::SemanticValidation)
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
    fn profiled_manifest_is_rejected_before_content_id_minting() {
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
            Err(PersistenceDecodeErrorV1::SemanticValidation)
        );
    }
}
