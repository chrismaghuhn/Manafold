// Ownership fragment: synthetic assembly continuation program evidence. Included lexically by tests.rs so
// every identity remains tests::<name>.

#[test]
fn continuation_checkpoint_restore_roundtrip_preserves_the_chain() {
    let source = environment_at_members_stage();
    let mid_checkpoint = source.checkpoint().unwrap();

    // Complete the chain on the source to capture the reference product.
    let source_p1 = source.bind_player(PlayerId(1)).unwrap();
    let reference_step = submit_answer(&source_p1, members_answer(&[0, 1]));
    let reference_after = source.checkpoint().unwrap();

    // Restore the mid-chain checkpoint into an equivalent environment and
    // submit the same next answer.
    let restored = TrustedEnvironmentController::new(backend());
    restored.restore(mid_checkpoint.clone()).unwrap();
    let restored_p1 = restored.bind_player(PlayerId(1)).unwrap();
    let replayed_step = submit_answer(&restored_p1, members_answer(&[0, 1]));

    assert_eq!(reference_step, replayed_step);
    assert_eq!(restored.checkpoint().unwrap(), reference_after);

    // The checkpoint itself contains the live continuation state.
    let state = &mid_checkpoint.state;
    assert_eq!(
        state
            .execution
            .continuations
            .keys()
            .copied()
            .collect::<Vec<_>>(),
        vec![ContinuationId(1)]
    );
}

#[test]
fn continuation_fork_equal_input_produces_equal_results() {
    let source = environment_at_members_stage();
    let fork_a = source.fork().unwrap();
    let fork_b = source.fork().unwrap();
    let a = fork_a.bind_player(PlayerId(1)).unwrap();
    let b = fork_b.bind_player(PlayerId(1)).unwrap();

    let step_a = submit_answer(&a, members_answer(&[0, 1]));
    let step_b = submit_answer(&b, members_answer(&[0, 1]));
    assert_eq!(step_a, step_b);
    assert_eq!(fork_a.checkpoint().unwrap(), fork_b.checkpoint().unwrap());

    // Divergence happens only through different valid inputs afterwards.
    let a2 = fork_a.bind_player(PlayerId(1)).unwrap();
    assert!(a2.visible_decision().unwrap().is_some());
}

#[test]
fn continuation_replay_full_chain_parity() {
    let controller = TrustedEnvironmentController::new(backend());
    let p1 = controller.bind_player(PlayerId(1)).unwrap();
    let c0 = controller.checkpoint().unwrap();

    let _ = submit_answer(&p1, order_entry_answer());
    let _ = submit_answer(&p1, number_answer(2));
    // One rejection during the active continuation must not enter the
    // accepted replay history.
    let before_rejection = controller.export_replay().unwrap();
    let before_checkpoint = controller.checkpoint().unwrap();
    let stale_request = p1.visible_decision().unwrap().unwrap();
    let wrong_cardinality = p1
        .submit(mtgml_decision::DecisionResponseV2 {
            schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
            player_decision_id: stale_request.player_decision_id,
            state_revision: stale_request.state_revision,
            answer: members_answer(&[0]),
        })
        .unwrap();
    assert_eq!(
        wrong_cardinality.submission,
        mtgml_observation::PlayerStepSubmissionV1::Rejected {
            code: mtgml_observation::PlayerSubmissionCodeV1::InvalidCardinality,
        }
    );
    assert_eq!(controller.export_replay().unwrap(), before_rejection);
    assert_eq!(controller.checkpoint().unwrap(), before_checkpoint);

    let _ = submit_answer(&p1, members_answer(&[0, 1]));
    let _ = submit_answer(&p1, order_answer(&[1, 0]));

    let live_after = controller.checkpoint().unwrap();
    let replay = controller.export_replay().unwrap();
    assert_eq!(replay.steps.len(), 4);
    for step in &replay.steps {
        assert!(step.accepted, "rejections are not replay steps");
    }

    // Replay reproduces the whole continuation progression deterministically.
    let report = controller
        .execute_replay_from_checkpoint(c0.clone(), replay.clone())
        .unwrap();
    assert_eq!(report.traces.len(), 4);
    assert_eq!(report.final_checkpoint, live_after);
    assert!(report
        .final_checkpoint
        .state
        .execution
        .continuations
        .is_empty());
}

