# M3.S3 End-to-End Interaction Implementation Plan

**Status:** executable implementation plan; implementation not authorized by this plan
**Date:** 2026-09-23
**Branch:** `chris/m3-s3-draw-card-design`
**Required base:** `b67cfdcc0a8e623da52a889ef2ae138a3e4256ac`
**Reviewed design:** [M3.S3 Draw-card interaction design](../specs/2026-09-23-m3-s3-draw-card-design.md)
**State identity cut:** [M3.S3.P0 design](../specs/2026-09-23-m3-s3-state-identity-cut-design.md)
**Capability lifecycles changed by planning:** NO

This plan decomposes the selected S3 architecture into reviewable, sequential
implementation tasks. It does not itself authorize production changes, create
a support claim, or promote a lifecycle.

## 1. Exact-head preconditions and current state

The remote was fetched before repository-specific claims:

```text
REMOTE = https://github.com/chrismaghuhn/Manafold
REQUIRED_ORIGIN_MASTER = b67cfdcc0a8e623da52a889ef2ae138a3e4256ac
VERIFIED_ORIGIN_MASTER = b67cfdcc0a8e623da52a889ef2ae138a3e4256ac
REQUIRED_BRANCH_HEAD = 35968fe64b4a439aaef2cb7a2f8755175431be56
VERIFIED_BRANCH_HEAD = 35968fe64b4a439aaef2cb7a2f8755175431be56
```

Current lifecycle, preserved throughout this plan until a later implementation
task produces and reviews the named evidence:

```text
rules/turn-structure = covered
rules/zone-incarnation = implemented / not covered
rules/state-based-actions-combat = specified
rules/basic-priority = specified
rules/draw-card = specified
```

The design's current architecture is binding for this plan:

```text
Capability dependencies:
  draw-card -> turn-structure, zone-incarnation
  basic-priority -> turn-structure, state-based-actions-combat
  state-based-actions-combat -> zone-incarnation

Runtime orchestration:
  turn-step-action
  -> draw-card
  -> zone-incarnation
  -> priority-sba-gate
  -> state-based-actions-combat
  -> basic-priority
```

There is no `draw-card -> basic-priority` capability edge. `damage-and-life`
is not an SBA dependency: SBA consumes authoritative life and marked-damage
facts regardless of which capability produced them.

## 2. S3.P0 authoritative state identity cut

The S3.A APNAP ordering continuation is authoritative state. Its continuation
payload cannot be added to `full-state-digest-input.v4` or interpreted by
`EnvironmentCheckpointV5` / Replay V5. The coordinated identity cut is
mandatory:

```text
FullStateDigestV5
full-state-digest-input.v5
mtgml.full-state-digest.v5

EnvironmentCheckpointV6
environment-checkpoint.v6

CheckpointDigestV6
environment-checkpoint-digest-input.v6
mtgml.checkpoint-digest.v6

ReplayManifestV6 / ReplayStepV6 / AuthoritativeReplayV6
replay-manifest.v6 / replay-step.v6 / authoritative-replay.v6
ReplayRecorderV6 / ReplaySchemaVersionsV6 / InitialEnvironmentIdentityV6
```

S3.P0 also adds the closed typed Magic continuation payload representation
that S3.A will later produce/consume. It performs no SBA derivation, Decision
generation, APNAP collection, zone movement, or lifecycle promotion. Until
S3.A is implemented, restore admission must reject a semantic contract that
claims the Magic SBA program is executable.

`FullStateDigestV4`, V5 checkpoint and V5 replay remain exact historical
meanings with no reinterpretation or automatic migration. V4 digest known
answers remain byte-identical under a detached verifier. Current writers move
to V5 state digest and V6 checkpoint/replay; ReplayStep V6 still contains one
real `DecisionResponseV2`, including each staged Order response. No forced-
progress input, fake response or Replay V7 is introduced.

S3.P0 must also preserve existing `synthetic-m3-observation.v1` bytes and
bind `magic-m3-observation.v1` as the APNAP progress codec in V6 manifests.
`ObservationEnvelopeV1` and `InformationStateDigestV2` remain unchanged; the
observation envelope already hashes a named payload codec. The exact state
cut, detached identity mappings, V4/V5 compatibility table and P0 gates are
specified in the linked S3.P0 design.

## 3. Frozen S3 order

The new authoritative Magic SBA-order continuation requires an explicit
identity cut before any shared response-transaction or S3.A implementation.
S3.P0 is state/schema/checkpoint/replay infrastructure only. It changes no
Magic rule or capability lifecycle. The previously reviewed shared
response-commit extraction remains the semantic-neutral S3.0 prerequisite
after S3.P0.

```text
S3.P0 authoritative state/checkpoint/replay identity cut

S3.0 shared response-commit primitive (semantic-neutral prerequisite)

S3.A rules/state-based-actions-combat
    × rules/zone-incarnation

S3.B rules/basic-priority
    × rules/turn-structure
    × rules/state-based-actions-combat

S3.C rules/draw-card
    × rules/turn-structure
    × rules/zone-incarnation

S3.D integrated replay witness:
    Upkeep explicit passes
    -> Draw
    -> zone incarnation
    -> selected SBA fixed point
    -> Draw priority
    -> backend-verified Replay V6
```

S3.A comes first because Basic Priority's accepted dependency requires SBA and
because SBA × zone-incarnation is one outstanding S2 producer interaction.
S3.B comes next because Draw must enter a real priority window through the
shared priority owner, after SBA. S3.C then adds the normal turn-based Draw
producer using the already implemented S2 executor. S3.D is integration and
evidence, not a new capability.

The order is not a coding convenience and must not be inverted. Each PR must
leave `master` buildable and the declared lifecycle truthful.

## 4. S3.A exact bounded SBA scope

Implement only the Foundation V2 selected actions over a validated two-player,
`FormatState::None`, inert-card profile with no unsupported layers, effects,
triggers, replacement/prevention, or unsupported permanent families:

1. Every player at life `<= 0` loses in that SBA check.
2. A selected creature with derived simple toughness `<= 0` is put into its
   owner's Graveyard.
3. A selected creature with lethal marked damage is destroyed and put into
   its owner's Graveyard.
4. All actions applicable to one check are derived from the same immutable
   round-start state and applied simultaneously as one rules workspace result.
5. Recompute from the resulting workspace until the derived action set is
   empty. Do not return priority between rounds.

For the current S3.A profile the creature-removal predicates are mutually
exclusive for one immutable SBA round-start state:

- derived toughness `<= 0` produces exactly `[ZeroToughness]`;
- derived toughness `> 0` with marked damage `>= toughness` produces exactly
  `[LethalDamage]`.

An object action therefore carries exactly one of these causes. CR 704.5f and
704.5g cannot both apply to the same creature in one check. Each selected
object still produces one S2 zone transition, one fresh `GameObjectId`, and
one OLD-reference closure; never attempt two incarnation transitions for the
same OLD.

Terminal status follows Foundation V2 exactly:

```text
one losing player:
  that player's has_lost = true
  Terminal(reason = rules_loss, loser = Loss, other = Win)

both players lose in the same SBA check:
  both has_lost = true
  Terminal(reason = simultaneous_outcome, both = Draw)
```

Player outcome arrays are canonical ascending `PlayerId`. No priority or
gameplay Decision follows a terminal result. A creature zone transition in
the same SBA check remains in the complete event/delta product before the
environment commits the terminal state.

Same-owner multi-object Graveyard order is a required player choice under the
pinned CR 404.3, not an implementation sort and not a fail-closed cardinality
exception. Foundation V2's selected simultaneous SBA scope remains unchanged.
Each owner receiving two or more cards in the same Graveyard in one SBA round
must explicitly order those cards before any action in that round is applied.
Use the already accepted Comprehensive Rules snapshot
`wotc-cr-2026-08-07-txt-20260819-sha256-4381ad1b39ab2c05f7d03633a20f711ed37277074d3266dcba5f38cbb527423f`
for CR 404.3 and the player-choice/APNAP rule. The implementation contract and
conformance references must pin the exact applicable APNAP paragraph from
that snapshot; no second authority snapshot is introduced.

The Decision uses existing public Decision V2 shapes:

```text
DecisionDomainV2::Order { minimum: N, maximum: N }
DecisionAnswerV2::Order { candidate_ids: complete permutation }
DecisionVisibility::ActingPlayerOnly; actor = current APNAP owner
CandidateIntent::SelectObject { object: owner-authorized OpaqueObjectId }
EngineCandidateBinding::SelectObject { object: trusted GameObjectId }
```

The candidate array contains exactly the N selected cards, uses authorized
owner-side opaque identities, is canonical/dense under
`CandidateOrderingV1`, and the answer is an exact permutation. No new public
Decision family or implicit GameObjectId/PhysicalCardId ordering is added.
Every offered card must already have an authorized owner-side opaque mapping;
otherwise the kernel fails closed before creating the continuation or
Decision. The first object in the accepted order is topmost in the new
Graveyard group; all remaining selected objects follow it and precede every
pre-round Graveyard member.

If both players need to order cards in the same SBA round, collect required
orders in APNAP order: active player first, then nonactive player. Skip an
owner who has fewer than two cards in that Graveyard. No zone, `has_lost`,
status, or other SBA action from the round mutates until every required order
has been received. `PriorityState` remains `None`; no priority or forced
advance is offered between ordering stages. Rejection at any stage preserves
the complete continuation/request/environment/player fingerprint.

### Magic SBA ordering continuation

The current `ContinuationPayloadV2` contains only
`SyntheticM2Assembly`. Add a distinct typed Magic-specific variant for this
SBA ordering program; do not overload, reinterpret, or insert Magic values
into the Synthetic payload. One active round continuation stores the exact
authoritative information needed to resume without event-history or
controller-local state:

```text
SbaRoundPlan:
  round_start_revision
  complete canonical selected action set and causes
  order_owners in APNAP order
  next_owner_index
  completed owner -> exact ordered GameObjectId permutation
```

These are semantic requirements, not frozen Rust field names. Each resume
re-derives the complete SBA action set from the unchanged authoritative game
facts and requires exact equality with the saved plan before accepting an
order. Prior staged responses may change revision, pending decision,
continuation data and allocators; they may not change life, zones, source
facts, priority or status. Do not persist cached characteristics or an
unverifiable full-state snapshot.

