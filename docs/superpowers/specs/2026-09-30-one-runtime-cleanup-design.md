# One Runtime Cleanup (Stage 1) — Design

**Status:** IMPLEMENTED on branch `chore/one-runtime`, 2026-09-30 (approved by the owner; executed inline).

**Goal:** leave exactly one execution path, the production V8 runtime, so that R1/W1 card work starts on a base where every test exercises production code and no document or gate asks for the old paths.

**Behavior:** unchanged. This stage only deletes code and fixes documents. No state, event, decision, checkpoint, replay or observation byte changes.

## 1. Why

- **Two worlds in one crate.** `mtgml-environment` compiles different modules under `cfg(test)` and under the `historical-conformance-runtime` feature. Two workspace members turn that feature on (`mtgml-conformance`, `tools/m2-semantic-adapter`), and Cargo unifies features. So `cargo test --workspace` builds the V6/V7 runtimes and never the production V8 controller. Production is only proven by `cargo test -p mtgml-environment`. `AGENTS.md` §4–5 already names this as a violation.
- **Dead weight.** The old kernel (`MagicRulesKernel`, `ProgramKernelV1`), the V6/V7 runtimes, the conformance crate and the M2 adapter add up to roughly 60,000 lines of code and tests. None of it runs in production.
- **Contradicting documents and gates.**
  - Gates pin the old paths. `run_v8_state_identity_gate.py` even requires the test/production module swap to exist.
  - Six policy documents require a new version for every change, which `AGENTS.md` §4 overrides.
  - The S1 spec and plan say "not authorized" while their code runs in production.

## 2. Stages

The cleanup is split into three stages. Each gets its own spec, plan and PR.

| Stage | Scope |
|---|---|
| **1 (this spec)** | One runtime. Delete the historical runtimes, the old kernel, the conformance crate, the M2 adapter, their features, gates, CI steps and pins. Fix the contradicting documents. Plus the RNG recomputation fix. |
| 2 | Historical formats: replay V1–V7, checkpoint V4–V7, decision V1–V3, observation/step V1–V3, and their schemas, wire and persistence fixtures, and Python modules. |
| 3 | Disentangle the current format: state V3 built on the V1 parts, digest V7 built through V6/V5, basic-land logic on V2. Then a single-pass digest and dropping the version suffixes from type names. |

## 3. Decisions

- **D1 — One runtime.** The production V8 runtime (`BasicLandEnvironmentRuntimeV8` behind `TrustedEnvironmentController` and the V4 player endpoints) is the only runtime.
  - `lib.rs` loses every `cfg(test)`/feature module swap. `controller`, `endpoint`, `controller_successor` and `endpoint_successor` always mean the V8/V4 versions.
  - `CurrentPlayerStep` is `PlayerStepV4` everywhere.
- **D2 — The old kernel is deleted entirely.** This includes its combat damage, blocker and state-based-action logic.
  - That logic models synthetic test creatures (`foundation_sources`), allows at most one blocker, and runs on the old state type. R1/W1 combat will be built on card characteristics (Card IR, S1 queries).
  - Git history keeps the old code as a reference.
- **D3 — Tests.**
  - A test that exercises only deleted code is deleted.
  - A test that exercises code that stays is kept. If it runs through a deleted runtime, it is ported to the V8 path.
  - For every remaining module, the plan names the tests that still cover it.
- **D4 — Features are removed:** `historical-conformance-runtime` (environment), and `historical-runtime-testkit`, `magic-conformance-testkit`, `synthetic-conformance-fixtures` (rules). After D1–D4, `cargo test --workspace` and `cargo test --workspace --all-features` build the same production code.
- **D5 — One admission scope.** Executable admission accepts only the full game closure. The content-only scope existed for V7 parity and goes with V7.
- **D6 — The capability registry is made honest.**
  - Capabilities the native turn progression implements point to its code and tests: `basic-priority`, `cleanup-reset`, `combat-phase`, `declare-attackers`, `draw-card`, `turn-structure`, `zone-incarnation`. Their notes state the native scope. For example, `declare-attackers` covers the empty declaration only, and `cleanup-reset` covers a single discard.
  - Capabilities that lose their only implementation go back to `specified`: `combat-damage`, `declare-blockers`, `damage-and-life`, `state-based-actions-combat`.
  - Lifecycles are set by the evidence that remains, not by what existed before.
