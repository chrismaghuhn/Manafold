//! Runtime semantic catalog and stateless V6 restore admission (spec §10, §12, §18;
//! ADR 0055 §2.8–§2.9).
//!
//! The catalog consumes ONLY the checked-in generated Task-3 material
//! (`semantic_catalog_generated.rs`). It performs NO digest generation
//! (the recompute KAT owns that), NO mutable/lazy state, NO filesystem or
//! network lookup. `resolve` answers "what immutable contract does this ID
//! mean?" while `supported` answers "does THIS runtime support executing
//! that pairing?" — deliberately different questions.
//!
//! The `admit_restore` function implements the spec §12 nine-phase admission
//! order as a PURE / STATELESS function (phases 1–8; phase 9 is Task 13).
//! Rejected admission mutates NOTHING (spec §2.8 / §12.6).
//
// Functions in this module are wired into the live controller restore path
// (Task 13). They are exercised by crate-internal tests and the KAT.

use mtgml_model::{
    ExecutionProgramV1, RulesAuthorityV1, RulesContractManifestV1, SemanticContractIdV1,
    SemanticContractManifestV1,
};
use mtgml_persistence::semantic_contract_digest::{
    calculate_rules_contract_id_v1, calculate_semantic_contract_id_v1,
};
use mtgml_rules::{validate_runtime_state_for_contract, ProgramKernelConstructionErrorV1};
use thiserror::Error;

use crate::checkpoint::{CheckpointValidationError, EnvironmentCheckpointV6};
use crate::errors::ControllerError;
use crate::semantic_catalog_generated::{
    magic_s3_a_ordered_sba_0_1_0_rules_manifest, magic_s3_a_ordered_sba_0_1_0_semantic_contract_id,
    magic_s3_a_ordered_sba_0_1_0_semantic_manifest, magic_s3_b_basic_priority_0_1_0_rules_manifest,
    magic_s3_b_basic_priority_0_1_0_semantic_contract_id,
    magic_s3_b_basic_priority_0_1_0_semantic_manifest,
    magic_s3_c_draw_interaction_0_1_0_rules_manifest,
    magic_s3_c_draw_interaction_0_1_0_semantic_contract_id,
    magic_s3_c_draw_interaction_0_1_0_semantic_manifest, magic_turn_structure_0_1_0_rules_manifest,
    magic_turn_structure_0_1_0_semantic_contract_id, magic_turn_structure_0_1_0_semantic_manifest,
    synthetic_legacy_default_rules_manifest, synthetic_legacy_default_semantic_contract_id,
    synthetic_legacy_default_semantic_manifest,
};

/// A single catalog entry: the frozen meaning of one semantic contract ID.
///
/// The catalog is a checked-in data table; entries are constructed purely
/// from the generated Task-3 constants. No public mutation API exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogEntry {
    pub semantic_contract_id: SemanticContractIdV1,
    pub manifest: SemanticContractManifestV1,
    pub rules_manifest: RulesContractManifestV1,
}

/// Immutable, deterministic, process-state-free runtime semantic catalog.
///
/// The authoritative source chain remains:
/// ```text
/// contracts/catalog/semantic-contracts.v1.json
///   → Task-3 generator (check-in)
///   → semantic_catalog_generated.rs
///   → RuntimeSemanticCatalog (this module)
/// ```
///
/// Forbidden: `static mut`, `OnceCell`, `LazyLock`, global mutable map,
/// filesystem loading, environment-variable selection, network lookup,
/// thread-local execution identity, runtime digest generation as the
/// catalog authority.
pub struct RuntimeSemanticCatalog {
    entries: Vec<CatalogEntry>,
}

impl RuntimeSemanticCatalog {
    /// The single authoritative production construction path: the checked-in
    /// generated catalog material. No runtime digest generation occurs —
    /// every value is a frozen generated constant (spec §10).
    pub fn production() -> Self {
        Self {
            entries: vec![
                CatalogEntry {
                    semantic_contract_id: synthetic_legacy_default_semantic_contract_id(),
                    manifest: synthetic_legacy_default_semantic_manifest(),
                    rules_manifest: synthetic_legacy_default_rules_manifest(),
                },
                CatalogEntry {
                    semantic_contract_id: magic_turn_structure_0_1_0_semantic_contract_id(),
                    manifest: magic_turn_structure_0_1_0_semantic_manifest(),
                    rules_manifest: magic_turn_structure_0_1_0_rules_manifest(),
                },
                CatalogEntry {
                    semantic_contract_id: magic_s3_a_ordered_sba_0_1_0_semantic_contract_id(),
                    manifest: magic_s3_a_ordered_sba_0_1_0_semantic_manifest(),
                    rules_manifest: magic_s3_a_ordered_sba_0_1_0_rules_manifest(),
                },
                CatalogEntry {
                    semantic_contract_id: magic_s3_b_basic_priority_0_1_0_semantic_contract_id(),
                    manifest: magic_s3_b_basic_priority_0_1_0_semantic_manifest(),
                    rules_manifest: magic_s3_b_basic_priority_0_1_0_rules_manifest(),
                },
                CatalogEntry {
                    semantic_contract_id: magic_s3_c_draw_interaction_0_1_0_semantic_contract_id(),
                    manifest: magic_s3_c_draw_interaction_0_1_0_semantic_manifest(),
                    rules_manifest: magic_s3_c_draw_interaction_0_1_0_rules_manifest(),
                },
            ],
        }
    }

