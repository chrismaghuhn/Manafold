//! Production protocol exploration through the REAL player-facing endpoint.
//!
//! Independence boundary (Issue #53): this module MUST NOT import anything
//! from the reference oracle (`super::oracle`). Discovery depends exclusively
//! on the player-visible request plus an explicit resource budget. The
//! trusted controller is used ONLY for fork/checkpoint/branch isolation and
//! rejection non-mutation — never for candidate, number or order enumeration.

use std::collections::{BTreeMap, BTreeSet};

use mtgml_decision::{CandidateIntent, DecisionAnswerV2, DecisionDomainV2, DecisionResponseV2, PlayerDecisionRequestV2, DECISION_RESPONSE_V2_SCHEMA};
use mtgml_environment::{PlayerEndpoint, TrustedEnvironmentController};
use mtgml_model::{CandidateIdV1, OpaqueObjectId, PlayerId};

use crate::legal_space::canonical::{
    CanonicalCompleteChoice, CanonicalStageChoice, SyntheticChoiceAtom,
};

/// Hard resource caps. Exceeding ANY cap fails closed: a broken or mutated
/// production surface must never turn the harness unbounded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExplorerBudget {
    pub max_candidates_per_request: u32,
    pub max_numeric_span: u64,
    pub max_depth: u32,
    pub max_total_nodes: u32,
    pub max_generated_answers: u64,
}

