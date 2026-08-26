// Ownership fragment: zone/allocator invariant evidence. Included lexically by tests.rs so
// every identity remains tests::<name>.

#[test]
fn global_decision_allocator_must_exceed_every_issued_identity() {
    let build = |next: u64| {
        let mut state = synthetic_state();
        state.allocators.next_decision_id = DecisionId(next);
        state
    };
    // next == issued must fail closed.
    assert_eq!(
        validate_engine_state(&build(1)),
        Err(EngineStateViolation::AllocatorBehind)
    );
    // next < issued must fail closed.
    assert_eq!(
        validate_engine_state(&build(0)),
        Err(EngineStateViolation::AllocatorBehind)
    );
    // A strictly greater cursor is accepted.
    validate_engine_state(&build(2)).unwrap();
    assert!(build(2).digest().is_ok());
}

#[test]
fn global_ability_allocator_must_exceed_every_issued_identity() {
    let issue = |state: &mut EngineState| {
        let identity = state
            .perspective_identities
            .players
            .get_mut(&PlayerId(1))
            .unwrap();
        identity
            .opaque_to_ability
            .insert(OpaqueAbilityId(1), AbilityInstanceId(5));
        identity
            .ability_to_opaque
            .insert(AbilityInstanceId(5), OpaqueAbilityId(1));
        identity.next_opaque_ability_id = OpaqueAbilityId(2);
    };
    let build = |next: u64| {
        let mut state = synthetic_state();
        issue(&mut state);
        state.allocators.next_ability_id = AbilityInstanceId(next);
        state
    };
    // next == issued must fail closed.
    assert_eq!(
        validate_engine_state(&build(5)),
        Err(EngineStateViolation::AllocatorBehind)
    );
    // next < issued must fail closed.
    assert_eq!(
        validate_engine_state(&build(4)),
        Err(EngineStateViolation::AllocatorBehind)
    );
    // A strictly greater cursor is accepted.
    validate_engine_state(&build(6)).unwrap();
}
