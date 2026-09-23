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
and only CR `108.4–108.4a`, `110.5`, `400.1–400.7`, `400.7j`,
`401.1–401.2`, `402.1–402.3`, `404.1–404.3`, and `700.4`. CR 404 is needed
because the selected battlefield move inserts the card on top of its owner's
graveyard and preserves the order already there. The other added clauses
ground canonical representation of owner/controller and permanent status,
plus hand visibility/arrangement. No producer-specific authority is added.

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
| Q1 authoritative mutation? | No production zone-transition mutation executor exists. `FixtureTransition::move_object_incarnation` is behind the conformance-fixture feature; it allocates an object ID and edits maps/vectors for synthetic cases, but only accepts an unordered destination and therefore cannot insert/reindex the ordered Graveyard. Typed event/product and validation infrastructure do exist. This is implementation work, not an alternative runtime authority. |
| Q2 executor owner? | One private authoritative executor in `mtgml-rules`' rules-owned transition pipeline, callable by validated future producers and the test facade. `mtgml-state` remains data/invariant owner; it must not become a second rules interpreter. |
| Q3 allocation owner? | The `mtgml-state::IdentityAllocatorState` is the sole persisted global allocator; the executor requests exactly one checked fresh `GameObjectId` from its `next_object_id` cursor. Add a checked object allocation operation there if needed. No parallel counter. |
| Q4 delta allocator proof? | `StateDelta` carries a complete `EngineStateParts` replacement, including allocators. It has no per-allocator audit operation. Event cursor validation checks zone identity shape but currently does not prove exact `next_object_id` progression. S2 must add explicit transition-contract proof: one ID step per accepted transition, no other allocator movement; delta reapplication proves the complete replacement. |
| Q5 opaque lifecycle? | `IdentityMutationV1::{Remap,Retire,Allocate}` and corresponding knowledge mutations already exist and are cursor-applied. Remap preserves the existing opaque ID without advancing its allocator; Retire is permanent; Allocate consumes the perspective-local next ID. The executor does not choose one global mapping policy: each perspective's authorized occurrence supplies its own lifecycle mutation under the closed pairing and distinguishability contracts. |
| Q6 battlefield→graveyard knowledge? | Known tracked objects retain their authorized identity/definition. On an observed public move while distinguishable, remap the same opaque ID to the new incarnation and update its current known location to the graveyard; the prior current fact moves into ordered history with its original provenance. Unknown/unobserved identity does not become known merely because the trusted transition carries a physical ID. No historical fact is deleted by S2. |
| Q7 library→hand knowledge? | For the owner, the selected test must model the private hand occurrence using existing private knowledge: allocate an opaque ID plus private acquisition when the new card first becomes distinguishable, or remap/update the same opaque ID if a valid prior private identity already existed and remains distinguishable. The non-owner receives no card identity, definition, physical ID, opaque mapping, knowledge acquisition, or visible event from this hidden-to-private movement. No hidden library position is newly disclosed. Exact paired cases establish this. |
| Q8 event/snapshot expressivity? | Yes. Existing `ZoneTransition` carries old/new GameObjectIds, optional PhysicalCardId, from/to, exact `last_known`, and complete `new_snapshot`; existing event and delta variants carry it without a schema change. Conformance must require `Some(PhysicalCardId)` for selected physical-card cases. |
| Q9 replay derivation? | Allocator state is checkpointed, so a replayable accepted transition would derive NEW from its restored allocator. Current `ReplayStepV5` accepts only `DecisionResponseV2`; an isolated S2 direct request has no legitimate V5 input. This is a required but deferred coverage obligation: it does not prevent S2 core implementation or `implemented`, but it blocks `covered` until either separately reviewed/versioned replay support or a later replayable producer/integration path reproduces this exact transition. |
| Q10 reusable validation? | Reuse `object_snapshots`, `apply_perspective_lifecycle`, `validate_occurrence_pairing`, `SemanticValidationCursor`, `validate_transition_contract`, `StateDelta::apply`, engine-state validation, the production projection APIs, and V5 checkpoint/restore/fork/replay executors. `FixtureTransition` is a harness/reference for product paths only, not production logic or an independent rules implementation. |

