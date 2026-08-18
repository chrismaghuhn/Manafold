# Card IR

**Status:** architectural direction accepted; concrete vocabulary experimental  
**Stability:** experimental until M2.5 capability census

## Required properties

Authoritative card definitions must be:

- typed, inspectable, serializable, deterministic, and versioned;
- free of arbitrary I/O, wall clock, global randomness, and hidden mutable state;
- composed from reusable rules/mechanic capabilities;
- explicit about targets, choices, dynamic values, bindings, durations, zones, visibility, and generated objects;
- statically checkable for unresolved references and unsupported capabilities;
- executable only through the rules kernel’s transition builder.

## Layering

```text
source provenance
    ↓
generated/review candidate
    ↓
typed Card IR definition
    ↓
capability validation and lowering
    ↓
reference rules kernel
```

The IR describes **what semantic program is requested**. The rules kernel owns **how Magic executes it**.

## Deferred concrete design

M0.2 intentionally does not freeze:

- final effect/condition/filter enum variants;
- cost and mana expression vocabulary;
- trigger event-binding syntax;
- continuous-effect layer/dependency representation;
- copy/copiable-value representation;
- native-executor calling convention.

These are driven by M2.5’s exact deck closure and M3 authority cases. The existing Rust enum is illustrative scaffolding, not a support claim.

## Maintainer rule

When multiple cards require the same behavior, add or extend a reusable capability rather than duplicate card-specific logic. See [`cards/ADDING_CARDS.md`](cards/ADDING_CARDS.md) and [`rules/ADDING_RULES_AND_MECHANICS.md`](rules/ADDING_RULES_AND_MECHANICS.md).
