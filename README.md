# MTG ML Engine Foundation

- **Current revision:** V0.2.2 — Executable Freeze & Maintainer Ergonomics
- **Project type:** independent greenfield MTG/ML engine foundation
- **Playable engine:** no
- **Real card support:** none

V0.2.1 closed the executable contract defects. V0.2.2 preserves those semantics and reduces maintainer drift with a narrow single-source contract catalog, generated cross-language vocabulary, staged checks, split CI, and a tested synthetic golden path.

V0.2.2 deliberately does **not** invent unpinned Magic semantics or claim a working game. The first executable kernel remains M1. M1 is unblocked.

## Start here

1. [`PROJECT_CHARTER.md`](PROJECT_CHARTER.md)
2. [`docs/M0_2_SPECIFICATION.md`](docs/M0_2_SPECIFICATION.md)
3. [`docs/V0_2_2_EXECUTABLE_FREEZE_AND_MAINTAINER_ERGONOMICS.md`](docs/V0_2_2_EXECUTABLE_FREEZE_AND_MAINTAINER_ERGONOMICS.md)
4. [`docs/V0_2_1_CONTRACT_CLOSURE.md`](docs/V0_2_1_CONTRACT_CLOSURE.md)
5. [`docs/NORMATIVE_HIERARCHY.md`](docs/NORMATIVE_HIERARCHY.md)
6. [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)
7. [`docs/DOMAIN_MODEL.md`](docs/DOMAIN_MODEL.md)
8. [`docs/EXECUTION_MODEL.md`](docs/EXECUTION_MODEL.md)
9. [`docs/contracts/M0_2_DESIGN_LOCK_MATRIX.md`](docs/contracts/M0_2_DESIGN_LOCK_MATRIX.md)
10. [`docs/contracts/ACCEPTANCE_GATES.md`](docs/contracts/ACCEPTANCE_GATES.md)
11. [`docs/ROADMAP.md`](docs/ROADMAP.md)

Generated verification evidence is intentionally external to the source archive. After running verification, read `dist/verification/FOUNDATION_VERIFICATION.md` and `dist/verification/FOUNDATION_BLOCKERS.md`.

## Durable boundaries

```text
Trusted kernel
  complete EngineState, authoritative events, exact StateDelta

Trusted environment controller
  configuration, seed, reset, complete checkpoint, restore, fork,
  authoritative replay, scheduling

Perspective-bound player endpoint
  observation, information state, visible decision,
  observed events, submit -> PlayerStep

Rules-free ML process
  models, rewards, replay buffer, experiment policy, analytics
```

No player endpoint can obtain full state, root seed, RNG counters, authoritative events, checkpoints, forks, or authoritative replay export.

## What the current foundation fixes

- machine-readable normative-document hierarchy and contradiction blocking;
- explicit crate ownership and dependency direction;
- closed authoritative state, identity allocators, continuations, knowledge, format state, and RNG;
- complete trusted checkpoints including status, limit counters, codec identity, and a typed checkpoint digest;
- canonical, domain-typed state and artifact digests;
- compositional ordered semantic-event validation inside atomic revisions;
- perspective-bound observations, information state, decisions, events, and safe errors;
- exact cross-language Rust/Python/JSON wire fixtures;
- detailed rule/mechanic and card contribution lifecycles;
- recursive capability and generated-object closure;
- native-executor quarantine derived from actual definition closure;
- conformance, noninterference, property/fuzz, replay, and benchmark authoring guides;
- state-independent deterministic source archives and adjacent verification evidence.

## Support claims

The project uses strict lifecycle language:

```text
Imported -> Parsed -> Implemented -> Covered -> Certified
```

Only **Certified** capability bundles are support claims. Parser success, compilation, a green unit test, or a raw card count is not certification.

## Local verification

Use the maintainer profiles:

```bash
just doctor
just check-fast
just check
just check-all
just release-candidate
```

The underlying structural gates are:

```bash
python scripts/verify_repository.py
python scripts/check_rust_source_structure.py
python scripts/check_documentation.py
python scripts/validate_schemas.py
python scripts/validate_maintainer_artifacts.py
python scripts/verify_python_toolchain.py
python scripts/run_python_tests.py
```

The pinned native/tooling gates are:

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
ruff format --check python scripts
ruff check python scripts
mypy --config-file python/pyproject.toml
```

Generate one authoritative adjacent result set with:

```bash
python scripts/run_verification.py
```

The command writes only beneath `dist/verification/`, which is excluded from the source archive. The deterministic archive gate runs last. `NOT_RUN` and `FAIL` are never promoted to `PASS`.

## Scope discipline

- no hidden or heuristic completion of player choices;
- unsupported semantics fail closed;
- no rules logic in Python or card generators;
- no real cards before exact V1 deck closure in M2.5;
- no optimized rollout backend before reference parity and profiling;
- no native card executor in a certified bundle until its policy and API are accepted;
- no broad support claim based on parsing, compilation, or raw card counts.
