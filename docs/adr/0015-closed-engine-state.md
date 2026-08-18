# ADR 0015: Closed checkpointable EngineState

**Status:** Accepted
**Date:** 2026-08-17

## Context

Core board data alone could not reproduce decisions because allocators,
continuations, knowledge, opaque IDs, Commander ledgers, and delayed effects
were absent.

## Decision

Every semantic input is part of `EngineState`: core, zones/stack, identity
allocators, execution, RNG, knowledge, perspective identity, and format state.
Kernels and projectors may not retain hidden mutable semantic state.

## Consequences

Checkpoint, fork, replay, and parallel environments have one closure boundary.
Derived caches remain allowed only if discardable and semantically inert.
