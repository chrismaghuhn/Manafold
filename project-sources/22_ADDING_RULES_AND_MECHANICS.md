# Adding Rules and Mechanics

**Status:** accepted maintainer workflow  
**Stability:** process contract

## Principle

General Magic behavior belongs to reusable rules/mechanic capabilities. A card definition declares and composes capabilities; it does not reimplement general rules.

## Workflow

### 1. Establish authority

Pin rules, Oracle examples, rulings, format policy, and known edge cases. State the supported scope and explicit exclusions.

### 2. Create or update a capability

Use:

```bash
python scripts/scaffold_capability.py rules/example-mechanic "Example Mechanic"
```

The proposal receives a stable capability key, category, lifecycle state, dependencies, owner role, spec path, and evidence placeholders.

### 3. Specify semantic surfaces

The mechanic specification must define:

- authoritative state and identities;
- events and replacement points;
- decisions, actors, cardinality, and ordering;
- public/private information effects;
- transaction/continuation behavior;
- interactions with zones, stack, costs, combat, SBA, layers, copy, and format state as applicable;
- unsupported cases and fail-closed behavior.

### 4. Write red conformance evidence

Add minimal cases before production behavior. Include ordinary, illegal, boundary, serialization, replay, information, and interaction cases. A new general capability normally needs property/fuzz hypotheses.

### 5. Implement the smallest reusable primitive

Do not add card-name checks or one-off state mutation. Extend typed state/events/decision/IR only as required by the accepted spec.

### 6. Validate all contracts

Run state invariants, event/delta parity, soundness/completeness, noninterference, replay/checkpoint parity, and performance diagnostics.

### 7. Advance lifecycle

A capability advances only with evidence:

```text
proposed -> specified -> implemented -> covered -> certified
```

Certification is bundle/snapshot specific. Changing semantics creates a new capability version or invalidates dependent certifications.

## When to extend core rules

Extend core rules only when the behavior is not card-specific and cannot be represented by existing primitives. Repeated native/card-specific implementations are evidence that a missing capability should be extracted.
