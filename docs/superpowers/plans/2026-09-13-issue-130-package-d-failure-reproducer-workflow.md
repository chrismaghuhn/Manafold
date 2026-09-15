# Issue #130 Package D — Deterministic Failure / Reproducer Workflow Implementation Plan

**Status:** provisional implementation plan; execution requires a separate user decision

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add deterministic first-difference diagnostics and a trusted, repository-private command failure capture/rerun workflow without adding semantic authority, public schemas, or M3 behavior.

**Architecture:** Keep typed comparison in mtgml-conformance and keep packet creation, source identity, checksums, and bounded subprocess orchestration in scripts/. The packet is a small command-level artifact, not the complete ADR-0036 portable reproduction bundle; command.argv and repository-relative command.cwd are the only machine-authoritative rerun inputs.

**Tech Stack:** Rust 1.85.1, Cargo locked workspace, Python 3.13.15 project .venv, Python standard library, existing run_verification.source_tree_fingerprint(), existing source_files() set, pytest, Ruff, Mypy, gh, and GitHub Actions.

---

## Scope and working rules

The implementation starts only after Task 1 revalidates the actual origin/master. The reference Package-C merge is 15f339cd0ca875a9f18a44c743796120ed87ade7; it is evidence, not a revision to force. If origin/master moved, integrate the actual remote master without resetting or rewriting it, and record the new base SHA.

Do not modify the user's dirty checkout at C:\Users\chris\Documents\Manafold. Work only in the isolated worktree C:\Users\chris\.config\superpowers\worktrees\Manafold\issue-130-d-failure-reproducer-workflow.

Do not change pytest, python/requirements-dev.lock, Cargo.lock, cargo-audit, the audit-only Rust toolchain, the Manafold Rust toolchain, dependency policy, branch protection, public schemas, cards, decks, rules semantics, replay/checkpoint semantics, run_checks.py profile membership, or the existing Package-C advisory.

Every production behavior change follows a red-green cycle. Run the smallest focused test after each implementation step, then commit only after the focused and relevant broader checks are green.

## File map

- Modify crates/mtgml-conformance/src/lib.rs to expose the detailed failure variant and apply the deterministic comparison order.
- Create crates/mtgml-conformance/src/diagnostics.rs for repository-private typed differences, sequence/map comparison helpers, and the explicit signature marker.
- Modify crates/mtgml-conformance/src/lifecycle.rs so its generic exact product assertion uses the shared event/digest/status diagnostic helpers.
- Create scripts/failure_packet.py for internal manifest constants, source/tool identity, marker parsing, safe-summary construction, checksum validation, packet validation, and atomic packet writing.
- Create scripts/capture_failure.py for the opt-in bounded capture CLI.
- Create scripts/rerun_failure.py for exact-head validation and rerun status reporting.
- Create python/tests/test_failure_packet.py for packet, capture, rerun, and information-safety tests using local temporary directories and commands.
- Modify scripts/verify_repository.py to require the three Package-D scripts and preserve the existing conformance input invariants.
- Modify docs/MAINTAINER_PLAYBOOK.md, docs/maintenance/DEVELOPER_SETUP.md, and scripts/README.md with the command-level workflow, trust boundary, status vocabulary, and reviewed promotion procedure.
- Modify docs/normative-document-register.v1.json to register this plan as a provisional process document.

No new Rust crate, Python dependency, JSON Schema, workflow, public endpoint, or public diagnostic DTO is created.

### Task 1: Revalidate the exact starting baseline

**Files:**
- Verify only; no source files are modified in this task.

- [ ] **Step 1: Fetch the live remote and record the actual base.**

Run from the isolated worktree:

~~~powershell
git fetch origin
$actualBase = git rev-parse origin/master
Write-Output "BASE=$actualBase"
git show -s --format='%H%n%P%n%s' origin/master
~~~

Expected: BASE is a 40-character SHA and the displayed commit is the current remote master. Do not compare it to the old SHA by forcing a reset.

- [ ] **Step 2: Verify worktree, branch, and Package-C ancestry.**

Run:

~~~powershell
$status = git status --short
if ($status) { throw "implementation worktree is not clean: $status" }
$branch = git branch --show-current
if ($branch -ne 'chris/130-d-failure-reproducer-workflow') { throw "wrong branch: $branch" }
$base = git rev-parse origin/master
git merge-base --is-ancestor 15f339cd0ca875a9f18a44c743796120ed87ade7 origin/master
if ($LASTEXITCODE -ne 0) { throw 'Package-C merge is not an ancestor of origin/master' }
Write-Output "BRANCH=$branch"
Write-Output "BASE=$base"
Write-Output 'PACKAGE_C_ANCESTRY=PASS'
~~~

Expected: clean worktree, the requested branch, and Package-C ancestry. If the remote moved, run `git merge --no-edit origin/master` before implementation, record the resulting base SHA, and stop if the merge conflicts. Do not reset, rebase, or force-update master.

- [ ] **Step 3: Verify Package-C state and protection without mutation.**

Run:

~~~powershell
gh pr view 158 --repo chrismaghuhn/Manafold --json state,mergeCommit,baseRefName
gh api repos/chrismaghuhn/Manafold/branches/master/protection
~~~