The missing production executor and allocator/cursor progression proof are
implementation obligations. Current V5 cannot replay the isolated direct S2
request, a known limitation of response-only V5 replay and forced progress.
S2 core implementation may proceed to `implemented` after explicit
selection/authorization. Replay remains required Foundation evidence and is
deferred only as a lifecycle promotion gate: `covered` is prohibited until
genuine authoritative replay evidence exists through either (A) separately
reviewed/versioned replay-contract support or (B) a later replayable producer
or integration path that executes this exact transition and reproduces its
typed event, resulting incarnation identity, state, delta, and player
products. Do not invent a response, treat a post-transition checkpoint or an
empty replay as transition replay, replay authoritative events as input, or
silently change V5. This disposition does not waive or redefine Foundation
V2's replay obligation and does not itself authorize replay-contract work.

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

## 8. OLD-incarnation reference closure

"OLD ceases to exist" is a complete `EngineState` condition, not merely
removal from `zones.objects`. Before mutation, the executor validates an
exhaustive typed-reference scan over the current `EngineState` representation.
It requires OLD to have no live authoritative reference except its expected
zone membership, the optional `foundation_sources` entry explicitly closed
below, and perspective identity mappings that are changed through the
authorized lifecycle contract:

* `zones.objects[OLD]`, `zones.locations[OLD]`, and all source ordered-zone
  membership are removed;
* `foundation_sources[OLD]` is removed when present for
  Battlefield→Graveyard. No `foundation_sources[NEW]` is created in either
  destination zone: source characteristics, marked damage, and control
  history belong to the old permanent incarnation;
* if `combat` attackers or blocker keys/values refer to OLD, reject before
  mutation. S2 does not rewrite combat assignments;
* if any stack record `source_object` refers to OLD, reject before mutation;
* if any pending decision trusted binding refers to OLD, reject before
  mutation. S2 does not repair candidates or continuations;
* existing effects, waiting/delayed triggers, continuations, and format state
  contain no direct `GameObjectId` reference in the inspected current model.
  The exhaustive scan must be updated and reviewed whenever a new such field
  is added; if it refers to OLD, fail closed unless a separately authorized
  exact cleanup rule exists;
* active `PerspectiveIdentityState` mappings are the deliberate exception:
  only existing per-perspective `Remap`/`Allocate`/`Retire` lifecycle
  mutations may update them. Knowledge records contain opaque IDs rather than
  a duplicate authoritative GameObjectId association. Event snapshots and
  deltas may retain OLD as historical audit, never as live EngineState.

This is a transition precondition plus exact bounded closure, not generic
reference cleanup. `s2.identity.old_reference_closure` proves every live
reference category; dedicated mutant cases seed each forbidden reference and
prove unchanged rejection. `s2.identity.foundation_source_cessation`
positively proves OLD source data is removed and no source data transfers to
NEW. The historical `s2.rejection.stale_old_incarnation` witness first runs a
real accepted OLD→NEW transition, then attempts a second admitted transition
using OLD from the resulting state and proves rejection plus the complete
unchanged after-state fingerprint. `s2.mutant.stale_old_lifecycle_occurrence`
adds a mutated subsequent lifecycle occurrence referencing OLD after the
transition event and proves contract rejection. Neither case fabricates a
player decision response.

## 9. Selected transition families and snapshot derivation

The executor validates one closed family and its exact source/destination
shape before mutation. Battlefield source is unordered and public. The
owner's Graveyard is ordered and public; NEW is inserted at its top. Existing
graveyard order is preserved: every prior member shifts down exactly one
offset. The library source is ordered with exact top `Top { offset: 0 }`; the
owner's Hand destination is hidden/owner-only and unordered. These are the
only admitted destination rules. S2 processes one object at a time;
simultaneous multi-object graveyard ordering is excluded.

The S2 input profile requires a face-up Battlefield source and face-up
existing Graveyard members; CR 404.2 requires the Graveyard pile to be
face-up. The selected Library top and destination Hand are not face-down
objects; their hidden status is represented by their zones. Reject an
otherwise structurally valid setup violating these admitted status facts;
S2 does not implement face-down transitions/reveals.

