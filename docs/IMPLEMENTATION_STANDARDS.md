# Implementation Standards

**Status:** accepted baseline

## Rust

- safe Rust by default; `unsafe` requires an ADR, isolated module, safety proof, and tests;
- deterministic ordered containers or canonical sorting for semantic data;
- no hidden semantic mutable state in kernel/controller objects;
- domain IDs are newtypes, not interchangeable integers/strings;
- constructors/decoders validate before exposing domain values;
- errors cross trust boundaries only through explicit mapping;
- card/native code never mutates `EngineState` directly;
- public serialization is fallible and canonical;
- no wall-clock, filesystem, network, locale, or global RNG in authoritative transitions;
- clippy warnings are denied in freeze/release gates.

## Python

- rules-free DTOs, codecs, orchestration, datasets, and ML only;
- strict typing and immutable DTOs for public contracts;
- canonical codecs use the same fixtures/error codes as Rust;
- no reconstruction of legal actions or hidden-state logic;
- bytecode/cache output never enters the source archive;
- Ruff and strict Mypy are release gates.

## Content

- stable IDs and immutable source provenance;
- generated candidates remain separate from reviewed definitions;
- capabilities declared explicitly;
- unsupported semantics fail closed;
- native executors prohibited unless policy/API/evidence are accepted;
- certification status is machine-readable and bundle-specific.

## Tests

- regression changes start red where practical;
- accepted/rejected paths both tested;
- exact semantic assertions preferred over counts;
- hidden-information changes require noninterference tests;
- state mutations require event/delta parity;
- nondeterministic/flaky tests are defects, not retries to ignore.

## Documentation

Normative claims use explicit status/stability and are registered. Examples are labeled illustrative, executable, or certified. No documentation may promote an unexecuted gate to `PASS`.
