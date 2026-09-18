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

**Execution preconditions (all must hold before Task 1):**

```bash
git fetch origin
git rev-parse origin/master        # must be 6932a9bdd61a5ca3567b391c43db977a4b337a0a
git status --porcelain=v2 --branch # must be clean
```

**Branch policy (binding):** Implementation starts ONLY after spec + plan are merged/accepted on `master`, the new `master` SHA is re-verified, and implementation is explicitly authorized. Create the future branch `chris/v5-execution-identity-implementation` from that verified `master`. No implementation may start from a documentation branch.

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

**RED gate** (compile-contract REDs are intended here — the missing types ARE the expected failure; accidental failures elsewhere do not count):

```bash
cargo test -p mtgml-model --all-features execution_identity
cargo test -p mtgml-model --all-features semantic_contract
```

Expected RED: unresolved types. Then implement; the same command must go GREEN covering the spec §11 matrix exactly, as separate structural-validation vs wire-decode tests: unknown `ExecutionProgramV1` string rejected (wire decode); `RulesAuthorityV1` closed variants (unknown variant rejected); capability key/version per spec §7c grammar (from `schemas/capability-registry.v1.schema.json`: key `^(rules|mechanic|decision|visibility|tooling|format/[a-z0-9-]+)/[a-z0-9][a-z0-9-]*(/[a-z0-9][a-z0-9-]*)*$`, version `^[0-9]+\.[0-9]+\.[0-9]+$` — invalid key, invalid version, duplicate key, unsorted closure rejected); `SyntheticLegacy` + non-null closure rejected; `ComprehensiveRules` + null/empty closure rejected; empty CR snapshot rejected; deny-unknown on every serde type; milestone-name values (`m3…`, `s1…`) rejected as program_kind strings.

**Negative/adversarial evidence:** the decode-rejection half of the same suite.

**Focused GREEN:** `cargo test -p mtgml-model --all-features`.

**Affected-package GREEN:** `cargo check --workspace --all-targets --all-features --locked` (no downstream breakage yet).

**Commit:** `model: V5 execution identity and semantic contract vocabulary`

---

## Task 2 — Canonical contract digest machinery (persistence)

**Objective:** `calculate_rules_contract_id_v1` and `calculate_semantic_contract_id_v1` in `crates/mtgml-persistence/src/` (new module `semantic_contract_digest.rs` beside `checkpoint_digest.rs`), byte-exact per spec §9 — Rust AND the Python mechanical mirror of BOTH functions in `python/src/mtgml/persistence.py` (Plan Fix-01: the Task 3 generator consumes these existing Python functions; it must not duplicate digest logic), plus the first Rust↔Python KAT parity for exactly these two IDs.

**Contracts (copy exactly):** envelope `mtgml.digest-envelope.v1`, algorithm `sha-256`, codec `mtgml.canonical-cbor.v1`; domains `mtgml.rules-contract.v1` / `mtgml.semantic-contract.v1`; input schemas `rules-contract-manifest.v1` / `semantic-contract-manifest.v1`; canonical payloads = spec §7 fixed 4-array (with `[variant_id, payload]` authority encoding and canonically sorted unique closure) and §8 fixed 5-array (`null` = absence, rules ID as raw 32-byte string). Reader-side canonical re-encode equality applies to any decode path.

**RED gate:**

```bash
cargo test -p mtgml-persistence --all-features semantic_contract_digest
.venv/bin/python -m unittest python.tests.test_v5_contract_digest -v
```

Expected RED: missing Rust functions/domains AND missing Python mirror module. GREEN = spec §19.1/§19.2 KAT vectors (SyntheticLegacy manifest → bytes → digest; ComprehensiveRules minimal one-entry manifest → bytes → digest; synthetic rules ID + null + null; KAT-only hypothetical Magic rules ID + null + null — NOT a catalog entry) committed as shared fixtures and asserted byte-identically by both suites.

**Negative/adversarial evidence:** schema/domain disagreement rejected; malformed child digest length rejected.

**Focused GREEN:** `cargo test -p mtgml-persistence --all-features`. **Affected-package GREEN:** `cargo check --workspace --all-targets --all-features --locked`.

**Commit:** `persistence: rules/semantic contract digest machinery with rust/python parity KATs`

---

