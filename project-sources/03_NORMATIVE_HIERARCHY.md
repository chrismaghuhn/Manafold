# Normative Hierarchy and Conflict Policy

**Status:** accepted  
**Stability:** normative  
**Owner:** architecture maintainers

## Why this exists

A rules/ML engine can invalidate years of data when two files appear to describe the same contract but disagree. M0.2 therefore does not choose a silent “winner.” A contradiction is a defect that blocks freeze, release, certification, and dataset publication.

## Artifact classes

### External authority snapshots

Pinned Comprehensive Rules, Oracle/card data, official rulings, Commander policy, and banlist snapshots are the source authority for scoped Magic behavior. They are immutable episode identities and are not copied into general documentation as an alternative authority.

### Accepted ADRs

ADRs record durable architectural choices, alternatives, consequences, compatibility, and the milestone unblocked. ADRs do not by themselves prove executable behavior.

### Normative contract documents

Documents registered as `normative` define semantic ownership, invariants, and proof obligations. They must avoid examples that contradict executable contracts.

### Executable public contracts

For a serialized surface, Rust DTO/codec, Python DTO/codec, JSON Schema, canonical golden fixtures, negative fixtures, and semantic validators are a **joint contract**. No one member may silently override another.

### Conformance cases

Pinned conformance cases are the executable authority for claims inside a declared rules/capability scope. They must cite the external authority snapshot and expected semantics.

### Process and informative documents

Process documents define required maintainer steps. Informative documents explain or motivate but cannot override normative or executable contracts.

## Conflict rule

When any two artifacts describing the same surface disagree:

1. record the contradiction as a blocking issue;
2. stop release/certification for the affected surface;
3. identify the intended contract using authority snapshots and accepted ADRs;
4. update every affected representation in one change;
5. add a regression fixture or conformance case;
6. document compatibility and migration consequences.

Do **not**:

- accept whichever layer is easiest to change;
- treat a green schema validator as proof of semantic parity;
- reinterpret an old enum value in place;
- publish data generated during the contradiction without provenance and quarantine.

## Machine-readable register

[`normative-document-register.v1.json`](../docs/normative-document-register.v1.json) lists every binding or process document, its owner role, stability, and change process. `scripts/check_documentation.py` verifies that registered paths exist, every project/process/template document is classified, and local Markdown links resolve. Per-capability specifications under `docs/rules/capabilities/` are governed by the versioned capability registry instead of being duplicated here.
