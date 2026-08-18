# Changelog

## V0.2.2 — Executable Freeze & Maintainer Ergonomics — 2026-08-18

### Added
- Narrow single-source contract vocabulary catalog with generated Rust/Python/schema/docs artifacts and drift check.
- `doctor`, `bootstrap`, `check-fast`, `check`, `check-all`, and `release-candidate` maintainer paths.
- Split PR-fast, integration, and nightly certification CI.
- Tested synthetic golden path spanning capability/card/bundle/wire/census/fail-closed certification.

### Unchanged
- No real Magic rules, cards, decks, or playable environment.
- M1 remains blocked until every native and Python freeze gate passes.

## V0.2.1 — Executable Contract Closure — 2026-08-18

### Fixed

- Removed the guaranteed Rust parser error in `mtgml-model`.
- Made the Rust negative-fixture harness accept and validate the shared reject-layer field.
- Replaced direct `EngineState` JSON hashing with a fallible, canonical, domain-separated full-state digest DTO and typed digest domains.
- Added a nonempty structured-zone digest regression test.
- Replaced bare-state checkpoints with `EnvironmentCheckpointV1`, including status, limit counters, codec identity, embedded full-state identity, and a typed complete-checkpoint digest.
- Made semantic event validation sequential/compositional inside one atomic revision.
- Made conformance assert the actual visible decision and actual submitted response.
- Made rejected replay steps preserve both revision and full-state identity.
- Added typed knowledge acquisition and per-object invalidation provenance.
- Derived native-executor closure from reachable definitions and reported undeclared/stale declarations.
- Moved generated verification reports/logs outside the reproducible source archive, added an explicit source-tree fingerprint gate, protected output cleanup with an ownership marker, and made the archive gate last.

### Verification policy

- Added a conservative Rust lexical-structure gate without representing it as compilation.
- Rust format/check/clippy/test and `Cargo.lock` remain mandatory native freeze gates.
- Missing native tools remain `NOT_RUN`; M1 stays blocked until every V0.2.1 gate passes.

## M0.2 — Specification and Maintainer Readiness — 2026-08-18

### Added

- Normative hierarchy, machine-readable document register, and conflict-blocking policy.
- Precise project/crate ownership, domain/state/identity, transition, error, format, digest, concurrency, observability, and ML trajectory specifications.
- Detailed rules/mechanic and card-content workflows.
- Versioned capability registry, recursive closure model, bundle certification and scope-impact schemas.
- Source-generation and native-executor quarantine policies.
- Conformance, noninterference, property/fuzz, API lifecycle, freeze-level, release, and ownership guides.
- Maintainer tools for card/capability scaffolding, capability census, certification preflight, documentation checks, and artifact semantic validation.
- Maintainer artifact examples and schemas.
- Typed certification closure reports for missing capabilities, dependency cycles,
  lifecycle blockers, missing definitions, and quarantined native executors.
- Semantic registry validation for repository-safe paths, existing specifications,
  implementation evidence, conformance evidence, and reviewed information risk.
- A single reproducibility manifest shared by source-archive creation,
  verification, local commands, and CI.
- Exact source-archive member and byte-for-byte source parity verification.
- Reference-toolchain validation for the exact Python interpreter, Rust channel,
  and pinned direct Python development tools.
- ADRs 0021–0026.

### Changed

- Version advanced to 0.2.0.
- M0.2 is now the required contract freeze before M1.
- `mtgml-state` is explicitly the sole authoritative state crate.
- Verification now includes documentation, maintainer artifacts, lockfile, and archive reproducibility gates.
- The normative document register now classifies every binding, process, and
  template document; unregistered documents fail repository verification.
- Capability census and certification reports now preserve typed blockers rather
  than flattening them into diagnostic strings.

### Removed

- Orphan duplicate `mtgml-engine-state` crate, which was not a workspace member and contradicted single state ownership.
- Duplicate source-archive builder and duplicate root Python development lock.

### Explicitly unchanged

- No real Magic rules, cards, decks, or playable environment.
- Concrete Card IR remains experimental until M2.5.
- Native executors remain prohibited in certified bundles.
- M1 remains blocked until every generated M0.2 freeze gate passes.

## M0.1.1 — Contract Closure — 2026-08-18

### Fixed

- Unified ReplayManifest fields and randomness identity across Rust, Python, and JSON Schema.
- Added both missing observed-event variants and a complete Python event model/codec.
- Made Python `PlayerClient` return the same `PlayerStep` semantics as Rust.
- Closed terminal and truncation reasons in every layer.
- Added shared Rust/Python negative fixture execution.
- Replaced exclusive-borrow player binding with coexisting perspective-bound handles.
- Added exact value-aware candidate binding validation.
- Added central cross-component `EngineState` validation.
- Replaced partial deltas with exact full-state-part patches plus a separate semantic audit trace.
- Added event/delta parity and rejection-nonmutation contracts.
- Added replay revision and empty-replay invariants.
- Made repository verification bytecode-free by construction.
- Made verification reporting generated from actual commands rather than manually asserted status.

### Explicitly unchanged

- Card IR remains experimental.
- No real Magic rules or cards were added.
- M1 remains blocked until the generated verification report is fully green.
