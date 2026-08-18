# ADR 0024 — Native Executor Quarantine

**Status:** accepted safe default  
**Date:** 2026-08-18

## Context

Some future cards may not fit a practical generic IR, but unrestricted native code would bypass determinism, information, replay, and invariant guarantees.

## Decision

Certified bundles reject native executors until a separate ADR accepts a bounded deterministic command-producing API. Any future executor cannot mutate state directly, perform I/O, hide decisions, or maintain unsnapshotted state.

## Consequences

Unusual cards may remain unsupported longer. The core contract remains trustworthy and the IR is not forced into an unsafe universal scripting language.
