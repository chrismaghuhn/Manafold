# Pre-M3 Remediation Batch D Implementation Plan

**Status:** provisional implementation plan for the PROPOSED Batch-D ADR

> For agentic workers: REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox syntax for tracking.

Goal: Resolve FND-002 and FND-006B with proposed contracts, RED-first characterization, minimal fail-closed validation, and exact evidence while preserving valid V3 bytes.

Architecture: Keep validate_engine_state() as the structural owner. Add one shared retained-record chronology helper, enforce canonical Top witnesses at live vector boundaries and retained location shapes, and keep digest/checkpoint code as validated consumers. Remove empty ordered-zone keys only in the fixture transition producer when the last member leaves; never normalize an input state during validation.

Tech Stack: Rust workspace, Cargo locked profiles, canonical CBOR V3 digest, serde, PowerShell, GitHub PR/Actions.

---

### Task 1: Record the approved Batch-D contract

Files:
- Create: docs/adr/candidates/2026-09-14-pre-m3-remediation-batch-d-knowledge-chronology-and-ordered-zone-canonicality.md
- Create: docs/superpowers/specs/2026-09-14-pre-m3-remediation-batch-d-knowledge-chronology-and-ordered-zone-canonicality-design.md
- Create: this implementation plan
- Modify: docs/INFORMATION_MODEL.md, docs/DOMAIN_MODEL.md, docs/contracts/ENGINE_STATE_CLOSURE.md, docs/STATE_HASHING.md, docs/M1_1_STATE_FOUNDATION_SPECIFICATION.md
- Modify: docs/OPEN_DECISIONS.md only if the existing register needs a Batch-D row
- Modify: docs/normative-document-register.v1.json for the new process artifacts

- [ ] Confirm the candidate states PROPOSED, documents exact acquisition/history/current/last-known/invalidation ordering, preserves same-occurrence equality, rejects noncanonical Bottom/Index, rejects empty keys, and records WIRE_CHANGE = NO, SCHEMA_CHANGE = NO, DIGEST_DOMAIN_CHANGE = NO, and VALID_FIXTURE_DIGEST_CHANGE = NO.
- [ ] Update normative prose consistently; do not claim the candidate is accepted or that M3 is authorized.
- [ ] Run python scripts/check_documentation.py after all documentation paths exist.
- [ ] Inspect the diff and commit only documentation/contract artifacts as docs: characterize Batch-D contract gaps.

### Task 2: Add FND-002 and FND-006B RED characterization

Files:
- Create: crates/mtgml-state/src/tests/batch_d.rs
- Modify: crates/mtgml-state/src/tests.rs
- Modify: crates/mtgml-state/src/tests/digest.rs
- Create or modify: crates/mtgml-environment/src/tests/batch_d.rs
- Modify: crates/mtgml-environment/src/tests.rs

- [ ] Add a two-object ordered-state helper that starts from synthetic_state(), adds GameObjectId(3) with a unique physical/card identity, assigns Top { offset: 1 }, appends it to the existing library vector, and advances next_object_id to GameObjectId(4).
- [ ] Add chronology cases and record their BASE result before the fix:

  | Test case | Approved result | Expected BASE |
  |---|---:|---:|
  | acquisition sequence later than a historical fact | REJECTED | ACCEPTED |
  | locally increasing history with first fact before acquisition | REJECTED | ACCEPTED |
  | active current older than newest history | REJECTED | ACCEPTED |
  | retired last-known older than newest history | REJECTED | ACCEPTED |
  | invalidation earlier than acquisition | REJECTED | ACCEPTED |
  | invalidation earlier than newest history | REJECTED | ACCEPTED |
  | invalidation earlier than last-known | REJECTED | ACCEPTED |
  | same full provenance for Acquire acquisition/current | ACCEPTED | ACCEPTED |
  | InitialConfiguration acquisition plus initial-prefix/observed history | ACCEPTED | ACCEPTED |
  | visible sequence gap in retained history | ACCEPTED | ACCEPTED |
  | observed provenance equal to next sequence | REJECTED | REJECTED |
  | observed provenance beyond next sequence | REJECTED | REJECTED |

- [ ] Add a lifecycle test that applies Acquire at sequence 4 with an observed current fact, then UpdateLocation at sequence 7; assert acquisition sequence 4, history sequence 4, current sequence 7, and successful validation.
- [ ] Add ordered-zone cases and record BASE behavior:
  - vector membership correct but Index values swapped;
  - two members both claim Top { offset: 0 };
  - out-of-range Index;
  - out-of-range Top;
  - out-of-range Bottom;
  - equivalent vector order represented by Index instead of Top;
  - valid vector plus unrelated empty key with declared player;
  - empty key with another valid declared key;
  - existing unordered-in-vector, ordered-missing, and duplicate-member controls.
- [ ] Add assertions that every rejected validator case leaves the cloned state unchanged.
- [ ] Run cargo test -p mtgml-state --locked batch_d and the focused environment test command before production edits. The new contract-rejection tests must fail because the BASE validator currently accepts the malformed states; existing future-bound and membership controls must remain green.
- [ ] Commit the RED evidence as tests: reproduce Batch-D chronology and ordering ambiguities.

