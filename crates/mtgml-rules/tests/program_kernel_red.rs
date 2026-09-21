//! Task 4 RED tests: the program-owned kernel boundary (spec §23a.1, ADR
//! 0055 `PROGRAM_OWNS_ALL_KERNEL_ENTRYPOINTS`).
//!
//! RED expectation: compile failure caused ONLY by the missing public
//! boundary API (`ProgramKernelV1`, `ProgramKernelConstructionErrorV1`).
//! After GREEN this suite is the permanent Task-4 contract test.

use mtgml_model::ExecutionProgramV1;
use mtgml_rules::{ProgramKernelConstructionErrorV1, ProgramKernelV1};

fn boundary() -> ProgramKernelV1 {
    ProgramKernelV1::for_program(ExecutionProgramV1::SyntheticRulesCompat)
        .expect("synthetic rules compat is supported pre-S1")
}

fn committed_state() -> mtgml_state::EngineState {
    mtgml_state::construct_synthetic_engine_state(mtgml_state::SyntheticResetInputs {
        players: [mtgml_model::PlayerId(1), mtgml_model::PlayerId(2)],
        root_seed: mtgml_random::RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap(),
        setup: mtgml_state::SyntheticV4Setup::m2_compatibility(),
    })
    .unwrap()
}

fn select_one_response() -> mtgml_decision::DecisionResponseV2 {
    mtgml_decision::DecisionResponseV2 {
        schema_version: mtgml_decision::DECISION_RESPONSE_V2_SCHEMA.into(),
        player_decision_id: mtgml_model::PlayerDecisionIdV1(1),
        state_revision: mtgml_model::StateRevision(0),
        answer: mtgml_decision::DecisionAnswerV2::SelectOne {
            candidate_id: mtgml_model::CandidateIdV1(0),
        },
    }
}

#[test]
fn for_program_constructs_the_synthetic_kernel() {
    let _kernel = boundary();
}

#[test]
fn magic_rules_is_unsupported_pre_s1() {
    let error = ProgramKernelV1::for_program(ExecutionProgramV1::MagicRules)
        .expect_err("no production Magic contract exists pre-S1");
    assert!(
        matches!(error, ProgramKernelConstructionErrorV1::UnsupportedProgram),
        "the only construction error pre-S1 is UnsupportedProgram"
    );
}

#[test]
fn apply_entry_point_dispatches_to_the_synthetic_kernel() {
    let mut kernel = boundary();
    let state = committed_state();
    // The boundary forwards the trusted apply transaction to the wrapped
    // kernel. A rev-0 state with no pending decision is a player-caused
    // rejection; the dispatch itself is the contract under test here.
    let _product = kernel
        .apply(&state, mtgml_model::PlayerId(1), &select_one_response())
        .expect("dispatched apply returns the kernel product");
}

#[test]
fn forced_progress_entry_point_dispatches_to_the_synthetic_kernel() {
    // Forced progress admits only a pristine decision-less setup: a freshly
    // constructed state carries a pending decision, so it must be cleared
    // first (same precondition as the rules-internal stabilization proof).
    let mut state = committed_state();
    state.execution.pending_decision = None;
    let mut kernel = boundary();
    let product = kernel
        .advance_forced_progress(&state)
        .expect("dispatched forced progress derives the synthetic entry decision");
    assert!(
        product.next_state.execution.pending_decision.is_some(),
        "forced progress on a pristine state creates the entry decision"
    );
}
