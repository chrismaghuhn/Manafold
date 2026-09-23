# M3.S2 `rules/zone-incarnation@0.1.0` Implementation Plan

**Status:** candidate plan; implementation blocked by the replay-input finding
in the Spec; requires independent review, explicit resolution, then S2
selection and authorization
**Implements:** nothing in this planning change
**Capability target:** `specified → implemented → covered`; not certified
**Design:** [M3.S2 Zone Incarnation Specification](../specs/2026-09-23-m3-s2-zone-incarnation-design.md)

## Preconditions and invariant set

Do not begin this plan until the exact current master is reviewed and a
separate record explicitly selects and authorizes
`rules/zone-incarnation@0.1.0`. Planning or approval of these documents alone
does not authorize production changes.

Before production implementation, separately resolve Spec §3/Q9. Current
`ReplayStepV5` carries only a player `DecisionResponseV2`, but the isolated S2
conformance request is not a player decision. Do not begin Task 6 or represent
its replay gate as satisfied until an accepted, version-correct way to replay
that transition exists, or an explicit reviewed scope decision changes where
the replay obligation is proved. This prerequisite may require a separate
design/ADR/version change; this plan does not authorize one.

The implementation is one coherent S2 PR with multiple reviewable logical
commits, not multiple mini PRs. Preserve these invariants throughout:

* only battlefield→owner graveyard and ordered library-top→owner hand;
* producer reason is outside S2; direct validated requests are allowed;
* one fresh `GameObjectId`, exact `PhysicalCardId` continuity, OLD ceases,
  NEW is live at destination;
* exact old/new snapshots, membership/order, event/delta/cursor parity;
* per-perspective identity/knowledge/observation follow existing contracts;
* no trusted identity leakage, no RNG, atomic rejection, deterministic V5
  checkpoint/restore/fork/replay;
* exclusions from Foundation V2 remain fail-closed; no lifecycle status beyond
  covered and no support claims.

Current substrate characterization from source base
`cf60c012113550d4e8449c72fea938167002fddc`: production zone executor absent;
fixture-only movement exists; `ZoneTransition` and snapshots already encode
the product; delta has a complete replacement but no allocator-specific
operation; semantic cursor validates incarnation shape but not object
allocator progression. The Spec resolves owner placement and lists exact
proposed conformance cases. These gaps are to be addressed in this one slice.

## Task 1 — RED conformance and exact substrate characterization

Add tests in the conformance/rules harness using explicit validated states and
direct primitive requests. Do not create fake SBA/death or Draw causality.
Start with RED cases for both selected families, transition snapshots, exact
membership/order, current allocator/cursor gaps, player products, delta and
deterministic identities. Record the current failing assertions before
production behavior changes.

Acceptance: tests use existing `ZoneTransition`, lifecycle, projection,
delta, and engine validation paths; they prove no second reference executor
is imported by production; no production or registry lifecycle is changed in
this task.

## Task 2 — Authoritative transition primitive and allocation

Implement the single rules-owned selected transition executor in
`mtgml-rules`' authoritative transition pipeline. Add only a checked
`GameObjectId` allocation method to the existing state allocator if required.
Perform full prevalidation, create NEW, transform exact object/location/order
membership and snapshots, and emit the existing `ZoneTransition` event.
Integrate with the ordinary accepted product path. Keep fixture helpers as
harnesses, never runtime authorities.

Acceptance: battlefield→owner-graveyard positive case passes; one fresh ID,
physical identity exact; OLD absent/NEW exact; unchanged unrelated allocators;
exact `ObjectSnapshot`, event, delta reapplication and digest. Failed
prevalidation does not expose partial workspace products.

## Task 3 — Perspective lifecycle and projection integration

Plan each perspective's lifecycle from authorized visibility and current
distinguishability. Use existing `PerspectiveLifecycleAuditV1`, pairing
validation, observation projector and information projector. Cover public
tracked remap plus location/history update; owner-only first-known allocation
and private acquisition; pre-known identity remap; no lifecycle for
non-owner hidden information. Keep event sequencing canonical and per-player.

Acceptance: exact `PerspectiveIdentityState`, retained/current/historical
knowledge and provenance; visible sequence; `PlayerObservation`,
`PlayerInformationState`, observed events; no trusted ID exposure. No
reveal/randomization extension or schema change without a returned design
review.

## Task 4 — Ordered library-to-hand family

Add the explicit validated request for the exact top of a nonempty ordered
owner library. Consume vector index zero only, shift all remaining `Top`
offsets exactly, insert NEW in owner hand, and run the same incarnation and
information path. Do not inspect Draw step or empty-library loss semantics.

