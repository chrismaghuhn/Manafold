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

**Disposition:** retain `rules/draw-card@0.1.0` as an S3 interaction
participant, but it is insufficient by itself for end-to-end Draw-Step
closure. Foundation V2's capability DAG remains authoritative and unchanged:

```text
rules/draw-card
├── rules/turn-structure
└── rules/zone-incarnation

rules/basic-priority
├── rules/turn-structure
└── rules/state-based-actions-combat
    └── rules/zone-incarnation
```

The runtime must orchestrate the Draw action and subsequent priority window;
that cross-domain sequence does not add `basic-priority` as a semantic
dependency of `draw-card`. Draw must not copy or fork S2's zone-incarnation
operation. The registry is unchanged.

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
- The integrated progression leaves the turn position at Draw, performs the
  pre-priority SBA gate, and establishes the active player's priority window
  through the one authoritative basic-priority implementation. The next
  pass-only Decision is owned by basic-priority.

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

**State representation can distinguish pending from completed Draw:** YES.
The existing pair can represent:

```text
Beginning(Draw) + PriorityState::None
    = ordinary turn-based draw pending

Beginning(Draw) + PriorityState::HeldBy(active_player, 0)
    = turn-based draw complete; priority window active
```

The position alone is insufficient, but `PriorityState` is authoritative,
included in the full-state digest, and checkpointed. The current S1 executable
support profile rejects held priority, and no executable Draw/priority
contract currently establishes the pairing. The integrated transition
validator must establish it, reject malformed Draw states, and prove one
pending-to-completed transition. A repeated attempt in the completed state
must reject without another zone move.

**Current executable Draw/priority contract exists:** NO.

**New authoritative Draw progress field required:** NO, conditional on atomic
integrated closure: Draw action, pre-priority SBA gate, and priority
establishment. If an accepted state is exposed after the zone move while
priority remains `None`, a new typed authoritative progress field is required
and must be included in state validation, digest, checkpoint/restore/fork,
serialization, and replay identity. Such a split state is not admitted by
this design.

The current semantic cursor already snapshots priority but cannot apply a
priority transition event. Basic-priority must supply its canonical event and
cursor contract; Draw must not implement a parallel priority mutation.

## 8. Turn-structure interaction

Turn structure admits the normal Draw Step boundary and fixes its place after
Upkeep and before Precombat Main. The Draw producer handles the mandatory
turn-based action at that boundary. `temporal_successor(Draw)` is not called to
move directly to Precombat Main. Runtime orchestration then runs the selected
pre-priority SBA gate and basic-priority's explicit window/pass protocol.
Only explicit passes later advance the turn position.

Capability dependency and runtime orchestration are distinct. The S3
end-to-end sequence is:

```text
turn-priority-progression

Beginning(Draw) + Priority=None
    -> turn-step-action
    -> rules/draw-card
    -> rules/zone-incarnation
    -> priority-sba-gate
    -> rules/state-based-actions-combat
    -> rules/basic-priority
    -> Beginning(Draw) + Priority=HeldBy(active_player, 0)
```

The capability dependency closure of `basic-priority` remains the accepted
Foundation V2 closure: `basic-priority` depends on `turn-structure` and
`state-based-actions-combat`; `state-based-actions-combat` depends on
`zone-incarnation`. This does not add a `draw-card -> basic-priority` edge.
The likely ordered S3 closure for independent design/review is:

```text
S3.A  state-based-actions-combat × zone-incarnation
S3.B  turn-structure × basic-priority × state-based-actions-combat
S3.C  turn-structure × draw-card × zone-incarnation,
      integrated into the priority-bearing Draw-Step path
```

This is a design candidate only. End-to-end Draw/priority cannot omit the
selected pre-priority SBA gate.

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

`DRAW_COMPLETED_EVENT = PROPOSED / UNRESOLVED`.

Two reviewed outcomes remain open:

- If `ZoneTransition` plus the canonical SBA/priority event structure does not
  independently prove that the turn-based Draw action occurred, a typed Draw
  semantic event may be justified. It must pair the active player and exactly
  one admitted S2 transition, and must expose no card identity.
- If existing or newly required canonical semantic events already prove the
  Draw action without ambiguity, do not add a duplicate trace-only event.

If a typed Draw event is justified, its candidate ordering is:

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

Any Draw semantic event is not a player-decision input. Final event authority,
ordering, cursor pairing, and projection remain unresolved for the S3 execution
design; code must not add a trace-only event.

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

The architectural path is a real accepted basic-priority pass response that
ends the Upkeep priority window and initiates the Draw-Step progression. The
response transaction can then execute rules-owned forced consequences; a
complete future implementation could merge Draw, S2 zone incarnation, the
pre-priority SBA gate, and the active-player priority request into the
accepted response product. V5 would record the actual accepted
`DecisionResponseV2` that initiated this causal chain. Backend replay would
re-execute that response and consequences, validate the transition contract,
and compare after-state/checkpoint identity. The fresh `GameObjectId`, exact
zone order, knowledge, and event trace would be derived from restored complete
state and checked by the final digest and product validation.

