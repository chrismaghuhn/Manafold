# ADR 0032 — Single-source mechanical contract vocabulary

**Status:** accepted

## Context

Rust, Python, JSON Schema, fixtures, and documentation repeat small closed vocabularies. Manual synchronization previously produced public contract contradictions.

## Decision

`contracts/catalog/contract-vocabulary.v1.json` is the source of truth only for mechanically duplicated closed vocabulary. `scripts/generate_contracts.py` generates the corresponding Rust/Python vocabulary, selected schemas, and reference documentation. CI fails on generated drift.

Magic semantics, DTO layout, cross-field validation, state invariants, information-flow rules, and capability semantics remain hand-written reviewed contracts.

## Consequences

Changing a catalog-owned value requires editing one catalog and regenerating. Generated artifacts must not be hand-edited. Expanding generator scope requires a separate ADR so code generation cannot silently become the semantic rules engine.
