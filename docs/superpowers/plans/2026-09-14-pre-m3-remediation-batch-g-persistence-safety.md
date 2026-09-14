# Pre-M3 Remediation Batch G: Persistence and Safety Implementation Plan

**Status:** implementation plan pending independent review

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox syntax for tracking. The user has explicitly prohibited subagents for this work.

**Goal:** Close the confirmed Batch-G persistence, RNG, generation, diagnostic, and helper defects while preserving V3 bytes, historical meanings, valid RNG output, public wire contracts, and M3_AUTHORIZED = NO.

**Architecture:** Keep every fix at its existing local owner. mtgml-persistence closes only detached checkpoint-digest input identity and local counter validity; mtgml-environment remains the complete in-memory checkpoint owner. The other changes remain isolated to the CBOR decoder, RNG raw-lane helper, contract generator, conformance renderer, and Commander helper. No universal validation, serialization, diagnostic, index, or Commander framework is introduced.

**Tech Stack:** Rust 1.85.1, Cargo locked workspace, Python 3.13.15 in .venv, pytest/unittest, PowerShell, canonical CBOR/envelope V3 identity, GitHub Actions.

---

## Execution gates

The approved design is committed at aa3d6e595f2d6491b1f5422f4933fd0d6eb61704.
The semantic Batch-G base remains 04a4831f4fd6e35aa5b6ac315e641b7af238fe9c.
The first implementation commit is forbidden until this plan receives an
independent review and the user changes:

~~~text
PLAN_AUTHORIZED = YES
PRODUCTION_IMPLEMENTATION_AUTHORIZED = YES
~~~

Until then, only this plan and its documentation metadata may change.

All RED test commits are test-only commits. Each RED command must fail for the
named behavioral reason, not because of a compile error, malformed fixture,
OOM, or test harness panic. Each GREEN step must run the narrow test and the
affected package suite before its fix commit.

## File ownership map

| File | Plan responsibility |
|---|---|
| docs/superpowers/specs/2026-09-14-pre-m3-remediation-batch-g-persistence-safety-design.md | Preserve the approved design and update only its approved status metadata. |
| docs/normative-document-register.v1.json | Register the final Batch-G evidence document before it is created. |
| crates/mtgml-persistence/src/checkpoint_digest.rs | Validate exact detached V3 digest-reference identity and local counter validity; hide the payload builder. |
| crates/mtgml-persistence/src/cbor.rs | Apply ADR-0040 array-limit-before-depth precedence. |
| crates/mtgml-persistence/src/tests.rs | RED/GREEN direct calculator and compound-CBOR evidence. |
| crates/mtgml-environment/src/checkpoint.rs | Remove top-level raw Serde and preserve high-level counter error ownership. |
| crates/mtgml-environment/src/tests.rs | Include the Batch-G environment characterization module. |
| crates/mtgml-environment/src/tests/batch_g.rs | Guard the top-level checkpoint serialization boundary. |
| crates/mtgml-random/src/hmac_counter.rs | Make raw-lane access checked and non-public; preserve valid extraction. |
| crates/mtgml-random/src/seed.rs | Add the typed internal InvalidRawLane error. |
| crates/mtgml-conformance/src/diagnostics.rs | Replace generic Debug rendering with bounded safe summaries. |
| crates/mtgml-conformance/src/lib.rs | Add the secret-sentinel diagnostic regression. |
| crates/mtgml-commander/src/lib.rs | Separate designation membership from cast-count lookup and add helper tests. |
| python/src/mtgml/persistence.py | Mirror local counter and codec-input rejection in the mechanical calculator. |
| python/tests/test_persistence_codec.py | Add Python persistence input and precedence parity tests. |
| python/tests/test_batch_g.py | Test generator raw-byte helpers and LF/CRLF behavior. |
| scripts/generate_contracts.py | Write and compare exact UTF-8 LF bytes. |
| docs/superpowers/specs/2026-09-14-pre-m3-remediation-batch-g-dispositions-and-evidence.md | Record final dispositions, RED/GREEN commits, integration rows, and gate statuses. |

Generated files are regenerated only by scripts/generate_contracts.py; no
hand-edited generated output is permitted.

## Task 1: Reconfirm the exact implementation starting state

**Files:** None.

- [ ] **Step 1: Fetch the current remote refs and verify the approved branch head.**

Run in PowerShell from the repository root:

~~~powershell
git fetch origin
if ((git rev-parse origin/master) -ne "04a4831f4fd6e35aa5b6ac315e641b7af238fe9c") { throw "origin/master moved; stop and re-review the approved design base" }
$approvedDesign = "aa3d6e595f2d6491b1f5422f4933fd0d6eb61704"
git merge-base --is-ancestor $approvedDesign (git rev-parse HEAD)
if ($LASTEXITCODE -ne 0) { throw "current head is not descended from the approved design head" }
$planningChanges = @(git diff --name-only "$approvedDesign..HEAD")
$allowedPlanningChanges = @(
  "docs/superpowers/specs/2026-09-14-pre-m3-remediation-batch-g-persistence-safety-design.md",
  "docs/superpowers/plans/2026-09-14-pre-m3-remediation-batch-g-persistence-safety.md",
  "docs/normative-document-register.v1.json"
)
if (@($planningChanges | Where-Object { $_ -notin $allowedPlanningChanges })) { throw "production changes exist before implementation authorization" }
if ((git branch --show-current) -ne "chris/pre-m3-remediation-batch-g-persistence-safety-hardening") { throw "wrong Batch-G branch" }
if (git status --porcelain=v1) { throw "worktree must be clean before implementation authorization" }
~~~

Expected: every guard succeeds and the worktree is clean. If any guard fails,
do not edit production code or silently rebase.

- [ ] **Step 2: Run the unchanged focused baseline.**

~~~powershell
cargo test -p mtgml-persistence --locked
cargo test -p mtgml-random --locked
cargo test -p mtgml-environment --locked
cargo test -p mtgml-conformance --locked
cargo test -p mtgml-commander --locked
.venv\Scripts\python.exe -m pytest python/tests/test_persistence_codec.py python/tests/test_failure_packet.py python/tests/test_v022_maintainer_ergonomics.py -q
~~~

Expected: all commands exit 0; no Batch-G source file is changed. Record the
actual test counts in the evidence document rather than copying historical
baseline counts.

## Task 2: RED and GREEN for FND-017A detached checkpoint-digest input closure

**Files:**

- Modify: crates/mtgml-persistence/src/tests.rs
- Modify: crates/mtgml-persistence/src/checkpoint_digest.rs
- Modify: crates/mtgml-environment/src/checkpoint.rs
- Modify: crates/mtgml-environment/src/tests.rs
- Create: crates/mtgml-environment/src/tests/batch_g.rs
- Modify: python/src/mtgml/persistence.py
- Modify: python/tests/test_persistence_codec.py

