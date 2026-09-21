# M3.S1 `rules/turn-structure@0.1.0` Implementation Plan

**Derived from:** [M3.S1 turn-structure specification](../specs/2026-09-21-m3-s1-turn-structure-design.md)
**Repository baseline:** `3bb404e750d93b82afd15e1a7e2c0cc6c0e4e635` (`origin/master`, verified)
**Current local source ancestor inspected:** `03e14f3`
**Status:** provisional implementation plan
**Task type:** implementation planning only; no implementation is authorized in this task

This plan is subordinate to the specification. It does not introduce a
competing design. Every task below must be checked against the specification
before editing. The plan ends at independent review; it does not begin S1
implementation in the current task.

## Scope firewall

```text
S1_ONLY = YES
PRIORITY_IMPLEMENTATION = NO
DRAW_IMPLEMENTATION = NO
CLEANUP_RESET_IMPLEMENTATION = NO
COMBAT_IMPLEMENTATION = NO
ZONE_TRANSITION_IMPLEMENTATION = NO
CARD_IMPLEMENTATION = NO
V5_REDESIGN = NO
CHECKPOINT_V6 = NO
REPLAY_V6 = NO
```

The current V5 checkpoint/replay/identity infrastructure is consumed exactly
as merged. If a task appears to require a downstream capability, public schema
churn, V5/V6 persistence, a second temporal authority, or a card/deck path,
stop with:

```text
SCOPE_DEPENDENCY_DISCOVERED
```

and record the exact correctness reason. Do not solve it silently.

## Shared execution rules

### One authority

The implementation must maintain exactly one path for each concern:

```text
one S1 supported-state predicate
one temporal successor relation
one rules-owned forced-progress primitive
one ordinary-untap derivation/mutation path
one MagicRules/S1 semantic-contract admission path
one authoritative event -> StateDelta audit mapping
one player observation projector
```

T0 and conformance provide explicit inputs and exact assertions. They do not
calculate turn legality, untap eligibility, affected sets, or expected runtime
products from a second rules model. Python remains a wire/DTO consumer.

### No silent defaults

The implementation and fixtures must not use a default target, default phase,
default pass, first candidate, random candidate, implicit untap choice,
implicit unsupported phase skip, or invalid-state repair. S1 has no
player-controlled decision surface.

### Commit discipline

Each task is performed from the exact reviewed parent of the preceding task.
Each RED command must fail for the intended semantic reason. A compile error
is not accepted as semantic RED evidence. Each GREEN command must run the
focused tests, then the task's broader profile. A commit boundary is created
only after focused GREEN, negative evidence, and `git diff --check` pass.

Hosted evidence is always reported separately from local evidence. A missing
or unexecuted tool is `NOT_RUN`, never `PASS`.

## Task 0 — Post-V5 baseline and authorization reconciliation

**BASE/PARENT:** `3bb404e750d93b82afd15e1a7e2c0cc6c0e4e635` exact remote master
**Purpose:** verify that no V5 work remains to be merged or reauthorized; make
no source change.

**Files allowed:** none. Read-only inspection of `README.md`, `docs/ROADMAP.md`,
`docs/adr/README.md`, ADR 0054, ADR 0055, Foundation V2, Issue #178, the
registry, the semantic catalog, and current Git status.

**RED command:**

```powershell
git ls-remote https://github.com/chrismaghuhn/Manafold.git refs/heads/master
git fetch origin master
git diff --stat 3bb404e750d93b82afd15e1a7e2c0cc6c0e4e635..HEAD
```

**Expected RED reason:** none is expected. If remote master differs from the
recorded baseline, or V5 is not closed/current, the plan parent must be
re-baselined before implementation.

**Implementation scope:** no implementation; verify `V5_EXECUTION_IDENTITY =
CLOSED`, current checkpoint/replay = V5, S1 authorization head
`587016574e4e8f9f797a713877f8caf1c5143cfb`, and lifecycle `specified`.

**Focused GREEN:**

```powershell
git rev-parse origin/master
git status --short --branch
```

**Broader GREEN:** inspect the Issue #178 current authoritative block and
confirm the three carried S1 minors are the only S1 review obligations.

**Negative evidence:** no V4-current claim is restored; no V5 implementation
slice is reopened; no capability lifecycle changes.

**Commit boundary:** none.

**HARD STOP:** if the exact remote head, V5 closure, S1 authorization, or
current lifecycle cannot be established, stop before Task 1.

