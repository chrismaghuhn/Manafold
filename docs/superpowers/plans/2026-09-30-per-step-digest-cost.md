# Per-Step Digest Cost Implementation Plan

**Status:** DRAFT for owner review, 2026-09-30. Implementation starts after
PR #261 (`feat/production-full-turn`) is merged, on a branch
`perf/digest-once-per-step` off `master`.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A production step computes each full-state digest once and validates the state once per query batch. Games play at least twice as fast, with byte-identical results.

**Architecture:**
- Today one accepted `submit` hashes the full state about 10 times, although it has only two distinct states (before and after). The cost is 2 each for:
  - the before checkpoint (rebuilt every call);
  - `between_structural_only`;
  - the rules' self-check via `apply_structural_only`;
  - the runtime's re-apply;
  - the after checkpoint (`build` hashes, then its own validation hashes again).
- In addition, `S1QueryAuthority::for_object` validates the whole state once per queried object.
- The plan removes only recomputations of a value computed in the same step from the same bytes. Every check on input from outside the runtime stays: checkpoint restore, replay steps, and `StateDeltaV3::apply` on foreign deltas.

**Tech Stack:** Rust workspace; crates `mtgml-rules`, `mtgml-environment`.

**Spec:** No separate spec. The authority is `AGENTS.md` (priorities: correctness, then determinism, and so on; performance comes after them) plus this evidence. The profile was recorded 2026-09-30 per `docs/agents/profiling.md`, on the release random smoke game at commit `db775ce`:
- `calculate_full_state_digest_v7_structural_only`: 72.5% of samples. Nearest callers:
  - `StateDeltaV3::apply_structural_only`: 30%
  - `EnvironmentCheckpointV8::build`: 27%
  - `EnvironmentCheckpointV8::validate_inner`: 27%
  - `StateDeltaV3::between_structural_only`: 16%
- `S1QueryAuthority::for_object`: 7–9%, of which 99.5% is `EngineStatePartsV3::validate_components`.
- Per-step time at turn 30 was 23–27 ms. `random_games_run_thirty_turns_deterministically_and_replay` took 63.6 s under samply.

## Global Constraints

- **No output byte changes.** State, delta, event, checkpoint, replay and player-step bytes stay identical. Existing digest, replay, parity and determinism tests must pass unchanged. No test expectation may be edited, except where a task names it.
- **Checks on external input stay.** `EnvironmentCheckpointV8::validate*`, `restore*`, `execute_replay`, `StateDeltaV3::apply` and `StateDeltaV3::apply_structural_only` keep recomputing digests.
- **Same errors.** Error variants returned for invalid input stay the same.
- **Tests go through the production path.** Production-path tests run with `cargo test -p mtgml-environment --locked`. The release random smoke stays green (`AGENTS.md` §5).
- **Commit before running gate scripts.**
- **Performance is evidence, not a test.** No timing thresholds in tests. Before/after numbers are measured on the same machine and recorded in the PR (Task 5).

## Review Focus

1. **Restore, then continue:** after `restore(checkpoint)`, `checkpoint()` must return exactly the restored checkpoint, and the next `submit` must build on it. Pinned in Task 4 by `restored_runtime_continues_from_the_restored_checkpoint`.
2. **Rejected responses:** the checkpoint and the replay stay unchanged. Already pinned by `current_successor_api.rs` (wrong actor, fabricated candidate); it must stay green in Task 4.
3. **Replay execution:** it compares each step's before digest with `runtime.checkpoint()`. A cached checkpoint must equal a rebuilt one. Pinned in Task 2 by `checkpoints_stay_bound_to_their_state_digest`, and by the existing ten-turn replay test.
4. **Batch S1 query on an invalid state:** the error must equal what querying the first object alone returns. Pinned in Task 1.
5. **A built delta checked against the wrong after-state** must be rejected. Pinned in Task 3.

---

### Task 1: S1 queries validate the state once per batch

**Files:**
- Modify: `crates/mtgml-rules/src/characteristic_query.rs` (`S1QueryAuthority`)
- Modify: `crates/mtgml-rules/src/turn_progression.rs` (`validate_slice`)
- Modify: `crates/mtgml-rules/src/basic_land_v4.rs` (`derive_basic_land_candidates_v4`)
- Test: `crates/mtgml-rules/src/basic_land.rs` (tests module, next to `s1_a_resolves_exact_admitted_live_object_without_mutation`)

