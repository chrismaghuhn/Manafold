//! Perspective-safe public decisions and exact authoritative bindings.

use mtgml_model::{
    AbilityInstanceId, CandidateIdV1, ContinuationId, DecisionId, GameObjectId, OpaqueAbilityId,
    OpaqueObjectId, PlayerDecisionIdV1, PlayerId, StateRevision,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use thiserror::Error;

pub const PLAYER_DECISION_REQUEST_SCHEMA: &str = "player-decision-request.v1";
pub const DECISION_RESPONSE_SCHEMA: &str = "decision-response.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionVisibility {
    Public,
    ActingPlayerOnly,
    Mixed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DecisionKind {
    ChooseOne,
    ChooseMany { minimum: u32, maximum: u32 },
    ChooseNumber { minimum: i64, maximum: i64 },
    Order { minimum: u32, maximum: u32 },
}

impl DecisionKind {
    pub fn selection_bounds(&self) -> Option<(u32, u32)> {
        match self {
            Self::ChooseOne => Some((1, 1)),
            Self::ChooseMany { minimum, maximum } | Self::Order { minimum, maximum } => {
                Some((*minimum, *maximum))
            }
            Self::ChooseNumber { .. } => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CandidateIntent {
    PassPriority,
    CastSpell { object: OpaqueObjectId },
    ActivateAbility { ability: OpaqueAbilityId },
    SelectObject { object: OpaqueObjectId },
    SelectPlayer { player: PlayerId },
    SelectMode { mode_index: u32 },
    ChooseBoolean { value: bool },
    DeclareNumber { value: i64 },
    Confirm,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionCandidate {
    pub candidate_id: String,
    pub semantic_key: String,
    pub intent: CandidateIntent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerDecisionRequest {
    pub schema_version: String,
    pub decision_id: DecisionId,
    pub state_revision: StateRevision,
    pub actor: PlayerId,
    pub visibility: DecisionVisibility,
    pub decision: DecisionKind,
    pub candidates: Vec<ActionCandidate>,
}

impl PlayerDecisionRequest {
    pub fn validate(&self) -> Result<(), DecisionValidationError> {
        if self.schema_version != PLAYER_DECISION_REQUEST_SCHEMA {
            return Err(DecisionValidationError::SchemaVersion);
        }
        let mut candidate_ids = BTreeSet::new();
        let mut semantic_keys = BTreeSet::new();
        for candidate in &self.candidates {
            if candidate.candidate_id.is_empty() || candidate.semantic_key.is_empty() {
                return Err(DecisionValidationError::EmptyCandidateIdentity);
            }
            if !candidate_ids.insert(candidate.candidate_id.as_str()) {
                return Err(DecisionValidationError::DuplicateCandidateId);
            }
            if !semantic_keys.insert(candidate.semantic_key.as_str()) {
                return Err(DecisionValidationError::DuplicateSemanticKey);
            }
        }
        match self.decision {
            DecisionKind::ChooseMany { minimum, maximum }
            | DecisionKind::Order { minimum, maximum } => {
                if minimum > maximum {
                    return Err(DecisionValidationError::InvertedBounds);
                }
                if usize::try_from(minimum).unwrap_or(usize::MAX) > self.candidates.len() {
                    return Err(DecisionValidationError::ImpossibleMinimum);
                }
            }
            DecisionKind::ChooseNumber { minimum, maximum } if minimum > maximum => {
                return Err(DecisionValidationError::InvertedBounds)
            }
            DecisionKind::ChooseOne if self.candidates.is_empty() => {
                return Err(DecisionValidationError::ImpossibleMinimum)
            }
            _ => {}
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateAssignment {
    pub candidate_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ordinal: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionResponse {
    pub schema_version: String,
    pub decision_id: DecisionId,
    pub state_revision: StateRevision,
    pub assignments: Vec<CandidateAssignment>,
}

pub const PLAYER_DECISION_REQUEST_V2_SCHEMA: &str = "player-decision-request.v2";
pub const DECISION_RESPONSE_V2_SCHEMA: &str = "decision-response.v2";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DecisionDomainV2 {
    ChooseOne,
    ChooseMany { minimum: u32, maximum: u32 },
    ChooseNumber { minimum: i64, maximum: i64 },
    Order { minimum: u32, maximum: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DecisionAnswerV2 {
    SelectOne { candidate_id: CandidateIdV1 },
    SelectMany { candidate_ids: Vec<CandidateIdV1> },
    ChooseNumber { value: i64 },
    Order { candidate_ids: Vec<CandidateIdV1> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VisibleCandidateV2 {
    pub candidate_id: CandidateIdV1,
    pub intent: CandidateIntent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerDecisionRequestV2 {
    pub schema_version: String,
    pub player_decision_id: PlayerDecisionIdV1,
    pub state_revision: StateRevision,
    pub actor: PlayerId,
    pub visibility: DecisionVisibility,
    pub decision: DecisionDomainV2,
    pub candidates: Vec<VisibleCandidateV2>,
}

impl DecisionDomainV2 {
    fn validate_candidates(&self, candidate_count: usize) -> Result<(), DecisionValidationError> {
        match self {
            Self::ChooseOne if candidate_count == 0 => {
                Err(DecisionValidationError::ImpossibleMinimum)
            }
            Self::ChooseMany { minimum, maximum } | Self::Order { minimum, maximum } => {
                if minimum > maximum {
                    return Err(DecisionValidationError::InvertedBounds);
                }
                if usize::try_from(*minimum).unwrap_or(usize::MAX) > candidate_count
                    || usize::try_from(*maximum).unwrap_or(usize::MAX) > candidate_count
                {
                    return Err(DecisionValidationError::ImpossibleMaximum);
                }
                Ok(())
            }
            Self::ChooseNumber { minimum, maximum } => {
                if minimum > maximum {
                    return Err(DecisionValidationError::InvertedBounds);
                }
                if candidate_count != 0 {
                    return Err(DecisionValidationError::CandidatesNotAllowed);
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

impl PlayerDecisionRequestV2 {
    pub fn validate(&self) -> Result<(), DecisionValidationError> {
        if self.schema_version != PLAYER_DECISION_REQUEST_V2_SCHEMA {
            return Err(DecisionValidationError::SchemaVersion);
        }
        self.decision.validate_candidates(self.candidates.len())?;
        CandidateOrderingV1::validate_public(&self.candidates)
    }

    pub fn answer(&self, answer: &DecisionAnswerV2) -> Result<(), DecisionValidationError> {
        self.validate()?;
        answer.validate_for(&self.decision, &self.candidates)
    }
}

impl DecisionAnswerV2 {
    pub fn validate_for(
        &self,
        domain: &DecisionDomainV2,
        candidates: &[VisibleCandidateV2],
    ) -> Result<(), DecisionValidationError> {
        let ids: BTreeSet<_> = candidates
            .iter()
            .map(|candidate| candidate.candidate_id)
            .collect();
        match (domain, self) {
            (DecisionDomainV2::ChooseOne, Self::SelectOne { candidate_id }) => {
                if ids.contains(candidate_id) {
                    Ok(())
                } else {
                    Err(DecisionValidationError::UnknownCandidate)
                }
            }
            (
                DecisionDomainV2::ChooseMany { minimum, maximum },
                Self::SelectMany { candidate_ids },
            ) => {
                validate_selection_ids(candidate_ids, &ids, true)?;
                validate_cardinality(candidate_ids.len(), *minimum, *maximum)
            }
            (DecisionDomainV2::ChooseNumber { minimum, maximum }, Self::ChooseNumber { value })
                if value >= minimum && value <= maximum =>
            {
                Ok(())
            }
            (DecisionDomainV2::Order { minimum, maximum }, Self::Order { candidate_ids }) => {
                validate_selection_ids(candidate_ids, &ids, false)?;
                validate_cardinality(candidate_ids.len(), *minimum, *maximum)
            }
            (DecisionDomainV2::ChooseNumber { .. }, Self::ChooseNumber { .. }) => {
                Err(DecisionValidationError::NumericOutOfBounds)
            }
            _ => Err(DecisionValidationError::AnswerDomainMismatch),
        }
    }
}

fn validate_selection_ids(
    candidate_ids: &[CandidateIdV1],
    available: &BTreeSet<CandidateIdV1>,
    require_ascending: bool,
) -> Result<(), DecisionValidationError> {
    let mut seen = BTreeSet::new();
    for window in candidate_ids.windows(2) {
        if require_ascending && window[0] >= window[1] {
            return Err(DecisionValidationError::NoncanonicalAnswer);
        }
    }
    for candidate_id in candidate_ids {
        if !available.contains(candidate_id) {
            return Err(DecisionValidationError::UnknownCandidate);
        }
        if !seen.insert(*candidate_id) {
            return Err(DecisionValidationError::DuplicateAnswerCandidate);
        }
    }
    Ok(())
}

fn validate_cardinality(
    actual: usize,
    minimum: u32,
    maximum: u32,
) -> Result<(), DecisionValidationError> {
    let actual = u32::try_from(actual).map_err(|_| DecisionValidationError::ValueOutOfRange)?;
    if actual < minimum || actual > maximum {
        return Err(DecisionValidationError::AnswerCardinality);
    }
    Ok(())
}

impl DecisionResponse {
    pub fn validate(&self) -> Result<(), DecisionValidationError> {
        if self.schema_version != DECISION_RESPONSE_SCHEMA {
            return Err(DecisionValidationError::SchemaVersion);
        }
        let mut ids = BTreeSet::new();
        if self
            .assignments
            .iter()
            .any(|assignment| assignment.candidate_id.is_empty())
        {
            return Err(DecisionValidationError::EmptyCandidateIdentity);
        }
        if self
            .assignments
            .iter()
            .any(|assignment| !ids.insert(assignment.candidate_id.as_str()))
        {
            return Err(DecisionValidationError::DuplicateAssignment);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EngineCandidateBinding {
    PassPriority,
    CastSpell { object: GameObjectId },
    ActivateAbility { ability: AbilityInstanceId },
    SelectObject { object: GameObjectId },
    SelectPlayer { player: PlayerId },
    SelectMode { mode_index: u32 },
    ChooseBoolean { value: bool },
    DeclareNumber { value: i64 },
    Confirm,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoritativeCandidateV2 {
    pub candidate_id: CandidateIdV1,
    pub visible_intent: CandidateIntent,
    pub trusted_binding: EngineCandidateBinding,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoritativeDecisionRequestV2 {
    pub decision_id: DecisionId,
    pub player_decision_id: PlayerDecisionIdV1,
    pub state_revision: StateRevision,
    pub actor: PlayerId,
    pub visibility: DecisionVisibility,
    pub decision: DecisionDomainV2,
    pub candidates: Vec<AuthoritativeCandidateV2>,
    pub continuation_id: Option<ContinuationId>,
}

impl AuthoritativeDecisionRequestV2 {
    pub fn validate(&self) -> Result<(), DecisionValidationError> {
        self.decision.validate_candidates(self.candidates.len())?;
        let visible = self
            .candidates
            .iter()
            .map(|candidate| VisibleCandidateV2 {
                candidate_id: candidate.candidate_id,
                intent: candidate.visible_intent.clone(),
            })
            .collect::<Vec<_>>();
        CandidateOrderingV1::validate_public(&visible)?;
        if self.candidates.iter().any(|candidate| {
            !candidate
                .trusted_binding
                .same_variant_as(&candidate.visible_intent)
        }) {
            return Err(DecisionValidationError::BindingVariantMismatch);
        }
        Ok(())
    }

    pub fn project_player_request(
        &self,
    ) -> Result<PlayerDecisionRequestV2, DecisionValidationError> {
        self.validate()?;
        Ok(PlayerDecisionRequestV2 {
            schema_version: PLAYER_DECISION_REQUEST_V2_SCHEMA.to_owned(),
            player_decision_id: self.player_decision_id,
            state_revision: self.state_revision,
            actor: self.actor,
            visibility: self.visibility,
            decision: self.decision.clone(),
            candidates: self
                .candidates
                .iter()
                .map(|candidate| VisibleCandidateV2 {
                    candidate_id: candidate.candidate_id,
                    intent: candidate.visible_intent.clone(),
                })
                .collect(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionResponseV2 {
    pub schema_version: String,
    pub player_decision_id: PlayerDecisionIdV1,
    pub state_revision: StateRevision,
    pub answer: DecisionAnswerV2,
}

impl DecisionResponseV2 {
    pub fn validate(&self) -> Result<(), DecisionValidationError> {
        if self.schema_version != DECISION_RESPONSE_V2_SCHEMA {
            return Err(DecisionValidationError::SchemaVersion);
        }
        match &self.answer {
            DecisionAnswerV2::SelectMany { candidate_ids } => {
                if candidate_ids
                    .windows(2)
                    .any(|window| window[0] >= window[1])
                {
                    return Err(DecisionValidationError::NoncanonicalAnswer);
                }
            }
            DecisionAnswerV2::Order { candidate_ids } => {
                let mut seen = BTreeSet::new();
                if candidate_ids
                    .iter()
                    .any(|candidate_id| !seen.insert(*candidate_id))
                {
                    return Err(DecisionValidationError::DuplicateAnswerCandidate);
                }
            }
            DecisionAnswerV2::SelectOne { .. } | DecisionAnswerV2::ChooseNumber { .. } => {}
        }
        Ok(())
    }

    pub fn validate_for(
        &self,
        request: &PlayerDecisionRequestV2,
    ) -> Result<(), DecisionValidationError> {
        self.validate()?;
        if self.player_decision_id != request.player_decision_id {
            return Err(DecisionValidationError::DecisionIdentityMismatch);
        }
        if self.state_revision != request.state_revision {
            return Err(DecisionValidationError::StateRevisionMismatch);
        }
        request.answer(&self.answer)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum CandidatePayloadKey {
    None,
    U64(u64),
    U32(u32),
    Bool(bool),
    I64(i64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct CandidateOrderingKey {
    rank: u8,
    payload: CandidatePayloadKey,
}

pub struct CandidateOrderingV1;

impl CandidateOrderingV1 {
    pub fn assign_dense(
        candidates: Vec<(CandidateIntent, EngineCandidateBinding)>,
    ) -> Result<Vec<AuthoritativeCandidateV2>, DecisionValidationError> {
        let mut keyed = candidates
            .into_iter()
            .map(|(visible_intent, trusted_binding)| {
                let key = ordering_key(&visible_intent)?;
                Ok((key, visible_intent, trusted_binding))
            })
            .collect::<Result<Vec<_>, DecisionValidationError>>()?;
        keyed.sort_by_key(|(key, _, _)| *key);
        if keyed.windows(2).any(|window| window[0].0 == window[1].0) {
            return Err(DecisionValidationError::DuplicateOrderingKey);
        }
        Ok(keyed
            .into_iter()
            .enumerate()
            .map(
                |(index, (_, visible_intent, trusted_binding))| AuthoritativeCandidateV2 {
                    candidate_id: CandidateIdV1(
                        u32::try_from(index).expect("candidate ordering is bounded by u32"),
                    ),
                    visible_intent,
                    trusted_binding,
                },
            )
            .collect())
    }

    pub fn validate_public(
        candidates: &[VisibleCandidateV2],
    ) -> Result<(), DecisionValidationError> {
        for (index, candidate) in candidates.iter().enumerate() {
            if candidate.candidate_id.0 != index as u32 {
                return Err(DecisionValidationError::CandidateIdsNotDense);
            }
            if let Some(previous) = candidates.get(index.wrapping_sub(1)) {
                if ordering_key(&previous.intent)? >= ordering_key(&candidate.intent)? {
                    return Err(DecisionValidationError::NoncanonicalCandidateOrder);
                }
            }
        }
        Ok(())
    }
}

fn ordering_key(intent: &CandidateIntent) -> Result<CandidateOrderingKey, DecisionValidationError> {
    let (rank, payload) = match intent {
        CandidateIntent::PassPriority => (0, CandidatePayloadKey::None),
        CandidateIntent::CastSpell { object } => (1, CandidatePayloadKey::U64(object.0)),
        CandidateIntent::ActivateAbility { ability } => (2, CandidatePayloadKey::U64(ability.0)),
        CandidateIntent::SelectObject { object } => (3, CandidatePayloadKey::U64(object.0)),
        CandidateIntent::SelectPlayer { player } => (4, CandidatePayloadKey::U64(player.0)),
        CandidateIntent::SelectMode { mode_index } => (5, CandidatePayloadKey::U32(*mode_index)),
        CandidateIntent::ChooseBoolean { value } => (6, CandidatePayloadKey::Bool(*value)),
        CandidateIntent::DeclareNumber { value } => (7, CandidatePayloadKey::I64(*value)),
        CandidateIntent::Confirm => (8, CandidatePayloadKey::None),
    };
    Ok(CandidateOrderingKey { rank, payload })
}

impl EngineCandidateBinding {
    /// Diagnostic helper only. Soundness validation must call
    /// [`validate_candidate_binding`], which also compares values and perspective mappings.
    pub fn same_variant_as(&self, visible: &CandidateIntent) -> bool {
        matches!(
            (self, visible),
            (Self::PassPriority, CandidateIntent::PassPriority)
                | (Self::CastSpell { .. }, CandidateIntent::CastSpell { .. })
                | (
                    Self::ActivateAbility { .. },
                    CandidateIntent::ActivateAbility { .. }
                )
                | (
                    Self::SelectObject { .. },
                    CandidateIntent::SelectObject { .. }
                )
                | (
                    Self::SelectPlayer { .. },
                    CandidateIntent::SelectPlayer { .. }
                )
                | (Self::SelectMode { .. }, CandidateIntent::SelectMode { .. })
                | (
                    Self::ChooseBoolean { .. },
                    CandidateIntent::ChooseBoolean { .. }
                )
                | (
                    Self::DeclareNumber { .. },
                    CandidateIntent::DeclareNumber { .. }
                )
                | (Self::Confirm, CandidateIntent::Confirm)
        )
    }
}

pub trait PerspectiveIdentityResolver {
    fn resolve_object(&self, perspective: PlayerId, opaque: OpaqueObjectId)
        -> Option<GameObjectId>;
    fn resolve_ability(
        &self,
        perspective: PlayerId,
        opaque: OpaqueAbilityId,
    ) -> Option<AbilityInstanceId>;
}

pub fn validate_candidate_binding(
    visible: &ActionCandidate,
    authoritative: &EngineCandidateBinding,
    perspective: PlayerId,
    identities: &impl PerspectiveIdentityResolver,
) -> Result<(), CandidateBindingError> {
    let valid = match (&visible.intent, authoritative) {
        (CandidateIntent::PassPriority, EngineCandidateBinding::PassPriority)
        | (CandidateIntent::Confirm, EngineCandidateBinding::Confirm) => true,
        (
            CandidateIntent::CastSpell { object: opaque },
            EngineCandidateBinding::CastSpell { object },
        )
        | (
            CandidateIntent::SelectObject { object: opaque },
            EngineCandidateBinding::SelectObject { object },
        ) => identities.resolve_object(perspective, *opaque) == Some(*object),
        (
            CandidateIntent::ActivateAbility { ability: opaque },
            EngineCandidateBinding::ActivateAbility { ability },
        ) => identities.resolve_ability(perspective, *opaque) == Some(*ability),
        (
            CandidateIntent::SelectPlayer { player: visible },
            EngineCandidateBinding::SelectPlayer { player: internal },
        ) => visible == internal,
        (
            CandidateIntent::SelectMode {
                mode_index: visible,
            },
            EngineCandidateBinding::SelectMode {
                mode_index: internal,
            },
        ) => visible == internal,
        (
            CandidateIntent::ChooseBoolean { value: visible },
            EngineCandidateBinding::ChooseBoolean { value: internal },
        ) => visible == internal,
        (
            CandidateIntent::DeclareNumber { value: visible },
            EngineCandidateBinding::DeclareNumber { value: internal },
        ) => visible == internal,
        _ => false,
    };
    valid.then_some(()).ok_or(CandidateBindingError::Mismatch)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum DecisionValidationError {
    #[error("unsupported schema version")]
    SchemaVersion,
    #[error("candidate identity fields must be non-empty")]
    EmptyCandidateIdentity,
    #[error("candidate IDs must be unique")]
    DuplicateCandidateId,
    #[error("semantic keys must be unique within a decision")]
    DuplicateSemanticKey,
    #[error("decision bounds are inverted")]
    InvertedBounds,
    #[error("minimum selection cannot be satisfied by the candidate set")]
    ImpossibleMinimum,
    #[error("maximum selection cannot be satisfied by the candidate set")]
    ImpossibleMaximum,
    #[error("this decision domain does not accept candidates")]
    CandidatesNotAllowed,
    #[error("decision response contains the same candidate more than once")]
    DuplicateAssignment,
    #[error("answer variant does not match the decision domain")]
    AnswerDomainMismatch,
    #[error("answer contains an unknown candidate")]
    UnknownCandidate,
    #[error("answer contains the same candidate more than once")]
    DuplicateAnswerCandidate,
    #[error("answer is not in its canonical representation")]
    NoncanonicalAnswer,
    #[error("answer cardinality is outside the decision bounds")]
    AnswerCardinality,
    #[error("numeric answer is outside the decision bounds")]
    NumericOutOfBounds,
    #[error("candidate IDs must be dense from zero")]
    CandidateIdsNotDense,
    #[error("candidate ordering is not canonical")]
    NoncanonicalCandidateOrder,
    #[error("candidate ordering contains a duplicate public key")]
    DuplicateOrderingKey,
    #[error("visible candidate and trusted binding variants differ")]
    BindingVariantMismatch,
    #[error("player decision identity does not match the request")]
    DecisionIdentityMismatch,
    #[error("state revision does not match the request")]
    StateRevisionMismatch,
    #[error("candidate value is outside the supported range")]
    ValueOutOfRange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum CandidateBindingError {
    #[error("visible candidate does not exactly match its authoritative binding")]
    Mismatch,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[derive(Default)]
    struct Ids {
        objects: BTreeMap<(PlayerId, OpaqueObjectId), GameObjectId>,
        abilities: BTreeMap<(PlayerId, OpaqueAbilityId), AbilityInstanceId>,
    }
    impl PerspectiveIdentityResolver for Ids {
        fn resolve_object(&self, p: PlayerId, o: OpaqueObjectId) -> Option<GameObjectId> {
            self.objects.get(&(p, o)).copied()
        }
        fn resolve_ability(&self, p: PlayerId, a: OpaqueAbilityId) -> Option<AbilityInstanceId> {
            self.abilities.get(&(p, a)).copied()
        }
    }

    fn candidate(intent: CandidateIntent) -> ActionCandidate {
        ActionCandidate {
            candidate_id: "c1".into(),
            semantic_key: "k1".into(),
            intent,
        }
    }

    #[test]
    fn mode_binding_compares_the_actual_index() {
        let ids = Ids::default();
        assert!(validate_candidate_binding(
            &candidate(CandidateIntent::SelectMode { mode_index: 0 }),
            &EngineCandidateBinding::SelectMode { mode_index: 1 },
            PlayerId(1),
            &ids,
        )
        .is_err());
    }

    #[test]
    fn boolean_binding_compares_the_actual_value() {
        let ids = Ids::default();
        assert!(validate_candidate_binding(
            &candidate(CandidateIntent::ChooseBoolean { value: true }),
            &EngineCandidateBinding::ChooseBoolean { value: false },
            PlayerId(1),
            &ids,
        )
        .is_err());
    }

    #[test]
    fn object_binding_uses_the_perspective_map() {
        let mut ids = Ids::default();
        ids.objects
            .insert((PlayerId(1), OpaqueObjectId(7)), GameObjectId(9));
        assert!(validate_candidate_binding(
            &candidate(CandidateIntent::SelectObject {
                object: OpaqueObjectId(7)
            }),
            &EngineCandidateBinding::SelectObject {
                object: GameObjectId(9)
            },
            PlayerId(1),
            &ids,
        )
        .is_ok());
    }

    #[test]
    fn candidate_ordering_v1_exact_matrix() {
        let candidates = vec![
            (
                CandidateIntent::CastSpell {
                    object: OpaqueObjectId(10),
                },
                EngineCandidateBinding::CastSpell {
                    object: GameObjectId(10),
                },
            ),
            (
                CandidateIntent::CastSpell {
                    object: OpaqueObjectId(2),
                },
                EngineCandidateBinding::CastSpell {
                    object: GameObjectId(2),
                },
            ),
        ];
        let ordered = CandidateOrderingV1::assign_dense(candidates).unwrap();
        assert_eq!(ordered[0].candidate_id, CandidateIdV1(0));
        assert_eq!(ordered[1].candidate_id, CandidateIdV1(1));
        assert!(matches!(
            ordered[0].visible_intent,
            CandidateIntent::CastSpell {
                object: OpaqueObjectId(2)
            }
        ));
        assert!(CandidateOrderingV1::assign_dense(vec![
            (CandidateIntent::Confirm, EngineCandidateBinding::Confirm,),
            (CandidateIntent::Confirm, EngineCandidateBinding::Confirm,),
        ])
        .is_err());

        let choose_number = AuthoritativeDecisionRequestV2 {
            decision_id: DecisionId(1),
            player_decision_id: PlayerDecisionIdV1(1),
            state_revision: StateRevision(0),
            actor: PlayerId(1),
            visibility: DecisionVisibility::Public,
            decision: DecisionDomainV2::ChooseNumber {
                minimum: 0,
                maximum: 10,
            },
            candidates: Vec::new(),
            continuation_id: None,
        };
        assert!(choose_number.validate().is_ok());
        let mut invalid_number = choose_number.clone();
        invalid_number.candidates = ordered;
        assert!(invalid_number.validate().is_err());

        let request = PlayerDecisionRequestV2 {
            schema_version: PLAYER_DECISION_REQUEST_V2_SCHEMA.to_owned(),
            player_decision_id: PlayerDecisionIdV1(1),
            state_revision: StateRevision(0),
            actor: PlayerId(1),
            visibility: DecisionVisibility::Public,
            decision: DecisionDomainV2::ChooseOne,
            candidates: vec![
                VisibleCandidateV2 {
                    candidate_id: CandidateIdV1(0),
                    intent: CandidateIntent::ChooseBoolean { value: false },
                },
                VisibleCandidateV2 {
                    candidate_id: CandidateIdV1(1),
                    intent: CandidateIntent::ChooseBoolean { value: true },
                },
            ],
        };
        assert!(request.validate().is_ok());
        let mut noncanonical = request.clone();
        noncanonical.candidates.swap(0, 1);
        assert!(noncanonical.validate().is_err());
    }

    #[test]
    fn candidate_id_overflow_is_rejected() {
        let response = r#"{
            "schema_version":"decision-response.v2",
            "player_decision_id":"1",
            "state_revision":"0",
            "answer":{"kind":"select_one","candidate_id":4294967296}
        }"#;
        assert!(serde_json::from_str::<DecisionResponseV2>(response).is_err());
    }

    #[test]
    fn candidate_generation_is_insertion_and_trusted_id_independent() {
        // Equivalent semantic candidates in every insertion order must produce
        // identical visible ordering and dense IDs, regardless of the trusted
        // bindings that ride along.
        let semantic_intents = [
            CandidateIntent::SelectMode { mode_index: 5 },
            CandidateIntent::PassPriority,
            CandidateIntent::ChooseBoolean { value: true },
            CandidateIntent::SelectObject {
                object: OpaqueObjectId(9),
            },
        ];
        let orders: Vec<Vec<usize>> = vec![vec![0, 1, 2, 3], vec![3, 2, 1, 0], vec![1, 3, 0, 2]];
        let mut reference: Option<Vec<(CandidateIdV1, CandidateIntent)>> = None;
        for order in &orders {
            for unrelated_binding in [
                EngineCandidateBinding::Confirm,
                EngineCandidateBinding::CastSpell {
                    object: GameObjectId(77),
                },
            ] {
                let pairs: Vec<(CandidateIntent, EngineCandidateBinding)> = order
                    .iter()
                    .map(|index| match &semantic_intents[*index] {
                        CandidateIntent::PassPriority => {
                            (semantic_intents[*index].clone(), unrelated_binding.clone())
                        }
                        other => (
                            other.clone(),
                            EngineCandidateBinding::SelectMode { mode_index: 0 },
                        ),
                    })
                    .collect();
                let assigned = CandidateOrderingV1::assign_dense(pairs).unwrap();
                let visible: Vec<(CandidateIdV1, CandidateIntent)> = assigned
                    .iter()
                    .map(|candidate| (candidate.candidate_id, candidate.visible_intent.clone()))
                    .collect();
                match &reference {
                    None => {
                        reference = Some(visible.clone());
                        assert_eq!(visible[0].0, CandidateIdV1(0));
                        assert_eq!(visible[3].0, CandidateIdV1(3));
                    }
                    Some(expected) => assert_eq!(&visible, expected),
                }
            }
        }
    }

    #[test]
    fn duplicate_public_keys_fail_closed_even_with_distinct_trusted_bindings() {
        // Two distinct trusted entities intentionally map to one public key.
        let pairs = vec![
            (
                CandidateIntent::SelectObject {
                    object: OpaqueObjectId(4),
                },
                EngineCandidateBinding::SelectObject {
                    object: GameObjectId(100),
                },
            ),
            (
                CandidateIntent::SelectObject {
                    object: OpaqueObjectId(4),
                },
                EngineCandidateBinding::SelectObject {
                    object: GameObjectId(200),
                },
            ),
        ];
        assert!(matches!(
            CandidateOrderingV1::assign_dense(pairs),
            Err(DecisionValidationError::DuplicateOrderingKey)
        ));
    }
}
