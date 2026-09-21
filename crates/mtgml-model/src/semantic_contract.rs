//! V5 semantic-contract vocabulary (spec §7, §7b, §7c, §8; ADR 0055 §2.6).
//!
//! Manifests are closed, deny-unknown wire objects mirroring the canonical
//! CBOR payloads. Capability closure entries bind exactly `(key, version)`;
//! the key/version grammar is frozen by the accepted capability registry
//! schema. Validation fails closed and never reorders caller input.

use serde::de::{Deserializer, Error as SerdeError, MapAccess, Visitor};
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

use crate::{ContentContractIdV1, FormatContractIdV1, RulesContractIdV1};

/// Closed rules authority (spec §7; ADR 0055 §2.6).
///
/// JSON: tagged object `{ "variant": "synthetic_legacy" }` or
/// `{ "variant": "comprehensive_rules", "snapshot_id": "<non-empty>" }`.
/// Decoding is strict per variant: unknown variants, missing payloads, and
/// payload-carrying `synthetic_legacy` objects all fail closed (spec §7b),
/// so deserialization is implemented manually instead of via serde's
/// internally-tagged derive (which ignores foreign fields on unit variants).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "variant", rename_all = "snake_case")]
pub enum RulesAuthorityV1 {
    SyntheticLegacy,
    ComprehensiveRules { snapshot_id: String },
}

const RULES_AUTHORITY_FIELDS: &[&str] = &["variant", "snapshot_id"];

impl<'de> Deserialize<'de> for RulesAuthorityV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct RulesAuthorityVisitor;

        impl<'de> Visitor<'de> for RulesAuthorityVisitor {
            type Value = RulesAuthorityV1;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a closed rules authority object")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut variant: Option<String> = None;
                let mut snapshot_id: Option<String> = None;
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "variant" => {
                            if variant.is_some() {
                                return Err(A::Error::duplicate_field("variant"));
                            }
                            variant = Some(map.next_value()?);
                        }
                        "snapshot_id" => {
                            if snapshot_id.is_some() {
                                return Err(A::Error::duplicate_field("snapshot_id"));
                            }
                            snapshot_id = Some(map.next_value()?);
                        }
                        unknown => {
                            return Err(A::Error::unknown_field(unknown, RULES_AUTHORITY_FIELDS))
                        }
                    }
                }
                match variant.as_deref() {
                    Some("synthetic_legacy") => {
                        if snapshot_id.is_some() {
                            return Err(A::Error::custom(
                                "synthetic_legacy authority must not carry snapshot_id",
                            ));
                        }
                        Ok(RulesAuthorityV1::SyntheticLegacy)
                    }
                    Some("comprehensive_rules") => {
                        let snapshot_id =
                            snapshot_id.ok_or_else(|| A::Error::missing_field("snapshot_id"))?;
                        Ok(RulesAuthorityV1::ComprehensiveRules { snapshot_id })
                    }
                    Some(other) => Err(A::Error::unknown_variant(
                        other,
                        &["synthetic_legacy", "comprehensive_rules"],
                    )),
                    None => Err(A::Error::missing_field("variant")),
                }
            }
        }

        deserializer.deserialize_struct(
            "RulesAuthorityV1",
            RULES_AUTHORITY_FIELDS,
            RulesAuthorityVisitor,
        )
    }
}

/// Semantic capability requirement binding exactly `(key, version)` (spec §7c).
///
/// No lifecycle state, owner, implementation path, conformance case, or
/// certification evidence belongs here; capability lifecycle stays registry
/// metadata and is never a digest input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityRequirementV1 {
    pub key: String,
    pub version: String,
}

/// Rules contract manifest (spec §7; ADR 0055 §2.6).
///
/// `capability_closure` is `None` ONLY for `SyntheticLegacy`;
/// `ComprehensiveRules` requires a non-empty closure sorted ascending
/// byte-wise by key with unique keys.
///
/// Deserialization is manual and strict (spec §7b): both properties are
/// required — an explicit `null` closure decodes to `None`, a missing
/// property rejects, unknown properties reject.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RulesContractManifestV1 {
    pub rules_authority: RulesAuthorityV1,
    pub capability_closure: Option<Vec<CapabilityRequirementV1>>,
}

const RULES_MANIFEST_FIELDS: &[&str] = &["rules_authority", "capability_closure"];

