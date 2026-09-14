# Pre-M3 Remediation Batch E: Cross-Layer Runtime and Replay Closure

**Status:** reviewed and approved for implementation
**Date:** 2026-09-14
**Base:** `9996cfd0fcd4ef67d98cb0422611cebabd20e46b`
**Branch:** `chris/pre-m3-remediation-batch-e-cross-layer-closure`
**Scope:** Issue #164 findings FND-007, FND-008, FND-012B, FND-020, FND-022B,
FND-022E, FND-023, FND-024, FND-025, and FND-026B-FND-026D
**Review provenance:** user review approval on 2026-09-14 after the four requested
scope corrections.

## Goal

Close the confirmed cross-layer runtime, checkpoint, replay, and
per-perspective evidence gaps that can be resolved under the current M2
contracts. Preserve the existing Rust-owned semantic kernel, environment
transaction boundary, V3 identity shape, historical replay meanings, and
information-safety rules. Record unresolved product semantics precisely rather
than inventing a new public abstraction.

The user-approved compatibility decision is:

```text
PUBLIC_API_CHANGE = NO
WIRE_CHANGE = NO
SCHEMA_CHANGE = NO
DIGEST_DOMAIN_CHANGE = NO
RNG_ALGORITHM_CHANGE = NO
HISTORICAL_REPLAY_CHANGE = NO
NEW_MAGIC_SEMANTICS = NO
M3_STARTED = NO
M3_AUTHORIZED = NO
```

## Authority and ownership

The implementation follows the accepted ownership boundaries in
`RULES_SEMANTICS.md`, `EXECUTION_MODEL.md`, `INFORMATION_MODEL.md`,
`REPLAY_AND_DETERMINISM.md`, `contracts/ENGINE_STATE_CLOSURE.md`,
`ML_ENVIRONMENT.md`, `STATE_HASHING.md`, `DECISION_PROTOCOL.md`, ADR 0035,
ADR 0039, ADR 0040, ADR 0041, and ADR 0049.

| Finding | Normative owner | Production owner | Current executable behavior | Design disposition |
|---|---|---|---|---|
| FND-007 | State closure plus transaction ownership | `mtgml-state` lifecycle; rules/environment commit | The lifecycle seam validates its local knowledge/identity result. The environment and rules transition boundary validate the complete candidate before commit. Fixture callers intentionally stage physical and perspective mutations separately. | Characterize the staging boundary and retain `RESOLVED_ON_BASE` for the remaining question if no successful production path escapes the atomic owner. |
| FND-008 | Rules semantic event/delta proof | `mtgml-rules::validate_transition_contract` and semantic cursor | Reachable synthetic mutations are covered by life, object, decision, RNG, lifecycle, delta, and allocator checks. Unsupported effect/trigger and turn/priority/format families are rejected or unreachable in M2. | Produce a field-family matrix. Fix only a reachable unexplained mutation; otherwise record `RESOLVED_ON_BASE / NOT_REACHABLE_CURRENT_FOUNDATION`. |
| FND-012B | Rules event/presentation pairing | `mtgml-rules::validate_occurrence_pairing` | `AnnouncedOutcome` is a separate nonempty public presentation policy and is not paired with RNG. | Characterize separately from random outcomes and retain `REJECTED` if the accepted synthetic presentation contract is confirmed. |
| FND-020 | Replay provenance and API lifecycle | Current environment replay manifest producer | V3 validation fixes most current IDs, but producer configuration can supply a false current schema identity, especially the observation identity. | Add one producer-owned current-identity guard. Keep detached historical readers less restrictive where their contract permits it. |
| FND-022B | Complete checkpoint/replay identity | `EnvironmentCheckpointV3`; replay manifest/deck boundary | `EpisodeStatus::validate()` rejects duplicates but does not compare closed outcomes with the authoritative player set. | Add a generic exact player-universe check at checkpoint and V3 replay boundaries. Do not add state knowledge to `mtgml-model`. |
| FND-022E | Explicit trusted replay-control trace | Environment replay executor | Replay fields exist, but replay currently carries resource and wall-clock values forward and rejects resealed forward progress instead of applying the recorded values. | Apply validated recorded external counters to the replay-owned backend through the existing checkpoint restore path. Never read host time. |
| FND-023 | Replay evidence boundary | `AuthoritativeReplayV3::validate`; environment replay executor | Structural replay validation and backend execution are separate, but the distinction is not explicit enough in the API/docs. | Document and test `validate()` as detached structural validation; use `ReplayExecutionReport` as backend/checkpoint-verified evidence. |
| FND-024 | Layered rejection contract | Environment endpoint, rules kernel, replay recorder | Wire and player semantic rejections produce no authoritative replay step. A trusted `accepted=false` transition is nonmutating and can be represented as an explicit diagnostic replay step. | Record one unified layer A-D matrix and preserve the current policy. No player rejection is silently added to authoritative replay. |
| FND-025 | Canonical identity encoding | V3 checkpoint/replay boundaries and checkpoint digest | Checkpoint digest sorts player outcomes during encoding; V3 manifest validation does not reject a permuted deck list, and the shared `EpisodeStatus` model must not acquire a new global ordering rule. | Reject noncanonical authoritative keyed-array order at V3 checkpoint/replay boundaries. Preserve the defensive sort inside the V3 digest encoder. Construction-time producer configuration may be sorted before identity is formed. |
| FND-026B | M2 per-perspective product validation | Environment precommit product | Every projected observed envelope is validated, but `PlayerStepV2.submission` has no neutral non-actor meaning. | `BLOCKED_CONTRACT_AMBIGUITY`; do not invent a non-actor `PlayerStepV2`. |
| FND-026C | M2 environment API lifecycle | Player endpoint/API boundary | `submit(response) -> PlayerStep` is actor-bound and no live non-actor delivery API is promised. | `DEFERRED_P2`; no queue, mailbox, polling method, callback, or hidden controller state. |
| FND-026D | Replay projection parity | Production lifecycle projector plus environment replay trace | Existing tests reproject live products and construct both perspective envelope batches, but do not execute an eventful replay trace and compare a non-actor batch. | Add a test-only eventful situation generator. Reprojection uses the production projector on replay `before/events/after`. |

