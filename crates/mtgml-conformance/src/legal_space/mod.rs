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

pub mod canonical;
pub mod comparator;
pub mod explorer;
pub mod oracle;

#[cfg(test)]
mod gate_evidence;
