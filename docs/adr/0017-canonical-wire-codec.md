# ADR 0017: Canonical UTF-8 JSON wire codec for M0.1

**Status:** Accepted
**Date:** 2026-08-17

## Context

Rust, Python, and JSON Schema previously allowed different variants, widths,
byte encodings, and cross-field behavior.

## Decision

Use canonical UTF-8 JSON with closed variants, canonical decimal IDs, canonical
Base64, exact ranges, duplicate-key rejection, and reader re-encoding. Share
byte-exact golden and negative fixtures. JSON Schema is structural; codecs also
perform semantic validation.

## Consequences

OD-010 is resolved for the initial contract. A future binary transport must
preserve the same semantic types and fixture-derived behavior.
