//! V5 execution identity vocabulary (spec §6, §7b; ADR 0055 §2.3/§2.4).
//!
//! `ExecutionProgramV1` is a closed, milestone-free dispatch family and
//! `ExecutionIdentityV1` is the full identity bound into V5 checkpoints and
//! replay surfaces. Wire values are frozen: `synthetic_rules_compat` and
//! `magic_rules`. Unknown values and unknown fields fail closed.

use serde::{Deserialize, Serialize};

use crate::SemanticContractIdV1;

/// Closed execution/dispatch family (spec §6; ADR 0055 §2.4).
///
/// JSON: bare strings `synthetic_rules_compat` / `magic_rules`; unknown
/// values fail decode. No fallback variant exists; no `Default` is derived.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionProgramV1 {
    SyntheticRulesCompat,
    MagicRules,
}

/// Full execution identity (spec §6; ADR 0055 §2.3).
///
/// The checkpoint binds the FULL struct; the dispatch tag alone is never
/// sufficient. No implementation/build identity belongs here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionIdentityV1 {
    pub program_kind: ExecutionProgramV1,
    pub semantic_contract_id: SemanticContractIdV1,
}
