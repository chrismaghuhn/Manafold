//! Fail-closed content validation through the canonical Capability Registry.

use crate::{
    BasicLandSubtypeV1, CardSemanticBindingV1, ContentContractManifestV1,
    ContentValidationDiagnosticV1, DefinitionClosureErrorV1, ProvenanceCatalogV1,
    VerifiedContentCatalogV1, BASIC_LAND_PROFILE_ID_V1,
};
use mtgml_model::{
    execution_program_matches_rules_authority, CapabilityRequirementV1, CardDefinitionId,
    ContentContractIdV1, ExecutionIdentityV1, ExecutionProgramV1, RulesAuthorityV1,
    RulesContractManifestV1, SemanticContractIdV1, SemanticContractManifestV1,
};
use mtgml_persistence::semantic_contract_digest::{
    calculate_rules_contract_id_v1, calculate_semantic_contract_id_v1,
};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

const GENERATED_CAPABILITY_REGISTRY: &str = include_str!("generated_capability_registry.json");
const BASIC_LAND_RULES_SNAPSHOT: &str = "wotc-cr-2026-09-25-txt-20260925-sha256-8d860e451f20f38865b725b42d82feb714c725373dd8f3b32b8652b3eeb070ca";
const ORACLE_SNAPSHOT: &str = "oracle-cards-20260925210158";
const ORACLE_RECORD_CODEC: &str = "scryfall.oracle-card-jsonl-record.v1";

const PROFILE_REQUIREMENT_ROOTS: [(&str, &str); 3] = [
    ("rules/basic-land-mana", "0.1.0"),
    ("rules/land-play", "0.1.0"),
    ("rules/mana-pool", "0.1.0"),
];

/// Rules every executable Magic game needs regardless of its cards: the turn
/// sequence, priority, drawing (and losing on an empty library), the combat
/// phase with attacker declaration, and cleanup. Card profiles add their own roots on top of these.
pub const MAGIC_GAME_RULE_ROOTS: &[(&str, &str)] = &[
    ("rules/basic-priority", "0.1.0"),
    ("rules/cleanup-reset", "0.1.0"),
    ("rules/combat-phase", "0.1.0"),
    ("rules/declare-attackers", "0.1.0"),
    ("rules/draw-card", "0.1.0"),
    ("rules/state-based-actions-empty-library", "0.1.0"),
    ("rules/turn-structure", "0.1.0"),
];

const BASIC_LAND_SOURCE_RECORDS: [(&str, BasicLandSubtypeV1, &str); 2] = [
    (
        "a3fb7228-e76b-4e96-a40e-20b5fed75685",
        BasicLandSubtypeV1::Mountain,
        "b57d8ce5dcbbb01e1c128aaa9ebeab8a26d5f297edb51eac6adce5c4af770033",
    ),
    (
        "bc71ebf6-2056-41f7-be35-b2e5c34afa99",
        BasicLandSubtypeV1::Plains,
        "af82e883368b8211c1845af680e1b4dab52666b41969cc6987bffdde7ada86b7",
    ),
];

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

/// Verified admission of the closed profile for a later RulesKernel
/// integration. This is an identity/requirement token only: it does not
/// construct a game, execute a transition, advance capability lifecycle, or
/// claim coverage/certification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutableProfileAdmissionV1 {
    content_contract_id: ContentContractIdV1,
    verified_catalog: VerifiedContentCatalogV1,
    direct_requirement_roots: Vec<CapabilityRequirementV1>,
    resolved_capabilities: Vec<CapabilityRequirementV1>,
    rules_contract_manifest: RulesContractManifestV1,
    semantic_contract_manifest: SemanticContractManifestV1,
    semantic_contract_id: SemanticContractIdV1,
    execution_identity: ExecutionIdentityV1,
}

impl ExecutableProfileAdmissionV1 {
    pub fn content_contract_id(&self) -> &ContentContractIdV1 {
        &self.content_contract_id
    }

    /// Immutable definitions whose identity, provenance, profile, and
    /// requirement closure were verified by this admission. Runtime
    /// construction consumes this catalog together with the identity chain;
    /// callers cannot substitute a second catalog after admission.
    pub fn verified_catalog(&self) -> &VerifiedContentCatalogV1 {
        &self.verified_catalog
    }

