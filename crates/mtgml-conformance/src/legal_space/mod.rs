//! M2.F — independent bounded synthetic legal-space oracle and
//! production protocol exploration harness (Issue #53).
//!
//! OWNERSHIP BOUNDARY: everything in this module tree is test/conformance
//! only. Production crates (rules/environment/decision/state) MUST NOT
//! depend on it; the gate runner asserts this dependency direction.
//!
//! The canonical complete-choice representation here is comparison-only:
//! it is NOT OD-011's future stable semantic action key, not a replay
//! action encoding, not a trajectory action id, and not a wire contract.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LegalSpaceBudget {
    pub max_candidates_per_request: u32,
    pub max_numeric_span: u64,
    pub max_depth: u32,
    pub max_total_nodes: u32,
    pub max_generated_answers: u64,
}

impl Default for LegalSpaceBudget {
    fn default() -> Self {
        Self {
            max_candidates_per_request: 8,
            max_numeric_span: 16,
            max_depth: 4,
            max_total_nodes: 64,
            max_generated_answers: 256,
        }
    }
}

pub mod canonical;
pub mod comparator;
pub mod explorer;
pub mod oracle;

#[cfg(test)]
mod gate_evidence;
