# M3.S3 End-to-End Interaction Implementation Plan

**Status:** executable implementation plan; implementation not authorized by this plan
**Date:** 2026-09-23
**Branch:** `chris/m3-s3-draw-card-design`
**Required base:** `b67cfdcc0a8e623da52a889ef2ae138a3e4256ac`
**Reviewed design:** [M3.S3 Draw-card interaction design](../specs/2026-09-23-m3-s3-draw-card-design.md)
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

## 2. Frozen S3 order

The proposed order is accepted with one process-only prerequisite before
S3.A: a semantic-neutral shared response-commit extraction. It changes no
Magic rule and no lifecycle.

```text
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
    -> backend-verified Replay V5
```

S3.A comes first because Basic Priority's accepted dependency requires SBA and
because SBA × zone-incarnation is one outstanding S2 producer interaction.
S3.B comes next because Draw must enter a real priority window through the
shared priority owner, after SBA. S3.C then adds the normal turn-based Draw
producer using the already implemented S2 executor. S3.D is integration and
evidence, not a new capability.

The order is not a coding convenience and must not be inverted. Each PR must
leave `master` buildable and the declared lifecycle truthful.

## 3. S3.A exact bounded SBA scope

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

If one object qualifies for both zero-toughness and lethal-damage actions, the
round contains one object-removal action with both applicable causes; it must
produce one S2 zone transition, one fresh `GameObjectId`, and one OLD-reference
closure. It must not attempt two incarnation transitions for the same OLD.

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

**S2 ordering bound:** S2 explicitly excludes simultaneous multi-object
Graveyard ordering. For S3.A, preflight must reject atomically if a single SBA
round would put more than one creature owned by the same player into that
player's ordered Graveyard. One move per owner in a round is admitted; moves
to different owners' Graveyards can coexist in the same simultaneous round.
Never choose an order using hidden/global ID order as a substitute for an
unreviewed Magic ordering rule. If Foundation V2 review determines that its
selected scope requires same-owner multi-object ordering, stop S3.A before
production implementation and revise the accepted scope/authority explicitly.

### S3.A event and delta shape

The action batch is rule-relevant semantic meaning, not tracing. Add one typed
rules event for each nonempty fixed-point round, conceptually:

```text
StateBasedActionsApplied {
  actions: canonical complete action set derived from the round-start state
}
```

Its closed action variants carry trusted player/object identity and the
selected cause (`player loses`, `zero toughness`, `lethal marked damage`, or
both creature causes). The event's matching `SemanticDeltaOperation` preserves
that batch. Then emit the already existing S2 `ZoneTransition` for each
creature action in canonical `(owner PlayerId, old GameObjectId)` order; the
per-owner cardinality bound ensures this audit ordering cannot decide
same-Graveyard semantics. All events carry the candidate revision. Terminal
`EpisodeStatus` is the environment result paired with the `player loses`
actions; do not add a string-coded `PublicOutcome` as a second authority.

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

## 4. S3.B exact Basic Priority scope

Support only the validated pass-only two-player profile. Before every window:

1. The `priority-sba-gate` runs S3.A to a stable result.
2. If terminal, stop without priority.
3. Derive and validate `PASS_ONLY_PRIORITY_PRECONDITION` from complete
   authoritative state: empty stack, no waiting/delayed triggers, effects,
   replacement/prevention, continuous-effect, mana, special-action,
   land-play, or action-bearing ability surface; inert card objects; no
   turn-based action still pending.
4. If any fact is absent or non-pass action remains unproven, reject before
   creating a Decision.
5. Otherwise give the active player priority and create one explicit pass
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

## 5. S3.C exact Draw scope and event disposition

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

## 6. Environment response transaction and one forced advance

Current seam at the required head:

```text
SyntheticM1EnvironmentBackend::execute_response
  owns response + forced consequence + V5 append + atomic commit

ReferenceEnvironmentBackend::execute_trusted_response
  calls kernel.apply only; does not commit or append replay

ReferenceEnvironmentBackend::submit_player_response
  returns UnavailableDecision while Running, otherwise EpisodeClosed
```

Task S3.0 extracts one environment-owned response transaction primitive and
routes both backends through it. Do not copy the Synthetic transaction into
Reference. Preserve ADR-0040 order exactly:

