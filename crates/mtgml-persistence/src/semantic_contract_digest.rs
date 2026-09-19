//! Canonical contract digest machinery for the V5 semantic vocabulary
//! (spec §5, §7, §8, §9; ADR 0055 §2.5/§2.6).
//!
//! Identity is content-derived under `mtgml.digest-envelope.v1` /
//! `sha-256` / `mtgml.canonical-cbor.v1` with distinct typed domains per
//! family. The canonical payloads are fixed-position CBOR arrays; digests
//! are SHA-256 over the envelope bytes. Manifest canonicality is validated
//! fail-closed through the model-side validator before hashing — no
//! non-canonical artifact can obtain an identity, and caller input is never
//! silently reordered.

use crate::{cbor, envelope, PersistenceDecodeErrorV1};
use mtgml_model::{
    CapabilityRequirementV1, RulesAuthorityV1, RulesContractIdV1, RulesContractManifestV1,
    SemanticContractIdV1, SemanticContractManifestV1,
};

pub const RULES_CONTRACT_DOMAIN: &str = "mtgml.rules-contract.v1";
pub const RULES_CONTRACT_INPUT_SCHEMA: &str = "rules-contract-manifest.v1";
pub const SEMANTIC_CONTRACT_DOMAIN: &str = "mtgml.semantic-contract.v1";
pub const SEMANTIC_CONTRACT_INPUT_SCHEMA: &str = "semantic-contract-manifest.v1";

/// Rules contract identity: SHA-256 over the digest envelope of the
/// canonical §7 fixed 4-array payload.
pub fn calculate_rules_contract_id_v1(
    manifest: &RulesContractManifestV1,
) -> Result<RulesContractIdV1, PersistenceDecodeErrorV1> {
    manifest
        .validate()
        .map_err(|_| PersistenceDecodeErrorV1::SemanticValidation)?;
    let payload = cbor::Value::Array(vec![
        cbor::Value::Text(RULES_CONTRACT_INPUT_SCHEMA.to_owned()),
        cbor::Value::Text(RULES_CONTRACT_DOMAIN.to_owned()),
        rules_authority_value(&manifest.rules_authority),
        capability_closure_value(manifest.capability_closure.as_deref()),
    ]);
    let bytes = cbor::encode_canonical(&payload)?;
    let envelope =
        envelope::encode_envelope(RULES_CONTRACT_DOMAIN, RULES_CONTRACT_INPUT_SCHEMA, &bytes)?;
    Ok(RulesContractIdV1::from_digest_bytes(
        envelope::hash_envelope(&envelope),
    ))
}

/// Semantic contract identity: SHA-256 over the digest envelope of the
/// canonical §8 fixed 5-array payload. Contract IDs travel as raw 32-byte
/// byte strings; the reserved format/content dimensions render canonical
/// `null` for absence.
pub fn calculate_semantic_contract_id_v1(
    manifest: &SemanticContractManifestV1,
) -> Result<SemanticContractIdV1, PersistenceDecodeErrorV1> {
    let payload = cbor::Value::Array(vec![
        cbor::Value::Text(SEMANTIC_CONTRACT_INPUT_SCHEMA.to_owned()),
        cbor::Value::Text(SEMANTIC_CONTRACT_DOMAIN.to_owned()),
        cbor::Value::Bytes(manifest.rules_contract_id.raw_bytes().to_vec()),
        optional_contract_bytes(manifest.format_contract_id.as_ref().map(|id| id.as_str())),
        optional_contract_bytes(manifest.content_contract_id.as_ref().map(|id| id.as_str())),
    ]);
    let bytes = cbor::encode_canonical(&payload)?;
    let envelope = envelope::encode_envelope(
        SEMANTIC_CONTRACT_DOMAIN,
        SEMANTIC_CONTRACT_INPUT_SCHEMA,
        &bytes,
    )?;
    Ok(SemanticContractIdV1::from_digest_bytes(
        envelope::hash_envelope(&envelope),
    ))
}

fn optional_contract_bytes(hex: Option<&str>) -> cbor::Value {
    match hex {
        Some(hex) => cbor::Value::Bytes(decode_id_bytes(hex)),
        None => cbor::Value::Null,
    }
}

fn decode_id_bytes(hex: &str) -> Vec<u8> {
    let bytes = (0..hex.len() / 2)
        .map(|index| u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16))
        .collect::<Result<Vec<u8>, _>>()
        .expect("typed contract id invariant: canonical lowercase hex");
    bytes
}

fn rules_authority_value(authority: &RulesAuthorityV1) -> cbor::Value {
    match authority {
        RulesAuthorityV1::SyntheticLegacy => variant("synthetic_legacy", cbor::Value::Null),
        RulesAuthorityV1::ComprehensiveRules { snapshot_id } => variant(
            "comprehensive_rules",
            cbor::Value::Text(snapshot_id.clone()),
        ),
    }
}

fn capability_closure_value(
    closure: Option<&[mtgml_model::CapabilityRequirementV1]>,
) -> cbor::Value {
    match closure {
        None => cbor::Value::Null,
        Some(entries) => cbor::Value::Array(
            entries
                .iter()
                .map(|entry| {
                    cbor::Value::Array(vec![
                        cbor::Value::Text(entry.key.clone()),
                        cbor::Value::Text(entry.version.clone()),
                    ])
                })
                .collect(),
        ),
    }
}

fn variant(name: &str, payload: cbor::Value) -> cbor::Value {
    cbor::Value::Array(vec![cbor::Value::Text(name.to_owned()), payload])
}
