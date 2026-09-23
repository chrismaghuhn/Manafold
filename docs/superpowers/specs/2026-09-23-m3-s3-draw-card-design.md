# M3.S3 `rules/draw-card@0.1.0` Interaction Design

**Status:** selection-candidate design; ready for independent review
**Capability lifecycle:** `specified`; no implementation or coverage claim
**Production implementation authorized by this document:** NO
**Date:** 2026-09-23
**Candidate capability:** `rules/draw-card@0.1.0`

This document closes the design analysis for ordinary Draw Step interaction
with turn structure, zone incarnation, information safety, and replay. It does
not authorize implementation or change the capability lifecycle.

## 1. Task identity, base, and authority

The requested work is M3.S3 selection/design and status closure. It is an
architecture, replay, information-safety, and interaction-design task. It
contains no production implementation.

The remote was fetched before repository-specific claims:

```text
REMOTE = https://github.com/chrismaghuhn/Manafold
REQUIRED_ORIGIN_MASTER = b67cfdcc0a8e623da52a889ef2ae138a3e4256ac
VERIFIED_ORIGIN_MASTER = b67cfdcc0a8e623da52a889ef2ae138a3e4256ac
DESIGN_BRANCH = chris/m3-s3-draw-card-design
```

S2 status closure is recorded separately: PR #208 merged, S2 exact-head
verification passed, and S2 remains `COMPLETE / IMPLEMENTED / NOT COVERED /
NOT CERTIFIED`. `S2_AUTHORITATIVE_REPLAY` remains
`DEFERRED_REQUIRED / BLOCKED_FOR_COVERED`.

Normative authority is the accepted hierarchy, Foundation V2, accepted ADRs,
current semantic contracts, and the current V5 production implementation.
Foundation V2 defines the bounded draw requirement and the outstanding
interaction obligations. ADR 0054 and the S2 design define the existing
Library-to-Hand incarnation and information contracts. ADR 0055 and the
current V5 replay implementation govern replay identity and execution. This
design does not reuse the older S2 V4 replay framing.

## 2. Current substrate characterization

`EngineState.core.position` stores `TurnPosition`; its `BeginningStep::Draw`
variant is a real position in the ordered temporal skeleton. The current
`temporal_successor(Beginning(Draw))` returns `PrecombatMain`. That relation
describes temporal order only; it does not perform the required Draw Step
action or the priority window that follows it.

`unsupported_rules_boundary(Beginning(Draw))` returns
`UnsupportedRulesBoundary::DrawCard`. The Magic forced-progress kernel validates
the complete state and turn-structure profile, performs only ordinary Untap
and quiescent Cleanup, then returns the typed unsupported-boundary error for
Draw. No library card is moved. The S1 downstream Draw conformance witness
asserts that fail-closed behavior.

The environment exposes `execute_forced_progress`. Its standalone form is
restricted to pristine setup and rebases an empty V5 replay recorder; it
records no replay step. Accepted player-response transactions have a separate
path: after applying a real response, the environment may execute one
rules-owned forced-progress transition, merge its events and delta into the
response product, then append one V5 step containing the original
`DecisionResponseV2`. The forced consequence is therefore replayed by
re-executing the accepted response, not represented as a fabricated response
or a second replay input.

`ReplayStepV5` requires a player actor and typed response. V5 backend replay
reconstructs the pending actor/decision, re-executes that response, validates
the transition contract, and compares the resulting state, full-state digest,
status, checkpoint identity, counters, and projections. It has no forced-work
input variant.

The current transition contract proves `TurnPositionChanged` against
`temporal_successor`; that event cannot justify skipping a mandatory boundary.
The semantic validation cursor tracks the current priority value, but has no
priority-change event arm. No current event family authorizes a Draw result or
the transition from no priority to active-player priority.

## 3. Capability identity and candidate disposition

The registry entry remains `rules/draw-card@0.1.0`, lifecycle `specified`,
with declared dependencies `rules/turn-structure` and
`rules/zone-incarnation`. The bounded semantic capability is one ordinary
turn-based draw in a normal Draw Step. It is not a generic draw effect.

