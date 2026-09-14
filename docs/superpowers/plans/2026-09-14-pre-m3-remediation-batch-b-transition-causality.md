# Pre-M3 Remediation Batch B: Transition, Causality, and Atomicity Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Characterize and minimally close confirmed current-runtime transition, causality, progression, and atomicity defects in FND-007, FND-008, FND-010, FND-011, and FND-012.

**Architecture:** Keep `mtgml-state` as the authoritative state/lifecycle owner and `mtgml-rules::validate_transition_contract` as the ordered transition-proof owner. Extend existing invariant ownership only where current contracts require it; do not add a second rules engine, reinterpret historical replay, or invent M3 event families.

**Tech Stack:** Rust workspace, `mtgml-state`, `mtgml-rules`, current V3 transition/delta/RNG contracts, Cargo locked tests/check/clippy.

---

## Scope

- FND-007: characterize the public lifecycle seam; confirm only if `Ok` can leave a complete `EngineState` invalid under the current state-closure contract.
- FND-008: characterize core/stack/format/continuation/allocator mutations separately; fix only contract-prohibited unexplained current-runtime mutations.
- FND-010A-F: classify revision, global allocator, trusted decision, perspective decision, continuation, and cross-perspective allocator claims separately. Historical Replay V1/V2 remains out of scope.
- FND-011: test an occurrence before its physical transition and require sequential causal rejection if the current contract demands it.
- FND-012: test RNG-backed and announced outcomes separately; do not force announced outcomes through RNG semantics.

## File map

- Modify `crates/mtgml-state/src/tests/lifecycle.rs` for FND-007 RED/GREEN and complete-state nonmutation.
- Modify `crates/mtgml-rules/src/tests/transition_contract.rs` for transition, progression, causality, and RNG-provenance RED/GREEN tests.
- Modify `crates/mtgml-rules/src/contract.rs`, `semantic_cursor.rs`, or `events.rs` only for confirmed owner-level fixes.
- Modify `crates/mtgml-state/src/lifecycle.rs` only if the public lifecycle seam is confirmed as a complete authoritative mutation boundary.
- Modify `docs/superpowers/specs/2026-09-14-pre-m3-remediation-batch-b-transition-causality-design.md` with final evidence-backed dispositions.
- Modify `docs/normative-document-register.v1.json` when the plan/spec artifacts are added.

No wire schema, digest domain, RNG algorithm, historical replay format, or M3 file is in scope.

### Task 1: Add Phase-1 lifecycle and transition RED probes

- [ ] **Step 1: Reproduce FND-007 at the public state lifecycle seam.**

Construct a valid synthetic state, submit an `Acquire` lifecycle mutation with
an otherwise-valid observed provenance and a `ZoneLocation.player` absent from
the declared players, then assert the current call returns `Ok` and the state
fails `validate_engine_state`. Capture the state before the call and assert the
call's current mutation behavior explicitly.

- [ ] **Step 2: Reproduce FND-008 with an unexplained core mutation.**

Build a valid transition product from a valid empty state whose only semantic
after-state change is `revision + 1` plus `core.turn_number + 1`, with an empty
event list and an exact `StateDelta`. Run the real
`validate_transition_contract`; record whether the current contract accepts it.
Repeat only for state families that remain independent after the first result.

- [ ] **Step 3: Reproduce FND-010A and FND-010B.**

Use a valid empty state and create accepted products with:

```text
after.revision = before.revision + 2
after.allocators.next_object_id = a valid lower cursor
```

Keep before/after states valid and delta-exact. Record the exact
`TransitionViolation` or unexpected `Ok` at the transition owner.

- [ ] **Step 4: Reproduce FND-010C-F.**

Create current V3 pending-decision products that reuse a trusted
`DecisionId`, reuse a perspective-local `PlayerDecisionIdV1`, replace a
continuation ID across a staged step, or issue one perspective's visible ID
from another perspective's cursor. Keep historical Replay V1/V2 tests and
types untouched.

- [ ] **Step 5: Reproduce FND-011 with reversed event order.**

Construct a valid before/after pair and event product containing a
`PerspectiveOccurrence` with a `Retire`/`Invalidate` lifecycle followed by the
physical `ZoneTransition` it claims to explain. Use the smallest valid
identity/knowledge setup. The current validator must be tested against the
event order, not a hand-written helper.

