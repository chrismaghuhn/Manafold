# M3.S2 `rules/zone-incarnation@0.1.0` Implementation Plan

**Status:** candidate plan; requires independent review and explicit S2
selection/authorization
**Implements:** nothing in this planning change
**Capability target:** `specified → implemented → covered`; not certified
**Design:** [M3.S2 Zone Incarnation Specification](../specs/2026-09-23-m3-s2-zone-incarnation-design.md)

## Preconditions and invariant set

Do not begin implementation until the exact current master is reviewed and a
separate record explicitly selects and authorizes
`rules/zone-incarnation@0.1.0`. Planning or approval of these documents alone
does not authorize production changes.

The implementation is one coherent S2 PR with multiple reviewable logical
commits, not multiple mini PRs. Preserve these invariants throughout:

* only battlefield→owner graveyard and ordered library-top→owner hand;
* battlefield→graveyard inserts NEW at Graveyard top and preserves prior
  relative order with exact offset shifts;
* producer reason is outside S2; direct validated requests are allowed;
* one fresh `GameObjectId`, exact `PhysicalCardId`, owner and
  `CardDefinitionId` continuity; destination-canonical status fields;
* OLD is closed across every authoritative `GameObjectId` reference;
  destination `foundation_sources` is absent;
* exact snapshots, membership/order, event/delta/cursor parity;
* per-perspective identity/knowledge/observation follow existing contracts;
* no trusted identity leakage, no RNG, atomic rejection;
* checkpoint/restore/fork/rerun parity; authoritative replay remains required
  before `covered` but is deferred until a valid replayable path exists;
* Foundation V2 exclusions remain fail-closed; no card/deck/format claim.

Current substrate characterization from source base
`cf60c012113550d4e8449c72fea938167002fddc`: no production zone executor;
fixture-only movement does not handle ordered positions; `ZoneTransition`
and snapshots already encode the product; `GameObject.controller` is required
and validated as a player; `StateDelta` has complete replacement state but
no allocator-specific operation; the semantic cursor does not yet prove
object allocator progression or source-record closure. Current authoritative
object references are in zone object/location/order indexes, optional combat
attackers/blockers, `foundation_sources`, stack-record `source_object`, and
pending decision candidate bindings. Existing effects, triggers,
continuations and format state contain no direct `GameObjectId`; the executor
must keep an exhaustive typed reference scan current as these shapes evolve.

## Task 1 — RED conformance and exact substrate characterization

Add test/conformance cases over explicit validated state and direct primitive
requests. Do not create fake SBA/death or Draw causality. Start with RED cases
for both selected moves, Graveyard top insertion and offset shifts,
destination-canonical snapshot, OLD-reference closure, event/delta/cursor,
player products and deterministic identity. Add test-only mutant products for
validator negatives; do not expose internal snapshots/lifecycle mutations as
request fields. Include the historical finality witness: execute a real
accepted OLD→NEW move, then attempt a second admitted request with OLD and
assert rejection plus complete nonmutation from that resulting state. Also
mutate a subsequent lifecycle occurrence to reference OLD and require
contract rejection without fabricating a player response. Record current
failures before production behavior changes.

Acceptance: cases reuse existing transition, lifecycle, projection, delta and
engine validation paths; no second reference executor enters production; no
production behavior or registry lifecycle changes in this task.

## Task 2 — Authoritative incarnation and ordered-zone primitive

Implement the single rules-owned selected transition executor in
`mtgml-rules`' authoritative transition pipeline and a checked
`GameObjectId` allocation method on the existing allocator if required.
Prevalidate source/destination family, order and destination owner. Capture
exact `last_known`; allocate NEW once; preserve PhysicalCardId, owner and
CardDefinitionId; normalize mandatory destination storage to
`controller = owner`, `tapped = false`, `face_down = false`; build exact
`new_snapshot`. For Battlefield→Graveyard insert NEW at vector index zero
and increment every old Graveyard offset. For Library→Hand consume exact
library index zero and shift its remaining offsets. Integrate the existing
`ZoneTransition` and ordinary accepted-product path. Keep fixture helpers as
harnesses, never runtime authorities.

Acceptance: both zone products have exact locations, ordered vectors, object
rows, snapshots, one fresh ID, event, delta reapplication and digest. Explain
storage normalization separately from Magic controller/status semantics.
No multi-object order choice or generic movement API is introduced.

## Task 3 — OLD-reference closure and perspective/knowledge integration

Before mutation, scan all current authoritative `GameObjectId` reference
sites. Reject if OLD occurs in combat assignments, stack source records,
pending decision bindings or any unhandled site. Remove `foundation_sources`
for OLD on Battlefield→Graveyard and never transfer/create a destination
source. Remove OLD from live object/location/source-order indexes. Treat only
per-perspective identity mapping updates as a deliberate exception, through
existing lifecycle mutations. Future state fields containing a
`GameObjectId` must join the scan before they can be used with this executor.

Integrate existing `PerspectiveLifecycleAuditV1`, pairing validator and
projection paths: public tracked remap/location-history update for the
Graveyard move, owner-only private acquisition/remap as appropriate for
Library→Hand, and no non-owner hidden identity/event update. Keep canonical
event order.

Acceptance: exact OLD closure; OLD foundation data ceases and NEW has none;
exact opaque mappings, current/historical retained knowledge and provenance,
visible sequence, observation, information state and observed events. No
trusted ID leaks.

## Task 4 — Library-to-hand selected family

Complete the explicit validated request for the exact top of a nonempty
ordered owner library. Consume vector index zero once, rewrite every remaining
`Top` offset, set owner Hand membership/location, and run the same fresh
incarnation and information path. Do not implement Draw timing or
empty-library loss.

