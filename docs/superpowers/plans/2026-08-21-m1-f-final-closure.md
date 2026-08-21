# M1.F Final Milestone Closure Implementation Plan

**Status:** provisional process plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a narrow reproducible M1 closure reporter, execute all ten M1 acceptance gates on one final source commit, and publish one exact-head Draft PR without changing gameplay semantics.

**Architecture:** Keep `scripts/run_verification.py` as the V0.2.2 Foundation reporter. Add `scripts/run_m1_closure.py` with an explicit immutable gate/test matrix, exact Rust test subprocesses, source/toolchain identity checks, and external JSON/Markdown evidence under `dist/verification/m1/`. Do not edit the stale `project-sources/33_CURRENT_PROJECT_STATE.md` export or introduce a tracked M1 status file.

**Tech Stack:** Python 3.13.15, Rust/Cargo 1.85.1, locked Cargo workspace, existing `run_checks.py` profiles, GitHub Actions, and `gh` CLI.

---

### Task 1: Register and freeze the closure process artifacts

**Files:**
- Create: `docs/superpowers/specs/2026-08-21-m1-f-final-closure-design.md`
- Create: `docs/superpowers/plans/2026-08-21-m1-f-final-closure.md`
- Modify: `docs/normative-document-register.v1.json`

- [x] **Step 1: Record the accepted design and evidence inventory.**

The design file records the exact source boundary, the V0.2.2 reporter boundary, the external output directory, the two-pass rule, and the ten gate-to-test mappings. It explicitly records that the old ChatGPT project-source export is not an M1 status authority.

- [x] **Step 2: Register both process artifacts.**

Add the following two objects to the `documents` array:

```json
{
  "change_process": "process-pr",
  "owner_role": "maintainer",
  "path": "docs/superpowers/plans/2026-08-21-m1-f-final-closure.md",
  "role": "process",
  "stability": "provisional"
},
{
  "change_process": "process-pr",
  "owner_role": "maintainer",
  "path": "docs/superpowers/specs/2026-08-21-m1-f-final-closure-design.md",
  "role": "process",
  "stability": "provisional"
}
```

- [ ] **Step 3: Verify the documentation registration.**

Run:

```text
C:\Python313\python.exe scripts/check_documentation.py
C:\Python313\python.exe scripts/verify_repository.py
```

Expected: both commands exit 0 and recognize the registered local paths.

### Task 2: Implement the M1 closure reporter

**Files:**
- Create: `scripts/run_m1_closure.py`
- Modify: `scripts/README.md`

- [ ] **Step 1: Define the immutable ten-gate test matrix.**

Represent each test as a tuple of gate name, owning Cargo package, exact Rust test identity, and the surface proved. Build every command as:

```python
[
    "cargo", "test", "--package", package,
    "--all-features", "--locked", "--lib", "--",
    test_name, "--exact",
]
```

The matrix must include every test listed in the accepted design inventory. A focused command is PASS only if its exit code is zero and its output contains exactly one executed test and one passed test. An unmatched `--exact` filter is FAIL, not PASS.

- [ ] **Step 2: Capture source and toolchain identity before execution.**

Capture `git rev-parse HEAD`, `git rev-parse HEAD^{tree}`, the tracked-source fingerprint, and `git status --porcelain=v1 --untracked-files=all`. Capture the exact outputs of `C:\Python313\python.exe --version`, `rustc --version`, `cargo --version`, and `rustup show active-toolchain`. Compare Python to `.python-version` and Rust to `rust-toolchain.toml`; a mismatch is a blocking report condition.

- [ ] **Step 3: Execute each exact test and preserve independent evidence.**

Continue after failures. Store command, return code, observed-test count, status, and a relative log path for every invocation. Use these status rules:

```text
PASS     exit 0 and exactly one expected test passed
FAIL     command ran but failed or the exact test was not observed
NOT_RUN  executable was unavailable
BLOCKED  command could not start because the host prevented execution
```

- [ ] **Step 4: Enforce external output ownership and source immutability.**

Create `dist/verification/m1/` only when it is absent or contains the reporter's marker `.mtgml-m1-closure-output`. Refuse to replace an unmarked directory. Recompute the tracked-source fingerprint and clean-status check after all test commands and before writing the generated reports.

- [ ] **Step 5: Generate one JSON/Markdown result set.**

Write `m1-verification-results.json`, `M1_VERIFICATION.md`, and `M1_BLOCKERS.md` from the same in-memory dictionary. Derive the final result as:

```python
complete = (
    source_identity["status"] == "PASS"
    and toolchains["status"] == "PASS"
    and all(gate["status"] == "PASS" for gate in gates)
)
```

Only `complete` may set `milestone_status` to `COMPLETE` and `m2_status` to `UNBLOCKED`; otherwise use `INCOMPLETE` and `BLOCKED`. Always emit the three non-goal claims as false.

- [ ] **Step 6: Document the new command without changing old meanings.**

Add one line to `scripts/README.md` identifying `run_m1_closure.py` as the external M1 ten-gate reporter. Retain the existing `run_verification.py` line as the V0.2.2 reporter.

### Task 3: Add reporter unit coverage

**Files:**
- Create: `python/tests/test_m1_closure_reporter.py`

- [ ] **Step 1: Test status aggregation with all ten gates PASS.**

