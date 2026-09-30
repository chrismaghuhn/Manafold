//! Detached Decision V3 request vocabulary.
//!
//! V3 adds a distinct public PlayLand intent and its trusted object binding.
//! It does not generate legal candidates or execute the special action.

use crate::authoritative::PerspectiveIdentityResolver;
use crate::common::{CandidateIntent, DecisionVisibility};
use crate::error::{CandidateBindingError, DecisionValidationError};
use crate::ordering::CandidateOrderingV2;
use crate::v2::{DecisionAnswerV2, DecisionDomainV2};
use crate::{AuthoritativeDecisionRequestV2, EngineCandidateBinding};
use mtgml_model::{
    AbilityInstanceId, CandidateIdV1, ContinuationId, DecisionId, GameObjectId, OpaqueAbilityId,
    OpaqueObjectId, PlayerDecisionIdV1, PlayerId, StateRevision, VisibleSequence,
};
use serde::{Deserialize, Serialize};

pub const DECISION_RESPONSE_V3_SCHEMA: &str = "decision-response.v3";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionResponseV3 {
    pub schema_version: String,
    pub player_decision_id: PlayerDecisionIdV1,
    pub view_sequence: VisibleSequence,
    pub answer: DecisionAnswerV2,
}

impl DecisionResponseV3 {
    pub fn validate(&self) -> Result<(), DecisionValidationError> {
        if self.schema_version != DECISION_RESPONSE_V3_SCHEMA {
            return Err(DecisionValidationError::SchemaVersion);
        }
        self.answer.validate_shape()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CandidateIntentV3 {
    PassPriority,
    PlayLand { object: OpaqueObjectId },
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
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EngineCandidateBindingV3 {
    PassPriority,
    PlayLand { object: GameObjectId },
    CastSpell { object: GameObjectId },
    ActivateAbility { ability: AbilityInstanceId },
    SelectObject { object: GameObjectId },
    SelectPlayer { player: PlayerId },
    SelectMode { mode_index: u32 },
    ChooseBoolean { value: bool },
    DeclareNumber { value: i64 },
    Confirm,
}

impl EngineCandidateBindingV3 {
    pub fn same_variant_as(&self, visible: &CandidateIntentV3) -> bool {
        matches!(
            (self, visible),
            (Self::PassPriority, CandidateIntentV3::PassPriority)
                | (Self::PlayLand { .. }, CandidateIntentV3::PlayLand { .. })
                | (Self::CastSpell { .. }, CandidateIntentV3::CastSpell { .. })
                | (
                    Self::ActivateAbility { .. },
                    CandidateIntentV3::ActivateAbility { .. }
                )
                | (
                    Self::SelectObject { .. },
                    CandidateIntentV3::SelectObject { .. }
                )
                | (
                    Self::SelectPlayer { .. },
                    CandidateIntentV3::SelectPlayer { .. }
                )
                | (
                    Self::SelectMode { .. },
                    CandidateIntentV3::SelectMode { .. }
                )
                | (
                    Self::ChooseBoolean { .. },
                    CandidateIntentV3::ChooseBoolean { .. }
                )
                | (
                    Self::DeclareNumber { .. },
                    CandidateIntentV3::DeclareNumber { .. }
                )
                | (Self::Confirm, CandidateIntentV3::Confirm)
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VisibleCandidateV3 {
    pub candidate_id: CandidateIdV1,
    pub intent: CandidateIntentV3,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoritativeCandidateV3 {
    pub candidate_id: CandidateIdV1,
    pub visible_intent: CandidateIntentV3,
    pub trusted_binding: EngineCandidateBindingV3,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoritativeDecisionRequestV3 {
    pub decision_id: DecisionId,
    pub player_decision_id: PlayerDecisionIdV1,
    pub state_revision: StateRevision,
    pub actor: PlayerId,
    pub visibility: DecisionVisibility,
    pub decision: DecisionDomainV2,
    pub candidates: Vec<AuthoritativeCandidateV3>,
    pub continuation_id: Option<ContinuationId>,
}

impl From<CandidateIntent> for CandidateIntentV3 {
    fn from(value: CandidateIntent) -> Self {
        match value {
            CandidateIntent::PassPriority => Self::PassPriority,
            CandidateIntent::CastSpell { object } => Self::CastSpell { object },
            CandidateIntent::ActivateAbility { ability } => Self::ActivateAbility { ability },
            CandidateIntent::SelectObject { object } => Self::SelectObject { object },
            CandidateIntent::SelectPlayer { player } => Self::SelectPlayer { player },
            CandidateIntent::SelectMode { mode_index } => Self::SelectMode { mode_index },
            CandidateIntent::ChooseBoolean { value } => Self::ChooseBoolean { value },
            CandidateIntent::DeclareNumber { value } => Self::DeclareNumber { value },
            CandidateIntent::Confirm => Self::Confirm,
        }
    }
}

impl From<EngineCandidateBinding> for EngineCandidateBindingV3 {
    fn from(value: EngineCandidateBinding) -> Self {
        match value {
            EngineCandidateBinding::PassPriority => Self::PassPriority,
            EngineCandidateBinding::CastSpell { object } => Self::CastSpell { object },
            EngineCandidateBinding::ActivateAbility { ability } => {
                Self::ActivateAbility { ability }
            }
            EngineCandidateBinding::SelectObject { object } => Self::SelectObject { object },
            EngineCandidateBinding::SelectPlayer { player } => Self::SelectPlayer { player },
            EngineCandidateBinding::SelectMode { mode_index } => Self::SelectMode { mode_index },
            EngineCandidateBinding::ChooseBoolean { value } => Self::ChooseBoolean { value },
            EngineCandidateBinding::DeclareNumber { value } => Self::DeclareNumber { value },
            EngineCandidateBinding::Confirm => Self::Confirm,
        }
    }
}

impl From<AuthoritativeDecisionRequestV2> for AuthoritativeDecisionRequestV3 {
    fn from(value: AuthoritativeDecisionRequestV2) -> Self {
        Self {
            decision_id: value.decision_id,
            player_decision_id: value.player_decision_id,
            state_revision: value.state_revision,
            actor: value.actor,
            visibility: value.visibility,
            decision: value.decision,
            candidates: value
                .candidates
                .into_iter()
                .map(|candidate| AuthoritativeCandidateV3 {
                    candidate_id: candidate.candidate_id,
                    visible_intent: candidate.visible_intent.into(),
                    trusted_binding: candidate.trusted_binding.into(),
                })
                .collect(),
            continuation_id: value.continuation_id,
        }
    }
}

impl AuthoritativeDecisionRequestV3 {
    pub fn validate(&self) -> Result<(), DecisionValidationError> {
        self.decision.validate_candidates(self.candidates.len())?;
        let candidates = self
            .candidates
            .iter()
            .map(|candidate| VisibleCandidateV3 {
                candidate_id: candidate.candidate_id,
                intent: candidate.visible_intent.clone(),
            })
            .collect::<Vec<_>>();
        CandidateOrderingV2::validate_public(&candidates)?;
        if self.candidates.iter().any(|candidate| {
            !candidate
                .trusted_binding
                .same_variant_as(&candidate.visible_intent)
        }) {
            return Err(DecisionValidationError::BindingVariantMismatch);
        }
        Ok(())
    }

    pub fn validate_bindings(
        &self,
        identities: &impl PerspectiveIdentityResolver,
    ) -> Result<(), CandidateBindingError> {
        for candidate in &self.candidates {
            let visible = VisibleCandidateV3 {
                candidate_id: candidate.candidate_id,
                intent: candidate.visible_intent.clone(),
            };
            validate_candidate_binding_v3(
                &visible,
                &candidate.trusted_binding,
                self.actor,
                identities,
            )?;
        }
        Ok(())
    }
}

pub fn validate_candidate_binding_v3(
    visible: &VisibleCandidateV3,
    authoritative: &EngineCandidateBindingV3,
    perspective: PlayerId,
    identities: &impl PerspectiveIdentityResolver,
) -> Result<(), CandidateBindingError> {
    let valid = match (&visible.intent, authoritative) {
        (CandidateIntentV3::PassPriority, EngineCandidateBindingV3::PassPriority)
        | (CandidateIntentV3::Confirm, EngineCandidateBindingV3::Confirm) => true,
        (
            CandidateIntentV3::PlayLand { object: opaque },
            EngineCandidateBindingV3::PlayLand { object },
        )
        | (
            CandidateIntentV3::CastSpell { object: opaque },
            EngineCandidateBindingV3::CastSpell { object },
        )
        | (
            CandidateIntentV3::SelectObject { object: opaque },
            EngineCandidateBindingV3::SelectObject { object },
        ) => identities.resolve_object(perspective, *opaque) == Some(*object),
        (
            CandidateIntentV3::ActivateAbility { ability: opaque },
            EngineCandidateBindingV3::ActivateAbility { ability },
        ) => identities.resolve_ability(perspective, *opaque) == Some(*ability),
        (
            CandidateIntentV3::SelectPlayer { player: visible },
            EngineCandidateBindingV3::SelectPlayer { player: internal },
        ) => visible == internal,
        (
            CandidateIntentV3::SelectMode {
                mode_index: visible,
            },
            EngineCandidateBindingV3::SelectMode {
                mode_index: internal,
            },
        ) => visible == internal,
        (
            CandidateIntentV3::ChooseBoolean { value: visible },
            EngineCandidateBindingV3::ChooseBoolean { value: internal },
        ) => visible == internal,
        (
            CandidateIntentV3::DeclareNumber { value: visible },
            EngineCandidateBindingV3::DeclareNumber { value: internal },
        ) => visible == internal,
        _ => false,
    };
    valid.then_some(()).ok_or(CandidateBindingError::Mismatch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ordering::CandidateOrderingV2;
    use mtgml_model::{
        CandidateIdV1, PlayerDecisionIdV1, PlayerId, StateRevision, VisibleSequence,
    };
    use std::collections::BTreeMap;

    #[derive(Default)]
    struct Identities {
        objects: BTreeMap<(PlayerId, OpaqueObjectId), GameObjectId>,
        abilities: BTreeMap<(PlayerId, OpaqueAbilityId), AbilityInstanceId>,
    }

    impl PerspectiveIdentityResolver for Identities {
        fn resolve_object(
            &self,
            perspective: PlayerId,
            opaque: OpaqueObjectId,
        ) -> Option<GameObjectId> {
            self.objects.get(&(perspective, opaque)).copied()
        }

        fn resolve_ability(
            &self,
            perspective: PlayerId,
            opaque: OpaqueAbilityId,
        ) -> Option<AbilityInstanceId> {
            self.abilities.get(&(perspective, opaque)).copied()
        }
    }

    fn visible(intent: CandidateIntentV3) -> VisibleCandidateV3 {
        VisibleCandidateV3 {
            candidate_id: CandidateIdV1(0),
            intent,
        }
    }

    #[test]
    fn response_v3_uses_the_shared_decision_answer_and_visible_cursor() {
        let response = DecisionResponseV3 {
            schema_version: DECISION_RESPONSE_V3_SCHEMA.to_owned(),
            player_decision_id: PlayerDecisionIdV1(u64::MAX),
            view_sequence: VisibleSequence(0),
            answer: DecisionAnswerV2::SelectOne {
                candidate_id: CandidateIdV1(2),
            },
        };
        response.validate().unwrap();
        let bytes = serde_json::to_vec(&response).unwrap();
        assert_eq!(
            bytes,
            br#"{"schema_version":"decision-response.v3","player_decision_id":"18446744073709551615","view_sequence":"0","answer":{"kind":"select_one","candidate_id":2}}"#
        );
        let decoded: DecisionResponseV3 = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(decoded, response);
        assert!(serde_json::from_slice::<DecisionResponseV3>(
            br#"{"schema_version":"decision-response.v3","player_decision_id":"1","view_sequence":"2","state_revision":"3","answer":{"kind":"select_one","candidate_id":0}}"#
        )
        .is_err());
    }

    #[test]
    fn response_v3_rejects_noncanonical_select_many() {
        let response = DecisionResponseV3 {
            schema_version: DECISION_RESPONSE_V3_SCHEMA.to_owned(),
            player_decision_id: PlayerDecisionIdV1(1),
            view_sequence: VisibleSequence(2),
            answer: DecisionAnswerV2::SelectMany {
                candidate_ids: vec![CandidateIdV1(2), CandidateIdV1(1)],
            },
        };
        assert_eq!(
            response.validate(),
            Err(DecisionValidationError::NoncanonicalAnswer)
        );
    }

    #[test]
    fn v3_order_inserts_play_land_between_pass_and_cast() {
        let assigned = CandidateOrderingV2::assign_dense(vec![
            (
                CandidateIntentV3::CastSpell {
                    object: OpaqueObjectId(4),
                },
                EngineCandidateBindingV3::CastSpell {
                    object: GameObjectId(40),
                },
            ),
            (
                CandidateIntentV3::PlayLand {
                    object: OpaqueObjectId(9),
                },
                EngineCandidateBindingV3::PlayLand {
                    object: GameObjectId(90),
                },
            ),
            (
                CandidateIntentV3::PassPriority,
                EngineCandidateBindingV3::PassPriority,
            ),
        ])
        .unwrap();
        assert_eq!(
            assigned
                .iter()
                .map(|candidate| candidate.candidate_id.0)
                .collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
        assert!(matches!(
            assigned[0].visible_intent,
            CandidateIntentV3::PassPriority
        ));
        assert!(matches!(
            assigned[1].visible_intent,
            CandidateIntentV3::PlayLand { .. }
        ));
        assert!(matches!(
            assigned[2].visible_intent,
            CandidateIntentV3::CastSpell { .. }
        ));
    }

    #[test]
    fn play_land_order_uses_only_opaque_object_identity() {
        let assigned = CandidateOrderingV2::assign_dense(vec![
            (
                CandidateIntentV3::PlayLand {
                    object: OpaqueObjectId(8),
                },
                EngineCandidateBindingV3::PlayLand {
                    object: GameObjectId(1),
                },
            ),
            (
                CandidateIntentV3::PlayLand {
                    object: OpaqueObjectId(3),
                },
                EngineCandidateBindingV3::PlayLand {
                    object: GameObjectId(999),
                },
            ),
        ])
        .unwrap();
        assert_eq!(assigned[0].candidate_id.0, 0);
        assert_eq!(
            assigned[0].visible_intent,
            CandidateIntentV3::PlayLand {
                object: OpaqueObjectId(3)
            }
        );
    }

    #[test]
    fn v3_preserves_the_complete_candidate_rank_sequence() {
        let candidates = vec![
            CandidateIntentV3::PassPriority,
            CandidateIntentV3::PlayLand {
                object: OpaqueObjectId(1),
            },
            CandidateIntentV3::CastSpell {
                object: OpaqueObjectId(1),
            },
            CandidateIntentV3::ActivateAbility {
                ability: OpaqueAbilityId(1),
            },
            CandidateIntentV3::SelectObject {
                object: OpaqueObjectId(1),
            },
            CandidateIntentV3::SelectPlayer {
                player: PlayerId(1),
            },
            CandidateIntentV3::SelectMode { mode_index: 0 },
            CandidateIntentV3::ChooseBoolean { value: false },
            CandidateIntentV3::DeclareNumber { value: 0 },
            CandidateIntentV3::Confirm,
        ];
        let inputs = candidates
            .into_iter()
            .map(|intent| {
                let binding = match &intent {
                    CandidateIntentV3::PassPriority => EngineCandidateBindingV3::PassPriority,
                    CandidateIntentV3::PlayLand { .. } => EngineCandidateBindingV3::PlayLand {
                        object: GameObjectId(1),
                    },
                    CandidateIntentV3::CastSpell { .. } => EngineCandidateBindingV3::CastSpell {
                        object: GameObjectId(1),
                    },
                    CandidateIntentV3::ActivateAbility { .. } => {
                        EngineCandidateBindingV3::ActivateAbility {
                            ability: AbilityInstanceId(1),
                        }
                    }
                    CandidateIntentV3::SelectObject { .. } => {
                        EngineCandidateBindingV3::SelectObject {
                            object: GameObjectId(1),
                        }
                    }
                    CandidateIntentV3::SelectPlayer { player } => {
                        EngineCandidateBindingV3::SelectPlayer { player: *player }
                    }
                    CandidateIntentV3::SelectMode { mode_index } => {
                        EngineCandidateBindingV3::SelectMode {
                            mode_index: *mode_index,
                        }
                    }
                    CandidateIntentV3::ChooseBoolean { value } => {
                        EngineCandidateBindingV3::ChooseBoolean { value: *value }
                    }
                    CandidateIntentV3::DeclareNumber { value } => {
                        EngineCandidateBindingV3::DeclareNumber { value: *value }
                    }
                    CandidateIntentV3::Confirm => EngineCandidateBindingV3::Confirm,
                };
                (intent, binding)
            })
            .collect();
        let assigned = CandidateOrderingV2::assign_dense(inputs).unwrap();
        assert_eq!(
            assigned
                .iter()
                .map(|candidate| candidate.candidate_id.0)
                .collect::<Vec<_>>(),
            (0..10).collect::<Vec<_>>()
        );
    }

    #[test]
    fn play_land_binding_requires_exact_perspective_object_mapping() {
        let candidate = visible(CandidateIntentV3::PlayLand {
            object: OpaqueObjectId(5),
        });
        let binding = EngineCandidateBindingV3::PlayLand {
            object: GameObjectId(50),
        };
        let identities = Identities {
            objects: BTreeMap::from([((PlayerId(1), OpaqueObjectId(5)), GameObjectId(50))]),
            ..Identities::default()
        };
        assert_eq!(
            validate_candidate_binding_v3(&candidate, &binding, PlayerId(1), &identities),
            Ok(())
        );
        assert_eq!(
            validate_candidate_binding_v3(&candidate, &binding, PlayerId(2), &identities),
            Err(CandidateBindingError::Mismatch)
        );
        assert_eq!(
            validate_candidate_binding_v3(
                &candidate,
                &EngineCandidateBindingV3::PlayLand {
                    object: GameObjectId(51),
                },
                PlayerId(1),
                &identities,
            ),
            Err(CandidateBindingError::Mismatch)
        );

        let request = AuthoritativeDecisionRequestV3 {
            decision_id: DecisionId(1),
            player_decision_id: PlayerDecisionIdV1(2),
            state_revision: StateRevision(3),
            actor: PlayerId(1),
            visibility: DecisionVisibility::ActingPlayerOnly,
            decision: DecisionDomainV2::ChooseOne,
            candidates: vec![AuthoritativeCandidateV3 {
                candidate_id: CandidateIdV1(0),
                visible_intent: candidate.intent,
                trusted_binding: binding,
            }],
            continuation_id: None,
        };
        assert_eq!(request.validate_bindings(&identities), Ok(()));
    }
}
