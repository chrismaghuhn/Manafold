# Freeze Levels and Claims

**Status:** accepted  
**Stability:** normative release vocabulary

## Levels

### Draft

Structure exists; contradictions and missing gates are expected. No downstream stability claim.

### Freeze candidate

Required design/code/artifacts are present, but one or more mandatory gates are `NOT_RUN` or `FAIL`. Downstream work may explore but cannot rely on frozen contracts.

### Contract frozen

All required contract, native build, lint, unit, fixture, schema, and documentation gates pass. Breaking changes require ADR and compatibility analysis.

### Capability covered

A capability implementation and its required evidence pass in the reference backend, but it is not yet a bundle support claim.

### Bundle certified

Exact snapshots, decks, definitions, recursive capability closure, and every acceptance gate pass. This is the support-claim level.

### Certification revoked/superseded

A discovered defect or newer artifact invalidates the old support claim. Provenance remains available.

## Rule

`NOT_RUN` is never equivalent to `PASS`. File existence, source review, or generated reports cannot substitute for executing a required gate.
