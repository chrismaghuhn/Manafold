//! Production protocol exploration through the REAL player-facing endpoint.
//!
//! Independence boundary (Issue #53): this module MUST NOT import anything
//! from the reference oracle module. Discovery depends exclusively
//! on the player-visible request plus an explicit resource budget. The
//! trusted controller is used ONLY for fork/checkpoint/branch isolation and
//! rejection non-mutation — never for candidate, number or order enumeration.

use std::collections::{BTreeMap, BTreeSet};

use mtgml_decision::{
    CandidateIntent, DecisionAnswerV2, DecisionDomainV2, DecisionResponseV2,
    PlayerDecisionRequestV2, DECISION_RESPONSE_V2_SCHEMA,
};
use mtgml_environment::{PlayerEndpoint, TrustedEnvironmentController};
use mtgml_model::{CandidateIdV1, OpaqueObjectId, PlayerId};

use crate::legal_space::canonical::{
    CanonicalCompleteChoice, CanonicalStageChoice, SyntheticChoiceAtom,
};

pub use super::LegalSpaceBudget as ExplorerBudget;

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
    #[error("Order probe range exceeds the bounded finite complement")]
    OrderProbeRangeExceeded,
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
    #[error("rejected probe mutated its isolated branch")]
    RejectedMutation,
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
        CandidateIntent::SelectMode { mode_index } => Ok(SyntheticChoiceAtom::Piece(*mode_index)),
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
    let ids: BTreeSet<u32> = request
        .candidates
        .iter()
        .map(|candidate| candidate.candidate_id.0)
        .collect();
    let in_bounds = |length: usize, minimum: u32, maximum: u32| {
        u32::try_from(length)
            .map(|length| minimum <= length && length <= maximum)
            .unwrap_or(false)
    };
    let unique =
        |values: &[u32]| values.iter().copied().collect::<BTreeSet<_>>().len() == values.len();
    match (&request.decision, shape) {
        (DecisionDomainV2::ChooseOne, AnswerShape::SelectOne(candidate)) => ids.contains(candidate),
        (
            DecisionDomainV2::ChooseMany { minimum, maximum },
            AnswerShape::SelectMany(candidate_ids),
        ) => {
            in_bounds(candidate_ids.len(), *minimum, *maximum)
                && unique(candidate_ids)
                && candidate_ids
                    .iter()
                    .all(|candidate| ids.contains(candidate))
                && candidate_ids.windows(2).all(|window| window[0] < window[1])
        }
        (DecisionDomainV2::ChooseNumber { minimum, maximum }, AnswerShape::Number(value)) => {
            minimum <= value && value <= maximum
        }
        (DecisionDomainV2::Order { minimum, maximum }, AnswerShape::Order(candidate_ids)) => {
            in_bounds(candidate_ids.len(), *minimum, *maximum)
                && unique(candidate_ids)
                && candidate_ids
                    .iter()
                    .all(|candidate| ids.contains(candidate))
        }
        _ => false,
    }
}

