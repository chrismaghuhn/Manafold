// Ownership fragment: synthetic decision-program stage evidence. Included lexically by tests.rs so
// every identity remains tests::<name>.

#[test]
fn synthetic_m2_choose_one_returns_authoritative_transition_product() {
    let state = synthetic_state();
    let result = apply(&state, &select_one_response(0, 0));

    assert!(result.accepted);
    assert_eq!(result.next_state.revision, StateRevision(1));
    assert_eq!(result.next_state.core.players[&PlayerId(1)].life, 38);
    // The entry choice creates the continuation and exposes stage 0.
    let continuation = result
        .next_state
        .execution
        .continuations
        .get(&ContinuationId(1))
        .expect("entry creates the assembly continuation");
    assert_eq!(continuation.actor, PlayerId(1));
    assert_eq!(continuation.created_at_revision, StateRevision(1));
    match &continuation.payload {
        ContinuationPayloadV2::SyntheticM2Assembly {
            stage: AssemblyStageV2::ChooseCount,
            selected_count: None,
            selected_piece_keys,
            ordered_piece_keys,
        } => {
            assert!(selected_piece_keys.is_empty() && ordered_piece_keys.is_empty());
        }
        other => panic!("unexpected stage payload {other:?}"),
    }
    let next = result.next_decision.as_ref().expect("stage decision");
    assert_eq!(next.decision_id, DecisionId(2));
    assert_eq!(next.player_decision_id, PlayerDecisionIdV1(2));
    assert_eq!(
        next.decision,
        DecisionDomainV2::ChooseNumber {
            minimum: 0,
            maximum: 3
        }
    );
    assert_eq!(next.candidates.len(), 0);
    assert_eq!(result.events.len(), 5);
    assert_eq!(result.delta.apply(&state).unwrap(), result.next_state);
    assert_eq!(result.delta.before_digest, state.digest().unwrap());
    assert_eq!(
        result.delta.after_digest,
        result.next_state.digest().unwrap()
    );
    validate_transition_contract(&state, &result).unwrap();
}

#[test]
fn continuation_chain_advances_with_fresh_explicit_identities() {
    let s0 = synthetic_state();
    let r0 = apply(&s0, &select_one_response(0, 0));

    // One ContinuationId; the entry decision is replaced by a fresh one.
    assert_eq!(
        r0.next_state
            .execution
            .continuations
            .keys()
            .copied()
            .collect::<Vec<_>>(),
        vec![ContinuationId(1)]
    );
    let d_stage0 = r0.next_decision.as_ref().unwrap().decision_id;
    assert_ne!(d_stage0, DecisionId(1));

    let r1 = apply(&r0.next_state, &number_response(2, 2, 1));
    let stage1_request = r1.next_decision.as_ref().unwrap();
    assert_ne!(stage1_request.decision_id, d_stage0);
    assert_ne!(stage1_request.player_decision_id, PlayerDecisionIdV1(2));
    assert_eq!(
        stage1_request.decision,
        DecisionDomainV2::ChooseMany {
            minimum: 2,
            maximum: 2
        }
    );
    assert_eq!(stage1_request.candidates.len(), 2);
    // Dense CandidateIdV1 assignment over the generated pieces.
    assert_eq!(stage1_request.candidates[0].candidate_id, CandidateIdV1(0));
    assert_eq!(stage1_request.candidates[1].candidate_id, CandidateIdV1(1));

    let r2 = apply(&r1.next_state, &many_response(3, &[0, 1], 2));
    let stage2_request = r2.next_decision.as_ref().unwrap();
    assert_ne!(stage2_request.decision_id, stage1_request.decision_id);
    assert_ne!(stage2_request.player_decision_id, PlayerDecisionIdV1(3));
    assert_eq!(
        stage2_request.decision,
        DecisionDomainV2::Order {
            minimum: 2,
            maximum: 2
        }
    );

    // The single continuation identity persisted across every stage, with
    // explicit partial values.
    for (state, stage, count) in [
        (&r1.next_state, AssemblyStageV2::ChooseMembers, Some(2_u32)),
        (&r2.next_state, AssemblyStageV2::OrderMembers, Some(2)),
    ] {
        assert_eq!(
            state
                .execution
                .continuations
                .keys()
                .copied()
                .collect::<Vec<_>>(),
            vec![ContinuationId(1)]
        );
        match &state
            .execution
            .continuations
            .get(&ContinuationId(1))
            .unwrap()
            .payload
        {
            ContinuationPayloadV2::SyntheticM2Assembly {
                stage: actual_stage,
                selected_count: actual_count,
                selected_piece_keys,
                ordered_piece_keys,
            } => {
                assert_eq!(actual_stage, &stage);
                assert_eq!(actual_count, &count);
                match stage {
                    AssemblyStageV2::ChooseMembers => {
                        assert!(selected_piece_keys.is_empty() && ordered_piece_keys.is_empty());
                    }
                    AssemblyStageV2::OrderMembers => {
                        assert_eq!(selected_piece_keys, &[0, 1]);
                        assert!(ordered_piece_keys.is_empty());
                    }
                    AssemblyStageV2::ChooseCount => {}
                }
            }
        }
    }

    let r3 = apply(&r2.next_state, &order_response(4, &[1, 0], 3));
    assert!(r3.accepted);
    // Completion removes the continuation and clears the pending decision.
    assert!(r3.next_state.execution.continuations.is_empty());
    assert!(r3.next_state.execution.pending_decision.is_none());
    assert_eq!(r3.delta.apply(&r2.next_state).unwrap(), r3.next_state);
    assert_eq!(r3.delta.before_digest, r2.next_state.digest().unwrap());
    assert_eq!(r3.delta.after_digest, r3.next_state.digest().unwrap());
}

