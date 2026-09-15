# Final Foundation Closure Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Status:** active inline execution plan

**Goal:** Reconcile all 53 canonical Issue #164 findings on one exact post-PR-#174 head, apply only required bounded P2 hardening, and publish a review-ready Final Foundation Closure evidence record without authorizing M3.

**Architecture:** Existing accepted owners, batch evidence, and ADRs remain authoritative. The closure report is a traceability artifact, not a second semantic contract. Only stale documentation or proof-only diagnostics may be corrected; no non-actor delivery semantics or new architecture is introduced.

**Tech Stack:** Locked Rust/Cargo gates, pinned Python 3.13 verification scripts, JSON Schemas, generated contract vocabulary, gh issue/PR evidence, and deterministic source-archive verification.

---

### Task 1: Establish exact identity and current authority

**Files:** Read AGENTS.md, README.md, docs/NORMATIVE_HIERARCHY.md, docs/ROADMAP.md, docs/contracts/ACCEPTANCE_GATES.md, ADRs 0049/0050, Batch-D/H disposition records, and the FND-028 evidence record.

- [ ] **Step 1: Verify the remote base and clean branch.**

~~~powershell
git ls-remote origin refs/heads/master
git rev-parse --show-toplevel
git branch --show-current
git rev-parse HEAD
git status --porcelain=v2 --branch
~~~

Expected: origin/master is 47457cfd66ff5240a2c65d8e1bc699222add39d7, the branch is chris/final-foundation-closure-20260915, and the worktree is clean before closure edits.

- [ ] **Step 2: Re-fetch Issue #164 and PR #165-#174.** Query Issue #164 and every PR with gh, including state, merge commit, head SHA, URL, body, and review/status context. Reconcile every current disposition against the live tracker rather than relying on historical summaries.

- [ ] **Step 3: Run the clean current-head baseline.**

~~~powershell
cargo test --workspace --all-features --locked
C:\Python313\python.exe scripts/run_python_tests.py --profile full
~~~

Record actual counts, skips, failures, and toolchain identity. A nonzero result is a closure blocker and is not hidden in documentation.

### Task 2: Characterize and classify HRD-001-HRD-006

**Files:** Read crates/mtgml-state/src/m2_shape.rs; Rust/Python observation and replay validators/tests; conformance diagnostics, isolation, and legal-space harnesses; the Python adapter and M2.H tests; Batch-G/H evidence.

- [ ] **Step 1: HRD-001.** Confirm that m2_shape is used by current EngineState and validate_engine_state while its header still claims detached/unreachable status. Classify the stale documentation as a bounded correction only.

- [ ] **Step 2: HRD-002.** Confirm V1 empty random labels return EmptyEventText, V2 currently returns RandomOutcome, and Python keeps the generic outer semantic.observed_event contract. If no accepted authority makes the difference intentional, fix only the Rust typed category and preserve Python's outer code.

- [ ] **Step 3: HRD-003.** Confirm shared Rust ReplayStepIdentity text says replay-step.v2 even for V3, while Python V3 is correct. Use version-neutral wording because the enum is shared by V2 and V3.

- [ ] **Step 4: HRD-004/005.** Verify safe semantic paths, mismatch kinds, first indices, lengths, and presence tokens in current diagnostics, and exact ordered-sequence comparison in Batch-H evidence. Classify as RESOLVED_ON_BASE when executable code proves the requirements.

- [ ] **Step 5: HRD-006.** Inspect the four adapter gaps: byte-level unknown-token uniformity, both-endpoint actor spying, stale revision/stage/token forms, and trusted-process panic integration. Record unit/core evidence and exact MTGML_M2_ADAPTER_BIN-dependent scenarios as NOT_RUN; classify DEFERRED_P2 unless a concrete gap falsifies a required core gate.

### Task 3: Apply only required HRD corrections with TDD

**Files:** Modify only crates/mtgml-state/src/m2_shape.rs, crates/mtgml-observation/src/observed_event.rs, crates/mtgml-observation/src/tests.rs, crates/mtgml-replay/src/validation.rs, and crates/mtgml-replay/src/tests.rs when Task 2 confirms the corrections.

- [ ] **Step 1: Add RED tests first.** Add a V2 empty-label test asserting Err(ObservationValidationError::EmptyEventText). Add a V3 manifest mismatch test asserting Err(ReplayValidationError::ReplayStepIdentity) and display text replay-step schema identity is invalid.

~~~powershell
cargo test -p mtgml-observation --locked observed_event_v2_random_empty_label_uses_empty_text_error
cargo test -p mtgml-replay --locked replay_step_identity_diagnostic_is_version_neutral
~~~

Both must fail for the characterized behavior, not because of a compile or fixture error, before production code is changed.

