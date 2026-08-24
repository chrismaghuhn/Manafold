//! Independent bounded reference automaton for the frozen M2.C synthetic
//! assembly scenario (Issue #53).
//!
//! This module OWNS the independent legal-space declaration: stage domains,
//! bounds, eligible pieces and the staged composition are declared HERE, not
//! derived from production candidate enumeration or continuation state.
//!
//! Deliberately NOT imported from production: `SYNTHETIC_COUNT_MIN/MAX`,
//! `validate_for`, candidate ordering, trusted bindings. If production
//! changes its bounds, request-soundness detects the divergence instead of
//! the oracle silently following along.

use std::collections::BTreeSet;

use crate::legal_space::canonical::{
    CanonicalCompleteChoice, CanonicalStageChoice, SyntheticChoiceAtom,
};

/// Independently declared scenario bounds (frozen M2.C fixture semantics).
pub const SCENARIO_COUNT_MIN: i64 = 0;
pub const SCENARIO_COUNT_MAX: i64 = 3;

/// Piece keys of the scenario. Declaration order is internal iteration
/// order only; enumeration output must be identical for any permutation
/// (insertion-order invariance evidence I1).
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ReferenceAssemblySpec {
    pub piece_iteration_order: Vec<u32>,
}

impl Default for ReferenceAssemblySpec {
    fn default() -> Self {
        Self {
            piece_iteration_order: vec![0, 1, 2],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceAssemblyState {
    Entry,
    ChooseCount,
    ChooseMembers { count: u32 },
    OrderMembers { count: u32, selected: BTreeSet<u32> },
    Complete,
}

/// Independently declared expected shape of the player-visible request at a
/// given reference state (request-soundness comparison target).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpectedDomain {
    ChooseOne,
    ChooseMany { minimum: u32, maximum: u32 },
    ChooseNumber { minimum: i64, maximum: i64 },
    Order { minimum: u32, maximum: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedRequest {
    pub domain: ExpectedDomain,
    /// Expected semantic atoms of the visible candidates, in canonical order.
    pub candidate_atoms: Vec<SyntheticChoiceAtom>,
}

/// Tiny dependent reference automaton for exactly the accepted synthetic
/// assembly scenario. It has no execution or commit authority whatsoever.
#[derive(Debug, Clone)]
pub struct ReferenceAutomaton {
    #[allow(dead_code)]
    #[allow(dead_code)]
    spec: ReferenceAssemblySpec,
    state: ReferenceAssemblyState,
}

impl ReferenceAutomaton {
    pub fn new(spec: ReferenceAssemblySpec) -> Self {
        Self {
            spec,
            state: ReferenceAssemblyState::Entry,
        }
    }

    pub fn initial() -> Self {
        Self::new(ReferenceAssemblySpec::default())
    }

    pub fn state(&self) -> &ReferenceAssemblyState {
        &self.state
    }

    /// Independently expected player-visible request at the current state.
    pub fn expected_request(&self) -> Option<ExpectedRequest> {
        Some(match &self.state {
            ReferenceAssemblyState::Entry => ExpectedRequest {
                domain: ExpectedDomain::ChooseOne,
                candidate_atoms: vec![SyntheticChoiceAtom::EntryAnchor],
            },
            ReferenceAssemblyState::ChooseCount => ExpectedRequest {
                domain: ExpectedDomain::ChooseNumber {
                    minimum: SCENARIO_COUNT_MIN,
                    maximum: SCENARIO_COUNT_MAX,
                },
                candidate_atoms: Vec::new(),
            },
            ReferenceAssemblyState::ChooseMembers { count } => ExpectedRequest {
                domain: ExpectedDomain::ChooseMany {
                    minimum: *count,
                    maximum: *count,
                },
                candidate_atoms: self
                    .spec
                    .piece_iteration_order
                    .iter()
                    .take(*count as usize)
                    .map(|piece| SyntheticChoiceAtom::Piece(*piece))
                    .collect(),
            },
            ReferenceAssemblyState::OrderMembers { selected, .. } => ExpectedRequest {
                domain: ExpectedDomain::Order {
                    minimum: selected.len() as u32,
                    maximum: selected.len() as u32,
                },
                candidate_atoms: selected
                    .iter()
                    .map(|piece| SyntheticChoiceAtom::Piece(*piece))
                    .collect(),
            },
            ReferenceAssemblyState::Complete => return None,
        })
    }

    /// Independently enumerated LEGAL choices at the current state.
    pub fn reference_choices(&self) -> Vec<CanonicalStageChoice> {
        match &self.state {
            ReferenceAssemblyState::Entry => vec![CanonicalStageChoice::Anchor],
            ReferenceAssemblyState::ChooseCount => (SCENARIO_COUNT_MIN..=SCENARIO_COUNT_MAX)
                .map(CanonicalStageChoice::Number)
                .collect(),
            ReferenceAssemblyState::ChooseMembers { count } => {
                let members: BTreeSet<SyntheticChoiceAtom> = self
                    .spec
                    .piece_iteration_order
                    .iter()
                    .take(*count as usize)
                    .map(|piece| SyntheticChoiceAtom::Piece(*piece))
                    .collect();
                vec![CanonicalStageChoice::Members(members)]
            }
            ReferenceAssemblyState::OrderMembers { selected, .. } => {
                let atom_set: BTreeSet<SyntheticChoiceAtom> = selected
                    .iter()
                    .map(|piece| SyntheticChoiceAtom::Piece(*piece))
                    .collect();
                permutations(&atom_set)
            }
            ReferenceAssemblyState::Complete => Vec::new(),
        }
    }

    /// Advances the automaton along one legal choice (must be one of
    /// [`Self::reference_choices`]).
    pub fn advance(&mut self, choice: &CanonicalStageChoice) {
        match (&self.state, choice) {
            (ReferenceAssemblyState::Entry, CanonicalStageChoice::Anchor) => {
                self.state = ReferenceAssemblyState::ChooseCount;
            }
            (ReferenceAssemblyState::ChooseCount, CanonicalStageChoice::Number(count)) => {
                self.state = ReferenceAssemblyState::ChooseMembers {
                    count: *count as u32,
                };
            }
            (
                ReferenceAssemblyState::ChooseMembers { count },
                CanonicalStageChoice::Members(selected),
            ) => {
                self.state = ReferenceAssemblyState::OrderMembers {
                    count: *count,
                    selected: selected
                        .iter()
                        .filter_map(|atom| match atom {
                            SyntheticChoiceAtom::Piece(piece) => Some(*piece),
                            _ => None,
                        })
                        .collect(),
                };
            }
            (ReferenceAssemblyState::OrderMembers { .. }, CanonicalStageChoice::Order(_)) => {
                self.state = ReferenceAssemblyState::Complete;
            }
            _ => {}
        }
    }

    /// Exhaustively enumerates every complete reference choice from the
    /// current state (bounded by the tiny frozen scenario).
    pub fn enumerate_complete_choices(&self) -> Vec<CanonicalCompleteChoice> {
        let mut out = Vec::new();
        let mut stack: Vec<(ReferenceAutomaton, Vec<CanonicalStageChoice>)> =
            vec![(self.clone(), Vec::new())];
        while let Some((automaton, path)) = stack.pop() {
            for choice in automaton.reference_choices() {
                let mut next_automaton = automaton.clone();
                next_automaton.advance(&choice);
                let mut next_path = path.clone();
                next_path.push(choice);
                match next_automaton.state {
                    ReferenceAssemblyState::Complete => {
                        out.push(CanonicalCompleteChoice(next_path))
                    }
                    _ => stack.push((next_automaton, next_path)),
                }
            }
        }
        out.sort();
        out.dedup();
        out
    }
}

fn permutations(atoms: &BTreeSet<SyntheticChoiceAtom>) -> Vec<CanonicalStageChoice> {
    let items: Vec<SyntheticChoiceAtom> = atoms.iter().copied().collect();
    let mut out = Vec::new();
    permute_recursive(&items, 0, &mut out);
    out
}

fn permute_recursive(
    items: &[SyntheticChoiceAtom],
    start: usize,
    out: &mut Vec<CanonicalStageChoice>,
) {
    if start == items.len() {
        out.push(CanonicalStageChoice::Order(items.to_vec()));
        return;
    }
    for i in start..items.len() {
        let mut permuted = items.to_vec();
        permuted.swap(start, i);
        permute_recursive(&permuted, start + 1, out);
    }
}

#[cfg(test)]
mod debug {
    use super::*;
    #[test]
    fn debug_enumerate_count() {
        let auto = ReferenceAutomaton::initial();
        let all = auto.enumerate_complete_choices();
        eprintln!("total: {}", all.len());
        for (i, choice) in all.iter().enumerate() {
            eprintln!("  [{i}] {choice:?}");
        }
    }
}