#[test]
fn optional_empty_chain_keeps_every_stage_explicit() {
    let s0 = synthetic_state();
    let r0 = apply(&s0, &select_one_response(0, 0));

    // count = 0: the optional empty case has exactly one canonical answer.
    let r1 = apply(&r0.next_state, &number_response(2, 0, 1));
    let stage1 = r1.next_decision.as_ref().unwrap();
    assert_eq!(
        stage1.decision,
        DecisionDomainV2::ChooseMany {
            minimum: 0,
            maximum: 0
        }
    );
    assert!(stage1.candidates.is_empty());
    // The empty selection is still an explicit accepted player step.
    let r2 = apply(&r1.next_state, &many_response(3, &[], 2));
    assert!(r2.accepted);
    let stage2 = r2.next_decision.as_ref().unwrap();
    assert_eq!(
        stage2.decision,
        DecisionDomainV2::Order {
            minimum: 0,
            maximum: 0
        }
    );
    // The empty order is still an explicit accepted player step.
    let r3 = apply(&r2.next_state, &order_response(4, &[], 3));
    assert!(r3.accepted);
    assert!(r3.next_state.execution.continuations.is_empty());
    assert!(r3.next_state.execution.pending_decision.is_none());
}

#[test]
fn choose_number_stage_bounds_matrix() {
    let stage = entry_stage0();

    // Minimum boundary.
    assert!(apply(&stage, &number_response(2, 0, 1)).accepted);
    // Maximum boundary.
    assert!(apply(&stage, &number_response(2, 3, 1)).accepted);
    // Interior value.
    assert!(apply(&stage, &number_response(2, 1, 1)).accepted);

    // Below the inclusive minimum.
    assert!(!apply(&stage, &number_response(2, -1, 1)).accepted);
    // Above the inclusive maximum.
    assert!(!apply(&stage, &number_response(2, 4, 1)).accepted);

    // Wrong answer variant for the visible domain.
    let wrong_variant = select_one_response(0, 1);
    assert!(!apply(&stage, &wrong_variant).accepted);

    // Stale visible identity and stale revision.
    assert!(!apply(&stage, &number_response(1, 1, 1)).accepted);
    assert!(!apply(&stage, &number_response(2, 1, 5)).accepted);
}