## Task 1 — S1 RED conformance characterization

**BASE/PARENT:** Task 0 exact reviewed parent
**Purpose:** add behavior-first RED evidence before adding S1 production
semantics.

**Files allowed:**

- `crates/mtgml-rules/src/tests.rs` only for module inclusion;
- new `crates/mtgml-rules/src/tests/magic_s1.rs`;
- new or current `crates/mtgml-environment/src/tests/magic_s1.rs` only for
  tests that compile against existing APIs;
- `python/tests/test_semantic_contract_catalog_generator.py` only for the
  catalog-policy RED assertion;
- no production Rust/Python code, catalog source, registry, schema, or
  generated file.

**RED command:**

```powershell
cargo test -p mtgml-rules --all-features --locked magic_s1
cargo test -p mtgml-environment --all-features --locked magic_s1
<project-python> scripts/run_python_tests.py --profile smoke
```

**Expected RED reason:** current `ProgramKernelV1::for_program(MagicRules)`
returns `UnsupportedProgram`, and current `validate_runtime_state(MagicRules,
...)` fails closed. The RED assertions must fail on that missing MagicRules
behavior, not merely fail to compile. Catalog RED must show that the current
production source has no exact comprehensive S1 entry.

**Implementation scope:** author independent setup/expected-value fixtures for:

- valid two-player Untap state expected to progress;
- valid two-player Cleanup state expected to switch to the other player;
- a three-player admission negative;
- active/nonactive tapped-object cases;
- unsupported downstream boundary and overflow fingerprints;
- exact S1 closure admission.

The fixtures must not call a future S1 function to calculate their expected
products.

**Focused GREEN command:** none; this task is intentionally RED-only.

**Broader GREEN command:** `git diff --check` plus the same focused commands;
the expected result remains RED until implementation tasks land.

**Negative evidence:** no test may add an implicit response, a pass, a card,
or a second turn implementation.

**Commit boundary:** one RED characterization commit containing tests only.

**HARD STOP:** if RED is only a compiler failure, rewrite the test until the
failure exercises the existing runtime's missing semantic behavior.

## Task 2 — Typed S1 event/delta/cursor vocabulary

**BASE/PARENT:** Task 1 RED commit
**Purpose:** add only the reviewed typed audit vocabulary needed by the
specification; do not implement a transition.

**Files allowed:**

- `crates/mtgml-rules/src/events.rs`;
- `crates/mtgml-rules/src/semantic_cursor.rs`;
- `crates/mtgml-rules/src/contract.rs`;
- `crates/mtgml-rules/src/errors.rs` and `validation.rs`;
- `crates/mtgml-state/src/delta.rs`;
- `crates/mtgml-rules/src/lib.rs` if exports are required;
- focused tests from Task 1.

**RED command:**

```powershell
cargo test -p mtgml-rules --all-features --locked magic_s1
```

**Expected RED reason:** the RED suite still fails because no production path
emits or applies the S1 event families. Any compile-only failure from a new
type reference is scaffolding feedback, not accepted semantic RED; resolve it
within this task before recording the RED result.

**Implementation scope:** add closed, typed representations for:

```text
TurnPositionChanged { from, to }
UntapCompleted { affected_objects }
ActivePlayerChanged { from, to }
TurnNumberChanged { from, to }
```

and matching `SemanticDeltaOperation` variants. Add semantic-cursor fields and
cursor arms for position, active player, turn number, affected-set ordering and
final parity. Add the narrowly required observation policy for public
`ObjectTapped` projection only if the existing lifecycle path needs a typed
policy arm; do not add a public wire variant because
`ObservedEventKindV2::ObjectTapped` already exists.

Define canonical serialization/ordering and typed validation errors. Do not
add a generic event bus, free-form labels, or speculative future event types.

**Focused GREEN command:**

```powershell
cargo fmt --all -- --check
cargo test -p mtgml-rules --all-features --locked magic_s1
```

**Broader GREEN command:**

```powershell
cargo test -p mtgml-rules --all-features --locked
```

**Negative evidence:** existing synthetic event/delta tests remain green;
rejected transitions still have empty audit; a malformed affected set is
rejected rather than normalized.

**Commit boundary:** one typed-vocabulary commit; no semantic behavior claim.

**HARD STOP:** if the vocabulary duplicates an existing semantic authority or
requires a public schema change, stop and return to the specification review.

