# ADR 0028 — Complete Environment Checkpoints

**Status:** accepted  
**Date:** 2026-08-18

## Context

`EngineState` alone does not carry terminal/truncation status or environment-limit counters. Hidden backend fields would break restore, fork, replay, and search parity.

## Decision

Trusted checkpoint and restore use `EnvironmentCheckpointV2`, containing `EngineState`, its typed digest, `EpisodeStatus`, declared limit counters, and checkpoint codec identity/version. Restore validates the whole object before mutation.

## Consequences

A checkpoint recreates complete environment behavior rather than only the board. New semantic controller state must extend the versioned checkpoint instead of living in mutable backend-local storage.