Expected: PR 158 is MERGED, its merge commit is present in remote history, and the protection response still identifies manafold-pr-gate. Record the raw values outside the source tree for the final report; do not edit protection.

- [ ] **Step 4: Re-run the start gate in the project environment.**

Run:

~~~powershell
.\.venv\Scripts\python.exe -B scripts/doctor.py --strict
.\.venv\Scripts\python.exe -B scripts/run_python_tests.py --profile full
cargo test -p mtgml-conformance --locked
~~~

Expected: doctor succeeds, the full Python suite succeeds with only its declared adapter skips, and the conformance crate succeeds. A pre-existing failure is recorded as baseline evidence and is not silently attributed to Package D.

### Task 2: Add red Rust first-difference tests

**Files:**
- Modify: crates/mtgml-conformance/src/lib.rs
- Create: crates/mtgml-conformance/src/diagnostics.rs

- [ ] **Step 1: Add the first red assertion for a current-decision detail.**

Extend the existing test module with:

~~~rust
#[test]
fn current_decision_failure_exposes_a_semantic_path() {
    let expected = decision(1);
    let actual = decision(2);
    let submitted = response(1);

    let failure = assert_conformance_inputs(
        Some(&actual),
        &submitted,
        Some(&expected),
        &submitted,
    )
    .expect_err("different current decisions must fail");

    assert!(
        failure.to_string().contains("current_decision"),
        "the current coarse error must become a path-bearing diagnostic"
    );
}
~~~

Run:

~~~powershell
cargo test -p mtgml-conformance --locked current_decision_failure_exposes_a_semantic_path
~~~

Expected: RED because the current coarse error text does not contain the
semantic path. After this first runtime RED, the later helper tests may use the
new diagnostic API and produce the expected compile-time RED until Task 3
implements it.

- [ ] **Step 2: Add focused sequence, scalar, player, and precedence tests.**

In the new diagnostics test module, use primitive synthetic values for the pure helpers:

~~~rust
#[test]
fn event_difference_reports_the_first_nonzero_index() {
    let difference = compare_sequence(
        ConformanceFailureClass::Events,
        "transition.events",
        &[10_u8, 20, 30, 40],
        &[10_u8, 20, 31, 40],
    )
    .expect("difference");

    assert_eq!(difference.semantic_path, "transition.events[2]");
    assert_eq!(difference.mismatch_kind, ConformanceMismatchKind::ValueChanged);
    assert_eq!(difference.sequence.unwrap().first_differing_index, 2);
}

#[test]
fn event_missing_at_end_is_distinct_from_an_extra_event() {
    let missing = compare_sequence(
        ConformanceFailureClass::Events,
        "transition.events",
        &[1_u8, 2, 3],
        &[1_u8, 2],
    )
    .expect("missing difference");
    let extra = compare_sequence(
        ConformanceFailureClass::Events,
        "transition.events",
        &[1_u8, 2],
        &[1_u8, 2, 3],
    )
    .expect("extra difference");

    assert_eq!(
        missing.mismatch_kind,
        ConformanceMismatchKind::ExpectedEntryMissing
    );
    assert_eq!(
        extra.mismatch_kind,
        ConformanceMismatchKind::UnexpectedExtraEntry
    );
    assert_eq!(missing.sequence.unwrap().first_differing_index, 2);
    assert_eq!(extra.sequence.unwrap().first_differing_index, 2);
}

#[test]
fn delta_difference_uses_the_delta_audit_path() {
    let difference = compare_sequence(
        ConformanceFailureClass::Delta,
        "transition.delta.audit",
        &[1_u8, 2, 3],
        &[1_u8, 9, 3],
    )
    .expect("difference");

    assert_eq!(difference.semantic_path, "transition.delta.audit[1]");
}

#[test]
fn scalar_and_player_differences_are_typed() {
    let next = compare_value(
        ConformanceFailureClass::NextDecision,
        "transition.next_decision",
        &Some(1_u8),
        &None,
    )
    .expect("next decision difference");
    assert_eq!(next.mismatch_kind, ConformanceMismatchKind::ValueChanged);

    let status = compare_value(
        ConformanceFailureClass::Status,
        "transition.status",
        &1_u8,
        &2_u8,
    )
    .expect("status difference");
    assert_eq!(status.semantic_path, "transition.status");

    let expected = std::collections::BTreeMap::from([
        (PlayerId(1), 10_u8),
        (PlayerId(2), 20_u8),
    ]);
    let actual = std::collections::BTreeMap::from([
        (PlayerId(1), 11_u8),
        (PlayerId(2), 20_u8),
    ]);
    let player = compare_player_map(&expected, &actual).expect("player difference");
    assert_eq!(player.semantic_path, "player_steps[player:1]");
    assert_eq!(
        player.mismatch_kind,
        ConformanceMismatchKind::PlayerStepDiffered
    );
}
~~~

Also add tests for acceptance, rejected mutation, contract separation, equal-input determinism, and first-causal-difference precedence. The precedence test creates both an event and status mismatch and asserts that the event difference wins. The rejected-mutation test calls the dedicated comparison helper with changed = true instead of manufacturing an invalid production transition.

