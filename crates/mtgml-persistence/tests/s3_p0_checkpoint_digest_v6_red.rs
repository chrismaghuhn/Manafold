//! S3.P0 Task 1 RED contract for CheckpointDigestV6.

use mtgml_model::{
    CheckpointCodecIdentity, CheckpointDigestV6, EnvironmentLimitCounters, EpisodeStatus,
    ExecutionIdentityV1, ExecutionProgramV1, FullStateDigestV5, SemanticContractIdV1,
};
use mtgml_persistence::checkpoint_digest::{
    calculate_checkpoint_digest_v6, CHECKPOINT_DOMAIN_V6, CHECKPOINT_INPUT_SCHEMA_V6,
};

fn codec(version: &str) -> CheckpointCodecIdentity {
    CheckpointCodecIdentity {
        codec_id: "in-memory-reference".to_owned(),
        semantic_version: version.to_owned(),
    }
}

fn execution_identity(contract: &str) -> ExecutionIdentityV1 {
    ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::MagicRules,
        semantic_contract_id: SemanticContractIdV1::parse(contract).unwrap(),
    }
}

#[test]
fn v6_checkpoint_preimage_binds_state_status_counters_codec_and_execution() {
    assert_eq!(CHECKPOINT_DOMAIN_V6, "mtgml.checkpoint-digest.v6");
    assert_eq!(
        CHECKPOINT_INPUT_SCHEMA_V6,
        "environment-checkpoint-digest-input.v6"
    );

    let status = EpisodeStatus::Running;
    let counters = EnvironmentLimitCounters::default();
    let identity_a = execution_identity(&"11".repeat(32));
    let identity_b = execution_identity(&"22".repeat(32));
    let digest = |state: FullStateDigestV5,
                  status: &EpisodeStatus,
                  counters: &EnvironmentLimitCounters,
                  codec: &CheckpointCodecIdentity,
                  identity: &ExecutionIdentityV1| {
        calculate_checkpoint_digest_v6(
            &state.as_digest_reference(),
            status,
            counters,
            codec,
            identity,
        )
        .unwrap()
    };

    let baseline = digest(
        FullStateDigestV5::from_digest_bytes([1; 32]),
        &status,
        &counters,
        &codec("6"),
        &identity_a,
    );
    assert_ne!(
        baseline,
        digest(
            FullStateDigestV5::from_digest_bytes([2; 32]),
            &status,
            &counters,
            &codec("6"),
            &identity_a,
        )
    );
    assert_ne!(
        baseline,
        digest(
            FullStateDigestV5::from_digest_bytes([1; 32]),
            &status,
            &EnvironmentLimitCounters {
                decisions_submitted: 1,
                accepted_transitions: 1,
                ..EnvironmentLimitCounters::default()
            },
            &codec("6"),
            &identity_a,
        )
    );
    assert_ne!(
        baseline,
        digest(
            FullStateDigestV5::from_digest_bytes([1; 32]),
            &EpisodeStatus::Terminal {
                reason: mtgml_model::TerminalReason::RulesLoss,
                players: vec![mtgml_model::PlayerOutcome {
                    player: mtgml_model::PlayerId(1),
                    result: mtgml_model::PlayerResult::Loss,
                }],
            },
            &counters,
            &codec("6"),
            &identity_a,
        )
    );
    assert_ne!(
        baseline,
        digest(
            FullStateDigestV5::from_digest_bytes([1; 32]),
            &status,
            &counters,
            &codec("6"),
            &identity_b,
        )
    );
    assert!(calculate_checkpoint_digest_v6(
        &FullStateDigestV5::from_digest_bytes([1; 32]).as_digest_reference(),
        &status,
        &counters,
        &codec("5"),
        &identity_a,
    )
    .is_err());
    let _: CheckpointDigestV6 = baseline;
}
