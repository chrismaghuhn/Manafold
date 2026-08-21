# M1.6 Checkpoint, Fork, and Replay Implementation Plan

**Status:** provisional

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox syntax for tracking.

**Goal:** Add one concrete synthetic environment transaction owner with exact V2 checkpoint, restore, fork, replay-segment, and complete rejection-nonmutation evidence.

**Architecture:** SyntheticM1EnvironmentBackend owns the complete environment product and invokes the existing SyntheticM1RulesKernel. ReplayRecorderV2 remains a small structural helper in mtgml-replay; semantic replay execution runs in an isolated environment backend and uses the same trusted transaction method as live execution. Restore and fork create a new empty replay segment rooted at the complete checkpoint.

**Tech Stack:** Rust 1.85.1, Cargo.lock, mtgml-rules, mtgml-state, mtgml-random, mtgml-replay, mtgml-wire, serde, SHA-256 V2 digests, and the repository just/Python verification profiles.

---

**Starting master:** a1545c5f8846d2a4780506c8f323f81ca4698cd5
**Branch:** chris/m1-6-checkpoint-fork-replay
**Worktree:** C:\Users\chris\.config\superpowers\worktrees\Manafold\m1-6-checkpoint-fork-replay

## Files and responsibilities

- Modify docs/adr/0028-complete-environment-checkpoints.md: correct the stale V1 label to the current V2 contract without changing fields or semantics.
- Modify docs/normative-document-register.v1.json: register the M1.6 design and plan.
- Create docs/superpowers/specs/2026-08-21-m1-6-checkpoint-fork-replay-design.md: reconciled design and segment-root decision.
- Create this plan at docs/superpowers/plans/2026-08-21-m1-6-checkpoint-fork-replay.md.
- Create docs/superpowers/m1-6-pr-body.md after final verification: exact Draft PR evidence and status table.
- Modify crates/mtgml-replay/src/validation.rs: add narrow typed recorder/manifest/semantic mismatch errors only where existing structural errors cannot express the failure.
- Create crates/mtgml-replay/src/recorder.rs: fallible ReplayRecorderV2 with accepted-step append, empty segment creation, and validated export.
- Modify crates/mtgml-replay/src/lib.rs and src/tests.rs: expose the recorder and prove V2 continuity/rejection behavior.
- Modify crates/mtgml-environment/Cargo.toml: add the runtime mtgml-rules dependency and test-only mtgml-wire dependency.
- Modify crates/mtgml-environment/src/errors.rs: add typed trusted kernel/checkpoint/replay/overflow/transaction errors and semantic replay divergence errors.
- Modify crates/mtgml-environment/src/controller.rs: add trusted execution and isolated semantic replay entry points without changing the player endpoint trait.
- Create crates/mtgml-environment/src/synthetic.rs: own explicit synthetic configuration, backend state, atomic transaction, checkpoint restore, and checkpoint-based fork.
- Create crates/mtgml-environment/src/replay.rs: own trusted replay trace/report types and first-divergence comparison helpers.
- Modify crates/mtgml-environment/src/lib.rs: register modules and export only trusted M1.6 types.
- Modify crates/mtgml-environment/src/tests.rs: add the complete M1.6 focused evidence using the real backend/kernel.

No player schema, PlayerEndpoint method, ReplayStepV2 field, V2 checkpoint field, generated vocabulary, or RulesKernel semantic behavior is changed.

## Task 1: Commit the reconciled process artifacts

**Files:** the four documentation files listed above.

- [ ] Step 1: Verify isolated source identity.

Run:

~~~
git branch --show-current
git rev-parse HEAD
git rev-parse origin/master
git status --short --branch
~~~

Expected: the requested branch, HEAD and origin/master both a1545c5f8846d2a4780506c8f323f81ca4698cd5, and no source changes.

- [ ] Step 2: Add the design, plan, register entry, and ADR version correction.

Keep the design decision explicit: EnvironmentCheckpointV2 has no replay prefix, restore starts a new segment at the checkpoint, and live rejected submissions do not append recorder steps. Change only the ADR's V1 contract name to V2; preserve its complete-checkpoint ownership and validation-before-mutation text. Add both M1.6 process paths to the register with role=process, stability=provisional, and change_process=process-pr.

- [ ] Step 3: Validate and commit only process artifacts.

Run:

~~~
python -m json.tool docs/normative-document-register.v1.json
python scripts/check_documentation.py
git diff --check
~~~