impl<'de> Deserialize<'de> for RulesContractManifestV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct RulesContractManifestVisitor;

        impl<'de> Visitor<'de> for RulesContractManifestVisitor {
            type Value = RulesContractManifestV1;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a rules contract manifest object")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut rules_authority: Option<RulesAuthorityV1> = None;
                let mut capability_closure: Option<Option<Vec<CapabilityRequirementV1>>> = None;
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "rules_authority" => {
                            if rules_authority.is_some() {
                                return Err(A::Error::duplicate_field("rules_authority"));
                            }
                            rules_authority = Some(map.next_value()?);
                        }
                        "capability_closure" => {
                            if capability_closure.is_some() {
                                return Err(A::Error::duplicate_field("capability_closure"));
                            }
                            capability_closure = Some(map.next_value()?);
                        }
                        unknown => {
                            return Err(A::Error::unknown_field(unknown, RULES_MANIFEST_FIELDS));
                        }
                    }
                }
                let rules_authority =
                    rules_authority.ok_or_else(|| A::Error::missing_field("rules_authority"))?;
                let capability_closure = capability_closure
                    .ok_or_else(|| A::Error::missing_field("capability_closure"))?;
                Ok(RulesContractManifestV1 {
                    rules_authority,
                    capability_closure,
                })
            }
        }

        deserializer.deserialize_struct(
            "RulesContractManifestV1",
            RULES_MANIFEST_FIELDS,
            RulesContractManifestVisitor,
        )
    }
}

/// Typed failure family of [`RulesContractManifestV1::validate`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum RulesContractManifestValidationError {
    #[error("synthetic_legacy authority must not claim a capability closure")]
    SyntheticLegacyClosurePresent,
    #[error("comprehensive_rules authority requires a non-empty capability closure")]
    ComprehensiveRulesClosureMissing,
    #[error("comprehensive_rules snapshot identity must not be empty")]
    EmptySnapshotId,
    #[error("capability closure entry key does not match the frozen key grammar")]
    CapabilityKeyInvalid,
    #[error("capability closure entry version does not match the frozen version grammar")]
    CapabilityVersionInvalid,
    #[error("capability closure contains a duplicate key")]
    CapabilityClosureDuplicateKey,
    #[error("capability closure is not sorted ascending byte-wise by key")]
    CapabilityClosureNotSorted,
}

impl RulesContractManifestV1 {
    /// Fail-closed structural validation (spec §7, §7c).
    ///
    /// Canonical order is an invariant that must already hold: caller input
    /// is never silently sorted.
    pub fn validate(&self) -> Result<(), RulesContractManifestValidationError> {
        match &self.rules_authority {
            RulesAuthorityV1::SyntheticLegacy => {
                if self.capability_closure.is_some() {
                    return Err(
                        RulesContractManifestValidationError::SyntheticLegacyClosurePresent,
                    );
                }
            }
            RulesAuthorityV1::ComprehensiveRules { snapshot_id } => {
                if snapshot_id.is_empty() {
                    return Err(RulesContractManifestValidationError::EmptySnapshotId);
                }
                let Some(closure) = self.capability_closure.as_ref() else {
                    return Err(
                        RulesContractManifestValidationError::ComprehensiveRulesClosureMissing,
                    );
                };
                if closure.is_empty() {
                    return Err(
                        RulesContractManifestValidationError::ComprehensiveRulesClosureMissing,
                    );
                }
                let mut previous_key: Option<&str> = None;
                for requirement in closure {
                    if !is_valid_capability_key(&requirement.key) {
                        return Err(RulesContractManifestValidationError::CapabilityKeyInvalid);
                    }
                    if !is_valid_capability_version(&requirement.version) {
                        return Err(RulesContractManifestValidationError::CapabilityVersionInvalid);
                    }
                    match previous_key {
                        Some(previous) if previous == requirement.key.as_str() => {
                            return Err(
                                RulesContractManifestValidationError::CapabilityClosureDuplicateKey,
                            );
                        }
                        Some(previous) if previous > requirement.key.as_str() => {
                            return Err(
                                RulesContractManifestValidationError::CapabilityClosureNotSorted,
                            );
                        }
                        _ => {}
                    }
                    previous_key = Some(requirement.key.as_str());
                }
            }
        }
        Ok(())
    }
}