One trusted `ContinuationId` persists across all APNAP Order stages. Each
stage receives a fresh checked `DecisionId` and actor-local
`PlayerDecisionIdV1`. The existing continuation record's actor coherence
becomes payload-specific: Synthetic stages preserve their current same-actor
rule; the Magic payload identifies the expected APNAP owner for the current
stage, and record actor/pending-request actor must match it. On stage transfer,
update the current-stage actor while preserving ContinuationId and creation
revision. A pending continuation always has a pending Decision; no accepted
continuation-only checkpoint is permitted.

Stage behavior:

```text
no owner needs Order (Task 7):
  derive complete round -> do not apply it yet; Task 9 owns application

one owner needs Order (Task 7):
  save complete round -> owner Order Decision
  final response remains unaccepted until Task 9 can apply the whole round

both owners need Order (Task 7):
  save complete round -> active-owner Order Decision
  accepted response -> store active order; same ContinuationId;
  -> fresh nonactive-owner Order Decision, no SBA mutation
  final response remains unaccepted until Task 9 can apply the whole round
```

Task 7 accepts only nonfinal APNAP responses, which commit the new stage and
next Decision without applying SBA actions. A one-owner response or the last
owner's response is not accepted as an order-only state: Task 9 must validate
that answer and apply the complete SBA batch/fixed point atomically in the
same rules transition. A round requiring no Order is likewise left unapplied
until Task 9. This prevents a final-choice checkpoint from existing without
its consequences.

Exact audit ordering for a staged owner response is:

```text
DecisionCleared(current Order Decision)
SbaGraveyardOrderChosen(current owner, exact top-to-bottom order)
if another owner remains: DecisionCreated(next APNAP Order Decision)
otherwise: StateBasedActionsApplied(complete saved action set)
          ZoneTransition(s) for the complete batch
          subsequent fixed-point event round(s), if any
          terminal outcome OR priority event + next Decision
```

The initial no-choice-to-order continuation transition creates the first
`DecisionCreated` event and stores the typed round plan in the complete state;
it emits no applied-SBA event because no SBA action has happened yet. Every
event sequence is checked by the semantic cursor and complete transition
contract at its own revision.

The selected semantic audit adds a typed `SbaGraveyardOrderChosen` event and
matching `SemanticDeltaOperation` for each accepted owner order. It records
the trusted continuation, owner and exact permutation and advances the
cursor's typed continuation state. The bound Order Decision is projected
only to its actor.

Continuation validation proves the current pending request maps to the
payload's next APNAP owner; the candidate set is exactly that owner's SBA
objects; each prior order is an exact permutation; no order is duplicated or
missing; and no SBA action has already been applied. On final completion,
remove the continuation in the same transition that applies the complete
round. Checkpoint/restore, fork and replay resume from this payload alone.

### Public APNAP choice visibility

CR 101.4b means a later player making an APNAP choice knows earlier players'
choices. After the active owner submits an order, the nonactive owner must
therefore see that exact arrangement before answering their own Order
Decision. `DecisionVisibility::ActingPlayerOnly` still limits each request
and candidate list to its actor; actor-only request visibility alone does not
satisfy this rule.

Project persisted order progress as public **current observation state** in a
distinct `magic-m3-observation.v1` payload codec carried by the existing
`ObservationEnvelopeV1` payload surface:

```text
pending_sba_ordering = null
  OR {
    completed_orders: [
      { owner: PlayerId, ordered_objects: [perspective-local OpaqueObjectId] }
    ],
    next_order_owner: PlayerId
  }
```

Each perspective resolves the same trusted `GameObjectId` permutation through
its own authorized opaque mapping. No observation contains GameObjectId,
PhysicalCardId, CardDefinitionId, ContinuationId, allocator state or another
actor's candidate binding. The payload is projected read-only from the
authoritative continuation; the continuation itself is never exposed. After
the complete SBA batch, `pending_sba_ordering` clears and the final public
Graveyard state carries the selected order.

This is a public current-state projection, not an event-history cache or a
second authority. It consumes no `VisibleSequence`: the typed
`SbaGraveyardOrderChosen` rule event remains the replay/audit authority, and
no `ObservedEventEnvelopeV2` is fabricated for an incomplete round. The
active submitter's `PlayerStepV2` includes the updated current observation;
the nonactive endpoint sees the same public order progress on its next
observation read, before its own Decision.

`synthetic-m3-observation.v1` bytes must remain unchanged. The Magic rules
contract binds `magic-m3-observation.v1` in `ReplayManifestV6`; V6 manifest
validation/schema expands its allowed payload codecs by bound semantic
contract. ObservationEnvelope V1, InformationState V2, PlayerStep V2,
Decision V2, ObservedEventEnvelope V2 and PlayerStep V2 shapes remain
unchanged. Update the Magic payload schema/Python projection, V6 manifest
schema/validator, and golden/negative fixtures. If codec binding cannot be
expressed without reinterpreting an existing codec or widening an
unauthorized player surface, stop before Order implementation.

#### Digest, checkpoint and V6 compatibility

The Magic payload/order state is authoritative EngineState and therefore must
be validated and included in canonical state identity. Extend the existing
`execution_v2.continuations[].continuation_payload` mapping with a unique
`magic_sba_graveyard_order` tag and canonical payload layout. Do not reuse a
tag. Existing `SyntheticM2Assembly` V4 historical bytes/golden vectors must
remain byte-identical. Update `STATE_HASHING.md`, the new V5 canonical codec,
reader/writer fixtures, state schemas/adapters if applicable, and
Rust/Python/digest parity evidence from the authoritative source.

`EnvironmentCheckpointV6` and `ReplayStepV6` are required: V6 binds
FullStateDigestV5 plus the new semantic contract identity, and each Order
answer is a real DecisionResponseV2 replay input. Historical V4/V5 artifacts
keep their exact old contract IDs and digest bytes; they are never
reinterpreted under this S3 closure. No automatic V5-to-V6 migration exists.

### S3.A event and delta shape

The action batch is rule-relevant semantic meaning, not tracing. Add one typed
rules event for each nonempty fixed-point round, conceptually:

```text
StateBasedActionsApplied {
  actions: canonical complete action set derived from the round-start state
}
```

Its closed action variants carry trusted player/object identity and the
selected cause (`player loses`, `zero toughness`, or `lethal marked damage`).
Its matching `SemanticDeltaOperation` preserves the
complete action batch. `SbaGraveyardOrderChosen` events and the typed
continuation preserve each player's accepted permutation. Emit the existing
S2 `ZoneTransition` once per creature action as one atomic rules workspace
product; all events carry the candidate revision. Event serialization order
is deterministic audit order only; it is not authority for Graveyard order.
The completed continuation choices are the sole authority for destination
arrangement. Terminal `EpisodeStatus` is the environment result paired with
the `player loses` actions; do not add a string-coded `PublicOutcome` as a
second authority.

The selected order is top-to-bottom: the first object in the owner's
`DecisionAnswerV2::Order` is topmost in the new Graveyard group. Existing
pre-round Graveyard members remain below the entire group and retain relative
order. Keep `rules/zone-incarnation@0.1.0` as the sole one-object OLD→NEW
transition authority, unchanged in semantic meaning. The S3.A rules-owned
round coordinator composes that executor on one uncommitted workspace after
all required orders are collected. Derive and validate the complete SBA
action/source set from the same round-start state before the first call. For
each owner, invoke the S2 top-inserting transition in reverse selected order
so the final owner vector realizes the accepted top-to-bottom permutation.
Process owner groups in fixed APNAP order only for deterministic allocation
and event audit. The group is one SBA transition and one environment commit;
no intermediate workspace is a supported S2 checkpoint or lifecycle state.
This composition does not change S2's declared single-transition semantics or
version. If the existing executor cannot compose safely without a second
mutation implementation or a semantic change to S2, stop and request the
required governance/version decision before coding.

`SemanticValidationCursor` must derive the complete applicable set from its
round-start facts and compare it exactly with the batch event before applying
any member. It then verifies one exact S2 transition per creature action,
canonical cause/move pairing, no extra or omitted moves, and fixed-point
re-evaluation until stable. The transition contract independently checks
after-state parity, terminal mapping, revision/event order, and delta
reapplication. Every OLD reference except the processed S2 source and
explicitly permitted perspective lifecycle remap is rejected.

The new typed event and delta variants must be added at the authoritative
source. Audit whether they affect schemas, golden/negative fixtures, generated
contract vocabulary, semantic-contract material, Rust/Python parity, and
conformance vocabulary; update all affected representations and run the
repository generator. Never hand-edit generated output.

## 5. S3.B exact Basic Priority scope

Support only the validated pass-only two-player profile. Before every window:

1. The `priority-sba-gate` derives the complete S3.A round.
2. If one or more owners require Graveyard ordering, create/resume the typed
   Magic SBA continuation and collect all required APNAP Order decisions
   before applying any SBA action. While this continuation is active, no
   priority Decision may exist or be offered.
3. Once the SBA fixed point is stable, if terminal, stop without priority.
4. Derive and validate `PASS_ONLY_PRIORITY_PRECONDITION` from complete
   authoritative state: empty stack, no waiting/delayed triggers, effects,
   replacement/prevention, continuous-effect, mana, special-action,
   land-play, or action-bearing ability surface; inert card objects; no
   turn-based action still pending.
5. If any fact is absent or non-pass action remains unproven, reject before
   creating a Decision.
6. Otherwise give the active player priority and create one explicit pass
   Decision.

The public Decision reuses the existing V2 contract:

```text
domain = ChooseOne
candidates = [candidate_id 0, CandidateIntent::PassPriority]
trusted binding = EngineCandidateBinding::PassPriority
answer = DecisionAnswerV2::SelectOne { candidate_id: 0 }
```

No new Decision domain, public response schema, `Confirm`, hidden action, or
trusted ID is exposed. `DecisionId` is global trusted state; the visible
`PlayerDecisionIdV1` is allocated from the actor's perspective-local identity
state; candidate ID is request-local dense zero. Actor must match
`PriorityState::HeldBy.player`; response revision and visible decision ID
must match the pending request. Existing state-owned pending-request binding
validation, `CandidateOrderingV1`, and Layers A/B response validation remain
the single authorities. The actor's endpoint is the only endpoint that
receives the request.