Expected: all commands exit 0.

~~~
git add -- docs/adr/0028-complete-environment-checkpoints.md docs/normative-document-register.v1.json docs/superpowers/specs/2026-08-21-m1-6-checkpoint-fork-replay-design.md docs/superpowers/plans/2026-08-21-m1-6-checkpoint-fork-replay.md
git commit -m "docs: define M1.6 checkpoint and replay semantics"
~~~

## Task 2: Add the replay recorder test-first

**Files:** crates/mtgml-replay/src/recorder.rs, src/lib.rs, src/validation.rs, src/tests.rs.

- [ ] Step 1: Write RED tests for an empty segment and accepted append.

Build a valid V2 manifest with initial revision 0 and a known digest. Assert ReplayRecorderV2::new(manifest) exports a valid empty replay whose final identity equals the manifest. Append one exact accepted ReplayStepV2 with step index 0, before revision 0, response revision 0, after revision 1, and the committed after digest. Assert export has one step and final revision 1.

- [ ] Step 2: Run the focused RED test.

~~~
cargo test -p mtgml-replay --locked recorder
~~~

Expected: compilation fails because ReplayRecorderV2 is not defined.

- [ ] Step 3: Implement the smallest validated recorder.

Use this public shape:

~~~
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayRecorderV2 {
    manifest: ReplayManifestV2,
    steps: Vec<ReplayStepV2>,
    final_state_revision: StateRevision,
    final_state_digest: FullStateDigestV2,
}

impl ReplayRecorderV2 {
    pub fn new(manifest: ReplayManifestV2) -> Result<Self, ReplayValidationError>;
    pub fn append(&mut self, step: ReplayStepV2) -> Result<(), ReplayValidationError>;
    pub fn export(&self) -> Result<AuthoritativeReplayV2, ReplayValidationError>;
    pub fn manifest(&self) -> &ReplayManifestV2;
    pub fn step_count(&self) -> usize;
}
~~~

new validates the manifest and roots final identity at the manifest. append clones the current steps, builds a candidate AuthoritativeReplayV2, calls validate, and replaces the vector/final identity only after validation. It rejects wrong index, before revision, response revision, accepted after revision, or rejected after identity without mutating the recorder.

- [ ] Step 4: Run recorder tests and the replay crate.

~~~
cargo test -p mtgml-replay --locked recorder
cargo test -p mtgml-replay --locked
~~~

Expected: all recorder and existing V1/V2 structural tests pass.

- [ ] Step 5: Commit the recorder.

~~~
git add -- crates/mtgml-replay/src/recorder.rs crates/mtgml-replay/src/lib.rs crates/mtgml-replay/src/validation.rs crates/mtgml-replay/src/tests.rs
git commit -m "feat: add validated V2 replay recorder"
~~~

## Task 3: Add typed trusted environment APIs test-first

**Files:** environment Cargo manifest, errors.rs, controller.rs, replay.rs, lib.rs, and focused tests.

- [ ] Step 1: Write RED API tests.

Add tests that construct a backend, call TrustedEnvironmentController::execute_trusted_response, and call execute_replay_from_checkpoint on a checkpoint/replay pair. Assert the returned report contains per-step before/after checkpoints and a final checkpoint. Keep endpoint calls unchanged and verify the concrete backend's player-facing methods return PlayerApiError::Unavailable.

- [ ] Step 2: Run the focused RED test.

~~~
cargo test -p mtgml-environment --locked trusted_execution
~~~

Expected: compilation fails because the concrete backend/controller/report types do not exist.

- [ ] Step 3: Add dependencies and typed errors.

