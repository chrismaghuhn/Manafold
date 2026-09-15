use crate::common::{CandidateIntent, DecisionVisibility};
use crate::error::{CandidateBindingError, DecisionValidationError};
use crate::ordering::CandidateOrderingV1;
use crate::v1::ActionCandidate;
use crate::v2::{
    DecisionDomainV2, PlayerDecisionRequestV2, VisibleCandidateV2,
    PLAYER_DECISION_REQUEST_V2_SCHEMA,
};
use mtgml_model::{
    AbilityInstanceId, CandidateIdV1, ContinuationId, DecisionId, GameObjectId, OpaqueAbilityId,
    OpaqueObjectId, PlayerDecisionIdV1, PlayerId, StateRevision,
};
use serde::{Deserialize, Serialize};

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