### Priority event and ordering contract

Add one typed `PriorityChanged { from, to }`-equivalent event, its
`SemanticDeltaOperation`, cursor arm, and transition-contract proof. The event
must prove the entire `PriorityState` value, including holder and pass count;
no unexplained direct `core.priority` mutation is allowed.

Canonical product ordering:

```text
Open a priority window:
  all completed SBA events and zone consequences
  PriorityChanged(None -> HeldBy(active, 0))
  DecisionCreated(new pass Decision)

First explicit pass:
  DecisionCleared(old Decision)
  PriorityChanged(HeldBy(active, 0) -> HeldBy(opponent, 1))
  DecisionCreated(next pass Decision)

Second explicit pass:
  DecisionCleared(old Decision)
  PriorityChanged(HeldBy(opponent, 1) -> None)
  TurnPositionChanged(current boundary -> temporal successor)
  then the shared response transaction may run one kernel forced advance
```

The cursor and state validator require pending pass Decision actor/domain /
candidate / trusted binding to agree with the held player and pass count.
`PriorityState::None` has no pending Basic Priority Decision. A terminal SBA
product clears no nonexistent Decision and creates no later priority event.
An active Magic SBA ordering continuation instead has `PriorityState::None`
and an `Order` Decision for its APNAP stage actor; it cannot coexist with a
pass-priority Decision.

## 6. S3.C exact Draw scope and event disposition

Use the accepted S3 design profile without widening:

- Ordinary Draw Step only, normal turn number `>= 2`, active player and owner
  of a nonempty own ordered Library.
- Consume the exact Library top once; move it through the existing S2
  Library-to-same-owner-Hand executor; preserve physical identity while using
  one fresh object incarnation.
- No RNG and no player Decision for Draw.
- Reuse S2 owner-private knowledge and opponent noninterference behavior.
- Reject turn 1/game start/mulligans, empty Library, replacement/prevention,
  multiple or modified draws, draw outside Draw Step, extra/skipped steps,
  triggers, and any unsupported library/effect profile.

Draw's exactly-once accepted states are:

```text
PRE:  Beginning(Draw), Priority=None, no pending Decision
POST: Beginning(Draw), Priority=HeldBy(active, 0), pending pass Decision
```

The whole transition includes Draw, S2 incarnation, pre-priority SBA fixed
point, and Basic Priority establishment. No accepted checkpoint may expose a
moved card with Draw position and `Priority=None`. Existing state and priority
are sufficient; no progress field is added.

**Draw event decision:** `DrawCompleted` is not required for this closed
profile. Before context (`Beginning(Draw)`, `Priority=None`, no Decision), the
single exact top Library-to-Hand `ZoneTransition`, and after context
(`Beginning(Draw)`, `HeldBy(active, 0)`, one bound pass Decision) jointly
prove the ordinary turn-based action. In this closure, no other supported
producer may request this Library-to-Hand family. The transition cursor must
require exactly one such S2 transition and the priority-state/Decision pair.
Do not add a trace-only Draw event. If a later admitted Library-to-Hand
producer makes this proof ambiguous, that is a separately reviewed semantic
version/event decision, not a reason to add a duplicate now.

## 7. Environment response transaction and one forced advance

Current seam at the required head:

```text
SyntheticM1EnvironmentBackend::execute_response
  owns response + forced consequence + current-head V5 append + atomic commit

ReferenceEnvironmentBackend::execute_trusted_response
  calls kernel.apply only; does not commit or append replay

ReferenceEnvironmentBackend::submit_player_response
  returns UnavailableDecision while Running, otherwise EpisodeClosed
```

Task S3.0 extracts one environment-owned response transaction primitive and
moves the existing Synthetic backend onto it. The primitive is designed for
later Reference use, but S3.0 does not activate Reference Magic submissions.
At this plan's base, `MagicRulesKernel::apply` rejects every
`DecisionResponseV2` with `UnsupportedPlayerResponse`; the Reference endpoint
therefore remains unavailable until S3.B implements Basic Priority. The
shared transaction accepts only responses the supplied kernel accepts and
must preserve this rejection. Preserve ADR-0040 order exactly:

1. Capture and validate the complete before `EnvironmentCheckpointV6`.
2. Validate endpoint/player/episode/pending actor and response layers; call
   `kernel.apply` with immutable before-state semantics.
3. If accepted, no Decision exists, and status is Running, call
   `kernel.advance_forced_progress` exactly once. Merge its complete event
   sequence and recompute one before-to-final `StateDelta` over the whole
   response transaction.
4. Validate the complete transition contract.
5. On semantic rejection, prove checkpoint, status, every counter, replay and
   projected player state equal their before values; commit nothing.
6. Calculate candidate counters and construct/validate candidate V6 checkpoint.
7. Append exactly one `ReplayStepV6` for the real input response and validate
   the candidate replay.
8. Project all perspective occurrence envelopes and validate candidate player
   products.
9. Run a fallible before-commit hook for the submitting `PlayerStepV2` /
   candidate projection.
10. Atomically replace state, status, counters, replay, and required cache-free
    derived products only after every prior operation succeeds.

No Magic semantics move into the environment. The environment does not loop
or schedule another forced advance. One kernel advance is sufficient only
because the rules kernel itself resolves every deterministic mandatory action
until the next real Decision, terminal outcome, or typed unsupported/error
boundary. For this Draw path one advance must include Draw, S2, SBA fixed
point, and the next Decision. If the selected SBA round requires ordering,
the one advance returns its first APNAP Order Decision with the complete
typed continuation and no round mutation. Each genuine Order response passes
through the same shared transaction. An intermediate APNAP response returns
the next Order Decision with no SBA action applied; the final APNAP response
applies the complete round, repeats the fixed point, and returns at the next
Decision/outcome/error. If the kernel cannot guarantee this with one progress
advance plus explicit Order responses, stop; do not add an arbitrary
environment loop.

In S3.0, only Synthetic executes accepted responses through the shared
primitive. Reference may receive unobservable internal plumbing only if
`player_visible_decision` and `submit_player_response` retain their current
unavailable behavior and `MagicRulesKernel::apply` continues to reject every
response. Do not make the Reference endpoint invoke the transaction to
manufacture acceptance. S3.B later owns the actor-authorized pass Decision,
Magic response acceptance, Reference submission and replay use. Public callers
never use `execute_trusted_response`. Synthetic behavior and byte products
must remain unchanged under semantic-neutral S3.0.

## 8. Replay and evidence contract

Replay is V6 after S3.P0. A forced consequence has no replay step and no input DTO.
The real response that causes the consequence owns it. The end-to-end witness
is:

The replay segment starts from a validated turn-`>= 2` checkpoint already at
Upkeep with the active player's pass Decision installed. This is an admitted
resume/test state, not game-start or first-turn implementation evidence.

```text
before: Beginning(Upkeep), HeldBy(active, 0), pass Decision for active

ReplayStep N:
  active explicitly passes
  after: HeldBy(opponent, 1), pass Decision for opponent

ReplayStep N+1:
  opponent explicitly passes
  kernel.apply clears the Decision, closes Upkeep, and enters Draw
  shared response transaction calls advance_forced_progress once
  kernel draws the exact Library top through S2
  kernel runs SBA to a stable result
  kernel establishes HeldBy(active, 0) and creates active pass Decision
  V6 records the actual opponent pass response in ReplayStep N+1
```

The primary S2 Library-to-Hand witness uses a no-SBA-action Draw fixture; its
real opponent pass step re-executes the Draw/S2 transition and ends at active
priority. A separate simultaneous-SBA ordering witness extends the chain:

```text
real pass response
  -> forced progress derives complete SBA round
  -> APNAP Order Decision for first required owner
ReplayStep N+2 = that owner's real Order response
  -> next APNAP Order Decision, if another owner needs one
ReplayStep N+3 = next owner's real Order response
  -> complete ordered-group S2 transitions + SBA fixed point
  -> next priority Decision/outcome/error
```

Each Order response is a genuine V6 step. The last response re-executes the
same saved typed round plan and applies the complete round atomically. Replay
and checkpoint/restore are tested before the first Order and between APNAP
stages; no history or controller-local buffer resumes the choice.

Backend replay from the initial V6 checkpoint must re-execute both real pass
responses and prove identical before identity, actor/request binding, capability
closure, allocator-derived fresh `GameObjectId`, zone/order, S2 knowledge and
perspective identity mutations, authoritative events, delta, full-state digest,
status, counters, after checkpoint identity, next Decision, and all player
products. It must include no forced-work replay step, fake response, or V7.

Checkpoint/restore and fork tests cover both pending and completed Draw state.
Paired hidden worlds vary top-card `CardDefinitionId` and
`PhysicalCardId` while keeping public facts equal. Opponent observation,
information, Decision, PlayerStep, observed events, sequence behavior, and
rejection class remain identical. Owner differences are exactly the S2
authorized private identity/knowledge differences. Priority DTOs never reveal
Draw identity.

## 9. Lifecycle and interaction gates

Implementation does not imply coverage:

```text
S3.A passes implementation gates -> state-based-actions-combat: specified -> implemented
S3.B passes implementation gates -> basic-priority: specified -> implemented
S3.C passes implementation gates -> draw-card: specified -> implemented
```

Those implementation promotions occur in the corresponding PR after exact
evidence, with registry, roadmap, README, and current-status tests updated in
the same PR. They do not certify any capability. The current S2 lifecycle
remains `implemented / not covered` until a distinct later S2 coverage review.

The candidate S2 interaction evidence is:

```text
state-based-actions-combat × zone-incarnation:
  real derived simultaneous creature SBA -> exact S2
  Battlefield-to-owner-Graveyard transition(s), typed APNAP Order choices for
  each owner with multiple incoming cards, complete event/delta/cursor, fresh
  identities, OLD-reference closure, and rejection nonmutation

draw-card × zone-incarnation:
  real mandatory Draw -> exact S2 Library-top-to-owner-Hand transition,
  correct private knowledge and opponent parity
```

