# ADR 0002: Rust core and Python ML

- **Status:** accepted
- **Date:** 2026-08-17
- **Supersedes:** none

## Context

Rules simulation needs predictable systems performance while ML research benefits from Python tooling.

## Decision

Implement authoritative semantics in Rust and expose a rules-free Python contract for training and evaluation.

## Consequences

The native boundary is replaceable; Python cannot define legality.

## Review trigger

Revisit only with new evidence that changes correctness, information safety,
maintainability, compatibility, or measured performance.
