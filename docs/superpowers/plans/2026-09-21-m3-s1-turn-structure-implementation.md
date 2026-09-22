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

## Durable naming rule

Milestone labels remain valid in planning, governance, task names, and
historical evidence. They are not durable production semantic identities.

```text
M3.S1 / S1
  allowed in planning and historical evidence

turn_structure
MagicRulesKernel
TurnStructureSupportProfile
TurnStructureError
ReferenceEnvironmentBackend
  durable production names
```

Do not introduce `MagicS1RulesKernel`, `magic_s1.rs`, `MagicS1EnvironmentBackend`,
`S1SupportPredicate`, or `S1Error`. The content-derived
`SemanticContractIdV1` is the persistent semantic identity; `S1` is not.

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

## Task 1 — First exact-contract RED characterization

**BASE/PARENT:** Task 0 exact reviewed parent
**Purpose:** characterize only the first missing seam: the exact S1 semantic
contract is not yet known to the runtime catalog. Do not add a broad future
S1 suite.

**Files allowed:**

- `crates/mtgml-environment/src/tests/semantic_catalog.rs`;
- no production Rust/Python code, catalog source, registry, schema, or
  generated file.

**RED command:**

```powershell
cargo test -p mtgml-environment --all-features --locked semantic_catalog -- exact_turn_structure_contract
```

**Expected RED reason:** the test computes the independently authored
comprehensive-rules manifest for exactly
`rules/turn-structure@0.1.0` and expects `RuntimeSemanticCatalog::resolve()`
to recognize its content-derived ID. The current production catalog has only
the synthetic entry, so this is a runtime semantic RED, not a compile break.

**Implementation scope:** one exact catalog-resolution RED only. Do not add
untap, Cleanup, observation, backend, or downstream capability tests yet.

**Focused GREEN command:** `NOT_RUN` by design; this commit is the RED
characterization commit.

**Broader GREEN command:**

```powershell
git diff --check
```

**Negative evidence:** the test must not construct a kernel, choose a default,
or calculate any Magic rule behavior.

**Commit boundary:** one RED characterization commit.

**HARD STOP:** if the RED is a compiler failure rather than catalog absence,
rewrite the test against the current public catalog/model APIs.

## Task 2 — Durable typed event/delta vocabulary

**BASE/PARENT:** Task 1 RED commit
**Purpose:** add only the reviewed typed audit vocabulary; no temporal
transition or Magic runtime activation.

**Files allowed:**

- `crates/mtgml-rules/src/events.rs`;
- `crates/mtgml-rules/src/semantic_cursor.rs`;
- `crates/mtgml-rules/src/contract.rs`;
- `crates/mtgml-rules/src/errors.rs` and `validation.rs`;
- `crates/mtgml-state/src/delta.rs`;
- `crates/mtgml-rules/src/lib.rs` if exports are required;
- focused rules tests named `turn_structure` or `magic_turn_structure`.

**RED command:** `NOT_APPLICABLE`.

**Expected RED reason:** this is a structural vocabulary-only task. It adds no
runtime semantic behavior, so a semantic RED is neither required nor valid.
Compile/format checks and the existing synthetic suite are the applicable
guards.

**Implementation scope:** add closed typed representations for
`TurnPositionChanged`, `UntapCompleted`, `ActivePlayerChanged`, and
`TurnNumberChanged`, with matching `SemanticDeltaOperation` variants and
sequential cursor arms. Use durable names such as `TurnStructureError`; do not
use `S1Error`, `MagicS1*`, or free-form labels. Do not add a public wire event
variant; existing `ObservedEventKindV2::ObjectTapped` is sufficient.

**Focused GREEN command:**

```powershell
cargo fmt --all -- --check
cargo test -p mtgml-rules --all-features --locked
```

**Broader GREEN command:**

```powershell
cargo test -p mtgml-state --all-features --locked
cargo test -p mtgml-rules --all-features --locked
```

**Negative evidence:** existing synthetic event/delta tests remain green;
rejected transitions retain empty audit; malformed affected sets are rejected.