Mark neither satisfied until executable witnesses pass and an independent
review accepts them. S2 covered promotion additionally requires backend-
verified authoritative V6 replay of a real S2 transition and every other S2
coverage gate in its accepted design. Promotion is a distinct governance task;
deterministic rerun alone is insufficient.

## 10. Sequential PR strategy

Use six sequential PRs, each based on the newly merged `master` head:

| PR | Tasks | Scope and merge exit |
| --- | --- | --- |
| A — S3.P0 state identity cut | 1–2 | New state/checkpoint/replay identity family, closed Magic continuation representation, old V4/V5 compatibility proof. No semantic capability lifecycle change. |
| B — S3.0 shared response transaction | 3–4 | Semantic-neutral shared transaction used by Synthetic; parity with existing Synthetic behavior; Reference remains unavailable for accepted Magic responses. No lifecycle change. |
| C — S3.A ordered SBA | 5–10 | RED, continuation program, APNAP choices/public progress observation, transactional S2 composition, fixed point/evidence; SBA may become `implemented` only after all gates pass. S2 remains not covered. |
| D — S3.B Basic Priority | 11–14 | Priority event/Decision kernel, then Reference player submission through the shared transaction, replay/checkpoint/fork evidence; Basic Priority may become `implemented` after gates pass. Draw remains specified. |
| E — S3.C/D Draw integration | 15–20 | Draw, one-advance Upkeep→Draw path, continuation-aware SBA ordering, hidden-world proof and real V6 replay; Draw may become `implemented`. S2 is not promoted automatically. |
| F — S2 interaction/coverage closure | 21–24 | Independent review of both S2 interactions and V6 replay; update S2 to `covered` only if every accepted S2 gate passes; otherwise retain not covered and report the exact blocker. |

This sequence keeps `master` green, exposes no unsupported half-contract as a
capability, and keeps each semantic owner independently reviewable. Do not
combine S3.A, S3.B, or S3.C into one large implementation PR. A maintainer may
split a PR further if review reveals a public contract/version boundary.

## 11. Task-by-task execution

Every task is a separate logical commit. Future implementation tasks require
their own explicit authorization; this plan is not that authorization. RED
commits are allowed only on the isolated implementation branch and are never
merged before the paired GREEN task. At each PR boundary run the applicable
package tests and `scripts/run_checks.py integration`; run the complete
repository release/freeze gates only when preparing the corresponding
certification or freeze decision.

### Task 1 — RED: S3.P0 V5/V6 identity and compatibility vectors

**Allowed files:** test-only state/model/persistence/checkpoint/replay identity
tests, versioned schemas and golden/negative fixtures.

**Forbidden:** production identity codecs, Magic rule producers, registry/
lifecycle changes, V5 reinterpretation or automatic migration.

**RED tests:** require V5 FullStateDigest/CheckpointDigestV6/Replay V6 KATs
and a typed Magic continuation fixture; pin all current V4 full-state known
answers and V5 checkpoint/replay detached-validation results. Check V6
ExecutionIdentity binding, schema identities and V5 migration rejection.
New identity/type cases are expected to fail before S3.P0 implementation.

**Objective:** inventory every writer, reader, verifier, schema, golden,
negative fixture and Python surface affected by the identity cut. Record the
exact current V4/V5 compatibility behavior and archived-runtime requirements.

**Verification:** focused state/digest/checkpoint/replay tests (expected RED
recorded), `scripts/validate_schemas.py`, documentation checks,
`git diff --check`.

**Commit boundary:** P0 RED tests and fixture inventory only.

**HARD STOP:** do not reinterpret or rewrite any V4/V5 bytes.

### Task 2 — implement S3.P0 state/checkpoint/replay identity cut

**Allowed files:** `mtgml-model`, `mtgml-state`, persistence/checkpoint/replay
crates, schemas, Python adapters, golden/negative fixtures, generators and
identity/support documentation.

**Forbidden:** kernel creation/consumption of SBA ordering continuation,
SBA/Priority/Draw behavior, capability dependencies/lifecycle, V5 changes or
automatic V5 migration.

**Objective:** implement the linked S3.P0 design: FullStateDigestV5 and its
detached canonical mapping; EnvironmentCheckpointV6; CheckpointDigestV6;
InitialEnvironmentIdentityV6; ReplayManifestV6, ReplayStepV6,
AuthoritativeReplayV6, ReplayRecorderV6 and schema inventory V6. Add the
closed typed Magic SBA-order continuation representation and structural
validation, but no production Rules producer. Bind V6 checkpoints to complete
`ExecutionIdentityV1`; a Magic SBA continuation is restore-executable only
under a contract/runtime that later admits S3.A.

Preserve V4/V5 exact bytes and meanings. Current writers/readers move to V5
state digest and V6 checkpoint/replay. Classify V4 digest and V5 replay as
`READABLE_VERIFIABLE_ONLY` under current runtime; V5 checkpoints are
`UNSUPPORTED` for current V6 restore and require the archived V5 runtime for
execution. `V5_TO_V6_AUTOMATIC_MIGRATION = NONE`. Keep
`ObservationEnvelopeV1`, `InformationStateDigestV2`, and
`ExecutionIdentityV1`; V6 manifest binds `magic-m3-observation.v1` separately.

**RED/GREEN:** Task 1 identity tests pass; all V4 KAT bytes remain exact; V5
artifacts are never interpreted as V6; V6 digest/checkpoint restore/fork and
Replay V6 backend response parity pass across Rust/Python/schema fixtures.

**Verification:** full state/digest/checkpoint/replay suites, generator/schema
drift, `scripts/run_checks.py integration`, repository/archive reproducibility.

**Commit boundary:** coordinated S3.P0 identity cut; PR A. No Magic producer
or capability lifecycle change.

**HARD STOP:** old V4/V5 meaning/bytes change, V6 omits an authoritative state
field or execution identity, or any S3 rule becomes reachable during P0.

### Task 3 — RED: characterize and expose the shared transaction seam

**Allowed files:** `crates/mtgml-environment/src/tests/response_transaction.rs`,
`crates/mtgml-environment/src/tests.rs`, and test-only setup helpers under
`crates/mtgml-environment/src/tests/`.

**Forbidden:** production source edits; rules semantics; registry/lifecycle;
replay DTO/schema changes.

**RED tests:** pin the current Synthetic accepted and rejected response
products as the behavioral oracle: status and revision, deterministic forced
consequence ownership, environment counters, checkpoint identity,
`ReplayStepV6`, occurrence projections, player projections and complete
rejection nonmutation. Add a focused test-only seam proving these expectations
are not yet exercised through one shared response-commit primitive. Record the
current Magic behavior as a separate invariant: `MagicRulesKernel::apply(any
DecisionResponseV2) = UnsupportedPlayerResponse`, and Reference submission
remains unavailable. Do not require an accepted Reference pass or create a
production response to satisfy this RED.

**Objective:** establish exact Synthetic transaction behavior and executable
evidence for the missing shared transaction authority without changing
production behavior.

**Verification:** `cargo test -p mtgml-environment response_transaction`
(expected RED failure is recorded, never called PASS); existing synthetic
response/replay test filters must remain green.

**Commit boundary:** tests only, `S3.0 RED: shared transaction seam`.

**HARD STOP:** do not implement rules, require an accepted Magic response, or
make the RED fixture pass by fabricating a replay response.

### Task 4 — semantic-neutral shared response-commit primitive

**Allowed files:** `crates/mtgml-environment/src/response_transaction.rs` (or
one reviewed common transaction module), `reference.rs`, `synthetic/commit.rs`,
`lib.rs`, and environment transaction tests.

**Forbidden:** Magic state/event semantics, Basic Priority, Draw, arbitrary
forced-progress loops, changes to historical V5 DTOs or the already frozen V6
identity shapes, or changes to ADR-0040 ordering.

**RED/GREEN:** make Synthetic use one shared transaction implementation.
Retain a single rules-owned `kernel.apply`, at most one
`advance_forced_progress`, one merged transition/delta, one V6 append for the
real response, candidate projection, hook, and final atomic commit. Add
test-only failure injection at transaction boundaries without runtime mutable
semantic state. Reference may be wired to the primitive only if its public
availability and rejection behavior remain byte-for-byte/exactly unchanged;
it is also valid to defer all Reference call-site wiring to S3.B.

Inject and assert full nonmutation for kernel response rejection, existing
Synthetic forced-progress failure, candidate checkpoint failure, replay
append/export failure, occurrence projection failure, player projection
validation failure, before-commit hook failure, and counter/identity overflow
where the current substrate supports injection. Do not require SBA, S2,
APNAP/Order, or Basic Priority failure cases in S3.0. Compare EngineState,
status, RNG, allocators, knowledge, perspective identities/history, events,
continuation/request where applicable, counters, replay, checkpoint identity,
and player bytes.

**Verification:** `cargo test -p mtgml-environment --all-features`,
`cargo fmt --all -- --check`, `scripts/run_checks.py fast`, then
`scripts/run_checks.py integration` before PR B.

**Commit boundary:** common transaction and parity evidence, separate from
Task 3 RED commit.

Required S3.0 exit evidence is:

```text
SYNTHETIC_TRANSACTION_PARITY = PASS
ONE_SHARED_TRANSACTION_AUTHORITY = YES
REFERENCE_MAGIC_ACCEPTED_RESPONSE = NO
MAGIC_RULE_BEHAVIOR_CHANGED = NO
MAGIC_LEGAL_ACTIONS_CHANGED = NO
CAPABILITY_LIFECYCLE_CHANGE = NONE
```

**HARD STOP:** any Synthetic bytes/counters/replay behavior changes, duplicated
commit logic remains, a failure commits partial environment state, or Magic
response rejection changes.

### Task 5 — S3.A1 RED: simultaneous SBA batch and ordering obligation

**Allowed files:** `crates/mtgml-conformance/src/state_based_actions.rs`,
`crates/mtgml-conformance/src/lib.rs`, rules test modules, and test-only state
construction.

**Forbidden:** production SBA code, cardinality rejection, implicit ordering,
zone executor changes, continuation production types, lifecycle edits.

