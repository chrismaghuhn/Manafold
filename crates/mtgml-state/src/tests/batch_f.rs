fn state_with_candidate(
    visible: mtgml_decision::CandidateIntent,
    trusted: mtgml_decision::EngineCandidateBinding,
) -> EngineState {
    let mut state = synthetic_state();
    let candidate = &mut state
        .execution
        .pending_decision
        .as_mut()
        .unwrap()
        .request
        .candidates[0];
    candidate.visible_intent = visible;
    candidate.trusted_binding = trusted;
    state
}

fn state_with_ability_candidate(
    resolved: mtgml_model::AbilityInstanceId,
    trusted: mtgml_model::AbilityInstanceId,
) -> EngineState {
    let mut state = synthetic_state();
    let identity = state
        .perspective_identities
        .players
        .get_mut(&PlayerId(1))
        .unwrap();
    identity
        .opaque_to_ability
        .insert(OpaqueAbilityId(1), resolved);
    identity
        .ability_to_opaque
        .insert(resolved, OpaqueAbilityId(1));
    identity.next_opaque_ability_id = OpaqueAbilityId(2);
    state.allocators.next_ability_id = mtgml_model::AbilityInstanceId(
        resolved.0.max(trusted.0) + 1,
    );
    let candidate = &mut state
        .execution
        .pending_decision
        .as_mut()
        .unwrap()
        .request
        .candidates[0];
    candidate.visible_intent = mtgml_decision::CandidateIntent::ActivateAbility {
        ability: OpaqueAbilityId(1),
    };
    candidate.trusted_binding = mtgml_decision::EngineCandidateBinding::ActivateAbility {
        ability: trusted,
    };
    state
}

#[test]
fn fnd_013_authoritative_state_rejects_scalar_binding_mismatches() {
    let cases = vec![
        (
            mtgml_decision::CandidateIntent::SelectPlayer { player: PlayerId(1) },
            mtgml_decision::EngineCandidateBinding::SelectPlayer { player: PlayerId(2) },
        ),
        (
            mtgml_decision::CandidateIntent::SelectMode { mode_index: 1 },
            mtgml_decision::EngineCandidateBinding::SelectMode { mode_index: 2 },
        ),
        (
            mtgml_decision::CandidateIntent::ChooseBoolean { value: true },
            mtgml_decision::EngineCandidateBinding::ChooseBoolean { value: false },
        ),
        (
            mtgml_decision::CandidateIntent::DeclareNumber { value: 1 },
            mtgml_decision::EngineCandidateBinding::DeclareNumber { value: 2 },
        ),
        (
            mtgml_decision::CandidateIntent::SelectObject {
                object: OpaqueObjectId(1),
            },
            mtgml_decision::EngineCandidateBinding::SelectObject {
                object: GameObjectId(2),
            },
        ),
    ];
    for (visible, trusted) in cases {
        let state = state_with_candidate(visible, trusted);
        let before = state.clone();
        assert_eq!(
            validate_engine_state(&state),
            Err(EngineStateViolation::PendingDecisionMismatch)
        );
        assert_eq!(state, before);
    }
}

#[test]
fn fnd_013_authoritative_state_rejects_ability_binding_mismatch() {
    let state = state_with_ability_candidate(AbilityInstanceId(1), AbilityInstanceId(2));
    let before = state.clone();
    assert_eq!(
        validate_engine_state(&state),
        Err(EngineStateViolation::PendingDecisionMismatch)
    );
    assert_eq!(state, before);
}

#[test]
fn fnd_013_authoritative_state_accepts_exact_binding_controls() {
    let cases = vec![
        (
            mtgml_decision::CandidateIntent::SelectPlayer { player: PlayerId(1) },
            mtgml_decision::EngineCandidateBinding::SelectPlayer { player: PlayerId(1) },
        ),
        (
            mtgml_decision::CandidateIntent::SelectMode { mode_index: 1 },
            mtgml_decision::EngineCandidateBinding::SelectMode { mode_index: 1 },
        ),
        (
            mtgml_decision::CandidateIntent::ChooseBoolean { value: true },
            mtgml_decision::EngineCandidateBinding::ChooseBoolean { value: true },
        ),
        (
            mtgml_decision::CandidateIntent::DeclareNumber { value: 1 },
            mtgml_decision::EngineCandidateBinding::DeclareNumber { value: 1 },
        ),
        (
            mtgml_decision::CandidateIntent::SelectObject {
                object: OpaqueObjectId(1),
            },
            mtgml_decision::EngineCandidateBinding::SelectObject {
                object: GameObjectId(1),
            },
        ),
    ];
    for (visible, trusted) in cases {
        assert!(validate_engine_state(&state_with_candidate(visible, trusted)).is_ok());
    }
    assert!(validate_engine_state(&state_with_ability_candidate(
        AbilityInstanceId(1),
        AbilityInstanceId(1),
    ))
    .is_ok());
}
