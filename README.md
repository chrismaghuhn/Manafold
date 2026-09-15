# Manafold

## Current status

- **Foundation closure/freeze:** `COMPLETE` (`FINAL_FOUNDATION_CLOSURE = PASS`, `PRE_M3_REMEDIATION_FREEZE = PASS`, `FOUNDATION_READY_FOR_M3 = YES`)
- **Core modularization:** Issue #162 `COMPLETE`, merged by PR #179; the refactor was semantic-neutral and did not change public, wire, schema, digest, replay, or rules contracts
- **M2.5 scope work:** `NOT_CLAIMED` / `NOT_FROZEN`; the abandoned census and research machinery remains historical Git evidence, not active engine scope
- **Current active work area:** M3 Entry governance under Issue #178; the separate Entry Decision is the next gate
- **Pre-M3 governance cleanup:** `COMPLETE` for this candidate; the next task is the separate M3 Entry Decision and Initial Semantic Foundation selection
- **M3 semantic implementation:** `NOT_STARTED`
- **M3 authorization:** `NOT_AUTHORIZED` (`AUTHORIZATION_HEAD = NOT_SET`); no Issue #178 checkbox or governance PR substitutes for the explicit reviewed authorization
- **Blocked:** no active Magic/card scope. External census M3 authorization does not authorize Manafold engine semantics.
- **Project type:** independent greenfield MTG/ML rules and simulation engine
- **Playable engine:** no
- **Real Magic rules:** no
- **Real card support:** none

Manafold prioritizes:

```text
correctness
→ determinism
→ information safety
→ decision completeness
→ replayability
→ maintainability
→ performance
→ ML scale
```

M1 established the deterministic synthetic kernel shell: complete state construction, accepted/rejected atomic transitions, exact state/event/delta parity, deterministic RNG/allocators, checkpoint/restore/fork/replay parity, and two bound synthetic player endpoints. M2 subsequently closed the decision and synthetic information-safety foundation under the accepted exact-head evidence above.

## Start here

1. [`PROJECT_CHARTER.md`](PROJECT_CHARTER.md)
2. [`AGENTS.md`](AGENTS.md)
3. [`docs/NORMATIVE_HIERARCHY.md`](docs/NORMATIVE_HIERARCHY.md)
4. [`docs/ROADMAP.md`](docs/ROADMAP.md)
5. [`docs/maintenance/MAINTAINER_PROFILES.md`](docs/maintenance/MAINTAINER_PROFILES.md)
6. [`docs/maintenance/DEVELOPER_SETUP.md`](docs/maintenance/DEVELOPER_SETUP.md)
7. [`docs/contracts/ACCEPTANCE_GATES.md`](docs/contracts/ACCEPTANCE_GATES.md)
8. [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)
9. [`docs/DOMAIN_MODEL.md`](docs/DOMAIN_MODEL.md)
10. [`docs/EXECUTION_MODEL.md`](docs/EXECUTION_MODEL.md)
11. [`docs/DECISION_PROTOCOL.md`](docs/DECISION_PROTOCOL.md)
12. [`docs/INFORMATION_MODEL.md`](docs/INFORMATION_MODEL.md)
13. [`docs/ML_ENVIRONMENT.md`](docs/ML_ENVIRONMENT.md)
14. [`docs/STATE_HASHING.md`](docs/STATE_HASHING.md)

The ADR index is [`docs/adr/README.md`](docs/adr/README.md).

Generated verification evidence is external to the reproducible source archive. Historical M1/M2 closure claims come from their recorded exact-head evidence and accepted ADRs; future changes must produce fresh evidence rather than relying on prose status.

The maintainer route is [`docs/maintenance/MAINTAINER_PROFILES.md`](docs/maintenance/MAINTAINER_PROFILES.md), with the durable setup path in [`docs/maintenance/DEVELOPER_SETUP.md`](docs/maintenance/DEVELOPER_SETUP.md). The mandatory PR checks are `PR Fast`, `PR Integration`, and the stable aggregate `manafold-pr-gate`.

## Durable boundaries

```text
Trusted kernel
  complete EngineState, authoritative events, exact StateDelta

Trusted environment controller
  configuration, seed, reset, complete checkpoint, restore, fork,
  authoritative replay, scheduling

Perspective-bound player endpoint
  observation, retained information state, visible decision,
  observed events, submit -> PlayerStep

Rules-free Python/ML
  DTO/client consumption, models, rewards, datasets, experiment policy
  no legality/state/RNG authority
```

No player endpoint can obtain full state, root seed, RNG internals, authoritative events, checkpoints, forks, authoritative replay, trusted IDs, or free-form diagnostics.

## M2 contract boundaries

The accepted M2.A architecture requires:

- separate trusted `DecisionId` and perspective-local `PlayerDecisionIdV1`;
- dense request-local `CandidateIdV1`;
- closed answer variants for choose-one/many/number/order;
- typed serialized continuations inside `EngineState`;
- read-only perspective projection;
- retained knowledge keyed through perspective-local opaque identity;
- opaque identity persistence only while distinguishability persists;
- retirement/new identity after hidden randomization;
- one perspective-local visible event sequence;
- independent bounded soundness/completeness proof;
- paired-state byte noninterference;
- a temporary rules-free Python semantic adapter without resolving production transport;
- one coordinated V3 state/digest/checkpoint/replay identity cut.

`EpisodeStatus` remains environment/PlayerStep semantics, not part of retained information state.

Malformed/noncanonical wire bytes fail before a semantic submission and do not synthesize a PlayerStep.

## Historical identity discipline

M2 must not reinterpret M1 V2 state/checkpoint/replay values.

When the runtime `EngineState` changes:

- V2 full-state production is retired;
- V2 in-memory checkpoint semantics are not kept executable by creating a duplicate legacy state model;
- historical fixtures/domains remain immutable evidence;
- historical replay/read/migration support is explicitly classified.

New V3 persisted semantic digests follow ADR 0038 and the byte-level specification in [`docs/STATE_HASHING.md`](docs/STATE_HASHING.md).

## Support claims

The project uses strict lifecycle language:

```text
Imported -> Parsed -> Implemented -> Covered -> Certified
```

Only a certified locked bundle is a real support claim. Parsed/imported/compiled/implemented artifacts are not automatically supported.

No real cards are added before an explicit reviewed scope and capability closure.

## Local verification

Use:

```bash
just doctor
just check-fast
just check
just check-all
just release-candidate
```

Core direct checks include:

```text
<project-python> scripts/verify_repository.py
<project-python> scripts/check_rust_source_structure.py
<project-python> scripts/check_documentation.py
<project-python> scripts/validate_schemas.py
<project-python> scripts/validate_maintainer_artifacts.py
<project-python> scripts/verify_python_toolchain.py
<project-python> scripts/run_python_tests.py

cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
```

Use the platform-specific `<project-python>` paths in the developer setup
document; the scripts reject a non-pinned Python interpreter. `PASS` is
reported only for commands actually executed successfully. Missing/unavailable
tools are `NOT_RUN` or `BLOCKED`.

## Scope discipline

- no hidden/heuristic completion of player choices;
- unsupported semantics fail closed;
- no rules logic in Python or card generators;
- no real cards before exact V1 deck closure is explicitly reviewed;
- no optimized rollout backend before reference parity and profiling;
- no native card executor in a certified bundle under the current quarantine policy;
- no broad support claim from parsing, compilation, or raw card counts.