## Task 3 — Executable supported-state/profile admission

**BASE/PARENT:** Task 2 typed-vocabulary commit
**Purpose:** make the S1 predicate explicit, reusable, and fail-closed.

**Files allowed:**

- new `crates/mtgml-rules/src/turn_structure.rs` for the pure S1 predicate,
  successor relation, other-player derivation, and typed S1 profile errors;
- `crates/mtgml-rules/src/lib.rs`;
- `crates/mtgml-rules/src/program_kernel.rs` only for the program-aware
  validation call;
- `crates/mtgml-rules/src/errors.rs`;
- `crates/mtgml-state/src/validation/core.rs` only if an existing structural
  invariant must be extended; do not place Magic legality there;
- S1 tests from Task 1.

**RED command:**

```powershell
cargo test -p mtgml-rules --all-features --locked magic_s1 -- two_players
cargo test -p mtgml-rules --all-features --locked magic_s1 -- unsupported_profile
```

**Expected RED reason:** current MagicRules admission fails for the valid
two-player state, and no executable S1 predicate distinguishes exact two
players, held priority, unsupported execution profile, and invalid temporal
state.

**Implementation scope:** implement the first check exactly as
`players.len() == 2`, then validate the rest of Section 7. Reuse existing
`TurnPosition`, `PriorityState`, `ExecutionState`, `ZoneState`, and generic
state validation. Derive the unique other player from the two-player set.
Reject pending decisions, continuations, effect/trigger state, active combat,
format state, held priority, zero turn number, unsupported profile, and
unresolvable public mappings. Do not persist any eligibility cache.

Expose one internal predicate to both MagicRules execution and V5 program-aware
admission. T0 must call this production predicate, never mirror it.

**Focused GREEN command:**

```powershell
cargo test -p mtgml-rules --all-features --locked magic_s1 -- two_players
cargo test -p mtgml-rules --all-features --locked magic_s1 -- unsupported_profile
```

**Broader GREEN command:**

```powershell
cargo test -p mtgml-state --all-features --locked
cargo test -p mtgml-rules --all-features --locked
```

**Negative evidence:** valid one-player and three-player states reject; held
priority, nonempty unsupported execution state, `turn_number == 0`, and
invalid mappings reject with unchanged fingerprints.

**Commit boundary:** one admission/predicate commit.

**HARD STOP:** if exact-two-player admission cannot be tested with a complete
valid state, add a structurally valid fixture builder; do not weaken the
predicate or test only a map length assertion in isolation.

## Task 4 — Program-owned S1 temporal forced-progress primitive

**BASE/PARENT:** Task 3 admission commit
**Purpose:** make `MagicRules` constructible and give it one authoritative
forced-progress path, while keeping response execution unsupported.

**Files allowed:**

- new `crates/mtgml-rules/src/magic.rs` for `MagicS1RulesKernel` and its
  program-owned dispatch;
- `crates/mtgml-rules/src/program_kernel.rs`;
- `crates/mtgml-rules/src/transition.rs` if the forced-progress trait seam
  needs the existing internal interface;
- `crates/mtgml-rules/src/product.rs`;
- `crates/mtgml-rules/src/turn_structure.rs`;
- S1 forced-progress tests.

**RED command:**

```powershell
cargo test -p mtgml-rules --all-features --locked magic_s1 -- magic_kernel
```

**Expected RED reason:** `ProgramKernelV1::for_program(MagicRules)` currently
fails closed and no Magic kernel can produce a temporal product.

**Implementation scope:** add a program-owned MagicRules inner variant and a
named constructor path that can be reached only after exact semantic-contract
admission. Keep the synthetic inner variant and behavior unchanged. Magic
`apply(DecisionResponseV2)` must reject/fail closed; it must never fall through
to `SyntheticM1RulesKernel`. `advance_forced_progress` must call the single
S1 temporal primitive and return a complete `TransitionResult`.

For `Beginning(Untap)`, produce the supported boundary product that clears the
derived set and advances to Upkeep. For all unsupported downstream positions,
return the typed boundary failure without mutation. Do not implement cleanup,
priority, draw, or combat in this task.

**Focused GREEN command:**

```powershell
cargo test -p mtgml-rules --all-features --locked magic_s1 -- magic_kernel
```

**Broader GREEN command:**

