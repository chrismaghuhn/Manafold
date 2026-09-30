use super::*;
use crate::ordering::validate_candidate_capacity;
use mtgml_model::{
    AbilityInstanceId, CandidateIdV1, DecisionId, GameObjectId, OpaqueAbilityId, OpaqueObjectId,
    PlayerDecisionIdV1, PlayerId, StateRevision,
};
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
}

#[test]
fn answer_validation_precedence_matrix() {
    let domain = DecisionDomainV2::ChooseMany {
        minimum: 2,
        maximum: 2,
    };
    let available = [CandidateIdV1(0), CandidateIdV1(1)];
    let check = |ids: &[u32]| {
        DecisionAnswerV2::validate_for_candidate_ids(
            &DecisionAnswerV2::SelectMany {
                candidate_ids: ids.iter().copied().map(CandidateIdV1).collect(),
            },
            &domain,
            &available,
        )
    };
    // Membership precedes canonical representation.
    assert_eq!(
        check(&[9, 0]),
        Err(DecisionValidationError::UnknownCandidate)
    );
    // Uniqueness precedes canonical representation.
    assert_eq!(
        check(&[0, 0]),
        Err(DecisionValidationError::DuplicateAnswerCandidate)
    );
    // Canonical representation for SelectMany sets.
    assert_eq!(
        check(&[1, 0]),
        Err(DecisionValidationError::NoncanonicalAnswer)
    );
    // Cardinality after representation checks pass.
    assert_eq!(check(&[0]), Err(DecisionValidationError::AnswerCardinality));
    // The exact program answer remains accepted.
    assert_eq!(check(&[0, 1]), Ok(()));
}

#[test]
fn closed_family_domain_boundaries_matrix() {
    // Two candidates: an inclusive maximum above the candidate count is
    // still a valid domain because the candidate set itself bounds the
    // reachable cardinality; only an unsatisfiable minimum is invalid.
    let widened_many = DecisionDomainV2::ChooseMany {
        minimum: 1,
        maximum: 3,
    };
    let widened_order = DecisionDomainV2::Order {
        minimum: 1,
        maximum: 3,
    };
    assert!(widened_many.validate_candidates(2).is_ok());
    assert!(widened_order.validate_candidates(2).is_ok());
    assert!(matches!(
        DecisionDomainV2::ChooseMany {
            minimum: 3,
            maximum: 3
        }
        .validate_candidates(2),
        Err(DecisionValidationError::ImpossibleMinimum)
    ));
    assert!(matches!(
        DecisionDomainV2::Order {
            minimum: 3,
            maximum: 3
        }
        .validate_candidates(2),
        Err(DecisionValidationError::ImpossibleMinimum)
    ));
    // Answer-side boundaries for a widened interval.
    let available = [CandidateIdV1(0), CandidateIdV1(1)];
    let many = |values: &[u32]| {
        DecisionAnswerV2::validate_for_candidate_ids(
            &DecisionAnswerV2::SelectMany {
                candidate_ids: values.iter().copied().map(CandidateIdV1).collect(),
            },
            &widened_many,
            &available,
        )
    };
    assert_eq!(many(&[0]), Ok(()));
    assert_eq!(many(&[0, 1]), Ok(()));
    assert!(many(&[]).is_err());
    assert!(many(&[0, 1, 0]).is_err());
    let order = |values: &[u32]| {
        DecisionAnswerV2::validate_for_candidate_ids(
            &DecisionAnswerV2::Order {
                candidate_ids: values.iter().copied().map(CandidateIdV1).collect(),
            },
            &widened_order,
            &available,
        )
    };
    assert_eq!(order(&[1]), Ok(()));
    assert_eq!(order(&[1, 0]), Ok(()));
    assert!(order(&[]).is_err());
}

#[test]
fn choose_many_zero_to_zero_with_no_candidates_requires_explicit_empty_answer() {
    let domain = DecisionDomainV2::ChooseMany {
        minimum: 0,
        maximum: 0,
    };
    assert!(domain.validate_candidates(0).is_ok());
    assert_eq!(
        DecisionAnswerV2::validate_for_candidate_ids(
            &DecisionAnswerV2::SelectMany {
                candidate_ids: vec![],
            },
            &domain,
            &[],
        ),
        Ok(())
    );
}

#[test]
fn candidate_id_overflow_is_rejected() {
    let response = r#"{
            "schema_version":"decision-response.v3",
            "player_decision_id":"1",
            "view_sequence":"0",
            "answer":{"kind":"select_one","candidate_id":4294967296}
        }"#;
    assert!(serde_json::from_str::<DecisionResponseV3>(response).is_err());
    let in_range = response.replace("4294967296", "4294967295");
    assert!(serde_json::from_str::<DecisionResponseV3>(&in_range).is_ok());
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

#[test]
fn candidate_capacity_uses_the_full_u32_id_domain_without_allocation() {
    let capacity = u64::from(u32::MAX) + 1;
    let Ok(last_count) = usize::try_from(capacity) else {
        return;
    };
    assert_eq!(validate_candidate_capacity(last_count), Ok(()));
    let first_unrepresentable = last_count.checked_add(1).unwrap();
    assert_eq!(
        validate_candidate_capacity(first_unrepresentable),
        Err(DecisionValidationError::CandidateCapacityExceeded)
    );
}

#[test]
fn dense_assignment_and_public_validation_remain_exact_for_small_inputs() {
    let assigned = CandidateOrderingV1::assign_dense(vec![
        (CandidateIntent::Confirm, EngineCandidateBinding::Confirm),
        (
            CandidateIntent::PassPriority,
            EngineCandidateBinding::PassPriority,
        ),
    ])
    .unwrap();
    assert_eq!(
        assigned
            .iter()
            .map(|candidate| candidate.candidate_id)
            .collect::<Vec<_>>(),
        vec![CandidateIdV1(0), CandidateIdV1(1)]
    );
    let visible = assigned
        .iter()
        .map(|candidate| VisibleCandidateV2 {
            candidate_id: candidate.candidate_id,
            intent: candidate.visible_intent.clone(),
        })
        .collect::<Vec<_>>();
    assert!(CandidateOrderingV1::validate_public(&visible).is_ok());
}
