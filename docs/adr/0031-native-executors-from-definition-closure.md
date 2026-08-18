# ADR 0031 — Native Executors Come From Definition Closure

**Status:** accepted  
**Date:** 2026-08-18

## Context

A bundle-authored native-executor list can omit an executor declared by a reachable card, allowing certification preflight to miss quarantined code.

## Decision

Traverse all reachable card and generated-object manifests and derive the native-executor set from those definitions. Compare it with the bundle declaration and report undeclared and stale entries. Any discovered native executor blocks certification under the current policy.

## Consequences

Certification cannot be bypassed by an incomplete bundle declaration. Bundle metadata remains a checked declaration rather than an authority over reachable implementation content.