    /// Test-only seam: construct a catalog from explicit entries.
    /// This is `#[cfg(test)]` so the production binary exposes exactly one
    /// catalog construction path (`production()`).
    #[cfg(test)]
    pub(crate) fn from_entries(entries: Vec<CatalogEntry>) -> Self {
        Self { entries }
    }

    /// KNOWN MEANING: resolve a semantic contract ID to its immutable manifest.
    /// Returns `None` if the ID is not a recognized content-derived identity.
    pub fn resolve(&self, id: &SemanticContractIdV1) -> Option<&CatalogEntry> {
        self.entries
            .iter()
            .find(|entry| entry.semantic_contract_id == *id)
    }

    /// SUPPORTED EXECUTION: does THIS runtime support executing this exact
    /// (program_kind, semantic_contract_id) pairing?
    ///
    /// Delegates to the shared runtime-support authority
    /// (`mtgml_rules::execution_contract_supported`) which is generated
    /// from the same manifest source. No independent pairing logic
    /// lives here.
    pub fn supported(&self, id: &SemanticContractIdV1, program: ExecutionProgramV1) -> bool {
        mtgml_rules::execution_contract_supported(program, id)
    }

    /// Number of entries (crate-internal, used by tests).
    #[cfg(test)]
    pub(crate) fn entry_count(&self) -> usize {
        self.entries.len()
    }
}

/// Frozen pairing rule (spec §7, ADR 0055 §2.4): the only valid
/// program × rules-authority pairings are:
/// - `SyntheticRulesCompat` ↔ `synthetic_legacy`
/// - `MagicRules` ↔ `comprehensive_rules`
///
/// No fallback is allowed. `MagicRules` with `synthetic_legacy` is a
/// `ProgramAuthorityMismatch`; `SyntheticRulesCompat` with
/// `comprehensive_rules` is a `ProgramAuthorityMismatch`.
fn program_authority_compatible(program: ExecutionProgramV1, authority: &RulesAuthorityV1) -> bool {
    matches!(
        (program, authority),
        (
            ExecutionProgramV1::SyntheticRulesCompat,
            RulesAuthorityV1::SyntheticLegacy
        ) | (
            ExecutionProgramV1::MagicRules,
            RulesAuthorityV1::ComprehensiveRules { .. }
        )
    )
}

/// Typed failure family for the V6 restore admission machinery (spec §18).
///
/// This is the pure admission-layer error. The environment maps these
/// onto the existing `CheckpointValidationError` / `ControllerError` families
/// at the controller boundary (Task 13).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum RestoreAdmissionError {
    #[error("checkpoint validation failed: {0}")]
    CheckpointValidation(CheckpointValidationError),

    #[error("semantic contract ID is unknown to this runtime")]
    SemanticContractUnknown,

    #[error("semantic contract manifest does not match its bound ID")]
    SemanticContractDigestMismatch,

    #[error("rules contract manifest does not match its bound ID")]
    RulesContractDigestMismatch,

    #[error("program kind is incompatible with the rules contract authority")]
    ProgramAuthorityMismatch,

    #[error("semantic contract is not supported by this runtime")]
    SemanticContractUnsupported,

    #[error("engine state is not compatible with the execution program")]
    ProgramStateIncompatible,
}

impl From<CheckpointValidationError> for RestoreAdmissionError {
    fn from(err: CheckpointValidationError) -> Self {
        RestoreAdmissionError::CheckpointValidation(err)
    }
}

impl From<RestoreAdmissionError> for ControllerError {
    fn from(err: RestoreAdmissionError) -> Self {
        match err {
            RestoreAdmissionError::CheckpointValidation(e) => {
                ControllerError::CheckpointValidation(e)
            }
            RestoreAdmissionError::SemanticContractUnknown => {
                ControllerError::CheckpointValidation(
                    CheckpointValidationError::SemanticContractUnknown,
                )
            }
            RestoreAdmissionError::SemanticContractDigestMismatch => {
                ControllerError::CheckpointValidation(
                    CheckpointValidationError::SemanticContractDigestMismatch,
                )
            }
            RestoreAdmissionError::RulesContractDigestMismatch => {
                ControllerError::CheckpointValidation(
                    CheckpointValidationError::RulesContractDigestMismatch,
                )
            }
            RestoreAdmissionError::ProgramAuthorityMismatch => {
                ControllerError::ProgramAuthorityMismatch
            }
            RestoreAdmissionError::SemanticContractUnsupported => {
                ControllerError::SemanticContractUnsupported
            }
            RestoreAdmissionError::ProgramStateIncompatible => {
                ControllerError::ProgramStateIncompatible
            }
        }
    }
}