#[test]
fn stale_stage_response_is_rejected_without_any_mutation() {
    let controller = TrustedEnvironmentController::new(backend());
    let p1 = controller.bind_player(PlayerId(1)).unwrap();

    // Complete the entry and capture the stage-0 (ChooseNumber) identity.
    let _ = submit_answer(&p1, order_entry_answer());
    let stage0_request = p1.visible_decision().unwrap().unwrap();
    let stage0_response = mtgml_decision::DecisionResponseV2 {
        schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
        player_decision_id: stage0_request.player_decision_id,
        state_revision: stage0_request.state_revision,
        answer: number_answer(1),
    };

    // Advance to stage 1 with a fresh visible identity.
    let _ = submit_answer(&p1, number_answer(2));
    let advanced = controller.checkpoint().unwrap();
    let advanced_replay = controller.export_replay().unwrap();

    // Resubmitting the earlier stage response is stale_decision as a typed
    // rejected step mirroring the unchanged product; nothing else mutates.
    let rejected_step = p1.submit(stage0_response).unwrap();
    rejected_step.validate().unwrap();
    assert_eq!(
        rejected_step.submission,
        mtgml_observation::PlayerStepSubmissionV1::Rejected {
            code: mtgml_observation::PlayerSubmissionCodeV1::StaleDecision,
        }
    );
    assert_eq!(
        rejected_step.information_state.state_revision,
        StateRevision(2)
    );
    assert_eq!(
        rejected_step
            .next_decision
            .as_ref()
            .unwrap()
            .player_decision_id,
        PlayerDecisionIdV1(3)
    );
    assert_eq!(controller.checkpoint().unwrap(), advanced);
    assert_eq!(controller.export_replay().unwrap(), advanced_replay);
}

#[test]
fn order_permutations_bind_distinct_replay_identity() {
    let run = |order: &[u32]| -> (AuthoritativeReplayV3, EnvironmentCheckpointV3) {
        let controller = TrustedEnvironmentController::new(backend());
        let p1 = controller.bind_player(PlayerId(1)).unwrap();
        let _ = submit_answer(&p1, order_entry_answer());
        let _ = submit_answer(&p1, number_answer(2));
        let _ = submit_answer(&p1, members_answer(&[0, 1]));
        let _ = submit_answer(&p1, order_answer(order));
        (
            controller.export_replay().unwrap(),
            controller.checkpoint().unwrap(),
        )
    };

    let (forward_replay, forward_checkpoint) = run(&[0, 1]);
    let (reverse_replay, reverse_checkpoint) = run(&[1, 0]);

    // The semantic order lives in the recorded authoritative response.
    fn last(replay: &AuthoritativeReplayV3) -> &DecisionResponseV2 {
        &replay.steps.last().unwrap().response
    }
    assert_eq!(
        last(&forward_replay).answer,
        mtgml_decision::DecisionAnswerV2::Order {
            candidate_ids: vec![CandidateIdV1(0), CandidateIdV1(1)]
        }
    );
    assert_eq!(
        last(&reverse_replay).answer,
        mtgml_decision::DecisionAnswerV2::Order {
            candidate_ids: vec![CandidateIdV1(1), CandidateIdV1(0)]
        }
    );
    // Neither order is repaired into the other: the recorded steps differ.
    assert_ne!(forward_replay, reverse_replay);
    assert_ne!(forward_replay.steps[3], reverse_replay.steps[3]);
    // The final environment identity is legitimately identical because this
    // synthetic domain persists no order; completion erases the sequence.
    assert_eq!(forward_checkpoint.state, reverse_checkpoint.state);
}