1. Capture and validate the complete before `EnvironmentCheckpointV5`.
2. Validate endpoint/player/episode/pending actor and response layers; call
   `kernel.apply` with immutable before-state semantics.
3. If accepted, no Decision exists, and status is Running, call
   `kernel.advance_forced_progress` exactly once. Merge its complete event
   sequence and recompute one before-to-final `StateDelta` over the whole
   response transaction.
4. Validate the complete transition contract.
5. On semantic rejection, prove checkpoint, status, every counter, replay and
   projected player state equal their before values; commit nothing.
6. Calculate candidate counters and construct/validate candidate V5 checkpoint.
7. Append exactly one `ReplayStepV5` for the real input response and validate
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
point, and the new pass Decision. If the kernel cannot guarantee this, stop;
do not add an arbitrary environment loop.

`ReferenceEnvironmentBackend::player_visible_decision` returns the projected
pass Decision only for its authorized actor. Its `submit_player_response`
uses existing Layer A/B and stable rejection mapping, invokes the shared
transaction, and returns the usual `PlayerStepV2`. Public callers never use
`execute_trusted_response`; replay uses that internal trusted entry and the
same shared transaction. Synthetic behavior and byte products must remain
unchanged under semantic-neutral S3.0.

## 7. Replay and evidence contract

Replay remains V5. A forced consequence has no replay step and no input DTO.
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
  V5 records the actual opponent pass response in ReplayStep N+1
```

Backend replay from the initial V5 checkpoint must re-execute both real pass
responses and prove identical before identity, actor/request binding, capability
closure, allocator-derived fresh `GameObjectId`, zone/order, S2 knowledge and
perspective identity mutations, authoritative events, delta, full-state digest,
status, counters, after checkpoint identity, next Decision, and all player
products. It must include no forced-work replay step, fake response, or V6.

Checkpoint/restore and fork tests cover both pending and completed Draw state.
Paired hidden worlds vary top-card `CardDefinitionId` and
`PhysicalCardId` while keeping public facts equal. Opponent observation,
information, Decision, PlayerStep, observed events, sequence behavior, and
rejection class remain identical. Owner differences are exactly the S2
authorized private identity/knowledge differences. Priority DTOs never reveal
Draw identity.

## 8. Lifecycle and interaction gates

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
  real derived creature SBA -> one exact S2 Battlefield-to-owner-Graveyard
  transition, complete event/delta/cursor, fresh identity, OLD-reference
  closure, and rejection nonmutation

draw-card × zone-incarnation:
  real mandatory Draw -> exact S2 Library-top-to-owner-Hand transition,
  correct private knowledge and opponent parity
```

Mark neither satisfied until executable witnesses pass and an independent
review accepts them. S2 covered promotion additionally requires backend-
verified authoritative V5 replay of a real S2 transition and every other S2
coverage gate in its accepted design. Promotion is a distinct governance task;
deterministic rerun alone is insufficient.

## 9. Sequential PR strategy

Use five sequential PRs, each based on the newly merged `master` head:

| PR | Tasks | Scope and merge exit |
| --- | --- | --- |
| A — shared response transaction | 1–2 | Semantic-neutral extraction; Synthetic regression parity; Reference remains unavailable until its separate implementation. No lifecycle change. |
| B — S3.A SBA | 3–5 | Bounded SBA fixed point, selected S2 move, conformance and rejection evidence; `state-based-actions-combat` may become `implemented` only after its gates pass. S2 remains not covered. |
| C — S3.B Basic Priority | 6–9 | Priority event/Decision kernel, Reference player submission through shared transaction, replay/checkpoint/fork evidence; Basic Priority may become `implemented` after gates pass. Draw remains specified. |
| D — S3.C/D Draw integration | 10–15 | Draw, one-advance end-to-end Upkeep→Draw→priority path, hidden-world proof and real V5 replay; Draw may become `implemented`. S2 is still not promoted automatically. |
| E — S2 interaction/coverage closure | 16–19 | Independent review of both S2 interactions and V5 replay; update S2 to `covered` only if every accepted S2 gate passes; otherwise retain not covered and report the exact blocker. |

