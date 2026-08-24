//! Authorization relations over authoritative state.
//!
//! Relations compare the AUTHORIZED projections of two authoritative states,
//! computed here directly from authoritative fields. Production projection
//! functions and raw trusted structs are never used as comparison inputs:
//! every ignored field below is invisible to a perspective by contract, not
//! by omission of the comparator.

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use mtgml_decision::AuthoritativeDecisionRequestV2;
use mtgml_model::{AbilityInstanceId, GameObjectId, OpaqueObjectId, PlayerId, ZoneKind};
use mtgml_state::{
    EngineState, KnownLocationFactV2, PlayerKnowledgeStateV2, RetiredKnowledgeRecordV2,
    ZoneLocation,
};

/// Outcome of one authorization relation over a state pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelationOutcome {
    Equal,
    Diverges {
        /// Precise field path of the first observed divergence.
        path: String,
    },
}

/// Outcome of the trusted renaming-bijection check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BijectionOutcome {
    Holds,
    UnexplainedDifference { path: String },
}

/// Closed violation vocabulary for one witness assertion; paths are precise
/// field-path strings for diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WitnessViolation {
    KnowledgeDivergence {
        path: String,
    },
    DecisionDivergence {
        path: String,
    },
    BijectionUnexplained {
        path: String,
    },
    /// The pair declared an authoritative difference that does not hold.
    VacuousPair,
    MissingPerspective,
}

/// Whether a pair declares an authoritative difference that must actually
/// hold. A `Required` pair whose sides are equal is vacuous evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NonVacuityPredicate {
    None,
    Required,
}

/// The authorized object-renaming bijection between paired states: hidden
/// authoritative identities may differ exactly along this mapping.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TrustedRenamingBijection {
    pub objects: BTreeMap<GameObjectId, GameObjectId>,
    pub abilities: BTreeMap<AbilityInstanceId, AbilityInstanceId>,
}

/// One perspective's complete witness for a state pair.
///
/// The knowledge and decision relations are computed from states A/B at
/// construction time; `bijection: None` authorizes no renaming, so any
/// mapping-target difference is then unexplained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairWitness {
    pub perspective: PlayerId,
    pub knowledge_relation: RelationOutcome,
    pub decision_relation: RelationOutcome,
    pub bijection: Option<TrustedRenamingBijection>,
    pub expected_difference: NonVacuityPredicate,
}

impl PairWitness {
    /// Builds the witness for `perspective` from states A/B.
    pub fn build(
        perspective: PlayerId,
        a: &EngineState,
        b: &EngineState,
        bijection: Option<TrustedRenamingBijection>,
        expected_difference: NonVacuityPredicate,
    ) -> Result<Self, WitnessViolation> {
        let knowledge_a = a
            .knowledge
            .players
            .get(&perspective)
            .ok_or(WitnessViolation::MissingPerspective)?;
        let knowledge_b = b
            .knowledge
            .players
            .get(&perspective)
            .ok_or(WitnessViolation::MissingPerspective)?;
        Ok(Self {
            perspective,
            knowledge_relation: relate_knowledge(knowledge_a, knowledge_b),
            decision_relation: relate_decision(pending_request(a), pending_request(b)),
            bijection,
            expected_difference,
        })
    }
}

fn pending_request(state: &EngineState) -> Option<&AuthoritativeDecisionRequestV2> {
    state
        .execution
        .pending_decision
        .as_ref()
        .map(|pending| &pending.request)
}

/// Public location part of a known-location fact: `{zone, player}` only.
///
/// `ZonePosition`, the visibility partition, and the partition identifier
/// are unauthorized detail and are deliberately reduced away.
fn location_public_part(fact: &KnownLocationFactV2) -> (ZoneKind, Option<PlayerId>) {
    let ZoneLocation { zone, player, .. } = &fact.location;
    (*zone, *player)
}

fn diverged(path: impl Into<String>) -> RelationOutcome {
    RelationOutcome::Diverges { path: path.into() }
}

fn relate_optional_fact(
    a: Option<&KnownLocationFactV2>,
    b: Option<&KnownLocationFactV2>,
    path: String,
) -> RelationOutcome {
    match (a, b) {
        (None, None) => RelationOutcome::Equal,
        (Some(fact_a), Some(fact_b)) => {
            if location_public_part(fact_a) != location_public_part(fact_b) {
                return diverged(format!("{path}.public_part"));
            }
            if fact_a.provenance != fact_b.provenance {
                return diverged(format!("{path}.provenance"));
            }
            RelationOutcome::Equal
        }
        _ => diverged(path),
    }
}

