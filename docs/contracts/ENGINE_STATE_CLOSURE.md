# Engine State Closure

**Status:** accepted state-closure contract  
**Stability:** normative


`EngineState` is the complete semantic input to a transition:

```text
EngineState
├── revision
├── CoreRulesState
├── ZoneState (objects, locations, ordered zones, stack records/order)
├── IdentityAllocatorState
├── ExecutionState (pending decision, continuations, effects, triggers)
├── RandomState
├── KnowledgeState
├── PerspectiveIdentityState
└── FormatState
```

No kernel object may retain hidden mutable semantic state. Caches must be derivable and disposable.

`validate_engine_state()` owns cross-component validation. Component presence alone is not sufficient. The validator checks player references, object/location and stack bijections, allocator monotonicity, pending decision bindings, continuation references, knowledge references, opaque-ID bijections, Commander ledger references, and RNG identity.

`StateDelta` contains a complete `EngineStateParts` replacement plus a semantic audit trace. The reference contract values correctness over compactness; a later rollout backend may use a compressed delta only after differential parity proves identical semantics.
