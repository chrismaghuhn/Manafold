//! Task 7 contract tests: `EnvironmentCheckpointV5` (spec §11; ADR 0055
//! §2.7/§2.8).
//!
//! RED expectation: compile failure caused ONLY by the missing Task-7
//! production API (`EnvironmentCheckpointV5`, the V5 schema/codec consts).
//! After GREEN this suite is the permanent Task-7 contract test.
//!
//! Structural contract: unchanged `EngineState`/`FullStateDigestV4`, plus
//! `execution_identity: ExecutionIdentityV1` and
//! `checkpoint_digest: CheckpointDigestV5`. `validate()` must recompute the
//! checkpoint digest FROM THE STORED `execution_identity` and require
//! equality — a tampered identity or tampered digest rejects fail-closed.

use mtgml_model::{
    CheckpointCodecIdentity, EnvironmentLimitCounters, EpisodeStatus, ExecutionIdentityV1,
    ExecutionProgramV1, SemanticContractIdV1,
};
use mtgml_random::RootSeed256;
use mtgml_state::{construct_synthetic_engine_state, SyntheticResetInputs, SyntheticV4Setup};

use mtgml_environment::{
    CheckpointValidationError, EnvironmentCheckpointV5, CHECKPOINT_CODEC_ID_V5,
    CHECKPOINT_CODEC_SEMANTIC_VERSION_V5, ENVIRONMENT_CHECKPOINT_SCHEMA_V5,
};

const CONTRACT_A: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const CONTRACT_B: &str = "2222222222222222222222222222222222222222222222222222222222222222";

fn codec_v5() -> CheckpointCodecIdentity {
    CheckpointCodecIdentity {
        codec_id: CHECKPOINT_CODEC_ID_V5.to_owned(),
        semantic_version: CHECKPOINT_CODEC_SEMANTIC_VERSION_V5.to_owned(),
    }
}

fn codec_semantic_version(version: &str) -> CheckpointCodecIdentity {
    CheckpointCodecIdentity {
        codec_id: CHECKPOINT_CODEC_ID_V5.to_owned(),
        semantic_version: version.to_owned(),
    }
}

fn synthetic_state() -> mtgml_state::EngineState {
    construct_synthetic_engine_state(SyntheticResetInputs {
        players: [mtgml_model::PlayerId(1), mtgml_model::PlayerId(2)],
        root_seed: RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap(),
        setup: SyntheticV4Setup::m2_compatibility(),
    })
    .unwrap()
}

fn synthetic_identity(contract_hex: &str) -> ExecutionIdentityV1 {
    ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::SyntheticRulesCompat,
        semantic_contract_id: SemanticContractIdV1::parse(contract_hex).unwrap(),
    }
}

fn counters() -> EnvironmentLimitCounters {
    EnvironmentLimitCounters::default()
}

#[test]
fn v5_schema_and_codec_constants_are_frozen() {
    assert_eq!(
        ENVIRONMENT_CHECKPOINT_SCHEMA_V5,
        "environment-checkpoint.v5"
    );
    assert_eq!(CHECKPOINT_CODEC_ID_V5, "in-memory-reference");
    assert_eq!(CHECKPOINT_CODEC_SEMANTIC_VERSION_V5, "5");
}

/// Spec §11: the struct carries the UNCHANGED V4 state/digest surfaces plus
/// the two NEW V5 fields; `new()` mirrors V4 (state digest, checkpoint
/// digest via `calculate_checkpoint_digest_v5` with the identity as final
/// input, then `validate()`).
#[test]
fn new_builds_and_self_validates_a_v5_checkpoint() {
    let checkpoint = EnvironmentCheckpointV5::new(
        synthetic_state(),
        EpisodeStatus::Running,
        counters(),
        codec_v5(),
        synthetic_identity(CONTRACT_A),
    )
    .unwrap();
    assert_eq!(checkpoint.schema_version, "environment-checkpoint.v5");
    assert_eq!(checkpoint.codec, codec_v5());
    assert_eq!(
        checkpoint.execution_identity,
        synthetic_identity(CONTRACT_A)
    );
    // Structural fields are the V4-era types (compile-level fact), and the
    // V5 checkpoint digest differs from a V4 digest of the same facts: the
    // identity participates in the preimage.
    checkpoint.validate().unwrap();
}

