# M3.S2 `rules/zone-incarnation@0.1.0` Specification

**Status:** selection-candidate design; ready for independent review
**Capability lifecycle:** `specified`; no implementation or coverage claim
**Production implementation authorized by this document:** NO
**Date:** 2026-09-23
**Capability:** `rules/zone-incarnation@0.1.0`

This is the design output for a possible next M3 slice. It does not select S2,
authorize implementation, or change production behavior.

## 1. Status, baseline, and authority

Before repository-specific claims, `origin/master` was fetched and verified:

```text
REMOTE = https://github.com/chrismaghuhn/Manafold
EXPECTED_MASTER = cf60c012113550d4e8449c72fea938167002fddc
VERIFIED_ORIGIN_MASTER = cf60c012113550d4e8449c72fea938167002fddc
SOURCE_BASE = cf60c012113550d4e8449c72fea938167002fddc
```

The source tree was clean before creating the design branch from that exact
remote head. Current repository status records S1 complete/covered/not
certified, ten specified Foundation capabilities, zero implemented, one
covered, zero certified, and the next gate as
`M3_S2_SELECTION_OR_AUTHORIZATION`. Issue #178 remains the M3 tracker. This
planning artifact is not an authorization record.

Binding authority is the accepted normative hierarchy, accepted ADRs, the
executable contracts, then this candidate design. Magic rules authority is
restricted to the accepted snapshot
`wotc-cr-2026-08-07-txt-20260819-sha256-4381ad1b39ab2c05f7d03633a20f711ed37277074d3266dcba5f38cbb527423f`
and only CR `400.1–400.7`, `400.7j`, `401.1–401.2`, `402.1`, and `700.4`.
No additional CR section is required by this isolated transition primitive;
producer-specific rules remain outside S2.

## 2. Problem statement

The repository has authoritative object/location/order state and a typed
`ZoneTransition` audit product, but no production executor that atomically
moves one admitted card between the two frozen S2 zone families while
allocating a fresh incarnation and updating every dependent product. The
fixture-only helper cannot be promoted into runtime behavior. The capability
must provide one reusable primitive for future producers and preserve exact
event, delta, identity, observation, and replay behavior.

## 3. Existing substrate and design questions

Inspected current owners include `mtgml-state::EngineState`,
`ZoneState::{objects,locations,ordered_zones}`, `IdentityAllocatorState`,
`ObjectSnapshot`, `ZoneTransition`, `StateDelta`,
`PerspectiveIdentityStateV2`, `KnowledgeStateV2`,
`PerspectiveLifecycleAuditV1`, observation projection, the semantic cursor,
the transition contract, and V5 checkpoint/replay/fork paths.