**Commit boundary:** one durable typed-vocabulary commit.

**HARD STOP:** if the vocabulary requires a public schema or milestone-named
production type, stop and return to spec review.

## Task 3 — Known S1 contract, still non-executable

**BASE/PARENT:** Task 2 typed-vocabulary commit
**Purpose:** make the exact content-derived contract known to V5 admission
without enabling Magic execution yet.

**Files allowed:**

- `contracts/catalog/semantic-contracts.v1.json`;
- `scripts/generate_semantic_contract_catalog.py`;
- generated `crates/mtgml-environment/src/semantic_catalog_generated.rs`;
- `crates/mtgml-environment/src/semantic_catalog.rs`;
- `crates/mtgml-environment/src/semantic_catalog_kat.rs`;
- `crates/mtgml-environment/src/lib.rs` exports;
- `crates/mtgml-environment/src/tests/semantic_catalog.rs`;
- `python/tests/test_semantic_contract_catalog_generator.py`;
- no `ProgramKernelV1::MagicRules` activation.

**RED command:**

```powershell
<project-python> scripts/generate_semantic_contract_catalog.py --check
cargo test -p mtgml-environment --all-features --locked semantic_catalog -- exact_turn_structure_contract
```

**Expected RED reason:** the source catalog and generated catalog do not yet
contain the exact comprehensive-rules entry.

**Implementation scope:** add one hand-authored comprehensive-rules entry with
exactly `[rules/turn-structure@0.1.0]`, the verified CR snapshot, and null
format/content dimensions. Extend generator policy to accept exactly the
synthetic entry plus this S1 entry. Regenerate; never hand-edit generated Rust.

At the end of this task:

```text
catalog.resolve(exact_s1_id) = Some
catalog.supported(exact_s1_id, MagicRules) = false
ProgramKernelV1::for_program(MagicRules) = unsupported
```

This is the intentional known-but-not-executable state. Add KATs for exact
IDs, wrong closure, wrong snapshot, unknown ID, and synthetic/Magic mismatch.

**Focused GREEN command:**

```powershell
<project-python> scripts/generate_semantic_contract_catalog.py --check
cargo test -p mtgml-environment --all-features --locked semantic_catalog
```

**Broader GREEN command:**

```powershell
<project-python> scripts/run_v5_execution_identity_gate.py
cargo test -p mtgml-model --all-features --locked
cargo test -p mtgml-persistence --all-features --locked
cargo test -p mtgml-replay --all-features --locked
```

**Negative evidence:** catalog knowledge does not imply runtime support;
MagicRules remains fail-closed and no arbitrary comprehensive contract is
accepted.

**Commit boundary:** one generated-catalog commit with runtime support still
disabled for Magic.

**HARD STOP:** if this requires checkpoint/replay schema changes, report the
exact V5 insufficiency; do not create V6.

## Task 4 — `TurnStructureSupportProfile` and temporal skeleton

**BASE/PARENT:** Task 3 known-contract commit
**Purpose:** implement the reusable S1 rules/profile authority without
implementing untap, Cleanup mutation, or Magic kernel dispatch.

**Files allowed:**

- new `crates/mtgml-rules/src/turn_structure.rs`;
- `crates/mtgml-rules/src/lib.rs`;
- `crates/mtgml-rules/src/errors.rs`;
- `crates/mtgml-rules/src/program_kernel.rs` only for state-validation wiring;
- rules tests named `turn_structure`.

**RED command:**

```powershell
cargo test -p mtgml-rules --all-features --locked turn_structure -- support_profile
```

**Expected RED reason:** current MagicRules state admission has no
`validate_turn_structure_support()` implementation and fails closed through
the pre-S1 unsupported path.

**Implementation scope:** add durable `TurnStructureSupportProfile`,
`validate_turn_structure_support()`, `TurnStructureError`, the exact first
condition `players.len() == 2`, unique-other-player derivation, closed temporal
successor relation, and downstream `UnsupportedRulesBoundary` classification.
Do not inspect perspective/opaque mappings here. Do not mutate tapped state or
advance position here. Do not enable `ProgramKernelV1::MagicRules` yet.