Acceptance: exact top source, vector/offset update, exact snapshots and
physical continuity, owner private knowledge, opponent noninterference, no
RNG. A non-top request rejects atomically.

## Task 5 — Negative matrix, atomicity, event/delta/cursor closure

Complete the Spec's rejected-request cases. Extend the semantic cursor and
transition contract to prove exact object-ID allocation progression, source
and destination families, transition/snapshot causality, and exact changed
state closure. Validate event/delta alignment and final zone membership/order.
Add complete before/after semantic fingerprints for each rejection, including
environment and visible products.

Acceptance: every negative case rejects with a closed error and preserves
EngineState, all IDs/allocators, zones/order, lifecycle/knowledge, RNG,
events/delta, replay/checkpoint identity, counters/status, and player bytes.
No unsupported family accepted; lifecycle evidence remains `specified` until
review of the production implementation.

## Task 6 — Checkpoint, restore, fork, replay, rerun, noninterference

Use current V5 execution contracts. Prove allocator and zone order survive
checkpoint restore; equal forks match; same checkpoint/request gives exact
same state/event/delta/products/digest; selected transition consumes no RNG.
Add paired worlds with different opponent hidden library identities and
compare complete non-owner-safe bytes. Actual replay of the direct transition
is blocked by the response-only V5 input contract; complete this portion only
after the separate design resolution above. Do not substitute empty replay,
an after-transition checkpoint, a fabricated response, or authoritative
events as replay input.

Acceptance: exact authoritative and per-perspective parity across each path,
including genuine replay of the selected transition under the separately
accepted replay solution; no replay-supplied NEW ID; no root seed, cursor or
global-allocation side channel. Use existing endpoint isolation and projection
rules. Until resolved and executed, this task and `covered` status remain
BLOCKED/NOT_RUN as appropriate.

## Task 7 — Lifecycle evidence and status/documentation closure

Only after production review, update the capability lifecycle through its
authoritative registry/generator process to `implemented` when actual reviewed
Rust behavior meets that definition, then to `covered` only when all
applicable cases and interaction-independent gates pass. Synchronize only
mechanically generated artifacts. Update current status, M3 tracker evidence
and relevant documentation so that S2 remains not certified and no
card/deck/format claim is introduced. Record future interactions as
unsatisfied: `state-based-actions-combat × zone-incarnation` and
`draw-card × zone-incarnation`.

Acceptance: exact status counts and generated reports agree; no registry or
status claim precedes its evidence. No selection/authorization is created by
this task or PR.

## Task 8 — Exact-head verification and PR closure

At the final integration boundary, inspect the complete staged/unstaged diff,
run the repository-required fast and integration checks, then the broad
conformance, information-safety, replay and reproducibility gates appropriate
to the cross-layer state change. Run exact-head verification after all source
changes. Expensive full checks need not repeat after mechanical edits; rerun
affected checks and the final profile once on the final candidate head. Do
not weaken exact-head verification or claim unavailable hosted CI.

Preserve reviewable commit boundaries for RED evidence, core semantics,
information-safety integration, negative/parity closure, and lifecycle/status
closure. Deliver one coherent S2 implementation PR. Do not merge it or
authorize a later slice as part of this plan.

## Verification and lifecycle gates

Use the pinned toolchain and applicable repository commands. At minimum the
implementation PR must execute relevant Rust format/check/clippy/tests,
`just check-fast`, `just check`, and `just check-all` because this changes
authoritative semantics, information boundaries, and replay behavior. Run
focused conformance continuously; generated contract checks only if any
authoritative generated source is changed. Final source/archive checks run
last after source-changing operations. Report every gate as `PASS`, `FAIL`,
`BLOCKED`, or `NOT_RUN` with its actual result. Python tests passing do not
stand in for Rust workspace evidence. No benchmark is required.

Lifecycle is advanced only with reviewed evidence:

| Status | Required evidence |
| --- | --- |
| `specified` | Reviewed design matches Foundation V2 exact capability identity and exclusions. |
| `implemented` | Actual production Rust executor and checked allocation exist; exact accepted state/event/delta behavior and fail-closed paths reviewed. A fixture-only implementation or successful compile is insufficient. |
| `covered` | Both positive families, full negative atomicity, event/cursor/delta closure, perspective knowledge/projection, noninterference, deterministic rerun, V5 checkpoint/restore/fork/replay all execute and pass at final exact head. |
| `certified` | Out of scope and explicitly not targeted. |

S2 coverage does not satisfy producer interactions. Future acceptance must
separately prove `state-based-actions-combat × zone-incarnation` and
`draw-card × zone-incarnation` when those producers are selected.
