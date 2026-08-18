# Ownership Model

**Status:** accepted baseline

## Roles

- **architecture maintainer:** trust boundaries, crate dependencies, ADRs, compatibility;
- **rules authority maintainer:** rules snapshots, interpretations, mechanic specs;
- **content maintainer:** card definitions, provenance, bundle closure;
- **information-safety reviewer:** observations, knowledge, opaque IDs, noninterference;
- **conformance maintainer:** case quality, harness, fuzz/property evidence;
- **wire/API maintainer:** Rust/Python/schema/fixture parity;
- **release maintainer:** clean-machine verification, manifests, checksums, certification;
- **performance maintainer:** benchmark methodology and regression evidence.

One person may hold several roles in a small project, but review checklists still name the role being exercised.

## High-risk changes

Changes to hidden information, object identity, RNG, decision completeness, replay identity, format state, native executors, or compatibility require at least one explicit reviewer role beyond the author when the project has multiple maintainers. In a solo phase, the PR records a separate self-review pass and evidence checklist.

## Ownership metadata

Capability entries and normative documents name owner roles rather than personal names where possible. Maintainer files map current people to roles.