```powershell
cargo test -p mtgml-rules --all-features --locked
cargo test -p mtgml-environment --all-features --locked forced_progress
```

**Negative evidence:** Magic cannot answer a response; synthetic still uses
its existing response path; unsupported progress leaves the input and all
identities unchanged.

**Commit boundary:** one program-dispatch/forced-progress commit.

**HARD STOP:** if the Magic kernel can be constructed with an arbitrary or
unknown semantic contract, stop and close the admission hole before moving
on. Do not accept program kind alone as sufficient identity.

## Task 5 — Ordinary untap affected-set and narrow mutation

**BASE/PARENT:** Task 4 Magic kernel commit
**Purpose:** implement only ordinary untap inside the S1 temporal primitive.

**Files allowed:**

- `crates/mtgml-rules/src/turn_structure.rs`;
- `crates/mtgml-rules/src/magic.rs`;
- `crates/mtgml-rules/src/events.rs`;
- `crates/mtgml-rules/src/semantic_cursor.rs`;
- `crates/mtgml-rules/src/contract.rs`;
- `crates/mtgml-state/src/delta.rs` only for the matching audit variant;
- S1 rule tests and independent expected-state fixtures.

**RED command:**

```powershell
cargo test -p mtgml-rules --all-features --locked magic_s1 -- untap
```

**Expected RED reason:** the temporal primitive exists but does not yet derive
the complete eligible set or produce the exact `UntapCompleted`/event/delta
product.

**Implementation scope:** derive all tapped battlefield objects controlled by
the active player from the before-state; sort by `GameObjectId`; validate the
simple no-modifier profile; mutate all `tapped` fields in one workspace
operation; emit `UntapCompleted`; emit only the reviewed position/observation
evidence; and validate the narrow mutation boundary before commit.

The implementation must compare before/after object snapshots and reject any
zone/location/identity/owner/controller/face-down mutation. Nonactive tapped
objects and already-untapped objects remain unchanged. No RNG, decision,
continuation, zone transition, or physical identity is touched.

**Focused GREEN command:**

```powershell
cargo test -p mtgml-rules --all-features --locked magic_s1 -- untap
```

**Broader GREEN command:**

```powershell
cargo test -p mtgml-rules --all-features --locked
cargo test -p mtgml-state --all-features --locked
```

**Negative evidence:** one-player/nonactive/control/profile/duplicate-order
mutants reject; affected-set omission and unrelated object-field mutation fail
the transition contract; empty affected set is exact and deterministic.

**Commit boundary:** one ordinary-untap semantic commit.

**HARD STOP:** if untap needs a persisted `can_untap`/`untap_eligible` field,
zone transition, object reincarnation, or choice, stop; the S1 specification
forbids that design.

## Task 6 — Quiescent Cleanup boundary and next-turn switch

**BASE/PARENT:** Task 5 ordinary-untap commit
**Purpose:** add only the S1-owned Cleanup-to-next-Untap temporal switch.

**Files allowed:**

- `crates/mtgml-rules/src/turn_structure.rs`;
- `crates/mtgml-rules/src/magic.rs`;
- `crates/mtgml-rules/src/events.rs`;
- `crates/mtgml-rules/src/semantic_cursor.rs`;
- `crates/mtgml-rules/src/contract.rs`;
- S1 rule tests.

**RED command:**

```powershell
cargo test -p mtgml-rules --all-features --locked magic_s1 -- cleanup
```

**Expected RED reason:** current S1 execution has no Cleanup temporal product,
checked player switch, or checked turn increment.

**Implementation scope:** validate that Cleanup is quiescent with respect to
all excluded cleanup-reset/discard/exception work; calculate the unique other
player; use `checked_add`; emit `TurnNumberChanged`,
`ActivePlayerChanged`, and `TurnPositionChanged` in the specified order; and
leave the final position at the next player's Untap boundary. Do not clear
damage, discard, expire durations, or generate cleanup triggers.

**Focused GREEN command:**

```powershell
cargo test -p mtgml-rules --all-features --locked magic_s1 -- cleanup
```

**Broader GREEN command:**

```powershell
cargo test -p mtgml-rules --all-features --locked
cargo test -p mtgml-environment --all-features --locked forced_progress
```

**Negative evidence:** `u64::MAX` rejects without wraparound; marked damage or
an unsupported cleanup profile rejects without switching players; active-player
and turn-number event order is exact.

**Commit boundary:** one cleanup-boundary commit.

