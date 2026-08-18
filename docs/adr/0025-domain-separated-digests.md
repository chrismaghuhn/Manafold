# ADR 0025 — Domain-Separated Digests

**Status:** accepted  
**Date:** 2026-08-18

## Context

Full state, public state, player information, observations, candidate sets, content bundles, and replays have different visibility and compatibility meaning.

## Decision

Use explicit domain/version separation and canonical inputs for each digest family. Never compare or expose a trusted full-state digest as a player information digest.

## Consequences

More identity types and manifest fields are required, but hidden-information safety, cache correctness, replay diagnosis, and migrations are clearer.