The new incarnation carries only identity/card continuity required here:
`PhysicalCardId`, `owner`, and `CardDefinitionId`. CR 400.7 establishes a new
object with no memory or relation to its previous existence; do not copy
permanent state as continuity. The existing `GameObject` storage requires a
`controller: PlayerId` even where Magic has no controller. For the admitted
Hand and Graveyard destinations, store `controller = owner` as the canonical
valid-player representation; this is storage normalization, **not** a claim
that Magic assigns control there. Reset `tapped = false`; neither destination
is a tapped permanent. Set `face_down = false`; a Graveyard card is face up,
and a hidden Hand is not a face-down card/permanent. Hidden-zone visibility is
encoded by `ZoneLocation.visibility`, not by retaining old face-down status.
These canonical values are part of exact `new_snapshot`/state assertions.

`last_known` remains byte/value-exactly the old live object plus its exact
source location, captured before mutation. `new_snapshot` names NEW, exact
destination, preserved physical/card/owner facts, and the destination
canonical storage values above; it agrees exactly with the new object-table
row. S2 does not invent new card characteristics.

For Battlefield→Graveyard, remove OLD from its unordered source, insert NEW
at graveyard vector index zero, and update every existing graveyard member's
`ZonePosition::Top` from offset `n` to `n+1`. NEW's location is
`Top { offset: 0 }`. For Library→Hand, consume the exact first library vector
member once, shift remaining library offsets from `n` to `n-1`, and insert NEW
into the owner's unordered hand. Vector order is authoritative;
`ZoneLocation.position` is its exact redundant witness. Remove an ordered-zone
key only when its final member is consumed, as required by canonical state.

## 10. Transaction model and allocation

The executor operates on a private candidate state/product. Validation,
OLD-reference closure, and all checked arithmetic happen before commit. The
accepted operation is one atomic kernel transition: one state revision, one
zone transition event, one new object allocation, zero RNG use, and the
prescribed per-perspective occurrences. No partial products escape.

`next_object_id` supplies NEW and advances exactly once with checked
arithmetic. Reject if the cursor is exhausted, collides with a live or
otherwise forbidden issued identity, or cannot advance. No unrelated global
allocator advances (event allocator advances only for the emitted event under
the existing event contract; decision/effect/ability/stack/trigger/
continuation allocators stay fixed). Perspective opaque allocators advance
only for a justified `Allocate` mutation. Rejection consumes no allocator.

Graveyard vector and all affected `ZoneLocation.position` updates are
computed from the pre-state in canonical order. No player-chosen ordering,
insertion policy, or randomized operation is involved.

## 11. Physical continuity

Both positive families require a physical-card object with `Some(CARD)`;
OLD's object field, old snapshot, transition field, and NEW's object/new
snapshot must all carry exactly CARD. No second PhysicalCardId map is added.
Existing authoritative zone state and validation remain the source of live
physical ownership/uniqueness. A token, copy-only object, absent or mismatched
physical card fails closed.

## 12. ObjectSnapshot / LKI boundary

The old snapshot is transition-bound LKI input for trusted event/audit and
conformance only. It is not retained as a live object, persistent post-state,
or general trigger input. `new_snapshot` is the exact resulting incarnation
and participates in event/cursor equality. S2 does not decide consumer
semantics for LKI.

## 13. Perspective identity lifecycle

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

## 14. Knowledge, observation, and observed events

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

## 15. Event, delta, and semantic cursor contract

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
incarnation relation, matching physical ID/locations, destination-canonical
new snapshot, one object allocator step, lifecycle causality, OLD
`foundation_sources` cessation, and final authoritative object snapshots. On
`ZoneTransition`, its sequential model removes OLD's foundation source and
does not create one for NEW. The transition contract additionally proves
exact Graveyard top insertion and shifted order witnesses, exact Library top
consumption and shifted order witnesses, all unrelated live reference
closure, and unchanged unrelated allocators. The event remains the existing
typed `ZoneTransition`; no aesthetic duplicate event is added. No new wire
schema is required; if implementation discovers an unrepresentable fact,
return to design review rather than extending a contract ad hoc.

## 16. Checkpoint, fork, replay, and deterministic execution