**HARD STOP:** if crossing Cleanup requires implementing cleanup-reset or
discard semantics, stop with `SCOPE_DEPENDENCY_DISCOVERED`.

## Task 7 — Exact V5 S1 semantic-contract activation

**BASE/PARENT:** Task 6 temporal commit
**Purpose:** register and admit exactly the first real Magic contract without
reopening V5.

**Files allowed:**

- `contracts/catalog/semantic-contracts.v1.json`;
- `scripts/generate_semantic_contract_catalog.py`;
- generated `crates/mtgml-environment/src/semantic_catalog_generated.rs`;
- `crates/mtgml-environment/src/semantic_catalog.rs`;
- `crates/mtgml-environment/src/semantic_catalog_kat.rs`;
- `crates/mtgml-environment/src/lib.rs` exports;
- `crates/mtgml-environment/src/tests/semantic_catalog.rs`;
- `crates/mtgml-environment/src/tests/restore_admission.rs`;
- `python/tests/test_semantic_contract_catalog_generator.py`;
- `crates/mtgml-rules/src/program_kernel.rs` only for the exact admission handoff;
- no checkpoint/replay version files.

**RED command:**

```powershell
<project-python> scripts/generate_semantic_contract_catalog.py --check
cargo test -p mtgml-environment --all-features --locked semantic_catalog
cargo test -p mtgml-environment --all-features --locked restore_admission
```

**Expected RED reason:** the current production catalog accepts only the
synthetic legacy entry; its generator policy rejects a second comprehensive
S1 entry; MagicRules restore is currently unsupported.

**Implementation scope:** add one hand-authored comprehensive-rules catalog
entry with exactly `[rules/turn-structure@0.1.0]`, null format/content
dimensions, and the verified CR snapshot. Extend the generator policy to
accept exactly the reviewed synthetic entry plus this reviewed S1 entry; keep
the renderer and digest derivation single-sourced. Regenerate the Rust catalog;
never hand-edit generated output.

Change `RuntimeSemanticCatalog::supported` from program-only support to exact
program/semantic-contract support. Keep the frozen program/authority pairing
and V5 admission order. Bind MagicRules construction to the admitted S1
contract; an arbitrary comprehensive contract must remain unsupported.

Add positive/negative KATs for both IDs, wrong closure, wrong snapshot,
synthetic/Magic mismatch, unknown ID, and exact catalog recomputation.

**Focused GREEN command:**

```powershell
<project-python> scripts/generate_semantic_contract_catalog.py --check
cargo test -p mtgml-environment --all-features --locked semantic_catalog
cargo test -p mtgml-environment --all-features --locked restore_admission
```

**Broader GREEN command:**

```powershell
<project-python> scripts/run_v5_execution_identity_gate.py
cargo test -p mtgml-model --all-features --locked
cargo test -p mtgml-persistence --all-features --locked
cargo test -p mtgml-replay --all-features --locked
```

**Negative evidence:** no V6 types, no child manifests in checkpoints, no
mutable catalog, no registry lookup at runtime, and no broad
`MagicRules => every comprehensive contract` behavior.

**Commit boundary:** one catalog/admission activation commit, with generated
drift clean.

**HARD STOP:** if the generator or V5 runtime requires a checkpoint/replay
schema redesign, stop and report the exact structural insufficiency; do not
create V6.

## Task 8 — Observation, public tapped consequences, and product closure

**BASE/PARENT:** Task 7 exact-contract commit
**Purpose:** close the existing observation/event projection without public
schema churn.

**Files allowed:**

- `crates/mtgml-rules/src/events.rs`;
- `crates/mtgml-environment/src/lifecycle_projection.rs`;
- `crates/mtgml-environment/src/reference.rs` and
  `crates/mtgml-environment/src/synthetic/projection.rs` only for shared
  projection wiring;
- `crates/mtgml-observation/src/observed_event.rs` only if validation needs a
  no-schema-change correction;
- `python/src/mtgml/events.py` only if byte-parity evidence proves the existing
  `object_tapped` meaning is inconsistent;
- Rust observation/environment tests.

**RED command:**

```powershell
cargo test -p mtgml-environment --all-features --locked magic_s1 -- observation
cargo test -p mtgml-observation --all-features --locked
```

**Expected RED reason:** the current occurrence projector does not yet map the
S1 public untap consequence from the new rules-owned policy to the existing
opaque `ObservedEventKindV2::ObjectTapped` path.