This sequence keeps `master` green, exposes no unsupported half-contract as a
capability, and keeps each semantic owner independently reviewable. Do not
combine S3.A, S3.B, or S3.C into one large implementation PR. A maintainer may
split a PR further if review reveals a public contract/version boundary.

## 10. Task-by-task execution

Every task is a separate logical commit. Future implementation tasks require
their own explicit authorization; this plan is not that authorization. RED
commits are allowed only on the isolated implementation branch and are never
merged before the paired GREEN task. At each PR boundary run the applicable
package tests and `scripts/run_checks.py integration`; run the complete
repository release/freeze gates only when preparing the corresponding
certification or freeze decision.

### Task 1 — RED: expose the Reference response transaction seam

**Allowed files:** `crates/mtgml-environment/src/tests/response_transaction.rs`,
`crates/mtgml-environment/src/tests.rs`, and test-only setup helpers under
`crates/mtgml-environment/src/tests/`.

**Forbidden:** production source edits; rules semantics; registry/lifecycle;
replay version or fixtures.

**RED test:** from a validated Reference Magic checkpoint with one pending
pass Decision, the authorized actor's visible Decision is projected and a
valid pass submission returns an accepted PlayerStep, advances the state,
increments exactly the decision/event counters, and appends one genuine V5
response step. Assert the current implementation fails because Reference
submission returns `UnavailableDecision`. Pin existing Synthetic response,
rejection, and replay product bytes as semantic-neutral baselines.

**Objective:** create executable evidence of the missing production Magic
player path and transaction behavior without changing production behavior.

**Verification:** `cargo test -p mtgml-environment response_transaction`
(expected RED failure is recorded, never called PASS); existing synthetic
response/replay test filters must remain green.

**Commit boundary:** tests only, `S3.0 RED: reference response transaction`.

**HARD STOP:** do not implement rules or make the RED fixture pass by
fabricating a replay response.

### Task 2 — semantic-neutral shared response-commit primitive

**Allowed files:** `crates/mtgml-environment/src/response_transaction.rs` (or
one reviewed common transaction module), `reference.rs`, `synthetic/commit.rs`,
`lib.rs`, and environment transaction tests.

**Forbidden:** Magic state/event semantics, Basic Priority, Draw, arbitrary
forced-progress loops, changes to V5 DTOs or ADR-0040 ordering.

**RED/GREEN:** make both Synthetic and Reference backends use one transaction
implementation. Retain a single rules-owned `kernel.apply`, at most one
`advance_forced_progress`, one merged transition/delta, one V5 append for the
real response, candidate projection, hook, and final atomic commit. Add
test-only failure injection at transaction boundaries without runtime mutable
semantic state.

Inject and assert full nonmutation for kernel response rejection,
forced-progress/SBA failure, S2 zone rejection, object/Decision/event/visible-
sequence exhaustion, candidate checkpoint failure, replay append/export
failure, occurrence projection failure, player projection validation failure,
and before-commit hook failure. Compare EngineState, status, RNG, allocators,
knowledge, perspective identities/history, events, counters, replay,
checkpoint identity, and player bytes.

**Verification:** `cargo test -p mtgml-environment --all-features`,
`cargo fmt --all -- --check`, `scripts/run_checks.py fast`, then
`scripts/run_checks.py integration` before PR A.

**Commit boundary:** common transaction and parity evidence, separate from
Task 1 RED commit.

**HARD STOP:** any Synthetic bytes/counters/replay behavior changes, duplicated
commit logic remains, or a failure commits partial environment state.

### Task 3 — RED: S3.A SBA fixed-point conformance

**Allowed files:** `crates/mtgml-conformance/src/state_based_actions.rs`,
`crates/mtgml-conformance/src/lib.rs`, `crates/mtgml-rules/src/tests/`, and
test-only state construction.

**Forbidden:** production SBA implementation, a second zone executor,
`damage-and-life` dependency, schema/lifecycle promotion.

**RED tests:** life at zero/negative; one player loss; simultaneous two-player
loss; zero toughness; lethal marked damage; both causes on one creature
(exactly one move); multiple applicable actions in one common round snapshot;
stable reevaluation; different-owner concurrent Graveyard moves; more than one
same-owner Graveyard move rejected before mutation; terminal plus creature
move; invalid/unsupported source profile; S2 rejection and ID exhaustion.