fn relate_fact_history(
    a: &[KnownLocationFactV2],
    b: &[KnownLocationFactV2],
    path: String,
) -> RelationOutcome {
    if a.len() != b.len() {
        return diverged(format!("{path}.len"));
    }
    for (index, (fact_a, fact_b)) in a.iter().zip(b.iter()).enumerate() {
        let outcome = relate_optional_fact(Some(fact_a), Some(fact_b), format!("{path}[{index}]"));
        if outcome != RelationOutcome::Equal {
            return outcome;
        }
    }
    RelationOutcome::Equal
}

/// Relates two perspectives' retained knowledge states.
///
/// Equal-set: active/retired opaque key sets; the definition-known marker
/// (`card_definition` compared as an `Option`, where present); the public
/// location part of current, historical, and last-known facts; provenance
/// values including their visible sequences as-is; invalidation records;
/// the next visible sequence.
///
/// Ignored by contract: `physical_card`, `ZonePosition`, visibility
/// partition, partition identifier, and authoritative mapping targets.
pub fn relate_knowledge(a: &PlayerKnowledgeStateV2, b: &PlayerKnowledgeStateV2) -> RelationOutcome {
    if a.next_visible_sequence != b.next_visible_sequence {
        return diverged("next_visible_sequence");
    }
    if active_keys(a) != active_keys(b) {
        return diverged("active.keys");
    }
    if retired_keys(a) != retired_keys(b) {
        return diverged("retired.keys");
    }
    for (opaque, record_a) in &a.active {
        let Some(record_b) = b.active.get(opaque) else {
            return diverged(format!("active[{opaque:?}]"));
        };
        let prefix = format!("active[{opaque:?}]");
        if record_a.card_definition != record_b.card_definition {
            return diverged(format!("{prefix}.card_definition"));
        }
        let outcome = relate_optional_fact(
            record_a.known_location.as_ref(),
            record_b.known_location.as_ref(),
            format!("{prefix}.known_location"),
        );
        if outcome != RelationOutcome::Equal {
            return outcome;
        }
        let outcome = relate_fact_history(
            &record_a.historical_locations,
            &record_b.historical_locations,
            format!("{prefix}.historical_locations"),
        );
        if outcome != RelationOutcome::Equal {
            return outcome;
        }
        if record_a.acquisition != record_b.acquisition {
            return diverged(format!("{prefix}.acquisition"));
        }
    }
    for (opaque, record_a) in &a.retired {
        let Some(record_b) = b.retired.get(opaque) else {
            return diverged(format!("retired[{opaque:?}]"));
        };
        let outcome = relate_retired_record(record_a, record_b);
        if outcome != RelationOutcome::Equal {
            return outcome;
        }
    }
    RelationOutcome::Equal
}

fn relate_retired_record(
    a: &RetiredKnowledgeRecordV2,
    b: &RetiredKnowledgeRecordV2,
) -> RelationOutcome {
    let opaque = a.opaque_object;
    let prefix = format!("retired[{opaque:?}]");
    if a.card_definition != b.card_definition {
        return diverged(format!("{prefix}.card_definition"));
    }
    let outcome = relate_optional_fact(
        a.last_known_location.as_ref(),
        b.last_known_location.as_ref(),
        format!("{prefix}.last_known_location"),
    );
    if outcome != RelationOutcome::Equal {
        return outcome;
    }
    let outcome = relate_fact_history(
        &a.historical_locations,
        &b.historical_locations,
        format!("{prefix}.historical_locations"),
    );
    if outcome != RelationOutcome::Equal {
        return outcome;
    }
    if a.acquisition != b.acquisition {
        return diverged(format!("{prefix}.acquisition"));
    }
    if a.invalidation != b.invalidation {
        return diverged(format!("{prefix}.invalidation"));
    }
    RelationOutcome::Equal
}

fn active_keys(state: &PlayerKnowledgeStateV2) -> BTreeSet<OpaqueObjectId> {
    state.active.keys().copied().collect()
}

fn retired_keys(state: &PlayerKnowledgeStateV2) -> BTreeSet<OpaqueObjectId> {
    state.retired.keys().copied().collect()
}