**RED tests:** all Foundation V2 predicates from one round-start state;
multiple same-owner deaths require a complete owner `Order`; different owners
are represented simultaneously; APNAP actor order when both owners need
choices; zero-toughness and lethal causes are mutually exclusive per-object
predicates at one round-start state; player loss plus creature death is one
round; both-player loss gets the exact
simultaneous-outcome mapping; no zone/life/status mutation happens while an
Order is pending; no order is chosen from IDs/containers; reject an incomplete
or duplicate order.

Assert order candidates map only through the owner's opaque identities,
answers are full permutations, complete action batch survives round staging,
no priority is offered, and every rejected answer preserves continuation,
Decision, state, allocators, history, replay and player bytes.

**Verification:** `cargo test -p mtgml-conformance state_based_actions` and
`cargo test -p mtgml-rules state_based_actions` (expected RED recorded).

**Commit boundary:** action-set/order RED cases only.

**HARD STOP:** no hidden-ID or deterministic sort workaround for CR 404.3.

### Task 6 — S3.A2 validate typed Magic continuation program

**Allowed files:** `crates/mtgml-state/src/m2_shape.rs`,
`crates/mtgml-state/src/m2_shape/continuation.rs`, related state continuation
tests, and the narrow Rules-owned read-only SBA semantic validator under
`crates/mtgml-rules/src/state_based_actions.rs` with its tests. This plan file
may be updated to record the ownership and restore boundary. S3.P0 owns
payload shape, V5 digest encoding and persistence identities.

**Forbidden:** changing `SyntheticM2Assembly` meaning, producer/decision
implementation, Priority/Draw code, implicit stage-local memory, or any
identity/schema version change.

**RED tests:** the state layer checks only structural continuation coherence:
canonical/unique actions and causes, existing referenced players/objects,
owner grouping, required-order-owner membership, completed-order
permutations, stage/actor/request coherence, and the `R + 1 + K` revision
relation. The Rules layer independently re-derives the bounded SBA action set
from the immutable current state, derives exact APNAP owners, and compares
both to the saved plan. It rejects stale, missing, extra, or inapplicable
actions without mutating state. Rules semantics do not move into
`mtgml-state`.

Production checkpoint restore/fork of an SBA continuation is not admissible
yet: there is no S3 production Magic `SemanticContractId`. Task 6 proves only
the V5 state/V6 checkpoint representation and retains the explicit production
S1 restore rejection. Real production restore/fork parity at each Order stage
is `DEFERRED_REQUIRED` to Task 10, after a separate S3 semantic contract is
introduced through the generated catalog. Do not make S1 admission accept the
SBA continuation to satisfy Task 6.

**Objective:** strengthen generic structural resumability in `mtgml-state`
and add a read-only, Rules-owned semantic plan validator under the fixed S3.A
conformance-candidate profile. Keep one ContinuationId across stages; fresh
DecisionId and actor-local PlayerDecisionId per stage. For Magic only, the
continuation record actor tracks the currently expected APNAP owner; Synthetic
continuations retain their fixed actor invariant. No continuation without a
pending Decision is valid checkpoint state.

Use the S3.P0 closed V5 continuation variant. Do not add a producer, execute
an Order response, mutate zones/life/status, allocate a production S3
SemanticContractId, change any digest/checkpoint shape, or edit the historical
FullStateDigestV4 codec. V5 continuation KATs must remain valid under the
newly pinned stage revision model.

**Verification:** state validation/continuation tests; Rules semantic
re-derivation tests; the V5 continuation KAT; S1 checkpoint admission
rejection; Task 5 producer RED remains red; `cargo fmt --all -- --check`.

**Commit boundary:** S3.A continuation semantic validation only; no payload,
digest or checkpoint type work and no producer.

**HARD STOP:** state-layer code derives Magic SBA semantics; a local cache or
event history is needed to resume the SBA round; S1 production admission is
weakened; or production checkpoint restore is claimed without an S3 semantic
contract.

### Task 7 — S3.A3 RED/GREEN: Order Decisions and APNAP stages

**Allowed files:** `crates/mtgml-rules/src/` SBA continuation program,
decision tests, candidate-binding contract/cursor tests and conformance.

**Forbidden:** any SBA or Graveyard mutation before all required owner orders;
new public Decision family; changing `SyntheticM2Assembly`; PriorityState
transition.

**RED tests:** request domain is `Order { minimum: N, maximum: N }`, candidates
are exactly the actor owner's selected SBA cards, candidate payloads use that
owner's opaque IDs and trusted `SelectObject` bindings, request visibility is
`ActingPlayerOnly`, and each answer is an exact permutation. One owner gets
one Order Decision. Two owners get active
then nonactive APNAP Decisions, same ContinuationId and fresh DecisionId /
PlayerDecisionId at each stage. The first response stores only the order and
creates the next Order Decision; state/zone/status/loss facts remain unchanged.
Wrong actor, stale response, invalid permutation, identity exhaustion, or
continuation mismatch rejects atomically.

**Objective:** derive and stage a fresh round requiring an Order; validate and
commit only nonfinal owner answers; preserve the complete unapplied batch and
advance to the next APNAP owner. The one-owner and final-owner responses stay
unaccepted until Task 9 owns their atomic choice-plus-batch transition.

**Verification:** order staging and nonfinal APNAP Task 5 cases turn green
through Decision V2; final-order and no-order SBA application cases remain
RED for Task 9. Run decision/state/rules/conformance suites and replay response
binding.

**Commit boundary:** APNAP Decision/continuation progression and trusted
order-choice audit, still no S2 multi-move integration or final order-only
acceptance.

**HARD STOP:** any zone/life/status mutation occurs before the last required
Order response, or any Order request leaks trusted GameObjectId.

### Task 8 — S3.A3b Magic public APNAP-order observation codec

**Allowed files:** `crates/mtgml-observation/src/`, its Python codec/DTO,
observation schemas and fixtures, `crates/mtgml-environment/src/` projection
and replay manifest validation, V6 manifest schemas/tests, observation docs.

**Forbidden:** new public Decision family, new `ObservedEvent`/`PlayerStep`
schema variant, exposing trusted object/card/continuation IDs, changing the
existing synthetic M3 observation bytes.

**RED tests:** after the active player supplies the first of two APNAP orders,
the nonactive player's current observation must contain the completed public
owner order before that player answers; current `synthetic-m3-observation.v1`
does not express it. Assert the existing PlayerStepV2 still returns only the
actor-bound Order Decision, while the nonactor's next observation sees the
previous order through opaque identities. For one-owner ordering, no partial
stage is exposed; after completion the field is null and final Graveyard state
shows the selected order. Missing an opaque mapping for any perspective fails
closed. No visible event sequence is consumed for this current-state field.

**Objective:** add `magic-m3-observation.v1` under existing
`ObservationEnvelopeV1.payload_codec`. Codec selection is an explicit closed
projection profile supplied by semantic execution context; it is not inferred
from `ExecutionProgramV1` or arbitrary state contents. Until the production S3
semantic contract exists, Magic projection is conformance/testkit-only.
Keep the existing V1 envelope,
InformationStateV2, PlayerStepV2, Decision V2 and ObservedEventEnvelopeV2
shapes. The Magic payload adds a closed `pending_sba_ordering` value with
APNAP completed orders expressed in each perspective's own `OpaqueObjectId`s
and the next owner. It never projects the continuation payload or trusted
IDs. The trusted `SbaGraveyardOrderChosen` event remains audit/replay
authority; the public current observation carries the order needed by the
next chooser.

Preserve and test the existing ReplayManifestV6 contract binding: the
SyntheticRulesCompat/S1 `synthetic-m3-observation.v1` codec remains unchanged;
the Magic codec is accepted only with the already-specified SBA capability
closure in detached identity fixtures. Do not allocate the production S3
semantic contract or broaden arbitrary strings. Update the Python codec and
all canonical schema/fixture representations.

**Verification:** observation and PlayerStep suites, paired noninterference
tests at each APNAP stage, V6 manifest/schema tests, `scripts/run_checks.py
fast`, Python full, generation/drift checks.

**Commit boundary:** versioned Magic current-observation projection + V6 codec
binding; no SBA state mutation code.

**HARD STOP:** the nonactive chooser cannot see the prior APNAP choice, a
trusted identity leaks, an event-sequence side channel appears, or existing
S1/Synthetic observation bytes change.

### Task 9 — S3.A4 ordered S2 composition and atomic SBA application

**Task 9A preparatory seam (authorized separately):** extract the mutation
inside the existing standalone `execute_selected_zone_transition()` into one
private S2 workspace primitive. The standalone wrapper continues to own its
single `R+1` accepted product; the workspace caller owns the candidate
revision, aggregate event cursor, final delta, and outer transition. Task 9A
does not derive or apply SBA actions and does not relax standalone OLD
reference checks.

Task 9B must close the final Order response's current pending Decision and
`MagicSbaGraveyardOrderV1` continuation inside its uncommitted outer workspace
before passing selected objects to S2. It may not expose that intermediate
state or weaken S2's standalone pending-decision rejection.

Task 9B0 resolves the bounded combat-reference design: S3.A may prune selected
combat references only after the sole normal CombatDamage assignment. The
current `CombatState` cannot preserve historical blocked status after a blocker
is removed, so pre-damage participant deaths and stale EndOfCombat removals
fail closed. Post-damage selected attackers/blockers are pruned only as
specified by the `StateBasedActionsApplied` actions; no general CombatState
mutation is licensed.

Task 9B0 freezes the following Task 9B semantic contract before production
implementation.

#### Atomic batch audit and continuation closure

Add one rule-relevant event and matching delta operation:

```text
StateBasedActionsApplied { actions: Vec<SbaSelectedActionV1> }
```

It carries the exact canonical complete action set re-derived from one
immutable round-start state. Its semantic effects are limited to: mark each
selected `PlayerLoses` player as lost; authorize selected battlefield
objects to leave combat under the combat policy below; retire the completed
Magic SBA continuation; and bind the following exact S2 `ZoneTransition`
set to the selected object actions. It does not perform zone or perspective
identity mutation itself.

For a final real Order response, the single accepted transition event order
is:

```text
DecisionCleared(old Decision)
SbaGraveyardOrderChosen(same continuation, final owner, exact order)
StateBasedActionsApplied(exact complete action set)
ZoneTransition / PerspectiveOccurrence for each selected creature
subsequent fixed-point round(s), if any
terminal status, or the existing supported next-decision boundary
```