/// Pure probe generation: reads ONLY the visible request and the budget.
pub fn generate_probes(
    request: &PlayerDecisionRequestV2,
    budget: &ExplorerBudget,
) -> Result<Vec<Probe>, ExplorationBoundError> {
    let candidate_count = u32::try_from(request.candidates.len())
        .map_err(|_| ExplorationBoundError::CandidatesExceeded)?;
    if candidate_count > budget.max_candidates_per_request {
        return Err(ExplorationBoundError::CandidatesExceeded);
    }
    let ids = request_ids(request);
    let mut probes = Vec::new();
    match &request.decision {
        DecisionDomainV2::ChooseOne => {
            if ids.is_empty() {
                return Err(ExplorationBoundError::MalformedVisibleRequest);
            }
            for id in &ids {
                let shape = AnswerShape::SelectOne(*id);
                let advertised = is_advertised(request, &shape);
                probes.push(Probe { shape, advertised });
            }
            if let Some(unknown) = unknown_candidate_id(&ids) {
                probes.push(Probe {
                    shape: AnswerShape::SelectOne(unknown),
                    advertised: false,
                });
            }
            probes.push(Probe {
                shape: AnswerShape::SelectMany(Vec::new()),
                advertised: false,
            });
        }
        DecisionDomainV2::ChooseMany { minimum, .. } => {
            if *minimum > candidate_count {
                return Err(ExplorationBoundError::MalformedVisibleRequest);
            }
            let subsets = 1u64
                .checked_shl(candidate_count)
                .ok_or(ExplorationBoundError::GeneratedAnswersExceeded)?;
            let unknown = unknown_candidate_id(&ids);
            let mut total = subsets;
            if !ids.is_empty() {
                total = total
                    .checked_add(1)
                    .ok_or(ExplorationBoundError::GeneratedAnswersExceeded)?;
            }
            if unknown.is_some() {
                total = total
                    .checked_add(2)
                    .ok_or(ExplorationBoundError::GeneratedAnswersExceeded)?;
            }
            if ids.len() >= 2 {
                total = total
                    .checked_add(1)
                    .ok_or(ExplorationBoundError::GeneratedAnswersExceeded)?;
            }
            total = total
                .checked_add(1)
                .ok_or(ExplorationBoundError::GeneratedAnswersExceeded)?;
            if total > budget.max_generated_answers {
                return Err(ExplorationBoundError::GeneratedAnswersExceeded);
            }
            probes.reserve(usize::try_from(total).unwrap_or(usize::MAX));
            for mask in 0..subsets {
                let chosen: Vec<u32> = ids
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| mask & (1 << index) != 0)
                    .map(|(_, id)| *id)
                    .collect();
                let shape = AnswerShape::SelectMany(chosen);
                let advertised = is_advertised(request, &shape);
                probes.push(Probe { shape, advertised });
            }
            if let Some(first) = ids.first().copied() {
                probes.push(Probe {
                    shape: AnswerShape::SelectMany(vec![first, first]),
                    advertised: false,
                });
            }
            if let Some(unknown) = unknown {
                probes.push(Probe {
                    shape: AnswerShape::SelectMany(vec![unknown]),
                    advertised: false,
                });
                probes.push(Probe {
                    shape: AnswerShape::SelectMany(
                        ids.iter()
                            .copied()
                            .chain(std::iter::once(unknown))
                            .collect(),
                    ),
                    advertised: false,
                });
            }
            if ids.len() >= 2 {
                probes.push(Probe {
                    shape: AnswerShape::SelectMany(vec![ids[1], ids[0]]),
                    advertised: false,
                });
            }
            probes.push(Probe {
                shape: AnswerShape::Number(0),
                advertised: false,
            });
        }
        DecisionDomainV2::ChooseNumber { minimum, maximum } => {
            if minimum > maximum {
                return Err(ExplorationBoundError::MalformedVisibleRequest);
            }
            let span = i128::from(*maximum) - i128::from(*minimum) + 1;
            if span > i128::from(budget.max_numeric_span) {
                return Err(ExplorationBoundError::NumericSpanExceeded);
            }
            let total = u64::try_from(span)
                .ok()
                .and_then(|value| value.checked_add(3))
                .ok_or(ExplorationBoundError::GeneratedAnswersExceeded)?;
            if total > budget.max_generated_answers {
                return Err(ExplorationBoundError::GeneratedAnswersExceeded);
            }
            probes.reserve(usize::try_from(total).unwrap_or(usize::MAX));
            for value in *minimum..=*maximum {
                let shape = AnswerShape::Number(value);
                let advertised = is_advertised(request, &shape);
                probes.push(Probe { shape, advertised });
            }
            // Boundary sentinels OUTSIDE the visible range: an accepted
            // sentinel proves a too-permissive numeric surface.
            for sentinel in [minimum.checked_sub(1), maximum.checked_add(1)]
                .into_iter()
                .flatten()
            {
                probes.push(Probe {
                    shape: AnswerShape::Number(sentinel),
                    advertised: false,
                });
            }
            probes.push(Probe {
                shape: AnswerShape::SelectOne(0),
                advertised: false,
            });
        }
        DecisionDomainV2::Order { minimum, maximum } => {
            if minimum > maximum {
                return Err(ExplorationBoundError::MalformedVisibleRequest);
            }
            if *minimum > candidate_count {
                return Err(ExplorationBoundError::MalformedVisibleRequest);
            }
            let maximum_probe_length = candidate_count
                .checked_add(1)
                .ok_or(ExplorationBoundError::OrderProbeRangeExceeded)?;
            if *maximum > maximum_probe_length {
                return Err(ExplorationBoundError::OrderProbeRangeExceeded);
            }
            let unknown = unknown_candidate_id(&ids);
            let extended_ids = unknown
                .map(|unknown| {
                    ids.iter()
                        .copied()
                        .chain(std::iter::once(unknown))
                        .collect()
                })
                .unwrap_or_else(|| ids.clone());
            let mut estimated = 0u64;
            for length in 0..=maximum_probe_length {
                let source_count = if length <= candidate_count {
                    u64::from(candidate_count)
                } else {
                    u64::from(maximum_probe_length)
                };
                estimated = estimated
                    .checked_add(
                        permutations_count(source_count, u64::from(length))
                            .ok_or(ExplorationBoundError::GeneratedAnswersExceeded)?,
                    )
                    .ok_or(ExplorationBoundError::GeneratedAnswersExceeded)?;
            }
            if !ids.is_empty() {
                estimated = estimated
                    .checked_add(1)
                    .ok_or(ExplorationBoundError::GeneratedAnswersExceeded)?;
            }
            if unknown.is_some() {
                estimated = estimated
                    .checked_add(1)
                    .ok_or(ExplorationBoundError::GeneratedAnswersExceeded)?;
            }
            estimated = estimated
                .checked_add(1)
                .ok_or(ExplorationBoundError::GeneratedAnswersExceeded)?;
            if estimated > budget.max_generated_answers {
                return Err(ExplorationBoundError::GeneratedAnswersExceeded);
            }
            probes.reserve(usize::try_from(estimated).unwrap_or(usize::MAX));
            for length in 0..=maximum_probe_length {
                let source = if length <= candidate_count {
                    &ids
                } else {
                    &extended_ids
                };
                for sequence in permutations_of(source, length as usize) {
                    let shape = AnswerShape::Order(sequence);
                    let advertised = is_advertised(request, &shape);
                    probes.push(Probe { shape, advertised });
                }
            }
            if let Some(first) = ids.first().copied() {
                probes.push(Probe {
                    shape: AnswerShape::Order(vec![first, first]),
                    advertised: false,
                });
            }
            if let Some(unknown) = unknown {
                probes.push(Probe {
                    shape: AnswerShape::Order(vec![unknown]),
                    advertised: false,
                });
            }
            probes.push(Probe {
                shape: AnswerShape::Number(0),
                advertised: false,
            });
        }
    }
    Ok(probes)
}

