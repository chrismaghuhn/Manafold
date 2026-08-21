# Player API Capability Matrix

**Status:** accepted capability boundary; M2 V2 refinements are freeze candidates  
**Stability:** normative

| Capability | Kernel | Trusted controller | Player endpoint |
|---|---:|---:|---:|
| Full `EngineState` | yes | checkpoint only | no |
| Authoritative events and exact state delta | yes | internal/replay | no |
| Root seed and RNG keys/cursors/raw words | yes | replay/configuration | no |
| Checkpoint / restore | internal | yes | no |
| Fork | internal | yes | no |
| Authoritative replay export | internal | yes | no |
| Current observation | projection source | orchestration | yes, bound perspective |
| Retained information state | projection source | orchestration | yes, bound perspective |
| Visible decision | projection source | orchestration | yes, acting perspective only |
| Observed events | audience source | orchestration | yes, redacted |
| Episode status | transition result | orchestration | yes, explicit step/status surface |
| Submit typed response | apply | schedule | yes, own visible decision only |
| Malformed wire decoding | no semantic call | transport/adapter boundary | closed wire error only |
| Trusted diagnostics | internal | yes | no |

A player handle is an owning shared handle, not a mutable borrow of the controller. Multiple handles may coexist. Each is permanently bound to one player and cannot select another perspective.

## M2 identity boundary

The player surface may expose perspective-local/request-local identities only:

```text
PlayerDecisionIdV1
CandidateIdV1
OpaqueObjectId
OpaqueAbilityId
```

It must not transitively expose:

```text
DecisionId
ContinuationId
GameObjectId
PhysicalCardId
AbilityInstanceId
RuleEventId
full-state/checkpoint/replay identity
RNG provenance
global allocator state
```

Player-visible opaque and player-decision IDs are allocated from perspective-local checkpointed state; their values cannot depend on hidden global allocation history.

## Information-state boundary

`PlayerInformationState` is current observation plus retained perspective knowledge. Episode status remains a separate environment/PlayerStep value and is excluded from the information-state digest.

## Error boundary

Three public layers remain distinct:

- wire decode failure: closed malformed-response code, no semantic `PlayerStep`;
- typed semantic rejection: closed player submission code and exact nonmutation;
- invariant/internal endpoint failure: closed service-unavailable code.

A private/wrong-actor request is not exposed as a distinct error oracle. Binding mismatch remains trusted/invariant failure.

## Projection purity

Player read APIs are deterministic and read-only. They cannot allocate opaque/player-decision IDs, advance visible sequence, mutate knowledge, consume RNG, append replay, or change environment counters.
