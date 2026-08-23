
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
    let orders: Vec<Vec<usize>> = vec![
        vec![0, 1, 2, 3],
        vec![3, 2, 1, 0],
        vec![1, 3, 0, 2],
    ];
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
