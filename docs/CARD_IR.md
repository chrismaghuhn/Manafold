# Card IR

**Status:** architectural direction accepted; executable vocabulary experimental
**Stability:** experimental pending an explicit reviewed capability scope

The immutable content envelope, content identity, provenance separation,
validation, and non-authorizing preflight are owned by the normative
[Card Definition and Content Contract V1](contracts/CARD_DEFINITION_CONTRACT.md).
That foundation does not make executable Card-IR vocabulary stable or admit
a semantic profile.

## Required properties

Future executable card definitions must be:

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
immutable CardDefinition content envelope
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

These are driven by a future reviewed deck closure and M3 authority cases. The existing Rust enum is illustrative scaffolding, not a support claim.

M4.1 freezes only the closed immutable content foundation. It does not
execute content or define a general Magic semantic language. The existing
`ExperimentalEffect` scaffold has no compatibility entitlement and is
removed or quarantined by its implementation contract; it is not promoted by
renaming it.

## Maintainer rule

When multiple cards require the same behavior, add or extend a reusable capability rather than duplicate card-specific logic. See [`cards/ADDING_CARDS.md`](cards/ADDING_CARDS.md) and [`rules/ADDING_RULES_AND_MECHANICS.md`](rules/ADDING_RULES_AND_MECHANICS.md).
