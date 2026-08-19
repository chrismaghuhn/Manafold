# V0.2.1 — Executable Contract Closure

**Status:** normative release-definition addendum  
**Stability:** accepted for the V0.2.1 freeze candidate

## Purpose

V0.2.0 established the project-wide architecture, maintenance model, and certification vocabulary. V0.2.1 closes defects that would otherwise make the first executable kernel unsafe or unverifiable. It does not add real Magic rules, real cards, or a playable environment.

V0.2.1 is complete only when the contracts below exist in code and every required native and Python gate passes under the pinned toolchains.

## Closed defects

### Rust source and shared fixtures

- Rust source has balanced lexical delimiters; the Python structural gate detects unmatched braces without pretending to replace `cargo check`.
- The Rust negative-fixture harness reads the complete shared manifest, including `expected_reject_layer`.
- Rust and Python require the same supported reject-layer identity and expected stable error code for every negative fixture.

### Full-state identity

`EngineState` is never hashed by directly serializing its internal representation. The normative `FullStateDigest` input is:

```text
FullStateDigestInputV1
├── explicit digest domain
├── explicit canonicalization schema
├── complete semantic EngineState components
└── complex-key maps represented as deterministic entry arrays
```

All JSON object keys are recursively sorted before hashing. The digest API is fallible; serialization failure cannot become a hidden panic. Domain-specific digest newtypes prevent accidental comparison of full-state, public-state, information-state, observation, candidate-set, content, replay, and checkpoint identities.

A nonempty ordered-zone test is mandatory because an empty `BTreeMap<ZoneKey, ...>` does not expose JSON map-key failures.

### Complete checkpoint

A trusted checkpoint is not merely an `EngineState`. `EnvironmentCheckpointV1` includes:

```text
state
full-state digest
EpisodeStatus
all declared environment limit counters
checkpoint codec identity/version
typed checkpoint digest over every field above except the full state bytes themselves
```

Restore validates the embedded full-state digest and complete checkpoint digest before replacing backend state. Terminal or truncated checkpoints cannot retain a pending player decision. No status, limit counter, RNG counter, or other semantic controller state may live in an uncheckpointed mutable field.

V0.2.1 closes the in-memory semantic checkpoint contract; it does not freeze a JSON checkpoint wire representation. A durable codec must name its own canonical encoding and cannot serialize complex-key internal maps through `serde_json` implicitly.

### Compositional transition validation

An accepted response remains one atomic revision, but its semantic event sequence is validated with an internal cursor:

```text
cursor = semantic projection of before-state
for event in authoritative event order:
    validate event against cursor
    apply event's semantic effect to cursor
assert cursor equals the corresponding semantic projection of after-state
```

This supports repeated life changes, repeated taps, multiple RNG consumptions, consecutive zone incarnations, and clearing one decision before creating the next. Comparing every event directly to only the global before/after pair is forbidden.

The exact `StateDelta` still reconstructs the entire final `EngineState`; the semantic event/audit sequence explains rule-significant changes.

### Conformance inputs

A conformance step asserts both sides of the interaction:

- the actual visible decision before submission;
- the exact response actually submitted;
- acceptance or complete rejection nonmutation;
- events, audit, full-state digest, next decision, player projections, and status.

Expected input fields that are never asserted are contract defects.

### Knowledge provenance

Each retained knowledge fact records:

- affected object incarnation;
- optional known physical/card identity and location;
- public/private history channel and sequence at acquisition;
- typed acquisition reason;
- typed per-object invalidation records with channel, sequence, and reason.

State validation checks history bounds, perspective mapping, known identities/locations, and event provenance. A global list of unscoped invalidation reasons is insufficient.

### Native-executor closure

Native executors are discovered by traversing every reachable card and generated-object manifest. The bundle declaration is a cross-check, not the source of truth.

The census reports:

```text
native_executors
undeclared_native_executors
stale_native_executor_declarations
```

Any nonempty set blocks certification until the native-executor policy and evidence gates explicitly permit it.

### Replay rejection identity

A rejected replay step preserves both its authoritative revision and its `FullStateDigest`. Replay validation carries the previous state identity through every step and rejects a diagnostic/rejected entry that changes either value. An empty replay must end at exactly its manifest initial revision and full-state identity.

### Reproducible verification

`run_verification.py` writes logs and generated reports only beneath an excluded output directory, by default `dist/verification/`. It does not modify archived source files.

The deterministic archive gate is last. No source file is changed after that gate. Generated verification reports are release evidence adjacent to the source archive, not inputs embedded into that archive.

## Required V0.2.1 gates

```text
REPOSITORY_VERIFIER_DIRECT_COMMAND
RUST_LEXICAL_STRUCTURE
DOCUMENTATION_REGISTER_AND_LINKS
SCHEMA_AND_MAINTAINER_ARTIFACT_VALIDATION
PYTHON_TESTS
SHARED_GOLDEN_AND_NEGATIVE_FIXTURES
NONEMPTY_FULL_STATE_DIGEST_TEST
COMPLETE_CHECKPOINT_TESTS
COMPLETE_CHECKPOINT_DIGEST_TESTS
REJECTED_REPLAY_FULL_IDENTITY_TESTS
COMPOSITIONAL_TRANSITION_TESTS
CONFORMANCE_INPUT_ASSERTIONS
NATIVE_EXECUTOR_CLOSURE_TESTS
CARGO_LOCK_COMMITTED
RUST_FMT
RUST_CHECK
RUST_CLIPPY_DENY_WARNINGS
RUST_TESTS
RUFF_FORMAT
RUFF
MYPY
DETERMINISTIC_SOURCE_ARCHIVE_LAST
```

`NOT_RUN` and `FAIL` block contract freeze. Passing Python and archive gates alone does not unblock M1.

## Non-goals

V0.2.1 does not:

- implement a real turn, priority, stack, combat, or card-resolution rule;
- freeze the concrete Card IR vocabulary;
- select the two V1 decks;
- certify any capability or card bundle;
- claim that the Rust workspace compiles until the native gates actually run;
- generate or fabricate `Cargo.lock` without the pinned Rust toolchain.

## Exit

V0.2.1 becomes `CONTRACT_FROZEN` only after every required gate is recorded as `PASS` by the external verification report. M1 was unblocked with V0.2.2.
