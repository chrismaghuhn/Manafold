# ADR 0003: Reference backend first

- **Status:** accepted
- **Date:** 2026-08-17
- **Supersedes:** none

## Context

Two early backends would duplicate bugs and maintenance before semantics stabilize.

## Decision

Build one audit-oriented reference kernel. Add a rollout backend only after profiling and a parity ADR.

## Consequences

Performance work follows evidence; both backends consume one IR.

## Review trigger

Revisit only with new evidence that changes correctness, information safety,
maintainability, compatibility, or measured performance.
