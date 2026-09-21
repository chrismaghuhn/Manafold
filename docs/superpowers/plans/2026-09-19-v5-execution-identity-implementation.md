# V5 Execution Identity Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Status:** review candidate — NOT authorized for execution

**Goal:** Implement the reviewed V5 Execution Identity cut (ADR 0055 + reviewed spec) in dependency-safe order: exact semantic vocabulary, canonical contract digest machinery, generated single-source catalog, program-owned kernel boundary, Checkpoint V5, runtime admission, Replay V5, wire/Python parity, current-producer migration, and gate/docs closure — preserving all synthetic legacy semantics byte-for-byte except intentionally changed V5 persistence identities, and starting no S1 work.

**Architecture:** One normative identity model per the reviewed spec (`ExecutionIdentityV1 { program_kind, semantic_contract_id }`; content-derived recursive contract IDs under `mtgml.digest-envelope.v1` / `mtgml.canonical-cbor.v1`; checked-in generated catalog constants; `ProgramKernelV1` dual-entry-point kernel boundary; `EnvironmentCheckpointV5` with codec `in-memory-reference` / `"5"`; Replay V5 with three-way identity binding; V4 retained as historical-only). No EngineState or FullStateDigest changes. No Format/Content manifests. No production Magic semantic contract.

**Tech Stack:** Rust 1.85.1 with committed lockfile (crates `mtgml-model`, `mtgml-persistence`, `mtgml-rules`, `mtgml-environment`, `mtgml-replay`, `mtgml-wire`, `mtgml-conformance`), Python 3.13 from `.venv` (`python/src/mtgml/`), canonical CBOR/JSON schemas under `schemas/`, fixtures under `wire/golden/` and `wire/negative/`, generated-contract pipeline following the existing `contracts/catalog/` + `scripts/generate_contracts.py` pattern, project Python `.venv/bin/python` (never a developer-local path).

---

## 0. Plan gate, exact source identities, and branch policy

**Read before executing any task:**

- `docs/adr/0055-v5-execution-identity.md` (accepted ADR — architecture authority)
- `docs/superpowers/specs/2026-09-18-v5-execution-identity-implementation-design.md` (reviewed spec @ `84fc20e51c52253168029a65cbf63d6885abcc14` — the implementation contract; section references `§N` below point here)

**Execution identities (Plan Fix-02 — the PLANNING base and the IMPLEMENTATION base are distinct; never hard-pin the planning SHA for implementation):**

```text
PLANNING_BASE         = 6932a9bdd61a5ca3567b391c43db977a4b337a0a  (master this plan was written against)
REVIEWED_SPEC_HEAD    = 84fc20e51c52253168029a65cbf63d6885abcc14
REVIEWED_PLAN_HEAD    = 7484b2c35686647d640d5cae51c2870ce44f1d24 (or later reviewed plan-fix head)
IMPLEMENTATION_BASE   = the exact NEW origin/master AFTER spec + plan merge — frozen at branch creation
```

**Branch policy (binding):** Implementation starts ONLY after spec + plan are merged/accepted on `master` and implementation is explicitly authorized. BEFORE creating `chris/v5-execution-identity-implementation` from `origin/master`, the executor must PROVE on the live post-merge master:

```bash
git fetch origin && git rev-parse origin/master
# Blob-equality PROOFS against the independently-authorized reviewed blobs (plan Fix-03:
# these must be TESTED comparisons, not printed hashes):
SPEC_BLOB=$(git show origin/master:docs/superpowers/specs/2026-09-18-v5-execution-identity-implementation-design.md | git hash-object --stdin)
PLAN_BLOB=$(git show origin/master:docs/superpowers/plans/2026-09-19-v5-execution-identity-implementation.md | git hash-object --stdin)
test "$SPEC_BLOB" = "<spec blob hash from the reviewed branch>" || { echo SPEC_DRIFT; exit 1; }
test "$PLAN_BLOB" = "<plan blob hash from the reviewed branch>" || { echo PLAN_DRIFT; exit 1; }
# ADR 0055 remains accepted (against origin/master, not the worktree):
git show origin/master:docs/adr/0055-v5-execution-identity.md | grep -F -- "- **Status:** accepted" >/dev/null || { echo ADR_NOT_ACCEPTED; exit 1; }
git status --porcelain=v2 --branch   # clean worktree
```

(The two `<…>` blob hashes are recorded once by the independent plan review; the executor substitutes the exact hashes from the final reviewed heads before branching.)

Then freeze `IMPLEMENTATION_BASE = $(git rev-parse origin/master)` and branch from exactly that SHA. The implementation baseline gate below runs against `IMPLEMENTATION_BASE`, never against `PLANNING_BASE`.

**Baseline gate (first future execution step, before Task 1):**

```bash
git rev-parse HEAD && git rev-parse origin/master
cargo check --workspace --all-targets --all-features --locked
cargo test --workspace --all-features --locked
.venv/bin/python scripts/run_python_tests.py --profile full
.venv/bin/python scripts/verify_repository.py
.venv/bin/python scripts/check_documentation.py
```

If any of these fail on untouched `master`: `V5_IMPLEMENTATION = BLOCKED` — report, do not hide pre-existing failures.

**Stop conditions (every task):** the reviewed spec §46 list applies verbatim. Any hit ⇒ stop, report `IMPLEMENTATION_BLOCKED = YES`, do not redesign silently.

---

## Task 1 — Model V5 semantic vocabulary

**Objective:** All §5 model-owned types with closed-variant validation, in `crates/mtgml-model`.

**Files:** create `crates/mtgml-model/src/execution_identity.rs` and `crates/mtgml-model/src/semantic_contract.rs`; re-export from `crates/mtgml-model/src/lib.rs` (digest newtypes via the existing macro at `lib.rs:276`, `as_digest_reference()` helpers at `lib.rs:332/345`).

**Types:** `ExecutionProgramV1` (`SyntheticRulesCompat` | `MagicRules`; JSON strings `synthetic_rules_compat` / `magic_rules`), `ExecutionIdentityV1`, `RulesAuthorityV1` (`SyntheticLegacy` | `ComprehensiveRules { snapshot_id }`), `CapabilityRequirementV1 { key, version }`, `RulesContractManifestV1`, `RulesContractIdV1`, `SemanticContractManifestV1`, `SemanticContractIdV1`, reserved `FormatContractIdV1` / `ContentContractIdV1` (domain consts `mtgml.format-contract.v1` / `mtgml.content-contract.v1`, NO manifest schema, no constructor from arbitrary bytes), `CheckpointDigestV5` (newtype via macro).

