# Domain Model

**Status:** accepted vocabulary; M2 identity refinements are freeze candidates; detailed Magic semantics deferred to M3  
**Stability:** normative identity/state model

## Identity families

The engine separates identity by meaning:

| Type | Meaning | Survives zone/incarnation change? | Player-visible? |
|---|---|---:|---:|
| `CardDefinitionId` | immutable authored definition identity | yes | only through a player-safe known-definition field |
| `PhysicalCardId` | concrete deck card / persistent designation identity | yes | never directly |
| `GameObjectId` | one current authoritative rules-object incarnation | no | never directly |
| `AbilityInstanceId` | one authoritative ability instance | context-dependent | never directly |
| `StackObjectId` | one authoritative stack object | no | never directly |
| `EffectInstanceId` | one executing/delayed effect | no | never directly |
| `TriggerInstanceId` | one detected trigger instance | no | never directly |
| `ContinuationId` | one trusted serialized staged-action identity | across its stages only | never directly |
| `DecisionId` | one trusted authoritative pending-decision identity | no | never in M2 player DTOs |
| `PlayerDecisionIdV1` | one perspective-local visible request identity | no | yes to that perspective only |
| `CandidateIdV1` | one candidate within one visible request | no | yes to that request only |
| `RuleEventId` | one authoritative semantic event | no | never directly |
| `VisibleSequence` | next/assigned perspective-local observed-history sequence | checkpointed monotonic state | yes to that perspective |
| `OpaqueObjectId` | perspective-specific distinguishable object identity | while distinguishability contract permits | yes to that perspective only |
| `OpaqueAbilityId` | perspective-specific distinguishable ability identity | while distinguishability contract permits | yes to that perspective only |

A zone transition creates a new `GameObjectId`. A physical card may persist. Last Known Information is captured from the old incarnation before transition commit.

Opaque player-visible identity is deliberately independent of authoritative incarnation identity: it may be remapped across a new `GameObjectId` while a perspective can still distinguish the object, and must be retired/replaced when hidden randomization destroys that distinguishability.

## Complete authoritative state

`EngineState` is the complete semantic input to a transition:

```text
EngineState
├── revision
├── core rules state
├── zones, objects, stack records and ordering
├── trusted/global identity allocators
├── authoritative pending decision
├── typed continuations, effects and triggers
├── typed RNG stream keys and raw-word cursors
├── per-player retained knowledge + next visible sequence
├── per-player opaque mappings + visible allocators + retired IDs
└── format state
```

No kernel/projector/controller/adapter object may retain hidden mutable semantic state.

Derivable caches are allowed only when discardable and incapable of affecting legal choices, transitions, event order, digest, replay, checkpoint behavior, or player projection.

## Decision identity

M2 separates trusted/request-local identity:

```text
DecisionId
    trusted global authoritative identity

PlayerDecisionIdV1
    allocated from perspective-local state
    identifies one visible request

CandidateIdV1
    dense after canonical public ordering
    valid only inside that request

ContinuationId
    trusted identity shared by staged requests
```

Global decision/object/event allocator history cannot be used to create player-visible IDs.

## Zone model

A `ZoneLocation` includes:

- zone kind;
- relevant player partition when applicable;
- ordered position or explicit unordered marker;
- visibility partition;
- optional group/partition identity.

`ZoneState` proves a bijection between live objects and locations. Ordered zones have one authoritative ordering. Stack records/order are mutually consistent.

A hidden location does not by itself determine whether a previous opaque identity survives; the rules/information contract separately determines distinguishability.

## Knowledge model

Player knowledge is authoritative state, not a projector cache.

M2 retained knowledge represents:

- current observation-independent known identity/definition;
- current known location where authorized;
- ordered historical location facts;
- provenance via perspective-local visible sequence;
- explicit typed invalidation/retirement.

Active object knowledge is associated with that perspective's opaque identity. `PerspectiveIdentityState` alone owns the live `OpaqueObjectId -> GameObjectId` association; knowledge does not duplicate it. That mapping may move to a new authoritative incarnation while the opaque identity persists.

A global event count is forbidden as observable knowledge metadata.

## Perspective identity model

Each perspective owns:

- active object/ability mappings;
- retired visible IDs;
- next opaque object/ability IDs;
- next player-decision ID.

These are checkpointed semantic state.

Projection is read-only and cannot allocate identity. Retired opaque identities are never reused in an episode.

## Format state

Format-specific semantic state is nested in `EngineState`. Commander designation, cast counts, damage and format-specific pending choices cannot live in mutable controllers/adapters.

M2 does not implement or certify real Commander semantics merely because structural format state exists.

## Invariant ownership

- local types validate local shape/ranges;
- `validate_engine_state` validates cross-component relationships;
- accepted transitions validate the candidate state before commit;
- perspective projections required for commit are validated before commit;
- checkpoint/restore/replay readers validate before exposing/using state;
- invariant failure is an implementation defect, never a legal game result.
