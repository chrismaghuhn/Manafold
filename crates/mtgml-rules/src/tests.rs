use super::*;

use mtgml_decision::{
    DecisionAnswerV2, DecisionDomainV2, DecisionResponseV2, DECISION_RESPONSE_V2_SCHEMA,
};

use mtgml_model::{
    CandidateIdV1, ContinuationId, DecisionId, PlayerDecisionIdV1, PlayerId, StateRevision,
    VisibleSequence,
};

use mtgml_random::RootSeed256;

use mtgml_state::{
    construct_synthetic_engine_state, AssemblyStageV2, BaseCharacteristics, ContinuationPayloadV2,
    ContinuationRecordV2, ControlHistory, EngineState, FoundationCreatureSource,
    FoundationSourceKind, IdentityMutationV1, PendingDecisionRecordV2, PerspectiveLifecycleAuditV1,
    PerspectiveLifecycleMutationV1, PriorityState, SyntheticResetInputs, SyntheticV4Setup,
};

use crate::ProgramKernelV1;

fn boundary_kernel() -> ProgramKernelV1 {
    ProgramKernelV1::for_program(mtgml_model::ExecutionProgramV1::SyntheticRulesCompat)
        .expect("the synthetic program is supported by the current kernel boundary")
}

fn synthetic_state() -> mtgml_state::EngineState {
    construct_synthetic_engine_state(SyntheticResetInputs {
        players: [PlayerId(1), PlayerId(2)],
        root_seed: RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap(),
        setup: SyntheticV4Setup::synthetic_compatibility(),
    })
    .unwrap()
}

fn state_without_pending_decision() -> EngineState {
    let mut state = synthetic_state();
    state.execution.pending_decision = None;
    state
}

fn continuation_state() -> EngineState {
    let mut state = state_without_pending_decision();
    state.execution.continuations.insert(
        mtgml_model::ContinuationId(1),
        ContinuationRecordV2 {
            id: mtgml_model::ContinuationId(1),
            actor: PlayerId(1),
            created_at_revision: StateRevision(0),
            stage_index: AssemblyStageV2::ChooseCount.stage_index(),
            payload: ContinuationPayloadV2::SyntheticM2Assembly {
                stage: AssemblyStageV2::ChooseCount,
                selected_count: None,
                selected_piece_keys: Vec::new(),
                ordered_piece_keys: Vec::new(),
            },
        },
    );
    state.execution.pending_decision = Some(PendingDecisionRecordV2 {
        request: mtgml_decision::AuthoritativeDecisionRequestV2 {
            decision_id: DecisionId(9),
            player_decision_id: PlayerDecisionIdV1(9),
            state_revision: StateRevision(0),
            actor: PlayerId(1),
            visibility: mtgml_decision::DecisionVisibility::Public,
            decision: mtgml_decision::DecisionDomainV2::ChooseNumber {
                minimum: 0,
                maximum: 3,
            },
            candidates: Vec::new(),
            continuation_id: Some(mtgml_model::ContinuationId(1)),
        },
    });
    state.allocators.next_decision_id = DecisionId(10);
    state.allocators.next_continuation_id = mtgml_model::ContinuationId(2);
    state
        .perspective_identities
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .next_player_decision_id = PlayerDecisionIdV1(10);
    mtgml_state::validate_engine_state(&state).unwrap();
    state
}

fn response(candidate_id: u32, revision: u64) -> DecisionResponseV2 {
    DecisionResponseV2 {
        schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
        player_decision_id: PlayerDecisionIdV1(1),
        state_revision: StateRevision(revision),
        answer: DecisionAnswerV2::SelectOne {
            candidate_id: CandidateIdV1(candidate_id),
        },
    }
}

fn select_one_response(candidate_id: u32, revision: u64) -> DecisionResponseV2 {
    DecisionResponseV2 {
        schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
        player_decision_id: PlayerDecisionIdV1(1),
        state_revision: StateRevision(revision),
        answer: DecisionAnswerV2::SelectOne {
            candidate_id: CandidateIdV1(candidate_id),
        },
    }
}