Same checkpoint plus same validated request produces identical resulting
state, fresh IDs, event sequence, delta, player products, state digest, and
checkpoint identity. Selected transitions use no RNG and rejected requests
consume no RNG. V5 checkpoint/restore carries all allocator, zone-order,
identity, knowledge, and sequence state required to resume. Forked equal
inputs match. Current V5's lack of a direct-transition input is a deferred
required evidence obligation, not a waiver: no invented player response,
after-state checkpoint, empty replay, or event-as-input counts as replay.
Genuine replay evidence is required before `covered`, through either
separately reviewed/versioned replay support or a later replayable producer /
integration path that reproduces the exact S2 transition and all its products.

## 17. Rejection atomicity

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

## 18. Unsupported behavior and negative-test taxonomy

Fail closed for source object absent; source location mismatch; any family
outside the two admitted moves; wrong-owner graveyard/hand; reused/colliding
GameObjectId; allocator exhaustion; absent/mismatched PhysicalCardId;
invalid before state; unsupported token/copy/attachment profile; forbidden
OLD references; and a library request that does not consume exact top offset
zero. Also fail closed for battlefield→
exile/command, graveyard→battlefield, hand→battlefield, cast/resolution,
shuffle/random relocation, replacement, triggers, general LKI consumers, and
unsupported reveal/face-down semantics. Empty-library loss is not part of
this capability.

Separate two proof families:

* **Typed request rejections** contain only caller-controlled request fields:
  absent source ID; mismatched claimed source location; unadmitted
  source/destination family; wrong-owner destination; non-top library source;
  invalid before state. Each proves the complete §17 nonmutation fingerprint.
* **Transition-contract / semantic-cursor / mutant negatives** mutate trusted
  internal candidate products in test-only code: reused/colliding NEW ID;
  exhausted allocator; malformed `last_known`/`new_snapshot` or physical
  continuity; malformed perspective lifecycle/event/delta pairing; bad
  graveyard/library vector or offset rewrite; stale OLD in combat/stack/
  pending binding; missing OLD source cessation or NEW source creation. These
  are never request fields or player inputs. Each proves product rejection
  and unchanged committed state.

Unsupported token/copy/attachment profiles are request rejections only when
the authoritative setup can detect them; otherwise the initial admitted
profile constrains fixtures and capability admission.

## 19. Conformance matrix

Proposed stable IDs only; do not register them in the capability registry
until a separate S2 selection/authorization and implementation change.

