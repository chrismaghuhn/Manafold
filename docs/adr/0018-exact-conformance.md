# ADR 0018: Exact per-step conformance and parity

**Status:** Accepted
**Date:** 2026-08-17

## Context

Final status and minimum event count could pass semantically incorrect
transitions.

## Decision

Conformance cases assert exact current decision, submission outcome, events,
delta, digests, next decision, per-player views, status, rejection nonmutation,
and optional checkpoint/fork/replay parity.

## Consequences

Conformance evidence can support correctness claims. Coarse counts remain
optional diagnostics only.