/// `new()` with the V4 codec version must fail closed: the V5 checkpoint
/// requires the frozen codec pair.
#[test]
fn new_rejects_non_v5_codec_identity() {
    let rejected = EnvironmentCheckpointV5::new(
        synthetic_state(),
        EpisodeStatus::Running,
        counters(),
        codec_semantic_version("4"),
        synthetic_identity(CONTRACT_A),
    );
    assert!(
        rejected.is_err(),
        "V5 checkpoint construction must reject a non-V5 codec identity"
    );
}

/// validate(): recomputes the digest from the STORED identity and equality.
#[test]
fn validate_accepts_the_stored_identity() {
    let checkpoint = EnvironmentCheckpointV5::new(
        synthetic_state(),
        EpisodeStatus::Running,
        counters(),
        codec_v5(),
        synthetic_identity(CONTRACT_A),
    )
    .unwrap();
    checkpoint.validate().unwrap();
}

/// Negative: a tampered `execution_identity` fails the digest recompute that
/// is bound FROM the stored identity.
#[test]
fn tampered_execution_identity_rejected() {
    let mut checkpoint = EnvironmentCheckpointV5::new(
        synthetic_state(),
        EpisodeStatus::Running,
        counters(),
        codec_v5(),
        synthetic_identity(CONTRACT_A),
    )
    .unwrap();
    checkpoint.execution_identity = synthetic_identity(CONTRACT_B);
    assert!(
        checkpoint.validate().is_err(),
        "a tampered execution identity must fail checkpoint validation"
    );
}

/// Negative: a tampered `checkpoint_digest` fails the equality check.
#[test]
fn tampered_checkpoint_digest_rejected() {
    let mut checkpoint = EnvironmentCheckpointV5::new(
        synthetic_state(),
        EpisodeStatus::Running,
        counters(),
        codec_v5(),
        synthetic_identity(CONTRACT_A),
    )
    .unwrap();
    // A different VALID identity's digest, so only the digest field lies.
    let forged = EnvironmentCheckpointV5::new(
        synthetic_state(),
        EpisodeStatus::Running,
        counters(),
        codec_v5(),
        synthetic_identity(CONTRACT_B),
    )
    .unwrap();
    checkpoint.checkpoint_digest = forged.checkpoint_digest;
    assert!(
        checkpoint.validate().is_err(),
        "a checkpoint digest from another identity must be rejected"
    );
}

/// Negative: tampered state digest (state unchanged, digest field forged)
/// fails the state-digest recompute.
#[test]
fn tampered_state_digest_rejected() {
    let mut checkpoint = EnvironmentCheckpointV5::new(
        synthetic_state(),
        EpisodeStatus::Running,
        counters(),
        codec_v5(),
        synthetic_identity(CONTRACT_A),
    )
    .unwrap();
    let other = EnvironmentCheckpointV5::new(
        construct_synthetic_engine_state(SyntheticResetInputs {
            players: [mtgml_model::PlayerId(1), mtgml_model::PlayerId(2)],
            root_seed: RootSeed256::from_lower_hex(&"22".repeat(32)).unwrap(),
            setup: SyntheticV4Setup::m2_compatibility(),
        })
        .unwrap(),
        EpisodeStatus::Running,
        counters(),
        codec_v5(),
        synthetic_identity(CONTRACT_A),
    )
    .unwrap();
    checkpoint.state_digest = other.state_digest;
    assert!(checkpoint.validate().is_err());
}

/// Negative: a wrong schema version string rejects.
#[test]
fn wrong_schema_version_rejected() {
    let mut checkpoint = EnvironmentCheckpointV5::new(
        synthetic_state(),
        EpisodeStatus::Running,
        counters(),
        codec_v5(),
        synthetic_identity(CONTRACT_A),
    )
    .unwrap();
    checkpoint.schema_version = "environment-checkpoint.v4".to_owned();
    assert!(matches!(
        checkpoint.validate(),
        Err(CheckpointValidationError::Identity)
    ));
}