**Disposition:** reject Draw as a self-contained S3 implementation slice.
Retain Draw as the semantic producer in a reviewed S3 closure only if the
priority prerequisite and registry dependency closure are resolved first.
Draw must not copy or fork S2's zone-incarnation operation. This document
does not change the registry.

## 4. Bounded supported profile

The proposed executable profile is:

- A validated, ordinary two-player game state with `FormatState::None`.
- `turn_number >= 2`; the Draw Step is the normal beginning phase step, not
  an extra or substituted step.
- `core.position == Beginning(Draw)` and the designated active player is the
  actor and owner of the source Library.
- Priority is `None` before the turn-based draw. There is no pending decision,
  replacement/effect state, trigger work, or unsupported modifier affecting
  the draw.
- The active player's ordered Library is nonempty and its first live object
  is exactly the source supplied to the draw producer.
- The object is admitted by the existing S2 owner-Library-top to same-owner
  Hand profile, with physical-card identity and current state invariants valid.
- Exactly one card is moved through the existing zone-incarnation executor.
- The action consumes no random stream. No player decision is created.
- Completion leaves the turn position at Draw and establishes the active
  player's priority window through the one authoritative basic-priority
  implementation. The next pass-only Decision is owned by basic-priority.

The `turn_number >= 2` admission excludes the first turn, where the first
player's ordinary draw is skipped. The ingress/admission contract must accept
only the selected normal-turn profile. An unclassified setup, history, or
turn/step modification fails closed; the kernel must not infer a game-start
history from missing facts.

## 5. Explicit exclusions

The candidate rejects, before mutation:

- Empty-library draws and loss processing.
- Replacement effects, draw prevention, draw-count modifiers, and multiple
  draws.
- Draw replacements, including dredge-like choices.
- Game-start draws, opening hands, mulligans, or any first-turn Draw Step.
- Extra, skipped, or modified Draw Steps and turns.
- Card-specific draw triggers and trigger processing.
- Any draw outside the selected ordinary Draw Step.
- Casting, spell resolution, and priority behavior beyond the admitted
  basic-priority continuation.
- Library profiles that do not satisfy S2's exact ordered-top contract.

## 6. Authoritative state ownership

`EngineState` is the complete semantic input. The rules kernel owns admission
and sequencing; `mtgml-state` owns the state and invariants; the sole
zone-incarnation executor owns the object/zone identity mutation; existing
knowledge and perspective-identity state own authorized remembered identity;
basic-priority owns the priority protocol. Environment commit remains atomic
over the complete transition product.

No event history, cache, local flag, prior function call, test fixture detail,
or uncheckpointed execution memory may decide whether the draw is pending.

## 7. Exactly-once Draw-Step execution

**Current state distinguishes pending from completed Draw:** YES, for the
admitted valid state machine. The existing pair is:

```text
Beginning(Draw) + PriorityState::None
    = ordinary turn-based draw pending

Beginning(Draw) + PriorityState::HeldBy(active_player, 0)
    = turn-based draw complete; priority window active
```

There is no intermediate accepted state between the turn-based action and
priority assignment. The position alone is insufficient, but `PriorityState`
is already authoritative, included in the full-state digest, and checkpointed.
The transition validator must establish this pairing, reject malformed Draw
states, and prove the single transition from pending to completed. A repeated
forced-progress attempt encounters held priority and rejects without another
zone move.

**New authoritative Draw progress field required:** NO, if the action and
priority grant commit atomically through basic-priority. **If Draw is stopped
after the zone move while priority remains `None`, a new typed authoritative
progress field is required** and must be included in state validation, digest,
checkpoint/restore/fork, serialization, and replay identity. Such a split
state is not admitted by this design.

The current semantic cursor already snapshots priority but cannot apply a
priority transition event. Basic-priority must supply its canonical event and
cursor contract; Draw must not implement a parallel priority mutation.

## 8. Turn-structure interaction

Turn structure admits the normal Draw Step boundary and fixes its place after
Upkeep and before Precombat Main. The Draw producer handles the mandatory
turn-based action at that boundary. `temporal_successor(Draw)` is not called to
move directly to Precombat Main. After the active player receives priority,
basic-priority resolves the explicit pass sequence and only then advances the
turn position.

