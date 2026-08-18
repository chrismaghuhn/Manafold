# ADR 0004: Typed card IR as the standard path

**Status:** Accepted
**Date:** 2026-08-17

## Decision

Cards normally compile to inspectable, serializable, typed IR. Arbitrary card
scripts are not the default execution model.

This ADR accepts the architectural direction only. The concrete Rust enum in
M0.1 is experimental vocabulary and is not a stable semantic or wire contract.
It will evolve from the exact M2.5 deck closure and M3 primitive requirements.

Native executors remain rejected until OD-012 is resolved.
