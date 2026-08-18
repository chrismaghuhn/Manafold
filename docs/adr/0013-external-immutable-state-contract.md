# ADR 0013: External immutable state contract

- **Status:** accepted
- **Date:** 2026-08-17
- **Supersedes:** none

## Context

A future fast backend may need reversible mutation without exposing aliasing to consumers.

## Decision

Expose state as immutable values or opaque snapshots; permit internal representation changes under parity.

## Consequences

Reference semantics remain stable while optimization is possible.

## Review trigger

Revisit only with new evidence that changes correctness, information safety,
maintainability, compatibility, or measured performance.
