# Native Executor Policy

**Status:** quarantine policy accepted; executable API unresolved  
**Stability:** normative safe default

## Default

Native card executors are rejected from certified bundles until OD-012 is fully resolved by an ADR and executable sandbox contract.

## Why an escape hatch may exist

Some future cards may be disproportionately awkward in the generic IR. The escape hatch prevents the IR from becoming an untyped universal programming language. It is not permission to bypass the kernel.

## Minimum future restrictions

Any accepted native executor must:

- be registered by stable capability and executor ID;
- be deterministic, pure with respect to provided inputs, and free of I/O/time/global RNG;
- access state only through a bounded read interface;
- propose typed rule commands/effects, never mutate `EngineState` directly;
- request randomness only through named checkpointable streams;
- request player choices only through the unified decision protocol;
- serialize/replay/checkpoint without hidden runtime state;
- pass dedicated authority, conformance, information, fuzz, and performance review;
- have a documented path for replacement by generic capabilities when practical.

## Certification

A bundle manifest lists every native executor. Empty is the normal state. Certification tooling fails closed if an executor lacks an accepted policy version and all required evidence.

## Definition-derived discovery

The certification census derives native executors by traversing every reachable card definition and generated-object manifest. `bundle.native_executors` is only a declaration to cross-check.

The census reports:

```text
native_executors                    # actually discovered
undeclared_native_executors         # discovered but absent from bundle declaration
stale_native_executor_declarations  # declared but not reachable
```

Every nonempty category blocks certification. A bundle cannot hide native code by omitting it from its manifest, and stale declarations are rejected because they make provenance ambiguous.
