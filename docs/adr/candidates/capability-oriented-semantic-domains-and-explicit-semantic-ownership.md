# ADR Candidate: Capability-Oriented Semantic Domains and Explicit Semantic Ownership

**Research baseline:** `5c170b1f94b821552a0eff7319565a8020c244ca`  
**Candidate number at research baseline:** `ADR 0041` — provisional only  
**Status:** `PROPOSED / NOT ACCEPTED`  
**Recommended acceptance window:** after successful `M2.Final`, before M2.5 implementation  
**Implementation evidence:** `NOT_RUN`

This candidate is review-complete but intentionally not authoritative yet. The
final ADR number is allocated only when the decision is accepted against the
then-current ADR index.

## Context

Manafold must eventually implement a large, interaction-heavy Magic rules
surface without turning individual cards, a universal interpreter, an event
bus, or a dynamic handler registry into alternate rules authorities.

Existing accepted contracts already establish typed Card IR, versioned
capability closure, one Decision protocol with checkpointable continuations,
complete `EngineState` closure, atomic state/event/delta semantics, sequential
semantic validation, information-safe projection, deterministic replay, and
fail-closed unsupported behavior. See:

- [ADR 0004](../0004-typed-card-ir.md) — typed Card IR as the standard path;
- [ADR 0022](../0022-versioned-capability-registry-and-bundle-certification.md) — capability registry and bundle certification;
- [ADR 0029](../0029-sequential-semantic-transition-validation.md) — sequential semantic validation;
- [ADR 0039](../0039-perspective-local-decision-identity-and-typed-staged-choices.md) — unified Decision V2 and typed continuations;
- [ADR 0040](../0040-m2-information-lifecycle-and-v3-state-identity.md) — information lifecycle and precommit projection requirements;
- [`ARCHITECTURE.md`](../../ARCHITECTURE.md) and [`EXECUTION_MODEL.md`](../../EXECUTION_MODEL.md) — current trusted-core and transaction boundaries.

The missing decision is how reusable Magic semantics are owned and composed as
card count and cross-domain interaction complexity grow.

## Decision

Manafold adopts a **capability-oriented semantic-domain architecture** under the
existing semantic-transition and environment-transaction contracts.

### General semantics are reusable and domain-owned

General Magic behavior is implemented as reusable typed semantics with one
identified primary semantic owner. Cards normally compose those semantics
through reviewed typed Card IR.

A card definition does not directly mutate authoritative state, allocate IDs,
consume RNG, emit authoritative events, construct deltas, project player data,
or commit environment state.

### Capability identity is not runtime dispatch

Capabilities remain versioned support/evidence identities. Runtime semantic
ownership is a separate concern.

The relationship is many-to-many:

```text
one semantic domain may implement several capabilities
one capability may involve several semantic domains
one internal primitive may support several capabilities
```

Capability identifiers are not runtime opcodes, handler names, service-locator
keys, or arbitrary string-dispatch instructions. Filesystem layout is not
support authority.

### Cross-domain processes have one primary semantic orchestrator

A process spanning several domains has one primary semantic owner responsible
for deterministic integration order. It delegates local behavior to the owning
domains rather than reimplementing it.

The exact domain inventory, process decomposition, and source tree are deferred
to the M2.5 capability closure and pinned M3 authority cases.

### Domains have bounded authority

Domains may own local legality, semantic queries, typed plans/proposals,
domain-local continuation meaning, event semantics, reusable capability
implementations, and conformance obligations.

Domains may not independently:

- commit authoritative state;
- commit environment/controller state;
- append accepted replay history;
- advance environment counters;
- construct player projections as an alternate authority;
- bypass the accepted Decision protocol;
- retain hidden mutable semantic state;
- receive unrestricted mutable access equivalent to a global `GameContext`.

The exact Rust query, plan-builder, trait, and mutation-port APIs remain deferred
to evidence from the first M3 vertical slices.

## Nested transaction authorities remain unchanged

This ADR does not collapse the accepted rules and environment boundaries.

```text
Rules semantic transition layer
    RulesKernel execution
      explicit EngineState + trusted actor + typed response
        ↓
      execute rule program or continuation
        ↓
      produce semantic candidate:
        next EngineState
        ordered AuthoritativeRuleEvent[]
        exact StateDelta
        next AuthoritativeDecisionRequest?
        EpisodeStatus
        ↓
    rules-owned transition-contract validation

Environment transaction authority
    validated semantic candidate
        ↓
    derive and validate:
      environment counters
      checkpoint candidate
      replay step / replay candidate
      every required perspective projection and PlayerStep product
        ↓
    atomic environment commit or complete discard
```

`RulesKernel` execution and rules-owned transition-contract validation are
distinct operations inside the rules semantic layer; this ADR does not require
`RulesKernel::apply()` itself to be the final validation call site.

The environment owns the complete precommit environment product and actual
backend commit/discard. It does not reimplement Magic legality. A semantic
domain may commit on neither level.

## Ownership planes

Manafold distinguishes these meanings of ownership:

| Plane | Owns | Does not own |
|---|---|---|
| Card content | reviewed characteristics, abilities, parameters, composition | general rule implementation or mutation |
| Capability support | stable support identity, dependencies, lifecycle, scope, evidence | runtime function dispatch |
| Runtime semantics | reusable Magic behavior and local legality | environment commit |
| Primary orchestration | deterministic integration order for one cross-domain process | duplicated local semantics |
| Physical DTO/storage | type placement required by state closure and dependency direction | semantic interpretation merely because a type is stored there |
| Rules semantic transition | candidate state/events/delta/decision/status and rules validation | checkpoint/replay/projection commit |
| Environment transaction | complete precommit environment product and backend commit/discard | alternate Magic legality |
| Projection | deterministic redaction and perspective-safe DTO construction | authoritative mutation or legality |
| Evidence | exact conformance and certification evidence | production shortcuts |

