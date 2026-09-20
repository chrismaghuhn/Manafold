//! Task 6 contract tests: `calculate_checkpoint_digest_v5` (spec §9, §19.3;
//! ADR 0055 §2.7).
//!
//! RED expectation: compile failure caused ONLY by the missing Task-6
//! production API (`calculate_checkpoint_digest_v5`, V5 domain/schema
//! constants). After GREEN this suite is the permanent Task-6 contract test.
//!
//! Payload contract (spec §9): the V4-verified six elements with V5
//! schema/domain strings plus `ExecutionIdentityV1` as the SEVENTH (last)
//! element, canonically encoded as
//! `[program_kind_variant, semantic_contract_id_32bytes]`.

use mtgml_model::{
    CheckpointCodecIdentity, EnvironmentLimitCounters, EpisodeStatus, ExecutionIdentityV1,
    ExecutionProgramV1, FullStateDigestV4, SemanticContractIdV1,
};
use mtgml_persistence::checkpoint_digest::{
    calculate_checkpoint_digest_v5, CHECKPOINT_DOMAIN_V5, CHECKPOINT_INPUT_SCHEMA_V5,
};

fn codec_v5() -> CheckpointCodecIdentity {
    CheckpointCodecIdentity {
        codec_id: "in-memory-reference".to_owned(),
        semantic_version: "5".to_owned(),
    }
}

fn codec_semantic_version(version: &str) -> CheckpointCodecIdentity {
    CheckpointCodecIdentity {
        codec_id: "in-memory-reference".to_owned(),
        semantic_version: version.to_owned(),
    }
}

fn identity(program: ExecutionProgramV1, contract_hex: &str) -> ExecutionIdentityV1 {
    ExecutionIdentityV1 {
        program_kind: program,
        semantic_contract_id: SemanticContractIdV1::parse(contract_hex).unwrap(),
    }
}

fn synthetic_identity(contract_hex: &str) -> ExecutionIdentityV1 {
    identity(ExecutionProgramV1::SyntheticRulesCompat, contract_hex)
}

const CONTRACT_A: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const CONTRACT_B: &str = "2222222222222222222222222222222222222222222222222222222222222222";

#[test]
fn v5_domain_and_schema_constants_are_frozen() {
    assert_eq!(CHECKPOINT_DOMAIN_V5, "mtgml.checkpoint-digest.v5");
    assert_eq!(
        CHECKPOINT_INPUT_SCHEMA_V5,
        "environment-checkpoint-digest-input.v5"
    );
}

#[test]
fn v5_digest_renders_64_lowercase_hex() {
    let digest = calculate_checkpoint_digest_v5(
        &FullStateDigestV4::from_digest_bytes([7; 32]).as_digest_reference(),
        &EpisodeStatus::Running,
        &EnvironmentLimitCounters::default(),
        &codec_v5(),
        &synthetic_identity(CONTRACT_A),
    )
    .unwrap();
    assert_eq!(digest.as_str().len(), 64);
    assert!(
        digest
            .as_str()
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
        "V5 checkpoint digest must render canonical 64-lowercase-hex"
    );
}

/// Spec §19.3 KAT: identical input ⇒ identical digest.
#[test]
fn identical_input_yields_identical_digest() {
    let state = FullStateDigestV4::from_digest_bytes([7; 32]).as_digest_reference();
    let counters = EnvironmentLimitCounters::default();
    let id = synthetic_identity(CONTRACT_A);
    let first = calculate_checkpoint_digest_v5(
        &state,
        &EpisodeStatus::Running,
        &counters,
        &codec_v5(),
        &id,
    )
    .unwrap();
    let second = calculate_checkpoint_digest_v5(
        &state,
        &EpisodeStatus::Running,
        &counters,
        &codec_v5(),
        &id,
    )
    .unwrap();
    assert_eq!(first, second);
}