Run the focused test module. Expected: RED because the helper types and functions do not yet exist.

### Task 3: Implement the typed Rust diagnostic substrate

**Files:**
- Modify: crates/mtgml-conformance/src/lib.rs
- Create: crates/mtgml-conformance/src/diagnostics.rs

- [ ] **Step 1: Add the internal diagnostic types and exports.**

Add mod diagnostics to lib.rs and re-export only trusted conformance types:

~~~rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConformanceFailureClass {
    CurrentDecision,
    Response,
    Acceptance,
    Events,
    Delta,
    StateDigest,
    NextDecision,
    Status,
    PlayerProjection,
    RejectedMutation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConformanceMismatchKind {
    ValueChanged,
    ExpectedEntryMissing,
    UnexpectedExtraEntry,
    PlayerMissing,
    UnexpectedPlayer,
    PlayerStepDiffered,
    RejectedMutation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequenceDifference {
    pub first_differing_index: usize,
    pub expected_length: usize,
    pub actual_length: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConformanceDifference {
    pub surface: ConformanceFailureClass,
    pub semantic_path: String,
    pub mismatch_kind: ConformanceMismatchKind,
    pub expected_summary: String,
    pub actual_summary: String,
    pub sequence: Option<SequenceDifference>,
}
~~~

The summary renderer may use Debug only as ephemeral trusted human output. It must not feed summaries into signature construction.

- [ ] **Step 2: Implement deterministic comparison helpers.**

Implement these pub(crate) helpers in diagnostics.rs:

~~~rust
pub(crate) fn compare_value<T: std::fmt::Debug + PartialEq>(
    surface: ConformanceFailureClass,
    path: &str,
    expected: &T,
    actual: &T,
) -> Option<ConformanceDifference>;

pub(crate) fn compare_sequence<T: std::fmt::Debug + PartialEq>(
    surface: ConformanceFailureClass,
    path: &str,
    expected: &[T],
    actual: &[T],
) -> Option<ConformanceDifference>;

pub(crate) fn compare_player_map<T: std::fmt::Debug + PartialEq>(
    expected: &std::collections::BTreeMap<PlayerId, T>,
    actual: &std::collections::BTreeMap<PlayerId, T>,
) -> Option<ConformanceDifference>;

pub(crate) fn rejected_mutation_difference(
    changed: bool,
) -> Option<ConformanceDifference>;
~~~

compare_sequence checks equal indices first, then classifies a shorter actual sequence as ExpectedEntryMissing and a longer actual sequence as UnexpectedExtraEntry. compare_player_map walks the sorted union of keys and emits PlayerMissing, UnexpectedPlayer, or PlayerStepDiffered at the first key. No serialization or container debug output locates a difference.

- [ ] **Step 3: Add the stable internal marker formatter.**

Implement a method that emits only stable fields:

~~~rust
impl ConformanceDifference {
    pub fn signature_marker(&self) -> String {
        format!(
            "MANAFOLD_FAILURE_SIGNATURE v1 surface={} path={} mismatch_kind={}",
            self.surface.as_token(),
            self.semantic_path,
            self.mismatch_kind.as_token(),
        )
    }
}
~~~

as_token returns fixed lowercase snake-case values. The marker contains no expected value, actual value, hidden state, trusted ID, seed, or log text. It is an internal diagnostic line, not a public schema.

- [ ] **Step 4: Run the Rust diagnostic tests and commit the substrate.**

Run:

~~~powershell
cargo test -p mtgml-conformance --locked diagnostics
~~~

Expected: all pure diagnostic tests pass.

Commit:

~~~powershell
git add -- crates/mtgml-conformance/src/diagnostics.rs crates/mtgml-conformance/src/lib.rs
git commit -m "feat(conformance): add deterministic first differences"
~~~

### Task 4: Integrate first differences into conformance assertions

**Files:**
- Modify: crates/mtgml-conformance/src/lib.rs
- Modify: crates/mtgml-conformance/src/lifecycle.rs

- [ ] **Step 1: Add the additive detailed failure variant and accessors.**

Keep every existing coarse ConformanceFailure variant and add:

~~~rust
Detailed {
    classification: ConformanceFailureClass,
    difference: ConformanceDifference,
},
~~~

Add:

~~~rust
impl ConformanceFailure {
    pub fn classification(&self) -> Option<ConformanceFailureClass>;
    pub fn difference(&self) -> Option<&ConformanceDifference>;
}
~~~

Contract(String) remains separate and returns None for both accessors. Existing coarse variants remain constructible; the default exact assertion returns Detailed for expected/actual mismatches.

- [ ] **Step 2: Preserve the exact comparison precedence.**

Refactor assert_exact_transition so it calls typed helpers in this exact order:

~~~text
current decision
submitted response
validate_transition_contract
acceptance
events
delta audit
state digest
next decision
status
player projections
rejected mutation
~~~

The first helper returning a difference maps to ConformanceFailure::Detailed. A digest calculation error remains the existing non-comparison failure. Contract validation remains ConformanceFailure::Contract(error.to_string()).

- [ ] **Step 3: Use the same sequence/scalar helpers in the product assertion.**

Update assert_exact_transition_product in lifecycle.rs to report event index details, state-digest value details, and status details while preserving its no-decision scope. Do not add player or response semantics to this helper.

- [ ] **Step 4: Add integration tests for the complete assertion.**

Use lifecycle::lifecycle_fixture() and the existing SyntheticM1RulesKernel to build one valid synthetic accepted product. Clone its expected events, delta, and status, then mutate one expected surface at a time. Assert all of the following through assert_exact_transition:

~~~text
acceptance mismatch
event mismatch at index 3
missing final event
unexpected extra event
delta mismatch at index 1
next-decision mismatch
status mismatch
first player projection mismatch
rejected-mutation classification
~~~

The same-input test calls the assertion twice and compares the complete ConformanceDifference. The multi-mismatch test verifies the earlier event surface wins over a later status mismatch. A contract-invalid synthetic product still returns ConformanceFailure::Contract, never Detailed.

- [ ] **Step 5: Run focused and full Rust conformance tests and commit.**

Run:

~~~powershell
cargo test -p mtgml-conformance --locked
cargo test --workspace --all-features --locked
~~~

Expected: both commands exit 0. Inspect the diff for any change to rules, state, information, RNG, replay, or public wire code.

Commit:

~~~powershell
git add -- crates/mtgml-conformance/src/lib.rs crates/mtgml-conformance/src/lifecycle.rs
git commit -m "feat(conformance): explain first causal mismatch"
~~~

### Task 5: Add red Python packet and safety tests

**Files:**
- Create: python/tests/test_failure_packet.py
- Create: scripts/failure_packet.py
- Create: scripts/capture_failure.py
- Create: scripts/rerun_failure.py

- [ ] **Step 1: Write the focused Python test module.**

Use unittest, tempfile.TemporaryDirectory, local sys.executable -c commands, and unittest.mock. Insert scripts/ into sys.path as existing maintainer tests do. Do not snapshot os.environ into a packet.

The test module contains these cases:

~~~text
test_failing_command_creates_packet
test_passing_command_creates_no_packet
test_missing_command_is_blocked_without_packet
test_packet_records_commit_tree_and_fingerprint_separately
test_packet_preserves_argv_as_a_list
test_packet_log_checksum_verifies
test_corrupt_log_checksum_is_blocked
test_unsupported_packet_version_is_blocked
test_wrong_head_is_blocked
test_source_fingerprint_mismatch_is_blocked
test_invalid_output_root_and_traversal_are_rejected
test_timeout_is_explicit_and_non_success
test_exact_failure_rerun_is_reproduced
test_changed_exit_status_is_not_reproduced
test_changed_structured_signature_is_not_reproduced
test_timeout_followed_by_ordinary_exit_is_not_reproduced
test_missing_rerun_prerequisite_is_blocked
test_capture_and_rerun_leave_source_identity_unchanged
test_safe_summary_omits_trusted_and_secret_sentinels
test_packet_does_not_snapshot_process_environment
test_capture_is_opt_in_and_does_not_enter_run_checks
~~~

For source tests, inject a deterministic SourceIdentity provider so the temporary repository remains independent of the implementation worktree. Add one non-injected test that compares failure_packet.source_tree_fingerprint() with run_verification.source_tree_fingerprint(); this proves the existing source_files()/hash implementation is reused rather than duplicated.

- [ ] **Step 2: Run the red Python tests.**

Run:

~~~powershell
.\.venv\Scripts\python.exe -B -m pytest -q python/tests/test_failure_packet.py
~~~

Expected: RED during collection because the new packet module and CLIs do not yet exist. Do not turn this into a skipped test or add a fake passing module.

### Task 6: Implement the internal packet core

**Files:**
- Create: scripts/failure_packet.py
- Test: python/tests/test_failure_packet.py

- [ ] **Step 1: Define packet constants and bounded result types.**

Implement only standard-library code with:

~~~python
FAILURE_PACKET_FORMAT = "manafold.failure-packet.v1"
MAX_SINGLE_REPRO_SUBPROCESS_RUNTIME_SECONDS = 600
CAPTURE_PASS = "CAPTURE_PASS"
CAPTURE_COMMAND_EXIT = "CAPTURE_COMMAND_EXIT"
CAPTURE_TIMEOUT = "CAPTURE_TIMEOUT"
CAPTURE_BLOCKED = "CAPTURE_BLOCKED"
COMMAND_EXIT = "COMMAND_EXIT"
COMMAND_TIMEOUT = "COMMAND_TIMEOUT"
RERUN_REPRODUCED = "REPRODUCED"
RERUN_NOT_REPRODUCED = "NOT_REPRODUCED"
RERUN_BLOCKED = "BLOCKED"
OUTPUT_MARKER = ".mtgml-failure-output"
~~~

Use frozen dataclasses for SourceIdentity and CommandOutcome. Keep packet dictionaries JSON-compatible and validate their types before use.

- [ ] **Step 2: Reuse the existing source identity implementation.**

Import source_tree_fingerprint from scripts/run_verification.py and call it without a replacement source-set implementation. Use Git subprocesses with shell=False, cwd=ROOT, captured output, and a 30-second metadata timeout to collect:

~~~text
git rev-parse HEAD          -> source.commit
git rev-parse HEAD^{tree}   -> source.tree
git status --porcelain --untracked-files=all -> source.clean
run_verification.source_tree_fingerprint() -> source.fingerprint
~~~

Treat any Git probe failure as BLOCKED. Store only the clean boolean and the three identities; never store the complete status output.

- [ ] **Step 3: Implement manifest validation and checksum handling.**

Validate this required structure:

~~~json
{
  "format": "manafold.failure-packet.v1",
  "packet_id": "safe-id",
  "sensitivity": "trusted",
  "origin": {"kind": "command", "case_id": "case"},
  "source": {"commit": "sha", "tree": "sha", "fingerprint": "sha", "clean": true},
  "command": {"argv": ["..."], "cwd": "."},
  "execution": {"outcome": "COMMAND_EXIT", "exit_status": 1, "timeout_seconds": 600},
  "failure": {"classification": "COMMAND_EXIT"},
  "failure_signature": {
    "origin_kind": "command",
    "case_id": "case",
    "failure_classification": "COMMAND_EXIT",
    "surface": null,
    "semantic_path": null,
    "mismatch_kind": null
  },
  "artifacts": {"log": {"path": "command.log", "sha256": "sha"}},
  "tools": {"capture_python": {"version": "3.13.15", "required": true}},
  "reproduction": {"display_command": "<project-python> scripts/rerun_failure.py <packet>"}
}
~~~

Require lowercase hexadecimal lengths for commit/tree/fingerprint/checksum, nonempty string argv entries without NUL, repository-relative cwd, and COMMAND_EXIT/COMMAND_TIMEOUT predicate consistency. reproduction is never read for execution.

- [ ] **Step 4: Implement safe output-root validation and atomic writing.**

Use the default ROOT / "dist" / "failures". Reject repository root, repository source paths, relative .. traversal, packet-member traversal, alternate streams, an existing unmarked root, and an existing packet ID. Allow an explicitly supplied absolute external temporary/output root.

Create .mtgml-failure-output only when creating a new output root. Stage the packet under a temporary sibling directory, write command.log, calculate its SHA-256, write manifest.json, validate both files, then atomically rename the staged directory to the final packet ID. Catch write/rename errors and never report a final packet path for an incomplete packet.

- [ ] **Step 5: Implement explicit marker parsing and safe-summary rendering.**

Accept at most one exact line matching:

~~~text
MANAFOLD_FAILURE_SIGNATURE v1 surface=<token> path=<token> mismatch_kind=<token>
~~~

No marker means a generic command signature. A marker prefix with malformed or duplicate fields is a blocking diagnostic. The parser returns only the three stable marker fields; it never returns expected/actual values or log text.

Implement safe_summary(manifest) by constructing a new dictionary from only:

~~~text
format, packet_id, origin.kind, origin.case_id,
failure.classification, failure_signature, source.commit
~~~

Do not redact a full manifest after formatting. Do not read command.log in the safe renderer.

- [ ] **Step 6: Run packet-core tests and commit.**

Run:

~~~powershell
.\.venv\Scripts\python.exe -B -m pytest -q python/tests/test_failure_packet.py -k "packet or checksum or output_root or summary or fingerprint"
~~~

Expected: packet structure, identity separation, checksums, output safety, marker parsing, source-set reuse, and safe-summary tests pass.

Commit:

~~~powershell
git add -- scripts/failure_packet.py python/tests/test_failure_packet.py
git commit -m "feat(maintainers): add trusted failure packet core"
~~~

### Task 7: Implement bounded capture

**Files:**
- Create: scripts/capture_failure.py
- Modify: scripts/failure_packet.py
- Test: python/tests/test_failure_packet.py

- [ ] **Step 1: Add the capture command parser and subprocess boundary.**

Support:

~~~text
<project-python> scripts/capture_failure.py --case-id CASE -- <argv...>
~~~

Require a nonempty argv after --. Run it as a list with shell=False, cwd=ROOT, capture_output=True, text=False, and timeout=MAX_SINGLE_REPRO_SUBPROCESS_RUNTIME_SECONDS. Do not retry and do not copy the process environment into the manifest.

- [ ] **Step 2: Implement capture status handling.**

Apply this exact decision table:

~~~text
returncode == 0 and before == after
  CAPTURE_PASS, no packet, exit 0

returncode > 0 and before == after
  CAPTURE_COMMAND_EXIT, COMMAND_EXIT packet, return original code

TimeoutExpired and before == after
  CAPTURE_TIMEOUT, COMMAND_TIMEOUT packet, exit 124

missing executable, metadata failure, unsafe output, malformed marker,
source/Git mutation, or packet-write failure
  CAPTURE_BLOCKED, no completed packet, exit 2
~~~

Capture compares commit, tree, source fingerprint, and clean status before and after the child. A changed source identity blocks even when the child failed. A missing executable is never packaged as a failure because no declared execution occurred.

- [ ] **Step 3: Record provenance without over-collection.**

Record capture_python version as a required tool prerequisite. Do not record arbitrary environment variables. Set sensitivity to trusted for the restricted local packet; provide no secret-capability option and no raw seed field. Store the combined stdout/stderr log only in the trusted packet and print the packet path plus a human display rerun command to the terminal.

- [ ] **Step 4: Run capture tests and commit.**

Run:

~~~powershell
.\.venv\Scripts\python.exe -B -m pytest -q python/tests/test_failure_packet.py -k "capture or timeout or missing_command or source"
~~~

Expected: pass/no-packet, failing command, timeout, missing command, source mutation, argv, packet path, and nonmutation cases pass.

Commit:

~~~powershell
git add -- scripts/capture_failure.py scripts/failure_packet.py python/tests/test_failure_packet.py
git commit -m "feat(maintainers): capture bounded deterministic failures"
~~~

### Task 8: Implement exact-head rerun and reproduction predicates

**Files:**
- Create: scripts/rerun_failure.py
- Modify: scripts/failure_packet.py
- Test: python/tests/test_failure_packet.py

- [ ] **Step 1: Validate every rerun prerequisite before execution.**

The rerunner validates, in order:

~~~text
packet directory and manifest format
required field types and supported version
manifest/log member paths and log checksum
current HEAD == source.commit
current HEAD tree == source.tree
current source fingerprint == source.fingerprint
current clean status == recorded clean requirement
capture_python version == recorded required version
repository-relative cwd exists and is a directory
argv is a nonempty list without NUL
argv executable exists or resolves on PATH
~~~

On any failure, print BLOCKED and a concrete diagnostic, return 2, and do not execute the child. Never call fetch, checkout, reset, clean, stash, or any other Git mutator.

- [ ] **Step 2: Execute the recorded argv directly and detect mutations.**

Run exactly manifest["command"]["argv"] with shell=False, exactly the recorded repository-relative cwd, and the 600-second timeout. Snapshot source identity before and after. A changed identity is BLOCKED, not NOT_REPRODUCED.

- [ ] **Step 3: Apply the explicit reproduction predicates.**

Implement:

~~~text
COMMAND_EXIT:
  same generic/structured failure signature
  AND rerun exits with the recorded nonzero exit_status
  -> REPRODUCED

COMMAND_TIMEOUT:
  same generic/structured failure signature
  AND rerun reaches the declared timeout
  -> REPRODUCED

valid execution with different exit status, zero exit, ordinary exit instead
of timeout, or different structured marker
  -> NOT_REPRODUCED

missing executable, wrong tool version, missing/malformed expected marker,
checksum/source/cwd/argv failure, or source mutation
  -> BLOCKED
~~~

The same command with captured exit 1 and rerun exit 2 must be NOT_REPRODUCED. The same nonzero exit with different structured marker fields must be NOT_REPRODUCED. A captured timeout followed by ordinary exit 1 must be NOT_REPRODUCED.

- [ ] **Step 4: Add explicit rerun output and exit codes.**

Print exactly one primary status line using REPRODUCED, NOT_REPRODUCED, or BLOCKED, followed by relevant command/identity detail. Use rerunner exit codes 0, 1, and 2 respectively. Never print PASS for reproduction.

- [ ] **Step 5: Run rerun and safety tests and commit.**

Run:

~~~powershell
.\.venv\Scripts\python.exe -B -m pytest -q python/tests/test_failure_packet.py -k "rerun or reproduced or not_reproduced or blocked or signature or environment"
~~~

Expected: all exact-head, checksum, tool, exit-status, timeout, structured marker, missing-prerequisite, and nonmutation cases pass.

Commit:

~~~powershell
git add -- scripts/rerun_failure.py scripts/failure_packet.py python/tests/test_failure_packet.py
git commit -m "feat(maintainers): rerun failure packets fail closed"
~~~

### Task 9: Add repository guards and maintainer documentation

**Files:**
- Modify: scripts/verify_repository.py
- Modify: docs/MAINTAINER_PLAYBOOK.md
- Modify: docs/maintenance/DEVELOPER_SETUP.md
- Modify: scripts/README.md
- Modify: docs/normative-document-register.v1.json
- Test: python/tests/test_failure_packet.py

- [ ] **Step 1: Register the new scripts in repository verification.**

Add these exact paths to verify_repository.py's required-file list:

~~~text
scripts/failure_packet.py
scripts/capture_failure.py
scripts/rerun_failure.py
~~~

Preserve all Package-C required files and existing conformance input checks. Add a test that run_checks.py does not invoke capture or rerun automatically.

- [ ] **Step 2: Document the command-level boundary.**

In docs/MAINTAINER_PLAYBOOK.md, document Windows and POSIX command forms, CAPTURE_* and rerun status tables, the 600-second limit, exact-head checks, no-Git-mutation rule, missing-executable blocker, packet location, and the distinction between this small command-level packet and the complete ADR-0036 portable bundle.

State that the packet is trusted local evidence, not a public CI artifact, player diagnostic, ML field, replay authority, checkpoint authority, or semantic fixture. Document the reviewed promotion lifecycle and the ban on copying actual output into expected output.

- [ ] **Step 3: Document setup and script ownership.**

In docs/maintenance/DEVELOPER_SETUP.md, show the .venv capture/rerun commands for Windows and WSL/Linux. State that command.argv is executed directly with shell=False, command.cwd is repository-relative, and reproduction.display_command is display-only.

In scripts/README.md, add entries for failure_packet.py, capture_failure.py, and rerun_failure.py, including generated output location and status semantics.

- [ ] **Step 4: Register this implementation plan.**

Add one provisional process entry to docs/normative-document-register.v1.json:

~~~json
{
  "change_process": "process-pr",
  "owner_role": "maintainer",
  "path": "docs/superpowers/plans/2026-09-13-issue-130-package-d-failure-reproducer-workflow.md",
  "role": "process",
  "stability": "provisional"
}
~~~

- [ ] **Step 5: Run documentation and repository checks and commit.**

Run:

~~~powershell
.\.venv\Scripts\python.exe -B -m pytest -q python/tests/test_failure_packet.py -k "documentation or opt_in"
.\.venv\Scripts\python.exe -B scripts/check_documentation.py
.\.venv\Scripts\python.exe -B scripts/verify_repository.py
git diff --check
~~~

Expected: all commands exit 0.

Commit:

~~~powershell
git add -- scripts/verify_repository.py docs/MAINTAINER_PLAYBOOK.md docs/maintenance/DEVELOPER_SETUP.md scripts/README.md docs/normative-document-register.v1.json python/tests/test_failure_packet.py
git commit -m "docs(maintainers): document deterministic failure workflow"
~~~

### Task 10: Perform the required synthetic demonstration

**Files:**
- Verify only; generated packets stay outside the source tree and are removed.

- [ ] **Step 1: Create a deliberate synthetic failing command.**

From a clean committed implementation worktree, run a local command that prints one valid marker and exits 1:

~~~powershell
$demo = "import sys; print('MANAFOLD_FAILURE_SIGNATURE v1 surface=events path=transition.events[3] mismatch_kind=value_changed'); sys.exit(1)"
.\.venv\Scripts\python.exe -B scripts/capture_failure.py --case-id PACKAGE_D_SYNTHETIC -- .\.venv\Scripts\python.exe -B -c $demo
~~~

Confirm the command returns nonzero, creates one packet below dist/failures/, and prints its exact HEAD plus display rerun command.

- [ ] **Step 2: Rerun the generated packet on the same HEAD.**

Run the emitted rerun command using the project Python. Confirm:

~~~text
first difference marker is present in the trusted log
packet source.commit == current HEAD
packet source.tree == current HEAD tree
packet source.fingerprint == current source fingerprint
rerun status == REPRODUCED
rerun exit == 0
~~~

- [ ] **Step 3: Prove source nonmutation and remove demonstration output.**

Capture source identity before capture and after rerun using run_verification.source_tree_fingerprint(), Git HEAD/tree, and clean status. Require equality for all values. Remove only the generated demonstration packet from the owned dist/failures/ output root after recording the result; do not remove any source, worktree, or unmarked directory.

- [ ] **Step 4: Verify the demonstration output is gone.**

Run:

~~~powershell
if (Test-Path -LiteralPath 'dist/failures/PACKAGE_D_SYNTHETIC') { throw 'demo packet remains' }
git status --short
~~~

Expected: no demonstration packet and no source changes.

### Task 11: Run the full local verification matrix

**Files:**
- Verify every changed file; do not edit source during the final evidence run.

- [ ] **Step 1: Run the required local commands with the project Python.**

Run each command separately and record its actual result:

~~~powershell
.\.venv\Scripts\python.exe -B scripts/doctor.py --strict
.\.venv\Scripts\python.exe -B -m pytest -q python/tests/test_failure_packet.py
.\.venv\Scripts\python.exe -B scripts/run_python_tests.py --profile full
cargo test -p mtgml-conformance --locked
cargo test --workspace --all-features --locked
.\.venv\Scripts\python.exe -B scripts/run_checks.py fast
.\.venv\Scripts\python.exe -B scripts/run_checks.py integration
.\.venv\Scripts\python.exe -B scripts/run_checks.py certification
.\.venv\Scripts\python.exe -B -m ruff format --check python scripts
.\.venv\Scripts\python.exe -B -m ruff check python scripts
.\.venv\Scripts\python.exe -B -m mypy --config-file python/pyproject.toml
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
.\.venv\Scripts\python.exe -B scripts/check_documentation.py
.\.venv\Scripts\python.exe -B scripts/verify_repository.py
.\.venv\Scripts\python.exe -B scripts/validate_schemas.py
git diff --check
git status --short
~~~

Only a command that actually exits successfully is PASS. A missing or unavailable command is NOT_RUN or BLOCKED, never green by inference.

- [ ] **Step 2: Inspect exact scope using tracked and untracked files.**

Run:

~~~powershell
git diff --name-only origin/master...HEAD
git ls-files --others --exclude-standard
git diff --stat origin/master...HEAD
~~~

Confirm the union contains only the planned Rust, Python, script, document, and register files plus no packet, log, cache, seed, checkpoint, card, deck, schema, lockfile, workflow, or generated contract changes.

- [ ] **Step 3: Verify safety invariants by inspection and tests.**

Confirm:

~~~text
no raw root-seed capture
no arbitrary environment serialization
no player/API/ML diagnostic exposure
no automatic expected/golden rewrite
no automatic promotion or support claim
no automatic Git mutation
no normal run_checks capture integration
all child subprocesses have a hard limit <= 600 seconds
failure summaries are not signatures
commit, tree, and source fingerprint remain separate
~~~

### Task 12: Push, open the PR, and stop at independent review

**Files:**
- Verify only; no merge or issue closure.

- [ ] **Step 1: Capture final local identity and protection state.**

Run:

~~~powershell
$head = git rev-parse HEAD
$base = git rev-parse origin/master
git status --short --branch
gh api repos/chrismaghuhn/Manafold/branches/master/protection
Write-Output "BASE=$base"
Write-Output "HEAD=$head"
~~~

Require a clean worktree and record the exact values. Do not claim hosted evidence from local results.

- [ ] **Step 2: Push the implementation branch.**

Run:

~~~powershell
git push --set-upstream origin chris/130-d-failure-reproducer-workflow
~~~

Expected: the remote branch points to the locally verified HEAD.

- [ ] **Step 3: Open the PR without merge or issue closure.**

Run:

~~~powershell
gh pr create --repo chrismaghuhn/Manafold --base master --head chris/130-d-failure-reproducer-workflow --title "chore(maintainers): add deterministic failure repro workflow" --body "Issue #130 Package D. Rules-neutral maintainer tooling; no M3; no real cards; no public diagnostic schema; no dependency remediation. Package D uses an internal command-level failure packet and deterministic first-difference diagnostics. No merge performed by implementation task."
~~~

Do not use Fixes #130, do not merge the PR, and do not close Issue #130.

- [ ] **Step 4: Verify exact-head hosted checks.**

Read the PR head SHA and wait for the actual checks:

~~~powershell
$pr = gh pr view --repo chrismaghuhn/Manafold --json number,headRefOid,url | ConvertFrom-Json
Write-Output "PR=$($pr.number)"
Write-Output "PR_HEAD=$($pr.headRefOid)"
gh pr checks $pr.number --repo chrismaghuhn/Manafold --watch
gh pr view $pr.number --repo chrismaghuhn/Manafold --json statusCheckRollup
~~~

Record exact-head results separately for fast, integration, windows-setup-smoke, manafold-pr-gate, Analyze (actions), Analyze (python), Analyze (rust), and CodeQL. If any hosted check fails, the implementation remains incomplete until the failure is fixed and the checks rerun on the final head.

- [ ] **Step 5: Report and stop.**

Return the requested final report with actual BASE, HEAD, changed files, Rust/Python/local/hosted statuses, packet/reproducer safety decisions, PYTEST_SECURITY_REMEDIATION = OUT_OF_SCOPE / TRACKED_SEPARATELY, M3 = NOT_AUTHORIZED, MERGE = NOT_PERFORMED, ISSUE_130_CLOSED = NO, and INDEPENDENT_EXACT_SHA_REVIEW = PENDING.

## Plan self-review checklist

Before executing this plan, verify the plan itself covers every approved design requirement:

- [x] command-level packet explicitly distinguished from the complete ADR-0036 bundle;
- [x] commit SHA, Git tree, and existing source fingerprint are separate;
- [x] existing run_verification.source_tree_fingerprint() and source_files() are reused;
- [x] command.argv and repository-relative command.cwd are sole machine execution authority;
- [x] shell=False and no shell-string reparsing are required;
- [x] trusted log, safe summary, and secret-seed boundary are separate;
- [x] missing executable is BLOCKED with no completed packet;
- [x] COMMAND_EXIT and COMMAND_TIMEOUT predicates are distinct;
- [x] exit-status differences and timeout-vs-exit differences are NOT_REPRODUCED;
- [x] structured marker differences are NOT_REPRODUCED and missing/malformed expected markers are BLOCKED;
- [x] summaries and Debug text cannot define failure identity;
- [x] Rust first-difference precedence, sequence indexes, player ordering, and contract separation are tested;
- [x] no public schema, diagnostics crate, M3, cards, dependencies, or automatic promotion is introduced;
- [x] source-tree nonmutation is tested during capture, rerun, and demonstration;
- [x] local and hosted evidence remain explicitly separate;
- [x] merge and Issue #130 closure remain outside the task.

Run the self-review before starting implementation:

~~~powershell
$markers = @('T' + 'O' + 'D' + 'O', 'T' + 'B' + 'D', 'F' + 'I' + 'X' + 'M' + 'E')
$hits = Select-String -Path 'docs/superpowers/plans/2026-09-13-issue-130-package-d-failure-reproducer-workflow.md' -Pattern $markers
if ($hits) { throw "plan contains an unresolved marker: $hits" }
.\.venv\Scripts\python.exe -B scripts/check_documentation.py
git diff --check
~~~

The marker scan must return no result; documentation and diff checks must exit 0.