**Focused GREEN command:**

```powershell
cargo test -p mtgml-rules --all-features --locked turn_structure -- support_profile
```

**Broader GREEN command:**

```powershell
cargo test -p mtgml-state --all-features --locked
cargo test -p mtgml-rules --all-features --locked
```

**Negative evidence:** one/three-player, held-priority, zero-turn,
unsupported-profile, invalid-temporal, and active-combat states reject without
any perspective mapping lookup.

**Commit boundary:** one support-profile/temporal-relation commit.

**HARD STOP:** if Rules Admission imports lifecycle projection or opaque-ID
state, remove that dependency before proceeding.

## Task 5 — Durable `MagicRulesKernel` shell, still not enabled

**BASE/PARENT:** Task 4 support-profile commit
**Purpose:** establish the milestone-free Magic kernel owner without enabling a
general MagicRules program through V5.

**Files allowed:**

- new `crates/mtgml-rules/src/magic.rs` containing `MagicRulesKernel`;
- `crates/mtgml-rules/src/transition.rs`;
- `crates/mtgml-rules/src/product.rs`;
- `crates/mtgml-rules/src/errors.rs`;
- private rules tests named `magic_turn_structure`.

**RED command:** `NOT_APPLICABLE`.

**Expected RED reason:** this is an unreachable internal kernel-shell task;
there is no player/runtime semantic behavior to characterize before the shell
exists. Compile/format checks are the applicable guards.

**Implementation scope:** add `MagicRulesKernel` with response execution
fail-closed and a private forced-progress shell that returns an explicit
`UnsupportedRulesBoundary` until the semantic operation is added. Do not add a
`ProgramKernelInner::MagicRules` production branch or make
`ProgramKernelV1::for_program(MagicRules)` succeed in this task. The shell is
tested only inside the rules crate and is not an executable environment path.

**Focused GREEN command:**

```powershell
cargo fmt --all -- --check
cargo test -p mtgml-rules --all-features --locked magic_turn_structure -- kernel_shell
```

**Broader GREEN command:**

```powershell
cargo test -p mtgml-rules --all-features --locked
```

**Negative evidence:** no public Magic runtime constructor is enabled; no
response, pass, default, or synthetic fallback is accepted.

**Commit boundary:** one durable kernel-shell commit.

**HARD STOP:** if the shell becomes reachable from an arbitrary semantic
contract or is named `MagicS1RulesKernel`, stop and rename/reorder it.

## Task 6 — Ordinary untap affected-set and narrow mutation

**BASE/PARENT:** Task 5 `MagicRulesKernel` shell commit
**Purpose:** implement only ordinary untap in the durable kernel owner.

**Files allowed:**

- `crates/mtgml-rules/src/turn_structure.rs`;
- `crates/mtgml-rules/src/magic.rs`;
- `crates/mtgml-rules/src/events.rs`;
- `crates/mtgml-rules/src/semantic_cursor.rs`;
- `crates/mtgml-rules/src/contract.rs`;
- `crates/mtgml-state/src/delta.rs` only for the matching audit variant;
- rules tests named `turn_structure` or `magic_turn_structure`.

**RED command:**

```powershell
cargo test -p mtgml-rules --all-features --locked magic_turn_structure -- untap
```

**Expected RED reason:** the kernel shell has no ordinary untap product,
affected-set derivation, or `UntapCompleted` event.

**Implementation scope:** derive all tapped Battlefield objects controlled by
the active player from the before-state; sort by `GameObjectId`; mutate only
the complete `tapped=true -> false` set; emit `UntapCompleted` and
`TurnPositionChanged`; and validate the narrow mutation boundary. Do not
advance through priority or implement any downstream capability.

**Focused GREEN command:**

```powershell
cargo test -p mtgml-rules --all-features --locked magic_turn_structure -- untap
```

**Broader GREEN command:**

```powershell
cargo test -p mtgml-rules --all-features --locked
cargo test -p mtgml-state --all-features --locked
```