The interaction closes with evidence for entry into Draw after the preceding
priority window, one ordinary draw, the active-player priority Decision, and
advancement after explicit passes. Draw itself has no choice. No implicit
priority pass is permitted.

## 9. Zone-incarnation interaction

Draw produces semantic intent identifying the exact top object and source/
destination profile. `rules/zone-incarnation` remains the sole operation that
performs:

```text
owner Library top -> same owner's Hand
old GameObjectId -> fresh GameObjectId
PhysicalCardId continuity preserved
```

It updates ordered-zone vectors, the identity allocator, snapshots and
locations, perspective-local mappings, retained knowledge, and lifecycle
products under the S2 contract. Draw may not allocate an object ID or repeat
any identity/knowledge lifecycle logic.

## 10. Decision boundary

`PLAYER_DECISION_CREATED = NO` for the draw. No Decision, DecisionResponseV2,
ChooseOne, Confirm, or synthetic player action represents this forced rule
work. After the action, basic-priority may create the normal explicit
pass-only decision. That is a distinct capability-owned decision at the
priority window, not a confirmation of the draw.

## 11. Information model

The drawing player receives the exact S2-authorized private identity and
knowledge outcome for the new Hand incarnation. The non-owner receives no
`GameObjectId`, `PhysicalCardId`, `CardDefinitionId`, opaque allocation,
card-bearing occurrence, hidden Library identity, or private knowledge
mutation. The S2 owner-Library/Hand observation and lifecycle contract is
reused verbatim; Draw creates no second projection or identity mapping policy.

Conformance uses paired worlds with identical authorized public state but
different hidden top-card identities. Opponent observation bytes,
information-state bytes, visible-decision bytes, observed event arrays,
sequence cursors, and rejection classes must be identical. The owner's
authorized private information must differ exactly as S2 specifies. Trusted
event logs may contain identities, but they are never projected as opponent
data. A generic draw-completed occurrence, if public under the reviewed event
contract, carries no card identity and has identical opponent-visible bytes
in the paired worlds.

## 12. Atomic rejection

Each rejection is evaluated on a validated immutable before-state and commits
nothing. Evidence compares complete state/checkpoint, RNG, every global and
perspective allocator, knowledge, history, events, revision, episode status,
environment counters, replay, and all player products.

Required rejected cases:

| Rejection | Required reason/evidence |
| --- | --- |
| Not in Draw Step | Reject any other phase/step; no successor or mutation. |
| Wrong actor/active player/Library owner | Reject actor mismatch or owner mismatch. |
| Empty Library | Reject before a zone request or loss/status change. |
| Top-order mismatch | Reject if supplied object is not the exact ordered top. |
| Unsupported Library profile | Propagate S2 admission rejection. |
| Replacement/effect/modifier state | Reject unknown or nonempty effect/trigger state affecting Draw. |
| Ambiguous first-turn/game-start profile | Reject turn 1 and unclassified ingress/history. |
| Invalid pre-state | Propagate complete EngineState validation failure. |
| Object-ID exhaustion | Propagate S2 checked-allocation failure. |
| S2 rejection | Propagate the exact zone-incarnation rejection; no partial Draw event. |
| Already completed Draw | Held priority/post-action state rejects repeat execution. |
| Priority closure unavailable | Reject rather than commit a draw with no valid completion boundary. |

Every rejected case preserves state, RNG, IDs, knowledge, history, events,
revision, episode status, environment counters, replay, and projections.

## 13. Event, delta, and cursor contract

The S2 `ZoneTransition` remains the authoritative object-incarnation event,
with its `OLD -> NEW` identity, old snapshot, new snapshot, physical-card
continuity, and existing perspective lifecycle. The Draw producer does not
restate those facts in another zone event.

A distinct semantic `DrawCompleted` event is required because a Library-to-Hand
transition alone does not prove that the turn-based Draw action occurred; the
same zone move can be produced by other rules. It records the active player
and is paired by the cursor with exactly one admitted S2 transition in the
same atomic product. It contains no card identity in player-visible form.
Ordering is:

```text
ZoneTransition
DrawCompleted(active_player)
priority-window event owned by basic-priority
```

All authoritative events in this product carry the one next state revision
under the current event contract. The `StateDelta` replaces the complete
state and contains this semantic audit order; reapplying it must reconstruct
the exact after-state and digest. The cursor verifies Draw position, actor,
source/destination, one transition only, no RNG change, and priority
completion. A failure is an implementation defect and rejects the whole
product.

`DrawCompleted` is not a player-decision input. If reviewed semantics
determine that S2's zone transition plus the priority event already proves the
draw without adding a distinct semantic fact, that must be resolved before
implementation; code must not add a trace-only event.

## 14. Checkpoint, restore, and fork

The admitted atomic before/after states and their existing V5 checkpoints
contain position, priority, zone/object state, identity allocator, knowledge,
and perspective identity. Restoring the pending state permits exactly one
draw; restoring the completed state permits priority progression and cannot
draw again. Forks of either state must produce byte-identical products for
identical inputs, and diverging inputs remain isolated.

No new Draw progress field means no new state digest or wire field. The new
event and semantic transition meaning still require semantic contract/version
and event-schema review as required by current evolution policy. If the design
changes to add progress state, all state/digest/checkpoint/wire/schema
representations must be updated coherently before implementation.

## 15. Replay analysis (V5)

The legitimate path is an accepted real basic-priority response that ends the
Upkeep priority window and leaves forced Draw work next. The existing response
transaction then executes exactly one rules-owned forced-progress closure;
Draw, S2 zone incarnation, and the active-player priority request are merged
into the accepted response product. V5 records the actual accepted
`DecisionResponseV2` that initiated this causal chain. Replay re-executes that
response, re-runs forced consequences, validates the transition contract and
compares the after-state/checkpoint identity. The fresh `GameObjectId`, exact
zone order, knowledge, and event trace are derived from the restored complete
state and checked by the final digest and product validation.

The standalone forced-progress endpoint is not this path: it is setup-only,
has no replay step, and cannot support a game-state Draw replay claim. No fake
response, event-as-input, test-only replay, or Replay V6 is proposed.

This path depends on the basic-priority pass response producing the Draw
boundary without silently passing the Draw priority window. The final pass
must execute the Draw action and then install a real explicit pass-only
Decision for the active player. If implementation cannot fit that single
response-plus-forced-progress contract, replay support is `UNPROVEN` and must
not be manufactured.

## 16. S2 authoritative replay disposition

```text
CAN_DRAW_PRODUCE_AUTHORITATIVE_S2_REPLAY = YES
S2_CAN_BECOME_COVERED_DURING_S3 = YES (only after required evidence and review)
```

The existing production response transaction and backend-verified V5 replay
runner provide the legitimate initiating input and re-execution path described
above. The current repository has no Draw producer, so this is a reviewed
future integration path, not evidence already earned. S2 remains
`implemented / not covered`; it is not promoted by this design or by
deterministic rerun alone. S3 may discharge the outstanding S2 replay
obligation only with a real kernel witness, V5 backend replay, exact new
incarnation and after-identity validation, and independent S2 lifecycle
review.

## 17. Conformance witness matrix

| Witness | Executable proof |
| --- | --- |
| Normal admitted Draw | Active player draws exactly one exact Library top; one fresh incarnation; same physical identity; no RNG; no player decision. |
| Turn-structure × Draw | Normal Upkeep closure enters Draw; Draw is not temporally skipped; action occurs once; passes later advance to Precombat Main. |
| Draw × zone-incarnation | Draw intent delegates to S2 executor; exact ordered Library removal/Hand insertion, `ZoneTransition`, identity allocation, and delta. |
| Draw × information | Owner gets exact S2 private knowledge; opponent gets no prohibited IDs, mapping, definition, or card-bearing occurrence. |
| Paired hidden worlds | Different top identities yield byte-identical opponent-visible products and only authorized owner-private differences. |
| Exactly once across checkpoint | Restore pre-action state and draw once; restore post-action state and prove draw cannot repeat. |
| Fork parity | Identical forks match products, digest, replay and player bytes; independent subsequent inputs stay isolated. |
| V5 authoritative replay | Genuine final priority response re-executes Draw/S2 forced consequences and validates complete after identity. |
| Fail-closed matrix | Each rejection in Section 12 preserves every listed state and environment product. |