- **D7 — Policy documents that require a new version for every change are marked superseded.** Their Status line reads "SUPERSEDED by AGENTS.md §4 (one current format, changed in place)", with a one-paragraph note at the top; their text stays as history. The documents are:
  - `docs/NORMATIVE_HIERARCHY.md` (the in-place rule)
  - `docs/maintenance/API_LIFECYCLE.md`
  - `docs/maintenance/SCHEMA_EVOLUTION.md`
  - `contracts/COMPATIBILITY_POLICY.md`
  - the version-preservation sections of `docs/REPLAY_AND_DETERMINISM.md` and `docs/STATE_HASHING.md`
- **D8 — The S1 spec and plan get their real status:** S1-A (object query authority) and S1-B (base characteristics) are implemented and used in production; S1-C (counters) and S1-D (attachments) are open.
- **D9 — Gates and CI.**
  - **Deleted scripts:**
    - `run_m1_closure.py`
    - `run_m2_b_contract_cut.py`, `run_m2_c_gates.py` … `run_m2_h_gates.py`
    - `run_m2_final_closure.py`
    - `run_v5_execution_identity_gate.py`, `run_v6_state_identity_gate.py`, `run_v8_state_identity_gate.py`
    - `generate_semantic_contract_catalog.py`, with the semantic contract catalog and both generated catalog files
    - `python/tests/test_current_status.py`
    - the Python tests of these runners
  - **`verify_repository.py`** loses only its historical token checks.
  - **The PR Fast workflow** keeps `run_checks.py fast` and the format/compile step; its M2.B–M2.H steps go. PR Integration is unchanged.
  - **`run_checks.py` and `justfile`** drop the deleted scripts and the default-features lint that only existed because of the features.
- **D10 — The RNG no longer recomputes per draw.**
  - Today `next_raw_u64` derives the stream key (an HMAC) and computes the 32-byte block (another HMAC) for every single u64, although a block serves four draws.
  - A stream reader caches the stream key and the current block for the length of one operation (a sampling call or one shuffle). The cursor stays the only persisted RNG state.
  - Output is byte-identical; the existing known-answer tests of `mtgml-random` prove it.

## 4. What is deleted (inventory)

> The semantic catalog was deferred to stage 2 and deleted there (`docs/superpowers/specs/2026-09-30-old-formats-cleanup-design.md`).

Sizes from the 2026-09-30 inventory at `9cfd35b`.

- **`crates/mtgml-environment`**
  - **Runtime modules:**
    - `controller_predecessor.rs`, `endpoint_predecessor.rs`
    - `controller_successor_v7.rs`, `endpoint_successor_v7.rs`
    - `reference.rs`
    - `replay.rs`, `replay_v7_execution.rs`
    - `response_transaction.rs`
    - `semantic_catalog.rs`
    - `successor_runtime.rs`, `successor_transaction.rs`
    - `synthetic.rs` with `synthetic/` and `replay_parity_tests.rs`
    - `lifecycle_projection.rs`
    - `semantic_catalog_generated.rs`, `semantic_catalog_kat.rs`
  - **The V2/V3 parts of** `player_projection.rs`, `successor_projection.rs` and `boundary.rs`.
  - **Historical unit tests:** `tests.rs` and `tests/*.rs` (about 17,000 lines). Kept: `successor_turn_projection.rs`, and `magic_basic_land_observation.rs` without its V7 parts.
  - **The four V7↔V8 parity tests** in `basic_land_runtime_v8.rs`.
  - **The historical branch** of `tests/p0_red.rs`.
  - **Error variants only historical code constructs.** Checkpoint formats remain until stage 2, so their error variants stay.
