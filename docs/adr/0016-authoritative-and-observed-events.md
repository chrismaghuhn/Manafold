# ADR 0016: Separate authoritative and observed events

**Status:** Accepted
**Date:** 2026-08-17

## Context

Authoritative events contain internal object IDs and RNG audit data and cannot
be safely returned to players.

## Decision

Keep `AuthoritativeRuleEvent` for kernel, replay, and conformance. Project it per
perspective to `ObservedEvent` with opaque IDs, public keys, redacted values, and
perspective-visible sequence.

## Consequences

Event traces remain auditable without becoming an information side channel.
Noninterference tests include observed events and history length.
