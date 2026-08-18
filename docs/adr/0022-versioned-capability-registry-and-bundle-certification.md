# ADR 0022 — Versioned Capability Registry and Bundle Certification

**Status:** accepted  
**Date:** 2026-08-18

## Context

Card counts and parser success do not prove semantic support. Cards depend on reusable mechanics, decisions, information behavior, generated objects, and format policy.

## Decision

Represent support as versioned capabilities with recursive dependencies and evidence. A locked bundle—not an isolated card—is the certification unit. Use the lifecycle `proposed -> specified -> implemented -> covered -> certified`, with revocation/supersession when defects are discovered.

## Consequences

Initial content work is more explicit. Later cards become easier when capabilities exist. Support claims are narrow, reproducible, and machine-checkable.