| Case IDs | Required evidence |
| --- | --- |
| `s2.zone.battlefield_graveyard` | Direct validated primitive request; NEW differs from OLD; inserted at exact owner Graveyard top; prior graveyard members retain relative order and shift one offset; OLD removed once; exact snapshots/event/delta/cursor; only `next_object_id` and event ID progress globally. No SBA cause is asserted. |
| `s2.zone.library_hand_top` | Consume only exact top object; exact vector/offset rewrite; owner hand insertion; new incarnation and physical continuity; no RNG. |
| `s2.identity.public_remap` | Each perspective with authorized public distinguishability remaps existing opaque ID; no opaque allocation; location history/provenance exact. |
| `s2.identity.owner_hand_private` | First private appearance allocates one owner-local opaque ID and private knowledge; test pre-known tracked variant remaps without allocation. |
| `s2.identity.old_reference_closure` | Inventory and assert absence of every OLD reference except the explicitly processed source and allowed lifecycle remap; each forbidden reference rejects atomically. |
| `s2.identity.foundation_source_cessation` | Graveyard move removes `foundation_sources[OLD]` when present; no destination source record is retained or created. |
| `s2.rejection.stale_old_incarnation` | Execute a real accepted OLD→NEW transition; then request a second admitted operation with OLD from the resulting state. Reject as non-live and preserve the complete resulting-state fingerprint. |
| `s2.mutant.stale_old_lifecycle_occurrence` | After a real ZoneTransition OLD→NEW, mutate a subsequent lifecycle occurrence to reference OLD; the transition contract/cursor rejects it. No DecisionResponse is synthesized. |
| `s2.identity.destination_canonical_state` | Both families prove new controller storage equals owner (normalization only), tapped=false, face_down=false, with PhysicalCardId, owner and CardDefinitionId preserved. |
| `s2.zone.graveyard_order` | NEW is top; existing Graveyard relative order is unchanged; every shifted `Top` offset agrees with vector ordinal. |
| `s2.observation.no_trusted_ids` | No PhysicalCardId/GameObjectId in player DTOs, events, metadata, or errors. |
| `s2.observation.library_noninterference` | Paired states differing only in opponent hidden library identity produce byte-equal non-owner observation, information, events, errors, and visible products. |
| `s2.observation.lifecycle_matrix` | Exact per-perspective visible event, opaque mapping, retained/current/historical knowledge, provenance and sequence for both move families. |
| `s2.rejection.source_absent` | Absent source preserves the full §17 fingerprint. |
| `s2.rejection.source_location` | Exact source-location mismatch preserves the full fingerprint. |
| `s2.rejection.unadmitted_family` | Each other source/destination family fails closed and preserves the fingerprint. |
| `s2.rejection.wrong_owner_destination` | Wrong-owner graveyard/hand fails closed and preserves the fingerprint. |
| `s2.mutant.object_id_reuse` | Mutated internal candidate with reused/colliding NEW ID fails transition validation; no allocator movement. |
| `s2.state.object_id_exhaustion` | A valid request against an exhausted authoritative allocator fails before commit; no partial product. |
| `s2.mutant.physical_continuity` | Mutated trusted event product with missing/mismatched physical identity is rejected by transition validation. |
| `s2.mutant.snapshot_pairing` | Mutated trusted old/new snapshot or invalid incarnation/location pair is rejected. |
| `s2.mutant.lifecycle_pairing` | Mutated internal lifecycle/knowledge/event pairing is rejected atomically. |
| `s2.mutant.delta_pairing` | Mutated event/delta audit or replacement mismatch is rejected. |
| `s2.mutant.graveyard_order` | Mutated destination insertion or shifted offsets reject; tests mutate validator products, never caller input. |
| `s2.mutant.old_reference` | Mutated candidate state leaving OLD in a forbidden reference fails contract/state validation. |
| `s2.rejection.unsupported_profile` | Token/copy/attachment/excluded profile is rejected where represented/detectable. |
| `s2.rejection.library_not_top` | Any library source other than the exact ordered top rejects without reordering. |
| `s2.state.invalid_before_state` | Invalid authoritative before-state fails state admission before transition mutation. |
| `s2.replay.checkpoint_restore` | Post-move checkpoint restore continues with exact allocator identity, state/digest and products. |
| `s2.replay.fork` | Equal fork inputs match all authoritative and player products. |
| `s2.replay.authoritative` | `DEFERRED_REQUIRED / BLOCKED for covered`: genuine replay must derive the same NEW/event/delta/digest and projections through separately reviewed/versioned replay support or a later producer/integration path. |
| `s2.replay.rerun` | Same initial checkpoint and direct transition gives byte/value-identical result; zero RNG delta. This is rerun parity, not replay evidence. |
| `s2.zone.delta_reapplication` | Delta reapplies to exact after-state and agrees with event audit and digest. |

Use existing production validation and projections as oracle surfaces; the
test reference must be independent of the executor and cannot enter runtime.

## 20. Lifecycle evidence target

The target is only `specified → implemented → covered`, never certified.

* `specified`: this reviewed capability scope and explicit exclusions match
  Foundation V2.
* `implemented`: after explicit S2 selection/authorization, reviewed
  authoritative Rust executor and allocator behavior exist; state/event/
  delta/cursor and information integration are real production behavior,
  with unsupported paths fail-closed. Compilation alone is insufficient.
  Genuine replay may remain deferred at this stage.
* `covered`: all applicable proposed cases, negative atomicity, ordered-zone
  invariants, paired-state information checks, delta reapplication,
  checkpoint/restore, fork, deterministic rerun, and genuine authoritative
  replay execute and pass at exact head. Replay may be provided by separately
  reviewed/versioned replay support or a later replayable producer /
  integration path. Until then `covered` is blocked. Tests do not imply
  certification.

No benchmark, card/deck/format support, or certification claim follows.

## 21. S3 and future interaction boundary

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

## 22. Non-goals

No production edits in this task; no schema/wire or generated contract
changes; no capability registry change; no new CR authority; no S2 selection
or authorization; no implementation start; no SBA/Draw, triggers, replacement
effects, card/deck/format work, certification, freeze, or merge.
