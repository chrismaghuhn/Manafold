//! ADR 0046 Slice-1 B2 closure/current-root contract vocabulary.
//!
//! This module defines shapes and fail-closed validation only. It does not
//! read repository artifacts, materialize closure v2, select a current root,
//! or migrate an authority consumer.

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const B2_CLOSURE_CURRENT_ROOT_SCHEMA: &str = "manafold.m2.5.b2.closure-current-root.v1";
pub const B2_CLOSURE_ARTIFACT_BINDING_SCHEMA: &str = "manafold.m2.5.b2.closure-artifact-binding.v1";
pub const B2_CLOSURE_V1_PATH: &str = "sources/m2_5/closures/B2/classification_closure.v1.json";
pub const B2_CLOSURE_V1_SCHEMA: &str = "manafold.m2.5.b2.classification-closure.v1";
pub const B2_CLOSURE_V2_PATH: &str = "sources/m2_5/closures/B2/classification_closure.v2.json";
pub const B2_CLOSURE_V2_SCHEMA: &str = "manafold.m2.5.b2.classification-closure.v2";
pub const B2_CLOSURE_SOURCE_PACKAGE_SHA256: &str =
    "99b33945a3e0c7b2982734e65f770715029ce6acd500104bde48e8466eed1a90";
pub const B2_CLOSURE_V1_RAW_SHA256: &str =
    "ed6a0bf4b0eb83c85027fdcc61eaf32bfa7bb06d4de78c77d0946d87212e7d43";

pub const B2_CLOSURE_ARTIFACT_ROLES: &[&str; 3] = &[
    "b2_classifications_v1",
    "b2_family_catalog_v1",
    "b2_projection_v1",
];

