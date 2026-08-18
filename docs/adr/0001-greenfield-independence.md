# ADR 0001: Greenfield independence

- **Status:** accepted
- **Date:** 2026-08-17
- **Supersedes:** none

## Context

The project needs freedom to optimize around ML-native contracts rather than inherit an application architecture.

## Decision

Build an independent repository with no runtime, source, schema, or roadmap
dependency on Argentum or another engine. External engines may be research
references and differential oracles only.

## Consequences

Interoperability must be explicit and narrow. No copied assumptions become authority.

## Review trigger

Revisit only with new evidence that changes correctness, information safety,
maintainability, compatibility, or measured performance.