impl Default for ExplorerBudget {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ExplorationBoundError {
    #[error("candidate count exceeds the exploration budget")]
    CandidatesExceeded,
    #[error("numeric range span exceeds the exploration budget")]
    NumericSpanExceeded,
    #[error("continuation depth exceeds the exploration budget")]
    DepthExceeded,
    #[error("total exploration node budget exceeded")]
    TotalNodesExceeded,
    #[error("generated answer budget exceeded")]
    GeneratedAnswersExceeded,
    #[error("visible request is malformed (inverted bounds)")]
    MalformedVisibleRequest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum MapperError {
    #[error("SelectObject payload does not match the declared scenario anchor")]
    WrongEntryAnchorPayload,
    #[error("candidate intent is not part of the declared scenario vocabulary")]
    UnexpectedIntent,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ExplorationFailure {
    #[error("exploration bound exceeded: {0}")]
    Bound(#[from] ExplorationBoundError),
    #[error("visible semantic mapper failed: {0}")]
    Mapper(#[from] MapperError),
    #[error("internal endpoint/backend failure during exploration")]
    Internal,
}

/// Independently declared scenario anchor context. MUST come from the
/// scenario declaration, never from the currently inspected decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScenarioBindingContext {
    pub entry_anchor_object: OpaqueObjectId,
}

/// The ONE translation site from visible production candidates into
/// semantic scenario atoms. Candidate ids stay transport-only.
pub fn map_candidate(
    intent: &CandidateIntent,
    context: &ScenarioBindingContext,
) -> Result<SyntheticChoiceAtom, MapperError> {
    match intent {
        CandidateIntent::SelectMode { mode_index } => {
            Ok(SyntheticChoiceAtom::Piece(*mode_index))
        }
        CandidateIntent::SelectObject { object } => {
            if *object == context.entry_anchor_object {
                Ok(SyntheticChoiceAtom::EntryAnchor)
            } else {
                Err(MapperError::WrongEntryAnchorPayload)
            }
        }
        _ => Err(MapperError::UnexpectedIntent),
    }
}

/// Mirrors the production domain shape using only visible data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObservedDomain {
    ChooseOne,
    ChooseMany { minimum: u32, maximum: u32 },
    ChooseNumber { minimum: i64, maximum: i64 },
    Order { minimum: u32, maximum: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedRequest {
    pub domain: ObservedDomain,
    pub candidate_atoms: Vec<SyntheticChoiceAtom>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnswerShape {
    SelectOne(u32),
    SelectMany(Vec<u32>),
    Number(i64),
    Order(Vec<u32>),
}

/// A bounded syntactic superset answer candidate. `advertised` is decided by
/// the conformance-only grammar over the VISIBLE request alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Probe {
    pub shape: AnswerShape,
    pub advertised: bool,
}

/// Grammar: which shapes does the VISIBLE request claim reachable?
fn is_advertised(request: &PlayerDecisionRequestV2, shape: &AnswerShape) -> bool {
    match (&request.decision, shape) {
        (DecisionDomainV2::ChooseOne, AnswerShape::SelectOne(_)) => true,
        (
            DecisionDomainV2::ChooseMany { minimum, maximum },
            AnswerShape::SelectMany(candidate_ids),
        ) => {
            let length = candidate_ids.len() as u32;
            *minimum <= length && length <= *maximum
        }
        (
            DecisionDomainV2::ChooseNumber { minimum, maximum },
            AnswerShape::Number(value),
        ) => {
            let (minimum, maximum) = (*minimum, *maximum);
            let value = *value;
            minimum <= value && value <= maximum
        }
        (
            DecisionDomainV2::Order { minimum, maximum },
            AnswerShape::Order(candidate_ids),
        ) => {
            let length = candidate_ids.len() as u32;
            *minimum <= length && length <= *maximum
        }
        _ => false,
    }
}

/// Pure probe generation: reads ONLY the visible request and the budget.
pub fn generate_probes(
    request: &PlayerDecisionRequestV2,
    budget: &ExplorerBudget,
) -> Result<Vec<Probe>, ExplorationBoundError> {
    let candidate_count = request.candidates.len() as u32;
    if candidate_count > budget.max_candidates_per_request {
        return Err(ExplorationBoundError::CandidatesExceeded);
    }
    let ids: Vec<u32> = request
        .candidates
        .iter()
        .map(|candidate| candidate.candidate_id.0)
        .collect();
    let mut probes = Vec::new();
    match &request.decision {
        DecisionDomainV2::ChooseOne => {
            for id in ids {
                probes.push(Probe {
                    shape: AnswerShape::SelectOne(id),
                    advertised: true,
                });
            }
        }
        DecisionDomainV2::ChooseMany { .. } => {
            let subsets = 1u64
                .checked_mul(1u64 << candidate_count.min(63))
                .ok_or(ExplorationBoundError::GeneratedAnswersExceeded)?;
            if subsets > budget.max_generated_answers {
                return Err(ExplorationBoundError::GeneratedAnswersExceeded);
            }
            for mask in 0..subsets {
                let chosen: Vec<u32> = (0..candidate_count)
                    .filter(|index| mask & (1 << index) != 0)
                    .collect();
                let advertised = match &request.decision {
                    DecisionDomainV2::ChooseMany { minimum, maximum } => {
                        let length = chosen.len() as u32;
                        minimum <= &length && length <= *maximum
                    }
                    _ => false,
                };
                probes.push(Probe {
                    shape: AnswerShape::SelectMany(chosen),
                    advertised,
                });
            }
        }
        DecisionDomainV2::ChooseNumber { minimum, maximum } => {
            if minimum > maximum {
                return Err(ExplorationBoundError::MalformedVisibleRequest);
            }
            let span = i128::from(*maximum) - i128::from(*minimum) + 1;
            if span > i128::from(budget.max_numeric_span) {
                return Err(ExplorationBoundError::NumericSpanExceeded);
            }
            for value in *minimum..=*maximum {
                probes.push(Probe {
                    shape: AnswerShape::Number(value),
                    advertised: true,
                });
            }
            // Boundary sentinels OUTSIDE the visible range: an accepted
            // sentinel proves a too-permissive numeric surface.
            probes.push(Probe {
                shape: AnswerShape::Number(minimum - 1),
                advertised: false,
            });
            probes.push(Probe {
                shape: AnswerShape::Number(maximum + 1),
                advertised: false,
            });
        }
        DecisionDomainV2::Order { minimum, maximum } => {
            if minimum > maximum {
                return Err(ExplorationBoundError::MalformedVisibleRequest);
            }
            let mut estimated = 0u64;
            for length in *minimum..=*maximum {
                estimated += permutations_count(candidate_count as u64, length as u64)
                    .ok_or(ExplorationBoundError::GeneratedAnswersExceeded)?;
            }
            if estimated > budget.max_generated_answers {
                return Err(ExplorationBoundError::GeneratedAnswersExceeded);
            }
            for length in *minimum..=*maximum {
                for sequence in permutations_of(&ids, length as usize) {
                    let len = sequence.len() as u32;
                    let advertised = *minimum <= len && len <= *maximum;
                    probes.push(Probe {
                        shape: AnswerShape::Order(sequence),
                        advertised,
                    });
                }
            }
        }
    }
    Ok(probes)
}

fn permutations_count(n: u64, k: u64) -> Option<u64> {
    if k > n {
        return Some(0);
    }
    let mut result: u64 = 1;
    for offset in 0..k {
        result = result.checked_mul(n - offset)?;
    }
    Some(result)
}

fn permutations_of(items: &[u32], length: usize) -> Vec<Vec<u32>> {
    if length > items.len() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut current: Vec<u32> = Vec::new();
    let mut used = vec![false; items.len()];
    recurse_permutations(items, length, &mut current, &mut used, &mut out);
    out
}

fn recurse_permutations(
    items: &[u32],
    length: usize,
    current: &mut Vec<u32>,
    used: &mut [bool],
    out: &mut Vec<Vec<u32>>,
) {
    if current.len() == length {
        out.push(current.clone());
        return;
    }
    for index in 0..items.len() {
        if used[index] {
            continue;
        }
        used[index] = true;
        current.push(items[index]);
        recurse_permutations(items, length, current, used, out);
        current.pop();
        used[index] = false;
    }
}

/// One accepted terminal protocol path plus everything needed to audit it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathRecord {
    pub stages: Vec<CanonicalStageChoice>,
    pub observed_requests: Vec<ObservedRequest>,
}

/// The production-reachable space P discovered through the real endpoint.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProductionSpace {
    /// Canonical complete choice -> every accepted protocol path reaching it.
    pub complete_paths: BTreeMap<CanonicalCompleteChoice, Vec<PathRecord>>,
    /// Advertised answers that production rejected (SOUNDNESS defect).
    pub advertised_rejected: Vec<String>,
    /// Out-of-contract probes that production ACCEPTED (SOUNDNESS defect).
    pub out_of_contract_accepted: u64,
    /// Out-of-contract probes correctly rejected (expected diagnostics).
    pub out_of_contract_rejected: u64,
}

impl ProductionSpace {
    pub fn record_terminal(
        &mut self,
        stages: Vec<CanonicalStageChoice>,
        observed: &[ObservedRequest],
    ) {
        self.complete_paths
            .entry(CanonicalCompleteChoice(stages.clone()))
            .or_default()
            .push(PathRecord {
                stages,
                observed_requests: observed.to_vec(),
            });
    }
}

/// Materializes a probe shape into a real typed response for the request.
fn materialize_response(
    shape: &AnswerShape,
    request: &PlayerDecisionRequestV2,
) -> mtgml_decision::DecisionResponseV2 {
    let id = |value: u32| CandidateIdV1(value);
    let answer = match shape {
        AnswerShape::SelectOne(candidate) => DecisionAnswerV2::SelectOne {
            candidate_id: id(*candidate),
        },
        AnswerShape::SelectMany(candidate_ids) => DecisionAnswerV2::SelectMany {
            candidate_ids: candidate_ids.iter().map(|value| id(*value)).collect(),
        },
        AnswerShape::Number(value) => DecisionAnswerV2::ChooseNumber { value: *value },
        AnswerShape::Order(candidate_ids) => DecisionAnswerV2::Order {
            candidate_ids: candidate_ids.iter().map(|value| id(*value)).collect(),
        },
    };
    DecisionResponseV2 {
        schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
        player_decision_id: request.player_decision_id,
        state_revision: request.state_revision,
        answer,
    }
}

fn observe_request(
    request: &PlayerDecisionRequestV2,
    context: &ScenarioBindingContext,
) -> Result<ObservedRequest, ExplorationFailure> {
    let domain = match &request.decision {
        DecisionDomainV2::ChooseOne => ObservedDomain::ChooseOne,
        DecisionDomainV2::ChooseMany { minimum, maximum } => ObservedDomain::ChooseMany {
            minimum: *minimum,
            maximum: *maximum,
        },
        DecisionDomainV2::ChooseNumber { minimum, maximum } => ObservedDomain::ChooseNumber {
            minimum: *minimum,
            maximum: *maximum,
        },
        DecisionDomainV2::Order { minimum, maximum } => ObservedDomain::Order {
            minimum: *minimum,
            maximum: *maximum,
        },
    };
    let mut atoms = Vec::new();
    for candidate in &request.candidates {
        atoms.push(map_candidate(&candidate.intent, context)?);
    }
    Ok(ObservedRequest {
        domain,
        candidate_atoms: atoms,
    })
}

fn canonicalize_stage(
    shape: &AnswerShape,
    request: &PlayerDecisionRequestV2,
    context: &ScenarioBindingContext,
) -> Result<CanonicalStageChoice, ExplorationFailure> {
    let atom_of_id = |id: u32| -> Result<SyntheticChoiceAtom, ExplorationFailure> {
        let candidate = request
            .candidates
            .iter()
            .find(|candidate| candidate.candidate_id.0 == id)
            .ok_or(ExplorationFailure::Internal)?;
        Ok(map_candidate(&candidate.intent, context)?)
    };
    Ok(match shape {
        AnswerShape::SelectOne(_) => CanonicalStageChoice::Anchor,
        AnswerShape::SelectMany(candidate_ids) => {
            let mut set = BTreeSet::new();
            for id in candidate_ids {
                set.insert(atom_of_id(*id)?);
            }
            CanonicalStageChoice::Members(set)
        }
        AnswerShape::Number(value) => CanonicalStageChoice::Number(*value),
        AnswerShape::Order(candidate_ids) => {
            let mut atoms = Vec::new();
            for id in candidate_ids {
                atoms.push(atom_of_id(*id)?);
            }
            CanonicalStageChoice::Order(atoms)
        }
    })
}

/// Explores the REAL production protocol tree through fork-isolated branches.
pub fn explore(
    controller: &TrustedEnvironmentController,
    perspective: PlayerId,
    context: &ScenarioBindingContext,
    budget: ExplorerBudget,
) -> Result<ProductionSpace, ExplorationFailure> {
    let mut space = ProductionSpace::default();
    let mut nodes = 0u32;
    let mut path = Vec::new();
    let mut observed = Vec::new();
    walk(
        controller.clone(),
        perspective,
        context,
        budget,
        &mut nodes,
        0,
        &mut path,
        &mut observed,
        &mut space,
    )?;
    Ok(space)
}

#[allow(clippy::too_many_arguments)]
fn walk(
    controller: TrustedEnvironmentController,
    perspective: PlayerId,
    context: &ScenarioBindingContext,
    budget: ExplorerBudget,
    nodes: &mut u32,
    depth: u32,
    path: &mut Vec<CanonicalStageChoice>,
    observed: &mut Vec<ObservedRequest>,
    space: &mut ProductionSpace,
) -> Result<(), ExplorationFailure> {
    *nodes += 1;
    if *nodes > budget.max_total_nodes {
        return Err(ExplorationFailure::Bound(
            ExplorationBoundError::TotalNodesExceeded,
        ));
    }
    let endpoint = controller.bind_player(perspective).map_err(|_| ExplorationFailure::Internal)?;
    let request = endpoint.visible_decision().map_err(|_| ExplorationFailure::Internal)?;
    let Some(request) = request else {
        // Terminal: continuation completed for this branch.
        space.record_terminal(path.clone(), observed);
        return Ok(());
    };
    observed.push(observe_request(&request, context)?);

    for probe in generate_probes(&request, &budget)? {
        *nodes += 1;
        if *nodes > budget.max_total_nodes {
            return Err(ExplorationFailure::Bound(
                ExplorationBoundError::TotalNodesExceeded,
            ));
        }
        let response = materialize_response(&probe.shape, &request);
        let mut branch = controller.fork().map_err(|_| ExplorationFailure::Internal)?;
        let branch_endpoint = branch.bind_player(perspective).map_err(|_| ExplorationFailure::Internal)?;
        let step = branch_endpoint.submit(response).map_err(|_| ExplorationFailure::Internal)?;
        match &step.submission {
            mtgml_observation::PlayerStepSubmissionV1::Accepted => {
                if depth + 1 > budget.max_depth {
                    return Err(ExplorationFailure::Bound(
                        ExplorationBoundError::DepthExceeded,
                    ));
                }
                if !probe.advertised {
                    space.out_of_contract_accepted += 1;
                }
                let stage = canonicalize_stage(&probe.shape, &request, context)?;
                path.push(stage);
                walk(
                    branch,
                    perspective,
                    context,
                    budget,
                    nodes,
                    depth + 1,
                    path,
                    observed,
                    space,
                )?;
                path.pop();
            }
            mtgml_observation::PlayerStepSubmissionV1::Rejected { code } => {
                let shape_debug = format!("{probe:?}");
                if probe.advertised {
                    space.advertised_rejected.push(format!(
                        "advertised answer rejected: {shape_debug} code={code:?}"
                    ));
                } else {
                    space.out_of_contract_rejected += 1;
                }
            }
        }
    }
    observed.pop();
    Ok(())
}
