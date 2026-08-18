# ADR 0029 — Sequential Semantic Transition Validation

**Status:** accepted  
**Date:** 2026-08-18

## Context

Validating each event independently against only the global before and after states rejects valid multi-event transitions and cannot prove event composition.

## Decision

Validate authoritative events in order against a semantic cursor derived from the before-state. Each event validates and advances the cursor; the final cursor must equal the corresponding projection of the after-state. The outer transition remains one atomic revision and `StateDelta` remains the exact full-state patch.

## Consequences

Repeated mutations and decision/RNG sequences are compositional. Every new semantic event family must define cursor validation/application and final-state parity tests.
