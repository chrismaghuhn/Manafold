# API Lifecycle

**Status:** accepted  
**Stability:** normative compatibility process

## Stability classes

| Class | Meaning |
|---|---|
| `internal` | may change freely within one change set; not consumed externally |
| `experimental` | externally visible for prototyping; versioned but may break with migration notes |
| `provisional-public` | intended public shape; breaking changes require ADR and migration |
| `frozen-public` | support commitment for declared versions; compatibility policy applies strictly |

## Current classification

- canonical v1 player/replay wire contracts: provisional-public until native gates and first external consumer;
- concrete Card IR variants: experimental;
- Rust crate APIs: internal/experimental unless explicitly registered;
- Python client DTOs/codecs matching v1 wire: provisional-public;
- maintainer artifact schemas: experimental in M0.2;
- semantic action keys and ML trajectory schema: experimental/open.

## Deprecation

A public value is never repurposed. New readers may support old and new versions; writers emit one declared version. Removal requires a published migration path, deprecation window appropriate to the consumer base, and preserved fixtures.

## Registration

The normative document register and compatibility policy identify binding public surfaces. Merely making a Rust item `pub` does not freeze it.