## Design

### E1: runtime semantic closure

Add focused characterization tests before any production fix. FND-007 tests
will contrast the public lifecycle staging primitive with the actual
environment/rules atomic owner. The primitive must remain usable for the
existing fixture ordering in which a physical incarnation transition precedes
its lifecycle occurrence. Full `validate_engine_state()` remains the complete
transition/checkpoint boundary, not an unconditional postcondition of the
lower-level staging helper.

FND-008 tests will probe only current authoritative mutation families. The
matrix will classify revision, core player values, active/priority/turn fields,
zones and locations, stack, allocators, execution records, random state,
knowledge, perspective identities, and format state as semantic-event-owned,
derived bookkeeping, immutable, unreachable, or contract-ambiguous. No
placeholder `StateChanged` event or M3 event family will be added. A reachable
unexplained accepted mutation remains a fail-closed rules-contract defect and
requires a RED test before a minimal fix.

FND-012B remains independent of RNG causality. `SawRandomOutcome` continues to
require its immediately preceding `RandomValueSampled` proof. `AnnouncedOutcome`
is accepted only as the existing separate nonempty presentation occurrence
whose own lifecycle occurrence is its authoritative state change. No universal
announcement event or RNG identity is inferred.

### E2: replay and checkpoint identity closure

#### Producer identity

The current synthetic environment producer gets one central validation helper
for the schema and contract IDs it actually emits. It compares the configured
values against the current Decision, Observation, Information, Observed Event,
PlayerStep, Replay Step, and RNG constants before creating the manifest. A
false producer configuration fails closed. Detached V3 validation continues to
accept only the restrictions belonging to the detached V3 contract; it does not
turn a historical reader into a current implementation probe.

#### Closed status player universe

`EpisodeStatus::validate()` remains the shared local V1/model validation: it
checks closed enum data and duplicate keys, but it does not impose a new global
serialization order on the shared `EpisodeStatus` type. V3-authoritative
checkpoint/replay boundaries separately require ascending keyed-array order and
the exact player universe. Those boundary checks compare:

```text
set(closed_status.players.player)
==
set(authoritative_player_universe)
```

