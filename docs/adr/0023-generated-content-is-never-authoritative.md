# ADR 0023 — Generated Content Is Never Authoritative

**Status:** accepted  
**Date:** 2026-08-18

## Context

Oracle parsers and LLM-assisted generation can accelerate card work but cannot prove full Magic semantics.

## Decision

Generated output is an intermediate candidate with immutable provenance. Human-reviewed promotion, capability validation, and conformance evidence are required before it becomes an authored executable definition.

## Consequences

Automation remains useful without becoming a hidden second rules authority. Parser metrics cannot be used as card-support claims.