- [ ] **Step 6: Reproduce FND-012A and FND-012B separately.**

For FND-012A, create a visible `SawRandomOutcome` occurrence with a valid
bound/value, no `RandomValueSampled` event, and unchanged RNG cursor. For
FND-012B, create an `AnnouncedOutcome` occurrence with no RNG event and a
non-empty code. Record whether each is accepted and whether current authority
defines a required binding.

- [ ] **Step 7: Run all Phase-1 probes and freeze dispositions before production edits.**

Run the smallest named tests individually and then the affected package tests.
Write the actual result and disposition into the design document. Do not edit
production code until every FND-007/008/010A-F/011/012 subclaim has a final
classification.

### Task 2: Implement confirmed fixes only

- [ ] **Step 1: Make the lifecycle seam transactional if FND-007 is confirmed.**

Apply lifecycle mutation to a cloned complete `EngineState`, run the existing
central `validate_engine_state` on the candidate, and assign the candidate only
after validation succeeds. Map the invariant failure to a closed lifecycle
application error without exposing trusted diagnostics. Preserve existing
local atomicity and identity/knowledge semantics.

- [ ] **Step 2: Centralize exact revision and identity progression checks.**

If confirmed, add the progression checks at `validate_transition_contract` or
one directly owned helper. Require exactly `before.revision + 1` for accepted
current-runtime transitions, reject allocator rewind, and reject trusted or
perspective-local identity reuse. Do not modify Replay V1/V2.

- [ ] **Step 3: Close unexplained mutation families only if contract-supported.**

For confirmed FND-008 subfamilies, use a fail-closed unsupported-mutation
violation or extend the existing cursor with the exact state projection and
event family required by current authority. Do not invent M3 phase/priority/
stack/trigger semantics. Preserve delta/event/final-state mutual consistency.

- [ ] **Step 4: Replace product-wide causal lookup with sequential binding.**

If FND-011 is confirmed, bind each occurrence only to the event/cursor state
available at that point in the ordered event list. Preserve the accepted
before/after identity snapshot rule for old/new opaque references. Future
events must not satisfy an earlier occurrence.

- [ ] **Step 5: Require trusted RNG provenance for confirmed FND-012A.**

Bind visible random outcome occurrences to the already validated
`RandomValueSampled` event/cursor result without exposing root seed, stream key,
cursor, or raw words. Keep announced/public outcomes separate unless their
current contract explicitly requires another authoritative event.

### Task 3: Strengthen atomicity and negative ordering evidence

- [ ] **Step 1: Add complete before/after fingerprints for every new rejection.**

Assert state, revision, pending request, continuations, RNG cursors, global and
perspective allocators, knowledge, identities, events, and recorder/environment
state remain unchanged at the actual owning API boundary.

- [ ] **Step 2: Add event-order reversal negatives.**

Keep the correctly ordered product test, reverse the occurrence/transition or
random/event order, and assert the exact closed transition violation.

- [ ] **Step 3: Run focused package tests.**

```text
cargo test -p mtgml-state --all-features --locked
cargo test -p mtgml-rules --all-features --locked
cargo test -p mtgml-environment --all-features --locked
cargo test -p mtgml-conformance --all-features --locked
```

Add replay/random packages only if a confirmed fix touches them.

### Task 4: Run final verification and prepare one PR

- [ ] **Step 1: Inspect the final scope and dispositions.**

Verify FND-002 and the unresolved FND-006 position semantics are still
explicitly excluded, historical replay is unchanged, and M3 remains `NO`.

- [ ] **Step 2: Run final native gates.**

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
```

- [ ] **Step 3: Run applicable maintainer/integration gates.**

Run direct constituent commands if the local `just` wrapper is blocked by the
host shell. Mark wrapper failures `BLOCKED`, never `PASS`; report Hosted CI
separately.

- [ ] **Step 4: Push and open one PR.**

Use title `Pre-M3 remediation Batch B: transition causality and atomicity`.
Include the full FND-007/FND-008/FND-010A-F/FND-011/FND-012 matrix and explicit
public API, wire, schema, digest, RNG algorithm, historical replay, and M3
change statuses. Do not merge.
