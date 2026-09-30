# Engine State Closure

**Status:** accepted state-closure contract including M2 field refinements; M2 executable closure is recorded as `COMPLETE` by accepted ADR 0041 at exact evidence head `352cd80c2ef58a406c30bf7db1cb792109fafc3f`
**Stability:** normative

`EngineState` is the complete semantic input to a transition:

```text
EngineState
├── revision
├── CoreRulesState
├── ZoneState
├── IdentityAllocatorState            # trusted/global allocators only
├── ExecutionState
│   ├── authoritative pending decision
│   ├── typed continuations
│   ├── effects
│   └── triggers/delayed effects
├── RandomState
├── KnowledgeState
│   └── per-perspective retained knowledge + next visible sequence
├── PerspectiveIdentityState
│   └── mappings + perspective-local opaque/player-decision allocators + retired IDs
└── FormatState
```

No kernel, projector, environment backend, adapter, or controller may retain hidden mutable semantic state outside this closure. Caches must be derivable, disposable, and semantically inert.

## M2 decision closure

The pending decision is an authoritative request, not a player DTO plus a separately drifting binding table.

The authoritative request contains:

- trusted `DecisionId`;
- perspective-local visible player-decision ID;
- revision and actor;
- closed decision domain;
- ordered authoritative candidates;
- exact visible intent and binding in each candidate;
- optional trusted continuation reference.

A serialized continuation and its stage/partial values live in `ExecutionState`. Controller callbacks, labels, closures, threads, or stack frames cannot be continuation authority.

## M2 information closure

Player knowledge is state, not projector cache.

The authoritative knowledge/identity closure includes:

- active and retired retained knowledge;
- current/historical known-location facts;
- typed provenance/invalidation;
- active opaque mappings, which solely own the current opaque→live-object relation;
- retired opaque IDs;
- next opaque object/ability IDs per perspective;
- next player-decision ID per perspective;
- next visible observed-event sequence per perspective.

Projection does not allocate or mutate any of these values.

A player-visible ID must not derive from global hidden allocation history.

## Validation ownership

`validate_engine_state()` owns cross-component validation. Component presence alone is insufficient.

For pending V2 decisions, `AuthoritativeDecisionRequestV2::validate()` owns
only local structural request validity. The state-owned
`validate_pending_authoritative_request()` boundary calls the exact candidate
binding check for every pending candidate, including scalar payload equality
and perspective-local object/ability resolver equality. `project_player_request()`
projects only after that authoritative boundary has passed; it is not a second
binding authority and never exposes trusted bindings.

It validates at least:

- player references;
- object/location and stack bijections;
- global internal allocator monotonicity;
- authoritative pending decision/candidate binding integrity;
- exact `CandidateOrderingV1`: candidate array already sorted by the frozen public semantic comparator, `candidate_id` equals its dense zero-based array index, no duplicate public ordering key exists, and `ChooseNumber` carries an empty candidate array;
- continuation reference/stage/payload consistency;
- knowledge/history/provenance relationships joined through the sole live mapping in `PerspectiveIdentityState`;
- opaque mapping bijections and retirement;
- perspective-local allocator monotonicity;
- Commander/format structural references;
- RNG identity/state.

An invariant failure is an implementation defect, not a legal game outcome.

ADR 0049 adds one chronology over each retained
record and rejects noncanonical ordered-zone representations. Acquisition,
history, current or last-known location, and invalidation are checked in
oldest-to-newest order with same-occurrence equality only for identical
Acquire-created provenance. Live ordered vectors are authoritative; every live
ordered location uses a Top offset equal to its vector ordinal, and empty
ordered-zone entries are invalid. The decision is accepted architecture; this
does not authorize M3, whose boundary remains unchanged.

## State delta

`StateDelta` contains a complete state replacement plus semantic audit trace. Applying it to the previous state reproduces the exact next state and full-state digest.

The reference contract prefers correctness/auditability over compactness. A later optimized backend may use compressed/reversible internal representation only after differential parity proves identical:

- acceptance/rejection;
- state digest;
- event order;
- next decision;
- per-player bytes;
- status.

## Versioning

Each state format has one current version and is changed in place (AGENTS.md
§4). The current aggregate is `EngineStatePartsV3`; `FullStateDigestV7` hashes
it through the V6 and V5 input layers specified in
[`../STATE_HASHING.md`](../STATE_HASHING.md).

## Combat state facts

`CombatState` records the attackers that became blocked, independently of live
blocker references, and a flag recording that the combat-damage turn-based
action completed. A blocked attacker remains blocked when its live blocker
reference is absent.

## Card-rules state families

The accepted M4 state-cut Semantic Spec defines the next current EngineState
shape with exactly six additional closed authoritative families:
`ManaState`, `TurnHistoryState`, `CounterState`, `AttachmentState`,
`FaceState`, and `AbilityAuthorityState`. Their owners, invariants, lifecycle,
cross-family validation, and external content-catalog joins are frozen in the
[Semantic Spec](../superpowers/specs/2026-09-26-m4-unified-state-cut-semantic-spec.md).
PR #248 crossed the plan's final activation boundary for the bounded M4.2
Mountain/Plains slice. These families remain part of the preserved state
substrate; their presence does not claim broader card/deck support.

## Current state closure

The current authoritative aggregate is `EngineStatePartsV3`, with
`ExecutionStateV4`, `PersistedExecutionV4`, `zones_v2`, typed stack records,
and typed continuation/effect/trigger authority. `StateDeltaV3` and
`AuthoritativeRuleEventV3` describe current transition products;
`FullStateDigestV7` binds the complete aggregate. `EnvironmentCheckpointV8`
and Replay V8 bind the successor digest and product identities. The current
runtime's executable admission remains the exact previously accepted M4.2
Mountain/Plains slice; this contract change does not implement G0 Shared
capabilities or broaden card support.
