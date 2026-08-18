# ADR 0008: Versioned deterministic replays

- **Status:** accepted
- **Date:** 2026-08-17
- **Supersedes:** none

## Context

Research artifacts are meaningless without exact semantic identity.

## Decision

Pin engine, rules, policy, cards, schemas, decks, RNG algorithm/derivation,
reproducible seed identity, revisions, and digests in authoritative replays.
Privileged RNG seed and named-stream counters are part of checkpointable state;
rejected decisions consume no draw.

## Consequences

Unknown or incompatible artifacts reject; migrations create new artifacts.
Checkpoints, forks, and replay execution cannot diverge through hidden RNG state.

## Review trigger

Revisit only with new evidence that changes correctness, information safety,
maintainability, compatibility, or measured performance.