- **`crates/mtgml-rules`**
  - **Modules:**
    - `magic.rs`, `program_kernel.rs`
    - `basic_priority.rs`, `combat_damage.rs`, `state_based_actions.rs`
    - `contract.rs`, `semantic_cursor.rs`
    - `product.rs`, `decision_stage.rs`
    - `synthetic.rs` with `synthetic/`
    - `successor_contract.rs`, `transition.rs`
    - `fixture_support.rs`, `semantic_execution_generated.rs`
  - **Old-only functions in mixed modules:**
    - `turn_structure.rs`: support profile, boundary classification, cleanup damage reset
    - `zone_incarnation.rs`: `execute_selected_zone_transition`, the SBA-batch move, the conformance kinds
    - `events.rs`: `validate_occurrence_pairing`
    - `basic_land.rs`: the V7 wrappers and the unreachable second-pass path
  - **Tests:** the tests of these modules (about 180), the `oracle` test in `turn_progression.rs`, and `tests/p0_red.rs` and `tests/program_kernel_red.rs`.
- **Whole workspace members:** `crates/mtgml-conformance` (about 22,000 lines) and `tools/m2-semantic-adapter` (about 2,700 lines).
- **Python**
  - `mtgml/_m2_adapter/`
  - `HistoricalPlayerClientV2` / `V3` and their exports
  - `python/tests/m2_h/`
  - `test_m2_adapter_unit.py`, `test_m2_h_gate_runner.py`, `test_m2_h_rules_free_guards.py`, `test_m2_e_gate_runner.py`
  - the runner self-tests of deleted runners
  - the historical parts of `test_player_api.py` and `test_python_test_profiles.py`
- **Scripts and CI:** see D9.
- **Documents**
  - `AGENTS.md` §4–5 lose the swap/feature-unification notes: the violation is fixed.
  - The plan `2026-09-30-production-full-turn.md` records that Plan B's runtime part is done here.
  - README lines that describe the historical runtimes are updated.

## 5. What stays untouched in stage 1

- **Historical format code**, for stage 2:
  - `checkpoint.rs` (V4–V6), `checkpoint_v7.rs`
  - `mtgml-replay` V1–V7
  - decision/observation V1–V3
  - digests V3–V6 and their KATs, schemas and fixtures
  - the `checkpoint_v5_red`/`checkpoint_v6_red` tests
- **The current format's internal layering**, for stage 3.
- **Every production code path.** Only dead branches are removed from files that production uses.

## 6. Verification

- **Byte identity.** Output bytes do not change. These known-answer tests and pinned values stay green unchanged:
  - V8 checkpoint KAT (`persistence/golden/checkpoint-digest-v8-kat`, `mtgml-replay` v8 checkpoint value)
  - V7 full-state digest KATs
  - RNG KATs
- **Behavior.** The random smoke (same seed, same trajectory; replay reproduces), `production_turn` and `current_successor_api` pass unchanged.
- **One path.**
  - No `historical`, `testkit` or `conformance` feature remains in any `Cargo.toml`.
  - `lib.rs` of `mtgml-environment` has no `#[path]` or `cfg(test)` module selection.
  - `cargo test --workspace --locked` and `cargo test --workspace --all-features --locked` both pass.
- **Gates.** `scripts/run_checks.py integration` passes. No remaining script names a deleted file.
- **Coverage.** The plan lists, for every remaining module of `mtgml-rules` and `mtgml-environment`, which tests still exercise it. A module left without tests gets a test or is named as a gap.

## 7. Risks

- **Deleting a test that is the only coverage of shared code.** Mitigated by D3 and the per-module coverage list.
- **CI still names a deleted script.** Found by running both workflows' commands locally and by a repository-wide search for deleted file names.
- **Python public API change.** The `Historical*` clients go. There are no external users (`AGENTS.md` §4).
- **Size.** The change deletes tens of thousands of lines. The plan orders the work so every commit builds and passes the suite:
  1. RNG
  2. rules kernel
  3. environment runtime
  4. features and workspace members
  5. gates and CI
  6. documents