- [ ] **Step 2: Apply the minimum corrections.** Return EmptyEventText for an empty V2 random label before range validation, retain RandomOutcome for invalid range values, change the shared replay display to replay-step schema identity is invalid, and replace the stale m2_shape header with current ownership documentation. Do not change schemas, wire bytes, digest/replay versions, endpoints, delivery, or semantic state behavior.

- [ ] **Step 3: Verify GREEN and affected packages.**

~~~powershell
cargo test -p mtgml-observation --locked
cargo test -p mtgml-replay --locked
cargo test -p mtgml-state --locked
~~~

### Task 4: Create the authoritative 53-row evidence document

**Files:** Modify docs/normative-document-register.v1.json; create docs/superpowers/specs/2026-09-15-final-foundation-closure.md.

- [ ] **Step 1: Register and validate the evidence path.** Add a process/provisional entry for docs/superpowers/specs/2026-09-15-final-foundation-closure.md, then run:

~~~powershell
C:\Python313\python.exe -c "import json; json.load(open('docs/normative-document-register.v1.json', encoding='utf-8')); print('PASS: register JSON')"
~~~

- [ ] **Step 2: Write the report from measured evidence.** Include TASK = FINAL_FOUNDATION_CLOSURE, verified BASE = 47457cfd66ff5240a2c65d8e1bc699222add39d7, exact counts 53/53, 32/32, 15/15, 6/6, blocker/deferred summaries, compatibility/scope status, change-aware review, and final gate matrix.

The single table must use exactly these columns: ID, ORIGINAL_CLASS, FINAL_DISPOSITION, ROOT_CAUSE_OR_SCOPE, CURRENT_OWNER, IMPLEMENTATION_OR_DECISION, EVIDENCE, PR_OR_COMMIT, FREEZE_IMPACT, REMAINING_LIMITATION. It must contain exactly FND-001-FND-032, EVD-001-EVD-015, and HRD-001-HRD-006, each once. Explain split provenance inside parent rows, including 006A/B, 010A-F, 021A-C, 022A-E, and 026A-D. Preserve FND-026B as blocked but nonblocking and FND-026C as deferred P2. Do not write a self-referential final SHA.

- [ ] **Step 3: Validate documentation and whitespace.**

~~~powershell
C:\Python313\python.exe scripts/check_documentation.py
C:\Python313\python.exe scripts/validate_maintainer_artifacts.py
git diff --check
~~~

### Task 5: Run final exact-head foundation gates

- [ ] **Step 1: Run Rust quality and workspace gates separately.** Run cargo fmt --all -- --check, cargo check --workspace --all-targets --all-features --locked, cargo clippy --workspace --all-targets --all-features --locked -- -D warnings, and cargo test --workspace --all-features --locked; record each exit code and output.

- [ ] **Step 2: Run direct Python and repository gates separately.** Run the full Python profile, contract-generation check, schema validation, repository verification, Rust source-structure check, documentation check, maintainer-artifact validation, golden-path validation, Python toolchain verification, direct fast/integration/certification profiles, and scripts/run_verification.py. Record optional adapter scenarios as NOT_RUN.

- [ ] **Step 3: Run just check-fast, just check, just check-all, just release-candidate, and just archive-check separately.** If WSL cannot start /bin/bash, record each wrapper as BLOCKED and keep direct native evidence separate.

- [ ] **Step 4: Run archive reproducibility only after all source edits.** Run scripts/verify_archive_reproducibility.py, git diff --check, and clean-status checks. After the evidence commit, rerun documentation, direct archive reproducibility, and git diff --check; make no source changes afterward.

### Task 6: Commit, push, and stop for independent review

**Files:** Only the plan, register, closure report, and confirmed bounded HRD files.

- [ ] **Step 1: Audit changed-file union.** Compare 47457cfd66ff5240a2c65d8e1bc699222add39d7..HEAD plus git ls-files --others --exclude-standard; reject card, capability, M3, delivery, schema-version, wire-fixture, and unrelated refactor paths.

- [ ] **Step 2: Commit only bounded closure work.** Stage the exact files, run git diff --cached --check, and commit:

~~~powershell
git commit -m "docs: record final foundation closure"
~~~

Verify the commit and clean status before push.

- [ ] **Step 3: Re-run post-commit documentation/archive checks and push.** Verify local HEAD, run documentation, direct archive reproducibility, and git diff --check, then push chris/final-foundation-closure-20260915 and verify the remote branch SHA equals local HEAD.

- [ ] **Step 4: End with the required external status.** Report FINAL_FOUNDATION_CLOSURE = PASS/FAIL, PRE_M3_REMEDIATION_FREEZE = NOT_YET_PERFORMED, FOUNDATION_READY_FOR_M3 = NO, M3_STARTED = NO, M3_AUTHORIZED = NO, HOSTED_CI = PENDING, and FINAL_FOUNDATION_CLOSURE_REVIEW = PENDING. Do not merge or close Issue #164.
