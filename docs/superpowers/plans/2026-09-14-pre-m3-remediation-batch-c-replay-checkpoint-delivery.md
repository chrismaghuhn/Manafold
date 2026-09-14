# Pre-M3 Remediation Batch C: Replay, Checkpoint, and Delivery Closure Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Characterize FND-021, FND-022, and FND-026 at their actual Rust ownership boundaries, fix only confirmed replay/counter/projection-validation defects, and record exact dispositions without changing wire schemas, historical replay meanings, or M3 scope.

**Architecture:** Keep `mtgml-replay` responsible for detached V3 structural identity and `mtgml-environment` responsible for checkpoint-anchored execution against the reconstructed authoritative request. Put the shared counter invariant on the model value used by both checkpoint and replay validation. Keep multi-perspective delivery read-only and fail closed at the production occurrence-projection boundary; do not add queues, mailboxes, or endpoint APIs.

**Tech Stack:** Rust workspace, `cargo test --locked`, canonical V3 replay/checkpoint DTOs, synthetic environment fixtures, production occurrence projection, Markdown evidence/design record.

---

## Scope and evidence rules

- Batch scope is only FND-021, FND-022, and FND-026.
- Preserve V1/V2 replay meaning, V3 wire shapes, checkpoint digest domains, RNG behavior, and M3 authorization status.
- Every confirmed production fix gets a focused RED test first, the exact pre-fix command/result is recorded, then the smallest GREEN change and nonmutation regression.
- Contract ambiguity is recorded as `BLOCKED_CONTRACT_AMBIGUITY`; it is not implemented through a guessed API.

### Task 1: Add the characterization and RED probes

**Files:**
- Modify: `crates/mtgml-environment/src/tests/checkpoint_replay.rs`
- Modify: `crates/mtgml-environment/src/tests/information_projection.rs`
- Modify: `crates/mtgml-environment/src/tests/error_nonmutation.rs` or `checkpoint_replay.rs` for closed trusted execution and exact counters
- Modify: `crates/mtgml-replay/src/tests.rs`

- [ ] Add a replay execution probe that clones a real accepted V3 step, changes only `response.player_decision_id`, reseals it as a structurally valid rejected diagnostic step, and asserts the authoritative source remains unchanged. Run:

```text
cargo test -p mtgml-environment --locked wrong_player_decision_id -- --exact --nocapture
```

Expected on the Batch-C base: FAIL because the step is accepted as a rejected diagnostic replay instead of being rejected at the request-bound boundary.

- [ ] Add a detached V3 initial-identity counter probe with a resealed checkpoint digest and `accepted_transitions > decisions_submitted`. Run:

```text
cargo test -p mtgml-replay --locked initial_identity_rejects_impossible_counters -- --exact --nocapture
```

Expected on the Batch-C base: FAIL because the initial identity validator accepts the impossible counter set.

- [ ] Add a production projector probe with two perspectives and one eventful occurrence per perspective. Assert both batches are constructed, each envelope is perspective-safe, actor and non-actor batches are distinct, and replay reprojection produces identical canonical bytes. Add a malformed non-actor observed envelope probe with a valid actor envelope. Run the exact focused test and record the base result before changing production code.

- [ ] Add passing base-characterization probes for terminal/truncated trusted execution, exact deterministic accepted counters, incomplete/foreign terminal outcomes, and resource/wall-clock replay progression. These tests must record current behavior without asserting an unapproved status-completeness or external-counter policy.

- [ ] Run the RED probes and commit the test-only characterization checkpoint:

```text
git add crates/mtgml-environment/src/tests crates/mtgml-replay/src/tests.rs
git commit -m "tests: characterize Batch-C replay and projection findings"
```

### Task 2: Bind replay responses to the authoritative pending request

**Files:**
- Modify: `crates/mtgml-environment/src/replay.rs`
- Modify: `crates/mtgml-environment/src/errors.rs`
- Test: `crates/mtgml-environment/src/tests/checkpoint_replay.rs`

- [ ] Before calling `execute_trusted_response`, compare `step.response.player_decision_id` with the current pending request's `player_decision_id` at the checkpoint-anchored replay boundary.
- [ ] Add a typed private `ReplayExecutionError` variant for this mismatch and preserve the existing actor error for actor mismatches.
- [ ] Keep trusted response shape/domain/candidate/continuation validation owned by the decision/rules boundary; do not duplicate it in replay.
- [ ] Run the focused wrong-identity test and the existing tampered-replay nonmutation matrix.
- [ ] Commit:

```text
git add crates/mtgml-environment/src/replay.rs crates/mtgml-environment/src/errors.rs crates/mtgml-environment/src/tests/checkpoint_replay.rs
git commit -m "fix: bind replay responses to authoritative requests"
```

### Task 3: Centralize structural environment-counter validation

**Files:**
- Modify: `crates/mtgml-model/src/lib.rs`
- Modify: `crates/mtgml-environment/src/checkpoint.rs`
- Modify: `crates/mtgml-replay/src/v3.rs`
- Test: model, checkpoint, and replay focused suites

- [ ] Add one model-owned structural validator for `EnvironmentLimitCounters` that rejects `accepted_transitions > decisions_submitted` and does not invent relationships for rule-event, resource, or wall-clock counters.
- [ ] Route `EnvironmentCheckpointV3::validate()` and `InitialEnvironmentIdentityV3::validate()` through that helper, mapping errors to their existing closed error layers.
- [ ] Keep accepted-step exact `+1/+1/+event_count` recomputation in the environment executor and keep detached resource/wall-clock validation monotonic/trace-oriented.
- [ ] Run the new RED test, focused model/replay/environment tests, and checkpoint nonmutation tests.
- [ ] Commit:

```text
git add crates/mtgml-model/src/lib.rs crates/mtgml-environment/src/checkpoint.rs crates/mtgml-replay/src/v3.rs
git commit -m "fix: close initial environment counter invariants"
```

### Task 4: Validate every projected observed-event envelope before commit

**Files:**
- Modify: `crates/mtgml-environment/src/lifecycle_projection.rs`
- Test: `crates/mtgml-environment/src/tests/information_projection.rs`

- [ ] Validate each constructed `ObservedEventEnvelopeV2` before placing it in the perspective batch, returning a typed `LifecycleProjectionError` on failure.
- [ ] Preserve read-only projection, opaque-ID substitution, sequence ownership, and existing actor-only return semantics.
- [ ] Run the malformed non-actor envelope RED test to GREEN, then run the two-perspective separation/reprojection and no-leak assertions.
- [ ] Commit:

```text
git add crates/mtgml-environment/src/lifecycle_projection.rs crates/mtgml-environment/src/tests/information_projection.rs
git commit -m "fix: validate every projected perspective envelope"
```

### Task 5: Record blocked and resolved slices

**Files:**
- Create: `docs/superpowers/specs/2026-09-14-pre-m3-remediation-batch-c-replay-checkpoint-delivery-design.md`

- [ ] Record the exact disposition of every required Batch-C slice, including the owning contract/boundary, focused test names, pre-fix result, post-fix result, and exact unresolved questions.
- [ ] Mark FND-022B blocked unless the current normative contract explicitly requires exactly one outcome for every `EngineState.core.players` member; do not implement foreign/missing outcome rejection on inference.
- [ ] Mark FND-022E split/blocked unless an accepted API contract defines how recorded resource/wall-clock progression is applied during replay; do not add a setter, hidden state, or schema field.
- [ ] Mark FND-026C blocked because no authorized non-actor delivery mechanism exists; do not add queues, mailboxes, polling, callbacks, or endpoint methods.
- [ ] State `M3_AUTHORIZED = NO` and all scope-accounting fields required by the task.

### Task 6: Verify, review, and deliver without merging

- [ ] Run focused package suites:

```text
cargo test -p mtgml-model --locked
cargo test -p mtgml-replay --locked
cargo test -p mtgml-environment --locked
cargo test -p mtgml-conformance --locked
```

- [ ] Run workspace verification:

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
```

- [ ] Run applicable maintainer/schema/Python checks and classify unavailable wrappers as `BLOCKED` or `NOT_RUN` rather than inferring success.
- [ ] Inspect the complete diff, tracked plus untracked scope, and exact local base/head identity.
- [ ] Request an independent exact-head review before creating the PR; resolve critical/important findings and rerun affected gates.
- [ ] Push the branch and open one PR against `master`; do not merge. Record hosted checks only if they ran on the exact PR head.

---

## Self-review checklist

- [ ] No production code was written before its confirmed RED test.
- [ ] No V1/V2 replay meaning, V3 wire shape, schema, digest domain, or RNG algorithm changed.
- [ ] No second replay executor, projector, hidden controller state, or player-visible trusted field was added.
- [ ] All exact `PASS`, `FAIL`, `BLOCKED`, and `NOT_RUN` labels are backed by commands actually run.
- [ ] The final disposition matrix includes every FND-021/FND-022/FND-026 sub-slice and ends with `M3_AUTHORIZED = NO`.

