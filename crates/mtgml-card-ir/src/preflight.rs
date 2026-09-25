//! Fail-closed content validation through the canonical Capability Registry.

use crate::{ContentValidationErrorV1, DefinitionClosureErrorV1, VerifiedContentCatalogV1};
use mtgml_model::{CapabilityRequirementV1, CardDefinitionId, ContentContractIdV1};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

const CANONICAL_CAPABILITY_REGISTRY: &str =
    include_str!("../../../cards/capabilities/registry.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RequiredCapabilityLifecycleV1 {
    Proposed,
    Specified,
    Implemented,
    Covered,
    Certified,
}

impl RequiredCapabilityLifecycleV1 {
    fn rank(self) -> i8 {
        match self {
            Self::Proposed => 0,
            Self::Specified => 1,
            Self::Implemented => 2,
            Self::Covered => 3,
            Self::Certified => 4,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentValidationReportV1 {
    pub content_contract_id: ContentContractIdV1,
    pub reachable_definitions: Vec<CardDefinitionId>,
    pub direct_requirement_roots: Vec<CapabilityRequirementV1>,
    pub resolved_capabilities: Vec<CapabilityRequirementV1>,
    pub required_lifecycle: RequiredCapabilityLifecycleV1,
    pub authorization: ContentAuthorizationV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentAuthorizationV1 {
    ValidationOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ContentPreflightErrorV1 {
    #[error("content structure is invalid")]
    InvalidContent(ContentValidationErrorV1),
    #[error("content digest does not match the supplied identity")]
    ContentIdentityMismatch,
    #[error("provenance does not exactly match content definitions")]
    ProvenanceCatalogMismatch,
    #[error("definition closure failed")]
    DefinitionClosure(DefinitionClosureErrorV1),
    #[error("the canonical capability registry is invalid")]
    InvalidCapabilityRegistry,
    #[error("capability key is absent from the canonical registry")]
    UnknownCapability { key: String },
    #[error("capability version does not match the canonical registry")]
    UnknownCapabilityVersion { key: String, requested: String },
    #[error("different versions were requested for one capability key")]
    CapabilityVersionConflict { key: String },
    #[error("capability dependency closure failed")]
    CapabilityDependencyFailure { path: Vec<String> },
    #[error("capability is below the requested lifecycle threshold")]
    CapabilityLifecycleBelowRequirement { key: String, lifecycle: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("M4.1 content validation does not admit executable profiles")]
pub struct NoExecutableProfileAdmitted;

pub fn construct_gameplay_from_content(
    _report: &ContentValidationReportV1,
) -> Result<(), NoExecutableProfileAdmitted> {
    Err(NoExecutableProfileAdmitted)
}

pub fn content_validation_only(
    canonical_manifest: &[u8],
    supplied_content_contract_id: &ContentContractIdV1,
    canonical_provenance: &[u8],
    roots: &[CardDefinitionId],
    required_lifecycle: RequiredCapabilityLifecycleV1,
) -> Result<ContentValidationReportV1, ContentPreflightErrorV1> {
    let catalog = VerifiedContentCatalogV1::build_from_bytes(
        canonical_manifest,
        supplied_content_contract_id,
        canonical_provenance,
    )
    .map_err(|error| match error {
        crate::CatalogBuildErrorV1::InvalidManifest(error) => {
            ContentPreflightErrorV1::InvalidContent(error)
        }
        crate::CatalogBuildErrorV1::ContentIdentityMismatch => {
            ContentPreflightErrorV1::ContentIdentityMismatch
        }
        crate::CatalogBuildErrorV1::ProvenanceCatalogMismatch => {
            ContentPreflightErrorV1::ProvenanceCatalogMismatch
        }
    })?;
    let reachable_definitions = catalog
        .close_definition_roots(supplied_content_contract_id, roots)
        .map_err(ContentPreflightErrorV1::DefinitionClosure)?;

    // UnprofiledV1 contains no executable semantics and therefore derives no
    // capability roots. Explicit roots are additive and references contribute
    // further definitions whose own explicit roots are included here.
    let mut derived = Vec::<CapabilityRequirementV1>::new();
    let mut explicit = Vec::<CapabilityRequirementV1>::new();
    for id in &reachable_definitions {
        let definition = catalog
            .get(supplied_content_contract_id, *id)
            .map_err(|_| {
                ContentPreflightErrorV1::DefinitionClosure(
                    DefinitionClosureErrorV1::MissingDefinition { id: *id },
                )
            })?;
        derived.extend(derived_requirement_roots(definition));
        explicit.extend(definition.explicit_additional_requirements.iter().cloned());
    }
    let direct_requirement_roots = normalize_requirement_roots(derived, explicit)?;
    let registry = parse_canonical_registry()?;
    let resolved_capabilities = registry.resolve(&direct_requirement_roots, required_lifecycle)?;
    Ok(ContentValidationReportV1 {
        content_contract_id: catalog.content_contract_id().clone(),
        reachable_definitions,
        direct_requirement_roots,
        resolved_capabilities,
        required_lifecycle,
        authorization: ContentAuthorizationV1::ValidationOnly,
    })
}

fn derived_requirement_roots(
    _definition: &crate::CardDefinitionEnvelopeV1,
) -> Vec<CapabilityRequirementV1> {
    // M4.1 admits only UnprofiledV1, which carries no semantic requirements.
    // A later reviewed profile owns its closed derivation descriptor.
    Vec::new()
}

fn normalize_requirement_roots(
    derived_roots: Vec<CapabilityRequirementV1>,
    explicit_roots: Vec<CapabilityRequirementV1>,
) -> Result<Vec<CapabilityRequirementV1>, ContentPreflightErrorV1> {
    let mut by_key = BTreeMap::<String, String>::new();
    for root in derived_roots.into_iter().chain(explicit_roots) {
        if let Some(previous) = by_key.get(&root.key) {
            if previous != &root.version {
                return Err(ContentPreflightErrorV1::CapabilityVersionConflict { key: root.key });
            }
        } else {
            by_key.insert(root.key, root.version);
        }
    }
    Ok(by_key
        .into_iter()
        .map(|(key, version)| CapabilityRequirementV1 { key, version })
        .collect())
}

#[derive(Debug, Deserialize)]
struct RegistryFile {
    schema_version: String,
    entries: Vec<RegistryEntry>,
}

#[derive(Debug, Deserialize)]
struct RegistryEntry {
    key: String,
    version: String,
    lifecycle: String,
    #[serde(default)]
    dependencies: Vec<String>,
}

#[derive(Debug)]
struct CapabilityRegistry {
    entries: BTreeMap<String, RegistryEntry>,
}

fn parse_canonical_registry() -> Result<CapabilityRegistry, ContentPreflightErrorV1> {
    let file: RegistryFile = serde_json::from_str(CANONICAL_CAPABILITY_REGISTRY)
        .map_err(|_| ContentPreflightErrorV1::InvalidCapabilityRegistry)?;
    if file.schema_version != "capability-registry.v1" {
        return Err(ContentPreflightErrorV1::InvalidCapabilityRegistry);
    }
    let mut entries = BTreeMap::new();
    for mut entry in file.entries {
        entry.dependencies.sort_unstable();
        if !valid_capability_key(&entry.key)
            || !valid_version(&entry.version)
            || lifecycle_rank(&entry.lifecycle).is_none()
            || entries.insert(entry.key.clone(), entry).is_some()
        {
            return Err(ContentPreflightErrorV1::InvalidCapabilityRegistry);
        }
    }
    for entry in entries.values() {
        let mut seen = BTreeSet::new();
        if entry.dependencies.iter().any(|key| !seen.insert(key)) {
            return Err(ContentPreflightErrorV1::InvalidCapabilityRegistry);
        }
        for dependency in &entry.dependencies {
            if !entries.contains_key(dependency) {
                return Err(ContentPreflightErrorV1::CapabilityDependencyFailure {
                    path: vec![entry.key.clone(), dependency.clone()],
                });
            }
        }
    }
    let registry = CapabilityRegistry { entries };
    registry.validate_acyclic()?;
    Ok(registry)
}

impl CapabilityRegistry {
    fn validate_acyclic(&self) -> Result<(), ContentPreflightErrorV1> {
        let mut states = BTreeMap::<String, u8>::new();
        let mut active = Vec::new();
        for key in self.entries.keys() {
            self.visit(key, &mut states, &mut active, &mut BTreeSet::new())?;
        }
        Ok(())
    }

    fn resolve(
        &self,
        roots: &[CapabilityRequirementV1],
        required: RequiredCapabilityLifecycleV1,
    ) -> Result<Vec<CapabilityRequirementV1>, ContentPreflightErrorV1> {
        let mut states = BTreeMap::<String, u8>::new();
        let mut active = Vec::new();
        let mut reached = BTreeSet::new();
        for root in roots {
            let Some(entry) = self.entries.get(&root.key) else {
                return Err(ContentPreflightErrorV1::UnknownCapability {
                    key: root.key.clone(),
                });
            };
            if entry.version != root.version {
                return Err(ContentPreflightErrorV1::UnknownCapabilityVersion {
                    key: root.key.clone(),
                    requested: root.version.clone(),
                });
            }
            self.visit(&root.key, &mut states, &mut active, &mut reached)?;
        }
        let mut closure = Vec::new();
        for key in reached {
            let entry = &self.entries[&key];
            if lifecycle_rank(&entry.lifecycle).unwrap_or(-1) < required.rank() {
                return Err(
                    ContentPreflightErrorV1::CapabilityLifecycleBelowRequirement {
                        key,
                        lifecycle: entry.lifecycle.clone(),
                    },
                );
            }
            closure.push(CapabilityRequirementV1 {
                key: entry.key.clone(),
                version: entry.version.clone(),
            });
        }
        Ok(closure)
    }

    fn visit(
        &self,
        key: &str,
        states: &mut BTreeMap<String, u8>,
        active: &mut Vec<String>,
        reached: &mut BTreeSet<String>,
    ) -> Result<(), ContentPreflightErrorV1> {
        match states.get(key).copied() {
            Some(2) => return Ok(()),
            Some(1) => {
                let index = active.iter().position(|entry| entry == key).unwrap_or(0);
                let mut path = active[index..].to_vec();
                path.push(key.to_owned());
                return Err(ContentPreflightErrorV1::CapabilityDependencyFailure { path });
            }
            _ => {}
        }
        let Some(entry) = self.entries.get(key) else {
            let mut path = active.clone();
            path.push(key.to_owned());
            return Err(ContentPreflightErrorV1::CapabilityDependencyFailure { path });
        };
        states.insert(key.to_owned(), 1);
        active.push(key.to_owned());
        reached.insert(key.to_owned());
        let mut dependencies = entry.dependencies.clone();
        dependencies.sort();
        for dependency in dependencies {
            self.visit(&dependency, states, active, reached)?;
        }
        active.pop();
        states.insert(key.to_owned(), 2);
        Ok(())
    }
}

fn lifecycle_rank(value: &str) -> Option<i8> {
    match value {
        "proposed" => Some(0),
        "specified" => Some(1),
        "implemented" => Some(2),
        "covered" => Some(3),
        "certified" => Some(4),
        "deprecated" => Some(-1),
        _ => None,
    }
}

fn valid_version(value: &str) -> bool {
    let parts = value.split('.').collect::<Vec<_>>();
    parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
}

fn valid_capability_key(value: &str) -> bool {
    let segments = value.split('/').collect::<Vec<_>>();
    let word = |segment: &str| {
        let mut bytes = segment.bytes();
        bytes
            .next()
            .is_some_and(|first| first.is_ascii_lowercase() || first.is_ascii_digit())
            && bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    };
    match segments.as_slice() {
        [head, rest @ ..]
            if ["rules", "mechanic", "decision", "visibility", "tooling"].contains(head) =>
        {
            !rest.is_empty() && rest.iter().all(|segment| word(segment))
        }
        ["format", namespace, rest @ ..] => {
            !namespace.is_empty()
                && namespace
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
                && !rest.is_empty()
                && rest.iter().all(|segment| word(segment))
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_only_profile_derived_requirement_cannot_be_suppressed_by_omission() {
        // This closed synthetic descriptor models a future reviewed profile;
        // it is not registered or admitted in production.
        struct TestOnlyProfileDescriptor {
            mandatory_requirement: CapabilityRequirementV1,
        }
        let descriptor = TestOnlyProfileDescriptor {
            mandatory_requirement: CapabilityRequirementV1 {
                key: "rules/test-only-derived".to_owned(),
                version: "1.0.0".to_owned(),
            },
        };
        let known_derived_root = vec![descriptor.mandatory_requirement];
        let normalized = normalize_requirement_roots(known_derived_root.clone(), vec![])
            .expect("omitting explicit roots must preserve derived roots");
        assert_eq!(normalized, known_derived_root);
    }

    #[test]
    fn registry_dependency_cycle_has_deterministic_path() {
        let registry = CapabilityRegistry {
            entries: BTreeMap::from([
                (
                    "rules/alpha".to_owned(),
                    RegistryEntry {
                        key: "rules/alpha".to_owned(),
                        version: "1.0.0".to_owned(),
                        lifecycle: "specified".to_owned(),
                        dependencies: vec!["rules/beta".to_owned()],
                    },
                ),
                (
                    "rules/beta".to_owned(),
                    RegistryEntry {
                        key: "rules/beta".to_owned(),
                        version: "1.0.0".to_owned(),
                        lifecycle: "specified".to_owned(),
                        dependencies: vec!["rules/alpha".to_owned()],
                    },
                ),
            ]),
        };
        assert_eq!(
            registry.validate_acyclic(),
            Err(ContentPreflightErrorV1::CapabilityDependencyFailure {
                path: vec![
                    "rules/alpha".to_owned(),
                    "rules/beta".to_owned(),
                    "rules/alpha".to_owned(),
                ]
            })
        );
    }
}
