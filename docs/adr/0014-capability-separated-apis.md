# ADR 0014: Capability-separated kernel, controller, and player APIs

**Status:** Accepted
**Date:** 2026-08-17

## Context

The original `Environment` trait was described as agent-facing but returned
authoritative events, decisions for arbitrary actors, checkpoints, forks, and
replay data. Direct-name checks could not detect transitive leaks.

## Decision

Use three capabilities: `TrustedKernelApi`, `TrustedEnvironmentController`, and
`PlayerEndpoint` bound to one perspective. Player errors are closed,
identifier-free codes. Public player types may not transitively reach privileged
types listed in the capability matrix.

## Consequences

Self-play orchestration remains powerful while model code receives only
perspective-safe data. Search requires a future explicit capability rather than
reusing full-state fork access.