### RED

- [ ] **Step 1: Add Rust helpers and failing tests for exact V3 reference identity, impossible counters, and payload visibility.**

Add this test-only code beside the existing checkpoint known-answer test in
crates/mtgml-persistence/src/tests.rs:

~~~rust
fn valid_full_state_reference() -> mtgml_model::DigestReferenceV1 {
    mtgml_model::DigestReferenceV1 {
        envelope_version: envelope::DIGEST_ENVELOPE_ID.to_owned(),
        algorithm_id: envelope::SHA256_ID.to_owned(),
        semantic_domain: "mtgml.full-state-digest.v3".to_owned(),
        payload_codec_id: envelope::CANONICAL_CBOR_ID.to_owned(),
        input_schema_id: "full-state-digest-input.v3".to_owned(),
        digest_bytes: [7; 32],
    }
}

fn valid_checkpoint_codec() -> CheckpointCodecIdentity {
    CheckpointCodecIdentity {
        codec_id: "in-memory-reference".to_owned(),
        semantic_version: "3".to_owned(),
    }
}

#[test]
fn fnd_017a_rejects_non_v3_full_state_reference_identity() {
    let cases: [(&str, fn(&mut mtgml_model::DigestReferenceV1)); 5] = [
        ("envelope", |reference| reference.envelope_version = "other".into()),
        ("algorithm", |reference| reference.algorithm_id = "sha-512".into()),
        ("domain", |reference| reference.semantic_domain = "other-domain".into()),
        ("codec", |reference| reference.payload_codec_id = "other-codec".into()),
        ("schema", |reference| reference.input_schema_id = "other-schema".into()),
    ];
    for (label, mutate) in cases {
        let mut reference = valid_full_state_reference();
        mutate(&mut reference);
        assert_eq!(
            checkpoint_digest::calculate_checkpoint_digest_v3(
                &reference,
                &EpisodeStatus::Running,
                &EnvironmentLimitCounters::default(),
                &valid_checkpoint_codec(),
            ),
            Err(PersistenceDecodeErrorV1::SemanticValidation),
            "invalid reference field: {label}"
        );
    }
}

#[test]
fn fnd_017a_rejects_impossible_checkpoint_counters() {
    let counters = EnvironmentLimitCounters {
        accepted_transitions: 1,
        decisions_submitted: 0,
        ..EnvironmentLimitCounters::default()
    };
    assert_eq!(
        checkpoint_digest::calculate_checkpoint_digest_v3(
            &valid_full_state_reference(),
            &EpisodeStatus::Running,
            &counters,
            &valid_checkpoint_codec(),
        ),
        Err(PersistenceDecodeErrorV1::SemanticValidation)
    );
}

#[test]
fn fnd_017a_checkpoint_payload_is_not_a_public_function() {
    let source = include_str!("checkpoint_digest.rs");
    assert!(!source.contains("pub fn checkpoint_payload"));
}
~~~

Run the exact RED tests:

~~~powershell
cargo test -p mtgml-persistence --locked fnd_017a_rejects_non_v3_full_state_reference_identity
cargo test -p mtgml-persistence --locked fnd_017a_rejects_impossible_checkpoint_counters
cargo test -p mtgml-persistence --locked fnd_017a_checkpoint_payload_is_not_a_public_function
~~~

Expected on the unfixed base: the first two tests observe Ok(CheckpointDigestV3)
instead of Err(SemanticValidation), and the third observes the public function
declaration. Leave these Rust RED tests uncommitted until the independent
Python RED is added in the next step.

- [ ] **Step 2: Add Python RED parity for empty codec identity and impossible counters.**

Add this test to python/tests/test_persistence_codec.py:

~~~python
def test_fnd_017a_rejects_local_checkpoint_identity_inputs(self) -> None:
    valid_counters = {
        "decisions_submitted": 0,
        "accepted_transitions": 0,
        "rule_events_emitted": 0,
        "resource_units_consumed": 0,
        "wall_clock_elapsed_millis": 0,
    }
    for codec_id, semantic_version, expected_label in (
        ("", "3", "empty codec id"),
        ("in-memory-reference", "", "empty semantic version"),
    ):
        with self.subTest(expected_label=expected_label):
            with self.assertRaises(PersistenceError) as caught:
                calculate_checkpoint_digest_v3(
                    "07" * 32,
                    EpisodeStatus.running(),
                    valid_counters,
                    codec_id,
                    semantic_version,
                )
            self.assertEqual(caught.exception.code, "semantic_validation")

    invalid_counters = {
        **valid_counters,
        "accepted_transitions": 1,
    }
    with self.assertRaises(PersistenceError) as caught:
        calculate_checkpoint_digest_v3(
            "07" * 32,
            EpisodeStatus.running(),
            invalid_counters,
            "in-memory-reference",
            "3",
        )
    self.assertEqual(caught.exception.code, "semantic_validation")
~~~

Run:

~~~powershell
.venv\Scripts\python.exe -m pytest python/tests/test_persistence_codec.py -k fnd_017a -q
~~~

Expected on the unfixed base: the empty codec calls and impossible counter call
return a digest instead of raising semantic_validation. Run the Rust RED tests
again, then commit both language RED tests together:

~~~powershell
cargo test -p mtgml-persistence --locked fnd_017a_rejects_non_v3_full_state_reference_identity
cargo test -p mtgml-persistence --locked fnd_017a_rejects_impossible_checkpoint_counters
cargo test -p mtgml-persistence --locked fnd_017a_checkpoint_payload_is_not_a_public_function
git add crates/mtgml-persistence/src/tests.rs python/tests/test_persistence_codec.py
git commit -m "test: characterize Batch-G checkpoint input closure"
~~~

### GREEN

- [ ] **Step 3: Add one private Rust validator for the detached V3 reference and local inputs.**

In crates/mtgml-persistence/src/checkpoint_digest.rs, add the exact local
validator before calculate_checkpoint_digest_v3:

~~~rust
fn validate_full_state_reference(
    reference: &DigestReferenceV1,
) -> Result<(), PersistenceDecodeErrorV1> {
    if reference.envelope_version != envelope::DIGEST_ENVELOPE_ID
        || reference.algorithm_id != envelope::SHA256_ID
        || reference.semantic_domain != FullStateDigestV3::DOMAIN
        || reference.payload_codec_id != envelope::CANONICAL_CBOR_ID
        || reference.input_schema_id != "full-state-digest-input.v3"
    {
        return Err(PersistenceDecodeErrorV1::SemanticValidation);
    }
    Ok(())
}

