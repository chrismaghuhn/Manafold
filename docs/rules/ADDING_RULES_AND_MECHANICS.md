# Adding Rules and Mechanics

**Status:** accepted maintainer workflow  
**Stability:** process contract

## Principle

General Magic behavior belongs to reusable rules/mechanic capabilities. A card
definition declares and composes capabilities; it does not reimplement general
rules. Future M3 work is capability-first: semantic progress is measured by
bounded, proven slices and their applicable interaction obligations, not by
the number of cards that mention a mechanic.

## Semantic witnesses

A **Semantic Witness** is a concrete card, rules example, ruling example, or
minimal scenario used to expose and prove a reusable capability. A witness can
be external to Manafold and can be useful before any Card IR definition exists.
It is not itself a card definition, a support claim, or certification evidence
for a card or bundle:

```text
witness != supported
implemented != covered
covered != certified
```

Witnesses should be minimal and diagnostic. Prefer a small semantic surface,
clear authority, few unrelated abilities, and limited hidden dependencies over
a complicated card. Planning may classify witnesses as foundational, usage,
interaction, or adversarial. These are selection categories, not quotas, and
usage frequency is a prioritization signal rather than semantic authority. A
future selection review should also consider reuse, interaction centrality,
decision and information value, and bounded implementation cost; highest usage
is not automatically first.

## Bounded capability slices

Complete one semantic capability slice before expanding to unrelated semantic
breadth. A future slice should record, at the planning level:

- capability identity and included scope;
- pinned rules/ruling authority and explicit exclusions;
- the minimal witness set;
- RED conformance cases and the intended implementation boundary;
- decision, information, state, event, and delta obligations where applicable;
- replay, checkpoint, and fork evidence where applicable;
- reviewed interaction obligations at applicable semantic seams;
- fail-closed behavior for unsupported cases; and
- the evidence required before lifecycle advancement.

This list describes review anatomy, not a new schema. If an evidence class does
not apply, the scope must explain why. The high-risk domains identified by
Issue #129 are an evidence-priority map; one narrow covered slice does not
claim that its entire domain is supported.

## Interaction obligations

Isolated capability evidence does not prove composition. For each new slice,
inspect known semantic seams and create only the applicable, reviewed
obligations:

```text
POTENTIAL_SEAM
    -> REVIEWED_OBLIGATION
    -> SATISFIED_EVIDENCE
```

Closure is risk-based. It requires meaningful interaction review without
requiring every Cartesian pair or every theoretically possible Magic
combination. Higher-risk cases may require ordered multi-capability pipelines,
properties, bounded exploration, semantic fuzzing, or targeted mutants when
the reviewed scope calls for them.

External-engine comparisons may reveal a divergence, but they remain a
diagnostic signal. Validate the expectation against pinned Magic authority and
Manafold's accepted contracts; do not treat an external engine as a rules
oracle. In particular, parsed or compiled shape, many implemented cards, or a
playable game cannot replace explicit semantic and interaction evidence.

## Workflow

### 1. Establish authority

Pin rules, Oracle examples, rulings, format policy, and known edge cases. State the supported scope and explicit exclusions.

### 2. Create or update a capability slice

Use:

```bash
python scripts/scaffold_capability.py rules/example-mechanic "Example Mechanic"
```

The proposal receives a stable capability key, category, lifecycle state,
dependencies, owner role, spec path, and evidence placeholders. The reviewed
scope must remain bounded; the registry entry does not authorize a global
Magic-support claim or runtime dispatch.

### 3. Specify semantic surfaces

The mechanic specification must define:

- authoritative state and identities;
- events and replacement points;
- decisions, actors, cardinality, and ordering;
- public/private information effects;
- transaction/continuation behavior;
- interactions with zones, stack, costs, combat, SBA, layers, copy, and format state as applicable;
- applicable semantic seams and the reviewed interaction obligations for them;
- unsupported cases and fail-closed behavior.

### 4. Write red conformance evidence

Add minimal RED cases before production behavior. Include ordinary, illegal,
boundary, serialization, replay, information, and applicable interaction cases.
A new general capability normally needs property/fuzz hypotheses. The cases
must drive the real authoritative kernel through the existing conformance
facade; the test helper must not calculate Magic semantics independently or
silently choose missing decisions.

### 5. Implement the smallest reusable primitive

Do not add card-name checks or one-off state mutation. Extend typed state/events/decision/IR only as required by the accepted spec.

### 6. Validate all contracts

Run the evidence applicable to the declared scope: state invariants,
event/delta parity, decision soundness/completeness, noninterference,
replay/checkpoint/fork parity, interaction closure, and performance diagnostics.
Keep unsupported combinations explicit and fail closed. A passing isolated
case or a playable scenario does not close an unreviewed interaction
obligation.

### 7. Advance lifecycle

A capability advances only with current evidence for its declared scope:

```text
proposed -> specified -> implemented -> covered -> certified
```

The `covered` lifecycle requires current evidence for every reviewed,
applicable interaction obligation in scope. Certification is
bundle/snapshot-specific. Changing semantics creates a new capability version
or invalidates dependent certifications.

## When to extend core rules

Extend core rules only when the behavior is not card-specific and cannot be represented by existing primitives. Repeated native/card-specific implementations are evidence that a missing capability should be extracted.