Physical type ownership and semantic ownership are therefore intentionally
separate. A mutation-relevant audit DTO may remain physically owned by
`mtgml-state` while a rules domain owns its meaning, event-to-audit mapping,
sequential validation semantics, and evidence obligations.

## Capability Registry V1 compatibility

Capability Registry V1 retains its existing meanings:

- `owners` remains maintainer/process ownership;
- `dependencies` remains capability-to-capability dependencies;
- `implementation_paths` remains implementation/evidence discoverability.

No V1 field is reinterpreted as a runtime semantic domain, primary orchestrator,
or touched-domain list.

M2.5 must make semantic ownership explicit and reviewable, but its
machine-readable representation is deferred. If it becomes part of the
capability registry, that requires an explicitly versioned registry evolution;
a separately versioned semantic-ownership artifact is also permitted.

## State, events, deltas, and composite validation

`mtgml-state` remains the sole authoritative state owner. All semantic state
continues to live in the accepted `EngineState` closure.

Domain modularization may move local event-application and validation logic
closer to its semantic owner, but sequential validation remains **one composite
ordered proof**. There is one semantic validation authority per authoritative
meaning; multiple derived validation views are allowed only when their
relationship is explicit and reconciled by a parity invariant with evidence.

Cross-domain causal invariants remain explicit at the transition-contract or
primary-orchestrator layer. Domain decomposition must not lose relationships
such as a physical zone transition being causally bound to information/identity
lifecycle effects.

Every committed rule-relevant mutation remains subject to the accepted exact
event/audit/delta and final-state parity contracts. This ADR does not freeze a
new event hierarchy or cursor representation.

## Decision and continuation integration

This ADR adds no second decision model. Domains may define when a player choice
is required, the complete legal choice domain, trusted semantic bindings, and
the typed partial state needed to resume. The accepted Decision protocol
continues to own request/response identity, actor binding, canonical
representation, validation, rejection, and player-safe projection.

Suspended execution remains typed and checkpointable in `EngineState`.

## Dependency rules

1. `mtgml-state` remains the sole authoritative state owner.
2. State/storage layers do not depend on rules behavior merely to host a DTO.
3. Rules domains may depend on accepted lower-level state and neutral value types.
4. Cross-domain processes have one primary semantic orchestrator.
5. Domain cycles are prohibited; neutral shared types may move downward, but shared behavior does not move downward merely to hide a cycle.
6. Observation and environment cannot become alternate legality engines.
7. Domains cannot depend on environment commit, replay append, or player-endpoint behavior.
8. Runtime semantic implementation is closed-world, typed, and deterministically identified. Arbitrary dynamic handler registration, reflection-defined semantics, and string-to-function semantic authority are rejected.
9. Capability support metadata is not runtime dispatch metadata.
10. Unsupported cross-domain compositions fail closed.

## Deferred by this ADR

This candidate deliberately does **not** freeze:

- the exact list of semantic domains;
- the Rust module/crate tree;
- final Card IR variants;
- exact event/audit hierarchy;
- exact continuation composition;
- mutation-port, plan-builder, or internal trait signatures;
- replacement, continuous-effect, copy, attachment, combat, or Commander APIs;
- optimized dispatch or rollout backends;
- machine-readable semantic-owner metadata.

These are evidence-driven by M2.5/M3 rather than speculatively designed here.

## Compatibility

Accepting this ADR by itself changes no Rust API, Card IR schema, capability
registry schema, Decision schema, state/checkpoint/replay identity, event/delta
representation, player wire contract, or M2 behavior.

Future changes to any of those surfaces continue to follow their existing ADR,
compatibility, schema-evolution, fixture, migration, and API-lifecycle rules.

## Adoption timing

1. Complete current M2 work without changing semantics for this candidate.
2. Use Issue #62 only for structural consolidation of existing M2 ownership.
3. Close M2.Final on one exact head.
4. Re-review this candidate for drift, allocate the then-current ADR number, and accept the narrow ownership decision.
5. During M2.5, derive the exact capability closure and a versioned, reviewable semantic-ownership graph.
6. During M3, introduce only domains and bounded interfaces justified by that locked closure, starting with complete vertical capability slices.
7. Optimize representations or dispatch only after profiling and parity evidence.

No current synthetic M2 implementation is rewritten merely to resemble future
M3 organization.

## Review triggers

Revisit the decision if:

- materially equivalent card-specific implementations recur;
- a domain requires unrestricted mutable full-state access;
- semantic ownership becomes cyclic;
- cross-domain causal invariants cannot be expressed in one composite ordered proof;
- event/audit semantics cannot remain mutually auditable;
- capability identity becomes coupled to routine source moves;
- the first locked M2.5 closure requires widespread escape hatches;
- replacement, continuous, copy, attachment, combat, or format semantics force an alternate authority path;
- native executors become common rather than exceptional;
- measured profiling plus parity evidence later demonstrates that the reference closed-world dispatch architecture is unsuitable.

## Review status

Independent review of R2 found `0 BLOCKER / 0 MAJOR`. Remaining wording-only
notes were incorporated into this repository candidate.

```text
ADR RESEARCH R2: APPROVED AS ACCEPTANCE CANDIDATE
ADR ACCEPTANCE: DEFERRED UNTIL AFTER M2.FINAL
M2.5: NOT STARTED
M3: NOT STARTED
```