/// Spec §19.3 mutation vector: different `program_kind` ⇒ different digest.
#[test]
fn different_program_kind_changes_digest() {
    let state = FullStateDigestV4::from_digest_bytes([7; 32]).as_digest_reference();
    let counters = EnvironmentLimitCounters::default();
    let synthetic = calculate_checkpoint_digest_v5(
        &state,
        &EpisodeStatus::Running,
        &counters,
        &codec_v5(),
        &synthetic_identity(CONTRACT_A),
    )
    .unwrap();
    let magic = calculate_checkpoint_digest_v5(
        &state,
        &EpisodeStatus::Running,
        &counters,
        &codec_v5(),
        &identity(ExecutionProgramV1::MagicRules, CONTRACT_A),
    )
    .unwrap();
    assert_ne!(synthetic, magic);
}

/// Spec §19.3 mutation vector: different `semantic_contract_id` ⇒ different
/// digest.
#[test]
fn different_semantic_contract_id_changes_digest() {
    let state = FullStateDigestV4::from_digest_bytes([7; 32]).as_digest_reference();
    let counters = EnvironmentLimitCounters::default();
    let with_a = calculate_checkpoint_digest_v5(
        &state,
        &EpisodeStatus::Running,
        &counters,
        &codec_v5(),
        &synthetic_identity(CONTRACT_A),
    )
    .unwrap();
    let with_b = calculate_checkpoint_digest_v5(
        &state,
        &EpisodeStatus::Running,
        &counters,
        &codec_v5(),
        &synthetic_identity(CONTRACT_B),
    )
    .unwrap();
    assert_ne!(with_a, with_b);
}

/// A different full-state input (a different V4 state digest) changes the
/// checkpoint digest: every payload element matters.
#[test]
fn different_full_state_changes_digest() {
    let counters = EnvironmentLimitCounters::default();
    let id = synthetic_identity(CONTRACT_A);
    let seven = calculate_checkpoint_digest_v5(
        &FullStateDigestV4::from_digest_bytes([7; 32]).as_digest_reference(),
        &EpisodeStatus::Running,
        &counters,
        &codec_v5(),
        &id,
    )
    .unwrap();
    let nine = calculate_checkpoint_digest_v5(
        &FullStateDigestV4::from_digest_bytes([9; 32]).as_digest_reference(),
        &EpisodeStatus::Running,
        &counters,
        &codec_v5(),
        &id,
    )
    .unwrap();
    assert_ne!(seven, nine);
}

/// The checkpoint binds the FULL identity struct: program kind and semantic
/// contract id BOTH participate (already proven separately above); this
/// pins the joint-mutation case as well.
#[test]
fn fully_different_identity_changes_digest() {
    let state = FullStateDigestV4::from_digest_bytes([7; 32]).as_digest_reference();
    let counters = EnvironmentLimitCounters::default();
    let baseline = calculate_checkpoint_digest_v5(
        &state,
        &EpisodeStatus::Running,
        &counters,
        &codec_v5(),
        &synthetic_identity(CONTRACT_A),
    )
    .unwrap();
    let mutated = calculate_checkpoint_digest_v5(
        &state,
        &EpisodeStatus::Running,
        &counters,
        &codec_v5(),
        &identity(ExecutionProgramV1::MagicRules, CONTRACT_B),
    )
    .unwrap();
    assert_ne!(baseline, mutated);
}

/// Spec §9: the V5 checkpoint digest is identity-bound — the SAME V4-era
/// checkpoint facts under a DIFFERENT identity produce a different digest.
/// The V4 digest of the same facts stays unchanged (versions are disjoint).
#[test]
fn same_checkpoint_facts_different_identity_different_digest() {
    let state = FullStateDigestV4::from_digest_bytes([3; 32]).as_digest_reference();
    let counters = EnvironmentLimitCounters::default();
    let one = calculate_checkpoint_digest_v5(
        &state,
        &EpisodeStatus::Running,
        &counters,
        &codec_v5(),
        &synthetic_identity(CONTRACT_A),
    )
    .unwrap();
    let two = calculate_checkpoint_digest_v5(
        &state,
        &EpisodeStatus::Running,
        &counters,
        &codec_v5(),
        &synthetic_identity(CONTRACT_B),
    )
    .unwrap();
    assert_ne!(one, two);
}