## Task 3 — Semantic-contract source, generator, and generated catalog constants

**Objective:** The spec §10 single-source-of-truth chain, resolving the filename the spec left open.

**Exact files:**

- Source of truth (the ONLY hand-authored manifest definition): `contracts/catalog/semantic-contracts.v1.json` (schema_version `semantic-contracts-catalog.v1`; one entry: SyntheticLegacy rules authority, null closure, null format/content dimensions)
- Generator: `scripts/generate_semantic_contract_catalog.py` (modeled on `scripts/generate_contracts.py`'s CATALOG/`--check`/drift-via-`SystemExit` pattern)
- Generated Rust output: `crates/mtgml-environment/src/semantic_catalog_generated.rs` (generated-file banner, do-not-edit note; emits manifest constants AND derived `RulesContractIdV1`/`SemanticContractIdV1` constants using Task 2 functions)
- Rust recompute KAT: `crates/mtgml-environment/src/semantic_catalog_kat.rs` (spec §19.4: recompute every ID constant from the emitted manifest constants via the §9 functions; byte equality; drift fails the build)

**Generator semantics:** reads ONLY `contracts/catalog/semantic-contracts.v1.json` (no Rust parsing, no Python duplicate manifest, no invoking Rust); derives IDs via the Task 2 Python mirror functions (`python/src/mtgml/persistence.py::calculate_rules_contract_id_v1` / `calculate_semantic_contract_id_v1`) — NO digest logic in the generator itself; deterministic; `--check` mode fails on stale output; rerun produces zero diff.

**RED gate:**

```bash
.venv/bin/python scripts/generate_semantic_contract_catalog.py --check   # RED: script/schema missing → SystemExit/non-zero
cargo test -p mtgml-environment --all-features semantic_catalog_kat      # RED: constants/KAT missing
```

GREEN = generator emits; `--check` passes; rerun `git diff --exit-code` on generated file; Rust KAT GREEN.

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

`ProgramKernelConstructionErrorV1` with variant `UnsupportedProgram`. Remove `#[derive(Default)]` from `SyntheticM1RulesKernel` (`synthetic.rs:54`); keep the unit struct module-private to `mtgml-rules`.

**Migration census (compile-time-explicit; from the verified spec):** literals at `crates/mtgml-rules/src/synthetic.rs:113`, `:140`, `:165` → internal `ProgramKernelV1` construction; `crates/mtgml-environment/src/synthetic.rs` field/`fork_boxed` reset (`:165`) → holds `ProgramKernelV1`; forced-progress call sites `crates/mtgml-environment/src/synthetic/commit.rs:97`, `:219` and `crates/mtgml-environment/src/tests/forced_progress.rs:199` → dispatch via `ProgramKernelV1`; test literals `crates/mtgml-environment/src/tests/checkpoint_replay.rs:741`, `tests/forced_progress.rs:198` → `for_program`.

**RED gate:**

```bash
cargo test -p mtgml-rules --all-features program_kernel
```

Expected RED: missing types. GREEN: `for_program(SyntheticRulesCompat)` → synthetic kernel (behavior unchanged); `for_program(MagicRules)` → `Err(ProgramKernelConstructionErrorV1::UnsupportedProgram)`; both entry points dispatch; pre-S1 enum has no Magic variant.

**Negative/adversarial evidence (residual gate):**

```bash
grep -rn "SyntheticM1RulesKernel" crates/ tools/ | grep -v "crates/mtgml-rules/src" | grep -v "ProgramKernelV1" | grep -v "for_program"
```

must return zero semantic-construction sites (mentions in type position via the boundary are allowed; naked construction is not). The V5 gate script (Task 12, created RED before the migration) asserts this permanently.

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

**RED gate:**

```bash
cargo test -p mtgml-persistence --all-features checkpoint_digest_v5
```

Expected RED: missing function/domain. GREEN = spec §19.3 KATs: one fixed V4-equivalent input; mutation vectors — different `program_kind` ⇒ different digest; different `semantic_contract_id` ⇒ different digest; identical input ⇒ identical digest. Codec semantic_version `"5"` enforced; `"4"` rejected.

**Negative/adversarial evidence:** wrong codec pair rejected; full-state reference domain mismatch rejected.

**Focused GREEN:** `cargo test -p mtgml-persistence --all-features`. **Affected-package GREEN:** workspace `cargo check`.

**Commit:** `persistence: checkpoint digest v5 with execution identity`

---

## Task 7 — EnvironmentCheckpointV5

**Objective:** Spec §11 beside retained V4 in `crates/mtgml-environment/src/checkpoint.rs`: consts `ENVIRONMENT_CHECKPOINT_SCHEMA_V5 = "environment-checkpoint.v5"`, `CHECKPOINT_CODEC_ID_V5 = "in-memory-reference"`, `CHECKPOINT_CODEC_SEMANTIC_VERSION_V5 = "5"`; struct with unchanged `EngineState`/`FullStateDigestV4`, plus `execution_identity: ExecutionIdentityV1` and `checkpoint_digest: CheckpointDigestV5`; `new()`/`validate()` mirroring V4 with digest recompute FROM the stored identity.

**RED gate:**

```bash
cargo test -p mtgml-environment --all-features checkpoint_v5
```

Expected RED: missing type. GREEN: validate chain green; V4 tests untouched and still green.

**Negative/adversarial evidence:** tampered `execution_identity` ⇒ digest mismatch rejection; tampered digest ⇒ rejection; codec `"4"` rejected; completed-with-pending-decision rule preserved.

**Focused GREEN:** `cargo test -p mtgml-environment --all-features`. **Affected-package GREEN:** workspace `cargo check`.

**Commit:** `environment: checkpoint v5 with execution identity binding`

---

## Task 8 — RuntimeSemanticCatalog + restore admission + error taxonomy

**Objective:** Spec §10/§12/§18 in `crates/mtgml-environment` (new module `semantic_catalog.rs`): catalog consuming ONLY generated Task 3 constants (no digest generation, no mutable/lazy state, no filesystem/network/env lookup; `resolve(id)` vs `supported(id, program)` distinction); the spec §12 nine-phase admission order owned exactly as its table states; error variants added to `CheckpointValidationError`/`ControllerError` (`ExecutionIdentity`, `SemanticContractUnknown`, `SemanticContractDigestMismatch`, `RulesContractDigestMismatch`, `ProgramAuthorityMismatch`, `SemanticContractUnsupported`, `ProgramStateIncompatible`) with the deterministic `ProgramKernelConstructionErrorV1 → SemanticContractUnsupported/ControllerError` mapping.

**RED gate:**

```bash
cargo test -p mtgml-environment --all-features semantic_catalog
cargo test -p mtgml-environment --all-features restore_admission
```

Expected RED: missing module/variants. GREEN covers: SyntheticLegacy resolves; unknown semantic ID rejects; known-meaning ≠ supported-execution; `MagicRules` resolves to NO synthetic contract (catalog inputs match generated constants); each of the nine phases rejects in its own typed failure family; atomic rejection — pre/post `checkpoint()` byte-equality on the controller for every rejection phase (spec §12 observable invariant).

**Negative/adversarial evidence:** program × authority mismatch; runtime-unsupported; recompute-mismatch of top-level/rules IDs at admission (invariant-breach classification).

**Focused GREEN:** `cargo test -p mtgml-environment --all-features`. **Affected-package GREEN:** `cargo test --workspace --all-features --locked`.

**Commit:** `environment: runtime semantic catalog, fail-closed admission, error taxonomy`

---

## Task 9 — Replay V5 (mtgml-replay)

**Objective:** Spec §13 in new `crates/mtgml-replay/src/v5.rs` (consts `replay-manifest.v5` / `authoritative-replay.v5` / `replay-step.v5`): `ReplaySchemaVersionsV5`, `InitialEnvironmentIdentityV5` (V4's 6 fields + V5 digest + `execution_identity`), `SemanticContractMaterialV5`, `ReplayManifestV5` (V4 fields + `execution_identity` + `semantic_contract`; `KernelIdentityV1` retained as provenance; `rules_snapshot` retained as provenance), `ReplayStepV5` (mechanical cut: identical 11-field shape, `CheckpointDigestV5` before/after, `replay-step.v5`; NO forced-progress steps, NO fabricated decisions), `AuthoritativeReplayV5` (+ `final_identity.execution_identity`), `ReplayRecorderV5`. Detached `validate()` = V4 chain-walk + spec §13 additions: semantic manifest hashes to material ID; rules manifest hashes to recomputed rules ID; three-way identity equality; child-ID nulls; `comprehensive_rules` rules_snapshot equality.

**RED gate:**

```bash
cargo test -p mtgml-replay --all-features v5
```

Expected RED: missing module/types. GREEN: record→validate→export round-trip; spec §24 three-way binding tests (each mismatch ⇒ detached rejection BEFORE any backend execution); spec §25 families (SyntheticLegacy ⇒ `rules_snapshot` informational; ComprehensiveRules ⇒ mismatch ⇒ detached rejection — KAT/validation-only, NO runtime Magic contract).

**Negative/adversarial evidence:** every detached rejection classified artifact-validation, mutating nothing.

**Focused GREEN:** `cargo test -p mtgml-replay --all-features`. **Affected-package GREEN:** workspace `cargo check`.

**Commit:** `replay: authoritative replay v5 with three-way identity binding`

---

## Task 10 — Wire/schema V5 + fixtures

**Objective:** Spec §14/§7b/§14.1 exactly: `schemas/replay-manifest.v5.schema.json`, `schemas/authoritative-replay.v5.schema.json` (consts, `program_kind` enum, `^[0-9a-f]{64}$` digests, required identity objects in manifest/initial/final, `additionalProperties: false`); `WireContract` impls + dispatch in `crates/mtgml-wire/src/replay.rs` beside retained V4; golden + negative fixtures under `wire/golden/`, `wire/negative/` (`*-v5-*` naming).

**Required negatives (each classified, from spec §19):** unknown `program_kind`; wrong digest length; semantic-contract mismatch; rules-contract mismatch; `rules_snapshot` mismatch (CR); identity three-way mismatch; unknown field; wrong schema version.

**RED gate (Plan Fix-01: `validate_schemas.py` alone cannot RED on absent V5 — it validates its existing V1–V4 inventory; the inventory test is the seam):**

```bash
.venv/bin/python -m unittest python.tests.test_schema_parity.SchemaParityTests.test_v5_replay_schemas_are_inventoried -v
```

Expected RED: `V5 schema inventory absent` — the test requires `replay-manifest.v5.schema.json` / `authoritative-replay.v5.schema.json` in `WIRE_MAPPING` and `schemas/README.json`. Implement the inventory/mapping entries, schemas, dispatch, and fixtures; then:

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

**RED gate:**

```bash
.venv/bin/python -m unittest python.tests.test_v5_persistence python.tests.test_v5_replay -v
```

Expected RED: missing checkpoint-digest mirror and V5 DTO modules (the Task 2 contract-ID mirrors already exist and stay green). GREEN: shared KAT vectors byte-identical Rust↔Python for spec §19.1–§19.5 (commands: `cargo test -p mtgml-persistence --all-features semantic_contract_digest` then `cargo test -p mtgml-persistence --all-features checkpoint_digest_v5`; Python `python -m unittest python.tests.test_v5_persistence -v`; vectors read from the same committed fixture files).

**Negative/adversarial evidence:** Python rejects every §19 negative-fixture case mechanically.

**Focused GREEN:** the unittest command. **Affected-package GREEN:** `.venv/bin/python scripts/run_python_tests.py --profile full`.

**Commit:** `python: v5 mechanical mirror with rust/python byte parity KATs`

---

## Task 12 — V5 gate script BEFORE migration (RED by design)

**Objective:** Create `scripts/run_v5_execution_identity_gate.py` FIRST (Plan Fix-01: this is TDD for the migration itself — the gate must observe its RED state while current producers are still V4, which is impossible if it is built after Task 13): assert the V5-current tokens, the §23a.1 residual-kernel-construction grep, and RESIDUAL_V4_CURRENT_PRODUCER_ZERO via the §22 allowlist — V4 tokens permitted ONLY in RETAIN rows; failure output names path, token, line, expected disposition.

**RED gate (expected to FAIL at this point — that IS the evidence):**

```bash
.venv/bin/python scripts/run_v5_execution_identity_gate.py
```

Expected RED: current producers are still V4 → named `CURRENT_*` census violations (path/token/line/disposition). Nothing is fixed in this task.

**Negative/adversarial evidence:** scratch-test a violation → gate names it; revert scratch.

**Commit:** `gate: v5 execution identity gate with residual-v4 enforcement (red until migration)`

---

## Task 13 — Current producer/consumer migration + conformance/parity closure (driven by the gate)

**Objective:** Flip every §22 `CURRENT_*` census row to V5: environment producer paths (`synthetic.rs`, `synthetic/commit.rs`, `synthetic/replay.rs:56` manifest construction, `controller.rs`, `replay.rs`, `replay_parity_tests.rs`, `tests.rs`, `lib.rs` re-exports), replay current recorder/export, conformance consumers (`facade.rs`, `lib.rs`, `lifecycle.rs`, `isolation/{paired,replay_parity,checkpoint_parity,fork_parity,rejection,fingerprint,endpoint_pair}.rs`, `legal_space/gate_evidence.rs`), Python public surfaces, and `tools/m2-semantic-adapter` runtime construction path (`session.rs::reset_synthetic` → V5 config/codec/replay-schema; its historical M2 validation evidence stays V4 historical, never reinterpreted — spec §30). `run_m2_final_closure.py` gets ONLY its posture comment. Historical V4 rows (§22 RETAIN) untouched.

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

**Files:** `scripts/verify_repository.py` (V4-current block → V5-current tokens + residual checks); `scripts/run_checks.py` (append `scripts/run_v5_execution_identity_gate.py` to `FAST`, so it runs in PR Fast, Windows Setup Smoke, PR Integration (integration = FAST + extras), Integration/master, Nightly (certification = FAST + integration extras + certification extras)); `justfile` `contracts` recipe (direct invocation beside `verify_repository.py`; include `generate_semantic_contract_catalog.py --check` there and inside the gate script); `scripts/run_m2_b_contract_cut.py` (post-`git mv` posture line only); `scripts/run_m2_final_closure.py` (posture comment only); docs per spec §22: `docs/contracts/ENGINE_STATE_CLOSURE.md`, `docs/STATE_HASHING.md`, `docs/REPLAY_AND_DETERMINISM.md`, `docs/contracts/WIRE_CONTRACT.md`, `docs/maintenance/API_LIFECYCLE.md` (V4 sections retained as DOC_HISTORY + V5 sections added — docs FOLLOW the executable implementation, never ahead of it).

**RED gate:**

```bash
.venv/bin/python scripts/run_checks.py fast    # RED until verify_repository V5 tokens + FAST wiring land
```

**Negative/adversarial evidence:** `generate_semantic_contract_catalog.py --check` fails on a scratch-stale generated file; revert.

**Focused GREEN:** `run_checks.py fast` + `.venv/bin/python scripts/check_documentation.py`.

**Commits:** `gate: wire v5 gate into fast checks and repository verification` then `docs: v5 execution identity contract documentation closure`

---

## Final verification matrix (exact commands, after Task 13)

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
| SYNTHETIC_LEGACY_PARITY | T5 lock + T12 re-run; golden player bytes unchanged |
| CHECKPOINT_RESTORE_PARITY | T12 record→checkpoint→restore→resume suites |
| FORK_PARITY | T12 fork proof (identity preserved, §20) |
| REPLAY_V5_PARITY | T9/T12 round-trip suites |
| REJECTED_RESTORE_NONMUTATION | T8 pre/post-checkpoint-equality tests |
| UNKNOWN_CONTRACT_FAIL_CLOSED | T8 catalog/admission tests |
| PROGRAM_AUTHORITY_MISMATCH_FAIL_CLOSED | T8 admission tests |
| RULES_SNAPSHOT_MISMATCH_FAIL_CLOSED | T9 detached rejection tests |
| WIRE_POSITIVE_FIXTURES | T10 golden fixtures |
| WIRE_NEGATIVE_FIXTURES | T10 classified negatives |
| SCHEMA_VALIDATION | T10 `validate_schemas.py` (hosted jsonschema gate) |
| RESIDUAL_V4_CURRENT_PRODUCER_ZERO | T12 gate (created RED) → GREEN at T13; §22 allowlist |
| HISTORICAL_V4_EVIDENCE_PRESERVED | T7/T10/T12: V4 fixtures/KATs/schemas untouched, still passing |
| MAINTAINER_GATES | T12/T14: gate script, verify_repository tokens, FAST wiring, split b-cut posture |
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