These witnesses close `turn-structure × draw-card` and
`draw-card × zone-incarnation` as interaction obligations; they do not create
separate pair-capabilities.

## 18. RED-test inventory

Before implementation, add failing tests for:

- Draw currently rejects at the S1 boundary; the S3 path requires the selected
  combined capability contract.
- Exact ordinary draw, Library ordering, Hand destination, fresh incarnation,
  physical continuity, and semantic event order.
- Draw-to-priority transition and exact-one invariant; no `Draw -> PrecombatMain`
  shortcut; no repeat draw after accepted completion.
- Priority `None`/`HeldBy` state admission and malformed-pair rejection.
- Owner identity and non-owner redaction; hidden-top paired-world byte parity.
- All atomic rejection cases in Section 12, including S2 error propagation and
  every allocator/RNG/counter/history surface.
- StateDelta reapplication; semantic cursor mutations and mutant products.
- Pending/completed checkpoint restore and fork parity.
- Real response-initiated V5 replay, including resulting object identity,
  complete after-state, observations, information, and events.
- No replay step for standalone forced progress and no fabricated Decision or
  response for Draw.
- Exact capability dependency closure and rejection when basic-priority or
  its dependencies are absent.

## 19. Lifecycle consequences

`rules/draw-card@0.1.0` remains `specified`. Design review alone does not make
it implemented, covered, or certified. S2 remains implemented and not covered
until its own replay obligation is evidenced and reviewed. No certification,
card, deck, format, Commander, or playability support is implied.

## 20. Unresolved blockers

1. The capability registry currently omits `rules/basic-priority` from Draw's
   dependencies. The required priority semantics and the priority capability's
   existing dependency closure must be reconciled through a separate reviewed
   registry/governance change; this design does not edit it.
2. No current priority event or semantic cursor arm proves `None` to
   `HeldBy(active_player)`; basic-priority must own and validate this transition.
3. The basic-priority closure currently depends on
   `state-based-actions-combat`, so exact S3 semantic scope/dependency closure
   must be selected and reviewed before implementation.
4. A semantic `DrawCompleted` event is proposed, but event audience, projection,
   and pairing with S2 must be resolved against the current event contract.
5. V5 Draw replay is a valid existing transaction pattern only if the real
   basic-priority response can initiate the complete forced consequence in
   one response transaction and produce the next explicit Decision.

## 21. Implementation-readiness verdict

The zone/incarnation and private-knowledge parts are bounded and reusable.
However, Draw alone is not an independently coherent S3 slice: it must finish
at an explicit priority window, and the repository does not yet have that
priority execution/event contract or a reviewed dependency closure. Do not
start implementation until those blockers are resolved and the S3 capability
bundle is independently selected and authorized.

## Required conclusions

```text
S3_CANDIDATE = rules/draw-card@0.1.0 / REJECTED (as a standalone slice)
DRAW_CARD_ALONE_FOR_S3 = INSUFFICIENT
CURRENT_STATE_DISTINGUISHES_PRE_DRAW_POST_DRAW = YES (position + PriorityState)
NEW_AUTHORITATIVE_DRAW_PROGRESS_STATE_REQUIRED = NO (atomic draw-to-priority)
BASIC_PRIORITY_REQUIRED_FOR_S3 = YES
DRAW_CARD_ZONE_INCARNATION_INTERACTION_CLOSABLE = YES
TURN_STRUCTURE_DRAW_CARD_INTERACTION_CLOSABLE = YES
CAN_DRAW_PRODUCE_AUTHORITATIVE_S2_REPLAY = YES (through the real response transaction)
S2_CAN_BECOME_COVERED_DURING_S3 = YES (conditional on required evidence/review)
DEPENDENCY_CHANGE_REQUIRED = YES
S3_IMPLEMENTATION_READY = NO
```
