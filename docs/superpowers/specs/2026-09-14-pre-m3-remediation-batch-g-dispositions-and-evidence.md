# Pre-M3 Remediation Batch G: Dispositions and Evidence

**Status:** evidence record

**Date:** 2026-09-15

**Repository:** chrismaghuhn/Manafold

**Branch:** chris/pre-m3-remediation-batch-g-persistence-safety-hardening

**BASE:** 04a4831f4fd6e35aa5b6ac315e641b7af238fe9c

**CODE_VERIFICATION_HEAD:** ea40e85e6a291b83168c390a1a4be98e7a6e28be

**EVIDENCE_INPUT_HEAD:** ea40e85e6a291b83168c390a1a4be98e7a6e28be

CODE_VERIFICATION_HEAD is the last implementation/style/type commit on which
the final direct checks ran. EVIDENCE_INPUT_HEAD is the exact HEAD immediately
before this evidence document and its register entry are committed. The final
evidence commit is recorded externally after commit; this document
intentionally has no self-referential final-head field.

## Dispositions

    FND-017A = CONFIRMED -> CLOSED_BY_BATCH_G
    FND-017B = RESOLVED_ON_BASE
    FND-017 parent = CLOSED
    FND-018 = CONFIRMED -> CLOSED_BY_BATCH_G
    FND-019 = CONFIRMED -> CLOSED_BY_BATCH_G
    FND-029 = CONFIRMED -> CLOSED_BY_BATCH_G
    FND-030 = CONFIRMED -> CLOSED_BY_BATCH_G
    FND-031 = CONFIRMED -> CLOSED_BY_BATCH_G
    FND-032 = CONFIRMED -> CLOSED_BY_BATCH_G

FND-017A closes only detached calculator input closure: exact V3
FullStateDigestV3 reference identity, local status/counter/codec validity, and
the private payload builder. FND-017B remains resolved by the existing
checkpoint and replay owners; no duplicate state authority was added.

## RED/GREEN commits

All named focused filters were invoked without Cargo --exact, so the executed
tests modules were included and every focused command ran at least one test.

| Finding | RED evidence | GREEN evidence |
|---|---|---|
| FND-017A | 96a342fbb53b49a683506f2997e47db88545a9bb; Rust direct-reference/counter tests and Python local-input tests failed because the unfixed calculator returned a digest instead of semantic_validation. | e4341fce9a78056cd9214202d61c4b2a9acc6d60; Rust persistence tests and Python FND-017A tests pass. ea40e85e6a291b83168c390a1a4be98e7a6e28be adds the typed counter intermediate and makes Mypy pass. |
| FND-017B | Existing-owner characterization, not a production RED: efb03c05931d38586943a9033736b28c5e02f551 records exact Err(CheckpointValidationError::CompletedWithDecision) and the named state/replay owner tests. | The environment and replay characterization commands pass on ea40e85e6a291b83168c390a1a4be98e7a6e28be; no production fix was authorized or added. |
| FND-018 | 799b73373f3a4fad73b0ad03c0172411a049c099; the source-surface guard failed while the top-level checkpoint still contained Serde. | The top-level removal landed with e4341fce9a78056cd9214202d61c4b2a9acc6d60 during the FND-017A implementation; the guard and environment/replay suites pass on ea40e85e6a291b83168c390a1a4be98e7a6e28be. This records the actual commit order rather than inventing a separate fix SHA. |
| FND-019 | 7c6f0a4d7b88a306f61adbb668cddf7d7d48cec9; Rust classified the nested oversized-array/depth input as DepthExceeded while Python classified it as array_too_large. | f3e97ad84da39bb1d47c6e254fc19097d0fbac15; Rust and Python persistence tests agree on ADR 0040 precedence. |
| FND-029 | f537d7f8c67b98f1c8f07f71d69275693c982687; invalid direct lane access hit the unfixed debug assertion/panic path. | 64d2f8bad324ad9a0249b7a5cdc1c41d5c7b8fb9; checked internal extraction returns InvalidRawLane without panic or cursor mutation and the random suite passes. |
| FND-030 | The initial raw-text characterization failed before the fix because Windows text I/O produced CRLF bytes; no separate RED commit was created. | 60470611502e8d01c2267f700867d7f8c78f0c68; exact UTF-8/LF helpers and the three raw-byte tests pass. Later commits f6c9aabf1378cdf18664e93cdfb2d7dabcec94ef and c3c9a97931fc6bda1a13894d6af5a7c2661bd6b6 are formatting-only. |
| FND-031 | c562d558b4bbd08bc147bd8efc317c3bb9a9cf1b; the secret-bearing Debug sentinel was exposed by default difference rendering. | 89851f2613cb8b9fb5e69ae730486f2e179708d7; bounded summaries preserve path/mismatch/presence shape and omit sensitive values. |
| FND-032 | 27b5bd674bef7ad3d1975b82891d3c74f406831c; a designated commander with no ledger entry returned NotDesignated. | 6419021babf2b135d39746543a93cb27af132903; designation membership is checked first and absent counts default to zero. |