Every event and zone occurrence has the one final candidate revision `R+1`.
The final order is represented only in a private cursor transient; it is
never stored as an EngineState continuation stage with all owners complete.
`StateBasedActionsApplied` consumes that transient together with saved earlier
orders and retires the cursor continuation. A standalone final
`SbaGraveyardOrderChosen` without its batch and exact S2 moves is rejected.

The outer scratch response workspace first validates the final response
against the immutable before-state and captures the trusted order. It then
clears the old pending Decision and removes the active SBA continuation in
scratch before calling S2. This is never externally validated or committed
as an intermediate state and does not weaken standalone S2 reference
rejection.

For a no-order round, emit `StateBasedActionsApplied` directly, without an
Order Decision or continuation. An owner with one selected card has its
unique order; owners with no selected cards have no move.

#### Loss and terminal mapping

`StateBasedActionsApplied(PlayerLoses(player))` is the only current S3.A
authority for `has_lost: false -> true`. Re-derive the exact action set;
require each applicable loss exactly once; reject missing, duplicate, extra,
or spurious loss actions and reject a before-state with `has_lost = true`.
No `PublicOutcome` string is added as a competing authority.

After the complete batch, one losing player maps to existing
`TerminalReason::RulesLoss` with canonical PlayerId-sorted Loss/Win outcomes;
two players losing in the same round map to
`TerminalReason::SimultaneousOutcome` with canonical Draw outcomes. Terminal
products have no next Decision. Use the existing EpisodeStatus contract.

#### Bounded combat-reference policy

The bound CR snapshot is the August 7, 2026 Comprehensive Rules artifact
identified by ADR 0051. CR 506.4 removes a permanent from combat when it
leaves the battlefield. CR 509.1h separately preserves an attacker's blocked
status when all blockers leave combat. The current `CombatState` stores live
attacker/blocker references but has no separate historical blocked bit.

S3.A may prune selected combat references only at
`TurnPosition::Combat(CombatDamage)`, after the bounded single normal damage
assignment. Foundation V2 excludes first/double strike and additional damage
steps, so the removed blocked-status distinction is not consumed by a later
damage assignment in this bounded slice. At BeginningOfCombat,
DeclareAttackers, or DeclareBlockers, a selected combat participant fails
closed. At EndOfCombat, a participant that should have been removed at the
post-damage SBA check also fails closed; this is not a second SBA opportunity.

At the admitted post-damage boundary, a selected dying attacker is removed
from `combat.attackers` and its attacker key is removed from `combat.blockers`,
preserving surviving attacker order. A selected dying blocker is removed from
its attacker's live blocker reference, represented as `None`. In this context
only, `None` means “no live blocker reference remains”; it does NOT represent
general blocked/unblocked history and cannot drive a later combat-damage
assignment. Preserve the enclosing CombatState/defending player. Any combat
mutation not derivable from the selected `StateBasedActionsApplied` actions
rejects. Do not add a general CombatStateChanged event or silently clear
combat state. A future pre-damage removal feature requires a separately
reviewed CombatState semantic representation for historical blocked status.

#### Multi-transition contract gate and fixed point

Standalone S2 continues to require exactly one selected ZoneTransition. An
SBA batch may contain N ZoneTransitions only when exactly one preceding
`StateBasedActionsApplied` event authorizes them; require a one-to-one mapping
to every and only `ObjectToOwnerGraveyard` action, with unique OLD and NEW
incarnations, no duplicate/omitted/extra moves, and exact S2 allocation and
destination order. No batch event means no multi-move exception.

The round plan and causes are derived from one immutable round-start snapshot.
Execute each owner's chosen top-to-bottom order in reverse through the single
S2 workspace. Task 9B applies one complete nonempty round; Task 10 proves the
full fixed point. The next round is derived from its completed result and is
applied before any priority. No Decision appears between fixed-point rounds,
except a new typed APNAP Order continuation when the next round requires an
actual order.

#### Remaining environment limitation

The production response transaction still performs exactly one forced
progress call after an accepted Running response. With Basic Priority not yet
implemented, a nonterminal final Order transition that has no next Decision
hits the existing `BasicPriority` unsupported boundary. Record
`NONTERMINAL_FINAL_ORDER_ENVIRONMENT_CLOSURE = BLOCKED_UNTIL_S3_B_OR_REVIEWED_TRANSACTION_DISPOSITION`.
Do not change the response transaction or hide the error in Task 9B0. This
blocks production replay/restore/lifecycle claims until Task 10 disposition.

Task 9B0 adds ignored/future-acceptance RED witnesses for final one- and
two-owner Order application, no-order batch application, terminal loss
products, post-damage combat pruning, and pre-damage/EndOfCombat fail-closed
boundaries. The no-order combat profile is pinned independently at fresh
BeginningOfCombat, DeclareAttackers, DeclareBlockers, and EndOfCombat states
with exactly one dying combat participant; its post-CombatDamage forced path
has a separate one-death/no-order application witness. Existing
one-/two-player loss producer REDs remain expected.
Standalone-final-order-without-batch and multi-move-without-batch rejection
are positive negative-contract guards.

**Allowed files:** sole S2 zone-incarnation executor/composition API,
`crates/mtgml-rules/src/` SBA result builder, events/delta/cursor/contract,
and conformance fixtures.

**Forbidden:** second zone-move implementation, object-ID sorting as chosen
order, per-object environment commits, `damage-and-life` dependency, invented
default Graveyard order.

**RED tests:** execute a selected top-to-bottom order `[A, B, C]` and assert
the final owner Graveyard top is exactly `[A, B, C]` above old entries;
physical identity is preserved, each OLD has one NEW, all old references
close, allocator progression is exact, and reapplication/cursor agree. Repeat
with both owners and APNAP collection. Inject exhaustion/S2 failure on a later
member and prove no partial batch is committed.

**Objective:** retain one authoritative S2 primitive. Derive every OLD and
cause from the same before-round snapshot. Compose all calls in one scratch
workspace; because the primitive inserts at top, execute each owner's
top-to-bottom selection in reverse within that owner group. Different owners
are independent; process owner groups in the fixed APNAP order for deterministic
allocator/event audit. Each accepted stage emits its own
`SbaGraveyardOrderChosen` event and response revision. The final APNAP Order
response emits its last choice event followed by `StateBasedActionsApplied`,
each existing `ZoneTransition`, fixed-point events and any resulting priority
Decision, all at that final candidate revision. Construct one complete
before-to-final delta and one transition product for that response. Earlier
staged choices remain represented in the before-state continuation/replay;
the environment does not observe or commit intermediate SBA workspace states.

**Verification:** all order and simultaneous action tests, S2 single-move
regressions, cursor/delta mutants, full atomic rejection fingerprint,
generation/schema checks.

**Commit boundary:** single-authority ordered composition + event/cursor proof;
no lifecycle/status promotion.

**HARD STOP:** S2's single move authority is duplicated, an intermediate move
is exposed, or final order differs from the player's accepted permutation.

### Task 10 — S3.A5 fixed point, evidence and lifecycle

**Allowed files:** SBA conformance, checkpoint/fork/replay tests and, after
all evidence passes, only SBA registry/status files.

**Forbidden:** Draw/Priority, changing capability dependencies, S2 coverage
promotion, certification.

**RED test/evidence:** terminal/no-priority and action-batch mutants remain
rejected; fixed-point reevaluation reaches an empty action set; pending APNAP
continuation cannot produce priority; restore/fork at each Order stage
reproduces identical next Decision and completion; replay each real Order
response and reproduce final batch.

**Objective:** close one- and two-owner order cases; terminal one-player and
simultaneous-loss products; combined death/loss; exact S2 identities/order;
noninterference; all atomic rejection surfaces. Prove only the current stage
actor can project the Order Decision; all candidates use that actor's opaque
IDs; the nonactive player sees prior public orders only through the
Magic-specific current observation required by CR 101.4b; no trusted identity
or incomplete zone result is exposed. No opponent `ObservedEventEnvelope` or
VisibleSequence is fabricated for an order-stage update. Final S2 public
movements disclose the complete authorized result. Only after independent S3.A review update
`state-based-actions-combat` to `implemented`. Record the S2
SBA interaction as evidence pending the later independent S2 coverage review.

**Verification:** rules/conformance/environment test suites,
`scripts/run_checks.py integration`, `cargo test --workspace --all-features
--locked`, schema/docs/status/registry checks.

**Commit boundary:** S3.A evidence + its justified implementation lifecycle
promotion; PR C. No S2 coverage promotion.

**HARD STOP:** no APNAP Order resume/replay proof, any mutation occurs before
all orders, or an S2 coverage claim is made here.

### Task 11 — RED: Basic Priority event, state and Decision contract

**Allowed files:** `crates/mtgml-rules/src/tests/`,
`crates/mtgml-conformance/src/basic_priority.rs` and module wiring, plus
test-only helpers.

**Forbidden:** production Priority event, decision generation, response
transaction, Draw.

**RED tests:** no-priority window admission, held-player actor binding,
pass-only precondition failure before Decision, exact one-pass candidate,
first pass transfer, second pass close, wrong actor/response/stale revision,
pending-decision/priority mismatch, RuleEventId/DecisionId/visible ID
exhaustion, terminal SBA prevents Decision, malformed transition product.
Pin the exact event ordering in Section 5.

**Verification:** `cargo test -p mtgml-rules basic_priority` and
`cargo test -p mtgml-conformance basic_priority` (expected RED recorded).

**Commit boundary:** Basic Priority RED cases only.

**HARD STOP:** no automatic pass, fallback actor, default response, or
Decision creation from an unproven pass-only surface.

### Task 12 — implement Basic Priority in the rules kernel

**Allowed files:** `crates/mtgml-rules/src/`, event/delta/cursor and
transition-contract authorities, generated contract sources if required, and
S3.B conformance.

**Forbidden:** environment commits, hidden player endpoint state, Draw,
non-pass action support, replay-version change.

**Objective:** implement the exact two-player empty-stack pass-only protocol.
Represent pass as the existing `ChooseOne` + single
`CandidateIntent::PassPriority` with exact trusted binding. Add a typed
`PriorityChanged`-equivalent event, matching semantic delta and cursor arm.
Validate priority holder/pass count against pending Decision actor, visibility,
candidate and binding. A second explicit pass closes the window and advances
only by the accepted temporal successor; forced continuation is returned to
the environment transaction.