**Interfaces:**
- Produces: `pub(crate) fn for_objects(admission: &'a ExecutableProfileAdmissionV1, state: &'a EngineStatePartsV3, objects: &[GameObjectId]) -> Result<Vec<Self>, S1QueryError>`.
  - The result equals calling `for_object` on each object in order and stopping at the first error.
  - An empty slice returns `Ok(Vec::new())` without any validation.
  - Otherwise, `validate_admission_binding` and `state.validate_structure()` run once. An invalid state is classified with the first object exactly as `for_object` does.
  - `for_object(a, s, o)` becomes `for_objects(a, s, &[o])` taking the single element.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn s1_query_for_objects_matches_one_query_per_object() {
    let admission = admission();
    let state = successor_state_with_two_lands();
    let objects: Vec<_> = state.predecessor_v5.zones.objects.iter()
        .filter(|(_, object)| !object.face_down).map(|(id, _)| *id).collect();
    assert!(objects.len() >= 2);
    let batch = crate::S1QueryAuthority::for_objects(&admission, &state, &objects).unwrap();
    let single: Vec<_> = objects.iter().map(|object| {
        crate::S1QueryAuthority::for_object(&admission, &state, *object).unwrap().queried_object()
    }).collect();
    assert_eq!(batch.iter().map(|a| a.queried_object()).collect::<Vec<_>>(), single);
    assert!(crate::S1QueryAuthority::for_objects(&admission, &state, &[]).unwrap().is_empty());

    let mut missing_location = state.clone();
    missing_location.predecessor_v5.zones.locations.remove(&objects[1]);
    for order in [[objects[0], objects[1]], [objects[1], objects[0]]] {
        assert_eq!(
            crate::S1QueryAuthority::for_objects(&admission, &missing_location, &order).unwrap_err(),
            crate::S1QueryAuthority::for_object(&admission, &missing_location, order[0]).unwrap_err(),
        );
    }
}
```

- [ ] **Step 2: Run it.** `cargo test -p mtgml-rules --lib --locked s1_query_for_objects`. Expected: compile error `no function or associated item named 'for_objects'`.
- [ ] **Step 3: Implement `for_objects`.** Move the per-object body of `for_object` after the validation into a private helper that the batch calls per object. Then switch both production loops to one `for_objects` call over the collected objects:
  - `validate_slice`: the battlefield objects, mapping errors to `TurnProgressUnsupported` as before;
  - `derive_basic_land_candidates_v4`: the candidate source objects, keeping the per-authority identity check.
- [ ] **Step 4: Run.** `cargo test -p mtgml-rules --lib --locked` and `cargo test -p mtgml-environment --locked`. Expected: all pass.
- [ ] **Step 5: Commit.** `perf: validate the state once per S1 query batch`

### Task 2: A checkpoint build hashes its state once

**Files:**
- Modify: `crates/mtgml-environment/src/checkpoint_v8.rs` (`build`, `validate_inner`)
- Test: `crates/mtgml-environment/tests/production_turn.rs`

**Interfaces:**
- Produces: private `fn validate_fields(&self, structurally_validated_by_rules: bool) -> Result<(), CheckpointV8Error>`. It holds every check of `validate_inner` except two: recomputing the state digest, and comparing the checkpoint digest.
  - `validate_inner` = `validate_fields` + both digest checks, so its behavior is unchanged.
  - `build` = its current structure check + one digest + checkpoint digest + `validate_fields`.

- [ ] **Step 1: Write the guard test** `checkpoints_stay_bound_to_their_state_digest`. Start from `Game::new(two_player_land_game(10, 3, 1))`. Call `game.respond(true)` until `game.at(UPKEEP, 3)`. After every response, assert for `checkpoint = game.controller.checkpoint().unwrap()`:
  - `checkpoint.state_digest == mtgml_state::calculate_full_state_digest_v7_structural_only(&checkpoint.state).unwrap()`;
  - `checkpoint.validate_for_basic_land_profile(&common::game_admission()).is_ok()`.
- [ ] **Step 2: Run it.** `cargo test -p mtgml-environment --locked --test production_turn checkpoints_stay`. Expected: PASS. This is a refactor guard: it pins the behavior Tasks 2 and 4 must keep. Ledger a ruling that it could not fail first.
- [ ] **Step 3: Implement the split** described in Interfaces.
- [ ] **Step 4: Run.** `cargo test -p mtgml-environment --locked`. Expected: all pass, including `v8_restore_rejects_state_digest_and_checkpoint_identity_tampering`.
- [ ] **Step 5: Commit.** `perf: hash a checkpoint's state once when building it`

### Task 3: The rules check a delta they just built without re-applying it

**Files:**
- Modify: `crates/mtgml-rules/src/events_v3.rs`
- Modify: `crates/mtgml-rules/src/turn_progression.rs` (`finish`)
- Modify: `crates/mtgml-rules/src/basic_land_v4.rs` (`execute_basic_land_response_v4`)
- Test: `crates/mtgml-rules/src/turn_progression.rs` (tests module)

**Interfaces:**
- Produces: `pub(crate) fn validate_events_for_built_delta_v3(before: &EngineStatePartsV3, after: &EngineStatePartsV3, events: &[AuthoritativeRuleEventV3], delta: &StateDeltaV3) -> Result<(), EventDeltaV3Error>`.
  - It is for a delta that `StateDeltaV3::between_structural_only(before, after, ..)` has just built.
  - Instead of applying the delta, which would recompute both digests, it requires:
    - `delta.before_revision == before.predecessor_v5.revision`;
    - `delta.after_revision == after.predecessor_v5.revision`;
    - `delta.replacement == *after`.
  - Then it runs every check that `validate_event_delta_state_v3_inner` runs after the apply.