The final direct code head also contains these non-semantic cleanup commits:
5cedd7ac3654d0d038828fc42d428d3fd4d9f1cb for test-table clarity,
c3c9a97931fc6bda1a13894d6af5a7c2661bd6b6 for Python import order, and
ea40e85e6a291b83168c390a1a4be98e7a6e28be for the Mypy-safe counter list.

## FND-017B owner evidence

The following existing-owner tests were executed and passed on the code
verification head. No FND-017B production change was made.

    cargo test -p mtgml-environment --locked batch_d_invalid_ordered_state_cannot_construct_checkpoint
    cargo test -p mtgml-environment --locked checkpoint_identity_tampering_is_rejected
    cargo test -p mtgml-environment --locked closed_status_player_outcomes_require_authoritative_player_universe
    cargo test -p mtgml-environment --locked fnd_025_checkpoint_rejects_noncanonical_status_order
    cargo test -p mtgml-environment --locked fnd_017b_closed_status_with_pending_decision_is_rejected_at_checkpoint_owner
    cargo test -p mtgml-replay --locked fnd_022b_manifest_requires_the_exact_deck_player_universe_for_closed_status
    cargo test -p mtgml-replay --locked fnd_025_manifest_rejects_noncanonical_deck_and_status_order
    cargo test -p mtgml-replay --locked replay_v3_rejects_corrupt_accepted_progression

These cover invalid EngineState, state/digest mismatch, closed-status exact
player-universe closure, canonical status/deck ordering, the exact completed
status plus pending-decision error, and replay V3 identity/status/counter/
revision continuity.

## Integration matrix

The following rows were measured at CODE_VERIFICATION_HEAD.

| # | Contract surface | Status | Executed evidence |
|---:|---|---|---|
| 1 | valid V3 checkpoint identity construction | PASS | cargo test -p mtgml-persistence --locked; known-answer test passed. |
| 2 | malformed direct V3 identity input fails at its owner | PASS | cargo test -p mtgml-persistence --locked; FND-017A identity/counter tests passed. |
| 3 | V3 checkpoint golden digest bytes unchanged | PASS | persistence package suite and workspace test suite passed; known-answer tests passed. |
| 4 | raw Serde and supported codec boundary characterized | PASS | cargo test -p mtgml-environment --locked; FND-018 guard and checkpoint/replay tests passed. |
| 5 | valid canonical persistence bytes decode identically Rust/Python | PASS | persistence package suite plus python -B scripts/run_python_tests.py --profile full. |
| 6 | compound persistence precedence: nested array + depth | PASS | Rust/Python FND-019 tests passed; both classify as array_too_large. |
| 7 | compound persistence precedence: payload framing + bound | PASS | persistence package suite and Python persistence full profile passed. |
| 8 | valid RNG lanes 0..3 | PASS | cargo test -p mtgml-random --locked; all raw-word and lane KAT tests passed. |
| 9 | invalid RNG lane fails closed without panic or cursor mutation | PASS | cargo test -p mtgml-random --locked; FND-029 test passed. |
| 10 | RNG KAT vectors unchanged | PASS | cargo test -p mtgml-random --locked; 43 tests passed. |
| 11 | generated LF/CRLF raw-byte determinism | PASS | python -B -m pytest python/tests/test_batch_g.py -q; 3 tests passed. |
| 12 | generated check catches semantic catalog drift | PASS | python -B scripts/generate_contracts.py --check; catalog matched. |
| 13 | default diagnostics omit prohibited sensitive material | PASS | cargo test -p mtgml-conformance --locked; sentinel regression passed. |
| 14 | default diagnostics retain useful semantic path/mismatch data | PASS | conformance package suite; path, mismatch, and presence-shape tests passed. |
| 15 | designated Commander with no ledger entry returns zero helper amount | PASS | cargo test -p mtgml-commander --locked; FND-032 matrix passed. |
| 16 | non-designated Commander control returns NotDesignated | PASS | cargo test -p mtgml-commander --locked; both non-designated controls passed. |
| 17 | no capability/support claim was created | PASS | python -B scripts/validate_maintainer_artifacts.py; 7 artifacts validated, with Batch-G scope audit clean. |
| 18 | workspace, checkpoint, replay, and relevant regression suites remain green | PASS | all 9 affected package suites, Rust workspace tests, and direct integration/certification profiles passed. |

