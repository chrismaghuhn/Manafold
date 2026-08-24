//! Authorization relations over authoritative state.
//!
//! Relations compare the AUTHORIZED projections of two authoritative states,
//! computed here directly from authoritative fields. Production projection
//! functions and raw trusted structs are never used as comparison inputs:
//! every ignored field below is invisible to a perspective by contract, not
//! by omission of the comparator.
//!
//! All relations are scoped to one witness perspective: another player's
//! private knowledge, identity mappings, allocators, and retirement history
//! are unauthorized hidden state and may legitimately differ between paired
//! states without affecting the relation outcome.

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
    NotInjective { path: String },
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
    /// The declared renaming bijection maps two sources onto one target and
    /// is therefore no renaming at all.
    BijectionNotInjective {
        path: String,
    },
    /// The pair declared an authoritative difference that does not hold.
    VacuousPair,
    MissingPerspective,
}

/// Whether a pair declares an authoritative difference that must actually
/// hold. `Required` demands whole-state inequality; every axis variant is an
/// executable declared-difference predicate over the authoritative states
/// (evaluated with the small composable helpers at the bottom of this
/// module). A pair whose declared difference does not hold is vacuous
/// evidence and must fail its axis test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NonVacuityPredicate {
    None,
    Required,
    OpponentHiddenDefinition,
    HiddenConcealedOrdering,
    ForeignPrivateLook,
    FaceDownIdentity,
    RootSeedPreAuth,
    HiddenRngCursor,
    ObjectRenaming,
    AbilityRenaming,
    GlobalAllocatorHistory,
    ForeignKnowledgeHistory,
}

/// The authorized object-renaming bijection between paired states: hidden
/// authoritative identities may differ exactly along this mapping.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TrustedRenamingBijection {
    pub objects: BTreeMap<GameObjectId, GameObjectId>,
    pub abilities: BTreeMap<AbilityInstanceId, AbilityInstanceId>,
}

/// State-independent witness policy for one perspective.
///
/// A witness carries only declared policy — whose perspective authorizes
/// the pair, which renaming differences are explained, and which
/// authoritative difference must hold. It never caches relation outcomes:
/// `assert_witness` recomputes every relation from the actual pair at
/// assertion time, so a stale witness can never accept a pair whose
/// authorized surfaces changed after construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairWitness {
    pub perspective: PlayerId,
    pub bijection: Option<TrustedRenamingBijection>,
    pub expected_difference: NonVacuityPredicate,
}

impl PairWitness {
    /// Builds the state-independent witness policy for `perspective`.
    pub fn new(
        perspective: PlayerId,
        bijection: Option<TrustedRenamingBijection>,
        expected_difference: NonVacuityPredicate,
    ) -> Self {
        Self {
            perspective,
            bijection,
            expected_difference,
        }
    }
}

fn pending_request(state: &EngineState) -> Option<&AuthoritativeDecisionRequestV2> {
    state
        .execution
        .pending_decision
        .as_ref()
        .map(|pending| &pending.request)
}