/// Spec §9: codec semantic_version must be exactly `"5"`.
#[test]
fn codec_semantic_version_5_enforced() {
    let state = FullStateDigestV4::from_digest_bytes([7; 32]).as_digest_reference();
    let counters = EnvironmentLimitCounters::default();
    let id = synthetic_identity(CONTRACT_A);
    let ok = calculate_checkpoint_digest_v5(
        &state,
        &EpisodeStatus::Running,
        &counters,
        &codec_v5(),
        &id,
    );
    assert!(ok.is_ok(), "semantic_version \"5\" must be accepted");
}

/// Negative: codec semantic_version `"4"` is rejected fail-closed.
#[test]
fn codec_semantic_version_4_rejected() {
    let state = FullStateDigestV4::from_digest_bytes([7; 32]).as_digest_reference();
    let counters = EnvironmentLimitCounters::default();
    let id = synthetic_identity(CONTRACT_A);
    let rejected = calculate_checkpoint_digest_v5(
        &state,
        &EpisodeStatus::Running,
        &counters,
        &codec_semantic_version("4"),
        &id,
    );
    assert!(rejected.is_err(), "semantic_version \"4\" must be rejected");
}

/// Negative: a wrong codec pair (different codec id) is rejected.
#[test]
fn wrong_codec_pair_rejected() {
    let state = FullStateDigestV4::from_digest_bytes([7; 32]).as_digest_reference();
    let counters = EnvironmentLimitCounters::default();
    let id = synthetic_identity(CONTRACT_A);
    let rejected = calculate_checkpoint_digest_v5(
        &state,
        &EpisodeStatus::Running,
        &counters,
        &CheckpointCodecIdentity {
            codec_id: "some-other-codec".to_owned(),
            semantic_version: "5".to_owned(),
        },
        &id,
    );
    assert!(rejected.is_err(), "wrong codec pair must be rejected");
}

/// Negative: the full-state reference must still validate against the
/// UNCHANGED V4 domain/schema (`mtgml.full-state-digest.v4` +
/// `full-state-digest-input.v4`) — a foreign-domain reference is rejected.
#[test]
fn full_state_reference_domain_mismatch_rejected() {
    let mut reference = FullStateDigestV4::from_digest_bytes([7; 32]).as_digest_reference();
    reference.semantic_domain = "mtgml.full-state-digest.v3".to_owned();
    let counters = EnvironmentLimitCounters::default();
    let id = synthetic_identity(CONTRACT_A);
    let rejected = calculate_checkpoint_digest_v5(
        &reference,
        &EpisodeStatus::Running,
        &counters,
        &codec_v5(),
        &id,
    );
    assert!(
        rejected.is_err(),
        "foreign full-state domain must be rejected"
    );
}

/// Negative: a full-state reference with the V4 domain but a WRONG input
/// schema id is rejected.
#[test]
fn full_state_reference_schema_mismatch_rejected() {
    let mut reference = FullStateDigestV4::from_digest_bytes([7; 32]).as_digest_reference();
    reference.input_schema_id = "full-state-digest-input.v3".to_owned();
    let counters = EnvironmentLimitCounters::default();
    let id = synthetic_identity(CONTRACT_A);
    let rejected = calculate_checkpoint_digest_v5(
        &reference,
        &EpisodeStatus::Running,
        &counters,
        &codec_v5(),
        &id,
    );
    assert!(
        rejected.is_err(),
        "wrong full-state input schema must be rejected"
    );
}

/// Negative: an empty codec id is rejected (fail-closed codec identity).
#[test]
fn empty_codec_identity_rejected() {
    let state = FullStateDigestV4::from_digest_bytes([7; 32]).as_digest_reference();
    let counters = EnvironmentLimitCounters::default();
    let id = synthetic_identity(CONTRACT_A);
    let rejected = calculate_checkpoint_digest_v5(
        &state,
        &EpisodeStatus::Running,
        &counters,
        &CheckpointCodecIdentity {
            codec_id: String::new(),
            semantic_version: "5".to_owned(),
        },
        &id,
    );
    assert!(rejected.is_err(), "empty codec id must be rejected");
}