fn validate_checkpoint_digest_inputs(
    full_state_digest: &DigestReferenceV1,
    status: &EpisodeStatus,
    counters: &EnvironmentLimitCounters,
    codec: &CheckpointCodecIdentity,
) -> Result<(), PersistenceDecodeErrorV1> {
    validate_full_state_reference(full_state_digest)?;
    status
        .validate()
        .map_err(|_| PersistenceDecodeErrorV1::SemanticValidation)?;
    counters
        .validate()
        .map_err(|_| PersistenceDecodeErrorV1::SemanticValidation)?;
    if codec.codec_id.is_empty() || codec.semantic_version.is_empty() {
        return Err(PersistenceDecodeErrorV1::SemanticValidation);
    }
    Ok(())
}
~~~

Import FullStateDigestV3 from mtgml_model. Call the validator once at the
start of calculate_checkpoint_digest_v3, remove duplicate status/codec checks
from that function, and change pub fn checkpoint_payload to private
fn checkpoint_payload. Do not add a state parameter and do not reject
noncanonical outcome order in this low-level helper; the existing defensive
sort and high-level ordering owner are intentional.

- [ ] **Step 4: Preserve high-level counter error ownership.**

In EnvironmentCheckpointV3::new, validate limit_counters after calculating the
state digest and before calling the persistence calculator:

~~~rust
limit_counters
    .validate()
    .map_err(|_| CheckpointValidationError::LimitCounters)?;
let checkpoint_digest = calculate_checkpoint_digest(
    &state_digest,
    &status,
    &limit_counters,
    &codec,
)?;
~~~

In EnvironmentCheckpointV3::validate, move the existing
self.limit_counters.validate() block before checkpoint-digest calculation and
remove the later duplicate. This keeps impossible counters in the high-level
LimitCounters error class while direct persistence callers receive
SemanticValidation.

- [ ] **Step 5: Mirror local Python input validation without adding state authority.**

In python/src/mtgml/persistence.py, add these checks in
calculate_checkpoint_digest_v3 after the digest reference is constructed and
before payload construction:

~~~python
if not isinstance(codec_id, str) or not codec_id:
    raise _error("semantic_validation", "codec_id must be non-empty")
if not isinstance(semantic_version, str) or not semantic_version:
    raise _error("semantic_validation", "semantic_version must be non-empty")
if counter_values[1] > counter_values[0]:
    raise _error(
        "semantic_validation",
        "accepted transitions exceed submitted decisions",
    )
~~~

Keep Python's existing status duplicate validation and defensive outcome sort.
Do not add EngineState, player-universe, or replay execution logic to Python.

- [ ] **Step 6: Run focused GREEN tests and commit the fix.**

~~~powershell
cargo test -p mtgml-persistence --locked fnd_017a -- --nocapture
cargo test -p mtgml-environment --locked checkpoint_v3_validation_and_restore_nonmutation_matrix
.venv\Scripts\python.exe -m pytest python/tests/test_persistence_codec.py -k fnd_017a -q
~~~

Expected: every named test passes; the existing
checkpoint_digest_v3_known_answer and FND-025 defensive-sort tests still pass.
Commit:

~~~powershell
git add crates/mtgml-persistence/src/checkpoint_digest.rs crates/mtgml-persistence/src/tests.rs crates/mtgml-environment/src/checkpoint.rs python/src/mtgml/persistence.py python/tests/test_persistence_codec.py
git commit -m "fix: close Batch-G checkpoint identity input boundary"
~~~

- [ ] **Step 7: Record FND-017B as resolved on the existing high-level owners.**

This is evidence-only. It does not change production code. Create the
Batch-G environment test module and include it from tests.rs:

~~~rust
include!("tests/batch_g.rs");
~~~

Add this characterization test to crates/mtgml-environment/src/tests/batch_g.rs:

~~~rust
#[test]
fn fnd_017b_closed_status_with_pending_decision_is_rejected_at_checkpoint_owner() {
    let controller = environment_at_members_stage();
    let checkpoint = controller.checkpoint().unwrap();
    assert!(checkpoint.state.execution.pending_decision.is_some());
    let status = EpisodeStatus::Terminal {
        reason: TerminalReason::Concession,
        players: vec![
            PlayerOutcome {
                player: PlayerId(1),
                result: PlayerResult::Loss,
            },
            PlayerOutcome {
                player: PlayerId(2),
                result: PlayerResult::Win,
            },
        ],
    };
    assert!(
        EnvironmentCheckpointV3::new(
            checkpoint.state.clone(),
            status,
            checkpoint.limit_counters.clone(),
            checkpoint.codec.clone(),
        )
        .is_err()
    );
}
~~~

Run the named existing boundary evidence and the new pending-decision check:

~~~powershell
cargo test -p mtgml-environment --locked batch_d_invalid_ordered_state_cannot_construct_checkpoint
cargo test -p mtgml-environment --locked checkpoint_identity_tampering_is_rejected
cargo test -p mtgml-environment --locked closed_status_player_outcomes_require_authoritative_player_universe
cargo test -p mtgml-environment --locked fnd_025_checkpoint_rejects_noncanonical_status_order
cargo test -p mtgml-environment --locked fnd_017b_closed_status_with_pending_decision_is_rejected_at_checkpoint_owner
cargo test -p mtgml-replay --locked fnd_022b_manifest_requires_the_exact_deck_player_universe_for_closed_status
cargo test -p mtgml-replay --locked fnd_025_manifest_rejects_noncanonical_deck_and_status_order
cargo test -p mtgml-replay --locked replay_v3_rejects_corrupt_accepted_progression
~~~

Expected: every named test runs and passes on the current high-level owners.
Record FND-017B as RESOLVED_ON_BASE; do not add a production fix. Commit this
characterization evidence separately:

~~~powershell
git add crates/mtgml-environment/src/tests.rs crates/mtgml-environment/src/tests/batch_g.rs
git commit -m "test: record Batch-G resolved checkpoint boundary"
~~~

## Task 3: RED and GREEN for FND-018 top-level checkpoint raw Serde narrowing

**Files:**

- Modify: crates/mtgml-environment/src/tests.rs
- Modify: crates/mtgml-environment/src/tests/batch_g.rs
- Modify: crates/mtgml-environment/src/checkpoint.rs

- [ ] **Step 1: Add the failing source-surface guard to the existing Batch-G module.**

Append this test to the existing crates/mtgml-environment/src/tests/batch_g.rs:

~~~rust
#[test]
fn fnd_018_environment_checkpoint_is_not_a_raw_serde_surface() {
    let source = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/checkpoint.rs"));
    assert!(!source.contains("Serialize"));
    assert!(!source.contains("Deserialize"));
}
~~~

Run:

~~~powershell
cargo test -p mtgml-environment --locked fnd_018_environment_checkpoint_is_not_a_raw_serde_surface
~~~