/// The decision surface `perspective` is authorized to compare. A pending
/// request exists in the relation only when its actor is the perspective;
/// a foreign-actor request is invisible to the relation, including the fact
/// that it exists.
fn authorized_decision_view(
    request: Option<&AuthoritativeDecisionRequestV2>,
    perspective: PlayerId,
) -> Option<&AuthoritativeDecisionRequestV2> {
    request.filter(|request| request.actor == perspective)
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

/// Relates the decision surface of `perspective` between two authoritative
/// states. Each side is first reduced to its authorized view: no pending
/// request, or a request whose actor is `perspective`. A foreign-actor
/// request therefore compares as absent — its existence is not P's knowledge.
///
/// Equal-set for authorized views: presence, `player_decision_id`,
/// `state_revision`, actor, visibility, domain variant with bounds, and
/// candidate count, order, `CandidateIdV1`s, and visible intent payloads.
///
/// Ignored by contract: `decision_id`, `continuation_id`, and trusted
/// candidate bindings.
pub fn relate_decision(
    perspective: PlayerId,
    a: Option<&AuthoritativeDecisionRequestV2>,
    b: Option<&AuthoritativeDecisionRequestV2>,
) -> RelationOutcome {
    let view_a = authorized_decision_view(a, perspective);
    let view_b = authorized_decision_view(b, perspective);
    match (view_a, view_b) {
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

/// Checks — scoped to `perspective` — that the perspective's opaque keys,
/// per-perspective allocators, and retired sets are equal between sides and
/// that every mapping-target difference is explained by the bijection. The
/// bijection itself must be injective in both directions (sources are unique
/// map keys; a duplicated target collapses two identities into one and is
/// rejected). Any unexplained difference is a violation.
///
/// Other players' private identity state is unauthorized hidden information:
/// their mappings, allocators, and retirement history may differ freely
/// between paired states and are deliberately not examined here.
pub fn check_bijection(
    perspective: PlayerId,
    a_state: &EngineState,
    b_state: &EngineState,
    bijection: &TrustedRenamingBijection,
) -> BijectionOutcome {
    if !is_injective(&bijection.objects) {
        return BijectionOutcome::NotInjective {
            path: "objects".to_owned(),
        };
    }
    if !is_injective(&bijection.abilities) {
        return BijectionOutcome::NotInjective {
            path: "abilities".to_owned(),
        };
    }
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
    BijectionOutcome::Holds
}

fn keys_of<Key: Copy + Ord, Value>(map: &BTreeMap<Key, Value>) -> BTreeSet<Key> {
    map.keys().copied().collect()
}

fn is_injective<Key: Copy + Ord, Target: Copy + Ord>(mapping: &BTreeMap<Key, Target>) -> bool {
    let targets: BTreeSet<Target> = mapping.values().copied().collect();
    targets.len() == mapping.len()
}

/// Runs the full witness assertion over a state pair, recomputing every
/// relation from the actual pair at assertion time: knowledge relation,
/// decision relation over authorized views, bijection explanation (with no
/// renaming authorized when absent), then the non-vacuity predicate. A
/// declared axis difference that does not hold is vacuous evidence and
/// fails closed. Because nothing is cached on the witness, a witness built
/// for one pair can never accept a later pair whose authorized surfaces
/// diverged.
pub fn assert_witness(
    a: &EngineState,
    b: &EngineState,
    witness: &PairWitness,
) -> Result<(), WitnessViolation> {
    let knowledge_a = a
        .knowledge
        .players
        .get(&witness.perspective)
        .ok_or(WitnessViolation::MissingPerspective)?;
    let knowledge_b = b
        .knowledge
        .players
        .get(&witness.perspective)
        .ok_or(WitnessViolation::MissingPerspective)?;
    match relate_knowledge(knowledge_a, knowledge_b) {
        RelationOutcome::Equal => {}
        RelationOutcome::Diverges { path } => {
            return Err(WitnessViolation::KnowledgeDivergence { path });
        }
    }
    match relate_decision(
        witness.perspective,
        authorized_decision_view(pending_request(a), witness.perspective),
        authorized_decision_view(pending_request(b), witness.perspective),
    ) {
        RelationOutcome::Equal => {}
        RelationOutcome::Diverges { path } => {
            return Err(WitnessViolation::DecisionDivergence { path });
        }
    }
    let default_bijection = TrustedRenamingBijection::default();
    let bijection = witness.bijection.as_ref().unwrap_or(&default_bijection);
    match check_bijection(witness.perspective, a, b, bijection) {
        BijectionOutcome::Holds => {}
        BijectionOutcome::UnexplainedDifference { path } => {
            return Err(WitnessViolation::BijectionUnexplained { path });
        }
        BijectionOutcome::NotInjective { path } => {
            return Err(WitnessViolation::BijectionNotInjective { path });
        }
    }
    let declared_difference_holds = match witness.expected_difference {
        NonVacuityPredicate::None => true,
        predicate => predicate.difference_holds(a, b, witness),
    };
    if !declared_difference_holds {
        return Err(WitnessViolation::VacuousPair);
    }
    Ok(())
}

impl NonVacuityPredicate {
    /// Evaluates the executable declared-difference predicate of this axis
    /// over the two authoritative states. Every axis variant is composed
    /// from the small helpers below; none uses reflection and all read only
    /// authoritative fields.
    pub fn difference_holds(self, a: &EngineState, b: &EngineState, witness: &PairWitness) -> bool {
        match self {
            Self::None => true,
            Self::Required => a != b,
            Self::OpponentHiddenDefinition => {
                hidden_definition_multiset(a) != hidden_definition_multiset(b)
                    && face_down_count(a) == face_down_count(b)
                    && authorized_visible_counts_equal(a, b, witness.perspective)
            }
            Self::HiddenConcealedOrdering => concealed_order_diverges(a, b),
            Self::ForeignPrivateLook => {
                foreign_record_set_differs(a, b, witness.perspective)
                    && authorized_visible_counts_equal(a, b, witness.perspective)
            }
            Self::FaceDownIdentity => face_down_physical_swap(a, b),
            Self::RootSeedPreAuth => {
                a.random.root_seed != b.random.root_seed
                    && a.revision == b.revision
                    && authorized_visible_counts_equal(a, b, witness.perspective)
            }
            Self::HiddenRngCursor => global_cursor(a) != global_cursor(b),
            Self::ObjectRenaming => renaming_difference_holds(a, b, &witness.bijection),
            Self::AbilityRenaming => ability_renaming_difference_holds(a, b, &witness.bijection),
            Self::GlobalAllocatorHistory => {
                a.allocators != b.allocators
                    && a.perspective_identities.players.get(&witness.perspective)
                        == b.perspective_identities.players.get(&witness.perspective)
            }
            Self::ForeignKnowledgeHistory => {
                foreign_history_differs(a, b, witness.perspective)
                    && authorized_visible_counts_equal(a, b, witness.perspective)
            }
        }
    }
}

/// Multiset of card definitions over face-down objects (trusted side).
fn hidden_definition_multiset(
    state: &EngineState,
) -> BTreeMap<mtgml_model::CardDefinitionId, usize> {
    let mut multiset = BTreeMap::new();
    for object in state
        .zones
        .objects
        .values()
        .filter(|object| object.face_down)
    {
        *multiset.entry(object.card_definition).or_default() += 1;
    }
    multiset
}

fn face_down_count(state: &EngineState) -> usize {
    state
        .zones
        .objects
        .values()
        .filter(|object| object.face_down)
        .count()
}

/// Authorized visible counts of `perspective`: opaque active-record keys and
/// the next visible sequence must be equal between sides.
fn authorized_visible_counts_equal(
    a: &EngineState,
    b: &EngineState,
    perspective: PlayerId,
) -> bool {
    let knowledge_a = a.knowledge.players.get(&perspective);
    let knowledge_b = b.knowledge.players.get(&perspective);
    match (knowledge_a, knowledge_b) {
        (Some(knowledge_a), Some(knowledge_b)) => {
            keys_of(&knowledge_a.active) == keys_of(&knowledge_b.active)
                && knowledge_a.next_visible_sequence == knowledge_b.next_visible_sequence
        }
        _ => false,
    }
}

/// The concealed ordered-zone vectors differ between sides while each
/// permuted vector preserves the other's member multiset and at least two
/// objects are concealed.
fn concealed_order_diverges(a: &EngineState, b: &EngineState) -> bool {
    if face_down_count(a) < 2 || face_down_count(b) < 2 {
        return false;
    }
    let mut diverges = false;
    for (key, members_a) in &a.zones.ordered_zones {
        if members_a.len() < 2 {
            continue;
        }
        let Some(members_b) = b.zones.ordered_zones.get(key) else {
            return false;
        };
        let mut sorted_a = members_a.clone();
        let mut sorted_b = members_b.clone();
        sorted_a.sort_unstable();
        sorted_b.sort_unstable();
        if sorted_a != sorted_b || members_a.len() != members_b.len() {
            return false;
        }
        if members_a != members_b {
            diverges = true;
        }
    }
    diverges
}

/// The non-witness player's retained record set differs while both sides
/// remain otherwise comparable (two-player M2 shape).
fn foreign_record_set_differs(a: &EngineState, b: &EngineState, perspective: PlayerId) -> bool {
    let mut differs = false;
    for (player, knowledge_a) in &a.knowledge.players {
        if *player == perspective {
            continue;
        }
        let Some(knowledge_b) = b.knowledge.players.get(player) else {
            return false;
        };
        if keys_of(&knowledge_a.active) != keys_of(&knowledge_b.active) {
            differs = true;
        }
    }
    differs && a.knowledge.players.len() == b.knowledge.players.len()
}

/// The assignment of physical cards to face-down incarnations differs while
/// the physical-card multiset is preserved (a swap among at least two).
fn face_down_physical_swap(a: &EngineState, b: &EngineState) -> bool {
    let assignment =
        |state: &EngineState| -> BTreeMap<GameObjectId, Option<mtgml_model::PhysicalCardId>> {
            state
                .zones
                .objects
                .values()
                .filter(|object| object.face_down)
                .map(|object| (object.id, object.physical_card))
                .collect()
        };
    let assignment_a = assignment(a);
    let assignment_b = assignment(b);
    let multiset = |values: &BTreeMap<
        GameObjectId,
        Option<mtgml_model::PhysicalCardId>,
    >| -> BTreeMap<Option<mtgml_model::PhysicalCardId>, usize> {
        let mut multiset = BTreeMap::new();
        for physical in values.values().copied() {
            *multiset.entry(physical).or_default() += 1;
        }
        multiset
    };
    assignment_a.len() >= 2
        && multiset(&assignment_a) == multiset(&assignment_b)
        && assignment_a != assignment_b
}

fn global_cursor(state: &EngineState) -> Option<u64> {
    state
        .random
        .lookup_stream(&mtgml_random::RandomStreamKeyV1::global(
            mtgml_random::RandomStreamKindV1::SyntheticM1,
        ))
        .ok()
        .map(|cursor| cursor.next_raw_u64)
}

/// True when at least one perspective's object-mapping targets differ
/// between sides exactly along the declared (non-empty) object bijection.
/// Other players' private identity state may legitimately differ and
/// imposes no equality constraint here; the witness perspective's opaque-key
/// equality is enforced by `check_bijection`.
fn renaming_difference_holds(
    a: &EngineState,
    b: &EngineState,
    bijection: &Option<TrustedRenamingBijection>,
) -> bool {
    let Some(bijection) = bijection else {
        return false;
    };
    if bijection.objects.is_empty() {
        return false;
    }
    let mut renamed_instance = false;
    for (player, identity_a) in &a.perspective_identities.players {
        let Some(identity_b) = b.perspective_identities.players.get(player) else {
            continue;
        };
        for (opaque, target_a) in &identity_a.opaque_to_object {
            if let Some(target_b) = identity_b.opaque_to_object.get(opaque) {
                if target_a != target_b && bijection.objects.get(target_a) == Some(target_b) {
                    renamed_instance = true;
                }
            }
        }
    }
    renamed_instance
}

/// True when at least one perspective's ability-mapping instances differ
/// between sides behind an identical opaque key, explained by the declared
/// (non-empty) ability bijection. Other players' private identity state may
/// legitimately differ and imposes no equality constraint here; the witness
/// perspective's opaque-key equality is enforced by `check_bijection`.
fn ability_renaming_difference_holds(
    a: &EngineState,
    b: &EngineState,
    bijection: &Option<TrustedRenamingBijection>,
) -> bool {
    let Some(bijection) = bijection else {
        return false;
    };
    if bijection.abilities.is_empty() {
        return false;
    }
    let mut renamed_instance = false;
    for (player, identity_a) in &a.perspective_identities.players {
        let Some(identity_b) = b.perspective_identities.players.get(player) else {
            continue;
        };
        for (opaque, instance_a) in &identity_a.opaque_to_ability {
            if let Some(instance_b) = identity_b.opaque_to_ability.get(opaque) {
                if instance_a != instance_b
                    && bijection.abilities.get(instance_a) == Some(instance_b)
                {
                    renamed_instance = true;
                }
            }
        }
    }
    renamed_instance
}

/// At least one non-witness record carries a different history-vector
/// content between sides.
fn foreign_history_differs(a: &EngineState, b: &EngineState, perspective: PlayerId) -> bool {
    let mut differs = false;
    for (player, knowledge_a) in &a.knowledge.players {
        if *player == perspective {
            continue;
        }
        let Some(knowledge_b) = b.knowledge.players.get(player) else {
            return false;
        };
        for (opaque, record_a) in &knowledge_a.active {
            match knowledge_b.active.get(opaque) {
                Some(record_b)
                    if record_a.historical_locations != record_b.historical_locations =>
                {
                    differs = true;
                }
                Some(_) => {}
                None => return false,
            }
        }
    }
    differs && a.knowledge.players.len() == b.knowledge.players.len()
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
            relate_decision(P2, pending_a, pending_b),
            RelationOutcome::Equal
        );

        let witness = PairWitness::new(P2, None, NonVacuityPredicate::None);
        assert_witness(&state, &clone, &witness).unwrap();

        // An identical clone cannot carry a declared difference.
        let vacuous = PairWitness::new(P2, None, NonVacuityPredicate::Required);
        assert_eq!(
            assert_witness(&state, &clone, &vacuous),
            Err(WitnessViolation::VacuousPair)
        );
    }

    #[test]
    fn renaming_bijection_accepts_target_differences() {
        let state_a = base_pair_state(&"11".repeat(32)).unwrap();
        let mut state_b = state_a.clone();
        rename_hidden_object(&mut state_b).unwrap();

        let explained = PairWitness::new(
            P2,
            Some(renaming_bijection()),
            NonVacuityPredicate::Required,
        );
        assert_witness(&state_a, &state_b, &explained).unwrap();

        let unexplained = PairWitness::new(P2, None, NonVacuityPredicate::Required);
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
            relate_decision(P1, Some(&request), Some(&rebound)),
            RelationOutcome::Equal
        );
        // Presence divergence carries its own path.
        assert_eq!(
            relate_decision(P1, Some(&request), None),
            RelationOutcome::Diverges {
                path: "presence".to_owned()
            }
        );
    }

    #[test]
    fn stale_witness_must_fail_on_later_authorized_divergence() {
        let state_a = base_pair_state(&"11".repeat(32)).unwrap();
        let equal_b = state_a.clone();
        // The witness is pure policy: it binds no relation outcomes.
        let witness = PairWitness::new(P1, None, NonVacuityPredicate::None);
        assert_witness(&state_a, &equal_b, &witness).unwrap();

        // After the witness exists, B's P1-authorized knowledge diverges on
        // the exact record path (the definition-known marker of P1's own
        // public record). A cached-relation witness would still accept this
        // pair; recomputation must reject it.
        let mut mutated_b = equal_b;
        mutated_b
            .knowledge
            .players
            .get_mut(&P1)
            .unwrap()
            .active
            .get_mut(&OpaqueObjectId(1))
            .unwrap()
            .card_definition = None;
        assert!(matches!(
            assert_witness(&state_a, &mutated_b, &witness),
            Err(WitnessViolation::KnowledgeDivergence { .. })
        ));
    }

    #[test]
    fn identity_equivalence_is_scoped_to_witness_perspective() {
        let state_a = base_pair_state(&"11".repeat(32)).unwrap();
        let mut state_b = state_a.clone();
        // P2-private allocator history differs: hidden from P1, so the pair
        // stays related even though the whole states are unequal.
        state_b
            .perspective_identities
            .players
            .get_mut(&P2)
            .unwrap()
            .next_player_decision_id
            .0 += 1;
        mtgml_state::validate_engine_state(&state_b).unwrap();
        let witness = PairWitness::new(P1, None, NonVacuityPredicate::Required);
        assert_witness(&state_a, &state_b, &witness).unwrap();

        // Negative control: the same private-history difference on P1's OWN
        // record is an unauthorized divergence the witness must flag.
        let mut state_c = state_a.clone();
        state_c
            .perspective_identities
            .players
            .get_mut(&P1)
            .unwrap()
            .next_player_decision_id
            .0 += 1;
        mtgml_state::validate_engine_state(&state_c).unwrap();
        let own = PairWitness::new(P1, None, NonVacuityPredicate::Required);
        assert!(matches!(
            assert_witness(&state_a, &state_c, &own),
            Err(WitnessViolation::BijectionUnexplained { .. })
        ));
    }

    #[test]
    fn decision_relation_hides_foreign_actor_requests() {
        let state = base_pair_state(&"11".repeat(32)).unwrap();
        let request = state
            .execution
            .pending_decision
            .as_ref()
            .unwrap()
            .request
            .clone();
        let mut foreign = request.clone();
        foreign.actor = P2;
        let mut rebound = request.clone();
        rebound.candidates[0].trusted_binding =
            mtgml_decision::EngineCandidateBinding::PassPriority;
        let mut foreign_rebound = foreign.clone();
        foreign_rebound.candidates[0].trusted_binding =
            mtgml_decision::EngineCandidateBinding::PassPriority;

        // (i) A P-owned request's presence is authorized surface.
        assert_eq!(
            relate_decision(P1, Some(&request), None),
            RelationOutcome::Diverges {
                path: "presence".to_owned()
            }
        );
        // (ii) A foreign-actor request is invisible to P1, existence included.
        assert_eq!(
            relate_decision(P1, Some(&foreign), None),
            RelationOutcome::Equal
        );
        // (iii) Two foreign-actor requests never diverge on trusted detail.
        assert_eq!(
            relate_decision(P1, Some(&foreign), Some(&foreign_rebound)),
            RelationOutcome::Equal
        );
        // (iv) P-owned requests ignore trusted binding changes.
        assert_eq!(
            relate_decision(P1, Some(&request), Some(&rebound)),
            RelationOutcome::Equal
        );
    }

    #[test]
    fn duplicate_bijection_targets_are_not_injective() {
        let state = base_pair_state(&"11".repeat(32)).unwrap();
        let clone = state.clone();
        let collapsing = TrustedRenamingBijection {
            objects: BTreeMap::from([
                (GameObjectId(1), GameObjectId(9)),
                (GameObjectId(2), GameObjectId(9)),
            ]),
            abilities: BTreeMap::new(),
        };
        assert_eq!(
            check_bijection(P2, &state, &clone, &collapsing),
            BijectionOutcome::NotInjective {
                path: "objects".to_owned()
            }
        );
        let witness = PairWitness::new(P2, Some(collapsing), NonVacuityPredicate::ObjectRenaming);
        assert_eq!(
            assert_witness(&state, &clone, &witness),
            Err(WitnessViolation::BijectionNotInjective {
                path: "objects".to_owned()
            })
        );
    }

    #[test]
    fn every_axis_predicate_fails_vacuous_identical_pairs() {
        let axis_predicates = [
            NonVacuityPredicate::OpponentHiddenDefinition,
            NonVacuityPredicate::HiddenConcealedOrdering,
            NonVacuityPredicate::ForeignPrivateLook,
            NonVacuityPredicate::FaceDownIdentity,
            NonVacuityPredicate::RootSeedPreAuth,
            NonVacuityPredicate::HiddenRngCursor,
            NonVacuityPredicate::ObjectRenaming,
            NonVacuityPredicate::AbilityRenaming,
            NonVacuityPredicate::GlobalAllocatorHistory,
            NonVacuityPredicate::ForeignKnowledgeHistory,
        ];
        for predicate in axis_predicates {
            let state = base_pair_state(&"11".repeat(32)).unwrap();
            let clone = state.clone();
            let bijection = match predicate {
                NonVacuityPredicate::ObjectRenaming => {
                    Some(crate::isolation::paired::test_support::renaming_bijection())
                }
                _ => None,
            };
            let witness = PairWitness::new(P2, bijection, predicate);
            assert_eq!(
                assert_witness(&state, &clone, &witness),
                Err(WitnessViolation::VacuousPair),
                "axis predicate {predicate:?} accepted an identical pair"
            );
        }
    }
}