/// Negative: codec "4" inside an otherwise valid V5 checkpoint rejects.
#[test]
fn codec_4_rejected_by_validate() {
    let mut checkpoint = EnvironmentCheckpointV5::new(
        synthetic_state(),
        EpisodeStatus::Running,
        counters(),
        codec_v5(),
        synthetic_identity(CONTRACT_A),
    )
    .unwrap();
    checkpoint.codec = codec_semantic_version("4");
    assert!(matches!(
        checkpoint.validate(),
        Err(CheckpointValidationError::Identity)
    ));
}

/// The completed-checkpoint/pending-decision rule is preserved for V5: a
/// terminal checkpoint with a pending decision rejects, and the same
/// checkpoint validates while the episode is running.
#[test]
fn completed_with_pending_decision_rejected() {
    use mtgml_decision::{DecisionDomainV2, DecisionVisibility};
    use mtgml_state::PendingDecisionRecordV2;

    let mut state = synthetic_state();
    state.execution.pending_decision = Some(PendingDecisionRecordV2 {
        request: mtgml_decision::AuthoritativeDecisionRequestV2 {
            decision_id: mtgml_model::DecisionId(1),
            player_decision_id: mtgml_model::PlayerDecisionIdV1(1),
            state_revision: mtgml_model::StateRevision(0),
            actor: mtgml_model::PlayerId(1),
            visibility: DecisionVisibility::Public,
            decision: DecisionDomainV2::ChooseNumber {
                minimum: 0,
                maximum: 3,
            },
            candidates: Vec::new(),
            continuation_id: None,
        },
    });
    let checkpoint = EnvironmentCheckpointV5::new(
        state.clone(),
        EpisodeStatus::Running,
        counters(),
        codec_v5(),
        synthetic_identity(CONTRACT_A),
    )
    .unwrap();
    checkpoint.validate().unwrap();

    let terminal = EnvironmentCheckpointV5::new(
        state,
        EpisodeStatus::Truncated {
            reason: mtgml_model::TruncationReason::DecisionLimit,
            players: vec![
                mtgml_model::PlayerOutcome {
                    player: mtgml_model::PlayerId(1),
                    result: mtgml_model::PlayerResult::Unresolved,
                },
                mtgml_model::PlayerOutcome {
                    player: mtgml_model::PlayerId(2),
                    result: mtgml_model::PlayerResult::Unresolved,
                },
            ],
        },
        counters(),
        codec_v5(),
        synthetic_identity(CONTRACT_A),
    );
    assert!(
        matches!(
            terminal,
            Err(CheckpointValidationError::CompletedWithDecision)
        ),
        "a completed checkpoint must not retain a pending decision"
    );
}

/// The identity participates at the environment level too: two checkpoints
/// over identical facts but different identities carry different
/// `CheckpointDigestV5` values.
#[test]
fn identical_facts_different_identity_differ_in_digest() {
    let state = synthetic_state();
    let one = EnvironmentCheckpointV5::new(
        state.clone(),
        EpisodeStatus::Running,
        counters(),
        codec_v5(),
        synthetic_identity(CONTRACT_A),
    )
    .unwrap();
    let two = EnvironmentCheckpointV5::new(
        state,
        EpisodeStatus::Running,
        counters(),
        codec_v5(),
        synthetic_identity(CONTRACT_B),
    )
    .unwrap();
    assert_ne!(one.checkpoint_digest, two.checkpoint_digest);
}

/// The retained V4 checkpoint surface stays untouched and green (Task 7
/// must not alter V4 semantics).
#[test]
fn v4_checkpoint_construction_still_works() {
    let checkpoint = mtgml_environment::checkpoint::EnvironmentCheckpointV4::new(
        synthetic_state(),
        EpisodeStatus::Running,
        counters(),
        CheckpointCodecIdentity {
            codec_id: "in-memory-reference".to_owned(),
            semantic_version: "4".to_owned(),
        },
    )
    .unwrap();
    checkpoint.validate().unwrap();
}