Acceptance: exact ordered source consumption, physical/card/owner continuity,
canonical destination storage, owner-private identity/knowledge,
opponent noninterference, no RNG. Non-top requests reject atomically.

## Task 5 — Negative matrix, event/delta/cursor and atomicity closure

Keep caller-controlled typed-request rejection tests distinct from
transition-contract/semantic-cursor/mutant negatives. Caller request tests
cover absent source, claimed-location mismatch, unadmitted family,
wrong-owner destination, non-top library source and invalid before state.
Mutant tests alter only private trusted products to cover reused/exhausted
allocator, malformed old/new snapshot or physical continuity, lifecycle and
event/delta pairing, order/offset corruption, stale OLD references, and
source-record cessation/creation errors.

Extend the semantic cursor and transition contract to prove exact allocation
progression, snapshot semantics, destination order, OLD reference closure,
FoundationSource removal, event/delta alignment and final state. Add complete
before/after fingerprints for every rejection, including state, products,
environment, checkpoint/replay state and player bytes.

Include both historical finality proofs from the Spec: after a committed
transition, a second real request using OLD rejects with the full resulting
state fingerprint unchanged; a test-mutated later lifecycle occurrence that
references OLD after `ZoneTransition(OLD, NEW)` fails cursor/contract
validation. Neither proof invents a `DecisionResponseV2`.

Acceptance: all request and mutant cases reject without commit or mutation;
no internal audit product becomes caller input; all supported transitions
reapply their delta exactly and preserve unrelated allocators/RNG.

## Task 6 — Checkpoint/restore, fork, rerun and noninterference

Execute these S2 gates directly: checkpoint/restore after each move preserves
allocator and order state; equal forks match; deterministic rerun from the
same checkpoint/request yields identical state, IDs, event, delta, products,
digest and zero RNG delta; paired worlds differing in opponent hidden library
identity produce equal non-owner-safe bytes.

Disposition by evidence type:

```text
checkpoint/restore       = executable in S2
fork                     = executable in S2
deterministic rerun      = executable in S2; NOT replay evidence
noninterference          = executable in S2
authoritative replay     = DEFERRED_REQUIRED / BLOCKED for covered
```

Do not fabricate a response, replay events as input, claim an after-state
checkpoint or empty replay proves transition replay, or add Replay V6 in this
slice. A later separate review may accept/version replay support, or a later
replayable producer/integration path may execute the exact transition. In
either case it must reproduce typed `ZoneTransition`, NEW identity, state,
delta and player products. Foundation V2 replay evidence remains required;
this deferral is not a waiver.

Acceptance for this S2 task: the four executable gates above pass. Record the
authoritative replay case as `DEFERRED_REQUIRED / BLOCKED`, never PASS.

## Task 7 — Lifecycle promotion to IMPLEMENTED only

After review of actual production Rust behavior, update the capability
lifecycle through its authoritative registry/generator process to
`implemented` only when the executor, allocation, ordered-zone mutation,
OLD-reference closure, exact state/event/delta behavior and fail-closed
paths exist. Update status evidence without promoting to `covered`. Preserve
the outstanding authoritative replay obligation and both unsatisfied future
interactions:
`state-based-actions-combat × zone-incarnation` and
`draw-card × zone-incarnation`.

Acceptance: counts and generated reports reflect S2 as implemented but not
covered or certified; no support claim is created. A later replay evidence
change is separately reviewed, executed, and required before covered
promotion.

## Task 8 — Exact-head verification and one PR closure

Inspect the complete staged/unstaged diff; run relevant Rust format/check/
clippy/tests, `just check-fast`, `just check`, and `just check-all` on the
appropriate integration/final boundaries. Include ordered-zone conformance,
information safety, rejection atomicity, replay/checkpoint and reproducibility
profiles. Run exact-head verification after all source changes. Do not weaken
exact-head verification or infer hosted CI.

Preserve reviewable logical commits for RED evidence, core zone semantics,
OLD closure/information integration, negative/parity closure, and lifecycle
closure. Deliver one coherent S2 implementation PR; do not merge it. The PR
may close at `implemented` while replay is deferred, but it may not claim
`covered`. A later `covered` promotion requires genuine authoritative replay
evidence at its own exact head.

## Verification and lifecycle gates

Use the pinned toolchain and applicable repository commands. Run focused
conformance throughout; generated contract checks only if an authoritative
generated source is changed. Final source/archive checks run last after all
source-changing operations. Report every gate as `PASS`, `FAIL`, `BLOCKED`, or
`NOT_RUN` with its actual result. Python tests passing do not stand in for
Rust workspace evidence. No benchmark is required.

| Status | Required evidence |
| --- | --- |
| `specified` | Reviewed design matches Foundation V2 exact identity, selected families and exclusions. |
| `implemented` | Explicit S2 selection/authorization; actual reviewed authoritative Rust executor, allocator, Graveyard ordering, canonical new-incarnation storage, complete OLD closure, exact event/delta/cursor behavior and fail-closed paths. Replay may remain deferred. |
| `covered` | All applicable positive/negative cases, atomicity, order, perspective knowledge/projection, noninterference, delta, checkpoint/restore, fork, rerun, plus genuine authoritative replay evidence. Replay may come from separately reviewed/versioned replay support or a later producer/integration path; until then coverage is blocked. |
| `certified` | Out of scope and explicitly not targeted. |

S2 coverage does not satisfy producer interactions. Future acceptance must
separately prove `state-based-actions-combat × zone-incarnation` and
`draw-card × zone-incarnation` when those producers are selected.
