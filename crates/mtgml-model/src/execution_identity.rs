//! V5 execution identity vocabulary (spec §6, §7b; ADR 0055 §2.3/§2.4).
//!
//! `ExecutionProgramV1` is a closed, milestone-free dispatch family and
//! `ExecutionIdentityV1` is the full identity bound into V5 checkpoints and
//! replay surfaces. The wire value is `magic_rules`. Unknown values and
//! unknown fields fail closed.

use serde::{Deserialize, Serialize};

use crate::{RulesAuthorityV1, SemanticContractIdV1};

/// Closed execution/dispatch family (spec §6; ADR 0055 §2.4).
///
/// JSON: the bare string `magic_rules`; unknown values fail decode. No
/// fallback variant exists; no `Default` is derived.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionProgramV1 {
    MagicRules,
}

/// Returns whether an execution program is paired with its only permitted
/// rules authority, as required by ADR 0055 §2.4.
pub fn execution_program_matches_rules_authority(
    program: ExecutionProgramV1,
    authority: &RulesAuthorityV1,
) -> bool {
    matches!(
        (program, authority),
        (
            ExecutionProgramV1::MagicRules,
            RulesAuthorityV1::ComprehensiveRules { .. }
        )
    )
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adr_0055_program_authority_pairs_are_closed_and_symmetric() {
        assert!(execution_program_matches_rules_authority(
            ExecutionProgramV1::MagicRules,
            &RulesAuthorityV1::ComprehensiveRules {
                snapshot_id: "cr-snapshot".to_owned(),
            },
        ));
    }
}