/// Relates two authoritative decision requests.
///
/// Equal-set: presence, `player_decision_id`, `state_revision`, actor,
/// visibility, domain variant with bounds, and candidate count, order,
/// `CandidateIdV1`s, and visible intent payloads.
///
/// Ignored by contract: `decision_id`, `continuation_id`, and trusted
/// candidate bindings.
pub fn relate_decision(
    a: Option<&AuthoritativeDecisionRequestV2>,
    b: Option<&AuthoritativeDecisionRequestV2>,
) -> RelationOutcome {
    match (a, b) {
        (None, None) => RelationOutcome::Equal,
        (Some(request_a), Some(request_b)) => {
            if request_a.player_decision_id != request_b.player_decision_id {
                return diverged("player_decision_id");
            }
            if request_a.state_revision != request_b.state_revision {
                return diverged("state_revision");
            }
            if request_a.actor != request_b.actor {
                return diverged("actor");
            }
            if request_a.visibility != request_b.visibility {
                return diverged("visibility");
            }
            if request_a.decision != request_b.decision {
                return diverged("domain");
            }
            if request_a.candidates.len() != request_b.candidates.len() {
                return diverged("candidates.len");
            }
            for (index, (candidate_a, candidate_b)) in request_a
                .candidates
                .iter()
                .zip(request_b.candidates.iter())
                .enumerate()
            {
                if candidate_a.candidate_id != candidate_b.candidate_id {
                    return diverged(format!("candidates[{index}].candidate_id"));
                }
                if candidate_a.visible_intent != candidate_b.visible_intent {
                    return diverged(format!("candidates[{index}].visible_intent"));
                }
            }
            RelationOutcome::Equal
        }
        _ => diverged("presence"),
    }
}

/// Checks that opaque keys, per-perspective allocators, and retired sets are
/// equal between sides and that every mapping-target difference is explained
/// by the bijection. Any unexplained difference is a violation.
pub fn check_bijection(
    a_state: &EngineState,
    b_state: &EngineState,
    bijection: &TrustedRenamingBijection,
) -> BijectionOutcome {
    let perspectives: BTreeSet<PlayerId> = a_state
        .perspective_identities
        .players
        .keys()
        .chain(b_state.perspective_identities.players.keys())
        .copied()
        .collect();
    for perspective in perspectives {
        let prefix = format!("perspectives[{perspective:?}]");
        let Some(identity_a) = a_state.perspective_identities.players.get(&perspective) else {
            return BijectionOutcome::UnexplainedDifference {
                path: format!("{prefix}.missing_in_a"),
            };
        };
        let Some(identity_b) = b_state.perspective_identities.players.get(&perspective) else {
            return BijectionOutcome::UnexplainedDifference {
                path: format!("{prefix}.missing_in_b"),
            };
        };
        if keys_of(&identity_a.opaque_to_object) != keys_of(&identity_b.opaque_to_object) {
            return BijectionOutcome::UnexplainedDifference {
                path: format!("{prefix}.opaque_object_keys"),
            };
        }
        if keys_of(&identity_a.opaque_to_ability) != keys_of(&identity_b.opaque_to_ability) {
            return BijectionOutcome::UnexplainedDifference {
                path: format!("{prefix}.opaque_ability_keys"),
            };
        }
        if identity_a.retired_object_ids != identity_b.retired_object_ids {
            return BijectionOutcome::UnexplainedDifference {
                path: format!("{prefix}.retired_object_ids"),
            };
        }
        if identity_a.retired_ability_ids != identity_b.retired_ability_ids {
            return BijectionOutcome::UnexplainedDifference {
                path: format!("{prefix}.retired_ability_ids"),
            };
        }
        if identity_a.next_opaque_object_id != identity_b.next_opaque_object_id
            || identity_a.next_opaque_ability_id != identity_b.next_opaque_ability_id
            || identity_a.next_player_decision_id != identity_b.next_player_decision_id
        {
            return BijectionOutcome::UnexplainedDifference {
                path: format!("{prefix}.allocators"),
            };
        }
        for (opaque, target_a) in &identity_a.opaque_to_object {
            let Some(target_b) = identity_b.opaque_to_object.get(opaque) else {
                return BijectionOutcome::UnexplainedDifference {
                    path: format!("{prefix}.object_mapping[{opaque:?}]"),
                };
            };
            if target_a != target_b && bijection.objects.get(target_a) != Some(target_b) {
                return BijectionOutcome::UnexplainedDifference {
                    path: format!("{prefix}.object_mapping[{opaque:?}]"),
                };
            }
        }
        for (opaque, target_a) in &identity_a.opaque_to_ability {
            let Some(target_b) = identity_b.opaque_to_ability.get(opaque) else {
                return BijectionOutcome::UnexplainedDifference {
                    path: format!("{prefix}.ability_mapping[{opaque:?}]"),
                };
            };
            if target_a != target_b && bijection.abilities.get(target_a) != Some(target_b) {
                return BijectionOutcome::UnexplainedDifference {
                    path: format!("{prefix}.ability_mapping[{opaque:?}]"),
                };
            }
        }
    }
    BijectionOutcome::Holds
}

fn keys_of<Key: Copy + Ord, Value>(map: &BTreeMap<Key, Value>) -> BTreeSet<Key> {
    map.keys().copied().collect()
}