fn number_response(player_decision: u64, value: i64, revision: u64) -> DecisionResponseV2 {
    DecisionResponseV2 {
        schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
        player_decision_id: PlayerDecisionIdV1(player_decision),
        state_revision: StateRevision(revision),
        answer: DecisionAnswerV2::ChooseNumber { value },
    }
}

fn many_response(player_decision: u64, candidate_ids: &[u32], revision: u64) -> DecisionResponseV2 {
    DecisionResponseV2 {
        schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
        player_decision_id: PlayerDecisionIdV1(player_decision),
        state_revision: StateRevision(revision),
        answer: DecisionAnswerV2::SelectMany {
            candidate_ids: candidate_ids.iter().map(|id| CandidateIdV1(*id)).collect(),
        },
    }
}

fn order_response(
    player_decision: u64,
    candidate_ids: &[u32],
    revision: u64,
) -> DecisionResponseV2 {
    DecisionResponseV2 {
        schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
        player_decision_id: PlayerDecisionIdV1(player_decision),
        state_revision: StateRevision(revision),
        answer: DecisionAnswerV2::Order {
            candidate_ids: candidate_ids.iter().map(|id| CandidateIdV1(*id)).collect(),
        },
    }
}

fn apply(state: &EngineState, response: &DecisionResponseV2) -> TransitionResult {
    let mut kernel = boundary_kernel();
    kernel.apply(state, PlayerId(1), response).unwrap()
}

#[test]
fn rng_exhaustion_is_a_typed_internal_failure_without_input_mutation() {
    use mtgml_random::{RandomStreamCursorV1, RandomStreamKeyV1, RandomStreamKindV1};
    let mut state = synthetic_state();
    let key = RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1);
    state
        .random
        .set_cursor(
            &key,
            RandomStreamCursorV1 {
                next_raw_u64: u64::MAX,
            },
        )
        .unwrap();
    let before = state.clone();
    let mut kernel = boundary_kernel();
    let error = kernel
        .apply(&state, PlayerId(1), &response(0, 0))
        .unwrap_err();
    assert!(matches!(error, KernelExecutionError::Random(_)));
    assert_eq!(state, before, "kernel input must never be mutated");
}

#[test]
fn effect_allocator_exhaustion_is_a_typed_internal_failure_before_rng() {
    use mtgml_model::EffectInstanceId;
    use mtgml_random::{RandomStreamKeyV1, RandomStreamKindV1};
    let mut state = synthetic_state();
    state.allocators.next_effect_id = EffectInstanceId(u64::MAX);
    let key = RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1);
    let cursor_before = state.random.lookup_stream(&key).unwrap().next_raw_u64;
    let mut kernel = boundary_kernel();
    let error = kernel
        .apply(&state, PlayerId(1), &response(0, 0))
        .unwrap_err();
    assert!(matches!(error, KernelExecutionError::IdentityAllocation(_)));
    assert_eq!(
        state.random.lookup_stream(&key).unwrap().next_raw_u64,
        cursor_before,
        "exhaustion must fail before any randomness is consumed"
    );
}

fn entry_stage0() -> EngineState {
    apply(&synthetic_state(), &select_one_response(0, 0)).next_state
}

// Lexical fragments: physical discoverability without changing any
// tests::<name> identity addressed by the M1/M2 gate runners.
include!("tests/forced_progress.rs");
include!("tests/synthetic_program.rs");
include!("tests/transition_contract.rs");
include!("tests/determinism.rs");
include!("tests/batch_e.rs");
include!("tests/batch_f.rs");
include!("tests/turn_structure.rs");
include!("tests/magic_turn_structure.rs");
include!("tests/state_based_actions.rs");
include!("tests/basic_priority.rs");
mod zone_incarnation_tests {
    use super::*;
    include!("tests/zone_incarnation.rs");
}

mod magic_execution_profile_tests {
    include!("tests/magic_execution_profile.rs");
}
