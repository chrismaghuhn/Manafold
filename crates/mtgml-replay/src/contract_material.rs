//! The semantic and content contract material that Replay V8 embeds.

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use mtgml_card_ir::{
    decode_content_manifest_v1, encode_content_manifest_v1, ContentContractManifestV1,
};
use mtgml_model::{
    ContentContractIdV1, RulesContractManifestV1, SemanticContractIdV1, SemanticContractManifestV1,
};
use serde::de::Error as DeError;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::validation::ReplayValidationError;

const MAX_CONTENT_MANIFEST_BYTES: usize = 64 * 1024 * 1024;
const MAX_CONTENT_MANIFEST_BASE64_CHARS: usize = 89_478_488;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentContractMaterialV1 {
    pub content_contract_id: ContentContractIdV1,
    pub manifest: ContentContractManifestV1,
}

impl ContentContractMaterialV1 {
    pub fn from_manifest(
        manifest: ContentContractManifestV1,
    ) -> Result<Self, ReplayValidationError> {
        let payload = encode_content_manifest_v1(&manifest)
            .map_err(|_| ReplayValidationError::SemanticContractMismatch)?;
        let content_contract_id =
            mtgml_persistence::content_contract_digest::calculate_content_contract_id_v1(&payload)
                .map_err(|_| ReplayValidationError::SemanticContractMismatch)?;
        Ok(Self {
            content_contract_id,
            manifest,
        })
    }

    pub fn canonical_manifest_bytes(&self) -> Result<Vec<u8>, ReplayValidationError> {
        encode_content_manifest_v1(&self.manifest)
            .map_err(|_| ReplayValidationError::SemanticContractMismatch)
    }

    fn validate(&self) -> Result<(), ReplayValidationError> {
        let payload = self.canonical_manifest_bytes()?;
        let decoded = decode_content_manifest_v1(&payload)
            .map_err(|_| ReplayValidationError::SemanticContractMismatch)?;
        if decoded != self.manifest {
            return Err(ReplayValidationError::SemanticContractMismatch);
        }
        let recomputed =
            mtgml_persistence::content_contract_digest::calculate_content_contract_id_v1(&payload)
                .map_err(|_| ReplayValidationError::SemanticContractMismatch)?;
        if recomputed != self.content_contract_id {
            return Err(ReplayValidationError::SemanticContractMismatch);
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContentContractMaterialWireV1 {
    content_contract_id: String,
    manifest_canonical_cbor_base64: String,
}

impl Serialize for ContentContractMaterialV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.validate().map_err(serde::ser::Error::custom)?;
        let payload = self
            .canonical_manifest_bytes()
            .map_err(serde::ser::Error::custom)?;
        if payload.len() > MAX_CONTENT_MANIFEST_BYTES {
            return Err(serde::ser::Error::custom(
                "content manifest exceeds size limit",
            ));
        }
        let encoded = STANDARD.encode(payload);
        let mut state = serializer.serialize_struct("ContentContractMaterialV1", 2)?;
        state.serialize_field("content_contract_id", self.content_contract_id.as_str())?;
        state.serialize_field("manifest_canonical_cbor_base64", &encoded)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for ContentContractMaterialV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ContentContractMaterialWireV1::deserialize(deserializer)?;
        if wire.manifest_canonical_cbor_base64.len() > MAX_CONTENT_MANIFEST_BASE64_CHARS {
            return Err(D::Error::custom(
                "content manifest Base64 exceeds size limit",
            ));
        }
        let content_contract_id = ContentContractIdV1::parse(wire.content_contract_id.clone())
            .map_err(D::Error::custom)?;
        if content_contract_id.as_str() != wire.content_contract_id {
            return Err(D::Error::custom(
                "content contract ID is not canonical lowercase hex",
            ));
        }
        let payload = STANDARD
            .decode(wire.manifest_canonical_cbor_base64.as_bytes())
            .map_err(D::Error::custom)?;
        if payload.len() > MAX_CONTENT_MANIFEST_BYTES
            || STANDARD.encode(&payload) != wire.manifest_canonical_cbor_base64
        {
            return Err(D::Error::custom("content manifest Base64 is not canonical"));
        }
        let manifest = decode_content_manifest_v1(&payload).map_err(D::Error::custom)?;
        let recomputed =
            mtgml_persistence::content_contract_digest::calculate_content_contract_id_v1(&payload)
                .map_err(D::Error::custom)?;
        if recomputed != content_contract_id {
            return Err(D::Error::custom(
                "content contract ID does not match manifest",
            ));
        }
        Ok(Self {
            content_contract_id,
            manifest,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticContractMaterialV7 {
    pub semantic_contract_id: SemanticContractIdV1,
    pub manifest: SemanticContractManifestV1,
    pub rules_manifest: RulesContractManifestV1,
    pub content_contract: Option<ContentContractMaterialV1>,
}

impl SemanticContractMaterialV7 {
    pub fn validate(&self) -> Result<(), ReplayValidationError> {
        if self.manifest.format_contract_id.is_some() {
            return Err(ReplayValidationError::SemanticContractMismatch);
        }
        let rules_id = mtgml_persistence::semantic_contract_digest::calculate_rules_contract_id_v1(
            &self.rules_manifest,
        )
        .map_err(|_| ReplayValidationError::SemanticContractMismatch)?;
        if rules_id != self.manifest.rules_contract_id {
            return Err(ReplayValidationError::SemanticContractMismatch);
        }
        let semantic_id =
            mtgml_persistence::semantic_contract_digest::calculate_semantic_contract_id_v1(
                &self.manifest,
            )
            .map_err(|_| ReplayValidationError::SemanticContractMismatch)?;
        if semantic_id != self.semantic_contract_id {
            return Err(ReplayValidationError::SemanticContractMismatch);
        }
        match (
            self.manifest.content_contract_id.as_ref(),
            self.content_contract.as_ref(),
        ) {
            (Some(expected), Some(child)) if expected == &child.content_contract_id => {
                child.validate()?;
            }
            (None, None) => {}
            _ => return Err(ReplayValidationError::SemanticContractMismatch),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_child_transport_constants_match_frozen_v7_bound() {
        assert_eq!(MAX_CONTENT_MANIFEST_BYTES, 67_108_864);
        assert_eq!(MAX_CONTENT_MANIFEST_BASE64_CHARS, 89_478_488);
    }
}
