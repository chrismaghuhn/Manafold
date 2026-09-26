//! Detached Decision V3 request vocabulary.
//!
//! V3 adds a distinct public PlayLand intent and its trusted object binding.
//! It does not generate legal candidates or execute the special action.

use crate::authoritative::PerspectiveIdentityResolver;
use crate::common::DecisionVisibility;
use crate::error::{CandidateBindingError, DecisionValidationError};
use crate::ordering::CandidateOrderingV2;
use crate::v2::{
    DecisionAnswerV2, DecisionDomainV2, DecisionResponseV2, DECISION_RESPONSE_V2_SCHEMA,
};
use mtgml_model::{
    AbilityInstanceId, CandidateIdV1, ContinuationId, DecisionId, GameObjectId, OpaqueAbilityId,
    OpaqueObjectId, PlayerDecisionIdV1, PlayerId, StateRevision,
};
use serde::{Deserialize, Serialize};

pub const PLAYER_DECISION_REQUEST_V3_SCHEMA: &str = "player-decision-request.v3";

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
pub struct PlayerDecisionRequestV3 {
    pub schema_version: String,
    pub player_decision_id: PlayerDecisionIdV1,
    pub state_revision: StateRevision,
    pub actor: PlayerId,
    pub visibility: DecisionVisibility,
    pub decision: DecisionDomainV2,
    pub candidates: Vec<VisibleCandidateV3>,
}

impl PlayerDecisionRequestV3 {
    pub fn validate(&self) -> Result<(), DecisionValidationError> {
        if self.schema_version != PLAYER_DECISION_REQUEST_V3_SCHEMA {
            return Err(DecisionValidationError::SchemaVersion);
        }
        self.decision.validate_candidates(self.candidates.len())?;
        CandidateOrderingV2::validate_public(&self.candidates)
    }

    /// V3 changes the request vocabulary only. The unchanged V2 response is
    /// checked against the V3 request's domain and request-local candidate IDs.
    pub fn validate_response(
        &self,
        response: &DecisionResponseV2,
    ) -> Result<(), DecisionValidationError> {
        if response.schema_version != DECISION_RESPONSE_V2_SCHEMA {
            return Err(DecisionValidationError::SchemaVersion);
        }
        if response.player_decision_id != self.player_decision_id {
            return Err(DecisionValidationError::DecisionIdentityMismatch);
        }
        if response.state_revision != self.state_revision {
            return Err(DecisionValidationError::StateRevisionMismatch);
        }
        self.validate()?;
        let ids = self
            .candidates
            .iter()
            .map(|candidate| candidate.candidate_id)
            .collect::<Vec<_>>();
        DecisionAnswerV2::validate_for_candidate_ids(&response.answer, &self.decision, &ids)
    }
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

