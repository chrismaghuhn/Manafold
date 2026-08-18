# ADR 0026 — Format State Is Authoritative State

**Status:** accepted  
**Date:** 2026-08-18

## Context

Commander tax, designation, damage, and format choices affect legal transitions and must survive checkpoint, fork, and replay.

## Decision

All semantic format data lives inside `EngineState.format`. Format modules are deterministic helpers over explicit state and cannot keep hidden mutable ledgers in controllers or adapters.

## Consequences

Checkpoints are complete and formats remain modular. Generic state carries a versioned format variant, while exact hook interfaces remain deferred to M3.