**Implementation scope:** reuse `synthetic-m3-observation.v1`; populate active
player, turn number, closed position, and explicit priority from the committed
state. Project public tap consequences only through existing opaque IDs and
perspective-local visible sequence state. Validate projections before atomic
commit. Do not expose execution identity, semantic catalog, checkpoint/replay
digests, trusted IDs, physical card IDs, or hidden information.

**Focused GREEN command:**

```powershell
cargo test -p mtgml-environment --all-features --locked magic_s1 -- observation
cargo test -p mtgml-observation --all-features --locked
```

**Broader GREEN command:**

```powershell
cargo test -p mtgml-environment --all-features --locked information_projection
cargo test -p mtgml-conformance --all-features --locked
```

**Negative evidence:** paired unauthorized states have identical player-safe
bytes; an unresolved opaque mapping fails closed; no new wire schema or Python
rules logic is introduced.

**Commit boundary:** one observation/projection closure commit.

**HARD STOP:** if the existing observation/event surface cannot carry the
authorized public tap consequence without exposing trusted identity, return to
spec review; do not add a privileged field or bypass the projector.

## Task 9 — Magic S1 reference backend and parity integration

**BASE/PARENT:** Task 8 observation commit
**Purpose:** make the exact S1 contract executable through the existing trusted
environment/controller path and prove V5 checkpoint/fork/replay parity.

**Files allowed:**

- new `crates/mtgml-environment/src/magic.rs` for the bounded S1 backend and
  explicit complete-state setup;
- new `crates/mtgml-environment/src/reference.rs` for shared reference-backend
  transaction, projection, checkpoint, and replay mechanics extracted from the
  current synthetic-only owner;
- `crates/mtgml-environment/src/lib.rs`, `controller.rs`, `replay.rs` only for
  wiring the existing traits;
- `crates/mtgml-environment/src/synthetic.rs` and its `commit.rs`, `replay.rs`,
  `projection.rs` only for mechanical delegation to `reference.rs`; synthetic
  semantics must remain unchanged;
- `crates/mtgml-conformance/src/facade.rs` and a new
  `crates/mtgml-conformance/src/m3_s1.rs` for real-kernel S1 cases;
- `crates/mtgml-environment/src/tests/magic_s1.rs` and parity tests;
- no `mtgml-replay` V5 schema change.

**RED command:**

```powershell
cargo test -p mtgml-environment --all-features --locked magic_s1
cargo test -p mtgml-conformance --all-features --locked s1
```

**Expected RED reason:** no current `EnvironmentBackend` constructs a
MagicRules S1 execution identity and no current environment path runs S1
forced progress through V5 checkpoints/projections.

**Implementation scope:** add an explicit bounded Magic S1 backend/config that
accepts complete trusted setup, exact S1 identity, V5 checkpoint/replay
metadata, and two players. Reuse the existing atomic commit sequence:

```text
before checkpoint
-> program-owned kernel product
-> forced-progress product
-> transition/event/delta validation
-> candidate V5 checkpoint
-> pre-commit projections
-> atomic state/status/counter/replay commit
```

Do not copy synthetic rules. Share only transaction, checkpoint, projection,
and replay mechanics. S1 forced progress must not append a V5 response step or
increment submitted-decision counters. Use the current V5 manifest shape and
deterministic scenario provenance; do not claim a deck or card bundle.

**Focused GREEN command:**

```powershell
cargo test -p mtgml-environment --all-features --locked magic_s1
cargo test -p mtgml-conformance --all-features --locked s1
```

**Broader GREEN command:**

```powershell
cargo test -p mtgml-environment --all-features --locked checkpoint_replay
cargo test -p mtgml-environment --all-features --locked replay_parity
cargo test -p mtgml-conformance --all-features --locked
```

**Negative evidence:** restore of wrong S1 ID, wrong program/authority pair,
unknown contract, invalid player count, and invalid S1 state leaves the
backend checkpoint/replay identity unchanged.

**Commit boundary:** one environment/conformance integration commit.

**HARD STOP:** if shared backend extraction causes synthetic behavior drift, or
if replay parity requires a fabricated response, stop and revert the extraction
within the task; no second environment rules path is acceptable.

## Task 10 — Full negative, nonmutation, and fail-closed matrix

