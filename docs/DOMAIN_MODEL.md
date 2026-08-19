# Domain Model

**Status:** accepted vocabulary; detailed Magic semantics deferred to M3  
**Stability:** normative identity/state model

## Identity families

The engine separates identity by meaning:

| Type | Meaning | Survives zone change? | Player-visible? |
|---|---|---:|---:|
| `CardDefinitionId` | immutable authored card definition | yes | only when definition is known |
| `PhysicalCardId` | concrete deck card or persistent Commander designation | yes | never directly |
| `GameObjectId` | one current rules-object incarnation | no | never directly |
| `AbilityInstanceId` | one authoritative ability instance | context-dependent | never directly |
| `StackObjectId` | one spell/ability object on the stack | no | never directly |
| `EffectInstanceId` | one executing or delayed effect instance | no | never directly |
| `TriggerInstanceId` | one detected trigger instance | no | never directly |
| `ContinuationId` | serialized suspended execution frame | no | never directly |
| `DecisionId` | one pending player decision | no | only through the authorized request |
| `RuleEventId` | one authoritative semantic event | no | never directly |
| `OpaqueObjectId` | perspective-specific visible object identity | while visibility contract permits | yes to that perspective only |
| `OpaqueAbilityId` | perspective-specific visible ability identity | while visibility contract permits | yes to that perspective only |

A zone transition creates a new `GameObjectId`. A physical card may persist. Last Known Information is captured from the old incarnation before the transition is committed.

## Complete authoritative state

`EngineState` is the complete semantic input to a transition:

```text
EngineState
├── revision
├── core rules state
├── zones, objects, stack records and ordering
├── deterministic identity allocators
├── pending decision, continuations, effects and triggers
├── typed RNG stream keys and raw-word cursors
├── per-player knowledge ledgers
├── per-player opaque identity mappings
└── format state, including Commander ledgers
```

A kernel object may hold derivable caches, but caches cannot affect legal choices, transition result, event ordering, digest, replay, or projection.

## Zone model

A `ZoneLocation` includes:

- zone kind;
- relevant player/owner partition when applicable;
- ordered position or explicit unordered marker;
- visibility partition;
- optional group/partition identity for cases such as separated face-down exile groups.

`ZoneState` must prove a bijection between live objects and their locations. Ordered zones use one authoritative ordering. Stack records and stack order must be mutually consistent.

## Knowledge model

Player knowledge is state, not a transient projector cache. It records what identity/location/definition a perspective is entitled to retain, why it learned it, and how/when that knowledge is invalidated. Visible history counters are perspective-local; global event counts are forbidden as observable metadata.

## Format state

Format-specific semantic state is nested in `EngineState`. Commander designation, cast counts, Commander damage, and format-specific pending choices cannot live in mutable controllers or adapters.

## Invariant ownership

- local types validate shape and local constraints;
- `validate_engine_state` validates all cross-component relationships;
- accepted transitions validate the resulting state before commit;
- checkpoint/restore and replay readers validate before exposing state;
- an invariant failure is an implementation defect, not a legal game outcome.
