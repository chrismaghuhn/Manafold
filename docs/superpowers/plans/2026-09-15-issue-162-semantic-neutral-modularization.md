# Issue #162: Semantic-Neutral Modularization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (- [ ]) syntax for tracking.

**Status:** active inline execution plan

**Goal:** Mechanically split four oversized internal modules while preserving every existing public path, validation result, wire byte, digest input, historical replay meaning, and rules-free Python boundary.

**Architecture:** Each target keeps a thin facade. Private modules receive the existing declarations without semantic edits, and the facade re-exports the old names. Version-specific validation stays with its original DTO family; no shared generic codec or validation abstraction is introduced.

**Tech Stack:** Rust 2021 workspace, serde, serde_json, Python 3.13 pinned repository runners, JSON fixtures, and existing direct/native verification commands.

---

### Task 1: Establish exact identity and baseline

**Files:** Read-only verification in C:/Users/chris/Documents/Manafold-issue-162.

- [x] Verify HEAD is bd0b2461a74f9f4c35e5b52736c794a7d980959f, the branch is chris/issue-162-semantic-neutral-modularization, and the worktree was clean before process artifacts.
- [x] Run cargo test --workspace --all-features --locked and scripts/run_python_tests.py --profile full; both exited 0.

### Task 2: Implement R1 Decision ownership split

**Files:**

- Modify: crates/mtgml-decision/src/lib.rs
- Create: crates/mtgml-decision/src/common.rs
- Create: crates/mtgml-decision/src/v1.rs
- Create: crates/mtgml-decision/src/v2.rs
- Create: crates/mtgml-decision/src/authoritative.rs
- Create: crates/mtgml-decision/src/ordering.rs
- Create: crates/mtgml-decision/src/error.rs
- Create: crates/mtgml-decision/src/tests.rs
- Preserve: crates/mtgml-decision/src/tests/batch_f.rs

- [ ] Move declarations without changing bodies: shared visibility/intent to common.rs; V1 DTOs/constants to v1.rs; V2 DTOs/validators to v2.rs; trusted bindings/request to authoritative.rs; ordering helper to ordering.rs; and error enums to error.rs.
- [ ] Replace the root body with private module declarations and exact re-exports for every existing public item, including all four schema constants and CandidateOrderingV1.
- [ ] Move the existing root unit-test body mechanically to tests.rs; retain the batch_f path and assertions unchanged. Use only crate-private visibility if a test needs the ordering capacity helper.
- [ ] Run:

~~~powershell
cargo fmt --all -- --check
cargo test -p mtgml-decision --all-features --locked
~~~

Expected: exit code 0 and the existing Decision tests execute under their
existing names.
- [ ] Run git diff --check, stage only crates/mtgml-decision/src, run git diff --cached --check, and commit:

~~~text
refactor(decision): split internal decision ownership
~~~

### Task 3: Implement R2 Wire ownership split

**Files:**

- Modify: crates/mtgml-wire/src/lib.rs
- Create: crates/mtgml-wire/src/error.rs
- Create: crates/mtgml-wire/src/contract.rs
- Create: crates/mtgml-wire/src/canonical_json.rs
- Create: crates/mtgml-wire/src/decision.rs
- Create: crates/mtgml-wire/src/observation.rs
- Create: crates/mtgml-wire/src/replay.rs
- Create: crates/mtgml-wire/src/fixtures.rs
- Create: crates/mtgml-wire/src/tests.rs
- Create: crates/mtgml-wire/src/constructive_producer_tests.rs

- [ ] Move existing declarations mechanically. Keep WireError, PlayerWireErrorCodeV1, FixtureVerificationError, canonicalization, contract implementations, fixture dispatch, and test bodies unchanged.
- [ ] Keep decode_canonical in the order serde_json decode, validate_wire, canonical re-encode, byte comparison. Keep mtgml_wire::decision_response_v2::decode_submission at the same public path and behavior.
- [ ] Re-export WireContract, canonical encode/decode, digest calculation, fixture verification, errors, and decision_response_v2 from the root.
- [ ] Run:

~~~powershell
cargo fmt --all -- --check
cargo test -p mtgml-wire --all-features --locked
~~~

