use super::*;
use crate::ordering::validate_candidate_capacity;
use mtgml_model::CandidateIdV1;

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
