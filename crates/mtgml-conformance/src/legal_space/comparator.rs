//! Typed soundness/completeness comparison between the independent
//! reference space R and the production-discovered space P.

use super::canonical::{CanonicalCompleteChoice, CanonicalStageChoice};
use super::explorer::{ObservedRequest, ProductionSpace};
use crate::legal_space::oracle::{ExpectedDomain, ReferenceAutomaton};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpaceDefect {
    /// Accepted terminal choice absent from the independent reference space.
    IllegalExtra { choice: CanonicalCompleteChoice },
    /// Reference-legal complete choice with zero production paths.
    MissingChoice { choice: CanonicalCompleteChoice },
    /// Reference-legal complete choice reached by more than one canonical path.
    DuplicatePath {
        choice: CanonicalCompleteChoice,
        path_count: usize,
    },
    /// The visible request advertised an answer that production rejected.
    AdvertisedRejected { detail: String },
    /// Production accepted a probe outside its own advertised surface.
    OutOfContractAccepted { count: u64 },
    /// Observed visible request diverges from the independent expectation.
    RequestShapeMismatch { index: usize, detail: String },
}

fn domains_match(
    expected: &ExpectedDomain,
    observed_domain: &super::explorer::ObservedDomain,
) -> bool {
    use super::explorer::ObservedDomain as O;
    match (expected, observed_domain) {
        (ExpectedDomain::ChooseOne, O::ChooseOne) => true,
        (
            ExpectedDomain::ChooseMany { minimum, maximum },
            O::ChooseMany {
                minimum: om,
                maximum: ox,
            },
        ) => minimum == om && maximum == ox,
        (
            ExpectedDomain::ChooseNumber { minimum, maximum },
            O::ChooseNumber {
                minimum: om,
                maximum: ox,
            },
        ) => minimum == om && maximum == ox,
        (
            ExpectedDomain::Order { minimum, maximum },
            O::Order {
                minimum: om,
                maximum: ox,
            },
        ) => minimum == om && maximum == ox,
        _ => false,
    }
}

/// Level A — request soundness: along every accepted protocol path, the
/// observed visible requests must equal the independently expected requests.
pub fn request_shape_mismatches(
    automaton: &ReferenceAutomaton,
    production: &ProductionSpace,
) -> Vec<SpaceDefect> {
    let mut out = Vec::new();
    for record in production
        .complete_paths
        .values()
        .flat_map(|paths| paths.iter())
    {
        out.extend(request_sequence_defects(
            automaton,
            &record.stages,
            &record.observed_requests,
        ));
    }
    out.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
    out.dedup();
    out
}

/// Request-soundness check for ONE protocol path.
pub fn request_sequence_defects(
    automaton: &ReferenceAutomaton,
    stages: &[CanonicalStageChoice],
    observed_requests: &[ObservedRequest],
) -> Vec<SpaceDefect> {
    let mut automaton = automaton.clone();
    let mut out = Vec::new();
    for (index, stage) in stages.iter().enumerate() {
        let expected = automaton.expected_request();
        let observed = observed_requests.get(index).cloned();
        match (expected, observed) {
            (Some(expected), Some(observed)) => {
                if !domains_match(&expected.domain, &observed.domain)
                    || expected.candidate_atoms != observed.candidate_atoms
                {
                    out.push(SpaceDefect::RequestShapeMismatch {
                        index,
                        detail: format!(
                            "expected {expected:?} observed {observed:?} at stage {index}"
                        ),
                    });
                }
            }
            _ => out.push(SpaceDefect::RequestShapeMismatch {
                index,
                detail: "missing expected or observed request".into(),
            }),
        }
        automaton.advance(stage);
    }
    out
}

/// Level B — complete-path soundness: P ⊆ R, plus emitted-side defects.
pub fn soundness_defects(
    reference: &[CanonicalCompleteChoice],
    production: &ProductionSpace,
) -> Vec<SpaceDefect> {
    let mut out = Vec::new();
    for choice in production.complete_paths.keys() {
        if !reference.contains(choice) {
            out.push(SpaceDefect::IllegalExtra {
                choice: choice.clone(),
            });
        }
    }
    for detail in &production.advertised_rejected {
        out.push(SpaceDefect::AdvertisedRejected {
            detail: detail.clone(),
        });
    }
    if production.out_of_contract_accepted > 0 {
        out.push(SpaceDefect::OutOfContractAccepted {
            count: production.out_of_contract_accepted,
        });
    }
    out
}

/// Completeness with the stronger Issue #53 semantics: every reference-legal
/// complete choice has EXACTLY ONE canonical reachable production path.
pub fn completeness_defects(
    reference: &[CanonicalCompleteChoice],
    production: &ProductionSpace,
) -> Vec<SpaceDefect> {
    let mut out = Vec::new();
    for choice in reference {
        match production.complete_paths.get(choice) {
            None => out.push(SpaceDefect::MissingChoice {
                choice: choice.clone(),
            }),
            Some(paths) if paths.len() > 1 => out.push(SpaceDefect::DuplicatePath {
                choice: choice.clone(),
                path_count: paths.len(),
            }),
            Some(_) => {}
        }
    }
    out
}
