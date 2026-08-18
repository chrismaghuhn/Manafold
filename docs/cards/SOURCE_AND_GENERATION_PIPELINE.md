# Source and Generation Pipeline

**Status:** accepted boundary  
**Stability:** normative provenance policy

## Pipeline

```text
immutable source snapshot
        ↓
normalization report + digest
        ↓
parser / generator / LLM-assisted candidate
        ↓
static structural validation
        ↓
human semantic review
        ↓
red conformance evidence
        ↓
promotion to authored card definition
        ↓
bundle coverage and certification
```

## Authority

Generated output is always a candidate. The engine never executes arbitrary Oracle text directly and never treats model confidence as semantic proof.

## Provenance

Each generated candidate records:

- source snapshot and source record ID;
- normalization tool/version;
- normalized input digest;
- generator/parser/model identity and configuration;
- generated output digest;
- reviewer and promotion commit;
- supersession history.

## Repository content

Commit only data whose redistribution is permitted. Bulk card text/images remain external when rights are unclear. Repository manifests may store identifiers, digests, retrieval instructions, and project-authored semantic definitions.

## Determinism

Given the same source artifact and pinned tool/configuration, normalization and generation should reproduce identical intermediate output or explicitly record nondeterminism. Non-reproducible generated content cannot enter a certified bundle.