Import pure report-building helpers and assert that ten PASS gates plus clean source/toolchain identity produce `milestone_status == "COMPLETE"` and `m2_status == "UNBLOCKED"`.

- [ ] **Step 2: Test fail-closed aggregation.**

Set one gate to `NOT_RUN`, then to `FAIL`, and assert both produce `milestone_status == "INCOMPLETE"`, `m2_status == "BLOCKED"`, and no complete claim. Test that a toolchain mismatch also prevents completion even when every gate is PASS.

- [ ] **Step 3: Test exact-test output validation.**

Assert that output containing `running 1 test` and one passing result is accepted, while `running 0 tests` and a nonzero return code are not accepted.

Run the focused test file with:

```text
C:\Python313\python.exe -m unittest python.tests.test_m1_closure_reporter
```

### Task 4: Commit the reporter and establish the final closure head

**Files:**
- Only the files listed in Tasks 1–3.

- [ ] **Step 1: Run the focused Python/reporting checks.**

```text
C:\Python313\python.exe scripts/check_documentation.py
C:\Python313\python.exe scripts/verify_repository.py
C:\Python313\python.exe -m unittest python.tests.test_m1_closure_reporter
```

- [ ] **Step 2: Inspect and commit only confirmed closure files.**

```text
git diff --check origin/master...HEAD
git status --short
git diff --stat origin/master...HEAD
git add -- docs/normative-document-register.v1.json docs/superpowers/specs/2026-08-21-m1-f-final-closure-design.md docs/superpowers/plans/2026-08-21-m1-f-final-closure.md scripts/run_m1_closure.py scripts/README.md python/tests/test_m1_closure_reporter.py
git commit -m "chore: add M1 closure evidence runner"
```

- [ ] **Step 3: Record the exact closure commit.**

```text
git rev-parse HEAD
git rev-parse origin/master
git status --short --branch
```

The closure commit must be clean and must have `bbbf0ee06b64889c318a7f0fd4d9b608d7181ed7` as its starting parent lineage.

### Task 5: Execute final local evidence on one exact tree

**Files:**
- Generated only under `dist/verification/m1/` and `dist/verification/`.

- [ ] **Step 1: Run the focused M1 closure reporter.**

```text
C:\Python313\python.exe scripts/run_m1_closure.py
```

Require all ten gates, source identity, and toolchains to be PASS. Save the generated result set outside the source archive.

- [ ] **Step 2: Run the required full workspace and repository gates.**

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
C:\Python313\python.exe scripts/generate_contracts.py --check
C:\Python313\python.exe scripts/verify_repository.py
C:\Python313\python.exe scripts/check_rust_source_structure.py
C:\Python313\python.exe scripts/check_documentation.py
C:\Python313\python.exe scripts/validate_schemas.py
C:\Python313\python.exe scripts/validate_maintainer_artifacts.py
C:\Python313\python.exe scripts/validate_golden_path.py
C:\Python313\python.exe scripts/verify_python_toolchain.py
C:\Python313\python.exe scripts/run_python_tests.py
C:\Python313\python.exe scripts/run_checks.py fast
C:\Python313\python.exe scripts/run_checks.py integration
C:\Python313\python.exe scripts/run_checks.py certification
C:\Python313\python.exe scripts/run_verification.py
git diff --check origin/master...HEAD
```

Record `just check-fast`, `just check`, and `just check-all` separately. If a wrapper cannot start because WSL/Hyper-V is unavailable, record that wrapper as `BLOCKED` and retain the direct command's actual result.

- [ ] **Step 3: Verify source immutability and generated-result identity.**

Confirm `git status --short` remains empty, the M1 report's before/after source identity is unchanged, and `run_verification.py` still generated only its V0.2.2-named artifacts. Do not call the V0.2.2 report M1 evidence.

- [ ] **Step 4: Repeat the final M1 reporter on the unchanged exact head.**

```text
C:\Python313\python.exe scripts/run_m1_closure.py
git rev-parse HEAD
git status --short
```

This second run is the final evidence pass. No tracked status source is edited between the runs, so the two-pass invalidation rule is satisfied without inventing a status file. If any tracked status representation is added later, run a new complete Pass B after that edit instead.

### Task 6: Hosted exact-head evidence and Draft PR

**Files:**
- External PR body only; no generated report is committed.

- [ ] **Step 1: Push the exact closure branch.**

```text
git push --set-upstream origin chris/m1-final-closure
```

- [ ] **Step 2: Open one Draft PR against `master`.**

Use title `M1.F: close deterministic kernel shell`, base `master`, head `chris/m1-final-closure`, and include `Closes #27` only if the final exact-head Pass B has all ten gates PASS. The body must include the starting SHA, final SHA, test matrix, local profiles, V0.2.2 foundation result, source-tree unchanged evidence, non-goals, and all hosted check statuses.

- [ ] **Step 3: Dispatch and inspect hosted workflows.**

Verify PR Fast, Integration, Nightly Certification Smoke, and CodeQL. For manually dispatched Integration and Nightly runs, record the exact `headSha`. Nightly's `python scripts/run_checks.py certification` result is hosted certification-profile evidence only; it does not replace the local M1 reporter or `run_verification.py`.

- [ ] **Step 4: Perform final scope audit.**

Inspect the complete diff and final tree for any playable-engine, real-Magic, real-card, M2, M3, broad legal-action, complete noninterference, or Python ML environment claim. Do not merge the Draft PR.