**Negative evidence:** nonactive and already-untapped objects remain unchanged;
affected-set omissions, duplicate order, and unrelated object/zone/identity
changes fail atomically. No perspective mapping is checked by the rules path.

**Commit boundary:** one ordinary-untap semantic commit.

**HARD STOP:** if untap needs `can_untap`, a zone transition, reincarnation,
or a choice, stop.

## Task 7 — Quiescent Cleanup boundary and next-turn switch

**BASE/PARENT:** Task 6 ordinary-untap commit
**Purpose:** add only the S1-owned Cleanup-to-next-Untap temporal switch.

**Files allowed:**

- `crates/mtgml-rules/src/turn_structure.rs`;
- `crates/mtgml-rules/src/magic.rs`;
- `crates/mtgml-rules/src/events.rs`;
- `crates/mtgml-rules/src/semantic_cursor.rs`;
- `crates/mtgml-rules/src/contract.rs`;
- rules tests named `turn_structure` or `magic_turn_structure`.

**RED command:**

```powershell
cargo test -p mtgml-rules --all-features --locked magic_turn_structure -- cleanup
```

**Expected RED reason:** the kernel has no Cleanup temporal product, checked
player switch, or checked turn increment.

**Implementation scope:** validate quiescent Cleanup; derive the unique other
player; use `checked_add`; emit `TurnNumberChanged`,
`ActivePlayerChanged`, and `TurnPositionChanged` in the specified order; and
leave the final position at the next player's Untap boundary. Do not clear
damage, discard, expire durations, or generate cleanup triggers.

The quiescent detection is the precise spec §11.1 predicate, not the reverted
hand-zone-presence approximation. It must live in the Cleanup boundary path of
the temporal product (the `Ending(Cleanup)` arm of the forced-progress switch),
never in the admission support predicate, because a hand of seven or fewer
cards is a valid quiescent cleanup. Concretely: fail closed (no mutation, no
turn switch) when the active player's hand size exceeds the ordinary maximum
hand size (CR `402.2`, normally seven; `514.1`), or when any live object
carries authoritative `marked_damage` greater than zero (`514.2`/`120.6`).
Duration and cleanup-trigger work is already excluded by the supported-state
predicate (no effects/triggers/delayed-effects). No `maximum_hand_size`
EngineState field is added; it is a derived rule constant. See
`TASK_7_AUTHORITY_REMEDIATION_01`.

**Focused GREEN command:**

```powershell
cargo test -p mtgml-rules --all-features --locked magic_turn_structure -- cleanup
```

**Broader GREEN command:**

```powershell
cargo test -p mtgml-rules --all-features --locked
```

**Negative evidence:** `u64::MAX` and required cleanup-reset work reject with
no player switch or wraparound.

**Commit boundary:** one cleanup-boundary commit.

**HARD STOP:** if crossing Cleanup requires cleanup-reset or discard semantics,
stop with `SCOPE_DEPENDENCY_DISCOVERED`.

## Task 8 — Atomic exact-contract admission and Magic runtime enablement

**BASE/PARENT:** Task 7 Cleanup commit
**Purpose:** make the existing known S1 contract executable only through exact
V5 admission and the durable `MagicRulesKernel`.

**Files allowed:**

- `crates/mtgml-environment/src/semantic_catalog.rs`;
- `crates/mtgml-environment/src/semantic_catalog_kat.rs`;
- `crates/mtgml-environment/src/tests/semantic_catalog.rs`;
- `crates/mtgml-environment/src/tests/restore_admission.rs`;
- `crates/mtgml-rules/src/program_kernel.rs`;
- `crates/mtgml-rules/src/magic.rs`;
- `crates/mtgml-environment/src/lib.rs` exports;
- no checkpoint/replay version files.

**RED command:**

```powershell
cargo test -p mtgml-environment --all-features --locked semantic_catalog -- exact_turn_structure_support
cargo test -p mtgml-environment --all-features --locked restore_admission -- exact_turn_structure_support
```