- Implement it by giving `validate_event_delta_state_v3_inner` a `DeltaCheck { Apply, ApplyStructuralOnly, BuiltFrom }` parameter instead of the bool.
- `validate_event_delta_state_v3_structural_only` becomes `#[cfg(test)]`; only the test helper `apply` uses it after this task.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn built_delta_check_rejects_another_after_state() {
    let (admission, state) = game(3);
    let product = submit(&admission, &state, pass_answer(pending(&state))).unwrap();
    crate::events_v3::validate_events_for_built_delta_v3(
        &state, &product.next_state, &product.events, &product.delta).unwrap();
    let mut other = product.next_state.clone();
    other.predecessor_v5.core.turn_number += 1;
    assert!(crate::events_v3::validate_events_for_built_delta_v3(
        &state, &other, &product.events, &product.delta).is_err());
}
```

- [ ] **Step 2: Run it.** `cargo test -p mtgml-rules --lib --locked built_delta_check`. Expected: compile error, function not found.
- [ ] **Step 3: Implement.** Switch `finish` and `execute_basic_land_response_v4` to `validate_events_for_built_delta_v3`.
- [ ] **Step 4: Run.** `cargo test -p mtgml-rules --lib --locked`, `cargo test -p mtgml-environment --locked`, and `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`. Expected: all pass, no `dead_code`.
- [ ] **Step 5: Commit.** `perf: check a just-built delta without re-hashing both states`

### Task 4: The runtime keeps its current checkpoint

**Files:**
- Modify: `crates/mtgml-environment/src/basic_land_runtime_v8.rs`
- Test: `crates/mtgml-environment/tests/production_turn.rs`

**Interfaces:**
- Consumes: Task 2's `build` (one digest per new checkpoint).
- Produces: `BasicLandEnvironmentRuntimeV8` replaces its fields `state`, `status` and `limit_counters` with `current: EnvironmentCheckpointV8`. The checkpoint always comes from `new_for_basic_land_profile`, or is the validated input of `restore`.
  - `checkpoint()` returns `Ok(self.current.clone())`.
  - `visible_decision`, `information_state` and `submit` read `self.current`. They do not re-validate the pending request, because it was validated when `current` was built and nothing else mutates it.
  - In `submit`:
    - `before` is `&self.current`.
    - A rejected response reuses `self.current` as its checkpoint.
    - An accepted response builds the new checkpoint first. It then replaces the re-apply of the delta with these comparisons, failing with `ServiceUnavailable`:
      - `delta.before_revision == before.state.predecessor_v5.revision`;
      - `delta.before_digest == before.state_digest`;
      - `delta.after_revision == checkpoint.state.predecessor_v5.revision`;
      - `delta.after_digest == checkpoint.state_digest`;
      - `delta.replacement == checkpoint.state`.

- [ ] **Step 1: Write the guard test** `restored_runtime_continues_from_the_restored_checkpoint`.
  - Play one turn and take `saved = controller.checkpoint()`. Play one more turn, then `controller.restore(saved.clone())`.
  - Assert `controller.checkpoint() == saved`.
  - One more response must be accepted, and the new checkpoint's revision must be `saved` + 1.
  - Use `Game` and `controller.restore` as `ten_turns_of_land_drops_are_deterministic` does.
- [ ] **Step 2: Run it.** `cargo test -p mtgml-environment --locked --test production_turn restored_runtime`. Expected: PASS, as a guard; ledger the ruling.
- [ ] **Step 3: Implement** as in Interfaces.
- [ ] **Step 4: Run.**
  - `cargo test -p mtgml-environment --locked`: all pass, including `current_successor_api` rejections and the V7/V8 parity tests.
  - `cargo test --release -p mtgml-environment --test random_smoke --locked`: PASS.
- [ ] **Step 5: Commit.** `perf: keep the runtime's current checkpoint instead of rebuilding it`

### Task 5: Evidence

**Files:**
- Modify: this plan (status)

- [ ] **Step 1:** Commit, then run `.venv/Scripts/python.exe scripts/run_checks.py integration`. Expected: PASS.
- [ ] **Step 2:** Measure on the same machine as the baseline. Record the before/after numbers for the PR description:
  - `cargo test --release -p mtgml-environment --test random_smoke --locked -- --exact random_games_run_thirty_turns_deterministically_and_replay`: note wall time, run twice and keep the lower value.
  - A samply summary per `docs/agents/profiling.md`: the digest share and its nearest callers.
  - Expected: the digest share drops well below 72%, and the smoke is at least 2x faster. If it is not, report the numbers; do not tune thresholds.
- [ ] **Step 3:** Set this plan's status to implemented and commit: `docs: record per-step digest cost results`.

## Next (not in this plan)

- The digest function itself encodes the state to CBOR, decodes it again and validates it before hashing. That is about half the cost of each remaining digest. Making it cheaper must keep every digest byte-identical; it is a separate plan with its own profile.
- `validate_basic_land_pending_request_v4` re-derives candidates during projection. Re-profile after this plan before deciding.