**BASE/PARENT:** Task 9 backend/parity commit
**Purpose:** close every negative/support-boundary obligation from the
specification before lifecycle promotion.

**Files allowed:**

- `crates/mtgml-rules/src/tests/magic_s1.rs`;
- `crates/mtgml-environment/src/tests/magic_s1.rs`;
- `crates/mtgml-environment/src/tests/restore_admission.rs`;
- `crates/mtgml-conformance/src/m3_s1.rs`;
- `crates/mtgml-conformance/src/isolation/*` only for reuse of existing
  fingerprint/parity helpers;
- no production semantic expansion.

**RED command:**

```powershell
cargo test -p mtgml-rules --all-features --locked magic_s1
cargo test -p mtgml-environment --all-features --locked magic_s1
cargo test -p mtgml-conformance --all-features --locked s1
```

**Expected RED reason:** any missing matrix case, incomplete rejection
fingerprint, incorrect stop boundary, or accidental state/counter/replay
mutation is exposed by an independently authored negative case.

**Implementation scope:** add/assert exact cases for one/three players,
unsupported profile, held priority, downstream priority/draw/combat/cleanup,
overflow, invalid temporal state, fabricated response, wrong semantic ID,
wrong closure, wrong authority, unsupported restore, and projection failure.
Use the existing complete state/checkpoint/replay/player-product fingerprints.

**Focused GREEN command:**

```powershell
cargo test -p mtgml-rules --all-features --locked magic_s1
cargo test -p mtgml-environment --all-features --locked magic_s1
cargo test -p mtgml-conformance --all-features --locked s1
```

**Broader GREEN command:**

```powershell
cargo test --workspace --all-features --locked
<project-python> scripts/run_python_tests.py --profile full
```

**Negative evidence:** every rejected/unsupported case proves no state, RNG,
IDs, knowledge, history, events, replay, counters, status, or player bytes
changed. Unsupported is never reported as a legal transition.

**Commit boundary:** one negative-matrix/evidence commit.

**HARD STOP:** if any rejection can mutate before classification, or if a
negative case passes only because the expected output was copied from the
implementation, stop for independent expected-value correction.

## Task 11 — Capability lifecycle and evidence registry update

**BASE/PARENT:** Task 10 negative-matrix commit
**Purpose:** advance only the S1 registry metadata justified by evidence.

**Files allowed:**

- `cards/capabilities/registry.json`;
- `cards/capabilities/README.md` only if its current lifecycle explanation
  requires an S1-specific note;
- registry validation tests/scripts;
- no other capability entry.

**RED command:**

```powershell
<project-python> scripts/validate_schemas.py
<project-python> scripts/validate_maintainer_artifacts.py
```

**Expected RED reason:** before this task, the S1 entry has no implementation
path or conformance case references. This task is not allowed to use a green
Rust test alone as a lifecycle promotion.

**Implementation scope:**

1. set `spec_path` to this S1 specification while preserving Foundation V2 and
   ADR authority references;
2. add exact Rust implementation paths only after Task 4–9 evidence and set
   lifecycle to `implemented`;
3. add exact S1 conformance case IDs only after Task 10 evidence and set
   lifecycle to `covered`;
4. leave all other 10 Foundation capabilities `specified` with empty evidence;
5. do not add benchmark/certification/card/deck claims.

**Focused GREEN command:**

```powershell
<project-python> scripts/validate_schemas.py
<project-python> scripts/validate_maintainer_artifacts.py
```

**Broader GREEN command:**

```powershell
<project-python> scripts/verify_repository.py
<project-python> scripts/run_python_tests.py --profile full
```

**Negative evidence:** no `certified` lifecycle, no downstream capability
promotion, no card/deck/format support claim, and no registry entry used as
runtime semantic authority.

**Commit boundary:** use two small commits if needed: `specified ->
implemented`, then `implemented -> covered`, each with its own evidence.

**HARD STOP:** if a required evidence class is `NOT_RUN`, `FAIL`, or
`BLOCKED`, leave lifecycle unchanged and report the missing evidence.

## Task 12 — Documentation, status, and generated-contract closure

**BASE/PARENT:** Task 11 evidence/registry commit
**Purpose:** reconcile status without rewriting historical V4 records or
claiming more than S1 evidence proves.

**Files allowed:**

- `docs/normative-document-register.v1.json`;
- `README.md`;
- `docs/ROADMAP.md`;
- `python/tests/test_current_status.py`;
- `docs/README.md` or capability documentation only when a link/status owner
  must be synchronized;