Add runtime mtgml-rules = { path = "../mtgml-rules" } and test-only mtgml-wire = { path = "../mtgml-wire" }. Add ControllerError variants wrapping KernelExecutionError, CheckpointValidationError, ReplayValidationError, and TransitionViolation, plus typed UnsupportedCheckpointCodec, CounterOverflow { counter: &'static str }, ReplayIdentityMismatch, and ReplayExecution(ReplayExecutionError).

Define ReplayExecutionError with exact step-indexed variants for manifest mismatch, actor unavailable, before revision/digest mismatch, outcome mismatch, after revision/digest mismatch, transition product mismatch, and final identity mismatch. None of these errors is mapped into player errors.

- [ ] Step 4: Add trusted controller/report signatures.

Use:

~~~
pub fn execute_trusted_response(
    &self,
    actor: PlayerId,
    response: DecisionResponse,
) -> Result<TransitionResult, ControllerError>;

pub fn execute_replay_from_checkpoint(
    &self,
    checkpoint: EnvironmentCheckpointV2,
    replay: AuthoritativeReplayV2,
) -> Result<ReplayExecutionReport, ControllerError>;

pub struct ReplayExecutionTrace {
    pub step_index: u64,
    pub before: EnvironmentCheckpointV2,
    pub transition: TransitionResult,
    pub after: EnvironmentCheckpointV2,
}

pub struct ReplayExecutionReport {
    pub traces: Vec<ReplayExecutionTrace>,
    pub final_checkpoint: EnvironmentCheckpointV2,
}
~~~

The controller creates a private fork, restores the supplied checkpoint, and executes all steps there. It never runs replay against the caller's backend.

- [ ] Step 5: Run the trusted API tests.

~~~
cargo test -p mtgml-environment --locked trusted_execution
~~~

Expected: only the missing concrete backend prevents green.

## Task 4: Implement explicit synthetic backend construction/checkpoint/restore/fork

**Files:** create crates/mtgml-environment/src/synthetic.rs; modify lib.rs and tests.

- [ ] Step 1: Write RED construction and checkpoint tests.

Use fixed players [PlayerId(1), PlayerId(2)] and an explicit root seed. Construct SyntheticM1EnvironmentBackend, capture a checkpoint, and assert revision 0, a pending decision, RNG cursor 0, effect/event cursors 1, and all counters 0. Assert a checkpoint roundtrip into a fresh compatible backend yields exact checkpoint equality. Add tests that tamper state, a counter, and codec/digest, and assert restore errors leave the original checkpoint unchanged.

- [ ] Step 2: Run the RED construction tests.

~~~
cargo test -p mtgml-environment --locked synthetic_backend
~~~

Expected: compilation fails because the backend/configuration constructors do not exist.

- [ ] Step 3: Define explicit static configuration and backend fields.

Use:

~~~
pub struct SyntheticM1EnvironmentConfig {
    pub codec: CheckpointCodecIdentity,
    pub replay: SyntheticM1ReplayConfig,
}

pub struct SyntheticM1ReplayConfig {
    pub engine_build: String,
    pub kernel: KernelIdentityV1,
    pub rules_snapshot: String,
    pub format_policy_snapshot: String,
    pub oracle_snapshot: String,
    pub card_bundle: String,
    pub schemas: ReplaySchemaVersionsV1,
    pub decks: Vec<DeckIdentityV1>,
}
~~~

Construct with new(players, root_seed, config). Validate exactly two distinct players through construct_synthetic_engine_state, require deck identities to match state players, and create the initial V2 manifest from actual checkpoint state/digest/root seed. Store no checkpoint cache.

- [ ] Step 4: Implement checkpoint and restore atomically.

checkpoint() calls EnvironmentCheckpointV2::new from the five current environment-owned values. restore() calls checkpoint.validate(), requires checkpoint.codec == self.config.codec, validates replay identity against checkpoint players, builds a new manifest and empty recorder, and only then assigns state/status/counters/recorder. Concrete player-facing methods return PlayerApiError::Unavailable.

- [ ] Step 5: Implement checkpoint-based fork.

fork_boxed() obtains self.checkpoint(), constructs a new backend from the complete checkpoint and cloned static configuration, and returns a boxed independent backend. The new recorder is empty and rooted at that checkpoint.

- [ ] Step 6: Run construction/restore/fork tests.

~~~
cargo test -p mtgml-environment --locked synthetic_backend
cargo test -p mtgml-environment --locked
~~~

Expected: all current and new checkpoint/fork construction tests pass.

- [ ] Step 7: Commit the backend foundation.

~~~
git add -- crates/mtgml-environment/Cargo.toml crates/mtgml-environment/src/synthetic.rs crates/mtgml-environment/src/controller.rs crates/mtgml-environment/src/errors.rs crates/mtgml-environment/src/replay.rs crates/mtgml-environment/src/lib.rs crates/mtgml-environment/src/tests.rs
git commit -m "feat: add synthetic environment checkpoint owner"
~~~

## Task 5: Implement the atomic transaction and rejection closure test-first

**Files:** synthetic.rs, controller.rs, errors.rs, tests.rs.

- [ ] Step 1: Write RED accepted/rejected/overflow tests.

For an exact accepted response assert revision 1, life 38, pending decision absent, RNG cursor 1, next_effect_id 2, next_rule_event_id 5, four ordered events, counters (1,1,4,0,0), one accepted replay step, and exact replay step digest.

For a structurally valid unknown-candidate response, capture checkpoint, accepted replay export, and canonical bytes before execution; execute through the trusted path; assert the rejected result and exact equality of every captured value afterward. For overflow set decisions_submitted=u64::MAX in a valid current environment and assert a typed overflow error plus complete state/checkpoint/replay equality.

- [ ] Step 2: Run the RED transaction tests.

~~~
cargo test -p mtgml-environment --locked accepted_environment
cargo test -p mtgml-environment --locked rejected_environment
cargo test -p mtgml-environment --locked counter_overflow
~~~

Expected: tests fail until the atomic transaction exists.

- [ ] Step 3: Implement execute_trusted_response on the backend.

The implementation order is: checkpoint current state; invoke RulesKernel::apply with actor and response; validate the returned transition contract; for a rejection, construct and compare an unchanged checkpoint and return without assignment; for acceptance, checked-increment decisions_submitted and accepted_transitions, checked-add events.len() to rule_events_emitted, build a candidate V2 checkpoint, build the exact accepted ReplayStepV2, append it to a cloned recorder, export/validate the candidate replay, and only then assign all state/status/counters/recorder fields. Resource and wall-clock counters remain unchanged. Never append rejected steps to the live recorder.

- [ ] Step 4: Run focused transaction tests and rules/state regressions.

~~~
cargo test -p mtgml-environment --locked accepted_environment
cargo test -p mtgml-environment --locked rejected_environment
cargo test -p mtgml-environment --locked counter_overflow
cargo test -p mtgml-rules --locked
cargo test -p mtgml-state --locked
~~~

Expected: all pass with M1.5 transition behavior unchanged.

- [ ] Step 5: Commit the transaction.

~~~
git add -- crates/mtgml-environment/src/synthetic.rs crates/mtgml-environment/src/controller.rs crates/mtgml-environment/src/errors.rs crates/mtgml-environment/src/tests.rs
git commit -m "feat: add atomic synthetic environment transactions"
~~~

## Task 6: Add exact checkpoint/restore and fork parity evidence

**Files:** crates/mtgml-environment/src/tests.rs; modify a runtime helper only when a failing parity assertion identifies a concrete missing comparison.

- [ ] Step 1: Write continuation parity tests.

Run the required sequence: normal rejection, checkpoint C0, accepted response, capture transition/C1/R1, restore C0, repeat the exact response, and compare transition/C1/R1 including canonical bytes, events/order, delta, status, counters, RNG cursor, and allocator cursors. Then restore accepted C1 into a fresh backend and assert exact checkpoint identity plus an empty replay with manifest/final revision 1 and the restored digest.

- [ ] Step 2: Write fork tests.

Fork twice from C0 and compare complete checkpoints, status/counters, empty segment manifests, and canonical replay bytes. Apply the same accepted response to both and compare complete transition/checkpoint/replay products. Fork again, apply accepted to one and a structurally valid unknown-candidate rejection to the other; assert only explicit input caused divergence and the original source remains at C0.

- [ ] Step 3: Run parity tests.

~~~
cargo test -p mtgml-environment --locked checkpoint_restore
cargo test -p mtgml-environment --locked fork
~~~

Expected: all exact parity assertions pass.

- [ ] Step 4: Commit parity evidence.

~~~
git add -- crates/mtgml-environment/src/tests.rs
git commit -m "test: prove checkpoint restore and fork parity"
~~~

## Task 7: Implement semantic replay and tamper tests

**Files:** controller.rs, replay.rs, errors.rs, environment tests.

- [ ] Step 1: Write RED semantic replay tests.

Export a live accepted replay from C0 and call execute_replay_from_checkpoint(C0, replay). Assert the report trace matches the live transition exactly and its final checkpoint equals live C1. Add a pure diagnostic replay with one unknown-candidate rejected step and assert revision/digest/checkpoint identity are unchanged while the live recorder stays empty. Add negative cases for wrong initial digest, wrong root seed, tampered accepted-step after digest, flipped accepted flag, and stale response; each returns a typed first-divergence error and leaves the caller checkpoint and replay unchanged.

- [ ] Step 2: Run RED replay tests.

~~~
cargo test -p mtgml-environment --locked semantic_replay
cargo test -p mtgml-environment --locked replay_tamper
~~~

Expected: compilation or assertion failures identify the missing runner.

- [ ] Step 3: Implement isolated replay execution.

Validate AuthoritativeReplayV2 first. Validate the start checkpoint and compare manifest dynamic fields (revision, digest, root seed) and all static config identity/decks. Fork/restore a private backend and compare its empty segment manifest with the replay manifest. For each step, compare current revision and digest, derive pending_decision.request.actor, call the backend's trusted execution method, and compare accepted flag, after revision, and after digest. Record before/transition/after in ReplayExecutionTrace. Stop immediately on the first mismatch. At the end compare final revision/digest and return the isolated final checkpoint. Never call trusted execution on the caller backend.

- [ ] Step 4: Run replay and canonical tests.

~~~
cargo test -p mtgml-environment --locked semantic_replay
cargo test -p mtgml-environment --locked replay_tamper
cargo test -p mtgml-wire --locked
cargo test -p mtgml-replay --locked
~~~

Expected: live/replayed exact products, diagnostic rejection identity, canonical encode/decode, and all tamper failures pass.

- [ ] Step 5: Commit semantic replay.

~~~
git add -- crates/mtgml-environment/src/controller.rs crates/mtgml-environment/src/replay.rs crates/mtgml-environment/src/errors.rs crates/mtgml-environment/src/tests.rs
git commit -m "feat: add isolated semantic replay execution"
~~~

## Task 8: Final verification and delivery evidence

**Files:** no further source changes after the final archive gate.

- [ ] Step 1: Run focused and required Rust checks.

~~~
cargo test -p mtgml-rules --locked
cargo test -p mtgml-state --locked
cargo test -p mtgml-random --locked
cargo test -p mtgml-replay --locked
cargo test -p mtgml-wire --locked
cargo test -p mtgml-environment --locked
cargo test --workspace --all-features --locked
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
~~~

- [ ] Step 2: Run repository checks.

~~~
python scripts/run_checks.py fast
python scripts/check_documentation.py
python scripts/verify_repository.py
just check-fast
just check
~~~

Record each actual result. An unavailable WSL2/Hyper-V profile is BLOCKED, not PASS; hosted CI is not inferred from local output.

- [ ] Step 3: Audit exact scope and source state.

~~~
git diff --check origin/master...HEAD
git diff --stat origin/master...HEAD
git diff --name-only origin/master...HEAD
git status --short --branch
~~~

Confirm no M1.7 endpoint binding, M2 information-safety work, new decision, real cards, V3 contract, hidden replay cache, duplicate codec, or second rules engine was added.

- [ ] Step 4: Push and open one Draft PR without merging.

After all local evidence is captured:

~~~
git push --set-upstream origin chris/m1-6-checkpoint-fork-replay
gh pr create --draft --base master --head chris/m1-6-checkpoint-fork-replay --title "M1.6: checkpoint, fork, and replay parity" --body-file docs/superpowers/m1-6-pr-body.md
~~~

The PR body must include starting/final SHA, changed files, transaction and counter policy, segment-root semantics, checkpoint/restore/fork/replay evidence, rejected diagnostic replay, canonical roundtrip, tamper cases, environment rejection closure, RNG/allocator continuity, information-safety assessment, M1.7 non-goal, and an exact PASS/FAIL/BLOCKED/NOT_RUN command table. Use Closes #25 only when the four M1.6 outcomes actually pass. Do not merge.

## Plan self-review

- The no-prefix checkpoint decision is explicit in the design and Tasks 1, 4, 6, and 7; no hidden prefix owner is permitted.
- Accepted-only live recording and derived rejected diagnostics are covered by Tasks 2, 5, 6, and 7.
- Complete checkpoint identity, codec rejection, restore atomicity, and fork independence are covered by Tasks 4 and 6.
- The same RulesKernel path is used for live and replay execution in Tasks 5 and 7; no rules dependency is added to mtgml-replay.
- Counter overflow and exact environment nonmutation are covered before commit in Task 5.
- M1.7 endpoint binding remains explicitly excluded and is never marked PASS.
- The final status table reports only executed commands and preserves unavailable or unexecuted gates as BLOCKED or NOT_RUN.
