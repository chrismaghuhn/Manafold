//! Conformance-only canonical representation of complete synthetic choices.
//!
//! This module exists exclusively for M2.F legal-space comparison inside
//! `mtgml-conformance`. It MUST NOT become:
//!
//! - OD-011's future stable semantic action key;
//! - a replay action encoding;
//! - a training trajectory action id;
//! - a public wire contract;
//! - a permanent ML action vocabulary.
//!
//! Semantic atoms deliberately avoid `CandidateIdV1`: request-local
//! candidate ids are protocol transport identity for exactly one request,
//! never a logical choice label. ChooseMany uses set semantics (BTreeSet);
//! Order uses sequence semantics (Vec) — the two must never be conflated.

use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SyntheticChoiceAtom {
    EntryAnchor,
    Piece(u32),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum CanonicalStageChoice {
    Anchor,
    /// Set semantics: canonicalized by the BTreeSet itself.
    Members(BTreeSet<SyntheticChoiceAtom>),
    Number(i64),
    /// Sequence semantics: element order is significant.
    Order(Vec<SyntheticChoiceAtom>),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CanonicalCompleteChoice(pub Vec<CanonicalStageChoice>);
