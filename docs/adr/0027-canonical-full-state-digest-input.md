# ADR 0027 — Canonical Full-State Digest Input

**Status:** accepted  
**Date:** 2026-08-18

## Context

Direct JSON serialization of internal Rust state can fail for maps with structured keys and couples persisted identity to incidental implementation layout. Empty maps conceal this defect.

## Decision

`FullStateDigest` is computed from an explicit versioned DTO. Structured-key maps are represented as deterministically sorted entry arrays; JSON object keys are recursively sorted; the digest is domain-separated and returned through a fallible API. Distinct digest domains use distinct Rust newtypes.

## Consequences

State identity is stable, nonempty real states cannot panic during hashing, and incompatible digest domains cannot be compared accidentally. Any canonicalization change requires a new input schema/domain version and replay provenance.