### Task 3: Implement the shared FND-002 chronology validator

Files:
- Modify: crates/mtgml-state/src/m2_shape/knowledge.rs
- Modify: crates/mtgml-state/src/m2_shape.rs only if a crate-visible helper re-export is needed
- Modify: crates/mtgml-state/src/validation/information.rs

- [ ] Implement one helper over acquisition, historical facts, optional current/last-known fact, and optional invalidation. It must:
  - reject more than one InitialConfiguration location fact;
  - reject an initial location fact after an observed fact;
  - reject initial location facts when observed acquisition already establishes a later lower bound;
  - require observed history to increase strictly;
  - allow acquisition/location equality only when the complete provenance values are equal;
  - require each later current/last-known observed fact to be newer than history and acquisition unless it is the identical Acquire-created fact with no later location;
  - require invalidation to be observed and strictly newer than every prior observed retained fact.
- [ ] Call the same helper for active and retired records from local M2 shape validation. Keep future bounds and channel/cause validation unchanged.
- [ ] Remove duplicate historical-window chronology checks from validation/information.rs; leave live-location, player-reference, mapping, and retirement checks there.
- [ ] Keep validation read-only and return the existing typed M2ShapeViolation::Knowledge or VisibleSequence categories without adding wire fields.
- [ ] Run the chronology RED tests and cargo test -p mtgml-state --locked; all focused cases must now match the approved result matrix.
- [ ] Commit as fix: enforce retained-knowledge chronology.

### Task 4: Implement FND-006B canonical ordered-zone validation

Files:
- Modify: crates/mtgml-state/src/validation/zones.rs
- Modify: crates/mtgml-state/src/m2_shape/knowledge.rs
- Modify: crates/mtgml-rules/src/fixture_support.rs
- Modify: crates/mtgml-state/src/tests/digest.rs

- [ ] Reject any empty ordered_zones vector before iterating members.
- [ ] For every ordered vector member, require a matching location with the same key and exactly ZonePosition::Top { offset }, where offset equals the zero-based vector ordinal and therefore is less than vector length.
- [ ] Preserve the existing set-based checks for ordered membership, exact uniqueness, unordered exclusion, key equality, and declared players.
- [ ] Reject Bottom and Index in every persisted ordered ZoneLocation, including retained historical and last-known facts. Leave Unordered valid for unordered facts.
- [ ] When fixture transition support removes the last member from an ordered vector, remove the map key from the workspace. Do not add any normalization to validate_engine_state().
- [ ] Replace the old valid Bottom digest-mutation case with:
  1. a negative Bottom/Index digest test expecting StateDigestError::StateInvariant;
  2. a valid two-object canonical reorder whose matching Top offsets change and whose digest differs.
- [ ] Run state tests and digest tests; verify the frozen canonical reset payload/digest remains byte-for-byte unchanged.
- [ ] Commit as fix: enforce ordered-zone canonicality.

### Task 5: Add digest/checkpoint closure evidence

Files:
- Modify: crates/mtgml-state/src/tests/batch_d.rs or crates/mtgml-state/src/tests/digest.rs
- Create or modify: crates/mtgml-environment/src/tests/batch_d.rs
- Modify: crates/mtgml-environment/src/tests.rs

- [ ] Assert that every invalid chronology and ordered-zone state returns StateDigestError::StateInvariant through the normal V3 digest path.
- [ ] Construct an EnvironmentCheckpointV3 with an invalid Bottom or empty-key state and assert CheckpointValidationError::StateDigest; do not bypass the normal checkpoint constructor.
- [ ] Verify a valid synthetic reset and valid two-object reorder still validate and digest normally.
- [ ] Run cargo test -p mtgml-state --locked, cargo test -p mtgml-environment --locked, and cargo test -p mtgml-conformance --locked.
- [ ] Commit as tests: prove Batch-D digest and checkpoint closure.

### Task 6: Update evidence and run workspace gates

Files:
- Modify: Batch-D ADR candidate/disposition documentation
- Modify: docs/normative-document-register.v1.json if final evidence paths are added

- [ ] Record the exact BASE, HEAD, branch, PR, BASE matrix, RED output, focused test counts, and all remaining statuses. Keep ADR status PROPOSED.
- [ ] Run:

  cargo fmt --all -- --check
  cargo check --workspace --all-targets --all-features --locked
  cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
  cargo test --workspace --all-features --locked

- [ ] Run applicable Python/schema/maintainer/integration checks. If just check-fast or just check cannot start because /bin/bash is unavailable, record both as BLOCKED and report direct constituent evidence separately.
- [ ] Inspect tracked plus untracked scope, staged/unstaged diffs, valid fixture digest output, and git status --porcelain.
- [ ] Commit as docs: record Batch-D dispositions and evidence.

### Task 7: Open exactly one PR without merging

Files: None beyond the committed branch.

- [ ] Push chris/pre-m3-remediation-batch-d-state-contract-resolution to origin.
- [ ] Open one PR against master titled Pre-M3 remediation Batch D: knowledge chronology and ordered-zone closure.
- [ ] Confirm hosted CI for the exact final PR head only.
- [ ] Do not merge. Report the next action as independent exact-head review and keep M3_AUTHORIZED = NO.