The checkpoint boundary uses `EngineState.core.players.keys()`. The detached
V3 replay boundary uses the manifest deck-player set. The V3 checkpoint digest
encoder may continue to sort `player_outcomes` defensively, as required by the
existing `STATE_HASHING.md` contract; the authoritative checkpoint/replay
boundary rejects an unsorted input before it becomes authoritative. This keeps
the accepted digest helper and all existing V3 KAT bytes stable without
globalizing a V3 rule into the shared V1/model validator. This handles terminal,
truncated, concession, draw, win/loss, eliminated, unresolved, and future
multi-player states without assuming two players. `Running` has no outcome
set. Missing, foreign, and duplicate outcomes fail closed before trusted
restore/commit.

#### External replay counters

`ReplayStepV3.environment_limit_counters_after` is already the explicit trusted
trace input. For a rejected trusted execution, every checkpoint identity field,
including both external counters, remains unchanged. For an accepted execution:

1. reconstruct and validate the deterministic before checkpoint;
2. execute the recorded typed response on the replay-owned backend;
3. validate the transition and deterministic decision/acceptance/event counters;
4. validate nondecreasing recorded resource and wall-clock counters;
5. apply the recorded external values to the replay-owned checkpoint/backend
   through the existing trusted checkpoint restore path;
6. recompute the checkpoint digest from the resulting state, status, counters,
   and codec identity;
7. compare the complete recorded after identity.

Replay never samples host time and never derives external counters from game
semantics. The current foundation has no reviewed threshold/status-transition
contract for a counter crossing. A replay that claims external-counter-driven
technical truncation or another status change therefore fails closed as an
unsupported contract case; it does not invent a limit policy or player
outcomes.

#### Structural versus verified replay

`AuthoritativeReplayV3::validate()` is explicitly documented as detached
structural validation. It proves schema, canonical ordering, manifest/deck
consistency, digest-chain shape, and DTO-level counter/revision rules. It does
not prove that a real backend has the recorded state, pending request,
transition, or player projection.

`TrustedEnvironmentController::execute_replay_from_checkpoint()` executes on a
forked backend and returns `ReplayExecutionReport`. Its traces and final
checkpoint are the backend/checkpoint-verified evidence. Privileged state,
replay traces, and checkpoint identities remain outside player endpoints.

#### Rejection policy

The replay contract records authoritative execution attempts, not every input
received by a player endpoint.

| Layer | Example | Replay step | Counters/state | PlayerStep |
|---|---|---:|---|---|
| A | malformed/noncanonical wire bytes | no | no mutation | no |
| B | stale, invalid candidate, wrong answer, closed endpoint submission | no | no mutation | typed rejected mirror for the actor |
| C | trusted kernel returns `accepted=false` | only when explicitly represented as a diagnostic step | complete checkpoint identity unchanged | no endpoint product is synthesized by replay |
| D1 | accepted transition whose already-defined deterministic limit result is `Truncated` | yes; the committed after status and complete counters are recorded | committed according to the existing accepted-transition contract | the normal actor product, with the truncated status |
| D2 | aborted internal/service failure before commit | no | candidate discarded; no mutation | closed service failure |
| D3 | external-counter-driven truncation without a reviewed threshold/transition contract | no successful replay step; fail closed as unsupported | no invented status or outcomes | no player product |

The current recorder/executor remains capable of validating a deliberately
recorded layer-C `accepted=false` diagnostic step. Live layer-B rejection does
not enter accepted replay history, does not advance `decisions_submitted`, and
is not trajectory data by this batch. No reward or training policy is added.

### E3: multi-perspective product closure

The current `PlayerStepV2` is actor-submission-bound because its
`submission` field describes the result of that perspective's typed response.
There is no neutral value for a perspective that did not submit. Batch E will
not overload `Accepted`, `Rejected`, or `UnavailableDecision` to mean a
non-actor transition product. FND-026B therefore remains blocked pending a
separately reviewed contract decision or versioned product.

FND-026C is deferred. The foundation exposes actor-bound pull/submit calls and
the existing replay/trajectory derivation path; it does not promise live
asynchronous non-actor event delivery. No mutable mailbox or queue is added to
`EngineState`, the controller, the checkpoint, or replay.

