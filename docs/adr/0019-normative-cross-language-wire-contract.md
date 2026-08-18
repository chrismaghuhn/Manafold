# ADR 0019: Normative Cross-Language Wire Contract

- Status: Accepted
- Date: 2026-08-18

## Decision

Public v1 contracts have one normative semantic model expressed consistently in Rust, Python, JSON Schema, and shared fixtures. Internal domain objects may differ only behind complete fallible conversions. Canonical JSON writers and readers are tested in both languages against the same positive and negative corpus.

## Consequences

A change touching a public field or variant must update all layers atomically. JSON Schema alone is not treated as sufficient for cross-field semantics.
