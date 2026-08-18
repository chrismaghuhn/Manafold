# Project Structure and Ownership

**Status:** accepted baseline  
**Stability:** normative crate/process boundary

## Repository layers

```text
crates/       authoritative Rust domain, rules, environment, replay, wire
python/       rules-free ML/client DTOs and codecs
schemas/      normative serialized shape constraints
wire/         shared canonical positive and negative fixtures
cards/        authored manifests, definitions, capabilities, bundles, evidence
conformance/  executable scoped semantic cases
benchmarks/   pinned workloads and raw evidence
sources/      provenance only unless redistribution is approved
docs/         architecture, contracts, process, ADRs, templates
scripts/      deterministic repository and maintainer automation
```

## Canonical Rust crates

| Crate | Owns | Must not own |
|---|---|---|
| `mtgml-model` | primitive identifiers, digest wrappers, shared small value types | rules, visibility, I/O |
| `mtgml-random` | checkpointable RNG identity/state interfaces | policy decisions, wall-clock randomness |
| `mtgml-decision` | closed player decision DTOs, authoritative bindings, validation | card execution, observation projection |
| `mtgml-state` | complete checkpointable `EngineState`, identity/knowledge/format state, exact delta | hidden mutable caches, network transport |
| `mtgml-card-ir` | experimental typed content vocabulary | authoritative source parsing, direct state mutation |
| `mtgml-rules` | transition semantics, authoritative events, rejection contract | Python/model logic, UI callbacks |
| `mtgml-observation` | perspective-safe observation, information state, observed-event projection | root seed, authoritative IDs/events |
| `mtgml-replay` | replay domain identity and validation | training rewards, silent migration |
| `mtgml-wire` | canonical public serialization and shared fixtures | Magic rules decisions |
| `mtgml-environment` | trusted controller and perspective-bound endpoints | alternate rule implementation |
| `mtgml-commander` | Commander policy helpers and evidence; state remains inside `EngineState` | mutable state outside checkpoint closure |
| `mtgml-conformance` | exact case vocabulary and harness contracts | production shortcuts |

`mtgml-state` is the single state crate. A parallel or orphan state implementation is forbidden.

## Dependency direction

```text
model/random
    ↓
decision/state/card-ir
    ↓
rules/observation/replay
    ↓
wire/environment/conformance
    ↓
Python client and ML orchestration
```

Lower layers cannot depend on environment, Python, or model-training concerns. Cycles require an ADR and should normally be resolved by moving a small neutral type downward.

## Public/internal classification

Every exported surface must be classified in [`maintenance/API_LIFECYCLE.md`](maintenance/API_LIFECYCLE.md) as:

- internal;
- experimental;
- provisional public;
- frozen public.

A `pub` Rust item is not automatically a stable contract.

## File-size and cohesion rule

Prefer modules with one semantic owner. Splitting by arbitrary line count is discouraged, but files that mix state identity, transition execution, wire encoding, and policy must be separated. New abstractions require a concrete ownership reason, not speculative indirection.
