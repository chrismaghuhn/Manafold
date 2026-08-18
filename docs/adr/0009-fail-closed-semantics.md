# ADR 0009: Fail-closed semantics

- **Status:** accepted
- **Date:** 2026-08-17
- **Supersedes:** none

## Context

Approximate execution produces plausible but poisoned trajectories.

## Decision

Unsupported or ambiguous semantics return typed capability failures and block bundle certification.

## Consequences

Coverage may grow slower, but claims remain trustworthy.

## Review trigger

Revisit only with new evidence that changes correctness, information safety,
maintainability, compatibility, or measured performance.