Assert exact status, `has_lost`, old/new identities, ordered zone vectors,
event/delta/cursor pairing, no RNG, no Decision at terminal, and full rejection
fingerprints. If review rejects the one-move-per-owner bound, stop and request
an explicit scope/ordering design update before production edits.

**Verification:** `cargo test -p mtgml-conformance state_based_actions` and
`cargo test -p mtgml-rules state_based_actions` (expected RED recorded).

**Commit boundary:** S3.A RED cases only.

**HARD STOP:** no deterministic GameObjectId sort may be used to invent
same-owner simultaneous Graveyard order.

### Task 4 — implement bounded S3.A SBA fixed point

**Allowed files:** `crates/mtgml-rules/src/`, authoritative event/delta/cursor
and transition validation; the authoritative event contract and affected
schema/catalog/generator sources; `crates/mtgml-conformance/src/`.

**Forbidden:** Draw, Priority, `damage-and-life` execution/dependency, arbitrary
SBA kinds, triggers/replacements, a new zone-incarnation implementation.

**Objective:** derive each complete SBA action batch from one round-start
snapshot; validate the no-unsupported-effect profile; record typed SBA action
meaning; invoke S2 exactly once for each admitted physical-card move; apply all
round consequences to one scratch state; re-evaluate until stable; then map
terminal status exactly. Cursor and transition contract prove the derived
batch, move correspondence, fixed-point order, full state, event revision,
delta, and status.

**RED test:** the Task 3 transition mutants and incomplete action batches must
remain rejected until this implementation provides the exact event/cursor
proof; no test may be weakened to permit unexplained `has_lost` or zone state.

`StateBasedActionsApplied` must be a typed authoritative event and matching
semantic delta. Its cursor arm must prove the complete canonical action set
and mutate the `has_lost` facts. Existing `ZoneTransition` remains the sole
incarnation event. Update every generated/schema representation from its
authoritative source.

**Verification:** RED cases from Task 3 turn GREEN;
`cargo test -p mtgml-rules --all-features`,
`cargo test -p mtgml-conformance --all-features`, generation/drift checks,
schema and Python fixture checks, `scripts/run_checks.py fast`.

**Commit boundary:** SBA interpreter/event/cursor implementation, no lifecycle
change before Task 5 evidence review.

**HARD STOP:** event audit cannot prove the same-snapshot action set, a selected
move bypasses S2, or the exact terminal mapping diverges from Foundation V2.

### Task 5 — S3.A evidence and implementation lifecycle

**Allowed files:** conformance evidence and, only after all S3.A gates pass,
`cards/capabilities/registry.json`, `README.md`, `docs/ROADMAP.md`, and
`python/tests/test_current_status.py`.

**Forbidden:** marking the S2 interaction satisfied or promoting S2 to
covered; unrelated capability status edits.

**Evidence:** exact simultaneous batch and fixed point; one S2
Battlefield-to-owner-Graveyard witness with fresh incarnation, physical
continuity, event/delta/cursor and OLD-reference closure; both terminal
mappings; no Decision/priority after terminal; rejection atomicity; checkpoint
restore/fork/rerun; perspective-safe public movement.

**RED test / evidence failure:** a contract mutant omitting one applicable
action, adding a duplicate zone move, or changing `has_lost` without the typed
SBA batch must fail transition validation; a missing evidence artifact keeps
lifecycle `specified`.

Update `state-based-actions-combat` from `specified` to `implemented` only
after independent evidence review and full relevant gates. Record the S2
interaction as candidate evidence for independent later review; current S2
status stays `implemented / not covered`.

**Verification:** `scripts/run_checks.py integration`,
`cargo test --workspace --all-features --locked`, docs/schema/registry/status
validators, exact commit diff.

**Commit boundary:** S3.A evidence + only its justified lifecycle/status sync;
PR B.

**HARD STOP:** an SBA interaction row or S2 lifecycle is promoted without an
independent review and all applicable evidence.

### Task 6 — RED: Basic Priority event, state and Decision contract

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
Pin the exact event ordering in Section 4.

**Verification:** `cargo test -p mtgml-rules basic_priority` and
`cargo test -p mtgml-conformance basic_priority` (expected RED recorded).

**Commit boundary:** Basic Priority RED cases only.