| Question | Current answer and consequence |
| --- | --- |
| Q1 authoritative mutation? | No production zone-transition mutation executor exists. `FixtureTransition::move_object_incarnation` is behind the conformance-fixture feature; it allocates an object ID and edits maps/vectors only for synthetic unordered-zone fixtures. Typed event/product and validation infrastructure do exist. This is implementation work, not an alternative runtime authority. |
| Q2 executor owner? | One private authoritative executor in `mtgml-rules`' rules-owned transition pipeline, callable by validated future producers and the test facade. `mtgml-state` remains data/invariant owner; it must not become a second rules interpreter. |
| Q3 allocation owner? | The `mtgml-state::IdentityAllocatorState` is the sole persisted global allocator; the executor requests exactly one checked fresh `GameObjectId` from its `next_object_id` cursor. Add a checked object allocation operation there if needed. No parallel counter. |
| Q4 delta allocator proof? | `StateDelta` carries a complete `EngineStateParts` replacement, including allocators. It has no per-allocator audit operation. Event cursor validation checks zone identity shape but currently does not prove exact `next_object_id` progression. S2 must add explicit transition-contract proof: one ID step per accepted transition, no other allocator movement; delta reapplication proves the complete replacement. |
| Q5 opaque lifecycle? | `IdentityMutationV1::{Remap,Retire,Allocate}` and corresponding knowledge mutations already exist and are cursor-applied. Remap preserves the existing opaque ID without advancing its allocator; Retire is permanent; Allocate consumes the perspective-local next ID. The executor does not choose one global mapping policy: each perspective's authorized occurrence supplies its own lifecycle mutation under the closed pairing and distinguishability contracts. |
| Q6 battlefield→graveyard knowledge? | Known tracked objects retain their authorized identity/definition. On an observed public move while distinguishable, remap the same opaque ID to the new incarnation and update its current known location to the graveyard; the prior current fact moves into ordered history with its original provenance. Unknown/unobserved identity does not become known merely because the trusted transition carries a physical ID. No historical fact is deleted by S2. |
| Q7 library→hand knowledge? | For the owner, the selected test must model the private hand occurrence using existing private knowledge: allocate an opaque ID plus private acquisition when the new card first becomes distinguishable, or remap/update the same opaque ID if a valid prior private identity already existed and remains distinguishable. The non-owner receives no card identity, definition, physical ID, opaque mapping, knowledge acquisition, or visible event from this hidden-to-private movement. No hidden library position is newly disclosed. Exact paired cases establish this. |
| Q8 event/snapshot expressivity? | Yes. Existing `ZoneTransition` carries old/new GameObjectIds, optional PhysicalCardId, from/to, exact `last_known`, and complete `new_snapshot`; existing event and delta variants carry it without a schema change. Conformance must require `Some(PhysicalCardId)` for selected physical-card cases. |
| Q9 replay derivation? | The allocator state is checkpointed, so a replayable accepted transition would derive NEW from its restored allocator. However, current `ReplayStepV5` accepts only `DecisionResponseV2`; the selected direct S2 request is not representable because S2 intentionally implements no producer decision. V5 cannot replay this isolated transition today. This is a **BLOCKER** for the requested S2 replay-parity case until a separately reviewed solution establishes an authorized replay input without fabricating a player response or reinterpreting V5. |
| Q10 reusable validation? | Reuse `object_snapshots`, `apply_perspective_lifecycle`, `validate_occurrence_pairing`, `SemanticValidationCursor`, `validate_transition_contract`, `StateDelta::apply`, engine-state validation, the production projection APIs, and V5 checkpoint/restore/fork/replay executors. `FixtureTransition` is a harness/reference for product paths only, not production logic or an independent rules implementation. |

The missing production executor and allocator/cursor progression proof are
major implementation obligations. A concrete replay-input gap is identified
in Q9 and remains a blocker to claiming this plan implementation-ready in all
respects. The current V5 response-only replay contract cannot execute the
required isolated direct S2 transition. Do not paper over this by inventing a
player response, treating an after-state checkpoint as replay of the
transition, replaying authoritative events as an input, or silently changing
V5. Independent review must resolve whether S2 should have a separately
versioned trusted semantic-transition replay input or whether the replay
parity obligation is properly deferred to producer interaction evidence.
Either choice requires explicit design approval before implementation; a
public persistence identity change also requires its own contract/version
work.

## 4. Capability identity

Use the accepted Foundation V2 identity verbatim:

```text
key = rules/zone-incarnation
version = 0.1.0
primary_semantic_owner = zones_identity
primary_orchestrator = zone-transition-pipeline
information_risk = high
dependencies = none
physical_state_owner = mtgml-state::EngineState
```

The producer asks for a transition; S2 executes the selected transition. It
does not derive why it is required.

## 5. Included scope

Only these moves are admitted:

1. `Battlefield → owner Graveyard`, with a validated explicit request
   identifying the exact live source incarnation and exact source location.
   This tests the reusable primitive without claiming an SBA cause.
2. `owner Library → same owner's Hand`, consuming the exact top member of a
   nonempty known ordered library. The caller is a validated explicit test
   request; S2 does not implement Draw timing or draw legality.

Every accepted move creates one new `GameObjectId`, preserves the same
physical-card continuity, snapshots the old incarnation before mutation,
updates exact zone membership/order, emits the existing typed transition, and
applies only perspective-authorized identity/knowledge/observation changes.

## 6. Explicit exclusions

The Foundation V2 exclusions remain exact: arbitrary zone movement,
shuffle/randomization, copy continuity, attachments, tokens, exile, command
zone, casting/resolution exceptions, general LKI-trigger semantics, and
hidden-randomization identity retirement beyond existing substrate behavior.
Also excluded: empty-library loss, SBA execution, Draw execution, replacement
effects, trigger scheduling, new reveal/face-down rules, and any card/deck/
format support claim. S2 cannot reinterpret these as a general zone engine.

## 7. Identity model

These identities have distinct owners and lifetimes:

```text
PhysicalCardId  = physical card continuity across selected moves
GameObjectId    = one authoritative game-object incarnation
OpaqueObjectId  = one perspective-local visible/distinguishable identity
```