/// The identity participates as the LAST payload element; mutating any of
/// the six V4-carried facts must also change the digest (element 4: status).
#[test]
fn different_episode_status_changes_digest() {
    use mtgml_model::{PlayerOutcome, PlayerResult, TerminalReason};
    let state = FullStateDigestV4::from_digest_bytes([7; 32]).as_digest_reference();
    let counters = EnvironmentLimitCounters::default();
    let id = synthetic_identity(CONTRACT_A);
    let running = calculate_checkpoint_digest_v5(
        &state,
        &EpisodeStatus::Running,
        &counters,
        &codec_v5(),
        &id,
    )
    .unwrap();
    let terminal = calculate_checkpoint_digest_v5(
        &state,
        &EpisodeStatus::Terminal {
            reason: TerminalReason::RulesLoss,
            players: vec![PlayerOutcome {
                player: mtgml_model::PlayerId(0),
                result: PlayerResult::Loss,
            }],
        },
        &counters,
        &codec_v5(),
        &id,
    )
    .unwrap();
    assert_ne!(running, terminal);
}

fn kat_fixture_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../persistence/golden/checkpoint-digest-v5-kat.v1.json")
}

fn kat_vectors() -> Vec<serde_json::Value> {
    let raw = std::fs::read(kat_fixture_path()).unwrap();
    let document: serde_json::Value = serde_json::from_slice(&raw).unwrap();
    document["vectors"].as_array().cloned().unwrap()
}

/// Spec §19.3 shared KAT: the Rust and Python suites must read the same
/// `persistence/golden/checkpoint-digest-v5-kat.v1.json` fixture and
/// reproduce every frozen `expected_digest`.
#[test]
fn shared_kat_vectors_reproduce_frozen_digests() {
    let vectors = kat_vectors();
    assert!(
        vectors.len() >= 3,
        "shared checkpoint-digest-v5 KAT fixture regressed"
    );

    let counters = EnvironmentLimitCounters::default();
    for vector in &vectors {
        let case = vector["case"].as_str().unwrap();
        let program_kind = vector["program_kind"].as_str().unwrap();
        let program = match program_kind {
            "synthetic_rules_compat" => ExecutionProgramV1::SyntheticRulesCompat,
            "magic_rules" => ExecutionProgramV1::MagicRules,
            other => panic!("unknown program_kind {other} in KAT case {case}"),
        };
        let contract_hex = vector["semantic_contract_id"].as_str().unwrap();
        let full_state_hex = vector["full_state_digest"].as_str().unwrap();
        let expected = vector["expected_digest"].as_str().unwrap();

        let digest = calculate_checkpoint_digest_v5(
            &FullStateDigestV4::from_digest_bytes(hex_to_bytes(full_state_hex))
                .as_digest_reference(),
            &EpisodeStatus::Running,
            &counters,
            &codec_v5(),
            &ExecutionIdentityV1 {
                program_kind: program,
                semantic_contract_id: SemanticContractIdV1::parse(contract_hex).unwrap(),
            },
        )
        .unwrap();
        assert_eq!(digest.as_str(), expected, "KAT case {case} drifted");
    }
}

fn hex_to_bytes(hex: &str) -> [u8; 32] {
    let bytes = decode_hex(hex).unwrap();
    assert_eq!(bytes.len(), 32);
    let mut result = [0u8; 32];
    result.copy_from_slice(&bytes);
    result
}

fn decode_hex(hex: &str) -> Result<Vec<u8>, String> {
    if hex.len() % 2 != 0 {
        return Err("odd-length hex string".to_owned());
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| {
            let hi = hex_digit(hex.as_bytes()[i])?;
            let lo = hex_digit(hex.as_bytes()[i + 1])?;
            Ok(hi * 16 + lo)
        })
        .collect()
}

fn hex_digit(byte: u8) -> Result<u8, String> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(format!("invalid hex digit: {}", byte as char)),
    }
}