/// Frozen key grammar (spec §7c, from `schemas/capability-registry.v1.schema.json`):
/// `^(rules|mechanic|decision|visibility|tooling|format/[a-z0-9-]+)/[a-z0-9][a-z0-9-]*(/[a-z0-9][a-z0-9-]*)*$`
fn is_valid_capability_key(key: &str) -> bool {
    const FIXED_HEADS: [&str; 5] = ["rules", "mechanic", "decision", "visibility", "tooling"];

    let mut segments = key.split('/');
    let head = segments.next().unwrap_or_default();
    let second = segments.next().unwrap_or_default();
    if second.is_empty() {
        return false;
    }
    if head == "format" {
        // `format/[a-z0-9-]+` is the namespace head alternative; the grammar
        // still requires at least one following capability segment, so a bare
        // `format/<namespace>` key is NOT in the regex language.
        if !is_format_namespace(second) {
            return false;
        }
        match segments.next() {
            Some(first) if is_word_segment(first) => segments.all(is_word_segment),
            _ => false,
        }
    } else if FIXED_HEADS.contains(&head) {
        is_word_segment(second) && segments.all(is_word_segment)
    } else {
        false
    }
}

/// `[a-z0-9-]+` format namespace segment.
fn is_format_namespace(segment: &str) -> bool {
    !segment.is_empty()
        && segment
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

/// `[a-z0-9][a-z0-9-]*` segment.
fn is_word_segment(segment: &str) -> bool {
    let mut chars = segment.chars();
    match chars.next() {
        Some(first) if first.is_ascii_lowercase() || first.is_ascii_digit() => {}
        _ => return false,
    }
    chars.all(|char| char.is_ascii_lowercase() || char.is_ascii_digit() || char == '-')
}

/// Frozen version grammar (spec §7c): `^[0-9]+\.[0-9]+\.[0-9]+$`.
fn is_valid_capability_version(version: &str) -> bool {
    let mut parts = 0;
    for part in version.split('.') {
        parts += 1;
        if parts > 3 || part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()) {
            return false;
        }
    }
    parts == 3
}

/// Semantic contract manifest (spec §8; ADR 0055 §2.6).
///
/// The format/content dimensions are typed seams that stay `None` for the
/// V5 slice; explicit `null` is their canonical JSON rendering.
///
/// Deserialization is manual and strict (spec §7b): all three properties are
/// required — explicit `null` decodes to `None`, a missing property rejects,
/// unknown properties reject.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SemanticContractManifestV1 {
    pub rules_contract_id: RulesContractIdV1,
    pub format_contract_id: Option<FormatContractIdV1>,
    pub content_contract_id: Option<ContentContractIdV1>,
}

const SEMANTIC_MANIFEST_FIELDS: &[&str] = &[
    "rules_contract_id",
    "format_contract_id",
    "content_contract_id",
];

impl<'de> Deserialize<'de> for SemanticContractManifestV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct SemanticContractManifestVisitor;

        impl<'de> Visitor<'de> for SemanticContractManifestVisitor {
            type Value = SemanticContractManifestV1;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a semantic contract manifest object")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut rules_contract_id: Option<RulesContractIdV1> = None;
                let mut format_contract_id: Option<Option<FormatContractIdV1>> = None;
                let mut content_contract_id: Option<Option<ContentContractIdV1>> = None;
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "rules_contract_id" => {
                            if rules_contract_id.is_some() {
                                return Err(A::Error::duplicate_field("rules_contract_id"));
                            }
                            rules_contract_id = Some(map.next_value()?);
                        }
                        "format_contract_id" => {
                            if format_contract_id.is_some() {
                                return Err(A::Error::duplicate_field("format_contract_id"));
                            }
                            format_contract_id = Some(map.next_value()?);
                        }
                        "content_contract_id" => {
                            if content_contract_id.is_some() {
                                return Err(A::Error::duplicate_field("content_contract_id"));
                            }
                            content_contract_id = Some(map.next_value()?);
                        }
                        unknown => {
                            return Err(A::Error::unknown_field(unknown, SEMANTIC_MANIFEST_FIELDS));
                        }
                    }
                }
                let rules_contract_id = rules_contract_id
                    .ok_or_else(|| A::Error::missing_field("rules_contract_id"))?;
                let format_contract_id = format_contract_id
                    .ok_or_else(|| A::Error::missing_field("format_contract_id"))?;
                let content_contract_id = content_contract_id
                    .ok_or_else(|| A::Error::missing_field("content_contract_id"))?;
                Ok(SemanticContractManifestV1 {
                    rules_contract_id,
                    format_contract_id,
                    content_contract_id,
                })
            }
        }

        deserializer.deserialize_struct(
            "SemanticContractManifestV1",
            SEMANTIC_MANIFEST_FIELDS,
            SemanticContractManifestVisitor,
        )
    }
}