    pub fn project_player_request(
        &self,
    ) -> Result<PlayerDecisionRequestV3, DecisionValidationError> {
        self.validate()?;
        Ok(PlayerDecisionRequestV3 {
            schema_version: PLAYER_DECISION_REQUEST_V3_SCHEMA.to_owned(),
            player_decision_id: self.player_decision_id,
            state_revision: self.state_revision,
            actor: self.actor,
            visibility: self.visibility,
            decision: self.decision.clone(),
            candidates: self
                .candidates
                .iter()
                .map(|candidate| VisibleCandidateV3 {
                    candidate_id: candidate.candidate_id,
                    intent: candidate.visible_intent.clone(),
                })
                .collect(),
        })
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
    use crate::v2::DecisionResponseV2;
    use mtgml_model::{CandidateIdV1, PlayerDecisionIdV1, PlayerId, StateRevision};
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
    fn v2_select_one_response_selects_v3_play_land_candidate_unchanged() {
        let request = PlayerDecisionRequestV3 {
            schema_version: PLAYER_DECISION_REQUEST_V3_SCHEMA.to_owned(),
            player_decision_id: PlayerDecisionIdV1(7),
            state_revision: StateRevision(12),
            actor: PlayerId(1),
            visibility: DecisionVisibility::ActingPlayerOnly,
            decision: DecisionDomainV2::ChooseOne,
            candidates: vec![visible(CandidateIntentV3::PlayLand {
                object: OpaqueObjectId(5),
            })],
        };
        let response = DecisionResponseV2 {
            schema_version: DECISION_RESPONSE_V2_SCHEMA.to_owned(),
            player_decision_id: PlayerDecisionIdV1(7),
            state_revision: StateRevision(12),
            answer: DecisionAnswerV2::SelectOne {
                candidate_id: CandidateIdV1(0),
            },
        };
        assert_eq!(request.validate_response(&response), Ok(()));
        assert_eq!(response.schema_version, "decision-response.v2");
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

    #[test]
    fn request_rejects_wrong_schema_and_response_identity_or_revision() {
        let mut request = PlayerDecisionRequestV3 {
            schema_version: PLAYER_DECISION_REQUEST_V3_SCHEMA.to_owned(),
            player_decision_id: PlayerDecisionIdV1(7),
            state_revision: StateRevision(12),
            actor: PlayerId(1),
            visibility: DecisionVisibility::ActingPlayerOnly,
            decision: DecisionDomainV2::ChooseOne,
            candidates: vec![visible(CandidateIntentV3::PlayLand {
                object: OpaqueObjectId(5),
            })],
        };
        assert_eq!(request.validate(), Ok(()));
        request.schema_version = "player-decision-request.v2".to_owned();
        assert_eq!(
            request.validate(),
            Err(DecisionValidationError::SchemaVersion)
        );
        request.schema_version = PLAYER_DECISION_REQUEST_V3_SCHEMA.to_owned();
        let mut response = DecisionResponseV2 {
            schema_version: DECISION_RESPONSE_V2_SCHEMA.to_owned(),
            player_decision_id: PlayerDecisionIdV1(8),
            state_revision: StateRevision(11),
            answer: DecisionAnswerV2::SelectOne {
                candidate_id: CandidateIdV1(0),
            },
        };
        assert_eq!(
            request.validate_response(&response),
            Err(DecisionValidationError::DecisionIdentityMismatch)
        );
        response.player_decision_id = request.player_decision_id;
        assert_eq!(
            request.validate_response(&response),
            Err(DecisionValidationError::StateRevisionMismatch)
        );
    }

    #[test]
    fn frozen_phase_two_play_land_wire_fixture_matches_rust_dto() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../schemas/examples/player-decision-request-v3-ordering.json");
        let raw = std::fs::read(path).unwrap();
        let request: PlayerDecisionRequestV3 = serde_json::from_slice(&raw).unwrap();
        request.validate().unwrap();
        let reencoded = serde_json::to_value(&request).unwrap();
        let expected: serde_json::Value = serde_json::from_slice(&raw).unwrap();
        assert_eq!(reencoded, expected);
    }

    #[test]
    fn authoritative_projection_strips_trusted_play_land_binding() {
        let request = AuthoritativeDecisionRequestV3 {
            decision_id: DecisionId(3),
            player_decision_id: PlayerDecisionIdV1(7),
            state_revision: StateRevision(12),
            actor: PlayerId(1),
            visibility: DecisionVisibility::ActingPlayerOnly,
            decision: DecisionDomainV2::ChooseOne,
            candidates: vec![AuthoritativeCandidateV3 {
                candidate_id: CandidateIdV1(0),
                visible_intent: CandidateIntentV3::PlayLand {
                    object: OpaqueObjectId(5),
                },
                trusted_binding: EngineCandidateBindingV3::PlayLand {
                    object: GameObjectId(50),
                },
            }],
            continuation_id: None,
        };
        let projected = request.project_player_request().unwrap();
        let value = serde_json::to_value(projected).unwrap();
        assert_eq!(value["candidates"][0]["intent"]["object"], "5");
        assert!(value["candidates"][0].get("trusted_binding").is_none());
        assert!(serde_json::to_string(&request)
            .unwrap()
            .contains("\"object\":\"50\""));
    }
}