**Expected RED reason:** Task 3 makes the exact ID resolvable, but
`catalog.supported(exact_s1_id, MagicRules)` remains false and the program
kernel still has no Magic inner dispatch.

**Implementation scope:** in one atomic enablement task, change the runtime
support predicate to recognize only the exact comprehensive turn-structure ID
and add a contract-aware constructor such as
`ProgramKernelV1::for_admitted_execution(...)`. `for_program(MagicRules)` alone
must remain fail-closed or be unavailable for production execution; it is not
semantic authorization.

The admitted construction carries a validated, compile-time/runtime-resolved
`MagicExecutionProfile` into the durable kernel:

```rust
MagicRulesKernel {
    profile: MagicExecutionProfile,
}
```

The profile is an internal executable interpretation of the already-admitted
`ExecutionIdentityV1`/`SemanticContractIdV1`; it is not a second contract, a
public registry, or a mutable lookup. Its capability predicates are exact for
the admitted contract. An old checkpoint admitted under Contract A therefore
cannot execute Contract B behavior merely because the same `MagicRulesKernel`
type later grows more capabilities.

The environment's existing V5 admission path must prove exact catalog
identity, authority pair, runtime support, profile construction, and
`validate_turn_structure_support()` before constructing or committing the Magic
kernel/backend. Synthetic remains unchanged. An arbitrary comprehensive
contract remains unsupported.

Add exact positive/negative KATs and restore nonmutation cases.

**Focused GREEN command:**

```powershell
cargo test -p mtgml-environment --all-features --locked semantic_catalog -- exact_turn_structure_support
cargo test -p mtgml-environment --all-features --locked restore_admission -- exact_turn_structure_support
```

**Broader GREEN command:**

```powershell
<project-python> scripts/generate_semantic_contract_catalog.py --check
<project-python> scripts/run_v5_execution_identity_gate.py
cargo test -p mtgml-model --all-features --locked
cargo test -p mtgml-persistence --all-features --locked
cargo test -p mtgml-replay --all-features --locked
```

**Negative evidence:** no generic MagicRules construction path, no synthetic
fallback, no mutable catalog, no registry runtime lookup, no V6 types, and no
wrong-contract execution.

**Commit boundary:** one atomic catalog-support/program-dispatch commit.

**HARD STOP:** if exact contract admission and Magic dispatch cannot be enabled
atomically, keep both disabled and report the dependency.

## Task 9 — Observation, public tapped consequences, and product closure

**BASE/PARENT:** Task 8 atomic exact-contract/runtime-enable commit
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
cargo test -p mtgml-environment --all-features --locked turn_structure -- observation
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
cargo test -p mtgml-environment --all-features --locked turn_structure -- observation
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

## Task 10 — Reference environment backend and parity integration

**BASE/PARENT:** Task 9 observation commit
**Purpose:** make the exact S1 contract executable through the existing trusted
environment/controller path and prove V5 checkpoint/fork/replay parity.

**Files allowed:**

- new `crates/mtgml-environment/src/reference.rs` for the durable
  `ReferenceEnvironmentBackend`, explicit complete-state setup, and shared
  reference-backend transaction/projection/checkpoint/replay mechanics
  extracted from the current synthetic-only owner;
- `crates/mtgml-environment/src/lib.rs`, `controller.rs`, `replay.rs` only for
  wiring the existing traits;
- `crates/mtgml-environment/src/synthetic.rs` and its `commit.rs`, `replay.rs`,
  `projection.rs` only for mechanical delegation to `reference.rs`; synthetic
  semantics must remain unchanged;
- `crates/mtgml-conformance/src/facade.rs` and a new
  `crates/mtgml-conformance/src/turn_structure.rs` for real-kernel cases;
- `crates/mtgml-environment/src/tests/turn_structure.rs` and parity tests;
- no `mtgml-replay` V5 schema change.

**RED command:**

```powershell
cargo test -p mtgml-environment --all-features --locked turn_structure
cargo test -p mtgml-conformance --all-features --locked turn_structure
```

**Expected RED reason:** no current `ReferenceEnvironmentBackend` constructs a
MagicRules execution identity and no current environment path runs the exact
admitted turn-structure contract through V5 checkpoints/projections.