/// Deterministic mapping from the program-owned kernel construction error
/// into the environment admission/support error model (spec §18 / Task-8 Plan
/// Fix-05). `ProgramKernelConstructionErrorV1::UnsupportedProgram` maps to
/// `ControllerError::SemanticContractUnsupported` — the runtime refuses to
/// support a program with no production kernel contract. This is explicitly
/// NOT a `KernelExecutionError` variant (construction/admission failures
/// precede execution and are semantically distinct).
///
/// The only variant in the current slice is `UnsupportedProgram`; additional
/// variants added in future slices must receive a deterministic, semantically
/// distinct mapping — never a flatten to `Backend(String)` or
/// `InvalidCheckpoint(String)`.
impl From<ProgramKernelConstructionErrorV1> for ControllerError {
    fn from(err: ProgramKernelConstructionErrorV1) -> Self {
        match err {
            ProgramKernelConstructionErrorV1::UnsupportedProgram => {
                ControllerError::SemanticContractUnsupported
            }
        }
    }
}

/// The spec §12 nine-phase V6 restore admission, implemented as a PURE /
/// STATELESS function (phases 1–8; phase 9 backend construction is Task 13).
///
/// Phases are evaluated in the exact frozen order. An earlier-phase defect
/// MUST NOT be reported as a later failure. Rejected admission mutates
/// nothing: state, RNG, IDs, knowledge, events, history, episode status,
/// replay recorder, backend semantic identity (spec §2.8 / §12.6).
///
/// ```text
/// 1. checkpoint.validate()                         (structural)
/// 2. checkpoint digest recompute                   (inside validate)
/// 3. resolve semantic_contract_id in catalog
/// 4. recompute semantic ID from resolved manifest; compare
/// 5. recompute rules ID from resolved rules manifest; compare
/// 6. program × rules authority compatibility (frozen pairing rule)
/// 7. runtime support check (catalog support predicate)
/// 8. program × EngineState semantic admission (rules kernel validator)
/// ```
pub fn admit_restore(
    catalog: &RuntimeSemanticCatalog,
    checkpoint: &EnvironmentCheckpointV6,
) -> Result<(), RestoreAdmissionError> {
    // Phase 1: structural validation (schema, codec, state digest,
    // checkpoint digest recompute from stored execution_identity, completed
    // status rule). Phase 2 (digest recompute) is owned inside validate().
    checkpoint.validate()?;

    // Phase 3: resolve the bound semantic_contract_id in this runtime's
    // immutable catalog. An unknown ID is fail-closed (spec §2.9).
    let entry = catalog
        .resolve(&checkpoint.execution_identity.semantic_contract_id)
        .ok_or(RestoreAdmissionError::SemanticContractUnknown)?;

    // Phase 4: recompute the semantic contract ID from the resolved manifest
    // and compare against the bound ID. A mismatch is an invariant breach —
    // the catalog entry's stored ID does not match its own manifest.
    let recomputed_semantic = calculate_semantic_contract_id_v1(&entry.manifest)
        .map_err(|_| RestoreAdmissionError::SemanticContractDigestMismatch)?;
    if recomputed_semantic != checkpoint.execution_identity.semantic_contract_id {
        return Err(RestoreAdmissionError::SemanticContractDigestMismatch);
    }

    // Phase 5: recompute the rules contract ID from the resolved rules
    // manifest and compare against the rules_contract_id field inside the
    // semantic manifest. A mismatch is an invariant breach — the resolved
    // rules manifest does not match what the semantic manifest declares.
    let recomputed_rules = calculate_rules_contract_id_v1(&entry.rules_manifest)
        .map_err(|_| RestoreAdmissionError::RulesContractDigestMismatch)?;
    if recomputed_rules != entry.manifest.rules_contract_id {
        return Err(RestoreAdmissionError::RulesContractDigestMismatch);
    }

    // Phase 6: program × rules authority compatibility — the frozen pairing
    // rule (spec §7 / ADR 0055 §2.4). No fallback.
    if !program_authority_compatible(
        checkpoint.execution_identity.program_kind,
        &entry.rules_manifest.rules_authority,
    ) {
        return Err(RestoreAdmissionError::ProgramAuthorityMismatch);
    }

    // Phase 7: runtime support check — does THIS runtime build support
    // executing this program × contract pairing?
    if !catalog.supported(
        &checkpoint.execution_identity.semantic_contract_id,
        checkpoint.execution_identity.program_kind,
    ) {
        return Err(RestoreAdmissionError::SemanticContractUnsupported);
    }

    // Phase 8: program × EngineState semantic admission — the program-aware
    // validator lives in the rules kernel (mtgml-rules), above the generic
    // EngineState which carries no program identity. Pre-S1: SyntheticRulesCompat
    // reuses existing synthetic runtime-state validation; MagicRules fails closed.
    validate_runtime_state_for_contract(
        checkpoint.execution_identity.program_kind,
        checkpoint.execution_identity.semantic_contract_id.clone(),
        &checkpoint.state,
        &checkpoint.status,
    )
    .map_err(|_| RestoreAdmissionError::ProgramStateIncompatible)?;

    // Phase 9: backend construction/commit — owned by Task 13.
    Ok(())
}