    pub fn direct_requirement_roots(&self) -> &[CapabilityRequirementV1] {
        &self.direct_requirement_roots
    }

    pub fn resolved_capabilities(&self) -> &[CapabilityRequirementV1] {
        &self.resolved_capabilities
    }

    pub fn rules_contract_manifest(&self) -> &RulesContractManifestV1 {
        &self.rules_contract_manifest
    }

    pub fn semantic_contract_manifest(&self) -> &SemanticContractManifestV1 {
        &self.semantic_contract_manifest
    }

    pub fn semantic_contract_id(&self) -> &SemanticContractIdV1 {
        &self.semantic_contract_id
    }

    pub fn execution_identity(&self) -> &ExecutionIdentityV1 {
        &self.execution_identity
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentAuthorizationV1 {
    ValidationOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ContentPreflightErrorV1 {
    #[error("content structure is invalid")]
    InvalidContent(ContentValidationDiagnosticV1),
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
    #[error("the closed executable profile is not admitted by this content catalog")]
    ExecutableProfileNotAdmitted,
    #[error("profile source provenance does not match the pinned Oracle records")]
    PinnedSourceProvenanceMismatch,
    #[error("the rules contract does not use the pinned comprehensive-rules snapshot")]
    RulesSnapshotMismatch,
    #[error("the supplied capability closure is not the complete profile closure")]
    CapabilityClosureMismatch,
    #[error("RulesContractIdV1 does not match the supplied rules manifest")]
    RulesIdentityMismatch,
    #[error("SemanticContractManifestV1 does not bind the verified rules/content identities")]
    SemanticManifestMismatch,
    #[error("SemanticContractIdV1 does not match the supplied semantic manifest")]
    SemanticIdentityMismatch,
    #[error("ExecutionIdentityV1 does not bind the admitted Magic semantic identity")]
    ExecutionIdentityMismatch,
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

    // Profile-derived roots are mandatory. Caller-declared roots are
    // additive only; omitting them can never suppress profile requirements.
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
    definition: &crate::CardDefinitionEnvelopeV1,
) -> Vec<CapabilityRequirementV1> {
    match &definition.semantic_binding {
        CardSemanticBindingV1::ProfiledV1 { profile_id, .. }
            if profile_id.as_str() == BASIC_LAND_PROFILE_ID_V1 =>
        {
            PROFILE_REQUIREMENT_ROOTS
                .iter()
                .map(|(key, version)| CapabilityRequirementV1 {
                    key: (*key).to_owned(),
                    version: (*version).to_owned(),
                })
                .collect()
        }
        _ => Vec::new(),
    }
}

/// Admit the exact Mountain/Plains profile for later executable integration
/// after provenance, recursive requirements, and the Rules/Semantic/
/// Execution identity chain all verify.
pub fn admit_executable_profile_v1(
    canonical_manifest: &[u8],
    supplied_content_contract_id: &ContentContractIdV1,
    canonical_provenance: &[u8],
    rules_manifest: &RulesContractManifestV1,
    semantic_manifest: &SemanticContractManifestV1,
    execution_identity: &ExecutionIdentityV1,
) -> Result<ExecutableProfileAdmissionV1, ContentPreflightErrorV1> {
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
    let manifest = crate::decode_content_manifest_v1(canonical_manifest)
        .map_err(ContentPreflightErrorV1::InvalidContent)?;
    let provenance = crate::decode_provenance_catalog_v1(canonical_provenance)
        .map_err(|_| ContentPreflightErrorV1::ProvenanceCatalogMismatch)?;
    validate_pinned_basic_land_profile(&manifest, &provenance, supplied_content_contract_id)?;

    let profile_definition_roots = manifest
        .definitions
        .iter()
        .map(|definition| definition.card_definition_id)
        .collect::<Vec<_>>();
    let report = content_validation_only(
        canonical_manifest,
        supplied_content_contract_id,
        canonical_provenance,
        &profile_definition_roots,
        RequiredCapabilityLifecycleV1::Specified,
    )?;
    // An executable game needs its content plus MAGIC_GAME_RULE_ROOTS; the
    // rules manifest must declare exactly that closure.
    let game_roots = MAGIC_GAME_RULE_ROOTS
        .iter()
        .map(|(key, version)| CapabilityRequirementV1 {
            key: (*key).to_owned(),
            version: (*version).to_owned(),
        })
        .collect();
    let direct_requirement_roots =
        normalize_requirement_roots(report.direct_requirement_roots, game_roots)?;
    let resolved_capabilities = parse_canonical_registry()?.resolve(
        &direct_requirement_roots,
        RequiredCapabilityLifecycleV1::Specified,
    )?;

    if rules_manifest.validate().is_err()
        || !matches!(
            &rules_manifest.rules_authority,
            RulesAuthorityV1::ComprehensiveRules { snapshot_id }
                if snapshot_id == BASIC_LAND_RULES_SNAPSHOT
        )
    {
        return Err(ContentPreflightErrorV1::RulesSnapshotMismatch);
    }
    if !execution_program_matches_rules_authority(
        execution_identity.program_kind,
        &rules_manifest.rules_authority,
    ) || execution_identity.program_kind != ExecutionProgramV1::MagicRules
    {
        return Err(ContentPreflightErrorV1::ExecutionIdentityMismatch);
    }

    if rules_manifest.capability_closure.as_ref() != Some(&resolved_capabilities) {
        return Err(ContentPreflightErrorV1::CapabilityClosureMismatch);
    }
    let rules_contract_id = calculate_rules_contract_id_v1(rules_manifest)
        .map_err(|_| ContentPreflightErrorV1::RulesIdentityMismatch)?;
    if semantic_manifest.rules_contract_id != rules_contract_id
        || semantic_manifest.format_contract_id.is_some()
        || semantic_manifest.content_contract_id.as_ref() != Some(supplied_content_contract_id)
    {
        return Err(ContentPreflightErrorV1::SemanticManifestMismatch);
    }
    let semantic_contract_id = calculate_semantic_contract_id_v1(semantic_manifest)
        .map_err(|_| ContentPreflightErrorV1::SemanticIdentityMismatch)?;
    if execution_identity.semantic_contract_id != semantic_contract_id {
        return Err(ContentPreflightErrorV1::ExecutionIdentityMismatch);
    }

    Ok(ExecutableProfileAdmissionV1 {
        content_contract_id: catalog.content_contract_id().clone(),
        verified_catalog: catalog,
        direct_requirement_roots,
        resolved_capabilities,
        rules_contract_manifest: rules_manifest.clone(),
        semantic_contract_manifest: semantic_manifest.clone(),
        semantic_contract_id,
        execution_identity: execution_identity.clone(),
    })
}

fn validate_pinned_basic_land_profile(
    manifest: &ContentContractManifestV1,
    provenance: &ProvenanceCatalogV1,
    content_contract_id: &ContentContractIdV1,
) -> Result<(), ContentPreflightErrorV1> {
    if manifest.definitions.len() != BASIC_LAND_SOURCE_RECORDS.len()
        || provenance.records.len() != BASIC_LAND_SOURCE_RECORDS.len()
    {
        return Err(ContentPreflightErrorV1::ExecutableProfileNotAdmitted);
    }
    let mut found_mountain = false;
    let mut found_plains = false;
    for record in &provenance.records {
        if &record.content_contract_id != content_contract_id
            || record.source_provenance.source_snapshot_id != ORACLE_SNAPSHOT
            || record.source_provenance.source_record_codec_id != ORACLE_RECORD_CODEC
        {
            return Err(ContentPreflightErrorV1::PinnedSourceProvenanceMismatch);
        }
        let Some((_, subtype, digest)) = BASIC_LAND_SOURCE_RECORDS
            .iter()
            .find(|(oracle_id, _, _)| *oracle_id == record.source_provenance.source_record_id)
        else {
            return Err(ContentPreflightErrorV1::PinnedSourceProvenanceMismatch);
        };
        let definition = manifest
            .definitions
            .iter()
            .find(|definition| definition.card_definition_id == record.card_definition_id)
            .ok_or(ContentPreflightErrorV1::PinnedSourceProvenanceMismatch)?;
        let profile_subtype = match &definition.semantic_binding {
            CardSemanticBindingV1::ProfiledV1 { profile_id, body }
                if profile_id.as_str() == BASIC_LAND_PROFILE_ID_V1 =>
            {
                body.subtype
            }
            _ => return Err(ContentPreflightErrorV1::ExecutableProfileNotAdmitted),
        };
        let already_found = match profile_subtype {
            BasicLandSubtypeV1::Mountain => std::mem::replace(&mut found_mountain, true),
            BasicLandSubtypeV1::Plains => std::mem::replace(&mut found_plains, true),
        };
        if profile_subtype != *subtype
            || record.source_provenance.source_record_digest != decode_digest(digest)
            || already_found
        {
            return Err(ContentPreflightErrorV1::PinnedSourceProvenanceMismatch);
        }
    }
    if !found_mountain || !found_plains {
        return Err(ContentPreflightErrorV1::PinnedSourceProvenanceMismatch);
    }
    Ok(())
}

fn decode_digest(value: &str) -> [u8; 32] {
    let mut digest = [0; 32];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        let digit = |byte: u8| match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            _ => unreachable!("pinned digest constant must be lowercase hex"),
        };
        digest[index] = (digit(pair[0]) << 4) | digit(pair[1]);
    }
    digest
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
    registry_id: String,
    entries: Vec<RegistryEntry>,
}

#[derive(Debug, Deserialize)]
struct RegistryEntry {
    key: String,
    version: String,
    lifecycle: String,
    lifecycle_rank: i8,
    #[serde(default)]
    dependencies: Vec<String>,
}

#[derive(Debug)]
struct CapabilityRegistry {
    entries: BTreeMap<String, RegistryEntry>,
}

fn parse_canonical_registry() -> Result<CapabilityRegistry, ContentPreflightErrorV1> {
    let file: RegistryFile = serde_json::from_str(GENERATED_CAPABILITY_REGISTRY)
        .map_err(|_| ContentPreflightErrorV1::InvalidCapabilityRegistry)?;
    if file.schema_version != "card-ir-capability-projection.v1" || file.registry_id.is_empty() {
        return Err(ContentPreflightErrorV1::InvalidCapabilityRegistry);
    }
    let mut entries = BTreeMap::new();
    for entry in file.entries {
        if entries.insert(entry.key.clone(), entry).is_some() {
            return Err(ContentPreflightErrorV1::InvalidCapabilityRegistry);
        }
    }
    Ok(CapabilityRegistry { entries })
}

impl CapabilityRegistry {
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
            if entry.lifecycle_rank < required.rank() {
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
                        lifecycle_rank: 1,
                        dependencies: vec!["rules/beta".to_owned()],
                    },
                ),
                (
                    "rules/beta".to_owned(),
                    RegistryEntry {
                        key: "rules/beta".to_owned(),
                        version: "1.0.0".to_owned(),
                        lifecycle: "specified".to_owned(),
                        lifecycle_rank: 1,
                        dependencies: vec!["rules/alpha".to_owned()],
                    },
                ),
            ]),
        };
        assert_eq!(
            registry.resolve(
                &[CapabilityRequirementV1 {
                    key: "rules/alpha".to_owned(),
                    version: "1.0.0".to_owned(),
                }],
                RequiredCapabilityLifecycleV1::Specified,
            ),
            Err(ContentPreflightErrorV1::CapabilityDependencyFailure {
                path: vec![
                    "rules/alpha".to_owned(),
                    "rules/beta".to_owned(),
                    "rules/alpha".to_owned(),
                ]
            })
        );
    }

    #[test]
    fn registry_rejects_uncovered_transitive_dependency() {
        let root = RegistryEntry {
            key: "rules/root".to_owned(),
            version: "1.0.0".to_owned(),
            lifecycle: "specified".to_owned(),
            lifecycle_rank: RequiredCapabilityLifecycleV1::Specified.rank(),
            dependencies: vec!["rules/missing-transitive".to_owned()],
        };
        let registry = CapabilityRegistry {
            entries: BTreeMap::from([(root.key.clone(), root)]),
        };

        assert_eq!(
            registry.resolve(
                &[CapabilityRequirementV1 {
                    key: "rules/root".to_owned(),
                    version: "1.0.0".to_owned(),
                }],
                RequiredCapabilityLifecycleV1::Specified,
            ),
            Err(ContentPreflightErrorV1::CapabilityDependencyFailure {
                path: vec![
                    "rules/root".to_owned(),
                    "rules/missing-transitive".to_owned()
                ]
            })
        );
    }

    #[test]
    fn registry_rejects_capability_below_required_lifecycle() {
        let entry = RegistryEntry {
            key: "rules/root".to_owned(),
            version: "1.0.0".to_owned(),
            lifecycle: "proposed".to_owned(),
            lifecycle_rank: RequiredCapabilityLifecycleV1::Proposed.rank(),
            dependencies: vec![],
        };
        let registry = CapabilityRegistry {
            entries: BTreeMap::from([(entry.key.clone(), entry)]),
        };

        assert_eq!(
            registry.resolve(
                &[CapabilityRequirementV1 {
                    key: "rules/root".to_owned(),
                    version: "1.0.0".to_owned(),
                }],
                RequiredCapabilityLifecycleV1::Specified,
            ),
            Err(
                ContentPreflightErrorV1::CapabilityLifecycleBelowRequirement {
                    key: "rules/root".to_owned(),
                    lifecycle: "proposed".to_owned(),
                }
            )
        );
    }

    #[test]
    fn rust_closure_matches_python_registry_owner_on_synthetic_cases() {
        let parity: serde_json::Value = serde_json::from_str(include_str!(
            "../tests/fixtures/capability_registry_parity.v1.json"
        ))
        .unwrap();
        assert_eq!(parity["registry_id"], "project/capabilities");
        for case in parity["cases"].as_array().unwrap() {
            let registry = CapabilityRegistry {
                entries: case["entries"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|entry| {
                        let registry_entry = RegistryEntry {
                            key: entry["key"].as_str().unwrap().to_owned(),
                            version: entry["version"].as_str().unwrap().to_owned(),
                            lifecycle: entry["lifecycle"].as_str().unwrap().to_owned(),
                            lifecycle_rank: entry["lifecycle_rank"].as_i64().unwrap() as i8,
                            dependencies: entry["dependencies"]
                                .as_array()
                                .unwrap()
                                .iter()
                                .map(|key| key.as_str().unwrap().to_owned())
                                .collect(),
                        };
                        (registry_entry.key.clone(), registry_entry)
                    })
                    .collect(),
            };
            let roots = case["roots"]
                .as_array()
                .unwrap()
                .iter()
                .map(|root| CapabilityRequirementV1 {
                    key: root["key"].as_str().unwrap().to_owned(),
                    version: root["version"].as_str().unwrap().to_owned(),
                })
                .collect::<Vec<_>>();
            let required = match case["minimum_lifecycle"].as_str().unwrap() {
                "proposed" => RequiredCapabilityLifecycleV1::Proposed,
                "specified" => RequiredCapabilityLifecycleV1::Specified,
                "implemented" => RequiredCapabilityLifecycleV1::Implemented,
                "covered" => RequiredCapabilityLifecycleV1::Covered,
                "certified" => RequiredCapabilityLifecycleV1::Certified,
                _ => panic!("unknown parity lifecycle"),
            };
            let below = case["below_required_lifecycle"].as_array().unwrap();
            let actual = registry.resolve(&roots, required);
            if below.is_empty() {
                let actual = actual.unwrap();
                let actual = actual
                    .iter()
                    .map(|entry| (entry.key.as_str(), entry.version.as_str()))
                    .collect::<Vec<_>>();
                let expected = case["resolved"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|key| {
                        let key = key.as_str().unwrap();
                        (key, registry.entries[key].version.as_str())
                    })
                    .collect::<Vec<_>>();
                assert_eq!(actual, expected);
            } else {
                let (key, lifecycle) =
                    (below[0][0].as_str().unwrap(), below[0][1].as_str().unwrap());
                assert_eq!(
                    actual,
                    Err(
                        ContentPreflightErrorV1::CapabilityLifecycleBelowRequirement {
                            key: key.to_owned(),
                            lifecycle: lifecycle.to_owned(),
                        }
                    )
                );
            }
        }
    }
}