**HARD STOP:** no automatic pass, fallback actor, default response, or
Decision creation from an unproven pass-only surface.

### Task 7 — implement Basic Priority in the rules kernel

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

**Verification:** Task 6 RED tests GREEN; rules/conformance package suites,
generated contract drift/schema checks, `scripts/run_checks.py fast`.

**Commit boundary:** rules-owned Basic Priority implementation and focused
conformance, no environment player API in this commit.

**HARD STOP:** `PriorityState` changes without a matching typed event/cursor
proof, or Decision and priority state disagree at any accepted boundary.

### Task 8 — Reference Magic player response path

**Allowed files:** `crates/mtgml-environment/src/reference.rs`, shared
transaction module, endpoint/error mapping, environment player endpoint tests.

**Forbidden:** second commit pipeline; public trusted execution; changes to
Synthetic semantics; any Magic-specific rule calculation in environment code.

**Objective:** make Reference project the pass Decision only to the actor and
submit via existing Layer A/B validation and the shared S3.0 transaction.
Return `PlayerStepV2`; map invalid/stale/no-decision/closed responses through
existing closed public codes. `execute_trusted_response` remains internal and
uses the same transaction for replay. No trusted kernel error or hidden state
is returned to a player.

**Verification:** Task 1 RED passes; actor/nonactor projection; all closed
rejection codes; replay and environment existing tests; `cargo test -p
mtgml-environment --all-features`; `scripts/run_checks.py fast`.

**Commit boundary:** Reference endpoint wiring, kept in PR C with Tasks 6–9.

**HARD STOP:** public API invokes `execute_trusted_response`, endpoint sees an
internal `DecisionId`/binding, or Reference and Synthetic use distinct
response-commit implementations.

### Task 9 — S3.B end-to-end priority evidence and lifecycle

**Allowed files:** Basic Priority conformance, environment tests, current
status/registry files only after evidence passes.

**Forbidden:** Draw execution; marking interactions with Draw satisfied; S2
coverage promotion.

**Evidence:** explicit active then opponent pass sequence; SBA gate before each
window; no-priority Untap/Cleanup unchanged; all selected priority-bearing
boundaries require pass-only validation; pass Decision soundness and
completeness; checkpoint/restore; fork parity; backend V5 replay/reprojection;
paired hidden-state bytes; every rejection path atomic.

**RED test / evidence failure:** a mutant automatic pass, mismatched
PriorityState/pending Decision pair, or invalid pass-only state must be rejected
without commit; missing replay/parity evidence keeps lifecycle `specified`.

After S3.B evidence review, update Basic Priority to `implemented`; retain its
Foundation V2 dependency closure and leave Draw specified.

**Verification:** `cargo test -p mtgml-rules --all-features`,
`cargo test -p mtgml-environment --all-features`,
`scripts/run_checks.py integration`, status/registry/docs/schema validation.

**Commit boundary:** evidence + Basic Priority lifecycle/status sync; PR C.

**HARD STOP:** any pass is implied, or priority can be granted before SBA and
the pass-only proof.

### Task 10 — freeze Draw event disposition

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
or cannot validate the context-transition-context pairing, block Task 10 and
re-open the event decision before code.

**Verification:** conformance contract review; `scripts/check_documentation.py`
and `git diff --check`.

**Commit boundary:** a small design-only decision commit inside PR D.

**HARD STOP:** event evidence relies on a trace-only event or ignores another
admitted Library-to-Hand producer.

### Task 11 — RED: ordinary Draw × S2 interaction

**Allowed files:** Draw conformance cases, Rules/Environment test setup, and
paired-world tests.

**Forbidden:** Draw production implementation, registry/status edits, Basic
Priority changes, second Library-to-Hand executor.

**RED tests:** valid turn `>= 2`, Draw position, active owner, no priority/no
pending Decision, nonempty exact-top Library; exactly one S2 transition and
fresh incarnation; no RNG; owner private knowledge; opponent redaction;
position remains Draw with `HeldBy(active,0)` plus exact pass Decision;
Restore completed state cannot redraw. Reject all Task 5/Design exclusions and
S2 rejection, with full environment/player fingerprint equality.

Paired hidden worlds vary physical and definition identities. Opponent
Observation, InformationState, Decision, PlayerStep, observed events and
sequence behavior must match; only authorized owner-private products differ.