**Verification:** Task 11 RED tests GREEN; rules/conformance package suites,
generated contract drift/schema checks, `scripts/run_checks.py fast`.

**Commit boundary:** rules-owned Basic Priority implementation and focused
conformance, no environment player API in this commit.

**HARD STOP:** `PriorityState` changes without a matching typed event/cursor
proof, or Decision and priority state disagree at any accepted boundary.

### Task 13 — RED/GREEN: Reference Magic player response path

**Allowed files:** `crates/mtgml-environment/src/reference.rs`, shared
transaction module, endpoint/error mapping, environment player endpoint tests.

**Prerequisite:** Task 12 Basic Priority kernel semantics and its accepted
`PassPriority` response contract must be implemented and pass its focused
tests. S3.0 alone cannot satisfy this task.

**Forbidden:** second commit pipeline; public trusted execution; changes to
Synthetic semantics; any Magic-specific rule calculation in environment code;
accepted Reference response before Task 12.

**RED tests:** after Task 12, from a validated Magic V6 checkpoint with a
real pending `PassPriority` Decision, require the authorized actor to receive
that Decision and a valid response to be accepted by `MagicRulesKernel`, then
committed through the shared S3.0 transaction into `PlayerStepV2` and one
`ReplayStepV6`. Before Task 12 this accepted-response witness is not required
and must remain unavailable.

**Objective:** make Reference project the pass Decision only to the actor and
submit via existing Layer A/B validation and the shared S3.0 transaction.
Map invalid/stale/no-decision/closed responses through existing closed public
codes. `execute_trusted_response` remains internal and uses the same
transaction for replay. No trusted kernel error or hidden state is returned
to a player.

**Verification:** Task 12 kernel response tests and this task's endpoint RED
pass; actor/nonactor projection; all closed
rejection codes; replay and environment existing tests; `cargo test -p
mtgml-environment --all-features`; `scripts/run_checks.py fast`.

**Commit boundary:** Reference endpoint wiring after Basic Priority kernel
semantics, within PR D.

**HARD STOP:** public API invokes `execute_trusted_response`, endpoint sees an
internal `DecisionId`/binding, or Reference and Synthetic use distinct
response-commit implementations.

### Task 14 — S3.B end-to-end priority evidence and lifecycle

**Allowed files:** Basic Priority conformance, environment tests, current
status/registry files only after evidence passes.

**Forbidden:** Draw execution; marking interactions with Draw satisfied; S2
coverage promotion.

**Evidence:** explicit active then opponent pass sequence; SBA gate before each
window; no-priority Untap/Cleanup unchanged; all selected priority-bearing
boundaries require pass-only validation; pass Decision soundness and
completeness; checkpoint/restore; fork parity; backend V6 replay/reprojection;
paired hidden-state bytes; every rejection path atomic.

**RED test / evidence failure:** a mutant automatic pass, mismatched
PriorityState/pending Decision pair, or invalid pass-only state must be rejected
without commit; missing replay/parity evidence keeps lifecycle `specified`.

After S3.B evidence review, update Basic Priority to `implemented`; retain its
Foundation V2 dependency closure and leave Draw specified.

**Verification:** `cargo test -p mtgml-rules --all-features`,
`cargo test -p mtgml-environment --all-features`,
`scripts/run_checks.py integration`, status/registry/docs/schema validation.

**Commit boundary:** evidence + Basic Priority lifecycle/status sync; PR D.

**HARD STOP:** any pass is implied, or priority can be granted before SBA and
the pass-only proof.

### Task 15 — freeze Draw event disposition

**Allowed files:** Draw design/implementation contract documentation and
conformance expectation only.

**Forbidden:** production event/DTO changes; adding an event for diagnostics;
altering capability dependencies.

**RED/decision:** assert the accepted closure proves Draw through before
context, the unique exact S2 Library-top-to-Hand transition, and after
priority/Decision context. The current admitted producers contain no other
Library-to-Hand path. Freeze:

```text
DRAW_COMPLETED_EVENT_REQUIRED = NO
```

Do not add `DrawCompleted`. If the code audit finds another admitted producer
or cannot validate the context-transition-context pairing, block Task 15 and
re-open the event decision before code.

**Verification:** conformance contract review; `scripts/check_documentation.py`
and `git diff --check`.

**Commit boundary:** a small design-only decision commit inside PR E.

**HARD STOP:** event evidence relies on a trace-only event or ignores another
admitted Library-to-Hand producer.

### Task 16 — RED: ordinary Draw × S2 interaction

**Allowed files:** Draw conformance cases, Rules/Environment test setup, and
paired-world tests.

**Forbidden:** Draw production implementation, registry/status edits, Basic
Priority changes, second Library-to-Hand executor.

**RED tests:** valid turn `>= 2`, Draw position, active owner, no priority/no
pending Decision, nonempty exact-top Library; exactly one S2 transition and
fresh incarnation; no RNG; owner private knowledge; opponent redaction;
position remains Draw with `HeldBy(active,0)` plus exact pass Decision;
Restore completed state cannot redraw. Reject all Design exclusions and
S2 rejection, with full environment/player fingerprint equality.

Paired hidden worlds vary physical and definition identities. Opponent
Observation, InformationState, Decision, PlayerStep, observed events and
sequence behavior must match; only authorized owner-private products differ.

**Verification:** relevant `mtgml-rules`, `mtgml-conformance`,
`mtgml-environment` filters (expected RED recorded); noninterference suite.

**Commit boundary:** Draw RED tests only.

**HARD STOP:** any decision is fabricated for Draw, or tests depend on test-only
knowledge/history to prevent repetition.

### Task 17 — implement Draw through S2

**Allowed files:** `crates/mtgml-rules/src/` Draw producer and orchestration,
S2 executor invocation, transition validator/cursor, conformance.

**Forbidden:** editing `zone_incarnation.rs` to add another move path, a
`draw-card -> basic-priority` dependency, Priority implementation changes,
RNG, Draw event, empty-library loss, triggers/replacements.

**Objective:** admit exactly the reviewed normal Draw profile; derive the
exact top from authoritative ordered Library; produce semantic intent; call
the one S2 executor; complete the same transition workspace through
`priority-sba-gate`; stop at terminal or create Basic Priority's actual
pass Decision. The forced-progress kernel returns one complete TransitionResult
through the next Decision/outcome or a typed unsupported/error boundary.

**Verification:** Task 16 RED GREEN; S2 conformance regression; exact
checkpoint/delta/cursor/event/rejection tests; generated contract and schema
drift checks; `scripts/run_checks.py fast`.

**Commit boundary:** Draw producer and focused kernel evidence, no final replay
claim until Task 19.

**HARD STOP:** any accepted state exposes moved card + Draw + Priority=None, or
the implementation moves a card without invoking the S2 executor.

### Task 18 — Upkeep pass → Draw → priority integration

**Allowed files:** environment turn/priority/Draw integration tests and
conformance only.

**Forbidden:** environment scheduler loop, fake responses, direct state
mutation, S2 lifecycle update.

**RED/GREEN witness:** start `Beginning(Upkeep)`, `HeldBy(active,0)`, active
pass Decision. Active pass produces opponent pass Decision. Opponent's real
pass closes Upkeep; shared transaction calls `advance_forced_progress` exactly
once. In the no-SBA-action Draw witness, that kernel call reaches Draw, S2,
SBA stability, and active `HeldBy(active,0)` with the next explicit pass
Decision. A companion same-owner multi-death case reaches the first APNAP
Order Decision without mutation and completes the SBA batch only after all
real owner order responses. Assert exact event/revision order and each atomic
TransitionResult.

Test terminal SBA and typed unsupported outcomes stop the advance correctly.

**Verification:** environment/conformance focused tests, package suites,
`scripts/run_checks.py fast`.

**Commit boundary:** integrated response-to-Draw witness, in PR E.

**HARD STOP:** temporal successor skips Draw, draw priority is passed
implicitly, or the environment performs a second progress call.

### Task 19 — authoritative Replay V6 witness for S2

**Allowed files:** environment backend replay, replay parity tests, conformance
evidence. No replay DTO version change.

**Forbidden:** Replay V7 or any replay DTO change beyond S3.P0, a
forced-progress replay step, event-as-input, fake DecisionResponse, or a
test-only replay executor.

**Evidence:** export the real two-pass V6 replay; execute it from the exact
initial checkpoint with Reference backend. Prove replay re-applies the
opponent's second pass and repeats the integrated forced consequence. Compare
the exact new `GameObjectId`, S2 `ZoneTransition`, physical continuity/order,
knowledge and perspective identity, event order, delta, status, counters,
full-state digest, after checkpoint identity, next Decision and player
projections. Restore/fork/rerun produce the same proof.

**RED test / evidence failure:** mutate each recorded after identity and the
replayed fresh object/knowledge/event result; backend replay must reject every
divergence. Structural V6 validation alone is not a passing witness.

**Verification:** `cargo test -p mtgml-environment authoritative_s2_replay`,
all V6 replay tests, `scripts/run_checks.py integration`.

**Commit boundary:** backend-verified replay witness; no S2 promotion in this
commit.

**HARD STOP:** V6 only structurally validates, but backend execution does not
reproduce the exact S2 transition and after identity.

### Task 20 — S3 interaction and information evidence; Draw lifecycle

**Allowed files:** conformance cases and, after evidence review, Draw registry
lifecycle and status test/docs entries.

**Forbidden:** S2 coverage promotion; certification; new Draw/priority public
Decision/event shape.

**Evidence:** close `turn-structure × draw-card`,
`draw-card × zone-incarnation`, and cross-priority/SBA ordering. Include owner
exact knowledge, opponent paired-world byte parity, checkpoint restore before
and after Draw, forks, rejection nonmutation, no RNG, no Decision for Draw,
and Task 19 authoritative V6 replay, including a separate checkpointed APNAP
ordering witness when a same-owner simultaneous Graveyard group occurs.

