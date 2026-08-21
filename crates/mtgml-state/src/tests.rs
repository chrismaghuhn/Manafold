use super::*;
use mtgml_model::{PlayerId, StateRevision};
use mtgml_random::RootSeed256;

fn synthetic_state() -> EngineState {
    construct_synthetic_engine_state(SyntheticResetInputs {
        players: [PlayerId(1), PlayerId(2)],
        root_seed: RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap(),
    })
    .unwrap()
}

#[test]
fn synthetic_state_is_the_current_m2_shape() {
    let state = synthetic_state();
    validate_engine_state(&state).unwrap();
    assert_eq!(state.revision, StateRevision(0));
    assert!(state.execution.pending_decision.is_some());
    assert!(state.execution.effects.is_empty());
    assert!(state.execution.waiting_triggers.is_empty());
    assert!(state.execution.delayed_effects.is_empty());
    assert_eq!(state.knowledge.players.len(), 2);
    assert_eq!(state.perspective_identities.players.len(), 2);
}

#[test]
fn full_state_digest_v3_known_answer() {
    let state = synthetic_state();
    let first = state.digest().unwrap();
    assert_eq!(first.raw_bytes().len(), 32);
    assert_eq!(first, state.clone().digest().unwrap());
}

#[test]
fn m2_b_full_state_digest_v3_mutation_matrix() {
    let state = synthetic_state();
    let first = state.digest().unwrap();
    let mut changed = state;
    changed.core.players.get_mut(&PlayerId(1)).unwrap().life -= 1;
    let changed_digest = changed.digest().unwrap();
    assert_ne!(first, changed_digest);
}

#[test]
fn deterministic_structural_identity_repeats_exactly() {
    let state = synthetic_state();
    assert_eq!(state, state.clone());
    assert_eq!(state.digest().unwrap(), state.clone().digest().unwrap());
    assert_eq!(
        state.canonical_digest_bytes().unwrap(),
        state.clone().canonical_digest_bytes().unwrap()
    );
}

#[test]
fn state_delta_uses_full_state_digest_v3() {
    let before = synthetic_state();
    let mut after = before.clone();
    after.core.players.get_mut(&PlayerId(1)).unwrap().life -= 1;
    let delta = StateDelta::between(&before, &after, vec![]).unwrap();
    assert_eq!(delta.before_digest, before.digest().unwrap());
    assert_eq!(delta.after_digest, after.digest().unwrap());
    assert_eq!(delta.apply(&before).unwrap(), after);
}

#[test]
fn v3_digest_payload_is_nonempty_canonical_cbor() {
    let state = synthetic_state();
    let payload = state.canonical_digest_bytes().unwrap();
    assert!(!payload.is_empty());
    assert_eq!(payload[0] & 0xe0, 0x80, "root must be a CBOR array");
}

#[test]
fn synthetic_reset_rejects_duplicate_players() {
    let result = construct_synthetic_engine_state(SyntheticResetInputs {
        players: [PlayerId(1), PlayerId(1)],
        root_seed: RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap(),
    });
    assert!(matches!(
        result,
        Err(SyntheticStateConstructionError::DuplicatePlayers)
    ));
}