## Code-head gate record

    Rust package matrix = PASS
      mtgml-model 8
      mtgml-state 96
      mtgml-random 43
      mtgml-persistence 10
      mtgml-replay 14
      mtgml-environment 61
      mtgml-wire 10
      mtgml-conformance 127
      mtgml-commander 4

    cargo fmt --all -- --check = PASS
    cargo check --workspace --all-targets --all-features --locked = PASS
    cargo clippy --workspace --all-targets --all-features --locked -- -D warnings = PASS
    cargo test --workspace --all-features --locked = PASS

    verify_repository.py = PASS (680 files, 38 golden, 46 negative fixtures)
    check_rust_source_structure.py = PASS (137 Rust files)
    check_documentation.py = PASS (44 ADRs and local links)
    validate_schemas.py = PASS (38 wire fixtures, 9 maintainer artifacts)
    validate_maintainer_artifacts.py = PASS (7 artifacts)
    verify_python_toolchain.py = PASS (Python 3.13.15, Rust 1.85.1, 6 pins)
    run_python_tests.py --profile full = PASS (396 executed, 3 M2.H adapter scenarios NOT_RUN because the adapter binary is absent)
    generate_contracts.py --check = PASS
    run_checks.py fast = PASS
    run_checks.py integration = PASS
    run_checks.py certification = PASS (the same 3 M2.H adapter scenarios remain NOT_RUN)

    just check-fast = BLOCKED (WSL could not start /bin/bash)
    just check = BLOCKED (WSL could not start /bin/bash)
    just check-all = BLOCKED (WSL could not start /bin/bash)
    just archive-check = BLOCKED (WSL could not start /bin/bash)

    verify_archive_reproducibility.py = PASS
      sha256 = b1ddac7cc0ff0083fba09c7e089351e54fac9af5f2654951a86577d2f2c37478

The earlier non-final integration attempts on c3c9a97 and the immediately
following retry on that style head exposed Ruff and then Mypy issues. They
were corrected by the committed import-order and typed-counter changes; the
final ea40e85 integration and certification profiles exited 0.

The full Python command itself exited 0 for the 396 executed tests. The three
M2.H adapter modules were explicitly skipped because
MTGML_M2_ADAPTER_BIN was unavailable; that prerequisite remains NOT_RUN and
is not upgraded to PASS by the surrounding suite result.

## Compatibility and scope

    RUST_API_CHANGE = YES
    RUST_API_CHANGE_CLASS = INTERNAL_EXPERIMENTAL_NARROWING
    FROZEN_PUBLIC_API_CHANGE = NO
    WIRE_CHANGE = NO
    SCHEMA_CHANGE = NO
    CHECKPOINT_SCHEMA_VERSION_CHANGE = NO
    REPLAY_VERSION_CHANGE = NO
    DIGEST_DOMAIN_CHANGE = NO
    RNG_ALGORITHM_CHANGE = NO
    HISTORICAL_REPLAY_CHANGE = NO
    HISTORICAL_CHECKPOINT_MEANING_CHANGE = NO
    COMMANDER_SUPPORT_CLAIM = NO
    CAPABILITY_REGISTRY_CHANGE = NO
    NEW_MAGIC_SEMANTICS = NO
    M3_STARTED = NO
    M3_AUTHORIZED = NO
    FOUNDATION_READY_FOR_M3 = NO
    MERGE_PERFORMED = NO

The changed-file union from BASE..CODE_VERIFICATION_HEAD, including tracked
and untracked files, contains only the approved Batch-G design/plan records,
the registered evidence target, direct owners, tests, and approved generated
outputs. No card, deck, capability, new schema, wire fixture, checkpoint
fixture, or M3 path was added.
