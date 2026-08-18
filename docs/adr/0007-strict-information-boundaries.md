# ADR 0007: Strict information and capability boundaries

**Status:** Accepted
**Date:** 2026-08-17

## Decision

Full state, observation, and information state are separate capabilities.
Knowledge and opaque identity are checkpointable state. Authoritative events and
observed events are distinct types. A player endpoint is bound to exactly one
perspective and cannot reach seeds, RNG internals, checkpoints, replay, full
state, internal IDs, or free-form diagnostics.

Paired-state noninterference tests cover every player-visible byte stream.
