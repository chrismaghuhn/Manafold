# One Runtime Cleanup (Stage 1) Implementation Plan

**Status:** DRAFT for owner review, 2026-09-30. Execution method chosen by the owner: inline.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Exactly one execution path, the production V8 runtime. No historical runtime, kernel, feature switch, gate or pin remains, and engine output is byte-identical.

**Architecture:** Delete from the outside in, so every commit builds and passes:
1. the RNG fix (independent);
2. the pins and gates that require old code;
3. the crates and tools that consume the old runtime;
4. the old environment runtime and its module swap;
5. the old rules kernel and its features;
6. admission and registry;
7. check profiles and documents;
8. final verification.

The spec's order ("rules kernel, environment runtime, …") cannot keep each commit building, because the environment runtime and the conformance crate use the kernel. This plan reverses it; the result is the same.

**Tech Stack:** Rust workspace, Python client and scripts, GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-09-30-one-runtime-cleanup-design.md` (decisions D1–D10).

## Global Constraints

- **Output bytes stay identical.** Engine output bytes do not change: state, delta, event, decision, checkpoint, replay, observation, digests and RNG. These known-answer tests stay green without edits:
  - `persistence/golden/checkpoint-digest-v8-kat`
  - the V7 full-state digest KATs
  - `mtgml-random` KATs
  - the V8 replay checkpoint value in `crates/mtgml-replay/src/v8.rs`
- **Production code paths do not change**, except three places:
  - deleted dead branches;
  - D5 (admission accepts only the full game closure);
  - D10 (RNG internals).
- **Delete code; do not comment it out.** No new version identifiers. No milestone labels in new identifiers.
- **Stage 2 and 3 code stays:**
  - `crates/mtgml-environment/src/checkpoint.rs`, `checkpoint_v7.rs`
  - `mtgml-replay` V1–V7
  - decision and observation V1–V3
  - digest V3–V6 with their tests
  - schemas and fixtures
  - `tests/checkpoint_v5_red.rs`, `tests/checkpoint_v6_red.rs`
- **Every commit passes before the next task starts:**
  - `cargo build --workspace --all-targets --locked`
  - `cargo test --workspace --locked`
  - `.venv/Scripts/python.exe scripts/run_checks.py fast`
- **Commit before running gate scripts** (`AGENTS.md` §5).
- **Test rule (D3):**
  - A test that exercises only deleted code is deleted.
  - A test that exercises remaining code is kept. If it depends on a deleted runtime, it is ported to the V8 path.
  - Tasks name the kept tests.

## Review Focus

1. **A remaining module could lose its only test.** The coverage table in Task 8 lists the tests of every remaining module. It is checked against `cargo test` output, not written from memory.
2. **The public root names of `mtgml-environment` must still resolve to the V8/V4 types** after the swap is gone: `TrustedEnvironmentController`, `PlayerEndpoint*`, `CurrentPlayerStep`, `submit_response_bytes`. `tests/current_successor_api.rs` compiles unchanged (Task 4).
3. **RNG reader at block and exhaustion boundaries.** At lane 3 → next block and at cursor `u64::MAX - 1` → `u64::MAX` → exhausted, the reader must match single draws exactly (Task 1 test).
4. **Admission after D5.** A content-only closure is rejected, and the full game closure still admits with the downgraded lifecycles (Task 6 tests).
5. **The capability registry still validates.** Paths exist, `covered` entries have conformance cases, and every conformance case names an existing test. `validate_maintainer_artifacts.py` passes (Task 6).

---

### Task 1: RNG stream reader (D10)

**Files:**
- Modify: `crates/mtgml-random/src/hmac_counter.rs`, `crates/mtgml-random/src/sampling.rs`

**Interfaces:**
- **Produces:** `pub struct RawStreamReader` in `hmac_counter.rs`, with:
  - `pub fn new(root: &RootSeed256, key: &RandomStreamKeyV1, cursor: RandomStreamCursorV1) -> Self`
  - `pub fn next_raw_u64(&mut self) -> Result<u64, RandomValidationError>`
  - `pub fn cursor(&self) -> RandomStreamCursorV1`
- **Behaviour:**
  - The stream key is derived once in `new`. A 32-byte block is computed only when the next word lies in a block not yet cached.
  - At cursor `u64::MAX` it returns `StreamExhausted` and does not advance.
  - `uniform_below_u64` and `shuffle` keep their public signatures and use one reader per call.
  - The free function `next_raw_u64` stays.

- [ ] **Step 1: Write the failing test** in the `hmac_counter.rs` test module.

```rust
#[test]
fn reader_matches_single_draws_across_block_boundaries() {
    let seed = RootSeed256::from_lower_hex(ALL_ZERO_SEED).unwrap();
    let key = global_key();
    for start in [0u64, 1, 3, 4, 7, u64::MAX - 6] {
        let mut single = RandomStreamCursorV1 { next_raw_u64: start };
        let mut reader = RawStreamReader::new(&seed, &key, single);
        for _ in 0..8 {
            let expected = next_raw_u64(&seed, &key, &single);
            let actual = reader.next_raw_u64();
            assert_eq!(actual, expected.as_ref().map(|(value, _)| *value).map_err(|e| *e));
            if let Ok((_, next)) = expected { single = next; }
            assert_eq!(reader.cursor(), single);
        }
    }
}
```

- [ ] **Step 2: Run it.** `cargo test -p mtgml-random --locked reader_matches`. Expected: compile error, `RawStreamReader` not found.
- [ ] **Step 3: Implement** `RawStreamReader`. Switch `uniform_below_u64` and `shuffle` to it.
- [ ] **Step 4: Run** `cargo test -p mtgml-random --locked`. Expected: all pass, including the unchanged KATs `shuffle_normative_kat`, `bound_ten_normative_kat`, `production_sampler_consumes_rejected_words_and_advances_the_cursor` and `shuffle_atomicity_on_stream_exhaustion`.
- [ ] **Step 5: Commit** `perf: derive each RNG stream key and block once per operation`.

### Task 2: Remove pins and M2 gates (D9, first part)

This task deletes checks only; no engine code changes. After it, no gate requires an old file, so later tasks can delete freely.

**Files:**
- **Delete (scripts):**
  - `scripts/run_v5_execution_identity_gate.py`, `scripts/run_v6_state_identity_gate.py`, `scripts/run_v8_state_identity_gate.py`
  - `scripts/run_m1_closure.py`, `scripts/run_m2_b_contract_cut.py`, `scripts/run_m2_c_gates.py` … `scripts/run_m2_h_gates.py`, `scripts/run_m2_final_closure.py`
- **Delete (Python tests):**
  - `python/tests/test_current_status.py`
  - the Python tests of these runners (`test_m1_closure_reporter.py`, `test_m2_d_gate_runner.py`, `test_m2_e_gate_runner.py`, `test_m2_final_gate_runner.py`, `test_m2_h_gate_runner.py`)
  - any other `python/tests/*` whose only subject is a deleted script (find them with `grep -l` on the script names)
- **Modify:**
  - `scripts/run_checks.py`: FAST and INTEGRATION lists
  - `justfile`: the `contracts` recipe
  - `python/tests/test_python_test_profiles.py`: drop the expectations about the deleted gates
  - `scripts/verify_repository.py`: delete only the checks that require historical tokens or test names:
    - the V3/V4 digest tokens;
    - `EnvironmentCheckpointV6` and its constants;
    - `CheckpointDigestV6`;
    - replay `v6.rs` tokens;
    - `calculate_checkpoint_digest_v3`;
    - the `mtgml-conformance` `lib.rs` tokens;
    - the named historical tests.
  - `.github/workflows/pr-fast.yml`: delete the M2.B–M2.H steps and their "Assert clean source" steps. Keep bootstrap, `run_checks.py fast`, and the toolchain + `cargo fmt`/`cargo check` step.
  - Other workflows (`nightly.yml`, `integration.yml`, `pr-integration.yml`): remove any step that calls a deleted script.

- [ ] **Step 1:** Delete and modify as listed. Then run `grep -rn "run_v5_execution\|run_v6_state\|run_v8_state\|run_m1_closure\|run_m2_" scripts python .github justfile`. Expected: no match outside deleted files.
- [ ] **Step 2:** Commit, then run `.venv/Scripts/python.exe scripts/run_checks.py fast` and `.venv/Scripts/python.exe -m pytest -q -p no:cacheprovider python/tests`. Expected: PASS.
- [ ] **Step 3: Commit** `chore: remove gates and pins that require the historical runtime`.

### Task 3: Remove the conformance crate and the M2 adapter (D4, first part)

**Files:**
- **Delete:**
  - `crates/mtgml-conformance/`
  - `tools/m2-semantic-adapter/`
  - `python/src/mtgml/_m2_adapter/`
  - `python/tests/m2_h/`
  - `python/tests/test_m2_adapter_unit.py`, `python/tests/test_m2_h_rules_free_guards.py`
- **Modify:**
  - root `Cargo.toml` members and `Cargo.lock`, via `cargo metadata --locked` after `cargo update -w --offline` if needed; the lock may only lose entries
  - `python/src/mtgml/player_client.py` and `python/src/mtgml/__init__.py`: delete `HistoricalPlayerClientV2`/`V3` and their exports
  - `python/tests/test_player_api.py`: delete the historical-client parts
  - `scripts/run_python_tests.py`: profiles that list deleted files, if any

- [ ] **Step 1:** Delete and modify as listed. Run:
  - `cargo build --workspace --all-targets --locked`
  - `cargo test --workspace --locked`
  - `.venv/Scripts/python.exe -m pytest -q -p no:cacheprovider python/tests`
  - `.venv/Scripts/python.exe -m mypy --config-file python/pyproject.toml`

  Expected: all pass. `Cargo.lock` loses only the two members and dependencies used by nobody else.
- [ ] **Step 2: Commit** `chore: remove the conformance crate and the M2 semantic adapter`.

### Task 4: One environment runtime (D1, D3, D4 environment part)

**Files:**
- **Delete** (`crates/mtgml-environment/src/`):
  - `controller_predecessor.rs`, `endpoint_predecessor.rs`
  - `controller_successor_v7.rs`, `endpoint_successor_v7.rs`
  - `reference.rs`
  - `replay.rs`, `replay_v7_execution.rs`
  - `response_transaction.rs`
  - `semantic_catalog.rs`
  - `successor_runtime.rs`, `successor_transaction.rs`
  - `synthetic.rs` and `synthetic/`, including `replay_parity_tests.rs`
  - `lifecycle_projection.rs`
  - `tests/*.rs`, except `successor_turn_projection.rs` and `magic_basic_land_observation.rs`
- **Modify:**
  - `lib.rs`:
    - no `#[path]`;
    - no `cfg(test)` or feature module selection;
    - `controller`, `endpoint`, `controller_successor` and `endpoint_successor` always V8/V4;
    - `CurrentPlayerStep = PlayerStepV4`;
    - `SuccessorEnvironmentRuntime` stays as the alias of `BasicLandEnvironmentRuntimeV8`;
    - the V8 types are exported unconditionally.
  - `boundary.rs`: decode V3 only.
  - `player_projection.rs`: delete the `cfg(any(test, feature))` sections and the V2 `project_successor_information_state`.
  - `successor_projection.rs`: delete `project_successor_events_v3` and `project_successor_player_steps` (the V3 functions).
  - `errors.rs`: delete `ReplayExecutionError::ReplayV7*` and the `ControllerError` variants that only deleted code constructs. Keep the checkpoint-format variants until stage 2.
  - `tests.rs`: keep only `mod magic_basic_land_observation; mod successor_turn_projection;` and the helpers they use.
  - `tests/magic_basic_land_observation.rs`: delete its `EnvironmentCheckpointV7` / `PlayerStepV3` parts and keep the V8 projection tests.
  - `basic_land_runtime_v8.rs`: delete the four `v7_v8_*` tests and `v7_manifest`.
  - `tests/p0_red.rs` (integration): keep the V8 branch only.
  - `Cargo.toml`: delete the feature `historical-conformance-runtime`.
  - `cards/capabilities/registry.json`: delete `implementation_paths` and `conformance_cases` entries that point at deleted environment files. Task 6 sets the final values; here the registry only has to validate.

- [ ] **Step 1:** Delete and modify as listed.
- [ ] **Step 2: Verify.** Run:
  - `grep -n "#\[path\|cfg(test)\|historical" crates/mtgml-environment/src/lib.rs`. Expected: only `#[cfg(test)] mod tests;` and `#[cfg(test)] mod semantic_catalog_kat;` (the latter goes in Task 5).
  - `cargo test --workspace --locked`. Expected: PASS, including `production_turn`, `current_successor_api` (unchanged file) and `successor_turn_projection`.
  - `cargo test --release -p mtgml-environment --test random_smoke --locked`. Expected: 2 passed.
- [ ] **Step 3:** Commit, then run `.venv/Scripts/python.exe scripts/run_checks.py fast` and `.venv/Scripts/python.exe scripts/validate_maintainer_artifacts.py`. Expected: PASS.
- [ ] **Step 4: Commit** `refactor: one environment runtime`.

### Task 5: Remove the old rules kernel (D2, D3, D4 rules part)

**Files:**
- **Delete** (`crates/mtgml-rules/src/`):
  - `magic.rs`, `program_kernel.rs`
  - `basic_priority.rs`, `combat_damage.rs`, `state_based_actions.rs`
  - `contract.rs`, `semantic_cursor.rs`
  - `product.rs`, `decision_stage.rs`
  - `synthetic.rs` and `synthetic/`
  - `successor_contract.rs`, `transition.rs`
  - `fixture_support.rs`, `semantic_execution_generated.rs`
  - `tests/*.rs`, except `tests/zone_incarnation.rs` (reduced below) and a new `tests/semantic_delta.rs`
- **Delete** `crates/mtgml-rules/tests/p0_red.rs` and `tests/program_kernel_red.rs`.
- **Delete** the generator and the catalog it reads:
  - `scripts/generate_semantic_contract_catalog.py`
  - `python/tests/test_semantic_contract_catalog_generator.py`
  - `contracts/catalog/semantic-contracts.v1.json` (its only readers are the generator and the deleted modules)
  - `crates/mtgml-environment/src/semantic_catalog_generated.rs` and `semantic_catalog_kat.rs`, and the `lib.rs` re-exports
- **Modify** these mixed modules; remove only the old-only items:
  - `turn_structure.rs`: `TurnStructureSupportProfile`, `validate_turn_structure_support`, `unsupported_rules_boundary`, `CleanupBoundaryViolation`, `validate_quiescent_cleanup_boundary`, `derive_cleanup_damage_reset_objects`, and their tests. Keep `temporal_successor`, `derive_ordinary_untap_affected_objects`, `ORDINARY_MAXIMUM_HAND_SIZE`, the error types and `temporal_successor_table_is_exact`.
  - `zone_incarnation.rs`: `execute_selected_zone_transition`, `apply_selected_zone_transition_in_sba_batch_workspace`, the `BattlefieldToOwnerGraveyard` kind, `ConformanceZoneTransitionKind` and `execute_selected_zone_transition_for_conformance`.
  - `events.rs`: `validate_occurrence_pairing` and `OccurrencePairingError`.
  - `basic_land.rs`: the V7 wrappers `execute_basic_land_response`, `BasicLandTransitionProductV1`, `install_basic_land_request`, `validate_basic_land_pending_request`, `selected_successor_decision`, `selected_magic_action_request`; the second-pass path of `execute_basic_land_decision_draft` with `priority_window_after_second_pass`; and the ten V7 tests named in the inventory.
  - `turn_progression.rs`: delete the `oracle` test module.
  - `lib.rs`: modules and exports.
  - `Cargo.toml`: delete the features `historical-runtime-testkit`, `magic-conformance-testkit` and `synthetic-conformance-fixtures`. Remove them from `mtgml-environment`'s `[dev-dependencies]` too.
  - `tests.rs`: keep only the kept fragments.
- **Keep tests:**
  - the four `semantic_delta()` mapping tests from `src/tests/turn_structure.rs` (currently at lines 47, 56, 68, 78). Move them to `src/tests/semantic_delta.rs` unchanged.
  - the production-primitive test in `src/tests/zone_incarnation.rs` (currently at line 250). Delete the other three tests there.

- [ ] **Step 1:** Delete and modify as listed. `cargo build -p mtgml-rules --locked` must report no dead-code warnings for the removed items.
- [ ] **Step 2: Verify.** Run:
  - `cargo test --workspace --locked` and `cargo test --workspace --all-features --locked`. Expected: both pass. With no features left, they build the same code.
  - `grep -rn "historical-\|testkit\|conformance-fixtures" --include=Cargo.toml .`. Expected: no match.
- [ ] **Step 3:** Commit, then run `.venv/Scripts/python.exe scripts/run_checks.py fast`. Expected: PASS.
- [ ] **Step 4: Commit** `refactor: remove the old rules kernel`.

### Task 6: One admission scope and an honest registry (D5, D6)

**Files:**
- **Modify:**
  - `crates/mtgml-card-ir/src/preflight.rs`: `admit_executable_profile_v1` accepts only a rules closure equal to content ∪ `MAGIC_GAME_RULE_ROOTS`; anything else fails with the existing closure error.
  - `crates/mtgml-card-ir/tests/profiled_content_admission.rs`.
  - Rules and environment test fixtures that build the content-only admission (`basic_land::content_only_admission_fixture`, the environment `basic_land_runtime_v8` test `admission()`). They switch to the game closure, or are deleted with the tests that only exist for the content-only scope.
  - `cards/capabilities/registry.json` and its projection, regenerated with `scripts/generate_card_ir_capability_projection.py`.

- [ ] **Step 1: Write the failing test** in `profiled_content_admission.rs`:

```rust
#[test]
fn only_the_full_game_closure_is_admitted() {
    let bytes = manifest_bytes();
    let id = calculate_content_contract_id_v1(&bytes).unwrap();
    let provenance = provenance_bytes(&id);
    let (rules, semantic, execution) = identities(id.clone(), content_only_closure());
    assert!(admit_executable_profile_v1(&bytes, &id, &provenance, &rules, &semantic, &execution).is_err());
    let (rules, semantic, execution) = identities(id.clone(), complete_closure());
    assert!(admit_executable_profile_v1(&bytes, &id, &provenance, &rules, &semantic, &execution).is_ok());
}
```

- [ ] **Step 2: Run it.** `cargo test -p mtgml-card-ir --locked only_the_full_game_closure`. Expected: FAIL at the first `is_err` (the content-only closure is admitted today).
- [ ] **Step 3: Implement** the single scope. Delete `content_only_closure_still_admits_without_game_rules`. Delete these tests:
  - `turn_progression::tests::content_only_admission_fails_closed_beyond_its_scope`
  - the content-only half of `turn_requests_are_valid_only_under_the_rules_that_create_them`

  Keep the `admits(..)` checks in `turn_progression.rs` as defence and ledger that they are now untestable. Switch the remaining content-only fixtures to the game closure.
- [ ] **Step 4: Set the registry.**
  - **Point to the native code** (lifecycle `covered`). `implementation_paths` includes `crates/mtgml-rules/src/turn_progression.rs`, plus `zone_incarnation.rs` for `rules/zone-incarnation` and `basic_land_v4.rs` for `rules/basic-priority`. The `notes` state the native scope. `conformance_cases` are existing test names:
    - `rules/turn-structure`: `end_step_passes_run_cleanup_and_open_next_upkeep`, `two_players_complete_a_turn_and_reach_the_next_upkeep`
    - `rules/basic-priority`: `first_pass_only_transfers_priority`, `acting_after_a_pass_restarts_the_pass_succession`
    - `rules/draw-card`: `upkeep_passes_draw_one_card_for_the_active_player`, `opponent_never_learns_drawn_cards_end_to_end`
    - `rules/combat-phase`: `beginning_of_combat_passes_reach_empty_attacker_declaration`, `empty_attack_declaration_ends_combat_without_damage`
    - `rules/declare-attackers`: `empty_attack_declaration_ends_combat_without_damage`; notes say "empty declaration only"
    - `rules/cleanup-reset`: `cleanup_with_eight_cards_asks_active_player_to_discard_one`, `discard_moves_chosen_card_then_next_turn_starts`; notes say "discard of one card; no damage to reset in this slice"
    - `rules/zone-incarnation`: `second_discard_shifts_the_known_graveyard_card`, `upkeep_passes_draw_one_card_for_the_active_player`
  - **Back to `specified`:** `rules/combat-damage`, `rules/declare-blockers`, `rules/damage-and-life`, `rules/state-based-actions-combat`. `implementation_paths` and `conformance_cases` are emptied, and `notes` say "implementation removed with the old kernel (2026-09-30); see git history".
- [ ] **Step 5: Run** `cargo test --workspace --locked`, `.venv/Scripts/python.exe scripts/generate_card_ir_capability_projection.py --check` and `.venv/Scripts/python.exe scripts/validate_maintainer_artifacts.py`. Expected: PASS.
- [ ] **Step 6: Commit** `refactor: admit only the full game closure and describe capabilities by what remains`.

### Task 7: Check profiles and documents (D7, D8, D9 rest)

**Files:**
- `scripts/run_checks.py`, `justfile`: with the features gone, delete these steps:
  - `cargo check -p mtgml-environment --locked`
  - `cargo test -p mtgml-environment --locked`
  - `cargo test -p mtgml-environment --test current_successor_api --locked`
  - the default-features `cargo clippy -p mtgml-environment`

  `cargo test --workspace` now runs the same tests. Keep the release `random_smoke` step. Update the comments that explain the swap.
- `AGENTS.md`:
  - §4: delete "Currently violated in `mtgml-environment`; being removed."
  - §5: delete the feature-unification paragraph. The production proof is `cargo test --workspace --locked`.
  - §7: commands.
- **Mark as superseded (D7).** Set `**Status:** SUPERSEDED by AGENTS.md §4 (one current format, changed in place)` in `docs/NORMATIVE_HIERARCHY.md`, `docs/maintenance/API_LIFECYCLE.md`, `docs/maintenance/SCHEMA_EVOLUTION.md` and `contracts/COMPATIBILITY_POLICY.md`, and add a two-sentence note at the top of each. In `docs/REPLAY_AND_DETERMINISM.md` and `docs/STATE_HASHING.md`, put the same note only above the sections that require a new version per change. Update the register `stability` of the four superseded documents to `deprecated`.
- **Correct the S1 status (D8).** In `docs/superpowers/specs/2026-09-28-m4-shared-s1-characteristic-object-query-authority.md` and the matching plan, the Status reads: "S1-A and S1-B implemented and used in production (`crates/mtgml-rules/src/characteristic_query.rs`); S1-C and S1-D open."
- `README.md`: the lines that describe V6/V7 as "historical read/verification only" and the historical runtimes.
- `docs/superpowers/plans/2026-09-30-production-full-turn.md`: Plan B's runtime part is done by this plan.
- This plan and the spec: set Status to implemented.

- [ ] **Step 1:** Edit as listed.
- [ ] **Step 2:** Commit, then run `.venv/Scripts/python.exe scripts/check_documentation.py` and `.venv/Scripts/python.exe scripts/run_checks.py fast`. Expected: PASS.
- [ ] **Step 3: Commit** `docs: one current format and one runtime`.

### Task 8: Final verification

- [ ] **Step 1: Deleted names.** Run `grep -rn` over `scripts python .github justfile Cargo.toml crates` for these names:
  - `controller_predecessor`, `successor_runtime`, `reference.rs`, `magic.rs`, `program_kernel`, `mtgml-conformance`, `m2-semantic-adapter`
  - `historical-conformance-runtime`, `historical-runtime-testkit`
  - `run_v5_execution`, `run_v8_state`, `run_m2_`

  Expected: no match, except historical plan/spec documents under `docs/`.
- [ ] **Step 2: Coverage table.** Check each row against `cargo test --workspace --locked -- --list` and fix any row that differs.

| Module | Tests that still exercise it |
|---|---|
| `mtgml-rules::turn_progression` | the native tests in its test module (19 after Task 6) |
| `mtgml-rules::basic_land_v4` | `turn_progression` tests (land play, mana ability), `basic_land` `v4_*` tests, environment `production_turn` |
| `mtgml-rules::basic_land` (candidates, draft) | `basic_land` candidate and S1 tests, `turn_progression` tests |
| `mtgml-rules::characteristic_query` | its 2 tests, 7 S1 tests in `basic_land` |
| `mtgml-rules::events_v3` | its 10 tests, `built_delta_check_rejects_another_after_state` |
| `mtgml-rules::events` | `tests/semantic_delta.rs` (4) |
| `mtgml-rules::zone_incarnation` | `tests/zone_incarnation.rs` (1), draw and discard tests in `turn_progression` |
| `mtgml-rules::turn_structure` | `temporal_successor_table_is_exact`, untap through `end_step_passes_run_cleanup_and_open_next_upkeep` |
| `mtgml-rules::snapshots`, `errors`, `validation` | indirectly, through `turn_progression` |
| `mtgml-environment::basic_land_runtime_v8` | its module tests, `production_turn` (8), `current_successor_api`, `random_smoke` |
| `mtgml-environment::checkpoint_v8` | its module tests, `checkpoints_stay_bound_to_their_state_digest` |
| `mtgml-environment::successor_projection` (V4) | `successor_turn_projection` (6), `magic_basic_land_observation` |
| `mtgml-environment::player_projection` (V3) | `magic_basic_land_observation`, `opponent_never_learns_drawn_cards_end_to_end` |
| `mtgml-environment::boundary` | `current_successor_api` |
| `mtgml-random` | its unit tests, including `reader_matches_single_draws_across_block_boundaries` |

- [ ] **Step 3: Full gate.** Commit, then run `.venv/Scripts/python.exe scripts/run_checks.py integration`. Expected: all PASS, and the known-answer tests from Global Constraints pass unedited: `git diff master -- persistence/golden crates/mtgml-random/src crates/mtgml-replay/src/v8.rs` shows only the Task 1 RNG code.