FND-026D will use a test-only backend/fixture to create one eventful accepted
situation. The fixture is only a situation generator. The live capture and
replay verification both call the existing production lifecycle projector and
the existing replay executor. The test compares exact canonical bytes for P1
and P2, verifies both batches are nonempty and distinct where the fixture
policy makes them distinct, checks the absence of trusted identifiers/RNG
provenance, and proves the original live controller and recorder remain
unchanged by replay execution.

### E4: bounded integration matrix

After independent E1-E3 evidence, add one bounded matrix covering accepted
eventless and eventful transitions, player rejection, trusted rejection,
terminal/truncated checkpoint validation, deterministic and trusted external
counter replay, current/tampered producer identities, tampered actor and
player-decision identity, keyed-array tampering, and non-actor replay
reprojection. Cases blocked by the unresolved non-actor `PlayerStepV2` meaning
will be reported as exact blocked contract cases rather than simulated with a
new product type.

## Testing and evidence

Every confirmed production defect follows this sequence:

```text
characterization
→ visible RED test on BASE
→ smallest production fix
→ focused GREEN test
→ affected package tests
→ workspace and maintainer gates
```

Required focused evidence includes the probes listed in the Batch-E task for
FND-007, FND-008, FND-012B, FND-020, FND-022B, FND-022E, FND-023, FND-024,
FND-025, FND-026B, and FND-026D. Tests assert exact state, status, counters,
events, decision identity, replay-step presence or absence, checkpoint digest,
player-visible bytes, and replay source nonmutation where applicable.

The baseline command executed before this design was:

```text
cargo test --workspace --all-features --locked
```

It passed on the exact base. Final evidence will distinguish direct Cargo
profiles from `just check-fast`/`just check` wrapper status, Python/schema/wire/
maintainer gates, and hosted CI on the exact final PR head.

## Compatibility and security constraints

No change may expose `GameObjectId`, `PhysicalCardId` beyond an existing
trusted-only contract, `DecisionId`, `RuleEventId`, root seed, RNG stream key or
cursor, raw RNG words, checkpoint/full-state digest, hidden object identity or
order, or another player's knowledge through a player endpoint.

Validation and projection remain read-only. Canonical keyed-array validation
does not normalize persisted authoritative input. Configuration arrays may be
canonicalized before the manifest identity is formed, but the persisted
manifest/status validators reject noncanonical ordering and duplicate keys.

No historical Replay V1/V2 meaning is changed. No new public endpoint method,
wire field, schema version, digest domain, RNG algorithm, or M3 semantic is
introduced.

## Acceptance boundary

Batch E may finish with independent slices in different states. In particular:

- FND-026B remains `BLOCKED_CONTRACT_AMBIGUITY` until a neutral non-actor
  product meaning is reviewed;
- FND-026C is `DEFERRED_P2` under the current actor-bound API;
- E3/E4 cannot be reported fully green if those exact boundary conditions keep
  their section gates blocked;
- `M3_STARTED = NO`, `M3_AUTHORIZED = NO`, and `MERGE_PERFORMED = NO` remain
  mandatory.

The final PR body and response will use the exact disposition matrix required
by the Batch-E task, with no unexecuted gate reported as `PASS`.

## Implementation evidence checkpoint

The current code/evidence head is `d7eef2e`. E1 is dispositioned `PASS` with
FND-007 resolved on the base, the reachable FND-008 mutation families covered,
and FND-012B rejected as a separate presentation policy. E2 is dispositioned
`PASS`: current producer identities are guarded, V3 checkpoint/replay status
and keyed-array boundaries are fail-closed, external replay counters apply on
the internal replay fork, and detached structural validation is documented
separately from backend verification. E3 is `PARTIAL`: FND-026D has exact
eventful replay reprojection evidence, FND-026C is `DEFERRED_P2`, and FND-026B
is `BLOCKED_CONTRACT_AMBIGUITY` because `PlayerStepV2.submission` has no neutral
non-actor meaning. E4 remains pending the final bounded matrix and complete
verification profile. Final exact-head values and gate statuses are recorded in
the separate Batch-E dispositions/evidence document.
