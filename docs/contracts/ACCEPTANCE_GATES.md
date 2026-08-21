# Acceptance Gates

**Status:** normative V0.2.2, M1, M2, and future certification gates

A gate is `PASS` only when its declared executable evidence ran successfully on the identified source/toolchain. `NOT_RUN` and `BLOCKED` do not satisfy closure.

Generated logs/reports live outside the reproducible source archive.

## V0.2.1 contract freeze

Every item required by the V0.2.1 freeze was defined as:

```text
REPOSITORY_VERIFIER_DIRECT_COMMAND
RUST_LEXICAL_STRUCTURE
DOCUMENTATION_REGISTER_AND_LINKS
MAINTAINER_ARTIFACT_SCHEMA_VALIDATION
MAINTAINER_ARTIFACT_SEMANTIC_VALIDATION
PYTHON_REFERENCE_TOOLCHAIN_AND_DIRECT_TOOL_PINS
PYTHON_TESTS
WIRE_SCHEMA_GOLDEN_FIXTURES
RUST_GOLDEN_AND_NEGATIVE_FIXTURES
NONEMPTY_FULL_STATE_DIGEST
COMPLETE_CHECKPOINT_STATUS_LIMITS_AND_DIGEST
REJECTED_REPLAY_REVISION_AND_STATE_IDENTITY
COMPOSITIONAL_MULTI_EVENT_TRANSITIONS
EXACT_CONFORMANCE_INPUT_ASSERTIONS
KNOWLEDGE_PROVENANCE_AND_INVALIDATION
NATIVE_EXECUTOR_DEFINITION_CLOSURE
RUST_FMT
RUST_CHECK
RUST_CLIPPY_DENY_WARNINGS
RUST_TESTS
RUFF_FORMAT
RUFF
MYPY
CARGO_LOCK_COMMITTED_AND_LOCKED_BUILD
DETERMINISTIC_SOURCE_ARCHIVE_LAST
SOURCE_TREE_UNCHANGED_BY_VERIFICATION
```

## V0.2.2 executable freeze

V0.2.2 additionally requires:

- generated-contract drift;
- synthetic golden path;
- committed locked `Cargo.lock`;
- Rust fmt/check/clippy/test;
- Ruff format/check and Mypy;
- no `NOT_RUN`/`FAIL` in the release-candidate result;
- archive reproducibility as the final source-tree gate.

## M1 — Closed deterministic kernel shell

```text
ENGINE_STATE_CONSTRUCTION_AND_INVARIANTS
ACCEPTED_TRANSITION_EXACT_PRODUCT
REJECTED_RESPONSE_COMPLETE_NONMUTATION
STATE_DELTA_FULL_REAPPLICATION
SEQUENTIAL_EVENT_DELTA_PARITY
CHECKPOINT_RESTORE_COMPLETE_IDENTITY
FORK_PARITY
REPLAY_PARITY
DETERMINISTIC_RNG_AND_ALLOCATORS
MULTI_PLAYER_ENDPOINT_BINDING
```

M1 closure evidence merged in PR #47 records all ten gates `PASS`. M2.Final must rerun the complete current M1 closure matrix; compilation or a hand-selected subset is insufficient.

## M2 — Decision Machinery and Synthetic Information Safety

M2 uses exactly these required closure gates:

```text
M2_EXECUTABLE_CONTRACT_AND_VERSION_CUT
CLOSED_DECISION_FAMILY_EXACTNESS
SERIALIZED_CONTINUATION_LIFECYCLE
VISIBLE_DECISION_CANONICAL_ORDER_AND_IDENTITY
PLAYER_PROJECTION_PERSPECTIVE_COHERENCE
PLAYER_SAFE_ERROR_MAPPING_AND_NONDISCLOSURE
KNOWLEDGE_RETENTION_INVALIDATION_AND_HISTORY
OPAQUE_ID_DISTINGUISHABILITY_LIFECYCLE
OBSERVED_EVENT_REDACTION_AND_SEQUENCE
SYNTHETIC_LEGAL_CHOICE_SOUNDNESS
SYNTHETIC_LEGAL_CHOICE_COMPLETENESS
M2_PAIRED_STATE_VISIBLE_BYTES_NONINTERFERENCE
M2_MULTI_ENDPOINT_INFORMATION_ISOLATION
M2_REJECTED_RESPONSE_COMPLETE_NONMUTATION
M2_CHECKPOINT_RESTORE_INFORMATION_IDENTITY
M2_FORK_INFORMATION_PARITY
M2_REPLAY_INFORMATION_PARITY
M2_RUST_PYTHON_PLAYER_WIRE_PARITY
M2_RULES_FREE_PYTHON_ADAPTER_PARITY
M1_GATE_REGRESSION_AND_M2_SCOPE_GUARD
```

### `M2_EXECUTABLE_CONTRACT_AND_VERSION_CUT`

Requires both architecture and executable structural evidence:

- accepted M2 decision/information ADRs;
- exact Decision V2 / information-event-step V2 meanings;
- exact V3 full-state/checkpoint/replay identity plan;
- ADR-0038 digest-envelope and `mtgml.canonical-cbor.v1` byte contract;
- explicit historical V1/V2 support/retirement classifications;
- no reinterpretation of old artifacts;
- Rust/Python/schema/fixture identity coherence where public DTOs exist.

M2.A documentation is only the contract half. The gate cannot become `PASS` until M2.B implements the structural cut and its fixtures.

### Decision gates

`CLOSED_DECISION_FAMILY_EXACTNESS` requires exact accepted/rejected evidence for ChooseOne, ChooseMany, ChooseNumber and Order.

