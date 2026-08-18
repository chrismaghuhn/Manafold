# ADR 0012: Independent schema compatibility

- **Status:** accepted
- **Date:** 2026-08-17
- **Supersedes:** none

## Context

Package versions do not capture the compatibility of decisions, observations, replays, or datasets.

## Decision

Version each public semantic surface independently and classify changes explicitly.

## Consequences

Semantic keys never change meaning in place.

## Review trigger

Revisit only with new evidence that changes correctness, information safety,
maintainability, compatibility, or measured performance.