#[test]
fn choose_many_stage_cardinality_matrix() {
    let stage = apply(&entry_stage0(), &number_response(2, 3, 1)).next_state;
    mtgml_state::validate_engine_state(&stage).unwrap();

    // Exact cardinality boundary of the fixed program stage.
    assert!(apply(&stage, &many_response(3, &[0, 1, 2], 2)).accepted);

    // Too few.
    assert!(!apply(&stage, &many_response(3, &[], 2)).accepted);
    // Duplicates violate the canonical set representation and are not
    // silently repaired.
    assert!(!apply(&stage, &many_response(3, &[1, 1], 2)).accepted);
    // Nonascending set syntax is rejected, never sorted.
    assert!(!apply(&stage, &many_response(3, &[2, 1], 2)).accepted);
    // Nonexistent candidate.
    assert!(!apply(&stage, &many_response(3, &[7], 2)).accepted);
    // Wrong answer variant.
    assert!(!apply(&stage, &order_response(3, &[0], 2)).accepted);

    // After every rejection the identical canonical answer still succeeds:
    // no partial execution or repair happened.
    assert!(apply(&stage, &many_response(3, &[0, 1, 2], 2)).accepted);
}

#[test]
fn order_stage_semantics_matrix() {
    let stage = {
        let after_members = apply(
            &apply(&entry_stage0(), &number_response(2, 2, 1)).next_state,
            &many_response(3, &[0, 1], 2),
        )
        .next_state;
        mtgml_state::validate_engine_state(&after_members).unwrap();
        after_members
    };

    // Both semantic orders are legal and accepted; neither is repaired into
    // the other.
    let forward = apply(&stage, &order_response(4, &[0, 1], 3));
    assert!(forward.accepted);
    let reverse_state = apply(&stage, &order_response(4, &[1, 0], 3));
    assert!(reverse_state.accepted);
    // The two orders are semantically distinct answers even though this
    // synthetic completion does not persist the sequence.
    // The two orders are distinct answers; the replay identity binds them.
    assert_ne!(
        forward.delta.before_digest, forward.delta.after_digest,
        "sanity: accepted transitions advance identity"
    );
    assert_eq!(
        forward.next_state.execution.continuations.is_empty(),
        reverse_state.next_state.execution.continuations.is_empty()
    );

    let restored_stage = apply(
        &apply(&entry_stage0(), &number_response(2, 2, 1)).next_state,
        &many_response(3, &[0, 1], 2),
    )
    .next_state;
    // Duplicate candidate IDs are rejected.
    assert!(!apply(&restored_stage, &order_response(4, &[0, 0], 3)).accepted);
    // Nonexistent candidate is rejected.
    assert!(!apply(&restored_stage, &order_response(4, &[0, 9], 3)).accepted);
    // Too few / too many.
    assert!(!apply(&restored_stage, &order_response(4, &[0], 3)).accepted);
    assert!(!apply(&restored_stage, &order_response(4, &[0, 1, 2], 3)).accepted);
    // Wrong answer variant.
    assert!(!apply(&restored_stage, &many_response(4, &[0, 1], 3)).accepted);
}

#[test]
fn unsatisfiable_authoritative_requests_are_internal_failures() {
    // Every unsatisfiable authoritative request fails closed during state
    // validation instead of becoming a player rejection.
    let cases: Vec<DecisionDomainV2> = vec![
        // ChooseMany inverted bounds.
        DecisionDomainV2::ChooseMany {
            minimum: 2,
            maximum: 1,
        },
        // ChooseMany minimum above the candidate count.
        DecisionDomainV2::ChooseMany {
            minimum: 3,
            maximum: 3,
        },
        // Order inverted bounds.
        DecisionDomainV2::Order {
            minimum: 2,
            maximum: 1,
        },
        // ChooseNumber inverted bounds.
        DecisionDomainV2::ChooseNumber {
            minimum: 5,
            maximum: -5,
        },
        // ChooseNumber with candidates.
        DecisionDomainV2::ChooseNumber {
            minimum: 0,
            maximum: 1,
        },
    ];
    for domain in cases {
        let mut state = synthetic_state();
        let pending = state.execution.pending_decision.as_mut().unwrap();
        let candidates = match &domain {
            DecisionDomainV2::ChooseNumber { .. } => pending.request.candidates.clone(),
            _ => pending.request.candidates.clone(),
        };
        pending.request.decision = domain;
        pending.request.candidates = candidates;
        assert!(
            kernel_apply_is_before_state_failure(&state),
            "expected an internal before-state failure"
        );
    }

    // An obligatory ChooseOne without candidates is invalid authoritative
    // state before it can become a player request.
    let mut state = synthetic_state();
    let pending = state.execution.pending_decision.as_mut().unwrap();
    pending.request.candidates.clear();
    assert!(kernel_apply_is_before_state_failure(&state));

    fn kernel_apply_is_before_state_failure(state: &EngineState) -> bool {
        let mut kernel = SyntheticM1RulesKernel;
        matches!(
            kernel.apply(state, PlayerId(1), &select_one_response(0, 0)),
            Err(KernelExecutionError::BeforeState(_))
        )
    }
}