For each selected move, `NEW != OLD`; OLD ceases to be live; NEW is live at
the exact destination; and PhysicalCardId is exactly preserved. PhysicalCardId
continuity does not imply player-visible identity continuity. Neither
PhysicalCardId nor trusted GameObjectId may occur in any player API, event,
error, digest input exposed to a player, or player-safe diagnostic. Projection
substitutes only authorized perspective-local opaque IDs.

## 8. Selected transition families and snapshot derivation

The executor validates one closed family and its exact source/destination
shape before mutation. Destination ownership is the card owner. Battlefield
source and graveyard destination are unordered in this selected profile. The
library source is `Top { offset: 0 }`; hand destination uses the admitted
unordered position. No target position or family is caller-free-form.

`last_known` is byte/value-exactly the old live object plus its exact source
location, captured before mutation. `new_snapshot` names NEW and the exact
destination, carries the same physical ID, card definition, owner, controller,
tapped and face-down fields as the selected source representation, and is
consistent with the new object-table row. The initial cases use non-face-down
physical cards and explicit validated controller/tapped values to avoid
introducing reveal or battlefield-status semantics. Implementations must not
infer new card characteristics or perform an unmodeled state reset.

For ordered library removal, remove the exact first vector element once,
remove OLD from locations/objects, renumber each remaining `ZonePosition::Top`
to its exact new vector offset, remove empty ordered-zone keys only where the
state contract allows (valid nonempty source means the source key disappears
only when the last member was consumed), and insert NEW exactly once into
the destination membership. Vector order is authoritative; position is its
redundant witness. Graveyard and hand order remain the frozen unordered case.

## 9. Transaction model and allocation

The executor operates on a private candidate state/product. Validation and
all checked arithmetic happen before commit. The accepted operation is one
atomic kernel transition: one state revision, one zone transition event, one
new object allocation, zero RNG use, and the prescribed per-perspective
occurrences. No partial products escape.

`next_object_id` supplies NEW and advances exactly once with checked
arithmetic. Reject if the cursor is exhausted, collides with a live or
otherwise forbidden issued identity, or cannot advance. No unrelated global
allocator advances (event allocator advances only for the emitted event under
the existing event contract; decision/effect/ability/stack/trigger/
continuation allocators stay fixed). Perspective opaque allocators advance
only for a justified `Allocate` mutation. Rejection consumes no allocator.

## 10. Physical continuity

Both positive families require a physical-card object with `Some(CARD)`;
OLD's object field, old snapshot, transition field, and NEW's object/new
snapshot must all carry exactly CARD. No second PhysicalCardId map is added.
Existing authoritative zone state and validation remain the source of live
physical ownership/uniqueness. A token, copy-only object, absent or mismatched
physical card fails closed.

## 11. ObjectSnapshot / LKI boundary

The old snapshot is transition-bound LKI input for trusted event/audit and
conformance only. It is not retained as a live object, persistent post-state,
or general trigger input. `new_snapshot` is the exact resulting incarnation
and participates in event/cursor equality. S2 does not decide consumer
semantics for LKI.

## 12. Perspective identity lifecycle

Apply mutations with existing `PerspectiveLifecycleAuditV1`, its
`PerspectiveLifecycleMutationV1`, and closed `PerspectiveObservationPolicyV1`
pairings. Derive independently for each perspective from authorized sight and
distinguishability:

* observed and continuously distinguishable: preserve opaque identity via
  `Remap`; it points to NEW after the lifecycle event and its allocator does
  not advance;
* first becomes distinguishable (the owner privately receives the card):
  `Allocate` exactly the perspective's next opaque ID;
* no authorized event/knowledge about an opponent's hidden card: leave its
  mappings, retired set, allocator and visible sequence unchanged;
* retire only if an already accepted information contract requires loss of
  distinguishability. These selected nonrandomizing cases do not justify
  retirement by themselves.

Never infer `old OpaqueObjectId == new OpaqueObjectId` solely from physical
continuity. Never link a public old identity to a new hidden identity unless
the per-perspective contract authorizes that link.

## 13. Knowledge, observation, and observed events

Every state-changing occurrence uses the existing perspective lifecycle
event, consuming exactly one sequence for that perspective; a hidden
non-occurrence consumes none. For a visible tracked battlefield move,
`MovedInSight` binds the exact old/new transition; `Remap` plus
`UpdateLocation` moves the current location to owner graveyard and archives
the prior current location. Public observed location facts use the enclosing
visible sequence and Public provenance. Historical facts and acquisition
provenance remain intact.