`SERIALIZED_CONTINUATION_LIFECYCLE` requires creation, stage advancement, fresh stage request identities, rejection atomicity, completion/removal, checkpoint/fork/replay parity.

`VISIBLE_DECISION_CANONICAL_ORDER_AND_IDENTITY` requires public-only ordering, dense request-local candidate IDs, perspective-local player-decision IDs, and insertion/global-allocation-history independence.

### Player projection/error gates

`PLAYER_PROJECTION_PERSPECTIVE_COHERENCE` requires observation/information/decision/step perspective+revision consistency and transitive absence of privileged types.

`PLAYER_SAFE_ERROR_MAPPING_AND_NONDISCLOSURE` covers three separate layers:

- malformed/noncanonical wire bytes → closed wire error, no semantic PlayerStep;
- typed semantic rejection → closed submission code and unchanged semantic/player state;
- invariant/internal failure → closed service failure only.

Wrong actor/private request and binding/internal mismatch must not become information oracles.

### Information gates

`KNOWLEDGE_RETENTION_INVALIDATION_AND_HISTORY` covers public/own-private/private-look/reveal/tracked-hidden/history/forget/randomization lifecycle.

`OPAQUE_ID_DISTINGUISHABILITY_LIFECYCLE` proves identity persistence while distinguishable, retirement after indistinguishability/randomization, deterministic new identity, no reuse, and read-only projection.

`OBSERVED_EVENT_REDACTION_AND_SEQUENCE` proves public/private/mixed/hidden audience behavior, contiguous perspective-local visible sequence, and absence of authoritative IDs/RNG provenance.

### Legal-space gates

`SYNTHETIC_LEGAL_CHOICE_SOUNDNESS`: every emitted/reachable protocol choice is legal under the independent bounded synthetic reference model.

`SYNTHETIC_LEGAL_CHOICE_COMPLETENESS`: every reference-legal synthetic complete choice has exactly one canonical reachable protocol path.

The conformance oracle is test-only and cannot be imported by production rules/environment code.

### Noninterference/endpoints

`M2_PAIRED_STATE_VISIBLE_BYTES_NONINTERFERENCE` compares canonical player bytes across paired valid states differing only in unauthorized information. It covers observation, information state, candidate set/order/IDs, events/sequence, errors, PlayerStep, digests and protocol metadata.

`M2_MULTI_ENDPOINT_INFORMATION_ISOLATION` proves two coexisting permanently bound endpoints, private/public/mixed projections, wrong-perspective nonmutation, restore binding, and fork-controller isolation.

### Determinism/rejection gates

`M2_REJECTED_RESPONSE_COMPLETE_NONMUTATION` covers the entire M2 semantic fingerprint: continuation, knowledge, perspective IDs/allocators, visible sequence, RNG, counters, replay and player bytes.

Wire-decode failure is earlier than semantic response rejection but must independently prove zero mutation.

`M2_CHECKPOINT_RESTORE_INFORMATION_IDENTITY`, `M2_FORK_INFORMATION_PARITY`, and `M2_REPLAY_INFORMATION_PARITY` require exact authoritative and per-perspective continuation/information identity.

### Python gates

`M2_RUST_PYTHON_PLAYER_WIRE_PARITY` requires byte-exact Rust/Python canonical fixtures for all public M2 DTOs and exact negative-corpus agreement.

`M2_RULES_FREE_PYTHON_ADAPTER_PARITY` requires a temporary non-published Python consumer to drive the real Rust perspective-safe endpoint without legality/state/RNG/replay authority. It does not resolve OD-009.

### Final scope/regression

`M1_GATE_REGRESSION_AND_M2_SCOPE_GUARD` requires:

- all ten M1 gates rerun successfully;
- no real Magic/card/deck/M2.5 capability work;
- no production Python/native transport;
- no stable trajectory/action-key contract;
- no search/determinization/vector/distributed training work;
- exact source/toolchain identity;
- unchanged source after verification;
- archive/reproducibility gate last.

Only the final generated M2 closure report may emit:

```text
M2 = COMPLETE
M2.5 = UNBLOCKED
```

## Capability coverage

Reusable capability proof families remain:

```text
AUTHORITY_AND_SCOPE_SPECIFIED
REFERENCE_IMPLEMENTATION
EXACT_CONFORMANCE_CASES
LEGAL_ACTION_SOUNDNESS
LEGAL_ACTION_COMPLETENESS
INFORMATION_NONINTERFERENCE
REPLAY_CHECKPOINT_PARITY
PROPERTY_FUZZ_REQUIREMENTS
PERFORMANCE_DIAGNOSTICS
```

## V1 bundle certification

```text
EXACT_DECK_AND_SOURCE_SNAPSHOTS
REACHABLE_DEFINITION_CLOSURE
RECURSIVE_CAPABILITY_CLOSURE
NATIVE_EXECUTOR_DISCOVERED_FROM_DEFINITIONS
NO_UNAPPROVED_NATIVE_EXECUTOR
NO_UNSUPPORTED_OR_HEURISTIC_CHOICE
ALL_CARD_AND_INTERACTION_CASES
MILLIONS_OF_LOCKED_RANDOM_OR_HEURISTIC_TRANSITIONS_WITHOUT_INVARIANT_FAILURE
SOAK_AND_MEMORY_GATES
REFERENCE_HARDWARE_PERFORMANCE_BUDGETS
CLEAN_MACHINE_REPRODUCTION
SIGNED_OR_CHECKSUMMED_EVIDENCE_MANIFEST
```

Numerical thresholds are set at roadmap M2.5 and cannot be retroactively weakened without an ADR and new bundle identity.