For APNAP ordering, checkpoint after the first owner's Order response and
restore/fork before the next owner responds. The nonactor endpoint receives no
trusted identity, partial order, continuation payload, Order Decision or
visible-sequence side channel. Compare paired worlds' opponent Observation,
InformationState, PlayerStep and observed-event bytes at each stage.

After the independent Draw scope review and applicable full gates, update only
`rules/draw-card` from `specified` to `implemented`. Mark no interaction row
or S2 lifecycle satisfied by implementation status alone.

**RED test / evidence failure:** any unauthorized opponent identity byte,
repeated Draw transition, or missing witness prevents the interaction review
and keeps Draw `specified`.

**Verification:** rules/conformance/environment suites,
`scripts/run_checks.py integration`, schema/document/status tests, exact
registry closure.

**Commit boundary:** S3.C evidence and justified Draw implementation status;
PR E.

**HARD STOP:** owner/opponent information differs outside S2 authorization or
S2 replay evidence is missing.

### Task 21 — independent S2 interaction and coverage review

**Allowed files:** evidence/status review artifacts and S2 lifecycle surfaces
only after the reviewer accepts every gate.

**Forbidden:** retroactive test weakening, automatic coverage due to merged
code, certification, changing S2 semantic scope in-place.

**Review bundle:**

```text
state-based-actions-combat × zone-incarnation = SATISFIED_EVIDENCE
draw-card × zone-incarnation = SATISFIED_EVIDENCE
authoritative V6 replay of real S2 transition = PASS
all S2 accepted-design evidence remains valid at exact head
```

**RED test / evidence failure:** independent review rejects either interaction,
V6 backend replay diverges, or any accepted S2 gate is absent; S2 remains
`implemented / not covered`.

If every gate passes, a separate authorized lifecycle decision may update S2
from `implemented / not covered` to `covered`. If any gate is missing, record
the precise blocker and keep S2 not covered. This is an independent review,
not an implementation-task assumption.

**Verification:** rerun the complete S2-specific conformance/replay suite and
the required exact-head status checks on the reviewed candidate.

**Commit boundary:** separate governance/evidence PR F; no bundled Draw code.

**HARD STOP:** reviewer rejects or any required S2 coverage gate is not PASS.

### Task 22 — lifecycle and interaction status closure

**Allowed files:** capability registry lifecycle fields, README, roadmap,
current-status tests, and accepted evidence/status artifact references.

**Forbidden:** semantic code changes, capability dependencies, certification,
claims beyond the exact witnessed scope.

**Objective:** synchronize lifecycle counts and statuses to merged evidence:
SBA, Basic Priority and Draw implemented only if their individual evidence
passed; turn structure remains covered; zone incarnation becomes covered only
if Task 21 passed. Keep explicit S2 replay and interaction results factual.
Update generated status/contract artifacts only through their authority.

**RED test / evidence failure:** status assertions deliberately compare the
registry, README and roadmap against stale or overclaimed states and must fail;
final status tests pass only for evidence-backed lifecycle values.

**Verification:** focused status tests, docs, schemas, maintainer artifacts,
contract generation/drift, `scripts/run_checks.py integration`.

**Commit boundary:** status-only synchronization following Task 21's review;
PR F.

**HARD STOP:** any unsupported lifecycle/coverage claim appears in registry,
README, roadmap, generated artifacts, or tests.

### Task 23 — cumulative exact-head verification

**Allowed files:** verification reports in the repository-defined external
verification location only; no source changes after final archive gate.

**Forbidden:** source edits after final verification, release/freeze claims,
certification, implementation PR creation without explicit later instruction.

**Verification:** fetch and pin actual `origin/master`; verify branch ancestor
and full staged/unstaged diff; run strict doctor, documentation, schema,
maintainer, full Python, fast, Rust fmt, integration/full Cargo gates, all S3
conformance/noninterference/replay cases, repository/archive reproducibility
gates required by the chosen delivery profile; inspect current generated
status/evidence artifacts. Record every result as PASS/FAIL/NOT_RUN/BLOCKED.

**RED test / evidence failure:** exact-head/fingerprint checks must fail for a
dirty tree, wrong base, generated drift, or missing required gate record.
NOT_RUN/BLOCKED must never be presented as PASS.

**Commit boundary:** verification/status evidence commit only if repository
practice requires tracked evidence; otherwise no code commit.

**HARD STOP:** any required gate fails or remains unknown; resolve it before
review handoff.

### Task 24 — implementation PR preparation

**Allowed files:** PR description/verification evidence; no source changes
unless a previous gate explicitly reopened a task.

**Forbidden:** creating the PR without a separate user instruction; merging,
release, freeze, or certification.

**Objective:** produce a complete diff summary, ordered commit list, exact base
and head, capability/interaction dispositions, actual gate results, skipped
or blocked gates, and explicit unchanged exclusions. Ask for independent
implementation review as the final step of the later authorized workflow.

**RED test / evidence failure:** the PR-readiness check rejects missing
base/head identity, dirty worktree, lifecycle mismatch, or any blocked required
gate; this task produces review material only.

**Verification:** confirm exact remote head/base relationship, clean worktree,
all required CI checks and no generated/source drift.

**Commit boundary:** none; this plan stops at PR-ready review material.

**HARD STOP:** do not create, merge, or publish a PR in this planning task.

## 12. Required conclusions

```text
S3_ORDER =
S3.P0 authoritative state/checkpoint/replay identity cut
-> S3.0 shared response transaction
-> S3.A1 simultaneous SBA derivation and Order requirement
-> S3.A2 validate typed Magic SBA continuation program
-> S3.A3 existing Order Decision and APNAP collection
-> S3.A4 ordered S2 incarnation composition and atomic round application
-> S3.A5 fixed point and evidence
-> S3.B basic-priority × turn-structure × state-based-actions-combat
-> S3.C draw-card × turn-structure × zone-incarnation
-> S3.D end-to-end Upkeep passes -> Draw -> SBA -> priority -> V6 replay

S3_A = full simultaneous selected SBA set; explicit per-owner Order Choices for same-Graveyard multi-card rounds; S3.P0-typed Magic continuation validated/resumed by S3.A; ordered S2 batch composition; fixed point and terminal mapping
S3_B = explicit pass-only priority, typed priority event/cursor, bound ChooseOne PassPriority Decision
S3_C = exactly-once ordinary Draw Step via S2; no DrawCompleted event

SHARED_RESPONSE_TRANSACTION_REQUIRED = YES
S3_0_SEMANTIC_NEUTRAL = YES
S3_0_CAN_ACCEPT_MAGIC_PLAYER_RESPONSE = NO
S3_0_CHANGES_MAGIC_LEGAL_ACTIONS = NO
MAGIC_PLAYER_RESPONSE_SEMANTICS_CHANGED_BY_S3_0 = NO
SYNTHETIC_FIRST_SHARED_TRANSACTION_WITNESS = YES
REFERENCE_TRANSACTION_PLUMBING_MAY_PREEXIST = YES, only if externally observable Magic behavior remains unchanged
BASIC_PRIORITY_REQUIRED_BEFORE_REFERENCE_ACCEPTED_PASS = YES
REFERENCE_ACCEPTED_MAGIC_RESPONSE_REQUIRES_S3_B = YES
REFERENCE_MAGIC_PLAYER_RESPONSE_PATH_OWNER = S3.B
REFERENCE_MAGIC_RESPONSE_REPLAY_OWNER = S3.B
REFERENCE_MAGIC_PLAYER_RESPONSE_PATH_REQUIRED = YES
ENVIRONMENT_FORCED_PROGRESS_LOOP_REQUIRED = NO
ONE_KERNEL_FORCED_ADVANCE_SUFFICIENT = YES, if it runs all forced work to next Decision/outcome/error
PRIORITY_EVENT_REQUIRED = YES
DRAW_COMPLETED_EVENT_REQUIRED = NO
DRAW_PROGRESS_STATE_REQUIRED = NO
NEW_AUTHORITATIVE_MAGIC_CONTINUATION_STATE = YES
FULL_STATE_DIGEST_V4_CAN_BE_EXTENDED_IN_PLACE = NO
FULL_STATE_DIGEST_V5_REQUIRED = YES
ENVIRONMENT_CHECKPOINT_V6_REQUIRED = YES
CHECKPOINT_DIGEST_V6_REQUIRED = YES
REPLAY_V6_REQUIRED = YES
OBSERVATION_ENVELOPE_V2_REQUIRED = NO
INFORMATION_STATE_DIGEST_V3_REQUIRED = NO, unless independent retained-information review finds a meaning change
MAGIC_OBSERVATION_PAYLOAD_CODEC = magic-m3-observation.v1
V5_ARTIFACTS_REINTERPRETED = NO
AUTOMATIC_V5_TO_V6_MIGRATION = NONE
S3_P0_REQUIRED = YES
S2_SBA_INTERACTION_CLOSABLE = YES, within the reviewed S2 admitted zone-order profile
S2_DRAW_INTERACTION_CLOSABLE = YES
S2_AUTHORITATIVE_REPLAY_PATH_CLOSABLE = YES, through a real response transaction plus backend V6 replay
S2_COVERED_PROMOTION_POSSIBLE_AFTER_PLAN = YES, only after independent review and all accepted S2 gates pass
SAME_OWNER_MULTI_GRAVEYARD_POLICY = EXPLICIT_ORDER_DECISION
GRAVEYARD_ORDER_DECISION_DOMAIN = DecisionDomainV2::Order
MULTI_PLAYER_ORDER_COLLECTION = APNAP
SBA_ORDER_CONTINUATION_REQUIRED = YES
NEW_PUBLIC_DECISION_FAMILY_REQUIRED = NO
NEW_TYPED_MAGIC_CONTINUATION_PAYLOAD_REQUIRED = YES
SBA_ZONE_BATCH_INTEGRATION_REQUIRED = YES
ZONE_INCARNATION_SEMANTIC_VERSION_CHANGE_REQUIRED = NO (S3.A composes the existing S2 executor atomically)
SCOPE_NARROWING_REQUIRED = NO
CAPABILITY_VERSION_CHANGE_REQUIRED_FOR_FAIL_CLOSED_BOUND = NOT_APPLICABLE — bound rejected
S3_IMPLEMENTATION_PLAN_READY = YES
S3_IMPLEMENTATION_AUTHORIZED = NO
```