**Verification:** relevant `mtgml-rules`, `mtgml-conformance`,
`mtgml-environment` filters (expected RED recorded); noninterference suite.

**Commit boundary:** Draw RED tests only.

**HARD STOP:** any decision is fabricated for Draw, or tests depend on test-only
knowledge/history to prevent repetition.

### Task 12 — implement Draw through S2

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

**Verification:** Task 11 RED GREEN; S2 conformance regression; exact
checkpoint/delta/cursor/event/rejection tests; generated contract and schema
drift checks; `scripts/run_checks.py fast`.

**Commit boundary:** Draw producer and focused kernel evidence, no final replay
claim until Task 14.

**HARD STOP:** any accepted state exposes moved card + Draw + Priority=None, or
the implementation moves a card without invoking the S2 executor.

### Task 13 — Upkeep pass → Draw → priority integration

**Allowed files:** environment turn/priority/Draw integration tests and
conformance only.

**Forbidden:** environment scheduler loop, fake responses, direct state
mutation, S2 lifecycle update.

**RED/GREEN witness:** start `Beginning(Upkeep)`, `HeldBy(active,0)`, active
pass Decision. Active pass produces opponent pass Decision. Opponent's real
pass closes Upkeep; shared transaction calls `advance_forced_progress` exactly
once. That kernel call reaches Draw, S2, SBA, and active `HeldBy(active,0)`
with the next explicit pass Decision. Assert exact event/revision order and
one transition product.

Test terminal SBA and typed unsupported outcomes stop the advance correctly.

**Verification:** environment/conformance focused tests, package suites,
`scripts/run_checks.py fast`.

**Commit boundary:** integrated response-to-Draw witness, in PR D.

**HARD STOP:** temporal successor skips Draw, draw priority is passed
implicitly, or the environment performs a second progress call.

### Task 14 — authoritative Replay V5 witness for S2

**Allowed files:** environment backend replay, replay parity tests, conformance
evidence. No replay DTO version change.

**Forbidden:** Replay V6, forced-progress replay step, event-as-input, fake
DecisionResponse, test-only replay executor.

**Evidence:** export the real two-pass V5 replay; execute it from the exact
initial checkpoint with Reference backend. Prove replay re-applies the
opponent's second pass and repeats the integrated forced consequence. Compare
the exact new `GameObjectId`, S2 `ZoneTransition`, physical continuity/order,
knowledge and perspective identity, event order, delta, status, counters,
full-state digest, after checkpoint identity, next Decision and player
projections. Restore/fork/rerun produce the same proof.

**RED test / evidence failure:** mutate each recorded after identity and the
replayed fresh object/knowledge/event result; backend replay must reject every
divergence. Structural V5 validation alone is not a passing witness.

**Verification:** `cargo test -p mtgml-environment authoritative_s2_replay`,
all V5 replay tests, `scripts/run_checks.py integration`.

**Commit boundary:** backend-verified replay witness; no S2 promotion in this
commit.

**HARD STOP:** V5 only structurally validates, but backend execution does not
reproduce the exact S2 transition and after identity.

### Task 15 — S3 interaction and information evidence; Draw lifecycle

**Allowed files:** conformance cases and, after evidence review, Draw registry
lifecycle and status test/docs entries.

**Forbidden:** S2 coverage promotion; certification; new Draw/priority public
Decision/event shape.

**Evidence:** close `turn-structure × draw-card`,
`draw-card × zone-incarnation`, and cross-priority/SBA ordering. Include owner
exact knowledge, opponent paired-world byte parity, checkpoint restore before
and after Draw, forks, rejection nonmutation, no RNG, no Decision for Draw,
and Task 14 authoritative V5 replay.

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
PR D.

**HARD STOP:** owner/opponent information differs outside S2 authorization or
S2 replay evidence is missing.

### Task 16 — independent S2 interaction and coverage review

**Allowed files:** evidence/status review artifacts and S2 lifecycle surfaces
only after the reviewer accepts every gate.

**Forbidden:** retroactive test weakening, automatic coverage due to merged
code, certification, changing S2 semantic scope in-place.

**Review bundle:**