For the library family, the owner receives only an authorized private
appearance/movement. On a first-known card, use the existing envelope-less
`NoEnvelope` occurrence with `Allocate` plus private `Acquire`: the
`Appeared` policy's current pairing contract is restricted to public explicit
reveal/public-event knowledge and must not be stretched to private hand
knowledge. If a valid preexisting private identity applies, use the existing
tracked move/remap/update pairing only where its exact policy permits the
source and destination. The non-owner receives no card-bearing visible
product and no lifecycle event. Owner product may identify the card through
opaque ID and authorized definition/location; opponent product must be
byte-identical across paired worlds differing only in that hidden card
identity.

`PlayerObservation`, `PlayerInformationState`, `PerspectiveIdentityState`,
retained knowledge, opaque lifecycle, and observed events are produced only
through existing authoritative contracts and read-only projections. Events
must preserve redaction rules and contiguous perspective-local sequences.
Tests choose known, non-face-down source states and do not add reveal,
face-down, or randomization behavior.

## 14. Event, delta, and semantic cursor contract

Use existing `AuthoritativeRuleEventKind::ZoneTransition` and
`SemanticDeltaOperation::ZoneTransition`. Do not add a second movement event.
Lifecycle occurrences use the existing `PerspectiveOccurrence` event and
matching `PerspectiveLifecycle` delta operation. Event order is deterministic:
zone transition, then per-perspective lifecycle occurrences in canonical
PlayerId order; event IDs are dense and bind to the new revision.

`StateDelta::between` contains the complete replacement state, including
zones, object allocator, knowledge, identity, RNG and counters. Its audit
mirrors the authoritative event trace in order. Applying it to the exact
before state must yield the exact after state and digest. Cursor extension
must prove old exact snapshot/source, unique fresh NEW, exact old/new
incarnation relation, matching physical ID/locations, one object allocator
step, lifecycle causality, and final authoritative object snapshots. The
transition contract additionally proves exact membership/order and that all
unrelated allocators are unchanged. No new wire schema is required unless
implementation discovers an unrepresentable fact; that would return to
design review rather than an ad hoc extension.

## 15. Checkpoint, fork, replay, and deterministic execution

Same checkpoint plus same validated request produces identical resulting
state, fresh IDs, event sequence, delta, player products, state digest, and
checkpoint identity. Selected transitions use no RNG and rejected requests
consume no RNG. V5 checkpoint/restore carries all allocator, zone-order,
identity, knowledge, and sequence state required to resume. Forked equal
inputs match. Replay would derive NEW from restored allocator state and
reproject the same products, with no caller-supplied NEW identity, but current
V5 has no direct-transition input. The replay requirement is BLOCKED pending
the design resolution recorded in §3/Q9; checkpoint/fork/rerun parity do not
substitute for actual transition replay.

## 16. Rejection atomicity

Every rejected typed request preserves a complete before fingerprint:

```text
EngineState and digest; zones, ordered zones, objects and locations;
all global/perspective allocators; PhysicalCardId associations;
perspective mappings/retired IDs; active/retired knowledge and provenance;
authoritative and observed event history; StateDelta/result products;
RNG roots/streams/cursors; checkpoint identity; replay cursor/state;
environment counters/status; PlayerObservation, PlayerInformationState,
observed events, decisions and all visible products.
```

Rejected input emits no committed event/delta, increments no revision or
counter, consumes no IDs/RNG/visible sequence, and changes no player bytes.
This is broader than helper-local workspace rollback.

## 17. Unsupported behavior and negative matrix

Fail closed for source object absent; source location mismatch; any family
outside the two admitted moves; wrong-owner graveyard/hand; reused/colliding
GameObjectId; allocator exhaustion; absent/mismatched PhysicalCardId;
invalid old/new snapshots; malformed lifecycle pairing; invalid before
state; unsupported token/copy/attachment profile; and a library request that
does not consume exact top offset zero. Also fail closed for battlefield→
exile/command, graveyard→battlefield, hand→battlefield, cast/resolution,
shuffle/random relocation, replacement, triggers, general LKI consumers, and
unsupported reveal/face-down semantics. Empty-library loss is not part of
this capability.

## 18. Conformance matrix

Proposed stable IDs only; do not register them in the capability registry
until a separate S2 selection/authorization and implementation change.

