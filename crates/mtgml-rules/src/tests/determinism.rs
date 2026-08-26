// Ownership fragment: determinism/stream-isolation evidence. Included lexically by tests.rs so
// every identity remains tests::<name>.

#[test]
fn deterministic_services_repeat_exact_transition_result() {
    let first = synthetic_state();
    let second = synthetic_state();
    let mut kernel_a = SyntheticM1RulesKernel;
    let mut kernel_b = SyntheticM1RulesKernel;
    let left = kernel_a
        .apply(&first, PlayerId(1), &response(0, 0))
        .unwrap();
    let right = kernel_b
        .apply(&second, PlayerId(1), &response(0, 0))
        .unwrap();
    assert_eq!(left, right);
    assert_eq!(
        left.next_state.digest().unwrap(),
        right.next_state.digest().unwrap()
    );
}

#[test]
fn deterministic_services_isolate_unrelated_stream_cursors() {
    use mtgml_random::{RandomStreamCursorV1, RandomStreamKindV1};
    let baseline_state = synthetic_state();
    let mut isolated_state = synthetic_state();
    isolated_state.random.streams.insert(
        mtgml_random::RandomStreamKeyV1::player_scoped(RandomStreamKindV1::SyntheticM1, 2),
        RandomStreamCursorV1 {
            next_raw_u64: 987_654_321,
        },
    );
    let mut kernel_a = SyntheticM1RulesKernel;
    let mut kernel_b = SyntheticM1RulesKernel;
    let baseline = kernel_a
        .apply(&baseline_state, PlayerId(1), &response(0, 0))
        .unwrap();
    let isolated = kernel_b
        .apply(&isolated_state, PlayerId(1), &response(0, 0))
        .unwrap();
    // The unrelated player-scoped cursor must not influence the transition:
    // identical events, audit trace, and consumed global-stream progression.
    assert_eq!(baseline.events, isolated.events);
    assert_eq!(baseline.delta.audit, isolated.delta.audit);
    let global =
        mtgml_random::RandomStreamKeyV1::global(mtgml_random::RandomStreamKindV1::SyntheticM1);
    assert_eq!(
        baseline
            .next_state
            .random
            .lookup_stream(&global)
            .unwrap()
            .next_raw_u64,
        isolated
            .next_state
            .random
            .lookup_stream(&global)
            .unwrap()
            .next_raw_u64
    );
}

#[test]
fn candidate_order_independent_of_global_allocator_history() {
    // Two environments in the identical semantic situation that differ ONLY
    // in unrelated global allocator history and unused RNG stream state.
    let build = |allocator_history: u64, unused_cursor: u64| {
        let after_entry = apply(&synthetic_state(), &select_one_response(0, 0)).next_state;
        let mut state = apply(&after_entry, &number_response(2, 2, 1)).next_state;
        state.allocators.next_effect_id = mtgml_model::EffectInstanceId(allocator_history);
        state.allocators.next_trigger_id = mtgml_model::TriggerInstanceId(allocator_history);
        state.allocators.next_stack_object_id = mtgml_model::StackObjectId(allocator_history);
        state.random.streams.insert(
            mtgml_random::RandomStreamKeyV1::player_scoped(
                mtgml_random::RandomStreamKindV1::SyntheticM1,
                2,
            ),
            mtgml_random::RandomStreamCursorV1 {
                next_raw_u64: unused_cursor,
            },
        );
        mtgml_state::validate_engine_state(&state).unwrap();
        state
    };

    let history_x = build(40, 1);
    let history_y = build(9_000, u64::from(u32::MAX));

    // The visible candidate surface must be identical.
    let visible = |state: &EngineState| {
        state
            .execution
            .pending_decision
            .as_ref()
            .unwrap()
            .request
            .project_player_request()
            .unwrap()
    };
    assert_eq!(visible(&history_x), visible(&history_y));

    // The next-stage candidate generation must be identical as well.
    let next_x = apply(&history_x, &many_response(3, &[0, 1], 2));
    let next_y = apply(&history_y, &many_response(3, &[0, 1], 2));
    let stage_request_x = next_x.next_decision.as_ref().unwrap();
    let stage_request_y = next_y.next_decision.as_ref().unwrap();
    assert_eq!(stage_request_x.candidates, stage_request_y.candidates);
}