Expected: exit code 0 with no fixture changes.
- [ ] Run git diff --check, stage only crates/mtgml-wire/src, run git diff --cached --check, and commit:

~~~text
refactor(wire): split internal wire ownership
~~~

### Task 4: Implement R3 Python observation/information ownership split

**Files:**

- Modify: python/src/mtgml/observation.py
- Create: python/src/mtgml/_observation_v1.py
- Create: python/src/mtgml/_knowledge.py
- Create: python/src/mtgml/_information_v2.py
- Create: python/src/mtgml/_events_v2.py
- Create: python/src/mtgml/_player_step_v2.py

- [ ] Move V1 envelopes and observation_digest_from_payload to _observation_v1.py.
- [ ] Move knowledge DTOs and provenance validation to _knowledge.py; InformationStateDigestInputV2 and PlayerInformationStateV2 to _information_v2.py; V2 events to _events_v2.py; and submission/PlayerStep V2 to _player_step_v2.py.
- [ ] Restore explicit facade imports for every current observation.py and mtgml.__init__ name, including all schema constants and the digest helper. Private modules must never import the facade.
- [ ] Run:

~~~powershell
scripts/run_python_tests.py --profile full
C:/Python313/python.exe -m unittest python/tests/test_m2_h_rules_free_guards.py
~~~

Expected: exit code 0, compatible imports, and a clean rules-free source/import scan.
- [ ] Run git diff --check, stage only the R3 Python modules/facade, run git diff --cached --check, and commit:

~~~text
refactor(python): split observation information internals
~~~

### Task 5: Implement R4 Python replay version ownership split

**Files:**

- Modify: python/src/mtgml/replay.py
- Create: python/src/mtgml/_replay_common.py
- Create: python/src/mtgml/_replay_v1.py
- Create: python/src/mtgml/_replay_v2.py
- Create: python/src/mtgml/_replay_v3.py

- [ ] Move shared identities to _replay_common.py without changing validation or wire rendering.
- [ ] Move Replay V1 declarations to _replay_v1.py, V2 declarations to _replay_v2.py, and all V3 declarations to _replay_v3.py. Keep each original validation order and error string.
- [ ] Restore every replay.py class and schema constant export. No version module may import through the facade or reinterpret another version.
- [ ] Run:

~~~powershell
scripts/run_python_tests.py --profile full
C:/Python313/python.exe -c "from mtgml.replay import AuthoritativeReplayV1, AuthoritativeReplayV2, AuthoritativeReplayV3, ReplayManifestV1, ReplayManifestV2, ReplayManifestV3, ReplayStepV1, ReplayStepV2, ReplayStepV3; print('PASS: replay facade imports')"
~~~

Expected: exit code 0, including all historical V1/V2 and current V3 replay tests.
- [ ] Run git diff --check, stage only the R4 Python modules/facade, run git diff --cached --check, and commit:

~~~text
refactor(python): split replay version internals
~~~

### Task 6: Verify compatibility and create the review-ready PR

**Files:** No source changes are permitted in this task.

- [ ] Run independently:

~~~powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
scripts/generate_contracts.py --check
scripts/verify_repository.py
scripts/check_rust_source_structure.py
scripts/check_documentation.py
scripts/validate_schemas.py
scripts/validate_maintainer_artifacts.py
scripts/validate_golden_path.py
scripts/run_python_tests.py --profile full
scripts/run_checks.py integration
git diff --check
~~~

Record actual exit status. A missing tool is NOT_RUN. A just wrapper failure caused by missing WSL /bin/bash is BLOCKED; native results remain separate.
- [ ] Audit git diff --name-status from the exact base plus git ls-files --others --exclude-standard. Only the four target facades/private modules and approved process artifacts may appear; no schemas, fixtures, generated vocabulary, cards, rules, M3, or conformance files.
- [ ] Confirm the eleven compatibility invariants from the design document with source diff, unchanged fixtures, focused tests, and rules-free guards.
- [ ] Push the branch, verify the remote branch SHA, and create a PR targeting master with title refactor: complete semantic-neutral module ownership cleanup and body containing Closes #162, all exact NO scope fields, statuses, gate results, and M3_AUTHORIZED = NO. Do not merge or close the issue.