**RED gate (Plan Fix-03: create the RED tests FIRST, then run — cargo runs ZERO tests and exits SUCCESS on an unmatched filter, which is not RED):** first add `crates/mtgml-model/tests/execution_identity_red.rs` and `crates/mtgml-model/tests/semantic_contract_red.rs` referencing the new types/APIs (integration-test files keep the production `lib.rs` wiring out of scope until GREEN), THEN run the commands below. Expected RED: compile failure on the unresolved types (a deliberate compile-contract RED — the missing types ARE the intended failure; accidental failures elsewhere do not count):

```bash
cargo test -p mtgml-model --test execution_identity_red
cargo test -p mtgml-model --test semantic_contract_red
```

Then implement (including the `lib.rs` re-exports) and run GREEN as:

```bash
cargo test -p mtgml-model --test execution_identity_red --test semantic_contract_red
```

covering the spec §11 matrix exactly, as separate structural-validation vs wire-decode tests: unknown `ExecutionProgramV1` string rejected (wire decode); `RulesAuthorityV1` closed variants (unknown variant rejected); capability key/version per spec §7c grammar (from `schemas/capability-registry.v1.schema.json`: key `^(rules|mechanic|decision|visibility|tooling|format/[a-z0-9-]+)/[a-z0-9][a-z0-9-]*(/[a-z0-9][a-z0-9-]*)*$`, version `^[0-9]+\.[0-9]+\.[0-9]+$` — invalid key, invalid version, duplicate key, unsorted closure rejected); `SyntheticLegacy` + non-null closure rejected; `ComprehensiveRules` + null/empty closure rejected; empty CR snapshot rejected; deny-unknown on every serde type; milestone-name values (`m3…`, `s1…`) rejected as program_kind strings.

**Negative/adversarial evidence:** the decode-rejection half of the same suite.

**Focused GREEN:** `cargo test -p mtgml-model --all-features`.

**Affected-package GREEN:** `cargo check --workspace --all-targets --all-features --locked` (no downstream breakage yet).

**Commit:** `model: V5 execution identity and semantic contract vocabulary`

---

## Task 2 — Canonical contract digest machinery (persistence)

**Objective:** `calculate_rules_contract_id_v1` and `calculate_semantic_contract_id_v1` in `crates/mtgml-persistence/src/` (new module `semantic_contract_digest.rs` beside `checkpoint_digest.rs`), byte-exact per spec §9 — Rust AND the Python mechanical mirror of BOTH functions in `python/src/mtgml/persistence.py` (Plan Fix-01: the Task 3 generator consumes these existing Python functions; it must not duplicate digest logic), plus the first Rust↔Python KAT parity for exactly these two IDs.

**Contracts (copy exactly):** envelope `mtgml.digest-envelope.v1`, algorithm `sha-256`, codec `mtgml.canonical-cbor.v1`; domains `mtgml.rules-contract.v1` / `mtgml.semantic-contract.v1`; input schemas `rules-contract-manifest.v1` / `semantic-contract-manifest.v1`; canonical payloads = spec §7 fixed 4-array (with `[variant_id, payload]` authority encoding and canonically sorted unique closure) and §8 fixed 5-array (`null` = absence, rules ID as raw 32-byte string). Reader-side canonical re-encode equality applies to any decode path.

**RED gate (create the RED tests FIRST — Plan Fix-03 sweep):** add `crates/mtgml-persistence/tests/semantic_contract_digest_red.rs` (importing the two new functions) and `python/tests/test_v5_contract_digest.py` (importing the two mirror functions), THEN run:

```bash
cargo test -p mtgml-persistence --test semantic_contract_digest_red
.venv/bin/python -m unittest python.tests.test_v5_contract_digest -v
```

Expected RED: the digest FUNCTIONS are missing (unresolved imports/functions in both suites) — not the test files; the RED evidence is created FIRST (add `python/tests/test_v5_contract_digest.py` importing the two mirror functions and the Rust `semantic_contract_digest` test module importing the two functions, then run). GREEN = spec §19.1/§19.2 KAT vectors (SyntheticLegacy manifest → bytes → digest; ComprehensiveRules minimal one-entry manifest → bytes → digest; synthetic rules ID + null + null; KAT-only hypothetical Magic rules ID + null + null — NOT a catalog entry) committed as shared fixtures and asserted byte-identically by both suites.

**Negative/adversarial evidence:** schema/domain disagreement rejected; malformed child digest length rejected.

**DISPOSITION_SCHEMA_DOMAIN_REJECTION_REQUIREMENT (authoritative, accepted 2026-09-19):** The
"schema/domain disagreement rejected" half of the gate above is **DEFERRED / NOT_RUN** for Task 2.
Task 2 ships only writer/calculator paths (`calculate_rules_contract_id_v1`,
`calculate_semantic_contract_id_v1`); no rules/semantic contract envelope decode surface exists
in any current representation — no decoder compares the payload's leading schema/domain fields
against envelope identity, and the cited `FullStateDigestInputV3` precedent implemented
detection (STATE_HASHING.md documents the check at decode) without a comparing rejection path in
production, while V4 validates digest equality, not leading-field agreement.
OWNER = the first actual rules/semantic contract envelope decode path.
CURRENT_V5_PLAN_OWNER = NONE — no task in this plan introduces such a decoder (Task 8 semantic
admission operates on already-constructed typed manifests, not on canonical-CBOR contract
envelopes, so the payload-vs-envelope comparison has no representation there).
The task that introduces the decoder — or a dedicated plan amendment creating it — MUST then
prove a genuine `payload leading schema/domain != envelope schema/domain → Err` case and mark
`SCHEMA_DOMAIN_DISAGREEMENT_REJECTED = PASS`; until then this gate remains
`DEFERRED / NOT_RUN` and must not be claimed by any other task.

**Focused GREEN:** `cargo test -p mtgml-persistence --all-features`. **Affected-package GREEN:** `cargo check --workspace --all-targets --all-features --locked`.

**Commit:** `persistence: rules/semantic contract digest machinery with rust/python parity KATs`

---

## Task 3 — Semantic-contract source, generator, and generated catalog constants

**Objective:** The spec §10 single-source-of-truth chain, resolving the filename the spec left open.

**Exact files:**