pub const B2_CLOSURE_SNAPSHOT_CONSTANTS: [(&str, u64); 7] = [
    ("oracle_semantic_identity_count", 402),
    ("classification_count", 402),
    ("terminal_assignment_edge_count", 1883),
    ("deck_row_count", 441),
    ("projection_row_count", 441),
    ("historical_family_count", 216),
    ("catalog_family_count", 216),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum B2ClosureContractErrorCode {
    CurrentRootMissing,
    MultipleCurrentRoots,
    UnknownClosureVersion,
    UnsupportedClosureVersion,
    CurrentRootRoleMismatch,
    CurrentRootPathMismatch,
    CurrentRootSchemaMismatch,
    CurrentRootDigestMismatch,
    SourcePackageMismatch,
    ReferencedSemanticArtifactMismatch,
    UnknownSourceRole,
    DuplicateSourceRole,
    CurrentConsumerVersionUnsupported,
    InvalidRootShape,
    InvalidArtifactBindingShape,
    NoncanonicalSourceRoleOrder,
}

impl B2ClosureContractErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CurrentRootMissing => "CURRENT_ROOT_MISSING",
            Self::MultipleCurrentRoots => "MULTIPLE_CURRENT_ROOTS",
            Self::UnknownClosureVersion => "UNKNOWN_CLOSURE_VERSION",
            Self::UnsupportedClosureVersion => "UNSUPPORTED_CLOSURE_VERSION",
            Self::CurrentRootRoleMismatch => "CURRENT_ROOT_ROLE_MISMATCH",
            Self::CurrentRootPathMismatch => "CURRENT_ROOT_PATH_MISMATCH",
            Self::CurrentRootSchemaMismatch => "CURRENT_ROOT_SCHEMA_MISMATCH",
            Self::CurrentRootDigestMismatch => "CURRENT_ROOT_DIGEST_MISMATCH",
            Self::SourcePackageMismatch => "SOURCE_PACKAGE_MISMATCH",
            Self::ReferencedSemanticArtifactMismatch => "REFERENCED_SEMANTIC_ARTIFACT_MISMATCH",
            Self::UnknownSourceRole => "UNKNOWN_SOURCE_ROLE",
            Self::DuplicateSourceRole => "DUPLICATE_SOURCE_ROLE",
            Self::CurrentConsumerVersionUnsupported => "CURRENT_CONSUMER_VERSION_UNSUPPORTED",
            Self::InvalidRootShape => "INVALID_ROOT_SHAPE",
            Self::InvalidArtifactBindingShape => "INVALID_ARTIFACT_BINDING_SHAPE",
            Self::NoncanonicalSourceRoleOrder => "NONCANONICAL_SOURCE_ROLE_ORDER",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{code:?}: {detail}")]
pub struct B2ClosureContractError {
    pub code: B2ClosureContractErrorCode,
    pub detail: String,
}

impl B2ClosureContractError {
    fn new(code: B2ClosureContractErrorCode, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum B2ClosureCurrentnessState {
    V1Current,
    V2ReadyNotAdopted,
    V2Adopted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct B2ClosureCurrentRootV1 {
    pub schema: String,
    pub artifact_role: String,
    pub closure_version: String,
    pub repository_relative_path: String,
    pub closure_schema_id: String,
    pub closure_raw_sha256: String,
    pub source_package_sha256: String,
}

impl B2ClosureCurrentRootV1 {
    pub fn validate(&self) -> Result<&Self, B2ClosureContractError> {
        if self.schema != B2_CLOSURE_CURRENT_ROOT_SCHEMA {
            return Err(B2ClosureContractError::new(
                B2ClosureContractErrorCode::CurrentRootSchemaMismatch,
                "current-root schema does not match",
            ));
        }
        if self.closure_version != "v1" && self.closure_version != "v2" {
            let code = if self
                .closure_version
                .strip_prefix('v')
                .is_some_and(|suffix| {
                    !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit())
                }) {
                B2ClosureContractErrorCode::UnsupportedClosureVersion
            } else {
                B2ClosureContractErrorCode::UnknownClosureVersion
            };
            return Err(B2ClosureContractError::new(
                code,
                "unsupported closure version",
            ));
        }
        if self.artifact_role != "b2_closure" && self.artifact_role != "b2_closure_v2" {
            return Err(B2ClosureContractError::new(
                B2ClosureContractErrorCode::UnknownSourceRole,
                "root role is not admitted",
            ));
        }
        let expected_role = if self.closure_version == "v1" {
            "b2_closure"
        } else {
            "b2_closure_v2"
        };
        if self.artifact_role != expected_role {
            return Err(B2ClosureContractError::new(
                B2ClosureContractErrorCode::CurrentRootRoleMismatch,
                "role/version pair is not admitted",
            ));
        }
        let (expected_path, expected_schema) = if self.closure_version == "v1" {
            (B2_CLOSURE_V1_PATH, B2_CLOSURE_V1_SCHEMA)
        } else {
            (B2_CLOSURE_V2_PATH, B2_CLOSURE_V2_SCHEMA)
        };
        if self.repository_relative_path != expected_path {
            return Err(B2ClosureContractError::new(
                B2ClosureContractErrorCode::CurrentRootPathMismatch,
                "current-root path does not match role/version",
            ));
        }
        if self.closure_schema_id != expected_schema {
            return Err(B2ClosureContractError::new(
                B2ClosureContractErrorCode::CurrentRootSchemaMismatch,
                "closure schema does not match role/version",
            ));
        }
        if !is_sha256_hex(&self.closure_raw_sha256)
            || (self.closure_version == "v1" && self.closure_raw_sha256 != B2_CLOSURE_V1_RAW_SHA256)
        {
            return Err(B2ClosureContractError::new(
                B2ClosureContractErrorCode::CurrentRootDigestMismatch,
                "closure raw SHA-256 is malformed or mismatched",
            ));
        }
        if !is_sha256_hex(&self.source_package_sha256)
            || self.source_package_sha256 != B2_CLOSURE_SOURCE_PACKAGE_SHA256
        {
            return Err(B2ClosureContractError::new(
                B2ClosureContractErrorCode::SourcePackageMismatch,
                "source package SHA-256 is malformed or mismatched",
            ));
        }
        Ok(self)
    }
}

pub fn validate_b2_closure_current_root_set(
    roots: &[B2ClosureCurrentRootV1],
) -> Result<&B2ClosureCurrentRootV1, B2ClosureContractError> {
    match roots {
        [] => Err(B2ClosureContractError::new(
            B2ClosureContractErrorCode::CurrentRootMissing,
            "no current root was admitted",
        )),
        [root] => root.validate(),
        _ => Err(B2ClosureContractError::new(
            B2ClosureContractErrorCode::MultipleCurrentRoots,
            "more than one current root was admitted",
        )),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct B2ClosureArtifactBindingV1 {
    pub artifact_role: String,
    pub repository_relative_path: String,
    pub schema_identifier: String,
    pub raw_sha256: String,
}

impl B2ClosureArtifactBindingV1 {
    pub fn validate(&self) -> Result<&Self, B2ClosureContractError> {
        let (path, schema, digest) = match self.artifact_role.as_str() {
            "b2_classifications_v1" => (
                "sources/m2_5/closures/B2/card_semantic_classifications.v1.json",
                "manafold.m2.5.b2.card-semantic-classifications.v1",
                "40cd5b9c37e26157a6df0449a75040f8a5879d825e3946dd500d666a502201d5",
            ),
            "b2_family_catalog_v1" => (
                "sources/m2_5/closures/B2/requirement_family_catalog.v1.json",
                "manafold.m2.5.b2.requirement-family-catalog.v1",
                "a9dc94b86a2efdb6885081191e53380cf5b3723a58487600b6372bcb789abb92",
            ),
            "b2_projection_v1" => (
                "sources/m2_5/closures/B2/deck_row_classification_refs.v1.csv",
                "manafold.m2.5.b2.deck-row-classification-refs.v1",
                "59a0f6ca00af6376fcce1d6c33c06ada3655c2c7fb3bf07354ab3643a96dba5a",
            ),
            _ => {
                return Err(B2ClosureContractError::new(
                    B2ClosureContractErrorCode::UnknownSourceRole,
                    "artifact role is not admitted",
                ))
            }
        };
        if self.repository_relative_path != path
            || self.schema_identifier != schema
            || self.raw_sha256 != digest
        {
            return Err(B2ClosureContractError::new(
                B2ClosureContractErrorCode::ReferencedSemanticArtifactMismatch,
                "artifact binding does not match the admitted registry",
            ));
        }
        if !is_sha256_hex(&self.raw_sha256) {
            return Err(B2ClosureContractError::new(
                B2ClosureContractErrorCode::ReferencedSemanticArtifactMismatch,
                "artifact digest is malformed",
            ));
        }
        Ok(self)
    }
}

pub fn validate_b2_closure_artifact_bindings(
    bindings: &[B2ClosureArtifactBindingV1],
) -> Result<(), B2ClosureContractError> {
    let mut roles = Vec::with_capacity(bindings.len());
    for binding in bindings {
        if roles.iter().any(|role| role == &binding.artifact_role) {
            return Err(B2ClosureContractError::new(
                B2ClosureContractErrorCode::DuplicateSourceRole,
                "artifact role occurs more than once",
            ));
        }
        roles.push(binding.artifact_role.as_str());
        binding.validate()?;
    }
    if roles.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(B2ClosureContractError::new(
            B2ClosureContractErrorCode::NoncanonicalSourceRoleOrder,
            "artifact roles are not in canonical order",
        ));
    }
    if roles.as_slice() != B2_CLOSURE_ARTIFACT_ROLES {
        return Err(B2ClosureContractError::new(
            B2ClosureContractErrorCode::ReferencedSemanticArtifactMismatch,
            "artifact role set is incomplete",
        ));
    }
    Ok(())
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}