/// Runs the full witness assertion over a state pair: knowledge relation,
/// decision relation, bijection explanation (with no renaming authorized
/// when absent), then the non-vacuity predicate.
pub fn assert_witness(
    a: &EngineState,
    b: &EngineState,
    witness: &PairWitness,
) -> Result<(), WitnessViolation> {
    match &witness.knowledge_relation {
        RelationOutcome::Equal => {}
        RelationOutcome::Diverges { path } => {
            return Err(WitnessViolation::KnowledgeDivergence { path: path.clone() });
        }
    }
    match &witness.decision_relation {
        RelationOutcome::Equal => {}
        RelationOutcome::Diverges { path } => {
            return Err(WitnessViolation::DecisionDivergence { path: path.clone() });
        }
    }
    let default_bijection = TrustedRenamingBijection::default();
    let bijection = witness.bijection.as_ref().unwrap_or(&default_bijection);
    match check_bijection(a, b, bijection) {
        BijectionOutcome::Holds => {}
        BijectionOutcome::UnexplainedDifference { path } => {
            return Err(WitnessViolation::BijectionUnexplained { path });
        }
    }
    match witness.expected_difference {
        NonVacuityPredicate::None => {}
        NonVacuityPredicate::Required if a == b => {
            return Err(WitnessViolation::VacuousPair);
        }
        NonVacuityPredicate::Required => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::isolation::paired::{
        base_pair_state,
        test_support::{rename_hidden_object, renaming_bijection},
    };

    const P1: PlayerId = PlayerId(1);
    const P2: PlayerId = PlayerId(2);

    #[test]
    fn identical_clone_satisfies_all_relations() {
        let state = base_pair_state(&"11".repeat(32)).unwrap();
        let clone = state.clone();
        let knowledge_a = &state.knowledge.players[&P1];
        assert_eq!(
            relate_knowledge(knowledge_a, &clone.knowledge.players[&P1]),
            RelationOutcome::Equal
        );
        let pending_a = state
            .execution
            .pending_decision
            .as_ref()
            .map(|pending| &pending.request);
        let pending_b = clone
            .execution
            .pending_decision
            .as_ref()
            .map(|pending| &pending.request);
        assert_eq!(
            relate_decision(pending_a, pending_b),
            RelationOutcome::Equal
        );

        let witness =
            PairWitness::build(P2, &state, &clone, None, NonVacuityPredicate::None).unwrap();
        assert_witness(&state, &clone, &witness).unwrap();

        // An identical clone cannot carry a declared difference.
        let vacuous =
            PairWitness::build(P2, &state, &clone, None, NonVacuityPredicate::Required).unwrap();
        assert_eq!(
            assert_witness(&state, &clone, &vacuous),
            Err(WitnessViolation::VacuousPair)
        );
    }

    #[test]
    fn renaming_bijection_accepts_target_differences() {
        let state_a = base_pair_state(&"11".repeat(32)).unwrap();
        let mut state_b = state_a.clone();
        rename_hidden_object(&mut state_b);

        let explained = PairWitness::build(
            P2,
            &state_a,
            &state_b,
            Some(renaming_bijection()),
            NonVacuityPredicate::Required,
        )
        .unwrap();
        assert_witness(&state_a, &state_b, &explained).unwrap();

        let unexplained =
            PairWitness::build(P2, &state_a, &state_b, None, NonVacuityPredicate::Required)
                .unwrap();
        assert!(matches!(
            assert_witness(&state_a, &state_b, &unexplained),
            Err(WitnessViolation::BijectionUnexplained { .. })
        ));
    }

    #[test]
    fn relations_detect_unauthorized_divergence_paths() {
        let mut state_a = base_pair_state(&"11".repeat(32)).unwrap();
        let mut state_b = state_a.clone();
        // P2's definition-knowledge marker is authorized surface: removing it
        // from B must diverge on the exact record path.
        state_b
            .knowledge
            .players
            .get_mut(&P2)
            .unwrap()
            .active
            .get_mut(&OpaqueObjectId(2))
            .unwrap()
            .card_definition = None;
        let knowledge_a = &state_a.knowledge.players[&P2];
        assert_eq!(
            relate_knowledge(knowledge_a, &state_b.knowledge.players[&P2]),
            RelationOutcome::Diverges {
                path: "active[OpaqueObjectId(2)].card_definition".to_owned()
            }
        );
        // A changed trusted binding alone stays inside the relation: the
        // visible intent is unchanged, so the pair remains related.
        let request = state_a.execution.pending_decision.take().unwrap().request;
        let mut rebound = request.clone();
        rebound.candidates[0].trusted_binding =
            mtgml_decision::EngineCandidateBinding::PassPriority;
        assert_eq!(
            relate_decision(Some(&request), Some(&rebound)),
            RelationOutcome::Equal
        );
        // Presence divergence carries its own path.
        assert_eq!(
            relate_decision(Some(&request), None),
            RelationOutcome::Diverges {
                path: "presence".to_owned()
            }
        );
    }
}
