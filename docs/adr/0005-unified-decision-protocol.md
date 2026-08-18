# ADR 0005: Unified decision protocol

- **Status:** accepted
- **Date:** 2026-08-17
- **Supersedes:** none

## Context

Controller-specific callbacks fragment legality and hide partial decisions.

## Decision

Represent every player choice as versioned request/response data with staged continuations.

## Consequences

No hidden auto-completion; soundness and completeness become gates.

## Review trigger

Revisit only with new evidence that changes correctness, information safety,
maintainability, compatibility, or measured performance.