**Implementation scope:** add the durable reference backend/config that accepts
complete trusted setup, exact content-derived turn-structure identity, V5 checkpoint/replay
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
cargo test -p mtgml-environment --all-features --locked turn_structure
cargo test -p mtgml-conformance --all-features --locked turn_structure
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

## Task 11 — Full negative, nonmutation, and fail-closed matrix

**BASE/PARENT:** Task 10 backend/parity commit
**Purpose:** close every negative/support-boundary obligation from the
specification before lifecycle promotion.

**Files allowed:**

- `crates/mtgml-rules/src/tests/magic_turn_structure.rs`;
- `crates/mtgml-environment/src/tests/turn_structure.rs`;
- `crates/mtgml-environment/src/tests/restore_admission.rs`;
- `crates/mtgml-conformance/src/turn_structure.rs`;
- `crates/mtgml-conformance/src/isolation/*` only for reuse of existing
  fingerprint/parity helpers;
- no production semantic expansion.

**RED command:**

```powershell
cargo test -p mtgml-rules --all-features --locked magic_turn_structure
cargo test -p mtgml-environment --all-features --locked turn_structure
cargo test -p mtgml-conformance --all-features --locked turn_structure
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
cargo test -p mtgml-rules --all-features --locked magic_turn_structure
cargo test -p mtgml-environment --all-features --locked turn_structure
cargo test -p mtgml-conformance --all-features --locked turn_structure
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

## Task 12 — Capability lifecycle and evidence registry update

**BASE/PARENT:** Task 11 negative-matrix commit
**Purpose:** advance only the turn-structure registry metadata justified by
evidence.

**Files allowed:**

- `cards/capabilities/registry.json`;
- `cards/capabilities/README.md` only if its current lifecycle explanation
  requires an S1-specific note;
- `python/tests/test_maintainer_artifacts.py` only for a focused lifecycle
  guard if the existing negative guard is insufficient;
- registry validation tests/scripts;
- no other capability entry.

**RED command:** `NOT_APPLICABLE`.

**Expected RED reason:** lifecycle metadata promotion is not runtime semantic
behavior. The existing negative guard
`test_implemented_capability_requires_existing_implementation` must be `PASS`
as a prerequisite; it is not a new RED condition and does not authorize
promotion by itself.

**Negative lifecycle guard command:**

```powershell
<project-python> -m unittest python.tests.test_maintainer_artifacts.MaintainerArtifactTests.test_implemented_capability_requires_existing_implementation
```

**Implementation scope:**

1. set `spec_path` to this turn-structure specification while preserving Foundation V2 and
   ADR authority references;
2. add exact Rust implementation paths only after Task 4–10 evidence and set
   lifecycle to `implemented`;
3. add exact turn-structure conformance case IDs only after Task 11 evidence and set
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

## Task 13 — Documentation, status, and generated-contract closure

**BASE/PARENT:** Task 12 evidence/registry commit
**Purpose:** reconcile status without rewriting historical V4 records or
claiming more than S1 evidence proves.

**Files allowed:**

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
<project-python> scripts/run_python_tests.py --profile smoke
```

**Expected RED reason:** after the intended narrow lifecycle/status change,
the existing current-status assertions still describe the pre-implementation
state. Document registration is already complete in the planning fix and is
not a future RED condition.

**Implementation scope:** preserve the V4 historical wording and name V5 as
current; update
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

## Task 14 — Exact-head final verification

**BASE/PARENT:** Task 13 documentation/status commit
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

Planning-fix review disposition:

```text
PLANNING_FIX_BASE = 214064b0d5f038bc03aeb672198e119638cfc984
PLAN_BLOCKER = 0
PLAN_MAJOR = 0
PLAN_MINOR = 0
IMPLEMENTATION_PLAN_READY = YES
```

The implementation branch must be created from the then-current `master`, not
from this planning branch's historical ancestor. The current task must stop
here. No Task 1–14 implementation task may be started in this planning turn.