```text
state-based-actions-combat × zone-incarnation = SATISFIED_EVIDENCE
draw-card × zone-incarnation = SATISFIED_EVIDENCE
authoritative V5 replay of real S2 transition = PASS
all S2 accepted-design evidence remains valid at exact head
```

**RED test / evidence failure:** independent review rejects either interaction,
V5 backend replay diverges, or any accepted S2 gate is absent; S2 remains
`implemented / not covered`.

If every gate passes, a separate authorized lifecycle decision may update S2
from `implemented / not covered` to `covered`. If any gate is missing, record
the precise blocker and keep S2 not covered. This is an independent review,
not an implementation-task assumption.

**Verification:** rerun the complete S2-specific conformance/replay suite and
the required exact-head status checks on the reviewed candidate.

**Commit boundary:** separate governance/evidence PR E; no bundled Draw code.

**HARD STOP:** reviewer rejects or any required S2 coverage gate is not PASS.

### Task 17 — lifecycle and interaction status closure

**Allowed files:** capability registry lifecycle fields, README, roadmap,
current-status tests, and accepted evidence/status artifact references.

**Forbidden:** semantic code changes, capability dependencies, certification,
claims beyond the exact witnessed scope.

**Objective:** synchronize lifecycle counts and statuses to merged evidence:
SBA, Basic Priority and Draw implemented only if their individual evidence
passed; turn structure remains covered; zone incarnation becomes covered only
if Task 16 passed. Keep explicit S2 replay and interaction results factual.
Update generated status/contract artifacts only through their authority.

**RED test / evidence failure:** status assertions deliberately compare the
registry, README and roadmap against stale or overclaimed states and must fail;
final status tests pass only for evidence-backed lifecycle values.

**Verification:** focused status tests, docs, schemas, maintainer artifacts,
contract generation/drift, `scripts/run_checks.py integration`.

**Commit boundary:** status-only synchronization following Task 16's review.

**HARD STOP:** any unsupported lifecycle/coverage claim appears in registry,
README, roadmap, generated artifacts, or tests.

### Task 18 — cumulative exact-head verification

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

### Task 19 — implementation PR preparation

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

## 11. Required conclusions

```text
S3_ORDER =
S3.0 shared response transaction
-> S3.A state-based-actions-combat × zone-incarnation
-> S3.B basic-priority × turn-structure × state-based-actions-combat
-> S3.C draw-card × turn-structure × zone-incarnation
-> S3.D end-to-end Upkeep passes -> Draw -> SBA -> priority -> V5 replay

S3_A = bounded simultaneous selected SBA fixed point, terminal mapping, S2 battlefield/graveyard moves
S3_B = explicit pass-only priority, typed priority event/cursor, bound ChooseOne PassPriority Decision
S3_C = exactly-once ordinary Draw Step via S2; no DrawCompleted event

SHARED_RESPONSE_TRANSACTION_REQUIRED = YES
REFERENCE_MAGIC_PLAYER_RESPONSE_PATH_REQUIRED = YES
ENVIRONMENT_FORCED_PROGRESS_LOOP_REQUIRED = NO
ONE_KERNEL_FORCED_ADVANCE_SUFFICIENT = YES, if it runs all forced work to next Decision/outcome/error
PRIORITY_EVENT_REQUIRED = YES
DRAW_COMPLETED_EVENT_REQUIRED = NO
DRAW_PROGRESS_STATE_REQUIRED = NO
REPLAY_V6_REQUIRED = NO
S2_SBA_INTERACTION_CLOSABLE = YES, within the reviewed S2 admitted zone-order profile
S2_DRAW_INTERACTION_CLOSABLE = YES
S2_AUTHORITATIVE_REPLAY_PATH_CLOSABLE = YES, through a real response transaction plus backend V5 replay
S2_COVERED_PROMOTION_POSSIBLE_AFTER_PLAN = YES, only after independent review and all accepted S2 gates pass
S3_IMPLEMENTATION_PLAN_READY = YES
S3_IMPLEMENTATION_AUTHORIZED = NO
```

The same-owner simultaneous Graveyard cardinality guard is an explicit S3.A
admission boundary. If its compatibility with the accepted Foundation V2
scope is rejected at Task 3, stop before implementation and update the
accepted scope/authority through its governance path; do not silently weaken
simultaneity or invent ordering.