#[test]
fn rejected_family_answers_preserve_the_complete_fingerprint() {
    // The full EngineState equality already covers pending request,
    // continuation, allocators, RNG, knowledge, identities, and revision;
    // the V3 digest binds the same values into persisted identity.
    fn fingerprint(state: &EngineState) -> ([u8; 32], EngineState) {
        (state.digest().unwrap().raw_bytes(), state.clone())
    }

    #[allow(unused_variables)]
    // Mid-chain fingerprint across every rejection family.
    let stage = entry_stage0();
    let before = fingerprint(&stage);
    for response in [
        number_response(2, 9, 1),
        number_response(2, -2, 1),
        number_response(1, 1, 1),
        number_response(2, 1, 9),
        select_one_response(0, 1),
    ] {
        assert!(!apply(&stage, &response).accepted);
        assert_eq!(fingerprint(&stage), before, "mutation on rejection");
    }

    let members = apply(&stage, &number_response(2, 3, 1)).next_state;
    let before = fingerprint(&members);
    for response in [
        many_response(3, &[0], 2),
        many_response(3, &[2, 1, 0], 2),
        order_response(3, &[0], 2),
    ] {
        assert!(!apply(&members, &response).accepted);
        assert_eq!(fingerprint(&members), before, "mutation on rejection");
    }

    let ordering = apply(&members, &many_response(3, &[0, 1, 2], 2)).next_state;
    let before = fingerprint(&ordering);
    for response in [
        order_response(4, &[0], 3),
        order_response(4, &[1, 1], 3),
        order_response(4, &[2, 1], 3),
        number_response(4, 1, 3),
    ] {
        assert!(!apply(&ordering, &response).accepted);
        assert_eq!(fingerprint(&ordering), before, "mutation on rejection");
    }
}

#[test]
fn completion_succeeds_when_stage_allocators_are_exhausted() {
    // Completion consumes no fresh decision or visible identity: both
    // cursors may sit at u64::MAX without blocking the legal final Order.
    let stage = {
        let after_entry = apply(&synthetic_state(), &select_one_response(0, 0)).next_state;
        let after_count = apply(&after_entry, &number_response(2, 2, 1)).next_state;
        let mut state = apply(&after_count, &many_response(3, &[0, 1], 2)).next_state;
        state.allocators.next_decision_id = DecisionId(u64::MAX);
        let identity = state
            .perspective_identities
            .players
            .get_mut(&PlayerId(1))
            .unwrap();
        identity.next_player_decision_id = PlayerDecisionIdV1(u64::MAX);
        mtgml_state::validate_engine_state(&state).unwrap();
        state
    };

    let result = apply(&stage, &order_response(4, &[1, 0], 3));
    assert!(result.accepted);
    assert!(result.next_state.execution.continuations.is_empty());
    assert!(result.next_state.execution.pending_decision.is_none());

    // Stage advancement still fails closed under the same exhaustion.
    let advanced_stage = {
        let after_entry = apply(&synthetic_state(), &select_one_response(0, 0)).next_state;
        let mut state = apply(&after_entry, &number_response(2, 2, 1)).next_state;
        state.allocators.next_decision_id = DecisionId(u64::MAX);
        mtgml_state::validate_engine_state(&state).unwrap();
        state
    };
    let mut kernel = SyntheticM1RulesKernel;
    assert!(matches!(
        kernel.apply(&advanced_stage, PlayerId(1), &many_response(3, &[0, 1], 2)),
        Err(KernelExecutionError::Exhaustion("decision"))
    ));
}