Expected on the base: FAIL because EnvironmentCheckpointV3 derives both traits.
Commit the test-only RED change:

~~~powershell
git add crates/mtgml-environment/src/tests.rs crates/mtgml-environment/src/tests/batch_g.rs
git commit -m "test: characterize Batch-G checkpoint serialization surface"
~~~

- [ ] **Step 2: Remove only the top-level derives.**

In crates/mtgml-environment/src/checkpoint.rs, change:

~~~rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
~~~

to:

~~~rust
#[derive(Debug, Clone, PartialEq, Eq)]
~~~

Remove the top-level #[serde(deny_unknown_fields)] attribute together with the
Serde import and derives. Do not remove Serde from EngineState, nested state components, model values,
replay DTOs, or internal test fixtures.

- [ ] **Step 3: Run GREEN environment checkpoint and replay tests.**

~~~powershell
cargo test -p mtgml-environment --locked fnd_018_environment_checkpoint_is_not_a_raw_serde_surface
cargo test -p mtgml-environment --locked checkpoint_v3_validation_and_restore_nonmutation_matrix
cargo test -p mtgml-environment --locked checkpoint_restore_repeats_exact_transition_and_replay_segment
cargo test -p mtgml-replay --locked
~~~

Expected: all commands exit 0; in-memory checkpoint, restore, fork, and replay
behavior remains unchanged. Commit:

~~~powershell
git add crates/mtgml-environment/src/checkpoint.rs
git commit -m "fix: narrow Batch-G checkpoint serialization surface"
~~~

## Task 4: RED and GREEN for FND-019 ADR-0040 precedence parity

**Files:**

- Modify: crates/mtgml-persistence/src/tests.rs
- Modify: crates/mtgml-persistence/src/cbor.rs
- Modify: python/tests/test_persistence_codec.py

- [ ] **Step 1: Add the nested array/depth compound case to Rust and Python tests.**

Add this Rust test to crates/mtgml-persistence/src/tests.rs:

~~~rust
#[test]
fn fnd_019_array_limit_precedes_depth_limit() {
    let mut bytes = vec![0x81; cbor::MAX_DEPTH];
    bytes.extend([0x9a, 0x00, 0x10, 0x00, 0x01]);
    assert_eq!(
        cbor::decode_canonical(&bytes),
        Err(PersistenceDecodeErrorV1::ArrayTooLarge)
    );
}
~~~

Add this Python test to python/tests/test_persistence_codec.py:

~~~python
def test_fnd_019_array_limit_precedes_depth_limit(self) -> None:
    from mtgml.persistence import MAX_DEPTH

    payload = (b"\x81" * MAX_DEPTH) + b"\x9a\x00\x10\x00\x01"
    with self.assertRaises(PersistenceError) as caught:
        decode_canonical(payload)
    self.assertEqual(caught.exception.code, "array_too_large")
~~~

- [ ] **Step 2: Run the matrix tests to prove the Rust/Python RED divergence.**

~~~powershell
cargo test -p mtgml-persistence --locked fnd_019_array_limit_precedes_depth_limit
.venv\Scripts\python.exe -m pytest python/tests/test_persistence_codec.py -k fnd_019 -q
~~~

Expected on the base: Rust returns depth_exceeded and the Python test passes
with array_too_large. Record the Rust test-only RED commit together with the
Python matrix test:

~~~powershell
git add crates/mtgml-persistence/src/tests.rs python/tests/test_persistence_codec.py
git commit -m "test: pin Batch-G persistence precedence parity"
~~~

- [ ] **Step 3: Reorder only the Rust array decoder checks.**

In crates/mtgml-persistence/src/cbor.rs, change the major == 4 arm from:

~~~rust
4 => {
    if depth >= MAX_DEPTH {
        return Err(PersistenceDecodeErrorV1::DepthExceeded);
    }
    let length = checked_length(
        argument,
        MAX_ARRAY_ELEMENTS,
        PersistenceDecodeErrorV1::ArrayTooLarge,
    )?;
~~~

to:

~~~rust
4 => {
    let length = checked_length(
        argument,
        MAX_ARRAY_ELEMENTS,
        PersistenceDecodeErrorV1::ArrayTooLarge,
    )?;
    if depth >= MAX_DEPTH {
        return Err(PersistenceDecodeErrorV1::DepthExceeded);
    }
~~~

Keep the item-limit check, allocation, decoder limits, and all other error
ordering unchanged.

- [ ] **Step 4: Run all persistence precedence evidence and commit.**

~~~powershell
cargo test -p mtgml-persistence --locked
.venv\Scripts\python.exe -m pytest python/tests/test_persistence_codec.py -q
~~~

Expected: Rust and Python agree on the committed negative corpus and the new
compound case; valid persistence known answers remain unchanged. Commit:

~~~powershell
git add crates/mtgml-persistence/src/cbor.rs
git commit -m "fix: align Batch-G persistence error precedence"
~~~

## Task 5: RED and GREEN for FND-029 checked internal RNG lane access

**Files:**

- Modify: crates/mtgml-random/src/hmac_counter.rs
- Modify: crates/mtgml-random/src/seed.rs

- [ ] **Step 1: Add a test that proves the public-era helper can panic.**

Add this test inside the existing hmac_counter test module:

~~~rust
#[test]
fn fnd_029_invalid_raw_lane_does_not_panic() {
    let result = std::panic::catch_unwind(|| raw_u64_at(&[0u8; 32], 4));
    assert!(result.is_ok(), "invalid raw lane must fail closed");
}
~~~

Run:

~~~powershell
cargo test -p mtgml-random --locked fnd_029_invalid_raw_lane_does_not_panic
~~~

Expected on the base: FAIL because debug builds hit the debug_assert and panic.
Commit the test-only RED change:

~~~powershell
git add crates/mtgml-random/src/hmac_counter.rs
git commit -m "test: characterize Batch-G invalid RNG lane"
~~~

- [ ] **Step 2: Add the typed invalid-lane error.**

In crates/mtgml-random/src/seed.rs, add this variant after InvalidRandomBound:

~~~rust
#[error("raw RNG lane is outside the four-lane block")]
InvalidRawLane,
~~~

- [ ] **Step 3: Replace direct indexing with checked internal extraction.**

In crates/mtgml-random/src/hmac_counter.rs, replace the public helper with:

~~~rust
fn raw_u64_at(
    block: &[u8; 32],
    lane: usize,
) -> Result<u64, RandomValidationError> {
    let offset = lane
        .checked_mul(8)
        .ok_or(RandomValidationError::InvalidRawLane)?;
    let end = offset
        .checked_add(8)
        .ok_or(RandomValidationError::InvalidRawLane)?;
    let bytes = block
        .get(offset..end)
        .ok_or(RandomValidationError::InvalidRawLane)?;
    let word: [u8; 8] = bytes
        .try_into()
        .map_err(|_| RandomValidationError::InvalidRawLane)?;
    Ok(u64::from_be_bytes(word))
}
~~~

Change next_raw_u64 from:

~~~rust
let value = raw_u64_at(&block, lane);
~~~

to:

~~~rust
let value = raw_u64_at(&block, lane)?;
~~~

Update the existing valid-lane KAT assertions to call unwrap on the checked
helper. Add the final assertion to the invalid-lane test:

~~~rust
let result = std::panic::catch_unwind(|| raw_u64_at(&[0u8; 32], 4));
assert!(result.is_ok());
assert_eq!(result.unwrap(), Err(RandomValidationError::InvalidRawLane));
~~~

- [ ] **Step 4: Run lane, KAT, and cursor evidence.**

~~~powershell
cargo test -p mtgml-random --locked fnd_029_invalid_raw_lane_does_not_panic
cargo test -p mtgml-random --locked hmac_counter::tests::raw_words_0_to_7_kat
cargo test -p mtgml-random --locked hmac_counter::tests::cursor_boundary_kat
cargo test -p mtgml-random --locked
~~~

Expected: invalid lane returns InvalidRawLane without panic; all four valid
lanes, valid raw words, cursor boundaries, sampling, shuffle, and the full
random crate suite pass with unchanged valid outputs. Commit:

~~~powershell
git add crates/mtgml-random/src/hmac_counter.rs crates/mtgml-random/src/seed.rs
git commit -m "fix: make Batch-G RNG lane access fail closed"
~~~

## Task 6: RED and GREEN for FND-030 generated-artifact raw-byte determinism

**Files:**

- Modify: scripts/generate_contracts.py
- Create: python/tests/test_batch_g.py

- [ ] **Step 1: Add the Windows raw-text characterization.**

Create python/tests/test_batch_g.py with this test first:

~~~python
from __future__ import annotations

import tempfile
import unittest
from pathlib import Path


class GeneratedArtifactByteTests(unittest.TestCase):
    def test_host_text_writer_is_not_an_acceptable_generated_writer(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / "generated.txt"
            target.write_text("one\ntwo\n", encoding="utf-8")
            self.assertEqual(target.read_bytes(), b"one\ntwo\n")


if __name__ == "__main__":
    unittest.main()
~~~

Run:

~~~powershell
.venv\Scripts\python.exe -m pytest python/tests/test_batch_g.py -q
~~~

Expected on Windows: FAIL because Path.write_text translates LF to CRLF.
Commit the test-only RED characterization:

~~~powershell
git add python/tests/test_batch_g.py
git commit -m "test: characterize Batch-G generated artifact line endings"
~~~

- [ ] **Step 2: Add exact-byte generator helpers and tests.**

Import the generator through sys.path.insert(0, str(ROOT / "scripts")) and add
these tests to python/tests/test_batch_g.py:

~~~python
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
import generate_contracts  # noqa: E402


class GeneratedArtifactByteTests(unittest.TestCase):
    def test_write_generated_emits_exact_lf_utf8_bytes(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / "generated.txt"
            generate_contracts.write_generated(target, "one\ntwo\n")
            self.assertEqual(target.read_bytes(), b"one\ntwo\n")

    def test_raw_byte_check_rejects_crlf_target(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / "generated.txt"
            target.write_bytes(b"one\r\ntwo\r\n")
            self.assertFalse(generate_contracts.generated_bytes_match(target, "one\ntwo\n"))

    def test_raw_byte_check_accepts_exact_lf_target(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / "generated.txt"
            target.write_bytes(b"one\ntwo\n")
            self.assertTrue(generate_contracts.generated_bytes_match(target, "one\ntwo\n"))
~~~

Keep the unittest class name from the RED test and replace the RED writer
assertion with these focused behavior tests after the production helper exists.

- [ ] **Step 3: Implement exact-byte generation and checking.**

Add these helpers to scripts/generate_contracts.py before main:

~~~python
def generated_bytes(content: str) -> bytes:
    return content.encode("utf-8")


def write_generated(target: Path, content: str) -> None:
    target.write_bytes(generated_bytes(content))


def generated_bytes_match(target: Path, content: str) -> bool:
    return target.is_file() and target.read_bytes() == generated_bytes(content)
~~~

In main, replace the normalized text comparison:

~~~python
if not target.is_file() or target.read_text(encoding="utf-8") != content:
~~~

with:

~~~python
if not generated_bytes_match(target, content):
~~~

Replace the write operation:

~~~python
target.write_text(content, encoding="utf-8")
~~~

with:

~~~python
write_generated(target, content)
~~~

- [ ] **Step 4: Run generator, drift, and raw-byte tests.**

~~~powershell
.venv\Scripts\python.exe -m pytest python/tests/test_batch_g.py -q
.venv\Scripts\python.exe scripts/generate_contracts.py
.venv\Scripts\python.exe scripts/generate_contracts.py --check
~~~

Expected: all Python tests pass; generation reports PASS; raw-byte check
reports PASS: contract vocabulary matches catalog; generated semantic text does
not change. Inspect with:

~~~powershell
git diff --check
git diff --stat -- crates/mtgml-model/src/generated_contract_vocab.rs python/src/mtgml/_generated_contract_vocab.py schemas/episode-status.v1.schema.json schemas/observed-event-envelope.v1.schema.json docs/generated/CONTRACT_VOCABULARY.md
git ls-files --eol -- crates/mtgml-model/src/generated_contract_vocab.rs python/src/mtgml/_generated_contract_vocab.py schemas/episode-status.v1.schema.json schemas/observed-event-envelope.v1.schema.json docs/generated/CONTRACT_VOCABULARY.md
~~~

Expected: generated worktree files report LF and no semantic generated diff.
Commit:

~~~powershell
git add scripts/generate_contracts.py python/tests/test_batch_g.py crates/mtgml-model/src/generated_contract_vocab.rs python/src/mtgml/_generated_contract_vocab.py schemas/episode-status.v1.schema.json schemas/observed-event-envelope.v1.schema.json docs/generated/CONTRACT_VOCABULARY.md
git commit -m "fix: make Batch-G generated artifacts byte deterministic"
~~~

## Task 7: RED and GREEN for FND-031 safe default conformance summaries

**Files:**

- Modify: crates/mtgml-conformance/src/lib.rs
- Modify: crates/mtgml-conformance/src/diagnostics.rs

- [ ] **Step 1: Add a secret-bearing Debug regression.**

Add this type and test inside the existing cfg(test) mod tests in
crates/mtgml-conformance/src/lib.rs:

~~~rust
#[derive(PartialEq)]
struct SecretDiagnosticValue(&'static str);

impl std::fmt::Debug for SecretDiagnosticValue {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "SECRET_DIAGNOSTIC_SENTINEL({})", self.0)
    }
}

#[test]
fn fnd_031_default_difference_does_not_render_debug_values() {
    let difference = crate::diagnostics::compare_value(
        crate::diagnostics::ConformanceFailureClass::StateDigest,
        "transition.secret",
        &SecretDiagnosticValue("root-seed"),
        &SecretDiagnosticValue("private-card"),
    )
    .expect("different values must produce a difference");
    let rendered = difference.to_string();
    assert!(rendered.contains("path=transition.secret"));
    assert!(rendered.contains("mismatch_kind=value_changed"));
    assert!(!rendered.contains("SECRET_DIAGNOSTIC_SENTINEL"));
    assert!(!rendered.contains("root-seed"));
    assert!(!rendered.contains("private-card"));
}
~~~

Run:

~~~powershell
cargo test -p mtgml-conformance --locked fnd_031_default_difference_does_not_render_debug_values
~~~

Expected on the base: FAIL because value_difference stores the secret Debug
strings. Commit the test-only RED change:

~~~powershell
git add crates/mtgml-conformance/src/lib.rs
git commit -m "test: characterize Batch-G conformance diagnostic sensitivity"
~~~

- [ ] **Step 2: Replace generic Debug summaries with safe summary tokens.**

In crates/mtgml-conformance/src/diagnostics.rs, change the import:

~~~rust
use std::fmt::{self, Debug};
~~~

to:

~~~rust
use std::fmt;
~~~

Add a summary constructor that receives only safe, fixed tokens:

~~~rust
fn summary_difference(
    surface: ConformanceFailureClass,
    semantic_path: String,
    mismatch_kind: ConformanceMismatchKind,
    expected_summary: &'static str,
    actual_summary: &'static str,
    sequence: Option<SequenceDifference>,
) -> ConformanceDifference {
    ConformanceDifference {
        surface,
        semantic_path,
        mismatch_kind,
        expected_summary: expected_summary.into(),
        actual_summary: actual_summary.into(),
        sequence,
    }
}

fn value_difference(
    surface: ConformanceFailureClass,
    semantic_path: String,
    mismatch_kind: ConformanceMismatchKind,
    sequence: Option<SequenceDifference>,
) -> ConformanceDifference {
    summary_difference(
        surface,
        semantic_path,
        mismatch_kind,
        "<different>",
        "<different>",
        sequence,
    )
}
~~~

Change compare_value to this exact shape so it no longer requires Debug:

~~~rust
pub(crate) fn compare_value<T: PartialEq>(
    surface: ConformanceFailureClass,
    path: &str,
    expected: &T,
    actual: &T,
) -> Option<ConformanceDifference> {
    (expected != actual).then(|| {
        value_difference(
            surface,
            path.to_owned(),
            ConformanceMismatchKind::ValueChanged,
            None,
        )
    })
}
~~~

Change compare_sequence to T: PartialEq. Its differing-entry branch uses
different/different, its expected-entry-missing branch uses present/missing,
and its unexpected-extra-entry branch uses missing/present. Retain the
existing semantic path, first-differing-index, expected length, and actual
length. The two length-mismatch branches use these exact summary pairs:

~~~rust
summary_difference(
    surface,
    format!("{path}[{index}]"),
    ConformanceMismatchKind::ExpectedEntryMissing,
    "<present>",
    "<missing>",
    Some(sequence),
)

summary_difference(
    surface,
    format!("{path}[{index}]"),
    ConformanceMismatchKind::UnexpectedExtraEntry,
    "<missing>",
    "<present>",
    Some(sequence),
)
~~~

Keep compare_player_map's present/missing tokens and
rejected_mutation_difference's unchanged/changed tokens. Do not remove any
authoritative type's Debug derive.

- [ ] **Step 3: Run diagnostic and full conformance tests.**

~~~powershell
cargo test -p mtgml-conformance --locked fnd_031_default_difference_does_not_render_debug_values
cargo test -p mtgml-conformance --locked tests::exact_transition_event_failure_exposes_the_first_event_path
cargo test -p mtgml-conformance --locked tests::exact_transition_same_inputs_produce_the_same_diagnostic
cargo test -p mtgml-conformance --locked
~~~

Expected: the secret sentinel is absent, semantic path/mismatch data remains,
and all existing conformance tests pass. Commit:

~~~powershell
git add crates/mtgml-conformance/src/diagnostics.rs
git commit -m "fix: bound Batch-G conformance diagnostic summaries"
~~~

## Task 8: RED and GREEN for FND-032 designation membership

**Files:**

- Modify: crates/mtgml-commander/src/lib.rs

- [ ] **Step 1: Add the helper behavior matrix as tests.**

Add a test module to crates/mtgml-commander/src/lib.rs:

~~~rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn commander_format(
        designations: Vec<PhysicalCardId>,
        cast_counts: Vec<(PhysicalCardId, u32)>,
    ) -> FormatState {
        FormatState::Commander {
            state: CommanderState {
                designations: BTreeMap::from([(PlayerId(1), designations)]),
                cast_counts: cast_counts.into_iter().collect(),
                damage: BTreeMap::new(),
            },
        }
    }

    #[test]
    fn fnd_032_designated_without_ledger_entry_has_zero_additional_cost() {
        let format = commander_format(vec![PhysicalCardId(7)], vec![]);
        assert_eq!(additional_cast_cost(&format, PhysicalCardId(7)), Ok(0));
    }

    #[test]
    fn fnd_032_designated_with_one_ledger_entry_has_existing_cost() {
        let format = commander_format(vec![PhysicalCardId(7)], vec![(PhysicalCardId(7), 1)]);
        assert_eq!(additional_cast_cost(&format, PhysicalCardId(7)), Ok(2));
    }

    #[test]
    fn fnd_032_ledger_entry_without_designation_is_not_designated() {
        let format = commander_format(vec![], vec![(PhysicalCardId(7), 1)]);
        assert_eq!(
            additional_cast_cost(&format, PhysicalCardId(7)),
            Err(CommanderError::NotDesignated)
        );
    }

    #[test]
    fn fnd_032_missing_designation_is_not_designated() {
        let format = commander_format(vec![PhysicalCardId(8)], vec![]);
        assert_eq!(
            additional_cast_cost(&format, PhysicalCardId(7)),
            Err(CommanderError::NotDesignated)
        );
    }
}
~~~

Run:

~~~powershell
cargo test -p mtgml-commander --locked fnd_032 -- --nocapture
~~~

Expected on the base: the designated/no-ledger test fails with
Err(NotDesignated). Commit the test-only RED change:

~~~powershell
git add crates/mtgml-commander/src/lib.rs
git commit -m "test: characterize Batch-G Commander helper membership"
~~~

- [ ] **Step 2: Check designation membership before reading the ledger.**

Replace the current implementation:

~~~rust
let casts = commander_state(format)?
    .cast_counts
    .get(&commander)
    .copied()
    .ok_or(CommanderError::NotDesignated)?;
Ok(casts.saturating_mul(2))
~~~

with:

~~~rust
let state = commander_state(format)?;
let designated = state
    .designations
    .values()
    .any(|cards| cards.contains(&commander));
if !designated {
    return Err(CommanderError::NotDesignated);
}
let casts = state.cast_counts.get(&commander).copied().unwrap_or(0);
Ok(casts.saturating_mul(2))
~~~

This helper-only change does not modify CommanderState, state validation,
format declarations, or capability registries.

- [ ] **Step 3: Run the Commander and state structural suites.**

~~~powershell
cargo test -p mtgml-commander --locked
cargo test -p mtgml-state --locked valid_commander_structural_references_are_accepted
cargo test -p mtgml-state --locked commander_ledger_must_reference_a_designated_physical_card
~~~

Expected: all commands exit 0; no Commander capability or support claim is
created. Commit:

~~~powershell
git add crates/mtgml-commander/src/lib.rs
git commit -m "fix: separate Batch-G Commander designation from cast counts"
~~~

## Task 9: Cross-finding integration evidence and final disposition record

**Files:**

- Modify: docs/normative-document-register.v1.json
- Create: docs/superpowers/specs/2026-09-14-pre-m3-remediation-batch-g-dispositions-and-evidence.md

- [ ] **Step 1: Register the final evidence document before creating it.**

Add this object to docs/normative-document-register.v1.json:

~~~json
{
  "change_process": "process-pr",
  "owner_role": "maintainer",
  "path": "docs/superpowers/specs/2026-09-14-pre-m3-remediation-batch-g-dispositions-and-evidence.md",
  "role": "process",
  "stability": "provisional"
}
~~~

Validate the JSON before proceeding:

~~~powershell
.venv\Scripts\python.exe -c "import json; json.load(open('docs/normative-document-register.v1.json', encoding='utf-8')); print('PASS: register JSON')"
~~~

Expected: PASS: register JSON.

- [ ] **Step 2: Write the evidence record from actual command output.**

Create the evidence document with these exact sections. Write the measured
HEAD and all statuses only after the corresponding commands have run:

~~~markdown
# Pre-M3 Remediation Batch G: Dispositions and Evidence

**Status:** evidence record

**BASE:** 04a4831f4fd6e35aa5b6ac315e641b7af238fe9c

**HEAD:** the exact final implementation SHA measured after all source changes

**CODE_VERIFICATION_HEAD:** the exact SHA used for code/workspace verification before the evidence-only commit

**FINAL_EVIDENCE_HEAD:** the exact SHA after the evidence-only commit and final documentation/archive rerun

## Dispositions

FND-017A = CONFIRMED -> CLOSED_BY_BATCH_G
FND-017B = RESOLVED_ON_BASE
FND-017 parent = CLOSED
FND-018 = CONFIRMED -> CLOSED_BY_BATCH_G
FND-019 = CONFIRMED -> CLOSED_BY_BATCH_G
FND-029 = CONFIRMED -> CLOSED_BY_BATCH_G
FND-030 = CONFIRMED -> CLOSED_BY_BATCH_G
FND-031 = CONFIRMED -> CLOSED_BY_BATCH_G
FND-032 = CONFIRMED -> CLOSED_BY_BATCH_G

## RED/GREEN commits

Record each test-only RED SHA, failing command/result, fix SHA, and passing command/result.

## Integration matrix

Record rows 1-18 from the approved design as PASS, FAIL, NOT_RUN, or BLOCKED, with the exact command and source SHA for each row.

## Compatibility and scope

RUST_API_CHANGE = YES
RUST_API_CHANGE_CLASS = INTERNAL_EXPERIMENTAL_NARROWING
FROZEN_PUBLIC_API_CHANGE = NO
WIRE_CHANGE = NO
SCHEMA_CHANGE = NO
CHECKPOINT_SCHEMA_VERSION_CHANGE = NO
REPLAY_VERSION_CHANGE = NO
DIGEST_DOMAIN_CHANGE = NO
RNG_ALGORITHM_CHANGE = NO
HISTORICAL_REPLAY_CHANGE = NO
HISTORICAL_CHECKPOINT_MEANING_CHANGE = NO
COMMANDER_SUPPORT_CLAIM = NO
CAPABILITY_REGISTRY_CHANGE = NO
NEW_MAGIC_SEMANTICS = NO
M3_STARTED = NO
M3_AUTHORIZED = NO
FOUNDATION_READY_FOR_M3 = NO
MERGE_PERFORMED = NO
~~~

Do not commit a document containing an unresolved instruction in its HEAD
field; replace it with the measured SHA before the evidence commit.

- [ ] **Step 3: Run the documentation and scope checks.**

~~~powershell
.venv\Scripts\python.exe scripts/check_documentation.py
git diff --check
~~~

Expected: documentation register and links pass; git diff --check is clean.

## Task 10: Focused package verification after all implementation slices

**Files:** None beyond the already completed implementation tasks.

- [ ] **Step 1: Run every affected Rust package suite.**

~~~powershell
cargo test -p mtgml-model --locked
cargo test -p mtgml-state --locked
cargo test -p mtgml-random --locked
cargo test -p mtgml-persistence --locked
cargo test -p mtgml-replay --locked
cargo test -p mtgml-environment --locked
cargo test -p mtgml-wire --locked
cargo test -p mtgml-conformance --locked
cargo test -p mtgml-commander --locked
~~~

Expected: every command exits 0; any unrelated baseline failure is recorded
as FAIL or BLOCKED, not repaired inside Batch G without authorization.

- [ ] **Step 2: Run the required Rust workspace quality gates.**

~~~powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
~~~

Expected: each command exits 0. Record each command and exact final SHA.

- [ ] **Step 3: Run the pinned Python, schema, wire, generator, and maintainer gates.**

~~~powershell
.venv\Scripts\python.exe scripts/verify_repository.py
.venv\Scripts\python.exe scripts/check_rust_source_structure.py
.venv\Scripts\python.exe scripts/check_documentation.py
.venv\Scripts\python.exe scripts/validate_schemas.py
.venv\Scripts\python.exe scripts/validate_maintainer_artifacts.py
.venv\Scripts\python.exe scripts/verify_python_toolchain.py
.venv\Scripts\python.exe scripts/run_python_tests.py --profile full
.venv\Scripts\python.exe scripts/generate_contracts.py --check
~~~

Expected: each command exits 0; all generated and schema representations
remain synchronized.

- [ ] **Step 4: Run the direct Batch-G profiles.**

~~~powershell
.venv\Scripts\python.exe scripts/run_checks.py fast
.venv\Scripts\python.exe scripts/run_checks.py integration
.venv\Scripts\python.exe scripts/run_checks.py certification
~~~

Expected: each profile reports its actual gate statuses. A skipped or missing
tool is NOT_RUN or BLOCKED, never PASS.

- [ ] **Step 5: Attempt the documented just profiles separately.**

~~~powershell
just check-fast
just check
just check-all
just archive-check
~~~

On this Windows host, a missing Bash/WSL wrapper is recorded as BLOCKED per
DEVELOPER_SETUP.md; direct profiles remain separate evidence and do not upgrade
a blocked wrapper result.

- [ ] **Step 6: Run archive reproducibility at the code-verification head.**

~~~powershell
.venv\Scripts\python.exe scripts/verify_archive_reproducibility.py
~~~

Expected: deterministic archive verification passes. Record this source SHA as
CODE_VERIFICATION_HEAD. Task 11 reruns archive verification after the final
evidence commit, because the evidence document itself changes the source tree.

## Task 11: Scope audit and final local handoff

- [ ] **Step 1: Verify the changed-file union against the approved scope.**

Run:

~~~powershell
$base = "04a4831f4fd6e35aa5b6ac315e641b7af238fe9c"
$tracked = @(git diff --name-only "$base..HEAD")
$untracked = @(git ls-files --others --exclude-standard)
$changed = @($tracked + $untracked | Sort-Object -Unique)
$allowed = @(
  "docs/normative-document-register.v1.json",
  "docs/superpowers/specs/2026-09-14-pre-m3-remediation-batch-g-persistence-safety-design.md",
  "docs/superpowers/specs/2026-09-14-pre-m3-remediation-batch-g-dispositions-and-evidence.md",
  "docs/superpowers/plans/2026-09-14-pre-m3-remediation-batch-g-persistence-safety.md",
  "crates/mtgml-persistence/src/checkpoint_digest.rs",
  "crates/mtgml-persistence/src/cbor.rs",
  "crates/mtgml-persistence/src/tests.rs",
  "crates/mtgml-environment/src/checkpoint.rs",
  "crates/mtgml-environment/src/tests.rs",
  "crates/mtgml-environment/src/tests/batch_g.rs",
  "crates/mtgml-random/src/hmac_counter.rs",
  "crates/mtgml-random/src/seed.rs",
  "crates/mtgml-conformance/src/diagnostics.rs",
  "crates/mtgml-conformance/src/lib.rs",
  "crates/mtgml-commander/src/lib.rs",
  "python/src/mtgml/persistence.py",
  "python/tests/test_persistence_codec.py",
  "python/tests/test_batch_g.py",
  "scripts/generate_contracts.py",
  "crates/mtgml-model/src/generated_contract_vocab.rs",
  "python/src/mtgml/_generated_contract_vocab.py",
  "schemas/episode-status.v1.schema.json",
  "schemas/observed-event-envelope.v1.schema.json",
  "docs/generated/CONTRACT_VOCABULARY.md"
)
$unexpected = @($changed | Where-Object { $_ -notin $allowed })
if ($unexpected) { $unexpected; throw "changed-file scope is outside the approved Batch-G set" }
Write-Output "PASS: Batch-G changed-file union is in scope"
~~~

Expected: only approved design/evidence/plan, direct owners, tests, and
generated contract outputs are listed. New card, deck, capability, schema
version, wire fixture, checkpoint fixture, or M3 paths fail this audit.

- [ ] **Step 2: Record final exact-head identities and statuses.**

Update the evidence document with the actual final HEAD, branch, RED/GREEN
commit SHAs, all focused/workspace/direct/wrapper/archive statuses, and the
required final matrix. Re-run git diff --check, then commit only the evidence
record and any required register update:

~~~powershell
git add docs/normative-document-register.v1.json docs/superpowers/specs/2026-09-14-pre-m3-remediation-batch-g-dispositions-and-evidence.md
git commit -m "docs: record Batch-G dispositions and evidence"
~~~

After this documentation-only commit, rerun the change-aware documentation
gate and record the new exact HEAD. Do not reuse pre-commit source evidence
for a post-commit source claim.

- [ ] **Step 3: Re-run final documentation and archive gates on the evidence head.**

Immediately after the evidence commit, record FINAL_EVIDENCE_HEAD and run:

~~~powershell
git rev-parse HEAD
.venv\Scripts\python.exe scripts/check_documentation.py
git diff --check
.venv\Scripts\python.exe scripts/verify_archive_reproducibility.py
just archive-check
~~~

Expected: documentation and direct archive verification pass on the final
evidence head. Record just archive-check as BLOCKED if the documented Bash/WSL
wrapper is unavailable. The final archive result must cite FINAL_EVIDENCE_HEAD,
not CODE_VERIFICATION_HEAD.

## Task 12: Hosted handoff after implementation approval

This task is unavailable until the user approves the implementation plan and
sets PRODUCTION_IMPLEMENTATION_AUTHORIZED = YES.

- [ ] **Step 1: Push exactly the one Batch-G branch.**

~~~powershell
git push --set-upstream origin chris/pre-m3-remediation-batch-g-persistence-safety-hardening
git ls-remote origin refs/heads/chris/pre-m3-remediation-batch-g-persistence-safety-hardening
~~~

Expected: remote branch SHA equals the final local HEAD.

- [ ] **Step 2: Open exactly one PR without merging.**

Use the approved title:

~~~text
Pre-M3 remediation Batch G: persistence and safety hardening
~~~

The PR body must contain BASE, final HEAD, all seven dispositions plus
FND-017A/B, compatibility fields, exact local gate results, direct/wrapper
statuses, and M3_AUTHORIZED = NO. Do not merge.

- [ ] **Step 3: Wait for exact-head hosted checks.**

Verify the actual final PR head, then wait for every required repository check:
manafold-pr-gate, PR Fast, PR Integration, Windows Setup Smoke, CodeQL, and
analyzer checks. Record HOSTED_CI = PASS only when every required exact-head
check succeeds. A prior SHA does not satisfy this step.

## Final handoff state

Before production authorization, the plan itself ends at:

~~~text
PLAN_REVIEW = PENDING
PRODUCTION_IMPLEMENTATION_AUTHORIZED = NO
M3_STARTED = NO
M3_AUTHORIZED = NO
FOUNDATION_READY_FOR_M3 = NO
MERGE_PERFORMED = NO
~~~
