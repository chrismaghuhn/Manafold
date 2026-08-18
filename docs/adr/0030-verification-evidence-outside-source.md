# ADR 0030 — Verification Evidence Outside Reproducible Source

**Status:** accepted  
**Date:** 2026-08-18

## Context

A verifier that writes logs/status files into the source set changes the object it claims to have verified and can invalidate a previously checked archive.

## Decision

Generated verification logs and reports live outside the archived source set, by default under `dist/verification/`. The archive reproducibility gate runs last and no archived file is modified afterward.

## Consequences

Source archives are state-independent and reproducible. Release evidence remains preservable and checksumable as adjacent artifacts rather than self-modifying archive inputs.