This is not current evidence: no real Basic-Priority response currently
produces Draw and the S2 Library-to-Hand transition. The current production
forced-progress path rejects at Draw, and its response transaction executes
at most one forced-progress closure. The integrated production path and its
backend replay must exist and execute before authoritative S2 replay can be
claimed.

The standalone forced-progress endpoint is not this path: it is setup-only,
has no replay step, and cannot support a game-state Draw replay claim. No fake
response, event-as-input, test-only replay, or Replay V6 is proposed.

This path depends on the basic-priority pass response producing the Draw
boundary without silently passing the Draw priority window. The Draw action,
SBA check, and establishment of the priority window must lead to a real
explicit pass-only Decision for the active player. If implementation cannot
fit that response-plus-forced-consequence contract, replay support remains
unproven and must not be manufactured.

## 16. S2 authoritative replay disposition

```text
REPLAY_ARCHITECTURE_PATH_IDENTIFIED = YES
CAN_DRAW_PRODUCE_AUTHORITATIVE_S2_REPLAY = UNPROVEN
S2_CAN_BECOME_COVERED_DURING_S3 = UNPROVEN
```

The response-initiated V5 path appears architecturally available, but becomes
evidence only after Basic Priority, Draw, and forced progression exist in
production and backend replay actually reproduces and validates the S2
transition. S2 remains `implemented / not covered`; this design does not
promote it.

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

These are proposed witnesses for `turn-structure × draw-card` and
`draw-card × zone-incarnation` interaction obligations. No interaction is
marked satisfied by this design, and the obligations do not create separate
pair-capabilities.

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
- Exact capability DAG conformance (Draw retains only its two Foundation V2
  dependencies) and runtime orchestration through the SBA/priority closure.

## 19. Lifecycle consequences

The lifecycle remains unchanged: `rules/draw-card` is `specified`,
`rules/basic-priority` is `specified`,
`rules/state-based-actions-combat` is `specified`,
`rules/zone-incarnation` is `implemented`, and `rules/turn-structure` is
`covered`. Design review alone advances none of them. No interaction is newly
claimed satisfied. S2 remains not covered until its replay obligation is
evidenced and reviewed. No certification, card, deck, format, Commander, or
playability support is implied.

## 20. Unresolved blockers

1. No current priority event or semantic cursor arm proves `None` to
   `HeldBy(active_player)`; basic-priority must own and validate this transition.
2. The accepted basic-priority dependency closure includes
   `state-based-actions-combat`, whose closure includes zone-incarnation. The
   likely S3.A/S3.B/S3.C order and orchestration must be selected and reviewed;
   this does not change Draw's capability dependencies.
3. `DrawCompleted` remains proposed/unresolved pending analysis of the
   canonical event/cursor proof; no trace-only event may be added.
4. V5 Draw replay remains unproven until a real response produces and backend
   replay validates the complete forced consequence.

## 21. Implementation-readiness verdict

The zone/incarnation and private-knowledge parts are bounded and reusable.
Draw's Foundation V2 dependencies are correct and need no change. Draw alone
is insufficient for end-to-end Draw-Step closure: runtime orchestration must
run the selected pre-priority SBA gate and basic-priority window. The
repository lacks that executable priority/event contract and has no replay
witness for Draw. Do not start implementation until the ordered S3 interaction
closure is selected, its event contract reviewed, and implementation is
independently authorized.

## Required conclusions

```text
S3_CANDIDATE = rules/draw-card@0.1.0 remains a selected S3 interaction participant; not sufficient by itself for end-to-end Draw-Step closure
DRAW_CARD_ALONE_FOR_S3 = INSUFFICIENT
DRAW_CARD_DEPENDENCIES = rules/turn-structure; rules/zone-incarnation
STATE_REPRESENTATION_CAN_DISTINGUISH_PRE_POST_DRAW = YES
CURRENT_EXECUTABLE_DRAW_PRIORITY_CONTRACT_EXISTS = NO
NEW_AUTHORITATIVE_DRAW_PROGRESS_STATE_REQUIRED = NO, conditional on atomic integrated closure
BASIC_PRIORITY_REQUIRED_FOR_END_TO_END_S3 = YES
DRAW_DEPENDS_ON_BASIC_PRIORITY = NO
DRAW_CARD_ZONE_INCARNATION_INTERACTION_CLOSABLE = YES
TURN_STRUCTURE_DRAW_CARD_INTERACTION_CLOSABLE = YES
REPLAY_ARCHITECTURE_PATH_IDENTIFIED = YES
CAN_DRAW_PRODUCE_AUTHORITATIVE_S2_REPLAY = UNPROVEN
S2_CAN_BECOME_COVERED_DURING_S3 = UNPROVEN
DEPENDENCY_CHANGE_REQUIRED = NO
DRAW_COMPLETED_EVENT = PROPOSED / UNRESOLVED
S3_IMPLEMENTATION_READY = NO
```