fn request_ids(request: &PlayerDecisionRequestV2) -> Vec<u32> {
    request
        .candidates
        .iter()
        .map(|candidate| candidate.candidate_id.0)
        .collect()
}

fn unknown_candidate_id(ids: &[u32]) -> Option<u32> {
    ids.iter()
        .copied()
        .max()
        .and_then(|id| id.checked_add(1))
        .or_else(|| ids.is_empty().then_some(0))
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
) -> DecisionResponseV2 {
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
    *nodes = nodes.checked_add(1).ok_or(ExplorationFailure::Bound(
        ExplorationBoundError::TotalNodesExceeded,
    ))?;
    if *nodes > budget.max_total_nodes {
        return Err(ExplorationFailure::Bound(
            ExplorationBoundError::TotalNodesExceeded,
        ));
    }
    let endpoint = controller
        .bind_player(perspective)
        .map_err(|_| ExplorationFailure::Internal)?;
    let request = endpoint
        .visible_decision()
        .map_err(|_| ExplorationFailure::Internal)?;
    let Some(request) = request else {
        // Terminal: continuation completed for this branch.
        space.record_terminal(path.clone(), observed);
        return Ok(());
    };
    observed.push(observe_request(&request, context)?);

    for probe in generate_probes(&request, &budget)? {
        let response = materialize_response(&probe.shape, &request);
        let branch = controller
            .fork()
            .map_err(|_| ExplorationFailure::Internal)?;
        let branch_endpoint = branch
            .bind_player(perspective)
            .map_err(|_| ExplorationFailure::Internal)?;
        let before_submit = branch
            .checkpoint()
            .map_err(|_| ExplorationFailure::Internal)?;
        let step = branch_endpoint
            .submit(response)
            .map_err(|_| ExplorationFailure::Internal)?;
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
                let after_submit = branch
                    .checkpoint()
                    .map_err(|_| ExplorationFailure::Internal)?;
                if after_submit != before_submit {
                    return Err(ExplorationFailure::RejectedMutation);
                }
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