| Case IDs | Required evidence |
| --- | --- |
| `s2.zone.battlefield_graveyard` | Direct validated primitive request; NEW differs from OLD; exact owner graveyard membership; OLD removed once; exact snapshots/event/delta/cursor; only `next_object_id` and event ID progress globally. No SBA cause is asserted. |
| `s2.zone.library_hand_top` | Consume only exact top object; exact vector/offset rewrite; owner hand insertion; new incarnation and physical continuity; no RNG. |
| `s2.identity.public_remap` | Each perspective with authorized public distinguishability remaps existing opaque ID; no opaque allocation; location history/provenance exact. |
| `s2.identity.owner_hand_private` | First private appearance allocates one owner-local opaque ID and private knowledge; test pre-known tracked variant remaps without allocation. |
| `s2.observation.no_trusted_ids` | No PhysicalCardId/GameObjectId in player DTOs, events, metadata, or errors. |
| `s2.observation.library_noninterference` | Paired states differing only in opponent hidden library identity produce byte-equal non-owner observation, information, events, errors, and visible products. |
| `s2.observation.lifecycle_matrix` | Exact per-perspective visible event, opaque mapping, retained/current/historical knowledge, provenance and sequence for both move families. |
| `s2.rejection.source_absent` | Absent source preserves the full §16 fingerprint. |
| `s2.rejection.source_location` | Exact source-location mismatch preserves the full fingerprint. |
| `s2.rejection.unadmitted_family` | Each other source/destination family fails closed and preserves the fingerprint. |
| `s2.rejection.wrong_owner_destination` | Wrong-owner graveyard/hand fails closed and preserves the fingerprint. |
| `s2.rejection.object_id_reuse` | Reused/colliding NEW ID fails closed; no allocator movement. |
| `s2.rejection.object_id_exhaustion` | Checked allocator exhaustion fails before commit; no partial product. |
| `s2.rejection.physical_continuity` | Missing or mismatched physical identity in any transition component rejects. |
| `s2.rejection.snapshot_pairing` | Old/new snapshot mismatch or invalid incarnation/location pair rejects. |
| `s2.rejection.lifecycle_pairing` | Malformed perspective lifecycle/knowledge pairing rejects atomically. |
| `s2.rejection.unsupported_profile` | Token/copy/attachment/excluded profile is rejected where represented/detectable. |
| `s2.rejection.library_not_top` | Any library source other than the exact ordered top rejects without reordering. |
| `s2.rejection.invalid_before_state` | Invalid authoritative state is rejected before transition mutation. |
| `s2.replay.checkpoint_restore` | Post-move checkpoint restore continues with exact allocator identity, state/digest and products. |
| `s2.replay.fork` | Equal fork inputs match all authoritative and player products. |
| `s2.replay.authoritative` | Re-execution derives same NEW/event/delta/digest and exact projections. |
| `s2.replay.rerun` | Same initial checkpoint and transition gives byte/value-identical result; zero RNG delta. |
| `s2.zone.delta_reapplication` | Delta reapplies to exact after-state and agrees with event audit and digest. |

Use existing production validation and projections as oracle surfaces; the
test reference must be independent of the executor and cannot enter runtime.

## 19. Lifecycle evidence target

The target is only `specified → implemented → covered`, never certified.

* `specified`: this reviewed capability scope and explicit exclusions match
  Foundation V2.
* `implemented`: reviewed authoritative Rust executor and allocator behavior
  exist; state/event/delta/cursor and information integration are real
  production behavior, with unsupported paths fail-closed. Compilation alone
  is insufficient.
* `covered`: all applicable proposed cases, negative atomicity, ordered-zone
  invariants, paired-state information checks, delta reapplication,
  checkpoint/restore, fork, replay and deterministic rerun execute and pass at
  exact head. Tests do not imply certification.

No benchmark, card/deck/format support, or certification claim follows.

## 20. S3 and future interaction boundary

S2 proves the reusable primitive in isolation. Required later obligations
remain unsatisfied:

```text
state-based-actions-combat × zone-incarnation
draw-card × zone-incarnation
```

A direct S2 request is not evidence that SBA derivation or Draw timing,
legality, producer transaction integration, or related fixed-point semantics
work. Future producer capabilities own the reason for requesting a move;
S2 owns the selected transition mechanics.

## 21. Non-goals

No production edits in this task; no schema/wire or generated contract
changes; no capability registry change; no new CR authority; no S2 selection
or authorization; no implementation start; no SBA/Draw, triggers, replacement
effects, card/deck/format work, certification, freeze, or merge.
