# ADR 0006: Authoritative state, events, and deltas

- **Status:** accepted
- **Date:** 2026-08-17
- **Supersedes:** none

## Context

State-only mutation loses causal evidence; event-only sourcing is awkward for simulation.

## Decision

Maintain authoritative state while every transition emits typed rule events and explicit deltas atomically.

## Consequences

State/event/delta disagreement is an invariant failure.

## Review trigger

Revisit only with new evidence that changes correctness, information safety,
maintainability, compatibility, or measured performance.