- Source of truth (the ONLY hand-authored manifest definition): `contracts/catalog/semantic-contracts.v1.json` (schema_version `semantic-contracts-catalog.v1`; one entry: SyntheticLegacy rules authority, null closure, null format/content dimensions)
- Generator: `scripts/generate_semantic_contract_catalog.py` (modeled on `scripts/generate_contracts.py`'s CATALOG/`--check`/drift-via-`SystemExit` pattern)
- Generated Rust output: `crates/mtgml-environment/src/semantic_catalog_generated.rs` (generated-file banner, do-not-edit note; emits manifest constants AND derived `RulesContractIdV1`/`SemanticContractIdV1` constants using Task 2 functions)
- Rust recompute KAT (THE one and only Rust semantic-catalog KAT location — there is NO external `tests/semantic_catalog_kat_red.rs` and NO other KAT copy): `crates/mtgml-environment/src/semantic_catalog_kat.rs` (spec §19.4: recompute every ID constant from the emitted manifest constants via the §9 functions; byte equality; drift fails the build); these are crate-internal tests for crate-internal catalog/ID surfaces — they must NOT force internal APIs to become public
- Module wiring (Plan Fix-03): `crates/mtgml-environment/src/lib.rs` — add `mod semantic_catalog_generated;` and `#[cfg(test)] mod semantic_catalog_kat;` (the generated module is otherwise not compiled and the KAT never runs); the KAT is reachable ONLY through this internal module wiring

**Generator semantics:** reads ONLY `contracts/catalog/semantic-contracts.v1.json` (no Rust parsing, no Python duplicate manifest, no invoking Rust); derives IDs via the Task 2 Python mirror functions (`python/src/mtgml/persistence.py::calculate_rules_contract_id_v1` / `calculate_semantic_contract_id_v1`) — NO digest logic in the generator itself; deterministic; `--check` mode fails on stale output; rerun produces zero diff.

**RED gate (create the RED evidence FIRST, in this exact order — the focused command is valid only AFTER the internal test module exists and is wired):** add `python/tests/test_semantic_contract_catalog_generator.py` (invoking the generator: deterministic emit, `--check` semantics, zero-diff rerun) AND create/wire the crate-internal Rust KAT test module first (`crates/mtgml-environment/src/semantic_catalog_kat.rs` + the `#[cfg(test)] mod semantic_catalog_kat;` line in `crates/mtgml-environment/src/lib.rs`), referencing the not-yet-existing generated catalog API/constants, THEN run:

```bash
.venv/bin/python -m unittest python.tests.test_semantic_contract_catalog_generator -v   # RED: generator behavior absent
cargo test -p mtgml-environment --all-features semantic_catalog_kat                     # RED: generated constants/API missing
```

Expected RED: the generator's behavior and the generated constants are missing — not merely the test files. The Rust RED MUST be caused by the missing production/generated API referenced by the internal KAT module (compile failure inside the wired test module), NEVER by zero matched tests (cargo exits SUCCESS on an unmatched filter — verify the failure output names the KAT module/missing API). GREEN = generator emits; `--check` passes; rerun `git diff --exit-code` on generated file; Rust KAT GREEN; generator unittest GREEN. After implementation, the focused GREEN commands are: `cargo test -p mtgml-environment --all-features semantic_catalog_kat` and the generator unittest above.

**Negative/adversarial evidence:** mutate one manifest fact in the source JSON (in a scratch copy or test): regenerated ID constants change; `--check` fails against stale generated output; no Magic production contract may be generated (source contains exactly one entry).

**Focused GREEN:** both commands above + `git diff --exit-code` after rerun. **Affected-package GREEN:** `cargo check -p mtgml-environment --all-features`.

**Commit:** `contracts: semantic-contract catalog source, generator, and generated constants`

---

## Task 4 — ProgramKernelV1 boundary (mtgml-rules)

**Objective:** Spec §23a.1 (Plan Fix-01 API closure): public `ProgramKernelV1` as an opaque struct wrapping a PRIVATE inner enum, so `SyntheticM1RulesKernel` stays private to `mtgml-rules` while `ProgramKernelV1` remains externally usable from `mtgml-environment` (no public variant exposing a private payload type; no literal bypass):

```rust
pub struct ProgramKernelV1 { inner: ProgramKernelInner }   // public, opaque
enum  ProgramKernelInner { SyntheticLegacy(SyntheticM1RulesKernel) }  // private; NO Magic variant pre-S1
impl ProgramKernelV1 {
    pub fn for_program(program_kind: ExecutionProgramV1)
        -> Result<ProgramKernelV1, ProgramKernelConstructionErrorV1>;   // ONLY construction path
    pub fn apply(&mut self, ...);            // dispatches to the kernel trait method
    pub fn advance_forced_progress(&mut self, ...);  // dispatches to the inherent method — legacy forced progress preserved
}
```

`ProgramKernelConstructionErrorV1` with variant `UnsupportedProgram`. Remove `#[derive(Default)]` from `SyntheticM1RulesKernel` (`synthetic.rs:54`); keep the unit struct module-private to `mtgml-rules`. Because the kernel becomes crate-internal, the PUBLIC re-export of `SyntheticM1RulesKernel` at `crates/mtgml-rules/src/lib.rs:24` (`pub use synthetic::{validate_synthetic_runtime_state, SyntheticM1RulesKernel};`) MUST be removed in this task — `validate_synthetic_runtime_state` stays exported, `SyntheticM1RulesKernel` does not; no public synthetic-kernel escape may remain after Task 4. Export path (Plan Fix-05): the boundary owns the new module `crates/mtgml-rules/src/program_kernel.rs` (declaring `ProgramKernelV1`, the private `ProgramKernelInner`, `ProgramKernelConstructionErrorV1`, and the single allowed `ProgramKernelInner::SyntheticLegacy(SyntheticM1RulesKernel)` construction), and `crates/mtgml-rules/src/lib.rs` wires and re-exports exactly:

```rust
mod program_kernel;
pub use program_kernel::{ProgramKernelConstructionErrorV1, ProgramKernelV1};
```

with line 24 reduced to `pub use synthetic::validate_synthetic_runtime_state;` — external `mtgml-environment` and the `p0_red` integration test consume the boundary ONLY through these public exports (without this explicit export wiring the task is incomplete and risks a visibility/compile failure downstream).

**Migration census (Plan Fix-03, fully grep-verified at baseline; `PROGRAM_OWNS_ALL_KERNEL_ENTRYPOINTS` closes only when EVERY literal outside the owning declaration module migrates):**

- `crates/mtgml-rules/src/lib.rs:24` — public re-export `pub use synthetic::{validate_synthetic_runtime_state, SyntheticM1RulesKernel};` → REMOVE the `SyntheticM1RulesKernel` name from the re-export (keep `validate_synthetic_runtime_state`); the kernel becomes crate-internal, so its public export must die with this task;
- `crates/mtgml-environment/src/synthetic.rs` — import at `:11` (`use mtgml_rules::{SyntheticM1RulesKernel, TransitionResult};`) drops `SyntheticM1RulesKernel`; production literals at `:113` (`new()`), `:140` (`from_checkpoint()`), `:165` (`restore()` reset) → construct via `ProgramKernelV1::for_program(ExecutionProgramV1::SyntheticRulesCompat)`/dispatch (the struct field at `:81` becomes `ProgramKernelV1`);
- forced-progress call sites `crates/mtgml-environment/src/synthetic/commit.rs:97`, `:219` → dispatch via `ProgramKernelV1`; test call site `crates/mtgml-environment/src/tests/forced_progress.rs:199` → same;
- `crates/mtgml-rules/src/tests.rs` (`:135`, `:154`, `:170`), `crates/mtgml-rules/src/tests/determinism.rs` (`:8/:9`, `:34/:35`), `crates/mtgml-rules/src/tests/forced_progress.rs` (`:14`), `crates/mtgml-rules/src/tests/synthetic_program.rs` (`:337`, `:425`), `crates/mtgml-rules/src/tests/transition_contract.rs` (`:84`, `:100`, `:151`, `:165`, `:178`, `:216`) → migrate to `ProgramKernelV1::for_program(SyntheticRulesCompat)` + dispatch (crates-internal tests construct through the public boundary like any external consumer);
- `crates/mtgml-rules/tests/p0_red.rs` — external integration-test crate importing `SyntheticM1RulesKernel` (`:7`) and constructing it (`:55`): P0-frozen RED evidence — migrate its CONSTRUCTION to `ProgramKernelV1::for_program(...)` while asserting the SAME frozen expectations (the P0 claims must keep passing; the construction path changes, the asserted evidence does not); if the private kernel makes a frozen import impossible, the test migrates to the boundary API and the historical claims remain byte-for-byte asserted;
- `crates/mtgml-environment/src/tests/checkpoint_replay.rs:741` → `for_program`;
- `crates/mtgml-environment/src/tests/forced_progress.rs:198` — direct `mtgml_rules::SyntheticM1RulesKernel` construction → `ProgramKernelV1::for_program(SyntheticRulesCompat)` (its `:199` forced-progress call then dispatches through the boundary);
- allowed to REMAIN (internal implementation references, never constructions): the declaration and impls in `crates/mtgml-rules/src/synthetic.rs` (`:55`, `:57`, `:102`), the child-module impl `crates/mtgml-rules/src/synthetic/stages.rs` (`:13` import, `:18` impl), and the `ProgramKernelInner::SyntheticLegacy(SyntheticM1RulesKernel)` construction in the new kernel-boundary module.

Ownership rule: `SyntheticM1RulesKernel`'s DECLARATION stays in its owning implementation module (`crates/mtgml-rules/src/synthetic.rs`); `ProgramKernelInner::SyntheticLegacy(...)` is the ONLY allowed construction owner; ALL other direct literals migrate to `ProgramKernelV1::for_program(...)`.

**RED gate (create the RED tests FIRST):** add `crates/mtgml-rules/tests/program_kernel_red.rs` referencing `ProgramKernelV1`/`ProgramKernelConstructionErrorV1`, THEN run:

```bash
cargo test -p mtgml-rules --test program_kernel_red
```

Expected RED: compile failure on the missing types (intended compile-contract RED). GREEN: `for_program(SyntheticRulesCompat)` → synthetic kernel (behavior unchanged); `for_program(MagicRules)` → `Err(ProgramKernelConstructionErrorV1::UnsupportedProgram)`; both entry points dispatch; pre-S1 enum has no Magic variant.

**Negative/adversarial evidence (residual gate — Plan Fix-03/Plan Fix-04: NO whole-file or whole-directory exemption may serve as proof; a `grep -v` on `crates/mtgml-rules/src/synthetic.rs` would hide future accidental constructions inside the entire file):** the immediate Task-4 evidence MAY use `rg`/grep to DISPLAY remaining references, but the acceptance claim is enforced by the exact T12 gate classifier (`scripts/run_v5_execution_identity_gate.py`, created RED in Task 12), which classifies each remaining reference NARROWLY:

```text
ALLOWED:  declaration of SyntheticM1RulesKernel; impl SyntheticM1RulesKernel blocks;
          internal child-module type/impl references required by its implementation
          (e.g. synthetic/stages.rs); the single
          ProgramKernelInner::SyntheticLegacy(SyntheticM1RulesKernel) construction
          owned by the kernel-boundary module.
FORBIDDEN: public re-export of SyntheticM1RulesKernel; external imports
          (mtgml_rules::… or use mtgml_rules::{… SyntheticM1RulesKernel …});
          direct literals in environment code; direct literals in tests;
          direct literals in tools; alternative semantic construction paths.
```

The classifier matches per-line/per-site (declaration/impl/construction/import), never per-file: no path is wholesale-exempted. Gate failures name path/token/line/classification.

**Focused GREEN:** `cargo test -p mtgml-rules --all-features` + `cargo test -p mtgml-environment --all-features` (call-site migration). **Affected-package GREEN:** workspace `cargo check`.

**Commit:** `rules: program-owned kernel boundary with both entry points`

---

## Task 5 — Legacy parity lock before persistence cut

**Objective:** Spec §16/§43 — prove `SyntheticRulesCompat` semantics are byte-equivalent BEFORE any consumer migration, on the current V4 persistence surface plus the new kernel boundary.

**Evidence (existing suites re-run + targeted parity evidence, not just "tests pass"):**

```bash
cargo test -p mtgml-environment --all-features
cargo test -p mtgml-conformance --all-features
```

Targeted: record→transition→record round-trips assert exact `EngineState`, `StateDelta` meaning, authoritative events, Decision products, observations, information state, observed events, `PlayerStep`, `EpisodeStatus`, RNG state/consumption, `FullStateDigestV4`, and forced-progress behavior against pre-Task-4 recorded expectations (the existing `replay_parity_tests.rs` and conformance isolation suites provide this; extend with a full-state-digest equality assertion over a forced-progress episode if absent).

**Gate:** `LEGACY_SEMANTIC_PARITY = PASS` recorded in the task log with the exact commands. Failure ⇒ STOP (spec §46).

**Commit:** none (evidence-only task; record results in the task log). If a parity assertion must be added, commit it as `test: legacy synthetic parity lock before V5 persistence cut`.

---

## Task 6 — CheckpointDigestV5 (persistence)

**Objective:** `calculate_checkpoint_digest_v5` in `crates/mtgml-persistence/src/checkpoint_digest.rs`, exactly spec §9's seven-element payload: V4's verified six elements (`environment-checkpoint-digest-input.v5`, `mtgml.checkpoint-digest.v5`, full-state `DigestReferenceV1` unchanged against `full-state-digest-input.v4` + `FullStateDigestV4::DOMAIN`, episode status, counters, `["in-memory-reference", "5"]` FROZEN) + `ExecutionIdentityV1` as LAST element (`[program_kind_variant, semantic_contract_id_32bytes]`).

**RED gate (create the RED test FIRST — exactly ONE location: the external integration-test file `crates/mtgml-persistence/tests/checkpoint_digest_v5_red.rs`; do NOT also add an inline test module to `checkpoint_digest.rs`, the inline-unit-test option is removed):** add `crates/mtgml-persistence/tests/checkpoint_digest_v5_red.rs` referencing the new function, THEN run:

```bash
cargo test -p mtgml-persistence --test checkpoint_digest_v5_red
```

Expected RED: compile failure on the missing function/domain (intended compile-contract RED). GREEN = spec §19.3 KATs: one fixed V4-equivalent input; mutation vectors — different `program_kind` ⇒ different digest; different `semantic_contract_id` ⇒ different digest; identical input ⇒ identical digest. Codec semantic_version `"5"` enforced; `"4"` rejected.

**Negative/adversarial evidence:** wrong codec pair rejected; full-state reference domain mismatch rejected.

**Focused GREEN:** `cargo test -p mtgml-persistence --all-features`. **Affected-package GREEN:** workspace `cargo check`.

**Commit:** `persistence: checkpoint digest v5 with execution identity`

---

## Task 7 — EnvironmentCheckpointV5

**Objective:** Spec §11 beside retained V4 in `crates/mtgml-environment/src/checkpoint.rs`: consts `ENVIRONMENT_CHECKPOINT_SCHEMA_V5 = "environment-checkpoint.v5"`, `CHECKPOINT_CODEC_ID_V5 = "in-memory-reference"`, `CHECKPOINT_CODEC_SEMANTIC_VERSION_V5 = "5"`; struct with unchanged `EngineState`/`FullStateDigestV4`, plus `execution_identity: ExecutionIdentityV1` and `checkpoint_digest: CheckpointDigestV5`; `new()`/`validate()` mirroring V4 with digest recompute FROM the stored identity.

**RED gate (create the RED test FIRST):** add `crates/mtgml-environment/tests/checkpoint_v5_red.rs` referencing `EnvironmentCheckpointV5`, THEN run:

```bash
cargo test -p mtgml-environment --test checkpoint_v5_red
```

Expected RED: compile failure on the missing type (intended compile-contract RED). GREEN: validate chain green; V4 tests untouched and still green.

**Negative/adversarial evidence:** tampered `execution_identity` ⇒ digest mismatch rejection; tampered digest ⇒ rejection; codec `"4"` rejected; completed-with-pending-decision rule preserved.

**Focused GREEN:** `cargo test -p mtgml-environment --all-features`. **Affected-package GREEN:** workspace `cargo check`.

**Commit:** `environment: checkpoint v5 with execution identity binding`

---

## Task 8 — RuntimeSemanticCatalog + restore admission + error taxonomy

**Objective:** Spec §10/§12/§18 in `crates/mtgml-environment` (new module `semantic_catalog.rs`): catalog consuming ONLY generated Task 3 constants (no digest generation, no mutable/lazy state, no filesystem/network/env lookup; `resolve(id)` vs `supported(id, program)` distinction); the spec §12 nine-phase admission order owned exactly as its table states; error variants added to `CheckpointValidationError`/`ControllerError` (`ExecutionIdentity`, `SemanticContractUnknown`, `SemanticContractDigestMismatch`, `RulesContractDigestMismatch`, `ProgramAuthorityMismatch`, `SemanticContractUnsupported`, `ProgramStateIncompatible`) with the deterministic `ProgramKernelConstructionErrorV1 → SemanticContractUnsupported/ControllerError` mapping. **Scope guard (Plan Fix-02):** this task builds the catalog and the STATELESS/PURE V5 admission machinery + typed errors at FUNCTION level. The real `EnvironmentBackend::restore` / `TrustedEnvironmentController::restore` surfaces are still V4 at this point (their migration is Task 13) — Task 8 must NOT migrate them and must NOT invent a parallel temporary restore API; wiring admission into controller restore and proving controller-level rejected-restore nonmutation happen in Task 13.

**RED gate (create the RED tests FIRST — crate-INTERNAL test files, because the catalog/admission APIs they exercise are private/`pub(crate)` implementation details and must NOT be made public solely for testing; external integration tests under `crates/mtgml-environment/tests/` cannot see them):** add `crates/mtgml-environment/src/tests/semantic_catalog.rs` and `crates/mtgml-environment/src/tests/restore_admission.rs`, wired in `crates/mtgml-environment/src/tests.rs` as NAMED INNER MODULES wrapping the includes — a naked `include!` creates NO `semantic_catalog::…` module path and would NOT guarantee the filters below:

```rust
mod semantic_catalog {
    use super::*;
    include!("tests/semantic_catalog.rs");
}

mod restore_admission {
    use super::*;
    include!("tests/restore_admission.rs");
}
```

referencing the new module/API/errors, THEN run (test-name filters — do NOT use `--test`, these are not integration-test binaries):

```bash
cargo test -p mtgml-environment --all-features semantic_catalog
cargo test -p mtgml-environment --all-features restore_admission
```

Expected RED: compile failure on the missing module/variants (intended compile-contract RED) INSIDE the wired internal test modules — never zero matched tests; because the named wrapper modules make the test paths `tests::semantic_catalog::…` / `tests::restore_admission::…`, the name filters below are GUARANTEED to match them (an unmatched-filter success cannot masquerade as RED). GREEN covers: SyntheticLegacy resolves; unknown semantic ID rejects; known-meaning ≠ supported-execution; `MagicRules` resolves to NO synthetic contract (catalog inputs match generated constants); each of the nine phases rejects in its own typed failure family — asserted at the admission-function level over constructed V5 checkpoints (NOT via `TrustedEnvironmentController::restore`, which stays V4 until Task 13). Controller-level rejected-restore nonmutation (pre/post `checkpoint()` byte-equality for every rejection phase; spec §12 observable invariant) is proven in Task 13 when the real surfaces flip.

**Negative/adversarial evidence:** program × authority mismatch; runtime-unsupported; recompute-mismatch of top-level/rules IDs at admission (invariant-breach classification).

**Focused GREEN:** `cargo test -p mtgml-environment --all-features`. **Affected-package GREEN:** `cargo test --workspace --all-features --locked`.

**Commit:** `environment: runtime semantic catalog, fail-closed admission, error taxonomy`

---

## Task 9 — Replay V5 (mtgml-replay)

**Objective:** Spec §13 in new `crates/mtgml-replay/src/v5.rs` (consts `replay-manifest.v5` / `authoritative-replay.v5` / `replay-step.v5`): `ReplaySchemaVersionsV5`, `InitialEnvironmentIdentityV5` (V4's 6 fields + V5 digest + `execution_identity`), `SemanticContractMaterialV5`, `ReplayManifestV5` (V4 fields + `execution_identity` + `semantic_contract`; `KernelIdentityV1` retained as provenance; `rules_snapshot` retained as provenance), `ReplayStepV5` (mechanical cut: identical 11-field shape, `CheckpointDigestV5` before/after, `replay-step.v5`; NO forced-progress steps, NO fabricated decisions), `AuthoritativeReplayV5` (+ `final_identity.execution_identity`), `ReplayRecorderV5`. Detached `validate()` = V4 chain-walk + spec §13 additions: semantic manifest hashes to material ID; rules manifest hashes to recomputed rules ID; three-way identity equality; child-ID nulls; `comprehensive_rules` rules_snapshot equality.

**RED gate (create the RED test FIRST):** add `crates/mtgml-replay/tests/replay_v5_red.rs` referencing the V5 types, THEN run:

```bash
cargo test -p mtgml-replay --test replay_v5_red
```

Expected RED: compile failure on the missing module/types (intended compile-contract RED). GREEN: record→validate→export round-trip; spec §24 three-way binding tests (each mismatch ⇒ detached rejection BEFORE any backend execution); spec §25 families (SyntheticLegacy ⇒ `rules_snapshot` informational; ComprehensiveRules ⇒ mismatch ⇒ detached rejection — KAT/validation-only, NO runtime Magic contract).

**Negative/adversarial evidence:** every detached rejection classified artifact-validation, mutating nothing.

**Focused GREEN:** `cargo test -p mtgml-replay --all-features`. **Affected-package GREEN:** workspace `cargo check`.

**Commit:** `replay: authoritative replay v5 with three-way identity binding`

---

## Task 10 — Wire/schema V5 + fixtures

**Objective:** Spec §14/§7b/§14.1 exactly: `schemas/replay-manifest.v5.schema.json`, `schemas/authoritative-replay.v5.schema.json` (consts, `program_kind` enum, `^[0-9a-f]{64}$` digests, required identity objects in manifest/initial/final, `additionalProperties: false`); `WireContract` impls + dispatch in `crates/mtgml-wire/src/replay.rs` beside retained V4; golden + negative fixtures under `wire/golden/`, `wire/negative/` (`*-v5-*` naming).

**Required negatives (each classified, from spec §19):** unknown `program_kind`; wrong digest length; semantic-contract mismatch; rules-contract mismatch; `rules_snapshot` mismatch (CR); identity three-way mismatch; unknown field; wrong schema version.

**RED gate (Plan Fix-01/Fix-02: `validate_schemas.py` alone cannot RED on absent V5 — it validates its existing V1–V4 inventory; the inventory test is the seam):** FIRST add the test method `SchemaParityTests.test_v5_replay_schemas_are_inventoried` to `python/tests/test_schema_parity.py` (asserting both V5 schemas in `WIRE_MAPPING` and `schemas/README.json`), THEN run:

```bash
.venv/bin/python -m unittest python.tests.test_schema_parity.SchemaParityTests.test_v5_replay_schemas_are_inventoried -v
```

Expected RED: `V5 schema inventory absent` — failing on the missing V5 mapping/inventory, not on a missing test. Implement the inventory/mapping entries, schemas, dispatch, and fixtures; then:

```bash
.venv/bin/python scripts/validate_schemas.py
cargo test -p mtgml-wire --all-features v5
```

GREEN: inventory test passes; `validate_schemas.py` green over V1–V5; positive V5 fixtures pass; every negative fails closed with the expected classification; V4 fixtures untouched and passing.

**Focused GREEN:** the two commands. **Affected-package GREEN:** `cargo test --workspace --all-features --locked`.

**Commit:** `wire: replay v5 schemas, dispatch, and positive/negative fixtures`

---

## Task 11 — Python V5 mechanical mirror + byte parity

**Objective:** Spec §15/§14.1: `python/src/mtgml/persistence.py` gains `calculate_checkpoint_digest_v5` (the two contract-ID mirrors already landed in Task 2; byte-exact mirror; V4 functions retained); new `python/src/mtgml/_replay_v5.py` (V5 DTOs, `from_wire`/`to_wire`, deny-unknown, detached recompute chain incl. three-way equality and CR snapshot equality); `replay.py`/`wire.py`/`__init__.py` re-exports per actual ownership (`replay.py:35` currently re-exports `_replay_v4`). Python does NOT decide legality/support/admission.

**RED gate (create the RED test modules FIRST):** add `python/tests/test_v5_persistence.py` (importing `calculate_checkpoint_digest_v5` from `python/src/mtgml/persistence.py`) and `python/tests/test_v5_replay.py` (importing the `_replay_v5` DTOs), THEN run:

```bash
.venv/bin/python -m unittest python.tests.test_v5_persistence python.tests.test_v5_replay -v
```

Expected RED: import/attribute failures naming the MISSING PRODUCTION API (the checkpoint-digest mirror and the V5 DTOs do not exist yet) — not module-file-not-found; the Task 2 contract-ID mirrors already exist and stay green. GREEN: shared KAT vectors byte-identical Rust↔Python for spec §19.1–§19.5 (commands: `cargo test -p mtgml-persistence --all-features semantic_contract_digest` then `cargo test -p mtgml-persistence --all-features checkpoint_digest_v5`; Python `python -m unittest python.tests.test_v5_persistence -v`; vectors read from the same committed fixture files).

**Negative/adversarial evidence:** Python rejects every §19 negative-fixture case mechanically.

**Focused GREEN:** the unittest command. **Affected-package GREEN:** `.venv/bin/python scripts/run_python_tests.py --profile full`.

**Commit:** `python: v5 mechanical mirror with rust/python byte parity KATs`

---

## Task 12 — V5 gate script BEFORE migration (RED by design)

**Objective:** Create `scripts/run_v5_execution_identity_gate.py` FIRST (Plan Fix-01: this is TDD for the migration itself — the gate must observe its RED state while current producers are still V4, which is impossible if it is built after Task 13): assert the V5-current tokens, the §23a.1 residual-kernel-construction ALLOWLIST via a deterministic script-level site classifier (Task 4's ALLOWED/FORBIDDEN classification — declaration/impl/internal-child-module/single `ProgramKernelInner` construction allowed; public re-exports, external imports, direct literals in environment code/tests/tools, and alternative semantic construction paths forbidden; per-site classification, never whole-file `grep -v` exemptions), and RESIDUAL_V4_CURRENT_PRODUCER_ZERO via the §22 allowlist — V4 tokens permitted ONLY in RETAIN rows; failure output names path, token, line, expected disposition.

**RED gate (expected to FAIL at this point — that IS the evidence):**

```bash
.venv/bin/python scripts/run_v5_execution_identity_gate.py
```

Expected RED: current producers are still V4 → named `CURRENT_*` census violations (path/token/line/disposition). Nothing is fixed in this task.

**Negative/adversarial evidence:** scratch-test a violation → gate names it; revert scratch.

**Commit:** `gate: v5 execution identity gate with residual-v4 enforcement (red until migration)`

---

## Task 13 — Current producer/consumer migration + conformance/parity closure (driven by the gate)

**Objective:** Flip every §22 `CURRENT_*` census row to V5: environment producer paths (`synthetic.rs`, `synthetic/commit.rs`, `synthetic/replay.rs:56` manifest construction, `controller.rs`, `replay.rs`, `replay_parity_tests.rs`, `tests.rs`, `lib.rs` re-exports), replay current recorder/export, conformance consumers (`facade.rs`, `lib.rs`, `lifecycle.rs`, `isolation/{paired,replay_parity,checkpoint_parity,fork_parity,rejection,fingerprint,endpoint_pair}.rs`, `legal_space/gate_evidence.rs`), Python public surfaces, and `tools/m2-semantic-adapter` runtime construction path (`session.rs::reset_synthetic` → V5 config/codec/replay-schema; its historical M2 validation evidence stays V4 historical, never reinterpreted — spec §30). Flip the real runtime surfaces HERE (Plan Fix-02): `EnvironmentBackend::checkpoint/restore/export_replay`, `TrustedEnvironmentController::checkpoint/restore`, and `execute_replay_from_checkpoint` become V5, wiring the Task 8 admission into restore — then prove controller-level rejected restore: pre/post `checkpoint()` byte-equality for every Task 8 rejection phase (spec §12 observable invariant; the `REJECTED_RESTORE_NONMUTATION` evidence lives HERE, not in Task 8). Historical V4 rows (§22 RETAIN) untouched. (`run_m2_final_closure.py` posture comment belongs to Task 14, not here — single owner.)

**Parity closure (spec §20/§45):** fork preserves `ExecutionIdentityV1`/`SemanticContractIdV1`/digest identity with identical behavior until explicit divergence; record live → Replay V5 → detached validate → execute from V5 checkpoint → exact parity; same checkpoint + same identity + same actions + same RNG ⇒ same replay/final state; different identity ⇒ different `CheckpointDigestV5`.

**Information-safety review (spec §44):** grep player-facing surfaces (`PlayerObservation`, `PlayerInformationState`, `PlayerStep`, decision products, player-visible events in `mtgml-observation`/`mtgml-decision`/`mtgml-wire`) for `ExecutionIdentityV1|SemanticContractIdV1|RulesContractIdV1|RuntimeSemanticCatalog|checkpoint_digest` — expected `PLAYER_INFORMATION_LEAK = NONE`; record as gate evidence.

**RED gate (Plan Fix-01: the gate from Task 12 is the characterizing RED, NOT a workspace compile break):**

```bash
.venv/bin/python scripts/run_v5_execution_identity_gate.py
```

Work the migration until the gate passes (each gate run names the remaining `CURRENT_*` rows); a mid-migration workspace compile break is incidental, never evidence. After the gate goes GREEN:

```bash
cargo test --workspace --all-features --locked
```

**Negative/adversarial evidence:** rejected-restore nonmutation re-run per §19 runtime/admission cases; fork/replay parity suites green.

**Focused GREEN:** workspace tests + conformance suite. **Affected-package GREEN:** full `cargo test` + `.venv/bin/python scripts/run_python_tests.py --profile full`.

**Commit:** `migrate current producers to v5 execution identity (conformance + adapter included)`

---

## Task 14 — FAST/justfile wiring + documentation closure

**Objective:** Spec §21/§21b wiring (the gate script itself already exists from Task 12) and ADR §2.16 documentation closure.

**Files:** `scripts/verify_repository.py` (V4-current block → V5-current tokens + residual checks); `scripts/run_checks.py` (append `scripts/run_v5_execution_identity_gate.py` to `FAST`, so it runs in PR Fast, Windows Setup Smoke, PR Integration (integration = FAST + extras), Integration/master, Nightly (certification = FAST + integration extras + certification extras)); `justfile` `contracts` recipe (direct invocation beside `verify_repository.py`; include `generate_semantic_contract_catalog.py --check` there and inside the gate script); `scripts/run_m2_b_contract_cut.py` (EXISTING PATH/NAME RETAINED — logical split only, NO_RENAME, allowed historical-posture line only); `scripts/run_m2_final_closure.py` (posture comment only); `python/tests/test_current_status.py` (audit classification per spec §21 — verified at the planning baseline it carries NO direct V4 checkpoint/replay pin, so update ONLY if its current-runtime pins are affected by the V5 cut; record `VERIFIED_NO_CHANGE` when untouched); docs per spec §22: `docs/contracts/ENGINE_STATE_CLOSURE.md`, `docs/STATE_HASHING.md`, `docs/REPLAY_AND_DETERMINISM.md`, `docs/contracts/WIRE_CONTRACT.md`, `docs/maintenance/API_LIFECYCLE.md` (V4 sections retained as DOC_HISTORY + V5 sections added — docs FOLLOW the executable implementation, never ahead of it).

**RED gate (create the RED evidence FIRST — Plan Fix-02: `run_checks.py fast` is GREEN today precisely because it does not know the V5 gate; the profile/entry-point tests are the seam):** add `test_fast_profile_includes_v5_execution_identity_gate` to `python/tests/test_python_test_profiles.py` (the `FAST` list must contain `scripts/run_v5_execution_identity_gate.py`) and a maintainer-entry-point assertion that `justfile contracts` includes `run_v5_execution_identity_gate.py` and `generate_semantic_contract_catalog.py --check`, THEN run:

```bash
.venv/bin/python -m unittest python.tests.test_python_test_profiles -v    # RED: FAST/justfile wiring absent
```

Expected RED: the wiring assertions fail — not the test file. Implement the `run_checks.py` FAST entry, `verify_repository.py` V5 tokens, and the `justfile contracts` lines; then:

```bash
.venv/bin/python -m unittest python.tests.test_python_test_profiles -v
.venv/bin/python scripts/run_checks.py fast
```

**Negative/adversarial evidence:** `generate_semantic_contract_catalog.py --check` fails on a scratch-stale generated file; revert.

**Focused GREEN:** `run_checks.py fast` + `.venv/bin/python scripts/check_documentation.py`.

**Commits:** `gate: wire v5 gate into fast checks and repository verification` then `docs: v5 execution identity contract documentation closure`

---

## Final verification matrix (exact commands, after Task 14)

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
.venv/bin/python scripts/run_python_tests.py --profile full
.venv/bin/python scripts/generate_contracts.py --check
.venv/bin/python scripts/generate_semantic_contract_catalog.py --check
.venv/bin/python scripts/verify_repository.py
.venv/bin/python scripts/run_v5_execution_identity_gate.py
.venv/bin/python scripts/check_rust_source_structure.py
.venv/bin/python scripts/check_documentation.py
.venv/bin/python scripts/validate_schemas.py
.venv/bin/python scripts/validate_maintainer_artifacts.py
.venv/bin/python scripts/validate_golden_path.py
.venv/bin/python scripts/run_checks.py fast
.venv/bin/python scripts/run_checks.py integration
.venv/bin/python scripts/run_checks.py certification
git diff --check
git status --porcelain   # clean except untracked .freebuff/
```

Hosted evidence: PR Fast, PR Integration, Windows Setup Smoke, Nightly all success on the implementation head (`HOSTED_CI`).

## Acceptance matrix (spec §27 → tasks)

| Criterion | Task + evidence |
|---|---|
| V5_TYPES | T1/T4/T6–T9; `cargo check --workspace --all-targets --all-features --locked` |
| RULES_CONTRACT_KATS | T2 (+T11) `cargo test -p mtgml-persistence … semantic_contract_digest` |
| SEMANTIC_CONTRACT_KATS | T2/T3 (+T11) KAT suites |
| CHECKPOINT_V5_KATS | T6 (+T11) `checkpoint_digest_v5` mutation vectors |
| RUST_PYTHON_BYTE_PARITY | T11 shared KAT commands |
| SYNTHETIC_LEGACY_PARITY | T5 lock + T13 re-run; golden player bytes unchanged |
| CHECKPOINT_RESTORE_PARITY | T13 record→checkpoint→restore→resume suites |
| FORK_PARITY | T13 fork proof (identity preserved, §20) |
| REPLAY_V5_PARITY | T9/T13 round-trip suites |
| REJECTED_RESTORE_NONMUTATION | T13 controller-restore nonmutation (T8 admission machinery) |
| UNKNOWN_CONTRACT_FAIL_CLOSED | T8 catalog/admission tests |
| PROGRAM_AUTHORITY_MISMATCH_FAIL_CLOSED | T8 admission tests |
| RULES_SNAPSHOT_MISMATCH_FAIL_CLOSED | T9 detached rejection tests |
| WIRE_POSITIVE_FIXTURES | T10 golden fixtures |
| WIRE_NEGATIVE_FIXTURES | T10 classified negatives |
| SCHEMA_VALIDATION | T10 `validate_schemas.py` (hosted jsonschema gate) |
| RESIDUAL_V4_CURRENT_PRODUCER_ZERO | T12 gate (created RED) → GREEN at T13; §22 allowlist |
| HISTORICAL_V4_EVIDENCE_PRESERVED | T7/T10/T13: V4 fixtures/KATs/schemas untouched, still passing |
| MAINTAINER_GATES | T12/T14: gate script, verify_repository tokens, FAST wiring, b-cut logical-split posture (existing path/name retained, NO_RENAME) |
| HOSTED_CI | Final matrix hosted runs |

Traceability: ADR 0055 requirements, spec §1–§28, the §22 residual-V4 census, and the §23/§23a direct-constructor census are mapped task-by-task above (`ADR_0055_REQUIREMENTS_MAPPED = YES`, `SPEC_SECTIONS_MAPPED = YES`, `ACCEPTANCE_CRITERIA_MAPPED = YES`, `RESIDUAL_V4_CENSUS_MAPPED = YES`, `DIRECT_CONSTRUCTOR_CENSUS_MAPPED = YES`).

## S1 firewall (binding for every task)

V5 may add: the `MagicRules` enum value, Magic-compatible manifest validation (KAT-only), the `MagicRules → UnsupportedProgram` dispatch path, admission infrastructure. V5 may NOT add: Untap Step, untap eligibility, Beginning Phase progression, Upkeep, priority, Magic events/deltas, real capability execution, or a first production Magic semantic contract. The S1 firewall stop condition fires the moment any of these becomes necessary.

## Final implementation authorization boundary

```text
ADR_0055 = ACCEPTED

V5_IMPLEMENTATION_SPEC = REVIEWED_PASS
V5_IMPLEMENTATION_PLAN = REVIEW_CANDIDATE

V5_IMPLEMENTATION_AUTHORIZED = NO
V5_TYPES_SCAFFOLD_AUTHORIZED = NO
V5_RED_AUTHORIZED = NO
V5_GREEN_AUTHORIZED = NO

S1_TYPES_SCAFFOLD_AUTHORIZED = NO
S1_RED_AUTHORIZED = NO
S1_GREEN_AUTHORIZED = NO

REAL_MAGIC_RULES = NO

NEXT_TASK = EXACT_HEAD_REVIEW_V5_IMPLEMENTATION_PLAN
```
