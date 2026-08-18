//! Perspective-safe public decisions and exact authoritative bindings.

use mtgml_model::{
    AbilityInstanceId, DecisionId, GameObjectId, OpaqueAbilityId, OpaqueObjectId, PlayerId,
    StateRevision,
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
    #[error("decision response contains the same candidate more than once")]
    DuplicateAssignment,
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
}
