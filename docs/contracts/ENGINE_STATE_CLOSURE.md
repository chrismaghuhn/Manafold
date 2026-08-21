# Engine State Closure

**Status:** accepted state-closure contract; M2 field refinements are freeze candidates  
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

It validates at least:

- player references;
- object/location and stack bijections;
- global internal allocator monotonicity;
- authoritative pending decision/candidate binding integrity;
- continuation reference/stage/payload consistency;
- knowledge/history/provenance relationships joined through the sole live mapping in `PerspectiveIdentityState`;
- opaque mapping bijections and retirement;
- perspective-local allocator monotonicity;
- Commander/format structural references;
- RNG identity/state.

An invariant failure is an implementation defect, not a legal game outcome.

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

M2 changes authoritative execution/knowledge/perspective-identity meaning and therefore requires a new V3 full-state identity. Historical V1/V2 state/checkpoint identities are never reinterpreted against the changed runtime `EngineState`.

The detached V3 semantic digest mapping is specified in [`../STATE_HASHING.md`](../STATE_HASHING.md).
