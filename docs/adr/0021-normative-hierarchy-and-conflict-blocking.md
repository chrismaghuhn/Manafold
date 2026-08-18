# ADR 0021 — Normative Hierarchy and Conflict Blocking

**Status:** accepted  
**Date:** 2026-08-18

## Context

The same public contract is represented by documentation, Rust, Python, JSON Schema, and fixtures. Silent precedence creates invalid replays and datasets.

## Decision

Classify documents/artifacts, maintain a machine-readable register, and treat any contradiction as a blocking defect. Serialized contracts are defined jointly by DTOs, codecs, schemas, fixtures, and semantic validators; no one layer silently wins.

## Consequences

Changes are broader but auditable. Verification includes document/register checks. Release and certification stop until contradictions are resolved with regression evidence.
