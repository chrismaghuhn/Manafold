# ADR 0010: Rules-free adapters

- **Status:** accepted
- **Date:** 2026-08-17
- **Supersedes:** none

## Context

Reward, model, UI, or transport code can accidentally redefine legality.

## Decision

Adapters consume engine contracts and cannot add, repair, remove, or execute legal choices.

## Consequences

Reward shaping and action abstraction require independent identities.

## Review trigger

Revisit only with new evidence that changes correctness, information safety,
maintainability, compatibility, or measured performance.