- generated semantic-catalog output only through its generator;
- no edits to historical ADR 0054/0055 wording except a separately reviewed
  factual status correction, and no edit to Foundation V2 history.

**RED command:**

```powershell
<project-python> scripts/check_documentation.py
<project-python> scripts/verify_repository.py
```

**Expected RED reason:** the new planning documents are not yet registered or
current-status tests do not yet describe the final narrow S1 evidence state.

**Implementation scope:** register the spec and plan as provisional process
artifacts; preserve the V4 historical wording and name V5 as current; update
status only to the exact lifecycle/evidence achieved; state that S1 remains
non-playable and adds no card/deck/format certification. Update generated
semantic catalog only from `contracts/catalog/semantic-contracts.v1.json`.

**Focused GREEN command:**

```powershell
<project-python> scripts/check_documentation.py
<project-python> scripts/verify_repository.py
<project-python> scripts/check_rust_source_structure.py
```

**Broader GREEN command:**

```powershell
<project-python> scripts/run_checks.py fast
```

**Negative evidence:** current status never says all Magic rules, playable
engine, cards, decks, Commander, or certified support. Historical V4 claims
are not silently changed to V5-current claims.

**Commit boundary:** one documentation/status closure commit after all actual
evidence is attached.

**HARD STOP:** if status prose would need to claim a broader capability than
the registry and conformance evidence prove, leave the status unchanged.

## Task 13 — Exact-head final verification

**BASE/PARENT:** Task 12 documentation/status commit
**Purpose:** verify the final source head and produce external verification
evidence without mutating the source being verified.

**Files allowed:** no source edits. Verification output belongs outside the
reproducible source archive, per ADR 0030.

**RED command:**

```powershell
git diff --check
git status --porcelain
```

**Expected RED reason:** any whitespace error, unexpected file, generated
drift, lifecycle mismatch, or untracked implementation artifact blocks the
final head.

**Implementation scope:** none; run and record the applicable local gates:

```powershell
<project-python> scripts/generate_semantic_contract_catalog.py --check
<project-python> scripts/generate_contracts.py --check
<project-python> scripts/run_v5_execution_identity_gate.py
<project-python> scripts/verify_repository.py
<project-python> scripts/check_rust_source_structure.py
<project-python> scripts/check_documentation.py
<project-python> scripts/validate_schemas.py
<project-python> scripts/validate_maintainer_artifacts.py
<project-python> scripts/validate_golden_path.py
<project-python> scripts/run_python_tests.py --profile full

cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked

<project-python> scripts/run_checks.py fast
<project-python> scripts/run_checks.py integration
<project-python> scripts/run_checks.py certification

git diff --check
git status --porcelain
```

`just check-fast`, `just check`, `just check-all`, and
`just release-candidate` may also be attempted because they are repository
workflow entry points. The current `justfile` invokes Bash and
`.venv/bin/python`; on Windows, unavailable shell/toolchain commands must be
reported `NOT_RUN` or `BLOCKED`, with the direct pinned commands above recorded
separately. The certification/archive gate remains last.

**Focused GREEN command:** rerun the smallest failed command after correction;
never weaken a gate or edit source after the final archive check.

**Broader GREEN command:** all applicable local commands above, followed by
separate hosted PR Fast/Integration/Certification evidence if a later
implementation review authorizes a PR. Hosted results are never inferred from
local output.

**Negative evidence:** no required gate is left unknown; no production source
is changed after final reproducibility verification; no untracked generated
report is mistaken for source evidence.

**Commit boundary:** no source commit after the final verification pass.

**HARD STOP:** any required `FAIL`, `BLOCKED`, or `NOT_RUN`, any generated
drift, any production file outside the approved task union, any lifecycle
claim without evidence, or any scope leak blocks handoff.

## Final implementation handoff condition

This plan is implementation-ready only because the specification review passed:

```text
SPEC_PATH = docs/superpowers/specs/2026-09-21-m3-s1-turn-structure-design.md
SPEC_REVIEW_BASE = 3bb404e750d93b82afd15e1a7e2c0cc6c0e4e635
SPEC_BLOCKER = 0
SPEC_MAJOR = 0
SPEC_MINOR = 0
SPEC_NIT = 0
```

The current task must stop here. No Task 1–13 implementation task may be
started in this planning turn.
